// SPDX-License-Identifier: GPL-3.0-or-later

//! The validator (4.1 rule 5, 4.2, 5.6): checks a `set` patch, a `do` and a `get` against a
//! device description and its current state, before anything is sent to the camera. The
//! daemon is authoritative; clients may run the same checks.
//!
//! The whole request is checked. When several things are wrong, the refusal with the first
//! code in this order wins, and within one code the first path in path order:
//! `invalid`, `unsupported`, `out_of_range`, `conflict`, `asleep` (the order of 5.6).

use crate::description::{Asleep, Bound, Description, Setting, SettingType, Warning};
use crate::message::ErrorCode;
use crate::state::{DAEMON_PATHS, Params, Patch, State};
use crate::values::{is_raw, same};
use serde_json::Value;
use std::collections::BTreeSet;

/// Why a request is refused. `path` names the leaf, parameter or action at fault.
#[derive(Debug, Clone, PartialEq)]
pub struct Refusal {
    pub code: ErrorCode,
    pub path: Option<String>,
    pub message: String,
}

impl Refusal {
    pub fn new(code: ErrorCode, path: &str, message: &str) -> Refusal {
        Refusal {
            code,
            path: Some(path.to_string()),
            message: message.to_string(),
        }
    }
}

/// What a value is checked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fit {
    /// A value a client writes: `writable[]`, bounds, step; never `null` or `raw:<n>`.
    Write,
    /// A value a state may show: `values[]`, bounds; `null` where `nullable`; `raw:<n>`.
    State,
}

fn rank(code: &ErrorCode) -> u8 {
    match code {
        ErrorCode::Invalid => 0,
        ErrorCode::Unsupported => 1,
        ErrorCode::OutOfRange => 2,
        ErrorCode::Conflict => 3,
        ErrorCode::Asleep => 4,
        _ => 5,
    }
}

/// Keeps the refusal that wins under the order above.
#[derive(Default)]
struct First(Option<Refusal>);

impl First {
    fn add(&mut self, code: ErrorCode, path: &str, message: String) {
        let better = match &self.0 {
            None => true,
            Some(old) => rank(&code) < rank(&old.code),
        };
        if better {
            let path = (!path.is_empty()).then(|| path.to_string());
            self.0 = Some(Refusal {
                code,
                path,
                message,
            });
        }
    }

    fn done(self) -> Result<(), Refusal> {
        self.0.map_or(Ok(()), Err)
    }
}

/// The leaves of a patch, nested or dotted, as (leaf path, value) in path order. Refuses an
/// empty patch, a group written as a value, a leaf named twice (`invalid`) and a path that is
/// not in the description (`unsupported`).
pub fn leaves(desc: &Description, patch: &Patch) -> Result<Vec<(String, Value)>, Refusal> {
    let (out, first) = split(desc, patch);
    first.done()?;
    Ok(out)
}

/// The leaves of a patch and the first structural problem, if any.
fn split(desc: &Description, patch: &Patch) -> (Vec<(String, Value)>, First) {
    let mut first = First::default();
    let mut out: Vec<(String, Value)> = Vec::new();
    if patch.is_empty() {
        first.add(ErrorCode::Invalid, "", "the patch is empty".into());
    }
    collect(desc, "", &patch.0, &mut out, &mut first);
    out.sort_by(|a, b| a.0.cmp(&b.0));
    for pair in out.windows(2) {
        if pair[0].0 == pair[1].0 {
            first.add(
                ErrorCode::Invalid,
                &pair[0].0,
                format!("the patch names {} twice", pair[0].0),
            );
        }
    }
    out.dedup_by(|a, b| a.0 == b.0);
    (out, first)
}

fn collect(
    desc: &Description,
    prefix: &str,
    map: &serde_json::Map<String, Value>,
    out: &mut Vec<(String, Value)>,
    first: &mut First,
) {
    for (key, value) in map {
        let path = if prefix.is_empty() {
            key.clone()
        } else {
            format!("{prefix}.{key}")
        };
        if desc.settings.contains_key(&path) {
            out.push((path, value.clone()));
        } else if desc.is_group(&path) {
            match value.as_object() {
                Some(inner) => collect(desc, &path, inner, out, first),
                None => first.add(
                    ErrorCode::Invalid,
                    &path,
                    format!("{path} is a group, not a leaf"),
                ),
            }
        } else {
            first.add(
                ErrorCode::Unsupported,
                &path,
                format!("{path} is not a setting of this device"),
            );
        }
    }
}

