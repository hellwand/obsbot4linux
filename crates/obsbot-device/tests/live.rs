// SPDX-License-Identifier: GPL-3.0-or-later

//! Live tests on a connected Tiny 2 Lite (design spec 10.3 track A, 11, 14: camera session
//! CS-1). Every test is `#[ignore]` and does nothing unless `OBSBOT_LIVE=1` is set; the tests
//! that write also need `OBSBOT_LIVE_WRITE=1` and put back what they wrote. They talk to the
//! camera only through the model, behind its send-table check, so they send only the model's
//! send table (8.3). Stop `obsbot4linux.socket` and `obsbot4linux.service` first. Run one at a
//! time:
//!
//! `OBSBOT_LIVE=1 cargo test -p obsbot-device --test live -- --ignored --nocapture --test-threads=1`
//!
//! The serial is never printed (4.2): only `short_id` of it.

use obsbot_device::bus::SysfsBus;
use obsbot_device::contract::{Bus, Camera, CameraError, Ctx, Found, Need};
use obsbot_device::family::link::{Link, Timing, process_sequence};
use obsbot_device::models::tiny2lite::model::SELECTORS;
use obsbot_device::models::tiny2lite::{decode, plan, send_table};
use obsbot_device::transport::checked::Checked;
use obsbot_device::transport::usb::UsbTransport;
use obsbot_proto::{
    Bound, Description, Params, Setting, SettingType, State, is_valid_serial, short_id, values,
};
use serde_json::{Value, json};
use std::time::{Duration, Instant};

fn enabled(var: &str) -> bool {
    std::env::var(var).as_deref() == Ok("1")
}

/// The first Tiny 2 Lite the bus finds, opened; `None` (and a line saying why) when the live
/// tests are not enabled or no camera is there.
fn camera(write: bool) -> Option<(Found, Box<dyn Camera>)> {
    if !enabled("OBSBOT_LIVE") || (write && !enabled("OBSBOT_LIVE_WRITE")) {
        eprintln!("skipped: set OBSBOT_LIVE=1 (and OBSBOT_LIVE_WRITE=1 for writes)");
        return None;
    }
    let bus = SysfsBus::new();
    let found = bus
        .scan()
        .into_iter()
        .find(|f| f.model.is_some_and(|m| m.key == "tiny2lite"))
        .expect("a connected Tiny 2 Lite (USB 3564:fef9)");
    let camera = bus.open(&found).expect("open");
    Some((found, camera))
}

/// A raw link to the camera behind the send-table check, for reads the model does not expose.
/// The extension unit is the one with the registry entry's GUID (8.3).
fn link(found: &Found) -> Link {
    let unit = found
        .usb
        .extension_units
        .iter()
        .find(|u| u.guid == plan::XU_GUID)
        .expect("the extension unit of the registry entry")
        .unit;
    let usb_dir = std::path::Path::new("/sys/bus/usb/devices").join(&found.usb.path);
    let t = UsbTransport::new(found.video_node.clone().unwrap(), unit, usb_dir);
    Link::new(
        Box::new(Checked::new(t, send_table::table(), SELECTORS)),
        process_sequence(),
        Timing::CAMERA,
    )
}

/// The group of `leaf` in the model's poll plan.
fn group_of(cam: &dyn Camera, leaf: &str) -> String {
    cam.poll_plan()
        .iter()
        .find(|g| g.leaves.iter().any(|l| l == leaf))
        .unwrap_or_else(|| panic!("{leaf} is in no group"))
        .name
        .clone()
}

/// `leaf` as one read of its group shows it.
fn read_leaf(cam: &mut dyn Camera, leaf: &str) -> Result<Value, CameraError> {
    let group = group_of(cam, leaf);
    Ok(cam.read(&group, &Ctx::none())?[leaf].clone())
}

