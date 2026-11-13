// SPDX-License-Identifier: GPL-3.0-or-later

//! `obsbot4linuxd`: the daemon binary (design spec 5.12). `--fake <models>` runs it on fake
//! devices; SIGTERM and SIGINT end it with exit code 0 (5.9). Exit codes: 0 after a clean
//! stop, `--help` or `--version`; 2 for a bad command line; 1 for every other error.

use nix::sys::signal::{SigSet, Signal};
use obsbot_device::models::fake::FakeBus;
use obsbot4linuxd::Config;
use obsbot4linuxd::args::{self, Args, USAGE};
use std::process::ExitCode;
use std::sync::Arc;
use std::sync::atomic::Ordering;

fn main() -> ExitCode {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let fake = match args::parse(&argv) {
        Err(e) => {
            eprintln!("obsbot4linuxd: {e}");
            eprintln!("{USAGE}");
            return ExitCode::from(2);
        }
        Ok(Args::Help) => {
            println!("{USAGE}");
            return ExitCode::SUCCESS;
        }
        Ok(Args::Version) => {
            println!("obsbot4linuxd {}", env!("CARGO_PKG_VERSION"));
            return ExitCode::SUCCESS;
        }
        Ok(Args::Run { fake }) => fake,
    };
    let Some(socket) = obsbot_proto::socket_path() else {
        eprintln!("obsbot4linuxd: neither OBSBOT4LINUX_SOCKET nor XDG_RUNTIME_DIR is set");
        return ExitCode::FAILURE;
    };
    let mut config = Config::new(socket);
    if let Some(models) = fake {
        let keys: Vec<&str> = models.iter().map(String::as_str).collect();
        match FakeBus::new(&keys) {
            Ok(bus) => config.fake = Some(Arc::new(bus)),
            Err(e) => {
                eprintln!("obsbot4linuxd: {e}");
                return ExitCode::from(2);
            }
        }
    }

    // Signals are blocked before any thread starts, so that every thread inherits the mask;
    // one thread waits for them and sets the stop flag (no code runs in signal context).
    let mut signals = SigSet::empty();
    signals.add(Signal::SIGTERM);
    signals.add(Signal::SIGINT);
    if let Err(e) = signals.thread_block() {
        eprintln!("obsbot4linuxd: cannot block signals: {e}");
        return ExitCode::FAILURE;
    }
    let stop = config.stop.clone();
    let waiter = std::thread::Builder::new()
        .name("signals".into())
        .spawn(move || {
            if signals.wait().is_ok() {
                stop.store(true, Ordering::SeqCst);
            }
        });
    if let Err(e) = waiter {
        eprintln!("obsbot4linuxd: cannot start the signal thread: {e}");
        return ExitCode::FAILURE;
    }

    match obsbot4linuxd::run(config) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("obsbot4linuxd: {e}");
            ExitCode::FAILURE
        }
    }
}
