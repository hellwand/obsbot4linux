// SPDX-License-Identifier: GPL-3.0-or-later

//! Table tests of the validator. Each row names the rule of the spec it pins.

use obsbot_proto::validate::{
    Fit, check_description, check_value, validate_do, validate_get, validate_set,
};
use obsbot_proto::{
    Action, Asleep, Description, ErrorCode, Patch, Setting, SettingType, State, Unit, Warning,
};
use serde_json::{Value, json};

/// A small device: every setting type, a `requires`, an `asleep: ignored` leaf, a read-only
/// leaf, a destructive action and an action with an optional parameter.
fn desc() -> Description {
    let w = |s: Setting| s.asleep(Asleep::Unknown).settle_ms(3000);
    Description::default()
        .setting("asleep", Setting::boolean("Asleep").readonly())
        .setting(
            "led.brightness",
            w(Setting::number("LED brightness", 0.0, 3.0).step(1.0)),
        )
        .setting(
            "zoom.factor",
            Setting::number("Zoom", 1.0, 4.0)
                .precision(0.01)
                .asleep(Asleep::Ignored)
                .settle_ms(3000),
        )
        .setting(
            "zoom.field_of_view",
            w(
                Setting::enumeration("Field of view", &["wide", "medium", "custom"])
                    .writable_only(&["wide", "medium"]),
            ),
        )
        .setting(
            "exposure.mode",
            w(Setting::enumeration("Exposure mode", &["auto", "manual"])),
        )
        .setting(
            "exposure.time",
            w(Setting::number("Exposure time", 100.0, 250_000.0).unit(Unit::Microseconds))
                .requires("exposure.mode", json!("manual")),
        )
        .setting(
            "zone.area.pan",
            w(Setting::range("Zone pan", -150.0, 150.0).unit(Unit::Deg)),
        )
        .setting(
            "zone.return",
            Setting::composite(
                SettingType::Pose,
                "Zone return",
                &[("pan", -150.0), ("tilt", -70.0), ("zoom", 1.0)],
                &[("pan", 150.0), ("tilt", 90.0), ("zoom", 4.0)],
            )
            .readonly(),
        )
        .setting("gimbal.invert", w(Setting::boolean("Invert")))
        .setting(
            "presets",
            Setting::list(
                "Presets",
                &[
                    ("slot", Setting::number("Slot", 0.0, 2.0)),
                    ("name", Setting::string("Name", 32, true)),
                ],
            ),
        )
        .action(
            "preset.save",
            Action::new("Save a preset", Asleep::Ignored)
                .param("slot", Setting::number("Slot", 0.0, 2.0).step(1.0))
                .param("name", Setting::string("Name", 32, true).optional()),
        )
        .action(
            "factory_reset",
            Action::new("Factory reset", Asleep::Unknown)
                .param("confirm", Setting::boolean("Confirm"))
                .warning(Warning::Destructive)
                .warning(Warning::Restart),
        )
        .limit("presets", 3)
}

fn awake() -> State {
    let mut s = State::new(true);
    s.set("asleep", json!(false));
    s.set("exposure.mode", json!("auto"));
    s
}

fn patch(v: Value) -> Patch {
    Patch(v.as_object().expect("a patch is an object").clone())
}

fn code_of_set(state: &State, p: Value) -> Option<ErrorCode> {
    validate_set(&desc(), state, &patch(p))
        .err()
        .map(|r| r.code)
}

#[test]
fn the_test_description_is_consistent() {
    assert_eq!(check_description(&desc()), Vec::<String>::new());
}

