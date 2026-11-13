// SPDX-License-Identifier: GPL-3.0-or-later

//! The USB facts of every video device, from sysfs only (design spec 4.2 `usb`, 5.3): no
//! device node is opened, nothing is sent. The attributes read are a choice of track A,
//! confirmed on the camera by CAM-25:
//!
//! - per video node `class/video4linux/videoN`: `index`, and the `device` link to its USB
//!   interface, whose parent directory is the USB device;
//! - per USB device: `idVendor`, `idProduct`, `bcdDevice`, `product` (optional), `serial`
//!   (whether it exists; its text is read only by the transport, for a model whose identity
//!   source it is), `devnum`, the interfaces' `bInterfaceNumber` and `bInterfaceClass`, and
//!   `descriptors` (the raw USB descriptors, for the extension units).

use obsbot_proto::{ExtensionUnit, Usb};
use std::fs;
use std::path::{Path, PathBuf};

/// One USB device with at least one video node.
#[derive(Debug, Clone, PartialEq)]
pub struct UsbDevice {
    pub usb: Usb,
    /// The device number on its bus (5.3: with the path it identifies one enumeration).
    pub devnum: u32,
    /// `/dev/videoN` of the lowest-numbered node with index 0, if any.
    pub video_node: Option<String>,
    /// The USB device's directory in sysfs.
    pub dir: PathBuf,
}

fn attr(dir: &Path, name: &str) -> Option<String> {
    fs::read_to_string(dir.join(name))
        .ok()
        .map(|s| s.trim().to_string())
}

/// A GUID in its usual text form (lower case, 8-4-4-4-12) from the 16 bytes of a USB
/// descriptor: the first three fields are little-endian.
pub fn guid(b: &[u8; 16]) -> String {
    format!(
        "{:08x}-{:04x}-{:04x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        u32::from_le_bytes([b[0], b[1], b[2], b[3]]),
        u16::from_le_bytes([b[4], b[5]]),
        u16::from_le_bytes([b[6], b[7]]),
        b[8],
        b[9],
        b[10],
        b[11],
        b[12],
        b[13],
        b[14],
        b[15]
    )
}

/// The UVC extension units in raw USB descriptors (the sysfs file `descriptors`): every
/// class-specific interface descriptor (type `0x24`) of subtype `0x06` inside a video control
/// interface (class `0x0e`, subclass `0x01`), as unit id, GUID and control count. Subtype
/// `0x06` means something else in an audio interface, so the interface is tracked.
pub fn extension_units(d: &[u8]) -> Vec<ExtensionUnit> {
    let mut units = Vec::new();
    let mut video_control = false;
    let mut i = 0;
    while i + 2 <= d.len() {
        let len = usize::from(d[i]);
        if len < 2 || i + len > d.len() {
            break;
        }
        let desc = &d[i..i + len];
        match desc[1] {
            0x04 if len >= 7 => video_control = desc[5] == 0x0e && desc[6] == 0x01,
            0x24 if video_control && len >= 21 && desc[2] == 0x06 => {
                let g: [u8; 16] = desc[4..20].try_into().expect("16 bytes");
                units.push(ExtensionUnit {
                    unit: desc[3],
                    guid: guid(&g),
                    controls: u32::from(desc[20]),
                });
            }
            _ => {}
        }
        i += len;
    }
    units
}

