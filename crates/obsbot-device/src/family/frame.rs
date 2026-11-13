// SPDX-License-Identifier: GPL-3.0-or-later
// Derived from Tiny4Linux (EUPL-1.2), used under EUPL-1.2 Article 5.

//! Framed commands (unit 2, selector 2): CRC-16/USB, query frames, sequence numbers.

use std::sync::atomic::{AtomicU16, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

pub const FRAME_SIZE: usize = 60;
const HEADER_SIZE: usize = 12;
const DATA_OFFSET: usize = 16;
/// Largest payload that fits after both headers.
pub const MAX_DATA: usize = FRAME_SIZE - DATA_OFFSET;

/// CRC-16/USB: reflected polynomial 0xA001, init 0xFFFF, final XOR 0xFFFF.
pub fn crc16_usb(data: &[u8]) -> u16 {
    let mut crc: u16 = 0xffff;
    for byte in data {
        crc ^= *byte as u16;
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0xa001
            } else {
                crc >> 1
            };
        }
    }
    crc ^ 0xffff
}

fn header(kind: u8, receiver: u8, command: u16, sequence: u16) -> [u8; FRAME_SIZE] {
    let mut b = [0u8; FRAME_SIZE];
    b[0..2].copy_from_slice(&[0xaa, kind]);
    b[2..4].copy_from_slice(&sequence.to_le_bytes());
    b[4..6].copy_from_slice(&(HEADER_SIZE as u16).to_le_bytes());
    b[8] = 0x0a;
    b[9] = receiver;
    b[10..12].copy_from_slice(&command.to_le_bytes());
    let crc = crc16_usb(&b[0..HEADER_SIZE]);
    b[6..8].copy_from_slice(&crc.to_le_bytes());
    b
}

/// Builds a set frame (type 0x25): header, then data length, data CRC and data.
///
/// # Panics
/// If `data` is longer than [`MAX_DATA`].
pub fn frame(receiver: u8, command: u16, data: &[u8], sequence: u16) -> [u8; FRAME_SIZE] {
    with_data(0x25, receiver, command, data, sequence)
}

/// Builds a query frame that carries a request (type 0x21), e.g. the id of the view to read.
///
/// # Panics
/// If `data` is longer than [`MAX_DATA`].
pub fn query_frame_with(
    receiver: u8,
    command: u16,
    data: &[u8],
    sequence: u16,
) -> [u8; FRAME_SIZE] {
    with_data(0x21, receiver, command, data, sequence)
}

fn with_data(kind: u8, receiver: u8, command: u16, data: &[u8], sequence: u16) -> [u8; FRAME_SIZE] {
    assert!(
        data.len() <= MAX_DATA,
        "frame data must not exceed {MAX_DATA} bytes"
    );
    let mut b = header(kind, receiver, command, sequence);
    let end = DATA_OFFSET + data.len();
    b[12..14].copy_from_slice(&(data.len() as u16).to_le_bytes());
    b[DATA_OFFSET..end].copy_from_slice(data);
    let crc = crc16_usb(&b[HEADER_SIZE..end]);
    b[14..16].copy_from_slice(&crc.to_le_bytes());
    b
}

/// Builds a query frame (type 0x01): data length 0 and data CRC 0.
pub fn query_frame(receiver: u8, command: u16, sequence: u16) -> [u8; FRAME_SIZE] {
    header(0x01, receiver, command, sequence)
}

/// Answer to a query, matched by sequence number and command id (both echoed), with good CRCs.
#[derive(Debug, PartialEq)]
pub enum Answer<'a> {
    /// Type 0x29: the value, in the format the setter takes.
    Value(&'a [u8]),
    /// Type 0x89: rejected, with the signed error code from the header extension.
    Rejected(i16),
    /// Type 0x09: an answer with no data section, which is how the camera says "nothing to list"
    /// (`0x1344` with no zone view stored, captures 11 and 18).
    Empty,
}

/// The error code of a rejected answer. Those frames have a 16-byte header (bytes 4..6) whose
/// extension at 12..16 is tag 0, length 2 and the i16 code; the bytes after it are not a data
/// section. 0 when the header has no such extension.
fn error_code(reply: &[u8]) -> i16 {
    let header = u16::from_le_bytes([reply[4], reply[5]]) as usize;
    if header >= DATA_OFFSET && reply[12..14] == [0x00, 0x02] {
        i16::from_le_bytes([reply[14], reply[15]])
    } else {
        0
    }
}

