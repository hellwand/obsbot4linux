// SPDX-License-Identifier: GPL-3.0-or-later

//! The evidence-level test of design spec 8.5: every field, value and bound of every
//! registered model has the level `captured` or `measured` (`daemon` for what is not a camera
//! value). Before 1.0 a mark of spec 9.3 may stand, listed in the model's `unbacked`; from the
//! 1.0 release candidate on (workspace version 1 or later), and for a `tested` model at any
//! version, none may. The levels follow the field ledger, every mark of 9.3 has a row of level
//! `none` (and the other way round) and agrees with the ledger's column §9.3, and the UVC
//! bounds and defaults of the Tiny 2 Lite equal the camera's range answers recorded in
//! captures 01, 14, 16 and 18 (the fixtures below cite 01 and 16).

use obsbot_device::contract::{ControlRange, ModelEntry, Transport};
use obsbot_device::evidence::{self, Evidence, Level, Row};
use obsbot_device::models::tiny2lite::{decode, levels, plan};
use obsbot_device::registry::REGISTRY;
use obsbot_device::transport::replay::Replay;
use obsbot_proto::{Description, Support};
use std::collections::{BTreeMap, BTreeSet};

/// The rule of spec 8.5 holds without exception from the 1.0 release candidate on.
fn strict() -> bool {
    !env!("CARGO_PKG_VERSION").starts_with("0.")
}

fn description_of(key: &str) -> Description {
    let path = format!("descriptions/{key}.json");
    obsbot_device::descriptions::all()
        .into_iter()
        .find(|(p, _)| *p == path)
        .map(|(_, d)| d)
        .unwrap_or_else(|| panic!("{key}: no {path} in descriptions::all"))
}

#[test]
fn every_registered_model_meets_the_evidence_rule() {
    assert!(!REGISTRY.models.is_empty());
    for entry in REGISTRY.models {
        let e = evidence::of(entry.key)
            .unwrap_or_else(|| panic!("{}: no evidence in evidence::of", entry.key));
        let problems = evidence::check(entry, &description_of(entry.key), &e, strict());
        assert!(
            problems.is_empty(),
            "{} (support {:?}, strict {}):\n{}",
            entry.key,
            entry.support,
            strict(),
            problems.join("\n")
        );
    }
}

/// The Tiny 2 Lite's evidence with one mark of 9.3 put back (CS-1 left none): the U-8 mark of
/// the area order, with its row of level none, so that the two tests below have a mark to fail.
fn with_a_mark() -> Evidence {
    let mut rows = levels::LEVELS.to_vec();
    rows.push(Row {
        key: "zone.area.pan#order",
        level: Level::None,
        evidence: "none",
    });
    Evidence {
        levels: Box::leak(rows.into_boxed_slice()),
        unbacked: &[("zone.area.pan#order", "U-8")],
        ..levels::EVIDENCE
    }
}

/// From 1.0 on, every mark of 9.3 still in the description fails, and nothing else does.
#[test]
fn strict_fails_exactly_the_marks_of_9_3() {
    let entry = &plan::ENTRY;
    let e = with_a_mark();
    let problems = evidence::check(entry, &description_of("tiny2lite"), &e, true);
    let want: Vec<String> = e
        .unbacked
        .iter()
        .map(|(k, m)| format!("{k}: {m} of spec 9.3 has no backing"))
        .collect();
    assert_eq!(problems, want);
}

/// Setting the support level to `tested` (stage W3) fails while a mark of 9.3 stands.
#[test]
fn a_tested_model_may_keep_no_mark_of_9_3() {
    let tested = ModelEntry {
        support: Support::Tested,
        ..plan::ENTRY
    };
    let e = with_a_mark();
    let problems = evidence::check(&tested, &description_of("tiny2lite"), &e, false);
    assert_eq!(problems.len(), 1, "{problems:#?}");
    assert!(
        problems
            .iter()
            .all(|p| p.contains("of spec 9.3 has no backing"))
    );
}

