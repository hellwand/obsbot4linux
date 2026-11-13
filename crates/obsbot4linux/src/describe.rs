// SPDX-License-Identifier: GPL-3.0-or-later

//! Human output of `describe` (design spec 6.2): a device entry, a setting or an action, one
//! line per setting and per action, every fact taken from the description. `--json` prints
//! the protocol objects instead and never goes through this module.

use crate::errors::device_line;
use obsbot_proto::{Action, Bound, Device, Setting, SettingType, UnitSpec};
use serde::Serialize;
use serde_json::Value;

/// The wire name of a protocol enum value (`range`, `deg/s`, `ignored`, `restart`).
fn wire<T: Serialize>(v: &T) -> String {
    match serde_json::to_value(v) {
        Ok(Value::String(s)) => s,
        Ok(other) => other.to_string(),
        Err(_) => "?".into(),
    }
}

fn bound(b: &Option<Bound>, component: Option<&str>) -> Option<String> {
    b.as_ref()
        .and_then(|b| b.get(component))
        .map(|x| obsbot_proto::values::number(x).to_string())
}

/// `lo..hi`, `≥ lo`, `≤ hi`, or nothing.
fn bounds(s: &Setting, component: Option<&str>) -> Option<String> {
    match (bound(&s.min, component), bound(&s.max, component)) {
        (Some(lo), Some(hi)) => Some(format!("{lo}..{hi}")),
        (Some(lo), None) => Some(format!("≥ {lo}")),
        (None, Some(hi)) => Some(format!("≤ {hi}")),
        (None, None) => None,
    }
}

/// A setting without its label: type, values or bounds, step, unit, then its flags.
pub fn setting_core(s: &Setting) -> String {
    let mut out = wire(&s.kind);
    let mut parts: Vec<String> = Vec::new();
    match &s.kind {
        SettingType::Enum => {
            let shown = if s.writable.is_empty() {
                &s.values
            } else {
                &s.writable
            };
            out.push(' ');
            out.push_str(&shown.join("|"));
            let extra: Vec<&String> = s.values.iter().filter(|v| !shown.contains(v)).collect();
            if !extra.is_empty() {
                let extra: Vec<&str> = extra.iter().map(|v| v.as_str()).collect();
                out.push_str(&format!(" (also shown: {})", extra.join("|")));
            }
        }
        SettingType::String => {
            if let Some(n) = s.max_len {
                out.push_str(&format!(" of at most {n} characters"));
            }
            if s.ascii {
                parts.push("printable ASCII".into());
            }
        }
        SettingType::List => {
            if let Some(item) = &s.item {
                let names: Vec<&str> = item.keys().map(String::as_str).collect();
                out.push_str(&format!(" of {}", names.join(", ")));
            }
        }
        kind => match kind.components() {
            Some(names) => {
                let each: Vec<String> = names
                    .iter()
                    .filter_map(|c| bounds(s, Some(c)).map(|b| format!("{c} {b}")))
                    .collect();
                if !each.is_empty() {
                    out.push(' ');
                    out.push_str(&each.join(", "));
                }
            }
            None => {
                if let Some(b) = bounds(s, None) {
                    out.push(' ');
                    out.push_str(&b);
                }
                if let Some(step) = bound(&s.step, None) {
                    parts.push(format!("step {step}"));
                }
            }
        },
    }
    match &s.unit {
        Some(UnitSpec::One(unit)) => parts.push(format!("unit {}", wire(unit))),
        Some(UnitSpec::Each(units)) => {
            let each: Vec<String> = units
                .iter()
                .map(|(c, u)| format!("{c} {}", wire(u)))
                .collect();
            parts.push(format!("unit {}", each.join(" ")));
        }
        None => {}
    }
    if s.readonly {
        parts.push("read-only".into());
    }
    if s.nullable {
        parts.push("nullable".into());
    }
    if !s.readback {
        parts.push("no readback".into());
    }
    if s.no_effect {
        parts.push("no effect".into());
    }
    if s.optional {
        parts.push("optional".into());
    }
    if let Some(asleep) = &s.asleep {
        parts.push(format!("asleep {}", wire(asleep)));
    }
    for r in &s.requires {
        parts.push(format!("requires {}={}", r.path, plain(&r.equals)));
    }
    if let Some(d) = &s.default {
        parts.push(format!("default {}", plain(d)));
    }
    if let Some(topic) = &s.live {
        parts.push(format!("live {topic}"));
    }
    if let Some(ms) = s.settle_ms {
        parts.push(format!("settle {ms} ms"));
    }
    for w in &s.warning {
        parts.push(format!("warning {}", wire(w)));
    }
    for p in parts {
        out.push_str(", ");
        out.push_str(&p);
    }
    out
}