/// The state after one read of every group, as the daemon publishes it (a group that fails
/// puts its leaves in `unread`): what the checks and the actions see.
fn state(cam: &mut dyn Camera) -> State {
    let mut s = State::new(true);
    for g in cam.poll_plan().to_vec() {
        match cam.read(&g.name, &Ctx::none()) {
            Ok(values) => values.into_iter().for_each(|(leaf, v)| s.set(&leaf, v)),
            Err(_) => s.unread.extend(g.leaves.iter().cloned()),
        }
    }
    s
}

/// `action` as the daemon runs it: the model's check, then the action; the error as text.
fn act(cam: &mut dyn Camera, action: &str, p: Value, st: &State) -> Result<Option<Params>, String> {
    let p = p.as_object().cloned().unwrap_or_default();
    cam.check(action, &p, st).map_err(|r| r.message)?;
    cam.act(action, &p, st, &Ctx::none())
        .map_err(|e| e.to_string())
}

/// The bound, step or default `key` of a setting (`min`, `max`, `step`, `default`).
fn number(s: &Setting, key: &str) -> Option<f64> {
    let b = |b: &Option<Bound>| b.as_ref().and_then(|b| b.get(None));
    match key {
        "min" => b(&s.min),
        "max" => b(&s.max),
        "step" => b(&s.step),
        _ => s.default.as_ref().and_then(Value::as_f64),
    }
}

/// A valid value of a writable leaf next to `before`, from its description: the other bool,
/// another writable enum value, a number one step (or a tenth of its range) up, or down at the
/// top. `None` for the other types (the area ranges are CAM-16's) and for an undecoded value.
fn other_value(s: &Setting, before: &Value) -> Option<Value> {
    match s.kind {
        SettingType::Bool => Some(json!(!before.as_bool()?)),
        SettingType::Enum => s
            .writable
            .iter()
            .find(|w| before.as_str() != Some(w.as_str()))
            .map(|w| json!(w)),
        SettingType::Number => {
            let b = before.as_f64()?;
            let (min, max) = (number(s, "min"), number(s, "max"));
            let step = match (number(s, "step"), min, max) {
                (Some(step), _, _) => step,
                (None, Some(lo), Some(hi)) => (hi - lo) / 10.0,
                _ => 1.0,
            };
            let up = b + step;
            let v = if max.is_none_or(|hi| up <= hi) {
                up
            } else {
                b - step
            };
            Some(match s.precision.as_ref().and_then(|p| p.get(None)) {
                Some(p) => values::rounded(v, p),
                None => values::number(v),
            })
        }
        _ => None,
    }
}

/// Leaves the sweeps leave alone: switching the microphone off can take the USB audio
/// interface away (CAM-8, a step of the session).
const NOT_SWEPT: [&str; 1] = ["audio.enabled"];

/// The writable bool, enum and number leaves with a readback, in the model's apply order.
fn swept(cam: &dyn Camera) -> Vec<String> {
    let desc = cam.description();
    let leaves: Vec<&str> = desc
        .settings
        .iter()
        .filter(|(path, s)| {
            !s.readonly
                && s.readback
                && !NOT_SWEPT.contains(&path.as_str())
                && matches!(
                    s.kind,
                    SettingType::Bool | SettingType::Enum | SettingType::Number
                )
        })
        .map(|(path, _)| path.as_str())
        .collect();
    cam.order(&leaves)
}

/// Puts `leaf` back to `before`. A value that cannot be written (a `raw:<n>` or a `null` read)
/// or a write that fails leaves the leaf as the test wrote it, with a warning.
fn restore(cam: &mut dyn Camera, leaf: &str, before: &Value) {
    let raw = before.as_str().is_some_and(|s| s.starts_with("raw:"));
    let result = if before.is_null() || raw {
        Err(format!("it read {before} before the test"))
    } else {
        cam.apply(leaf, before, &Ctx::none())
            .map_err(|e| e.to_string())
    };
    if let Err(why) = result {
        eprintln!("WARNING: {leaf} not put back: {why}");
    }
}