#[test]
fn set_table() {
    use ErrorCode::*;
    let rows: Vec<(&str, Value, Option<ErrorCode>)> = vec![
        ("valid, dotted", json!({"led.brightness": 2}), None),
        ("valid, nested", json!({"led": {"brightness": 2}}), None),
        (
            "valid, float that is a whole number",
            json!({"led.brightness": 2.0}),
            None,
        ),
        (
            "valid range",
            json!({"zone.area.pan": {"min": -20, "max": 20}}),
            None,
        ),
        (
            "valid range, nested and dotted mixed",
            json!({"zone": {"area.pan": {"min": -20, "max": 20}}}),
            None,
        ),
        (
            "wrong type: string for a number",
            json!({"led.brightness": "2"}),
            Some(Invalid),
        ),
        (
            "wrong type: number for a bool",
            json!({"gimbal.invert": 1}),
            Some(Invalid),
        ),
        (
            "wrong type: null",
            json!({"gimbal.invert": null}),
            Some(Invalid),
        ),
        (
            "wrong type: range without max",
            json!({"zone.area.pan": {"min": -20}}),
            Some(Invalid),
        ),
        (
            "wrong type: range with an extra field",
            json!({"zone.area.pan": {"min": -20, "max": 20, "mid": 0}}),
            Some(Invalid),
        ),
        (
            "read-only leaf (4.2)",
            json!({"zone.return": {"pan": 0, "tilt": 0, "zoom": 1}}),
            Some(Invalid),
        ),
        ("read-only list", json!({"presets": []}), Some(Invalid)),
        (
            "a group is not a leaf (4.2 Leaves)",
            json!({"zoom": 2}),
            Some(Invalid),
        ),
        (
            "a leaf named twice",
            json!({"zoom": {"factor": 2}, "zoom.factor": 3}),
            Some(Invalid),
        ),
        ("empty patch", json!({}), Some(Invalid)),
        (
            "unknown path, a typo",
            json!({"led.brigthness": 2}),
            Some(Unsupported),
        ),
        (
            "a path of another model, not on this device",
            json!({"tracking.mode": "static"}),
            Some(Unsupported),
        ),
        ("above max", json!({"led.brightness": 4}), Some(OutOfRange)),
        ("below min", json!({"zoom.factor": 0.5}), Some(OutOfRange)),
        (
            "not on the step",
            json!({"led.brightness": 1.5}),
            Some(OutOfRange),
        ),
        (
            "enum value shown but not writable",
            json!({"zoom.field_of_view": "custom"}),
            Some(OutOfRange),
        ),
        (
            "enum value unknown",
            json!({"zoom.field_of_view": "narrow"}),
            Some(OutOfRange),
        ),
        (
            "range component out of bounds",
            json!({"zone.area.pan": {"min": -160, "max": 20}}),
            Some(OutOfRange),
        ),
        (
            "range min above max",
            json!({"zone.area.pan": {"min": 30, "max": 20}}),
            Some(OutOfRange),
        ),
        (
            "requires not met: time alone while auto (4.2)",
            json!({"exposure.time": 20000}),
            Some(Conflict),
        ),
        (
            "requires met by the same patch (4.2)",
            json!({"exposure": {"mode": "manual", "time": 20000}}),
            None,
        ),
        (
            "invalid wins over unsupported",
            json!({"led.brightness": "x", "nope": 1}),
            Some(Invalid),
        ),
        (
            "unsupported wins over out_of_range",
            json!({"led.brightness": 9, "nope": 1}),
            Some(Unsupported),
        ),
        (
            "out_of_range wins over conflict",
            json!({"led.brightness": 9, "exposure.time": 20000}),
            Some(OutOfRange),
        ),
        // Review Focus 1: float noise from a client's arithmetic is not a step violation.
        (
            "float noise on a stepped value",
            json!({"led.brightness": 2.000_000_000_1}),
            None,
        ),
        // Review Focus 2: the CLI's `min,max` syntax sent as a JSON array.
        (
            "a range as an array",
            json!({"zone.area.pan": [-20, 20]}),
            Some(Invalid),
        ),
        // Review Focus 3: malformed paths are refused, never a panic.
        (
            "an empty path segment",
            json!({"led..brightness": 2}),
            Some(Unsupported),
        ),
        ("a trailing dot", json!({"led.": 2}), Some(Unsupported)),
        ("an empty key", json!({"": 2}), Some(Unsupported)),
    ];
    for (name, p, want) in rows {
        assert_eq!(code_of_set(&awake(), p), want, "row: {name}");
    }
}

#[test]
fn asleep_ignored_is_refused_only_while_asleep() {
    let mut asleep = awake();
    asleep.set("asleep", json!(true));
    assert_eq!(
        code_of_set(&asleep, json!({"zoom.factor": 2})),
        Some(ErrorCode::Asleep),
        "ignored while asleep"
    );
    assert_eq!(
        code_of_set(&awake(), json!({"zoom.factor": 2})),
        None,
        "awake: written"
    );
    assert_eq!(
        code_of_set(&asleep, json!({"led.brightness": 2})),
        None,
        "asleep: unknown is written (5.7)"
    );
    assert_eq!(
        code_of_set(&asleep, json!({"zoom.factor": 2, "exposure.time": 300})),
        Some(ErrorCode::Conflict),
        "conflict wins over asleep"
    );
}

#[test]
fn a_refusal_names_the_path_at_fault() {
    let r = validate_set(
        &desc(),
        &awake(),
        &patch(json!({"led.brightness": 2, "zoom.factor": 9})),
    )
    .unwrap_err();
    assert_eq!(r.code, ErrorCode::OutOfRange);
    assert_eq!(r.path.as_deref(), Some("zoom.factor"));
    assert!(r.message.contains("zoom.factor"), "{}", r.message);
}

#[test]
fn a_valid_patch_comes_back_as_leaves_in_path_order() {
    let leaves = validate_set(
        &desc(),
        &awake(),
        &patch(json!({"zoom": {"factor": 2}, "led.brightness": 1})),
    )
    .unwrap();
    assert_eq!(
        leaves,
        vec![
            ("led.brightness".to_string(), json!(1)),
            ("zoom.factor".to_string(), json!(2))
        ]
    );
}