#[test]
fn a_missing_wrong_or_stray_row_fails() {
    let d = description_of("tiny2lite");
    // untested and before 1.0, so that only the broken row shows
    let entry = &ModelEntry {
        support: Support::Untested,
        ..plan::ENTRY
    };
    let all = levels::LEVELS;
    let with = |rows: Vec<Row>| Evidence {
        levels: Box::leak(rows.into_boxed_slice()),
        ..levels::EVIDENCE
    };

    let without_led: Vec<Row> = all
        .iter()
        .copied()
        .filter(|r| r.key != "led.brightness")
        .collect();
    let p = evidence::check(entry, &d, &with(without_led), false);
    assert_eq!(p, ["led.brightness: no level"]);

    let mut wrong: Vec<Row> = all.to_vec();
    let i = wrong
        .iter()
        .position(|r| r.key == "led.brightness")
        .unwrap();
    wrong[i].level = Level::Measured; // its evidence starts with "capture 07"
    let p = evidence::check(entry, &d, &with(wrong), false);
    assert_eq!(p.len(), 1, "{p:?}");
    assert!(p[0].starts_with("led.brightness: level Measured does not fit"));

    let mut unbacked: Vec<Row> = all.to_vec();
    unbacked[i] = Row {
        key: "led.brightness",
        level: Level::None,
        evidence: "none",
    };
    let p = evidence::check(entry, &d, &with(unbacked), false);
    assert_eq!(p, ["led.brightness: no backing and no mark of 9.3"]);

    let mut stray: Vec<Row> = all.to_vec();
    stray.push(Row {
        key: "led.colour",
        level: Level::Captured,
        evidence: "capture 07",
    });
    let p = evidence::check(entry, &d, &with(stray), false);
    assert_eq!(
        p,
        ["led.colour: not a field, value or bound of the description"]
    );

    // a mark of 9.3 on an aspect needs its row of level none, and a row of level none a mark;
    // CS-1 left no mark, so the check runs on one put in for the test (the U-8 mark it had)
    let marked = |rows: Vec<Row>| Evidence {
        levels: Box::leak(rows.into_boxed_slice()),
        unbacked: &[("zone.area.pan#order", "U-8")],
        ..levels::EVIDENCE
    };
    let mut with_row: Vec<Row> = all.to_vec();
    with_row.push(Row {
        key: "zone.area.pan#order",
        level: Level::None,
        evidence: "none",
    });
    let p = evidence::check(entry, &d, &marked(with_row.clone()), false);
    assert!(p.is_empty(), "{p:?}");
    let p = evidence::check(entry, &d, &marked(all.to_vec()), false);
    assert_eq!(
        p,
        ["zone.area.pan#order (U-8): a mark of 9.3 without a row of level none"]
    );
    let mut inherited = with_row;
    let last = inherited.len() - 1;
    inherited[last] = Row {
        key: "zone.area.pan#order",
        level: Level::Captured,
        evidence: "capture 11",
    };
    let p = evidence::check(entry, &d, &marked(inherited), false);
    assert_eq!(
        p,
        ["zone.area.pan#order (U-8): a mark of 9.3, but its row has level Captured, not none"]
    );
    let mut no_mark: Vec<Row> = all.to_vec();
    no_mark.push(Row {
        key: "exposure.anti_flicker=50hz",
        level: Level::None,
        evidence: "none",
    });
    let p = evidence::check(entry, &d, &with(no_mark), false);
    assert_eq!(
        p,
        ["exposure.anti_flicker=50hz: no backing and no mark of 9.3"]
    );

    let mut daemon: Vec<Row> = all.to_vec();
    daemon[i] = Row {
        key: "led.brightness",
        level: Level::Daemon,
        evidence: "daemon",
    };
    let p = evidence::check(entry, &d, &with(daemon), false);
    assert_eq!(p.len(), 1, "{p:?}");
    assert!(p[0].starts_with("led.brightness: level Daemon does not fit"));
}

/// Splits one Markdown table row at its unescaped pipes.
fn cells(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cell = String::new();
    let mut chars = line.trim().chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' if chars.peek() == Some(&'|') => {
                cell.push('|');
                chars.next();
            }
            '|' => out.push(std::mem::take(&mut cell).trim().to_string()),
            _ => cell.push(c),
        }
    }
    out.remove(0);
    out
}

/// The row of every field carries the evidence column of `docs/field-ledger.md` word for
/// word, so that a change of the ledger shows here and the other way round.
#[test]
fn the_field_rows_follow_the_ledger() {
    let ledger = include_str!("../../../docs/field-ledger.md");
    let mut header: Option<Vec<String>> = None;
    let mut seen = 0;
    for line in ledger.lines() {
        if line.starts_with("| Path") || line.starts_with("| Action") {
            header = Some(cells(line));
            continue;
        }
        let Some(h) = &header else { continue };
        if !line.starts_with("| `") {
            continue;
        }
        let c = cells(line);
        let name = c[0].split('`').nth(1).unwrap();
        let key = if h[0] == "Action" {
            format!("do:{name}")
        } else {
            name.to_string()
        };
        if ["available", "pending", "unread"].contains(&key.as_str()) {
            continue;
        }
        let evidence = &c[h.iter().position(|x| x == "Evidence").unwrap()];
        let row = levels::LEVELS
            .iter()
            .find(|r| r.key == key)
            .unwrap_or_else(|| panic!("{key}: in the ledger, not in LEVELS"));
        assert_eq!(row.evidence, evidence, "{key}");
        seen += 1;
    }
    let d = description_of("tiny2lite");
    assert_eq!(seen, d.settings.len() + d.actions.len());
}

