// SPDX-License-Identifier: GPL-3.0-or-later

//! The device table (5.1): every entry's id, description, rev and last state, and the
//! connections that receive events, under one lock. Every change of an entry goes through
//! here, so the events of 4.1 leave in one order for every connection, and a `hello` snapshot
//! and the start of its event delivery are atomic (4.1 `hello` timing).
//!
//! An enumeration of a supported device is a plug (`PlugId`); its worker addresses the table
//! by it. A plug's entry stays hidden until the worker announces it after its first read
//! attempt (5.2), or until a `hello` reaches its deadline (4.1).

use crate::identity::{PortReason, choose_id};
use crate::names;
use crate::outbound::{ConnId, Outbound, ok_line};
use crate::outbound::{error, error_line};
use crate::queue::{DeviceQueue, Job, Work};
use obsbot_device::contract::ModelEntry;
use obsbot_proto::ErrorCode;
use obsbot_proto::validate::validate_get;
use obsbot_proto::values::same;
use obsbot_proto::{
    Description, Device, Event, Hello, HelloOk, OkBody, RequestId, RevState, State,
};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::time::Instant;

/// One enumeration of a device, numbered by the registry from 1.
pub type PlugId = u64;

/// Who changed a state (4.1 rule 8): a request (its batch's re-read), or a periodic read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cause {
    Request,
    Poll,
}

/// Where a reader sends a device request (4.1 rule 2).
pub enum Routed {
    /// No listed device has this id.
    Unknown,
    /// An unsupported device: `get` from its fixed state, `set` and `do` refused.
    Unsupported(RevState, Description),
    /// An unplugged `sn:` device: `get` from its published state, `set` and `do` refused.
    Unavailable(RevState, Description),
    /// A supported, available device: its queue.
    Worker(Arc<DeviceQueue>),
}

impl Routed {
    /// Answers a request the reader (or a worker that ended) answers at once (4.1 rule 2):
    /// `unknown_device`; for an unsupported or unplugged device a `get` from its state (after
    /// `validate_get`), a `set` or `do` with `unsupported_device` or `unavailable`.
    pub fn answer(&self, job: &Job) {
        todo!("b-06-table")
    }
}

enum Route {
    Unsupported,
    Worker(Arc<DeviceQueue>),
    Unplugged,
}

struct Entry {
    device: Device,
    rev: u64,
    state: State,
    announced: bool,
    plug: Option<PlugId>,
    model: Option<&'static ModelEntry>,
    route: Route,
}

struct Sub {
    out: Outbound,
    subscribe: bool,
    live: BTreeSet<String>,
}

struct Inner {
    entries: Vec<Entry>,
    subs: BTreeMap<ConnId, Sub>,
    startup: BTreeSet<PlugId>,
}

pub struct Table {
    inner: Mutex<Inner>,
    startup_done: Condvar,
    daemon: String,
}

/// The live topics of a state change that touches only volatile leaves (4.1 rule 8);
/// `None` when it touches anything else (a leaf without a live topic, `available`, `pending`
/// or `unread`).
pub fn volatile_topics(desc: &Description, old: &State, new: &State) -> Option<BTreeSet<String>> {
    todo!("b-06-table")
}

fn same_state(a: &State, b: &State) -> bool {
    todo!("b-06-table")
}

fn line(event: &Event) -> String {
    todo!("b-06-table")
}

impl Inner {
    fn listed(&self) -> Vec<Device> {
        todo!("b-06-table")
    }

    fn by_plug(&self, plug: PlugId) -> Option<usize> {
        todo!("b-06-table")
    }

    fn by_id(&self, id: &str) -> Option<usize> {
        todo!("b-06-table")
    }

    /// Sends one line to the subscribed connections; `only` limits it to connections whose
    /// live list holds one of these topics.
    fn send(&self, text: &str, only: Option<&BTreeSet<String>>) {
        todo!("b-06-table")
    }

    fn send_devices(&self) {
        todo!("b-06-table")
    }

    fn send_state(&self, i: usize, only: Option<&BTreeSet<String>>) {
        todo!("b-06-table")
    }

    /// Lists a hidden entry: `devices`, then its first `state` (4.1 Events).
    fn list(&mut self, i: usize) {
        todo!("b-06-table")
    }

    fn held(&self, id: &str, except: usize) -> bool {
        todo!("b-06-table")
    }

    /// The hand-over of 5.5 from the listed `port:` entry `i` to `sn_id`: three events in a
    /// row, the rev above both entries, `pending` and `unread` carried over.
    fn hand_over(&mut self, i: usize, sn_id: &str, serial: &str) {
        todo!("b-06-table")
    }
}

impl Table {
    /// An empty table; `daemon` is the version the `hello` reply names.
    pub fn new(daemon: &str) -> Table {
        todo!("b-06-table")
    }

