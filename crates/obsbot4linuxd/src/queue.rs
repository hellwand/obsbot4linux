// SPDX-License-Identifier: GPL-3.0-or-later

//! The queue of one supported device (4.1 rules 2 and 9, 5.6): at most `MAX_QUEUED` requests,
//! FIFO, except that a stop is taken before every queued request and is accepted whatever the
//! length. Connection readers push,
//! the device's worker takes batches, the deadline sweeper removes requests not started within
//! `REQUEST_DEADLINE`. The queue also holds the continuing-motion record (5.6) and the flags
//! that cut a camera operation short.

use crate::MAX_QUEUED;
use crate::outbound::{ConnId, Outbound, error, error_line};
use obsbot_device::contract::Motion;
use obsbot_proto::{ErrorCode, Params, Patch, RequestId};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

/// What a device request asks.
#[derive(Debug, Clone, PartialEq)]
pub enum Work {
    /// `paths: None` for the full state.
    Get(Option<Vec<String>>),
    Set(Patch),
    Do {
        action: String,
        params: Params,
    },
}

/// One queued device request.
#[derive(Clone)]
pub struct Job {
    pub id: RequestId,
    /// Where the reply goes; `None` for the daemon's own stop of a continuing action.
    pub out: Option<Outbound>,
    /// The connection that sent it; 0 for the daemon's own stop.
    pub conn: ConnId,
    /// The device id the request named.
    pub device: String,
    pub work: Work,
    pub arrived: Instant,
    /// A stop action (`Motion::Stop`): taken before every queued request.
    pub stop: bool,
}

impl Job {
    /// The daemon's own stop of a continuing action whose connection closed (5.6).
    pub fn own_stop(action: &str, now: Instant) -> Job {
        Job {
            id: RequestId::Int(0),
            out: None,
            conn: 0,
            device: String::new(),
            work: Work::Do {
                action: action.to_string(),
                params: Params::new(),
            },
            arrived: now,
            stop: true,
        }
    }

    /// Sends a reply line to the job's connection, if it has one.
    pub fn answer(&self, line: String) {
        if let Some(out) = &self.out {
            out.send(line);
        }
    }

    /// Answers the job with an error.
    pub fn fail(&self, code: ErrorCode, message: &str) {
        self.answer(error_line(Some(&self.id), error(code, message)));
    }
}

/// Which connection's continuing action runs on a device (5.6), and the stop that ends it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Record {
    turn: Option<(ConnId, String)>,
}

impl Record {
    /// An action ran (the camera accepted it). A continuing action without `keep: true`
    /// records its connection; with `keep: true` nothing is recorded, since closing the
    /// connection must not stop it. Any other motion action clears the record; a stop clears it
    /// only when it is the stop the record names.
    pub fn after_act(&mut self, action: &str, motion: Motion, conn: ConnId, keep: bool) {
        match motion {
            Motion::None => {}
            Motion::Move => self.turn = None,
            Motion::Continuing { stop } => {
                self.turn = (!keep && conn != 0).then(|| (conn, stop.to_string()));
            }
            Motion::Stop => {
                if self.turn.as_ref().is_some_and(|(_, s)| s == action) {
                    self.turn = None;
                }
            }
        }
    }

    /// The connection `conn` closed: the stop to queue for it, if its continuing action is the
    /// recorded one. The record is cleared.
    pub fn on_close(&mut self, conn: ConnId) -> Option<String> {
        match &self.turn {
            Some((c, _)) if *c == conn => self.turn.take().map(|(_, stop)| stop),
            _ => None,
        }
    }
}

/// Why the queue did not take a job.
pub enum Refused {
    /// `MAX_QUEUED` requests wait already: answer `busy` (never for a stop).
    Busy(Box<Job>),
    /// The device became unavailable and its worker ended: answer as for an unavailable device.
    Closed(Box<Job>),
}

struct Inner {
    jobs: VecDeque<Job>,
    closed: bool,
    stops: Vec<String>,
    record: Record,
}

