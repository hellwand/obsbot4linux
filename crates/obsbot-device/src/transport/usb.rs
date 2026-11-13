// SPDX-License-Identifier: GPL-3.0-or-later
// Derived from Tiny4Linux (EUPL-1.2), used under EUPL-1.2 Article 5.

//! The USB transport (design spec 3.1, 5.8): UVC extension-unit queries and V4L2 controls on
//! one camera's video node, and its facts from sysfs. Carried from `transport/uvc.rs`
//! (`Device`), with three changes: the extension unit is the one the registry entry's GUID
//! names in the USB descriptor (no fixed unit), the node is given (no detection by name), and
//! errors are `CameraError`s (a node that is gone is `Gone` at once).
//!
//! The node is opened per operation and closed after it, so streaming applications keep
//! working: control ioctls need no exclusive access. Each operation holds an exclusive `flock`
//! on the node (`NODE_LOCK_WAIT`, 5 s), so that another process that talks to the same camera
//! (a second daemon, the firmware updater) takes turns with this one; [`Transport::hold`] keeps
//! it over a whole exchange. flock is advisory: streaming applications do not take it.

use crate::contract::camera::Result;
use crate::contract::{CameraError, ControlRange, OnAir, Transport};
use crate::enumeration::sysfs;
use crate::family::streaming;
use nix::errno::Errno;
use nix::fcntl::{Flock, FlockArg};
use nix::ioctl_readwrite;
use obsbot_proto::Usb;
use std::fs::{self, File};
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// `NODE_LOCK_WAIT` (5.8): how long an operation waits for another process's lock on the node.
pub const NODE_LOCK_WAIT: Duration = Duration::from_secs(5);

const UVC_SET_CUR: u8 = 0x01;
const UVC_GET_CUR: u8 = 0x81;
const UVC_GET_LEN: u8 = 0x85;
const V4L2_CTRL_FLAG_DISABLED: u32 = 0x0001;

#[repr(C)]
struct UvcXuControlQuery {
    unit: u8,
    selector: u8,
    query: u8,
    size: u16,
    data: *mut u8,
}

#[repr(C)]
#[derive(Default)]
struct V4l2Control {
    id: u32,
    value: i32,
}

#[repr(C)]
#[derive(Default)]
struct V4l2QueryCtrl {
    id: u32,
    kind: u32,
    name: [u8; 32],
    minimum: i32,
    maximum: i32,
    step: i32,
    default_value: i32,
    flags: u32,
    reserved: [u32; 2],
}

ioctl_readwrite!(uvcioc_ctrl_query, b'u', 0x21, UvcXuControlQuery);
ioctl_readwrite!(vidioc_g_ctrl, b'V', 27, V4l2Control);
ioctl_readwrite!(vidioc_s_ctrl, b'V', 28, V4l2Control);
ioctl_readwrite!(vidioc_queryctrl, b'V', 36, V4l2QueryCtrl);

/// The error of a failed system call: a node or device that is gone is `Gone` (5.3: reported
/// at once), anything else `Io` with the errno's text.
pub fn from_errno(e: Errno) -> CameraError {
    match e {
        Errno::ENODEV | Errno::ENOENT | Errno::ENXIO | Errno::ESHUTDOWN => CameraError::Gone,
        other => CameraError::Io(other.desc().to_string()),
    }
}

fn from_io(e: std::io::Error) -> CameraError {
    from_errno(Errno::from_raw(e.raw_os_error().unwrap_or(0)))
}

/// Takes an exclusive flock on `file`, retrying every 20 ms until `wait` has passed; `Locked`
/// then. Carried from `transport/uvc.rs` (`lock_exclusive`).
pub fn lock_exclusive(mut file: File, wait: Duration) -> Result<Flock<File>> {
    let deadline = Instant::now() + wait;
    loop {
        match Flock::lock(file, FlockArg::LockExclusiveNonblock) {
            Ok(lock) => return Ok(lock),
            Err((f, Errno::EWOULDBLOCK)) if Instant::now() < deadline => {
                file = f;
                std::thread::sleep(Duration::from_millis(20));
            }
            Err((_, Errno::EWOULDBLOCK)) => return Err(CameraError::Locked),
            Err((_, e)) => return Err(from_errno(e)),
        }
    }
}

/// The transport of one camera.
pub struct UsbTransport {
    node: PathBuf,
    unit: u8,
    usb_dir: PathBuf,
    held: Option<Flock<File>>,
}

impl UsbTransport {
    /// The camera at `node` (`/dev/videoN`), its extension unit `unit` (found by GUID in the
    /// USB descriptor by the bus) and its USB device directory in sysfs. Opens nothing.
    pub fn new(node: impl Into<PathBuf>, unit: u8, usb_dir: impl Into<PathBuf>) -> UsbTransport {
        UsbTransport {
            node: node.into(),
            unit,
            usb_dir: usb_dir.into(),
            held: None,
        }
    }

