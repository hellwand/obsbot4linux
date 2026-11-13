// SPDX-License-Identifier: GPL-3.0-or-later

//! A scripted camera for tests: a [`Device`](crate::transport::uvc::Device) that answers from a
//! table instead of ioctls, so the flows of `Camera` run without a camera. It sits behind the
//! same four methods the real device uses (`xu_get`, `xu_set`, `get_ctrl`, `ctrl_range`) and is
//! compiled in tests only.

use crate::error::{Error, Result};
use crate::transport::uvc::{CtrlRange, XU_SIZE};
use std::cell::RefCell;
use std::collections::BTreeMap;

/// How the scripted camera treats a framed query.
pub(crate) enum Answer {
    /// Type 0x29 with this data.
    Value(Vec<u8>),
    /// Type 0x89 with this error code.
    Rejected(i16),
    /// Sending the query fails (the write to selector 2 returns an error).
    SendFails,
}

/// The table behind a scripted [`Device`]. A query that is not in `answers` is rejected, as a
/// camera rejects a query it does not know; that keeps a test free of waiting for polls.
pub(crate) struct Scripted {
    /// What a read of selector 6 (the status buffer) returns.
    pub status: [u8; XU_SIZE],
    pub answers: BTreeMap<u16, Answer>,
    /// V4L2 controls the camera has, with their current values.
    pub ctrls: BTreeMap<u32, i32>,
    /// The last frame written to selector 2, which the next read of selector 2 answers.
    last: RefCell<[u8; XU_SIZE]>,
}

impl Scripted {
    pub fn new(status: [u8; XU_SIZE]) -> Self {
        Self {
            status,
            answers: BTreeMap::new(),
            ctrls: BTreeMap::new(),
            last: RefCell::new([0; XU_SIZE]),
        }
    }

    pub fn xu_get(&self, selector: u8) -> Result<[u8; XU_SIZE]> {
        match selector {
            6 => Ok(self.status),
            2 => {
                let last = *self.last.borrow();
                let command = u16::from_le_bytes([last[10], last[11]]);
                Ok(match self.answers.get(&command) {
                    Some(Answer::Value(data)) => answer(0x29, &last, None, data),
                    Some(Answer::Rejected(code)) => answer(0x89, &last, Some(*code), &[]),
                    Some(Answer::SendFails) | None => answer(0x89, &last, Some(-1), &[]),
                })
            }
            _ => Err(Error::Invalid(format!(
                "selector {selector} is not scripted"
            ))),
        }
    }

    pub fn xu_set(&self, selector: u8, payload: &[u8]) -> Result<()> {
        if selector == 2 {
            let mut frame = [0u8; XU_SIZE];
            frame[..payload.len()].copy_from_slice(payload);
            let command = u16::from_le_bytes([frame[10], frame[11]]);
            if matches!(self.answers.get(&command), Some(Answer::SendFails)) {
                return Err(Error::Io(std::io::Error::other("scripted write failure")));
            }
            *self.last.borrow_mut() = frame;
        }
        Ok(())
    }

    pub fn get_ctrl(&self, id: u32) -> Result<i32> {
        self.ctrls
            .get(&id)
            .copied()
            .ok_or_else(|| Error::Invalid(format!("control {id:#x} is not scripted")))
    }

    pub fn ctrl_range(&self, id: u32) -> Result<Option<CtrlRange>> {
        Ok(self.ctrls.contains_key(&id).then_some(CtrlRange {
            min: 0,
            max: 100,
            step: 1,
            default: 50,
        }))
    }
}

/// An answer to the query frame `query`: same sequence number, header length 16, then either
/// the error code of a rejection or the data section, as `parse_answer` reads it.
fn answer(kind: u8, query: &[u8; XU_SIZE], code: Option<i16>, data: &[u8]) -> [u8; XU_SIZE] {
    let mut r = [0u8; XU_SIZE];
    r[0] = 0xaa;
    r[1] = kind;
    r[2..4].copy_from_slice(&query[2..4]);
    r[4..6].copy_from_slice(&16u16.to_le_bytes());
    r[10..12].copy_from_slice(&query[10..12]);
    if let Some(code) = code {
        r[12..14].copy_from_slice(&[0x00, 0x02]);
        r[14..16].copy_from_slice(&code.to_le_bytes());
    } else {
        r[12..14].copy_from_slice(&(data.len() as u16).to_le_bytes());
    }
    r[16..16 + data.len()].copy_from_slice(data);
    r
}
