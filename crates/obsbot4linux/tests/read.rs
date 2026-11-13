// SPDX-License-Identifier: GPL-3.0-or-later

//! `status`, `get`, `describe` and `devices` against a scripted daemon (design spec 6.2):
//! the one request each sends (the script fails on any further line), device selection,
//! the refusal of an unsupported device before anything is sent, the `--json` shapes, and
//! what an unreachable or older daemon gives.

mod common;

use common::{cli, fake_a, fake_b, hello_line, script, socket, three, unknown};
use obsbot_proto::client::script::ScriptServer;
use serde_json::{Value, json};

fn parse(out: &str) -> Value {
    serde_json::from_str(out.trim_end()).unwrap_or_else(|e| panic!("{e}: {out}"))
}

#[test]
fn status_sends_one_get_for_the_full_state() {
    let server = ScriptServer::start(
        &socket("status"),
        &script(
            &three(),
            &[
                r#"> {"id":2,"device":"sn:FAKEA0000001","get":null}"#,
                r#"< {"id":2,"ok":{"rev":4,"state":{"available":true,"pending":{"zoom.factor":2},"unread":[],"zoom":{"factor":1.5},"tracking":{"mode":"normal"}}}}"#,
            ],
        ),
    );
    let (code, out, err) = cli(Some(server.path()), &["status", "--device", "fake-a"]);
    assert_eq!((code, err.as_str()), (0, ""));
    assert_eq!(
        out,
        "…0001  Fake A 0001  fake-a\n\
         available: yes\n\
         tracking\n  mode: normal\n\
         zoom\n  factor: 2× (pending)\n"
    );
    assert_eq!(server.finish().unwrap().len(), 2);
}

