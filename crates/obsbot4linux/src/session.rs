// SPDX-License-Identifier: GPL-3.0-or-later

//! Connecting to the daemon, and what the command line says when that fails (design spec
//! 5.10 and 6.2). The command line never starts, stops or restarts a daemon.

use crate::VERSION;
use crate::errors::{CliCode, CliError};
use crate::select::{refuse_unsupported, select};
use obsbot_proto::client::{Client, ClientError};
use obsbot_proto::{Device, ErrorCode, Hello};
use std::path::Path;

/// The line a user runs to start the daemon (spec 5.10).
pub const ENABLE: &str = "systemctl --user enable --now obsbot4linux.socket";

/// Where issue reports go (spec 6.2, 8.4).
pub const ISSUES: &str = "https://github.com/hellwand/obsbot4linux/issues";

/// The `client` of this command line's `hello`.
pub fn client_name() -> String {
    format!("obsbot4linux {VERSION}")
}

/// Connects and sends `hello` with `subscribe: false` (spec 6.2, "One-shot commands").
pub fn connect(socket: Option<&Path>) -> Result<Client, CliError> {
    let path = socket.ok_or_else(|| client_error(ClientError::NoSocketPath))?;
    Client::connect_to(path, Hello::new(&client_name())).map_err(client_error)
}

/// Connects, selects the device (`--device` or the only usable one, spec 6.2) and refuses an
/// unsupported one before anything is sent after `hello`. For `status`, `get`, `set`, `do`.
pub fn connect_device(
    socket: Option<&Path>,
    wanted: Option<&str>,
) -> Result<(Client, Device), CliError> {
    let client = connect(socket)?;
    let device = select(client.devices(), wanted)?.clone();
    refuse_unsupported(&device)?;
    Ok((client, device))
}

/// What the command line reports for a client error (spec 6.2 table, 5.10).
pub fn client_error(e: ClientError) -> CliError {
    match e {
        ClientError::NoSocketPath => CliError::new(
            CliCode::Daemon,
            format!(
                "no daemon socket: neither OBSBOT4LINUX_SOCKET nor XDG_RUNTIME_DIR is set; \
                 start the daemon with: {ENABLE}"
            ),
        ),
        ClientError::Connect { path, source } => CliError::new(
            CliCode::Daemon,
            format!(
                "cannot connect to the obsbot4linux daemon at {} ({source}); \
                 start it with: {ENABLE}",
                path.display()
            ),
        ),
        ClientError::Disconnected(_) => CliError::new(
            CliCode::Daemon,
            "the obsbot4linux daemon closed the connection",
        ),
        ClientError::Timeout => CliError::new(
            CliCode::Daemon,
            "the obsbot4linux daemon did not reply within 30 s",
        ),
        ClientError::OldDaemon => CliError::new(
            CliCode::Proto(ErrorCode::Protocol),
            "an obsbot4linux daemon older than 1.0 is running: restart it or log out",
        ),
        ClientError::Protocol { supported, .. } => {
            let numbers: Vec<String> = supported.iter().map(u32::to_string).collect();
            CliError::new(
                CliCode::Proto(ErrorCode::Protocol),
                format!(
                    "the obsbot4linux daemon speaks protocol {}, this client speaks 2: \
                     install the same version of both and restart the daemon",
                    numbers.join(", ")
                ),
            )
        }
        ClientError::Refused(body) => CliError::refused(*body),
        ClientError::NotAnObject => CliError::usage("action parameters must be an object"),
        other => CliError::new(
            CliCode::Proto(ErrorCode::Internal),
            format!("the obsbot4linux daemon sent something this client cannot read: {other}"),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use obsbot_proto::ErrorBody;
    use std::io;
    use std::path::PathBuf;

    #[test]
    fn an_unreachable_daemon_is_exit_4_with_the_enable_line() {
        let e = client_error(ClientError::Connect {
            path: PathBuf::from("/run/user/1000/obsbot4linux.sock"),
            source: io::Error::from(io::ErrorKind::NotFound),
        });
        assert_eq!(e.code, CliCode::Daemon);
        assert_eq!(e.code.exit(), 4);
        assert!(e.message.contains("/run/user/1000/obsbot4linux.sock"));
        assert!(e.message.ends_with(ENABLE), "{}", e.message);
        let e = client_error(ClientError::NoSocketPath);
        assert_eq!(e.code.exit(), 4);
        assert!(e.message.ends_with(ENABLE));
        assert_eq!(client_error(ClientError::Timeout).code.exit(), 4);
        assert_eq!(client_error(ClientError::Disconnected(None)).code.exit(), 4);
    }

    #[test]
    fn an_older_daemon_is_exit_4_with_the_restart_message() {
        let e = client_error(ClientError::OldDaemon);
        assert_eq!(e.code.as_str(), "protocol");
        assert_eq!(e.code.exit(), 4);
        assert_eq!(
            e.message,
            "an obsbot4linux daemon older than 1.0 is running: restart it or log out"
        );
        let e = client_error(ClientError::Protocol {
            supported: vec![3],
            message: "protocol 2 is not supported".into(),
        });
        assert_eq!(
            e.message,
            "the obsbot4linux daemon speaks protocol 3, this client speaks 2: \
             install the same version of both and restart the daemon"
        );
        assert_eq!(e.code.exit(), 4);
    }

    #[test]
    fn a_refusal_keeps_the_daemons_code() {
        let e = client_error(ClientError::Refused(Box::new(ErrorBody::new(
            ErrorCode::OutOfRange,
            "led.brightness: 4 is above the maximum 3",
        ))));
        assert_eq!(e.code, CliCode::Proto(ErrorCode::OutOfRange));
        assert_eq!(e.code.exit(), 2);
        assert_eq!(e.message, "led.brightness: 4 is above the maximum 3");
        assert!(e.body.is_some());
    }

    #[test]
    fn an_unreadable_reply_is_internal() {
        let e = client_error(ClientError::Malformed("not JSON".into()));
        assert_eq!(e.code.exit(), 1);
    }
}
