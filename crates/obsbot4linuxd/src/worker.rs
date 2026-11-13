// SPDX-License-Identifier: GPL-3.0-or-later

//! The worker of one supported device (5.1, 5.2, 5.5 to 5.7): the only code that touches the
//! device. Written in task b-07-worker.

use crate::Timing;
use crate::daemon::Fatal;
use crate::identity::{PortReason, Series};
use crate::names;
use crate::outbound::{error, error_line, ok_line};
use crate::pending::Pending;
use crate::queue::{self, DeviceQueue, Job, Work};
use crate::schedule::Schedule;
use crate::table::{Cause, PlugId, Table};
use obsbot_device::contract::{
    Bus, Camera, CameraError, Ctx, Found, Group, ModelEntry, Motion, Need, Tier,
};
use obsbot_proto::validate::validate_get;
use obsbot_proto::values::is_raw;
use obsbot_proto::{Description, OkBody, RevState, State, port_id};
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
            drop(w.queue.take_all());
            w.queue.close();
            return;
        }
        if w.queue.len() > 0 {
            let jobs = w.queue.take_all();
            w.serve(jobs);
            continue;
        }
        if let Some(tier) = w.schedule.due(Instant::now()) {
            w.tick(tier);
            continue;
        }
        w.queue.wait(w.schedule.next_due());
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
            if self.queue.halt.load(Ordering::SeqCst) {
                break;
            }
            let ok = self.read_group(&name, &self.poll_ctx());
            if self.group(&name).is_some_and(|g| g.tier == Tier::Touched) {
                self.schedule.touched_read(&name, ok);
            }
        }
        self.schedule.ticked(tier, Instant::now(), &self.timing);
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
        let plan = queue::plan(&jobs, |_| false, |_| None);
        if plan.take == 0 {
            return;
        }
        let rest = jobs.split_off(plan.take);
        self.queue.put_back(rest);
        self.batch(jobs, plan.coalesced);
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
            timing: fast(),
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

    /// A worker over one fake device, not started.
    fn worker_of(model: &str) -> Worker {
        let bus = Arc::new(FakeBus::new(&[model]).unwrap());
        let found = bus.scan().remove(0);
        let entry = found.model.unwrap();
        let cam = bus.open(&found).unwrap();
        let p = Plugged {
            plug: 1,
            found,
            model: entry,
            bus,
            table: Arc::new(Table::new("test")),
            queue: DeviceQueue::new(),
            timing: fast(),
        };
        Worker::new(&p, cam)
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