/// Spec 6.2: `status --json` is `{"device":<Device>,"rev","state"}` with the full id.
#[test]
fn status_json_is_device_rev_and_state() {
    let server = ScriptServer::start(
        &socket("status-json"),
        &script(
            &three(),
            &[
                r#"> {"id":2,"device":"sn:FAKEB0000001","get":null}"#,
                r#"< {"id":2,"ok":{"rev":3,"state":{"available":true,"pending":{},"unread":[],"led":{"brightness":2}}}}"#,
            ],
        ),
    );
    let (code, out, _) = cli(
        Some(server.path()),
        &["--json", "status", "--device", "B0000001"],
    );
    assert_eq!(code, 0);
    let v = parse(&out);
    assert_eq!(v["device"]["id"], "sn:FAKEB0000001");
    assert_eq!(v["device"]["serial"], "FAKEB0000001");
    assert_eq!(v["rev"], 3);
    assert_eq!(v["state"]["led"]["brightness"], 2);
    let keys: Vec<&String> = v.as_object().unwrap().keys().collect();
    assert_eq!(keys, ["device", "rev", "state"]);
    assert!(
        out.starts_with(r#"{"device":{"id":"sn:FAKEB0000001","#),
        "{out}"
    );
    server.finish().unwrap();
}

#[test]
fn get_sends_the_paths_as_given_and_prints_the_ok_payload() {
    let server = ScriptServer::start(
        &socket("get"),
        &script(
            &three(),
            &[
                r#"> {"id":2,"device":"sn:FAKEA0000001","get":["zoom.factor","presets"]}"#,
                r#"< {"id":2,"ok":{"rev":5,"state":{"available":true,"pending":{},"unread":[],"zoom":{"factor":1.5},"presets":[{"slot":0,"name":"","pose":{"pan":0,"tilt":0,"zoom":1}}]}}}"#,
            ],
        ),
    );
    let (code, out, _) = cli(
        Some(server.path()),
        &[
            "get",
            "zoom.factor",
            "presets",
            "--device",
            "fake-a",
            "--json",
        ],
    );
    assert_eq!(code, 0);
    // The ok payload exactly, the empty name as the camera reported it (spec 6.2).
    assert_eq!(
        out,
        "{\"rev\":5,\"state\":{\"available\":true,\"pending\":{},\"unread\":[],\"presets\":[{\"name\":\"\",\"pose\":{\"pan\":0,\"tilt\":0,\"zoom\":1},\"slot\":0}],\"zoom\":{\"factor\":1.5}}}\n"
    );
    server.finish().unwrap();
}

/// Spec 6.2, "Preset names": a placeholder for an empty name, as display only.
#[test]
fn get_in_human_form_shows_an_unnamed_preset_with_a_placeholder() {
    let server = ScriptServer::start(
        &socket("get-human"),
        &script(
            &three(),
            &[
                r#"> {"id":2,"device":"sn:FAKEA0000001","get":["presets"]}"#,
                r#"< {"id":2,"ok":{"rev":5,"state":{"available":true,"pending":{},"unread":[],"presets":[{"slot":0,"name":"","pose":{"pan":0,"tilt":0,"zoom":1}}]}}}"#,
            ],
        ),
    );
    let (code, out, _) = cli(
        Some(server.path()),
        &["get", "presets", "--device", "fake-a"],
    );
    assert_eq!(code, 0);
    assert_eq!(
        out,
        "available: yes\npresets:\n  - name (unnamed), pose pan 0° tilt 0° zoom 1×, slot 0\n"
    );
    server.finish().unwrap();
}

/// The daemon answers a path it does not have; its code and exit code stay.
#[test]
fn get_of_an_unknown_path_is_the_daemons_unsupported() {
    let server = ScriptServer::start(
        &socket("get-unknown"),
        &script(
            &three(),
            &[
                r#"> {"id":2,"device":"sn:FAKEB0000001","get":["gimbal.pan"]}"#,
                r#"< {"id":2,"error":{"code":"unsupported","message":"gimbal.pan is not a path of this device"}}"#,
            ],
        ),
    );
    let (code, out, _) = cli(
        Some(server.path()),
        &["--json", "get", "gimbal.pan", "--device", "fake-b"],
    );
    assert_eq!(code, 2);
    assert_eq!(
        out,
        "{\"error\":{\"code\":\"unsupported\",\"message\":\"gimbal.pan is not a path of this device\"}}\n"
    );
    server.finish().unwrap();
}

/// Spec 6.2 and 8.4 step 5: an unsupported device addressed by status or get is refused before anything is sent (exit 3, `unsupported_device`).
#[test]
fn an_unsupported_device_is_refused_before_anything_is_sent() {
    for (n, args) in [
        vec!["status", "--device", "unknown"],
        vec!["get", "zoom", "--device", "port:9-3"],
    ]
    .into_iter()
    .enumerate()
    {
        let server =
            ScriptServer::start(&socket(&format!("unsupported-{n}")), &script(&three(), &[]));
        let (code, out, err) = cli(Some(server.path()), &args);
        assert_eq!(code, 3, "{args:?}");
        assert_eq!(out, "");
        assert!(
            err.starts_with(&format!(
                "obsbot4linux: Unknown Camera 9-3 (USB 3564:fa00) is not supported by obsbot4linux {}. \
                 `obsbot4linux devices --report` prints what an issue report needs: \
                 https://github.com/hellwand/obsbot4linux/issues",
                env!("CARGO_PKG_VERSION")
            )),
            "{err}"
        );
        assert_eq!(server.finish().unwrap().len(), 1, "only hello: {args:?}");
    }
}

#[test]
fn two_usable_devices_without_device_are_ambiguous() {
    let server = ScriptServer::start(&socket("ambiguous"), &script(&three(), &[]));
    let (code, out, _) = cli(Some(server.path()), &["status", "--json"]);
    assert_eq!(code, 2);
    assert_eq!(
        out,
        "{\"error\":{\"code\":\"ambiguous_device\",\"devices\":[\"sn:FAKEA0000001\",\"sn:FAKEB0000001\"],\"message\":\"2 usable devices: choose one with --device\"}}\n"
    );
    server.finish().unwrap();
}

/// Spec 8.4 step 5: with a supported camera connected too, it is used without `--device`.
#[test]
fn the_only_usable_device_is_used_beside_an_unsupported_one() {
    let server = ScriptServer::start(
        &socket("only-usable"),
        &script(
            &[fake_a(), unknown()],
            &[
                r#"> {"id":2,"device":"sn:FAKEA0000001","get":["zoom.factor"]}"#,
                r#"< {"id":2,"ok":{"rev":2,"state":{"available":true,"pending":{},"unread":[],"zoom":{"factor":1.5}}}}"#,
            ],
        ),
    );
    let (code, out, _) = cli(Some(server.path()), &["get", "zoom.factor"]);
    assert_eq!(code, 0);
    assert_eq!(out, "available: yes\nzoom\n  factor: 1.5×\n");
    server.finish().unwrap();
}

#[test]
fn only_an_unsupported_device_is_no_device_with_the_list_and_the_hint() {
    let server = ScriptServer::start(&socket("no-device"), &script(&[unknown()], &[]));
    let (code, out, err) = cli(Some(server.path()), &["status"]);
    assert_eq!(code, 3);
    assert_eq!(out, "");
    assert_eq!(
        err,
        "obsbot4linux: no usable OBSBOT camera\n  \
         port:9-3  Unknown Camera 9-3  unknown  unsupported\n\
         `obsbot4linux devices --report` prints what an issue report needs: \
         https://github.com/hellwand/obsbot4linux/issues\n"
    );
    server.finish().unwrap();
}

#[test]
fn a_device_nobody_has_is_unknown_device() {
    let server = ScriptServer::start(&socket("unknown-device"), &script(&three(), &[]));
    let (code, out, _) = cli(
        Some(server.path()),
        &["--json", "status", "--device", "fake-z"],
    );
    assert_eq!(code, 2);
    assert_eq!(parse(&out)["error"]["code"], "unknown_device");
    server.finish().unwrap();
}

/// Spec 6.2: `describe` prints the device entry; with a path or action, that object.
#[test]
fn describe_json_prints_the_device_entry_setting_or_action() {
    let server = ScriptServer::start(&socket("describe"), &script(&three(), &[]));
    let (code, out, _) = cli(
        Some(server.path()),
        &["describe", "--device", "fake-b", "--json"],
    );
    assert_eq!(code, 0);
    assert_eq!(parse(&out), serde_json::to_value(fake_b()).unwrap());
    server.finish().unwrap();

    let server = ScriptServer::start(&socket("describe-setting"), &script(&three(), &[]));
    let (code, out, _) = cli(
        Some(server.path()),
        &["describe", "beacon.color", "--device", "fake-b", "--json"],
    );
    assert_eq!(code, 0);
    assert_eq!(
        parse(&out),
        serde_json::to_value(&fake_b().description.settings["beacon.color"]).unwrap()
    );
    server.finish().unwrap();

    let server = ScriptServer::start(&socket("describe-action"), &script(&three(), &[]));
    let (code, out, _) = cli(
        Some(server.path()),
        &["describe", "beacon.flash", "--device", "fake-b", "--json"],
    );
    assert_eq!(code, 0);
    assert_eq!(
        parse(&out),
        serde_json::to_value(&fake_b().description.actions["beacon.flash"]).unwrap()
    );
    server.finish().unwrap();
}

/// Spec 6.2: a path or action not in the description exits 2 with `unsupported`.
#[test]
fn describe_of_something_the_device_lacks_is_unsupported() {
    let server = ScriptServer::start(&socket("describe-missing"), &script(&three(), &[]));
    let (code, out, _) = cli(
        Some(server.path()),
        &["--json", "describe", "gimbal.pan", "--device", "fake-b"],
    );
    assert_eq!(code, 2);
    assert_eq!(
        out,
        "{\"error\":{\"code\":\"unsupported\",\"message\":\"gimbal.pan is not a setting or an action of Fake B 0001\"}}\n"
    );
    server.finish().unwrap();
}

/// Spec 6.2: describe may address an unsupported device; its entry has empty settings.
#[test]
fn describe_prints_an_unsupported_entry() {
    let server = ScriptServer::start(&socket("describe-unknown"), &script(&three(), &[]));
    let (code, out, _) = cli(Some(server.path()), &["describe", "--device", "unknown"]);
    assert_eq!(code, 0);
    assert_eq!(
        out,
        "port:9-3  Unknown Camera 9-3  unknown  unsupported\n\
         firmware: –\n\
         video node: /dev/video106\n\
         usb: 3564:fa00 Unknown Camera, path 9-3\n\
         settings: (none)\n\
         actions: (none)\n"
    );
    server.finish().unwrap();
}

/// Spec 6.2: `devices --json` is `{"devices":[<Device>…]}`, the list of the hello reply.
#[test]
fn devices_json_is_the_hello_list() {
    let server = ScriptServer::start(&socket("devices-json"), &script(&three(), &[]));
    let (code, out, _) = cli(Some(server.path()), &["devices", "--json"]);
    assert_eq!(code, 0);
    assert_eq!(
        parse(&out),
        json!({ "devices": serde_json::to_value(three()).unwrap() })
    );
    server.finish().unwrap();
}

/// Spec 8.6: the support level next to every device that is not tested.
#[test]
fn devices_in_human_form_marks_untested_and_unsupported() {
    let server = ScriptServer::start(&socket("devices"), &script(&three(), &[]));
    let (code, out, _) = cli(Some(server.path()), &["devices"]);
    assert_eq!(code, 0);
    assert_eq!(
        out,
        "…0001  Fake A 0001  fake-a\n\
         …0001  Fake B 0001  fake-b  untested\n\
         port:9-3  Unknown Camera 9-3  unknown  unsupported\n"
    );
    server.finish().unwrap();
}

/// Spec 6.2: `devices --report` prints JSON in both modes, with no id and no serial value.
#[test]
fn devices_report_has_usb_facts_and_no_serial() {
    let server = ScriptServer::start(&socket("report"), &script(&three(), &[]));
    let (code, out, _) = cli(
        Some(server.path()),
        &["devices", "--report", "--device", "fake-a"],
    );
    assert_eq!(code, 0);
    let v = parse(&out);
    assert_eq!(
        v,
        json!({"devices": [{
            "model": "fake-a",
            "support": "tested",
            "firmware": "1.0.0.0",
            "video_node": "/dev/video102",
            "usb": serde_json::to_value(&fake_a().usb).unwrap(),
        }]})
    );
    assert!(!out.contains("FAKEA0000001"), "{out}");
    assert!(!out.contains("\"id\""), "{out}");
    server.finish().unwrap();
}

/// Parity exception PX-5: `devices` exits 3 (`no_device`) when the list is empty.
#[test]
fn devices_with_an_empty_list_is_no_device() {
    let server = ScriptServer::start(&socket("devices-empty"), &script(&[], &[]));
    let (code, out, _) = cli(Some(server.path()), &["devices", "--json"]);
    assert_eq!(code, 3);
    assert_eq!(parse(&out)["error"]["code"], "no_device");
    server.finish().unwrap();
}

/// Spec 5.10 and 10.3 C: no socket gives exit 4 and the systemctl line.
#[test]
fn no_socket_is_exit_4_with_the_enable_line() {
    let path = socket("absent");
    let _ = std::fs::remove_file(&path);
    let (code, out, err) = cli(Some(&path), &["status"]);
    assert_eq!(code, 4);
    assert_eq!(out, "");
    assert!(
        err.trim_end()
            .ends_with("start it with: systemctl --user enable --now obsbot4linux.socket"),
        "{err}"
    );
    let (code, _, err) = cli(None, &["devices"]);
    assert_eq!(code, 4);
    assert!(
        err.contains("systemctl --user enable --now obsbot4linux.socket"),
        "{err}"
    );
}

/// Spec 5.10 and 10.3 C: a non-protocol-2 hello reply gives exit 4 and the restart message.
#[test]
fn an_older_daemon_is_exit_4_with_the_restart_message() {
    let server = ScriptServer::start(
        &socket("old"),
        &[
            hello_line(),
            r#"< {"id":null,"ok":false,"error":"bad request: missing field `args` at line 1 column 83"}"#.to_string(),
        ]
        .join("\n"),
    );
    let (code, out, err) = cli(Some(server.path()), &["devices"]);
    assert_eq!(code, 4);
    assert_eq!(out, "");
    assert_eq!(
        err,
        "obsbot4linux: an obsbot4linux daemon older than 1.0 is running: restart it or log out\n"
    );
    server.finish().unwrap();
}

#[test]
fn a_daemon_of_another_protocol_is_exit_4_with_its_number() {
    let server = ScriptServer::start(
        &socket("protocol"),
        &[
            hello_line(),
            r#"< {"id":null,"error":{"code":"protocol","message":"protocol 2 is not supported","supported":[3]}}"#.to_string(),
            "!close".to_string(),
        ]
        .join("\n"),
    );
    let (code, out, _) = cli(Some(server.path()), &["--json", "devices"]);
    assert_eq!(code, 4);
    assert_eq!(
        out,
        "{\"error\":{\"code\":\"protocol\",\"message\":\"the obsbot4linux daemon speaks protocol 3, this client speaks 2: install the same version of both and restart the daemon\"}}\n"
    );
    server.finish().unwrap();
}
