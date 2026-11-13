// SPDX-License-Identifier: GPL-3.0-or-later

//! B-1: the smallest daemon tracks C and D run their live tests against (spec 10.3): the
//! fake devices over the socket, the five verbs, the validator's errors, the write path with
//! its touched re-read, `pending` and `rev`, events before replies, periodic reads, the
//! test-control socket and the `--fake` binary.

mod common;

use common::*;
use obsbot_device::models::fake::Op;
use serde_json::{Value, json};
use std::time::Duration;

const T: Duration = Duration::from_secs(5);

#[test]
fn hello_lists_the_three_fakes_with_their_states() {
    let d = Daemon::start();
    let (_c, ok) = d.hello(false, &[]);
    assert_eq!(ok["protocol"], 2);
    assert_eq!(ids(&ok["devices"]), [A, B, UNKNOWN]);
    let support: Vec<&Value> = ok["devices"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| &d["support"])
        .collect();
    assert_eq!(
        support,
        [&json!("tested"), &json!("untested"), &json!("unsupported")]
    );
    assert_eq!(ok["devices"][0]["name"], "Fake A 0001");
    assert_eq!(ok["devices"][0]["firmware"], "1.0.0.0");
    let a = &ok["states"][A];
    assert_eq!(a["rev"], 1);
    assert_eq!(
        at(&a["state"], "led.brightness"),
        &json!(0),
        "read at plug-in"
    );
    assert_eq!(a["state"]["unread"], json!([]));
    assert_eq!(
        ok["states"][UNKNOWN],
        json!({"rev": 1, "state": {"available": true, "pending": {}, "unread": []}})
    );
}

/// 4.1 rule 3: the state event reaches every subscribed connection, the requester included,
/// before the reply; the reply's rev is that state's rev.
#[test]
fn a_set_publishes_its_state_before_the_reply() {
    let d = Daemon::start();
    let (mut c, _) = d.hello(true, &[]);
    let (mut other, _) = d.hello(true, &[]);
    let r = c.set(A, json!({"led.brightness": 2}));
    let rev = r["ok"]["rev"].clone();
    let ev = c
        .events
        .iter()
        .find(|e| state_of(A)(e))
        .expect("the event came first")
        .clone();
    assert_eq!(ev["rev"], rev);
    assert_eq!(at(&ev["state"], "led.brightness"), &json!(2));
    let seen = other.event(T, |e| state_of(A)(e) && e["rev"] == rev);
    assert!(seen.is_some(), "the other subscriber gets it too");
    let got = c.get(A, Some(&["led"]));
    assert_eq!(got["ok"]["rev"], rev);
    assert_eq!(at(&got["ok"]["state"], "led.brightness"), &json!(2));
    assert!(
        got["ok"]["state"].get("zoom").is_none(),
        "only the paths asked for"
    );
}

/// 4.1 rules 3 and 4 with fake-b's lagging readback: the reply's state lists the leaf in
/// `pending`; a later read shows it and it leaves `pending`.
#[test]
fn a_lagging_readback_is_pending_until_a_read_shows_it() {
    let d = Daemon::start();
    let (mut c, _) = d.hello(true, &[]);
    let r = c.set(B, json!({"led": {"brightness": 3}}));
    let rev = r["ok"]["rev"].as_u64().unwrap();
    let ev = c
        .events
        .iter()
        .find(|e| state_of(B)(e) && e["rev"] == json!(rev))
        .expect("the state of the reply's rev")
        .clone();
    assert_eq!(ev["state"]["pending"], json!({"led.brightness": 3}));
    assert_eq!(
        at(&ev["state"], "led.brightness"),
        &json!(0),
        "the camera's value"
    );
    let cleared = c
        .event(T, |e| state_of(B)(e) && e["state"]["pending"] == json!({}))
        .expect("pending clears");
    assert_eq!(at(&cleared["state"], "led.brightness"), &json!(3));
    assert!(cleared["rev"].as_u64().unwrap() > rev);
}