/// Checks a `set` against the description and the current state; returns its leaves in path
/// order. The model's apply order is `Camera::order`, not this order.
pub fn validate_set(
    desc: &Description,
    state: &State,
    patch: &Patch,
) -> Result<Vec<(String, Value)>, Refusal> {
    let (leaves, mut first) = split(desc, patch);
    for (path, value) in &leaves {
        let setting = &desc.settings[path];
        if setting.readonly {
            first.add(ErrorCode::Invalid, path, format!("{path} is read-only"));
        } else if let Err((code, message)) = check_value(setting, value, Fit::Write) {
            first.add(code, path, format!("{path}: {message}"));
        }
    }
    let next = state.with(&leaves);
    for (path, _) in &leaves {
        let setting = &desc.settings[path];
        for req in &setting.requires {
            if !next.at(&req.path).is_some_and(|v| same(v, &req.equals)) {
                first.add(
                    ErrorCode::Conflict,
                    path,
                    format!("{path} requires {} = {}", req.path, req.equals),
                );
            }
        }
        if state.asleep() && setting.asleep == Some(Asleep::Ignored) {
            first.add(
                ErrorCode::Asleep,
                path,
                format!("the camera is asleep and ignores {path}"),
            );
        }
    }
    first.done()?;
    Ok(leaves)
}

/// Checks a `do` against the description and the current state: the action exists, every
/// parameter is known and fits, no required parameter is missing, a destructive action has
/// `confirm: true`, its `requires` hold and the camera is not asleep for an `ignored` action.
pub fn validate_do(
    desc: &Description,
    state: &State,
    action: &str,
    params: &Params,
) -> Result<(), Refusal> {
    let Some(spec) = desc.actions.get(action) else {
        return Err(Refusal::new(
            ErrorCode::Unsupported,
            action,
            &format!("{action} is not an action of this device"),
        ));
    };
    let mut first = First::default();
    for (name, value) in params {
        match spec.params.get(name) {
            None => first.add(
                ErrorCode::Invalid,
                name,
                format!("{action} has no parameter {name}"),
            ),
            Some(setting) => {
                if let Err((code, message)) = check_value(setting, value, Fit::Write) {
                    first.add(code, name, format!("{action} {name}: {message}"));
                }
            }
        }
    }
    for (name, setting) in &spec.params {
        if !setting.optional && !params.contains_key(name) {
            first.add(
                ErrorCode::Invalid,
                name,
                format!("{action} needs the parameter {name}"),
            );
        }
    }
    if spec.warning.contains(&Warning::Destructive)
        && params.get("confirm") != Some(&Value::Bool(true))
    {
        first.add(
            ErrorCode::Invalid,
            "confirm",
            format!("{action} deletes data on the camera: send confirm: true"),
        );
    }
    for req in &spec.requires {
        if !state.at(&req.path).is_some_and(|v| same(v, &req.equals)) {
            first.add(
                ErrorCode::Conflict,
                action,
                format!("{action} requires {} = {}", req.path, req.equals),
            );
        }
    }
    if state.asleep() && spec.asleep == Asleep::Ignored {
        first.add(
            ErrorCode::Asleep,
            action,
            format!("the camera is asleep and ignores {action}"),
        );
    }
    first.done()
}

/// Checks the paths of a `get`: each is a daemon field, a leaf or a group of the description.
pub fn validate_get(desc: &Description, paths: &[String]) -> Result<(), Refusal> {
    let mut first = First::default();
    for path in paths {
        let known = DAEMON_PATHS.contains(&path.as_str())
            || desc.settings.contains_key(path)
            || desc.is_group(path);
        if !known {
            first.add(
                ErrorCode::Unsupported,
                path,
                format!("{path} is not a path of this device"),
            );
        }
    }
    first.done()
}

fn as_number(v: &Value) -> Option<f64> {
    v.as_f64().filter(|x| x.is_finite())
}

const EPS: f64 = 1e-9;

fn check_number(
    s: &Setting,
    x: f64,
    component: Option<&str>,
    fit: Fit,
) -> Result<(), (ErrorCode, String)> {
    let bound = |b: &Option<Bound>| b.as_ref().and_then(|b| b.get(component));
    let what = component.map_or(String::new(), |c| format!("{c} "));
    if let Some(min) = bound(&s.min)
        && x < min - EPS
    {
        return Err((
            ErrorCode::OutOfRange,
            format!("{what}{x} is below the minimum {min}"),
        ));
    }
    if let Some(max) = bound(&s.max)
        && x > max + EPS
    {
        return Err((
            ErrorCode::OutOfRange,
            format!("{what}{x} is above the maximum {max}"),
        ));
    }
    if fit == Fit::Write
        && let Some(step) = bound(&s.step).filter(|st| *st > 0.0)
    {
        let k = (x - bound(&s.min).unwrap_or(0.0)) / step;
        if (k - k.round()).abs() > 1e-6 {
            return Err((
                ErrorCode::OutOfRange,
                format!("{what}{x} is not a multiple of the step {step}"),
            ));
        }
    }
    Ok(())
}

