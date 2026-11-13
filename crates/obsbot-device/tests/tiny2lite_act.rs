// SPDX-License-Identifier: GPL-3.0-or-later

//! The Tiny 2 Lite model's actions and checks on a recorded camera (replay transport): each
//! action sends the frames of its ledger row and nothing else (spec 2, 15.1 rows 88 and 90).

mod support;

use obsbot_device::contract::{Camera, CameraError, Ctx};
use obsbot_proto::{ErrorCode, Params, State};
use serde_json::{Value, json};
use support::*;

fn params(v: Value) -> Params {
    v.as_object().unwrap().clone()
}

/// The published state the actions see: one preset in slot 0, one view with id 0, mirror and
/// invert off.
fn state() -> State {
    let mut s = State::new(true);
    s.set(
        "presets",
        json!([{"slot": 0, "name": "Desk", "pose": {"pan": 12.5, "tilt": 3.3, "zoom": 1.5}}]),
    );
    s.set(
        "views",
        json!([{"id": 0, "name": "Bereich1", "pose": {"pan": 0.2, "tilt": 0, "zoom": 1.48}}]),
    );
    s.set("image.mirror", json!(false));
    s.set("gimbal.invert", json!(false));
    s
}

fn run(
    action: &str,
    p: Value,
    st: &State,
) -> (
    Option<Params>,
    Vec<obsbot_device::contract::Frame>,
    Vec<Vec<u8>>,
) {
    let (mut cam, log) = model(recorded());
    let p = params(p);
    cam.check(action, &p, st).unwrap();
    let result = cam.act(action, &p, st, &Ctx::none()).unwrap();
    (result, commands(&log), payloads(&log))
}

fn floats(values: &[f32]) -> Vec<u8> {
    values.iter().flat_map(|v| v.to_le_bytes()).collect()
}

#[test]
fn a_gimbal_move_sends_only_the_move() {
    // spec 2 and 15.1 row 88: no short 16 02 (tracking static) before the move
    let (_, frames, data) = run("gimbal.move", json!({"pan": 100, "tilt": -30}), &state());
    assert_eq!(frames, [framed(0x6444)]);
    // roll kept (-1000), pitch +30 (tilt -30 is down), yaw 100
    assert_eq!(data, [floats(&[-1000.0, 30.0, 100.0])]);
    // a missing axis keeps its position (4.4): -1000 in its place
    let (_, _, data) = run("gimbal.move", json!({"tilt": 20}), &state());
    assert_eq!(data, [floats(&[-1000.0, -20.0, -1000.0])]);
    // with speed: 0x6544, three speeds then the targets
    let (_, frames, data) = run(
        "gimbal.move",
        json!({"pan": 20, "tilt": 10, "speed": 5}),
        &state(),
    );
    assert_eq!(frames, [framed(0x6544)]);
    assert_eq!(data, [floats(&[5.0, 5.0, 5.0, -1000.0, -10.0, 20.0])]);
}

#[test]
fn a_gimbal_move_needs_an_axis() {
    let (cam, _) = model(recorded());
    let refused = cam
        .check("gimbal.move", &params(json!({"speed": 5})), &state())
        .unwrap_err();
    assert_eq!(refused.code, ErrorCode::Invalid);
}

#[test]
fn a_gimbal_turn_sends_only_the_turn_with_the_yaw_sign_of_mirror_xor_invert() {
    let (_, frames, data) = run(
        "gimbal.turn",
        json!({"pan_speed": 5, "keep": true}),
        &state(),
    );
    assert_eq!(
        frames,
        [framed(0x6484)],
        "no tracking switch before it (15.1 row 88)"
    );
    // mirror and invert off: a positive pan speed is a negative yaw on the wire (#98)
    assert_eq!(data, [floats(&[0.0, 0.0, -5.0])]);
    let mut mirrored = state();
    mirrored.set("image.mirror", json!(true));
    let (_, _, data) = run(
        "gimbal.turn",
        json!({"pan_speed": 5, "tilt_speed": 3}),
        &mirrored,
    );
    assert_eq!(data, [floats(&[0.0, -3.0, 5.0])]);
    let mut both = mirrored.clone();
    both.set("gimbal.invert", json!(true));
    let (_, _, data) = run("gimbal.turn", json!({"pan_speed": 5}), &both);
    assert_eq!(data, [floats(&[0.0, 0.0, -5.0])]);
}

