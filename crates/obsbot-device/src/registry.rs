// SPDX-License-Identifier: GPL-3.0-or-later

//! The registry of the real models (design spec 8.1). The types are in `contract::bus`.

use crate::contract::Registry;
use crate::models::tiny2lite;

/// The vendor table and the registered models (design spec 8.1): the OBSBOT vendor id read
/// from a unit in hand (`0x3564`, measured on the Tiny 2 Lite) and the Tiny 2 Lite,
/// `3564:fef9`. Nothing else.
pub static REGISTRY: Registry = Registry {
    vendors: &[0x3564],
    models: &[tiny2lite::plan::ENTRY],
};
