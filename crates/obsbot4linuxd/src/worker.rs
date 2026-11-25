// SPDX-License-Identifier: GPL-3.0-or-later

//! The worker of one supported device (5.1, 5.2, 5.5 to 5.7): the only code that touches the
//! device. Written in task b-07-worker.

use crate::Timing;
use crate::daemon::Fatal;
use crate::identity::{PortReason, Series};
use crate::names;
use crate::outbound::ok_line;
use crate::pending::Pending;
use crate::queue::{self, DeviceQueue, Job, Work};
use crate::schedule::Schedule;
use crate::table::{Cause, PlugId, Table};
use obsbot_device::contract::{
    Bus, Camera, CameraError, Ctx, Found, Group, ModelEntry, Motion, Need, Tier,
};
use obsbot_proto::validate::{Fit, check_value, leaves, validate_get};
use obsbot_proto::values::is_raw;
use obsbot_proto::{Description, OkBody, Params, Patch, RevState, State, Warning, port_id};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::io;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

/// Everything a worker gets from the registry thread when its device is plugged in.
pub struct Plugged {
    pub plug: PlugId,
    pub found: Found,
    pub model: &'static ModelEntry,
    pub bus: Arc<dyn Bus>,
    pub table: Arc<Table>,
    pub queue: Arc<DeviceQueue>,
    pub timing: Timing,
}

/// Starts the worker thread `worker <usb path>`; a panic in it is fatal (5.9).
pub fn spawn(p: Plugged, fatal: Arc<Fatal>) -> io::Result<JoinHandle<()>> {
    let name = format!("worker {}", p.found.usb.path);
    crate::daemon::critical(name, fatal, move || run(p))
}

/// One worker's view of its device. `state` holds the values and `unread`; `pending` is kept
/// in `pending` and copied into the published state by `published`.
pub(crate) struct Worker {
    pub plug: PlugId,
    /// The USB path of the enumeration: names the device in logs until it is listed.
    pub usb_path: String,
    pub cam: Box<dyn Camera>,
    pub desc: Description,
    pub plan: Vec<Group>,
    /// `Camera::motion` of every action of the description, asked once at open.
    pub motion: BTreeMap<String, Motion>,
    pub model: &'static ModelEntry,
    pub table: Arc<Table>,
    pub queue: Arc<DeviceQueue>,
    pub timing: Timing,
    pub state: State,
    pub pending: Pending,
    pub schedule: Schedule,
    pub series: Series,
    /// After a frame with a `restart` warning: nothing is sent until then (5.6).
    pub quiet_until: Option<Instant>,
    /// `(leaf, raw value)` pairs already logged (4.2).
    pub raw_logged: BTreeSet<(String, String)>,
}

/// The worker's whole life: open, first identity attempt and first read, announce, then the
/// loop of batches and ticks until the device becomes unavailable or the daemon stops.
pub fn run(p: Plugged) {
    let cam = match p.bus.open(&p.found) {
        Ok(cam) => cam,
        Err(e) => return open_failed(&p, &e),
    };
    let mut w = Worker::new(&p, cam);
    w.queue.set_stops(
        w.motion
            .iter()
            .filter(|(_, m)| **m == Motion::Stop)
            .map(|(name, _)| name.clone())
            .collect(),
    );
    w.table.describe(w.plug, w.desc.clone());
    if let Some(need) = w.series.at_plug_in() {
        w.attempt(need);
    }
    let retry = w.first_read();
    w.schedule = Schedule::new(Instant::now(), &w.timing);
    for name in retry {
        w.schedule.touched_read(&name, false);
    }
    w.table.announce(
        w.plug,
        w.series.serial.as_deref(),
        w.series.reason(),
        w.series.firmware.as_deref(),
        w.published(),
    );
    loop {
        if w.queue.is_ended() {
            w.end();
            return;
        }
        if w.queue.is_shutdown() {
            w.own_stops(w.queue.take_all());
            w.queue.close();
            return;
        }
        if let Some(until) = w.quiet_until {
            if until > Instant::now() {
                w.queue.wait_quiet(until);
                continue;
            }
            w.quiet_until = None;
        }
        if w.queue.len() > 0 {
            let jobs = w.queue.take_all();
            w.serve(jobs);
            continue;
        }
        if w.pending
            .next_settle()
            .is_some_and(|at| at <= Instant::now())
        {
            w.settle();
            continue;
        }
        if let Some(tier) = w.schedule.due(Instant::now()) {
            w.tick(tier);
            continue;
        }
        let next = w.schedule.next_due();
        w.queue
            .wait(w.pending.next_settle().map_or(next, |at| at.min(next)));
    }
}

/// The camera could not be opened (decision 17): the device is listed with every leaf unread,
/// `get` is answered from that state, `set` and `do` with `camera`; nothing is sent.
fn open_failed(p: &Plugged, e: &CameraError) {
    let id = port_id(&p.found.usb.path);
    eprintln!("{}", names::log_open_failed(&id, &e.to_string()));
    p.table
        .announce(p.plug, None, PortReason::NoSerial, None, State::new(true));
    let empty = Description::default();
    loop {
        if p.queue.is_ended() {
            answer_ended(&p.queue, &p.table);
            return;
        }
        if p.queue.is_shutdown() {
            p.queue.close();
            return;
        }
        let jobs = p.queue.take_all();
        if jobs.is_empty() {
            p.queue.wait(Instant::now() + Duration::from_secs(1));
            continue;
        }
        for job in jobs {
            match &job.work {
                Work::Get(paths) => {
                    if let Some(paths) = paths
                        && let Err(r) = validate_get(&empty, paths)
                    {
                        job.fail(r.code, &r.message);
                        continue;
                    }
                    match p.table.current(p.plug) {
                        Some((rev, state)) => {
                            let state = match paths {
                                Some(paths) => state.select(paths),
                                None => state,
                            };
                            job.answer(ok_line(&job.id, OkBody::State(RevState { rev, state })));
                        }
                        None => p.table.route(&job.device).answer(&job),
                    }
                }
                Work::Set(_) | Work::Do { .. } => job.fail(
                    obsbot_proto::ErrorCode::Camera,
                    &format!("the camera could not be opened: {e}"),
                ),
            }
        }
    }
}

