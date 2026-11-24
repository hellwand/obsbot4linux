// SPDX-License-Identifier: GPL-3.0-or-later

//! Every row of the parity table (design spec 9.1) turned into its 1.0 command and run
//! against `fake-a` of an in-process `obsbot4linuxd --fake fake-a,fake-b,unknown`: the exit
//! code, and exactly the camera writes the row names. A `set` reaches the fake as one
//! `apply` per leaf and no `act`; a `do` as exactly one `act` and no `apply` (spec 2): the
//! gimbal rows move without touching the tracking. `watch` and `watch --live pose` are
//! covered by `e2e.rs` (`watch_sees_a_change_made_by_another_command`). Needs track B's
//! milestone B-1.

mod common;
mod fake_daemon;

use fake_daemon::{FakeDaemon, eventually};
use obsbot_device::models::fake::Op;
use obsbot_proto::values::same;
use serde_json::{Map, Value, json};

const A: &str = "sn:FAKEA0000001";

/// What a row sends to the camera.
enum Sends {
    /// No camera write (a read, a refusal, help).
    Nothing,
    /// One `apply` per (leaf, JSON value), in any order, and no `act`.
    Apply(&'static [(&'static str, &'static str)]),
    /// Exactly one `act` with these parameters (a JSON object), and no `apply`.
    Act(&'static str, &'static str),
}

use Sends::{Act, Apply, Nothing};

struct Row {
    /// The 0.13.2 command of 9.1.
    was: &'static str,
    /// The 1.0 command; `--device fake-a` is added unless it is `devices`, help or version.
    now: &'static [&'static str],
    exit: u8,
    sends: Sends,
}