/// Writes the `requires` values of `s` and returns what those leaves read before.
fn meet(cam: &mut dyn Camera, s: &Setting) -> Vec<(String, Value)> {
    let mut before = Vec::new();
    for r in &s.requires {
        if let Ok(v) = read_leaf(cam, &r.path) {
            before.push((r.path.clone(), v));
        }
        if let Err(e) = cam.apply(&r.path, &r.equals, &Ctx::none()) {
            eprintln!("WARNING: {} = {} not written: {e}", r.path, r.equals);
        }
    }
    before
}

/// The preset in `slot` of a state, when it is listed.
fn preset(st: &State, slot: u64) -> Option<&Value> {
    st.at("presets")?
        .as_array()?
        .iter()
        .find(|p| p.get("slot") == Some(&json!(slot)))
}

/// The lowest preset slot that holds no preset; `None` when all are taken or the list is not
/// read (no preset of the owner is overwritten).
fn free_preset_slot(desc: &Description, st: &State) -> Option<u64> {
    st.at("presets")?;
    let slots = u64::from(*desc.limits.get("presets")?);
    (0..slots).find(|slot| preset(st, *slot).is_none())
}

/// Reads the status group until `asleep` is `want`, every 500 ms for at most 20 s; how long it
/// took.
fn until_asleep_is(cam: &mut dyn Camera, want: bool) -> Option<Duration> {
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(20) {
        if read_leaf(cam, "asleep").ok() == Some(json!(want)) {
            return Some(start.elapsed());
        }
        std::thread::sleep(Duration::from_millis(500));
    }
    None
}

#[test]
#[ignore = "needs a Tiny 2 Lite and OBSBOT_LIVE=1"]
fn live_cam_25_the_sysfs_facts() {
    let Some((found, _)) = camera(false) else {
        return;
    };
    println!("{}", serde_json::to_string_pretty(&found.usb).unwrap());
    println!("devnum {} video node {:?}", found.devnum, found.video_node);
    assert_eq!(
        (found.usb.vendor.as_str(), found.usb.product.as_str()),
        ("3564", "fef9")
    );
    assert!(found.video_node.is_some(), "a node with index 0");
    assert!(
        found
            .usb
            .extension_units
            .iter()
            .any(|u| u.guid == plan::XU_GUID),
        "the extension unit of the registry entry"
    );
}

#[test]
#[ignore = "needs a Tiny 2 Lite and OBSBOT_LIVE=1"]
fn live_cam_1_the_identity_queries_answer() {
    let Some((_, mut cam)) = camera(false) else {
        return;
    };
    let start = Instant::now();
    let id = cam
        .identity(
            Need {
                serial: true,
                firmware: true,
            },
            &Ctx::none(),
        )
        .unwrap();
    println!("identity attempt took {:?}", start.elapsed());
    let serial = id.serial.expect("a serial answer");
    println!("serial {} firmware {:?}", short_id(&serial), id.firmware);
    assert!(is_valid_serial(&serial), "the serial format of 5.5");
    assert_eq!(serial.len(), 14, "14 ASCII characters (8.8)");
}

#[test]
#[ignore = "needs a Tiny 2 Lite and OBSBOT_LIVE=1"]
fn live_cam_10_cam_20_a_cold_read_of_every_group() {
    let Some((found, mut cam)) = camera(false) else {
        return;
    };
    // CAM-20: the first byte of the status block (0 while the camera boots?)
    let block = link(&found).read(6, &Ctx::none()).unwrap();
    println!("status block byte 0: {:#04x}", block[0]);
    let mut total = Duration::ZERO;
    for g in cam.poll_plan().to_vec() {
        let start = Instant::now();
        let r = cam.read(&g.name, &Ctx::none());
        let took = start.elapsed();
        total += took;
        match r {
            Ok(v) => println!("{:<12} {:>6} ms  {}", g.name, took.as_millis(), json!(v)),
            Err(e) => println!("{:<12} {:>6} ms  FAILED: {e}", g.name, took.as_millis()),
        }
    }
    println!(
        "a cold read of every group: {} ms (CAM-10)",
        total.as_millis()
    );
}