/// Checks one value against one setting. The error carries the code and an English message
/// without the path.
pub fn check_value(s: &Setting, v: &Value, fit: Fit) -> Result<(), (ErrorCode, String)> {
    if v.is_null() {
        return if fit == Fit::State && s.nullable {
            Ok(())
        } else {
            Err((
                ErrorCode::Invalid,
                "null is not a value of this setting".into(),
            ))
        };
    }
    if fit == Fit::State && v.as_str().is_some_and(is_raw) {
        return Ok(());
    }
    let invalid = |m: &str| Err((ErrorCode::Invalid, m.to_string()));
    match &s.kind {
        SettingType::Bool => {
            if v.is_boolean() {
                Ok(())
            } else {
                invalid("expected true or false")
            }
        }
        SettingType::Enum => {
            let Some(text) = v.as_str() else {
                return invalid("expected one of the names of this setting");
            };
            let allowed = if fit == Fit::Write {
                &s.writable
            } else {
                &s.values
            };
            if allowed.iter().any(|a| a == text) {
                Ok(())
            } else {
                Err((
                    ErrorCode::OutOfRange,
                    format!("{text} is not one of {}", allowed.join("|")),
                ))
            }
        }
        SettingType::Number => match as_number(v) {
            Some(x) => check_number(s, x, None, fit),
            None => invalid("expected a number"),
        },
        SettingType::String => {
            let Some(text) = v.as_str() else {
                return invalid("expected a string");
            };
            if let Some(max) = s.max_len
                && text.chars().count() > max as usize
            {
                return Err((
                    ErrorCode::OutOfRange,
                    format!("longer than {max} characters"),
                ));
            }
            if s.ascii && !text.bytes().all(|b| (0x20..=0x7e).contains(&b)) {
                return Err((
                    ErrorCode::OutOfRange,
                    "only printable ASCII characters are allowed".into(),
                ));
            }
            Ok(())
        }
        SettingType::List => {
            if fit == Fit::Write {
                return invalid("a list is read-only");
            }
            let Some(items) = v.as_array() else {
                return invalid("expected a list");
            };
            let fields = s.item.clone().unwrap_or_default();
            for item in items {
                let Some(obj) = item.as_object() else {
                    return invalid("expected a list of objects");
                };
                for (name, field) in &fields {
                    let value = obj.get(name).unwrap_or(&Value::Null);
                    check_value(field, value, Fit::State)
                        .map_err(|(c, m)| (c, format!("item {name}: {m}")))?;
                }
            }
            Ok(())
        }
        SettingType::Other(kind) => {
            if fit == Fit::State {
                Ok(())
            } else {
                invalid(&format!("a setting of type {kind} cannot be written"))
            }
        }
        kind => {
            let names = kind
                .components()
                .expect("the remaining types are object-shaped");
            let Some(obj) = v.as_object() else {
                return invalid(&format!("expected an object with {}", names.join(", ")));
            };
            let keys: BTreeSet<&str> = obj.keys().map(String::as_str).collect();
            if keys != names.iter().copied().collect() {
                return invalid(&format!("expected exactly the fields {}", names.join(", ")));
            }
            for name in names {
                let Some(x) = as_number(&obj[*name]) else {
                    return invalid(&format!("{name} must be a number"));
                };
                check_number(s, x, Some(name), fit)?;
            }
            if *kind == SettingType::Range && as_number(&obj["min"]) > as_number(&obj["max"]) {
                return Err((ErrorCode::OutOfRange, "min is above max".into()));
            }
            Ok(())
        }
    }
}

fn path_ok(path: &str) -> bool {
    !path.is_empty()
        && path.split('.').all(|seg| {
            let mut bytes = seg.bytes();
            bytes.next().is_some_and(|b| b.is_ascii_lowercase())
                && bytes.all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
        })
}

