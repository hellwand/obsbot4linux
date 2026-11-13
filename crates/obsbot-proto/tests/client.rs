// SPDX-License-Identifier: GPL-3.0-or-later

//! The blocking client (spec 6.1) against a scripted daemon: the exact lines it sends, the
//! replies it returns, and how it reports an older daemon, a protocol mismatch, a refusal,
//! a lost connection and a missing reply (4.1, 5.8, 5.10).

use obsbot_proto::client::script::ScriptServer;
use obsbot_proto::client::{Client, ClientError, Connection};
use obsbot_proto::{ErrorCode, Hello, PROTOCOL, Patch};
use serde_json::json;
use std::path::PathBuf;
use std::time::Duration;

fn socket(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("c1-{name}.sock"))
}

const HELLO: &str =
    r#"> {"id":1,"hello":{"protocol":2,"client":"test 1.0","subscribe":false,"live":[]}}"#;
const HELLO_OK: &str =
    r#"< {"id":1,"ok":{"protocol":2,"daemon":"1.0.0","devices":[],"states":{}}}"#;

fn script(lines: &[&str]) -> String {
    lines.join("\n")
}

#[test]
fn the_blocking_client_sends_one_line_per_call_and_returns_the_reply() {
    let server = ScriptServer::start(
        &socket("calls"),
        &script(&[
            HELLO,
            HELLO_OK,
            r#"> {"id":2,"device":"sn:EXAMPLE0000001","get":["led"]}"#,
            r#"< {"id":2,"ok":{"rev":6,"state":{"available":true,"pending":{},"unread":[],"led":{"brightness":2}}}}"#,
            r#"> {"id":3,"device":"sn:EXAMPLE0000001","get":null}"#,
            r#"< {"id":3,"ok":{"rev":6,"state":{"available":true,"pending":{},"unread":[]}}}"#,
            r#"> {"id":4,"device":"sn:EXAMPLE0000001","set":{"led.brightness":2}}"#,
            r#"< {"id":4,"ok":{"rev":7}}"#,
            r#"> {"id":5,"device":"sn:EXAMPLE0000001","do":"view.save","with":{"name":"Desk"}}"#,
            r#"< {"id":5,"ok":{"rev":8,"result":{"id":3}}}"#,
            r#"> {"id":6,"device":"sn:EXAMPLE0000001","do":"gimbal.stop"}"#,
            r#"< {"id":6,"ok":{"rev":8}}"#,
        ]),
    );
    // `connect_to` sends subscribe:false even when the hello asks for events.
    let mut c = Client::connect_to(server.path(), Hello::new("test 1.0").subscribe(true)).unwrap();
    assert_eq!(c.hello().protocol, PROTOCOL);
    assert!(c.devices().is_empty());
    let (rev, state) = c.get("sn:EXAMPLE0000001", Some(&["led"])).unwrap();
    assert_eq!((rev, state.at("led.brightness")), (6, Some(&json!(2))));
    let (rev, _) = c.get("sn:EXAMPLE0000001", None).unwrap();
    assert_eq!(rev, 6);
    let rev = c
        .set("sn:EXAMPLE0000001", Patch::new().leaf("led.brightness", 2))
        .unwrap();
    assert_eq!(rev, 7);
    let (rev, result) = c
        .act("sn:EXAMPLE0000001", "view.save", json!({"name": "Desk"}))
        .unwrap();
    assert_eq!(rev, 8);
    assert_eq!(result.unwrap()["id"], json!(3));
    let (rev, result) = c
        .act("sn:EXAMPLE0000001", "gimbal.stop", serde_json::Value::Null)
        .unwrap();
    assert_eq!((rev, result), (8, None));
    drop(c);
    assert_eq!(server.finish().unwrap().len(), 6);
}

