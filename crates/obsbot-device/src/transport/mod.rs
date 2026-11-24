// SPDX-License-Identifier: GPL-3.0-or-later

//! The transports (design spec 3.1): the USB transport, the replay transport for tests, and
//! the send-table check that wraps a model's transport.

pub mod checked;
pub mod replay;
pub mod usb;