#[test]
#[ignore = "needs a Tiny 2 Lite and OBSBOT_LIVE=1"]
fn live_the_uvc_ranges_the_description_holds() {
    // the W0 plan's decision 12: these bounds, steps and defaults are not backed yet; the camera
    // reports its own (a read). A mismatch is fixed by a contract-change PR of the description.
    let Some((found, cam)) = camera(false) else {
        return;
    };
    let desc = cam.description().clone();
    let mut link = link(&found);
    let mut mismatches = Vec::new();
    for (leaf, id) in decode::UVC {
        let range = link.transport().ctrl_range(id).unwrap();
        let s = &desc.settings[leaf];
        println!("{leaf:<34} camera {range:?}");
        let Some(r) = range else {
            mismatches.push(format!("{leaf}: the camera has no control {id:#010x}"));
            continue;
        };
        // exposure.time is in 100 µs units on the camera, µs in the description
        let scale = if leaf == "exposure.time" { 100.0 } else { 1.0 };
        for (key, camera) in [
            ("min", r.min),
            ("max", r.max),
            ("step", r.step),
            ("default", r.default),
        ] {
            if let Some(ours) = number(s, key)
                && (ours - f64::from(camera) * scale).abs() > 1e-9
            {
                mismatches.push(format!(
                    "{leaf}#{key}: description {ours}, camera {}",
                    f64::from(camera) * scale
                ));
            }
        }
    }
    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
}

/// Writes `value` to `leaf`, then reads the leaf's group once a second for up to 10 s until it
/// shows `value`; puts the value read before back. Returns the lag, or the last value read (or
/// why the write failed).
fn write_and_watch(cam: &mut dyn Camera, leaf: &str, value: &Value) -> Result<Duration, Value> {
    let before = read_leaf(cam, leaf).unwrap();
    if let Err(e) = cam.apply(leaf, value, &Ctx::none()) {
        return Err(json!(format!("not written: {e}")));
    }
    let start = Instant::now();
    let outcome = loop {
        let last = read_leaf(cam, leaf).unwrap();
        if values::same(&last, value) {
            break Ok(start.elapsed());
        }
        if start.elapsed() > Duration::from_secs(10) {
            break Err(last);
        }
        std::thread::sleep(Duration::from_secs(1));
    };
    restore(cam, leaf, &before);
    outcome
}

#[test]
#[ignore = "writes to a Tiny 2 Lite: OBSBOT_LIVE=1 and OBSBOT_LIVE_WRITE=1"]
fn live_cam_23_the_u_rows_that_read_back() {
    // U-4 (offset bounds), U-5 (hand axes), U-6 (hand return time), U-18 (start modes);
    // each written, read back, restored. The lags are CAM-5 data.
    let Some((_, mut cam)) = camera(true) else {
        return;
    };
    let rows: &[(&str, Value)] = &[
        ("framing.offset.x", json!(-0.16)),
        ("framing.offset.x", json!(0.16)),
        ("framing.offset.y", json!(-0.16)),
        ("framing.offset.y", json!(0.16)),
        ("hand.axes.pan", json!(false)),
        ("hand.axes.tilt", json!(false)),
        ("hand.return_after", json!(3.0)),
        ("power_on.start.mode", json!("static")),
        ("power_on.start.mode", json!("upper_body")),
        ("power_on.start.mode", json!("headless")),
        ("power_on.start.mode", json!("group")),
    ];
    let mut failed = Vec::new();
    for (leaf, value) in rows {
        match write_and_watch(cam.as_mut(), leaf, value) {
            Ok(lag) => println!("pass {leaf} = {value} after {} ms", lag.as_millis()),
            Err(last) => {
                println!("FAIL {leaf} = {value}: reads {last}");
                failed.push(format!("{leaf} = {value}"));
            }
        }
    }
    assert!(
        failed.is_empty(),
        "rows that failed (their U-row becomes PX-U-n): {failed:?}"
    );
}

