// SPDX-License-Identifier: GPL-3.0-or-later

//! Asleep, wake and retries (spec 5.7, 10.3 B): `asleep: ignored` refused, `asleep: unknown`
//! written and pending until the first read after wake, the wake read of `unread` groups, the
//! retry of a `touched` read that failed or could not run, and the settle bound.

mod common;

use common::*;
use obsbot_device::models::fake::Op;
use serde_json::json;
use std::time::Duration;

const T: Duration = Duration::from_secs(5);

fn asleep(v: bool) -> impl Fn(&serde_json::Value) -> bool {
    move |e| state_of(B)(e) && e["state"]["asleep"] == json!(v)
}

#[test]
fn asleep_ignored_is_refused_and_unknown_is_pending_until_wake() {
    let d = Daemon::start();
    let (mut c, _) = d.hello(true, &[]);
    d.control(&format!("{{\"sleep\":\"{B}\"}}"));
    c.event(T, asleep(true))
        .expect("the status read shows asleep");
    let skip = d.ops(B).len();
    assert_eq!(
        c.set(B, json!({"zoom.factor": 1.5}))["error"]["code"],
        "asleep"
    );
    let r = c.set(B, json!({"led.brightness": 4}));
    assert!(r["ok"].is_object(), "{r}");
    let ev = c
        .events
        .iter()
        .rev()
        .find(|e| state_of(B)(e))
        .unwrap()
        .clone();
    assert_eq!(ev["state"]["pending"], json!({"led.brightness": 4}));
    assert!(
        !d.ops(B)[skip..].contains(&Op::Read("led".into())),
        "no read of a group the camera cannot answer asleep"
    );
    std::thread::sleep(Duration::from_millis(1500));
    let still = c.get(B, Some(&["led"]));
    assert_eq!(
        still["ok"]["state"]["pending"],
        json!({"led.brightness": 4}),
        "asleep, the bound of 1000 ms has not started"
    );
    d.control(&format!("{{\"wake\":\"{B}\"}}"));
    let ev = c
        .event(T, |e| state_of(B)(e) && e["state"]["pending"] == json!({}))
        .expect("the first read after wake shows it");
    assert_eq!(at(&ev["state"], "led.brightness"), &json!(4));
    assert_eq!(ev["state"]["asleep"], false);
}

/// fake-b stores `exposure.time` rounded down to 100: the readback never shows 1234, and the
/// settle bound (1000 ms) ends `pending` at a read of its group; the camera's value stands.
#[test]
fn the_settle_bound_ends_pending_and_the_camera_value_stands() {
    let d = Daemon::start();
    let (mut c, _) = d.hello(true, &[]);
    let r = c.set(B, json!({"exposure": {"mode": "manual", "time": 1234}}));
    assert!(r["ok"].is_object(), "{r}");
    let ev = c
        .events
        .iter()
        .rev()
        .find(|e| state_of(B)(e))
        .unwrap()
        .clone();
    assert_eq!(ev["state"]["pending"], json!({"exposure.time": 1234}));
    let t0 = std::time::Instant::now();
    let done = c
        .event(T, |e| state_of(B)(e) && e["state"]["pending"] == json!({}))
        .expect("the bound ends it");
    assert!(
        t0.elapsed() >= Duration::from_millis(800),
        "not before the bound"
    );
    assert_eq!(at(&done["state"], "exposure.time"), &json!(1200));
}

/// 5.7: a `touched` group whose re-read could not run while asleep is read again at the
/// wake, with the other groups that have a leaf in `unread`.
#[test]
fn the_wake_read_reads_the_touched_group_the_camera_could_not_answer() {
    let d = Daemon::start();
    let (mut c, _) = d.hello(true, &[]);
    assert!(c.act(B, "preset.save", json!({"slot": 0, "name": "Desk"}))["ok"].is_object());
    d.control(&format!("{{\"sleep\":\"{B}\"}}"));
    c.event(T, asleep(true)).unwrap();
    let r = c.act(B, "preset.delete", json!({"slot": 0}));
    assert!(r["ok"].is_object(), "asleep: unknown is sent: {r}");
    let st = c.get(B, None);
    let unread = st["ok"]["state"]["unread"].clone();
    assert!(
        unread.as_array().unwrap().contains(&json!("presets")),
        "{unread}"
    );
    assert_eq!(
        c.act(B, "preset.delete", json!({"slot": 0}))["error"]["code"],
        "conflict",
        "a slot action while its list is unread"
    );
    d.control(&format!("{{\"wake\":\"{B}\"}}"));
    let ev = c
        .event(T, |e| state_of(B)(e) && e["state"]["unread"] == json!([]))
        .expect("the wake read");
    assert_eq!(ev["state"]["presets"], json!([]), "the delete shows");
}

