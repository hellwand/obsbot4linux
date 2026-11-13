// SPDX-License-Identifier: GPL-3.0-or-later

//! The identity series of one plug-in (5.5) as a state machine, and the choice of the id.
//!
//! - The first attempt is sent at plug-in, asleep or awake. Attempts 2 and 3 ride the next two
//!   ticks of the tier the model names (`ModelEntry::identity_tier`) while the camera is awake.
//! - If the first attempt gave no valid serial and the first read of the group holding `asleep`
//!   reported the camera asleep, the first series ends there; one further series of three
//!   attempts runs from the camera's first wake: at the wake, then with the next two ticks.
//! - A series ends at the first attempt after which serial and firmware are both answered; an
//!   attempt asks only for what is missing. A model with neither query sends no attempt.
//!
//! No more than one attempt is ever sent to a sleeping camera.

use obsbot_device::contract::{Identity, IdentitySource, Need};
use obsbot_proto::{is_valid_serial, port_id, sn_id};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    /// Before the plug-in attempt.
    Plug,
    /// Attempt 1 was sent; waiting for the first status read.
    First,
    /// `left` attempts ride the next ticks of the identity tier.
    Ticks {
        left: u8,
    },
    /// The first series ended asleep; the wake series starts at the first wake.
    AwaitWake,
    Done,
}

/// Why a supported device got a `port:` id (5.11 log line).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortReason {
    NoSerial,
    InvalidSerial,
    Duplicate,
}

impl PortReason {
    pub fn text(self) -> &'static str {
        match self {
            PortReason::NoSerial => "no serial answered",
            PortReason::InvalidSerial => "an invalid serial",
            PortReason::Duplicate => "a duplicate serial",
        }
    }
}

/// The identity series of one plug-in, and what it has learnt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Series {
    source: IdentitySource,
    firmware_query: bool,
    phase: Phase,
    woke: bool,
    /// A valid serial, once answered. Never logged (4.2).
    pub serial: Option<String>,
    /// The camera answered a serial that is not valid (and no valid one yet).
    pub invalid: bool,
    pub firmware: Option<String>,
}

impl Series {
    pub fn new(source: IdentitySource, firmware_query: bool) -> Series {
        Series {
            source,
            firmware_query,
            phase: Phase::Plug,
            woke: false,
            serial: None,
            invalid: false,
            firmware: None,
        }
    }

    fn need(&self, plug_in: bool) -> Need {
        Need {
            serial: match self.source {
                IdentitySource::Query => self.serial.is_none(),
                IdentitySource::UsbSerial => plug_in,
                IdentitySource::Absent => false,
            },
            firmware: self.firmware_query && self.firmware.is_none(),
        }
    }

    fn complete(&self) -> bool {
        let n = self.need(false);
        !n.serial && !n.firmware
    }

    /// Attempt 1, at plug-in; `None` when the model has nothing to ask.
    pub fn at_plug_in(&mut self) -> Option<Need> {
        if self.phase != Phase::Plug {
            return None;
        }
        let need = self.need(true);
        if !need.serial && !need.firmware {
            self.phase = Phase::Done;
            return None;
        }
        self.phase = Phase::First;
        Some(need)
    }

    /// The outcome of an attempt: `None` when it failed. Answers are kept only once given.
    pub fn answered(&mut self, got: Option<&Identity>) {
        if let Some(id) = got {
            if self.serial.is_none()
                && let Some(s) = &id.serial
            {
                if is_valid_serial(s) {
                    self.serial = Some(s.clone());
                    self.invalid = false;
                } else {
                    self.invalid = true;
                }
            }
            if self.firmware.is_none() {
                self.firmware.clone_from(&id.firmware);
            }
        }
        if self.complete() || self.phase == (Phase::Ticks { left: 0 }) {
            self.phase = Phase::Done;
        }
    }

