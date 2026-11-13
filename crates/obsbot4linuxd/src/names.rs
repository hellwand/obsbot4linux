// SPDX-License-Identifier: GPL-3.0-or-later

//! Device entries, first states and log lines (4.2, 5.4, 5.11). Human-facing text shows a
//! serial or an `sn:` id only through `obsbot_proto::short_id`; no log line prints an
//! identity answer.

use crate::identity::PortReason;
use obsbot_device::contract::{Found, ModelEntry};
use obsbot_proto::{Description, Device, State, Support, short_id};
use serde_json::Value;

/// The `name` of a supported device (4.2): the display name plus the last four characters of
/// the serial for an `sn:` id, else plus the USB path.
pub fn name_of(display: &str, id: &str, usb_path: &str) -> String {
    match id.strip_prefix("sn:") {
        Some(serial) => {
            let chars: Vec<char> = serial.chars().collect();
            let tail: String = chars[chars.len().saturating_sub(4)..].iter().collect();
            format!("{display} {tail}")
        }
        None => format!("{display} {usb_path}"),
    }
}

/// The entry of a supported device. `serial` is a valid serial or `None`.
pub fn supported_device(
    found: &Found,
    model: &ModelEntry,
    description: Description,
    id: &str,
    serial: Option<&str>,
    firmware: Option<&str>,
) -> Device {
    Device {
        id: id.to_string(),
        name: name_of(model.display_name, id, &found.usb.path),
        model: model.key.to_string(),
        support: model.support.clone(),
        serial: serial.map(String::from),
        firmware: firmware.map(String::from),
        video_node: found.video_node.clone(),
        available: true,
        replaced_by: None,
        usb: found.usb.clone(),
        description,
    }
}

/// The entry of an unsupported device (5.4): `port:` id, model `unknown`, no serial, no
/// firmware, empty settings, actions and limits; its name is the USB product string plus the
/// path, or `USB <vendor>:<product>` plus the path without a product string (4.2).
pub fn unsupported_device(found: &Found) -> Device {
    let usb = &found.usb;
    let name = match &usb.product_name {
        Some(product) => format!("{product} {}", usb.path),
        None => format!("USB {}:{} {}", usb.vendor, usb.product, usb.path),
    };
    Device {
        id: obsbot_proto::port_id(&usb.path),
        name,
        model: "unknown".into(),
        support: Support::Unsupported,
        serial: None,
        firmware: None,
        video_node: found.video_node.clone(),
        available: true,
        replaced_by: None,
        usb: usb.clone(),
        description: Description::default(),
    }
}

/// The state of a device before its first read: every leaf `null`, every leaf with a readback
/// in `unread` (4.2 "Absent vs null"); a `readback: false` leaf is `null` until written and is
/// never in `unread`, since it is never read.
pub fn initial_state(desc: &Description) -> State {
    let mut state = State::new(true);
    for (path, setting) in &desc.settings {
        state.set(path, Value::Null);
        if setting.readback {
            state.unread.push(path.clone());
        }
    }
    state
}

fn support_text(support: &Support) -> String {
    serde_json::to_value(support)
        .ok()
        .and_then(|v| v.as_str().map(String::from))
        .unwrap_or_default()
}

/// The longest foreign text a log line carries, in characters, the cut marker included.
const LOG_TEXT_CAP: usize = 120;

/// Text the daemon did not write (a client name, a device's answer, a sysfs name) made safe for
/// ONE log line: every control character (`char::is_control`: line breaks, carriage return,
/// escape, the C1 range) and the line and paragraph separators U+2028 and U+2029 become `?`,
/// and anything past [`LOG_TEXT_CAP`] characters is cut (never inside a character) and ends in
/// `…`. Ordinary text comes back unchanged.
pub fn printable(text: &str) -> String {
    let mut out = String::new();
    for (count, c) in text.chars().enumerate() {
        if count == LOG_TEXT_CAP {
            out.pop();
            out.push('…');
            break;
        }
        out.push(if c.is_control() || matches!(c, '\u{2028}' | '\u{2029}') {
            '?'
        } else {
            c
        });
    }
    out
}

/// 5.11: one line for every device that appears: id, model, USB ids, USB path, support level,
/// and for a `port:` id of a supported model the reason.
pub fn log_appeared(device: &Device, reason: Option<PortReason>) -> String {
    let mut line = format!(
        "obsbot4linuxd: device {} appeared: model {}, USB {}:{}, path {}, support {}",
        printable(&short_id(&device.id)),
        device.model,
        printable(&device.usb.vendor),
        printable(&device.usb.product),
        printable(&device.usb.path),
        support_text(&device.support)
    );
    if let Some(r) = reason {
        line.push_str(&format!(", port id: {}", r.text()));
    }
    line
}

/// 5.11: one line for a hand-over, with the old and the new id.
pub fn log_handover(old: &str, new: &str) -> String {
    format!(
        "obsbot4linuxd: device {} is now {}",
        printable(&short_id(old)),
        printable(&short_id(new))
    )
}

