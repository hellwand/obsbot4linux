// SPDX-License-Identifier: GPL-3.0-or-later

//! The send table of the Tiny 2 Lite (design spec 8.3): every frame the model may write, each
//! with its evidence from the field ledger. Entries that only 0.13.2 sends (the U-rows of spec
//! 9.3 without any recording) say so until camera session CS-1 measures or removes them.

use crate::contract::{Frame, SendEntry, SendTable};
use crate::models::tiny2lite::plan::{
    FRAMED, GIMBAL, IDENTITY, POWER, PRESET_LIST, SHORT, TRACKING,
};

const fn short(command: u8, evidence: &'static str) -> SendEntry {
    SendEntry {
        frame: Frame::Short {
            selector: SHORT,
            command,
        },
        evidence,
        identity: false,
    }
}

const fn framed(receiver: u8, command: u16, evidence: &'static str) -> SendEntry {
    SendEntry {
        frame: Frame::Framed {
            selector: FRAMED,
            receiver,
            command,
        },
        evidence,
        identity: false,
    }
}

const fn query(receiver: u8, command: u16, evidence: &'static str) -> SendEntry {
    SendEntry {
        frame: Frame::Query {
            selector: FRAMED,
            receiver,
            command,
        },
        evidence,
        identity: false,
    }
}

const fn identity(command: u16, evidence: &'static str) -> SendEntry {
    SendEntry {
        frame: Frame::Query {
            selector: FRAMED,
            receiver: IDENTITY,
            command,
        },
        evidence,
        identity: true,
    }
}

const fn uvc(control: u32, evidence: &'static str) -> SendEntry {
    SendEntry {
        frame: Frame::Uvc { control },
        evidence,
        identity: false,
    }
}