/// Whether the header CRC of a reply is right: CRC-16/USB over the header (its length is the
/// word at bytes 4..6, 12 or 16 bytes) with the CRC field at bytes 6..8 taken as zero. The
/// recorded answers follow it (captures 01, 04, 08, 11).
fn header_crc_ok(reply: &[u8]) -> bool {
    let len = u16::from_le_bytes([reply[4], reply[5]]) as usize;
    if !(HEADER_SIZE..=DATA_OFFSET).contains(&len) || reply.len() < len {
        return false;
    }
    let mut header = [0u8; DATA_OFFSET];
    header[..len].copy_from_slice(&reply[..len]);
    header[6..8].fill(0);
    crc16_usb(&header[..len]) == u16::from_le_bytes([reply[6], reply[7]])
}

/// Whether the data CRC of a value answer is right: CRC-16/USB over bytes 12 to the end of the
/// data with the CRC field at 14..16 taken as zero, as in the frames this crate builds and in
/// the recorded value answers (captures 04, 08, 11), whose header is 12 bytes long.
fn data_crc_ok(reply: &[u8], len: usize) -> bool {
    let mut section = reply[HEADER_SIZE..DATA_OFFSET + len].to_vec();
    section[2..4].fill(0);
    crc16_usb(&section) == u16::from_le_bytes([reply[14], reply[15]])
}

/// Parses a selector-2 read; None when it is not the answer to the query with `sequence` and
/// `command` (yet). An answer must echo both and carry a good header CRC; a value answer also a
/// 12-byte header and a good data CRC (#61). So a reply to another process's query with the
/// same sequence number, or a damaged one, is not taken for ours.
pub fn parse_answer(reply: &[u8], sequence: u16, command: u16) -> Option<Answer<'_>> {
    if reply.len() < DATA_OFFSET
        || reply[0] != 0xaa
        || reply[2..4] != sequence.to_le_bytes()
        || reply[10..12] != command.to_le_bytes()
        || !header_crc_ok(reply)
    {
        return None;
    }
    match reply[1] {
        0x29 => {
            let len = u16::from_le_bytes([reply[12], reply[13]]) as usize;
            let header = u16::from_le_bytes([reply[4], reply[5]]) as usize;
            if header != HEADER_SIZE || DATA_OFFSET + len > reply.len() || !data_crc_ok(reply, len)
            {
                return None;
            }
            Some(Answer::Value(&reply[DATA_OFFSET..DATA_OFFSET + len]))
        }
        0x89 => Some(Answer::Rejected(error_code(reply))),
        0x09 => Some(Answer::Empty),
        _ => None,
    }
}

/// Sequence numbers for frames. The camera drops a frame that repeats the previous
/// number, so every frame gets a fresh one; seeding from the clock keeps two
/// short-lived processes from starting at the same number.
pub struct Sequence(AtomicU16);

impl Sequence {
    pub fn new() -> Self {
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u16)
            .unwrap_or(0);
        Self(AtomicU16::new(seed))
    }

    /// A sequence whose first [`Sequence::next`] is `first`: for tests that compare frames with
    /// recorded ones at their recorded sequence numbers.
    pub fn starting_at(first: u16) -> Self {
        Self(AtomicU16::new(first.wrapping_sub(1)))
    }

    pub fn next(&self) -> u16 {
        self.0.fetch_add(1, Ordering::Relaxed).wrapping_add(1)
    }
}

impl Default for Sequence {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc16_usb_check_value() {
        assert_eq!(crc16_usb(b"123456789"), 0xb4c8);
    }

    #[test]
    fn layout() {
        let f = frame(0x04, 0x3244, &[0x11, 0x22, 0x33], 0x1234);
        assert_eq!(f[0..2], [0xaa, 0x25]);
        assert_eq!(f[2..4], [0x34, 0x12], "sequence little endian");
        assert_eq!(f[10..12], [0x44, 0x32], "full 16 bit command id");
        assert_eq!(f[12..14], [0x03, 0x00]);
        assert_eq!(f[16..19], [0x11, 0x22, 0x33]);
        assert!(f[19..].iter().all(|b| *b == 0));
    }

    #[test]
    fn checksums_cover_header_and_data() {
        let f = frame(0x04, 0x30c4, &[0x01], 7);
        let mut h = f[0..12].to_vec();
        h[6..8].fill(0);
        assert_eq!(f[6..8], crc16_usb(&h).to_le_bytes());
        let mut d = f[12..17].to_vec();
        d[2..4].fill(0);
        assert_eq!(f[14..16], crc16_usb(&d).to_le_bytes());
    }

    #[test]
    #[should_panic]
    fn too_much_data_panics() {
        frame(0x04, 1, &[0u8; MAX_DATA + 1], 0);
    }

    #[test]
    fn query_matches_capture() {
        // Query 0x0104 recorded on the owner's device (capture 01, seq 7).
        let captured = hex::decode("aa0107000c00344e0a04040100000000").unwrap();
        let q = query_frame(0x04, 0x0104, 7);
        assert_eq!(q[..captured.len()], captured[..]);
        assert!(q[captured.len()..].iter().all(|b| *b == 0));
    }

