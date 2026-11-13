// SPDX-License-Identifier: GPL-3.0-or-later

//! How the daemon finds devices: the registry types (8.1) and `trait Bus`, the enumeration
//! seam. The real bus (track A) scans sysfs once a second and opens the USB transport; the
//! fake bus (`models::fake::FakeBus`) holds fake devices in memory (5.12). The daemon's
//! registry thread calls `scan` and starts a worker with `open` for every supported device.

use crate::contract::camera::{Camera, Result, Tier};
use obsbot_proto::{Support, Usb};

/// A USB (vendor id, product id) pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct UsbId {
    pub vendor: u16,
    pub product: u16,
}

impl UsbId {
    /// From the four-hex-digit strings of `Usb` (`"3564"`, `"fef9"`).
    pub fn from_usb(usb: &Usb) -> Option<UsbId> {
        let vendor = u16::from_str_radix(&usb.vendor, 16).ok()?;
        let product = u16::from_str_radix(&usb.product, 16).ok()?;
        Some(UsbId { vendor, product })
    }
}

/// Where a model's serial comes from (5.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdentitySource {
    /// The USB serial string, read from sysfs once at plug-in; no request is sent for it.
    UsbSerial,
    /// The model's identity query, sent while no valid serial has been answered.
    Query,
    /// None: the device keeps its `port:` id.
    Absent,
}

/// The registry entry of one model (8.2): data only. The bus that opens a model's devices
/// knows its constructor.
#[derive(Debug)]
pub struct ModelEntry {
    /// `tiny2lite`, `fake-a`, …
    pub key: &'static str,
    /// The display name of 4.2 `name` (`OBSBOT Tiny 2 Lite`).
    pub display_name: &'static str,
    /// Every USB pair this model answers to.
    pub usb: &'static [UsbId],
    pub support: Support,
    /// Where the serial comes from (5.5).
    pub identity: IdentitySource,
    /// Whether the model has a firmware query. Without one, `firmware` is never asked for
    /// and stays "not reported"; with neither query nor `Query` source no attempt is sent.
    pub firmware_query: bool,
    /// The tier whose next two reads carry identity attempts 2 and 3 (5.5).
    pub identity_tier: Tier,
    /// The GUID of the model's extension unit, found in the USB descriptor (8.3); `None` for a
    /// model without one.
    pub xu_guid: Option<&'static str>,
    /// The extension-unit selectors the model uses; every other selector is refused (8.3).
    pub selectors: &'static [u8],
}

/// The vendor table and the registered models (8.1).
#[derive(Debug)]
pub struct Registry {
    pub vendors: &'static [u16],
    pub models: &'static [ModelEntry],
}

/// What the registry says about one USB device.
#[derive(Debug, Clone, Copy)]
pub enum Lookup {
    /// The vendor id is not in the vendor table: the device is not listed at all.
    NotListed,
    /// A known vendor id with an unregistered product id: an unsupported entry (5.4).
    Unsupported,
    Model(&'static ModelEntry),
}

impl Registry {
    /// Matches by (vendor id, product id) only; the product string is never used (5.3).
    pub fn lookup(&self, id: UsbId) -> Lookup {
        if !self.vendors.contains(&id.vendor) {
            return Lookup::NotListed;
        }
        match self.models.iter().find(|m| m.usb.contains(&id)) {
            Some(model) => Lookup::Model(model),
            None => Lookup::Unsupported,
        }
    }

    /// Pairs that belong to more than one model; the registry test requires none (8.1).
    pub fn shared_pairs(&self) -> Vec<UsbId> {
        let mut all: Vec<UsbId> = self
            .models
            .iter()
            .flat_map(|m| m.usb.iter().copied())
            .collect();
        all.sort();
        let mut shared: Vec<UsbId> = all
            .windows(2)
            .filter(|w| w[0] == w[1])
            .map(|w| w[0])
            .collect();
        shared.dedup();
        shared
    }
}

/// One USB device with a video node and a vendor id from the vendor table, as one scan saw it.
#[derive(Debug, Clone)]
pub struct Found {
    pub usb: Usb,
    /// The device number on its bus: with `usb.path` it identifies one enumeration (5.3).
    pub devnum: u32,
    /// `/dev/videoN` of the node with index 0.
    pub video_node: Option<String>,
    /// The registered model, or `None` for an unsupported device.
    pub model: Option<&'static ModelEntry>,
}

/// The enumeration seam of the daemon.
pub trait Bus: Send + Sync {
    /// One enumeration pass. Sends nothing to any device and opens no video node.
    fn scan(&self) -> Vec<Found>;

    /// Opens the camera of a supported device (`found.model` is `Some`). Sends nothing.
    fn open(&self, found: &Found) -> Result<Box<dyn Camera>>;
}
