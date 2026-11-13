// SPDX-License-Identifier: GPL-3.0-or-later

//! The trait conformance check (design spec 10.3 W0, 8.7 step 6). Every model and every fake
//! passes it: the fakes in W0, `tiny2lite` on the replay transport in track A.
//!
//! It checks: the description is consistent; every leaf with a readback is in exactly one
//! group and every readable leaf reads back a value that fits its setting; every writable
//! leaf with a readback is in a group with a periodic tier (5.7); every writable leaf takes a
//! valid written value, and after the touched re-read its value fits (it shows the written
//! value, or the daemon would list it in `pending`); `touched` includes the leaf's own group;
//! `order` puts `requires` paths first; the stop of every continuing action is an action that
//! the model marks as a stop.
//! The camera must be awake. Writes that restart the camera (`warning: restart`) are skipped.

use crate::contract::camera::{Camera, Ctx, Motion, Target, Tier};
use obsbot_proto::validate::{Fit, check_description, check_value};
use obsbot_proto::values::{number, same};
use obsbot_proto::{Bound, Setting, SettingType, Warning};
use serde_json::{Map, Value, json};
use std::collections::{BTreeMap, BTreeSet};

/// Runs the check; returns the problems, empty when the camera conforms.
pub fn check(camera: &mut dyn Camera) -> Vec<String> {
    let mut problems: Vec<String> = Vec::new();
    let desc = camera.description().clone();
    problems.extend(
        check_description(&desc)
            .into_iter()
            .map(|p| format!("description: {p}")),
    );

    let plan = camera.poll_plan().to_vec();
    let mut group_of: BTreeMap<String, String> = BTreeMap::new();
    let mut names = BTreeSet::new();
    for group in &plan {
        if !names.insert(group.name.clone()) {
            problems.push(format!("group {}: named twice", group.name));
        }
        for leaf in &group.leaves {
            if let Some(other) = group_of.insert(leaf.clone(), group.name.clone()) {
                problems.push(format!("{leaf}: in groups {other} and {}", group.name));
            }
            match desc.settings.get(leaf) {
                None => problems.push(format!(
                    "group {}: {leaf} is not a described leaf",
                    group.name
                )),
                Some(s) if !s.readback => problems.push(format!(
                    "{leaf}: readback false but in group {}",
                    group.name
                )),
                Some(_) => {}
            }
        }
        if let Some(live) = &group.live
            && !group
                .leaves
                .iter()
                .any(|l| desc.settings[l].live.as_deref() == Some(live.topic.as_str()))
        {
            problems.push(format!(
                "group {}: live tier for {} but no leaf of that topic",
                group.name, live.topic
            ));
        }
    }
    for (path, s) in &desc.settings {
        if s.readback && !group_of.contains_key(path) {
            problems.push(format!("{path}: has a readback but is in no group"));
        }
        if !s.readonly && s.readback {
            let tier = group_of
                .get(path)
                .and_then(|g| plan.iter().find(|p| &p.name == g))
                .map(|g| g.tier);
            if tier == Some(Tier::Touched) {
                problems.push(format!(
                    "{path}: writable with a readback, so its group needs a periodic tier (5.7)"
                ));
            }
        }
    }

    let mut current: BTreeMap<String, Value> = BTreeMap::new();
    for group in &plan {
        match camera.read(&group.name, &Ctx::none()) {
            Err(e) => problems.push(format!("group {}: read failed: {e}", group.name)),
            Ok(values) => {
                let got: BTreeSet<&String> = values.keys().collect();
                let want: BTreeSet<&String> = group.leaves.iter().collect();
                if got != want {
                    problems.push(format!(
                        "group {}: read returned {got:?}, the plan says {want:?}",
                        group.name
                    ));
                }
                for (leaf, value) in values {
                    if let Some(s) = desc.settings.get(&leaf)
                        && let Err((_, m)) = check_value(s, &value, Fit::State)
                    {
                        problems.push(format!("{leaf}: read {value} does not fit: {m}"));
                    }
                    current.insert(leaf, value);
                }
            }
        }
    }

    let writable: Vec<(&String, &Setting)> =
        desc.settings.iter().filter(|(_, s)| !s.readonly).collect();
    for (path, s) in &writable {
        for req in &s.requires {
            let order = camera.order(&[path.as_str(), req.path.as_str()]);
            let at = |p: &str| order.iter().position(|o| o == p);
            if at(&req.path).zip(at(path)).is_none_or(|(r, l)| r > l) {
                problems.push(format!(
                    "{path}: order {order:?} does not put {} first",
                    req.path
                ));
            }
        }
    }
    let all: Vec<&str> = writable.iter().map(|(p, _)| p.as_str()).collect();
    let mut ordered = camera.order(&all);
    ordered.sort();
    let mut expected: Vec<String> = all.iter().map(|p| p.to_string()).collect();
    expected.sort();
    if ordered != expected {
        problems.push("order: not a permutation of the leaves it was given".into());
    }

    for (path, s) in &writable {
        if s.warning.contains(&Warning::Restart) {
            continue;
        }
        let Some(value) = sample_write(s, current.get(*path)) else {
            problems.push(format!("{path}: no writable value to test with"));
            continue;
        };
        for req in &s.requires {
            if let Err(e) = camera.apply(&req.path, &req.equals, &Ctx::none()) {
                problems.push(format!(
                    "{path}: writing its requirement {} failed: {e}",
                    req.path
                ));
            }
        }
        if let Err(e) = camera.apply(path, &value, &Ctx::none()) {
            problems.push(format!("{path}: apply {value} failed: {e}"));
            continue;
        }
        let touched = camera.touched(Target::Leaf(path));
        if let Some(own) = group_of.get(*path)
            && !touched.contains(own)
        {
            problems.push(format!(
                "{path}: touched {touched:?} lacks its own group {own}"
            ));
        }
        for group in &touched {
            if !names.contains(group) {
                problems.push(format!("{path}: touches the unknown group {group}"));
                continue;
            }
            match camera.read(group, &Ctx::none()) {
                Err(e) => problems.push(format!("{path}: re-read of {group} failed: {e}")),
                Ok(values) => {
                    if let Some(read) = values.get(*path)
                        && let Err((_, m)) = check_value(s, read, Fit::State)
                    {
                        problems.push(format!(
                            "{path}: after writing {value} it reads {read}, which does not fit: {m}"
                        ));
                    }
                    current.extend(values);
                }
            }
        }
    }

    for name in desc.actions.keys() {
        if let Motion::Continuing { stop } = camera.motion(name) {
            if !desc.actions.contains_key(stop) {
                problems.push(format!(
                    "{name}: continuing, but its stop {stop} is not an action"
                ));
            } else if camera.motion(stop) != Motion::Stop {
                problems.push(format!(
                    "{name}: continuing, but its stop {stop} is not a stop"
                ));
            }
        }
        for group in camera.touched(Target::Action(name)) {
            if !names.contains(&group) {
                problems.push(format!("{name}: touches the unknown group {group}"));
            }
        }
    }
    problems
}

