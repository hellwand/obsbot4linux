// SPDX-License-Identifier: GPL-3.0-or-later

//! The send-table test of design spec 8.3 and the trait conformance check (8.7 step 6) for the
//! Tiny 2 Lite, on a recorded camera (replay transport, behind the send-table check):
//!
//! - every setting at its boundary values and every action sends exactly the frames of its
//!   field-ledger row, in that order, and nothing before or after them (spec 2);
//! - every frame the model writes is in its send table (the check refuses anything else, so a
//!   frame outside it fails here), and every entry of the table is used;
//! - the model passes `obsbot_device::contract::conformance::check`.

mod support;

use obsbot_device::contract::conformance;
use obsbot_device::contract::{Camera, Ctx, Frame, Need};
use obsbot_device::models::tiny2lite::model::SELECTORS;
use obsbot_device::models::tiny2lite::send_table;
use obsbot_device::transport::checked::classify;
use obsbot_device::transport::replay::Sent;
use obsbot_proto::{Bound, Params, Setting, SettingType, State};
use serde_json::{Value, json};
use std::collections::BTreeSet;
use support::*;

/// The frames of each writable leaf's ledger row (`docs/field-ledger.md`, column "Write").
fn ledger_row(leaf: &str) -> Vec<Frame> {
    let one = |c: u16| vec![framed(c)];
    match leaf {
        "gimbal.invert" => one(0x3b84),
        "zoom.factor" => vec![uvc(0x009a_090d)],
        "zoom.field_of_view" => vec![short(0x04)],
        "tracking.mode" => vec![short(0x16)],
        "tracking.profile" => one(0x0cc4),
        "tracking.speed" => one(0x0944),
        "framing.auto_zoom" => one(0x0a44),
        "framing.size" => one(0x0ac4),
        "framing.zoom_speed" => one(0x0b44),
        "framing.offset.x" => one(0x0bc4),
        "framing.offset.y" => one(0x0c44),
        "framing.part" => one(0x1584),
        "gestures.target_lock" => one(0x30c4),
        "gestures.zoom" => one(0x3144),
        "gestures.zoom_ratio" => one(0x3244),
        "gestures.dynamic_zoom" => one(0x3344),
        "gestures.direction_change" => one(0x33c4),
        "zone.enabled" => one(0x08c4),
        "zone.follow" => one(0x4e04),
        "zone.area.pan" => vec![framed(0x4c04), framed(0x4c84)],
        "zone.area.tilt" => vec![framed(0x4d04), framed(0x4d84)],
        "hand.side" => one(0x2244),
        "hand.area.pan" => vec![framed(0x2044), framed(0x20c4)],
        "hand.area.tilt" => vec![framed(0x2144), framed(0x21c4)],
        "hand.axes.pan" => one(0x2344),
        "hand.axes.tilt" => one(0x23c4),
        "hand.return_after" => one(0x2444),
        "hand.move_gimbal" => one(0x26c4),
        "exposure.mode" => vec![uvc(0x009a_0901)],
        "exposure.region" => vec![short(0x03)],
        "exposure.time" => vec![uvc(0x009a_0902)],
        "exposure.gain" => vec![uvc(0x0098_0913)],
        "exposure.compensation" => vec![uvc(0x0098_091c)],
        "exposure.hdr" => vec![short(0x01)],
        "exposure.anti_flicker" => vec![uvc(0x0098_0918)],
        "image.brightness" => vec![uvc(0x0098_0900)],
        "image.contrast" => vec![uvc(0x0098_0901)],
        "image.saturation" => vec![uvc(0x0098_0902)],
        "image.hue" => vec![uvc(0x0098_0903)],
        "image.sharpness" => vec![uvc(0x0098_091b)],
        "image.white_balance.auto" => vec![uvc(0x0098_090c)],
        "image.white_balance.temperature" => vec![uvc(0x0098_091a)],
        "image.mirror" => vec![short(0x14)],
        "focus.mode" => vec![framed_to(0x02, 0x3442)],
        "focus.region" => vec![framed_to(0x02, 0x3602)],
        "focus.position" => vec![uvc(0x009a_090a)],
        "audio.enabled" => vec![short(0x1c)],
        "audio.volume" => vec![framed_to(0x02, 0x8102)],
        "audio.noise_reduction" => vec![short(0x0a)],
        "audio.auto_gain" => vec![short(0x17)],
        "audio.in_sleep" => vec![short(0x13)],
        "led.brightness" => vec![short(0x1a)],
        "led.blink" => vec![short(0x18)],
        "sleep.auto.enabled" | "sleep.auto.seconds" => vec![short(0x0b)],
        "sleep.background" => vec![short(0x0e)],
        "sleep.background_mirror" => vec![short(0x0e)],
        "sleep.wake_zoom" => vec![short(0x15)],
        "power_on.mode" => vec![short(0x19)],
        "power_on.start.mode" => one(0x3e44),
        other => panic!("{other}: no ledger row in this test"),
    }
}

fn bound(b: &Option<Bound>, component: Option<&str>) -> Option<f64> {
    b.as_ref().and_then(|b| b.get(component))
}