/// The queue of one device, shared by the readers, the worker and the sweeper.
pub struct DeviceQueue {
    inner: Mutex<Inner>,
    cv: Condvar,
    /// Cuts a group read at the next frame boundary: a stop arrived, the device went, or the
    /// daemon stops (5.6 "Stops").
    pub cut: Arc<AtomicBool>,
    /// Cuts an identity attempt or a poll: the device went or the daemon stops.
    pub halt: Arc<AtomicBool>,
    /// Cuts a started request: the device went (5.6 "Unavailable").
    pub ended: Arc<AtomicBool>,
    shutdown: AtomicBool,
}

impl DeviceQueue {
    pub fn new() -> Arc<DeviceQueue> {
        Arc::new(DeviceQueue {
            inner: Mutex::new(Inner {
                jobs: VecDeque::new(),
                closed: false,
                stops: Vec::new(),
                record: Record::default(),
            }),
            cv: Condvar::new(),
            cut: Arc::new(AtomicBool::new(false)),
            halt: Arc::new(AtomicBool::new(false)),
            ended: Arc::new(AtomicBool::new(false)),
            shutdown: AtomicBool::new(false),
        })
    }

    fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// The stop actions of the device's model (`Motion::Stop`), set by the worker once it has
    /// opened the camera, before the device is announced.
    pub fn set_stops(&self, stops: Vec<String>) {
        self.lock().stops = stops;
    }

    pub fn is_stop(&self, action: &str) -> bool {
        self.lock().stops.iter().any(|s| s == action)
    }

    /// Queues a job. A stop is accepted whatever the length, goes before every queued request
    /// that is not a stop, and sets `cut` so that a group read in progress ends at its next
    /// frame boundary. Every other request is refused as `busy` when `MAX_QUEUED` wait.
    pub fn push(&self, job: Job) -> Result<(), Refused> {
        let mut inner = self.lock();
        if inner.closed {
            return Err(Refused::Closed(Box::new(job)));
        }
        if !job.stop && inner.jobs.len() >= MAX_QUEUED {
            return Err(Refused::Busy(Box::new(job)));
        }
        if job.stop {
            let at = inner.jobs.iter().take_while(|j| j.stop).count();
            inner.jobs.insert(at, job);
            self.cut.store(true, Ordering::SeqCst);
        } else {
            inner.jobs.push_back(job);
        }
        drop(inner);
        self.cv.notify_all();
        Ok(())
    }

    /// Takes every queued job; the worker plans its batch from them and puts the rest back.
    pub fn take_all(&self) -> Vec<Job> {
        let mut inner = self.lock();
        let jobs: Vec<Job> = inner.jobs.drain(..).collect();
        if !self.halt.load(Ordering::SeqCst) {
            self.cut.store(false, Ordering::SeqCst);
        }
        jobs
    }

    /// Puts jobs back at the front, in their order (the requests after a failed one, 4.1 rule
    /// 9, or the rest of a batch plan). A stop pushed meanwhile stays in front of them.
    pub fn put_back(&self, jobs: Vec<Job>) {
        let mut inner = self.lock();
        let stops = inner.jobs.iter().take_while(|j| j.stop).count();
        for (i, job) in jobs.into_iter().enumerate() {
            inner.jobs.insert(stops + i, job);
        }
    }

    pub fn len(&self) -> usize {
        self.lock().jobs.len()
    }

    /// The connection `conn` closed: its queued requests are dropped (4.1 rule 9), and if its
    /// continuing action is recorded, the model's stop is queued for it (5.6). Returns the stop.
    pub fn close_conn(&self, conn: ConnId, now: Instant) -> Option<String> {
        let mut inner = self.lock();
        inner.jobs.retain(|j| j.conn != conn);
        let stop = inner.record.on_close(conn)?;
        if inner.closed {
            return None;
        }
        let at = inner.jobs.iter().take_while(|j| j.stop).count();
        inner.jobs.insert(at, Job::own_stop(&stop, now));
        drop(inner);
        self.cut.store(true, Ordering::SeqCst);
        self.cv.notify_all();
        Some(stop)
    }