#[test]
fn events_on_the_connection_are_skipped_while_a_reply_is_awaited() {
    let server = ScriptServer::start(
        &socket("events"),
        &script(&[
            HELLO,
            HELLO_OK,
            r#"> {"id":2,"device":"sn:EXAMPLE0000001","set":{"led.brightness":2}}"#,
            r#"< {"event":"state","device":"sn:EXAMPLE0000001","rev":5,"state":{"available":true,"pending":{"led.brightness":2},"unread":[]}}"#,
            r#"< {"event":"frobnicate","from":"a newer daemon"}"#,
            r#"< {"id":2,"ok":{"rev":5}}"#,
        ]),
    );
    let mut c = Client::connect_to(server.path(), Hello::new("test 1.0")).unwrap();
    let rev = c
        .set("sn:EXAMPLE0000001", Patch::new().leaf("led.brightness", 2))
        .unwrap();
    assert_eq!(rev, 5);
    drop(c);
    server.finish().unwrap();
}

#[test]
fn a_refusal_carries_the_daemons_error_body() {
    let server = ScriptServer::start(
        &socket("refused"),
        &script(&[
            HELLO,
            HELLO_OK,
            r#"> {"id":2,"device":"sn:EXAMPLE0000001","set":{"exposure.mode":"manual","exposure.time":20000}}"#,
            r#"< {"id":2,"error":{"code":"camera","message":"the camera did not answer","rev":9,"applied":["exposure.mode"],"failed":"exposure.time"}}"#,
            r#"> {"id":3,"device":"sn:EXAMPLE0000001","get":null}"#,
            r#"< {"id":3,"error":{"code":"teapot","message":"a code from a newer daemon"}}"#,
        ]),
    );
    let mut c = Client::connect_to(server.path(), Hello::new("test 1.0")).unwrap();
    let patch = Patch::new()
        .leaf("exposure.mode", "manual")
        .leaf("exposure.time", 20000);
    match c.set("sn:EXAMPLE0000001", patch) {
        Err(ClientError::Refused(body)) => {
            assert_eq!(body.code, ErrorCode::Camera);
            assert_eq!(body.rev, Some(9));
            assert_eq!(body.applied, Some(vec!["exposure.mode".to_string()]));
            assert_eq!(body.failed.as_deref(), Some("exposure.time"));
        }
        other => panic!("{other:?}"),
    }
    match c.get("sn:EXAMPLE0000001", None) {
        Err(ClientError::Refused(body)) => {
            assert_eq!(body.code, ErrorCode::Other("teapot".into()));
        }
        other => panic!("{other:?}"),
    }
    drop(c);
    server.finish().unwrap();
}

/// Spec 5.10: a 0.13 daemon answers the hello line with `ok:false` and a string `error`.
#[test]
fn a_0_13_daemon_is_reported_as_older_than_1_0() {
    let server = ScriptServer::start(
        &socket("old"),
        &script(&[
            HELLO,
            r#"< {"id":null,"ok":false,"error":"bad request: missing field `args` at line 1 column 83"}"#,
        ]),
    );
    let got = Client::connect_to(server.path(), Hello::new("test 1.0"));
    assert!(
        matches!(got, Err(ClientError::OldDaemon)),
        "{:?}",
        got.err()
    );
    server.finish().unwrap();
}

#[test]
fn a_daemon_of_another_protocol_is_reported_with_its_numbers() {
    let server = ScriptServer::start(
        &socket("protocol"),
        &script(&[
            HELLO,
            r#"< {"id":null,"error":{"code":"protocol","message":"protocol 2 is not supported","supported":[3]}}"#,
            "!close",
        ]),
    );
    match Client::connect_to(server.path(), Hello::new("test 1.0")) {
        Err(ClientError::Protocol { supported, .. }) => assert_eq!(supported, vec![3]),
        other => panic!("{:?}", other.err()),
    }
    server.finish().unwrap();
}

#[test]
fn no_socket_is_a_connect_error_naming_the_path() {
    let path = socket("missing");
    let _ = std::fs::remove_file(&path);
    match Client::connect_to(&path, Hello::new("test 1.0")) {
        Err(ClientError::Connect { path: p, .. }) => assert_eq!(p, path),
        other => panic!("{:?}", other.err()),
    }
}

