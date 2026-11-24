// SPDX-License-Identifier: GPL-3.0-or-later

//! Stops and continuing motion (spec 4.1 rule 2, 5.6, 5.9, 15.1 rows 88 and 89): a stop
//! overtakes the queue and cuts a group read; a `gimbal.turn` without `keep: true` is stopped
//! when its connection closes, a shutdown included; no other stop is ever sent: not when the
//! camera falls asleep, not for a `keep: true` turn.

mod common;

use common::*;
use obsbot_device::models::fake::Op;
use serde_json::{Value, json};
use std::time::{Duration, Instant};

const T: Duration = Duration::from_secs(5);

fn stops(ops: &[Op]) -> usize {
    ops.iter()
        .filter(|op| matches!(op, Op::Act(a, _) if a == "gimbal.stop"))
        .count()
}

fn turn(keep: bool) -> Value {
    if keep {
        json!({"pan_speed": 10, "keep": true})
    } else {
        json!({"pan_speed": 10})
    }
}

#[test]
fn a_stop_overtakes_the_queued_requests() {
    let d = Daemon::start();
    let (mut c, _) = d.hello(false, &[]);
    d.bus.set_delay("fake-a", Duration::from_millis(300));
    c.send(&json!({"id": 1, "device": A, "set": {"led.brightness": 1}}));
    let started = Op::Apply("led.brightness".into(), json!(1));
    assert!(eventually(T, || d.ops(A).contains(&started)));
    c.send(&json!({"id": 2, "device": A, "set": {"gimbal.invert": true}}));
    c.send(&json!({"id": 3, "device": A, "do": "gimbal.move", "with": {"pan": 5}}));
    c.send(&json!({"id": 4, "device": A, "do": "gimbal.stop"}));
    let order: Vec<i64> = (0..4)
        .map(|_| c.line(T).unwrap()["id"].as_i64().unwrap())
        .collect();
    assert_eq!(order, [1, 4, 2, 3]);
    let ops = d.ops(A);
    let at = |want: &dyn Fn(&Op) -> bool| ops.iter().position(want).unwrap();
    assert!(
        at(&|op| matches!(op, Op::Act(a, _) if a == "gimbal.stop"))
            < at(&|op| matches!(op, Op::Apply(l, _) if l == "gimbal.invert"))
    );
}

/// 5.6 "Stops": a group read in progress ends at the next frame boundary.
#[test]
fn a_stop_cuts_a_group_read_in_progress() {
    let d = Daemon::start();
    let (mut c, _) = d.hello(false, &[]);
    d.bus.set_delay("fake-a", Duration::from_secs(3));
    let skip = d.ops(A).len();
    assert!(eventually(T, || d.ops(A)[skip..]
        .iter()
        .any(|op| matches!(op, Op::Read(_)))));
    d.bus.set_delay("fake-a", Duration::ZERO);
    let t0 = Instant::now();
    let r = c.act(A, "gimbal.stop", json!({}));
    assert!(r["ok"].is_object(), "{r}");
    assert!(
        t0.elapsed() < Duration::from_secs(1),
        "not after the 3 s read"
    );
}

#[test]
fn a_turn_without_keep_is_stopped_when_its_connection_closes() {
    let d = Daemon::start();
    let (mut x, _) = d.hello(false, &[]);
    assert!(x.act(A, "gimbal.turn", turn(false))["ok"].is_object());
    let skip = d.ops(A).len();
    x.close();
    assert!(
        eventually(T, || stops(&d.ops(A)[skip..]) == 1),
        "one stop for it"
    );
    std::thread::sleep(Duration::from_millis(300));
    assert_eq!(stops(&d.ops(A)[skip..]), 1);
    assert_eq!(
        d.ops(A)[skip..].iter().find(|op| matches!(op, Op::Act(..))),
        Some(&Op::Act("gimbal.stop".into(), Default::default())),
        "the stop is sent with no parameters"
    );
}

/// 15.1 row 89: a `keep: true` turn goes on; the daemon sends no stop for it.
#[test]
fn a_keep_turn_is_never_stopped_by_the_daemon() {
    let d = Daemon::start();
    let (mut y, _) = d.hello(false, &[]);
    assert!(y.act(A, "gimbal.turn", turn(true))["ok"].is_object());
    let skip = d.ops(A).len();
    y.close();
    std::thread::sleep(Duration::from_millis(800));
    assert_eq!(stops(&d.ops(A)[skip..]), 0);
}

/// 5.6: any other motion action of the device clears the record, so closing the turning
/// connection does not stop the other client's motion.
#[test]
fn another_motion_clears_the_turn_record() {
    let d = Daemon::start();
    let (mut x, _) = d.hello(false, &[]);
    let (mut y, _) = d.hello(false, &[]);
    assert!(x.act(A, "gimbal.turn", turn(false))["ok"].is_object());
    assert!(y.act(A, "zoom.move", json!({"factor": 2, "speed": "medium"}))["ok"].is_object());
    let skip = d.ops(A).len();
    x.close();
    std::thread::sleep(Duration::from_millis(800));
    assert_eq!(stops(&d.ops(A)[skip..]), 0);
}

/// 15.1 row 89 and CAM-7: the daemon sends no stop when the camera falls asleep during a turn.
#[test]
fn no_stop_when_the_camera_falls_asleep_during_a_turn() {
    let d = Daemon::start();
    let (mut x, _) = d.hello(true, &[]);
    assert!(x.act(A, "gimbal.turn", turn(false))["ok"].is_object());
    let skip = d.ops(A).len();
    d.control(&format!("{{\"sleep\":\"{A}\"}}"));
    x.event(T, |e| state_of(A)(e) && e["state"]["asleep"] == true)
        .unwrap();
    std::thread::sleep(Duration::from_millis(800));
    assert_eq!(stops(&d.ops(A)[skip..]), 0);
}

/// 5.9: a shutdown closes the connections, so a turn without `keep` gets its stop; a
/// `keep: true` turn goes on.
#[test]
fn shutdown_stops_a_turn_without_keep_only() {
    for (keep, want) in [(false, 1), (true, 0)] {
        let d = Daemon::start();
        let (mut x, _) = d.hello(false, &[]);
        assert!(x.act(A, "gimbal.turn", turn(keep))["ok"].is_object());
        let skip = d.ops(A).len();
        let bus = d.bus.clone();
        d.stop().expect("a clean stop");
        assert_eq!(stops(&bus.ops(A)[skip..]), want, "keep: {keep}");
    }
}
