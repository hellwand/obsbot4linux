// SPDX-License-Identifier: GPL-3.0-or-later

//! What 0.13 keeps around the camera and what only its daemon and CLI use. The rebuild does not
//! carry it over (spec section 3.2); the 0.13 daemon and CLI use it until they are replaced.

pub mod current_pose;
pub mod full_status;
pub mod state;
