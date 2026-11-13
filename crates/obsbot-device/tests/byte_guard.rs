// SPDX-License-Identifier: GPL-3.0-or-later

//! Byte guard of the groundwork stage (rebuild plan, stage G): every frame and short command the
//! builders emit for a fixed list of calls, one line per call, compared with
//! `tests/fixtures/byte-guard.txt`. That file was written from the 0.13 code before the
//! workspace move, so a move that changes one byte that would go to the camera fails here.
//!
//! `BYTE_GUARD_BLESS=1 cargo test --test byte_guard` writes the file: once when this test is
//! added, and on `main` in a change that alters what a builder sends on purpose (that change
//! lists the changed lines). A move never writes it; every other run compares.

use clap::ValueEnum;
use obsbot_device::commands::{self as cmd, *};
use obsbot_device::frame::{self, FRAME_SIZE};
use obsbot_device::presets::{self, Pose, StartupAi};
use std::fmt::Display;

const FIXTURE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/byte-guard.txt");
const SEQ: u16 = 0x1234;
const SEQS: [u16; 4] = [SEQ, SEQ + 1, SEQ + 2, SEQ + 3];

/// How one builder result is written into the fixture.
trait Show {
    fn show(&self) -> String;
}

impl<const N: usize> Show for [u8; N] {
    fn show(&self) -> String {
        hex::encode(self)
    }
}

impl<const N: usize, const M: usize> Show for [[u8; N]; M] {
    fn show(&self) -> String {
        self.iter().map(hex::encode).collect::<Vec<_>>().join(" ")
    }
}

impl Show for u8 {
    fn show(&self) -> String {
        self.to_string()
    }
}

impl Show for u16 {
    fn show(&self) -> String {
        format!("{self:#06x}")
    }
}

impl Show for bool {
    fn show(&self) -> String {
        self.to_string()
    }
}

impl<T: Show> Show for Option<T> {
    fn show(&self) -> String {
        self.as_ref().map_or("none".into(), Show::show)
    }
}

impl<T: Show, E: Display> Show for Result<T, E> {
    fn show(&self) -> String {
        match self {
            Ok(v) => v.show(),
            Err(e) => format!("error: {e}"),
        }
    }
}

fn rec(out: &mut Vec<String>, label: String, value: impl Show) {
    out.push(format!("{label} = {}", value.show()));
}

/// Poses at and beyond every bound the pose builders check, and an asymmetric one.
const POSES: [(f32, f32, f32); 10] = [
    (10.5, -5.25, 2.0),
    (0.0, 0.0, 1.0),
    (-150.0, -70.0, 1.0),
    (150.0, 90.0, 4.0),
    (150.1, 0.0, 1.0),
    (0.0, 90.5, 1.0),
    (0.0, -70.5, 1.0),
    (0.0, 0.0, 4.01),
    (0.0, 0.0, 0.99),
    (f32::NAN, 0.0, 1.0),
];

fn poses() -> impl Iterator<Item = Pose> {
    POSES
        .into_iter()
        .map(|(pan, tilt, zoom)| Pose { pan, tilt, zoom })
}

/// Area ranges (min, max) in the camera's angles, at and beyond the bounds, and wrong ways round.
const RANGES: [(f32, f32); 11] = [
    (-45.0, 45.0),
    (-150.0, 150.0),
    (-150.5, 0.0),
    (0.0, 150.5),
    (-90.0, 70.0),
    (-90.5, 0.0),
    (0.0, 70.5),
    (-30.0, 10.0),
    (10.0, 10.0),
    (10.0, -10.0),
    (f32::NAN, 1.0),
];

type Short3 = fn(bool) -> [u8; 3];
type Framed = fn(bool, u16) -> [u8; FRAME_SIZE];
type SeqOnly = fn(u16) -> [u8; FRAME_SIZE];