/// The marks of spec 9.3 are the ledger's: per field, the `U-n` of its §9.3 column equal the
/// marks of `UNBACKED` that belong to the field, and a ledger evidence cell that says `none`
/// for a value or bound comes with such a mark. A mark that the ledger does not carry, or a
/// ledger mark that `UNBACKED` lacks, fails here.
#[test]
fn the_marks_of_9_3_follow_the_ledger() {
    let ledger = include_str!("../../../docs/field-ledger.md");
    let d = description_of("tiny2lite");
    let mut from_ledger: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut says_none: BTreeSet<String> = BTreeSet::new();
    let mut header: Option<Vec<String>> = None;
    for line in ledger.lines() {
        if line.starts_with("| Path") || line.starts_with("| Action") {
            header = Some(cells(line));
            continue;
        }
        let Some(h) = &header else { continue };
        if !line.starts_with("| `") {
            continue;
        }
        let c = cells(line);
        let name = c[0].split('`').nth(1).unwrap();
        let key = if h[0] == "Action" {
            format!("do:{name}")
        } else {
            name.to_string()
        };
        let column = |title: &str| &c[h.iter().position(|x| x == title).unwrap()];
        let marks: BTreeSet<String> = column("§9.3")
            .split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '-'))
            .filter(|w| {
                w.strip_prefix("U-")
                    .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
            })
            .map(str::to_string)
            .collect();
        if !marks.is_empty() {
            from_ledger.insert(key.clone(), marks);
        }
        if column("Evidence")
            .split(|ch: char| !ch.is_ascii_alphabetic())
            .any(|w| w == "none")
        {
            says_none.insert(key);
        }
    }
    let mut from_model: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (key, mark) in levels::EVIDENCE.unbacked {
        let field = evidence::field_of(key, &d).unwrap_or_else(|| panic!("{key}: no field"));
        from_model
            .entry(field)
            .or_default()
            .insert((*mark).to_string());
    }
    assert_eq!(
        from_model, from_ledger,
        "UNBACKED (left) against the ledger's column §9.3 (right)"
    );
    for field in &says_none {
        assert!(
            from_model.contains_key(field),
            "{field}: the ledger's evidence says none, but UNBACKED has no mark for it"
        );
    }
}

