// SPDX-License-Identifier: GPL-3.0-or-later

//! Evidence levels of the registered models (design spec 8.5). Each model records, as
//! constants in its module, the level of every field of its description (every setting path
//! and every action), taken from its field ledger, and a row of its own for each value or
//! bound whose evidence differs from its field's; a value or bound without a row takes its
//! field's level, as the ledger's rule says. The level `none` is never inherited: a value,
//! bound or aspect without backing has a row of level `none` of its own and a mark of spec 9.3
//! in the model's `unbacked` list, and [`check`] holds the two lists against each other. Only
//! `captured` and `measured` let a field, value or bound into a released description; `daemon`
//! marks what is not a camera value. [`check`] lists every way a model falls short;
//! `tests/evidence.rs` runs it for every model of the registry.

use crate::contract::ModelEntry;
use crate::models::tiny2lite;
use obsbot_proto::{Bound, Description, Setting, Support};
use std::collections::BTreeSet;

/// The level of one field, value or bound (spec 8.5; the words of the spec's preamble).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    /// In a capture of the camera's USB traffic, cited by its key ("capture N").
    Captured,
    /// Written from Linux and read back on the camera, or its effect observed.
    Measured,
    /// Not a camera value: a Linux fact the family reads for every model (`on_air.*`).
    Daemon,
    /// No backing today: allowed only for a mark of spec 9.3 in the model's `unbacked` list,
    /// and every such mark that names a value, bound or aspect has a row of this level.
    None,
}

/// One row: a key, its level and its evidence as the ledger gives it. Keys: `<path>`,
/// `<path>=<value>`, `<path>@min`, `<path>@max`, `<path>#step`, `<path>#default`,
/// `<path>#precision`, a component of an `each` bound as `<path>@min.<component>`, a field of
/// a list item as `<path>[].<field>`, and the same under `do:<action>`, `do:<action>.<param>`
/// and `do:<action>#result.<field>` for actions. A mark of spec 9.3 may also name an aspect
/// the description does not enumerate (`<path>#direction`, `#order`, `#values`): such a key is
/// valid for a row when `unbacked` lists it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Row {
    pub key: &'static str,
    pub level: Level,
    pub evidence: &'static str,
}

/// What a model records about its evidence.
#[derive(Debug, Clone, Copy)]
pub struct Evidence {
    /// One row per field and action, plus the values and bounds with evidence of their own.
    pub levels: &'static [Row],
    /// The marks of spec 9.3: (key, row).
    pub unbacked: &'static [(&'static str, &'static str)],
    /// Bounds, steps and defaults the description holds without backing yet.
    pub unconfirmed: &'static [&'static str],
}

/// The evidence of a registered model, by its registry key. A model added to the registry
/// adds its line here; `tests/evidence.rs` fails for a registered model without one.
pub fn of(key: &str) -> Option<Evidence> {
    match key {
        "tiny2lite" => Some(tiny2lite::levels::EVIDENCE),
        _ => None,
    }
}

/// Every key a row may name: the fields, and the values, bounds, steps and defaults the
/// description holds, down to the fields of list items, action parameters and action results.
pub fn keys(d: &Description) -> BTreeSet<String> {
    let mut keys = BTreeSet::new();
    for (path, s) in &d.settings {
        setting(path, s, &mut keys);
    }
    for (name, a) in &d.actions {
        let field = format!("do:{name}");
        for (param, s) in &a.params {
            setting(&format!("{field}.{param}"), s, &mut keys);
        }
        for (name, s) in a.result.iter().flatten() {
            setting(&format!("{field}#result.{name}"), s, &mut keys);
        }
        keys.insert(field);
    }
    keys
}

fn setting(key: &str, s: &Setting, keys: &mut BTreeSet<String>) {
    keys.insert(key.to_string());
    for v in s.values.iter().chain(&s.writable) {
        keys.insert(format!("{key}={v}"));
    }
    for (bound, suffix) in [
        (&s.min, "@min"),
        (&s.max, "@max"),
        (&s.step, "#step"),
        (&s.precision, "#precision"),
    ] {
        if let Some(bound) = bound {
            keys.insert(format!("{key}{suffix}"));
            if let Bound::Each(components) = bound {
                for c in components.keys() {
                    keys.insert(format!("{key}{suffix}.{c}"));
                }
            }
        }
    }
    if s.default.is_some() {
        keys.insert(format!("{key}#default"));
    }
    for (name, item) in s.item.iter().flatten() {
        setting(&format!("{key}[].{name}"), item, keys);
    }
}

