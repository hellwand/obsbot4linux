// SPDX-License-Identifier: GPL-3.0-or-later

//! Shared by the command-line tests: running the command line in-process, socket paths, and
//! the `hello` exchange of a daemon with three devices shaped like the fakes of spec 5.12
//! (`fake-a` with the Tiny 2 Lite's description, `fake-b`, and an unsupported `unknown`).

#![allow(dead_code)]

use obsbot_proto::{
    Description, Device, HelloOk, OkBody, Reply, ReplyBody, RequestId, RevState, State, Support,
    Usb,
};
use serde_json::json;
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

/// A socket path for one test, unique per name. The name is kept short: a Unix socket path
/// holds at most 107 bytes, and a daemon adds `.lock` and `.test` (5 bytes) to its path.
pub fn socket(name: &str) -> PathBuf {
    let path = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("c-{name}"));
    assert!(
        path.as_os_str().len() <= 100,
        "socket path too long for a Unix socket: {}",
        path.display()
    );
    path
}

/// Runs the command line in-process against `socket`: (exit code, stdout, stderr).
pub fn cli(socket: Option<&Path>, args: &[&str]) -> (u8, String, String) {
    let args: Vec<OsString> = std::iter::once("obsbot4linux")
        .chain(args.iter().copied())
        .map(OsString::from)
        .collect();
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let code = obsbot4linux::run(
        &args,
        socket.map(Path::to_path_buf),
        false,
        &mut out,
        &mut err,
    );
    (
        code,
        String::from_utf8(out).unwrap(),
        String::from_utf8(err).unwrap(),
    )
}

/// The `hello` line the command line sends (spec 6.2: `subscribe: false`).
pub fn hello_line() -> String {
    format!(
        r#"> {{"id":1,"hello":{{"protocol":2,"client":"obsbot4linux {}","subscribe":false,"live":[]}}}}"#,
        env!("CARGO_PKG_VERSION")
    )
}

fn description(file: &str) -> Description {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../descriptions")
        .join(file);
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn device(id: &str, name: &str, model: &str, support: Support, product: &str, port: u32) -> Device {
    let serial = id.strip_prefix("sn:").map(str::to_string);
    let description = match model {
        "fake-a" => description("tiny2lite.json"),
        "fake-b" => description("fake/fake-b.json"),
        _ => Description::default(),
    };
    Device {
        id: id.into(),
        name: name.into(),
        model: model.into(),
        support,
        firmware: serial.as_ref().map(|_| "1.0.0.0".to_string()),
        serial,
        video_node: Some(format!("/dev/video{}", 100 + 2 * port)),
        available: true,
        replaced_by: None,
        usb: Usb {
            vendor: "3564".into(),
            product: product.into(),
            product_name: Some(name.trim_end_matches(char::is_numeric).trim().into()),
            bcd_device: "0100".into(),
            path: format!("9-{port}"),
            serial_present: false,
            interfaces: vec!["0e".into(), "0e".into()],
            extension_units: vec![],
        },
        description,
    }
}

pub fn fake_a() -> Device {
    device(
        "sn:FAKEA0000001",
        "Fake A 0001",
        "fake-a",
        Support::Tested,
        "fa0a",
        1,
    )
}

pub fn fake_b() -> Device {
    device(
        "sn:FAKEB0000001",
        "Fake B 0001",
        "fake-b",
        Support::Untested,
        "fa0b",
        2,
    )
}

pub fn unknown() -> Device {
    let mut d = device(
        "port:9-3",
        "Unknown Camera 9-3",
        "unknown",
        Support::Unsupported,
        "fa00",
        3,
    );
    d.usb.product_name = Some("Unknown Camera".into());
    d
}

/// A small state for each device.
pub fn state_of(d: &Device) -> State {
    let mut s = State::new(d.available);
    match d.model.as_str() {
        "fake-a" => {
            s.set("zoom.factor", json!(1.5));
            s.set("tracking.mode", json!("normal"));
            s.set(
                "presets",
                json!([{"slot": 0, "name": "", "pose": {"pan": 0, "tilt": 0, "zoom": 1}}]),
            );
        }
        "fake-b" => {
            s.set("led.brightness", json!(2));
            s.set("beacon.color", json!("red"));
        }
        _ => {}
    }
    s
}

/// The `hello` reply line (`< …`) of a daemon listing `devices`.
pub fn hello_reply_line(devices: &[Device]) -> String {
    let states: BTreeMap<String, RevState> = devices
        .iter()
        .map(|d| {
            (
                d.id.clone(),
                RevState {
                    rev: 1,
                    state: state_of(d),
                },
            )
        })
        .collect();
    let reply = Reply {
        id: Some(RequestId::Int(1)),
        body: ReplyBody::Ok(OkBody::Hello(HelloOk {
            protocol: 2,
            daemon: "1.0.0".into(),
            devices: devices.to_vec(),
            states,
        })),
    };
    format!("< {}", serde_json::to_string(&reply).unwrap())
}

/// A script: the hello exchange for `devices`, then `lines`.
pub fn script(devices: &[Device], lines: &[&str]) -> String {
    let mut all = vec![hello_line(), hello_reply_line(devices)];
    all.extend(lines.iter().map(|l| l.to_string()));
    all.join("\n")
}

/// The three devices of the fake daemon.
pub fn three() -> Vec<Device> {
    vec![fake_a(), fake_b(), unknown()]
}