#[test]
#[ignore = "writes to a Tiny 2 Lite: OBSBOT_LIVE=1 and OBSBOT_LIVE_WRITE=1"]
fn live_cam_16_an_area_wholly_beyond_the_stored_one() {
    // U-8: the bounds go min then max; a new range wholly above or below the stored one
    let Some((_, mut cam)) = camera(true) else {
        return;
    };
    let mut failed = Vec::new();
    for leaf in [
        "zone.area.pan",
        "zone.area.tilt",
        "hand.area.pan",
        "hand.area.tilt",
    ] {
        let before = read_leaf(cam.as_mut(), leaf).unwrap();
        for (lo, hi) in [(-20.0, 20.0), (40.0, 60.0), (-60.0, -40.0)] {
            let v = json!({"min": lo, "max": hi});
            match write_and_watch(cam.as_mut(), leaf, &v) {
                Ok(_) => println!("pass {leaf} = {v}"),
                Err(last) => {
                    println!("FAIL {leaf} = {v}: reads {last}");
                    failed.push(format!("{leaf} = {v}"));
                }
            }
        }
        restore(cam.as_mut(), leaf, &before);
    }
    assert!(failed.is_empty(), "{failed:?}");
}

#[test]
#[ignore = "writes to a Tiny 2 Lite: OBSBOT_LIVE=1 and OBSBOT_LIVE_WRITE=1"]
fn live_cam_5_every_writable_leaf_reads_back_and_its_lag() {
    // CAM-5: each writable leaf with a readback written to a value next to its own (its
    // `requires` met first), read back once a second, put back; the lags are `settle_ms` data
    let Some((_, mut cam)) = camera(true) else {
        return;
    };
    let cam = cam.as_mut();
    let desc = cam.description().clone();
    let mut failed = Vec::new();
    for leaf in swept(cam) {
        let s = &desc.settings[&leaf];
        let met = meet(cam, s);
        let before = read_leaf(cam, &leaf);
        match before.as_ref().ok().and_then(|b| other_value(s, b)) {
            None => println!("skip {leaf}: it reads {before:?}"),
            Some(value) => match write_and_watch(cam, &leaf, &value) {
                Ok(lag) => println!("pass {leaf} = {value} after {} ms", lag.as_millis()),
                Err(last) => {
                    println!("FAIL {leaf} = {value}: reads {last}");
                    failed.push(format!("{leaf} = {value}"));
                }
            },
        }
        for (path, v) in met.iter().rev() {
            restore(cam, path, v);
        }
    }
    assert!(
        failed.is_empty(),
        "leaves that did not read back within 10 s: {failed:?}"
    );
}

