// SPDX-License-Identifier: GPL-3.0-or-later

//! The Tiny 2 Lite behind `trait Camera` (design spec 8.2, 8.8). The model owns its link to
//! the camera; reading is in `read`, writing in `write`, actions in `act`, the plan data in
//! `plan` and the send table in `send_table`.

use crate::contract::camera::Result;
use crate::contract::{
    Camera, CameraError, Ctx, Group, Identity, Motion, Need, SendTable, Target, Transport,
};
use crate::family::frame::Sequence;
use crate::family::link::{Link, Timing};
use crate::models::tiny2lite::description::description;
use crate::models::tiny2lite::plan::{self, ENTRY, FRAMED, IDENTITY, SHORT};
use crate::models::tiny2lite::{act, decode, read, send_table, write};
use crate::transport::checked::{Checked, Selectors};
use obsbot_proto::validate::Refusal;
use obsbot_proto::{Description, Params, State, Values};
use serde_json::Value;

/// Where the Tiny 2 Lite's frames go (its registry entry, 8.8).
pub const SELECTORS: Selectors = Selectors {
    framed: FRAMED,
    short: SHORT,
    all: ENTRY.selectors,
};

/// The identity queries (receiver `0x0d`, 8.8): serial and firmware.
pub const SERIAL_QUERY: u16 = 0x18c8;
pub const FIRMWARE_QUERY: u16 = 0x0408;

/// What the model keeps from its own last reads and writes: the tracking mode (the target is
/// read only while it is not `static`, 8.8), the values of the last target read (the group's
/// values while it is not read, trait `Camera::read`) and the auto-sleep word
/// (`sleep.auto.enabled` and `sleep.auto.seconds` are one word on the camera, 4.3).
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Memory {
    pub mode: Option<Value>,
    pub target: Option<Values>,
    pub sleep_word: Option<i16>,
}

/// One Tiny 2 Lite.
pub struct Tiny2Lite {
    link: Link,
    description: Description,
    plan: Vec<Group>,
    table: SendTable,
    memory: Memory,
}

impl Tiny2Lite {
    /// The model on a transport that is already checked against its send table; private, so
    /// that [`Tiny2Lite::checked`] is the only way to build one.
    fn new(transport: Box<dyn Transport>, seq: &'static Sequence, timing: Timing) -> Tiny2Lite {
        Tiny2Lite {
            link: Link::new(transport, seq, timing),
            description: description(),
            plan: plan::groups(),
            table: send_table::table(),
            memory: Memory::default(),
        }
    }

    /// The model on `transport`, wrapped in the send-table check (8.3): what a bus opens, and
    /// what every test uses, so that a frame outside the table fails the test.
    pub fn checked<T: Transport + 'static>(
        transport: T,
        seq: &'static Sequence,
        timing: Timing,
    ) -> Tiny2Lite {
        Tiny2Lite::new(
            Box::new(Checked::new(transport, send_table::table(), SELECTORS)),
            seq,
            timing,
        )
    }

    /// What the model keeps from its reads and writes (tests).
    pub fn memory(&self) -> &Memory {
        &self.memory
    }
}

/// One identity query: its answer, `None` when the camera did not answer or refused it (the
/// attempt is spent), an error for anything else (gone, cut, locked).
fn ask(link: &mut Link, command: u16, ctx: &Ctx) -> Result<Option<Vec<u8>>> {
    match link.query(FRAMED, IDENTITY, command, ctx) {
        Ok(d) => Ok(Some(d)),
        Err(CameraError::NoAnswer(_) | CameraError::Rejected(_)) => Ok(None),
        Err(e) => Err(e),
    }
}

impl Camera for Tiny2Lite {
    fn description(&self) -> &Description {
        &self.description
    }

    fn identity(&mut self, need: Need, ctx: &Ctx) -> Result<Identity> {
        let mut id = Identity::default();
        if need.serial {
            id.serial = ask(&mut self.link, SERIAL_QUERY, ctx)?.and_then(|d| decode::serial(&d));
        }
        if need.firmware {
            id.firmware =
                ask(&mut self.link, FIRMWARE_QUERY, ctx)?.and_then(|d| decode::firmware(&d));
        }
        Ok(id)
    }

    fn poll_plan(&self) -> &[Group] {
        &self.plan
    }

    fn read(&mut self, group: &str, ctx: &Ctx) -> Result<Values> {
        read::read(&mut self.link, &mut self.memory, group, ctx)
    }

    fn order(&self, leaves: &[&str]) -> Vec<String> {
        plan::order(leaves)
    }

    fn apply(&mut self, leaf: &str, value: &Value, ctx: &Ctx) -> Result<()> {
        write::apply(&mut self.link, &mut self.memory, leaf, value, ctx)
    }

    fn check(
        &self,
        action: &str,
        params: &Params,
        state: &State,
    ) -> std::result::Result<(), Refusal> {
        act::check(action, params, state)
    }

    fn act(
        &mut self,
        action: &str,
        params: &Params,
        state: &State,
        ctx: &Ctx,
    ) -> Result<Option<Params>> {
        act::act(&mut self.link, action, params, state, ctx)
    }

    fn touched(&self, target: Target<'_>) -> Vec<String> {
        plan::touched(&self.plan, target)
    }

    fn motion(&self, action: &str) -> Motion {
        plan::motion(action)
    }

    fn send_table(&self) -> &SendTable {
        &self.table
    }
}
