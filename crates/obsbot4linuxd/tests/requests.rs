// SPDX-License-Identifier: GPL-3.0-or-later

//! The write path and the queue rules (spec 10.3 B): rev order and event-before-reply,
//! pipelined requests, shared re-reads, single-leaf coalescing, whole-patch validation, the
//! apply order, `applied`/`failed`, `requires`, live-topic filtering, and a request sending
//! exactly what it names (section 2).

mod common;

use common::*;
use obsbot_device::models::fake::Op;
use serde_json::{Value, json};
use std::time::Duration;

const T: Duration = Duration::from_secs(5);

fn reads_of(d: &Daemon, id: &str, group: &str, skip: usize) -> usize {
    d.ops(id)[skip..]
        .iter()
        .filter(|op| **op == Op::Read(group.into()))
        .count()
}

/// 4.1 rule 2: requests may be pipelined; one device's requests are answered in order.
#[test]
fn pipelined_requests_are_answered_in_request_order() {
    let d = Daemon::start();
    let (mut c, _) = d.hello(false, &[]);
    for (i, v) in [1, 2, 3, 0].iter().enumerate() {
        c.send(&json!({"id": i, "device": A, "set": {"led.brightness": v}}));
    }
    c.send(&json!({"id": 4, "device": A, "get": ["led"]}));
    let mut order = Vec::new();
    for _ in 0..5 {
        let r = c.line(T).expect("a reply");
        order.push(r["id"].as_i64().unwrap());
        if r["id"] == 4 {
            assert_eq!(
                at(&r["ok"]["state"], "led.brightness"),
                &json!(0),
                "the get sees the batch"
            );
        }
    }
    assert_eq!(order, [0, 1, 2, 3, 4]);
}

/// 4.1 rule 9 and 10.3: N pipelined sets on one connection cause at most two re-reads
/// (fake-b's `beacon` group is read only every 60 s, so every read of it here is a re-read).
#[test]
fn pipelined_sets_share_at_most_two_rereads() {
    let d = Daemon::start();
    let (mut c, _) = d.hello(false, &[]);
    let skip = d.ops(B).len();
    let colors = [
        "green", "blue", "red", "green", "blue", "red", "green", "blue",
    ];
    for (i, color) in colors.iter().enumerate() {
        c.send(&json!({"id": i, "device": B, "set": {"beacon.color": color}}));
    }
    for i in 0..colors.len() {
        assert!(c.reply(i as i64)["ok"].is_object());
    }
    let rereads = reads_of(&d, B, "beacon", skip);
    assert!((1..=2).contains(&rereads), "{rereads} re-reads of beacon");
}

/// 4.1 rule 9: a single-leaf `set` is answered `ok` without being sent when a later queued
/// `set` of the same connection has the same leaf, dotted or nested.
#[test]
fn a_single_leaf_set_is_coalesced_with_a_later_one() {
    let d = Daemon::start();
    let (mut c, _) = d.hello(false, &[]);
    d.bus.set_delay("fake-b", Duration::from_millis(300));
    c.send(&json!({"id": 1, "device": B, "set": {"tracking.mode": "person"}}));
    let started = Op::Apply("tracking.mode".into(), json!("person"));
    assert!(
        eventually(T, || d.ops(B).contains(&started)),
        "request 1 runs"
    );
    let skip = d.ops(B).len();
    c.send(&json!({"id": 2, "device": B, "set": {"beacon.color": "green"}}));
    c.send(&json!({"id": 3, "device": B, "set": {"beacon": {"color": "blue"}}}));
    c.send(&json!({"id": 4, "device": B, "set": {"beacon.color": "nope"}}));
    for id in 1..=4 {
        let r = c.reply(id);
        if id == 4 {
            assert_eq!(r["error"]["code"], "out_of_range");
        } else {
            assert!(r["ok"]["rev"].is_u64(), "{r}");
        }
    }
    d.bus.set_delay("fake-b", Duration::ZERO);
    let applied: Vec<Op> = writes_after(&d, B, skip);
    assert_eq!(
        applied,
        [Op::Apply("beacon.color".into(), json!("blue"))],
        "green is never sent; the refused value does not replace blue"
    );
}

