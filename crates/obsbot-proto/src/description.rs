// SPDX-License-Identifier: GPL-3.0-or-later

//! Device entries and descriptions (4.2): `Device`, `Usb`, `Description`, `Setting`, `Action`
//! and their enums, plus builder methods that models use to write their descriptions.
//!
//! Wire rules: maps are sorted by key; `readonly`, `nullable`, `readback` and `no_effect` are
//! always written; every other optional field is left out when it is empty. Each public enum
//! has a catch-all variant, so a newer daemon's additive values (4.1 rule 7) still parse.

use crate::values::number;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Number, Value};
use std::collections::BTreeMap;

fn yes() -> bool {
    true
}

fn is_false(b: &bool) -> bool {
    !*b
}

/// One entry of the device list (4.2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Device {
    /// `sn:<serial>` or `port:<usb path>` (5.5).
    pub id: String,
    pub name: String,
    /// The model key (`tiny2lite`, `fake-a`, `fake-b`), or `unknown` for an unsupported device.
    pub model: String,
    pub support: Support,
    pub serial: Option<String>,
    pub firmware: Option<String>,
    /// `/dev/videoN` of the video node with sysfs index 0.
    pub video_node: Option<String>,
    pub available: bool,
    /// Set only on the old `port:` entry during an identity hand-over (5.5).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replaced_by: Option<String>,
    pub usb: Usb,
    #[serde(flatten)]
    pub description: Description,
}

/// The USB facts of a device, read from sysfs at enumeration (4.2). Never holds the serial.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Usb {
    /// Four lower-case hex digits (`"3564"`).
    pub vendor: String,
    pub product: String,
    pub product_name: Option<String>,
    pub bcd_device: String,
    /// The USB device name in sysfs (`"3-2.1"`).
    pub path: String,
    pub serial_present: bool,
    /// Interface classes in interface order, two lower-case hex digits (`"0e"`, `"01"`).
    pub interfaces: Vec<String>,
    pub extension_units: Vec<ExtensionUnit>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ExtensionUnit {
    pub unit: u8,
    /// Lower-case `8-4-4-4-12` form.
    pub guid: String,
    pub controls: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Support {
    Tested,
    Untested,
    Unsupported,
    /// A level this crate does not know (a newer daemon).
    #[serde(untagged)]
    Other(String),
}

/// What a model offers (4.2): the part of a `Device` that `descriptions/<model>.json` holds.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Description {
    /// Leaf path → setting.
    pub settings: BTreeMap<String, Setting>,
    /// Action name → action.
    pub actions: BTreeMap<String, Action>,
    /// List path → maximum item count, where the camera has one.
    pub limits: BTreeMap<String, u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum SettingType {
    Bool,
    Enum,
    Number,
    /// `{min, max}`
    Range,
    /// `{pan, tilt, zoom}`
    Pose,
    /// `{x, y}`
    Point,
    /// `{left, top, right, bottom}`
    Box,
    String,
    /// A read-only list of objects; `item` describes their fields.
    List,
    #[serde(untagged)]
    Other(String),
}

impl SettingType {
    /// The component names of an object-shaped type, `None` for the other types.
    pub fn components(&self) -> Option<&'static [&'static str]> {
        match self {
            SettingType::Range => Some(&["min", "max"]),
            SettingType::Pose => Some(&["pan", "tilt", "zoom"]),
            SettingType::Point => Some(&["x", "y"]),
            SettingType::Box => Some(&["left", "top", "right", "bottom"]),
            _ => None,
        }
    }
}

/// The closed unit list of 4.2; absent for a raw camera value. New units are additive.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[non_exhaustive]
pub enum Unit {
    #[serde(rename = "deg")]
    Deg,
    #[serde(rename = "deg/s")]
    DegPerSecond,
    #[serde(rename = "factor")]
    Factor,
    #[serde(rename = "us")]
    Microseconds,
    #[serde(rename = "K")]
    Kelvin,
    #[serde(rename = "percent")]
    Percent,
    #[serde(rename = "s")]
    Seconds,
    #[serde(rename = "fraction")]
    Fraction,
    #[serde(rename = "fps")]
    Fps,
    #[serde(untagged)]
    Other(String),
}

/// A bound, step or precision: one number for `number`; for `range`, `pose`, `point` and
/// `box` an object of the same shape as the value, one number per component.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum Bound {
    One(Number),
    Each(BTreeMap<String, Number>),
}

impl Bound {
    /// The bound of one component (`None` for the whole value of a `number`).
    pub fn get(&self, component: Option<&str>) -> Option<f64> {
        match (self, component) {
            (Bound::One(n), _) => n.as_f64(),
            (Bound::Each(map), Some(c)) => map.get(c).and_then(Number::as_f64),
            (Bound::Each(_), None) => None,
        }
    }
}

