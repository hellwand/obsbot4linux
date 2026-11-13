// SPDX-License-Identifier: GPL-3.0-or-later

//! The daemon's limits (spec 4.1, 5.1, 10.3 B): the 64 KiB request line, 64 connections, and
//! a subscriber that does not read.

mod common;

use common::*;
use serde_json::json;
use std::time::Duration;

const T: Duration = Duration::from_secs(5);

#[test]
fn a_request_line_of_64_kib_is_read_and_one_byte_more_closes_the_connection() {
    let d = Daemon::start();
    let (mut c, _) = d.hello(false, &[]);
    let mut line = br#"{"id":1,"live":[]}"#.to_vec();
    line.resize(obsbot_proto::MAX_REQUEST_LINE, b' ');
    line.push(b'\n');
    c.send_raw(&line);
    assert_eq!(c.reply(1), json!({"id": 1, "ok": {}}));
    let mut long = vec![b' '; obsbot_proto::MAX_REQUEST_LINE + 1];
    long.push(b'\n');
    c.send_raw(&long);
    assert!(c.closed_within(T));
}

#[test]
fn the_65th_connection_is_closed_at_once() {
    let d = Daemon::start();
    let mut open: Vec<Client> = (0..obsbot4linuxd::MAX_CONNECTIONS)
        .map(|_| d.connect())
        .collect();
    let (mut first, _) = (open.remove(0), ());
    first.hello(false, &[]);
    let mut extra = d.connect();
    assert!(extra.closed_within(Duration::from_secs(2)), "the 65th");
    first.close();
    assert!(eventually(T, || {
        let mut c = d.connect();
        c.send(&json!({"id": 1, "hello": {"protocol": 2, "client": "late"}}));
        c.line(Duration::from_millis(500))
            .is_some_and(|r| r["ok"].is_object())
    }));
    drop(open);
}

/// 5.1: when a connection's 64-line outbound queue is full, the connection is closed; the
/// daemon and the other connections go on.
#[test]
fn a_subscriber_that_does_not_read_is_disconnected() {
    let d = Daemon::start();
    let (mut slow, _) = d.hello(true, &[]);
    let (mut c, _) = d.hello(false, &[]);
    for i in 0..600 {
        let r = c.set(A, json!({"led.brightness": i % 4}));
        assert!(r["ok"].is_object());
    }
    assert!(
        slow.closed_within(Duration::from_secs(10)),
        "closed for its full queue"
    );
    assert!(c.get(A, None)["ok"].is_object(), "the rest goes on");
}
