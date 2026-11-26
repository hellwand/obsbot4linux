// SPDX-License-Identifier: GPL-3.0-or-later

//! The queue's limits (spec 4.1 rule 9, 5.6, 10.3 B): the 15 s deadline, `busy` at 32,
//! requests of a closed connection dropped, and the queue of a device that goes.

mod common;

use common::*;
use obsbot_device::models::fake::Op;
use serde_json::json;
use std::time::{Duration, Instant};

const T: Duration = Duration::from_secs(5);

/// 4.1 rule 9: every device request is answered within 15 s of its arrival, started or not.
#[test]
fn every_request_is_answered_within_the_deadline() {
    let d = Daemon::start();
    let (mut c, _) = d.hello(false, &[]);
    d.bus.set_delay("fake-a", Duration::from_secs(20));
    let t0 = Instant::now();
    c.send(&json!({"id": 1, "device": A, "set": {"led.brightness": 1}}));
    c.send(&json!({"id": 2, "device": A, "get": ["led"]}));
    for id in [1, 2] {
        let r = c.reply(id);
        assert_eq!(r["error"]["code"], "camera", "{r}");
    }
    let took = t0.elapsed();
    assert!(took >= Duration::from_secs(14), "{took:?}");
    assert!(took < Duration::from_millis(15_500), "{took:?}");
}

/// 4.1 `busy`: 32 requests are already queued for the device; answered at once.
#[test]
fn the_request_after_32_queued_ones_is_busy() {
    let d = Daemon::start();
    let (mut c, _) = d.hello(false, &[]);
    d.bus.set_delay("fake-a", Duration::from_millis(1500));
    c.send(&json!({"id": 0, "device": A, "set": {"led.brightness": 1}}));
    let started = Op::Apply("led.brightness".into(), json!(1));
    assert!(eventually(T, || d.ops(A).contains(&started)));
    for id in 1..=32 {
        c.send(&json!({"id": id, "device": A, "get": ["led"]}));
    }
    let t0 = Instant::now();
    c.send(&json!({"id": 33, "device": A, "get": ["led"]}));
    let first = c.line(T).expect("a reply");
    assert_eq!(first["id"], 33, "answered before the queue moves: {first}");
    assert_eq!(first["error"]["code"], "busy");
    assert!(t0.elapsed() < Duration::from_millis(500));
    d.bus.set_delay("fake-a", Duration::ZERO);
    for id in 0..=32 {
        assert!(c.reply(id)["ok"].is_object(), "{id}");
    }
}

/// 4.1 rule 9: a request whose connection closed before the worker started it is dropped.
#[test]
fn queued_requests_of_a_closed_connection_are_never_sent() {
    let d = Daemon::start();
    let (mut c, _) = d.hello(false, &[]);
    d.bus.set_delay("fake-a", Duration::from_millis(500));
    c.send(&json!({"id": 1, "device": A, "set": {"led.brightness": 1}}));
    let started = Op::Apply("led.brightness".into(), json!(1));
    assert!(eventually(T, || d.ops(A).contains(&started)));
    c.send(&json!({"id": 2, "device": A, "set": {"gimbal.invert": true}}));
    c.send(&json!({"id": 3, "device": A, "do": "gimbal.move", "with": {"pan": 10}}));
    std::thread::sleep(Duration::from_millis(50));
    c.close();
    std::thread::sleep(Duration::from_millis(1500));
    d.bus.set_delay("fake-a", Duration::ZERO);
    let ops = d.ops(A);
    assert!(ops.contains(&started), "a started request runs to the end");
    assert!(
        !ops.iter()
            .any(|op| matches!(op, Op::Apply(l, _) if l == "gimbal.invert"))
    );
    assert!(
        !ops.iter()
            .any(|op| matches!(op, Op::Act(a, _) if a == "gimbal.move"))
    );
}

