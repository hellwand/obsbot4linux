// SPDX-License-Identifier: GPL-3.0-or-later

//! `set` and `do` against a scripted daemon (design spec 6.2): the one request each sends and
//! nothing after it (spec 2), the value syntax on the wire, `keep: true` for a continuing
//! action, and one test per error code the daemon sends, for its exit code.

mod common;

use common::{cli, script, socket, three};
use obsbot_proto::ErrorCode;
use obsbot_proto::client::script::ScriptServer;

#[test]
fn set_sends_one_patch_and_prints_nothing() {
    let server = ScriptServer::start(
        &socket("set"),
        &script(
            &three(),
            &[
                r#"> {"id":2,"device":"sn:FAKEA0000001","set":{"exposure.mode":"manual","exposure.time":20000,"zone.area.pan":{"min":-20,"max":20},"tracking.mode":"upper_body","image.mirror":true}}"#,
                r#"< {"id":2,"ok":{"rev":7}}"#,
            ],
        ),
    );
    let (code, out, err) = cli(
        Some(server.path()),
        &[
            "set",
            "--device",
            "fake-a",
            "exposure.mode=manual",
            "exposure.time=20000",
            "zone.area.pan=-20,20",
            "tracking.mode=upper-body",
            "image.mirror=on",
        ],
    );
    assert_eq!((code, out.as_str(), err.as_str()), (0, "", ""));
    assert_eq!(server.finish().unwrap().len(), 2, "hello and one set");
}

#[test]
fn set_json_prints_the_rev() {
    let server = ScriptServer::start(
        &socket("set-json"),
        &script(
            &three(),
            &[
                r#"> {"id":2,"device":"sn:FAKEB0000001","set":{"led.brightness":5}}"#,
                r#"< {"id":2,"ok":{"rev":9}}"#,
            ],
        ),
    );
    let (code, out, _) = cli(
        Some(server.path()),
        &["--json", "set", "led.brightness=5", "--device", "fake-b"],
    );
    assert_eq!((code, out.as_str()), (0, "{\"rev\":9}\n"));
    server.finish().unwrap();
}

