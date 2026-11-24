// SPDX-License-Identifier: GPL-3.0-or-later

//! Hotplug and identity (spec 4.1 Events, 5.3 to 5.5, 10.3 B): a device-number change as an
//! unplug plus a plug, the `port:` → `sn:` hand-over with `replaced_by` and its rev, an
//! identity series that ends at the first complete answer, and requests for an unavailable
//! device answered at once.

mod common;

use common::*;
use obsbot_device::models::fake::Op;
use serde_json::{Value, json};
use std::time::{Duration, Instant};

const T: Duration = Duration::from_secs(5);

/// The events of one device id and the device lists, in arrival order, until `pred` holds.
fn collect_until(c: &mut Client, pred: impl Fn(&Value) -> bool) -> Vec<Value> {
    let mut out = Vec::new();
    let until = Instant::now() + T;
    while Instant::now() < until {
        let Some(ev) = c.line(Duration::from_millis(200)) else {
            continue;
        };
        let stop = pred(&ev);
        out.push(ev);
        if stop {
            return out;
        }
    }
    panic!("the expected event did not come: {out:?}");
}

fn summary(events: &[Value], id: &str) -> Vec<String> {
    events
        .iter()
        .filter_map(|e| match e["event"].as_str() {
            Some("devices") => Some(format!("devices {}", ids(&e["devices"]).join(","))),
            Some("state") if e["device"] == id => {
                Some(format!("state {}", e["state"]["available"]))
            }
            _ => None,
        })
        .collect()
}

/// 5.3: a device-number change under the same path is handled as an unplug followed by a plug,
/// even when no scan saw the path missing; the sn: entry is kept and taken back.
#[test]
fn a_new_device_number_is_an_unplug_and_a_plug() {
    let d = Daemon::start();
    let (mut c, _) = d.hello(true, &[]);
    d.control(&format!("{{\"unplug\":\"{A}\"}}"));
    d.control("{\"plug\":\"fake-a\"}");
    let events = collect_until(&mut c, |e| {
        state_of(A)(e) && e["state"]["available"] == true
    });
    assert_eq!(
        summary(&events, A),
        [
            format!("devices {A}!,{B},{UNKNOWN}"),
            "state false".to_string(),
            format!("devices {A},{B},{UNKNOWN}"),
            "state true".to_string(),
        ]
    );
}

/// 5.5 and the golden hotplug.jsonl: the first identity attempt after the re-plug fails, the
/// device enters as `port:`; the next attempt answers the serial: two devices events and a
/// state event in a row, the old id answered `unknown_device` from then on.
#[test]
fn the_hand_over_from_port_to_sn() {
    let d = Daemon::start();
    let (mut c, _) = d.hello(true, &[]);
    d.control(&format!("{{\"unplug\":\"{A}\"}}"));
    let off = c
        .event(T, |e| state_of(A)(e) && e["state"]["available"] == false)
        .unwrap();
    let old_rev = off["rev"].as_u64().unwrap();
    d.bus.fail_next(A).unwrap();
    d.control("{\"plug\":\"fake-a\"}");
    let port = c
        .event(T, state_of("port:9-1"))
        .expect("enters as port:9-1");
    assert_eq!(port["rev"], 1);
    let events = collect_until(&mut c, state_of(A));
    let tail: Vec<&Value> = events.iter().rev().take(3).collect();
    let (handed, without, state) = (tail[2], tail[1], tail[0]);
    assert!(devices(handed) && devices(without), "{events:?}");
    let old = handed["devices"]
        .as_array()
        .unwrap()
        .iter()
        .find(|dev| dev["id"] == "port:9-1")
        .expect("the old entry in event 1");
    assert_eq!(old["available"], false);
    assert_eq!(old["replaced_by"], A);
    assert_eq!(ids(&without["devices"]), [A, B, UNKNOWN]);
    assert!(
        state["rev"].as_u64().unwrap() > old_rev.max(1),
        "the rev never goes down"
    );
    assert_eq!(state["state"]["available"], true);
    assert_eq!(c.get("port:9-1", None)["error"]["code"], "unknown_device");
}

/// 10.3: an identity series ends at the first complete answer: one attempt, no more.
#[test]
fn an_identity_series_ends_at_the_first_complete_answer() {
    let d = Daemon::start();
    let (_c, _) = d.hello(false, &[]);
    std::thread::sleep(Duration::from_millis(1600));
    let attempts = d
        .ops(A)
        .iter()
        .filter(|op| matches!(op, Op::Identity(_)))
        .count();
    assert_eq!(attempts, 1, "three 500 ms identity ticks passed");
}

/// 4.1 rule 2: requests for an unavailable device are answered by the reader at once, even
/// while other devices' workers are busy.
#[test]
fn requests_for_an_unavailable_device_are_answered_at_once() {
    let d = Daemon::start();
    let (mut c, _) = d.hello(true, &[]);
    d.control(&format!("{{\"unplug\":\"{B}\"}}"));
    c.event(T, |e| state_of(B)(e) && e["state"]["available"] == false)
        .unwrap();
    d.bus.set_delay("fake-a", Duration::from_secs(3));
    let t0 = Instant::now();
    assert_eq!(
        c.set(B, json!({"led.brightness": 1}))["error"]["code"],
        "unavailable"
    );
    assert_eq!(c.act(B, "sleep", json!({}))["error"]["code"], "unavailable");
    assert_eq!(c.get(B, Some(&["led"]))["ok"]["state"]["available"], false);
    assert!(t0.elapsed() < Duration::from_millis(300));
    d.bus.set_delay("fake-a", Duration::ZERO);
}
