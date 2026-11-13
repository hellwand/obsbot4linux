// SPDX-License-Identifier: GPL-3.0-or-later

//! End to end against an in-process `obsbot4linuxd --fake fake-a,fake-b,unknown` (design spec
//! 10.3 track C, 11): the command line, the daemon of track B and the fakes of W0 together.
//! Needs track B's milestone B-1.

mod common;
mod fake_daemon;

use fake_daemon::{FakeDaemon, eventually, snapshot};
use obsbot_device::models::fake::{self, Op};
use serde_json::{Value, json};
use std::io::Write;
use std::sync::{Arc, Mutex};

const A: &str = "sn:FAKEA0000001";
const B: &str = "sn:FAKEB0000001";
const UNKNOWN: &str = "port:9-3";

fn parse(out: &str) -> Value {
    serde_json::from_str(out.trim_end()).unwrap_or_else(|e| panic!("{e}: {out}"))
}

#[test]
fn devices_lists_the_two_fakes_and_the_unsupported_device() {
    let d = FakeDaemon::start("devices");
    let (code, out, _) = d.cli(&["devices", "--json"]);
    assert_eq!(code, 0);
    let v = parse(&out);
    let mut seen: Vec<(String, String, String)> = v["devices"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| {
            (
                d["id"].as_str().unwrap().to_string(),
                d["model"].as_str().unwrap().to_string(),
                d["support"].as_str().unwrap().to_string(),
            )
        })
        .collect();
    seen.sort();
    assert_eq!(
        seen,
        [
            (UNKNOWN.into(), "unknown".into(), "unsupported".into()),
            (A.into(), "fake-a".into(), "tested".into()),
            (B.into(), "fake-b".into(), "untested".into()),
        ]
    );
    let (code, out, _) = d.cli(&["devices"]);
    assert_eq!(code, 0);
    assert!(
        !out.contains("FAKEA0000001"),
        "human output shortens: {out}"
    );
    snapshot("devices.txt", &out);
    let (code, out, _) = d.cli(&["devices", "--report"]);
    assert_eq!(code, 0);
    assert!(!out.contains("FAKE"), "no serial in a report: {out}");
    snapshot("devices-report.json", &out);
}

#[test]
fn two_usable_fakes_need_a_device() {
    let d = FakeDaemon::start("ambiguous");
    let (code, out, _) = d.cli(&["--json", "status"]);
    assert_eq!(code, 2);
    let v = parse(&out);
    assert_eq!(v["error"]["code"], "ambiguous_device");
    assert_eq!(v["error"]["devices"], json!([A, B]));
}

#[test]
fn status_of_fake_a_in_both_forms() {
    let d = FakeDaemon::start("status");
    let (code, out, _) = d.cli(&["status", "--device", "fake-a", "--json"]);
    assert_eq!(code, 0);
    let v = parse(&out);
    assert_eq!(v["device"]["id"], A);
    assert_eq!(v["state"]["available"], true);
    assert!(v["state"]["tracking"]["mode"].is_string());
    let (code, out, _) = d.cli(&["status", "--device", "fake-a"]);
    assert_eq!(code, 0);
    snapshot("status-fake-a.txt", &out);
}

/// Spec 8.4 steps 4 to 6 with the `unknown` fake: listed, refused by the CLI before anything
/// is sent, and it never receives a camera operation.
#[test]
fn the_unsupported_device_is_refused_and_receives_nothing() {
    let d = FakeDaemon::start("unsupported");
    for args in [
        vec!["status", "--device", "unknown"],
        vec!["set", "led.brightness=1", "--device", "unknown"],
        vec!["do", "sleep", "--device", UNKNOWN],
    ] {
        let (code, _, err) = d.cli(&args);
        assert_eq!(code, 3, "{args:?}");
        assert!(err.contains("is not supported by obsbot4linux"), "{err}");
    }
    let (code, out, _) = d.cli(&["describe", "--device", "unknown", "--json"]);
    assert_eq!(code, 0);
    assert_eq!(parse(&out)["settings"], json!({}));
    assert!(d.bus.ops(UNKNOWN).is_empty(), "{:?}", d.bus.ops(UNKNOWN));
}

#[test]
fn a_set_reaches_fake_b_as_one_apply_and_shows_in_the_state() {
    let d = FakeDaemon::start("set");
    let from = d.op_count(B);
    let (code, out, err) = d.cli(&["set", "led.brightness=5", "--device", "fake-b"]);
    assert_eq!((code, out.as_str(), err.as_str()), (0, "", ""));
    assert_eq!(
        d.writes_since(B, from),
        [Op::Apply("led.brightness".into(), json!(5))]
    );
    // fake-b's readback lags (W0 decision 13): pending first, or the value once read.
    let (code, out, _) = d.cli(&["--json", "get", "led", "--device", "fake-b"]);
    assert_eq!(code, 0);
    let v = parse(&out);
    assert!(
        v["state"]["pending"]["led.brightness"] == json!(5)
            || v["state"]["led"]["brightness"] == json!(5),
        "{v}"
    );
    eventually("led.brightness to read back", || {
        let (_, out, _) = d.cli(&["--json", "get", "led", "--device", "fake-b"]);
        parse(&out)["state"]["led"]["brightness"] == json!(5)
    });
}