/// The field a key belongs to (`exposure.time@min` → `exposure.time`,
/// `do:gimbal.move.speed@max` → `do:gimbal.move`), if the description has it.
pub fn field_of(key: &str, d: &Description) -> Option<String> {
    let base = key.split(['=', '@', '#', '[']).next().unwrap_or(key);
    match base.strip_prefix("do:") {
        Some(rest) => d
            .actions
            .keys()
            .find(|a| rest == a.as_str() || rest.starts_with(&format!("{a}.")))
            .map(|a| format!("do:{a}")),
        None => d.settings.contains_key(base).then(|| base.to_string()),
    }
}

/// Whether `key` names a whole field or action of `d`, not a value, bound or aspect of one.
fn is_field(key: &str, d: &Description) -> bool {
    d.settings.contains_key(key)
        || key
            .strip_prefix("do:")
            .is_some_and(|a| d.actions.contains_key(a))
}

/// Every way `e` falls short for the model `entry` with the description `d`; empty when it
/// passes. Every row names something the description has (or a mark of 9.3), has a level that
/// fits its evidence, and a row of level `none` and a mark of 9.3 go together (a mark on a
/// whole field may keep the field's own level: the ledger's evidence for the field, with the
/// mark as its caveat). `strict` (every release from 1.0 on) or a `tested` model allows no mark
/// of 9.3 and no unconfirmed number; otherwise each must be listed and nothing else may lack
/// backing.
pub fn check(entry: &ModelEntry, d: &Description, e: &Evidence, strict: bool) -> Vec<String> {
    let mut problems = Vec::new();
    let keys = keys(d);
    let mut seen = BTreeSet::new();
    for row in e.levels {
        if !seen.insert(row.key) {
            problems.push(format!("{}: two rows", row.key));
        }
        if !keys.contains(row.key) && !e.unbacked.iter().any(|(k, _)| *k == row.key) {
            problems.push(format!(
                "{}: not a field, value or bound of the description",
                row.key
            ));
        }
        let fits = match row.level {
            Level::Captured => row.evidence.starts_with("capture "),
            Level::Measured => row.evidence.starts_with("measured "),
            Level::Daemon => row.evidence.starts_with("daemon") && row.key.starts_with("on_air."),
            Level::None => row.evidence == "none",
        };
        if !fits {
            problems.push(format!(
                "{}: level {:?} does not fit its evidence \"{}\"",
                row.key, row.level, row.evidence
            ));
        }
        if row.level == Level::None && !e.unbacked.iter().any(|(k, _)| *k == row.key) {
            problems.push(format!("{}: no backing and no mark of 9.3", row.key));
        }
    }
    for path in d.settings.keys() {
        if !seen.contains(path.as_str()) {
            problems.push(format!("{path}: no level"));
        }
    }
    for name in d.actions.keys() {
        if !seen.contains(format!("do:{name}").as_str()) {
            problems.push(format!("do:{name}: no level"));
        }
    }
    for (key, mark) in e.unbacked {
        if field_of(key, d).is_none() {
            problems.push(format!("{key} ({mark}): not in the description"));
        }
        // no inheritance of `none`: a mark names a value, bound or aspect without backing
        // only if that value, bound or aspect has its own row of level none
        match e.levels.iter().find(|r| r.key == *key) {
            Some(r) if r.level == Level::None => {}
            Some(_) if is_field(key, d) => {}
            Some(r) => problems.push(format!(
                "{key} ({mark}): a mark of 9.3, but its row has level {:?}, not none",
                r.level
            )),
            None => problems.push(format!(
                "{key} ({mark}): a mark of 9.3 without a row of level none"
            )),
        }
    }
    for key in e.unconfirmed {
        if !keys.contains(*key) {
            problems.push(format!("{key} (unconfirmed): not in the description"));
        }
    }
    if strict || entry.support == Support::Tested {
        for (key, mark) in e.unbacked {
            problems.push(format!("{key}: {mark} of spec 9.3 has no backing"));
        }
        for key in e.unconfirmed {
            problems.push(format!("{key}: unconfirmed"));
        }
    }
    problems
}