/// 5.6 "Unavailable": the device went; every queued request is answered as for an unavailable
/// device. Publishes nothing.
fn answer_ended(queue: &DeviceQueue, table: &Table) {
    for job in queue.close() {
        if job.out.is_some() {
            table.route(&job.device).answer(&job);
        }
    }
}

impl Worker {
    fn new(p: &Plugged, cam: Box<dyn Camera>) -> Worker {
        let desc = cam.description().clone();
        let motion = desc
            .actions
            .keys()
            .map(|name| (name.clone(), cam.motion(name)))
            .collect();
        Worker {
            plug: p.plug,
            usb_path: p.found.usb.path.clone(),
            plan: cam.poll_plan().to_vec(),
            motion,
            model: p.model,
            table: p.table.clone(),
            queue: p.queue.clone(),
            timing: p.timing,
            state: names::initial_state(&desc),
            pending: Pending::new(),
            schedule: Schedule::new(Instant::now(), &p.timing),
            series: Series::new(p.model.identity, p.model.firmware_query),
            quiet_until: None,
            raw_logged: BTreeSet::new(),
            desc,
            cam,
        }
    }

    /// The state requests are validated against: the camera's values with every `pending`
    /// value laid over its leaf, so that a value written but not yet shown counts as current
    /// for `requires` and for the asleep check (4.1 rule 5, decision of 2026-11-13).
    pub(crate) fn view(&self) -> State {
        let mut state = self.published();
        for (leaf, value) in &state.pending.clone() {
            state.set(leaf, value.clone());
        }
        state
    }

    /// The state the table publishes: what the camera showed, with `pending` and `unread`
    /// beside it (4.1 rules 3 and 4: until a read shows the write, the camera's value stands).
    pub(crate) fn published(&self) -> State {
        State {
            available: true,
            pending: self.pending.map(),
            unread: self.state.unread.clone(),
            values: self.state.values.clone(),
        }
    }

    pub(crate) fn publish(&mut self, cause: Cause) -> u64 {
        self.table.publish(self.plug, &self.published(), cause)
    }

    /// The id the device is logged under: its listed id, else its port id.
    fn log_id(&self) -> String {
        self.table
            .id_of(self.plug)
            .unwrap_or_else(|| port_id(&self.usb_path))
    }

    /// Every error of a camera operation passes here first (decision 22, 8.3): a frame the
    /// model's send table refused is logged; every other error is not logged here.
    pub(crate) fn failed(&self, error: &CameraError) {
        if let CameraError::NotInSendTable(message) = error {
            eprintln!("{}", names::log_refused_frame(&self.log_id(), message));
        }
    }

    /// The daemon's own stops at shutdown (5.9): the stop of a continuing action whose connection
    /// closed is sent, not dropped; every other queued job is dropped unsent. Nothing is sent
    /// during the quiet time after a `restart` write (5.6). The stop runs without a cut flag, so
    /// the shutdown does not cancel it; its deadline is the request deadline.
    fn own_stops(&mut self, jobs: Vec<Job>) {
        if self.quiet_until.is_some_and(|until| until > Instant::now()) {
            return;
        }
        for job in jobs {
            let Work::Do { action, .. } = &job.work else {
                continue;
            };
            if job.out.is_some() || !job.stop {
                continue;
            }
            let ctx = Ctx::new(Some(job.arrived + self.timing.request_deadline), None);
            if let Err(err) = self.cam.act(action, &Params::new(), &self.state, &ctx) {
                self.failed(&err);
            }
        }
    }

    /// The context of a poll, the first read and a touched re-read without a deadline: a stop
    /// cuts it at its next frame boundary.
    pub(crate) fn poll_ctx(&self) -> Ctx {
        Ctx::new(None, Some(self.queue.cut.clone()))
    }

    fn group(&self, name: &str) -> Option<Group> {
        self.plan.iter().find(|g| g.name == name).cloned()
    }

    /// Reads one group (5.7); `true` when the camera answered.
    pub(crate) fn read_group(&mut self, name: &str, ctx: &Ctx) -> bool {
        let Some(group) = self.group(name) else {
            return false;
        };
        if self.state.asleep() && !group.asleep {
            if group.tier == Tier::Touched {
                for leaf in &group.leaves {
                    if !self.state.unread.contains(leaf) {
                        self.state.unread.push(leaf.clone());
                    }
                }
                self.schedule.touched_read(name, false);
            }
            return false;
        }
        match self.cam.read(name, ctx) {
            Ok(values) => {
                for leaf in &group.leaves {
                    let v = values.get(leaf).cloned().unwrap_or(Value::Null);
                    if let Value::String(text) = &v
                        && is_raw(text)
                        && self.raw_logged.insert((leaf.clone(), text.clone()))
                    {
                        eprintln!("{}", names::log_raw(&self.log_id(), leaf, text));
                    }
                    self.state.set(leaf, v);
                    self.state.unread.retain(|u| u != leaf);
                }
                self.pending
                    .group_read(&self.desc.settings, &values, Instant::now());
                true
            }
            Err(e) => {
                self.failed(&e);
                for leaf in &group.leaves {
                    if !self.state.unread.contains(leaf) {
                        self.state.unread.push(leaf.clone());
                    }
                }
                false
            }
        }
    }

    /// 5.2 steps 3 and 4: the group holding `asleep` first, then every other group in plan
    /// order. Returns the `touched` groups whose read did not happen or failed.
    fn first_read(&mut self) -> Vec<String> {
        let mut retry = Vec::new();
        let status = self
            .plan
            .iter()
            .find(|g| g.leaves.iter().any(|l| l == "asleep"))
            .map(|g| g.name.clone());
        let mut asleep = false;
        if let Some(name) = &status {
            self.read_group(name, &self.poll_ctx());
            asleep = self.state.asleep();
        }
        self.series.after_first_status(asleep);
        for group in self.plan.clone() {
            if Some(&group.name) == status.as_ref() {
                continue;
            }
            if self.queue.halt.load(Ordering::SeqCst) {
                break;
            }
            if asleep && !group.asleep {
                continue;
            }
            let ok = self.read_group(&group.name, &self.poll_ctx());
            if group.tier == Tier::Touched && !ok {
                retry.push(group.name.clone());
            }
        }
        retry
    }

