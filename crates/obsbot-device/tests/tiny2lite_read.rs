// SPDX-License-Identifier: GPL-3.0-or-later

//! The Tiny 2 Lite model reading its groups and its identity on a recorded camera (replay
//! transport): what each read sends, what it decodes, and how a read fails.

mod support;

use obsbot_device::contract::{Camera, CameraError, Ctx, Frame, Need};
use obsbot_device::transport::replay::{Sent, Step};
use serde_json::json;
use support::*;

fn queries(log: &obsbot_device::transport::replay::Log) -> Vec<(u8, u16)> {
    log.sent()
        .into_iter()
        .filter_map(|s| match s {
            Sent::Xu { selector: 2, data } if matches!(data[1], 0x01 | 0x21) => {
                Some((data[9], u16::from_le_bytes([data[10], data[11]])))
            }
            _ => None,
        })
        .collect()
}

#[test]
fn an_identity_attempt_sends_only_the_missing_queries() {
    let (mut cam, log) = model(recorded());
    let both = cam
        .identity(
            Need {
                serial: true,
                firmware: true,
            },
            &Ctx::none(),
        )
        .unwrap();
    assert_eq!(both.serial.as_deref(), Some("EXAMPLE0000001"));
    assert_eq!(both.firmware.as_deref(), Some("6.2.7.4"));
    assert_eq!(queries(&log), [(0x0d, 0x18c8), (0x0d, 0x0408)]);

    log.clear();
    let firmware_only = cam
        .identity(
            Need {
                serial: false,
                firmware: true,
            },
            &Ctx::none(),
        )
        .unwrap();
    assert_eq!(firmware_only.serial, None);
    assert_eq!(queries(&log), [(0x0d, 0x0408)]);

    log.clear();
    cam.identity(
        Need {
            serial: false,
            firmware: false,
        },
        &Ctx::none(),
    )
    .unwrap();
    assert!(log.sent().is_empty(), "nothing asked, nothing sent (5.5)");
}

#[test]
fn an_unanswered_identity_query_spends_the_attempt_without_failing_it() {
    // a camera that does not answer the serial query (CAM-1): no serial, the firmware still
    let (mut cam, _) = model(obsbot_device::transport::replay::Replay::new(2).answer(
        0x0d,
        0x0408,
        &hex("0407020600000000"),
    ));
    let id = cam
        .identity(
            Need {
                serial: true,
                firmware: true,
            },
            &Ctx::none(),
        )
        .unwrap();
    assert_eq!((id.serial, id.firmware.as_deref()), (None, Some("6.2.7.4")));
    // a refused query is spent the same way
    let (mut cam, _) =
        model(obsbot_device::transport::replay::Replay::new(2).reject(0x0d, 0x18c8, -3));
    let id = cam
        .identity(
            Need {
                serial: true,
                firmware: false,
            },
            &Ctx::none(),
        )
        .unwrap();
    assert_eq!(id.serial, None);
}

#[test]
fn an_identity_attempt_on_a_gone_camera_fails() {
    let (mut cam, _) = model(recorded().gone());
    assert_eq!(
        cam.identity(
            Need {
                serial: true,
                firmware: true
            },
            &Ctx::none()
        ),
        Err(CameraError::Gone)
    );
}

#[test]
fn every_group_reads_exactly_its_leaves_and_sends_no_command() {
    // the tracking on, so that the target group is asked too
    let (mut cam, log) = model(tracking());
    let plan = cam.poll_plan().to_vec();
    for group in &plan {
        let values = cam.read(&group.name, &Ctx::none()).unwrap();
        let mut got: Vec<&String> = values.keys().collect();
        let mut want: Vec<&String> = group.leaves.iter().collect();
        got.sort();
        want.sort();
        assert_eq!(got, want, "group {}", group.name);
    }
    // reads write queries and the preset list rewind, and nothing else (spec 2)
    assert_eq!(commands(&log), [Frame::Block { selector: 0x0c }]);
}

