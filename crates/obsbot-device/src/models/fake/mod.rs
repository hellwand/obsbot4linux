// SPDX-License-Identifier: GPL-3.0-or-later

//! The fake models of fake mode (design spec 5.12): second implementations of `trait Camera`,
//! in memory, and the fake bus that plugs, unplugs, sleeps and wakes them.
//!
//! - `fake-a` has the Tiny 2 Lite's description, for client work; `support: tested`.
//! - `fake-b` is deliberately different: other enum values, no `gimbal` and no `views`, the
//!   group `beacon` that no client knows, 5 presets, two `readback: false` leaves, leaves with
//!   `asleep: ignored`, `accepted` and `unknown`, a `requires`, a readback that lags its write
//!   by [`b::LAG`], a readback that rounds down, `support: untested`.
//! - `unknown` has a known vendor id and an unregistered product id: an unsupported device.
//!
//! The fakes send nothing: their send tables are empty and they have no transport.

pub mod a;
pub mod b;
mod bus;
mod camera;

pub use bus::{FakeBus, Op};

use crate::contract::Registry;
use obsbot_proto::Description;
use std::time::Duration;

/// The vendor id of every fake device: a vendor id of the real vendor table, so that
/// `unknown` is unsupported, not ignored (5.3).
pub const VENDOR: u16 = 0x3564;
/// The product id of `unknown`; no fake model registers it.
pub const UNKNOWN_PRODUCT: u16 = 0xfa00;
/// The model keys `obsbot4linuxd --fake` accepts.
pub const KEYS: [&str; 3] = ["fake-a", "fake-b", "unknown"];
/// How long a fake device is gone after a write or action with `warning: restart`.
pub const RESTART: Duration = Duration::from_secs(1);

/// The registry of fake mode: the vendor table and the two fake models.
pub static REGISTRY: Registry = Registry {
    vendors: &[VENDOR],
    models: &[a::ENTRY, b::ENTRY],
};

/// The description of a fake model (`fake-a`, `fake-b`); `None` for any other key.
pub fn description(key: &str) -> Option<Description> {
    match key {
        "fake-a" => Some(a::description()),
        "fake-b" => Some(b::description()),
        _ => None,
    }
}