/// A unit: one for the whole value, or one per component (a `pose` has degrees and a factor).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum UnitSpec {
    One(Unit),
    Each(BTreeMap<String, Unit>),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Asleep {
    Accepted,
    Ignored,
    Unknown,
    #[serde(untagged)]
    Other(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Warning {
    Restart,
    Destructive,
    #[serde(untagged)]
    Other(String),
}

/// A condition on another path, checked against the current state with the patch applied.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Requirement {
    pub path: String,
    pub equals: Value,
}

/// One leaf of the state, or one action parameter or result field (4.2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Setting {
    #[serde(rename = "type")]
    pub kind: SettingType,
    pub label: String,
    #[serde(default)]
    pub readonly: bool,
    #[serde(default)]
    pub nullable: bool,
    /// `false`: the camera accepts the setting but cannot report it.
    #[serde(default = "yes")]
    pub readback: bool,
    /// Present on every writable leaf; absent on read-only leaves and parameters.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub asleep: Option<Asleep>,
    #[serde(default)]
    pub no_effect: bool,
    /// enum: every value a state may show.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub values: Vec<String>,
    /// enum: the values a `set` accepts. Present on every writable enum, absent on read-only.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub writable: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min: Option<Bound>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max: Option<Bound>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub step: Option<Bound>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<UnitSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub precision: Option<Bound>,
    /// string: the maximum length in characters.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_len: Option<u32>,
    /// string: printable ASCII only (0x20 to 0x7e).
    #[serde(default, skip_serializing_if = "is_false")]
    pub ascii: bool,
    /// list: the fields of one item.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item: Option<BTreeMap<String, Setting>>,
    /// Action parameters only: the parameter may be left out.
    #[serde(default, skip_serializing_if = "is_false")]
    pub optional: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub requires: Vec<Requirement>,
    /// The camera's own default as it reports it (UVC).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<Value>,
    /// The live topic of a volatile leaf.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub live: Option<String>,
    /// The longest time the readback may lag a write of this leaf (4.1 rule 4).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub settle_ms: Option<u32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warning: Vec<Warning>,
}

/// One action of a device (4.2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Action {
    pub label: String,
    /// Parameter name → setting; empty for an action without parameters.
    #[serde(default)]
    pub params: BTreeMap<String, Setting>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<BTreeMap<String, Setting>>,
    pub asleep: Asleep,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub requires: Vec<Requirement>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warning: Vec<Warning>,
}

fn num(x: f64) -> Number {
    match number(x) {
        Value::Number(n) => n,
        _ => panic!("description bound {x} is not a finite number"),
    }
}

fn each(pairs: &[(&str, f64)]) -> Bound {
    Bound::Each(
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), num(*v)))
            .collect(),
    )
}

/// Builders for model descriptions. They never touch the wire format.
impl Setting {
    /// A writable setting of `kind` with no constraints.
    pub fn new(kind: SettingType, label: &str) -> Setting {
        Setting {
            kind,
            label: label.to_string(),
            readonly: false,
            nullable: false,
            readback: true,
            asleep: None,
            no_effect: false,
            values: Vec::new(),
            writable: Vec::new(),
            min: None,
            max: None,
            step: None,
            unit: None,
            precision: None,
            max_len: None,
            ascii: false,
            item: None,
            optional: false,
            requires: Vec::new(),
            default: None,
            live: None,
            settle_ms: None,
            warning: Vec::new(),
        }
    }

    pub fn boolean(label: &str) -> Setting {
        Setting::new(SettingType::Bool, label)
    }

    /// An enum whose values are all writable; narrow with [`Setting::writable_only`].
    pub fn enumeration(label: &str, values: &[&str]) -> Setting {
        let mut s = Setting::new(SettingType::Enum, label);
        s.values = values.iter().map(|v| v.to_string()).collect();
        s.writable = s.values.clone();
        s
    }

    /// A number between `min` and `max`.
    pub fn number(label: &str, min: f64, max: f64) -> Setting {
        Setting::new(SettingType::Number, label).min(min).max(max)
    }

    /// A string of at most `max_len` characters.
    pub fn string(label: &str, max_len: u32, ascii: bool) -> Setting {
        let mut s = Setting::new(SettingType::String, label);
        s.max_len = Some(max_len);
        s.ascii = ascii;
        s
    }

    /// A read-only list whose items have the given fields.
    pub fn list(label: &str, item: &[(&str, Setting)]) -> Setting {
        let mut s = Setting::new(SettingType::List, label).readonly();
        s.item = Some(
            item.iter()
                .map(|(k, v)| (k.to_string(), v.clone()))
                .collect(),
        );
        s
    }

