// SPDX-License-Identifier: GPL-3.0-or-later
// Derived from Tiny4Linux (EUPL-1.2), used under EUPL-1.2 Article 5.

//! The exchange with the camera's extension unit: short commands, framed commands and the
//! polling for their answers, with one sequence number counter for the whole process.

use crate::error::{Error, Result};
use crate::family::frame::{Answer, Sequence, parse_answer, query_frame};
use crate::transport::uvc::{Device, XU_SIZE};
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
        poll_answer(command, seq, REPLY_GAP, || self.dev.xu_get(2))
    }
}

/// The poll of [`Camera::answer`] on any source of selector-2 reads, so a test can script the
/// replies. Waits `gap` before each of `REPLY_ATTEMPTS` reads.
pub(crate) fn poll_answer(
    command: u16,
    seq: u16,
    gap: Duration,
    mut read: impl FnMut() -> Result<[u8; XU_SIZE]>,
) -> Result<Vec<u8>> {
    for _ in 0..REPLY_ATTEMPTS {
        sleep(gap);
        // a read before the answer is ready can fail; just poll again
        if let Ok(reply) = read() {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    const COMMAND: u16 = 0x1344;
    const SEQ: u16 = 7;

    /// A selector-2 reply of the given type for `SEQ`, as `parse_answer` reads it: header
    /// length 16, an optional error code in the header extension, an optional data section.
    fn reply(kind: u8, command: u16, code: Option<i16>, data: &[u8]) -> [u8; XU_SIZE] {
        let mut r = [0u8; XU_SIZE];
        r[0] = 0xaa;
        r[1] = kind;
        r[2..4].copy_from_slice(&SEQ.to_le_bytes());
        r[4..6].copy_from_slice(&16u16.to_le_bytes());
        r[10..12].copy_from_slice(&command.to_le_bytes());
        if let Some(code) = code {
            r[12..14].copy_from_slice(&[0x00, 0x02]);
            r[14..16].copy_from_slice(&code.to_le_bytes());
        } else {
            r[12..14].copy_from_slice(&(data.len() as u16).to_le_bytes());
        }
        r[16..16 + data.len()].copy_from_slice(data);
        r
    }

    /// Polls with scripted reads, no waiting; returns the result and the number of reads taken.
    fn poll(script: impl Fn(usize) -> Result<[u8; XU_SIZE]>) -> (Result<Vec<u8>>, usize) {
        let reads = Cell::new(0);
        let result = poll_answer(COMMAND, SEQ, Duration::ZERO, || {
            let n = reads.get();
            reads.set(n + 1);
            script(n)
        });
        (result, reads.get())
    }

    #[test]
    fn a_rejected_answer_returns_the_cameras_code_at_the_first_poll() {
        // a refused command must return the camera's code at once, not poll on until "no answer"
        let (result, reads) = poll(|_| Ok(reply(0x89, COMMAND, Some(-3), &[])));
        assert!(
            matches!(result, Err(Error::Rejected(COMMAND, -3))),
            "{result:?}"
        );
        assert_eq!(reads, 1);
    }

    #[test]
    fn an_empty_answer_is_an_empty_list_not_a_missing_one() {
        let (result, reads) = poll(|_| Ok(reply(0x09, COMMAND, None, &[])));
        assert_eq!(result.unwrap(), Vec::<u8>::new());
        assert_eq!(reads, 1);
    }

    #[test]
    fn without_a_matching_answer_the_poll_gives_up_after_all_attempts() {
        // stale replies: the answer to another query (sequence) never matches
        let (result, reads) = poll(|_| {
            let mut stale = reply(0x29, COMMAND, None, &[1]);
            stale[2..4].copy_from_slice(&(SEQ + 1).to_le_bytes());
            Ok(stale)
        });
        assert!(
            matches!(result, Err(Error::NoAnswer(COMMAND))),
            "{result:?}"
        );
        // 20 reads at 50 ms: the one second a caller waits for a camera that does not answer
        assert_eq!(reads, 20);
    }

    #[test]
    fn a_failed_read_is_polled_again_and_a_later_value_is_returned() {
        // the camera is not ready for the first reads; the value that follows is the answer
        let (result, reads) = poll(|n| {
            if n < 2 {
                Err(Error::Invalid("not ready".into()))
            } else {
                Ok(reply(0x29, COMMAND, None, &[4, 5, 6]))
            }
        });
        assert_eq!(result.unwrap(), vec![4, 5, 6]);
        assert_eq!(reads, 3);
    }
}
