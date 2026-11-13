// SPDX-License-Identifier: GPL-3.0-or-later

//! Writing one leaf of the Tiny 2 Lite. Written in track A task A-10; until then nothing is
//! written.

use crate::contract::camera::Result;
use crate::contract::{CameraError, Ctx};
use crate::family::link::Link;
use crate::models::tiny2lite::model::Memory;
use serde_json::Value;

/// Writes `leaf` = `value`: the frames of its ledger row (A-10).
pub fn apply(
    _link: &mut Link,
    _memory: &mut Memory,
    leaf: &str,
    _value: &Value,
    _ctx: &Ctx,
) -> Result<()> {
    Err(CameraError::Io(format!("not sent: {leaf} is not writable")))
}