    #[test]
    fn query_with_request_matches_capture() {
        // 0x1504 (name of zone view 0) recorded on the owner's device (capture 11, seq 450): type 0x21
        let captured = hex::decode("aa21c2010c00873d0a0404150400be0700000000").unwrap();
        let q = query_frame_with(0x04, 0x1504, &0u32.to_le_bytes(), 450);
        assert_eq!(q[..captured.len()], captured[..]);
        assert!(q[captured.len()..].iter().all(|b| *b == 0));
    }

    #[test]
    fn answer_without_data_section_is_empty_not_missing() {
        // Reply to the zone view list (0x1344, capture 11 seq 374) while no view is stored: type
        // 0x09, a 12-byte header and no data section; the bytes after the header are leftovers of
        // the previous read. It used to count as "no answer" and ran into the poll timeout.
        let mut r = [0u8; 60];
        let captured = hex::decode(concat!(
            "aa0976010c0092b6040a4413",
            "04006856",
            "0000c8c20101"
        ))
        .unwrap();
        r[..captured.len()].copy_from_slice(&captured);
        assert_eq!(parse_answer(&r, 374, 0x1344), Some(Answer::Empty));
        assert_eq!(parse_answer(&r, 375, 0x1344), None);
        // same sequence, but the reply is for another query: not an answer to this one
        assert_eq!(parse_answer(&r, 374, 0x1444), None);
    }

    use crate::transport::replay::answer_frame;

    /// A value answer in the recorded layout (12-byte header, data CRC).
    fn answer(seq: u16, command: u16, data: &[u8]) -> [u8; 60] {
        answer_frame(0x29, seq, 0x04, command, None, data)
    }

    #[test]
    fn recorded_value_answers_parse() {
        // capture 04 (0x6804, sequence 0x2f) and capture 11 (0x1504, sequence 0x01c2)
        let mut r = [0u8; 60];
        let rec = hex::decode("aa292f000c00966b040a046804003a9aae473140").unwrap();
        r[..rec.len()].copy_from_slice(&rec);
        assert_eq!(
            parse_answer(&r, 0x2f, 0x6804),
            Some(Answer::Value(&[0xae, 0x47, 0x31, 0x40]))
        );
        let mut r = [0u8; 60];
        let rec = hex::decode("aa29c2010c00ce76040a041508008f814265726569636831").unwrap();
        r[..rec.len()].copy_from_slice(&rec);
        assert_eq!(
            parse_answer(&r, 0x01c2, 0x1504),
            Some(Answer::Value(b"Bereich1"))
        );
    }

    #[test]
    fn an_answer_must_echo_sequence_and_command_and_carry_good_crcs() {
        let r = answer(7, 0x0104, &[1, 2, 3]);
        assert_eq!(parse_answer(&r, 7, 0x0104), Some(Answer::Value(&[1, 2, 3])));
        assert_eq!(parse_answer(&r, 8, 0x0104), None, "other sequence");
        assert_eq!(
            parse_answer(&r, 7, 0x0144),
            None,
            "the answer to another query with the same sequence number (#61)"
        );
        let mut bad = r;
        bad[6] ^= 0x01;
        assert_eq!(parse_answer(&bad, 7, 0x0104), None, "header CRC wrong");
        let mut bad = r;
        bad[9] ^= 0x01;
        assert_eq!(parse_answer(&bad, 7, 0x0104), None, "a header byte changed");
        let mut bad = r;
        bad[17] ^= 0x01;
        assert_eq!(parse_answer(&bad, 7, 0x0104), None, "a data byte changed");
        let mut bad = r;
        bad[14] ^= 0x01;
        assert_eq!(parse_answer(&bad, 7, 0x0104), None, "data CRC wrong");
        assert_eq!(parse_answer(&[0u8; 60], 7, 0), None, "not a frame");
    }

    #[test]
    fn a_value_answer_needs_the_12_byte_header_and_its_whole_data() {
        let mut r = answer(7, 0x0104, &[1, 2, 3]);
        r[4] = 16;
        r[6..8].fill(0);
        let crc = crc16_usb(&r[..16]);
        r[6..8].copy_from_slice(&crc.to_le_bytes());
        assert_eq!(parse_answer(&r, 7, 0x0104), None, "16-byte header");
        let r = answer(7, 0x0104, &[0xab; 44]);
        assert!(
            parse_answer(&r, 7, 0x0104).is_some(),
            "44 bytes fill the frame"
        );
        assert_eq!(
            parse_answer(&r[..59], 7, 0x0104),
            None,
            "the data runs past the read"
        );
    }

