// SPDX-License-Identifier: GPL-3.0-or-later

//! The registry thread (5.3): one scan a second through `trait Bus`, plug and unplug.
//! Written in task b-09-registry.

use crate::Timing;
use crate::daemon::Fatal;
use crate::hotplug::{self, Seen};
use crate::names;
use crate::queue::DeviceQueue;
use crate::table::{PlugId, Table};
use crate::worker::Plugged;
use obsbot_device::contract::Bus;
use obsbot_proto::Description;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

/// The longest sleep between two looks at `stop` (5.3, 5.9).
const STEP: Duration = Duration::from_millis(50);
/// How often the shutdown looks at the workers.
const JOIN_POLL: Duration = Duration::from_millis(10);

/// Starts the worker of a plugged supported device.
pub type Spawner = Box<dyn Fn(Plugged) -> std::io::Result<JoinHandle<()>> + Send>;

struct Known {
    seen: Seen,
    plug: PlugId,
    queue: Option<Arc<DeviceQueue>>,
}

pub struct Registry {
    bus: Arc<dyn Bus>,
    table: Arc<Table>,
    timing: Timing,
    spawn: Spawner,
    known: Vec<Known>,
    next: PlugId,
    workers: Vec<(PlugId, JoinHandle<()>)>,
}

impl Registry {
    pub fn new(bus: Arc<dyn Bus>, table: Arc<Table>, timing: Timing, spawn: Spawner) -> Registry {
        Registry {
            bus,
            table,
            timing,
            spawn,
            known: Vec::new(),
            next: 1,
            workers: Vec::new(),
        }
    }

    /// One enumeration pass; returns the plugs of the supported devices it added. With
    /// `startup`, each of them joins the set every `hello` waits for (5.2), before its worker
    /// starts.
    pub fn scan(&mut self, startup: bool) -> Vec<PlugId> {
        let found = self.bus.scan();
        let seens: Vec<Seen> = self.known.iter().map(|k| k.seen.clone()).collect();
        let d = hotplug::diff(&seens, &found);
        // A removal is its own `devices` event before any event that adds an entry under the
        // same id (4.1 Events).
        for &i in d.gone.iter().rev() {
            let k = self.known.remove(i);
            self.table.unplug(k.plug);
            if let Some(q) = k.queue {
                q.end();
            }
        }
        let mut added = Vec::new();
        for &i in &d.new {
            let plug = self.next;
            self.next += 1;
            let f = found[i].clone();
            let seen = Seen::of(&f);
            let Some(model) = f.model else {
                self.table
                    .plug_unsupported(plug, names::unsupported_device(&f));
                self.known.push(Known {
                    seen,
                    plug,
                    queue: None,
                });
                continue;
            };
            let q = DeviceQueue::new();
            let device = names::supported_device(
                &f,
                model,
                Description::default(),
                &obsbot_proto::port_id(&f.usb.path),
                None,
                None,
            );
            self.table.plug_hidden(plug, model, device, q.clone());
            if startup {
                self.table.startup(&[plug]);
            }
            let path = f.usb.path.clone();
            let plugged = Plugged {
                plug,
                found: f,
                model,
                bus: self.bus.clone(),
                table: self.table.clone(),
                queue: q.clone(),
                timing: self.timing,
            };
            match (self.spawn)(plugged) {
                Ok(h) => self.workers.push((plug, h)),
                Err(e) => {
                    eprintln!("obsbot4linuxd: cannot start the worker of {path}: {e}");
                    self.table.unplug(plug);
                }
            }
            self.known.push(Known {
                seen,
                plug,
                queue: Some(q),
            });
            added.push(plug);
        }
        added
    }

