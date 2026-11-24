// SPDX-License-Identifier: GPL-3.0-or-later

//! Start-up, the daemon's threads and shutdown (5.1, 5.2, 5.9), and the panic policy: a panic
//! in a worker, the registry thread, the acceptor or the sweeper ends the daemon with an error
//! (`run` returns it, `main` exits non-zero and systemd restarts the service); a panic in a
//! connection thread closes only that connection (`conn`).

use crate::conn::{Conns, Shared, accept_loop};
use crate::registry::{Registry, Spawner};
use crate::socket::{self, Bound};
use crate::table::Table;
use crate::{Config, SWEEP_EVERY, Timing, control, queue, worker};
use obsbot_device::bus::SysfsBus;
use obsbot_device::contract::Bus;
use std::io;
use std::os::unix::net::UnixStream;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

/// The first panic of a thread the daemon cannot run without.
#[derive(Default)]
pub struct Fatal(Mutex<Option<String>>);

impl Fatal {
    pub fn new() -> Arc<Fatal> {
        Arc::new(Fatal::default())
    }

    /// Records that the thread `name` panicked; the first record wins.
    pub fn set(&self, name: &str) {
        let mut slot = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        if slot.is_none() {
            *slot = Some(name.to_string());
        }
    }

    pub fn get(&self) -> Option<String> {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

/// Spawns a thread the daemon cannot run without: a panic in it is recorded in `fatal`.
pub fn critical<F>(name: String, fatal: Arc<Fatal>, f: F) -> io::Result<JoinHandle<()>>
where
    F: FnOnce() + Send + 'static,
{
    std::thread::Builder::new()
        .name(name.clone())
        .spawn(move || {
            if catch_unwind(AssertUnwindSafe(f)).is_err() {
                fatal.set(&name);
            }
        })
}

/// How long shutdown waits for the acceptor, and for the connections to close (5.9).
const ACCEPTOR_JOIN: Duration = Duration::from_secs(1);
const CONNECTIONS_JOIN: Duration = Duration::from_secs(2);
/// How often the main thread looks at `stop` and `fatal`, and how often a join is polled.
const WAIT_STEP: Duration = Duration::from_millis(50);
const JOIN_POLL: Duration = Duration::from_millis(10);

/// Waits until `done` holds or `limit` has passed.
fn wait_until(limit: Duration, done: impl Fn() -> bool) {
    let until = Instant::now() + limit;
    while !done() && Instant::now() < until {
        std::thread::sleep(JOIN_POLL);
    }
}

/// Joins `handle` when it ends within `limit`; a thread that does not end is left behind.
fn join_within(handle: JoinHandle<()>, limit: Duration) {
    wait_until(limit, || handle.is_finished());
    if handle.is_finished() {
        let _ = handle.join();
    }
}

/// The daemon: see `crate::run`.
pub fn run(config: Config, timing: Timing) -> io::Result<()> {
    // The lock first, then the inherited or bound socket (5.9); an error names the lock file.
    let bound = socket::open(&config.socket)?;
    // Fake mode serves the fake devices; without it the daemon scans sysfs (B-2).
    let bus: Arc<dyn Bus> = match config.fake.clone() {
        Some(fake) => fake,
        None => Arc::new(SysfsBus::new()),
    };
    let result = serve(&config, timing, &bound, bus);
    // Only a socket file this daemon bound is removed; an inherited one is systemd's (decision 18).
    if let Some(path) = &bound.owned {
        let _ = std::fs::remove_file(path);
    }
    result
}

/// Everything between the bound socket and its removal: the threads and the shutdown in the
/// order of 5.9. The lock is released when `bound` is dropped by the caller. The test-control
/// socket runs in fake mode only (`config.fake`).
fn serve(config: &Config, timing: Timing, bound: &Bound, bus: Arc<dyn Bus>) -> io::Result<()> {
    let control_stop = Arc::new(AtomicBool::new(false));
    let control = match config.fake.clone() {
        Some(fake) => Some(control::serve(&config.socket, fake, control_stop.clone())?),
        None => None,
    };
    let control_path = control::control_path(&config.socket);

    let table = Arc::new(Table::new(env!("CARGO_PKG_VERSION")));
    let fatal = Fatal::new();
    let spawn_fatal = fatal.clone();
    let spawner: Spawner = Box::new(move |p| worker::spawn(p, spawn_fatal.clone()));
    let mut registry = Registry::new(bus, table.clone(), timing, spawner);
    registry.scan(true);

    let shared = Arc::new(Shared {
        table: table.clone(),
        timing,
        conns: Conns::new(),
    });
    let accept_stop = Arc::new(AtomicBool::new(false));
    let registry_stop = Arc::new(AtomicBool::new(false));
    let sweep_stop = Arc::new(AtomicBool::new(false));

    let started = start_threads(
        bound,
        &shared,
        registry,
        timing,
        &fatal,
        &accept_stop,
        &registry_stop,
        &sweep_stop,
    );
    let (acceptor, registry_thread, sweeper) = match started {
        Ok(threads) => threads,
        Err(e) => {
            // A thread that did start must not outlive the error: wake the acceptor, which
            // may already run, so that it ends too.
            accept_stop.store(true, Ordering::SeqCst);
            let _ = UnixStream::connect(&config.socket);
            registry_stop.store(true, Ordering::SeqCst);
            stop_control(&control_stop, &control_path, control);
            return Err(e);
        }
    };

    while !config.stop.load(Ordering::SeqCst) && fatal.get().is_none() {
        std::thread::sleep(WAIT_STEP);
    }

    // Shutdown (5.9): no new connections, then the open ones close (a connection's guard
    // queues the stop of a continuing action without `keep`), then the workers end, then the
    // sweeper and the test-control socket.
    accept_stop.store(true, Ordering::SeqCst);
    let _ = UnixStream::connect(&config.socket);
    join_within(acceptor, ACCEPTOR_JOIN);
    shared.conns.close_all();
    wait_until(CONNECTIONS_JOIN, || shared.conns.len() == 0);
    registry_stop.store(true, Ordering::SeqCst);
    let _ = registry_thread.join();
    sweep_stop.store(true, Ordering::SeqCst);
    let _ = sweeper.join();
    stop_control(&control_stop, &control_path, control);

    match fatal.get() {
        Some(name) => Err(io::Error::other(format!("the {name} thread panicked"))),
        None => Ok(()),
    }
}

/// Starts the acceptor, the registry thread and the sweeper, each as a thread the daemon
/// cannot run without.
#[allow(clippy::too_many_arguments)]
fn start_threads(
    bound: &Bound,
    shared: &Arc<Shared>,
    registry: Registry,
    timing: Timing,
    fatal: &Arc<Fatal>,
    accept_stop: &Arc<AtomicBool>,
    registry_stop: &Arc<AtomicBool>,
    sweep_stop: &Arc<AtomicBool>,
) -> io::Result<(JoinHandle<()>, JoinHandle<()>, JoinHandle<()>)> {
    let listener = bound.listener.try_clone()?;
    let acceptor = {
        let (shared, stop) = (shared.clone(), accept_stop.clone());
        critical("acceptor".into(), fatal.clone(), move || {
            accept_loop(listener, shared, stop)
        })?
    };
    let registry_thread = {
        let (stop, f) = (registry_stop.clone(), fatal.clone());
        critical("registry".into(), fatal.clone(), move || {
            registry.run(stop, f)
        })?
    };
    let sweeper = {
        let (table, stop) = (shared.table.clone(), sweep_stop.clone());
        critical("sweeper".into(), fatal.clone(), move || {
            while !stop.load(Ordering::SeqCst) {
                for q in table.queues() {
                    queue::answer_late(q.expire(Instant::now(), timing.request_deadline));
                }
                std::thread::sleep(SWEEP_EVERY);
            }
        })?
    };
    Ok((acceptor, registry_thread, sweeper))
}

/// Ends the test-control thread, removes its socket file; a no-op without fake mode.
fn stop_control(stop: &AtomicBool, path: &Path, thread: Option<JoinHandle<()>>) {
    let Some(thread) = thread else { return };
    stop.store(true, Ordering::SeqCst);
    let _ = UnixStream::connect(path);
    join_within(thread, ACCEPTOR_JOIN);
    let _ = std::fs::remove_file(path);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader, Write};

    /// 5.9: a panic in a critical thread is recorded, so that `run` ends with an error.
    #[test]
    fn a_panic_in_a_critical_thread_is_fatal() {
        let fatal = Fatal::new();
        let h = critical("worker 9-1".into(), fatal.clone(), || {
            panic!("a daemon bug")
        })
        .unwrap();
        h.join().unwrap();
        assert_eq!(fatal.get().as_deref(), Some("worker 9-1"));
        critical("registry".into(), fatal.clone(), || panic!("later"))
            .unwrap()
            .join()
            .unwrap();
        assert_eq!(
            fatal.get().as_deref(),
            Some("worker 9-1"),
            "the first one is reported"
        );
        let calm = Fatal::new();
        critical("acceptor".into(), calm.clone(), || ())
            .unwrap()
            .join()
            .unwrap();
        assert_eq!(calm.get(), None);
    }

    /// B-2: without `--fake` the daemon scans sysfs through track A's bus; on a root without
    /// any video device it serves an empty device list and stops cleanly. No real device is
    /// touched: the root is an empty directory.
    #[test]
    fn the_real_path_serves_what_the_sysfs_scan_finds() {
        let dir = std::env::temp_dir().join(format!("o4ld-real-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("sys")).unwrap();
        let socket = dir.join("s.sock");
        // The daemon adds `.lock`; a Unix socket path holds at most 107 bytes.
        assert!(
            socket.as_os_str().len() <= 100,
            "socket path too long for a Unix socket: {}",
            socket.display()
        );
        let config = Config::new(socket.clone());
        assert!(
            config.fake.is_none(),
            "the real path runs without fake mode"
        );
        let stop = config.stop.clone();
        let bus: Arc<dyn Bus> = Arc::new(SysfsBus::with_root(dir.join("sys")));
        let bound = socket::open(&socket).unwrap();
        let h = std::thread::spawn(move || serve(&config, Timing::SPEC, &bound, bus));
        let until = Instant::now() + Duration::from_secs(5);
        while UnixStream::connect(&socket).is_err() {
            assert!(Instant::now() < until, "the daemon did not serve");
            std::thread::sleep(Duration::from_millis(10));
        }
        let mut s = UnixStream::connect(&socket).unwrap();
        writeln!(s, r#"{{"id":1,"hello":{{"protocol":2,"client":"t 1"}}}}"#).unwrap();
        let mut line = String::new();
        BufReader::new(s).read_line(&mut line).unwrap();
        let v: serde_json::Value = serde_json::from_str(&line).unwrap();
        assert_eq!(v["ok"]["devices"], serde_json::json!([]), "{line}");
        stop.store(true, Ordering::SeqCst);
        h.join().unwrap().unwrap();
        assert!(
            !dir.join("s.sock.test").exists(),
            "no test-control socket outside fake mode"
        );
        let _ = std::fs::remove_dir_all(dir);
    }
}