/// 4.1 rule 5: the whole patch is validated first; one bad leaf sends nothing.
#[test]
fn one_bad_leaf_refuses_the_whole_patch() {
    let d = Daemon::start();
    let (mut c, _) = d.hello(false, &[]);
    let skip = d.ops(A).len();
    let r = c.set(A, json!({"led.brightness": 1, "zoom.factor": 9}));
    assert_eq!(r["error"]["code"], "out_of_range");
    assert_eq!(writes_after(&d, A, skip), Vec::<Op>::new());
}

/// 4.2 and 8.8: `requires` is checked against the state with the patch applied, and the
/// model's apply order puts the required path first.
#[test]
fn requires_and_the_apply_order() {
    let d = Daemon::start();
    let (mut c, _) = d.hello(false, &[]);
    assert_eq!(
        c.set(A, json!({"exposure.time": 20000}))["error"]["code"],
        "conflict"
    );
    let skip = d.ops(A).len();
    let r = c.set(A, json!({"exposure": {"time": 20000, "mode": "manual"}}));
    assert!(r["ok"].is_object(), "{r}");
    assert_eq!(
        writes_after(&d, A, skip),
        [
            Op::Apply("exposure.mode".into(), json!("manual")),
            Op::Apply("exposure.time".into(), json!(20000))
        ]
    );
}

/// The current state inside a batch includes the earlier requests' written leaves, so a
/// pipelined `exposure.mode=manual` lets the next request write `exposure.time`.
#[test]
fn a_pipelined_requirement_counts_within_one_batch() {
    let d = Daemon::start();
    let (mut c, _) = d.hello(false, &[]);
    d.bus.set_delay("fake-a", Duration::from_millis(200));
    c.send(&json!({"id": 1, "device": A, "set": {"led.brightness": 1}}));
    let started = Op::Apply("led.brightness".into(), json!(1));
    assert!(
        eventually(T, || d.ops(A).contains(&started)),
        "request 1 runs"
    );
    c.send(&json!({"id": 2, "device": A, "set": {"exposure.mode": "manual"}}));
    c.send(&json!({"id": 3, "device": A, "set": {"exposure.time": 20000}}));
    for id in 1..=3 {
        assert!(c.reply(id)["ok"].is_object(), "request {id}");
    }
}

/// 4.1 rule 5: a camera failure midway stops the patch, re-reads, and answers `camera` with
/// `applied`, `failed` and the rev of the re-read state. `fail_next` hits the second apply:
/// it is set once that apply has started and waits out the fake's delay (the fake checks
/// `fail_next` after its delay).
#[test]
fn a_failure_midway_reports_applied_and_failed() {
    let d = Daemon::start();
    let (mut c, _) = d.hello(true, &[]);
    d.bus.set_delay("fake-a", Duration::from_millis(400));
    c.send(
        &json!({"id": 1, "device": A, "set": {"exposure.mode": "manual", "exposure.time": 20000}}),
    );
    let second = Op::Apply("exposure.time".into(), json!(20000));
    assert!(
        eventually(T, || d.ops(A).contains(&second)),
        "the second apply started"
    );
    d.bus.fail_next(A).unwrap();
    let r = c.reply(1);
    d.bus.set_delay("fake-a", Duration::ZERO);
    assert_eq!(r["error"]["code"], "camera", "{r}");
    assert_eq!(r["error"]["applied"], json!(["exposure.mode"]));
    assert_eq!(r["error"]["failed"], "exposure.time");
    assert!(r["error"]["rev"].is_u64(), "re-read before answering");
}