    /// Updates the continuing-motion record after the camera accepted an action.
    pub fn record_act(&self, action: &str, motion: Motion, conn: ConnId, keep: bool) {
        self.lock().record.after_act(action, motion, conn, keep);
    }

    /// Removes the jobs not started within `deadline` of their arrival (4.1 rule 9).
    pub fn expire(&self, now: Instant, deadline: Duration) -> Vec<Job> {
        let mut inner = self.lock();
        let (late, keep): (Vec<Job>, Vec<Job>) = inner
            .jobs
            .drain(..)
            .partition(|j| now.duration_since(j.arrived) >= deadline);
        inner.jobs = keep.into();
        late
    }

    /// The device became unavailable: every camera operation is cut and the worker ends.
    pub fn end(&self) {
        self.ended.store(true, Ordering::SeqCst);
        self.halt.store(true, Ordering::SeqCst);
        self.cut.store(true, Ordering::SeqCst);
        self.cv.notify_all();
    }

    /// The daemon stops: polls and identity attempts are cut; the worker sends the stops still
    /// queued and ends (5.9).
    pub fn shutdown(&self) {
        self.shutdown.store(true, Ordering::SeqCst);
        self.halt.store(true, Ordering::SeqCst);
        self.cut.store(true, Ordering::SeqCst);
        self.cv.notify_all();
    }

    pub fn is_ended(&self) -> bool {
        self.ended.load(Ordering::SeqCst)
    }

    pub fn is_shutdown(&self) -> bool {
        self.shutdown.load(Ordering::SeqCst)
    }

    /// The worker is about to end: no job is taken any more, and the queued ones are returned
    /// to be answered as for an unavailable device.
    pub fn close(&self) -> Vec<Job> {
        let mut inner = self.lock();
        inner.closed = true;
        inner.jobs.drain(..).collect()
    }

    /// Waits until a job is queued, the device ends or the daemon stops, or `until` passes.
    pub fn wait(&self, until: Instant) {
        let mut inner = self.lock();
        loop {
            if !inner.jobs.is_empty() || self.is_ended() || self.is_shutdown() {
                return;
            }
            let now = Instant::now();
            if now >= until {
                return;
            }
            inner = self
                .cv
                .wait_timeout(inner, until - now)
                .unwrap_or_else(PoisonError::into_inner)
                .0;
        }
    }
}

/// Answers the requests the sweeper removed: `camera`, not started within the deadline.
pub fn answer_late(jobs: Vec<Job>) {
    for job in jobs {
        job.fail(ErrorCode::Camera, "the request was not started within 15 s");
    }
}

/// How the worker takes a batch (4.1 rule 9, 5.6): `take` jobs from the front of the queue;
/// `coalesced[i]` marks a single-leaf `set` that a later `set` of the same connection in the
/// batch, with the same single leaf, makes unnecessary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    pub take: usize,
    pub coalesced: Vec<bool>,
}

