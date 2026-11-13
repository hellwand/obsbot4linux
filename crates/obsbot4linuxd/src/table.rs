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
        let id = &job.device;
        let (rs, desc, refusal) = match self {
            Routed::Unknown => {
                return job.fail(ErrorCode::UnknownDevice, &format!("no device {id}"));
            }
            Routed::Worker(_) => {
                return job.fail(ErrorCode::Internal, "a queued request was answered at once");
            }
            Routed::Unsupported(rs, desc) => (
                rs,
                desc,
                (
                    ErrorCode::UnsupportedDevice,
                    format!("{id} is not supported by obsbot4linux"),
                ),
            ),
            Routed::Unavailable(rs, desc) => (
                rs,
                desc,
                (ErrorCode::Unavailable, format!("{id} is unplugged")),
            ),
        };
        match &job.work {
            Work::Get(paths) => {
                if let Some(paths) = paths
                    && let Err(r) = validate_get(desc, paths)
                {
                    return job.answer(error_line(Some(&job.id), error(r.code, &r.message)));
                }
                let state = match paths {
                    Some(p) => rs.state.select(p),
                    None => rs.state.clone(),
                };
                job.answer(ok_line(
                    &job.id,
                    OkBody::State(RevState { rev: rs.rev, state }),
                ));
            }
            Work::Set(_) | Work::Do { .. } => job.fail(refusal.0, &refusal.1),
        }
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
    if old.available != new.available || old.unread != new.unread {
        return None;
    }
    let pending_same = old.pending.len() == new.pending.len()
        && old
            .pending
            .iter()
            .all(|(k, v)| new.pending.get(k).is_some_and(|w| same(v, w)));
    if !pending_same {
        return None;
    }
    let mut topics = BTreeSet::new();
    for (path, setting) in &desc.settings {
        let a = old.at(path).unwrap_or(&Value::Null);
        let b = new.at(path).unwrap_or(&Value::Null);
        if !same(a, b) {
            topics.insert(setting.live.clone()?);
        }
    }
    Some(topics)
}

fn same_state(a: &State, b: &State) -> bool {
    a.available == b.available
        && a.unread == b.unread
        && same(
            &serde_json::to_value(&a.pending).unwrap_or(Value::Null),
            &serde_json::to_value(&b.pending).unwrap_or(Value::Null),
        )
        && same(
            &Value::Object(a.values.clone()),
            &Value::Object(b.values.clone()),
        )
}

fn line(event: &Event) -> String {
    serde_json::to_string(event).expect("an event always serialises")
}

impl Inner {
    fn listed(&self) -> Vec<Device> {
        self.entries
            .iter()
            .filter(|e| e.announced)
            .map(|e| e.device.clone())
            .collect()
    }

    fn by_plug(&self, plug: PlugId) -> Option<usize> {
        self.entries.iter().position(|e| e.plug == Some(plug))
    }

    fn by_id(&self, id: &str) -> Option<usize> {
        self.entries
            .iter()
            .position(|e| e.announced && e.device.id == id)
    }

    /// Sends one line to the subscribed connections; `only` limits it to connections whose
    /// live list holds one of these topics.
    fn send(&self, text: &str, only: Option<&BTreeSet<String>>) {
        for sub in self.subs.values().filter(|s| s.subscribe) {
            if let Some(topics) = only
                && sub.live.is_disjoint(topics)
            {
                continue;
            }
            sub.out.send(text.to_string());
        }
    }

    fn send_devices(&self) {
        let devices = self.listed();
        self.send(&line(&Event::Devices { devices }), None);
    }

    fn send_state(&self, i: usize, only: Option<&BTreeSet<String>>) {
        let e = &self.entries[i];
        let event = Event::State {
            device: e.device.id.clone(),
            rev: e.rev,
            state: e.state.clone(),
        };
        self.send(&line(&event), only);
    }

    /// Lists a hidden entry: `devices`, then its first `state` (4.1 Events).
    fn list(&mut self, i: usize) {
        self.entries[i].announced = true;
        self.entries[i].rev = 1;
        self.send_devices();
        self.send_state(i, None);
    }

    fn held(&self, id: &str, except: usize) -> bool {
        self.entries
            .iter()
            .enumerate()
            .any(|(k, e)| k != except && e.announced && e.device.available && e.device.id == id)
    }

