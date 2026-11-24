// SPDX-License-Identifier: GPL-3.0-or-later

//! Start-up (spec 4.1 `hello` timing, 5.2): `hello` waits for the first enumeration and the
//! first read attempts, at most 15 s; a device still reading is listed with its leaves `null`
//! and in `unread`; requests that follow `hello` are handled after its reply.

mod common;

use common::*;
use obsbot_device::models::fake::KEYS;
use serde_json::json;
use std::time::{Duration, Instant};

#[test]
fn hello_waits_for_the_first_read_attempts() {
    let d = Daemon::start_with(&KEYS, fast(), |bus| {
        bus.set_delay("fake-b", Duration::from_millis(150))
    });
    let t0 = Instant::now();
    let (_c, ok) = d.hello(false, &[]);
    assert!(
        t0.elapsed() >= Duration::from_millis(600),
        "fake-b's first read takes 1.2 s"
    );
    let b = &ok["states"][B]["state"];
    assert_eq!(at(b, "led.brightness"), &json!(0), "read, not null");
    assert_eq!(b["unread"], json!([]));
}

/// 4.1 `hello` timing: answered at the latest 15 s after it arrived; the slow device is listed
/// with its leaves null and in `unread`; a request pipelined behind the `hello` is answered
/// after the reply.
#[test]
fn hello_is_answered_by_its_deadline_while_a_first_read_is_slow() {
    let d = Daemon::start_with(&KEYS, fast(), |bus| {
        bus.set_delay("fake-a", Duration::from_secs(20))
    });
    let mut c = d.connect();
    let t0 = Instant::now();
    c.send(&json!({"id": 1, "hello": {"protocol": 2, "client": "test 1.0", "subscribe": false}}));
    c.send(&json!({"id": 2, "device": B, "get": ["led"]}));
    let first = c.line(Duration::from_secs(20)).expect("the hello reply");
    let took = t0.elapsed();
    assert_eq!(first["id"], 1, "the reply comes first");
    assert!(
        took >= Duration::from_secs(14) && took < Duration::from_millis(15_500),
        "{took:?}"
    );
    let ok = &first["ok"];
    assert_eq!(
        ids(&ok["devices"]),
        ["port:9-1", B, UNKNOWN],
        "no identity answered yet"
    );
    let a = &ok["states"]["port:9-1"]["state"];
    assert_eq!(at(a, "led.brightness"), &json!(null));
    assert!(
        a["unread"]
            .as_array()
            .unwrap()
            .contains(&json!("led.brightness"))
    );
    let second = c.line(Duration::from_secs(1)).expect("the get");
    assert_eq!(second["id"], 2);
    assert!(second["ok"].is_object());
}