#[test]
fn a_closed_connection_ends_the_call() {
    let server = ScriptServer::start(&socket("closed"), &script(&[HELLO, HELLO_OK, "!close"]));
    let mut c = Client::connect_to(server.path(), Hello::new("test 1.0")).unwrap();
    server.finish().unwrap();
    let got = c.get("sn:EXAMPLE0000001", None);
    assert!(matches!(got, Err(ClientError::Disconnected(_))), "{got:?}");
}

/// Spec 5.8: no reply within the reply timeout is reported, not waited for forever.
#[test]
fn a_missing_reply_times_out() {
    let server = ScriptServer::start(
        &socket("timeout"),
        &script(&[
            HELLO,
            HELLO_OK,
            r#"> {"id":2,"device":"sn:EXAMPLE0000001","get":null}"#,
        ]),
    );
    let mut c = Client::connect_to(server.path(), Hello::new("test 1.0")).unwrap();
    c.set_reply_timeout(Duration::from_millis(200));
    let got = c.get("sn:EXAMPLE0000001", None);
    assert!(matches!(got, Err(ClientError::Timeout)), "{got:?}");
    drop(c);
    server.finish().unwrap();
}

#[test]
fn a_hello_without_reply_times_out() {
    let server = ScriptServer::start(&socket("hello-timeout"), &script(&[HELLO]));
    let mut conn = Connection::open(server.path()).unwrap();
    conn.set_reply_timeout(Duration::from_millis(200));
    let got = conn.hello(Hello::new("test 1.0"));
    assert!(matches!(got, Err(ClientError::Timeout)), "{:?}", got.err());
    drop(conn);
    server.finish().unwrap();
}

#[test]
fn action_parameters_must_be_an_object_and_nothing_is_sent_otherwise() {
    let server = ScriptServer::start(&socket("not-object"), &script(&[HELLO, HELLO_OK]));
    let mut c = Client::connect_to(server.path(), Hello::new("test 1.0")).unwrap();
    let got = c.act("sn:EXAMPLE0000001", "preset.recall", json!([1]));
    assert!(matches!(got, Err(ClientError::NotAnObject)), "{got:?}");
    drop(c);
    assert_eq!(server.finish().unwrap().len(), 1, "only hello was sent");
}

/// Spec 4.1: lines from the daemon have no length limit, and clients must not cap them.
#[test]
fn a_long_line_from_the_daemon_is_read_whole() {
    let long = "x".repeat(200 * 1024);
    let reply = format!(
        r#"< {{"id":1,"ok":{{"protocol":2,"daemon":"{long}","devices":[],"states":{{}}}}}}"#
    );
    let server = ScriptServer::start(&socket("long"), &script(&[HELLO, &reply]));
    let c = Client::connect_to(server.path(), Hello::new("test 1.0")).unwrap();
    assert_eq!(c.hello().daemon.len(), long.len());
    drop(c);
    server.finish().unwrap();
}

#[test]
fn a_reply_to_another_request_is_malformed() {
    let server = ScriptServer::start(
        &socket("other-id"),
        &script(&[
            HELLO,
            HELLO_OK,
            r#"> {"id":2,"device":"sn:EXAMPLE0000001","get":null}"#,
            r#"< {"id":7,"ok":{"rev":1,"state":{"available":true,"pending":{},"unread":[]}}}"#,
        ]),
    );
    let mut c = Client::connect_to(server.path(), Hello::new("test 1.0")).unwrap();
    let got = c.get("sn:EXAMPLE0000001", None);
    assert!(matches!(got, Err(ClientError::Malformed(_))), "{got:?}");
    drop(c);
    server.finish().unwrap();
}

/// The scripted daemon itself: a client that sends one request more than the script holds
/// fails the script (the CLI's one-request tests rely on it).
#[test]
fn the_script_catches_an_extra_request() {
    let server = ScriptServer::start(&socket("extra"), &script(&[HELLO, HELLO_OK]));
    let mut c = Client::connect_to(server.path(), Hello::new("test 1.0")).unwrap();
    c.set_reply_timeout(Duration::from_millis(100));
    let _ = c.get("sn:EXAMPLE0000001", None);
    drop(c);
    let err = server.finish().unwrap_err();
    assert!(err.contains("one line more"), "{err}");
}