#[test]
fn do_table() {
    use ErrorCode::*;
    let rows: Vec<(&str, &str, Value, Option<ErrorCode>)> = vec![
        ("valid", "preset.save", json!({"slot": 1}), None),
        (
            "valid with the optional name",
            "preset.save",
            json!({"slot": 1, "name": "Desk"}),
            None,
        ),
        (
            "unknown action",
            "preset.load",
            json!({"slot": 1}),
            Some(Unsupported),
        ),
        ("missing parameter", "preset.save", json!({}), Some(Invalid)),
        (
            "unknown parameter",
            "preset.save",
            json!({"slot": 1, "speed": 3}),
            Some(Invalid),
        ),
        (
            "wrong type",
            "preset.save",
            json!({"slot": "one"}),
            Some(Invalid),
        ),
        (
            "out of range",
            "preset.save",
            json!({"slot": 3}),
            Some(OutOfRange),
        ),
        (
            "name too long",
            "preset.save",
            json!({"slot": 0, "name": "x".repeat(33)}),
            Some(OutOfRange),
        ),
        (
            "name not ASCII",
            "preset.save",
            json!({"slot": 0, "name": "Küche"}),
            Some(OutOfRange),
        ),
        (
            "destructive without confirm (4.2 warning)",
            "factory_reset",
            json!({}),
            Some(Invalid),
        ),
        (
            "destructive with confirm false",
            "factory_reset",
            json!({"confirm": false}),
            Some(Invalid),
        ),
        (
            "destructive with confirm true",
            "factory_reset",
            json!({"confirm": true}),
            None,
        ),
    ];
    for (name, action, params, want) in rows {
        let params = params.as_object().unwrap().clone();
        let got = validate_do(&desc(), &awake(), action, &params)
            .err()
            .map(|r| r.code);
        assert_eq!(got, want, "row: {name}");
    }
    let mut asleep = awake();
    asleep.set("asleep", json!(true));
    let params = json!({"slot": 1}).as_object().unwrap().clone();
    assert_eq!(
        validate_do(&desc(), &asleep, "preset.save", &params)
            .err()
            .map(|r| r.code),
        Some(ErrorCode::Asleep)
    );
}

#[test]
fn get_accepts_leaves_groups_and_daemon_fields_only() {
    let ok = |paths: &[&str]| {
        validate_get(
            &desc(),
            &paths.iter().map(|p| p.to_string()).collect::<Vec<_>>(),
        )
    };
    assert!(
        ok(&[
            "zoom.factor",
            "zone",
            "zone.area",
            "pending",
            "available",
            "unread"
        ])
        .is_ok()
    );
    assert_eq!(ok(&["tracking"]).unwrap_err().code, ErrorCode::Unsupported);
    assert_eq!(
        ok(&["zoom.factorx"]).unwrap_err().code,
        ErrorCode::Unsupported
    );
    // Review Focus 5: a component of a leaf is not a path of its own.
    assert_eq!(
        ok(&["zone.area.pan.min"]).unwrap_err().code,
        ErrorCode::Unsupported
    );
}

#[test]
fn state_values_may_be_null_where_nullable_and_raw_where_undecodable() {
    let mode = Setting::enumeration("Mode", &["a", "b"]);
    assert!(
        check_value(&mode, &json!("raw:7"), Fit::State).is_ok(),
        "4.2 Undecodable values"
    );
    assert!(
        check_value(&mode, &json!("raw:7"), Fit::Write).is_err(),
        "raw values are never written"
    );
    assert!(
        check_value(&mode, &Value::Null, Fit::State).is_err(),
        "null only where nullable"
    );
    assert!(check_value(&mode.clone().nullable(), &Value::Null, Fit::State).is_ok());
}

#[test]
fn check_description_finds_broken_descriptions() {
    let broken = Description::default()
        .setting("Zoom.Factor", Setting::number("Zoom", 4.0, 1.0))
        .setting(
            "zone.area",
            Setting::boolean("Zone")
                .asleep(Asleep::Unknown)
                .settle_ms(3000),
        )
        .setting("zone.area.pan", Setting::range("Pan", -150.0, 150.0))
        .setting(
            "mode",
            Setting::enumeration("Mode", &["a"])
                .writable_only(&["b"])
                .asleep(Asleep::Unknown)
                .settle_ms(3000),
        )
        .action(
            "factory_reset",
            Action::new("Reset", Asleep::Unknown).warning(Warning::Destructive),
        )
        .limit("zone.area", 3);
    let problems = check_description(&broken).join("\n");
    for expected in [
        "Zoom.Factor: not lower-case",
        "Zoom.Factor: min is above max",
        "Zoom.Factor: a writable leaf needs asleep",
        "zone.area: a leaf that is also a group",
        "zone.area.pan: a writable leaf needs asleep",
        "mode: writable holds a value that values does not",
        "factory_reset: a destructive action needs the bool parameter confirm",
        "limits.zone.area: not a list setting",
    ] {
        assert!(
            problems.contains(expected),
            "missing {expected:?} in:\n{problems}"
        );
    }
}
