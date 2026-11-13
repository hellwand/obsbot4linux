// SPDX-License-Identifier: GPL-3.0-or-later

//! The registry (design spec 8.1) and the real bus on a sysfs tree written by the test
//! (5.3, 5.4, 8.4): a registered device is supported, a known vendor with an unknown product is
//! unsupported, any other vendor is not listed; opening sends nothing.

mod sysfs_tree;

use obsbot_device::bus::SysfsBus;
use obsbot_device::contract::{Bus, CameraError, Lookup, UsbId};
use obsbot_device::registry::REGISTRY;
use sysfs_tree::*;

#[test]
fn the_registry_holds_the_vendor_table_and_the_tiny_2_lite_only() {
    assert_eq!(REGISTRY.vendors, [0x3564]);
    let keys: Vec<&str> = REGISTRY.models.iter().map(|m| m.key).collect();
    assert_eq!(keys, ["tiny2lite"]);
    assert!(
        REGISTRY.shared_pairs().is_empty(),
        "no pair belongs to two models (8.1)"
    );
    let at = |vendor, product| REGISTRY.lookup(UsbId { vendor, product });
    assert!(matches!(at(0x3564, 0xfef9), Lookup::Model(m) if m.key == "tiny2lite"));
    assert!(matches!(at(0x3564, 0xfa02), Lookup::Unsupported));
    assert!(matches!(at(0xffff, 0xfef9), Lookup::NotListed));
}

#[test]
fn a_scan_sorts_devices_into_supported_unsupported_and_not_listed() {
    let root = root();
    plug(&root, &LITE);
    plug(
        &root,
        &Device {
            path: "3-3",
            product: "fa02",
            nodes: &[(6, 0, 0)],
            ..LITE
        },
    );
    plug(
        &root,
        &Device {
            path: "1-1",
            vendor: "ffff",
            product: "0001",
            nodes: &[(0, 0, 0)],
            ..LITE
        },
    );
    let bus = SysfsBus::with_root(&root);
    let found = bus.scan();
    let seen: Vec<(&str, Option<&str>)> = found
        .iter()
        .map(|f| (f.usb.path.as_str(), f.model.map(|m| m.key)))
        .collect();
    assert_eq!(seen, [("3-2", Some("tiny2lite")), ("3-3", None)]);
    assert_eq!(found[0].devnum, 7);
    assert_eq!(found[0].video_node.as_deref(), Some("/dev/video2"));
}

#[test]
fn opening_builds_the_model_and_sends_nothing() {
    let root = root();
    plug(&root, &LITE);
    let bus = SysfsBus::with_root(&root);
    let found = bus.scan();
    // /dev/video2 does not exist here: opening must not touch it
    let camera = bus.open(&found[0]).unwrap();
    assert_eq!(camera.description().settings.len(), 84);
    assert!(!camera.send_table().entries.is_empty());
}

#[test]
fn an_unsupported_device_or_one_without_the_extension_unit_is_not_opened() {
    let unknown = root();
    plug(
        &unknown,
        &Device {
            product: "fa02",
            ..LITE
        },
    );
    let bus = SysfsBus::with_root(&unknown);
    let found = bus.scan();
    assert!(matches!(bus.open(&found[0]), Err(CameraError::Io(_))));
    let root2 = root();
    plug(&root2, &LITE);
    let lite = SysfsBus::with_root(&root2);
    let mut f = lite.scan().remove(0);
    f.usb.extension_units.clear();
    let r = lite.open(&f);
    assert!(
        matches!(&r, Err(CameraError::Io(m)) if m.contains("9a1e7291")),
        "{:?}",
        r.err()
    );
}
