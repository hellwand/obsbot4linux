// SPDX-License-Identifier: GPL-3.0-or-later

//! The worker of one supported device (5.1, 5.2, 5.5 to 5.7): the only code that touches the
//! device. Written in task b-07-worker.

use crate::Timing;
use crate::daemon::Fatal;
use crate::queue::DeviceQueue;
use crate::table::{PlugId, Table};
use obsbot_device::contract::{Bus, Found, ModelEntry};
use std::io;
use std::sync::Arc;
use std::thread::JoinHandle;

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

/// The worker's whole life: open, first identity attempt and first read, announce, then the
/// loop of batches and ticks until the device becomes unavailable or the daemon stops.
pub fn run(p: Plugged) {
    let _ = p;
    todo!("task b-07-worker")
}