    /// The hand-over of 5.5 from the listed `port:` entry `i` to `sn_id`: three events in a
    /// row, the rev above both entries, `pending` and `unread` carried over.
    fn hand_over(&mut self, i: usize, sn_id: &str, serial: &str) {
        let old_id = self.entries[i].device.id.clone();
        let taken = self
            .entries
            .iter()
            .position(|e| e.announced && e.device.id == sn_id);
        let rev = self.entries[i]
            .rev
            .max(taken.map_or(0, |t| self.entries[t].rev))
            + 1;
        let display = self.entries[i].model.map_or("", |m| m.display_name);
        let mut device = self.entries[i].device.clone();
        device.id = sn_id.to_string();
        device.name = names::name_of(display, sn_id, &device.usb.path);
        device.serial = Some(serial.to_string());
        device.available = true;
        device.replaced_by = None;
        let mut ghost = self.entries[i].device.clone();
        ghost.available = false;
        ghost.replaced_by = Some(sn_id.to_string());
        // Event 1: the sn: entry (available) and the old port: entry with replaced_by.
        let mut first: Vec<Device> = Vec::new();
        for (k, e) in self.entries.iter().enumerate().filter(|(_, e)| e.announced) {
            if Some(k) == taken {
                first.push(device.clone());
            } else if k == i {
                if taken.is_none() {
                    first.push(device.clone());
                }
                first.push(ghost.clone());
            } else {
                first.push(e.device.clone());
            }
        }
        self.send(&line(&Event::Devices { devices: first }), None);
        let entry = self.entries.remove(i);
        let target = match taken {
            Some(t) => {
                let t = if t > i { t - 1 } else { t };
                self.entries[t].plug = entry.plug;
                self.entries[t].route = entry.route;
                self.entries[t].model = entry.model;
                t
            }
            None => {
                self.entries.insert(
                    i,
                    Entry {
                        device: device.clone(),
                        rev: 0,
                        state: State::new(true),
                        announced: true,
                        plug: entry.plug,
                        model: entry.model,
                        route: entry.route,
                    },
                );
                i
            }
        };
        let mut state = entry.state;
        state.available = true;
        self.entries[target].device = device;
        self.entries[target].state = state;
        self.entries[target].rev = rev;
        self.send_devices();
        self.send_state(target, None);
        eprintln!("{}", names::log_handover(&old_id, sn_id));
    }
}

impl Table {
    /// An empty table; `daemon` is the version the `hello` reply names.
    pub fn new(daemon: &str) -> Table {
        Table {
            inner: Mutex::new(Inner {
                entries: Vec::new(),
                subs: BTreeMap::new(),
                startup: BTreeSet::new(),
            }),
            startup_done: Condvar::new(),
            daemon: daemon.to_string(),
        }
    }

    fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// The plugs of the first enumeration whose first read attempt every `hello` waits for.
    pub fn startup(&self, plugs: &[PlugId]) {
        self.lock().startup.extend(plugs.iter().copied());
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
        let until = arrived + deadline;
        let mut inner = self.lock();
        while !inner.startup.is_empty() {
            let now = Instant::now();
            if now >= until {
                break;
            }
            inner = self
                .startup_done
                .wait_timeout(inner, until - now)
                .unwrap_or_else(PoisonError::into_inner)
                .0;
        }
        let late: Vec<PlugId> = std::mem::take(&mut inner.startup).into_iter().collect();
        for plug in late {
            if let Some(i) = inner.by_plug(plug)
                && !inner.entries[i].announced
            {
                inner.list(i);
                eprintln!(
                    "{}",
                    names::log_appeared(&inner.entries[i].device, Some(PortReason::NoSerial))
                );
            }
        }
        let devices = inner.listed();
        let states = inner
            .entries
            .iter()
            .filter(|e| e.announced)
            .map(|e| {
                (
                    e.device.id.clone(),
                    RevState {
                        rev: e.rev,
                        state: e.state.clone(),
                    },
                )
            })
            .collect();
        let reply = OkBody::Hello(HelloOk {
            protocol: obsbot_proto::PROTOCOL,
            daemon: self.daemon.clone(),
            devices,
            states,
        });
        out.set_client(&hello.client);
        let sent = out.send(ok_line(id, reply));
        inner.subs.insert(
            out.conn(),
            Sub {
                out: out.clone(),
                subscribe: hello.subscribe,
                live: hello.live.iter().cloned().collect(),
            },
        );
        sent
    }

    /// The `live` verb: replaces the connection's live list.
    pub fn set_live(&self, conn: ConnId, topics: &[String]) {
        if let Some(sub) = self.lock().subs.get_mut(&conn) {
            sub.live = topics.iter().cloned().collect();
        }
    }

    /// The connection closed.
    pub fn remove_conn(&self, conn: ConnId) {
        self.lock().subs.remove(&conn);
    }

    /// The live topics at least one subscribed connection holds (5.7 live tiers).
    pub fn topics(&self) -> BTreeSet<String> {
        self.lock()
            .subs
            .values()
            .filter(|s| s.subscribe)
            .flat_map(|s| s.live.iter().cloned())
            .collect()
    }

    /// Where a request for `id` goes (4.1 rule 2).
    pub fn route(&self, id: &str) -> Routed {
        let inner = self.lock();
        let Some(i) = inner.by_id(id) else {
            return Routed::Unknown;
        };
        let e = &inner.entries[i];
        let rs = RevState {
            rev: e.rev,
            state: e.state.clone(),
        };
        match &e.route {
            Route::Unsupported => Routed::Unsupported(rs, e.device.description.clone()),
            Route::Unplugged => Routed::Unavailable(rs, e.device.description.clone()),
            Route::Worker(q) => Routed::Worker(q.clone()),
        }
    }

