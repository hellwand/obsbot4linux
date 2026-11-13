// SPDX-License-Identifier: GPL-3.0-or-later
// Derived from Tiny4Linux (EUPL-1.2), used under EUPL-1.2 Article 5.

//! The pose 0.13 saves when none is given: the remembered gimbal pose, else the one V4L2
//! last set, with the live zoom factor.

use crate::error::Result;
use crate::family::exchange::Camera;
use crate::legacy::state::State;
use crate::models::tiny2lite::presets::{MAX_ZOOM, Pose};

const PAN_ABSOLUTE: u32 = 0x009a_0908;
const TILT_ABSOLUTE: u32 = 0x009a_0909;

impl Camera {
    /// The pose to save: the remembered gimbal pose, else the one V4L2 last set (stale
    /// after tracking moved the gimbal), plus the live zoom factor.
    pub(crate) fn current_pose(&self) -> Result<Pose> {
        let state = State::load().gimbal;
        let v4l2 = |id| self.dev.get_ctrl(id).map(|v| v as f32 / 3600.0);
        let pan = match state.and_then(|g| g.pan) {
            Some(p) => p,
            None => v4l2(PAN_ABSOLUTE)?,
        };
        let tilt = match state.and_then(|g| g.tilt) {
            Some(t) => t,
            None => v4l2(TILT_ABSOLUTE)?,
        };
        Ok(Pose {
            pan,
            tilt,
            zoom: self.zoom_factor()?.clamp(1.0, MAX_ZOOM),
        })
    }
}
