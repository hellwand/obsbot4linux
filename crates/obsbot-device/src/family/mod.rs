// SPDX-License-Identifier: GPL-3.0-or-later

//! Building blocks that are not tied to one camera model: the frame codec, the link over a
//! transport (answer matching, short commands, queries) and the on-air probe.

pub mod frame;
pub mod link;
pub mod streaming;
