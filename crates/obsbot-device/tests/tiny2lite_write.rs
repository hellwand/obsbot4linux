// SPDX-License-Identifier: GPL-3.0-or-later

//! The Tiny 2 Lite model writing leaves on a recorded camera (replay transport): each leaf
//! sends the frames of its ledger row and nothing before or after them (spec 2).

mod support;

use obsbot_device::contract::{Camera, CameraError, Ctx};
use serde_json::{Value, json};
use support::*;

fn write(leaf: &str, value: Value) -> (Vec<obsbot_device::contract::Frame>, Vec<Vec<u8>>) {
    let (mut cam, log) = model(recorded());
    cam.apply(leaf, &value, &Ctx::none()).unwrap();
    (commands(&log), payloads(&log))
}

#[test]
fn short_command_leaves_send_their_short_command() {
    let rows: &[(&str, Value, &[u8])] = &[
        ("zoom.field_of_view", json!("narrow"), &[0x04, 0x01, 0x02]),
        (
            "tracking.mode",
            json!("close_up"),
            &[0x16, 0x02, 0x02, 0x02],
        ),
        ("tracking.mode", json!("group"), &[0x16, 0x02, 0x01, 0x00]),
        ("exposure.region", json!("face"), &[0x03, 0x01, 0x01]),
        ("exposure.hdr", json!(false), &[0x01, 0x01, 0x00]),
        ("image.mirror", json!(true), &[0x14, 0x01, 0x01]),
        ("audio.enabled", json!(false), &[0x1c, 0x01, 0x00]),
        (
            "audio.noise_reduction",
            json!("strong"),
            &[0x0a, 0x01, 0x03],
        ),
        ("audio.auto_gain", json!(true), &[0x17, 0x01, 0x01]),
        ("audio.in_sleep", json!(false), &[0x13, 0x01, 0x00]),
        ("led.brightness", json!(2), &[0x1a, 0x01, 0x02]),
        ("led.blink", json!(true), &[0x18, 0x01, 0x01]),
        (
            "sleep.background",
            json!("image"),
            &[0x0e, 0x02, 0x00, 0x01],
        ),
        (
            "sleep.background_mirror",
            json!(true),
            &[0x0e, 0x02, 0x02, 0x01],
        ),
        ("sleep.wake_zoom", json!(2.0), &[0x15, 0x02, 0x64, 33]),
        ("power_on.mode", json!("upper_body"), &[0x19, 0x01, 0x21]),
    ];
    for (leaf, value, bytes) in rows {
        let (frames, data) = write(leaf, value.clone());
        assert_eq!(frames, [short(bytes[0])], "{leaf}");
        assert_eq!(data, [bytes.to_vec()], "{leaf} = {value}");
    }
}

#[test]
fn framed_leaves_send_one_frame_with_their_value() {
    let rows: &[(&str, Value, u8, u16, Vec<u8>)] = &[
        ("gimbal.invert", json!(true), 0x04, 0x3b84, vec![1]),
        ("tracking.profile", json!("headroom"), 0x04, 0x0cc4, vec![1]),
        ("tracking.speed", json!("very_fast"), 0x04, 0x0944, vec![4]),
        ("framing.auto_zoom", json!(true), 0x04, 0x0a44, vec![1]),
        ("framing.size", json!("big_close_up"), 0x04, 0x0ac4, vec![5]),
        ("framing.zoom_speed", json!("fast"), 0x04, 0x0b44, vec![2]),
        (
            "framing.offset.x",
            json!(0.1),
            0x04,
            0x0bc4,
            0.1f32.to_le_bytes().to_vec(),
        ),
        (
            "framing.offset.y",
            json!(-0.16),
            0x04,
            0x0c44,
            (-0.16f32).to_le_bytes().to_vec(),
        ),
        ("framing.part", json!("lower_body"), 0x04, 0x1584, vec![2]),
        ("gestures.target_lock", json!(false), 0x04, 0x30c4, vec![0]),
        ("gestures.zoom", json!(true), 0x04, 0x3144, vec![1]),
        (
            "gestures.zoom_ratio",
            json!(2.0),
            0x04,
            0x3244,
            2.0f32.to_le_bytes().to_vec(),
        ),
        ("gestures.dynamic_zoom", json!(false), 0x04, 0x3344, vec![0]),
        (
            "gestures.direction_change",
            json!(true),
            0x04,
            0x33c4,
            vec![1],
        ),
        ("zone.enabled", json!(true), 0x04, 0x08c4, vec![1]),
        ("zone.follow", json!(false), 0x04, 0x4e04, vec![0]),
        ("hand.side", json!("left"), 0x04, 0x2244, vec![1]),
        ("hand.axes.pan", json!(false), 0x04, 0x2344, vec![0]),
        ("hand.axes.tilt", json!(true), 0x04, 0x23c4, vec![1]),
        (
            "hand.return_after",
            json!(2.5),
            0x04,
            0x2444,
            2.5f32.to_le_bytes().to_vec(),
        ),
        ("hand.move_gimbal", json!(false), 0x04, 0x26c4, vec![0]),
        (
            "focus.mode",
            json!("manual"),
            0x02,
            0x3442,
            3u32.to_le_bytes().to_vec(),
        ),
        (
            "focus.region",
            json!("face"),
            0x02,
            0x3602,
            1u32.to_le_bytes().to_vec(),
        ),
        (
            "audio.volume",
            json!(55),
            0x02,
            0x8102,
            55i16.to_le_bytes().to_vec(),
        ),
    ];
    for (leaf, value, receiver, command, data) in rows {
        let (frames, sent) = write(leaf, value.clone());
        assert_eq!(frames, [framed_to(*receiver, *command)], "{leaf}");
        assert_eq!(sent, std::slice::from_ref(data), "{leaf} = {value}");
    }
}