#[test]
#[ignore = "writes to a Tiny 2 Lite: OBSBOT_LIVE=1 and OBSBOT_LIVE_WRITE=1"]
fn live_cam_2_cam_3_cam_22_asleep() {
    // CAM-2: which groups read and which writes and actions the camera takes while asleep,
    // and whether it keeps them after the wake; CAM-3: `zoom.factor` asleep, and the time from
    // `wake` until the status reports awake; CAM-22: the live pose asleep, the go-return
    // actions and the pose-taking actions (a save into a free preset slot, a new view, both
    // deleted again). Everything written is put back.
    let Some((_, mut cam)) = camera(true) else {
        return;
    };
    let cam = cam.as_mut();
    let desc = cam.description().clone();
    let awake = state(cam);
    assert_eq!(awake.at("asleep"), Some(&json!(false)), "start awake");
    act(cam, "sleep", json!({}), &awake).unwrap();
    let slept = until_asleep_is(cam, true).expect("asleep within 20 s");
    println!("asleep {} ms after `sleep`", slept.as_millis());

    for g in cam.poll_plan().to_vec() {
        match cam.read(&g.name, &Ctx::none()) {
            Ok(v) => println!("asleep read {:<12} {}", g.name, json!(v)),
            Err(e) => println!("asleep read {:<12} FAILED: {e}", g.name),
        }
    }
    let mut written = Vec::new();
    for leaf in swept(cam) {
        let Some(before) = awake.at(&leaf).cloned() else {
            continue;
        };
        let Some(value) = other_value(&desc.settings[&leaf], &before) else {
            continue;
        };
        match cam.apply(&leaf, &value, &Ctx::none()) {
            Ok(()) => {
                println!("asleep write {leaf} = {value}: sent");
                written.push((leaf, before, value));
            }
            Err(e) => println!("asleep write {leaf} = {value}: {e}"),
        }
    }
    let slot = free_preset_slot(&desc, &awake);
    let mut actions = vec![
        ("gimbal.move", json!({"pan": 10, "tilt": 5})),
        ("zoom.move", json!({"speed": "fast", "factor": 2})),
        ("zone.go_return", json!({})),
        ("hand.go_return", json!({})),
    ];
    if let Some(slot) = slot {
        actions.push(("preset.save", json!({ "slot": slot })));
    }
    for (action, p) in &actions {
        println!(
            "asleep do {action} {p}: {:?}",
            act(cam, action, p.clone(), &awake)
        );
    }
    let view = act(cam, "view.save", json!({"name": "cam22"}), &awake);
    println!("asleep do view.save: {view:?}");

    act(cam, "wake", json!({}), &awake).unwrap();
    let woke = until_asleep_is(cam, false).expect("awake within 20 s");
    println!("awake {} ms after `wake` (CAM-3)", woke.as_millis());
    for (leaf, _, value) in &written {
        let now = read_leaf(cam, leaf);
        let kept = now.as_ref().is_ok_and(|v| values::same(v, value));
        println!(
            "after the wake {leaf}: wrote {value}, reads {now:?}: {}",
            if kept { "kept" } else { "not kept" }
        );
    }
    let after = state(cam);
    println!(
        "after the wake: pan {:?} tilt {:?} zoom {:?}",
        after.at("gimbal.pan"),
        after.at("gimbal.tilt"),
        after.at("zoom.factor")
    );

    // put back: the saved preset and view, the writes, the pose
    if let Some(slot) = slot
        && let Some(p) = preset(&after, slot)
    {
        println!("the preset saved asleep is there: {p}");
        act(cam, "preset.delete", json!({ "slot": slot }), &after).unwrap();
    }
    if let Ok(Some(result)) = &view {
        let id = result["id"].clone();
        println!(
            "view.save asleep gave id {id}; views now {:?}",
            after.at("views")
        );
        if let Err(e) = act(cam, "view.delete", json!({ "id": id }), &after) {
            eprintln!("WARNING: view {id} not deleted: {e}");
        }
    }
    for (leaf, before, _) in written.iter().rev() {
        restore(cam, leaf, before);
    }
    if let (Some(pan), Some(tilt)) = (awake.at("gimbal.pan"), awake.at("gimbal.tilt")) {
        let back = json!({"pan": pan, "tilt": tilt});
        if let Err(e) = act(cam, "gimbal.move", back, &awake) {
            eprintln!("WARNING: the gimbal not moved back: {e}");
        }
    }
}

#[test]
#[ignore = "writes to a Tiny 2 Lite: OBSBOT_LIVE=1 and OBSBOT_LIVE_WRITE=1"]
fn live_cam_4_the_zoom_readback_during_a_slow_zoom_move() {
    // CAM-4: during a speed-limited `zoom.move`, does the zoom readback show the target or the
    // factor on the way? The readings every 200 ms are the answer. The zoom is put back.
    let Some((_, mut cam)) = camera(true) else {
        return;
    };
    let cam = cam.as_mut();
    let before = read_leaf(cam, "zoom.factor").unwrap();
    let target = if before.as_f64().unwrap_or(1.0) < 2.5 {
        4.0
    } else {
        1.0
    };
    let st = State::new(true);
    act(
        cam,
        "zoom.move",
        json!({"speed": "slow", "factor": target}),
        &st,
    )
    .unwrap();
    let start = Instant::now();
    let mut readings = Vec::new();
    while start.elapsed() < Duration::from_secs(6) {
        let v = read_leaf(cam, "zoom.factor");
        println!("{:>5} ms  zoom.factor {v:?}", start.elapsed().as_millis());
        readings.push(v);
        std::thread::sleep(Duration::from_millis(200));
    }
    if let Err(e) = act(
        cam,
        "zoom.move",
        json!({"speed": "fast", "factor": before}),
        &st,
    ) {
        eprintln!("WARNING: the zoom not put back: {e}");
    }
    let first = readings.first().and_then(|r| r.as_ref().ok());
    let shows_target = first.is_some_and(|v| values::same(v, &json!(target)));
    println!(
        "the first reading {first:?} shows {}",
        if shows_target {
            "the target"
        } else {
            "the factor on the way"
        }
    );
    assert!(
        readings.iter().all(Result::is_ok),
        "every zoom read answered"
    );
}

