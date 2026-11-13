// SPDX-License-Identifier: GPL-3.0-or-later

//! A sysfs tree written by a test, laid out as the kernel lays it out:
//! `class/video4linux/videoN` with `index` and a `device` link to the USB interface, whose
//! parent is the USB device; `bus/usb/devices/<path>` links to the USB device.

#![allow(dead_code)]

use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

/// A fresh, empty directory under cargo's temporary directory for tests.
pub fn root() -> PathBuf {
    static N: AtomicU32 = AtomicU32::new(0);
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "sysfs-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

pub fn write(path: &Path, text: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

/// The extension unit descriptor recorded in capture 01, after a video control interface.
pub fn descriptors() -> Vec<u8> {
    let mut d = hex::decode("09040000010e010000").unwrap();
    d.extend(hex::decode("1d24060291721e9a436883466d9239bc7906ee4913010304ffff3f0000").unwrap());
    d
}

pub struct Device<'a> {
    pub path: &'a str,
    pub vendor: &'a str,
    pub product: &'a str,
    pub devnum: u32,
    pub serial: bool,
    /// (interface number, class)
    pub interfaces: &'a [(u8, &'a str)],
    /// (videoN, index, interface number)
    pub nodes: &'a [(u32, u32, u8)],
}

pub fn plug(root: &Path, d: &Device) {
    let dev = root.join("devices/pci0000:00/usb3").join(d.path);
    write(&dev.join("idVendor"), &format!("{}\n", d.vendor));
    write(&dev.join("idProduct"), &format!("{}\n", d.product));
    write(&dev.join("bcdDevice"), "0510\n");
    write(&dev.join("product"), "OBSBOT Tiny 2 Lite\n");
    write(&dev.join("devnum"), &format!("{}\n", d.devnum));
    if d.serial {
        write(&dev.join("serial"), "EXAMPLE0000001\n");
    }
    fs::write(dev.join("descriptors"), descriptors()).unwrap();
    let devices = root.join("bus/usb/devices");
    fs::create_dir_all(&devices).unwrap();
    symlink(&dev, devices.join(d.path)).unwrap();
    for (number, class) in d.interfaces {
        let iface = dev.join(format!("{}:1.{number}", d.path));
        write(&iface.join("bInterfaceNumber"), &format!("{number:02x}\n"));
        write(&iface.join("bInterfaceClass"), &format!("{class}\n"));
    }
    for (n, index, number) in d.nodes {
        let iface = dev.join(format!("{}:1.{number}", d.path));
        let node = iface.join(format!("video4linux/video{n}"));
        write(&node.join("index"), &format!("{index}\n"));
        symlink(&iface, node.join("device")).unwrap();
        let class = root.join("class/video4linux");
        fs::create_dir_all(&class).unwrap();
        symlink(&node, class.join(format!("video{n}"))).unwrap();
    }
}

pub const LITE: Device = Device {
    path: "3-2",
    vendor: "3564",
    product: "FEF9",
    devnum: 7,
    serial: false,
    interfaces: &[(0, "0e"), (1, "0e"), (2, "01"), (3, "01")],
    nodes: &[(2, 0, 0), (3, 1, 0)],
};
