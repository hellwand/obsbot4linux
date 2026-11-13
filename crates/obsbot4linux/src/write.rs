// SPDX-License-Identifier: GPL-3.0-or-later

//! The writing commands (design spec 6.2). Written in task C-10;
//! this stub fixes the signatures `lib.rs` calls.

use crate::Ctx;
use crate::errors::{CliCode, CliError};
use obsbot_proto::ErrorCode;

/// The parameter that keeps a continuing action going after its connection closes
/// (spec 4.4, `gimbal.turn`).
pub const KEEP: &str = "keep";

pub fn set(ctx: &mut Ctx, assignments: &[String]) -> Result<(), CliError> {
    let _ = (ctx, assignments);
    Err(CliError::new(
        CliCode::Proto(ErrorCode::Internal),
        "set is not built yet (track C)",
    ))
}

pub fn act(ctx: &mut Ctx, action: &str, args: &[String]) -> Result<(), CliError> {
    let _ = (ctx, action, args);
    Err(CliError::new(
        CliCode::Proto(ErrorCode::Internal),
        "do is not built yet (track C)",
    ))
}