/// 5.11: one line for a device that goes.
pub fn log_gone(id: &str) -> String {
    format!(
        "obsbot4linuxd: device {} went away",
        printable(&short_id(id))
    )
}

/// A device whose camera could not be opened (it stays listed, every leaf unread).
pub fn log_open_failed(id: &str, error: &str) -> String {
    format!(
        "obsbot4linuxd: device {} could not be opened: {}",
        printable(&short_id(id)),
        printable(error)
    )
}

/// 4.2: a camera value the model cannot decode, logged once per device and value.
pub fn log_raw(id: &str, leaf: &str, value: &str) -> String {
    format!(
        "obsbot4linuxd: device {} reported {} = {}, which its model cannot decode",
        printable(&short_id(id)),
        printable(leaf),
        printable(value)
    )
}

/// 8.3: a frame outside the model's send table was refused before it was sent
/// (`CameraError::NotInSendTable`); the request is answered `internal`.
pub fn log_refused_frame(id: &str, message: &str) -> String {
    format!(
        "obsbot4linuxd: device {}: a frame outside the send table was refused and not sent: {}",
        printable(&short_id(id)),
        printable(message)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use obsbot_device::models::fake;
    use obsbot_proto::Setting;
    use serde_json::json;

    fn scan() -> Vec<Found> {
        use obsbot_device::contract::Bus;
        fake::FakeBus::new(&fake::KEYS).unwrap().scan()
    }

    #[test]
    fn names_follow_4_2() {
        assert_eq!(
            name_of("OBSBOT Tiny 2 Lite", "sn:EXAMPLE0000001", "3-2.1"),
            "OBSBOT Tiny 2 Lite 0001"
        );
        assert_eq!(name_of("Fake A", "port:9-1", "9-1"), "Fake A 9-1");
        let mut unknown = scan().remove(2);
        assert_eq!(unsupported_device(&unknown).name, "Unknown Camera 9-3");
        unknown.usb.product_name = None;
        assert_eq!(unsupported_device(&unknown).name, "USB 3564:fa00 9-3");
    }

    /// 5.4: the unsupported entry holds no serial, no firmware and nothing to set or do.
    #[test]
    fn the_unsupported_entry_is_empty() {
        let d = unsupported_device(&scan()[2]);
        assert_eq!(d.id, "port:9-3");
        assert_eq!(d.model, "unknown");
        assert_eq!(d.support, Support::Unsupported);
        assert_eq!((d.serial, d.firmware), (None, None));
        assert!(d.description.settings.is_empty() && d.description.actions.is_empty());
        assert!(d.available);
        assert_eq!(d.video_node.as_deref(), Some("/dev/video106"));
    }

    #[test]
    fn a_supported_entry_takes_the_model_and_the_answers() {
        let found = scan().remove(0);
        let model = found.model.unwrap();
        let d = supported_device(
            &found,
            model,
            fake::description("fake-a").unwrap(),
            "sn:FAKEA0000001",
            Some("FAKEA0000001"),
            Some("1.0.0.0"),
        );
        assert_eq!(d.name, "Fake A 0001");
        assert_eq!(d.model, "fake-a");
        assert_eq!(d.support, Support::Tested);
        assert_eq!(d.usb.path, "9-1");
        assert!(d.description.settings.contains_key("led.brightness"));
    }

    #[test]
    fn the_first_state_is_all_null_and_unread_except_readback_false() {
        let desc = Description::default()
            .setting("asleep", Setting::boolean("Asleep").readonly())
            .setting("led.blink", Setting::boolean("Blink").no_readback())
            .setting("zoom.factor", Setting::number("Zoom", 1.0, 4.0));
        let s = initial_state(&desc);
        assert_eq!(s.at("led.blink"), Some(&Value::Null));
        assert_eq!(s.at("zoom.factor"), Some(&Value::Null));
        assert_eq!(s.unread, ["asleep", "zoom.factor"]);
        assert!(s.pending.is_empty() && s.available);
        assert_eq!(
            serde_json::to_value(&s).unwrap(),
            json!({"available": true, "pending": {}, "unread": ["asleep", "zoom.factor"],
                   "asleep": null, "led": {"blink": null}, "zoom": {"factor": null}})
        );
    }

    /// 4.2, 5.11: logs show `…` and the last four characters, never a whole serial.
    #[test]
    fn log_lines_never_show_a_whole_serial() {
        let found = scan().remove(0);
        let d = supported_device(
            &found,
            found.model.unwrap(),
            Description::default(),
            "sn:FAKEA0000001",
            Some("FAKEA0000001"),
            None,
        );
        let lines = [
            log_appeared(&d, None),
            log_handover("port:9-1", "sn:FAKEA0000001"),
            log_gone("sn:FAKEA0000001"),
            log_open_failed("sn:FAKEA0000001", "gone"),
            log_raw("sn:FAKEA0000001", "tracking.mode", "raw:9"),
            log_refused_frame("sn:FAKEA0000001", "an example frame"),
        ];
        for line in &lines {
            assert!(!line.contains("FAKEA0000001"), "{line}");
        }
        assert_eq!(
            lines[0],
            "obsbot4linuxd: device …0001 appeared: model fake-a, USB 3564:fa0a, path 9-1, support tested"
        );
        assert_eq!(lines[1], "obsbot4linuxd: device port:9-1 is now …0001");
        assert_eq!(lines[2], "obsbot4linuxd: device …0001 went away");
        assert_eq!(
            lines[4],
            "obsbot4linuxd: device …0001 reported tracking.mode = raw:9, which its model cannot decode"
        );
        assert_eq!(
            lines[5],
            "obsbot4linuxd: device …0001: a frame outside the send table was refused and not sent: an example frame"
        );
        let port = Device {
            id: "port:9-1".into(),
            ..d
        };
        assert_eq!(
            log_appeared(&port, Some(PortReason::NoSerial)),
            "obsbot4linuxd: device port:9-1 appeared: model fake-a, USB 3564:fa0a, path 9-1, support tested, port id: no serial answered"
        );
        let unknown = unsupported_device(&scan()[2]);
        assert_eq!(
            log_appeared(&unknown, None),
            "obsbot4linuxd: device port:9-3 appeared: model unknown, USB 3564:fa00, path 9-3, support unsupported"
        );
    }

    /// Foreign text that would break a log line: line breaks, a carriage return, an escape
    /// sequence, U+2028, and a string far past the cap.
    fn hostile() -> Vec<String> {
        vec![
            "a\nb".into(),
            "a\rb".into(),
            "a\x1b[31mred".into(),
            "a\u{2028}b\u{2029}c\u{85}d".into(),
            "x".repeat(5000),
        ]
    }

    /// One line, no control character, and not longer than the fixed text plus the cap.
    fn assert_one_line(line: &str, fixed: usize) {
        assert!(
            !line.contains(['\n', '\r', '\u{2028}', '\u{2029}'])
                && !line.chars().any(char::is_control),
            "{line:?}"
        );
        assert!(line.chars().count() <= fixed + 5 * LOG_TEXT_CAP, "{line:?}");
    }

    /// The point of the issue: foreign text must not add lines or escape sequences to the log.
    #[test]
    fn printable_replaces_controls_and_caps_the_length() {
        assert_eq!(
            printable("a\nb\rc\x1bd\u{2028}e\u{2029}f\u{85}g"),
            "a?b?c?d?e?f?g"
        );
        let long = printable(&"x".repeat(5000));
        assert_eq!(long.chars().count(), LOG_TEXT_CAP);
        assert!(long.ends_with('…'));
        // Exactly at the cap nothing is cut; multi-byte characters are never split.
        assert_eq!(
            printable(&"x".repeat(LOG_TEXT_CAP)),
            "x".repeat(LOG_TEXT_CAP)
        );
        let wide = printable(&"ä".repeat(500));
        assert_eq!(wide.chars().count(), LOG_TEXT_CAP);
    }

    #[test]
    fn ordinary_text_is_unchanged() {
        for t in [
            "widget 1.0",
            "tracking.mode",
            "raw:9",
            "port:3-2.1",
            "Größe …0001",
        ] {
            assert_eq!(printable(t), t);
        }
    }

    #[test]
    fn foreign_text_in_log_lines_stays_on_one_line() {
        for h in hostile() {
            assert_one_line(&log_open_failed("sn:FAKEA0000001", &h), 120);
            assert_one_line(&log_raw("sn:FAKEA0000001", &h, "v"), 120);
            assert_one_line(&log_raw("sn:FAKEA0000001", "leaf", &h), 120);
            assert_one_line(&log_raw("sn:FAKEA0000001", &h, &h), 120);
            assert_one_line(&log_refused_frame("sn:FAKEA0000001", &h), 120);
            // A port id carries the sysfs path, so the id is foreign too.
            let port = format!("port:{h}");
            assert_one_line(&log_gone(&port), 120);
            assert_one_line(&log_handover(&port, &port), 120);
            assert_one_line(&log_open_failed(&port, "e"), 120);
            assert_one_line(&log_raw(&port, "l", "v"), 120);
            assert_one_line(&log_refused_frame(&port, "m"), 120);
            // A device entry from sysfs: path, vendor and product are read, not written.
            let mut found = scan().remove(0);
            found.usb.path = h.clone();
            found.usb.vendor = h.clone();
            found.usb.product = h.clone();
            let d = supported_device(
                &found,
                found.model.unwrap(),
                Description::default(),
                &format!("port:{h}"),
                None,
                None,
            );
            assert_one_line(&log_appeared(&d, Some(PortReason::NoSerial)), 160);
        }
    }
}
