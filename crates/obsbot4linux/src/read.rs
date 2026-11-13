// SPDX-License-Identifier: GPL-3.0-or-later

//! The reading commands (design spec 6.2): `status` and `get` send one `get`; `describe` and
//! `devices` send nothing after `hello`, whose reply holds the device list.

use crate::errors::{CliCode, CliError};
use crate::select::{report_hint, select};
use crate::{Ctx, describe as text, render, session};
use obsbot_proto::{Device, ErrorCode, RevState, Support, Usb};
use serde::Serialize;

fn to_json<T: Serialize>(value: &T) -> String {
    serde_json::to_string(value).expect("protocol objects serialise")
}

fn print(ctx: &mut Ctx, text: &str) {
    let _ = ctx.out.write_all(text.as_bytes());
}

/// `status`: `get` with `null`. `--json`: `{"device":<Device>,"rev","state"}`.
pub fn status(ctx: &mut Ctx) -> Result<(), CliError> {
    let (mut client, device) =
        session::connect_device(ctx.socket.as_deref(), ctx.device.as_deref())?;
    let (rev, state) = client
        .get(&device.id, None)
        .map_err(session::client_error)?;
    let out = if ctx.json {
        format!(
            "{{\"device\":{},\"rev\":{rev},\"state\":{}}}\n",
            to_json(&device),
            to_json(&state)
        )
    } else {
        render::status_text(&device, &state)
    };
    print(ctx, &out);
    Ok(())
}

/// `get <path>…`: one `get` with the paths as given; the daemon answers paths it does not
/// have. `--json`: the `ok` payload `{"rev","state"}`.
pub fn get(ctx: &mut Ctx, paths: &[String]) -> Result<(), CliError> {
    let (mut client, device) =
        session::connect_device(ctx.socket.as_deref(), ctx.device.as_deref())?;
    let paths: Vec<&str> = paths.iter().map(String::as_str).collect();
    let (rev, state) = client
        .get(&device.id, Some(&paths))
        .map_err(session::client_error)?;
    let out = if ctx.json {
        format!("{}\n", to_json(&RevState { rev, state }))
    } else {
        render::state_text(&device.description, &state)
    };
    print(ctx, &out);
    Ok(())
}

/// `describe [<path|action>]`: the device entry, or one `Setting` or `Action` of it. It may
/// address an unsupported device (its entry has empty settings and actions).
pub fn describe(ctx: &mut Ctx, target: Option<&str>) -> Result<(), CliError> {
    let client = session::connect(ctx.socket.as_deref())?;
    let device = select(client.devices(), ctx.device.as_deref())?.clone();
    let desc = &device.description;
    let out = match target {
        None if ctx.json => format!("{}\n", to_json(&device)),
        None => text::device_text(&device),
        Some(name) => match (desc.settings.get(name), desc.actions.get(name)) {
            (Some(s), _) if ctx.json => format!("{}\n", to_json(s)),
            (Some(s), _) => format!("{name}: {}\n", text::setting_text(s)),
            (None, Some(a)) if ctx.json => format!("{}\n", to_json(a)),
            (None, Some(a)) => format!("{name}: {}\n", text::action_text(a)),
            (None, None) => {
                return Err(CliError::new(
                    CliCode::Proto(ErrorCode::Unsupported),
                    format!("{name} is not a setting or an action of {}", device.name),
                ));
            }
        },
    };
    print(ctx, &out);
    Ok(())
}

/// One device of `devices --report`: what an issue report needs, without id and serial.
#[derive(Serialize)]
struct ReportEntry<'a> {
    model: &'a str,
    support: &'a Support,
    firmware: &'a Option<String>,
    video_node: &'a Option<String>,
    usb: &'a Usb,
}

#[derive(Serialize)]
struct Report<'a> {
    devices: Vec<ReportEntry<'a>>,
}

/// `devices [--report]`: every listed device, or the one `--device` matches. Exit 3
/// (`no_device`) when the list is empty (parity exception PX-5). `--report` prints JSON in
/// both modes, pretty-printed for pasting into an issue, with `serial_present` and no id or
/// serial value.
pub fn devices(ctx: &mut Ctx, report: bool) -> Result<(), CliError> {
    let client = session::connect(ctx.socket.as_deref())?;
    let list: Vec<Device> = match ctx.device.as_deref() {
        Some(wanted) => vec![select(client.devices(), Some(wanted))?.clone()],
        None => client.devices().to_vec(),
    };
    if list.is_empty() {
        return Err(
            CliError::new(CliCode::NoDevice, "no OBSBOT camera is listed").with_hint(report_hint()),
        );
    }
    let out = if report {
        let entries = list
            .iter()
            .map(|d| ReportEntry {
                model: &d.model,
                support: &d.support,
                firmware: &d.firmware,
                video_node: &d.video_node,
                usb: &d.usb,
            })
            .collect();
        let report = Report { devices: entries };
        format!(
            "{}\n",
            serde_json::to_string_pretty(&report).expect("a report serialises")
        )
    } else if ctx.json {
        format!("{{\"devices\":{}}}\n", to_json(&list))
    } else {
        render::devices_text(&list)
    };
    print(ctx, &out);
    Ok(())
}