/// Section 2 and 10.3: a `set` causes at most one `apply` per leaf of its patch and no `act`,
/// a `do` exactly one `act` and no `apply`; apart from reads nothing else reaches the fake.
#[test]
fn a_request_sends_exactly_what_it_names() {
    let d = Daemon::start();
    let (mut c, _) = d.hello(false, &[]);
    c.set(A, json!({"tracking.mode": "normal"}));
    let cases: Vec<(Value, Vec<Op>)> = vec![
        (
            json!({"device": A, "set": {"led.brightness": 2, "gimbal.invert": true}}),
            vec![
                Op::Apply("gimbal.invert".into(), json!(true)),
                Op::Apply("led.brightness".into(), json!(2)),
            ],
        ),
        (
            json!({"device": A, "do": "gimbal.move", "with": {"pan": 20}}),
            vec![Op::Act(
                "gimbal.move".into(),
                json!({"pan": 20}).as_object().unwrap().clone(),
            )],
        ),
        (
            json!({"device": A, "do": "gimbal.turn", "with": {"pan_speed": 10, "keep": true}}),
            vec![Op::Act(
                "gimbal.turn".into(),
                json!({"pan_speed": 10, "keep": true})
                    .as_object()
                    .unwrap()
                    .clone(),
            )],
        ),
        (
            json!({"device": A, "do": "preset.save", "with": {"slot": 0}}),
            vec![Op::Act(
                "preset.save".into(),
                json!({"slot": 0}).as_object().unwrap().clone(),
            )],
        ),
    ];
    for (mut req, want) in cases {
        let skip = d.ops(A).len();
        let id = c.id();
        req["id"] = json!(id);
        c.send(&req);
        assert!(c.reply(id)["ok"].is_object());
        let mut got = writes_after(&d, A, skip);
        if let Op::Apply(..) = want[0] {
            got.sort_by_key(|op| format!("{op:?}"));
        }
        assert_eq!(got, want, "{req}");
    }
    let state = c.get(A, Some(&["tracking.mode"]));
    assert_eq!(
        at(&state["ok"]["state"], "tracking.mode"),
        &json!("normal"),
        "left alone"
    );
}

/// 4.1 rule 8: a change of volatile leaves that no request caused goes only to connections
/// subscribing its topic. fake-a's `gimbal.move` touches no group: the pose change is found
/// by a periodic read.
#[test]
fn a_volatile_change_reaches_only_its_live_subscribers() {
    let d = Daemon::start();
    let (mut pose, _) = d.hello(true, &["pose"]);
    let (mut plain, _) = d.hello(true, &[]);
    let (mut c, _) = d.hello(false, &[]);
    c.act(A, "gimbal.move", json!({"pan": 30}));
    assert!(
        pose.event(T, |e| state_of(A)(e)
            && at(&e["state"], "gimbal.pan") == &json!(30))
            .is_some()
    );
    assert!(
        plain
            .event(Duration::from_millis(800), state_of(A))
            .is_none(),
        "no event for a connection without the topic"
    );
    c.set(A, json!({"led.brightness": 3}));
    let ev = plain
        .event(T, state_of(A))
        .expect("a non-volatile change reaches everyone");
    assert_eq!(
        at(&ev["state"], "gimbal.pan"),
        &json!(30),
        "with the new pose in it"
    );
}

/// Section 2 and review focus 1: a request sends the value it names. A client's float noise
/// on a stepped leaf (a QML slider, JavaScript arithmetic) is put on its step; a parameter
/// without a step is not rounded to its precision (fake-a's `pan`: precision 0.1).
#[test]
fn a_value_is_sent_as_the_request_names_it() {
    let d = Daemon::start();
    let (mut c, _) = d.hello(false, &[]);
    let skip = d.ops(A).len();
    assert!(c.set(A, json!({"led.brightness": 2.9999999999999996}))["ok"].is_object());
    assert!(c.act(A, "gimbal.move", json!({"pan": 10.04}))["ok"].is_object());
    assert_eq!(
        writes_after(&d, A, skip),
        [
            Op::Apply("led.brightness".into(), json!(3)),
            Op::Act(
                "gimbal.move".into(),
                json!({"pan": 10.04}).as_object().unwrap().clone()
            ),
        ]
    );
    let got = c.get(A, Some(&["led"]));
    assert_eq!(
        got["ok"]["state"]["pending"],
        json!({}),
        "the readback 3 shows the write at once"
    );
}