/// 5.7: a `touched` read that failed is read again with the next tick of the 60 s tier
/// (3 s under the test timing). `fail_next` is set once the re-read has started; the fake
/// checks it after its delay.
#[test]
fn a_failed_touched_read_is_read_again_with_the_minute_tier() {
    let d = Daemon::start();
    let (mut c, _) = d.hello(true, &[]);
    d.bus.set_delay("fake-b", Duration::from_millis(300));
    c.send(&json!({"id": 1, "device": B, "do": "preset.save", "with": {"slot": 1}}));
    let act = d.ops(B).len();
    assert!(eventually(T, || d.ops(B)[act..].contains(&Op::Read("presets".into()))));
    d.bus.fail_next(B).unwrap();
    assert!(c.reply(1)["ok"].is_object(), "the action was accepted");
    d.bus.set_delay("fake-b", Duration::ZERO);
    let st = c.get(B, Some(&["presets"]));
    assert!(
        st["ok"]["state"]["unread"]
            .as_array()
            .unwrap()
            .contains(&json!("presets"))
    );
    let ev = c
        .event(Duration::from_secs(8), |e| {
            state_of(B)(e)
                && !e["state"]["unread"]
                    .as_array()
                    .unwrap()
                    .contains(&json!("presets"))
        })
        .expect("read again");
    assert_eq!(ev["state"]["presets"][0]["slot"], 1);
}

/// 5.6 "Restart": a write with `warning: restart` is answered once the camera accepted it,
/// without a re-read; its leaf is pending; nothing else is sent until the device goes (the
/// request pipelined behind it is answered as for an unavailable device), which empties
/// `pending`; the device comes back as ordinary hotplug.
#[test]
fn a_restart_write_is_answered_on_acceptance_and_then_nothing_is_sent() {
    let d = Daemon::start();
    let (mut c, _) = d.hello(true, &[]);
    let skip = d.ops(A).len();
    let mut lines = Vec::new();
    for req in [
        json!({"id": 1, "device": A, "set": {"audio.enabled": true}}),
        json!({"id": 2, "device": A, "set": {"led.brightness": 1}}),
    ] {
        lines.extend_from_slice(req.to_string().as_bytes());
        lines.push(b'\n');
    }
    c.send_raw(&lines);
    let r = c.reply(1);
    assert!(r["ok"]["rev"].is_u64(), "{r}");
    let ev = c
        .events
        .iter()
        .rev()
        .find(|e| state_of(A)(e))
        .unwrap()
        .clone();
    assert_eq!(ev["state"]["pending"], json!({"audio.enabled": true}));
    assert_eq!(
        c.reply(2)["error"]["code"],
        "unavailable",
        "not sent after the restart write; answered when the device went"
    );
    let gone = c
        .event(T, |e| state_of(A)(e) && e["state"]["available"] == false)
        .unwrap();
    assert_eq!(gone["state"]["pending"], json!({}));
    c.event(T, |e| state_of(A)(e) && e["state"]["available"] == true)
        .expect("back");
    let after: Vec<Op> = d.ops(A)[skip..].to_vec();
    let write = Op::Apply("audio.enabled".into(), json!(true));
    let sent = after.iter().position(|op| *op == write).expect("sent");
    let open = after
        .iter()
        .position(|op| *op == Op::Open)
        .expect("re-opened");
    assert!(sent < open, "{after:?}");
    assert!(
        after[sent + 1..open].is_empty(),
        "nothing after the restart write until the device went: {after:?}"
    );
}
