// SPDX-License-Identifier: GPL-3.0-or-later

//! Golden protocol transcripts (`tests/golden/*.jsonl` except `types.jsonl`): `> ` lines go
//! from a client to the daemon, `< ` lines from the daemon to a client, `#` lines are
//! comments. Every line round-trips byte for byte, and the ordering rules of 4.1 that a
//! transcript can show hold in each file. Tracks C and D build against these files.

use obsbot_proto::*;
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

fn transcripts() -> Vec<(String, String)> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden");
    let mut files: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == "jsonl") && !p.ends_with("types.jsonl"))
        .collect();
    files.sort();
    files
        .into_iter()
        .map(|p| {
            (
                p.file_name().unwrap().to_string_lossy().into_owned(),
                std::fs::read_to_string(&p).unwrap(),
            )
        })
        .collect()
}

/// The daemon's lines of one transcript, parsed, after checking every line's round trip.
fn parse(name: &str, text: &str) -> Vec<ServerMessage> {
    let mut out = Vec::new();
    for (n, line) in text.lines().enumerate() {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let at = format!("{name}:{}", n + 1);
        if let Some(json) = line.strip_prefix("> ") {
            let req = Request::from_line(json).unwrap_or_else(|e| panic!("{at}: {e}"));
            assert_eq!(
                serde_json::to_string(&req).unwrap(),
                json,
                "{at}: not byte-equal"
            );
        } else if let Some(json) = line.strip_prefix("< ") {
            let msg: ServerMessage =
                serde_json::from_str(json).unwrap_or_else(|e| panic!("{at}: {e}"));
            assert_eq!(
                serde_json::to_string(&msg).unwrap(),
                json,
                "{at}: not byte-equal"
            );
            out.push(msg);
        } else {
            panic!("{at}: a line starts with '> ', '< ' or '#'");
        }
    }
    out
}

#[test]
fn the_three_transcripts_exist() {
    let names: Vec<String> = transcripts().into_iter().map(|(n, _)| n).collect();
    assert_eq!(names, ["errors.jsonl", "hotplug.jsonl", "session.jsonl"]);
}

#[test]
fn every_transcript_line_round_trips_byte_equal() {
    for (name, text) in transcripts() {
        parse(&name, &text);
    }
}

#[test]
fn the_error_transcript_holds_every_error_code() {
    let (_, text) = transcripts()
        .into_iter()
        .find(|(n, _)| n == "errors.jsonl")
        .unwrap();
    let codes: Vec<ErrorCode> = parse("errors.jsonl", &text)
        .into_iter()
        .map(|m| match m {
            ServerMessage::Reply(Reply {
                body: ReplyBody::Error(e),
                ..
            }) => e.code,
            other => panic!("errors.jsonl holds only error replies, not {other:?}"),
        })
        .collect();
    assert_eq!(codes, ErrorCode::ALL.to_vec());
}

/// 4.1 rule 1 and Events: a rev only goes up per entry; a state event names a listed device;
/// a client forgets an id's rev when a devices event no longer lists it.
#[test]
fn revs_and_device_lists_follow_the_ordering_rules() {
    for (name, text) in transcripts() {
        let mut listed: Option<BTreeSet<String>> = None;
        let mut revs: BTreeMap<String, u64> = BTreeMap::new();
        for msg in parse(&name, &text) {
            match msg {
                ServerMessage::Reply(Reply {
                    body: ReplyBody::Ok(OkBody::Hello(h)),
                    ..
                }) => {
                    listed = Some(h.devices.iter().map(|d| d.id.clone()).collect());
                    revs = h.states.iter().map(|(id, s)| (id.clone(), s.rev)).collect();
                }
                ServerMessage::Event(Event::Devices { devices }) => {
                    let ids: BTreeSet<String> = devices.iter().map(|d| d.id.clone()).collect();
                    revs.retain(|id, _| ids.contains(id));
                    listed = Some(ids);
                }
                ServerMessage::Event(Event::State { device, rev, .. }) => {
                    if let Some(ids) = &listed {
                        assert!(
                            ids.contains(&device),
                            "{name}: state of {device}, which no list holds"
                        );
                    }
                    if let Some(old) = revs.get(&device) {
                        assert!(
                            rev > *old,
                            "{name}: rev of {device} went from {old} to {rev}"
                        );
                    }
                    revs.insert(device, rev);
                }
                _ => {}
            }
        }
    }
}

/// 5.5: the hand-over is devices (old entry with replaced_by), devices (without it), state.
#[test]
fn the_hand_over_is_three_events_in_order() {
    let (_, text) = transcripts()
        .into_iter()
        .find(|(n, _)| n == "hotplug.jsonl")
        .unwrap();
    let msgs = parse("hotplug.jsonl", &text);
    let at = msgs
        .iter()
        .position(|m| matches!(m, ServerMessage::Event(Event::Devices { devices }) if devices.iter().any(|d| d.replaced_by.is_some())))
        .expect("a devices event with replaced_by");
    let ServerMessage::Event(Event::Devices { devices: first }) = &msgs[at] else {
        unreachable!()
    };
    let old = first.iter().find(|d| d.replaced_by.is_some()).unwrap();
    assert!(!old.available);
    let new_id = old.replaced_by.clone().unwrap();
    assert!(first.iter().any(|d| d.id == new_id && d.available));
    let ServerMessage::Event(Event::Devices { devices: second }) = &msgs[at + 1] else {
        panic!("devices event expected")
    };
    assert!(second.iter().all(|d| d.id != old.id));
    assert!(
        matches!(&msgs[at + 2], ServerMessage::Event(Event::State { device, .. }) if *device == new_id)
    );
}

/// Public text (spec 8.5): a golden file may name the Tiny 2 Lite's own USB id (`fef9`) and
/// otherwise only synthetic product ids of the fakes' range `fa0x`, never another real model's
/// id. Covers `types.jsonl` too, as `"product":"<id>"` and in a name as `3564:<id>`.
#[test]
fn golden_files_name_no_other_real_product_id() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden");
    let allowed = |id: &str| id == "fef9" || (id.len() == 4 && id.starts_with("fa0"));
    let mut seen = 0;
    for entry in std::fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        let text = std::fs::read_to_string(&path).unwrap();
        for marker in ["\"product\":\"", "3564:"] {
            for rest in text.split(marker).skip(1) {
                let id: String = rest.chars().take_while(|c| c.is_ascii_hexdigit()).collect();
                assert!(
                    allowed(&id),
                    "{}: product id {id} is neither fef9 nor fa0x",
                    path.display()
                );
                seen += 1;
            }
        }
    }
    assert!(seen > 0, "the golden files name product ids");
}
