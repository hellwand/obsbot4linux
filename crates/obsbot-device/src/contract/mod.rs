// SPDX-License-Identifier: GPL-3.0-or-later

//! The contract of the device crate (design spec 8.2, 10.3 W0): `trait Camera`,
//! `trait Transport`, `trait Bus` and the registry types, plus the trait conformance check.
//! These files change only by a contract-change PR (10.1).

pub mod bus;
pub mod camera;
pub mod conformance;
pub mod transport;

pub use bus::{Bus, Found, IdentitySource, Lookup, ModelEntry, Registry, UsbId};
pub use camera::{
    Camera, CameraError, Ctx, Frame, Group, Identity, LiveTier, Motion, Need, SendEntry, SendTable,
    Target, Tier,
};
pub use transport::{ControlRange, OnAir, Transport};
