// SPDX-License-Identifier: GPL-3.0-or-later

//! The send-table check (design spec 8.3): one wrapper around a model's transport that looks at
//! every outgoing write before it is sent. A write that is not in the model's send table is
//! refused with `CameraError::NotInSendTable` (the daemon answers `internal` and logs it) and
//! never reaches the camera. Independently of the table it refuses two whole classes: any
//! extension-unit selector the model's registry entry does not declare (reads included: no
//! probing), and on a receiver that answers identity queries every frame that is not one of the
//! model's identity queries.

use crate::contract::camera::Result;
use crate::contract::{CameraError, ControlRange, Frame, OnAir, SendTable, Transport};
use obsbot_proto::Usb;
use std::collections::BTreeSet;

/// Where a model's frames go: the selector of its framed commands and queries, the selector
/// of its short commands, and every selector it uses (its registry entry). A write to any other
/// declared selector is a whole-buffer write ([`Frame::Block`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Selectors {
    pub framed: u8,
    pub short: u8,
    pub all: &'static [u8],
}

/// The frame a write is, by the selector it goes to and its bytes; `Err` with the reason when it
/// is no frame of a known kind.
pub fn classify(
    selectors: &Selectors,
    selector: u8,
    data: &[u8],
) -> std::result::Result<Frame, String> {
    if !selectors.all.contains(&selector) {
        return Err(format!(
            "selector {selector:#04x} is not a selector of this model"
        ));
    }
    if selector == selectors.framed {
        if data.len() < 12 || data[0] != 0xaa {
            return Err(format!(
                "{} bytes on selector {selector:#04x} that are no frame",
                data.len()
            ));
        }
        let receiver = data[9];
        let command = u16::from_le_bytes([data[10], data[11]]);
        return match data[1] {
            0x25 => Ok(Frame::Framed {
                selector,
                receiver,
                command,
            }),
            0x01 | 0x21 => Ok(Frame::Query {
                selector,
                receiver,
                command,
            }),
            kind => Err(format!(
                "frame type {kind:#04x} on selector {selector:#04x}"
            )),
        };
    }
    if selector == selectors.short {
        return match data.first() {
            Some(command) => Ok(Frame::Short {
                selector,
                command: *command,
            }),
            None => Err(format!(
                "an empty short command on selector {selector:#04x}"
            )),
        };
    }
    Ok(Frame::Block { selector })
}

/// A short description of a frame for error messages and logs.
pub fn describe(frame: &Frame) -> String {
    match frame {
        Frame::Framed {
            selector,
            receiver,
            command,
        } => {
            format!("framed {command:#06x} to receiver {receiver:#04x} on selector {selector:#04x}")
        }
        Frame::Query {
            selector,
            receiver,
            command,
        } => {
            format!("query {command:#06x} to receiver {receiver:#04x} on selector {selector:#04x}")
        }
        Frame::Short { selector, command } => {
            format!("short {command:#04x} on selector {selector:#04x}")
        }
        Frame::Uvc { control } => format!("UVC control {control:#010x}"),
        Frame::Block { selector } => format!("buffer write on selector {selector:#04x}"),
    }
}

/// A transport that sends only what the model's send table allows.
pub struct Checked<T: Transport> {
    inner: T,
    table: SendTable,
    selectors: Selectors,
    identity_receivers: BTreeSet<u8>,
}

impl<T: Transport> Checked<T> {
    pub fn new(inner: T, table: SendTable, selectors: Selectors) -> Checked<T> {
        let identity_receivers = table
            .entries
            .iter()
            .filter(|e| e.identity)
            .filter_map(|e| match e.frame {
                Frame::Query { receiver, .. } | Frame::Framed { receiver, .. } => Some(receiver),
                _ => None,
            })
            .collect();
        Checked {
            inner,
            table,
            selectors,
            identity_receivers,
        }
    }

    /// Whether `frame` may be sent: in the table, and not on an identity receiver unless it is
    /// one of the identity queries.
    pub fn allows(&self, frame: &Frame) -> std::result::Result<(), String> {
        let identity_receiver = match frame {
            Frame::Framed { receiver, .. } | Frame::Query { receiver, .. } => {
                self.identity_receivers.contains(receiver)
            }
            _ => false,
        };
        let entry = self.table.entries.iter().find(|e| e.frame == *frame);
        match entry {
            Some(e) if identity_receiver && !e.identity => Err(format!(
                "{}: the receiver answers identity queries and this is none",
                describe(frame)
            )),
            Some(_) => Ok(()),
            None if identity_receiver => Err(format!(
                "{}: the receiver answers identity queries and this is none",
                describe(frame)
            )),
            None => Err(format!("{} is not in the send table", describe(frame))),
        }
    }