/// The boundary values of a writable leaf: both bools, every writable enum value, the minimum
/// and the maximum of a number (the minimum and one more where it has no maximum), the widest
/// range.
fn boundaries(s: &Setting) -> Vec<Value> {
    match &s.kind {
        SettingType::Bool => vec![json!(true), json!(false)],
        SettingType::Enum => s.writable.iter().map(|v| json!(v)).collect(),
        SettingType::Number => {
            let lo = bound(&s.min, None).unwrap();
            let hi = bound(&s.max, None).unwrap_or(lo + 1.0);
            vec![json!(lo), json!(hi)]
        }
        SettingType::Range => vec![json!({
            "min": bound(&s.min, Some("min")).unwrap(),
            "max": bound(&s.max, Some("max")).unwrap(),
        })],
        other => panic!("no boundaries for a writable {other:?}"),
    }
}

/// A published state with presets in slots 0 and 2 (slot 1 empty) and view 0.
fn full_state() -> State {
    let mut s = State::new(true);
    let preset =
        |slot: u8| json!({"slot": slot, "name": "x", "pose": {"pan": 0, "tilt": 0, "zoom": 1}});
    s.set("presets", json!([preset(0), preset(2)]));
    s.set(
        "views",
        json!([{"id": 0, "name": "x", "pose": {"pan": 0, "tilt": 0, "zoom": 1}}]),
    );
    s.set("image.mirror", json!(false));
    s.set("gimbal.invert", json!(true));
    s
}

/// Every action at its boundary values, with the frames of its ledger row (table "Actions").
fn action_rows() -> Vec<(&'static str, Value, Vec<Frame>)> {
    let name32 = "N".repeat(32);
    let pose_lo = json!({"pan": -150, "tilt": -70, "zoom": 1});
    let pose_hi = json!({"pan": 150, "tilt": 90, "zoom": 4});
    let mut rows = vec![
        (
            "gimbal.move",
            json!({"pan": -150, "tilt": -70}),
            vec![framed(0x6444)],
        ),
        (
            "gimbal.move",
            json!({"pan": 150, "tilt": 90}),
            vec![framed(0x6444)],
        ),
        (
            "gimbal.move",
            json!({"pan": 0, "speed": 1}),
            vec![framed(0x6544)],
        ),
        (
            "gimbal.move",
            json!({"tilt": 90, "speed": 180}),
            vec![framed(0x6544)],
        ),
        ("gimbal.stop", json!({}), vec![framed(0x6704)]),
        (
            "gimbal.turn",
            json!({"pan_speed": -178, "tilt_speed": 178}),
            vec![framed(0x6484)],
        ),
        (
            "gimbal.turn",
            json!({"pan_speed": 178, "keep": true}),
            vec![framed(0x6484)],
        ),
        ("zoom.stop", json!({}), vec![framed(0x68c4)]),
        ("preset.save", json!({"slot": 0}), vec![framed(0x3e04)]),
        ("preset.save", json!({"slot": 1}), vec![framed(0x3944)]),
        (
            "preset.save",
            json!({"slot": 2, "name": name32, "pose": pose_hi}),
            vec![framed(0x3e04), framed(0x3a84)],
        ),
        ("preset.recall", json!({"slot": 2}), vec![framed(0x39c4)]),
        (
            "preset.rename",
            json!({"slot": 0, "name": name32}),
            vec![framed(0x3a84)],
        ),
        (
            "preset.delete",
            json!({"slot": 2}),
            vec![framed(0x3984), framed(0x3d84)],
        ),
        (
            "view.save",
            json!({"name": "a", "pose": pose_lo}),
            vec![framed(0x1384), framed(0x1544)],
        ),
        ("view.go", json!({"id": 0}), vec![framed(0x1604)]),
        (
            "view.rename",
            json!({"id": 0, "name": name32}),
            vec![framed(0x1544)],
        ),
        ("view.delete", json!({"id": 0}), vec![framed(0x13c4)]),
        ("zone.return_here", json!({}), vec![framed(0x4e84)]),
        ("zone.go_return", json!({}), vec![framed(0x4f44)]),
        ("hand.return_here", json!({}), vec![framed(0x24c4)]),
        ("hand.go_return", json!({}), vec![framed(0x2584)]),
        ("hand.return_clear", json!({}), vec![framed(0x2544)]),
        (
            "hand.area_reset",
            json!({}),
            vec![
                framed(0x25c4),
                framed(0x2604),
                framed(0x2644),
                framed(0x2684),
            ],
        ),
        (
            "focus.point",
            json!({"x": 0, "y": 1}),
            vec![framed_to(0x02, 0x3582)],
        ),
        (
            "power_on.set_pose",
            json!({"pose": pose_lo}),
            vec![framed(0x3ec4), framed(0x3e44)],
        ),
        (
            "power_on.set_pose",
            json!({"pose": pose_hi}),
            vec![framed(0x3ec4), framed(0x3e44)],
        ),
        (
            "power_on.use_current",
            json!({}),
            vec![framed(0x3ec4), framed(0x3e44)],
        ),
        (
            "power_on.use_preset",
            json!({"slot": 2}),
            vec![framed(0x3ec4), framed(0x3e44)],
        ),
        ("power_on.go", json!({}), vec![short(0x00)]),
        ("sleep", json!({}), vec![framed_to(0x02, 0xa0c2)]),
        ("wake", json!({}), vec![framed_to(0x02, 0xa0c2)]),
        (
            "factory_reset",
            json!({"confirm": true}),
            vec![framed_to(0x02, 0xa802)],
        ),
    ];
    for factor in [1.0, 4.0] {
        for speed in ["slow", "medium", "fast"] {
            rows.push((
                "zoom.move",
                json!({"factor": factor, "speed": speed}),
                vec![framed(0x6884)],
            ));
        }
    }
    rows
}