/// Every entry, grouped as the field ledger lists them.
pub const ENTRIES: &[SendEntry] = &[
    // identity (5.5, 8.8): the serial and the firmware query, nothing else on receiver 0x0d
    identity(0x18c8, "capture 01"),
    identity(0x0408, "capture 01"),
    // short commands (selector 6)
    short(0x04, "capture 09"),
    short(0x16, "capture 11"),
    short(0x03, "capture 12"),
    short(0x01, "capture 12"),
    short(0x14, "capture 05"),
    short(0x1c, "capture 14"),
    short(0x0a, "capture 06"),
    short(0x17, "capture 06"),
    short(0x13, "capture 06"),
    short(0x1a, "capture 07"),
    short(0x18, "capture 11, 18"),
    short(0x0b, "capture 02"),
    short(0x0e, "capture 03"),
    short(0x15, "measured 2026-10-31 (#97)"),
    short(0x19, "measured 2026-10-31 (#97)"),
    short(0x00, "measured 2026-10-31 (#79)"),
    // framed settings
    framed(TRACKING, 0x3b84, "capture 05, 17"),
    framed(TRACKING, 0x0cc4, "capture 11"),
    framed(TRACKING, 0x0944, "measured 2026-10-31 (#98)"),
    framed(TRACKING, 0x0a44, "measured 2026-10-31 (#98)"),
    framed(TRACKING, 0x0ac4, "measured 2026-10-31 (#98)"),
    framed(TRACKING, 0x0b44, "measured 2026-10-31 (#98)"),
    framed(TRACKING, 0x0bc4, "measured 2026-10-31 (#98)"),
    framed(TRACKING, 0x0c44, "measured 2026-10-31 (#98)"),
    framed(TRACKING, 0x1584, "measured 2026-10-31 (#98)"),
    framed(TRACKING, 0x30c4, "capture 04"),
    framed(TRACKING, 0x3144, "capture 04"),
    framed(TRACKING, 0x3244, "capture 04"),
    framed(TRACKING, 0x3344, "capture 04"),
    framed(TRACKING, 0x33c4, "capture 04"),
    framed(TRACKING, 0x08c4, "capture 11"),
    framed(TRACKING, 0x4e04, "capture 11, 18"),
    framed(TRACKING, 0x4c04, "capture 11"),
    framed(TRACKING, 0x4c84, "capture 11"),
    framed(TRACKING, 0x4d04, "capture 11"),
    framed(TRACKING, 0x4d84, "capture 11"),
    framed(TRACKING, 0x2244, "capture 11, 17"),
    framed(TRACKING, 0x2044, "capture 11"),
    framed(TRACKING, 0x20c4, "capture 11"),
    framed(TRACKING, 0x2144, "capture 11"),
    framed(TRACKING, 0x21c4, "capture 11"),
    framed(TRACKING, 0x2344, "0.13.2 only, U-5"),
    framed(TRACKING, 0x23c4, "0.13.2 only, U-5"),
    framed(TRACKING, 0x2444, "0.13.2 only, U-6"),
    framed(TRACKING, 0x26c4, "capture 11"),
    framed(POWER, 0x3442, "capture 12; measured 2026-10-31 (#97)"),
    framed(POWER, 0x3602, "capture 12"),
    framed(POWER, 0x8102, "measured 2026-10-31 (#97)"),
    // framed actions
    framed(TRACKING, 0x6444, "capture 18"),
    framed(TRACKING, 0x6544, "measured 2026-10-31 (#98); limits U-9"),
    framed(TRACKING, 0x6704, "measured 2026-10-31 (#98)"),
    framed(TRACKING, 0x6484, "capture 10"),
    framed(TRACKING, 0x6884, "capture 11; speeds slow and fast U-12"),
    framed(TRACKING, 0x68c4, "measured 2026-10-31 (#98)"),
    framed(TRACKING, 0x3944, "capture 10"),
    framed(TRACKING, 0x3e04, "capture 10"),
    framed(TRACKING, 0x3a84, "capture 10"),
    framed(TRACKING, 0x39c4, "capture 10"),
    framed(TRACKING, 0x3984, "capture 10"),
    framed(TRACKING, 0x3d84, "capture 10"),
    framed(TRACKING, 0x1384, "capture 11"),
    framed(TRACKING, 0x1544, "capture 11"),
    framed(TRACKING, 0x1604, "measured 2026-10-31 (#100)"),
    framed(TRACKING, 0x13c4, "capture 11"),
    framed(TRACKING, 0x4e84, "capture 11"),
    framed(TRACKING, 0x4f44, "measured 2026-10-25"),
    framed(TRACKING, 0x24c4, "capture 11"),
    framed(TRACKING, 0x2584, "measured 2026-10-25"),
    framed(TRACKING, 0x2544, "measured 2026-10-25"),
    framed(TRACKING, 0x25c4, "0.13.2 only, U-7"),
    framed(TRACKING, 0x2604, "0.13.2 only, U-7"),
    framed(TRACKING, 0x2644, "0.13.2 only, U-7"),
    framed(TRACKING, 0x2684, "0.13.2 only, U-7"),
    framed(POWER, 0x3582, "measured 2026-10-31 (#97); U-16"),
    framed(TRACKING, 0x3ec4, "capture 08"),
    framed(TRACKING, 0x3e44, "capture 08"),
    framed(POWER, 0xa0c2, "capture 13"),
    framed(POWER, 0xa802, "capture 16"),
    // queries (reads, but frames the model writes)
    query(GIMBAL, 0x0043, "measured 2026-10-31 (#67)"),
    query(TRACKING, 0x0104, "capture 01"),
    query(TRACKING, 0x6804, "capture 04, 11"),
    query(TRACKING, 0x01c4, "measured 2026-10-25"),
    query(TRACKING, 0x0984, "measured 2026-10-31 (#98)"),
    query(TRACKING, 0x0a84, "measured 2026-10-31 (#98)"),
    query(TRACKING, 0x0b04, "measured 2026-10-31 (#98)"),
    query(TRACKING, 0x0b84, "measured 2026-10-31 (#98)"),
    query(TRACKING, 0x0c04, "measured 2026-10-31 (#98)"),
    query(TRACKING, 0x0c84, "measured 2026-10-31 (#98)"),
    query(TRACKING, 0x15c4, "measured 2026-10-31 (#98)"),
    query(POWER, 0x3642, "measured 2026-10-31 (#97)"),
    query(POWER, 0x80c2, "measured 2026-10-31 (#97)"),
    query(TRACKING, 0x4c44, "capture 11"),
    query(TRACKING, 0x4cc4, "capture 11"),
    query(TRACKING, 0x4d44, "capture 11"),
    query(TRACKING, 0x4dc4, "capture 11"),
    query(TRACKING, 0x4e44, "capture 11, 18"),
    query(TRACKING, 0x4ec4, "capture 11, 18"),
    query(TRACKING, 0x2004, "capture 11"),
    query(TRACKING, 0x2504, "capture 11"),
    query(TRACKING, 0x2484, "0.13.2 only, U-6"),
    query(TRACKING, 0x3884, "capture 08"),
    query(TRACKING, 0x3e84, "capture 08"),
    query(TRACKING, 0x1344, "capture 11, 18"),
    query(TRACKING, 0x1504, "capture 11"),
    query(TRACKING, 0x1444, "capture 11"),
    // UVC controls
    uvc(0x009a_090d, "capture 04, 11; measured 2026-10-24"),
    uvc(0x009a_0901, "measured 2026-10-31 (#97)"),
    uvc(0x009a_0902, "measured 2026-10-31 (#97)"),
    uvc(0x0098_0913, "capture 12"),
    uvc(0x0098_091c, "capture 08b, 12"),
    uvc(0x0098_0918, "capture 12"),
    uvc(0x0098_0900, "measured 2026-10-24"),
    uvc(0x0098_0901, "capture 08b, 12"),
    uvc(0x0098_0902, "capture 08b, 12"),
    uvc(0x0098_0903, "capture 08b, 12"),
    uvc(0x0098_091b, "capture 08b, 12"),
    uvc(0x0098_090c, "capture 08b, 12"),
    uvc(0x0098_091a, "capture 08b, 12"),
    uvc(0x009a_090a, "capture 12"),
    // the preset list rewind (8.8): the one known exception of section 2 until CAM-19
    SendEntry {
        frame: Frame::Block {
            selector: PRESET_LIST,
        },
        evidence: "measured 2026-10-31 (#104); CAM-19",
        identity: false,
    },
];

