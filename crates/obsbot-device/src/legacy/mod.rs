// SPDX-License-Identifier: GPL-3.0-or-later

//! What 0.13 keeps around the camera. The rebuild does not carry it over (spec section 3.2); the
//! carried Tiny 2 Lite flows use `state` and `current_pose` until track A rewrites them.

pub mod current_pose;
pub mod state;
