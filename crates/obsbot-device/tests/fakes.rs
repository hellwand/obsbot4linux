// SPDX-License-Identifier: GPL-3.0-or-later

//! Behaviour of the fake models through `trait Camera` and of the fake bus (design spec
//! 5.12). The daemon's tests (track B) rely on every behaviour pinned here.

use obsbot_device::contract::conformance::check;
use obsbot_device::contract::{
    Bus, Camera, CameraError, Ctx, Found, IdentitySource, Motion, Need, Target,
};
use obsbot_device::models::fake::{self, FakeBus, Op};
use obsbot_proto::validate::validate_set;
use obsbot_proto::{ErrorCode, Params, Patch, State};
use serde_json::{Value, json};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant};

fn bus() -> FakeBus {
    FakeBus::new(&fake::KEYS).expect("the three fakes")
}

fn found(bus: &FakeBus, product: &str) -> Found {
    bus.scan()
        .into_iter()
        .find(|f| f.usb.product == product)
        .unwrap_or_else(|| panic!("no device with product {product}"))
}

fn open(bus: &FakeBus, product: &str) -> Box<dyn Camera> {
    bus.open(&found(bus, product)).expect("open")
}

fn params(v: Value) -> Params {
    v.as_object().expect("an object").clone()
}

/// The state a daemon would publish after reading every group once.
fn state_of(cam: &mut dyn Camera) -> State {
    let mut state = State::new(true);
    for group in cam.poll_plan().to_vec() {
        match cam.read(&group.name, &Ctx::none()) {
            Ok(values) => values.into_iter().for_each(|(k, v)| state.set(&k, v)),
            Err(_) => state.unread.extend(group.leaves.clone()),
        }
    }
    state
}

fn read_leaf(cam: &mut dyn Camera, group: &str, leaf: &str) -> Value {
    cam.read(group, &Ctx::none()).expect("read")[leaf].clone()
}

#[test]
fn fake_a_and_fake_b_pass_the_conformance_check() {
    let bus = bus();
    assert_eq!(
        check(&mut *open(&bus, "fa0a")),
        Vec::<String>::new(),
        "fake-a"
    );
    assert_eq!(
        check(&mut *open(&bus, "fa0b")),
        Vec::<String>::new(),
        "fake-b"
    );
}

#[test]
fn the_registry_has_no_shared_pair_and_finds_the_unknown_device_unsupported() {
    assert!(fake::REGISTRY.shared_pairs().is_empty());
    let scan = bus().scan();
    let models: Vec<Option<&str>> = scan.iter().map(|f| f.model.map(|m| m.key)).collect();
    assert_eq!(
        models,
        [Some("fake-a"), Some("fake-b"), None],
        "unknown enters through the registry lookup (5.12)"
    );
    for f in &scan {
        assert_eq!(f.usb.vendor, "3564");
        assert!(
            f.video_node
                .as_deref()
                .is_some_and(|n| n.starts_with("/dev/video"))
        );
    }
    assert_eq!(
        scan[0].model.unwrap().support,
        obsbot_proto::Support::Tested
    );
    assert_eq!(
        scan[1].model.unwrap().support,
        obsbot_proto::Support::Untested
    );
}

#[test]
fn fake_b_differs_from_the_tiny_2_lite_where_the_spec_says() {
    let a = fake::description("fake-a").unwrap();
    let b = fake::description("fake-b").unwrap();
    assert!(
        !b.settings.keys().any(|k| k.starts_with("gimbal.")),
        "no gimbal"
    );
    assert!(!b.settings.contains_key("views"), "no views");
    assert!(
        b.settings.contains_key("beacon.color") && b.actions.contains_key("beacon.flash"),
        "a group no client knows"
    );
    assert_eq!(b.limits["presets"], 5);
    assert_ne!(
        a.settings["tracking.mode"].values, b.settings["tracking.mode"].values,
        "other enum values"
    );
    let asleep: Vec<_> = b
        .settings
        .values()
        .filter_map(|s| s.asleep.clone())
        .collect();
    for kind in [
        obsbot_proto::Asleep::Ignored,
        obsbot_proto::Asleep::Unknown,
        obsbot_proto::Asleep::Accepted,
    ] {
        assert!(asleep.contains(&kind), "{kind:?}");
    }
    assert_eq!(
        b.settings.values().filter(|s| !s.readback).count(),
        2,
        "two readback:false leaves"
    );
    assert!(!b.settings["exposure.time"].requires.is_empty());
}

