// SPDX-License-Identifier: GPL-3.0-or-later

//! The write path of a worker (4.1 rules 3 to 6 and 9, 5.6). One batch is the requests the
//! queue plan took: each is validated against the state the earlier ones left, what the
//! request names is sent through the model, the groups it touched are re-read once for the
//! whole batch, the resulting state is published, and only then is every request answered.

use crate::outbound::{error, error_line, ok_line};
use crate::pending::{echo, sent};
use crate::queue::{Job, Work};
use crate::table::Cause;
use crate::worker::Worker;
use obsbot_device::contract::{CameraError, Ctx, Target, Tier};
use obsbot_proto::validate::{Refusal, validate_do, validate_get, validate_set};
use obsbot_proto::{ErrorBody, ErrorCode, OkBody, Params, RevOk, RevState};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::time::Instant;

/// What became of one request of a batch.
enum Outcome {
    /// Not answered: its connection closed, or it was answered already (late).
    Skip,
    Get(Option<Vec<String>>),
    Done(Option<Params>),
    Refused(Refusal),
    /// The camera failed midway: the leaves applied before it, and the one that failed.
    Failed {
        applied: Vec<String>,
        failed: String,
        error: CameraError,
    },
}

/// A leaf the camera accepted: its echo, when, and whether its group can be read now.
struct Written {
    leaf: String,
    echo: Value,
    at: Instant,
    readable: bool,
}

