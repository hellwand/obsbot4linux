// SPDX-License-Identifier: GPL-3.0-or-later

//! Why a client call failed (spec 6.1: errors are typed; the daemon's code is an
//! [`ErrorCode`](crate::ErrorCode), which has a variant for codes this crate does not know).

use crate::message::ErrorBody;
use std::fmt;
use std::io;
use std::path::PathBuf;

/// Why a client call failed.
#[derive(Debug)]
#[non_exhaustive]
pub enum ClientError {
    /// Neither `OBSBOT4LINUX_SOCKET` nor `XDG_RUNTIME_DIR` names a socket.
    NoSocketPath,
    /// The socket could not be connected: no socket file, connection refused, no permission.
    Connect { path: PathBuf, source: io::Error },
    /// The daemon closed the connection, or reading or writing it failed.
    Disconnected(Option<io::Error>),
    /// No reply within the reply timeout (`CLIENT_REPLY_TIMEOUT`, spec 5.8).
    Timeout,
    /// The reply to `hello` is not a protocol-2 object: a daemon older than 1.0 (spec 5.10).
    OldDaemon,
    /// The daemon speaks other protocol numbers: it answered `hello` with the error `protocol`,
    /// or with a `hello` reply of another protocol.
    Protocol {
        supported: Vec<u32>,
        message: String,
    },
    /// The daemon refused the request; the body as the daemon sent it (boxed: it is large).
    Refused(Box<ErrorBody>),
    /// A line from the daemon is not a protocol-2 message, or a reply does not fit its request.
    Malformed(String),
    /// Action parameters that are not a JSON object (nothing was sent).
    NotAnObject,
}

impl fmt::Display for ClientError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ClientError::NoSocketPath => {
                f.write_str("neither OBSBOT4LINUX_SOCKET nor XDG_RUNTIME_DIR is set")
            }
            ClientError::Connect { path, source } => {
                write!(f, "cannot connect to {}: {source}", path.display())
            }
            ClientError::Disconnected(None) => f.write_str("the daemon closed the connection"),
            ClientError::Disconnected(Some(e)) => write!(f, "the connection failed: {e}"),
            ClientError::Timeout => f.write_str("no reply from the daemon in time"),
            ClientError::OldDaemon => f.write_str("the daemon does not speak protocol 2"),
            ClientError::Protocol { supported, .. } => {
                write!(f, "the daemon speaks protocol {supported:?}, not 2")
            }
            ClientError::Refused(body) => write!(f, "{}: {}", body.code.as_str(), body.message),
            ClientError::Malformed(m) => write!(f, "unreadable message from the daemon: {m}"),
            ClientError::NotAnObject => f.write_str("action parameters must be a JSON object"),
        }
    }
}

impl std::error::Error for ClientError {}