#[test]
fn a_written_value_reads_back() {
    let bus = bus();
    let mut cam = open(&bus, "fa0a");
    cam.apply("led.brightness", &json!(2), &Ctx::none())
        .unwrap();
    assert_eq!(cam.touched(Target::Leaf("led.brightness")), ["led"]);
    assert_eq!(read_leaf(&mut *cam, "led", "led.brightness"), json!(2));
}

#[test]
fn fake_b_lags_one_readback_and_rounds_another_so_pending_and_settle_both_happen() {
    let bus = bus();
    let mut cam = open(&bus, "fa0b");
    cam.apply("led.brightness", &json!(3), &Ctx::none())
        .unwrap();
    assert_eq!(
        read_leaf(&mut *cam, "led", "led.brightness"),
        json!(0),
        "the touched re-read does not show it yet: pending"
    );
    std::thread::sleep(fake::b::LAG + Duration::from_millis(50));
    assert_eq!(
        read_leaf(&mut *cam, "led", "led.brightness"),
        json!(3),
        "a later read shows it: it leaves pending"
    );
    assert!(
        fake::b::LAG.as_millis() < u128::from(fake::b::SETTLE_MS),
        "the lag is inside the settle bound"
    );

    cam.apply("exposure.mode", &json!("manual"), &Ctx::none())
        .unwrap();
    cam.apply("exposure.time", &json!(1234), &Ctx::none())
        .unwrap();
    assert_eq!(
        read_leaf(&mut *cam, "exposure", "exposure.time"),
        json!(1200),
        "never 1234: only the settle bound ends pending"
    );
}

#[test]
fn asleep_the_fake_reads_only_groups_readable_asleep_and_keeps_writes_until_wake() {
    let bus = bus();
    let mut cam = open(&bus, "fa0b");
    bus.sleep("sn:FAKEB0000001").unwrap();
    assert_eq!(read_leaf(&mut *cam, "status", "asleep"), json!(true));
    assert!(
        matches!(cam.read("led", &Ctx::none()), Err(CameraError::NoAnswer(_))),
        "led is not readable asleep"
    );

    let state = state_of(&mut *cam);
    assert!(state.asleep());
    let desc = cam.description().clone();
    let refused = validate_set(&desc, &state, &Patch::new().leaf("zoom.factor", 1.5)).unwrap_err();
    assert_eq!(
        refused.code,
        ErrorCode::Asleep,
        "asleep: ignored is refused"
    );
    assert!(
        validate_set(&desc, &state, &Patch::new().leaf("led.brightness", 4)).is_ok(),
        "asleep: unknown is written"
    );

    cam.apply("led.brightness", &json!(4), &Ctx::none())
        .unwrap();
    bus.wake("sn:FAKEB0000001").unwrap();
    std::thread::sleep(fake::b::LAG + Duration::from_millis(50));
    assert_eq!(
        read_leaf(&mut *cam, "led", "led.brightness"),
        json!(4),
        "the first read after wake shows it"
    );
}

#[test]
fn sleep_and_wake_actions_change_asleep() {
    let bus = bus();
    let mut cam = open(&bus, "fa0a");
    cam.act("sleep", &Params::new(), &State::new(true), &Ctx::none())
        .unwrap();
    assert_eq!(read_leaf(&mut *cam, "status", "asleep"), json!(true));
    cam.act("wake", &Params::new(), &State::new(true), &Ctx::none())
        .unwrap();
    assert_eq!(read_leaf(&mut *cam, "status", "asleep"), json!(false));
}