#[test]
#[ignore = "writes to a Tiny 2 Lite: OBSBOT_LIVE=1 and OBSBOT_LIVE_WRITE=1"]
fn live_cam_13_anti_flicker_and_exposure_mode_in_both_orders() {
    // CAM-13: `exposure.anti_flicker` and `exposure.mode` written one right after the other, in
    // both orders; both must read back for the order not to matter (8.8 apply order)
    let Some((_, mut cam)) = camera(true) else {
        return;
    };
    let cam = cam.as_mut();
    let desc = cam.description().clone();
    let leaves = ["exposure.anti_flicker", "exposure.mode"];
    let before: Vec<Value> = leaves.iter().map(|l| read_leaf(cam, l).unwrap()).collect();
    let wanted: Vec<Value> = leaves
        .iter()
        .zip(&before)
        .map(|(l, b)| other_value(&desc.settings[*l], b).unwrap())
        .collect();
    let mut failed = Vec::new();
    for order in [[0, 1], [1, 0]] {
        for i in order {
            cam.apply(leaves[i], &wanted[i], &Ctx::none()).unwrap();
        }
        std::thread::sleep(Duration::from_secs(3));
        let name = format!("{} then {}", leaves[order[0]], leaves[order[1]]);
        for (leaf, value) in leaves.iter().zip(&wanted) {
            let now = read_leaf(cam, leaf).unwrap();
            let ok = values::same(&now, value);
            println!(
                "{name}: {leaf} wrote {value}, reads {now}: {}",
                if ok { "pass" } else { "FAIL" }
            );
            if !ok {
                failed.push(format!("{name}: {leaf}"));
            }
        }
        for (leaf, value) in leaves.iter().zip(&before).rev() {
            restore(cam, leaf, value);
        }
        std::thread::sleep(Duration::from_secs(3));
    }
    assert!(
        failed.is_empty(),
        "an order that loses a write becomes an apply-order rule (8.8): {failed:?}"
    );
}

#[test]
#[ignore = "writes to a Tiny 2 Lite: OBSBOT_LIVE=1 and OBSBOT_LIVE_WRITE=1"]
fn live_cam_15_a_manual_value_while_its_automatic_control_is_on() {
    // CAM-15: the white-balance temperature and the absolute focus written while their
    // automatic control is on (the model sends what it is given; the daemon's `requires` check
    // refuses it before). The outcome printed is the answer; both are put back.
    let Some((_, mut cam)) = camera(true) else {
        return;
    };
    let cam = cam.as_mut();
    let desc = cam.description().clone();
    for (auto, on, leaf) in [
        (
            "image.white_balance.auto",
            json!(true),
            "image.white_balance.temperature",
        ),
        ("focus.mode", json!("continuous"), "focus.position"),
    ] {
        let auto_before = read_leaf(cam, auto).unwrap();
        let before = read_leaf(cam, leaf).unwrap();
        cam.apply(auto, &on, &Ctx::none()).unwrap();
        let value = other_value(&desc.settings[leaf], &before).unwrap();
        let sent = cam.apply(leaf, &value, &Ctx::none());
        std::thread::sleep(Duration::from_secs(2));
        println!(
            "{auto} = {on}, then {leaf} = {value}: {sent:?}; it reads {:?}",
            read_leaf(cam, leaf)
        );
        meet(cam, &desc.settings[leaf]);
        restore(cam, leaf, &before);
        restore(cam, auto, &auto_before);
    }
}

