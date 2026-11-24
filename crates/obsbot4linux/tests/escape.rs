// SPDX-License-Identifier: GPL-3.0-or-later

//! Issue #250: text from the daemon (names stored on the camera, device names, string
//! values, error texts) reaches a terminal only through `escape`. Each test lets a scripted
//! daemon send a string with an escape sequence, a bell, line breaks, U+2028 and U+202E to
//! one print site; a site that bypasses the helper leaks one of them and fails.

mod common;

use common::{cli, fake_a, fake_b, script, socket};
use obsbot_proto::client::script::ScriptServer;
use obsbot_proto::{Action, Asleep, Device, Setting};
use serde_json::Value;

/// As JSON text, and as the escaped form human output must show.
const HOSTILE_JSON: &str = r"\u001b[31mX\u0007\n\r\u2028\u202e";
const SHOWN: &str = r"\x1b[31mX\x07\x0a\x0d\u{2028}\u{202e}";

fn assert_safe(text: &str) {
    for c in text.chars() {
        assert!(
            c == '\n' || !(c.is_control() || ('\u{2028}'..='\u{202e}').contains(&c)),
            "{c:?} reached the terminal in {text:?}"
        );
    }
}

/// `fake-a` and `fake-b` with hostile names, models and firmware.
fn hostile_devices() -> Vec<Device> {
    let hostile: String = serde_json::from_str(&format!("\"{HOSTILE_JSON}\"")).unwrap();
    let (mut a, mut b) = (fake_a(), fake_b());
    a.name = hostile.clone();
    b.firmware = Some(hostile.clone());
    b.usb.vendor = hostile.clone();
    b.usb.product = hostile.clone();
    let d = std::mem::take(&mut b.description);
    b.description = d
        .setting(&hostile, Setting::string(&hostile, 8, false))
        .action(
            &format!("x{hostile}"),
            Action::new(&hostile, Asleep::Unknown),
        );
    vec![a, b]
}

#[test]
fn a_preset_name_from_get_and_status_is_shown_escaped() {
    let reply = format!(
        r#"< {{"id":2,"ok":{{"rev":4,"state":{{"available":true,"pending":{{}},"unread":[],"presets":[{{"slot":0,"name":"{HOSTILE_JSON}"}}]}}}}}}"#
    );
    for (name, args) in [
        ("esc-get", vec!["get", "presets", "--device", "fake-a"]),
        ("esc-status", vec!["status", "--device", "fake-a"]),
    ] {
        let get = if args[0] == "get" {
            r#"> {"id":2,"device":"sn:FAKEA0000001","get":["presets"]}"#
        } else {
            r#"> {"id":2,"device":"sn:FAKEA0000001","get":null}"#
        };
        let server =
            ScriptServer::start(&socket(name), &script(&hostile_devices(), &[get, &reply]));
        let (code, out, err) = cli(Some(server.path()), &args);
        assert_eq!((code, err.as_str()), (0, ""), "{name}");
        assert_safe(&out);
        assert!(
            out.contains(&format!("  - name {SHOWN}, slot 0\n")),
            "{out:?}"
        );
        // The device line of `status` carries the hostile device name, escaped too.
        if args[0] == "status" {
            assert!(
                out.starts_with(&format!("…0001  {SHOWN}  fake-a\n")),
                "{out:?}"
            );
        }
        server.finish().unwrap();
    }
}

#[test]
fn a_string_value_is_shown_escaped() {
    let reply = format!(
        r#"< {{"id":2,"ok":{{"rev":4,"state":{{"available":true,"pending":{{}},"unread":[],"beacon":{{"color":"{HOSTILE_JSON}"}}}}}}}}"#
    );
    let server = ScriptServer::start(
        &socket("esc-str"),
        &script(
            &hostile_devices(),
            &[
                r#"> {"id":2,"device":"sn:FAKEB0000001","get":["beacon.color"]}"#,
                &reply,
            ],
        ),
    );
    let (code, out, _) = cli(
        Some(server.path()),
        &["get", "beacon.color", "--device", "fake-b"],
    );
    assert_eq!(code, 0);
    assert_safe(&out);
    assert!(out.contains(&format!("color: {SHOWN}\n")), "{out:?}");
    server.finish().unwrap();
}

#[test]
fn devices_and_describe_show_names_and_firmware_escaped() {
    for (name, args) in [
        ("esc-devs", vec!["devices"]),
        ("esc-desc", vec!["describe", "--device", "fake-b"]),
    ] {
        let server = ScriptServer::start(&socket(name), &script(&hostile_devices(), &[]));
        let (code, out, err) = cli(Some(server.path()), &args);
        assert_eq!((code, err.as_str()), (0, ""), "{name}");
        assert_safe(&out);
        if args[0] == "devices" {
            assert!(
                out.contains(&format!("…0001  {SHOWN}  fake-a\n")),
                "{out:?}"
            );
        } else {
            assert!(out.contains(&format!("firmware: {SHOWN}\n")), "{out:?}");
            assert!(
                out.contains(&format!("usb: {SHOWN}:{SHOWN} ")),
                "usb ids: {out:?}"
            );
        }
        server.finish().unwrap();
    }
}

