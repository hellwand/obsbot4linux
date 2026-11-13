// SPDX-License-Identifier: GPL-3.0-or-later

//! Control library for the OBSBOT Tiny 2 Lite.

pub mod camera;
pub mod error;
pub mod state;

pub mod family;
pub use family::{frame, streaming};

pub mod device;
pub mod registry;

pub mod commands;
pub mod presets;
pub mod status;
