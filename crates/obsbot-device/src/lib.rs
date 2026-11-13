// SPDX-License-Identifier: GPL-3.0-or-later

//! Control library for the OBSBOT Tiny 2 Lite.

pub mod error;
pub mod legacy;
pub use legacy::state;

pub mod family;
pub use family::{frame, streaming};

pub mod enumeration;
pub mod registry;
pub mod transport;
/// The 0.13 path of the device module, until Task 7 of the groundwork plan removes it.
pub mod device {
    pub use crate::enumeration::{Detected, detect};
    pub use crate::transport::uvc::{CtrlRange, Device};
}

pub mod models;
pub use models::tiny2lite::{commands, presets, status};
