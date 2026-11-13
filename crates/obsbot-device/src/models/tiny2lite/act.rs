// SPDX-License-Identifier: GPL-3.0-or-later

//! The actions of the Tiny 2 Lite and the model's checks. Written in track A task A-11; until
//! then every action is refused before any frame.

use crate::contract::camera::Result;
use crate::contract::{CameraError, Ctx};
use crate::family::link::Link;
use obsbot_proto::validate::Refusal;
use obsbot_proto::{ErrorCode, Params, State};

/// The model's checks before any frame (A-11).
pub fn check(action: &str, _params: &Params, _state: &State) -> std::result::Result<(), Refusal> {
    Err(Refusal::new(
        ErrorCode::Internal,
        action,
        "actions are not built yet",
    ))
}

/// Runs `action` (A-11).
pub fn act(
    _link: &mut Link,
    action: &str,
    _params: &Params,
    _state: &State,
    _ctx: &Ctx,
) -> Result<Option<Params>> {
    Err(CameraError::Io(format!("not sent: no action {action}")))
}
