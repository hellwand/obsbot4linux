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
        todo!("b-04-identity")
    }

    fn need(&self, plug_in: bool) -> Need {
        todo!("b-04-identity")
    }

    fn complete(&self) -> bool {
        todo!("b-04-identity")
    }

    /// Attempt 1, at plug-in; `None` when the model has nothing to ask.
    pub fn at_plug_in(&mut self) -> Option<Need> {
        todo!("b-04-identity")
    }

    /// The outcome of an attempt: `None` when it failed. Answers are kept only once given.
    pub fn answered(&mut self, got: Option<&Identity>) {
        todo!("b-04-identity")
    }

    /// The first read of the group holding `asleep` (5.2 step 3) reported `asleep`; `false`
    /// as well for a model without an `asleep` leaf.
    pub fn after_first_status(&mut self, asleep: bool) {
        todo!("b-04-identity")
    }

    /// A tick of the identity tier: the attempt that rides it, if any (only while awake).
    pub fn at_tick(&mut self, awake: bool) -> Option<Need> {
        todo!("b-04-identity")
    }

    /// `asleep` turned false: the first attempt of the wake series, if one is due.
    pub fn at_wake(&mut self) -> Option<Need> {
        todo!("b-04-identity")
    }

    /// No further attempt will be sent for this plug-in.
    pub fn done(&self) -> bool {
        todo!("b-04-identity")
    }

    /// Why the id is a `port:` id when no valid serial is known.
    pub fn reason(&self) -> PortReason {
        todo!("b-04-identity")
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
