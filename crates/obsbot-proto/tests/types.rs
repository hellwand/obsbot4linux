// SPDX-License-Identifier: GPL-3.0-or-later

//! Every protocol type round-trips byte for byte against `tests/golden/types.jsonl`, and the
//! additive-change rules of 4.1 rule 7 hold: unknown enum values, types and codes parse.

use obsbot_proto::*;
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::json;

const GOLDEN: &str = include_str!("golden/types.jsonl");

fn round<T: Serialize + DeserializeOwned>(json: &str) -> T {
    let value: T =
        serde_json::from_str(json).unwrap_or_else(|e| panic!("does not parse: {e}\n{json}"));
    let back = serde_json::to_string(&value).unwrap();
    assert_eq!(back, json, "not byte-equal after a round trip");
    value
}

#[test]
fn every_golden_line_round_trips_byte_equal() {
    let mut seen = std::collections::BTreeSet::new();
    for line in GOLDEN
        .lines()
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
    {
        let (name, json) = line.split_once(' ').expect("type name, space, JSON");
        seen.insert(name);
        match name {
            "RequestId" => drop(round::<RequestId>(json)),
            "Support" => drop(round::<Support>(json)),
            "Unit" => drop(round::<Unit>(json)),
            "UnitSpec" => drop(round::<UnitSpec>(json)),
            "Bound" => drop(round::<Bound>(json)),
            "Asleep" => drop(round::<Asleep>(json)),
            "Warning" => drop(round::<Warning>(json)),
            "SettingType" => drop(round::<SettingType>(json)),
            "ErrorCode" => drop(round::<ErrorCode>(json)),
            "Requirement" => drop(round::<Requirement>(json)),
            "ExtensionUnit" => drop(round::<ExtensionUnit>(json)),
            "Usb" => drop(round::<Usb>(json)),
            "Setting" => drop(round::<Setting>(json)),
            "Action" => drop(round::<Action>(json)),
            "Description" => drop(round::<Description>(json)),
            "Device" => drop(round::<Device>(json)),
            "State" => drop(round::<State>(json)),
            "Patch" => drop(round::<Patch>(json)),
            "Hello" => drop(round::<Hello>(json)),
            "Request" => drop(round::<Request>(json)),
            "Reply" => drop(round::<Reply>(json)),
            "ErrorBody" => drop(round::<ErrorBody>(json)),
            "Event" => drop(round::<Event>(json)),
            "ServerMessage" => drop(round::<ServerMessage>(json)),
            other => panic!("unknown type name {other}"),
        }
    }
    assert_eq!(seen.len(), 24, "every type has at least one golden line");
}

#[test]
fn additive_values_from_a_newer_daemon_still_parse() {
    assert_eq!(round::<Support>(r#""beta""#), Support::Other("beta".into()));
    assert_eq!(
        round::<SettingType>(r#""matrix""#),
        SettingType::Other("matrix".into())
    );
    assert_eq!(round::<Unit>(r#""lux""#), Unit::Other("lux".into()));
    assert_eq!(
        round::<ErrorCode>(r#""rate_limited""#),
        ErrorCode::Other("rate_limited".into())
    );
    let unknown: ServerMessage = serde_json::from_str(r#"{"event":"battery","level":3}"#).unwrap();
    assert_eq!(
        unknown,
        ServerMessage::Event(Event::Unknown),
        "unknown event types are ignorable"
    );
    let reply: Reply =
        serde_json::from_str(r#"{"id":4,"ok":{"rev":6,"extra":true},"trace":"x"}"#).unwrap();
    assert_eq!(
        reply.body,
        ReplyBody::Ok(OkBody::Rev(RevOk {
            rev: 6,
            result: None
        })),
        "unknown fields are ignored"
    );
}

#[test]
fn requests_parse_into_their_verbs() {
    // Review Focus 4: a newer client's extra fields are ignored, not refused.
    let extra = Request::from_line(r#"{"id":9,"device":"d","get":null,"trace":"abc"}"#).unwrap();
    assert_eq!(
        extra.verb,
        Verb::Get {
            device: "d".into(),
            paths: None
        }
    );
    let get = Request::from_line(r#"{"id":2,"device":"sn:EXAMPLE0000001","get":null}"#).unwrap();
    assert_eq!(
        get.verb,
        Verb::Get {
            device: "sn:EXAMPLE0000001".into(),
            paths: None
        },
        "get:null is the full state"
    );
    let stop = Request::from_line(r#"{"id":6,"device":"d","do":"gimbal.stop"}"#).unwrap();
    assert_eq!(
        stop.verb,
        Verb::Do {
            device: "d".into(),
            action: "gimbal.stop".into(),
            params: Params::new()
        },
        "with defaults to {{}}"
    );
    let hello: Request =
        serde_json::from_str(r#"{"id":1,"hello":{"protocol":2,"client":"x"}}"#).unwrap();
    assert_eq!(
        hello.verb,
        Verb::Hello(Hello {
            protocol: 2,
            client: "x".into(),
            subscribe: true,
            live: vec![]
        }),
        "subscribe defaults to true on the wire"
    );
    assert_eq!(
        serde_json::to_value(Hello::new("cli 1.0")).unwrap(),
        json!({"protocol":2,"client":"cli 1.0","subscribe":false,"live":[]})
    );
}

#[test]
fn malformed_requests_are_refused_with_their_id_when_it_is_usable() {
    let cases = [
        (r#"{"id":1}"#, Some(RequestId::Int(1)), "no verb"),
        (
            r#"{"id":1,"device":"d","get":null,"set":{}}"#,
            Some(RequestId::Int(1)),
            "more than one verb",
        ),
        (r#"{"device":"d","get":null}"#, None, "no id"),
        (
            r#"{"id":"x","get":null}"#,
            Some(RequestId::Str("x".into())),
            "names no device",
        ),
        (
            r#"{"id":2,"device":"d","set":[1]}"#,
            Some(RequestId::Int(2)),
            "malformed",
        ),
        (r#"{"id":1.5,"live":[]}"#, None, "malformed"),
        ("not json", None, "not JSON"),
        (r#"[1,2]"#, None, "malformed"),
    ];
    for (line, id, part) in cases {
        let err = Request::from_line(line).unwrap_err();
        assert_eq!(err.id, id, "{line}");
        assert!(err.message.contains(part), "{line}: {}", err.message);
    }
}
