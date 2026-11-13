// SPDX-License-Identifier: GPL-3.0-or-later

//! Errors and exit codes of the command line (design spec 6.2, "Errors" and "Exit codes").
//! An error the daemon sent keeps its code; `usage`, `ambiguous_device`, `no_device` and
//! `daemon` exist only here and are never sent by the daemon.

use obsbot_proto::{Device, ErrorBody, ErrorCode, Support, short_id};
use serde_json::{Value, json};
use std::io::Write;

/// The code of an error the command line reports.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CliCode {
    /// A protocol error code: sent by the daemon, or found by the command line itself
    /// (`unknown_device`, `unsupported_device`, `unsupported`, `protocol`).
    Proto(ErrorCode),
    /// Bad arguments, a value that does not parse.
    Usage,
    /// `--device` matches several devices; several usable devices without it.
    AmbiguousDevice,
    /// No usable device.
    NoDevice,
    /// No socket, connection refused, no reply within `CLIENT_REPLY_TIMEOUT`.
    Daemon,
}

impl CliCode {
    /// The code as `--json` prints it.
    pub fn as_str(&self) -> &str {
        match self {
            CliCode::Proto(code) => code.as_str(),
            CliCode::Usage => "usage",
            CliCode::AmbiguousDevice => "ambiguous_device",
            CliCode::NoDevice => "no_device",
            CliCode::Daemon => "daemon",
        }
    }

    /// The exit code: 0 ok; 1 `internal`; 2 invalid input; 3 device not usable; 4 daemon
    /// unreachable. A code this client does not know counts as `internal` (spec 4.1 rule 7).
    pub fn exit(&self) -> u8 {
        match self {
            CliCode::Usage | CliCode::AmbiguousDevice => 2,
            CliCode::NoDevice => 3,
            CliCode::Daemon => 4,
            CliCode::Proto(code) => match code {
                ErrorCode::Invalid
                | ErrorCode::OutOfRange
                | ErrorCode::Unsupported
                | ErrorCode::UnknownDevice => 2,
                ErrorCode::Conflict
                | ErrorCode::Unavailable
                | ErrorCode::Asleep
                | ErrorCode::Busy
                | ErrorCode::Camera
                | ErrorCode::UnsupportedDevice => 3,
                ErrorCode::Protocol => 4,
                _ => 1,
            },
        }
    }
}

/// An error the command line reports and exits with.
#[derive(Debug, Clone, PartialEq)]
pub struct CliError {
    pub code: CliCode,
    /// English, one line; `--json` prints it as `message`.
    pub message: String,
    /// The devices the error is about (`ambiguous_device`, `no_device`): human output lists
    /// them with shortened ids, `--json` lists their full ids under `devices`.
    pub devices: Vec<Device>,
    /// A line human output prints last.
    pub hint: Option<String>,
    /// The daemon's error as it sent it; `--json` prints it whole.
    pub body: Option<Box<ErrorBody>>,
}

impl CliError {
    pub fn new(code: CliCode, message: impl Into<String>) -> CliError {
        CliError {
            code,
            message: message.into(),
            devices: Vec::new(),
            hint: None,
            body: None,
        }
    }

    pub fn usage(message: impl Into<String>) -> CliError {
        CliError::new(CliCode::Usage, message)
    }

    /// The error the daemon sent, with its code and every field.
    pub fn refused(body: ErrorBody) -> CliError {
        CliError {
            code: CliCode::Proto(body.code.clone()),
            message: body.message.clone(),
            devices: Vec::new(),
            hint: None,
            body: Some(Box::new(body)),
        }
    }

    pub fn with_devices(mut self, devices: Vec<Device>) -> CliError {
        self.devices = devices;
        self
    }

    pub fn with_hint(mut self, hint: impl Into<String>) -> CliError {
        self.hint = Some(hint.into());
        self
    }

    /// The line `--json` prints: `{"error":{"code","message"}}`; the daemon's error body as
    /// it sent it (every field, in protocol order) when there is one; `devices` (full ids)
    /// when the error names devices.
    pub fn json_line(&self) -> String {
        let inner = match &self.body {
            Some(body) => serde_json::to_string(body).expect("an error body serialises"),
            None => {
                let mut error = json!({"code": self.code.as_str(), "message": self.message});
                if !self.devices.is_empty() {
                    let ids = self.devices.iter().map(|d| json!(d.id)).collect();
                    error["devices"] = Value::Array(ids);
                }
                error.to_string()
            }
        };
        format!("{{\"error\":{inner}}}")
    }