#[test]
fn fail_next_fails_exactly_one_operation_so_its_group_goes_unread_once() {
    let bus = bus();
    let mut cam = open(&bus, "fa0a");
    bus.fail_next("sn:FAKEA0000001").unwrap();
    assert!(matches!(
        cam.read("led", &Ctx::none()),
        Err(CameraError::NoAnswer(_))
    ));
    assert!(cam.read("led", &Ctx::none()).is_ok());
    let state = {
        bus.fail_next("sn:FAKEA0000001").unwrap();
        state_of(&mut *cam)
    };
    assert!(
        !state.unread.is_empty(),
        "the failed group's leaves are unread"
    );
}

#[test]
fn identity_answers_only_what_is_needed() {
    // The daemon asks only what the entry declares (5.5): the fakes have both queries, so
    // their serial and firmware reach the device list.
    for entry in [&fake::a::ENTRY, &fake::b::ENTRY] {
        assert_eq!(
            (entry.identity, entry.firmware_query),
            (IdentitySource::Query, true),
            "{}",
            entry.key
        );
    }
    let bus = bus();
    let mut cam = open(&bus, "fa0a");
    let both = cam
        .identity(
            Need {
                serial: true,
                firmware: true,
            },
            &Ctx::none(),
        )
        .unwrap();
    assert_eq!(both.serial.as_deref(), Some("FAKEA0000001"));
    assert!(obsbot_proto::is_valid_serial(
        both.serial.as_deref().unwrap()
    ));
    assert!(both.firmware.is_some());
    let firmware_only = cam
        .identity(
            Need {
                serial: false,
                firmware: true,
            },
            &Ctx::none(),
        )
        .unwrap();
    assert_eq!(firmware_only.serial, None);
    bus.fail_next("sn:FAKEA0000001").unwrap();
    assert!(
        cam.identity(
            Need {
                serial: true,
                firmware: true
            },
            &Ctx::none()
        )
        .is_err(),
        "an identity read fails too"
    );
}

#[test]
fn unplug_and_replug_give_the_same_device_under_a_new_device_number() {
    let bus = bus();
    let before = found(&bus, "fa0a");
    let mut old = bus.open(&before).unwrap();
    bus.unplug("sn:FAKEA0000001").unwrap();
    assert_eq!(bus.scan().len(), 2);
    assert_eq!(
        old.read("led", &Ctx::none()),
        Err(CameraError::Gone),
        "a read of a gone device fails at once"
    );
    assert_eq!(
        bus.plug("fake-a").unwrap(),
        "sn:FAKEA0000001",
        "the same serial comes back"
    );
    let after = found(&bus, "fa0a");
    assert_eq!(after.usb.path, before.usb.path, "same port, same path");
    assert_ne!(after.devnum, before.devnum, "a new enumeration (5.3)");
    assert_eq!(
        old.read("led", &Ctx::none()),
        Err(CameraError::Gone),
        "the old handle stays gone"
    );
    assert!(bus.open(&after).unwrap().read("led", &Ctx::none()).is_ok());
    assert!(
        bus.open(&before).is_err(),
        "the old enumeration cannot be opened"
    );
}

#[test]
fn the_unknown_device_receives_nothing() {
    let bus = bus();
    let unknown = found(&bus, "fa00");
    assert!(unknown.model.is_none());
    assert!(bus.open(&unknown).is_err());
    let id = format!("port:{}", unknown.usb.path);
    for _ in 0..3 {
        bus.scan();
    }
    assert_eq!(bus.ops(&id), Vec::<Op>::new());
}