/// Spec 6.2 and 8.4 step 5: an unsupported device addressed by set or do is refused before anything is sent (exit 3, `unsupported_device`).
#[test]
fn an_unsupported_device_is_refused_before_set_or_do() {
    for (n, args) in [
        vec!["set", "led.brightness=1", "--device", "unknown"],
        vec!["do", "sleep", "--device", "unknown"],
    ]
    .into_iter()
    .enumerate()
    {
        let server = ScriptServer::start(
            &socket(&format!("write-unsupported-{n}")),
            &script(&three(), &[]),
        );
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

/// A value that does not parse is a usage error found after hello; nothing else is sent.
#[test]
fn a_value_that_does_not_parse_sends_nothing() {
    let server = ScriptServer::start(&socket("set-usage"), &script(&three(), &[]));
    let (code, out, _) = cli(
        Some(server.path()),
        &["--json", "set", "zoom.factor=wide", "--device", "fake-a"],
    );
    assert_eq!(code, 2);
    assert_eq!(
        out,
        "{\"error\":{\"code\":\"usage\",\"message\":\"zoom.factor: 'wide' is not a number\"}}\n"
    );
    assert_eq!(server.finish().unwrap().len(), 1, "only hello");
}

/// Spec 2 and 6.2: `do gimbal.move` sends the move alone, no tracking switch before it.
#[test]
fn do_sends_one_action_with_its_parameters() {
    let server = ScriptServer::start(
        &socket("do"),
        &script(
            &three(),
            &[
                r#"> {"id":2,"device":"sn:FAKEA0000001","do":"gimbal.move","with":{"pan":-20,"tilt":5.5}}"#,
                r#"< {"id":2,"ok":{"rev":3}}"#,
            ],
        ),
    );
    let (code, out, _) = cli(
        Some(server.path()),
        &[
            "do",
            "gimbal.move",
            "pan=-20",
            "tilt=5.5",
            "--device",
            "fake-a",
        ],
    );
    assert_eq!((code, out.as_str()), (0, ""));
    assert_eq!(server.finish().unwrap().len(), 2, "hello and one do");
}

/// Spec 4.4 and 6.2: the CLI always sends keep:true with gimbal.turn.
#[test]
fn a_turn_is_sent_with_keep_true() {
    let server = ScriptServer::start(
        &socket("turn"),
        &script(
            &three(),
            &[
                r#"> {"id":2,"device":"sn:FAKEA0000001","do":"gimbal.turn","with":{"pan_speed":20,"keep":true}}"#,
                r#"< {"id":2,"ok":{"rev":3}}"#,
            ],
        ),
    );
    let (code, _, _) = cli(
        Some(server.path()),
        &["do", "gimbal.turn", "pan_speed=20", "--device", "fake-a"],
    );
    assert_eq!(code, 0);
    server.finish().unwrap();
}

/// Spec 4.4 and 12.2: `preset.save` without `name` sends no name; the CLI invents none.
#[test]
fn a_preset_saved_without_a_name_is_sent_without_one() {
    let server = ScriptServer::start(
        &socket("preset"),
        &script(
            &three(),
            &[
                r#"> {"id":2,"device":"sn:FAKEA0000001","do":"preset.save","with":{"slot":1}}"#,
                r#"< {"id":2,"ok":{"rev":3}}"#,
            ],
        ),
    );
    let (code, _, _) = cli(
        Some(server.path()),
        &["do", "preset.save", "slot=1", "--device", "fake-a"],
    );
    assert_eq!(code, 0);
    assert_eq!(server.finish().unwrap().len(), 2);
}

#[test]
fn an_action_result_is_printed() {
    let server = ScriptServer::start(
        &socket("result"),
        &script(
            &three(),
            &[
                r#"> {"id":2,"device":"sn:FAKEA0000001","do":"view.save","with":{"name":"Desk","pose":{"pan":0,"tilt":5,"zoom":1.2}}}"#,
                r#"< {"id":2,"ok":{"rev":3,"result":{"id":2}}}"#,
            ],
        ),
    );
    let (code, out, _) = cli(
        Some(server.path()),
        &[
            "do",
            "view.save",
            "name=Desk",
            "pose=0,5,1.2",
            "--device",
            "fake-a",
        ],
    );
    assert_eq!((code, out.as_str()), (0, "id: 2\n"));
    server.finish().unwrap();

    let server = ScriptServer::start(
        &socket("result-json"),
        &script(
            &three(),
            &[
                r#"> {"id":2,"device":"sn:FAKEA0000001","do":"view.save","with":{"name":"Desk"}}"#,
                r#"< {"id":2,"ok":{"rev":3,"result":{"id":2}}}"#,
            ],
        ),
    );
    let (code, out, _) = cli(
        Some(server.path()),
        &[
            "--json",
            "do",
            "view.save",
            "name=Desk",
            "--device",
            "fake-a",
        ],
    );
    assert_eq!(
        (code, out.as_str()),
        (0, "{\"rev\":3,\"result\":{\"id\":2}}\n")
    );
    server.finish().unwrap();
}

/// Parity row `factory-reset` without `--yes`: the daemon refuses with `invalid` (exit 2);
/// the CLI adds no confirmation of its own.
#[test]
fn a_destructive_action_without_confirm_is_the_daemons_invalid() {
    let server = ScriptServer::start(
        &socket("reset"),
        &script(
            &three(),
            &[
                r#"> {"id":2,"device":"sn:FAKEA0000001","do":"factory_reset"}"#,
                r#"< {"id":2,"error":{"code":"invalid","message":"factory_reset deletes data on the camera: send confirm: true"}}"#,
            ],
        ),
    );
    let (code, _, err) = cli(
        Some(server.path()),
        &["do", "factory_reset", "--device", "fake-a"],
    );
    assert_eq!(code, 2);
    assert_eq!(
        err,
        "obsbot4linux: factory_reset deletes data on the camera: send confirm: true\n"
    );
    server.finish().unwrap();
}

/// Spec 10.3 C: one test per error code, for its exit code; the daemon's code is kept.
#[test]
fn every_daemon_error_code_gives_its_exit_code() {
    let mut codes: Vec<(String, u8)> = ErrorCode::ALL
        .iter()
        .map(|c| {
            let exit = match c {
                ErrorCode::Internal => 1,
                ErrorCode::Invalid
                | ErrorCode::UnknownDevice
                | ErrorCode::Unsupported
                | ErrorCode::OutOfRange => 2,
                ErrorCode::Protocol => 4,
                _ => 3,
            };
            (c.as_str().to_string(), exit)
        })
        .collect();
    codes.push(("teapot".into(), 1));
    for (n, (code, exit)) in codes.iter().enumerate() {
        let reply =
            format!(r#"< {{"id":2,"error":{{"code":"{code}","message":"scripted {code}"}}}}"#);
        let server = ScriptServer::start(
            &socket(&format!("code-{n}")),
            &script(
                &three(),
                &[
                    r#"> {"id":2,"device":"sn:FAKEB0000001","set":{"led.brightness":1}}"#,
                    &reply,
                ],
            ),
        );
        let (got, out, _) = cli(
            Some(server.path()),
            &["--json", "set", "led.brightness=1", "--device", "fake-b"],
        );
        assert_eq!(got, *exit, "{code}");
        assert_eq!(
            out,
            format!("{{\"error\":{{\"code\":\"{code}\",\"message\":\"scripted {code}\"}}}}\n"),
            "{code}"
        );
        server.finish().unwrap();
    }
}

#[test]
fn keep_false_is_refused_before_the_request() {
    let server = ScriptServer::start(&socket("keep-false"), &script(&three(), &[]));
    let (code, _, err) = cli(
        Some(server.path()),
        &[
            "do",
            "gimbal.turn",
            "pan_speed=20",
            "keep=off",
            "--device",
            "fake-a",
        ],
    );
    assert_eq!(code, 2);
    assert!(err.contains("keep=true"), "{err}");
    assert_eq!(server.finish().unwrap().len(), 1);
}

/// Review focus: 0.13 habits type dashes in paths. Only enum values take `-` for `_`; a path
/// is sent as typed and the daemon answers it (`unsupported`, exit 2).
#[test]
fn a_dashed_path_is_sent_as_typed() {
    let server = ScriptServer::start(
        &socket("dashed-path"),
        &script(
            &three(),
            &[
                r#"> {"id":2,"device":"sn:FAKEA0000001","set":{"framing.zoom-speed":"fast"}}"#,
                r#"< {"id":2,"error":{"code":"unsupported","message":"framing.zoom-speed is not a setting of this device"}}"#,
            ],
        ),
    );
    let (code, _, err) = cli(
        Some(server.path()),
        &["set", "framing.zoom-speed=fast", "--device", "fake-a"],
    );
    assert_eq!(code, 2);
    assert_eq!(
        err,
        "obsbot4linux: framing.zoom-speed is not a setting of this device\n"
    );
    server.finish().unwrap();
}