    fn selector_declared(&self, selector: u8) -> Result<()> {
        if self.selectors.all.contains(&selector) {
            Ok(())
        } else {
            Err(CameraError::NotInSendTable(format!(
                "selector {selector:#04x} is not a selector of this model"
            )))
        }
    }
}

impl<T: Transport> Transport for Checked<T> {
    fn xu_len(&mut self, selector: u8) -> Result<u16> {
        self.selector_declared(selector)?;
        self.inner.xu_len(selector)
    }

    fn xu_get(&mut self, selector: u8, buf: &mut [u8]) -> Result<()> {
        self.selector_declared(selector)?;
        self.inner.xu_get(selector, buf)
    }

    fn xu_set(&mut self, selector: u8, data: &[u8]) -> Result<()> {
        let frame =
            classify(&self.selectors, selector, data).map_err(CameraError::NotInSendTable)?;
        self.allows(&frame).map_err(CameraError::NotInSendTable)?;
        self.inner.xu_set(selector, data)
    }

    fn ctrl_get(&mut self, id: u32) -> Result<i32> {
        self.inner.ctrl_get(id)
    }

    fn ctrl_set(&mut self, id: u32, value: i32) -> Result<()> {
        self.allows(&Frame::Uvc { control: id })
            .map_err(CameraError::NotInSendTable)?;
        self.inner.ctrl_set(id, value)
    }

    fn ctrl_range(&mut self, id: u32) -> Result<Option<ControlRange>> {
        self.inner.ctrl_range(id)
    }

    fn usb(&mut self) -> Result<Usb> {
        self.inner.usb()
    }

    fn usb_serial(&mut self) -> Result<Option<String>> {
        self.inner.usb_serial()
    }

    fn on_air(&mut self) -> Result<OnAir> {
        self.inner.on_air()
    }

    fn hold(&mut self) -> Result<()> {
        self.inner.hold()
    }