/// The facts of the USB device at `dir`, read now. `None` when it is not a USB device (no
/// `idVendor`) or is gone.
pub fn usb_device(dir: &Path) -> Option<(Usb, u32)> {
    let name = dir.file_name()?.to_str()?.to_string();
    let hex4 = |a: &str| attr(dir, a).map(|v| v.to_lowercase());
    let vendor = hex4("idVendor")?;
    let product = hex4("idProduct")?;
    let bcd_device = hex4("bcdDevice").unwrap_or_default();
    let devnum = attr(dir, "devnum")?.parse().ok()?;
    let mut interfaces: Vec<(u32, String)> = fs::read_dir(dir)
        .ok()?
        .flatten()
        .filter(|e| {
            e.file_name()
                .to_str()
                .is_some_and(|n| n.starts_with(&format!("{name}:")))
        })
        .filter_map(|e| {
            let p = e.path();
            let number = u32::from_str_radix(&attr(&p, "bInterfaceNumber")?, 16).ok()?;
            Some((number, attr(&p, "bInterfaceClass")?.to_lowercase()))
        })
        .collect();
    interfaces.sort();
    let descriptors = fs::read(dir.join("descriptors")).unwrap_or_default();
    Some((
        Usb {
            vendor,
            product,
            product_name: attr(dir, "product"),
            bcd_device,
            path: name,
            serial_present: dir.join("serial").exists(),
            interfaces: interfaces.into_iter().map(|(_, c)| c).collect(),
            extension_units: extension_units(&descriptors),
        },
        devnum,
    ))
}

/// Every USB device with a video node under `root` (normally `/sys`), sorted by USB path. One
/// device is one entry, whatever number of video nodes it has (5.3).
pub fn scan(root: &Path) -> Vec<UsbDevice> {
    let Ok(nodes) = fs::read_dir(root.join("class/video4linux")) else {
        return Vec::new();
    };
    let mut nodes: Vec<(u32, PathBuf)> = nodes
        .flatten()
        .filter_map(|e| {
            let n = e
                .file_name()
                .to_str()?
                .strip_prefix("video")?
                .parse()
                .ok()?;
            Some((n, e.path()))
        })
        .collect();
    nodes.sort();
    let mut devices: Vec<UsbDevice> = Vec::new();
    for (n, node) in nodes {
        let Some(dir) = fs::canonicalize(node.join("device"))
            .ok()
            .and_then(|iface| iface.parent().map(Path::to_path_buf))
        else {
            continue;
        };
        let index0 = attr(&node, "index").as_deref() == Some("0");
        if let Some(known) = devices.iter_mut().find(|d| d.dir == dir) {
            if index0 && known.video_node.is_none() {
                known.video_node = Some(format!("/dev/video{n}"));
            }
            continue;
        }
        let Some((usb, devnum)) = usb_device(&dir) else {
            continue;
        };
        devices.push(UsbDevice {
            usb,
            devnum,
            video_node: index0.then(|| format!("/dev/video{n}")),
            dir,
        });
    }
    devices.sort_by(|a, b| a.usb.path.cmp(&b.usb.path));
    devices
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The extension unit descriptor recorded in capture 01: unit 2, 19 controls.
    const XU: &str = "1d24060291721e9a436883466d9239bc7906ee4913010304ffff3f0000";

    #[test]
    fn the_recorded_extension_unit_is_found_by_its_guid() {
        // a video control interface (class 0e, subclass 01), then the unit
        let mut d = hex::decode("09040000010e010000").unwrap();
        d.extend(hex::decode(XU).unwrap());
        assert_eq!(
            extension_units(&d),
            [ExtensionUnit {
                unit: 2,
                guid: "9a1e7291-6843-4683-6d92-39bc7906ee49".into(),
                controls: 19,
            }]
        );
    }

    #[test]
    fn subtype_6_outside_video_control_is_no_extension_unit() {
        // the same bytes after an audio control interface (class 01, subclass 01)
        let mut d = hex::decode("090403000001010000").unwrap();
        d.extend(hex::decode(XU).unwrap());
        assert!(extension_units(&d).is_empty());
        // a truncated descriptor ends the walk without a panic
        let d = hex::decode("09040000010e0100001d2406").unwrap();
        assert!(extension_units(&d).is_empty());
        assert!(
            extension_units(&[0, 0, 0]).is_empty(),
            "a zero length ends the walk"
        );
    }

    #[test]
    fn guids_read_their_first_three_fields_little_endian() {
        let b: [u8; 16] = std::array::from_fn(|i| i as u8);
        assert_eq!(guid(&b), "03020100-0504-0706-0809-0a0b0c0d0e0f");
    }
}
