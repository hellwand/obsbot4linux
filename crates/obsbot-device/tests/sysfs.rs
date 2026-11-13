// SPDX-License-Identifier: GPL-3.0-or-later

//! The sysfs scan (design spec 5.3) on a sysfs tree written by the test, laid out as the kernel
//! lays it out: `class/video4linux/videoN` with `index` and a `device` link to the USB
//! interface, whose parent is the USB device. No device node is opened.

mod sysfs_tree;

use obsbot_device::enumeration::sysfs::{UsbDevice, scan};
use std::fs;
use std::os::unix::fs::symlink;
use sysfs_tree::*;

#[test]
fn one_usb_device_with_two_nodes_is_one_entry_with_its_facts() {
    let root = root();
    plug(&root, &LITE);
    let found = scan(&root);
    assert_eq!(found.len(), 1);
    let UsbDevice {
        usb,
        devnum,
        video_node,
        ..
    } = &found[0];
    assert_eq!(*devnum, 7);
    assert_eq!(
        video_node.as_deref(),
        Some("/dev/video2"),
        "the node with index 0"
    );
    assert_eq!(
        (usb.vendor.as_str(), usb.product.as_str()),
        ("3564", "fef9"),
        "lower case"
    );
    assert_eq!(usb.bcd_device, "0510");
    assert_eq!(usb.product_name.as_deref(), Some("OBSBOT Tiny 2 Lite"));
    assert_eq!(usb.path, "3-2");
    assert!(!usb.serial_present);
    assert_eq!(usb.interfaces, ["0e", "0e", "01", "01"]);
    assert_eq!(usb.extension_units.len(), 1);
    assert_eq!(usb.extension_units[0].unit, 2);
    assert_eq!(
        usb.extension_units[0].guid,
        "9a1e7291-6843-4683-6d92-39bc7906ee49"
    );
}

#[test]
fn devices_are_sorted_by_usb_path_and_the_serial_is_only_a_flag() {
    let root = root();
    plug(
        &root,
        &Device {
            path: "3-2.1",
            serial: true,
            nodes: &[(4, 0, 0)],
            ..LITE
        },
    );
    plug(&root, &LITE);
    let found = scan(&root);
    let paths: Vec<&str> = found.iter().map(|d| d.usb.path.as_str()).collect();
    assert_eq!(paths, ["3-2", "3-2.1"]);
    assert!(found[1].usb.serial_present);
    let json = serde_json::to_string(&found[1].usb).unwrap();
    assert!(
        !json.contains("EXAMPLE"),
        "the serial string never enters usb (4.2)"
    );
}

#[test]
fn a_device_without_an_index_0_node_has_no_video_node() {
    let root = root();
    plug(
        &root,
        &Device {
            nodes: &[(5, 1, 0)],
            ..LITE
        },
    );
    assert_eq!(scan(&root)[0].video_node, None);
}

#[test]
fn a_node_whose_device_is_no_usb_device_is_skipped() {
    let root = root();
    plug(&root, &LITE);
    // a platform camera: its device has no idVendor
    let platform = root.join("devices/platform/cam0");
    let node = platform.join("video4linux/video9");
    write(&node.join("index"), "0\n");
    fs::create_dir_all(platform.join("port")).unwrap();
    symlink(platform.join("port"), node.join("device")).unwrap();
    symlink(&node, root.join("class/video4linux/video9")).unwrap();
    let found = scan(&root);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].usb.path, "3-2");
}

#[test]
fn no_video4linux_class_is_no_device() {
    assert!(scan(&root()).is_empty());
}