#[test]
fn a_gimbal_turn_is_refused_until_mirror_and_invert_are_read() {
    let (cam, _) = model(recorded());
    let mut unread = state();
    unread.set("gimbal.invert", Value::Null);
    let refused = cam
        .check("gimbal.turn", &params(json!({"pan_speed": 5})), &unread)
        .unwrap_err();
    assert_eq!(
        (refused.code, refused.path.as_deref()),
        (ErrorCode::Conflict, Some("gimbal.invert"))
    );
    let refused = cam
        .check("gimbal.turn", &params(json!({"keep": true})), &state())
        .unwrap_err();
    assert_eq!(refused.code, ErrorCode::Invalid, "no speed at all");
}

#[test]
fn stops_and_zoom_moves() {
    let (_, frames, _) = run("gimbal.stop", json!({}), &state());
    assert_eq!(frames, [framed(0x6704)]);
    let (_, frames, _) = run("zoom.stop", json!({}), &state());
    assert_eq!(frames, [framed(0x68c4)]);
    for (speed, raw) in [("slow", 10i32), ("medium", 30), ("fast", 50)] {
        let (_, frames, data) = run(
            "zoom.move",
            json!({"factor": 2.5, "speed": speed}),
            &state(),
        );
        assert_eq!(frames, [framed(0x6884)]);
        let mut want = raw.to_le_bytes().to_vec();
        want.extend(2.5f32.to_le_bytes());
        assert_eq!(data, [want], "{speed}");
    }
}

#[test]
fn a_preset_saved_without_a_name_gets_none() {
    // 15.1 row 90: one frame, no rename; slot 1 is empty, so it is created at the live pose
    let (_, frames, data) = run("preset.save", json!({"slot": 1}), &state());
    assert_eq!(frames, [framed(0x3944)]);
    let mut want = 1u32.to_le_bytes().to_vec();
    // live pose of the recording: pan 25, tilt 10 up (pitch -10), zoom 1.6
    want.extend(floats(&[25.0, -10.0, 0.0, 1.6, -1000.0]));
    assert_eq!(data, [want]);
}

#[test]
fn a_preset_saved_with_a_name_is_saved_then_renamed() {
    // slot 0 holds a preset: the update frame, then the name
    let (_, frames, data) = run(
        "preset.save",
        json!({"slot": 0, "name": "Door", "pose": {"pan": 1, "tilt": 2, "zoom": 1}}),
        &state(),
    );
    assert_eq!(frames, [framed(0x3e04), framed(0x3a84)]);
    let mut name = 0u32.to_le_bytes().to_vec();
    name.extend(b"Door");
    assert_eq!(data[1], name);
}

#[test]
fn a_name_the_camera_cannot_take_stops_the_save_before_any_frame() {
    // audit: the pose went out before the name was checked
    let (mut cam, log) = model(recorded());
    let p = params(json!({"slot": 0, "name": "Küche"}));
    assert!(cam.act("preset.save", &p, &state(), &Ctx::none()).is_err());
    assert!(log.sent().is_empty(), "not even the live pose was read");
    let refused = cam
        .check(
            "preset.save",
            &params(json!({"slot": 0, "name": ""})),
            &state(),
        )
        .unwrap_err();
    assert_eq!(refused.code, ErrorCode::OutOfRange);
}