    fn release(&mut self) {
        self.inner.release()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract::SendEntry;
    use crate::family::frame::{frame, query_frame};
    use crate::transport::replay::{Replay, Sent, Step};

    const SELECTORS: Selectors = Selectors {
        framed: 2,
        short: 6,
        all: &[2, 6, 0x0c, 0x0d],
    };

    fn entry(frame: Frame, identity: bool) -> SendEntry {
        SendEntry {
            frame,
            evidence: "capture 01",
            identity,
        }
    }

    fn table() -> SendTable {
        SendTable {
            entries: vec![
                entry(
                    Frame::Framed {
                        selector: 2,
                        receiver: 0x04,
                        command: 0x3b84,
                    },
                    false,
                ),
                entry(
                    Frame::Query {
                        selector: 2,
                        receiver: 0x04,
                        command: 0x0104,
                    },
                    false,
                ),
                entry(
                    Frame::Query {
                        selector: 2,
                        receiver: 0x0d,
                        command: 0x0408,
                    },
                    true,
                ),
                // a broken table entry: a command on the identity receiver
                entry(
                    Frame::Framed {
                        selector: 2,
                        receiver: 0x0d,
                        command: 0x0408,
                    },
                    false,
                ),
                entry(
                    Frame::Short {
                        selector: 6,
                        command: 0x1a,
                    },
                    false,
                ),
                entry(
                    Frame::Uvc {
                        control: 0x0098_0900,
                    },
                    false,
                ),
                entry(Frame::Block { selector: 0x0c }, false),
            ],
        }
    }

    fn checked() -> (Checked<Replay>, crate::transport::replay::Log) {
        let replay = Replay::new(2).buffer(0x0c, [1; 60]).buffer(0x07, [0; 60]);
        let log = replay.log();
        (Checked::new(replay, table(), SELECTORS), log)
    }

    #[test]
    fn what_the_table_names_goes_through() {
        let (mut t, log) = checked();
        t.xu_set(2, &frame(0x04, 0x3b84, &[1], 1)).unwrap();
        t.xu_set(2, &query_frame(0x04, 0x0104, 2)).unwrap();
        t.xu_set(2, &query_frame(0x0d, 0x0408, 3)).unwrap();
        t.xu_set(6, &[0x1a, 0x01, 0x02]).unwrap();
        t.xu_set(0x0c, &[1; 60]).unwrap();
        t.ctrl_set(0x0098_0900, 5).unwrap();
        assert_eq!(log.sent().len(), 6);
    }

    #[test]
    fn the_lock_reaches_the_inner_transport() {
        // the per-operation node lock (5.8) is the inner transport's: without these two the
        // trait's empty defaults would run every exchange of a model without the lock
        let (mut t, log) = checked();
        t.hold().unwrap();
        t.xu_set(2, &query_frame(0x04, 0x0104, 1)).unwrap();
        t.release();
        assert_eq!(log.steps(), [Step::Hold, Step::Set(2), Step::Release]);
    }

    #[test]
    fn anything_else_is_refused_and_never_sent() {
        let (mut t, log) = checked();
        let refused = |r: Result<()>| matches!(r, Err(CameraError::NotInSendTable(_)));
        assert!(
            refused(t.xu_set(2, &frame(0x04, 0x3b44, &[1], 1))),
            "another command id"
        );
        assert!(
            refused(t.xu_set(2, &frame(0x02, 0x3b84, &[1], 1))),
            "another receiver"
        );
        assert!(
            refused(t.xu_set(2, &frame(0x04, 0x0104, &[], 1))),
            "a command with the id of an allowed query"
        );
        assert!(
            refused(t.xu_set(2, &query_frame(0x04, 0x3b84, 1))),
            "a query with the id of an allowed command"
        );
        assert!(
            refused(t.xu_set(6, &[0x1b, 0x01, 0x00])),
            "another short command"
        );
        assert!(
            refused(t.xu_set(0x0d, &[0; 60])),
            "a buffer write on another selector"
        );
        assert!(refused(t.ctrl_set(0x0098_0901, 1)), "another control");
        assert!(
            refused(t.xu_set(2, &[0xaa, 0x99, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0])),
            "unknown type"
        );
        assert!(refused(t.xu_set(2, &[1, 2, 3])), "no frame");
        assert!(refused(t.xu_set(6, &[])), "an empty short command");
        assert!(log.sent().is_empty(), "nothing reached the transport");
    }

    #[test]
    fn an_undeclared_selector_is_refused_for_reads_too() {
        let (mut t, log) = checked();
        let mut buf = [0u8; 60];
        assert!(matches!(
            t.xu_get(0x07, &mut buf),
            Err(CameraError::NotInSendTable(_))
        ));
        assert!(matches!(
            t.xu_len(0x07),
            Err(CameraError::NotInSendTable(_))
        ));
        assert!(matches!(
            t.xu_set(0x07, &[0; 60]),
            Err(CameraError::NotInSendTable(_))
        ));
        assert!(
            t.xu_get(0x0c, &mut buf).is_ok(),
            "a declared selector reads"
        );
        assert!(log.sent().is_empty());
    }

    #[test]
    fn the_identity_receiver_takes_only_identity_queries() {
        let (mut t, log) = checked();
        // in the table, but a command on the receiver that answers identity queries (8.3)
        let r = t.xu_set(2, &frame(0x0d, 0x0408, &[0], 1));
        assert!(
            matches!(&r, Err(CameraError::NotInSendTable(m)) if m.contains("identity")),
            "{r:?}"
        );
        // not in the table and on that receiver: the identity rule names it too
        let r = t.xu_set(2, &query_frame(0x0d, 0x1948, 2));
        assert!(
            matches!(&r, Err(CameraError::NotInSendTable(m)) if m.contains("identity")),
            "{r:?}"
        );
        assert!(log.sent().is_empty());
        t.xu_set(2, &query_frame(0x0d, 0x0408, 3)).unwrap();
        assert_eq!(
            log.sent(),
            [Sent::Xu {
                selector: 2,
                data: query_frame(0x0d, 0x0408, 3).to_vec()
            }]
        );
    }

    #[test]
    fn classify_names_each_kind() {
        assert_eq!(
            classify(&SELECTORS, 2, &frame(0x04, 0x6444, &[0; 12], 1)),
            Ok(Frame::Framed {
                selector: 2,
                receiver: 0x04,
                command: 0x6444
            })
        );
        assert_eq!(
            classify(
                &SELECTORS,
                2,
                &crate::family::frame::query_frame_with(0x04, 0x1504, &[0; 4], 1)
            ),
            Ok(Frame::Query {
                selector: 2,
                receiver: 0x04,
                command: 0x1504
            })
        );
        assert_eq!(
            classify(&SELECTORS, 6, &[0x16, 0x02, 0x00, 0x00]),
            Ok(Frame::Short {
                selector: 6,
                command: 0x16
            })
        );
        assert_eq!(
            classify(&SELECTORS, 0x0c, &[0; 60]),
            Ok(Frame::Block { selector: 0x0c })
        );
        assert!(classify(&SELECTORS, 0x09, &[0; 60]).is_err());
    }
}
