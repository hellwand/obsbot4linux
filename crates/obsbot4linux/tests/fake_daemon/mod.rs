// SPDX-License-Identifier: GPL-3.0-or-later

//! An in-process `obsbot4linuxd --fake fake-a,fake-b,unknown` on a temporary socket (design
//! spec 5.12 and 11; track B's milestone B-1), the camera operations its fakes received, and
//! snapshot files. A test file uses it with `mod common; mod fake_daemon;`.

#![allow(dead_code)]

use obsbot_device::models::fake::{FakeBus, Op};
use obsbot4linuxd::Config;
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

/// A running fake daemon. Dropping it stops it.
pub struct FakeDaemon {
    pub socket: PathBuf,
    pub bus: Arc<FakeBus>,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<std::io::Result<()>>>,
}

impl FakeDaemon {
    /// Starts the daemon with `fake-a`, `fake-b` and `unknown` and waits (at most 20 s) until
    /// it accepts connections.
    pub fn start(name: &str) -> FakeDaemon {
        let socket = crate::common::socket(&format!("e-{name}"));
        let _ = std::fs::remove_file(&socket);
        let bus = Arc::new(FakeBus::new(&["fake-a", "fake-b", "unknown"]).expect("the fakes"));
        let mut config = Config::new(socket.clone());
        config.fake = Some(Arc::clone(&bus));
        let stop = Arc::clone(&config.stop);
        let thread = thread::spawn(move || obsbot4linuxd::run(config));
        let until = Instant::now() + Duration::from_secs(20);
        while UnixStream::connect(&socket).is_err() {
            if thread.is_finished() {
                panic!("the fake daemon ended: {:?}", thread.join());
            }
            assert!(
                Instant::now() < until,
                "the fake daemon does not accept on {}",
                socket.display()
            );
            thread::sleep(Duration::from_millis(20));
        }
        FakeDaemon {
            socket,
            bus,
            stop,
            thread: Some(thread),
        }
    }

    /// Runs the command line in-process against this daemon.
    pub fn cli(&self, args: &[&str]) -> (u8, String, String) {
        crate::common::cli(Some(&self.socket), args)
    }

    /// The number of camera operations the device with this id has received.
    pub fn op_count(&self, id: &str) -> usize {
        self.bus.ops(id).len()
    }

    /// The writes (`Apply`, `Act`) the device with this id received after its first `from`
    /// operations. Reads, identity attempts and opens are left out.
    pub fn writes_since(&self, id: &str, from: usize) -> Vec<Op> {
        self.bus
            .ops(id)
            .into_iter()
            .skip(from)
            .filter(|op| matches!(op, Op::Apply(..) | Op::Act(..)))
            .collect()
    }

    /// Stops the daemon and waits for `run` to return.
    pub fn stop(mut self) {
        self.halt();
    }

    fn halt(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(thread) = self.thread.take() {
            let ended = thread.join().expect("the daemon thread does not panic");
            assert!(ended.is_ok(), "run returned {ended:?}");
        }
    }
}

impl Drop for FakeDaemon {
    fn drop(&mut self) {
        if self.thread.is_some() && !thread::panicking() {
            self.halt();
        }
    }
}

/// Compares `text` with `tests/snapshots/<name>`; `OBSBOT_REGENERATE=1` writes the file.
/// A new or changed snapshot is reviewed in the pull request like code.
pub fn snapshot(name: &str, text: &str) {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/snapshots")
        .join(name);
    if std::env::var_os("OBSBOT_REGENERATE").is_some() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, text).unwrap();
        return;
    }
    let have = std::fs::read_to_string(&path).unwrap_or_default();
    assert!(
        have == text,
        "tests/snapshots/{name} differs (OBSBOT_REGENERATE=1 rewrites it; review the diff):\n{text}"
    );
}

/// Waits (at most 10 s) until `check` holds.
pub fn eventually(what: &str, mut check: impl FnMut() -> bool) {
    let until = Instant::now() + Duration::from_secs(10);
    while !check() {
        assert!(Instant::now() < until, "timed out waiting for {what}");
        thread::sleep(Duration::from_millis(50));
    }
}
