// SPDX-License-Identifier: GPL-3.0-or-later

//! The replay transport (design spec 3.1, 11): it answers from recorded exchanges, so that a
//! model's encoders, decoders and flows run in tests without a camera. It sends nothing
//! anywhere and opens no file.
//!
//! - A query written to the framed selector is answered from the table of recorded answers,
//!   by receiver, command and request: the next read of that selector returns the answer with
//!   the query's sequence number. A query without a recorded answer leaves the buffer as it
//!   was, so the model sees no answer, as from a camera that does not answer.
//! - A read of another selector returns its recorded buffer (the status block), or the next of
//!   its recorded reads (the entries of a list), in order.
//! - A UVC control reads its recorded value; a write stores the new value, as V4L2 keeps the
//!   last value written.
//! - Every write (extension-unit buffer or UVC control) is appended to the log, in order.
//! - Every hold and release of the node lock and every extension-unit read and write is
//!   appended to the log's steps, in order, for the tests of the lock (5.8).

use crate::contract::camera::Result;
use crate::contract::{CameraError, ControlRange, OnAir, Transport};
use crate::family::frame::{FRAME_SIZE, crc16_usb};
use obsbot_proto::Usb;
use std::collections::{BTreeMap, VecDeque};
use std::sync::{Arc, Mutex, PoisonError};

/// One write the transport received.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Sent {
    /// An extension-unit SET_CUR: the selector and the bytes as given.
    Xu { selector: u8, data: Vec<u8> },
    /// A UVC control write.
    Ctrl { id: u32, value: i32 },
}

/// One step of the transport, for the tests of the node lock (5.8): a hold or a release of
/// the lock, or an extension-unit write or read on a selector.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    Hold,
    Release,
    Set(u8),
    Get(u8),
}

/// The log of a replay transport, kept by a test after the transport moved into a model.
#[derive(Debug, Clone, Default)]
pub struct Log {
    sent: Arc<Mutex<Vec<Sent>>>,
    steps: Arc<Mutex<Vec<Step>>>,
}

