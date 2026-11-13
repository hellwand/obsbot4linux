// SPDX-License-Identifier: GPL-3.0-or-later

//! `watch` (design spec 6.2). Written in task C-11;
//! this stub fixes the signatures `lib.rs` calls.

use crate::Ctx;
use crate::errors::{CliCode, CliError};
use obsbot_proto::ErrorCode;

pub fn watch(ctx: &mut Ctx, live: &[String]) -> Result<(), CliError> {
    let _ = (ctx, live);
    Err(CliError::new(
        CliCode::Proto(ErrorCode::Internal),
        "watch is not built yet (track C)",
    ))
}