#[test]
fn slot_and_id_actions_are_refused_for_an_empty_slot_an_unknown_id_or_an_unread_list() {
    let (cam, _) = model(recorded());
    let conflict =
        |action: &str, p: Value, st: &State| cam.check(action, &params(p), st).unwrap_err().code;
    for action in ["preset.recall", "preset.delete", "power_on.use_preset"] {
        assert_eq!(
            conflict(action, json!({"slot": 2}), &state()),
            ErrorCode::Conflict,
            "{action}"
        );
    }
    assert_eq!(
        conflict("preset.rename", json!({"slot": 2, "name": "x"}), &state()),
        ErrorCode::Conflict
    );
    for action in ["view.go", "view.delete"] {
        assert_eq!(
            conflict(action, json!({"id": 5}), &state()),
            ErrorCode::Conflict
        );
    }
    let mut unread = state();
    unread.unread.push("presets".into());
    unread.unread.push("views".into());
    assert_eq!(
        conflict("preset.save", json!({"slot": 0}), &unread),
        ErrorCode::Conflict
    );
    assert_eq!(
        conflict("preset.recall", json!({"slot": 0}), &unread),
        ErrorCode::Conflict
    );
    assert_eq!(
        conflict("view.save", json!({"name": "x"}), &unread),
        ErrorCode::Conflict
    );
    // a used slot and a known id pass
    assert!(
        cam.check("preset.recall", &params(json!({"slot": 0})), &state())
            .is_ok()
    );
    assert!(
        cam.check("view.go", &params(json!({"id": 0})), &state())
            .is_ok()
    );
}

#[test]
fn preset_actions() {
    let (_, frames, data) = run("preset.recall", json!({"slot": 0}), &state());
    assert_eq!(frames, [framed(0x39c4)]);
    let mut want = 0u32.to_le_bytes().to_vec();
    want.extend(floats(&[1.0, 1.0, 1.0, 1.0]));
    assert_eq!(data, [want]);
    let (_, frames, _) = run(
        "preset.rename",
        json!({"slot": 0, "name": "Desk"}),
        &state(),
    );
    assert_eq!(frames, [framed(0x3a84)]);
    let (_, frames, data) = run("preset.delete", json!({"slot": 0}), &state());
    assert_eq!(frames, [framed(0x3984), framed(0x3d84)]);
    assert_eq!(data[0], 0u32.to_le_bytes());
}

#[test]
fn a_view_is_saved_under_the_lowest_free_id_and_named() {
    let (result, frames, data) = run("view.save", json!({"name": "Door"}), &state());
    assert_eq!(result, Some(params(json!({"id": 1}))));
    assert_eq!(frames, [framed(0x1384), framed(0x1544)]);
    assert_eq!(data[0][..4], 1u32.to_le_bytes());
    let mut name = 1u32.to_le_bytes().to_vec();
    name.extend(b"Door");
    assert_eq!(data[1], name);
    let (_, frames, _) = run("view.go", json!({"id": 0}), &state());
    assert_eq!(frames, [framed(0x1604)]);
    let (_, frames, _) = run("view.rename", json!({"id": 0, "name": "Desk"}), &state());
    assert_eq!(frames, [framed(0x1544)]);
    let (_, frames, _) = run("view.delete", json!({"id": 0}), &state());
    assert_eq!(frames, [framed(0x13c4)]);
}

#[test]
fn return_poses_go_returns_and_the_hand_area_reset() {
    let (_, frames, data) = run("zone.return_here", json!({}), &state());
    assert_eq!(frames, [framed(0x4e84)]);
    let mut want = 0u32.to_le_bytes().to_vec();
    want.extend(floats(&[25.0, -10.0, 0.0, 1.6, 0.0]));
    assert_eq!(data, [want.clone()]);
    let (_, frames, data) = run("hand.return_here", json!({}), &state());
    assert_eq!(frames, [framed(0x24c4)]);
    assert_eq!(data, [want]);
    for (action, command) in [
        ("zone.go_return", 0x4f44),
        ("hand.go_return", 0x2584),
        ("hand.return_clear", 0x2544),
    ] {
        let (_, frames, data) = run(action, json!({}), &state());
        assert_eq!(frames, [framed(command)], "{action}");
        assert_eq!(data, [Vec::<u8>::new()], "{action}: no data");
    }
    let (_, frames, _) = run("hand.area_reset", json!({}), &state());
    assert_eq!(
        frames,
        [
            framed(0x25c4),
            framed(0x2604),
            framed(0x2644),
            framed(0x2684)
        ]
    );
}