#[test]
fn a_daemon_error_is_shown_escaped_on_stderr() {
    let error = format!(
        r#"< {{"id":2,"error":{{"code":"camera","message":"{HOSTILE_JSON}","applied":["{HOSTILE_JSON}"],"failed":"{HOSTILE_JSON}"}}}}"#
    );
    let server = ScriptServer::start(
        &socket("esc-err"),
        &script(
            &hostile_devices(),
            &[
                r#"> {"id":2,"device":"sn:FAKEA0000001","set":{"zoom.factor":2}}"#,
                &error,
            ],
        ),
    );
    let (code, out, err) = cli(
        Some(server.path()),
        &["set", "--device", "fake-a", "zoom.factor=2"],
    );
    assert_eq!((code, out.as_str()), (3, ""));
    assert_safe(&err);
    assert_eq!(err.matches(SHOWN).count(), 3, "{err:?}");
    server.finish().unwrap();
}

/// `--json` keeps the text as the daemon sent it: the serializer, not the helper, escapes.
#[test]
fn json_output_carries_the_original_text() {
    let reply = format!(
        r#"< {{"id":2,"ok":{{"rev":4,"state":{{"available":true,"pending":{{}},"unread":[],"presets":[{{"slot":0,"name":"{HOSTILE_JSON}"}}]}}}}}}"#
    );
    let server = ScriptServer::start(
        &socket("esc-json"),
        &script(
            &hostile_devices(),
            &[
                r#"> {"id":2,"device":"sn:FAKEA0000001","get":null}"#,
                &reply,
            ],
        ),
    );
    let (code, out, _) = cli(
        Some(server.path()),
        &["--json", "status", "--device", "fake-a"],
    );
    assert_eq!(code, 0);
    let v: Value = serde_json::from_str(out.trim_end()).unwrap();
    let want: String = serde_json::from_str(&format!("\"{HOSTILE_JSON}\"")).unwrap();
    assert_eq!(v["state"]["presets"][0]["name"], want.as_str());
    assert_eq!(v["device"]["name"], want.as_str());
    assert!(!out.contains(SHOWN), "JSON must not hold the human escapes");
    server.finish().unwrap();
}

/// `describe <target>` shows the name and the setting or action line escaped.
#[test]
fn describe_of_one_target_is_shown_escaped() {
    let hostile: String = serde_json::from_str(&format!("\"{HOSTILE_JSON}\"")).unwrap();
    for (name, target, shown, label) in [
        ("esc-dset", hostile.clone(), SHOWN.to_string(), "string"),
        (
            "esc-dact",
            format!("x{hostile}"),
            format!("x{SHOWN}"),
            "no parameters",
        ),
    ] {
        let server = ScriptServer::start(&socket(name), &script(&hostile_devices(), &[]));
        let (code, out, err) = cli(
            Some(server.path()),
            &["describe", &target, "--device", "fake-b"],
        );
        assert_eq!((code, err.as_str()), (0, ""), "{name}");
        assert_safe(&out);
        assert!(out.starts_with(&format!("{shown}: ")), "{out:?}");
        assert!(out.contains(label), "{out:?}");
        assert!(out.trim_end().ends_with(&format!("({SHOWN})")), "{out:?}");
        server.finish().unwrap();
    }
}

/// `do`: the result's field name and an undescribed value (serde leaves U+2028 and U+202E
/// raw in its text) are shown escaped.
#[test]
fn a_do_result_is_shown_escaped() {
    let reply =
        format!(r#"< {{"id":2,"ok":{{"rev":3,"result":{{"{HOSTILE_JSON}":"{HOSTILE_JSON}"}}}}}}"#);
    let server = ScriptServer::start(
        &socket("esc-do"),
        &script(
            &hostile_devices(),
            &[
                r#"> {"id":2,"device":"sn:FAKEB0000001","do":"beacon.flash","with":{"times":2}}"#,
                &reply,
            ],
        ),
    );
    let (code, out, err) = cli(
        Some(server.path()),
        &["do", "beacon.flash", "times=2", "--device", "fake-b"],
    );
    assert_eq!((code, err.as_str()), (0, ""));
    assert_safe(&out);
    assert!(out.starts_with(&format!("{SHOWN}: ")), "{out:?}");
    assert!(out.contains(r"\u{2028}\u{202e}"), "{out:?}");
    server.finish().unwrap();
}