/// One UVC control's range answers as recorded: GET_MIN, GET_MAX, GET_RES, GET_DEF, each the
/// little-endian bytes the camera answered (spec 8.5 fixtures: single exchanges).
struct Recorded {
    leaf: &'static str,
    capture: &'static str,
    answers: [&'static str; 4],
}

const RECORDED: [Recorded; 11] = [
    Recorded {
        leaf: "exposure.time",
        capture: "capture 01",
        answers: ["01000000", "c4090000", "01000000", "4a010000"],
    },
    Recorded {
        leaf: "exposure.gain",
        capture: "capture 01",
        answers: ["0100", "4000", "0100", "0100"],
    },
    Recorded {
        leaf: "exposure.compensation",
        capture: "capture 01",
        answers: ["0000", "1200", "0100", "0900"],
    },
    Recorded {
        leaf: "image.brightness",
        capture: "capture 01",
        answers: ["0000", "6400", "0100", "3200"],
    },
    Recorded {
        leaf: "image.contrast",
        capture: "capture 01",
        answers: ["0000", "6400", "0100", "3200"],
    },
    Recorded {
        leaf: "image.saturation",
        capture: "capture 01",
        answers: ["0000", "6400", "0100", "3200"],
    },
    Recorded {
        leaf: "image.hue",
        capture: "capture 01",
        answers: ["0000", "6400", "0100", "3200"],
    },
    Recorded {
        leaf: "image.sharpness",
        capture: "capture 01",
        answers: ["0000", "6400", "0100", "3200"],
    },
    Recorded {
        leaf: "image.white_balance.temperature",
        capture: "capture 01",
        answers: ["d007", "1027", "6400", "8813"],
    },
    Recorded {
        leaf: "focus.position",
        capture: "capture 16",
        answers: ["0000", "6400", "0100", "0000"],
    },
    // a boolean: the recordings hold only its GET_DEF; min, max and step are V4L2's 0, 1, 1
    Recorded {
        leaf: "image.white_balance.auto",
        capture: "capture 16",
        answers: ["", "", "", "01"],
    },
];

/// The control id the model reads a leaf's range from: its own table, not a copy of it here.
fn control(leaf: &str) -> u32 {
    if leaf == "focus.position" {
        return decode::FOCUS_ABSOLUTE;
    }
    decode::UVC
        .iter()
        .find(|(l, _)| *l == leaf)
        .unwrap_or_else(|| panic!("{leaf}: not a UVC leaf of the model"))
        .1
}

fn le(hex: &str) -> Option<i32> {
    if hex.is_empty() {
        return None;
    }
    let b: Vec<u8> = (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
        .collect();
    Some(match b.len() {
        1 => i32::from(b[0]),
        2 => i32::from(i16::from_le_bytes([b[0], b[1]])),
        4 => i32::from_le_bytes([b[0], b[1], b[2], b[3]]),
        n => panic!("a {n}-byte answer"),
    })
}

/// The replay transport answers the range requests with the recorded answers, as the camera
/// did, and the description's bounds, steps and defaults equal them: the comparison of the live
/// test `live_the_uvc_ranges_the_description_holds` (tests/live.rs), made on the recorded data
/// of two firmware versions. This does not replace that live test: the replay returns only what
/// is fed to it here, so it proves the description against the recording, not the camera of
/// the session (camera session CS-1 runs the live test on the unit). The control ids are the
/// model's own (`decode::UVC`).
#[test]
fn the_recorded_uvc_ranges_equal_the_description() {
    let d = description_of("tiny2lite");
    let mut replay = Replay::new(2);
    for r in &RECORDED {
        let [min, max, step, default] = r.answers.map(le);
        replay = replay.range(
            control(r.leaf),
            ControlRange {
                min: min.unwrap_or(0),
                max: max.unwrap_or(1),
                step: step.unwrap_or(1),
                default: default.unwrap(),
            },
        );
    }
    // every UVC leaf of the model that has a bound, step or default is among the fixtures
    for (leaf, _) in decode::UVC {
        let s = &d.settings[leaf];
        if [&s.min, &s.max, &s.step].iter().any(|b| b.is_some()) || s.default.is_some() {
            assert!(
                RECORDED.iter().any(|r| r.leaf == leaf),
                "{leaf}: has bounds, but no recorded range answers"
            );
        }
    }
    let mut mismatches = Vec::new();
    for r in &RECORDED {
        let range = replay.ctrl_range(control(r.leaf)).unwrap().unwrap();
        let s = &d.settings[r.leaf];
        // exposure.time is in 100 µs units on the camera, µs in the description
        let scale = if r.leaf == "exposure.time" {
            100.0
        } else {
            1.0
        };
        let ours = |b: &Option<obsbot_proto::Bound>| b.as_ref().and_then(|b| b.get(None));
        let default = s
            .default
            .as_ref()
            .and_then(|v| v.as_f64().or_else(|| v.as_bool().map(f64::from)));
        for (key, ours, camera, recorded) in [
            ("min", ours(&s.min), range.min, !r.answers[0].is_empty()),
            ("max", ours(&s.max), range.max, !r.answers[1].is_empty()),
            ("step", ours(&s.step), range.step, !r.answers[2].is_empty()),
            ("default", default, range.default, true),
        ] {
            let Some(ours) = ours else { continue };
            assert!(recorded, "{}#{key}: not recorded", r.leaf);
            if (ours - f64::from(camera) * scale).abs() > 1e-9 {
                mismatches.push(format!(
                    "{}#{key}: description {ours}, {} {}",
                    r.leaf,
                    r.capture,
                    f64::from(camera) * scale
                ));
            }
        }
    }
    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
    // every bound, step and default these answers back has its row, with the capture
    for r in &RECORDED {
        for row in levels::LEVELS.iter().filter(|row| {
            ["@min", "@max", "#step", "#default"]
                .iter()
                .any(|suffix| row.key == format!("{}{suffix}", r.leaf))
        }) {
            assert_eq!(row.level, Level::Captured, "{}", row.key);
            assert!(
                row.evidence.contains(&r.capture["capture ".len()..]),
                "{}: {} is not in \"{}\"",
                row.key,
                r.capture,
                row.evidence
            );
        }
    }
}