/// The send table.
pub fn table() -> SendTable {
    SendTable {
        entries: ENTRIES.to_vec(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn every_frame_is_named_once_with_its_evidence() {
        let mut seen = BTreeSet::new();
        for e in ENTRIES {
            assert!(seen.insert(format!("{:?}", e.frame)), "{:?} twice", e.frame);
            assert!(
                e.evidence.starts_with("capture ")
                    || e.evidence.starts_with("measured ")
                    || e.evidence.starts_with("0.13.2 only, U-"),
                "{:?}: {}",
                e.frame,
                e.evidence
            );
        }
    }

    #[test]
    fn only_the_two_identity_queries_go_to_the_identity_receiver() {
        let on_identity: Vec<&SendEntry> = ENTRIES
            .iter()
            .filter(|e| {
                matches!(e.frame, Frame::Framed { receiver, .. } | Frame::Query { receiver, .. } if receiver == IDENTITY)
            })
            .collect();
        assert_eq!(on_identity.len(), 2);
        assert!(on_identity.iter().all(|e| e.identity));
        assert!(ENTRIES.iter().filter(|e| e.identity).count() == 2);
    }

    #[test]
    fn the_frames_only_0_13_2_sends_are_the_u_rows() {
        let only: BTreeSet<String> = ENTRIES
            .iter()
            .filter(|e| e.evidence.starts_with("0.13.2 only"))
            .map(|e| format!("{:?}", e.frame))
            .collect();
        let want: BTreeSet<String> = [
            (FRAMED, TRACKING, 0x2344u16, false),
            (FRAMED, TRACKING, 0x23c4, false),
            (FRAMED, TRACKING, 0x2444, false),
            (FRAMED, TRACKING, 0x2484, true),
            (FRAMED, TRACKING, 0x25c4, false),
            (FRAMED, TRACKING, 0x2604, false),
            (FRAMED, TRACKING, 0x2644, false),
            (FRAMED, TRACKING, 0x2684, false),
        ]
        .into_iter()
        .map(|(selector, receiver, command, is_query)| {
            format!(
                "{:?}",
                if is_query {
                    Frame::Query {
                        selector,
                        receiver,
                        command,
                    }
                } else {
                    Frame::Framed {
                        selector,
                        receiver,
                        command,
                    }
                }
            )
        })
        .collect();
        assert_eq!(only, want);
    }
}