    /// An unsupported device appeared (5.4): listed at once with its fixed state, rev 1.
    pub fn plug_unsupported(&self, plug: PlugId, device: Device) {
        let mut inner = self.lock();
        eprintln!("{}", names::log_appeared(&device, None));
        inner.entries.push(Entry {
            device,
            rev: 1,
            state: State::new(true),
            announced: false,
            plug: Some(plug),
            model: None,
            route: Route::Unsupported,
        });
        let i = inner.entries.len() - 1;
        inner.list(i);
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
        let state = names::initial_state(&device.description);
        self.lock().entries.push(Entry {
            device,
            rev: 0,
            state,
            announced: false,
            plug: Some(plug),
            model: Some(model),
            route: Route::Worker(queue),
        });
    }

    /// The worker opened the camera: the hidden entry takes the model's description, and its
    /// state every described leaf, `null` and in `unread` (what a `hello` at its deadline
    /// shows, 4.1).
    pub fn describe(&self, plug: PlugId, description: Description) {
        let mut inner = self.lock();
        if let Some(i) = inner.by_plug(plug) {
            inner.entries[i].state = names::initial_state(&description);
            inner.entries[i].device.description = description;
        }
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
        let mut inner = self.lock();
        let i = inner.by_plug(plug)?;
        let path = inner.entries[i].device.usb.path.clone();
        let (id, why) = choose_id(serial, reason, &path, |id| inner.held(id, i));
        let display = inner.entries[i].model.map_or("", |m| m.display_name);
        if inner.entries[i].announced {
            inner.entries[i].state = State {
                available: true,
                ..state
            };
            if firmware.is_some() && inner.entries[i].device.firmware.is_none() {
                inner.entries[i].device.firmware = firmware.map(String::from);
            }
            if id.starts_with("sn:") && inner.entries[i].device.id.starts_with("port:") {
                inner.hand_over(i, &id, serial.unwrap_or_default());
            } else {
                inner.entries[i].rev += 1;
                inner.send_devices();
                inner.send_state(i, None);
            }
        } else {
            {
                let e = &mut inner.entries[i];
                e.device.id = id.clone();
                e.device.name = names::name_of(display, &id, &path);
                e.device.serial = if why.is_none() {
                    serial.map(String::from)
                } else {
                    None
                };
                e.device.firmware = firmware.map(String::from);
                e.state = State {
                    available: true,
                    ..state
                };
            }
            let taken = inner
                .entries
                .iter()
                .position(|e| e.announced && !e.device.available && e.device.id == id);
            eprintln!("{}", names::log_appeared(&inner.entries[i].device, why));
            match taken {
                Some(t) => {
                    let new = inner.entries.remove(i);
                    let t = if t > i { t - 1 } else { t };
                    let e = &mut inner.entries[t];
                    e.rev += 1;
                    e.device = new.device;
                    e.state = new.state;
                    e.plug = new.plug;
                    e.model = new.model;
                    e.route = new.route;
                    inner.send_devices();
                    inner.send_state(t, None);
                }
                None => inner.list(i),
            }
        }
        inner.startup.remove(&plug);
        drop(inner);
        self.startup_done.notify_all();
        Some(id)
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
        let mut inner = self.lock();
        let i = inner.by_plug(plug)?;
        if !inner.entries[i].announced {
            return None;
        }
        let mut changed = false;
        if let Some(fw) = firmware
            && inner.entries[i].device.firmware.is_none()
        {
            inner.entries[i].device.firmware = Some(fw.to_string());
            changed = true;
        }
        if let Some(s) = serial
            && inner.entries[i].device.id.starts_with("port:")
        {
            let path = inner.entries[i].device.usb.path.clone();
            let (id, why) = choose_id(Some(s), PortReason::NoSerial, &path, |id| inner.held(id, i));
            if why.is_none() {
                inner.hand_over(i, &id, s);
                return Some(id);
            }
        }
        if changed {
            inner.send_devices();
        }
        None
    }

    /// Publishes a worker's view of its device: values, `pending` and `unread`. A change gets
    /// a new rev and a `state` event; a change caused by no request that touches only volatile
    /// leaves goes only to the connections that subscribe one of their live topics (4.1 rule
    /// 8). An unchanged state keeps its rev and sends nothing. Returns the rev that reflects
    /// the state. A plug that no longer serves its entry (unplugged) changes nothing.
    pub fn publish(&self, plug: PlugId, state: &State, cause: Cause) -> u64 {
        let mut inner = self.lock();
        let Some(i) = inner.by_plug(plug) else {
            return 0;
        };
        let new = State {
            available: inner.entries[i].state.available,
            ..state.clone()
        };
        if !inner.entries[i].announced {
            inner.entries[i].state = new;
            return inner.entries[i].rev;
        }
        if same_state(&inner.entries[i].state, &new) {
            return inner.entries[i].rev;
        }
        let only = match cause {
            Cause::Request => None,
            Cause::Poll => volatile_topics(
                &inner.entries[i].device.description,
                &inner.entries[i].state,
                &new,
            ),
        };
        inner.entries[i].state = new;
        inner.entries[i].rev += 1;
        inner.send_state(i, only.as_ref());
        inner.entries[i].rev
    }