fn check_setting(
    where_: &str,
    s: &Setting,
    desc: &Description,
    leaf: bool,
    problems: &mut Vec<String>,
) {
    let mut bad = |m: &str| problems.push(format!("{where_}: {m}"));
    if s.label.is_empty() {
        bad("has no label");
    }
    match &s.kind {
        SettingType::Enum => {
            if s.values.is_empty() {
                bad("an enum without values");
            }
            if s.writable.iter().any(|w| !s.values.contains(w)) {
                bad("writable holds a value that values does not");
            }
            if leaf && s.readonly && !s.writable.is_empty() {
                bad("a read-only enum with writable values");
            }
            if (!leaf || !s.readonly) && s.writable.is_empty() {
                bad("a writable enum without writable values");
            }
        }
        SettingType::Number => {
            for (name, b) in [
                ("min", &s.min),
                ("max", &s.max),
                ("step", &s.step),
                ("precision", &s.precision),
            ] {
                if matches!(b, Some(Bound::Each(_))) {
                    bad(&format!("{name} of a number must be one number"));
                }
            }
        }
        SettingType::List => {
            if !s.readonly || s.item.is_none() {
                bad("a list must be read-only and describe its item");
            }
        }
        kind => {
            if let Some(names) = kind.components() {
                for (name, b) in [("min", &s.min), ("max", &s.max)] {
                    if let Some(Bound::Each(map)) = b {
                        let keys: BTreeSet<&str> = map.keys().map(String::as_str).collect();
                        if keys != names.iter().copied().collect() {
                            bad(&format!(
                                "{name} must have exactly the fields {}",
                                names.join(", ")
                            ));
                        }
                    } else if b.is_some() {
                        bad(&format!(
                            "{name} of an object-shaped setting must be an object"
                        ));
                    }
                }
            }
        }
    }
    let components: Vec<Option<&str>> = match s.kind.components() {
        Some(names) => names.iter().map(|n| Some(*n)).collect(),
        None => vec![None],
    };
    for c in components {
        if let (Some(lo), Some(hi)) = (
            s.min.as_ref().and_then(|b| b.get(c)),
            s.max.as_ref().and_then(|b| b.get(c)),
        ) && lo > hi
        {
            bad("min is above max");
        }
    }
    if let Some(default) = &s.default
        && check_value(s, default, Fit::State).is_err()
    {
        bad("the default does not fit the setting");
    }
    for req in &s.requires {
        match desc.settings.get(&req.path) {
            None => bad(&format!("requires the unknown path {}", req.path)),
            Some(target) => {
                if check_value(target, &req.equals, Fit::State).is_err() {
                    bad(&format!("requires a value that {} cannot have", req.path));
                }
            }
        }
    }
    if let Some(topic) = &s.live
        && !path_ok(topic)
    {
        bad("the live topic is not a lower-case name");
    }
    if leaf {
        if s.optional {
            bad("optional is for action parameters only");
        }
        if s.readonly && (s.asleep.is_some() || s.settle_ms.is_some()) {
            bad("a read-only leaf has neither asleep nor settle_ms");
        }
        if !s.readonly && s.asleep.is_none() {
            bad("a writable leaf needs asleep");
        }
        if !s.readonly && s.readback && s.settle_ms.is_none() {
            bad("a writable leaf with a readback needs settle_ms");
        }
        if !s.readback && s.settle_ms.is_some() {
            bad("a leaf without readback has no settle_ms");
        }
    }
}

/// Checks that a description is consistent in itself; returns the problems, empty when there
/// are none. Every model and fake must pass (trait conformance, 10.3 W0).
pub fn check_description(desc: &Description) -> Vec<String> {
    let mut problems = Vec::new();
    for (path, setting) in &desc.settings {
        if !path_ok(path) {
            problems.push(format!(
                "{path}: not lower-case snake_case segments joined by '.'"
            ));
        }
        if DAEMON_PATHS.contains(&path.as_str()) {
            problems.push(format!("{path}: reserved for the daemon"));
        }
        if desc.is_group(path) {
            problems.push(format!("{path}: a leaf that is also a group"));
        }
        check_setting(path, setting, desc, true, &mut problems);
        for (field, item) in setting.item.iter().flatten() {
            check_setting(
                &format!("{path}[].{field}"),
                item,
                desc,
                false,
                &mut problems,
            );
        }
    }
    for (name, action) in &desc.actions {
        if !path_ok(name) {
            problems.push(format!(
                "{name}: not lower-case snake_case segments joined by '.'"
            ));
        }
        if action.label.is_empty() {
            problems.push(format!("{name}: has no label"));
        }
        for (param, setting) in action.params.iter().chain(action.result.iter().flatten()) {
            if !path_ok(param) || param.contains('.') {
                problems.push(format!(
                    "{name}.{param}: a parameter name is one lower-case segment"
                ));
            }
            check_setting(
                &format!("{name}.{param}"),
                setting,
                desc,
                false,
                &mut problems,
            );
        }
        if action.warning.contains(&Warning::Destructive)
            && action
                .params
                .get("confirm")
                .is_none_or(|c| c.kind != SettingType::Bool)
        {
            problems.push(format!(
                "{name}: a destructive action needs the bool parameter confirm"
            ));
        }
        for req in &action.requires {
            if !desc.settings.contains_key(&req.path) {
                problems.push(format!("{name}: requires the unknown path {}", req.path));
            }
        }
    }
    for list in desc.limits.keys() {
        if desc
            .settings
            .get(list)
            .is_none_or(|s| s.kind != SettingType::List)
        {
            problems.push(format!("limits.{list}: not a list setting"));
        }
    }
    problems
}