    fn open(&self) -> Result<Flock<File>> {
        let file = File::open(&self.node).map_err(from_io)?;
        lock_exclusive(file, NODE_LOCK_WAIT)
    }

    /// Runs `f` on the node under the lock: the held one, or one taken for this call.
    fn with_node<T>(&mut self, f: impl FnOnce(&File) -> Result<T>) -> Result<T> {
        match &self.held {
            Some(file) => f(file),
            None => {
                let file = self.open()?;
                f(&file)
            }
        }
    }

    fn xu(file: &File, unit: u8, selector: u8, query: u8, data: &mut [u8]) -> Result<()> {
        let mut q = UvcXuControlQuery {
            unit,
            selector,
            query,
            size: data.len() as u16,
            data: data.as_mut_ptr(),
        };
        unsafe { uvcioc_ctrl_query(file.as_raw_fd(), &mut q) }.map_err(from_errno)?;
        Ok(())
    }

    fn len(file: &File, unit: u8, selector: u8) -> Result<usize> {
        let mut len = [0u8; 2];
        Self::xu(file, unit, selector, UVC_GET_LEN, &mut len)?;
        Ok(usize::from(u16::from_le_bytes(len)))
    }
}

/// The kernel fails GET_CUR and SET_CUR with ENOBUFS unless the buffer is exactly the
/// selector's length; a write is padded with zeros to it. Carried from `transport/uvc.rs`
/// (`check_len`, `Device::xu_set`).
pub fn padded(selector: u8, len: usize, data: &[u8]) -> Result<Vec<u8>> {
    if data.len() > len {
        return Err(CameraError::Io(format!(
            "{} bytes for selector {selector:#04x}, which takes {len}",
            data.len()
        )));
    }
    let mut buf = vec![0u8; len];
    buf[..data.len()].copy_from_slice(data);
    Ok(buf)
}

impl Transport for UsbTransport {
    fn xu_len(&mut self, selector: u8) -> Result<u16> {
        let unit = self.unit;
        self.with_node(|f| Self::len(f, unit, selector).map(|l| l as u16))
    }

    fn xu_get(&mut self, selector: u8, buf: &mut [u8]) -> Result<()> {
        let unit = self.unit;
        self.with_node(|f| {
            let len = Self::len(f, unit, selector)?;
            if len != buf.len() {
                return Err(CameraError::Io(format!(
                    "selector {selector:#04x} is {len} bytes, not {}",
                    buf.len()
                )));
            }
            Self::xu(f, unit, selector, UVC_GET_CUR, buf)
        })
    }

    fn xu_set(&mut self, selector: u8, data: &[u8]) -> Result<()> {
        let unit = self.unit;
        self.with_node(|f| {
            let len = Self::len(f, unit, selector)?;
            let mut buf = padded(selector, len, data)?;
            Self::xu(f, unit, selector, UVC_SET_CUR, &mut buf)
        })
    }

    fn ctrl_get(&mut self, id: u32) -> Result<i32> {
        self.with_node(|f| {
            let mut c = V4l2Control { id, value: 0 };
            unsafe { vidioc_g_ctrl(f.as_raw_fd(), &mut c) }.map_err(from_errno)?;
            Ok(c.value)
        })
    }

    fn ctrl_set(&mut self, id: u32, value: i32) -> Result<()> {
        self.with_node(|f| {
            let mut c = V4l2Control { id, value };
            unsafe { vidioc_s_ctrl(f.as_raw_fd(), &mut c) }.map_err(from_errno)?;
            Ok(())
        })
    }

    fn ctrl_range(&mut self, id: u32) -> Result<Option<ControlRange>> {
        self.with_node(|f| {
            let mut q = V4l2QueryCtrl {
                id,
                ..Default::default()
            };
            match unsafe { vidioc_queryctrl(f.as_raw_fd(), &mut q) } {
                Ok(_) if q.flags & V4L2_CTRL_FLAG_DISABLED == 0 => Ok(Some(ControlRange {
                    min: q.minimum,
                    max: q.maximum,
                    step: q.step,
                    default: q.default_value,
                })),
                Ok(_) | Err(Errno::EINVAL) => Ok(None),
                Err(e) => Err(from_errno(e)),
            }
        })
    }

    fn usb(&mut self) -> Result<Usb> {
        sysfs::usb_device(&self.usb_dir)
            .map(|(usb, _)| usb)
            .ok_or(CameraError::Gone)
    }

