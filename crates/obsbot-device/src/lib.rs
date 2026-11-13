// SPDX-License-Identifier: GPL-3.0-or-later

//! Control library for the OBSBOT Tiny 2 Lite.

pub mod bus;
pub mod contract;
pub mod descriptions;
pub mod enumeration;
pub mod error;
pub mod family;
pub mod legacy;
pub mod models;
pub mod registry;
#[cfg(test)]
pub(crate) mod scripted;
pub mod transport;
