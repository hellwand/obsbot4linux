// SPDX-License-Identifier: GPL-3.0-or-later

//! A recorded Tiny 2 Lite on the replay transport, for the model's tests. Every answer is a
//! recorded one where a recording exists (the capture is named), or a value measured on the
//! camera; the serial is synthetic (identity answers are never fixtures, spec 8.5).

#![allow(dead_code)]

use obsbot_device::contract::{Frame, OnAir};
use obsbot_device::family::frame::Sequence;
use obsbot_device::family::link::Timing;
use obsbot_device::models::tiny2lite::model::{SELECTORS, Tiny2Lite};
use obsbot_device::transport::checked::classify;
use obsbot_device::transport::replay::{Log, Replay, Sent};
use obsbot_proto::Usb;

/// The status block read in captures 05, 06 and 07 before any change: awake, static,
/// mirror off, auto-sleep off with 120 s kept.
pub const STATUS: &str = "2701000121000101010188ff000101000003000001002100000103000000001e00031000020000000000000000000000000000000000000000000000";

pub fn hex(s: &str) -> Vec<u8> {
    hex::decode(s).unwrap()
}

pub fn block(s: &str) -> [u8; 60] {
    let mut b = [0u8; 60];
    let d = hex(s);
    b[..d.len()].copy_from_slice(&d);
    b
}

/// The status block `STATUS` with the tracking mode `normal`.
pub fn status_tracking() -> [u8; 60] {
    let mut b = block(STATUS);
    b[0x18] = 2;
    b
}

/// An answer to the target query `0x01c4` with a target (layout measured 2026-10-25, #71):
/// detected, box 0.25, 0.5, 0.75, 1.0.
pub fn target_answer() -> Vec<u8> {
    let mut d = vec![1, 1, 1, 1, 0, 0];
    d.extend(floats(&[0.25, 0.5, 0.75, 1.0]));
    d
}

/// The recorded camera with the tracking on (`normal`) and a target in view.
pub fn tracking() -> Replay {
    recorded()
        .buffer(6, status_tracking())
        .answer(0x04, 0x01c4, &target_answer())
}

/// A preset list entry read from a Tiny 2 Lite (pan 12.5, pitch -3.25, zoom 1.5, "t4ltest").
pub fn preset_entry(slot: u8) -> [u8; 60] {
    let mut b = block("00000000bbfee2049600644452736447567a64413d3d");
    b[1] = slot;
    b
}

/// The USB facts of the camera: the descriptor recorded in capture 01 (USB 3564:fef9,
/// bcdDevice 0510, no serial string, video and audio interfaces, extension unit 2).
pub fn usb() -> Usb {
    Usb {
        vendor: "3564".into(),
        product: "fef9".into(),
        product_name: Some("OBSBOT Tiny 2 Lite".into()),
        bcd_device: "0510".into(),
        path: "3-2".into(),
        serial_present: false,
        interfaces: vec!["0e".into(), "0e".into(), "01".into(), "01".into()],
        extension_units: vec![obsbot_proto::ExtensionUnit {
            unit: 2,
            guid: "9a1e7291-6843-4683-6d92-39bc7906ee49".into(),
            controls: 19,
        }],
    }
}

fn floats(values: &[f32]) -> Vec<u8> {
    values.iter().flat_map(|v| v.to_le_bytes()).collect()
}