    /// One tick of a periodic tier (5.7).
    fn tick(&mut self, tier: Tier) {
        let was_asleep = self.state.asleep();
        let topics = self.table.topics();
        if tier == self.model.identity_tier
            && let Some(need) = self.series.at_tick(!was_asleep)
        {
            self.attempt(need);
        }
        for name in self.schedule.groups(&self.plan, tier, &topics, was_asleep) {
            // A device that went or a daemon that stops serves nothing more: the loop of `run`
            // answers the queue (5.6 "Unavailable").
            if self.queue.halt.load(Ordering::SeqCst) {
                break;
            }
            if self.queue.len() > 0 {
                let jobs = self.queue.take_all();
                self.serve(jobs);
            }
            // A `restart` write in that batch: nothing more is sent until the quiet time ends
            // (5.6 "Restart").
            if self.queue.halt.load(Ordering::SeqCst) || self.quiet_until.is_some() {
                break;
            }
            let ok = self.read_group(&name, &self.poll_ctx());
            if self.group(&name).is_some_and(|g| g.tier == Tier::Touched) {
                self.schedule.touched_read(&name, ok);
            }
        }
        self.schedule.ticked(tier, Instant::now(), &self.timing);
        self.publish(Cause::Poll);
        if was_asleep && !self.state.asleep() && self.quiet_until.is_none() {
            self.wake();
        }
    }

    /// The settle read (5.6, #335): a pending leaf whose bound has passed gets one read of its
    /// group, so that `pending` ends within `settle_ms` and the camera's value stands, also
    /// when the camera kept another value than the one written. A group whose tier is not
    /// slower than the leaf's `settle_ms` is not read here: its own tier ends `pending` in time.
    fn settle(&mut self) {
        let leaves = self.pending.settle_due(Instant::now());
        let topics = self.table.topics();
        let slower = |g: &Group, leaf: &str| {
            let settle = self.desc.settings.get(leaf).and_then(|s| s.settle_ms);
            let settle = Duration::from_millis(u64::from(settle.unwrap_or(0)));
            self.timing
                .period(crate::schedule::effective(g, &topics))
                .is_none_or(|period| period > settle)
        };
        let groups: Vec<String> = self
            .plan
            .iter()
            .filter(|g| leaves.iter().any(|l| g.leaves.contains(l) && slower(g, l)))
            .map(|g| g.name.clone())
            .collect();
        if groups.is_empty() {
            return;
        }
        for name in groups {
            if self.queue.halt.load(Ordering::SeqCst) {
                break;
            }
            if self.queue.len() > 0 {
                let jobs = self.queue.take_all();
                self.serve(jobs);
            }
            // A `restart` write in that batch: nothing more is sent until the quiet time ends
            // (5.6 "Restart").
            if self.queue.halt.load(Ordering::SeqCst) || self.quiet_until.is_some() {
                break;
            }
            self.read_group(&name, &self.poll_ctx());
        }
        self.publish(Cause::Poll);
    }

    /// The camera woke (5.7 "Wake and retries"): every group with a leaf in `unread` or
    /// `pending` is read now, the `touched` ones included, and the next tick of each tier read
    /// counts from here.
    pub(crate) fn wake(&mut self) {
        if let Some(need) = self.series.at_wake() {
            self.attempt(need);
        }
        // A pending leaf's group is read as well: this read ends `pending` or starts its bound
        // (5.6, 5.7), where a settle read while asleep could not.
        let mut leaves = self.state.unread.clone();
        leaves.extend(self.pending.map().into_keys());
        let groups = Schedule::wake_groups(&self.plan, &leaves);
        for name in &groups {
            if self.queue.halt.load(Ordering::SeqCst) {
                break;
            }
            let ok = self.read_group(name, &self.poll_ctx());
            if self.group(name).is_some_and(|g| g.tier == Tier::Touched) {
                self.schedule.touched_read(name, ok);
            }
        }
        let topics = self.table.topics();
        self.schedule
            .woke(&self.plan, &groups, &topics, Instant::now(), &self.timing);
        self.publish(Cause::Poll);
    }

    /// One identity attempt (5.5).
    fn attempt(&mut self, need: Need) {
        let ctx = Ctx::new(None, Some(self.queue.halt.clone()));
        let got = self.cam.identity(need, &ctx);
        if let Err(e) = &got {
            self.failed(e);
        }
        self.series.answered(got.ok().as_ref());
        self.table.identity(
            self.plug,
            self.series.serial.as_deref(),
            self.series.firmware.as_deref(),
        );
    }

    /// The device went (5.6 "Unavailable").
    fn end(&mut self) {
        answer_ended(&self.queue, &self.table);
    }