#[test]
fn the_start_pose_is_set_and_the_block_written_back_unchanged() {
    let block =
        hex("00ffffffffffffff80ffffffffffffff00000000ffffffff00000000000000000000000000000000");
    let (_, frames, data) = run(
        "power_on.set_pose",
        json!({"pose": {"pan": -3, "tilt": 17.1, "zoom": 1}}),
        &state(),
    );
    assert_eq!(frames, [framed(0x3ec4), framed(0x3e44)]);
    let mut pose = 3u32.to_le_bytes().to_vec();
    pose.extend(floats(&[-3.0, -17.1, 0.0, 1.0, 0.0]));
    assert_eq!(
        data,
        [pose, block.clone()],
        "the start mode and the settings stay (4.4)"
    );
    let (_, frames, data) = run("power_on.use_current", json!({}), &state());
    assert_eq!(frames, [framed(0x3ec4), framed(0x3e44)]);
    assert_eq!(data[1], block);
    let (_, frames, data) = run("power_on.use_preset", json!({"slot": 0}), &state());
    assert_eq!(frames, [framed(0x3ec4), framed(0x3e44)]);
    let mut pose = 0u32.to_le_bytes().to_vec();
    pose.extend(floats(&[12.5, -3.3, 0.0, 1.5, 0.0]));
    assert_eq!(
        data[0], pose,
        "the slot's pose from the presets list, lead = slot"
    );
    let (_, frames, data) = run("power_on.go", json!({}), &state());
    assert_eq!(
        (frames, data),
        (vec![short(0x00)], vec![vec![0x00, 0x01, 0x01]])
    );
}

#[test]
fn a_start_block_with_settings_is_written_back_with_the_settings() {
    // capture 08b: a block with the start mode and picture settings set
    let block =
        hex("feffff010000ff010c32001000ffd80e00000000ffffffff00000000000000000000000000000000");
    let (mut cam, log) = model(recorded().answer(0x04, 0x3e84, &block));
    let p = params(json!({"pose": {"pan": 0, "tilt": 0, "zoom": 1}}));
    cam.act("power_on.set_pose", &p, &state(), &Ctx::none())
        .unwrap();
    assert_eq!(
        payloads(&log)[1],
        block,
        "the start mode and the settings stay (4.4)"
    );
}

#[test]
fn a_short_initial_state_block_stops_the_start_pose_before_any_frame() {
    let (mut cam, log) = model(recorded().answer(0x04, 0x3e84, &[0; 10]));
    let p = params(json!({"pose": {"pan": 0, "tilt": 0, "zoom": 1}}));
    assert!(matches!(
        cam.act("power_on.set_pose", &p, &state(), &Ctx::none()),
        Err(CameraError::NoAnswer(_))
    ));
    assert!(commands(&log).is_empty());
}

#[test]
fn sleep_wake_focus_point_and_factory_reset() {
    let (_, frames, data) = run("sleep", json!({}), &state());
    assert_eq!(
        (frames, data),
        (
            vec![framed_to(0x02, 0xa0c2)],
            vec![1u32.to_le_bytes().to_vec()]
        )
    );
    let (_, _, data) = run("wake", json!({}), &state());
    assert_eq!(data, [0u32.to_le_bytes().to_vec()]);
    let (_, frames, data) = run("focus.point", json!({"x": 0.3, "y": 0.4}), &state());
    assert_eq!(frames, [framed_to(0x02, 0x3582)]);
    let mut want = floats(&[0.3, 0.4]);
    for v in [1u32, 100, 100] {
        want.extend(v.to_le_bytes());
    }
    assert_eq!(data, [want]);
    let (_, frames, data) = run("factory_reset", json!({"confirm": true}), &state());
    assert_eq!(
        (frames, data),
        (vec![framed_to(0x02, 0xa802)], vec![vec![1]])
    );
}

#[test]
fn every_action_of_the_description_is_known_to_act() {
    let (mut cam, _) = model(recorded());
    let names: Vec<String> = cam.description().actions.keys().cloned().collect();
    for name in names {
        let r = cam.act(&name, &Params::new(), &state(), &Ctx::none());
        assert!(
            !matches!(&r, Err(CameraError::Io(m)) if m.contains("no action")),
            "{name}: {r:?}"
        );
    }
}