#[test]
fn every_setting_at_its_boundaries_sends_exactly_its_ledger_row() {
    let (cam, _) = model(recorded());
    let settings: Vec<(String, Setting)> = cam
        .description()
        .settings
        .iter()
        .filter(|(_, s)| !s.readonly)
        .map(|(p, s)| (p.clone(), s.clone()))
        .collect();
    assert_eq!(settings.len(), 60, "every writable leaf of 4.3 is driven");
    for (leaf, setting) in settings {
        for value in boundaries(&setting) {
            let (mut cam, log) = model(recorded());
            cam.apply(&leaf, &value, &Ctx::none())
                .unwrap_or_else(|e| panic!("{leaf} = {value}: {e}"));
            assert_eq!(commands(&log), ledger_row(&leaf), "{leaf} = {value}");
        }
    }
}

#[test]
fn every_action_at_its_boundaries_sends_exactly_its_ledger_row() {
    let (cam, _) = model(recorded());
    let described: BTreeSet<String> = cam.description().actions.keys().cloned().collect();
    let rows = action_rows();
    let driven: BTreeSet<String> = rows.iter().map(|(a, _, _)| a.to_string()).collect();
    assert_eq!(driven, described, "every action of 4.4 is driven");
    for (action, params, frames) in rows {
        let (mut cam, log) = model(recorded());
        let p: Params = params.as_object().unwrap().clone();
        let state = full_state();
        cam.check(action, &p, &state)
            .unwrap_or_else(|r| panic!("{action} {params}: {r:?}"));
        cam.act(action, &p, &state, &Ctx::none())
            .unwrap_or_else(|e| panic!("{action} {params}: {e}"));
        assert_eq!(commands(&log), frames, "{action} {params}");
    }
}

#[test]
fn the_send_table_names_exactly_what_the_model_sends() {
    let mut sent: BTreeSet<String> = BTreeSet::new();
    let mut note = |log: &obsbot_device::transport::replay::Log| {
        for s in log.sent() {
            let f = match s {
                Sent::Xu { selector, data } => classify(&SELECTORS, selector, &data).unwrap(),
                Sent::Ctrl { id, .. } => Frame::Uvc { control: id },
            };
            sent.insert(format!("{f:?}"));
        }
    };
    // identity and every group, with the tracking on so that the target is read
    let mut tracking = block(STATUS);
    tracking[0x18] = 2;
    let mut target = vec![1, 1, 1, 1, 0, 0];
    target.extend([0u8; 16]);
    let (mut cam, log) = model(recorded().buffer(6, tracking).answer(0x04, 0x01c4, &target));
    cam.identity(
        Need {
            serial: true,
            firmware: true,
        },
        &Ctx::none(),
    )
    .unwrap();
    for g in cam.poll_plan().to_vec() {
        cam.read(&g.name, &Ctx::none()).unwrap();
    }
    note(&log);
    // every setting and every action
    let (cam, _) = model(recorded());
    let desc = cam.description().clone();
    for (leaf, setting) in desc.settings.iter().filter(|(_, s)| !s.readonly) {
        for value in boundaries(setting) {
            let (mut cam, log) = model(recorded());
            cam.apply(leaf, &value, &Ctx::none()).unwrap();
            note(&log);
        }
    }
    for (action, params, _) in action_rows() {
        let (mut cam, log) = model(recorded());
        cam.act(
            action,
            params.as_object().unwrap(),
            &full_state(),
            &Ctx::none(),
        )
        .unwrap();
        note(&log);
    }
    let table: BTreeSet<String> = send_table::ENTRIES
        .iter()
        .map(|e| format!("{:?}", e.frame))
        .collect();
    let unused: Vec<&String> = table.difference(&sent).collect();
    assert!(
        unused.is_empty(),
        "entries the model never sends: {unused:?}"
    );
    // the check would have refused anything else; this makes it explicit
    assert!(sent.is_subset(&table));
}

#[test]
fn the_model_passes_the_trait_conformance_check() {
    // the tracking on: on a static camera the target group has no read to return before its
    // first one (decision 8), which the check counts as a failed read
    let (mut cam, _) = model(tracking());
    assert_eq!(conformance::check(&mut cam), Vec::<String>::new());
}