#[test]
fn the_status_read_decodes_the_recorded_block_and_remembers_the_mode() {
    let (mut cam, log) = model(recorded());
    let v = cam.read("status", &Ctx::none()).unwrap();
    assert_eq!(v["tracking.mode"], json!("static"));
    assert_eq!(v["led.brightness"], json!(3));
    assert_eq!(v["sleep.auto.seconds"], json!(120));
    assert_eq!(cam.memory().mode, Some(json!("static")));
    assert_eq!(cam.memory().sleep_word, Some(-120));
    assert!(
        log.sent().is_empty(),
        "a status read is a read of selector 6, no write"
    );
}

#[test]
fn the_target_is_read_only_while_the_tracking_mode_is_known_and_not_static() {
    // the mode not read yet: no query and no value the camera did not report, so the leaves
    // stay in `unread` (`detected` is not nullable)
    let (mut cam, log) = model(tracking());
    assert!(matches!(
        cam.read("target", &Ctx::none()),
        Err(CameraError::NoAnswer(_))
    ));
    assert!(log.sent().is_empty());
    // static and never read: the same
    let (mut cam, log) = model(recorded());
    cam.read("status", &Ctx::none()).unwrap();
    assert!(matches!(
        cam.read("target", &Ctx::none()),
        Err(CameraError::NoAnswer(_))
    ));
    assert!(log.sent().is_empty());

    // tracking: the query and the camera's answer; then static: the values of the last read,
    // without a query (trait `Camera::read`)
    let replay = recorded()
        .reads(6, &[status_tracking()])
        .answer(0x04, 0x01c4, &target_answer());
    let (mut cam, log) = model(replay);
    cam.read("status", &Ctx::none()).unwrap();
    let v = cam.read("target", &Ctx::none()).unwrap();
    assert_eq!(queries(&log), [(0x04, 0x01c4)]);
    assert_eq!(v["tracking.target.detected"], json!(true));
    assert_eq!(
        v["tracking.target.box"],
        json!({"left": 0.25, "top": 0.5, "right": 0.75, "bottom": 1})
    );
    cam.read("status", &Ctx::none()).unwrap();
    assert_eq!(cam.memory().mode, Some(json!("static")));
    assert_eq!(cam.read("target", &Ctx::none()).unwrap(), v);
    assert_eq!(queries(&log), [(0x04, 0x01c4)]);
}

#[test]
fn a_query_of_the_model_holds_the_node_lock_over_the_frame_and_its_answer() {
    // 5.8: one hold covers the query and the reads of its answer, on the transport a bus opens
    // (the model behind the send-table check)
    let (mut cam, log) = model(recorded());
    cam.read("zoom", &Ctx::none()).unwrap();
    assert_eq!(
        log.steps(),
        [Step::Hold, Step::Set(2), Step::Get(2), Step::Release]
    );
}

#[test]
fn the_live_pose_comes_from_its_own_query_in_the_units_of_4_2() {
    let (mut cam, log) = model(recorded());
    let v = cam.read("pose", &Ctx::none()).unwrap();
    assert_eq!(
        (&v["gimbal.pan"], &v["gimbal.tilt"]),
        (&json!(25), &json!(10))
    );
    assert_eq!(
        queries(&log),
        [(0x03, 0x0043)],
        "receiver 3, query 0x0043 only (8.8)"
    );
}