    /// Plans one batch of the queued requests and runs it; the requests the plan does not take
    /// go back to the front of the queue.
    fn serve(&mut self, mut jobs: Vec<Job>) {
        let desc = &self.desc;
        // 4.1 rule 9: the one leaf of a `set` whose value the setting can take.
        let single_leaf = |patch: &Patch| {
            let mut found = leaves(desc, patch).ok()?;
            let (leaf, value) = found.pop().filter(|_| found.is_empty())?;
            let setting = desc.settings.get(&leaf)?;
            (!setting.readonly && check_value(setting, &value, Fit::Write).is_ok()).then_some(leaf)
        };
        // 5.6 "Restart": a `set` with a leaf whose setting warns `restart`, or a `do` whose
        // action does.
        let restarts = |job: &Job| match &job.work {
            Work::Set(patch) => leaves(desc, patch).is_ok_and(|found| {
                found.iter().any(|(leaf, _)| {
                    desc.settings
                        .get(leaf)
                        .is_some_and(|s| s.warning.contains(&Warning::Restart))
                })
            }),
            Work::Do { action, .. } => desc
                .actions
                .get(action)
                .is_some_and(|a| a.warning.contains(&Warning::Restart)),
            Work::Get(_) => false,
        };
        let plan = queue::plan(&jobs, restarts, single_leaf);
        if plan.take == 0 {
            return;
        }
        let rest = jobs.split_off(plan.take);
        // 4.1 rule 2: the sweeper must not answer a queued request before this batch.
        self.queue.start(&jobs, rest);
        self.batch(jobs, plan.coalesced);
        self.queue.served();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::names;
    use crate::outbound::Outbound;
    use crate::queue::{Job, Work};
    use obsbot_device::contract::Found;
    use obsbot_device::models::fake::{FakeBus, Op};
    use obsbot_proto::{Description, Hello, Patch, RequestId};
    use serde_json::{Value, json};
    use std::sync::Mutex;
    use std::sync::mpsc::Receiver;
    use std::time::{Duration, Instant};

    fn fast() -> Timing {
        Timing {
            scan: Duration::from_millis(100),
            request_deadline: Duration::from_secs(15),
            second: Duration::from_millis(100),
            five_seconds: Duration::from_millis(500),
            minute: Duration::from_secs(3),
        }
    }

    struct Rig {
        bus: Arc<FakeBus>,
        table: Arc<Table>,
        queue: Arc<DeviceQueue>,
        events: Receiver<String>,
        out: Outbound,
        replies: Receiver<String>,
        handle: JoinHandle<()>,
    }

    /// One worker over one fake device, as the registry would start it, with one subscribed
    /// connection (`events`) and one requester (`out`/`replies`).
    fn rig(model: &str, setup: impl FnOnce(&FakeBus)) -> Rig {
        rig_over(model, setup, |bus| bus)
    }

    /// [`rig`] with the worker opening the device through `wrap(bus)`.
    fn rig_over(
        model: &str,
        setup: impl FnOnce(&FakeBus),
        wrap: impl FnOnce(Arc<FakeBus>) -> Arc<dyn Bus>,
    ) -> Rig {
        rig_timed(model, setup, wrap, fast())
    }

    /// [`rig_over`] on the tier periods of `timing`.
    fn rig_timed(
        model: &str,
        setup: impl FnOnce(&FakeBus),
        wrap: impl FnOnce(Arc<FakeBus>) -> Arc<dyn Bus>,
        timing: Timing,
    ) -> Rig {
        let bus = Arc::new(FakeBus::new(&[model]).unwrap());
        setup(&bus);
        let found = bus.scan().remove(0);
        let entry = found.model.unwrap();
        let table = Arc::new(Table::new("test"));
        let queue = DeviceQueue::new();
        let id = obsbot_proto::port_id(&found.usb.path);
        let device =
            names::supported_device(&found, entry, Description::default(), &id, None, None);
        table.plug_hidden(1, entry, device, queue.clone());
        let (sub, events) = Outbound::new(1, None, 10_000);
        table.hello(
            &sub,
            &RequestId::Int(0),
            &Hello::new("t 1").subscribe(true),
            Instant::now(),
            Duration::ZERO,
        );
        events.recv().unwrap();
        let (out, replies) = Outbound::new(2, None, 10_000);
        let p = Plugged {
            plug: 1,
            found,
            model: entry,
            bus: wrap(bus.clone()),
            table: table.clone(),
            queue: queue.clone(),
            timing,
        };
        let handle = std::thread::spawn(move || run(p));
        Rig {
            bus,
            table,
            queue,
            events,
            out,
            replies,
            handle,
        }
    }

    fn next(rx: &Receiver<String>) -> Value {
        serde_json::from_str(&rx.recv_timeout(Duration::from_secs(5)).expect("a line")).unwrap()
    }

    fn until(rx: &Receiver<String>, pred: impl Fn(&Value) -> bool) -> Value {
        loop {
            let v = next(rx);
            if pred(&v) {
                return v;
            }
        }
    }

    fn job(r: &Rig, id: i64, device: &str, work: Work) -> Job {
        Job {
            id: RequestId::Int(id),
            out: Some(r.out.clone()),
            conn: 2,
            device: device.into(),
            work,
            arrived: Instant::now(),
            stop: false,
        }
    }

    fn stop(r: Rig) {
        r.queue.shutdown();
        r.handle.join().unwrap();
    }

    /// 5.2: identity attempt, then the group holding `asleep`, then every other group; then
    /// the entry is listed (devices, then state at rev 1).
    #[test]
    fn the_first_read_attempt_is_announced() {
        let r = rig("fake-a", |_| {});
        let devices = until(&r.events, |v| v["event"] == "devices");
        assert_eq!(devices["devices"][0]["id"], "sn:FAKEA0000001");
        let st = until(&r.events, |v| v["event"] == "state");
        assert_eq!(st["rev"], 1);
        assert_eq!(st["state"]["led"]["brightness"], 0);
        assert_eq!(st["state"]["unread"], json!([]));
        let ops = r.bus.ops("sn:FAKEA0000001");
        assert_eq!(
            ops[..3],
            [
                Op::Open,
                Op::Identity(obsbot_device::contract::Need {
                    serial: true,
                    firmware: true
                }),
                Op::Read("status".into())
            ]
        );
        assert!(
            ops.contains(&Op::Read("presets".into())),
            "touched groups are read at plug-in"
        );
        stop(r);
    }

    /// 5.2 and 5.5: asleep at plug-in, only the groups readable asleep are read; one identity
    /// attempt; the rest is null and unread.
    #[test]
    fn asleep_at_plug_in_only_the_groups_readable_asleep_are_read() {
        let r = rig("fake-b", |bus| bus.sleep("sn:FAKEB0000001").unwrap());
        let st = until(&r.events, |v| v["event"] == "state");
        assert_eq!(st["state"]["asleep"], true);
        assert!(
            st["state"]["unread"]
                .as_array()
                .unwrap()
                .contains(&json!("led.brightness"))
        );
        assert_eq!(st["state"]["led"]["brightness"], Value::Null);
        std::thread::sleep(Duration::from_millis(1200));
        let ops = r.bus.ops("sn:FAKEB0000001");
        let reads: Vec<&Op> = ops.iter().filter(|o| matches!(o, Op::Read(_))).collect();
        assert!(
            reads.iter().all(|o| **o == Op::Read("status".into())),
            "{reads:?}"
        );
        assert_eq!(
            ops.iter().filter(|o| matches!(o, Op::Identity(_))).count(),
            1
        );
        stop(r);
    }

    /// 4.1 rules 3 and 4: the state that reflects the write is published before the reply,
    /// with the leaf pending while the readback lags.
    #[test]
    fn a_set_publishes_before_it_replies() {
        let r = rig("fake-b", |_| {});
        until(&r.events, |v| v["event"] == "state");
        r.queue
            .push(job(
                &r,
                7,
                "sn:FAKEB0000001",
                Work::Set(Patch::new().leaf("led.brightness", 3)),
            ))
            .ok()
            .unwrap();
        let reply = next(&r.replies);
        let ev = until(&r.events, |v| v["event"] == "state");
        assert_eq!(reply["ok"]["rev"], ev["rev"]);
        assert_eq!(ev["state"]["pending"], json!({"led.brightness": 3}));
        let later = until(&r.events, |v| {
            v["event"] == "state" && v["state"]["pending"] == json!({})
        });
        assert_eq!(later["state"]["led"]["brightness"], 3);
        stop(r);
    }

    /// 5.6 "Unavailable": the worker ends; a queued request is answered as for an unavailable
    /// device.
    #[test]
    fn the_end_answers_the_queue() {
        let r = rig("fake-a", |_| {});
        until(&r.events, |v| v["event"] == "state");
        r.bus.set_delay("fake-a", Duration::from_secs(2));
        std::thread::sleep(Duration::from_millis(300));
        r.queue
            .push(job(&r, 8, "sn:FAKEA0000001", Work::Get(None)))
            .ok()
            .unwrap();
        r.queue
            .push(job(
                &r,
                9,
                "sn:FAKEA0000001",
                Work::Set(Patch::new().leaf("led.brightness", 1)),
            ))
            .ok()
            .unwrap();
        r.bus.unplug("sn:FAKEA0000001").unwrap();
        r.table.unplug(1);
        r.queue.end();
        let t0 = Instant::now();
        r.handle.join().unwrap();
        assert!(
            t0.elapsed() < Duration::from_secs(1),
            "the 2 s operation was cut"
        );
        let a = next(&r.replies);
        let b = next(&r.replies);
        assert_eq!(a["ok"]["state"]["available"], false);
        assert_eq!(b["error"]["code"], "unavailable");
    }

    /// A bus whose cameras are the fake's, except that the transport wrapper refuses the frame
    /// of one leaf (8.3).
    struct RefusingBus(Arc<FakeBus>, &'static str);

    impl Bus for RefusingBus {
        fn scan(&self) -> Vec<Found> {
            self.0.scan()
        }

        fn open(&self, found: &Found) -> Result<Box<dyn Camera>, CameraError> {
            Ok(Box::new(Refusing(self.0.open(found)?, self.1)))
        }
    }

    struct Refusing(Box<dyn Camera>, &'static str);

    impl Camera for Refusing {
        fn description(&self) -> &Description {
            self.0.description()
        }

        fn identity(
            &mut self,
            need: Need,
            ctx: &Ctx,
        ) -> Result<obsbot_device::contract::Identity, CameraError> {
            self.0.identity(need, ctx)
        }

        fn poll_plan(&self) -> &[Group] {
            self.0.poll_plan()
        }

        fn read(&mut self, group: &str, ctx: &Ctx) -> Result<obsbot_proto::Values, CameraError> {
            self.0.read(group, ctx)
        }

        fn order(&self, leaves: &[&str]) -> Vec<String> {
            self.0.order(leaves)
        }

        fn apply(&mut self, leaf: &str, value: &Value, ctx: &Ctx) -> Result<(), CameraError> {
            if leaf == self.1 {
                return Err(CameraError::NotInSendTable(leaf.to_string()));
            }
            self.0.apply(leaf, value, ctx)
        }

        fn check(
            &self,
            action: &str,
            params: &obsbot_proto::Params,
            state: &State,
        ) -> Result<(), obsbot_proto::validate::Refusal> {
            self.0.check(action, params, state)
        }

        fn act(
            &mut self,
            action: &str,
            params: &obsbot_proto::Params,
            state: &State,
            ctx: &Ctx,
        ) -> Result<Option<obsbot_proto::Params>, CameraError> {
            self.0.act(action, params, state, ctx)
        }

        fn touched(&self, target: obsbot_device::contract::Target<'_>) -> Vec<String> {
            self.0.touched(target)
        }

        fn motion(&self, action: &str) -> Motion {
            self.0.motion(action)
        }

        fn send_table(&self) -> &obsbot_device::contract::SendTable {
            self.0.send_table()
        }
    }

    /// 8.3 and decision 22: a frame outside the send table is never sent and the request is
    /// answered `internal` (not `camera`), naming the leaf; nothing reaches the camera.
    #[test]
    fn a_refused_frame_is_answered_internal() {
        let r = rig_over(
            "fake-b",
            |_| {},
            |bus| Arc::new(RefusingBus(bus, "led.brightness")),
        );
        until(&r.events, |v| v["event"] == "state");
        r.queue
            .push(job(
                &r,
                5,
                "sn:FAKEB0000001",
                Work::Set(Patch::new().leaf("led.brightness", 3)),
            ))
            .ok()
            .unwrap();
        let reply = next(&r.replies);
        assert_eq!(reply["error"]["code"], "internal", "{reply}");
        assert_eq!(reply["error"]["failed"], "led.brightness");
        assert_eq!(reply["error"]["applied"], json!([]));
        assert!(
            !r.bus
                .ops("sn:FAKEB0000001")
                .iter()
                .any(|o| matches!(o, Op::Apply(..))),
            "nothing was sent"
        );
        stop(r);
    }

    /// Set by the test once the worker is up: the table and queue the camera ends.
    /// Runs once, in the worker, right after the next `apply` the camera accepted.
    type Hook = Arc<Mutex<Option<Box<dyn FnOnce() + Send>>>>;

    /// A camera that runs the hook after an `apply` succeeded, while its batch is not answered.
    struct AfterApply(Box<dyn Camera>, Hook);

    struct AfterApplyBus(Arc<FakeBus>, Hook);

    impl Bus for AfterApplyBus {
        fn scan(&self) -> Vec<Found> {
            self.0.scan()
        }

        fn open(&self, found: &Found) -> Result<Box<dyn Camera>, CameraError> {
            Ok(Box::new(AfterApply(self.0.open(found)?, self.1.clone())))
        }
    }

    impl Camera for AfterApply {
        fn description(&self) -> &Description {
            self.0.description()
        }

        fn identity(
            &mut self,
            need: Need,
            ctx: &Ctx,
        ) -> Result<obsbot_device::contract::Identity, CameraError> {
            self.0.identity(need, ctx)
        }

        fn poll_plan(&self) -> &[Group] {
            self.0.poll_plan()
        }

        fn read(&mut self, group: &str, ctx: &Ctx) -> Result<obsbot_proto::Values, CameraError> {
            self.0.read(group, ctx)
        }

        fn order(&self, leaves: &[&str]) -> Vec<String> {
            self.0.order(leaves)
        }

        fn apply(&mut self, leaf: &str, value: &Value, ctx: &Ctx) -> Result<(), CameraError> {
            self.0.apply(leaf, value, ctx)?;
            let hook = self.1.lock().unwrap().take();
            if let Some(hook) = hook {
                hook();
            }
            Ok(())
        }

        fn check(
            &self,
            action: &str,
            params: &obsbot_proto::Params,
            state: &State,
        ) -> Result<(), obsbot_proto::validate::Refusal> {
            self.0.check(action, params, state)
        }

        fn act(
            &mut self,
            action: &str,
            params: &obsbot_proto::Params,
            state: &State,
            ctx: &Ctx,
        ) -> Result<Option<obsbot_proto::Params>, CameraError> {
            self.0.act(action, params, state, ctx)
        }

        fn touched(&self, target: obsbot_device::contract::Target<'_>) -> Vec<String> {
            self.0.touched(target)
        }

        fn motion(&self, action: &str) -> Motion {
            self.0.motion(action)
        }

        fn send_table(&self) -> &obsbot_device::contract::SendTable {
            self.0.send_table()
        }
    }

    /// 5.6: the device goes after a request was sent and before the groups are re-read: the
    /// request is answered `ok` with the rev of the last state published before (not 0, which
    /// no state ever had).
    #[test]
    fn an_ok_after_the_device_went_carries_the_last_published_rev() {
        let slot = Arc::new(Mutex::new(None));
        let hook = slot.clone();
        let r = rig_over(
            "fake-b",
            |_| {},
            move |bus| Arc::new(AfterApplyBus(bus, hook)),
        );
        let st = until(&r.events, |v| v["event"] == "state");
        let published = st["rev"].as_u64().unwrap();
        assert!(published > 0);
        // The device goes as the registry does on unplug: the entry is unplugged, the queue ended.
        let (table, queue) = (r.table.clone(), r.queue.clone());
        *slot.lock().unwrap() = Some(Box::new(move || {
            table.unplug(1);
            queue.end();
        }));
        r.queue
            .push(job(
                &r,
                6,
                "sn:FAKEB0000001",
                Work::Set(Patch::new().leaf("led.brightness", 3)),
            ))
            .ok()
            .unwrap();
        r.handle.join().unwrap();
        let reply = next(&r.replies);
        assert_eq!(reply["ok"]["rev"], published, "{reply}");
    }

    /// 4.1 rules 2 and 9: request 2 is queued while request 1 runs, and the sweeper looks at
    /// the queue after 2's deadline but before 1 is answered (a frame of 1 ran past 1's
    /// deadline). Request 2 must not be answered before request 1: the sweeper leaves it, and
    /// the worker answers it after the batch of 1.
    #[test]
    fn the_sweeper_never_answers_a_request_before_the_started_one_ahead_of_it() {
        let slot = Arc::new(Mutex::new(None));
        let hook = slot.clone();
        let r = rig_over(
            "fake-b",
            |_| {},
            move |bus| Arc::new(AfterApplyBus(bus, hook)),
        );
        until(&r.events, |v| v["event"] == "state");
        let second = job(&r, 2, "sn:FAKEB0000001", Work::Get(None));
        let queue = r.queue.clone();
        *slot.lock().unwrap() = Some(Box::new(move || {
            queue.push(second).ok().unwrap();
            let sweep_at = Instant::now() + Duration::from_secs(16);
            crate::queue::answer_late(queue.expire(sweep_at, Duration::from_secs(15)));
        }));
        r.queue
            .push(job(
                &r,
                1,
                "sn:FAKEB0000001",
                Work::Set(Patch::new().leaf("led.brightness", 3)),
            ))
            .ok()
            .unwrap();
        let first = next(&r.replies);
        assert_eq!(first["id"], 1, "request 1 is answered first: {first}");
        assert!(first["ok"].is_object(), "{first}");
        let then = next(&r.replies);
        assert_eq!(then["id"], 2, "{then}");
        stop(r);
    }

    /// 5.6 (#335): the camera keeps another value than the one written (measured on the Tiny 2
    /// Lite: a zone area clamped to the stored bound). The leaf is pending after the touched
    /// re-read; one read of its group when `settle_ms` (3 s) has passed ends `pending`, and the
    /// state shows the camera's value. Without that read the leaf stayed pending, showing the
    /// written range as if the write had taken, until the group's 60 s tier.
    #[test]
    fn a_value_the_camera_clamped_stops_being_pending_once_settle_ms_has_passed() {
        let slot = Arc::new(Mutex::new(None));
        let hook = slot.clone();
        let r = rig_timed(
            "fake-a",
            |_| {},
            move |bus| Arc::new(AfterApplyBus(bus, hook)),
            Timing {
                minute: Duration::from_secs(60),
                ..fast()
            },
        );
        until(&r.events, |v| v["event"] == "state");
        let id = "sn:FAKEA0000001";
        let clamped = json!({"min": -40, "max": 20});
        let bus = r.bus.clone();
        let kept = clamped.clone();
        *slot.lock().unwrap() = Some(Box::new(move || {
            bus.store(id, "zone.area.pan", kept).unwrap();
        }));
        let reads = |r: &Rig| {
            r.bus
                .ops(id)
                .iter()
                .filter(|o| **o == Op::Read("zone".into()))
                .count()
        };
        let before = reads(&r);
        let written = Instant::now();
        r.queue
            .push(job(
                &r,
                3,
                id,
                Work::Set(Patch::new().leaf("zone.area.pan", json!({"min": -20, "max": 20}))),
            ))
            .ok()
            .unwrap();
        let reply = next(&r.replies);
        assert!(reply["ok"].is_object(), "{reply}");
        let ev = until(&r.events, |v| v["rev"] == reply["ok"]["rev"]);
        assert_eq!(
            ev["state"]["pending"],
            json!({"zone.area.pan": {"min": -20, "max": 20}}),
            "the touched re-read shows the clamped range: pending"
        );
        let settled = loop {
            let left = Duration::from_secs(10).saturating_sub(written.elapsed());
            let line = r
                .events
                .recv_timeout(left)
                .expect("pending ends within settle_ms, not at the 60 s tier");
            let v: Value = serde_json::from_str(&line).unwrap();
            if v["event"] == "state" && v["state"]["pending"] == json!({}) {
                break v;
            }
        };
        assert!(
            written.elapsed() >= Duration::from_millis(3000),
            "not before settle_ms"
        );
        assert_eq!(settled["state"]["zone"]["area"]["pan"], clamped);
        assert_eq!(
            reads(&r) - before,
            2,
            "the touched re-read and one settle read"
        );
        stop(r);
    }

    /// A worker over one fake device, not started.
    fn worker_of(model: &str) -> Worker {
        worker_timed(model, fast()).0
    }

    /// [`worker_of`] on the tier periods of `timing`, with its bus.
    fn worker_timed(model: &str, timing: Timing) -> (Worker, Arc<FakeBus>) {
        let bus = Arc::new(FakeBus::new(&[model]).unwrap());
        let found = bus.scan().remove(0);
        let entry = found.model.unwrap();
        let cam = bus.open(&found).unwrap();
        let p = Plugged {
            plug: 1,
            found,
            model: entry,
            bus: bus.clone(),
            table: Arc::new(Table::new("test")),
            queue: DeviceQueue::new(),
            timing,
        };
        (Worker::new(&p, cam), bus)
    }

    /// The fast timing with the real 60 s tier, so that only a settle read can end `pending`
    /// of a leaf in a 60 s group within a test.
    fn slow_minute() -> Timing {
        Timing {
            minute: Duration::from_secs(60),
            ..fast()
        }
    }

    /// Makes `leaf` pending with `value`, its bound started `ago`.
    fn pending_since(w: &mut Worker, leaf: &str, value: Value, ago: Duration) {
        let setting = w.desc.settings[leaf].clone();
        w.pending
            .written(&setting, leaf, value, None, Some(Instant::now() - ago));
    }

    fn reads(ops: &[Op]) -> usize {
        ops.iter().filter(|o| matches!(o, Op::Read(_))).count()
    }

    /// 5.6 "Restart" (#336 review): a `restart` write served before a settle read ends the
    /// reads; nothing more reaches the device until the quiet time is over, because the camera
    /// is restarting and a read would be a frame it may not take.
    #[test]
    fn a_restart_write_served_before_a_settle_read_ends_the_reads() {
        let (mut w, bus) = worker_timed("fake-a", slow_minute());
        let id = "sn:FAKEA0000001";
        // Not started: no first read listed the leaves; nothing is unread yet.
        w.state.unread.clear();
        pending_since(
            &mut w,
            "zone.area.pan",
            json!({"min": -20, "max": 20}),
            Duration::from_secs(4),
        );
        w.queue
            .push(Job {
                id: RequestId::Int(1),
                out: None,
                conn: 2,
                device: id.into(),
                work: Work::Set(Patch::new().leaf("audio.enabled", true)),
                arrived: Instant::now(),
                stop: false,
            })
            .ok()
            .unwrap();
        let before = bus.ops(id).len();
        w.settle();
        let ops = bus.ops(id)[before..].to_vec();
        assert!(
            ops.iter()
                .any(|o| matches!(o, Op::Apply(l, _) if l == "audio.enabled")),
            "{ops:?}"
        );
        assert!(w.quiet_until.is_some());
        assert_eq!(reads(&ops), 0, "no read during the quiet time: {ops:?}");
        // The fake records no operation while it restarts; a read tried then fails and lists the
        // group's leaves in `unread`.
        assert!(
            !w.state.unread.iter().any(|l| l.starts_with("zone.")),
            "no read was tried: {:?}",
            w.state.unread
        );
    }

    /// 5.6: a leaf whose group's tier is not slower than its `settle_ms` gets no settle read
    /// (fake-b's `led.brightness`, 1000 ms, in a group read every 100 ms here): its tier ends
    /// `pending` in time, and an extra read would only add traffic.
    #[test]
    fn a_group_whose_tier_is_not_slower_than_settle_ms_gets_no_settle_read() {
        let (mut w, bus) = worker_timed("fake-b", fast());
        let id = "sn:FAKEB0000001";
        pending_since(&mut w, "led.brightness", json!(3), Duration::from_secs(2));
        let before = bus.ops(id).len();
        w.settle();
        assert_eq!(reads(&bus.ops(id)[before..]), 0);
        assert!(
            w.pending.next_settle().is_none(),
            "declined, not asked again"
        );
    }

    /// 5.6, 5.7 (#336 review): the camera falls asleep before `settle_ms` has passed, so the
    /// settle read cannot run (the zone group is not readable asleep). The wake read reads the
    /// group of every pending leaf, so `pending` ends at wake, not at the 60 s tier.
    #[test]
    fn a_leaf_still_pending_at_wake_is_read_by_the_wake_read() {
        let slot = Arc::new(Mutex::new(None));
        let hook = slot.clone();
        let r = rig_timed(
            "fake-a",
            |_| {},
            move |bus| Arc::new(AfterApplyBus(bus, hook)),
            slow_minute(),
        );
        until(&r.events, |v| v["event"] == "state");
        let id = "sn:FAKEA0000001";
        let bus = r.bus.clone();
        *slot.lock().unwrap() = Some(Box::new(move || {
            bus.store(id, "zone.area.pan", json!({"min": -40, "max": 20}))
                .unwrap();
        }));
        r.queue
            .push(job(
                &r,
                4,
                id,
                Work::Set(Patch::new().leaf("zone.area.pan", json!({"min": -20, "max": 20}))),
            ))
            .ok()
            .unwrap();
        let reply = next(&r.replies);
        let ev = until(&r.events, |v| v["rev"] == reply["ok"]["rev"]);
        assert_ne!(ev["state"]["pending"], json!({}), "{ev}");
        r.bus.sleep(id).unwrap();
        until(&r.events, |v| v["state"]["asleep"] == true);
        std::thread::sleep(Duration::from_millis(3500));
        r.bus.wake(id).unwrap();
        let woke = Instant::now();
        let settled = loop {
            let left = Duration::from_secs(5).saturating_sub(woke.elapsed());
            let line = r
                .events
                .recv_timeout(left)
                .expect("pending ends at the wake read, not at the 60 s tier");
            let v: Value = serde_json::from_str(&line).unwrap();
            if v["event"] == "state" && v["state"]["pending"] == json!({}) {
                break v;
            }
        };
        assert_eq!(settled["state"]["asleep"], false);
        assert_eq!(
            settled["state"]["zone"]["area"]["pan"],
            json!({"min": -40, "max": 20})
        );
        stop(r);
    }

    /// 4.1 rule 5 (decision of 2026-11-13): a value in `pending` counts as the current value
    /// for `requires`; the published state keeps the camera's value beside it.
    #[test]
    fn view_lays_pending_values_over_the_leaves() {
        use obsbot_proto::ErrorCode;
        use obsbot_proto::validate::validate_set;
        let mut w = worker_of("fake-b");
        w.state.set("exposure.mode", json!("auto"));
        let patch = Patch::new().leaf("exposure.time", 2000);
        assert_eq!(
            validate_set(&w.desc, &w.view(), &patch).unwrap_err().code,
            ErrorCode::Conflict,
            "the camera shows auto and nothing is pending"
        );
        w.pending.written(
            &w.desc.settings["exposure.mode"],
            "exposure.mode",
            json!("manual"),
            None,
            Some(Instant::now()),
        );
        assert_eq!(w.view().at("exposure.mode"), Some(&json!("manual")));
        assert!(validate_set(&w.desc, &w.view(), &patch).is_ok());
        let published = w.published();
        assert_eq!(published.at("exposure.mode"), Some(&json!("auto")));
        assert_eq!(published.pending["exposure.mode"], json!("manual"));
    }

    /// The same rule across two batches: while the camera is asleep the mode stays pending,
    /// and a later `set` of the leaf that requires it is accepted, not a `conflict`.
    #[test]
    fn a_pending_requirement_counts_in_a_later_batch() {
        let r = rig("fake-b", |bus| bus.sleep("sn:FAKEB0000001").unwrap());
        until(&r.events, |v| v["event"] == "state");
        r.queue
            .push(job(
                &r,
                1,
                "sn:FAKEB0000001",
                Work::Set(Patch::new().leaf("exposure.mode", "manual")),
            ))
            .ok()
            .unwrap();
        let first = next(&r.replies);
        assert!(first["ok"].is_object(), "{first}");
        let ev = until(&r.events, |v| {
            v["event"] == "state" && v["state"]["pending"].get("exposure.mode").is_some()
        });
        assert_eq!(ev["state"]["pending"]["exposure.mode"], "manual");
        r.queue
            .push(job(
                &r,
                2,
                "sn:FAKEB0000001",
                Work::Set(Patch::new().leaf("exposure.time", 2000)),
            ))
            .ok()
            .unwrap();
        let second = next(&r.replies);
        assert!(second["ok"].is_object(), "{second}");
        stop(r);
    }

    /// Section 2 and decision 9: the camera gets the value the request names, with float noise
    /// put on its step; `pending` holds the echo (4.2). fake-b's `led.brightness` lags its
    /// write, so the leaf is pending.
    #[test]
    fn the_camera_gets_the_named_value_and_pending_its_echo() {
        let r = rig("fake-b", |_| {});
        until(&r.events, |v| v["event"] == "state");
        let noisy = 2.999_999_999_999_999_6;
        r.queue
            .push(job(
                &r,
                3,
                "sn:FAKEB0000001",
                Work::Set(Patch::new().leaf("led.brightness", noisy)),
            ))
            .ok()
            .unwrap();
        let reply = next(&r.replies);
        assert!(reply["ok"].is_object(), "{reply}");
        let apply = r
            .bus
            .ops("sn:FAKEB0000001")
            .into_iter()
            .find_map(|o| match o {
                Op::Apply(leaf, value) if leaf == "led.brightness" => Some(value),
                _ => None,
            })
            .expect("one apply");
        assert_eq!(apply.to_string(), "3", "sent on its step");
        let ev = until(&r.events, |v| {
            v["event"] == "state" && v["rev"] == reply["ok"]["rev"]
        });
        assert_eq!(ev["state"]["pending"], json!({"led.brightness": 3}));
        // A precision without a step: nothing is rounded before it is sent.
        r.queue
            .push(job(
                &r,
                4,
                "sn:FAKEB0000001",
                Work::Set(Patch::new().leaf("zoom.factor", 1.2345)),
            ))
            .ok()
            .unwrap();
        assert!(next(&r.replies)["ok"].is_object());
        assert!(
            r.bus
                .ops("sn:FAKEB0000001")
                .contains(&Op::Apply("zoom.factor".into(), json!(1.2345))),
            "sent as named, not rounded to the precision"
        );
        stop(r);
    }

    /// 4.1 rule 9: a request whose connection closed before the worker started it is not
    /// sent and not answered.
    #[test]
    fn a_request_of_a_closed_connection_is_not_sent() {
        let r = rig("fake-b", |_| {});
        until(&r.events, |v| v["event"] == "state");
        r.out.mark_closed();
        r.queue
            .push(job(
                &r,
                4,
                "sn:FAKEB0000001",
                Work::Set(Patch::new().leaf("led.brightness", 2)),
            ))
            .ok()
            .unwrap();
        let t0 = Instant::now();
        while r.queue.len() > 0 && t0.elapsed() < Duration::from_secs(5) {
            std::thread::sleep(Duration::from_millis(10));
        }
        std::thread::sleep(Duration::from_millis(200));
        assert!(
            !r.bus
                .ops("sn:FAKEB0000001")
                .iter()
                .any(|o| matches!(o, Op::Apply(..))),
            "nothing was sent"
        );
        assert!(r.replies.try_recv().is_err(), "nothing was answered");
        stop(r);
    }
}