fn record() -> Vec<String> {
    let mut out = Vec::new();
    let o = &mut out;

    // frame codec
    for seq in [0x0000, 0x0001, SEQ, 0xffff] {
        let q = frame::query_frame(0x04, 0x0104, seq);
        rec(
            o,
            format!("frame::query_frame(0x04, 0x0104, {seq:#06x})"),
            q,
        );
        let f = frame::frame(0x02, 0xa0c2, &[1, 0, 0, 0], seq);
        rec(
            o,
            format!("frame::frame(0x02, 0xa0c2, [1 0 0 0], {seq:#06x})"),
            f,
        );
        let w = frame::query_frame_with(0x04, 0x1504, &[7, 0, 0, 0], seq);
        rec(
            o,
            format!("frame::query_frame_with(0x04, 0x1504, [7 0 0 0], {seq:#06x})"),
            w,
        );
    }
    let full = frame::frame(0x04, 0x3e44, &[0xa5; frame::MAX_DATA], SEQ);
    rec(
        o,
        "frame::frame(0x04, 0x3e44, [0xa5; MAX_DATA])".into(),
        full,
    );
    rec(
        o,
        "frame::crc16_usb(\"123456789\")".into(),
        frame::crc16_usb(b"123456789"),
    );

    // short commands (selector 6)
    let short3: [(&str, Short3); 6] = [
        ("hdr", cmd::hdr),
        ("led_blink", cmd::led_blink),
        ("mirror", cmd::mirror),
        ("mic_in_sleep", cmd::mic_in_sleep),
        ("auto_gain", cmd::auto_gain),
        ("microphone", cmd::microphone),
    ];
    for (name, f) in short3 {
        for on in [false, true] {
            rec(o, format!("cmd::{name}({on})"), f(on));
        }
    }
    for on in [false, true] {
        rec(
            o,
            format!("cmd::sleep_background_mirror({on})"),
            cmd::sleep_background_mirror(on),
        );
    }
    for m in AiMode::value_variants() {
        rec(o, format!("cmd::ai_mode({m:?})"), cmd::ai_mode(*m));
        rec(
            o,
            format!("cmd::default_work_mode({m:?})"),
            cmd::default_work_mode(*m),
        );
    }
    for s in [i16::MIN, -600, -1, 0, 1, 600, i16::MAX] {
        rec(o, format!("cmd::auto_sleep({s})"), cmd::auto_sleep(s));
    }
    for s in [0u16, 1, 600, 32767, 32768, u16::MAX] {
        rec(
            o,
            format!("cmd::auto_sleep_off({s})"),
            cmd::auto_sleep_off(s),
        );
    }
    for b in SleepBackground::value_variants() {
        rec(
            o,
            format!("cmd::sleep_background({b:?})"),
            cmd::sleep_background(*b),
        );
    }
    for b in 0..=4u8 {
        rec(o, format!("cmd::led({b})"), cmd::led(b));
    }
    for p in [0u8, 1, 50, 100, 101, 255] {
        rec(o, format!("cmd::wake_zoom({p})"), cmd::wake_zoom(p));
    }
    for f in [0.99f32, 1.0, 1.5, 2.5, 4.0, 4.01, f32::NAN] {
        rec(
            o,
            format!("cmd::wake_zoom_percent({f})"),
            cmd::wake_zoom_percent(f),
        );
    }
    for v in FieldOfView::value_variants() {
        rec(
            o,
            format!("cmd::field_of_view({v:?})"),
            cmd::field_of_view(*v),
        );
    }
    for v in NoiseReduction::value_variants() {
        rec(
            o,
            format!("cmd::noise_reduction({v:?})"),
            cmd::noise_reduction(*v),
        );
    }
    for v in PickupDistance::value_variants() {
        rec(
            o,
            format!("cmd::pickup_distance({v:?})"),
            cmd::pickup_distance(*v),
        );
    }
    for v in ExposureMode::value_variants() {
        rec(
            o,
            format!("cmd::exposure_mode_select({v:?})"),
            cmd::exposure_mode_select(*v),
        );
    }
    rec(o, "cmd::startup_go()".into(), cmd::startup_go());

    // framed commands (selector 2)
    let framed: [(&str, Framed); 9] = [
        ("invert", cmd::invert),
        ("zone_tracking", cmd::zone_tracking),
        ("hand_pan", cmd::hand_pan),
        ("hand_tilt", cmd::hand_tilt),
        ("hand_gimbal", cmd::hand_gimbal),
        ("autofocus_mode", cmd::autofocus_mode),
        ("zone_follow", cmd::zone_follow),
        ("sleep", cmd::sleep),
        ("auto_zoom", cmd::auto_zoom),
    ];
    for (name, f) in framed {
        for on in [false, true] {
            rec(o, format!("cmd::{name}({on})"), f(on, SEQ));
        }
    }
    let seq_only: [(&str, SeqOnly); 6] = [
        ("zone_return_go", cmd::zone_return_go),
        ("hand_return_go", cmd::hand_return_go),
        ("hand_return_reset", cmd::hand_return_reset),
        ("factory_reset", cmd::factory_reset),
        ("gimbal_stop", cmd::gimbal_stop),
        ("zoom_stop", cmd::zoom_stop),
    ];
    for (name, f) in seq_only {
        rec(o, format!("cmd::{name}()"), f(SEQ));
    }
    for v in TrackingSpeed::value_variants() {
        rec(
            o,
            format!("cmd::tracking_speed({v:?})"),
            cmd::tracking_speed(*v, SEQ),
        );
    }
    for v in SpeedMode::value_variants() {
        rec(
            o,
            format!("cmd::speed_mode({v:?})"),
            cmd::speed_mode(*v, SEQ),
        );
    }
    for v in FramingSize::value_variants() {
        rec(
            o,
            format!("cmd::framing_size({v:?})"),
            cmd::framing_size(*v, SEQ),
        );
    }
    for v in AutoZoomSpeed::value_variants() {
        rec(
            o,
            format!("cmd::auto_zoom_speed({v:?})"),
            cmd::auto_zoom_speed(*v, SEQ),
        );
    }
    for v in PartMode::value_variants() {
        rec(o, format!("cmd::part_mode({v:?})"), cmd::part_mode(*v, SEQ));
    }
    for v in HandSide::value_variants() {
        rec(o, format!("cmd::hand_side({v:?})"), cmd::hand_side(*v, SEQ));
    }
    for v in AutofocusMode::value_variants() {
        rec(
            o,
            format!("cmd::autofocus_mode_select({v:?})"),
            cmd::autofocus_mode_select(*v, SEQ),
        );
    }
    for g in Gesture::value_variants() {
        for on in [false, true] {
            rec(
                o,
                format!("cmd::gesture({g:?}, {on})"),
                cmd::gesture(*g, on, SEQ),
            );
        }
    }
    for r in [0.99f32, 1.0, 2.5, 4.0, 4.01, f32::NAN] {
        rec(
            o,
            format!("cmd::gesture_zoom_ratio({r})"),
            cmd::gesture_zoom_ratio(r, SEQ),
        );
    }
    for s in [-1.0f32, -0.0, 0.0, 2.5, 600.0, f32::INFINITY, f32::NAN] {
        rec(
            o,
            format!("cmd::hand_return_after({s})"),
            cmd::hand_return_after(s, SEQ),
        );
    }
    for v in [0u8, 1, 50, 100, 101, 255] {
        rec(
            o,
            format!("cmd::audio_volume({v})"),
            cmd::audio_volume(v, SEQ),
        );
    }
    for x in [-0.01f32, 0.0, 0.25, 1.0, 1.01] {
        for y in [-0.01f32, 0.0, 0.75, 1.0, 1.01] {
            rec(
                o,
                format!("cmd::focus_point({x}, {y})"),
                cmd::focus_point(x, y, SEQ),
            );
            rec(
                o,
                format!("cmd::view_center({x}, {y})"),
                cmd::view_center(x, y, SEQ),
            );
        }
    }
    for r in RANGES {
        for (yaw, pitch) in [(r, (-30.0, 10.0)), ((-45.0, 45.0), r)] {
            rec(
                o,
                format!("cmd::zone_area({yaw:?}, {pitch:?})"),
                cmd::zone_area(yaw, pitch, SEQS),
            );
            rec(
                o,
                format!("cmd::hand_area({yaw:?}, {pitch:?})"),
                cmd::hand_area(yaw, pitch, SEQS),
            );
        }
    }
    rec(
        o,
        "cmd::hand_area_reset()".into(),
        cmd::hand_area_reset(SEQS),
    );
    let axes = [
        None,
        Some(-150.0f32),
        Some(-70.0),
        Some(0.0),
        Some(12.5),
        Some(90.0),
        Some(150.0),
        Some(150.5),
        Some(-90.5),
    ];
    for yaw in axes {
        for pitch in axes {
            rec(
                o,
                format!("cmd::gimbal_pose({yaw:?}, {pitch:?})"),
                cmd::gimbal_pose(yaw, pitch, SEQ),
            );
        }
    }
    for (yaw, pitch) in [
        (None, None),
        (Some(12.5f32), None),
        (None, Some(-5.0f32)),
        (Some(12.5), Some(-5.0)),
        (Some(150.5), Some(0.0)),
    ] {
        for speed in [0.99f32, 1.0, 45.0, 180.0, 180.5, f32::NAN] {
            let f = cmd::gimbal_pose_speed(yaw, pitch, speed, SEQ);
            rec(
                o,
                format!("cmd::gimbal_pose_speed({yaw:?}, {pitch:?}, {speed})"),
                f,
            );
        }
    }
    for pan in [-178.5f32, -178.0, -0.0, 0.0, 12.5, 178.0, 178.5, f32::NAN] {
        for tilt in [-178.5f32, -178.0, -0.0, 0.0, 5.0, 178.0, 178.5] {
            let f = cmd::gimbal_rate(pan, tilt, false, false, SEQ);
            rec(
                o,
                format!("cmd::gimbal_rate({pan}, {tilt}, false, false)"),
                f,
            );
        }
    }
    for (mirror, invert) in [(true, false), (false, true), (true, true)] {
        let f = cmd::gimbal_rate(12.5, 5.0, mirror, invert, SEQ);
        rec(
            o,
            format!("cmd::gimbal_rate(12.5, 5, {mirror}, {invert})"),
            f,
        );
    }
    for speed in [0i32, 1, 10, 30, 50, i32::MAX] {
        for ratio in [0.99f32, 1.0, 2.5, 4.0, 4.01] {
            rec(
                o,
                format!("cmd::zoom_to({speed}, {ratio})"),
                cmd::zoom_to(speed, ratio, SEQ),
            );
        }
    }
    for vertical in [false, true] {
        for f in [-0.17f32, -0.16, -0.0, 0.0, 0.1, 0.16, 0.17, f32::NAN] {
            let b = cmd::framing_offset(vertical, f, SEQ);
            rec(o, format!("cmd::framing_offset({vertical}, {f})"), b);
        }
    }
    for pan in [-150.5f32, -150.0, 0.0, 150.0, 150.5] {
        for tilt in [-70.5f32, -70.0, 0.0, 90.0, 90.5] {
            rec(
                o,
                format!("cmd::gimbal_angles_ok({pan}, {tilt})"),
                cmd::gimbal_angles_ok(pan, tilt),
            );
        }
    }

    // presets, zone views, start pose and return positions
    let names = [
        "",
        "Desk",
        "Bereich1",
        "äöü",
        "12345678901234567890123456789012",
        "123456789012345678901234567890123",
    ];
    for slot in [0u8, 1, 2, 3, 255] {
        for p in poses() {
            rec(
                o,
                format!("presets::create({slot}, {p:?})"),
                presets::create(slot, p, SEQ),
            );
            rec(
                o,
                format!("presets::update({slot}, {p:?})"),
                presets::update(slot, p, SEQ),
            );
        }
        for name in names {
            rec(
                o,
                format!("presets::rename({slot}, {name:?})"),
                presets::rename(slot, name, SEQ),
            );
        }
        rec(
            o,
            format!("presets::delete({slot})"),
            presets::delete(slot, SEQ, SEQ + 1),
        );
        rec(
            o,
            format!("presets::recall({slot})"),
            presets::recall(slot, SEQ),
        );
    }
    for id in [0u32, 1, 7, 255, u32::MAX] {
        for p in poses() {
            rec(
                o,
                format!("presets::view_add({id}, {p:?})"),
                presets::view_add(id, p, SEQ),
            );
        }
        for name in names {
            rec(
                o,
                format!("presets::view_rename({id}, {name:?})"),
                presets::view_rename(id, name, SEQ),
            );
        }
        rec(
            o,
            format!("presets::view_remove({id})"),
            presets::view_remove(id, SEQ),
        );
        rec(
            o,
            format!("presets::view_go({id})"),
            presets::view_go(id, SEQ),
        );
        for command in [0x1504u16, 0x1444] {
            let q = presets::view_query(command, id, SEQ);
            rec(o, format!("presets::view_query({command:#06x}, {id})"), q);
        }
    }
    for source in [0u32, 1, 2, 3, 4] {
        for p in poses() {
            let f = presets::startup_pose(source, p, SEQ);
            rec(o, format!("presets::startup_pose({source}, {p:?})"), f);
        }
    }
    for p in poses() {
        rec(
            o,
            format!("presets::zone_return_set({p:?})"),
            presets::zone_return_set(p, SEQ),
        );
        rec(
            o,
            format!("presets::hand_return_set({p:?})"),
            presets::hand_return_set(p, SEQ),
        );
    }
    for ai in StartupAi::value_variants() {
        let f = presets::startup_picture_settings(ai.byte(), SEQ);
        rec(o, format!("presets::startup_picture_settings({ai:?})"), f);
    }
    let block: Vec<u8> = (0u8..48).collect();
    for len in [0usize, 39, 40, 48] {
        for ai in [0x00u8, 0xfe, 0xff] {
            let f = presets::startup_with_ai(&block[..len], ai, SEQ);
            rec(
                o,
                format!("presets::startup_with_ai(0..{len}, {ai:#04x})"),
                f,
            );
        }
    }
    out
}

#[test]
fn builders_emit_the_recorded_bytes() {
    let now = record().join("\n") + "\n";
    if std::env::var_os("BYTE_GUARD_BLESS").is_some() {
        std::fs::write(FIXTURE, &now).unwrap();
        return;
    }
    let recorded = std::fs::read_to_string(FIXTURE).expect("tests/fixtures/byte-guard.txt");
    let (now, recorded): (Vec<&str>, Vec<&str>) =
        (now.lines().collect(), recorded.lines().collect());
    assert_eq!(
        now.len(),
        recorded.len(),
        "the number of recorded calls changed"
    );
    for (n, r) in now.iter().zip(&recorded) {
        assert_eq!(n, r, "a builder emits other bytes than before the move");
    }
}
