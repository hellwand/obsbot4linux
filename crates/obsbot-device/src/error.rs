// SPDX-License-Identifier: GPL-3.0-or-later

//! The error of the byte builders: a value a builder refuses. The models turn it into a
//! `CameraError` before anything is sent.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("invalid value: {0}")]
    Invalid(String),
}

pub type Result<T> = std::result::Result<T, Error>;
