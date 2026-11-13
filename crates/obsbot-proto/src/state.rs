// SPDX-License-Identifier: GPL-3.0-or-later

//! The generic payload: state, patches, action parameters and path helpers (4.1, 4.2, 6.1).
//!
//! A state is a nested JSON object in the shape of the model's state tree (section 4.3 for
//! the Tiny 2 Lite), plus the daemon's fields `available`, `pending` (leaf path → written
//! value) and `unread` (leaf paths). A leaf path joins the object keys with `.`.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::BTreeMap;

/// Action parameters and action results: name → value.
pub type Params = Map<String, Value>;

/// Leaf path → value, as a model reads one group.
pub type Values = BTreeMap<String, Value>;

/// The paths the daemon adds to every state; they are never settings.
pub const DAEMON_PATHS: [&str; 3] = ["available", "pending", "unread"];

/// The published state of one device.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct State {
    pub available: bool,
    /// Leaf path → the value written; the camera accepted it but does not show it yet.
    #[serde(default)]
    pub pending: BTreeMap<String, Value>,
    /// Leaf paths never read since the device last became available, or whose last read failed.
    #[serde(default)]
    pub unread: Vec<String>,
    /// The state tree: group → … → leaf value.
    #[serde(flatten)]
    pub values: Map<String, Value>,
}

impl State {
    /// An empty state: no values, nothing pending, nothing unread.
    pub fn new(available: bool) -> State {
        State {
            available,
            pending: BTreeMap::new(),
            unread: Vec::new(),
            values: Map::new(),
        }
    }

    /// The value at a leaf or group path of the state tree. `None` when the path is absent.
    /// The daemon's fields are struct fields, not paths of the tree.
    pub fn at(&self, path: &str) -> Option<&Value> {
        let mut parts = path.split('.');
        let mut node = self.values.get(parts.next()?)?;
        for part in parts {
            node = node.as_object()?.get(part)?;
        }
        Some(node)
    }

    /// Sets the value at a leaf path, creating the groups on the way.
    pub fn set(&mut self, path: &str, value: Value) {
        let parts: Vec<&str> = path.split('.').collect();
        let (last, groups) = parts.split_last().expect("split always yields one part");
        let mut map = &mut self.values;
        for group in groups {
            let entry = map
                .entry(group.to_string())
                .or_insert_with(|| Value::Object(Map::new()));
            if !entry.is_object() {
                *entry = Value::Object(Map::new());
            }
            map = entry.as_object_mut().expect("made an object above");
        }
        map.insert(last.to_string(), value);
    }

    /// A copy with the given leaves set: the state a `requires` check sees (4.2).
    pub fn with(&self, leaves: &[(String, Value)]) -> State {
        let mut next = self.clone();
        for (path, value) in leaves {
            next.set(path, value.clone());
        }
        next
    }

    /// The reply of a `get` for these paths (4.1 rule 9): the paths asked for, plus
    /// `available`, `pending` and `unread` in full. Absent paths are left out.
    pub fn select(&self, paths: &[String]) -> State {
        let mut out = State {
            available: self.available,
            pending: self.pending.clone(),
            unread: self.unread.clone(),
            values: Map::new(),
        };
        for path in paths {
            if DAEMON_PATHS.contains(&path.as_str()) {
                continue;
            }
            if let Some(value) = self.at(path) {
                out.set(path, value.clone());
            }
        }
        out
    }

    /// Whether the state shows the camera asleep (`asleep: true`).
    pub fn asleep(&self) -> bool {
        self.at("asleep") == Some(&Value::Bool(true))
    }
}

/// The payload of `set`: leaf paths, nested (`{"zoom":{"factor":2}}`) or dotted
/// (`{"zoom.factor":2}`). [`crate::validate::leaves`] turns it into leaves.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct Patch(pub Map<String, Value>);

impl Patch {
    pub fn new() -> Patch {
        Patch(Map::new())
    }

    /// Adds one leaf under its dotted path.
    pub fn leaf(mut self, path: &str, value: impl Into<Value>) -> Patch {
        self.0.insert(path.to_string(), value.into());
        self
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn sample() -> State {
        let mut s = State::new(true);
        s.set("zoom.factor", json!(1.5));
        s.set("zone.area.pan", json!({"min": -20, "max": 20}));
        s.set("asleep", json!(false));
        s.pending.insert("zoom.factor".into(), json!(2));
        s.unread.push("presets".into());
        s
    }

    #[test]
    fn paths_walk_the_nested_tree() {
        let s = sample();
        assert_eq!(s.at("zoom.factor"), Some(&json!(1.5)));
        assert_eq!(
            s.at("zone.area"),
            Some(&json!({"pan": {"min": -20, "max": 20}}))
        );
        assert_eq!(
            s.at("zone.area.pan.min"),
            Some(&json!(-20)),
            "a component is reachable too"
        );
        assert_eq!(
            s.at("gimbal.pan"),
            None,
            "absent paths are None, never null"
        );
        assert!(!s.asleep());
    }

    #[test]
    fn a_get_reply_keeps_the_daemon_fields_in_full() {
        let got = sample().select(&["zoom".to_string(), "nothing.here".to_string()]);
        assert_eq!(got.at("zoom.factor"), Some(&json!(1.5)));
        assert_eq!(got.at("zone.area.pan"), None, "only the paths asked for");
        assert_eq!(got.pending.get("zoom.factor"), Some(&json!(2)));
        assert_eq!(got.unread, vec!["presets".to_string()]);
    }

    #[test]
    fn with_applies_leaves_without_touching_the_original() {
        let s = sample();
        let next = s.with(&[("exposure.mode".into(), json!("manual"))]);
        assert_eq!(next.at("exposure.mode"), Some(&json!("manual")));
        assert_eq!(s.at("exposure.mode"), None);
    }

    #[test]
    fn a_patch_built_with_leaf_uses_dotted_keys() {
        let p = Patch::new()
            .leaf("led.brightness", 2)
            .leaf("zoom.factor", 1.5);
        assert_eq!(
            serde_json::to_string(&p).unwrap(),
            r#"{"led.brightness":2,"zoom.factor":1.5}"#
        );
    }
}
