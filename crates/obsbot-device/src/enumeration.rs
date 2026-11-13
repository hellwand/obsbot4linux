// SPDX-License-Identifier: GPL-3.0-or-later
// Derived from Tiny4Linux (EUPL-1.2), used under EUPL-1.2 Article 5.

//! Finding a supported camera without talking to it: `/dev/v4l/by-id` links first, the V4L2
//! card name as fallback.

use crate::registry::{self, Model};
use nix::ioctl_read;
use std::fs::{self, File};
use std::os::fd::AsRawFd;
use std::path::PathBuf;

pub mod sysfs;

const V4L2_CAP_META_CAPTURE: u32 = 0x0080_0000;

#[repr(C)]
#[derive(Default)]
struct V4l2Capability {
    driver: [u8; 16],
    card: [u8; 32],
    bus_info: [u8; 32],
    version: u32,
    capabilities: u32,
    device_caps: u32,
    reserved: [u32; 3],
}

ioctl_read!(vidioc_querycap, b'V', 0, V4l2Capability);

/// A camera found by enumeration, before any command is sent to it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Detected {
    pub model: &'static Model,
    /// The V4L2 node to stream from: the stable `/dev/v4l/by-id/...-video-index0` link when
    /// there is one, else `/dev/videoN`.
    pub device: PathBuf,
}

fn card_name(file: &File) -> Option<(String, u32)> {
    let mut cap = V4l2Capability::default();
    unsafe { vidioc_querycap(file.as_raw_fd(), &mut cap) }.ok()?;
    let end = cap
        .card
        .iter()
        .position(|b| *b == 0)
        .unwrap_or(cap.card.len());
    Some((
        String::from_utf8_lossy(&cap.card[..end]).into_owned(),
        cap.device_caps,
    ))
}

/// Finds a supported camera without talking to it: by-id symlink first, V4L2 card name
/// (VIDIOC_QUERYCAP, a read-only ioctl) as fallback.
pub fn detect() -> Option<Detected> {
    let by_id = fs::read_dir("/dev/v4l/by-id")
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .collect();
    if let Some(detected) = first_by_id(by_id) {
        return Some(detected);
    }

    let mut paths: Vec<PathBuf> = fs::read_dir("/dev")
        .ok()?
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .is_some_and(|n| n.to_string_lossy().starts_with("video"))
        })
        .collect();
    paths.sort();
    for path in paths {
        if let Ok(file) = File::open(&path)
            && let Some((card, caps)) = card_name(&file)
            && caps & V4L2_CAP_META_CAPTURE == 0
            && let Some(model) = registry::from_card(&card)
        {
            return Some(Detected {
                model,
                device: path,
            });
        }
    }
    None
}

/// The first supported camera among the by-id links, sorted: `read_dir` order is unspecified,
/// so with two cameras the controlled one could change between runs.
fn first_by_id(mut links: Vec<PathBuf>) -> Option<Detected> {
    links.sort();
    links.into_iter().find_map(|device| {
        let model = registry::from_by_id(&device.file_name()?.to_string_lossy())?;
        Some(Detected { model, device })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn by_id_detection_does_not_depend_on_directory_order() {
        let link = |serial: &str| {
            PathBuf::from(format!(
                "/dev/v4l/by-id/usb-Remo_Tech_Co.__Ltd._OBSBOT_Tiny_2_Lite_{serial}-video-index0"
            ))
        };
        let other = PathBuf::from("/dev/v4l/by-id/usb-Logitech_BRIO-video-index0");
        let picked = first_by_id(vec![other.clone(), link("B"), link("A")]).unwrap();
        assert_eq!(picked.device, link("A"));
        let picked = first_by_id(vec![link("A"), link("B"), other]).unwrap();
        assert_eq!(picked.device, link("A"));
        assert_eq!(first_by_id(Vec::new()), None);
    }
}