    fn lock(&self) -> MutexGuard<'_, Inner> {
        todo!("b-06-table")
    }

    /// The plugs of the first enumeration whose first read attempt every `hello` waits for.
    pub fn startup(&self, plugs: &[PlugId]) {
        todo!("b-06-table")
    }

    /// Answers a `hello` that arrived at `arrived` (4.1 `hello` timing): waits until the first
    /// read attempt of every device of the first enumeration has ended, at most until
    /// `arrived + deadline`; a device still in its first read is then listed with its leaves
    /// `null` and in `unread`. The reply and the connection's registration for events happen
    /// under one lock. Returns `false` when the reply could not be queued.
    pub fn hello(
        &self,
        out: &Outbound,
        id: &RequestId,
        hello: &Hello,
        arrived: Instant,
        deadline: std::time::Duration,
    ) -> bool {
        todo!("b-06-table")
    }

    /// The `live` verb: replaces the connection's live list.
    pub fn set_live(&self, conn: ConnId, topics: &[String]) {
        todo!("b-06-table")
    }

    /// The connection closed.
    pub fn remove_conn(&self, conn: ConnId) {
        todo!("b-06-table")
    }

    /// The live topics at least one subscribed connection holds (5.7 live tiers).
    pub fn topics(&self) -> BTreeSet<String> {
        todo!("b-06-table")
    }

    /// Where a request for `id` goes (4.1 rule 2).
    pub fn route(&self, id: &str) -> Routed {
        todo!("b-06-table")
    }

    /// An unsupported device appeared (5.4): listed at once with its fixed state, rev 1.
    pub fn plug_unsupported(&self, plug: PlugId, device: Device) {
        todo!("b-06-table")
    }

    /// A supported device appeared: a hidden entry under its `port:` id until its worker
    /// announces it.
    pub fn plug_hidden(
        &self,
        plug: PlugId,
        model: &'static ModelEntry,
        device: Device,
        queue: Arc<DeviceQueue>,
    ) {
        todo!("b-06-table")
    }

    /// The worker opened the camera: the hidden entry takes the model's description, and its
    /// state every described leaf, `null` and in `unread` (what a `hello` at its deadline
    /// shows, 4.1).
    pub fn describe(&self, plug: PlugId, description: Description) {
        todo!("b-06-table")
    }

    /// The worker's first read attempt ended (5.2): the entry is listed under `sn:<serial>`
    /// (taking an unplugged entry with that id, 5.3) or under its `port:` id. An entry a
    /// `hello` already listed at its deadline is handed over instead. Returns the id.
    pub fn announce(
        &self,
        plug: PlugId,
        serial: Option<&str>,
        reason: PortReason,
        firmware: Option<&str>,
        state: State,
    ) -> Option<String> {
        todo!("b-06-table")
    }

    /// A later identity attempt answered (5.5): a valid serial that no available device holds
    /// hands a `port:` entry over to `sn:`; a firmware answered for the first time changes the
    /// entry (a `devices` event). Returns the new id after a hand-over.
    pub fn identity(
        &self,
        plug: PlugId,
        serial: Option<&str>,
        firmware: Option<&str>,
    ) -> Option<String> {
        todo!("b-06-table")
    }

    /// Publishes a worker's view of its device: values, `pending` and `unread`. A change gets
    /// a new rev and a `state` event; a change caused by no request that touches only volatile
    /// leaves goes only to the connections that subscribe one of their live topics (4.1 rule
    /// 8). An unchanged state keeps its rev and sends nothing. Returns the rev that reflects
    /// the state. A plug that no longer serves its entry (unplugged) changes nothing.
    pub fn publish(&self, plug: PlugId, state: &State, cause: Cause) -> u64 {
        todo!("b-06-table")
    }

    /// The rev and the published state of a plug's entry.
    pub fn current(&self, plug: PlugId) -> Option<(u64, State)> {
        todo!("b-06-table")
    }

    /// The id a plug's entry is listed under now.
    pub fn id_of(&self, plug: PlugId) -> Option<String> {
        todo!("b-06-table")
    }

    /// The enumeration went away (5.3): an unsupported or `port:` entry is removed (one
    /// `devices` event); an `sn:` entry stays listed with `available: false`, no video node,
    /// `pending` emptied and its `readback: false` leaves `null` (4.2), then one `state`
    /// event. A hidden entry goes without an event.
    pub fn unplug(&self, plug: PlugId) {
        todo!("b-06-table")
    }

    /// The queues of every available supported device: for the deadline sweeper, and for a
    /// closing connection (its queued requests are dropped, 4.1 rule 9; its continuing action
    /// stopped, 5.6).
    pub fn queues(&self) -> Vec<Arc<DeviceQueue>> {
        todo!("b-06-table")
    }

    /// The listed devices, for tests and logs.
    pub fn devices(&self) -> Vec<Device> {
        todo!("b-06-table")
    }
}