    /// The rev and the published state of a plug's entry.
    pub fn current(&self, plug: PlugId) -> Option<(u64, State)> {
        let inner = self.lock();
        let i = inner.by_plug(plug)?;
        Some((inner.entries[i].rev, inner.entries[i].state.clone()))
    }

    /// The id a plug's entry is listed under now.
    pub fn id_of(&self, plug: PlugId) -> Option<String> {
        let inner = self.lock();
        inner
            .by_plug(plug)
            .map(|i| inner.entries[i].device.id.clone())
    }

    /// The enumeration went away (5.3): an unsupported or `port:` entry is removed (one
    /// `devices` event); an `sn:` entry stays listed with `available: false`, no video node,
    /// `pending` emptied and its `readback: false` leaves `null` (4.2), then one `state`
    /// event. A hidden entry goes without an event.
    pub fn unplug(&self, plug: PlugId) {
        let mut inner = self.lock();
        let Some(i) = inner.by_plug(plug) else {
            return;
        };
        inner.startup.remove(&plug);
        if !inner.entries[i].announced {
            inner.entries.remove(i);
            drop(inner);
            self.startup_done.notify_all();
            return;
        }
        let id = inner.entries[i].device.id.clone();
        eprintln!("{}", names::log_gone(&id));
        if id.starts_with("sn:") {
            let e = &mut inner.entries[i];
            e.plug = None;
            e.route = Route::Unplugged;
            e.device.available = false;
            e.device.video_node = None;
            e.state.available = false;
            e.state.pending.clear();
            let blind: Vec<String> = e
                .device
                .description
                .settings
                .iter()
                .filter(|(_, s)| !s.readback)
                .map(|(p, _)| p.clone())
                .collect();
            for path in blind {
                e.state.set(&path, Value::Null);
            }
            e.rev += 1;
            inner.send_devices();
            inner.send_state(i, None);
        } else {
            inner.entries.remove(i);
            inner.send_devices();
        }
    }

    /// The queues of every available supported device: for the deadline sweeper, and for a
    /// closing connection (its queued requests are dropped, 4.1 rule 9; its continuing action
    /// stopped, 5.6).
    pub fn queues(&self) -> Vec<Arc<DeviceQueue>> {
        self.lock()
            .entries
            .iter()
            .filter_map(|e| match &e.route {
                Route::Worker(q) => Some(q.clone()),
                _ => None,
            })
            .collect()
    }