const fn row(was: &'static str, now: &'static [&'static str], exit: u8, sends: Sends) -> Row {
    Row {
        was,
        now,
        exit,
        sends,
    }
}

/// In the order they run: presets and views exist before the rows that need them; the
/// tracking is on before the gimbal rows; the rows whose `warning` holds `restart` are last.
const ROWS: &[Row] = &[
    row("--help", &["--help"], 0, Nothing),
    row("--version", &["--version"], 0, Nothing),
    row("--direct (PX-3)", &["--direct", "status"], 2, Nothing),
    row("detect", &["devices"], 0, Nothing),
    row("detect --json", &["devices", "--json"], 0, Nothing),
    row("status", &["status"], 0, Nothing),
    row("status --json", &["status", "--json"], 0, Nothing),
    row("sleep", &["do", "sleep"], 0, Act("sleep", "{}")),
    row("wake", &["do", "wake"], 0, Act("wake", "{}")),
    row(
        "wake zoom F",
        &["set", "sleep.wake_zoom=2"],
        0,
        Apply(&[("sleep.wake_zoom", "2")]),
    ),
    row(
        "wake mode M",
        &["set", "power_on.mode=upper_body"],
        0,
        Apply(&[("power_on.mode", "\"upper_body\"")]),
    ),
    row(
        "tracking MODE",
        &["set", "tracking.mode=normal"],
        0,
        Apply(&[("tracking.mode", "\"normal\"")]),
    ),
    row(
        "speed standard|sport",
        &["set", "tracking.profile=sport"],
        0,
        Apply(&[("tracking.profile", "\"sport\"")]),
    ),
    row(
        "speed headroom",
        &["set", "tracking.profile=headroom"],
        0,
        Apply(&[("tracking.profile", "\"headroom\"")]),
    ),
    row(
        "speed-mode MODE",
        &["set", "tracking.speed=fast"],
        0,
        Apply(&[("tracking.speed", "\"fast\"")]),
    ),
    row(
        "framing auto-zoom on|off",
        &["set", "framing.auto_zoom=on"],
        0,
        Apply(&[("framing.auto_zoom", "true")]),
    ),
    row(
        "framing size S",
        &["set", "framing.size=close_up"],
        0,
        Apply(&[("framing.size", "\"close_up\"")]),
    ),
    row(
        "framing zoom-speed S",
        &["set", "framing.zoom_speed=fast"],
        0,
        Apply(&[("framing.zoom_speed", "\"fast\"")]),
    ),
    row(
        "framing offset --x X --y Y",
        &["set", "framing.offset.x=0.1", "framing.offset.y=-0.1"],
        0,
        Apply(&[("framing.offset.x", "0.1"), ("framing.offset.y", "-0.1")]),
    ),
    row(
        "framing part P",
        &["set", "framing.part=head_removal"],
        0,
        Apply(&[("framing.part", "\"head_removal\"")]),
    ),
    row(
        "hdr on|off",
        &["set", "exposure.hdr=on"],
        0,
        Apply(&[("exposure.hdr", "true")]),
    ),
    row(
        "exposure global|face",
        &["set", "exposure.mode=auto", "exposure.region=face"],
        0,
        Apply(&[
            ("exposure.mode", "\"auto\""),
            ("exposure.region", "\"face\""),
        ]),
    ),
    row(
        "exposure manual",
        &["set", "exposure.mode=manual"],
        0,
        Apply(&[("exposure.mode", "\"manual\"")]),
    ),
    row(
        "exposure manual --time T",
        &["set", "exposure.mode=manual", "exposure.time=20000"],
        0,
        Apply(&[("exposure.mode", "\"manual\""), ("exposure.time", "20000")]),
    ),
    row(
        "exposure manual --gain G",
        &["set", "exposure.mode=manual", "exposure.gain=8"],
        0,
        Apply(&[("exposure.mode", "\"manual\""), ("exposure.gain", "8")]),
    ),
    row(
        "zoom F",
        &["set", "zoom.factor=2"],
        0,
        Apply(&[("zoom.factor", "2")]),
    ),
    row(
        "zoom F --speed S",
        &["do", "zoom.move", "factor=1.5", "speed=medium"],
        0,
        Act("zoom.move", r#"{"factor":1.5,"speed":"medium"}"#),
    ),
    row("zoom stop", &["do", "zoom.stop"], 0, Act("zoom.stop", "{}")),
    row(
        "gimbal --pan P --tilt T",
        &["do", "gimbal.move", "pan=10", "tilt=5"],
        0,
        Act("gimbal.move", r#"{"pan":10,"tilt":5}"#),
    ),
    row(
        "gimbal --pan P (one axis)",
        &["do", "gimbal.move", "pan=-10"],
        0,
        Act("gimbal.move", r#"{"pan":-10}"#),
    ),
    row(
        "gimbal … --speed S",
        &["do", "gimbal.move", "pan=10", "tilt=5", "speed=30"],
        0,
        Act("gimbal.move", r#"{"pan":10,"tilt":5,"speed":30}"#),
    ),
    row(
        "gimbal stop",
        &["do", "gimbal.stop"],
        0,
        Act("gimbal.stop", "{}"),
    ),
    row(
        "gimbal rate --pan P --tilt T",
        &["do", "gimbal.turn", "pan_speed=20", "tilt_speed=0"],
        0,
        Act(
            "gimbal.turn",
            r#"{"pan_speed":20,"tilt_speed":0,"keep":true}"#,
        ),
    ),
    row(
        "gimbal stop (after rate)",
        &["do", "gimbal.stop"],
        0,
        Act("gimbal.stop", "{}"),
    ),
    row(
        "fov wide|medium|narrow",
        &["set", "zoom.field_of_view=wide"],
        0,
        Apply(&[("zoom.field_of_view", "\"wide\"")]),
    ),
    row(
        "led N",
        &["set", "led.brightness=2"],
        0,
        Apply(&[("led.brightness", "2")]),
    ),
    row(
        "led blink on|off",
        &["set", "led.blink=on"],
        0,
        Apply(&[("led.blink", "true")]),
    ),
    row(
        "mirror on|off",
        &["set", "image.mirror=on"],
        0,
        Apply(&[("image.mirror", "true")]),
    ),
    row(
        "invert on|off",
        &["set", "gimbal.invert=on"],
        0,
        Apply(&[("gimbal.invert", "true")]),
    ),
    row(
        "auto-sleep off",
        &["set", "sleep.auto.enabled=off"],
        0,
        Apply(&[("sleep.auto.enabled", "false")]),
    ),
    row(
        "auto-sleep N",
        &["set", "sleep.auto.enabled=on", "sleep.auto.seconds=300"],
        0,
        Apply(&[
            ("sleep.auto.enabled", "true"),
            ("sleep.auto.seconds", "300"),
        ]),
    ),
    row(
        "auto-sleep 0 (PX-7)",
        &["set", "sleep.auto.seconds=0"],
        2,
        Nothing,
    ),
    row(
        "sleep-background black|image",
        &["set", "sleep.background=black"],
        0,
        Apply(&[("sleep.background", "\"black\"")]),
    ),
    row(
        "sleep-background-mirror on|off",
        &["set", "sleep.background_mirror=on"],
        0,
        Apply(&[("sleep.background_mirror", "true")]),
    ),
    row(
        "audio noise L",
        &["set", "audio.noise_reduction=strong"],
        0,
        Apply(&[("audio.noise_reduction", "\"strong\"")]),
    ),
    row(
        "audio gain on|off",
        &["set", "audio.auto_gain=on"],
        0,
        Apply(&[("audio.auto_gain", "true")]),
    ),
    row(
        "audio distance near",
        &["set", "audio.volume=10"],
        0,
        Apply(&[("audio.volume", "10")]),
    ),
    row(
        "audio distance standard",
        &["set", "audio.volume=50"],
        0,
        Apply(&[("audio.volume", "50")]),
    ),
    row(
        "audio distance far",
        &["set", "audio.volume=100"],
        0,
        Apply(&[("audio.volume", "100")]),
    ),
    row(
        "audio volume N",
        &["set", "audio.volume=70"],
        0,
        Apply(&[("audio.volume", "70")]),
    ),
    row(
        "audio sleep-mic on|off",
        &["set", "audio.in_sleep=on"],
        0,
        Apply(&[("audio.in_sleep", "true")]),
    ),
    row(
        "gesture set target-lock",
        &["set", "gestures.target_lock=on"],
        0,
        Apply(&[("gestures.target_lock", "true")]),
    ),
    row(
        "gesture set zoom",
        &["set", "gestures.zoom=on"],
        0,
        Apply(&[("gestures.zoom", "true")]),
    ),
    row(
        "gesture set dynamic-zoom",
        &["set", "gestures.dynamic_zoom=on"],
        0,
        Apply(&[("gestures.dynamic_zoom", "true")]),
    ),
    row(
        "gesture set direction-change",
        &["set", "gestures.direction_change=on"],
        0,
        Apply(&[("gestures.direction_change", "true")]),
    ),
    row(
        "gesture zoom-ratio R",
        &["set", "gestures.zoom_ratio=2.5"],
        0,
        Apply(&[("gestures.zoom_ratio", "2.5")]),
    ),
    row(
        "hand right|left",
        &["set", "hand.side=left"],
        0,
        Apply(&[("hand.side", "\"left\"")]),
    ),
    row(
        "hand return-here",
        &["do", "hand.return_here"],
        0,
        Act("hand.return_here", "{}"),
    ),
    row(
        "hand return-go",
        &["do", "hand.go_return"],
        0,
        Act("hand.go_return", "{}"),
    ),
    row(
        "hand return-reset",
        &["do", "hand.return_clear"],
        0,
        Act("hand.return_clear", "{}"),
    ),
    row(
        "hand area --pan --tilt",
        &["set", "hand.area.pan=-45,45", "hand.area.tilt=-30,30"],
        0,
        Apply(&[
            ("hand.area.pan", r#"{"min":-45,"max":45}"#),
            ("hand.area.tilt", r#"{"min":-30,"max":30}"#),
        ]),
    ),
    row(
        "hand area-reset",
        &["do", "hand.area_reset"],
        0,
        Act("hand.area_reset", "{}"),
    ),
    row(
        "hand pan on|off",
        &["set", "hand.axes.pan=on"],
        0,
        Apply(&[("hand.axes.pan", "true")]),
    ),
    row(
        "hand tilt on|off",
        &["set", "hand.axes.tilt=off"],
        0,
        Apply(&[("hand.axes.tilt", "false")]),
    ),
    row(
        "hand return-after S",
        &["set", "hand.return_after=5"],
        0,
        Apply(&[("hand.return_after", "5")]),
    ),
    row(
        "hand gimbal on|off",
        &["set", "hand.move_gimbal=off"],
        0,
        Apply(&[("hand.move_gimbal", "false")]),
    ),
    row(
        "zone on|off",
        &["set", "zone.enabled=on"],
        0,
        Apply(&[("zone.enabled", "true")]),
    ),
    row(
        "zone follow on|off",
        &["set", "zone.follow=on"],
        0,
        Apply(&[("zone.follow", "true")]),
    ),
    row(
        "zone area --pan --tilt",
        &["set", "zone.area.pan=-20,20", "zone.area.tilt=-10,10"],
        0,
        Apply(&[
            ("zone.area.pan", r#"{"min":-20,"max":20}"#),
            ("zone.area.tilt", r#"{"min":-10,"max":10}"#),
        ]),
    ),
    row(
        "zone return-here",
        &["do", "zone.return_here"],
        0,
        Act("zone.return_here", "{}"),
    ),
    row(
        "zone return-go",
        &["do", "zone.go_return"],
        0,
        Act("zone.go_return", "{}"),
    ),
    row("zone view list", &["get", "views"], 0, Nothing),
    row(
        "zone view save NAME",
        &["do", "view.save", "name=Desk"],
        0,
        Act("view.save", r#"{"name":"Desk"}"#),
    ),
    row(
        "zone view save NAME --pan --tilt --zoom",
        &["do", "view.save", "name=Door", "pose=10,5,1.2"],
        0,
        Act(
            "view.save",
            r#"{"name":"Door","pose":{"pan":10,"tilt":5,"zoom":1.2}}"#,
        ),
    ),
    row(
        "zone view rename ID NAME",
        &["do", "view.rename", "id=0", "name=Desk2"],
        0,
        Act("view.rename", r#"{"id":0,"name":"Desk2"}"#),
    ),
    row(
        "zone view go ID",
        &["do", "view.go", "id=0"],
        0,
        Act("view.go", r#"{"id":0}"#),
    ),
    row(
        "zone view delete ID",
        &["do", "view.delete", "id=1"],
        0,
        Act("view.delete", r#"{"id":1}"#),
    ),
    row(
        "focus mode face|global",
        &["set", "focus.region=face"],
        0,
        Apply(&[("focus.region", "\"face\"")]),
    ),
    row(
        "focus auto on",
        &["set", "focus.mode=continuous"],
        0,
        Apply(&[("focus.mode", "\"continuous\"")]),
    ),
    row(
        "focus auto off (CAM-14)",
        &["set", "focus.mode=manual"],
        0,
        Apply(&[("focus.mode", "\"manual\"")]),
    ),
    row(
        "focus af continuous",
        &["set", "focus.mode=continuous"],
        0,
        Apply(&[("focus.mode", "\"continuous\"")]),
    ),
    row(
        "focus af manual",
        &["set", "focus.mode=manual"],
        0,
        Apply(&[("focus.mode", "\"manual\"")]),
    ),
    row(
        "focus af single",
        &["set", "focus.mode=single"],
        0,
        Apply(&[("focus.mode", "\"single\"")]),
    ),
    row(
        "focus point X Y",
        &["do", "focus.point", "x=0.5", "y=0.5"],
        0,
        Act("focus.point", r#"{"x":0.5,"y":0.5}"#),
    ),
    row(
        "focus set N",
        &["set", "focus.mode=manual", "focus.position=40"],
        0,
        Apply(&[("focus.mode", "\"manual\""), ("focus.position", "40")]),
    ),
    row("preset list", &["get", "presets"], 0, Nothing),
    row(
        "preset save N",
        &["do", "preset.save", "slot=0"],
        0,
        Act("preset.save", r#"{"slot":0}"#),
    ),
    row(
        "preset save N --name S",
        &["do", "preset.save", "slot=1", "name=Door"],
        0,
        Act("preset.save", r#"{"slot":1,"name":"Door"}"#),
    ),
    row(
        "preset save N --pan --tilt --zoom",
        &["do", "preset.save", "slot=2", "pose=10,0,1"],
        0,
        Act(
            "preset.save",
            r#"{"slot":2,"pose":{"pan":10,"tilt":0,"zoom":1}}"#,
        ),
    ),
    row(
        "preset rename N S",
        &["do", "preset.rename", "slot=0", "name=Desk"],
        0,
        Act("preset.rename", r#"{"slot":0,"name":"Desk"}"#),
    ),
    row(
        "preset recall N",
        &["do", "preset.recall", "slot=0"],
        0,
        Act("preset.recall", r#"{"slot":0}"#),
    ),
    row(
        "preset delete N",
        &["do", "preset.delete", "slot=2"],
        0,
        Act("preset.delete", r#"{"slot":2}"#),
    ),
    row(
        "startup set --pan --tilt --zoom",
        &["do", "power_on.set_pose", "pose=0,10,1"],
        0,
        Act(
            "power_on.set_pose",
            r#"{"pose":{"pan":0,"tilt":10,"zoom":1}}"#,
        ),
    ),
    row(
        "startup set --preset N",
        &["do", "power_on.use_preset", "slot=0"],
        0,
        Act("power_on.use_preset", r#"{"slot":0}"#),
    ),
    row(
        "startup set --current",
        &["do", "power_on.use_current"],
        0,
        Act("power_on.use_current", "{}"),
    ),
    // `startup set <pose option> --ai M` is two commands: the `do` of the pose option, then
    // the `set` (spec 9.1); each is its own row and its own request.
    row(
        "startup set --current --ai M (1 of 2)",
        &["do", "power_on.use_current"],
        0,
        Act("power_on.use_current", "{}"),
    ),
    row(
        "startup set --current --ai M (2 of 2)",
        &["set", "power_on.start.mode=upper_body"],
        0,
        Apply(&[("power_on.start.mode", "\"upper_body\"")]),
    ),
    row(
        "startup set --ai M",
        &["set", "power_on.start.mode=upper_body"],
        0,
        Apply(&[("power_on.start.mode", "\"upper_body\"")]),
    ),
    row(
        "startup set --ai none",
        &["set", "power_on.start.mode=unset"],
        0,
        Apply(&[("power_on.start.mode", "\"unset\"")]),
    ),
    row("startup show", &["get", "power_on"], 0, Nothing),
    row(
        "startup go",
        &["do", "power_on.go"],
        0,
        Act("power_on.go", "{}"),
    ),
    row(
        "image brightness V",
        &["set", "image.brightness=60"],
        0,
        Apply(&[("image.brightness", "60")]),
    ),
    row(
        "image contrast V",
        &["set", "image.contrast=60"],
        0,
        Apply(&[("image.contrast", "60")]),
    ),
    row(
        "image saturation V",
        &["set", "image.saturation=60"],
        0,
        Apply(&[("image.saturation", "60")]),
    ),
    row(
        "image hue V",
        &["set", "image.hue=60"],
        0,
        Apply(&[("image.hue", "60")]),
    ),
    row(
        "image sharpness V",
        &["set", "image.sharpness=60"],
        0,
        Apply(&[("image.sharpness", "60")]),
    ),
    row(
        "image white_balance_auto 0",
        &["set", "image.white_balance.auto=off"],
        0,
        Apply(&[("image.white_balance.auto", "false")]),
    ),
    row(
        "image white_balance_temperature V",
        &[
            "set",
            "image.white_balance.auto=off",
            "image.white_balance.temperature=4500",
        ],
        0,
        Apply(&[
            ("image.white_balance.auto", "false"),
            ("image.white_balance.temperature", "4500"),
        ]),
    ),
    row(
        "image white_balance_auto 1",
        &["set", "image.white_balance.auto=on"],
        0,
        Apply(&[("image.white_balance.auto", "true")]),
    ),
    row(
        "image gain V",
        &["set", "exposure.gain=8"],
        0,
        Apply(&[("exposure.gain", "8")]),
    ),
    row(
        "image power_line_frequency 0",
        &["set", "exposure.anti_flicker=off"],
        0,
        Apply(&[("exposure.anti_flicker", "\"off\"")]),
    ),
    row(
        "image power_line_frequency 2",
        &["set", "exposure.anti_flicker=60hz"],
        0,
        Apply(&[("exposure.anti_flicker", "\"60hz\"")]),
    ),
    row(
        "image power_line_frequency 1",
        &["set", "exposure.anti_flicker=50hz"],
        0,
        Apply(&[("exposure.anti_flicker", "\"50hz\"")]),
    ),
    row(
        "image backlight_compensation V",
        &["set", "exposure.compensation=12"],
        0,
        Apply(&[("exposure.compensation", "12")]),
    ),
    row(
        "image auto_exposure 0|3",
        &["set", "exposure.mode=auto"],
        0,
        Apply(&[("exposure.mode", "\"auto\"")]),
    ),
    row(
        "image auto_exposure 1",
        &["set", "exposure.mode=manual"],
        0,
        Apply(&[("exposure.mode", "\"manual\"")]),
    ),
    row(
        "image exposure_time V",
        &["set", "exposure.mode=manual", "exposure.time=10000"],
        0,
        Apply(&[("exposure.mode", "\"manual\""), ("exposure.time", "10000")]),
    ),
    row(
        "image focus V",
        &["set", "focus.mode=manual", "focus.position=30"],
        0,
        Apply(&[("focus.mode", "\"manual\""), ("focus.position", "30")]),
    ),
    row(
        "image autofocus 0 (CAM-14)",
        &["set", "focus.mode=manual"],
        0,
        Apply(&[("focus.mode", "\"manual\"")]),
    ),
    row(
        "image autofocus 1",
        &["set", "focus.mode=continuous"],
        0,
        Apply(&[("focus.mode", "\"continuous\"")]),
    ),
    row(
        "image gamma V (PX-2)",
        &["set", "image.gamma=1"],
        2,
        Nothing,
    ),
    row(
        "factory-reset without --yes",
        &["do", "factory_reset"],
        2,
        Nothing,
    ),
];

/// The rows whose write restarts the camera, run last, each followed by a wait until the
/// device is back.
const RESTARTS: &[Row] = &[
    row(
        "audio mic on|off",
        &["set", "audio.enabled=off"],
        0,
        Apply(&[("audio.enabled", "false")]),
    ),
    row(
        "factory-reset --yes",
        &["do", "factory_reset", "confirm=true"],
        0,
        Act("factory_reset", r#"{"confirm":true}"#),
    ),
];

fn json(text: &str) -> Value {
    serde_json::from_str(text).unwrap_or_else(|e| panic!("{e}: {text}"))
}

fn args_of(row: &Row) -> Vec<&'static str> {
    let mut args = row.now.to_vec();
    let global = matches!(args[0], "devices" | "--help" | "--version");
    if !global {
        args.extend(["--device", "fake-a"]);
    }
    args
}

fn check(d: &FakeDaemon, row: &Row) {
    let from = d.op_count(A);
    let args = args_of(row);
    let (code, out, err) = d.cli(&args);
    assert_eq!(code, row.exit, "{}: {args:?}\n{out}{err}", row.was);
    let writes = d.writes_since(A, from);
    match &row.sends {
        Nothing => assert!(writes.is_empty(), "{}: {writes:?}", row.was),
        Act(action, params) => {
            let params: Map<String, Value> = json(params).as_object().unwrap().clone();
            assert_eq!(
                writes,
                [Op::Act(action.to_string(), params)],
                "{}: one act, no apply",
                row.was
            );
        }
        Apply(leaves) => {
            assert!(
                writes.iter().all(|w| matches!(w, Op::Apply(..))),
                "{}: no act: {writes:?}",
                row.was
            );
            assert_eq!(writes.len(), leaves.len(), "{}: {writes:?}", row.was);
            for (leaf, value) in *leaves {
                let want = json(value);
                assert!(
                    writes
                        .iter()
                        .any(|w| matches!(w, Op::Apply(l, v) if l == leaf && same(v, &want))),
                    "{}: no apply of {leaf} = {value}: {writes:?}",
                    row.was
                );
            }
        }
    }
}

fn tracking_mode(d: &FakeDaemon) -> Value {
    let (_, out, _) = d.cli(&["--json", "get", "tracking.mode", "--device", "fake-a"]);
    json(out.trim_end())["state"]["tracking"]["mode"].clone()
}

#[test]
fn every_parity_row_reaches_fake_a_as_its_one_request() {
    let d = FakeDaemon::start("parity");
    for row in ROWS {
        check(&d, row);
        // Spec 2 and 4.4: the gimbal rows leave the tracking as it is.
        if row.now.first() == Some(&"do") && row.now[1].starts_with("gimbal.") {
            assert_eq!(tracking_mode(&d), json!("normal"), "{}", row.was);
        }
    }
    for row in RESTARTS {
        // The restart is ordinary hotplug (spec 5.6): fake-a goes away and comes back with a
        // new device number. Until it goes, the daemon still shows it available, so the next
        // row waits for the re-open first, then for the published availability.
        let from = d.op_count(A);
        check(&d, row);
        eventually("fake-a re-opened after the restart", || {
            d.bus
                .ops(A)
                .iter()
                .skip(from)
                .any(|op| matches!(op, Op::Open))
        });
        eventually("fake-a back after the restart", || {
            let (code, out, _) = d.cli(&["--json", "get", "available", "--device", "fake-a"]);
            code == 0 && json(out.trim_end())["state"]["available"] == json!(true)
        });
    }
}

/// Every 0.13.2 command of 9.1 has a row above, or is covered elsewhere.
#[test]
fn every_0_13_command_of_the_parity_table_has_a_row() {
    const COMMANDS: [&str; 35] = [
        "--json",
        "--direct",
        "--help",
        "--version",
        "daemon",
        "watch",
        "live-pose",
        "detect",
        "status",
        "sleep",
        "wake",
        "tracking",
        "speed",
        "speed-mode",
        "framing",
        "hdr",
        "exposure",
        "zoom",
        "gimbal",
        "fov",
        "led",
        "mirror",
        "invert",
        "auto-sleep",
        "sleep-background",
        "sleep-background-mirror",
        "audio",
        "gesture",
        "hand",
        "zone",
        "focus",
        "startup",
        "image",
        "preset",
        "factory-reset",
    ];
    let elsewhere = [
        "watch: e2e.rs watch_sees_a_change_made_by_another_command",
        "live-pose: watch --live pose, the same e2e test",
        "daemon: obsbot4linuxd, track B",
        "gimbal center: PX-1, no 1.0 command",
    ];
    for command in COMMANDS {
        let has_row = ROWS
            .iter()
            .chain(RESTARTS)
            .any(|r| r.was.split_whitespace().any(|w| w == command));
        let named = elsewhere
            .iter()
            .any(|e| e.starts_with(&format!("{command}:")));
        assert!(has_row || named, "{command} has no row");
    }
}