    /// The first read of the group holding `asleep` (5.2 step 3) reported `asleep`; `false`
    /// as well for a model without an `asleep` leaf.
    pub fn after_first_status(&mut self, asleep: bool) {
        if self.phase != Phase::First {
            return;
        }
        self.phase = if self.serial.is_none() && asleep {
            Phase::AwaitWake
        } else {
            Phase::Ticks { left: 2 }
        };
    }

    /// A tick of the identity tier: the attempt that rides it, if any (only while awake).
    pub fn at_tick(&mut self, awake: bool) -> Option<Need> {
        let Phase::Ticks { left } = self.phase else {
            return None;
        };
        if !awake || left == 0 {
            return None;
        }
        self.phase = Phase::Ticks { left: left - 1 };
        Some(self.need(false))
    }

    /// `asleep` turned false: the first attempt of the wake series, if one is due.
    pub fn at_wake(&mut self) -> Option<Need> {
        if self.phase != Phase::AwaitWake || self.woke {
            return None;
        }
        self.woke = true;
        self.phase = Phase::Ticks { left: 2 };
        Some(self.need(false))
    }

    /// No further attempt will be sent for this plug-in.
    pub fn done(&self) -> bool {
        self.phase == Phase::Done
    }

    /// Why the id is a `port:` id when no valid serial is known.
    pub fn reason(&self) -> PortReason {
        if self.invalid {
            PortReason::InvalidSerial
        } else {
            PortReason::NoSerial
        }
    }
}