/// A JSON value as text without the quotes of a string.
fn plain(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

/// A setting on one line, its label last.
pub fn setting_text(s: &Setting) -> String {
    format!("{} ({})", setting_core(s), s.label)
}

/// An action on one line: its parameters, asleep behaviour, conditions, warnings and result,
/// its label last.
pub fn action_text(a: &Action) -> String {
    let mut parts: Vec<String> = Vec::new();
    if a.params.is_empty() {
        parts.push("no parameters".into());
    }
    for (name, p) in &a.params {
        parts.push(format!("{name} ({})", setting_core(p)));
    }
    parts.push(format!("asleep {}", wire(&a.asleep)));
    for r in &a.requires {
        parts.push(format!("requires {}={}", r.path, plain(&r.equals)));
    }
    for w in &a.warning {
        parts.push(format!("warning {}", wire(w)));
    }
    if let Some(result) = &a.result {
        for (name, r) in result {
            parts.push(format!("result {name} ({})", setting_core(r)));
        }
    }
    format!("{} ({})", parts.join(", "), a.label)
}

/// The device entry: its line, firmware, video node, USB facts, then every setting, action
/// and limit.
pub fn device_text(d: &Device) -> String {
    let dash = |v: &Option<String>| v.clone().unwrap_or_else(|| "–".into());
    let mut out = format!("{}\n", device_line(d));
    out.push_str(&format!("firmware: {}\n", dash(&d.firmware)));
    out.push_str(&format!("video node: {}\n", dash(&d.video_node)));
    out.push_str(&format!(
        "usb: {}:{} {}, path {}\n",
        d.usb.vendor,
        d.usb.product,
        dash(&d.usb.product_name),
        d.usb.path
    ));
    let desc = &d.description;
    if desc.settings.is_empty() {
        out.push_str("settings: (none)\n");
    } else {
        out.push_str("settings:\n");
        for (path, s) in &desc.settings {
            out.push_str(&format!("  {path}: {}\n", setting_text(s)));
        }
    }
    if desc.actions.is_empty() {
        out.push_str("actions: (none)\n");
    } else {
        out.push_str("actions:\n");
        for (name, a) in &desc.actions {
            out.push_str(&format!("  {name}: {}\n", action_text(a)));
        }
    }
    if !desc.limits.is_empty() {
        out.push_str("limits:\n");
        for (path, n) in &desc.limits {
            out.push_str(&format!("  {path}: {n}\n"));
        }
    }
    out
}
#[cfg(test)]
mod tests {
    use super::*;
    use obsbot_proto::{Description, HelloOk, OkBody, Reply, ReplyBody, ServerMessage};

    const SESSION: &str = include_str!("../../obsbot-proto/tests/golden/session.jsonl");
    const TINY2LITE: &str = include_str!("../../../descriptions/tiny2lite.json");
    const FAKE_B: &str = include_str!("../../../descriptions/fake/fake-b.json");

    fn hello() -> HelloOk {
        let line = SESSION.lines().find_map(|l| l.strip_prefix("< ")).unwrap();
        match serde_json::from_str::<ServerMessage>(line).unwrap() {
            ServerMessage::Reply(Reply {
                body: ReplyBody::Ok(OkBody::Hello(h)),
                ..
            }) => h,
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn the_golden_device_entry_renders() {
        let h = hello();
        assert_eq!(
            device_text(&h.devices[0]),
            "…0001  Example Camera 0001  example\n\
             firmware: 1.2.3.4\n\
             video node: /dev/video2\n\
             usb: 3564:fa01 Example Camera, path 3-2.1\n\
             settings:\n\
             \x20 asleep: bool, read-only (Asleep)\n\
             \x20 gimbal.pan: number, unit deg, read-only, live pose (Pan)\n\
             \x20 led.brightness: number 0..3, step 1, asleep unknown, settle 3000 ms (LED brightness)\n\
             \x20 presets: list of name, slot, read-only (Presets)\n\
             actions:\n\
             \x20 preset.recall: slot (number 0..2, step 1), asleep unknown (Recall a preset)\n\
             limits:\n\
             \x20 presets: 3\n"
        );
        assert_eq!(
            device_text(&h.devices[1]),
            "port:3-2.4  USB 3564:fa02 3-2.4  unknown  unsupported\n\
             firmware: –\n\
             video node: /dev/video6\n\
             usb: 3564:fa02 –, path 3-2.4\n\
             settings: (none)\n\
             actions: (none)\n"
        );
    }

    #[test]
    fn the_tiny_2_lite_lines_show_every_constraint() {
        let d: Description = serde_json::from_str(TINY2LITE).unwrap();
        let s = |p: &str| setting_text(&d.settings[p]);
        let a = |n: &str| action_text(&d.actions[n]);
        assert_eq!(
            s("exposure.time"),
            "number 100..250000, unit us, asleep unknown, requires exposure.mode=manual, \
             default 33000, live auto, settle 3000 ms (Exposure time)"
        );
        assert_eq!(
            s("zoom.field_of_view"),
            "enum wide|medium|narrow (also shown: custom), asleep ignored, settle 3000 ms (Field of view)"
        );
        assert_eq!(
            s("zone.area.pan"),
            "range min -150..150, max -150..150, unit deg, asleep unknown, settle 3000 ms (Zone pan range)"
        );
        assert_eq!(
            s("led.blink"),
            "bool, no readback, asleep unknown (LED blink)"
        );
        assert_eq!(
            a("factory_reset"),
            "confirm (bool), asleep unknown, warning destructive, warning restart (Factory reset)"
        );
        assert_eq!(
            a("view.save"),
            "name (string of at most 32 characters, printable ASCII), \
             pose (pose pan -150..150, tilt -70..90, zoom 1..4, unit pan deg tilt deg zoom factor, optional), \
             asleep ignored, result id (number ≥ 0, step 1) (Save a view)"
        );
    }

    #[test]
    fn fake_b_lines_come_from_its_own_description() {
        let d: Description = serde_json::from_str(FAKE_B).unwrap();
        assert_eq!(
            setting_text(&d.settings["beacon.color"]),
            "enum red|green|blue, asleep accepted, settle 1000 ms (Beacon colour)"
        );
        assert_eq!(
            action_text(&d.actions["beacon.flash"]),
            "times (number 1..3, step 1), asleep accepted (Flash the beacon)"
        );
    }
}