    /// An object-shaped setting (`range`, `pose`, `point`, `box`) with per-component bounds.
    pub fn composite(
        kind: SettingType,
        label: &str,
        min: &[(&str, f64)],
        max: &[(&str, f64)],
    ) -> Setting {
        let mut s = Setting::new(kind, label);
        s.min = Some(each(min));
        s.max = Some(each(max));
        s
    }

    /// A `range` whose two ends both lie between `lo` and `hi`.
    pub fn range(label: &str, lo: f64, hi: f64) -> Setting {
        Setting::composite(
            SettingType::Range,
            label,
            &[("min", lo), ("max", lo)],
            &[("min", hi), ("max", hi)],
        )
    }

    pub fn min(mut self, x: f64) -> Setting {
        self.min = Some(Bound::One(num(x)));
        self
    }

    pub fn max(mut self, x: f64) -> Setting {
        self.max = Some(Bound::One(num(x)));
        self
    }

    pub fn step(mut self, x: f64) -> Setting {
        self.step = Some(Bound::One(num(x)));
        self
    }

    pub fn precision(mut self, x: f64) -> Setting {
        self.precision = Some(Bound::One(num(x)));
        self
    }

    pub fn precisions(mut self, pairs: &[(&str, f64)]) -> Setting {
        self.precision = Some(each(pairs));
        self
    }

    pub fn unit(mut self, unit: Unit) -> Setting {
        self.unit = Some(UnitSpec::One(unit));
        self
    }

    pub fn units(mut self, pairs: &[(&str, Unit)]) -> Setting {
        self.unit = Some(UnitSpec::Each(
            pairs
                .iter()
                .map(|(k, u)| (k.to_string(), u.clone()))
                .collect(),
        ));
        self
    }

    /// Narrows the writable values of an enum.
    pub fn writable_only(mut self, values: &[&str]) -> Setting {
        self.writable = values.iter().map(|v| v.to_string()).collect();
        self
    }

    /// Read-only: no writable values, no asleep behaviour, no settle bound.
    pub fn readonly(mut self) -> Setting {
        self.readonly = true;
        self.writable.clear();
        self.asleep = None;
        self.settle_ms = None;
        self
    }

    pub fn nullable(mut self) -> Setting {
        self.nullable = true;
        self
    }

    /// `readback: false`: the camera accepts the value but cannot report it; no settle bound.
    pub fn no_readback(mut self) -> Setting {
        self.readback = false;
        self.settle_ms = None;
        self
    }

    pub fn asleep(mut self, asleep: Asleep) -> Setting {
        self.asleep = Some(asleep);
        self
    }

    pub fn no_effect(mut self) -> Setting {
        self.no_effect = true;
        self
    }

    pub fn optional(mut self) -> Setting {
        self.optional = true;
        self
    }

    pub fn requires(mut self, path: &str, equals: Value) -> Setting {
        self.requires.push(Requirement {
            path: path.to_string(),
            equals,
        });
        self
    }

    pub fn default_value(mut self, value: Value) -> Setting {
        self.default = Some(value);
        self
    }

    pub fn live(mut self, topic: &str) -> Setting {
        self.live = Some(topic.to_string());
        self
    }

    pub fn settle_ms(mut self, ms: u32) -> Setting {
        self.settle_ms = Some(ms);
        self
    }

    pub fn warning(mut self, warning: Warning) -> Setting {
        self.warning.push(warning);
        self
    }
}

impl Action {
    pub fn new(label: &str, asleep: Asleep) -> Action {
        Action {
            label: label.to_string(),
            params: BTreeMap::new(),
            result: None,
            asleep,
            requires: Vec::new(),
            warning: Vec::new(),
        }
    }

    pub fn param(mut self, name: &str, setting: Setting) -> Action {
        self.params.insert(name.to_string(), setting);
        self
    }

    pub fn result(mut self, name: &str, setting: Setting) -> Action {
        self.result
            .get_or_insert_with(BTreeMap::new)
            .insert(name.to_string(), setting);
        self
    }

    pub fn requires(mut self, path: &str, equals: Value) -> Action {
        self.requires.push(Requirement {
            path: path.to_string(),
            equals,
        });
        self
    }

    pub fn warning(mut self, warning: Warning) -> Action {
        self.warning.push(warning);
        self
    }
}

impl Description {
    pub fn setting(mut self, path: &str, setting: Setting) -> Description {
        self.settings.insert(path.to_string(), setting);
        self
    }

    pub fn action(mut self, name: &str, action: Action) -> Description {
        self.actions.insert(name.to_string(), action);
        self
    }

    pub fn limit(mut self, list: &str, max_items: u32) -> Description {
        self.limits.insert(list.to_string(), max_items);
        self
    }

    /// Whether `path` is a group: a proper prefix (followed by `.`) of a setting path.
    pub fn is_group(&self, path: &str) -> bool {
        let prefix = format!("{path}.");
        self.settings.keys().any(|k| k.starts_with(&prefix))
    }
}