#[test]
fn restart_warnings_make_the_device_reenumerate_and_factory_reset_clears_presets() {
    let bus = bus();
    let before = found(&bus, "fa0a");
    let mut cam = bus.open(&before).unwrap();
    cam.act(
        "preset.save",
        &params(json!({"slot": 0})),
        &State::new(true),
        &Ctx::none(),
    )
    .unwrap();
    assert_eq!(
        read_leaf(&mut *cam, "presets", "presets")
            .as_array()
            .unwrap()
            .len(),
        1
    );
    let t0 = Instant::now();
    cam.act(
        "factory_reset",
        &params(json!({"confirm": true})),
        &State::new(true),
        &Ctx::none(),
    )
    .unwrap();
    assert!(
        bus.scan().iter().all(|f| f.usb.path != before.usb.path),
        "gone while it restarts"
    );
    assert_eq!(cam.read("led", &Ctx::none()), Err(CameraError::Gone));
    std::thread::sleep(fake::RESTART.saturating_sub(t0.elapsed()) + Duration::from_millis(50));
    let after = found(&bus, "fa0a");
    assert_ne!(after.devnum, before.devnum, "back as a new enumeration");
    let mut cam = bus.open(&after).unwrap();
    assert_eq!(
        read_leaf(&mut *cam, "presets", "presets"),
        json!([]),
        "the presets are deleted"
    );

    cam.apply("audio.enabled", &json!(false), &Ctx::none())
        .unwrap();
    assert!(
        bus.scan().iter().all(|f| f.usb.path != before.usb.path),
        "the microphone write restarts the camera too"
    );
}

#[test]
fn check_refuses_what_the_state_does_not_allow() {
    let bus = bus();
    let mut cam = open(&bus, "fa0a");
    let state = state_of(&mut *cam);
    let code = |cam: &dyn Camera, action: &str, p: Value, state: &State| {
        cam.check(action, &params(p), state).err().map(|r| r.code)
    };
    assert_eq!(
        code(&*cam, "gimbal.move", json!({}), &state),
        Some(ErrorCode::Invalid),
        "4.4: at least one axis"
    );
    assert_eq!(code(&*cam, "gimbal.move", json!({"pan": 10}), &state), None);
    assert_eq!(
        code(&*cam, "preset.recall", json!({"slot": 1}), &state),
        Some(ErrorCode::Conflict),
        "empty slot"
    );
    assert_eq!(
        code(&*cam, "view.go", json!({"id": 7}), &state),
        Some(ErrorCode::Conflict),
        "unknown view id"
    );
    let mut unread = state.clone();
    unread.unread.push("presets".into());
    assert_eq!(
        code(&*cam, "preset.save", json!({"slot": 0}), &unread),
        Some(ErrorCode::Conflict),
        "list in unread"
    );

    cam.act(
        "preset.save",
        &params(json!({"slot": 1, "name": "Desk"})),
        &state,
        &Ctx::none(),
    )
    .unwrap();
    let state = state_of(&mut *cam);
    assert_eq!(
        code(&*cam, "preset.recall", json!({"slot": 1}), &state),
        None
    );
    assert_eq!(state.at("presets").unwrap()[0]["name"], json!("Desk"));
}

#[test]
fn actions_change_the_state_and_view_save_returns_its_id() {
    let bus = bus();
    let mut cam = open(&bus, "fa0a");
    let first = cam
        .act(
            "view.save",
            &params(json!({"name": "Desk"})),
            &State::new(true),
            &Ctx::none(),
        )
        .unwrap();
    let second = cam
        .act(
            "view.save",
            &params(json!({"name": "Door"})),
            &State::new(true),
            &Ctx::none(),
        )
        .unwrap();
    assert_eq!(first, Some(params(json!({"id": 0}))));
    assert_eq!(second, Some(params(json!({"id": 1}))));
    cam.act(
        "gimbal.move",
        &params(json!({"pan": 20})),
        &State::new(true),
        &Ctx::none(),
    )
    .unwrap();
    let state = state_of(&mut *cam);
    assert_eq!(state.at("gimbal.pan"), Some(&json!(20)));
}

