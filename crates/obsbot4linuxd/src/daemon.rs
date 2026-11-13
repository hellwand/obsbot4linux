// SPDX-License-Identifier: GPL-3.0-or-later

//! Start-up, the daemon's threads and shutdown (5.1, 5.2, 5.9), and the panic policy: a panic
//! in a worker, the registry thread, the acceptor or the sweeper ends the daemon with an error
//! (`run` returns it, `main` exits non-zero and systemd restarts the service); a panic in a
//! connection thread closes only that connection (`conn`).

use crate::{Config, Timing};
use std::io;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread::JoinHandle;

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

/// The daemon: see `crate::run`.
pub fn run(config: Config, timing: Timing) -> io::Result<()> {
    let _ = (config, timing);
    todo!("task b-10-daemon")
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