impl Log {
    /// Everything written so far, in order.
    pub fn sent(&self) -> Vec<Sent> {
        self.sent
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// Every hold, release, write and read so far, in order.
    pub fn steps(&self) -> Vec<Step> {
        self.steps
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// Forgets what was written and done so far.
    pub fn clear(&self) {
        self.sent
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clear();
        self.steps
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clear();
    }

    fn push(&self, sent: Sent) {
        self.sent
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(sent);
    }

    fn step(&self, step: Step) {
        self.steps
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(step);
    }
}

#[derive(Debug, Clone)]
enum Reply {
    Value(Vec<u8>),
    Rejected(i16),
    Empty,
}

/// An answer frame in the layout the camera's answers have in the recordings: `aa`, the type
/// (`0x29` value, `0x89` rejected, `0x09` empty), the query's sequence number, the header
/// length, the header CRC (CRC-16/USB over the header with the CRC field zero), the query's
/// receiver at 8 and `0x0a` at 9, the command echoed at 10..12. A value has a 12-byte header, its length at 12..14, the data CRC at 14..16
/// (CRC-16/USB over bytes 12 to the end of the data with that field zero) and the data from 16
/// (captures 04, 08, 11). An empty answer has a 12-byte header and nothing after it (capture
/// 11). A rejection has a 16-byte header whose extension `00 02` and i16 code fill 12..16
/// (capture 01).
///
/// # Panics
/// If `data` is longer than the 44 bytes after the headers.
pub fn answer_frame(
    kind: u8,
    seq: u16,
    receiver: u8,
    command: u16,
    code: Option<i16>,
    data: &[u8],
) -> [u8; FRAME_SIZE] {
    assert!(
        data.len() <= FRAME_SIZE - 16,
        "an answer holds at most 44 bytes"
    );
    let mut r = [0u8; FRAME_SIZE];
    r[0] = 0xaa;
    r[1] = kind;
    r[2..4].copy_from_slice(&seq.to_le_bytes());
    r[8] = receiver;
    r[9] = 0x0a;
    r[10..12].copy_from_slice(&command.to_le_bytes());
    let header = match code {
        Some(code) => {
            r[12..14].copy_from_slice(&[0x00, 0x02]);
            r[14..16].copy_from_slice(&code.to_le_bytes());
            16
        }
        None => {
            if kind != 0x09 {
                r[12..14].copy_from_slice(&(data.len() as u16).to_le_bytes());
                r[16..16 + data.len()].copy_from_slice(data);
                let crc = crc16_usb(&r[12..16 + data.len()]);
                r[14..16].copy_from_slice(&crc.to_le_bytes());
            }
            12
        }
    };
    r[4..6].copy_from_slice(&(header as u16).to_le_bytes());
    let crc = crc16_usb(&r[..header]);
    r[6..8].copy_from_slice(&crc.to_le_bytes());
    r
}

/// The replay transport. Build it with the recorded answers, then hand it to a model.
#[derive(Debug)]
pub struct Replay {
    framed: u8,
    answers: BTreeMap<(u8, u16, Vec<u8>), Reply>,
    buffers: BTreeMap<u8, [u8; FRAME_SIZE]>,
    reads: BTreeMap<u8, VecDeque<[u8; FRAME_SIZE]>>,
    ctrls: BTreeMap<u32, i32>,
    ranges: BTreeMap<u32, ControlRange>,
    usb: Option<Usb>,
    serial: Option<String>,
    on_air: OnAir,
    pending: [u8; FRAME_SIZE],
    gone: bool,
    log: Log,
}

impl Replay {
    /// A replay whose framed commands, queries and answers use `framed` (2 on the Tiny 2 Lite).
    pub fn new(framed: u8) -> Replay {
        Replay {
            framed,
            answers: BTreeMap::new(),
            buffers: BTreeMap::new(),
            reads: BTreeMap::new(),
            ctrls: BTreeMap::new(),
            ranges: BTreeMap::new(),
            usb: None,
            serial: None,
            on_air: OnAir::default(),
            pending: [0; FRAME_SIZE],
            gone: false,
            log: Log::default(),
        }
    }

    /// The recorded value answer to `command` on `receiver` (a query without a request).
    pub fn answer(self, receiver: u8, command: u16, data: &[u8]) -> Replay {
        self.answer_to(receiver, command, &[], data)
    }

    /// The recorded value answer to `command` on `receiver` for one request (a view id).
    pub fn answer_to(mut self, receiver: u8, command: u16, request: &[u8], data: &[u8]) -> Replay {
        self.answers.insert(
            (receiver, command, request.to_vec()),
            Reply::Value(data.to_vec()),
        );
        self
    }

    /// A recorded refusal (type `0x89`) with the camera's error code.
    pub fn reject(mut self, receiver: u8, command: u16, code: i16) -> Replay {
        self.answers
            .insert((receiver, command, Vec::new()), Reply::Rejected(code));
        self
    }

    /// A recorded answer without data (type `0x09`: nothing to list).
    pub fn empty(mut self, receiver: u8, command: u16) -> Replay {
        self.answers
            .insert((receiver, command, Vec::new()), Reply::Empty);
        self
    }

    /// What every read of `selector` returns (the status block).
    pub fn buffer(mut self, selector: u8, block: [u8; FRAME_SIZE]) -> Replay {
        self.buffers.insert(selector, block);
        self
    }

    /// Successive reads of `selector` (the entries of the preset list), before its buffer.
    pub fn reads(mut self, selector: u8, blocks: &[[u8; FRAME_SIZE]]) -> Replay {
        self.reads
            .entry(selector)
            .or_default()
            .extend(blocks.iter().copied());
        self
    }

    pub fn ctrl(mut self, id: u32, value: i32) -> Replay {
        self.ctrls.insert(id, value);
        self
    }

    pub fn range(mut self, id: u32, range: ControlRange) -> Replay {
        self.ranges.insert(id, range);
        self
    }

    pub fn usb(mut self, usb: Usb) -> Replay {
        self.usb = Some(usb);
        self
    }

    pub fn serial(mut self, serial: &str) -> Replay {
        self.serial = Some(serial.to_string());
        self
    }

    pub fn on_air(mut self, on_air: OnAir) -> Replay {
        self.on_air = on_air;
        self
    }

    /// A device that is gone: every call fails with `Gone`.
    pub fn gone(mut self) -> Replay {
        self.gone = true;
        self
    }

    /// The log of every write, shared with this transport.
    pub fn log(&self) -> Log {
        self.log.clone()
    }

    fn here(&self) -> Result<()> {
        if self.gone {
            Err(CameraError::Gone)
        } else {
            Ok(())
        }
    }

    /// A query frame (`aa 01` or `aa 21`) written to the framed selector: prepares its answer.
    fn query(&mut self, data: &[u8]) {
        if data.len() < 16 || data[0] != 0xaa || !matches!(data[1], 0x01 | 0x21) {
            return;
        }
        let seq = u16::from_le_bytes([data[2], data[3]]);
        let receiver = data[9];
        let command = u16::from_le_bytes([data[10], data[11]]);
        let request = if data[1] == 0x21 {
            let len = usize::from(u16::from_le_bytes([data[12], data[13]]));
            data.get(16..16 + len).unwrap_or(&[]).to_vec()
        } else {
            Vec::new()
        };
        if let Some(reply) = self.answers.get(&(receiver, command, request)) {
            self.pending = match reply {
                Reply::Value(d) => answer_frame(0x29, seq, receiver, command, None, d),
                Reply::Rejected(code) => {
                    answer_frame(0x89, seq, receiver, command, Some(*code), &[])
                }
                Reply::Empty => answer_frame(0x09, seq, receiver, command, None, &[]),
            };
        }
    }
}

impl Transport for Replay {
    fn xu_len(&mut self, _selector: u8) -> Result<u16> {
        self.here()?;
        Ok(FRAME_SIZE as u16)
    }

    fn xu_get(&mut self, selector: u8, buf: &mut [u8]) -> Result<()> {
        self.here()?;
        self.log.step(Step::Get(selector));
        let block = if selector == self.framed {
            self.pending
        } else if let Some(next) = self.reads.get_mut(&selector).and_then(VecDeque::pop_front) {
            next
        } else if let Some(block) = self.buffers.get(&selector) {
            *block
        } else {
            return Err(CameraError::Io(format!(
                "selector {selector:#04x} is not recorded"
            )));
        };
        let n = buf.len().min(FRAME_SIZE);
        buf[..n].copy_from_slice(&block[..n]);
        Ok(())
    }

    fn xu_set(&mut self, selector: u8, data: &[u8]) -> Result<()> {
        self.here()?;
        self.log.step(Step::Set(selector));
        self.log.push(Sent::Xu {
            selector,
            data: data.to_vec(),
        });
        if selector == self.framed {
            self.query(data);
        }
        Ok(())
    }

    fn ctrl_get(&mut self, id: u32) -> Result<i32> {
        self.here()?;
        self.ctrls
            .get(&id)
            .copied()
            .ok_or_else(|| CameraError::Io(format!("control {id:#010x} is not recorded")))
    }

    fn ctrl_set(&mut self, id: u32, value: i32) -> Result<()> {
        self.here()?;
        self.log.push(Sent::Ctrl { id, value });
        self.ctrls.insert(id, value);
        Ok(())
    }

    fn ctrl_range(&mut self, id: u32) -> Result<Option<ControlRange>> {
        self.here()?;
        Ok(self.ranges.get(&id).copied())
    }

    fn usb(&mut self) -> Result<Usb> {
        self.here()?;
        self.usb.clone().ok_or(CameraError::Gone)
    }

    fn usb_serial(&mut self) -> Result<Option<String>> {
        self.here()?;
        Ok(self.serial.clone())
    }

    fn on_air(&mut self) -> Result<OnAir> {
        self.here()?;
        Ok(self.on_air.clone())
    }

    fn hold(&mut self) -> Result<()> {
        self.log.step(Step::Hold);
        Ok(())
    }

    fn release(&mut self) {
        self.log.step(Step::Release);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::family::frame::{Answer, frame, parse_answer, query_frame, query_frame_with};

    fn read(t: &mut Replay, selector: u8) -> [u8; FRAME_SIZE] {
        let mut buf = [0u8; FRAME_SIZE];
        t.xu_get(selector, &mut buf).unwrap();
        buf
    }

    #[test]
    fn a_recorded_query_is_answered_with_its_sequence_number() {
        let mut t = Replay::new(2).answer(0x04, 0x0104, &[1, 2, 3]);
        t.xu_set(2, &query_frame(0x04, 0x0104, 0x0123)).unwrap();
        let reply = read(&mut t, 2);
        assert_eq!(
            parse_answer(&reply, 0x0123, 0x0104),
            Some(Answer::Value(&[1, 2, 3]))
        );
    }

    #[test]
    fn answers_are_kept_apart_by_receiver_and_request() {
        let mut t = Replay::new(2)
            .answer(0x0d, 0x0408, &[1, 8, 2, 6])
            .answer_to(0x04, 0x1504, &0u32.to_le_bytes(), b"Desk")
            .answer_to(0x04, 0x1504, &1u32.to_le_bytes(), b"Door");
        t.xu_set(2, &query_frame_with(0x04, 0x1504, &1u32.to_le_bytes(), 9))
            .unwrap();
        assert_eq!(
            parse_answer(&read(&mut t, 2), 9, 0x1504),
            Some(Answer::Value(b"Door"))
        );
        // the same command on another receiver has no recorded answer: the buffer stays
        let before = read(&mut t, 2);
        t.xu_set(2, &query_frame(0x04, 0x0408, 10)).unwrap();
        assert_eq!(parse_answer(&read(&mut t, 2), 10, 0x0408), None);
        assert_eq!(
            read(&mut t, 2),
            before,
            "an unrecorded query leaves the answer buffer as it was"
        );
        t.xu_set(2, &query_frame(0x0d, 0x0408, 11)).unwrap();
        assert_eq!(
            parse_answer(&read(&mut t, 2), 11, 0x0408),
            Some(Answer::Value(&[1, 8, 2, 6]))
        );
    }

    #[test]
    fn refusals_and_empty_answers_are_replayed_as_recorded() {
        let mut t = Replay::new(2).reject(0x04, 0x01c4, -3).empty(0x04, 0x1344);
        t.xu_set(2, &query_frame(0x04, 0x01c4, 1)).unwrap();
        assert_eq!(
            parse_answer(&read(&mut t, 2), 1, 0x01c4),
            Some(Answer::Rejected(-3))
        );
        t.xu_set(2, &query_frame(0x04, 0x1344, 2)).unwrap();
        assert_eq!(
            parse_answer(&read(&mut t, 2), 2, 0x1344),
            Some(Answer::Empty)
        );
    }

    #[test]
    fn a_command_is_logged_and_leaves_the_answer_buffer_alone() {
        let mut t = Replay::new(2).answer(0x04, 0x0104, &[5]);
        let log = t.log();
        t.xu_set(2, &query_frame(0x04, 0x0104, 1)).unwrap();
        let command = frame(0x04, 0x3b84, &[1], 2);
        t.xu_set(2, &command).unwrap();
        assert_eq!(
            parse_answer(&read(&mut t, 2), 1, 0x0104),
            Some(Answer::Value(&[5])),
            "still the answer to the query"
        );
        assert_eq!(
            log.sent()[1],
            Sent::Xu {
                selector: 2,
                data: command.to_vec()
            }
        );
        assert_eq!(log.sent().len(), 2, "the query is a write too");
    }

    #[test]
    fn buffers_and_successive_reads() {
        let entry = |slot: u8| {
            let mut b = [0u8; FRAME_SIZE];
            b[1] = slot;
            b
        };
        let mut t = Replay::new(2)
            .buffer(6, [7; FRAME_SIZE])
            .reads(0x0d, &[entry(0), entry(2)]);
        assert_eq!(read(&mut t, 6), [7; FRAME_SIZE]);
        assert_eq!(
            read(&mut t, 6),
            [7; FRAME_SIZE],
            "a buffer reads the same each time"
        );
        assert_eq!(read(&mut t, 0x0d)[1], 0);
        assert_eq!(read(&mut t, 0x0d)[1], 2);
        let mut buf = [0u8; FRAME_SIZE];
        assert!(
            matches!(t.xu_get(0x0d, &mut buf), Err(CameraError::Io(_))),
            "no third entry was recorded"
        );
    }

    #[test]
    fn controls_keep_the_last_value_written() {
        let mut t = Replay::new(2).ctrl(0x0098_0900, 50);
        let log = t.log();
        assert_eq!(t.ctrl_get(0x0098_0900).unwrap(), 50);
        t.ctrl_set(0x0098_0900, 20).unwrap();
        assert_eq!(t.ctrl_get(0x0098_0900).unwrap(), 20);
        assert_eq!(
            log.sent(),
            [Sent::Ctrl {
                id: 0x0098_0900,
                value: 20
            }]
        );
        assert!(matches!(t.ctrl_get(0x0098_0901), Err(CameraError::Io(_))));
        assert_eq!(t.ctrl_range(0x0098_0900).unwrap(), None);
    }

    #[test]
    fn a_gone_device_fails_every_call_with_gone() {
        let mut t = Replay::new(2).gone();
        let mut buf = [0u8; FRAME_SIZE];
        assert_eq!(t.xu_get(6, &mut buf), Err(CameraError::Gone));
        assert_eq!(t.xu_set(6, &[1]), Err(CameraError::Gone));
        assert_eq!(t.ctrl_set(1, 1), Err(CameraError::Gone));
        assert!(t.log().sent().is_empty(), "nothing reached the log");
    }

    #[test]
    fn answer_frames_have_the_recorded_layout() {
        // capture 04: the answer to 0x6804 (zoom 2.77) with sequence 0x2f, byte for byte
        let recorded = "aa292f000c00966b040a046804003a9aae473140";
        let built = answer_frame(
            0x29,
            0x2f,
            0x04,
            0x6804,
            None,
            &hex::decode("ae473140").unwrap(),
        );
        assert_eq!(hex::encode(&built[..20]), recorded);
        assert!(built[20..].iter().all(|b| *b == 0));
        // capture 11: 0x4e44 (zone follow off) with sequence 0x0175
        let built = answer_frame(0x29, 0x0175, 0x04, 0x4e44, None, &[0]);
        assert_eq!(
            hex::encode(&built[..17]),
            "aa2975010c000880040a444e0100e63f00"
        );
        // capture 11: the empty answer to 0x1344, sequence 0x0176 (its header only)
        let built = answer_frame(0x09, 0x0176, 0x04, 0x1344, None, &[]);
        assert_eq!(hex::encode(&built[..12]), "aa0976010c0092b6040a4413");
        // capture 01: query 0xc442 to receiver 2 refused with code -3, sequence 4
        let built = answer_frame(0x89, 4, 0x02, 0xc442, Some(-3), &[]);
        assert_eq!(
            hex::encode(&built[..16]),
            "aa890400100046b7020a42c40002fdff"
        );
    }
}