    /// Scans every `timing.scan` until `stop` is set, then shuts the workers down and joins
    /// them, at most `timing.request_deadline` long.
    pub fn run(mut self, stop: Arc<AtomicBool>, fatal: Arc<Fatal>) {
        let _ = fatal;
        'scanning: loop {
            let until = Instant::now() + self.timing.scan;
            loop {
                if stop.load(Ordering::SeqCst) {
                    break 'scanning;
                }
                let left = until.saturating_duration_since(Instant::now());
                if left.is_zero() {
                    break;
                }
                std::thread::sleep(left.min(STEP));
            }
            self.scan(false);
            self.workers.retain(|(_, h)| !h.is_finished());
        }
        for k in &self.known {
            if let Some(q) = &k.queue {
                q.shutdown();
            }
        }
        let limit = Instant::now() + self.timing.request_deadline + Duration::from_secs(1);
        while Instant::now() < limit && self.workers.iter().any(|(_, h)| !h.is_finished()) {
            std::thread::sleep(JOIN_POLL);
        }
        for (_, h) in self.workers {
            if h.is_finished() {
                let _ = h.join();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use obsbot_device::models::fake::{FakeBus, KEYS};
    use std::sync::Mutex;
    use std::time::Duration;

    type Started = Arc<Mutex<Vec<(PlugId, Arc<DeviceQueue>)>>>;

    fn registry(bus: Arc<FakeBus>, table: Arc<Table>) -> (Registry, Started) {
        let started = Arc::new(Mutex::new(Vec::new()));
        let seen = started.clone();
        let spawn: Spawner = Box::new(move |p: Plugged| {
            seen.lock().unwrap().push((p.plug, p.queue.clone()));
            Ok(std::thread::spawn(|| ()))
        });
        let bus: Arc<dyn Bus> = bus;
        let timing = crate::Timing {
            scan: Duration::from_millis(100),
            ..crate::Timing::SPEC
        };
        (Registry::new(bus, table, timing, spawn), started)
    }

    #[test]
    fn the_first_scan_lists_the_unsupported_device_and_starts_two_workers() {
        let bus = Arc::new(FakeBus::new(&KEYS).unwrap());
        let table = Arc::new(Table::new("t"));
        let (mut reg, started) = registry(bus.clone(), table.clone());
        assert_eq!(reg.scan(true), [1, 2]);
        assert_eq!(
            started
                .lock()
                .unwrap()
                .iter()
                .map(|s| s.0)
                .collect::<Vec<_>>(),
            [1, 2]
        );
        let listed: Vec<String> = table.devices().iter().map(|d| d.id.clone()).collect();
        assert_eq!(
            listed,
            ["port:9-3"],
            "supported devices wait for their workers"
        );
        assert_eq!(
            reg.scan(false),
            Vec::<PlugId>::new(),
            "an unchanged scan does nothing"
        );
        assert!(bus.ops("port:9-3").is_empty());
    }

    #[test]
    fn an_unplug_ends_the_worker_and_a_new_device_number_starts_a_new_one() {
        let bus = Arc::new(FakeBus::new(&KEYS).unwrap());
        let table = Arc::new(Table::new("t"));
        let (mut reg, started) = registry(bus.clone(), table.clone());
        reg.scan(true);
        let first = started.lock().unwrap()[0].1.clone();
        bus.unplug("sn:FAKEA0000001").unwrap();
        bus.plug("fake-a").unwrap();
        assert_eq!(reg.scan(false), [4], "a new plug for the new enumeration");
        assert!(first.is_ended(), "the old worker is told to end");
        bus.unplug("port:9-3").unwrap();
        reg.scan(false);
        assert!(
            table.devices().is_empty(),
            "the unsupported entry is removed"
        );
    }

    #[test]
    fn run_ends_when_stopped_and_shuts_the_workers_down() {
        let bus = Arc::new(FakeBus::new(&KEYS).unwrap());
        let table = Arc::new(Table::new("t"));
        let (mut reg, started) = registry(bus, table);
        reg.scan(true);
        let stop = Arc::new(AtomicBool::new(true));
        reg.run(stop, crate::daemon::Fatal::new());
        assert!(started.lock().unwrap().iter().all(|(_, q)| q.is_shutdown()));
    }
}
