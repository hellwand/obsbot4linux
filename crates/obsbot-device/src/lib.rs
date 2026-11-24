// SPDX-License-Identifier: GPL-3.0-or-later

//! Camera access for obsbot4linux (design spec 3.1): the device contract, the transports,
//! the family building blocks, the models and their registry, and the sysfs enumeration.

pub mod bus;
pub mod contract;
pub mod descriptions;
pub mod enumeration;
pub mod error;
pub mod evidence;
pub mod family;
pub mod models;
pub mod registry;
pub mod transport;