    fn usb_serial(&mut self) -> Result<Option<String>> {
        match fs::read_to_string(self.usb_dir.join("serial")) {
            Ok(s) => Ok(Some(s.trim().to_string())),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                if self.usb_dir.exists() {
                    Ok(None)
                } else {
                    Err(CameraError::Gone)
                }
            }
            Err(e) => Err(from_io(e)),
        }
    }

    fn on_air(&mut self) -> Result<OnAir> {
        if !Path::new(&self.node).exists() {
            return Err(CameraError::Gone);
        }
        // one sample: both taken at the same moment (4.3, audit)
        let active = streaming::is_streaming(&self.node);
        let apps = streaming::apps_using(&self.node);
        Ok(OnAir { active, apps })
    }

    fn hold(&mut self) -> Result<()> {
        if self.held.is_none() {
            self.held = Some(self.open()?);
        }
        Ok(())
    }

    fn release(&mut self) {
        self.held = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    fn temp(name: &str) -> PathBuf {
        static N: AtomicU32 = AtomicU32::new(0);
        std::env::temp_dir().join(format!(
            "o4l-usb-{name}-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ))
    }

    #[test]
    fn a_node_that_is_gone_is_gone_at_once() {
        for e in [Errno::ENODEV, Errno::ENOENT, Errno::ENXIO, Errno::ESHUTDOWN] {
            assert_eq!(from_errno(e), CameraError::Gone, "{e}");
        }
        assert!(matches!(from_errno(Errno::EIO), CameraError::Io(_)));
        assert!(matches!(from_errno(Errno::EACCES), CameraError::Io(_)));
        let mut t = UsbTransport::new("/nonexistent/obsbot-video0", 2, "/nonexistent/usb");
        let start = Instant::now();
        assert_eq!(t.xu_len(2), Err(CameraError::Gone));
        assert_eq!(t.hold(), Err(CameraError::Gone));
        assert_eq!(t.usb(), Err(CameraError::Gone));
        assert_eq!(t.usb_serial(), Err(CameraError::Gone));
        assert_eq!(t.on_air(), Err(CameraError::Gone));
        assert!(
            start.elapsed() < Duration::from_secs(1),
            "no waiting for a gone node"
        );
    }

    #[test]
    fn a_second_holder_waits_and_then_gets_locked() {
        let path = temp("lock");
        File::create(&path).unwrap();
        let mut first = UsbTransport::new(&path, 2, "/nonexistent");
        first.hold().unwrap();
        let busy = lock_exclusive(File::open(&path).unwrap(), Duration::from_millis(100));
        assert!(
            matches!(busy, Err(CameraError::Locked)),
            "a held node is not shared"
        );
        let waiting = {
            let path = path.clone();
            std::thread::spawn(move || lock_exclusive(File::open(&path).unwrap(), NODE_LOCK_WAIT))
        };
        std::thread::sleep(Duration::from_millis(100));
        first.release();
        assert!(
            waiting.join().unwrap().is_ok(),
            "released, the next one gets it"
        );
        fs::remove_file(&path).unwrap();
    }

    #[test]
    fn a_refused_ioctl_is_an_error_not_a_value() {
        // a plain file: the kernel refuses every UVC and V4L2 ioctl on it
        let path = temp("plain");
        File::create(&path).unwrap();
        let mut t = UsbTransport::new(&path, 2, "/nonexistent");
        let mut buf = [0u8; 60];
        assert!(matches!(t.xu_get(2, &mut buf), Err(CameraError::Io(_))));
        assert!(matches!(t.xu_set(2, &[0; 60]), Err(CameraError::Io(_))));
        assert!(matches!(t.ctrl_get(1), Err(CameraError::Io(_))));
        assert!(matches!(t.ctrl_set(1, 0), Err(CameraError::Io(_))));
        assert!(matches!(t.ctrl_range(1), Err(CameraError::Io(_))));
        fs::remove_file(&path).unwrap();
    }

    #[test]
    fn a_write_is_padded_to_the_selector_length_and_never_cut() {
        assert_eq!(
            padded(6, 60, &[0x1a, 0x01, 0x02]).unwrap()[..4],
            [0x1a, 0x01, 0x02, 0]
        );
        assert_eq!(padded(6, 60, &[0x1a, 0x01, 0x02]).unwrap().len(), 60);
        assert_eq!(padded(2, 60, &[7; 60]).unwrap(), vec![7; 60]);
        assert!(matches!(padded(2, 60, &[0; 61]), Err(CameraError::Io(_))));
    }

    #[test]
    fn the_serial_string_is_read_from_sysfs_only_where_it_exists() {
        let dir = temp("usbdir");
        fs::create_dir_all(&dir).unwrap();
        let mut t = UsbTransport::new("/nonexistent", 2, &dir);
        assert_eq!(t.usb_serial(), Ok(None), "no serial string");
        fs::write(dir.join("serial"), "EXAMPLE0000001\n").unwrap();
        assert_eq!(t.usb_serial(), Ok(Some("EXAMPLE0000001".into())));
        fs::remove_dir_all(&dir).unwrap();
    }
}