/// Spec 2: an action sends exactly what was asked, nothing before it and nothing after it.
/// 0.13 switched the tracking to static before a manual move; 1.0 does not, so a move while
/// the tracking is on is one operation and leaves `tracking.mode` alone. What the camera
/// itself does then is CAM-34; the fake only moves.
#[test]
fn a_move_while_tracking_sends_only_the_move() {
    let bus = bus();
    let mut cam = open(&bus, "fa0a");
    cam.apply("tracking.mode", &json!("normal"), &Ctx::none())
        .unwrap();
    for (action, with) in [
        ("gimbal.move", json!({"pan": 20})),
        ("gimbal.turn", json!({"pan_speed": 10})),
    ] {
        let before = bus.ops("sn:FAKEA0000001").len();
        cam.act(
            action,
            &params(with.clone()),
            &State::new(true),
            &Ctx::none(),
        )
        .unwrap();
        assert_eq!(
            bus.ops("sn:FAKEA0000001")[before..],
            [Op::Act(action.into(), params(with))],
            "an action sends exactly what was asked (spec 2): {action} is one command, no tracking switch before or after it"
        );
        assert_eq!(
            read_leaf(&mut *cam, "tracking", "tracking.mode"),
            json!("normal"),
            "an action sends exactly what was asked (spec 2): {action} leaves tracking.mode alone"
        );
        assert_eq!(
            cam.touched(Target::Action(action)),
            Vec::<String>::new(),
            "{action} touches no group: only the camera could change the tracking, and that is CAM-34"
        );
    }
}

/// Spec 2 and 4.4: `preset.save` without `name` sends no rename and the daemon invents no
/// name (0.13 named the preset `Preset <slot + 1>`). The fakes store an empty name for it,
/// their own behaviour; what the camera reports for an unnamed preset is CAM-35.
#[test]
fn a_preset_saved_without_a_name_gets_none() {
    let bus = bus();
    for (product, id) in [("fa0a", "sn:FAKEA0000001"), ("fa0b", "sn:FAKEB0000001")] {
        let mut cam = open(&bus, product);
        let before = bus.ops(id).len();
        cam.act(
            "preset.save",
            &params(json!({"slot": 0})),
            &State::new(true),
            &Ctx::none(),
        )
        .unwrap();
        assert_eq!(
            bus.ops(id)[before..],
            [Op::Act("preset.save".into(), params(json!({"slot": 0})))],
            "{product}: an action sends exactly what was asked (spec 2): one act, no rename after it"
        );
        let state = state_of(&mut *cam);
        assert_eq!(
            state.at("presets").unwrap()[0]["name"],
            json!(""),
            "{product}: the daemon invents no name (spec 2, 4.4)"
        );
        cam.act(
            "preset.save",
            &params(json!({"slot": 1, "name": "Desk"})),
            &State::new(true),
            &Ctx::none(),
        )
        .unwrap();
        let state = state_of(&mut *cam);
        assert_eq!(
            state.at("presets").unwrap()[1]["name"],
            json!("Desk"),
            "{product}: a name that was given is stored"
        );
    }
}

/// 4.1 rules 2 and 6, 5.6: the daemon learns from `motion` which actions jump the queue and
/// which clear a continuing turn's record. Without the `Move` marks, a connection that turned
/// the gimbal and closed later would stop another client's preset recall.
#[test]
fn motion_marks_follow_rules_2_and_6_of_4_1() {
    let bus = bus();
    let a = open(&bus, "fa0a");
    let motion: Vec<(String, Motion)> = a
        .description()
        .actions
        .keys()
        .map(|name| (name.clone(), a.motion(name)))
        .filter(|(_, m)| *m != Motion::None)
        .collect();
    let want = [
        ("gimbal.move", Motion::Move),
        ("gimbal.stop", Motion::Stop),
        (
            "gimbal.turn",
            Motion::Continuing {
                stop: "gimbal.stop",
            },
        ),
        ("hand.go_return", Motion::Move),
        ("power_on.go", Motion::Move),
        ("preset.recall", Motion::Move),
        ("view.go", Motion::Move),
        ("zone.go_return", Motion::Move),
        ("zoom.move", Motion::Move),
        ("zoom.stop", Motion::Stop),
    ]
    .map(|(n, m)| (n.to_string(), m));
    assert_eq!(motion, want);
    assert_eq!(
        a.motion("preset.recall"),
        Motion::Move,
        "a recall after a turn clears the turn's record, so closing the turning connection does not stop the recall"
    );
    assert_eq!(a.motion("preset.save"), Motion::None);
    let b = open(&bus, "fa0b");
    assert_eq!(b.motion("preset.recall"), Motion::Move);
    assert!(
        b.description()
            .actions
            .keys()
            .all(|n| !matches!(b.motion(n), Motion::Stop | Motion::Continuing { .. })),
        "fake-b has no stops and no continuing action"
    );
    assert!(
        a.send_table().entries.is_empty() && b.send_table().entries.is_empty(),
        "the fakes send nothing"
    );
}