/// The daemon's refusals keep their codes; nothing reaches the fake.
#[test]
fn refusals_reach_no_camera() {
    let d = FakeDaemon::start("refusals");
    let (a, b) = (d.op_count(A), d.op_count(B));
    let cases: [(&[&str], &str, u8); 4] = [
        (
            &["set", "zoom.factor=3", "--device", "fake-b"],
            "out_of_range",
            2,
        ),
        (
            &["set", "exposure.time=500", "--device", "fake-b"],
            "conflict",
            3,
        ),
        (&["set", "gimbal.pan=1", "--device", "fake-a"], "invalid", 2),
        (
            &["set", "image.gamma=1", "--device", "fake-a"],
            "unsupported",
            2,
        ),
    ];
    for (args, code, exit) in cases {
        let mut all = vec!["--json"];
        all.extend_from_slice(args);
        let (got, out, _) = d.cli(&all);
        assert_eq!(got, exit, "{args:?}");
        assert_eq!(parse(&out)["error"]["code"], code, "{args:?}");
    }
    assert!(d.writes_since(A, a).is_empty());
    assert!(d.writes_since(B, b).is_empty());
}

/// fake-b's `zoom.factor` is `asleep: ignored`: refused with `asleep` while it sleeps.
#[test]
fn an_ignored_setting_is_refused_while_asleep() {
    let d = FakeDaemon::start("asleep");
    d.bus.sleep(B).unwrap();
    eventually("fake-b to report asleep", || {
        let (_, out, _) = d.cli(&["--json", "get", "asleep", "--device", "fake-b"]);
        parse(&out)["state"]["asleep"] == json!(true)
    });
    let (code, out, _) = d.cli(&["--json", "set", "zoom.factor=1.5", "--device", "fake-b"]);
    assert_eq!(code, 3);
    assert_eq!(parse(&out)["error"]["code"], "asleep");
    d.bus.wake(B).unwrap();
}

#[test]
fn describe_of_fake_b_is_its_own_description() {
    let d = FakeDaemon::start("describe");
    let desc = fake::description("fake-b").unwrap();
    let (code, out, _) = d.cli(&["describe", "--device", "fake-b", "--json"]);
    assert_eq!(code, 0);
    let v = parse(&out);
    assert_eq!(v["settings"], serde_json::to_value(&desc.settings).unwrap());
    assert_eq!(v["actions"], serde_json::to_value(&desc.actions).unwrap());
    assert_eq!(v["limits"], json!({"presets": 5}));
    let (code, out, _) = d.cli(&["describe", "beacon.flash", "--device", "fake-b", "--json"]);
    assert_eq!(code, 0);
    assert_eq!(
        parse(&out),
        serde_json::to_value(&desc.actions["beacon.flash"]).unwrap()
    );
    let (code, out, _) = d.cli(&["describe", "--device", "fake-b"]);
    assert_eq!(code, 0);
    snapshot("describe-fake-b.txt", &out);
}

#[test]
fn an_action_of_a_group_no_client_knows_is_one_act() {
    let d = FakeDaemon::start("beacon");
    let from = d.op_count(B);
    let (code, _, _) = d.cli(&["do", "beacon.flash", "times=2", "--device", "fake-b"]);
    assert_eq!(code, 0);
    let mut params = serde_json::Map::new();
    params.insert("times".into(), json!(2));
    assert_eq!(
        d.writes_since(B, from),
        [Op::Act("beacon.flash".into(), params)]
    );
}

/// Spec 6.2, "Preset names": no name is sent, `--json` shows the empty name the fake stores,
/// human output shows a placeholder.
#[test]
fn a_preset_saved_without_a_name_shows_a_placeholder_only_for_people() {
    let d = FakeDaemon::start("preset");
    let (code, _, _) = d.cli(&["do", "preset.save", "slot=1", "--device", "fake-a"]);
    assert_eq!(code, 0);
    let (_, out, _) = d.cli(&["--json", "get", "presets", "--device", "fake-a"]);
    let presets = parse(&out)["state"]["presets"].clone();
    assert!(
        presets
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p["slot"] == 1 && p["name"] == ""),
        "{presets}"
    );
    let (_, out, _) = d.cli(&["get", "presets", "--device", "fake-a"]);
    assert!(out.contains("name (unnamed)"), "{out}");
}

/// A writer the watch thread and the test share.
#[derive(Clone, Default)]
struct Shared(Arc<Mutex<Vec<u8>>>);

impl Write for Shared {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl Shared {
    fn text(&self) -> String {
        String::from_utf8(self.0.lock().unwrap().clone()).unwrap()
    }
}

/// `watch` prints the hello reply, then the events a `set` from another connection causes
/// (spec 4.1 rule 3: every subscribed connection), and exits 4 when the daemon stops.
#[test]
fn watch_sees_a_change_made_by_another_command() {
    let d = FakeDaemon::start("watch");
    let socket = d.socket.clone();
    let out = Shared::default();
    let mut writer = out.clone();
    let watch = std::thread::spawn(move || {
        let args: Vec<std::ffi::OsString> = ["obsbot4linux", "watch", "--live", "pose"]
            .iter()
            .map(Into::into)
            .collect();
        obsbot4linux::run(
            &args,
            Some(socket),
            false,
            &mut writer,
            &mut std::io::sink(),
        )
    });
    eventually("the hello reply", || out.text().lines().count() >= 1);
    let first = out.text().lines().next().unwrap().to_string();
    assert!(
        first.starts_with(r#"{"id":1,"ok":{"protocol":2"#),
        "{first}"
    );
    let (code, _, _) = d.cli(&["set", "beacon.color=blue", "--device", "fake-b"]);
    assert_eq!(code, 0);
    eventually("a state event of fake-b with blue", || {
        out.text().lines().skip(1).any(|l| {
            let v: Value = serde_json::from_str(l).unwrap();
            v["event"] == "state" && v["device"] == B && v["state"]["beacon"]["color"] == "blue"
        })
    });
    d.stop();
    assert_eq!(watch.join().unwrap(), 4);
}
