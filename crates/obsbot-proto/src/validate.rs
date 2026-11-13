// SPDX-License-Identifier: GPL-3.0-or-later

//! The validator. Written in W0 task 2.

use crate::message::ErrorCode;

/// Why a request is refused. `path` names the leaf, parameter or action at fault.
#[derive(Debug, Clone, PartialEq)]
pub struct Refusal {
    pub code: ErrorCode,
    pub path: Option<String>,
    pub message: String,
}

impl Refusal {
    pub fn new(code: ErrorCode, path: &str, message: &str) -> Refusal {
        Refusal {
            code,
            path: Some(path.to_string()),
            message: message.to_string(),
        }
    }
}