#[test]
fn a_delay_is_cut_by_the_deadline_and_by_a_stop() {
    let bus = bus();
    bus.set_delay("fake-a", Duration::from_secs(5));
    let mut cam = open(&bus, "fa0a");
    let t0 = Instant::now();
    let cut = cam.apply(
        "led.brightness",
        &json!(1),
        &Ctx::until(Instant::now() + Duration::from_millis(100)),
    );
    assert_eq!(cut, Err(CameraError::Cut));
    assert!(
        t0.elapsed() < Duration::from_secs(1),
        "cut at the next frame boundary, not after the delay"
    );
    let flag = Arc::new(AtomicBool::new(true));
    assert_eq!(
        cam.read("led", &Ctx::interruptible(flag)),
        Err(CameraError::Cut)
    );
}

#[test]
fn every_operation_is_recorded_in_order() {
    let bus = bus();
    let mut cam = open(&bus, "fa0b");
    cam.identity(
        Need {
            serial: true,
            firmware: false,
        },
        &Ctx::none(),
    )
    .unwrap();
    cam.read("led", &Ctx::none()).unwrap();
    cam.apply("led.brightness", &json!(1), &Ctx::none())
        .unwrap();
    cam.act(
        "beacon.flash",
        &params(json!({"times": 2})),
        &State::new(true),
        &Ctx::none(),
    )
    .unwrap();
    assert_eq!(
        bus.ops("sn:FAKEB0000001"),
        [
            Op::Open,
            Op::Identity(Need {
                serial: true,
                firmware: false
            }),
            Op::Read("led".into()),
            Op::Apply("led.brightness".into(), json!(1)),
            Op::Act("beacon.flash".into(), params(json!({"times": 2}))),
        ]
    );
}

#[test]
fn the_test_control_lines_of_5_12() {
    let bus = bus();
    assert_eq!(
        bus.handle_line(r#"{"plug":"fake-b"}"#),
        r#"{"ok":"sn:FAKEB0000002"}"#,
        "a second fake-b"
    );
    assert_eq!(
        bus.handle_line(r#"{"sleep":"sn:FAKEB0000002"}"#),
        r#"{"ok":true}"#
    );
    assert_eq!(
        bus.handle_line(r#"{"wake":"sn:FAKEB0000002"}"#),
        r#"{"ok":true}"#
    );
    assert_eq!(
        bus.handle_line(r#"{"fail_next":"sn:FAKEB0000002"}"#),
        r#"{"ok":true}"#
    );
    assert_eq!(
        bus.handle_line(r#"{"unplug":"port:9-3"}"#),
        r#"{"ok":true}"#,
        "the unknown device by its port: id"
    );
    assert_eq!(bus.scan().len(), 3);
    assert!(
        bus.handle_line(r#"{"plug":"tiny2lite"}"#)
            .starts_with(r#"{"error":"unknown fake model tiny2lite"#)
    );
    assert!(
        bus.handle_line(r#"{"unplug":"sn:NOPE00000000"}"#)
            .starts_with(r#"{"error":"#)
    );
    assert!(
        bus.handle_line(r#"{"plug":"fake-a","sleep":"x"}"#)
            .starts_with(r#"{"error":"#)
    );
    assert!(
        bus.handle_line("nonsense")
            .starts_with(r#"{"error":"not JSON"#)
    );
}
