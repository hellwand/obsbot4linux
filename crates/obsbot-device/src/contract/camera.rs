// SPDX-License-Identifier: GPL-3.0-or-later

//! `trait Camera` (design spec 8.2) and its supporting types: groups and tiers, touched
//! targets, identity, the per-operation context and the send table. The daemon knows only
//! these: no command id, no offset, no selector of any model (section 5).

use obsbot_proto::validate::Refusal;
use obsbot_proto::{Description, Params, State, Values};
use serde_json::Value;
use std::fmt;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

/// Why a camera operation failed. The daemon maps every variant to the error code `camera`,
/// except `NotInSendTable`, which is `internal` (8.3).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum CameraError {
    /// A query was not answered within `ANSWER_POLL` (5.8).
    NoAnswer(String),
    /// The camera answered with a refusal.
    Rejected(String),
    /// The device node is gone (unplug): reported at once, without waiting for a timeout.
    Gone,
    /// The per-operation lock on the video node was not free within `NODE_LOCK_WAIT`.
    Locked,
    /// The context's deadline passed or a stop interrupted a group read (`Ctx::check`).
    Cut,
    /// The transport wrapper refused a frame outside the model's send table (8.3).
    NotInSendTable(String),
    /// Any other operating-system error.
    Io(String),
}

impl fmt::Display for CameraError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CameraError::NoAnswer(what) => write!(f, "the camera did not answer {what}"),
            CameraError::Rejected(what) => write!(f, "the camera refused {what}"),
            CameraError::Gone => f.write_str("the camera is gone"),
            CameraError::Locked => f.write_str("the video node stayed locked by another process"),
            CameraError::Cut => f.write_str("cut short by the request deadline or a stop"),
            CameraError::NotInSendTable(what) => {
                write!(f, "refused a frame outside the send table: {what}")
            }
            CameraError::Io(what) => write!(f, "I/O error: {what}"),
        }
    }
}

impl std::error::Error for CameraError {}

pub type Result<T> = std::result::Result<T, CameraError>;

/// How often a group is read (5.7). Intervals count from the end of the previous read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Tier {
    Second,
    FiveSeconds,
    Minute,
    /// Read at plug-in and after a request that touches it, never periodically.
    Touched,
}

impl Tier {
    /// The period of a periodic tier; `None` for `Touched`.
    pub fn period(self) -> Option<Duration> {
        match self {
            Tier::Second => Some(Duration::from_secs(1)),
            Tier::FiveSeconds => Some(Duration::from_secs(5)),
            Tier::Minute => Some(Duration::from_secs(60)),
            Tier::Touched => None,
        }
    }
}

/// A faster tier a group uses while at least one connection subscribes the topic (5.7).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiveTier {
    pub topic: String,
    pub tier: Tier,
}

/// One group of the poll plan: the unit of reading (5.7).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Group {
    pub name: String,
    pub tier: Tier,
    /// Whether the group can be read while the camera is asleep.
    pub asleep: bool,
    pub live: Option<LiveTier>,
    /// The leaf paths one read of this group returns. Every described leaf with
    /// `readback: true` is in exactly one group; a `readback: false` leaf is in none.
    pub leaves: Vec<String>,
}

/// What an identity attempt asks for (5.5): only what is still missing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Need {
    pub serial: bool,
    pub firmware: bool,
}

/// What an identity attempt got. `serial` is the raw answer: the daemon checks it with
/// `obsbot_proto::is_valid_serial`. Never logged (4.2).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Identity {
    pub serial: Option<String>,
    pub firmware: Option<String>,
}

/// What a re-read follows: a written leaf or a finished action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target<'a> {
    Leaf(&'a str),
    Action(&'a str),
}

/// The bounds of one camera operation. A model calls [`Ctx::check`] before every frame it
/// sends: a request stops at its deadline (4.1 rule 9), a group read stops when a stop action
/// arrives (5.6 "Stops"). Both end as `CameraError::Cut`.
#[derive(Debug, Clone, Default)]
pub struct Ctx {
    deadline: Option<Instant>,
    interrupt: Option<Arc<AtomicBool>>,
}

impl Ctx {
    /// Any combination: a request's touched re-read has the request's deadline and is also a
    /// group read that a stop cuts.
    pub fn new(deadline: Option<Instant>, interrupt: Option<Arc<AtomicBool>>) -> Ctx {
        Ctx {
            deadline,
            interrupt,
        }
    }

    /// No deadline, no interrupt (tests, identity attempts).
    pub fn none() -> Ctx {
        Ctx::default()
    }

    /// A request: stops once `deadline` has passed.
    pub fn until(deadline: Instant) -> Ctx {
        Ctx {
            deadline: Some(deadline),
            interrupt: None,
        }
    }

    /// A poll: stops once `flag` is set.
    pub fn interruptible(flag: Arc<AtomicBool>) -> Ctx {
        Ctx {
            deadline: None,
            interrupt: Some(flag),
        }
    }

    /// `Err(Cut)` when the deadline has passed or the interrupt flag is set.
    pub fn check(&self) -> Result<()> {
        let late = self.deadline.is_some_and(|d| Instant::now() >= d);
        let stopped = self
            .interrupt
            .as_ref()
            .is_some_and(|f| f.load(Ordering::SeqCst));
        if late || stopped {
            Err(CameraError::Cut)
        } else {
            Ok(())
        }
    }
}

/// What an action is to the worker's queue and its continuing-motion record (4.1 rules 2
/// and 6, 5.6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Motion {
    /// Not a motion action.
    None,
    /// A motion action that ends by itself (`preset.recall`, `gimbal.move`). It clears the
    /// continuing-motion record of its device.
    Move,
    /// A motion action that goes on until `stop` (`gimbal.turn` → `gimbal.stop`). It replaces
    /// the record of its device with the connection that sent it.
    Continuing { stop: &'static str },
    /// A stop: taken before every queued request (4.1 rule 2). The stop a record names ends
    /// that motion, so it clears the record; another stop leaves it.
    Stop,
}

/// One frame a model may send (8.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Frame {
    /// A framed command or query on an extension-unit selector.
    Framed {
        selector: u8,
        receiver: u8,
        command: u16,
    },
    /// A short command on an extension-unit selector.
    Short { selector: u8, command: u8 },
    /// A write of a standard UVC control (V4L2 control id).
    Uvc { control: u32 },
    /// A framed query (frame type `0x01`, or `0x21` with a request such as a view id) on an
    /// extension-unit selector. A query is a read, but it is a frame the model writes, so the
    /// send table names it like a command; `Framed` names commands (type `0x25`) only.
    Query {
        selector: u8,
        receiver: u8,
        command: u16,
    },
    /// The whole buffer of a selector written back as it was read: on the Tiny 2 Lite the
    /// preset list rewind on selector `0x0c` (spec 8.8, CAM-19).
    Block { selector: u8 },
}

