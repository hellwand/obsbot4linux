// SPDX-License-Identifier: GPL-3.0-or-later

//! The reading commands (design spec 6.2). Written in task C-9;
//! this stub fixes the signatures `lib.rs` calls.

use crate::Ctx;
use crate::errors::{CliCode, CliError};
use obsbot_proto::ErrorCode;

pub fn status(ctx: &mut Ctx) -> Result<(), CliError> {
    let _ = ctx;
    Err(CliError::new(
        CliCode::Proto(ErrorCode::Internal),
        "status is not built yet (track C)",
    ))
}

pub fn get(ctx: &mut Ctx, paths: &[String]) -> Result<(), CliError> {
    let _ = (ctx, paths);
    Err(CliError::new(
        CliCode::Proto(ErrorCode::Internal),
        "get is not built yet (track C)",
    ))
}

pub fn describe(ctx: &mut Ctx, target: Option<&str>) -> Result<(), CliError> {
    let _ = (ctx, target);
    Err(CliError::new(
        CliCode::Proto(ErrorCode::Internal),
        "describe is not built yet (track C)",
    ))
}

pub fn devices(ctx: &mut Ctx, report: bool) -> Result<(), CliError> {
    let _ = (ctx, report);
    Err(CliError::new(
        CliCode::Proto(ErrorCode::Internal),
        "devices is not built yet (track C)",
    ))
}