/// 4.1 rule 6: a motion action replies when the camera accepted it; fake-a's `gimbal.move`
/// touches no group, so the pose arrives with a later read.
#[test]
fn an_action_replies_on_acceptance_and_its_result_is_returned() {
    let d = Daemon::start();
    let (mut c, _) = d.hello(true, &["pose"]);
    let r = c.act(A, "gimbal.move", json!({"pan": 20}));
    assert!(r["ok"]["rev"].is_u64(), "{r}");
    assert!(
        c.event(T, |e| state_of(A)(e)
            && at(&e["state"], "gimbal.pan") == &json!(20))
            .is_some()
    );
    let v = c.act(A, "view.save", json!({"name": "Desk"}));
    assert_eq!(v["ok"]["result"], json!({"id": 0}));
    let p = c.act(A, "preset.save", json!({"slot": 0}));
    assert!(p["ok"].is_object());
    let got = c.get(A, Some(&["presets"]));
    assert_eq!(
        got["ok"]["state"]["presets"][0]["name"], "",
        "no name is invented (spec 2)"
    );
}

/// The validator's codes reach the client (4.1 Errors), and nothing reaches the camera.
#[test]
fn refused_requests_send_nothing() {
    let d = Daemon::start();
    let (mut c, _) = d.hello(false, &[]);
    let before = d.ops(A).len();
    let code = |r: Value| r["error"]["code"].as_str().unwrap_or("ok").to_string();
    assert_eq!(code(c.set(A, json!({"led.brightness": "high"}))), "invalid");
    assert_eq!(code(c.set(A, json!({"led.brightnes": 1}))), "unsupported");
    assert_eq!(code(c.set(A, json!({"led.brightness": 4}))), "out_of_range");
    assert_eq!(
        code(c.set(A, json!({"exposure.time": 20000}))),
        "conflict",
        "requires exposure.mode=manual"
    );
    assert_eq!(
        code(c.act(A, "gimbal.move", json!({}))),
        "invalid",
        "the model's check"
    );
    assert_eq!(
        code(c.act(A, "preset.recall", json!({"slot": 2}))),
        "conflict",
        "empty slot"
    );
    assert_eq!(
        code(c.act(A, "factory_reset", json!({}))),
        "invalid",
        "no confirm"
    );
    assert_eq!(code(c.get(A, Some(&["zone.area.pan.min"]))), "unsupported");
    assert_eq!(
        code(c.set("sn:NOPE00000000", json!({"led.brightness": 1}))),
        "unknown_device"
    );
    assert_eq!(writes_after(&d, A, before), Vec::<Op>::new());
}

/// 5.4 and walk-through 2 (8.4): the unsupported device is listed, answered at once, and
/// receives nothing at all.
#[test]
fn the_unknown_device_is_answered_at_once_and_receives_nothing() {
    let d = Daemon::start();
    let (mut c, _) = d.hello(false, &[]);
    let got = c.get(UNKNOWN, None);
    assert_eq!(
        got["ok"],
        json!({"rev": 1, "state": {"available": true, "pending": {}, "unread": []}})
    );
    assert_eq!(
        c.set(UNKNOWN, json!({"led.brightness": 1}))["error"]["code"],
        "unsupported_device"
    );
    assert_eq!(
        c.act(UNKNOWN, "sleep", json!({}))["error"]["code"],
        "unsupported_device"
    );
    assert_eq!(d.ops(UNKNOWN), Vec::<Op>::new());
}

/// 4.1: the first message must be `hello`; a second `hello` is `invalid`; `live` replies `{}`.
#[test]
fn the_hello_rules_of_4_1() {
    let d = Daemon::start();
    let mut first = d.connect();
    first.send(&json!({"id": 1, "device": A, "get": null}));
    let r = first.line(T).unwrap();
    assert_eq!(r["error"]["code"], "protocol");
    assert_eq!(r["error"]["supported"], json!([2]));
    assert_eq!(r["id"], 1);
    assert!(first.closed_within(T), "the connection is closed");

    let mut old = d.connect();
    old.send(&json!({"id": 1, "hello": {"protocol": 3, "client": "future 9"}}));
    assert_eq!(old.line(T).unwrap()["error"]["code"], "protocol");
    assert!(old.closed_within(T));

    let mut junk = d.connect();
    junk.send_raw(b"not json\n");
    let r = junk.line(T).unwrap();
    assert_eq!(
        (r["id"].clone(), r["error"]["code"].clone()),
        (json!(null), json!("protocol"))
    );

    let (mut c, _) = d.hello(false, &[]);
    c.send(&json!({"id": 7, "hello": {"protocol": 2, "client": "again"}}));
    assert_eq!(c.reply(7)["error"]["code"], "invalid");
    c.send(&json!({"id": 8, "live": ["pose"]}));
    assert_eq!(c.reply(8), json!({"id": 8, "ok": {}}));
    c.send_raw(b"{\"id\":9}\n");
    assert_eq!(c.reply(9)["error"]["code"], "invalid", "no verb");
}