    /// The listed devices, for tests and logs.
    pub fn devices(&self) -> Vec<Device> {
        self.lock().listed()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use obsbot_device::contract::{Bus, Found};
    use obsbot_device::models::fake::{self, FakeBus, KEYS};
    use obsbot_proto::{Patch, ServerMessage};
    use serde_json::json;
    use std::sync::mpsc::Receiver;
    use std::time::Duration;

    fn found() -> Vec<Found> {
        FakeBus::new(&KEYS).unwrap().scan()
    }

    fn conn(n: ConnId) -> (Outbound, Receiver<String>) {
        Outbound::new(n, None, 64)
    }

    fn drain(rx: &Receiver<String>) -> Vec<ServerMessage> {
        rx.try_iter()
            .map(|l| serde_json::from_str(&l).unwrap())
            .collect()
    }

    fn kinds(msgs: &[ServerMessage]) -> Vec<String> {
        msgs.iter()
            .map(|m| match m {
                ServerMessage::Event(Event::Devices { devices }) => format!(
                    "devices[{}]",
                    devices
                        .iter()
                        .map(|d| {
                            let mut s = d.id.clone();
                            if !d.available {
                                s.push('!');
                            }
                            if let Some(r) = &d.replaced_by {
                                s.push_str(&format!(">{r}"));
                            }
                            s
                        })
                        .collect::<Vec<_>>()
                        .join(",")
                ),
                ServerMessage::Event(Event::State { device, rev, state }) => {
                    format!(
                        "state {device} {rev}{}",
                        if state.available { "" } else { " off" }
                    )
                }
                ServerMessage::Reply(_) => "reply".into(),
                _ => "other".into(),
            })
            .collect()
    }

    fn subscribed(t: &Table, n: ConnId, live: &[&str]) -> Receiver<String> {
        let (out, rx) = conn(n);
        assert!(t.hello(
            &out,
            &RequestId::Int(1),
            &Hello::new("test 1.0").subscribe(true).live(live),
            Instant::now(),
            Duration::from_secs(15)
        ));
        drain(&rx);
        std::mem::forget(out);
        rx
    }

    fn supported(t: &Table, plug: PlugId, f: &Found) -> Arc<DeviceQueue> {
        let q = DeviceQueue::new();
        let model = f.model.unwrap();
        let desc = fake::description(model.key).unwrap();
        let device = names::supported_device(
            f,
            model,
            desc,
            &obsbot_proto::port_id(&f.usb.path),
            None,
            None,
        );
        t.plug_hidden(plug, model, device, q.clone());
        q
    }

    fn some_state(v: i64) -> State {
        let mut s = State::new(true);
        s.set("led.brightness", json!(v));
        s
    }

    /// 4.1 Events: an added entry is a `devices` event, then its `state` at rev 1; an
    /// unsupported entry is removed on unplug with one `devices` event and no state.
    #[test]
    fn an_unsupported_device_comes_and_goes() {
        let t = Table::new("1.0.0");
        let rx = subscribed(&t, 1, &[]);
        let f = found();
        t.plug_unsupported(3, names::unsupported_device(&f[2]));
        assert_eq!(
            kinds(&drain(&rx)),
            ["devices[port:9-3]", "state port:9-3 1"]
        );
        assert!(matches!(t.route("port:9-3"), Routed::Unsupported(rs, _) if rs.rev == 1));
        t.unplug(3);
        assert_eq!(kinds(&drain(&rx)), ["devices[]"]);
        assert!(matches!(t.route("port:9-3"), Routed::Unknown));
    }

    /// 5.2: a supported device is listed only after its first read attempt.
    #[test]
    fn a_supported_device_is_listed_when_announced() {
        let t = Table::new("1.0.0");
        let rx = subscribed(&t, 1, &[]);
        let f = found();
        supported(&t, 1, &f[0]);
        assert!(drain(&rx).is_empty(), "hidden until its first read ended");
        assert!(matches!(t.route("port:9-1"), Routed::Unknown));
        let id = t.announce(
            1,
            Some("FAKEA0000001"),
            PortReason::NoSerial,
            Some("1.0.0.0"),
            some_state(1),
        );
        assert_eq!(id.as_deref(), Some("sn:FAKEA0000001"));
        assert_eq!(
            kinds(&drain(&rx)),
            ["devices[sn:FAKEA0000001]", "state sn:FAKEA0000001 1"]
        );
        let d = &t.devices()[0];
        assert_eq!(
            (d.name.as_str(), d.firmware.as_deref()),
            ("Fake A 0001", Some("1.0.0.0"))
        );
        assert!(matches!(t.route("sn:FAKEA0000001"), Routed::Worker(_)));
    }

    /// 4.1 Events and 4.2: an unplugged sn: entry stays listed, available false, no video
    /// node, pending emptied, readback:false leaves null; the re-plugged device takes the entry
    /// back once its identity is read: devices, then state, its rev going on.
    #[test]
    fn an_sn_device_is_kept_while_unplugged_and_taken_back_on_replug() {
        let t = Table::new("1.0.0");
        let f = found();
        supported(&t, 1, &f[0]);
        let mut s = some_state(1);
        s.set("led.blink", json!(true));
        s.pending.insert("led.brightness".into(), json!(2));
        t.announce(1, Some("FAKEA0000001"), PortReason::NoSerial, None, s);
        let rx = subscribed(&t, 1, &[]);
        t.unplug(1);
        let msgs = drain(&rx);
        assert_eq!(
            kinds(&msgs),
            ["devices[sn:FAKEA0000001!]", "state sn:FAKEA0000001 2 off"]
        );
        let ServerMessage::Event(Event::State { state, .. }) = &msgs[1] else {
            panic!()
        };
        assert!(state.pending.is_empty());
        assert_eq!(
            state.at("led.blink"),
            Some(&Value::Null),
            "readback:false: null again"
        );
        assert_eq!(
            state.at("led.brightness"),
            Some(&json!(1)),
            "the last known values"
        );
        assert_eq!(t.devices()[0].video_node, None);
        assert!(matches!(t.route("sn:FAKEA0000001"), Routed::Unavailable(rs, _) if rs.rev == 2));
        t.publish(1, &some_state(9), Cause::Poll);
        assert!(
            drain(&rx).is_empty(),
            "the old plug publishes nothing any more"
        );

        supported(&t, 2, &f[0]);
        t.announce(
            2,
            Some("FAKEA0000001"),
            PortReason::NoSerial,
            None,
            some_state(1),
        );
        assert_eq!(
            kinds(&drain(&rx)),
            ["devices[sn:FAKEA0000001]", "state sn:FAKEA0000001 3"]
        );
        assert_eq!(t.devices().len(), 1, "one entry");
        assert!(matches!(t.route("sn:FAKEA0000001"), Routed::Worker(_)));
    }

    /// 5.5 and the golden `hotplug.jsonl`: the hand-over is three events in a row; the old
    /// port: entry first shows replaced_by, then is gone; the rev is above both entries'.
    #[test]
    fn the_hand_over_is_three_events_and_takes_the_unplugged_entry() {
        let t = Table::new("1.0.0");
        let f = found();
        supported(&t, 1, &f[0]);
        t.announce(
            1,
            Some("FAKEA0000001"),
            PortReason::NoSerial,
            None,
            some_state(1),
        );
        for v in 2..=8 {
            t.publish(1, &some_state(v), Cause::Request);
        }
        t.unplug(1);
        supported(&t, 2, &f[0]);
        let rx = subscribed(&t, 1, &[]);
        t.announce(2, None, PortReason::NoSerial, None, some_state(2));
        assert_eq!(
            kinds(&drain(&rx)),
            ["devices[sn:FAKEA0000001!,port:9-1]", "state port:9-1 1"]
        );
        let mut s = some_state(2);
        s.pending.insert("led.brightness".into(), json!(3));
        t.publish(2, &s, Cause::Request);
        drain(&rx);
        assert_eq!(
            t.identity(2, Some("FAKEA0000001"), Some("1.0.0.0"))
                .as_deref(),
            Some("sn:FAKEA0000001")
        );
        let msgs = drain(&rx);
        assert_eq!(
            kinds(&msgs),
            [
                "devices[sn:FAKEA0000001,port:9-1!>sn:FAKEA0000001]",
                "devices[sn:FAKEA0000001]",
                "state sn:FAKEA0000001 10"
            ],
            "rev = max(port rev 2, sn rev 9) + 1"
        );
        let ServerMessage::Event(Event::State { state, .. }) = &msgs[2] else {
            panic!()
        };
        assert_eq!(
            state.pending.get("led.brightness"),
            Some(&json!(3)),
            "pending carries over"
        );
        assert!(
            matches!(t.route("port:9-1"), Routed::Unknown),
            "the old id is gone"
        );
        assert!(matches!(t.route("sn:FAKEA0000001"), Routed::Worker(_)));
        assert_eq!(t.devices()[0].firmware.as_deref(), Some("1.0.0.0"));
        assert_eq!(
            t.publish(2, &some_state(4), Cause::Request),
            11,
            "the plug serves the sn: entry"
        );
    }

    #[test]
    fn a_hand_over_without_an_old_entry_renames_in_place() {
        let t = Table::new("1.0.0");
        let f = found();
        supported(&t, 1, &f[0]);
        t.announce(1, None, PortReason::NoSerial, None, some_state(1));
        let rx = subscribed(&t, 1, &[]);
        t.identity(1, Some("FAKEA0000001"), None);
        assert_eq!(
            kinds(&drain(&rx)),
            [
                "devices[sn:FAKEA0000001,port:9-1!>sn:FAKEA0000001]",
                "devices[sn:FAKEA0000001]",
                "state sn:FAKEA0000001 2"
            ]
        );
    }

    /// 5.5 duplicates: a serial an available device holds keeps the newcomer on port:.
    #[test]
    fn a_duplicate_serial_stays_on_its_port_id() {
        let t = Table::new("1.0.0");
        let f = found();
        supported(&t, 1, &f[0]);
        t.announce(
            1,
            Some("FAKEA0000001"),
            PortReason::NoSerial,
            None,
            some_state(1),
        );
        supported(&t, 2, &f[1]);
        let id = t.announce(
            2,
            Some("FAKEA0000001"),
            PortReason::NoSerial,
            None,
            some_state(1),
        );
        assert_eq!(id.as_deref(), Some("port:9-2"));
        assert_eq!(t.devices()[1].serial, None);
        assert_eq!(
            t.identity(2, Some("FAKEA0000001"), None),
            None,
            "still held: no hand-over"
        );
    }

    /// 4.1 rule 2: the reader's answers for unknown, unsupported and unplugged devices.
    #[test]
    fn requests_that_need_no_worker_are_answered_at_once() {
        use crate::queue::{Job, Work};
        let t = Table::new("1.0.0");
        let f = found();
        t.plug_unsupported(3, names::unsupported_device(&f[2]));
        supported(&t, 1, &f[0]);
        t.announce(
            1,
            Some("FAKEA0000001"),
            PortReason::NoSerial,
            None,
            some_state(1),
        );
        t.unplug(1);
        let (out, rx) = conn(1);
        let ask = |device: &str, work: Work| {
            let job = Job {
                id: RequestId::Int(5),
                out: Some(out.clone()),
                conn: 1,
                device: device.into(),
                work,
                arrived: Instant::now(),
                stop: false,
            };
            t.route(device).answer(&job);
            let line: Value = serde_json::from_str(&rx.try_recv().unwrap()).unwrap();
            line
        };
        let set = || Work::Set(Patch::new().leaf("led.brightness", 1));
        let code = |v: Value| v["error"]["code"].clone();
        assert_eq!(code(ask("sn:NOPE00000000", set())), json!("unknown_device"));
        assert_eq!(code(ask("port:9-3", set())), json!("unsupported_device"));
        assert_eq!(
            ask("port:9-3", Work::Get(None))["ok"],
            json!({"rev": 1, "state": {"available": true, "pending": {}, "unread": []}})
        );
        assert_eq!(
            code(ask("port:9-3", Work::Get(Some(vec!["led".into()])))),
            json!("unsupported")
        );
        assert_eq!(code(ask("sn:FAKEA0000001", set())), json!("unavailable"));
        let got = ask("sn:FAKEA0000001", Work::Get(Some(vec!["led".into()])));
        assert_eq!(got["ok"]["rev"], 2);
        assert_eq!(got["ok"]["state"]["available"], false);
        assert_eq!(
            got["ok"]["state"]["led"],
            json!({"blink": null, "brightness": 1}),
            "readback:false is null again after unplug"
        );
        assert_eq!(
            code(ask(
                "sn:FAKEA0000001",
                Work::Do {
                    action: "sleep".into(),
                    params: Default::default()
                }
            )),
            json!("unavailable")
        );
    }

    /// A described hidden entry shows every leaf null and unread at a `hello` deadline.
    #[test]
    fn describe_gives_the_hidden_entry_its_leaves() {
        let t = Table::new("1.0.0");
        let f = found();
        let model = f[0].model.unwrap();
        let device =
            names::supported_device(&f[0], model, Description::default(), "port:9-1", None, None);
        t.plug_hidden(1, model, device, DeviceQueue::new());
        t.describe(1, fake::description("fake-a").unwrap());
        t.startup(&[1]);
        let (out, rx) = conn(1);
        t.hello(
            &out,
            &RequestId::Int(1),
            &Hello::new("x 1"),
            Instant::now(),
            Duration::ZERO,
        );
        let msgs = drain(&rx);
        let ServerMessage::Reply(r) = &msgs[0] else {
            panic!()
        };
        let obsbot_proto::ReplyBody::Ok(OkBody::Hello(h)) = &r.body else {
            panic!()
        };
        let s = &h.states["port:9-1"].state;
        assert_eq!(s.at("led.brightness"), Some(&Value::Null));
        assert!(s.unread.contains(&"led.brightness".to_string()));
        assert_eq!(
            h.devices[0].description.settings.len(),
            fake::description("fake-a").unwrap().settings.len()
        );
    }

    /// 4.1 rules 1 and 3: an unchanged state keeps its rev and sends nothing.
    #[test]
    fn only_a_change_gets_a_rev_and_an_event() {
        let t = Table::new("1.0.0");
        let f = found();
        supported(&t, 1, &f[0]);
        t.announce(
            1,
            Some("FAKEA0000001"),
            PortReason::NoSerial,
            None,
            some_state(1),
        );
        let rx = subscribed(&t, 1, &[]);
        assert_eq!(t.publish(1, &some_state(1), Cause::Poll), 1);
        let mut float = State::new(true);
        float.set("led.brightness", json!(1.0));
        assert_eq!(
            t.publish(1, &float, Cause::Poll),
            1,
            "1 and 1.0 are the same value"
        );
        assert!(drain(&rx).is_empty());
        assert_eq!(t.publish(1, &some_state(2), Cause::Poll), 2);
        assert_eq!(kinds(&drain(&rx)), ["state sn:FAKEA0000001 2"]);
    }

    /// 4.1 rule 8: a change of volatile leaves only, caused by no request, goes only to the
    /// connections subscribing one of their topics; a request's change goes to everyone.
    #[test]
    fn volatile_changes_reach_only_their_live_subscribers() {
        let t = Table::new("1.0.0");
        let f = found();
        supported(&t, 1, &f[0]);
        let mut s = State::new(true);
        s.set("gimbal.pan", json!(0));
        s.set("led.brightness", json!(1));
        t.announce(
            1,
            Some("FAKEA0000001"),
            PortReason::NoSerial,
            None,
            s.clone(),
        );
        let pose = subscribed(&t, 1, &["pose"]);
        let none = subscribed(&t, 2, &[]);
        let (quiet, quiet_rx) = conn(3);
        t.hello(
            &quiet,
            &RequestId::Int(1),
            &Hello::new("cli 1.0"),
            Instant::now(),
            Duration::from_secs(1),
        );
        drain(&quiet_rx);
        assert_eq!(
            t.topics(),
            ["pose".to_string()].into(),
            "subscribe:false connections do not count"
        );
        s.set("gimbal.pan", json!(20));
        t.publish(1, &s, Cause::Poll);
        assert_eq!(kinds(&drain(&pose)), ["state sn:FAKEA0000001 2"]);
        assert!(drain(&none).is_empty());
        s.set("led.brightness", json!(2));
        t.publish(1, &s, Cause::Poll);
        assert_eq!(
            drain(&none).len(),
            1,
            "a non-volatile change reaches everyone, with the new pose"
        );
        s.set("gimbal.pan", json!(30));
        t.publish(1, &s, Cause::Request);
        assert_eq!(drain(&none).len(), 1, "caused by a request: everyone");
        assert!(drain(&quiet_rx).is_empty(), "no events without subscribe");
        t.set_live(2, &["pose".to_string()]);
        s.set("gimbal.pan", json!(40));
        t.publish(1, &s, Cause::Poll);
        assert_eq!(drain(&none).len(), 1, "after live [pose]");
    }

    #[test]
    fn volatile_topics_treat_pending_unread_and_available_as_changes() {
        let desc = fake::description("fake-a").unwrap();
        let a = State::new(true);
        let mut b = a.clone();
        b.set("gimbal.pan", json!(1));
        assert_eq!(
            volatile_topics(&desc, &a, &b),
            Some(["pose".to_string()].into())
        );
        b.unread.push("led.brightness".into());
        assert_eq!(volatile_topics(&desc, &a, &b), None);
        let mut c = a.clone();
        c.pending.insert("led.brightness".into(), json!(1));
        assert_eq!(volatile_topics(&desc, &a, &c), None);
        let _ = Patch::new();
    }

    /// 4.1 `hello` timing: the reply waits for the first read attempts of the first
    /// enumeration, at most until its deadline; a device still reading is then listed with its
    /// leaves null and unread; its first read later publishes a new state.
    #[test]
    fn hello_waits_for_the_first_enumeration_until_its_deadline() {
        let t = Arc::new(Table::new("1.0.0"));
        let f = found();
        supported(&t, 1, &f[0]);
        supported(&t, 2, &f[1]);
        t.startup(&[1, 2]);
        t.announce(
            1,
            Some("FAKEA0000001"),
            PortReason::NoSerial,
            None,
            some_state(1),
        );
        let (out, rx) = conn(1);
        let t0 = Instant::now();
        let hello = Hello::new("test 1.0").subscribe(true);
        assert!(t.hello(
            &out,
            &RequestId::Int(1),
            &hello,
            t0,
            Duration::from_millis(200)
        ));
        assert!(t0.elapsed() >= Duration::from_millis(200));
        let msgs = drain(&rx);
        let ServerMessage::Reply(r) = &msgs[0] else {
            panic!()
        };
        let obsbot_proto::ReplyBody::Ok(OkBody::Hello(h)) = &r.body else {
            panic!()
        };
        assert_eq!(
            h.devices.iter().map(|d| d.id.as_str()).collect::<Vec<_>>(),
            ["sn:FAKEA0000001", "port:9-2"]
        );
        let late = &h.states["port:9-2"];
        assert_eq!(late.rev, 1);
        assert_eq!(late.state.at("led.brightness"), Some(&Value::Null));
        assert!(late.state.unread.contains(&"led.brightness".to_string()));
        t.announce(
            2,
            Some("FAKEB0000001"),
            PortReason::NoSerial,
            None,
            some_state(3),
        );
        assert_eq!(
            kinds(&drain(&rx)),
            [
                "devices[sn:FAKEA0000001,sn:FAKEB0000001,port:9-2!>sn:FAKEB0000001]",
                "devices[sn:FAKEA0000001,sn:FAKEB0000001]",
                "state sn:FAKEB0000001 2"
            ]
        );
        let (out2, rx2) = conn(2);
        let t1 = Instant::now();
        t.hello(
            &out2,
            &RequestId::Int(1),
            &hello,
            t1,
            Duration::from_secs(15),
        );
        assert!(
            t1.elapsed() < Duration::from_millis(100),
            "later hellos do not wait"
        );
        assert_eq!(drain(&rx2).len(), 1);
    }

    #[test]
    fn hello_returns_once_the_first_reads_end() {
        let t = Arc::new(Table::new("1.0.0"));
        let f = found();
        supported(&t, 1, &f[0]);
        t.startup(&[1]);
        let t2 = t.clone();
        let h = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(100));
            t2.announce(
                1,
                Some("FAKEA0000001"),
                PortReason::NoSerial,
                None,
                some_state(1),
            );
        });
        let (out, rx) = conn(1);
        let t0 = Instant::now();
        t.hello(
            &out,
            &RequestId::Int(1),
            &Hello::new("x 1"),
            t0,
            Duration::from_secs(15),
        );
        assert!(t0.elapsed() < Duration::from_secs(2));
        h.join().unwrap();
        let msgs = drain(&rx);
        let ServerMessage::Reply(r) = &msgs[0] else {
            panic!()
        };
        let obsbot_proto::ReplyBody::Ok(OkBody::Hello(h)) = &r.body else {
            panic!()
        };
        assert_eq!(
            h.states["sn:FAKEA0000001"].state.at("led.brightness"),
            Some(&json!(1))
        );
        assert_eq!(h.daemon, "1.0.0");
        assert_eq!(h.protocol, 2);
    }

    /// 5.1: a slow subscriber is closed when its queue is full; the others keep receiving.
    #[test]
    fn a_full_subscriber_is_closed_and_the_rest_go_on() {
        let t = Table::new("1.0.0");
        let f = found();
        supported(&t, 1, &f[0]);
        t.announce(
            1,
            Some("FAKEA0000001"),
            PortReason::NoSerial,
            None,
            some_state(0),
        );
        let (slow, _slow_rx) = Outbound::new(9, None, 3);
        t.hello(
            &slow,
            &RequestId::Int(1),
            &Hello::new("slow 1").subscribe(true),
            Instant::now(),
            Duration::from_secs(1),
        );
        let fast = subscribed(&t, 2, &[]);
        for v in 1..=10 {
            t.publish(1, &some_state(v), Cause::Request);
        }
        assert!(slow.is_closed());
        assert_eq!(drain(&fast).len(), 10);
    }
}
