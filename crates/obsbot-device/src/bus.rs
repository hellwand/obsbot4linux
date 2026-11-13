// SPDX-License-Identifier: GPL-3.0-or-later

//! The real bus (design spec 5.3, 8.1): one sysfs scan per call, each device looked up in the
//! registry by its USB (vendor id, product id), and the constructor of each registered model.
//! It opens no node while it scans; `open` builds the camera and sends nothing either.

use crate::contract::camera::Result;
use crate::contract::{Bus, Camera, CameraError, Found, Lookup, UsbId};
use crate::enumeration::sysfs;
use crate::family::link::{Timing, process_sequence};
use crate::models::tiny2lite::model::Tiny2Lite;
use crate::registry::REGISTRY;
use crate::transport::usb::UsbTransport;
use std::path::PathBuf;

/// The bus of the real devices, with the registry of the registered models ([`REGISTRY`]).
pub struct SysfsBus {
    root: PathBuf,
}

impl SysfsBus {
    /// The bus on `/sys`.
    pub fn new() -> SysfsBus {
        SysfsBus::with_root("/sys")
    }

    /// A bus on another sysfs root (tests).
    pub fn with_root(root: impl Into<PathBuf>) -> SysfsBus {
        SysfsBus { root: root.into() }
    }
}

impl Default for SysfsBus {
    fn default() -> Self {
        SysfsBus::new()
    }
}

impl Bus for SysfsBus {
    fn scan(&self) -> Vec<Found> {
        sysfs::scan(&self.root)
            .into_iter()
            .filter_map(|d| {
                let model = match REGISTRY.lookup(UsbId::from_usb(&d.usb)?) {
                    Lookup::NotListed => return None,
                    Lookup::Unsupported => None,
                    Lookup::Model(entry) => Some(entry),
                };
                Some(Found {
                    usb: d.usb,
                    devnum: d.devnum,
                    video_node: d.video_node,
                    model,
                })
            })
            .collect()
    }

    fn open(&self, found: &Found) -> Result<Box<dyn Camera>> {
        let entry = found
            .model
            .ok_or_else(|| CameraError::Io("an unsupported device is never opened".into()))?;
        let node = found
            .video_node
            .clone()
            .ok_or_else(|| CameraError::Io("the device has no video node with index 0".into()))?;
        let guid = entry.xu_guid.unwrap_or_default();
        let unit = found
            .usb
            .extension_units
            .iter()
            .find(|u| u.guid == guid)
            .map(|u| u.unit)
            .ok_or_else(|| {
                CameraError::Io(format!("no extension unit {guid} in the USB descriptor"))
            })?;
        let usb_dir = self.root.join("bus/usb/devices").join(&found.usb.path);
        let transport = UsbTransport::new(node, unit, usb_dir);
        match entry.key {
            "tiny2lite" => Ok(Box::new(Tiny2Lite::checked(
                transport,
                process_sequence(),
                Timing::CAMERA,
            ))),
            other => Err(CameraError::Io(format!(
                "no constructor for the model {other}"
            ))),
        }
    }
}