/// 5.12: the test-control socket plugs and unplugs; 4.1 Events: an unplugged sn: entry stays
/// listed unavailable, a port: entry is removed, a re-plug is devices then state.
#[test]
fn the_test_control_socket_drives_hotplug() {
    let d = Daemon::start();
    let (mut c, _) = d.hello(true, &[]);
    assert_eq!(
        d.control(&format!("{{\"unplug\":\"{UNKNOWN}\"}}")),
        json!({"ok": true})
    );
    let ev = c.event(T, devices).expect("devices");
    assert_eq!(ids(&ev["devices"]), [A, B]);
    assert_eq!(d.control("{\"plug\":\"unknown\"}"), json!({"ok": UNKNOWN}));
    let ev = c.event(T, devices).unwrap();
    assert_eq!(ids(&ev["devices"]), [A, B, UNKNOWN]);
    let st = c.event(T, state_of(UNKNOWN)).unwrap();
    assert_eq!(st["rev"], 1, "a re-added port: entry starts at rev 1");

    d.control(&format!("{{\"unplug\":\"{B}\"}}"));
    let ev = c.event(T, devices).unwrap();
    assert_eq!(ids(&ev["devices"]), [A, &format!("{B}!"), UNKNOWN]);
    let st = c.event(T, state_of(B)).unwrap();
    assert_eq!(st["state"]["available"], false);
    assert_eq!(
        c.set(B, json!({"led.brightness": 1}))["error"]["code"],
        "unavailable"
    );
    assert_eq!(
        c.get(B, None)["ok"]["state"]["available"],
        false,
        "get from the last state"
    );
    d.control("{\"plug\":\"fake-b\"}");
    let ev = c.event(T, devices).unwrap();
    assert_eq!(ids(&ev["devices"]), [A, B, UNKNOWN]);
    let st = c.event(T, state_of(B)).unwrap();
    assert_eq!(st["state"]["available"], true);
    assert!(c.set(B, json!({"led.brightness": 1}))["ok"].is_object());
}

/// 5.12 and 10.3: `obsbot4linuxd --fake fake-a,fake-b,unknown` serves the fakes on the
/// socket that `OBSBOT4LINUX_SOCKET` names, with the test-control socket beside it, and
/// exits 0 on SIGTERM (5.9).
#[test]
fn the_binary_serves_fake_mode_and_ends_on_sigterm() {
    let dir = temp_dir();
    let socket = dir.join("s.sock");
    let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_obsbot4linuxd"))
        .args(["--fake", "fake-a,fake-b,unknown"])
        .env("OBSBOT4LINUX_SOCKET", &socket)
        .env_remove("LISTEN_PID")
        .env_remove("LISTEN_FDS")
        .spawn()
        .unwrap();
    assert!(eventually(T, || std::os::unix::net::UnixStream::connect(
        &socket
    )
    .is_ok()));
    let mut c = Client::connect(&socket);
    let ok = c.hello(false, &[]);
    assert_eq!(ids(&ok["devices"]), [A, B, UNKNOWN]);
    let mut test = socket.as_os_str().to_owned();
    test.push(".test");
    assert!(std::path::Path::new(&test).exists());
    nix::sys::signal::kill(
        nix::unistd::Pid::from_raw(child.id() as i32),
        nix::sys::signal::Signal::SIGTERM,
    )
    .unwrap();
    let status = child.wait().unwrap();
    assert_eq!(status.code(), Some(0));
    assert!(!socket.exists(), "a socket it bound is removed at exit");
    let _ = std::fs::remove_dir_all(dir);
}
