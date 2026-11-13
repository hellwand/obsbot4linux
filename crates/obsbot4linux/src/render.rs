// SPDX-License-Identifier: GPL-3.0-or-later

//! Human output of states and device lists (design spec 6.2, "Output"): grouped by the first
//! path segment, with units, serials shortened (4.2), pending values marked `(pending)`.
//! Every label, unit and type comes from the device's description; nothing here knows a
//! model. `--json` never goes through this module.

use crate::errors::device_line;
use obsbot_proto::{Description, Device, Setting, SettingType, State, Unit, UnitSpec};
use serde_json::Value;

/// The suffix a unit adds to a number.
pub fn unit_suffix(unit: &Unit) -> String {
    match unit {
        Unit::Deg => "°".into(),
        Unit::DegPerSecond => "°/s".into(),
        Unit::Factor => "×".into(),
        Unit::Microseconds => " µs".into(),
        Unit::Kelvin => " K".into(),
        Unit::Percent => "%".into(),
        Unit::Seconds => " s".into(),
        Unit::Fraction => String::new(),
        Unit::Fps => " fps".into(),
        other => match serde_json::to_value(other) {
            Ok(Value::String(name)) => format!(" {name}"),
            _ => String::new(),
        },
    }
}

fn unit_of<'a>(setting: &'a Setting, component: Option<&str>) -> Option<&'a Unit> {
    match &setting.unit {
        Some(UnitSpec::One(unit)) => Some(unit),
        Some(UnitSpec::Each(map)) => component.and_then(|c| map.get(c)),
        None => None,
    }
}