/// The recorded camera: one preset in slot 0, one view, no target (static).
pub fn recorded() -> Replay {
    let mut focus = 1u32.to_le_bytes().to_vec();
    focus.extend(0u32.to_le_bytes());
    focus.extend(floats(&[0.5, 0.5]));
    let mut pose = vec![0u8; 12];
    pose[8..10].copy_from_slice(&(-1000i16).to_le_bytes());
    pose[10..12].copy_from_slice(&2500i16.to_le_bytes());
    Replay::new(2)
        .buffer(6, block(STATUS))
        // identity: synthetic serial; firmware 6.2.7.4 (capture 01)
        .answer(0x0d, 0x18c8, b"EXAMPLE0000001")
        .answer(0x0d, 0x0408, &hex("0407020600000000"))
        // capture 01
        .answer(0x04, 0x0104, &hex("000000010101001400020000"))
        // live pose: pitch -10.00 (up), yaw 25.00
        .answer(0x03, 0x0043, &pose)
        // capture 04: zoom 1.6
        .answer(0x04, 0x6804, &hex("cdcccc3f"))
        // framing readbacks (measured 2026-10-31, #98): speed normal, auto zoom off, medium
        // shot, zoom speed never set (30), offsets 0, part normal
        .answer(0x04, 0x0984, &[2])
        .answer(0x04, 0x0a84, &[0])
        .answer(0x04, 0x0b04, &[2])
        .answer(0x04, 0x0b84, &[30])
        .answer(0x04, 0x0c04, &floats(&[0.0]))
        .answer(0x04, 0x0c84, &floats(&[0.0]))
        .answer(0x04, 0x15c4, &[0])
        // focus continuous, position 0 (measured 2026-10-31, #97); volume 55 (#97)
        .answer(0x02, 0x3642, &focus)
        .answer(0x02, 0x80c2, &[0x37, 0x00])
        // capture 11: zone pan -100..100, pitch -30..30, follow off, return 0/0/1
        .answer(0x04, 0x4c44, &hex("0000c8c2"))
        .answer(0x04, 0x4cc4, &hex("0000c842"))
        .answer(0x04, 0x4d44, &hex("0000f0c1"))
        .answer(0x04, 0x4dc4, &hex("0000f041"))
        .answer(0x04, 0x4e44, &[0])
        .answer(
            0x04,
            0x4ec4,
            &hex("000000000000000000000000000000000000803f00000000"),
        )
        // capture 11: hand area -45..45 / -30..30, axes on, no return pose
        .answer(
            0x04,
            0x2004,
            &hex("000034c2000034420000f0c10000f041ffffffff00050101"),
        )
        .answer(
            0x04,
            0x2504,
            &hex("ffffffff0000000000000000000000000000803f00000000"),
        )
        .answer(0x04, 0x2484, &floats(&[2.5]))
        // capture 08: own start pose, start mode normal, no settings
        .answer(
            0x04,
            0x3884,
            &hex("03000000676666bfcdcc88c1000000000000c03f00000000"),
        )
        .answer(
            0x04,
            0x3e84,
            &hex(
                "00ffffffffffffff80ffffffffffffff00000000ffffffff00000000000000000000000000000000",
            ),
        )
        // capture 11: one view, id 0, "Bereich1"
        .answer(0x04, 0x1344, &hex("00000000"))
        .answer_to(0x04, 0x1504, &[0, 0, 0, 0], &hex("4265726569636831"))
        .answer_to(
            0x04,
            0x1444,
            &[0, 0, 0, 0],
            &hex("00000000cdcc4c3e0000000000000000a370bd3f00000000"),
        )
        // one preset in slot 0 (measured 2026-10-31, #104)
        .buffer(0x0c, block("0100"))
        .buffer(0x0d, preset_entry(0))
        // UVC controls
        .ctrl(0x009a_0901, 3)
        .ctrl(0x009a_0902, 330)
        .ctrl(0x0098_0913, 1)
        .ctrl(0x0098_091c, 9)
        .ctrl(0x0098_0900, 50)
        .ctrl(0x0098_0901, 50)
        .ctrl(0x0098_0902, 50)
        .ctrl(0x0098_0903, 50)
        .ctrl(0x0098_091b, 50)
        .ctrl(0x0098_090c, 1)
        .ctrl(0x0098_091a, 5000)
        .ctrl(0x009a_090d, 20)
        .ctrl(0x009a_090a, 0)
        .ctrl(0x0098_0918, 1)
        .usb(usb())
        .on_air(OnAir::default())
}

/// A sequence for one test, starting at `first`.
pub fn seq(first: u16) -> &'static Sequence {
    Box::leak(Box::new(Sequence::starting_at(first)))
}

/// The model on `replay`, behind the send-table check, without waits; and the replay's log.
pub fn model(replay: Replay) -> (Tiny2Lite, Log) {
    let log = replay.log();
    (Tiny2Lite::checked(replay, seq(1), Timing::instant()), log)
}

/// What was sent, as frames: queries left out (they are reads), commands, buffer writes and
/// UVC writes in order.
pub fn commands(log: &Log) -> Vec<Frame> {
    log.sent()
        .into_iter()
        .filter_map(|s| match s {
            Sent::Xu { selector, data } => {
                let f = classify(&SELECTORS, selector, &data).expect("only frames are sent");
                (!matches!(f, Frame::Query { .. })).then_some(f)
            }
            Sent::Ctrl { id, .. } => Some(Frame::Uvc { control: id }),
        })
        .collect()
}

/// The data of every command sent on selector 2 (bytes 16 and on, as long as the frame says),
/// and the payload of every short command, in order; queries left out.
pub fn payloads(log: &Log) -> Vec<Vec<u8>> {
    log.sent()
        .into_iter()
        .filter_map(|s| match s {
            Sent::Xu { selector: 2, data } if data[1] == 0x25 => {
                let len = usize::from(u16::from_le_bytes([data[12], data[13]]));
                Some(data[16..16 + len].to_vec())
            }
            Sent::Xu { selector: 6, data } => Some(data),
            Sent::Ctrl { value, .. } => Some(value.to_le_bytes().to_vec()),
            _ => None,
        })
        .collect()
}

pub fn framed(command: u16) -> Frame {
    Frame::Framed {
        selector: 2,
        receiver: 0x04,
        command,
    }
}

pub fn framed_to(receiver: u8, command: u16) -> Frame {
    Frame::Framed {
        selector: 2,
        receiver,
        command,
    }
}

pub fn short(command: u8) -> Frame {
    Frame::Short {
        selector: 6,
        command,
    }
}

pub fn uvc(control: u32) -> Frame {
    Frame::Uvc { control }
}