/// One entry of a send table, with its evidence (`"capture 11"`, `"measured 2026-10-31"`,
/// or `"0.13.2 only, U-9"` until camera session CS-1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SendEntry {
    pub frame: Frame,
    pub evidence: &'static str,
    /// An identity query: the only frames allowed on the identity receiver (8.3).
    pub identity: bool,
}

/// Every frame a model may send. The transport wrapper refuses anything else.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SendTable {
    pub entries: Vec<SendEntry>,
}

impl SendTable {
    /// The table of a model that sends nothing (the fakes).
    pub fn empty() -> SendTable {
        SendTable::default()
    }

    pub fn allows(&self, frame: &Frame) -> bool {
        self.entries.iter().any(|e| e.frame == *frame)
    }
}

/// One camera model behind the daemon (8.2). The daemon calls it from one worker thread per
/// device; nothing else touches the device. Signatures change only by a contract-change PR.
pub trait Camera: Send {
    /// Settings (with `settle_ms`), actions and limits.
    fn description(&self) -> &Description;

    /// One identity attempt (5.5): sends only the queries `need` asks for. A model whose
    /// identity source is the USB serial string reads it from sysfs and sends no request.
    /// The daemon builds `need` from the registry entry (`ModelEntry::identity` and
    /// `firmware_query`) and calls this only when `need` asks for something.
    fn identity(&mut self, need: Need, ctx: &Ctx) -> Result<Identity>;

    /// The groups, their tiers, whether they are readable asleep, their live topics.
    fn poll_plan(&self) -> &[Group];

    /// Reads one group: every leaf of the group → its value (`null` where nullable and not
    /// set, `raw:<n>` where undecodable). An error means the group's leaves go to `unread`.
    /// A group whose read depends on a condition (on the Tiny 2 Lite the target, read only
    /// while the tracking mode is not `static`, 8.8) is decided by the model from its own
    /// last reads: while the condition does not hold, it returns its last values, or `null`
    /// for a nullable leaf, without sending a frame.
    fn read(&mut self, group: &str, ctx: &Ctx) -> Result<Values>;

    /// The safe apply order of the leaves of one patch: a permutation of `leaves` in which
    /// every `requires` path comes before the leaf that requires it.
    fn order(&self, leaves: &[&str]) -> Vec<String>;

    /// Writes one leaf; the value has passed the validator.
    fn apply(&mut self, leaf: &str, value: &Value, ctx: &Ctx) -> Result<()>;

    /// The model's own checks of an action against the published state, before any frame is
    /// sent: an empty preset slot, an unknown view id, a list in `unread` (`conflict`), an
    /// action whose parameters miss a combination the description cannot express (`invalid`).
    /// The daemon calls it only after `obsbot_proto::validate::validate_do` passed, so its
    /// codes come after every code of the validator.
    fn check(
        &self,
        action: &str,
        params: &Params,
        state: &State,
    ) -> std::result::Result<(), Refusal>;

    /// Runs one action; its parameters have passed the validator and [`Camera::check`].
    /// `state` is the published state that `check` saw (a used or empty preset slot, the
    /// mirror and invert flags). Returns the result the description declares
    /// (`view.save` → `{"id": …}`), else `None`.
    fn act(
        &mut self,
        action: &str,
        params: &Params,
        state: &State,
        ctx: &Ctx,
    ) -> Result<Option<Params>>;

    /// The groups to re-read after a leaf was written or an action ran. For a leaf with a
    /// readback this includes the leaf's own group. Beyond that, only the groups whose values
    /// the camera itself changes in response: a model sends exactly the frames of the request
    /// (design spec 2), so no group is touched by a frame of the model's own.
    fn touched(&self, target: Target<'_>) -> Vec<String>;

    /// What `action` is to the queue and to the continuing-motion record (4.1 rules 2 and 6,
    /// 5.6). `Motion::None` for every name that is not a motion action or a stop. The daemon
    /// may ask once per action of the description when it opens the device.
    fn motion(&self, action: &str) -> Motion;

    /// Every frame this model may send (8.3).
    fn send_table(&self) -> &SendTable;
}