/// 4.1: a client may shut down only its sending side after its last request and still read.
/// That is not a close: hello and the requests sent with it in one write, and requests still
/// queued behind a slow one, are all handled and answered; then the daemon closes. A daemon
/// that took the end of the stream for a close dropped them and sent only the hello reply
/// (measured 2026-11-26, live test G).
#[test]
fn a_half_closed_connection_still_gets_every_reply() {
    let d = Daemon::start();
    let mut c = d.connect();
    d.bus.set_delay("fake-a", Duration::from_millis(300));
    let mut lines = Vec::new();
    for req in [
        json!({"id": 1, "hello": {"protocol": 2, "client": "test 1.0", "subscribe": false}}),
        json!({"id": 2, "device": A, "set": {"led.brightness": 1}}),
        json!({"id": 3, "device": A, "set": {"gimbal.invert": true}}),
        json!({"id": 4, "device": A, "get": ["led", "gimbal.invert"]}),
    ] {
        lines.extend_from_slice(req.to_string().as_bytes());
        lines.push(b'\n');
    }
    c.send_raw(&lines);
    c.shut_write();
    assert!(c.reply(1)["ok"]["devices"].is_array());
    assert!(
        c.reply(2)["ok"]["rev"].is_u64(),
        "queued behind the slow apply"
    );
    assert!(c.reply(3)["ok"]["rev"].is_u64());
    let got = c.reply(4);
    assert_eq!(
        at(&got["ok"]["state"], "led.brightness"),
        &json!(1),
        "{got}"
    );
    assert_eq!(
        at(&got["ok"]["state"], "gimbal.invert"),
        &json!(true),
        "{got}"
    );
    assert!(c.closed_within(T), "closed once every request is answered");
    let ops = d.ops(A);
    assert!(ops.contains(&Op::Apply("gimbal.invert".into(), json!(true))));
}

/// 4.1 rule 9 for requests already taken into a batch: the three writes arrive in one write
/// while a slow poll runs, so the worker takes them as one batch; the connection closes during
/// the first apply, and the other two are never sent.
#[test]
fn a_batch_skips_the_requests_of_a_connection_that_closed() {
    let d = Daemon::start();
    let (mut c, _) = d.hello(false, &[]);
    d.bus.set_delay("fake-a", Duration::from_millis(400));
    let skip = d.ops(A).len();
    assert!(
        eventually(T, || d.ops(A)[skip..]
            .iter()
            .any(|op| matches!(op, Op::Read(_)))),
        "a slow poll is in flight"
    );
    let mut lines = Vec::new();
    for req in [
        json!({"id": 1, "device": A, "set": {"gimbal.invert": true}}),
        json!({"id": 2, "device": A, "do": "gimbal.move", "with": {"pan": 10}}),
        json!({"id": 3, "device": A, "set": {"led.brightness": 2}}),
    ] {
        lines.extend_from_slice(req.to_string().as_bytes());
        lines.push(b'\n');
    }
    c.send_raw(&lines);
    let first = Op::Apply("gimbal.invert".into(), json!(true));
    assert!(
        eventually(T, || d.ops(A)[skip..].contains(&first)),
        "request 1 runs"
    );
    c.close();
    std::thread::sleep(Duration::from_millis(1500));
    d.bus.set_delay("fake-a", Duration::ZERO);
    assert_eq!(writes_after(&d, A, skip), [first]);
}

/// 5.6 "Unavailable": the started request is answered `camera` without `rev`, the queued
/// ones as for an unavailable device; a re-plug starts a new worker with an empty queue.
#[test]
fn the_queue_of_a_device_that_goes_is_answered_and_a_replug_starts_afresh() {
    let d = Daemon::start();
    let (mut c, _) = d.hello(false, &[]);
    d.bus.set_delay("fake-a", Duration::from_secs(2));
    c.send(&json!({"id": 1, "device": A, "set": {"led.brightness": 1}}));
    let started = Op::Apply("led.brightness".into(), json!(1));
    assert!(eventually(T, || d.ops(A).contains(&started)));
    c.send(&json!({"id": 2, "device": A, "set": {"led.brightness": 2}}));
    c.send(&json!({"id": 3, "device": A, "get": ["led"]}));
    let t0 = Instant::now();
    d.control(&format!("{{\"unplug\":\"{A}\"}}"));
    let first = c.reply(1);
    assert_eq!(first["error"]["code"], "camera");
    assert!(
        first["error"].get("rev").is_none(),
        "nothing was re-read: {first}"
    );
    assert_eq!(first["error"]["failed"], "led.brightness");
    assert_eq!(c.reply(2)["error"]["code"], "unavailable");
    assert_eq!(c.reply(3)["ok"]["state"]["available"], false);
    assert!(
        t0.elapsed() < Duration::from_millis(1500),
        "not after the fake's delay"
    );
    d.bus.set_delay("fake-a", Duration::ZERO);
    d.control("{\"plug\":\"fake-a\"}");
    assert!(eventually(T, || {
        let mut probe = d.connect();
        probe.hello(false, &[]);
        probe.set(A, json!({"led.brightness": 3}))["ok"].is_object()
    }));
}
