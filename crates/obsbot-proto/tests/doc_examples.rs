// SPDX-License-Identifier: GPL-3.0-or-later

//! The worked transcripts and the client example of `docs/protocol-2.md` (sections 11 and 12)
//! stay true: every line of a `jsonl` block round-trips through the protocol types byte for
//! byte, as the golden transcripts do; a state event comes before the reply that carries its
//! rev (4.1 rule 3); and the `rust` block is the example `examples/watch.rs`, which the build
//! compiles.

use obsbot_proto::*;
use std::collections::BTreeMap;
use std::path::PathBuf;

fn doc() -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs/protocol-2.md");
    std::fs::read_to_string(path).unwrap()
}

/// The fenced blocks of one language, in order.
fn blocks(text: &str, lang: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current: Option<String> = None;
    for line in text.lines() {
        match current.as_mut() {
            None if line == format!("```{lang}") => current = Some(String::new()),
            Some(_) if line == "```" => out.push(current.take().unwrap()),
            Some(block) => {
                block.push_str(line);
                block.push('\n');
            }
            None => {}
        }
    }
    out
}

#[test]
fn every_transcript_line_round_trips_and_events_come_before_their_replies() {
    let text = doc();
    let blocks = blocks(&text, "jsonl");
    assert!(!blocks.is_empty(), "section 11 has a jsonl block");
    let mut lines = 0;
    for block in blocks {
        // the device of each request, and the revs of the state events seen so far
        let mut device_of: BTreeMap<String, String> = BTreeMap::new();
        let mut seen: BTreeMap<String, Vec<u64>> = BTreeMap::new();
        for line in block.lines() {
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            lines += 1;
            if let Some(json) = line.strip_prefix("> ") {
                let req = Request::from_line(json).unwrap_or_else(|e| panic!("{line}: {e}"));
                assert_eq!(serde_json::to_string(&req).unwrap(), json, "not byte-equal");
                let v: serde_json::Value = serde_json::from_str(json).unwrap();
                if let (Some(id), Some(device)) = (v.get("id"), v["device"].as_str()) {
                    device_of.insert(id.to_string(), device.to_string());
                }
            } else if let Some(json) = line.strip_prefix("< ") {
                let msg: ServerMessage =
                    serde_json::from_str(json).unwrap_or_else(|e| panic!("{line}: {e}"));
                assert_eq!(serde_json::to_string(&msg).unwrap(), json, "not byte-equal");
                let v: serde_json::Value = serde_json::from_str(json).unwrap();
                if v["event"] == "state" {
                    let device = v["device"].as_str().unwrap().to_string();
                    seen.entry(device)
                        .or_default()
                        .push(v["rev"].as_u64().unwrap());
                } else if let (Some(rev), Some(device), None) = (
                    v["ok"]["rev"].as_u64(),
                    device_of.get(&v["id"].to_string()),
                    v["ok"].get("state"),
                ) {
                    assert!(
                        seen.get(device).is_some_and(|revs| revs.contains(&rev)),
                        "{line}: no state event with rev {rev} before the reply"
                    );
                }
            } else {
                panic!("{line}: a transcript line starts with '> ', '< ' or '#'");
            }
        }
    }
    assert!(lines >= 10, "the transcripts hold {lines} lines");
}

#[test]
fn the_client_example_is_the_one_the_build_compiles() {
    let text = doc();
    let rust = blocks(&text, "rust");
    let example = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/watch.rs");
    let example = std::fs::read_to_string(example).unwrap();
    assert_eq!(
        rust,
        [example],
        "section 12 shows examples/watch.rs as it is"
    );
}