/// The id of a supported device (5.5): `sn:<serial>` for a valid serial that no available
/// device holds, else `port:<usb path>` with the reason.
pub fn choose_id(
    serial: Option<&str>,
    reason: PortReason,
    usb_path: &str,
    held_by_an_available_device: impl Fn(&str) -> bool,
) -> (String, Option<PortReason>) {
    match serial {
        Some(s) => {
            let id = sn_id(s);
            if held_by_an_available_device(&id) {
                (port_id(usb_path), Some(PortReason::Duplicate))
            } else {
                (id, None)
            }
        }
        None => (port_id(usb_path), Some(reason)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn both() -> Need {
        Need {
            serial: true,
            firmware: true,
        }
    }

    fn answer(serial: Option<&str>, firmware: Option<&str>) -> Identity {
        Identity {
            serial: serial.map(String::from),
            firmware: firmware.map(String::from),
        }
    }

    /// 10.3 B: "an identity series ending at the first complete answer".
    #[test]
    fn a_complete_first_answer_ends_the_series() {
        let mut s = Series::new(IdentitySource::Query, true);
        assert_eq!(s.at_plug_in(), Some(both()));
        s.answered(Some(&answer(Some("FAKEA0000001"), Some("1.0"))));
        assert!(s.done());
        s.after_first_status(false);
        assert_eq!(s.at_tick(true), None, "nothing more is sent");
        assert_eq!(s.serial.as_deref(), Some("FAKEA0000001"));
    }

    /// Attempts 2 and 3 ride the next two ticks of the identity tier and ask only what is missing.
    #[test]
    fn missing_answers_are_asked_again_twice_and_only_what_is_missing() {
        let mut s = Series::new(IdentitySource::Query, true);
        s.at_plug_in();
        s.answered(Some(&answer(None, Some("1.0"))));
        s.after_first_status(false);
        assert_eq!(
            s.at_tick(true),
            Some(Need {
                serial: true,
                firmware: false
            })
        );
        s.answered(None);
        assert!(!s.done());
        assert_eq!(
            s.at_tick(false),
            None,
            "the tier is not read asleep: the attempt waits"
        );
        assert!(s.at_tick(true).is_some(), "attempt 3");
        s.answered(None);
        assert!(s.done(), "three attempts at most");
        assert_eq!(s.at_tick(true), None);
        assert_eq!(
            s.at_wake(),
            None,
            "the wake series is only for a camera asleep at plug-in"
        );
    }

    #[test]
    fn a_later_attempt_that_completes_ends_the_series() {
        let mut s = Series::new(IdentitySource::Query, true);
        s.at_plug_in();
        s.answered(None);
        s.after_first_status(false);
        assert_eq!(s.at_tick(true), Some(both()));
        s.answered(Some(&answer(Some("FAKEA0000001"), Some("1.0"))));
        assert!(s.done());
        assert_eq!(s.at_tick(true), None);
    }

    /// 5.5 wake series: asleep at plug-in without a serial, one attempt only, then three from
    /// the first wake: so no more than one attempt reaches a sleeping camera.
    #[test]
    fn asleep_at_plug_in_the_series_waits_for_the_first_wake() {
        let mut s = Series::new(IdentitySource::Query, true);
        assert!(s.at_plug_in().is_some());
        s.answered(None);
        s.after_first_status(true);
        assert_eq!(s.at_tick(true), None, "the first series has ended");
        assert_eq!(s.at_wake(), Some(both()), "attempt 1 of the wake series");
        s.answered(None);
        assert!(s.at_tick(true).is_some());
        s.answered(None);
        assert!(s.at_tick(true).is_some());
        s.answered(None);
        assert!(s.done());
        assert_eq!(s.at_wake(), None, "one wake series only");
    }

    #[test]
    fn asleep_with_a_serial_answered_needs_no_wake_series() {
        let mut s = Series::new(IdentitySource::Query, true);
        s.at_plug_in();
        s.answered(Some(&answer(Some("FAKEA0000001"), None)));
        s.after_first_status(true);
        assert_eq!(s.at_wake(), None);
        assert_eq!(
            s.at_tick(true),
            Some(Need {
                serial: false,
                firmware: true
            }),
            "the firmware is still asked, with the ticks"
        );
    }

    /// 5.5: the USB serial is read once at plug-in; later attempts are the firmware query alone;
    /// a model with neither query sends nothing.
    #[test]
    fn usb_serial_models_and_models_without_queries() {
        let mut s = Series::new(IdentitySource::UsbSerial, true);
        assert_eq!(s.at_plug_in(), Some(both()));
        s.answered(Some(&answer(None, None)));
        s.after_first_status(false);
        assert_eq!(
            s.at_tick(true),
            Some(Need {
                serial: false,
                firmware: true
            })
        );
        let mut none = Series::new(IdentitySource::Absent, false);
        assert_eq!(none.at_plug_in(), None);
        assert!(none.done());
        let mut fw = Series::new(IdentitySource::Absent, true);
        assert_eq!(
            fw.at_plug_in(),
            Some(Need {
                serial: false,
                firmware: true
            })
        );
    }

    #[test]
    fn an_invalid_serial_is_not_kept_and_is_the_reason_for_the_port_id() {
        let mut s = Series::new(IdentitySource::Query, false);
        s.at_plug_in();
        s.answered(Some(&answer(Some("BAD-1"), None)));
        assert_eq!(s.serial, None);
        assert_eq!(s.reason(), PortReason::InvalidSerial);
        assert!(!s.done(), "asked again");
    }

    /// 5.5 duplicates: a serial an available device already holds gives a `port:` id.
    #[test]
    fn the_id_is_the_serial_unless_an_available_device_holds_it() {
        assert_eq!(
            choose_id(Some("FAKEA0000001"), PortReason::NoSerial, "9-1", |_| false),
            ("sn:FAKEA0000001".to_string(), None)
        );
        assert_eq!(
            choose_id(Some("FAKEA0000001"), PortReason::NoSerial, "9-4", |id| id
                == "sn:FAKEA0000001"),
            ("port:9-4".to_string(), Some(PortReason::Duplicate))
        );
        assert_eq!(
            choose_id(None, PortReason::InvalidSerial, "9-1", |_| false),
            ("port:9-1".to_string(), Some(PortReason::InvalidSerial))
        );
    }
}