fn bound(b: &Option<Bound>, component: Option<&str>) -> Option<f64> {
    b.as_ref().and_then(|b| b.get(component))
}

/// A writable value for `s` that differs from `current` where the setting allows it.
pub fn sample_write(s: &Setting, current: Option<&Value>) -> Option<Value> {
    let differs = |v: &Value| current.is_none_or(|c| !same(c, v));
    let pick = |candidates: Vec<Value>| {
        let fitting: Vec<Value> = candidates
            .into_iter()
            .filter(|v| check_value(s, v, Fit::Write).is_ok())
            .collect();
        fitting
            .iter()
            .find(|v| differs(v))
            .or(fitting.first())
            .cloned()
    };
    match &s.kind {
        SettingType::Bool => pick(vec![json!(true), json!(false)]),
        SettingType::Enum => pick(s.writable.iter().map(|v| json!(v)).collect()),
        SettingType::Number => {
            let lo = bound(&s.min, None);
            let hi = bound(&s.max, None);
            let mut c: Vec<Value> = [hi, lo].into_iter().flatten().map(number).collect();
            c.push(number(lo.unwrap_or(0.0) + 1.0));
            pick(c)
        }
        SettingType::String => {
            let len = s.max_len.unwrap_or(16).min(11) as usize;
            pick(vec![json!(&"Conformance"[..len]), json!("")])
        }
        kind => {
            let names = kind.components()?;
            let corner = |high: bool| {
                let mut m = Map::new();
                for name in names {
                    let lo = bound(&s.min, Some(name)).unwrap_or(0.0);
                    let hi = bound(&s.max, Some(name)).unwrap_or(lo);
                    let x = if *kind == SettingType::Range {
                        if *name == "min" { lo } else { hi }
                    } else if high {
                        hi
                    } else {
                        lo
                    };
                    m.insert(name.to_string(), number(x));
                }
                Value::Object(m)
            };
            pick(vec![corner(true), corner(false)])
        }
    }
}