fn scalar(setting: &Setting, component: Option<&str>, v: &Value) -> String {
    match v {
        Value::Null => "–".into(),
        Value::Bool(b) => match (setting.readonly, b) {
            (true, true) => "yes".into(),
            (true, false) => "no".into(),
            (false, true) => "on".into(),
            (false, false) => "off".into(),
        },
        Value::Number(n) => {
            let suffix = unit_of(setting, component)
                .map(unit_suffix)
                .unwrap_or_default();
            format!("{n}{suffix}")
        }
        Value::String(s) if s.is_empty() => "\"\"".into(),
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

/// One value of a setting on one line. `null` is `–`; a bool is `on`/`off` when writable and
/// `yes`/`no` when read-only; an object-shaped value lists its components in order
/// (`pan 12.5° tilt -3° zoom 1.5×`); a value of an unexpected shape, an undecodable `raw:<n>`
/// included, is shown as it is.
pub fn value_text(setting: &Setting, v: &Value) -> String {
    match (setting.kind.components(), v) {
        (Some(names), Value::Object(map)) => names
            .iter()
            .filter_map(|c| {
                map.get(*c)
                    .map(|x| format!("{c} {}", scalar(setting, Some(c), x)))
            })
            .collect::<Vec<_>>()
            .join(" "),
        _ => scalar(setting, None, v),
    }
}

/// One item of a list: its fields in name order, `field value`, joined by `, `. A `name`
/// that is empty shows as `(unnamed)`: display only, never written (spec 6.2, "Preset
/// names").
pub fn item_text(setting: &Setting, item: &Value) -> String {
    let Value::Object(fields) = item else {
        return item.to_string();
    };
    fields
        .iter()
        .map(|(name, v)| {
            let described = setting.item.as_ref().and_then(|i| i.get(name));
            let text = match (name.as_str(), v, described) {
                ("name", Value::String(s), _) if s.is_empty() => "(unnamed)".to_string(),
                (_, _, Some(field)) => value_text(field, v),
                (_, _, None) => v.to_string(),
            };
            format!("{name} {text}")
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// The state, grouped: `available`, the `unread` paths if any, then every leaf of the
/// description that the state holds, in path order, under a line with its first segment.
/// A pending path shows its pending value, marked `(pending)`; an unread path is marked
/// `(unread)`. Paths the state does not hold are left out.
pub fn state_text(desc: &Description, state: &State) -> String {
    let mut out = format!(
        "available: {}\n",
        if state.available { "yes" } else { "no" }
    );
    if !state.unread.is_empty() {
        out.push_str(&format!("unread: {}\n", state.unread.join(", ")));
    }
    let mut group: Option<&str> = None;
    for (path, setting) in &desc.settings {
        let pending = state.pending.get(path);
        let Some(value) = pending.or_else(|| state.at(path)) else {
            continue;
        };
        let (indent, name) = match path.split_once('.') {
            Some((first, rest)) => {
                if group != Some(first) {
                    out.push_str(first);
                    out.push('\n');
                    group = Some(first);
                }
                ("  ", rest)
            }
            None => {
                group = None;
                ("", path.as_str())
            }
        };
        let mut marks = String::new();
        if pending.is_some() {
            marks.push_str(" (pending)");
        }
        if state.unread.iter().any(|p| p == path) {
            marks.push_str(" (unread)");
        }
        match (&setting.kind, value) {
            (SettingType::List, Value::Array(items)) => {
                let empty = if items.is_empty() { " (none)" } else { "" };
                out.push_str(&format!("{indent}{name}:{empty}{marks}\n"));
                for item in items {
                    out.push_str(&format!("{indent}  - {}\n", item_text(setting, item)));
                }
            }
            _ => out.push_str(&format!(
                "{indent}{name}: {}{marks}\n",
                value_text(setting, value)
            )),
        }
    }
    out
}

/// `status` in human form: the device line, then the state.
pub fn status_text(device: &Device, state: &State) -> String {
    format!(
        "{}\n{}",
        device_line(device),
        state_text(&device.description, state)
    )
}

/// `devices` in human form: one line per device.
pub fn devices_text(devices: &[Device]) -> String {
    devices
        .iter()
        .map(|d| format!("{}\n", device_line(d)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use obsbot_proto::{HelloOk, OkBody, Reply, ReplyBody, ServerMessage};
    use serde_json::json;

    const SESSION: &str = include_str!("../../obsbot-proto/tests/golden/session.jsonl");
    const HOTPLUG: &str = include_str!("../../obsbot-proto/tests/golden/hotplug.jsonl");
    const FAKE_B: &str = include_str!("../../../descriptions/fake/fake-b.json");
    const TINY2LITE: &str = include_str!("../../../descriptions/tiny2lite.json");

    fn messages(text: &str) -> Vec<ServerMessage> {
        text.lines()
            .filter_map(|l| l.strip_prefix("< "))
            .map(|l| serde_json::from_str(l).unwrap())
            .collect()
    }

    fn hello() -> HelloOk {
        match messages(SESSION).into_iter().next().unwrap() {
            ServerMessage::Reply(Reply {
                body: ReplyBody::Ok(OkBody::Hello(h)),
                ..
            }) => h,
            other => panic!("{other:?}"),
        }
    }

    fn event_state(text: &str, rev: u64) -> State {
        messages(text)
            .into_iter()
            .find_map(|m| match m {
                ServerMessage::Event(obsbot_proto::Event::State { rev: r, state, .. })
                    if r == rev =>
                {
                    Some(state)
                }
                _ => None,
            })
            .unwrap()
    }

    /// Spec 10.3 C: rendering snapshots from the golden states.
    #[test]
    fn the_golden_hello_renders_grouped_with_units() {
        let h = hello();
        let device = &h.devices[0];
        let state = &h.states[&device.id].state;
        assert_eq!(
            status_text(device, state),
            "…0001  Example Camera 0001  example\n\
             available: yes\n\
             asleep: no\n\
             gimbal\n  pan: 12.5°\n\
             led\n  brightness: 1\n\
             presets:\n  - name Desk, slot 0\n"
        );
    }

    #[test]
    fn a_pending_value_is_shown_and_marked() {
        let h = hello();
        let state = event_state(SESSION, 5);
        assert_eq!(
            state_text(&h.devices[0].description, &state),
            "available: yes\nasleep: no\ngimbal\n  pan: 12.5°\nled\n  brightness: 2 (pending)\n\
             presets:\n  - name Desk, slot 0\n"
        );
    }

    #[test]
    fn an_unplugged_device_and_an_unsupported_entry_render() {
        let h = hello();
        let state = event_state(HOTPLUG, 8);
        assert!(state_text(&h.devices[0].description, &state).starts_with("available: no\n"));
        let unknown = &h.devices[1];
        assert_eq!(
            status_text(unknown, &h.states[&unknown.id].state),
            "port:3-2.4  USB 3564:fa02 3-2.4  unknown  unsupported\navailable: yes\n"
        );
        assert_eq!(
            devices_text(&h.devices),
            "…0001  Example Camera 0001  example\nport:3-2.4  USB 3564:fa02 3-2.4  unknown  unsupported\n"
        );
    }

    #[test]
    fn null_unread_raw_and_unnamed_presets_render_as_display_only() {
        let desc: Description = serde_json::from_str(TINY2LITE).unwrap();
        let mut state = State::new(true);
        state.set("hand.return", Value::Null);
        state.set("exposure.anti_flicker", json!("raw:7"));
        state.set("zoom.factor", json!(1.5));
        state.set(
            "presets",
            json!([{"slot": 0, "name": "", "pose": {"pan": 10, "tilt": -5, "zoom": 1}}]),
        );
        state.set("zone.area.pan", json!({"min": -20, "max": 20}));
        state.unread.push("zoom.factor".into());
        assert_eq!(
            state_text(&desc, &state),
            "available: yes\n\
             unread: zoom.factor\n\
             exposure\n  anti_flicker: raw:7\n\
             hand\n  return: –\n\
             presets:\n  - name (unnamed), pose pan 10° tilt -5° zoom 1×, slot 0\n\
             zone\n  area.pan: min -20° max 20°\n\
             zoom\n  factor: 1.5× (unread)\n"
        );
    }

    /// fake-b: a group no client knows renders like any other; units from the description.
    #[test]
    fn a_description_of_another_shape_renders_without_model_knowledge() {
        let desc: Description = serde_json::from_str(FAKE_B).unwrap();
        let mut state = State::new(true);
        state.set("beacon.color", json!("green"));
        state.set("exposure.time", json!(2000));
        state.set("presets", json!([]));
        state.set("on_air.active", json!(true));
        state.set("on_air.apps", json!([{"name": "zoom"}]));
        assert_eq!(
            state_text(&desc, &state),
            "available: yes\n\
             beacon\n  color: green\n\
             exposure\n  time: 2000 µs\n\
             on_air\n  active: yes\n  apps:\n    - name zoom\n\
             presets: (none)\n"
        );
    }

    #[test]
    fn every_unit_has_a_suffix() {
        let units = [
            (Unit::Deg, "°"),
            (Unit::DegPerSecond, "°/s"),
            (Unit::Factor, "×"),
            (Unit::Microseconds, " µs"),
            (Unit::Kelvin, " K"),
            (Unit::Percent, "%"),
            (Unit::Seconds, " s"),
            (Unit::Fraction, ""),
            (Unit::Fps, " fps"),
            (Unit::Other("lux".into()), " lux"),
        ];
        for (unit, suffix) in units {
            assert_eq!(unit_suffix(&unit), suffix);
        }
    }
}