#[test]
fn the_preset_list_is_rewound_once_and_read_whole() {
    let (mut cam, log) = model(recorded());
    let v = cam.read("presets", &Ctx::none()).unwrap();
    assert_eq!(
        v["presets"],
        json!([{"slot": 0, "name": "t4ltest", "pose": {"pan": 12.5, "tilt": 3.3, "zoom": 1.5}}])
    );
    assert_eq!(
        log.sent(),
        [Sent::Xu {
            selector: 0x0c,
            data: block("0100").to_vec()
        }],
        "the list is written back as it was read"
    );
    // an empty list: nothing to rewind
    let (mut cam, log) = model(recorded().buffer(0x0c, block("00")));
    assert_eq!(
        cam.read("presets", &Ctx::none()).unwrap()["presets"],
        json!([])
    );
    assert!(log.sent().is_empty());
    // the count says two, one entry decodes: a failed read, not a short list
    let mut end = preset_entry(1);
    end[0] = 2;
    let (mut cam, _) = model(
        recorded()
            .buffer(0x0c, block("020001"))
            .reads(0x0d, &[preset_entry(0), end]),
    );
    assert!(matches!(
        cam.read("presets", &Ctx::none()),
        Err(CameraError::NoAnswer(_))
    ));
}

#[test]
fn the_views_are_listed_with_their_names_and_poses() {
    let (mut cam, log) = model(recorded());
    let v = cam.read("views", &Ctx::none()).unwrap();
    assert_eq!(
        v["views"],
        json!([{"id": 0, "name": "Bereich1", "pose": {"pan": 0.2, "tilt": 0, "zoom": 1.48}}])
    );
    assert_eq!(
        queries(&log),
        [(0x04, 0x1344), (0x04, 0x1504), (0x04, 0x1444)]
    );
    // no view stored: the empty answer of capture 11
    let (mut cam, _) = model(recorded().empty(0x04, 0x1344));
    assert_eq!(cam.read("views", &Ctx::none()).unwrap()["views"], json!([]));
}

#[test]
fn a_failed_query_fails_its_group_only() {
    // the hand return timer (U-6) unanswered: the timer group fails, the hand group reads
    let (mut cam, _) = model(recorded().reject(0x04, 0x2484, -1));
    assert!(matches!(
        cam.read("hand_timer", &Ctx::none()),
        Err(CameraError::Rejected(_))
    ));
    assert!(cam.read("hand", &Ctx::none()).is_ok());
    // an unanswered query of a group with several: the group fails at it
    let (mut cam, log) = model(obsbot_device::transport::replay::Replay::new(2).answer(
        0x04,
        0x4c44,
        &hex("0000c8c2"),
    ));
    assert!(matches!(
        cam.read("zone", &Ctx::none()),
        Err(CameraError::NoAnswer(_))
    ));
    assert_eq!(
        queries(&log),
        [(0x04, 0x4c44), (0x04, 0x4cc4)],
        "the read stops at its first unanswered query (5.7)"
    );
}

#[test]
fn a_gone_camera_fails_a_read_at_once() {
    let (mut cam, _) = model(recorded().gone());
    assert_eq!(cam.read("status", &Ctx::none()), Err(CameraError::Gone));
    assert_eq!(cam.read("zoom", &Ctx::none()), Err(CameraError::Gone));
}

#[test]
fn a_stop_cuts_a_group_read_before_its_next_query() {
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;
    let (mut cam, log) = model(recorded());
    let ctx = Ctx::interruptible(Arc::new(AtomicBool::new(true)));
    assert_eq!(cam.read("framing", &ctx), Err(CameraError::Cut));
    assert!(log.sent().is_empty());
}

#[test]
fn the_audio_group_takes_the_microphone_from_the_usb_interfaces() {
    let mut usb = usb();
    usb.interfaces.truncate(2);
    let (mut cam, _) = model(recorded().usb(usb));
    let v = cam.read("audio", &Ctx::none()).unwrap();
    assert_eq!(
        (&v["audio.enabled"], &v["audio.volume"]),
        (&json!(false), &json!(55))
    );
}

#[test]
fn an_unknown_group_is_an_error_and_sends_nothing() {
    let (mut cam, log) = model(recorded());
    assert!(matches!(
        cam.read("gimbal", &Ctx::none()),
        Err(CameraError::Io(_))
    ));
    assert!(log.sent().is_empty());
}
