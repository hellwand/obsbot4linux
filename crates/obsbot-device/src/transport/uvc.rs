// SPDX-License-Identifier: GPL-3.0-or-later
// Derived from Tiny4Linux (EUPL-1.2), used under EUPL-1.2 Article 5.

//! Device lookup and ioctls: UVC extension-unit queries and V4L2 controls.
//!
//! The device is opened per operation and closed on drop, so streaming applications keep
//! working: control ioctls do not need exclusive access. Each handle holds an exclusive flock
//! on the node, so two obsbot4linux processes (the daemon and a `--direct` call, or two direct
//! calls) take turns: the vendor unit has one answer buffer, and answers are matched by sequence
//! number only. flock is advisory; streaming applications do not take it and are not held up.

use crate::enumeration::{Detected, detect};
use crate::error::{Error, Result};
use nix::errno::Errno;
use nix::fcntl::{Flock, FlockArg};
use nix::ioctl_readwrite;
use std::fs::File;
use std::os::fd::AsRawFd;
use std::time::{Duration, Instant};

/// How long `Device::open` waits for another obsbot4linux process to release the camera.
const LOCK_WAIT: Duration = Duration::from_secs(5);

const UVC_SET_CUR: u8 = 0x01;
const UVC_GET_CUR: u8 = 0x81;
const UVC_GET_LEN: u8 = 0x85;

/// Size of every vendor buffer on unit 2.
pub const XU_SIZE: usize = 60;

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

/// Range and default of a V4L2 control.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize)]
pub struct CtrlRange {
    pub min: i32,
    pub max: i32,
    pub step: i32,
    pub default: i32,
}

pub struct Device {
    file: Flock<File>,
    detected: Detected,
}

/// Takes an exclusive flock on `file`, retrying until `wait` has passed.
fn lock_exclusive(mut file: File, wait: Duration) -> Result<Flock<File>> {
    let deadline = Instant::now() + wait;
    loop {
        match Flock::lock(file, FlockArg::LockExclusiveNonblock) {
            Ok(lock) => return Ok(lock),
            Err((f, Errno::EWOULDBLOCK)) if Instant::now() < deadline => {
                file = f;
                std::thread::sleep(Duration::from_millis(20));
            }
            Err((_, Errno::EWOULDBLOCK)) => return Err(Error::Busy),
            Err((_, e)) => return Err(Error::Io(e.into())),
        }
    }
}

/// The kernel fails GET_CUR and SET_CUR with ENOBUFS unless the buffer is exactly the
/// selector's length, and both always pass [`XU_SIZE`] bytes.
fn check_len(selector: u8, len: usize) -> Result<()> {
    if len == XU_SIZE {
        Ok(())
    } else {
        Err(Error::Unsupported(format!(
            "selector {selector:#x} is {len} bytes"
        )))
    }
}

impl Device {
    /// Opens the first supported camera.
    pub fn open() -> Result<Self> {
        let detected = detect().ok_or(Error::NoCamera)?;
        let file = lock_exclusive(File::open(&detected.device)?, LOCK_WAIT)?;
        Ok(Self { file, detected })
    }

    pub fn detected(&self) -> &Detected {
        &self.detected
    }

    fn xu(&self, selector: u8, query: u8, data: &mut [u8]) -> Result<()> {
        let mut q = UvcXuControlQuery {
            unit: 2,
            selector,
            query,
            size: data.len() as u16,
            data: data.as_mut_ptr(),
        };
        unsafe { uvcioc_ctrl_query(self.file.as_raw_fd(), &mut q) }?;
        Ok(())
    }

    fn xu_len(&self, selector: u8) -> Result<usize> {
        let mut len = [0u8; 2];
        self.xu(selector, UVC_GET_LEN, &mut len)?;
        Ok(u16::from_le_bytes(len) as usize)
    }

    /// GET_CUR on unit 2, after checking the buffer length like the vendor tools do.
    pub fn xu_get(&self, selector: u8) -> Result<[u8; XU_SIZE]> {
        check_len(selector, self.xu_len(selector)?)?;
        let mut data = [0u8; XU_SIZE];
        self.xu(selector, UVC_GET_CUR, &mut data)?;
        Ok(data)
    }

    /// SET_CUR on unit 2 with a zero-padded 60-byte buffer.
    pub fn xu_set(&self, selector: u8, payload: &[u8]) -> Result<()> {
        if payload.len() > XU_SIZE {
            return Err(Error::Invalid("payload longer than 60 bytes".into()));
        }
        check_len(selector, self.xu_len(selector)?)?;
        let mut data = [0u8; XU_SIZE];
        data[..payload.len()].copy_from_slice(payload);
        self.xu(selector, UVC_SET_CUR, &mut data)
    }

    pub fn get_ctrl(&self, id: u32) -> Result<i32> {
        let mut c = V4l2Control { id, value: 0 };
        unsafe { vidioc_g_ctrl(self.file.as_raw_fd(), &mut c) }?;
        Ok(c.value)
    }

    pub fn set_ctrl(&self, id: u32, value: i32) -> Result<()> {
        let mut c = V4l2Control { id, value };
        unsafe { vidioc_s_ctrl(self.file.as_raw_fd(), &mut c) }?;
        Ok(())
    }

    /// Range of a control, or None when the camera does not have it (or it is disabled).
    pub fn ctrl_range(&self, id: u32) -> Result<Option<CtrlRange>> {
        let mut q = V4l2QueryCtrl {
            id,
            ..Default::default()
        };
        match unsafe { vidioc_queryctrl(self.file.as_raw_fd(), &mut q) } {
            Ok(_) if q.flags & V4L2_CTRL_FLAG_DISABLED == 0 => Ok(Some(CtrlRange {
                min: q.minimum,
                max: q.maximum,
                step: q.step,
                default: q.default_value,
            })),
            Ok(_) | Err(nix::errno::Errno::EINVAL) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn a_second_handle_waits_for_the_first() {
        // the daemon and a --direct call must not exchange frames at the same time
        let path = std::env::temp_dir().join(format!("o4l-lock-{}", std::process::id()));
        File::create(&path).unwrap();
        let first = lock_exclusive(File::open(&path).unwrap(), LOCK_WAIT).unwrap();
        let busy = lock_exclusive(File::open(&path).unwrap(), Duration::from_millis(100));
        assert!(
            matches!(busy, Err(Error::Busy)),
            "a held camera is not shared"
        );
        let waiting = {
            let path = path.clone();
            std::thread::spawn(move || lock_exclusive(File::open(&path).unwrap(), LOCK_WAIT))
        };
        std::thread::sleep(Duration::from_millis(100));
        drop(first);
        assert!(
            waiting.join().unwrap().is_ok(),
            "released on drop, the next one gets it"
        );
        fs::remove_file(&path).unwrap();
    }

    #[test]
    fn only_the_exact_buffer_length_is_accepted() {
        // a shorter or longer selector would fail in the kernel with a bare ENOBUFS
        assert!(check_len(2, XU_SIZE).is_ok());
        assert!(matches!(check_len(2, 32), Err(Error::Unsupported(_))));
        assert!(matches!(check_len(2, 64), Err(Error::Unsupported(_))));
    }
}
