// SPDX-License-Identifier: GPL-3.0-or-later

//! The queue of one supported device (4.1 rules 2 and 9, 5.6): at most `MAX_QUEUED` requests,
//! FIFO, except that a stop is taken before every queued request. Connection readers push,
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
        todo!("b-05-queue")
    }

    /// The connection `conn` closed: the stop to queue for it, if its continuing action is the
    /// recorded one. The record is cleared.
    pub fn on_close(&mut self, conn: ConnId) -> Option<String> {
        todo!("b-05-queue")
    }
}

/// Why the queue did not take a job.
pub enum Refused {
    /// `MAX_QUEUED` requests wait already: answer `busy`.
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
        todo!("b-05-queue")
    }

    pub fn is_stop(&self, action: &str) -> bool {
        todo!("b-05-queue")
    }

    /// Queues a job. A stop goes before every queued request that is not a stop, and sets
    /// `cut` so that a group read in progress ends at its next frame boundary.
    pub fn push(&self, job: Job) -> Result<(), Refused> {
        todo!("b-05-queue")
    }

    /// Takes every queued job; the worker plans its batch from them and puts the rest back.
    pub fn take_all(&self) -> Vec<Job> {
        todo!("b-05-queue")
    }

    /// Puts jobs back at the front, in their order (the requests after a failed one, 4.1 rule
    /// 9, or the rest of a batch plan). A stop pushed meanwhile stays in front of them.
    pub fn put_back(&self, jobs: Vec<Job>) {
        todo!("b-05-queue")
    }

    pub fn len(&self) -> usize {
        todo!("b-05-queue")
    }

    /// The connection `conn` closed: its queued requests are dropped (4.1 rule 9), and if its
    /// continuing action is recorded, the model's stop is queued for it (5.6). Returns the stop.
    pub fn close_conn(&self, conn: ConnId, now: Instant) -> Option<String> {
        todo!("b-05-queue")
    }

    /// Updates the continuing-motion record after the camera accepted an action.
    pub fn record_act(&self, action: &str, motion: Motion, conn: ConnId, keep: bool) {
        todo!("b-05-queue")
    }

    /// Removes the jobs not started within `deadline` of their arrival (4.1 rule 9).
    pub fn expire(&self, now: Instant, deadline: Duration) -> Vec<Job> {
        todo!("b-05-queue")
    }

    /// The device became unavailable: every camera operation is cut and the worker ends.
    pub fn end(&self) {
        todo!("b-05-queue")
    }

    /// The daemon stops: polls and identity attempts are cut; the worker sends the stops still
    /// queued and ends (5.9).
    pub fn shutdown(&self) {
        todo!("b-05-queue")
    }

    pub fn is_ended(&self) -> bool {
        todo!("b-05-queue")
    }

    pub fn is_shutdown(&self) -> bool {
        todo!("b-05-queue")
    }

    /// The worker is about to end: no job is taken any more, and the queued ones are returned
    /// to be answered as for an unavailable device.
    pub fn close(&self) -> Vec<Job> {
        todo!("b-05-queue")
    }

    /// Waits until a job is queued, the device ends or the daemon stops, or `until` passes.
    pub fn wait(&self, until: Instant) {
        todo!("b-05-queue")
    }
}

/// Answers the requests the sweeper removed: `camera`, not started within the deadline.
pub fn answer_late(jobs: Vec<Job>) {
    todo!("b-05-queue")
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
    todo!("b-05-queue")
}