/// Plans one batch. A stop runs alone; a request with a `restart` warning runs alone and ends
/// the batch before it; otherwise the batch runs up to the first such request. `restarts`
/// tells a request with a `restart` warning; `single_leaf` gives the one leaf of a `set`
/// whose patch has exactly one leaf with a value the setting can take, else `None`.
pub fn plan(
    jobs: &[Job],
    restarts: impl Fn(&Job) -> bool,
    single_leaf: impl Fn(&Patch) -> Option<String>,
) -> Plan {
    let take = match jobs.first() {
        None => 0,
        Some(first) if first.stop || restarts(first) => 1,
        Some(_) => jobs
            .iter()
            .position(|j| j.stop || restarts(j))
            .unwrap_or(jobs.len()),
    };
    let leaves: Vec<Option<String>> = jobs[..take]
        .iter()
        .map(|j| match &j.work {
            Work::Set(patch) => single_leaf(patch),
            _ => None,
        })
        .collect();
    // Only when nothing between them can change what the later one is validated against
    // (`requires`, `asleep`): every request in between is a `get` or a single-leaf `set` of
    // the same leaf. Otherwise the later one may be refused after the earlier one was
    // answered `ok` without being sent.
    let same_leaf_or_get = |j: usize, leaf: &String| {
        matches!(jobs[j].work, Work::Get(_)) || leaves[j].as_ref() == Some(leaf)
    };
    let coalesced = (0..take)
        .map(|i| {
            leaves[i].as_ref().is_some_and(|leaf| {
                (i + 1..take).any(|k| {
                    jobs[k].conn == jobs[i].conn
                        && leaves[k].as_ref() == Some(leaf)
                        && (i + 1..k).all(|j| same_leaf_or_get(j, leaf))
                })
            })
        })
        .collect();
    Plan { take, coalesced }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn job(conn: ConnId, work: Work) -> Job {
        Job {
            id: RequestId::Int(conn as i64),
            out: None,
            conn,
            device: "sn:FAKEA0000001".into(),
            work,
            arrived: Instant::now(),
            stop: false,
        }
    }

    fn set(conn: ConnId, leaf: &str, v: i64) -> Job {
        job(conn, Work::Set(Patch::new().leaf(leaf, v)))
    }

    fn act(conn: ConnId, action: &str) -> Job {
        job(
            conn,
            Work::Do {
                action: action.into(),
                params: Params::new(),
            },
        )
    }

    fn stop(conn: ConnId) -> Job {
        Job {
            stop: true,
            ..act(conn, "gimbal.stop")
        }
    }

    fn names(jobs: &[Job]) -> Vec<String> {
        jobs.iter()
            .map(|j| match &j.work {
                Work::Get(_) => "get".to_string(),
                Work::Set(p) => format!("set {}", p.0.keys().next().unwrap()),
                Work::Do { action, .. } => action.clone(),
            })
            .collect()
    }

    fn single(p: &Patch) -> Option<String> {
        (p.0.len() == 1).then(|| p.0.keys().next().unwrap().clone())
    }

    #[test]
    fn requests_queue_in_order_and_a_stop_overtakes_them() {
        let q = DeviceQueue::new();
        q.push(set(1, "led.brightness", 1)).ok().unwrap();
        q.push(act(1, "preset.recall")).ok().unwrap();
        assert!(!q.cut.load(Ordering::SeqCst));
        q.push(stop(2)).ok().unwrap();
        assert!(
            q.cut.load(Ordering::SeqCst),
            "a group read in progress is cut"
        );
        q.push(Job {
            stop: true,
            ..act(3, "zoom.stop")
        })
        .ok()
        .unwrap();
        assert_eq!(
            names(&q.take_all()),
            [
                "gimbal.stop",
                "zoom.stop",
                "set led.brightness",
                "preset.recall"
            ]
        );
        assert!(
            !q.cut.load(Ordering::SeqCst),
            "taking the stops clears the cut"
        );
    }

    /// 4.1 `busy`: 32 requests are already queued for the device; a stop is the exception
    /// (decision of 2026-11-13) and goes before the queued requests.
    #[test]
    fn the_33rd_queued_request_is_busy() {
        let q = DeviceQueue::new();
        for i in 0..MAX_QUEUED {
            assert!(q.push(set(1, "led.brightness", i as i64)).is_ok());
        }
        assert!(matches!(
            q.push(set(1, "led.brightness", 0)),
            Err(Refused::Busy(_))
        ));
        assert!(
            matches!(q.push(act(1, "preset.recall")), Err(Refused::Busy(_))),
            "an action is busy too"
        );
        assert!(
            q.push(stop(1)).is_ok(),
            "a stop is accepted when 32 requests are queued"
        );
        assert!(q.cut.load(Ordering::SeqCst), "and cuts a group read");
        assert!(
            q.push(Job {
                stop: true,
                ..act(1, "zoom.stop")
            })
            .is_ok(),
            "a second stop is accepted too"
        );
        assert!(
            matches!(q.push(set(1, "led.brightness", 0)), Err(Refused::Busy(_))),
            "a request that is not a stop stays busy while 32 or more wait"
        );
        let jobs = q.take_all();
        assert_eq!(jobs.len(), MAX_QUEUED + 2);
        assert_eq!(
            names(&jobs[..3]),
            ["gimbal.stop", "zoom.stop", "set led.brightness"],
            "the stops are placed before the queued requests"
        );
    }

    #[test]
    fn a_closed_queue_takes_nothing() {
        let q = DeviceQueue::new();
        q.push(set(1, "led.brightness", 1)).ok().unwrap();
        assert_eq!(q.close().len(), 1);
        assert!(matches!(
            q.push(set(1, "led.brightness", 1)),
            Err(Refused::Closed(_))
        ));
    }

    /// 4.1 rule 9: a request whose connection closed before the worker started it is dropped.
    #[test]
    fn a_closed_connection_drops_its_queued_requests() {
        let q = DeviceQueue::new();
        q.push(set(1, "led.brightness", 1)).ok().unwrap();
        q.push(set(2, "led.brightness", 2)).ok().unwrap();
        assert_eq!(q.close_conn(1, Instant::now()), None);
        assert_eq!(q.take_all().iter().map(|j| j.conn).collect::<Vec<_>>(), [2]);
    }

    /// 5.6: the connection that started a continuing action without `keep` closes: the
    /// model's stop is queued for it, ahead of the other requests.
    #[test]
    fn closing_the_turning_connection_queues_the_stop() {
        let q = DeviceQueue::new();
        let turn = Motion::Continuing {
            stop: "gimbal.stop",
        };
        q.record_act("gimbal.turn", turn, 5, false);
        q.push(set(6, "led.brightness", 1)).ok().unwrap();
        assert_eq!(
            q.close_conn(6, Instant::now()),
            None,
            "another connection: nothing"
        );
        q.push(set(7, "led.brightness", 1)).ok().unwrap();
        assert_eq!(
            q.close_conn(5, Instant::now()).as_deref(),
            Some("gimbal.stop")
        );
        let jobs = q.take_all();
        assert_eq!(
            names(&jobs),
            ["gimbal.stop", "set led.brightness"],
            "the stop goes first; connection 6's set was dropped when it closed"
        );
        assert_eq!(jobs[1].conn, 7);
        assert!(jobs[0].out.is_none() && jobs[0].conn == 0 && jobs[0].stop);
        assert_eq!(q.close_conn(5, Instant::now()), None, "only once");
    }

    /// 5.6 and 15.1 row 89: no stop for a `keep: true` turn; another motion clears the record;
    /// only the recorded stop clears it among the stops.
    #[test]
    fn the_record_follows_the_motion_marks() {
        let turn = Motion::Continuing {
            stop: "gimbal.stop",
        };
        let mut r = Record::default();
        r.after_act("gimbal.turn", turn, 5, true);
        assert_eq!(
            r.on_close(5),
            None,
            "keep: true is never stopped by the daemon"
        );
        r.after_act("gimbal.turn", turn, 5, false);
        r.after_act("preset.save", Motion::None, 6, false);
        r.after_act("zoom.stop", Motion::Stop, 6, false);
        assert_eq!(
            r.clone().on_close(5).as_deref(),
            Some("gimbal.stop"),
            "another stop leaves it"
        );
        r.after_act("preset.recall", Motion::Move, 6, false);
        assert_eq!(r.on_close(5), None, "a recall after a turn clears it");
        r.after_act("gimbal.turn", turn, 5, false);
        r.after_act("gimbal.stop", Motion::Stop, 6, false);
        assert_eq!(r.on_close(5), None, "its own stop clears it");
        r.after_act("gimbal.turn", turn, 5, false);
        r.after_act("gimbal.turn", turn, 6, false);
        assert_eq!(r.on_close(5), None, "a later turn replaces the record");
        assert_eq!(r.on_close(6).as_deref(), Some("gimbal.stop"));
    }

    #[test]
    fn requests_not_started_within_the_deadline_expire() {
        let q = DeviceQueue::new();
        let t = Instant::now();
        let mut old = set(1, "led.brightness", 1);
        old.arrived = t - Duration::from_secs(16);
        q.push(old).ok().unwrap();
        q.push(set(2, "led.brightness", 2)).ok().unwrap();
        let late = q.expire(t, Duration::from_secs(15));
        assert_eq!(late.iter().map(|j| j.conn).collect::<Vec<_>>(), [1]);
        assert_eq!(q.len(), 1);
    }

    #[test]
    fn put_back_keeps_order_behind_new_stops() {
        let q = DeviceQueue::new();
        q.push(set(1, "a.b", 1)).ok().unwrap();
        q.push(set(1, "c.d", 1)).ok().unwrap();
        let mut jobs = q.take_all();
        let rest = jobs.split_off(1);
        q.push(stop(2)).ok().unwrap();
        q.put_back(rest);
        assert_eq!(names(&q.take_all()), ["gimbal.stop", "set c.d"]);
    }

    #[test]
    fn a_batch_runs_up_to_a_stop_or_a_restart_and_those_run_alone() {
        let restarts =
            |j: &Job| matches!(&j.work, Work::Do { action, .. } if action == "factory_reset");
        let jobs = vec![
            set(1, "a.b", 1),
            act(1, "preset.recall"),
            act(1, "factory_reset"),
            set(1, "c.d", 1),
        ];
        assert_eq!(plan(&jobs, restarts, single).take, 2);
        assert_eq!(plan(&jobs[2..], restarts, single).take, 1);
        assert_eq!(plan(&[stop(1), set(1, "a.b", 1)], restarts, single).take, 1);
        assert_eq!(plan(&[], restarts, single).take, 0);
    }

    /// 4.1 rule 9: a single-leaf `set` is answered without being sent when a later queued `set`
    /// of the same connection has the same single leaf.
    #[test]
    fn single_leaf_sets_of_one_connection_coalesce() {
        let none = |_: &Job| false;
        let jobs = vec![
            set(1, "led.brightness", 1),
            set(2, "led.brightness", 2),
            set(1, "led.brightness", 3),
            job(
                1,
                Work::Set(
                    Patch::new()
                        .leaf("led.brightness", 1)
                        .leaf("zoom.factor", 2),
                ),
            ),
            set(1, "zoom.factor", 2),
        ];
        assert_eq!(
            plan(&jobs, none, single).coalesced,
            [true, false, false, false, false],
            "only an earlier single-leaf set of the same connection and leaf"
        );
        assert_eq!(
            plan(
                &[set(1, "led.brightness", 1), set(1, "led.brightness", 9)],
                none,
                |_| None
            )
            .coalesced,
            [false, false],
            "a later set whose value the setting cannot take does not replace an earlier one"
        );
        let _ = json!(null);
    }

    /// A write in between (here the mode `exposure.time` requires) can make the later request
    /// a `conflict`; the earlier one must then be sent, not answered `ok` unsent.
    #[test]
    fn a_write_in_between_keeps_both_sets() {
        let none = |_: &Job| false;
        let jobs = [
            set(1, "exposure.time", 1000),
            job(1, Work::Set(Patch::new().leaf("exposure.mode", "auto"))),
            set(1, "exposure.time", 2000),
        ];
        assert_eq!(plan(&jobs, none, single).coalesced, [false, false, false]);
        let jobs = [
            set(1, "exposure.time", 1000),
            job(2, Work::Get(None)),
            act(2, "preset.recall"),
            set(1, "exposure.time", 2000),
        ];
        assert_eq!(
            plan(&jobs, none, single).coalesced,
            [false, false, false, false],
            "an action in between keeps both"
        );
        let jobs = [
            set(1, "exposure.time", 1000),
            job(2, Work::Get(None)),
            set(1, "exposure.time", 2000),
        ];
        assert_eq!(
            plan(&jobs, none, single).coalesced,
            [true, false, false],
            "a get in between does not"
        );
    }
}
