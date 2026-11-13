// SPDX-License-Identifier: GPL-3.0-or-later

//! The obsbot4linux daemon (design spec 5) as a library, so that tests run it in-process on a
//! temporary socket (section 11). Stage W0 fixes only this entry point; track B writes the
//! daemon behind it.

use obsbot_device::models::fake::FakeBus;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

/// How one daemon runs.
pub struct Config {
    /// The socket to bind when systemd passes none (`LISTEN_FDS`). The lock file is
    /// `<socket>.lock` (5.9); in fake mode the test-control socket is `<socket>.test` (5.12).
    pub socket: PathBuf,
    /// `None`: the real devices (the sysfs scan). `Some`: fake mode with these fake devices
    /// (`obsbot4linuxd --fake fake-a,fake-b,unknown` builds it with `FakeBus::new`).
    pub fake: Option<Arc<FakeBus>>,
    /// Set to `true` to end `run` as on SIGTERM (5.9).
    pub stop: Arc<AtomicBool>,
}

impl Config {
    /// Real devices on `socket`, not stopped.
    pub fn new(socket: PathBuf) -> Config {
        Config {
            socket,
            fake: None,
            stop: Arc::new(AtomicBool::new(false)),
        }
    }
}

/// Runs the daemon until `config.stop` is set or a signal ends it. Returns once the socket,
/// the lock and every worker are released.
pub fn run(config: Config) -> std::io::Result<()> {
    let _ = config;
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "obsbot4linuxd is not built yet (stage W0)",
    ))
}