#[test]
#[ignore = "writes to a Tiny 2 Lite: OBSBOT_LIVE=1 and OBSBOT_LIVE_WRITE=1"]
fn live_cam_17_gimbal_moves_with_speed() {
    // CAM-17 (U-9): `gimbal.move` with a speed near the tilt limit and at the speed ceiling, and
    // with one axis only; each must arrive within 1°. The pose is put back.
    let Some((_, mut cam)) = camera(true) else {
        return;
    };
    let cam = cam.as_mut();
    let desc = cam.description().clone();
    let p = &desc.actions["gimbal.move"].params;
    let tilt_max = number(&p["tilt"], "max").unwrap();
    let speed_max = number(&p["speed"], "max").unwrap();
    let st = State::new(true);
    let pose = |cam: &mut dyn Camera| {
        let v = cam.read("pose", &Ctx::none()).unwrap();
        (
            v["gimbal.pan"].as_f64().unwrap(),
            v["gimbal.tilt"].as_f64().unwrap(),
        )
    };
    let start = pose(cam);
    let mut failed = Vec::new();
    for (params, want) in [
        (
            json!({"pan": 0, "tilt": tilt_max - 1.0, "speed": speed_max}),
            (Some(0.0), Some(tilt_max - 1.0)),
        ),
        (
            json!({"pan": 30, "speed": speed_max / 2.0}),
            (Some(30.0), None),
        ),
        (
            json!({"tilt": -10, "speed": speed_max / 2.0}),
            (None, Some(-10.0)),
        ),
    ] {
        act(cam, "gimbal.move", params.clone(), &st).unwrap();
        std::thread::sleep(Duration::from_secs(4));
        let (pan, tilt) = pose(cam);
        let near = |got: f64, want: Option<f64>| want.is_none_or(|w| (got - w).abs() <= 1.0);
        let ok = near(pan, want.0) && near(tilt, want.1);
        println!(
            "gimbal.move {params}: pan {pan} tilt {tilt}: {}",
            if ok { "pass" } else { "FAIL" }
        );
        if !ok {
            failed.push(params.to_string());
        }
    }
    let back = json!({"pan": start.0, "tilt": start.1});
    if let Err(e) = act(cam, "gimbal.move", back, &st) {
        eprintln!("WARNING: the gimbal not moved back: {e}");
    }
    assert!(
        failed.is_empty(),
        "moves that did not arrive (U-9): {failed:?}"
    );
}

#[test]
#[ignore = "writes to a Tiny 2 Lite: OBSBOT_LIVE=1 and OBSBOT_LIVE_WRITE=1"]
fn live_cam_35_a_preset_saved_without_a_name() {
    // CAM-35: what the camera reports as the name of a preset saved without one (obsbot4linux
    // sends none). The preset goes into a free slot and is deleted again; with no free slot
    // the test writes nothing.
    let Some((_, mut cam)) = camera(true) else {
        return;
    };
    let cam = cam.as_mut();
    let desc = cam.description().clone();
    let st = state(cam);
    let Some(slot) = free_preset_slot(&desc, &st) else {
        eprintln!("skipped: no free preset slot, or the presets did not read");
        return;
    };
    act(cam, "preset.save", json!({ "slot": slot }), &st).unwrap();
    std::thread::sleep(Duration::from_secs(1));
    let st = state(cam);
    let saved = preset(&st, slot).cloned();
    println!("preset {slot}, saved without a name, reads {saved:?}");
    if let Err(e) = act(cam, "preset.delete", json!({ "slot": slot }), &st) {
        eprintln!("WARNING: preset {slot} not deleted: {e}");
    }
    let saved = saved.expect("the saved preset is listed");
    assert!(
        saved.get("name").is_some(),
        "the item has its name field: a string, or null"
    );
}
