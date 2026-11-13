// SPDX-License-Identifier: GPL-3.0-or-later
// Derived from Tiny4Linux (EUPL-1.2), used under EUPL-1.2 Article 5.

//! The exchange with the camera's extension unit: short commands, framed commands and the
//! polling for their answers, with one sequence number counter for the whole process.

use crate::error::{Error, Result};
use crate::family::frame::{Answer, Sequence, parse_answer, query_frame};
use crate::transport::uvc::Device;
use std::sync::OnceLock;
use std::thread::sleep;
use std::time::Duration;

const REPLY_GAP: Duration = Duration::from_millis(50);
const REPLY_ATTEMPTS: usize = 20;
/// Gap between the two frames of a pair (preset delete): back to back the camera applies neither.
pub(crate) const PAIR_GAP: Duration = Duration::from_millis(100);

/// One sequence for the whole process, so frames from separate `Camera` handles (the daemon
/// opens the device per operation) never repeat a number.
fn sequence() -> &'static Sequence {
    static SEQ: OnceLock<Sequence> = OnceLock::new();
    SEQ.get_or_init(Sequence::new)
}

pub struct Camera {
    pub(crate) dev: Device,
    pub(crate) seq: &'static Sequence,
}

impl Camera {
    pub fn open() -> Result<Self> {
        Ok(Self {
            dev: Device::open()?,
            seq: sequence(),
        })
    }

    pub(crate) fn short(&self, payload: &[u8]) -> Result<()> {
        self.dev.xu_set(6, payload)
    }

    pub(crate) fn framed(&self, frame: &[u8]) -> Result<()> {
        self.dev.xu_set(2, frame)
    }

    pub(crate) fn pair(&self, frames: [[u8; 60]; 2]) -> Result<()> {
        self.framed(&frames[0])?;
        sleep(PAIR_GAP);
        self.framed(&frames[1])
    }

    /// Sends a query and polls selector 2 for the answer with the same sequence number.
    pub(crate) fn query(&self, command: u16) -> Result<Vec<u8>> {
        self.query_to(0x04, command)
    }

    pub(crate) fn query_to(&self, receiver: u8, command: u16) -> Result<Vec<u8>> {
        let seq = self.seq.next();
        self.framed(&query_frame(receiver, command, seq))?;
        self.answer(command, seq)
    }

    /// Polls selector 2 for the answer to the query with `seq`.
    pub(crate) fn answer(&self, command: u16, seq: u16) -> Result<Vec<u8>> {
        for _ in 0..REPLY_ATTEMPTS {
            sleep(REPLY_GAP);
            // a read before the answer is ready can fail; just poll again
            if let Ok(reply) = self.dev.xu_get(2) {
                match parse_answer(&reply, seq, command) {
                    Some(Answer::Value(data)) => return Ok(data.to_vec()),
                    Some(Answer::Empty) => return Ok(Vec::new()),
                    Some(Answer::Rejected(code)) => return Err(Error::Rejected(command, code)),
                    None => {}
                }
            }
        }
        Err(Error::NoAnswer(command))
    }
}