    /// The lines human output prints on stderr.
    pub fn human(&self) -> String {
        let mut out = format!("obsbot4linux: {}\n", self.message);
        for d in &self.devices {
            out.push_str(&format!("  {}\n", device_line(d)));
        }
        if let Some(body) = &self.body {
            if let Some(applied) = &body.applied {
                out.push_str(&format!("  applied: {}\n", applied.join(", ")));
            }
            if let Some(failed) = &body.failed {
                out.push_str(&format!("  failed: {failed}\n"));
            }
        }
        if let Some(hint) = &self.hint {
            out.push_str(&format!("{hint}\n"));
        }
        out
    }
}

/// One device as human output lists it: shortened id, name, model, and the support level
/// when it is not `tested` (spec 8.6), and `unavailable` while it is unplugged.
pub fn device_line(d: &Device) -> String {
    let mut line = format!("{}  {}  {}", short_id(&d.id), d.name, d.model);
    match &d.support {
        Support::Tested => {}
        Support::Untested => line.push_str("  untested"),
        Support::Unsupported => line.push_str("  unsupported"),
        other => line.push_str(&format!("  {}", support_name(other))),
    }
    if !d.available {
        line.push_str("  unavailable");
    }
    line
}

/// The wire name of a support level, a level this client does not know included.
pub fn support_name(s: &Support) -> String {
    match serde_json::to_value(s) {
        Ok(Value::String(name)) => name,
        _ => "unknown".into(),
    }
}

/// Prints an error: `--json` on stdout, human output on stderr. Returns the exit code.
pub fn report(e: &CliError, json: bool, out: &mut dyn Write, err: &mut dyn Write) -> u8 {
    if json {
        let _ = writeln!(out, "{}", e.json_line());
    } else {
        let _ = write!(err, "{}", e.human());
    }
    e.code.exit()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Spec 6.2, "Exit codes": one row per code, the CLI-only codes included.
    #[test]
    fn every_code_has_its_exit_code() {
        let table: &[(CliCode, &str, u8)] = &[
            (CliCode::Proto(ErrorCode::Internal), "internal", 1),
            (CliCode::Proto(ErrorCode::Invalid), "invalid", 2),
            (CliCode::Proto(ErrorCode::OutOfRange), "out_of_range", 2),
            (CliCode::Proto(ErrorCode::Unsupported), "unsupported", 2),
            (
                CliCode::Proto(ErrorCode::UnknownDevice),
                "unknown_device",
                2,
            ),
            (CliCode::Usage, "usage", 2),
            (CliCode::AmbiguousDevice, "ambiguous_device", 2),
            (CliCode::Proto(ErrorCode::Conflict), "conflict", 3),
            (CliCode::Proto(ErrorCode::Unavailable), "unavailable", 3),
            (CliCode::Proto(ErrorCode::Asleep), "asleep", 3),
            (CliCode::Proto(ErrorCode::Busy), "busy", 3),
            (CliCode::Proto(ErrorCode::Camera), "camera", 3),
            (CliCode::NoDevice, "no_device", 3),
            (
                CliCode::Proto(ErrorCode::UnsupportedDevice),
                "unsupported_device",
                3,
            ),
            (CliCode::Daemon, "daemon", 4),
            (CliCode::Proto(ErrorCode::Protocol), "protocol", 4),
            // 4.1 rule 7: an unknown code is treated as internal.
            (
                CliCode::Proto(ErrorCode::Other("teapot".into())),
                "teapot",
                1,
            ),
        ];
        for (code, name, exit) in table {
            assert_eq!(code.as_str(), *name);
            assert_eq!(code.exit(), *exit, "{name}");
        }
        // Every code the daemon sends is in the table.
        for code in ErrorCode::ALL {
            assert!(
                table
                    .iter()
                    .any(|(c, ..)| *c == CliCode::Proto(code.clone())),
                "{}",
                code.as_str()
            );
        }
    }

    #[test]
    fn the_json_error_has_code_and_message() {
        let e = CliError::usage("zoom.factor: 'x' is not a number");
        assert_eq!(
            e.json_line(),
            r#"{"error":{"code":"usage","message":"zoom.factor: 'x' is not a number"}}"#
        );
    }

    #[test]
    fn a_daemon_error_keeps_every_field_it_sent() {
        let mut body = ErrorBody::new(ErrorCode::Camera, "the camera did not answer");
        body.rev = Some(9);
        body.applied = Some(vec!["exposure.mode".into()]);
        body.failed = Some("exposure.time".into());
        let e = CliError::refused(body);
        assert_eq!(e.code.exit(), 3);
        assert_eq!(
            e.json_line(),
            r#"{"error":{"code":"camera","message":"the camera did not answer","rev":9,"applied":["exposure.mode"],"failed":"exposure.time"}}"#
        );
        assert_eq!(
            e.human(),
            "obsbot4linux: the camera did not answer\n  applied: exposure.mode\n  failed: exposure.time\n"
        );
    }
}