impl Worker {
    /// Runs one batch (the plan's `take` requests, in order).
    pub(crate) fn batch(&mut self, jobs: Vec<Job>, coalesced: Vec<bool>) {
        let _ = coalesced;
        let deadline = self.timing.request_deadline;
        // The rev of the last state this worker published: read before anything can unplug the
        // entry, for the answers of a batch the device leaves (5.6).
        let last_rev = self.table.current(self.plug).map_or(0, |(rev, _)| rev);
        let mut working = self.view();
        let mut touched: BTreeSet<String> = BTreeSet::new();
        let mut written: Vec<Written> = Vec::new();
        let mut outcomes: Vec<Outcome> = Vec::new();
        let mut deadlines: Vec<Instant> = Vec::new();
        let mut stopped = None;

        for (i, job) in jobs.iter().enumerate() {
            if job.out.as_ref().is_some_and(|out| out.is_closed()) {
                outcomes.push(Outcome::Skip);
                continue;
            }
            if Instant::now() >= job.arrived + deadline {
                job.fail(ErrorCode::Camera, "the request was not started within 15 s");
                outcomes.push(Outcome::Skip);
                continue;
            }
            match &job.work {
                Work::Get(paths) => {
                    let refused = paths
                        .as_ref()
                        .and_then(|paths| validate_get(&self.desc, paths).err());
                    outcomes.push(match refused {
                        Some(r) => Outcome::Refused(r),
                        None => Outcome::Get(paths.clone()),
                    });
                }
                Work::Set(patch) => {
                    let leaves = match validate_set(&self.desc, &working, patch) {
                        Ok(leaves) => leaves,
                        Err(r) => {
                            outcomes.push(Outcome::Refused(r));
                            continue;
                        }
                    };
                    deadlines.push(job.arrived + deadline);
                    let ctx =
                        Ctx::new(Some(job.arrived + deadline), Some(self.queue.ended.clone()));
                    let named: Vec<&str> = leaves.iter().map(|(leaf, _)| leaf.as_str()).collect();
                    let order = self.cam.order(&named);
                    let values: BTreeMap<&str, &Value> =
                        leaves.iter().map(|(leaf, v)| (leaf.as_str(), v)).collect();
                    let mut applied: Vec<String> = Vec::new();
                    let mut failure = None;
                    for leaf in order {
                        let Some(value) = values.get(leaf.as_str()) else {
                            continue;
                        };
                        let setting = &self.desc.settings[&leaf];
                        let readback = setting.readback;
                        let e = echo(setting, value);
                        match self.cam.apply(&leaf, &sent(setting, value), &ctx) {
                            Ok(()) => {
                                working.set(&leaf, e.clone());
                                touched.extend(self.cam.touched(Target::Leaf(&leaf)));
                                if readback {
                                    let readable = !self.state.asleep()
                                        || self
                                            .plan
                                            .iter()
                                            .find(|g| g.leaves.contains(&leaf))
                                            .is_some_and(|g| g.asleep);
                                    written.push(Written {
                                        leaf: leaf.clone(),
                                        echo: e,
                                        at: Instant::now(),
                                        readable,
                                    });
                                } else {
                                    self.state.set(&leaf, e);
                                }
                                applied.push(leaf);
                            }
                            Err(err) => {
                                self.failed(&err);
                                failure = Some((leaf, err));
                                break;
                            }
                        }
                    }
                    match failure {
                        None => outcomes.push(Outcome::Done(None)),
                        Some((failed, error)) => {
                            outcomes.push(Outcome::Failed {
                                applied,
                                failed,
                                error,
                            });
                            stopped = Some(i);
                            break;
                        }
                    }
                }
                Work::Do { action, params } => {
                    if let Err(r) = validate_do(&self.desc, &working, action, params) {
                        outcomes.push(Outcome::Refused(r));
                        continue;
                    }
                    if let Err(r) = self.cam.check(action, params, &working) {
                        outcomes.push(Outcome::Refused(r));
                        continue;
                    }
                    deadlines.push(job.arrived + deadline);
                    let spec = &self.desc.actions[action];
                    let p: Params = params
                        .iter()
                        .map(|(name, value)| {
                            let value = match spec.params.get(name) {
                                Some(setting) => sent(setting, value),
                                None => value.clone(),
                            };
                            (name.clone(), value)
                        })
                        .collect();
                    let ctx =
                        Ctx::new(Some(job.arrived + deadline), Some(self.queue.ended.clone()));
                    match self.cam.act(action, &p, &working, &ctx) {
                        Ok(result) => {
                            touched.extend(self.cam.touched(Target::Action(action)));
                            outcomes.push(Outcome::Done(result));
                        }
                        Err(err) => {
                            self.failed(&err);
                            outcomes.push(Outcome::Failed {
                                applied: Vec::new(),
                                failed: action.clone(),
                                error: err,
                            });
                            stopped = Some(i);
                            break;
                        }
                    }
                }
            }
        }

        // 4.1 rule 9: the requests after a camera failure go back to the front of the queue.
        let mut jobs = jobs;
        if let Some(i) = stopped {
            let rest = jobs.split_off(i + 1);
            self.queue.put_back(rest);
        }

        // 5.6 "Unavailable": the device went during the batch.
        if self.queue.is_ended() {
            for (job, outcome) in jobs.iter().zip(&outcomes) {
                match outcome {
                    Outcome::Get(_) => self.table.route(&job.device).answer(job),
                    Outcome::Done(result) => job.answer(ok_line(
                        &job.id,
                        OkBody::Rev(RevOk {
                            rev: last_rev,
                            result: result.clone(),
                        }),
                    )),
                    Outcome::Refused(r) => {
                        job.answer(error_line(Some(&job.id), error(r.code.clone(), &r.message)));
                    }
                    Outcome::Failed {
                        applied,
                        failed,
                        error,
                    } => job.answer(error_line(
                        Some(&job.id),
                        failure_body(error, applied, failed, None),
                    )),
                    Outcome::Skip => {}
                }
            }
            return;
        }

        // The groups the batch touched are read once, with the earliest deadline of the
        // requests that sent something.
        let ctx = Ctx::new(
            deadlines.iter().min().copied(),
            Some(self.queue.cut.clone()),
        );
        let mut ok_groups: BTreeSet<String> = BTreeSet::new();
        for group in self.plan.clone() {
            if !touched.contains(&group.name) {
                continue;
            }
            let ok = self.read_group(&group.name, &ctx);
            if ok {
                ok_groups.insert(group.name.clone());
            }
            if group.tier == Tier::Touched {
                self.schedule.touched_read(&group.name, ok);
            }
        }
        for w in written {
            let read = self
                .plan
                .iter()
                .find(|g| g.leaves.contains(&w.leaf))
                .is_some_and(|g| ok_groups.contains(&g.name))
                .then(|| self.state.at(&w.leaf))
                .flatten();
            self.pending.written(
                &self.desc.settings[&w.leaf],
                &w.leaf,
                w.echo,
                read,
                w.readable.then_some(w.at),
            );
        }
        let rev = self.publish(Cause::Request);

        for (job, outcome) in jobs.iter().zip(outcomes) {
            match outcome {
                Outcome::Skip => {}
                Outcome::Get(paths) => {
                    let Some((_, state)) = self.table.current(self.plug) else {
                        self.table.route(&job.device).answer(job);
                        continue;
                    };
                    let state = match paths {
                        Some(paths) => state.select(&paths),
                        None => state,
                    };
                    job.answer(ok_line(&job.id, OkBody::State(RevState { rev, state })));
                }
                Outcome::Done(result) => {
                    job.answer(ok_line(&job.id, OkBody::Rev(RevOk { rev, result })));
                }
                Outcome::Refused(r) => {
                    job.answer(error_line(Some(&job.id), error(r.code.clone(), &r.message)));
                }
                Outcome::Failed {
                    applied,
                    failed,
                    error,
                } => job.answer(error_line(
                    Some(&job.id),
                    failure_body(&error, &applied, &failed, Some(rev)),
                )),
            }
        }
    }
}

/// The `camera` error of a request the camera failed (`internal` for a refused frame, which
/// `Worker::failed` has logged); `rev` is the batch's, absent when the device went.
fn failure_body(
    error: &CameraError,
    applied: &[String],
    failed: &str,
    rev: Option<u64>,
) -> ErrorBody {
    let code = match error {
        CameraError::NotInSendTable(_) => ErrorCode::Internal,
        _ => ErrorCode::Camera,
    };
    ErrorBody {
        rev,
        applied: Some(applied.to_vec()),
        failed: Some(failed.to_string()),
        ..ErrorBody::new(code, &error.to_string())
    }
}
