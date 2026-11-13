// SPDX-License-Identifier: GPL-3.0-or-later

//! `trait Transport` (design spec 3.1): extension-unit get, set and length by selector, UVC
//! control ioctls, and the facts read from sysfs. One implementation talks to USB UVC devices
//! (track A); a replay implementation answers from recorded exchanges (track A). A model owns
//! one transport; the daemon never sees it.

use crate::contract::camera::Result;
use obsbot_proto::Usb;

/// Range and default of a UVC control, as the camera reports them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ControlRange {
    pub min: i32,
    pub max: i32,
    pub step: i32,
    pub default: i32,
}

/// One sample of the on-air probe (4.3 `on_air`): taken together.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OnAir {
    pub active: bool,
    /// Names of processes that hold the video node open, best effort.
    pub apps: Vec<String>,
}

pub trait Transport: Send {
    /// GET_LEN of an extension-unit selector.
    fn xu_len(&mut self, selector: u8) -> Result<u16>;
    /// GET_CUR of an extension-unit selector into `buf` (its length is the selector's length).
    fn xu_get(&mut self, selector: u8, buf: &mut [u8]) -> Result<()>;
    /// SET_CUR of an extension-unit selector. The wrapper checks the frame first (8.3).
    fn xu_set(&mut self, selector: u8, data: &[u8]) -> Result<()>;
    /// VIDIOC_G_CTRL.
    fn ctrl_get(&mut self, id: u32) -> Result<i32>;
    /// VIDIOC_S_CTRL. The wrapper checks the control against the send table first.
    fn ctrl_set(&mut self, id: u32, value: i32) -> Result<()>;
    /// VIDIOC_QUERYCTRL; `None` when the camera does not have the control.
    fn ctrl_range(&mut self, id: u32) -> Result<Option<ControlRange>>;
    /// The USB facts, read from sysfs now (the audio interface of 4.3 `audio.enabled` comes
    /// and goes with the camera's restarts).
    fn usb(&mut self) -> Result<Usb>;
    /// The USB serial string from sysfs, for a model whose identity source it is. Never logged.
    fn usb_serial(&mut self) -> Result<Option<String>>;
    /// One sample of the on-air probe on the video node with index 0.
    fn on_air(&mut self) -> Result<OnAir>;
    /// Takes the per-operation lock on the video node (5.8 `NODE_LOCK_WAIT`) and keeps it
    /// until [`Transport::release`], so that one exchange (a command, or a query and the
    /// reads of its answer) is not split by another process that talks to the same camera.
    /// `Err(Locked)` when the lock stayed taken for `NODE_LOCK_WAIT`. A transport without
    /// a lock (replay, fakes) keeps the default, which does nothing.
    fn hold(&mut self) -> Result<()> {
        Ok(())
    }
    /// Gives the lock of [`Transport::hold`] back. Calling it without a hold does nothing.
    fn release(&mut self) {}
}