#[test]
fn uvc_leaves_write_their_control_and_no_other() {
    // no automatic control is switched off first (spec 2): exposure.time, the white balance
    // temperature and focus.position each write their own control; the patch names the mode
    let rows: &[(&str, Value, u32, i32)] = &[
        ("zoom.factor", json!(1.0), 0x009a_090d, 0),
        ("zoom.factor", json!(2.5), 0x009a_090d, 50),
        ("zoom.factor", json!(4.0), 0x009a_090d, 100),
        ("exposure.mode", json!("manual"), 0x009a_0901, 1),
        ("exposure.mode", json!("auto"), 0x009a_0901, 0),
        ("exposure.time", json!(100), 0x009a_0902, 1),
        ("exposure.time", json!(33050), 0x009a_0902, 330),
        ("exposure.time", json!(250000), 0x009a_0902, 2500),
        ("exposure.gain", json!(64), 0x0098_0913, 64),
        ("exposure.compensation", json!(18), 0x0098_091c, 18),
        ("exposure.anti_flicker", json!("60hz"), 0x0098_0918, 2),
        ("image.brightness", json!(0), 0x0098_0900, 0),
        ("image.contrast", json!(100), 0x0098_0901, 100),
        ("image.saturation", json!(10), 0x0098_0902, 10),
        ("image.hue", json!(20), 0x0098_0903, 20),
        ("image.sharpness", json!(30), 0x0098_091b, 30),
        ("image.white_balance.auto", json!(false), 0x0098_090c, 0),
        (
            "image.white_balance.temperature",
            json!(3800),
            0x0098_091a,
            3800,
        ),
        ("focus.position", json!(40), 0x009a_090a, 40),
    ];
    for (leaf, value, control, raw) in rows {
        let (frames, sent) = write(leaf, value.clone());
        assert_eq!(frames, [uvc(*control)], "{leaf}");
        assert_eq!(sent, [raw.to_le_bytes().to_vec()], "{leaf} = {value}");
    }
}

#[test]
fn an_area_axis_sends_min_then_max_in_the_cameras_angles() {
    // capture 11: zone pan -100..100; the asymmetric tilt -30..10 is pitch -10..30 by the conversion of tilt_to_pitch_range
    let (frames, data) = write("zone.area.pan", json!({"min": -100, "max": 100}));
    assert_eq!(frames, [framed(0x4c04), framed(0x4c84)]);
    assert_eq!(
        data,
        [
            (-100f32).to_le_bytes().to_vec(),
            100f32.to_le_bytes().to_vec()
        ]
    );
    let (frames, data) = write("zone.area.tilt", json!({"min": -30, "max": 10}));
    assert_eq!(frames, [framed(0x4d04), framed(0x4d84)]);
    assert_eq!(
        data,
        [
            (-10f32).to_le_bytes().to_vec(),
            30f32.to_le_bytes().to_vec()
        ]
    );
    let (frames, _) = write("hand.area.pan", json!({"min": -45, "max": 45}));
    assert_eq!(frames, [framed(0x2044), framed(0x20c4)]);
    let (frames, data) = write("hand.area.tilt", json!({"min": -30, "max": 10}));
    assert_eq!(frames, [framed(0x2144), framed(0x21c4)]);
    assert_eq!(
        data,
        [
            (-10f32).to_le_bytes().to_vec(),
            30f32.to_le_bytes().to_vec()
        ]
    );
}

#[test]
fn an_area_with_equal_bounds_is_not_sent() {
    // the W0 validator accepts min == max; the carried builder refuses it (U-8, CAM-16)
    let (mut cam, log) = model(recorded());
    let r = cam.apply(
        "zone.area.pan",
        &json!({"min": 10, "max": 10}),
        &Ctx::none(),
    );
    assert!(matches!(r, Err(CameraError::Io(_))), "{r:?}");
    assert!(log.sent().is_empty());
}

