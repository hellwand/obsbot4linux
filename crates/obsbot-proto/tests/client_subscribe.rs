// SPDX-License-Identifier: GPL-3.0-or-later

//! The subscribed client (spec 6.1) replays the golden session transcript of W0 and reports
//! a lost connection and an overdue reply (5.8).

use obsbot_proto::client::script::ScriptServer;
use obsbot_proto::client::{Client, ClientError, Incoming};
use obsbot_proto::{ErrorCode, Event, Hello, OkBody, Patch, ReplyBody, RequestId};
use serde_json::json;
use std::path::PathBuf;
use std::time::Duration;

fn socket(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("c10-{name}.sock"))
}

fn golden(name: &str) -> String {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden");
    std::fs::read_to_string(dir.join(name)).unwrap()
}

fn next(rx: &mut obsbot_proto::client::Receiver) -> Incoming {
    rx.next().expect("a message").expect("not an error")
}

fn reply_id(msg: &Incoming) -> Option<RequestId> {
    match msg {
        Incoming::Reply { id, .. } => id.clone(),
        _ => None,
    }
}

fn state_rev(msg: &Incoming) -> Option<u64> {
    match msg {
        Incoming::Event(Event::State { rev, .. }) => Some(*rev),
        _ => None,
    }
}

#[test]
fn the_golden_session_replays_through_sender_and_receiver() {
    let server = ScriptServer::start(&socket("session"), &golden("session.jsonl"));
    let (hello, sender, mut rx) =
        Client::subscribe_to(server.path(), Hello::new("widget 1.0").live(&["pose"])).unwrap();
    assert_eq!(hello.devices.len(), 2);
    assert_eq!(hello.states["sn:EXAMPLE0000001"].rev, 4);

    let id = sender
        .set("sn:EXAMPLE0000001", Patch::new().leaf("led.brightness", 2))
        .unwrap();
    assert_eq!(id, RequestId::Int(2));
    // 4.1 rule 3: the state event comes before the reply.
    assert_eq!(state_rev(&next(&mut rx)), Some(5));
    let reply = next(&mut rx);
    assert_eq!(reply_id(&reply), Some(RequestId::Int(2)));
    assert_eq!(state_rev(&next(&mut rx)), Some(6));

    assert_eq!(
        sender.get("sn:EXAMPLE0000001", Some(&["led"])).unwrap(),
        RequestId::Int(3)
    );
    match next(&mut rx) {
        Incoming::Reply {
            body: ReplyBody::Ok(OkBody::State(rs)),
            ..
        } => assert_eq!(rs.state.at("led.brightness"), Some(&json!(2))),
        other => panic!("{other:?}"),
    }

    sender
        .act("sn:EXAMPLE0000001", "preset.recall", json!({"slot": 0}))
        .unwrap();
    assert_eq!(reply_id(&next(&mut rx)), Some(RequestId::Int(4)));
    assert_eq!(state_rev(&next(&mut rx)), Some(7));

    sender.live(&["pose", "target"]).unwrap();
    match next(&mut rx) {
        Incoming::Reply {
            id: Some(RequestId::Int(5)),
            body: ReplyBody::Ok(OkBody::Empty(_)),
        } => {}
        other => panic!("{other:?}"),
    }

    sender.get("port:3-2.4", None).unwrap();
    assert_eq!(reply_id(&next(&mut rx)), Some(RequestId::Int(6)));
    sender
        .set("port:3-2.4", Patch::new().leaf("led.brightness", 2))
        .unwrap();
    match next(&mut rx) {
        Incoming::Reply {
            body: ReplyBody::Error(e),
            ..
        } => assert_eq!(e.code, ErrorCode::UnsupportedDevice),
        other => panic!("{other:?}"),
    }
    drop(sender);
    drop(rx);
    assert_eq!(server.finish().unwrap().len(), 7);
}

#[test]
fn unknown_event_types_are_skipped() {
    let server = ScriptServer::start(
        &socket("unknown-event"),
        &[
            r#"> {"id":1,"hello":{"protocol":2,"client":"test 1.0","subscribe":true,"live":[]}}"#,
            r#"< {"id":1,"ok":{"protocol":2,"daemon":"1.0.0","devices":[],"states":{}}}"#,
            r#"< {"event":"frobnicate","from":"a newer daemon"}"#,
            r#"< {"event":"devices","devices":[]}"#,
            "!close",
        ]
        .join("\n"),
    );
    let (_, _sender, mut rx) = Client::subscribe_to(server.path(), Hello::new("test 1.0")).unwrap();
    assert_eq!(
        next(&mut rx),
        Incoming::Event(Event::Devices { devices: vec![] })
    );
    assert!(matches!(
        rx.next(),
        Some(Err(ClientError::Disconnected(None)))
    ));
    assert!(rx.next().is_none(), "the receiver ends after an error");
    server.finish().unwrap();
}

/// Spec 5.8: a reply that does not come within the reply timeout ends the receiver.
#[test]
fn an_overdue_reply_ends_the_receiver_with_a_timeout() {
    let server = ScriptServer::start(
        &socket("overdue"),
        &[
            r#"> {"id":1,"hello":{"protocol":2,"client":"test 1.0","subscribe":true,"live":[]}}"#,
            r#"< {"id":1,"ok":{"protocol":2,"daemon":"1.0.0","devices":[],"states":{}}}"#,
            r#"> {"id":2,"device":"sn:EXAMPLE0000001","get":null}"#,
        ]
        .join("\n"),
    );
    let (_, mut sender, mut rx) =
        Client::subscribe_to(server.path(), Hello::new("test 1.0")).unwrap();
    sender.set_reply_timeout(Duration::from_millis(200));
    sender.get("sn:EXAMPLE0000001", None).unwrap();
    assert!(matches!(rx.next(), Some(Err(ClientError::Timeout))));
    assert!(rx.next().is_none());
    drop(sender);
    server.finish().unwrap();
}