    #[test]
    fn a_header_length_outside_12_to_16_is_no_answer() {
        for len in [0u16, 11, 17, 60] {
            let mut r = answer_frame(0x09, 7, 0x04, 0x1344, None, &[]);
            r[4..6].copy_from_slice(&len.to_le_bytes());
            r[6..8].fill(0);
            let crc = crc16_usb(&r[..usize::from(len).clamp(12, 16)]);
            r[6..8].copy_from_slice(&crc.to_le_bytes());
            assert_eq!(parse_answer(&r, 7, 0x1344), None, "header length {len}");
        }
    }

    #[test]
    fn rejected_answer_carries_the_code_from_the_header_extension() {
        // Captured rejection of query 0xc442 (capture 01): header length 16, extension
        // `00 02 fd ff` = -3. The bytes after it are not an error code: reading them from
        // offset 16 as before gave the wrong number.
        let mut r = [0u8; 60];
        let captured = hex::decode("aa890400100046b7020a42c40002fdff0000803f01010014").unwrap();
        r[..captured.len()].copy_from_slice(&captured);
        assert_eq!(parse_answer(&r, 4, 0xc442), Some(Answer::Rejected(-3)));
    }

    #[test]
    fn rejected_answer_without_extension_has_no_code() {
        // 12-byte header: nothing to read, and the data section of an error is ignored
        let mut r = answer(7, 0x0104, &[0x0a, 0x00]);
        r[1] = 0x89;
        r[6..8].fill(0);
        let crc = crc16_usb(&r[..12]);
        r[6..8].copy_from_slice(&crc.to_le_bytes());
        assert_eq!(parse_answer(&r, 7, 0x0104), Some(Answer::Rejected(0)));
    }

    #[test]
    fn sequence_never_repeats_consecutively() {
        let s = Sequence::new();
        let mut prev = s.next();
        for _ in 0..70_000 {
            let cur = s.next();
            assert_eq!(cur, prev.wrapping_add(1));
            prev = cur;
        }
    }

    #[test]
    fn a_frame_holds_exactly_44_bytes_of_data() {
        // 60 bytes in all, 16 of them headers: 44 bytes of data fill the frame to its end
        assert_eq!(MAX_DATA, 44);
        let f = frame(0x04, 1, &[0xab; 44], 0);
        assert!(f[16..].iter().all(|b| *b == 0xab));
    }

    #[test]
    #[should_panic(expected = "must not exceed 44 bytes")]
    fn one_byte_past_the_data_section_is_refused() {
        frame(0x04, 1, &[0u8; 45], 0);
    }

    #[test]
    fn a_rejection_needs_a_16_byte_header_and_the_extension_tag() {
        // layout: header length at 4..6, the extension `00 02` at 12..14, the i16 code at 14..16
        let rejected = |header: u16, extension: [u8; 2]| {
            let mut r = [0u8; 60];
            r[0] = 0xaa;
            r[1] = 0x89;
            r[2..4].copy_from_slice(&7u16.to_le_bytes());
            r[4..6].copy_from_slice(&header.to_le_bytes());
            r[10..12].copy_from_slice(&0x0104u16.to_le_bytes());
            r[12..14].copy_from_slice(&extension);
            r[14..16].copy_from_slice(&(-3i16).to_le_bytes());
            let crc = crc16_usb(&r[..usize::from(header)]);
            r[6..8].copy_from_slice(&crc.to_le_bytes());
            parse_answer(&r, 7, 0x0104).map(|a| matches!(a, Answer::Rejected(-3)))
        };
        assert_eq!(rejected(16, [0x00, 0x02]), Some(true));
        assert_eq!(rejected(12, [0x00, 0x02]), Some(false), "12-byte header");
        assert_eq!(rejected(16, [0x01, 0x02]), Some(false), "another extension");
    }

    #[test]
    fn a_reply_must_hold_both_headers_to_be_an_answer() {
        let full = answer(7, 0x0104, &[]);
        assert_eq!(parse_answer(&full[..12], 7, 0x0104), None, "12 bytes");
        assert_eq!(parse_answer(&full[..15], 7, 0x0104), None, "one byte short");
        assert_eq!(
            parse_answer(&full[..16], 7, 0x0104),
            Some(Answer::Value(&[])),
            "both headers and no data"
        );
    }

    #[test]
    fn a_sequence_can_start_at_a_recorded_number() {
        let s = Sequence::starting_at(0x0186);
        assert_eq!(s.next(), 0x0186);
        assert_eq!(s.next(), 0x0187);
        let wrap = Sequence::starting_at(0);
        assert_eq!(wrap.next(), 0, "0 is a number like any other");
        assert_eq!(wrap.next(), 1);
    }
}