#[test]
fn auto_sleep_writes_one_word_with_the_other_half_as_the_camera_has_it() {
    // recorded status: off with 120 s (word -120)
    let (mut cam, log) = model(recorded());
    cam.apply("sleep.auto.enabled", &json!(true), &Ctx::none())
        .unwrap();
    assert_eq!(
        payloads(&log),
        [vec![0x0b, 0x02, 0x78, 0x00]],
        "on with the 120 s kept"
    );
    // the word the model wrote is the one the next leaf of a patch starts from
    cam.apply("sleep.auto.seconds", &json!(600), &Ctx::none())
        .unwrap();
    assert_eq!(payloads(&log)[1], [0x0b, 0x02, 0x58, 0x02], "on, 600 s");
    cam.apply("sleep.auto.enabled", &json!(false), &Ctx::none())
        .unwrap();
    assert_eq!(
        payloads(&log)[2],
        [0x0b, 0x02, 0xa8, 0xfd],
        "off, 600 s kept (-600)"
    );
    // seconds alone while off keeps it off (4.3)
    cam.apply("sleep.auto.seconds", &json!(30), &Ctx::none())
        .unwrap();
    assert_eq!(
        payloads(&log)[3],
        [0x0b, 0x02, 0xe2, 0xff],
        "off, 30 s (-30)"
    );
    assert_eq!(commands(&log), vec![short(0x0b); 4]);
}

#[test]
fn auto_sleep_reads_the_status_block_when_it_has_no_word_yet_and_after_a_status_read() {
    let (mut cam, log) = model(recorded());
    assert_eq!(cam.memory().sleep_word, None);
    cam.apply("sleep.auto.seconds", &json!(300), &Ctx::none())
        .unwrap();
    assert_eq!(
        payloads(&log),
        [vec![0x0b, 0x02, 0xd4, 0xfe]],
        "off with 300 s (-300)"
    );
    // a status read replaces what the model wrote with what the camera reports
    let mut on = block(STATUS);
    on[0x0a] = 0x1e;
    on[0x0b] = 0x00;
    let (mut cam, log) = model(recorded().buffer(6, on));
    cam.read("status", &Ctx::none()).unwrap();
    cam.apply("sleep.auto.enabled", &json!(false), &Ctx::none())
        .unwrap();
    assert_eq!(payloads(&log), [vec![0x0b, 0x02, 0xe2, 0xff]]);
}

#[test]
fn auto_sleep_with_0_s_stored_sends_nothing_for_the_switch_alone() {
    // the camera cannot store "off" with 0 s, and "on with 0 s" was never recorded (PX-7);
    // obsbot4linux writes no time of its own (spec 2)
    let mut zero = block(STATUS);
    zero[0x0a] = 0;
    zero[0x0b] = 0;
    for on in [true, false] {
        let (mut cam, log) = model(recorded().buffer(6, zero));
        let r = cam.apply("sleep.auto.enabled", &json!(on), &Ctx::none());
        assert!(matches!(r, Err(CameraError::Io(_))), "{r:?}");
        assert!(commands(&log).is_empty(), "{on}");
    }
}

#[test]
fn the_start_mode_writes_back_the_block_it_read_with_byte_0_changed() {
    let (mut cam, log) = model(recorded());
    cam.apply("power_on.start.mode", &json!("unset"), &Ctx::none())
        .unwrap();
    assert_eq!(commands(&log), [framed(0x3e44)]);
    // capture 08: the block of 0x3e84 with start mode normal, written with "not set"
    assert_eq!(
        payloads(&log),
        [hex(
            "feffffffffffffff80ffffffffffffff00000000ffffffff00000000000000000000000000000000"
        )]
    );
    // a short block is not written back
    let (mut cam, log) = model(recorded().answer(0x04, 0x3e84, &[0xfe; 39]));
    assert!(matches!(
        cam.apply("power_on.start.mode", &json!("normal"), &Ctx::none()),
        Err(CameraError::NoAnswer(_))
    ));
    assert!(commands(&log).is_empty());
}

#[test]
fn a_value_the_builder_refuses_sends_nothing() {
    // the validator would refuse these; the model still sends nothing for them
    let (mut cam, log) = model(recorded());
    for (leaf, value) in [
        ("led.brightness", json!(4)),
        ("framing.offset.x", json!(0.5)),
        ("tracking.mode", json!("off")),
        ("gimbal.pan", json!(1)),
    ] {
        assert!(
            cam.apply(leaf, &value, &Ctx::none()).is_err(),
            "{leaf} = {value}"
        );
    }
    assert!(log.sent().is_empty());
}

#[test]
fn a_write_on_a_gone_camera_fails_with_gone() {
    let (mut cam, _) = model(recorded().gone());
    assert_eq!(
        cam.apply("led.brightness", &json!(1), &Ctx::none()),
        Err(CameraError::Gone)
    );
}
