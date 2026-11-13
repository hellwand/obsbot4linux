// SPDX-License-Identifier: GPL-3.0-or-later

//! Protocol 2 of obsbot4linux (design spec, section 4): the message envelopes, the error
//! codes, the device descriptions, the validator, path helpers and the JSON schema. The
//! synchronous client is the module `client`.
//!
//! State, patches and action parameters are generic: leaf path → JSON value, checked at run
//! time against the device description. There are no per-setting types or methods.

pub mod client;
pub mod description;
pub mod ids;
pub mod message;
pub mod schema;
pub mod state;
pub mod validate;
pub mod values;

pub use description::{
    Action, Asleep, Bound, Description, Device, ExtensionUnit, Requirement, Setting, SettingType,
    Support, Unit, UnitSpec, Usb, Warning,
};
pub use ids::{RequestId, is_valid_serial, port_id, short_id, sn_id};
pub use message::{
    Empty, ErrorBody, ErrorCode, Event, Hello, HelloOk, OkBody, Reply, ReplyBody, Request,
    RequestError, RevOk, RevState, ServerMessage, Verb,
};
pub use state::{Params, Patch, State, Values};

use std::path::PathBuf;
use std::time::Duration;

/// The protocol number this crate speaks (`hello.protocol`).
pub const PROTOCOL: u32 = 2;
/// Environment variable that overrides the socket path.
pub const SOCKET_ENV: &str = "OBSBOT4LINUX_SOCKET";
/// File name of the socket in `$XDG_RUNTIME_DIR`.
pub const SOCKET_NAME: &str = "obsbot4linux.sock";
/// A request line longer than this closes the connection (4.1).
pub const MAX_REQUEST_LINE: usize = 64 * 1024;
/// Every device request is answered within this time of its arrival (4.1 rule 9, 5.8).
pub const REQUEST_DEADLINE: Duration = Duration::from_secs(15);
/// The client library reports a lost daemon after this time without a reply (5.8).
pub const CLIENT_REPLY_TIMEOUT: Duration = Duration::from_secs(30);

/// The socket path: `$OBSBOT4LINUX_SOCKET` if set and not empty, else
/// `$XDG_RUNTIME_DIR/obsbot4linux.sock`; `None` when neither variable is set.
pub fn socket_path() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os(SOCKET_ENV).filter(|p| !p.is_empty()) {
        return Some(PathBuf::from(path));
    }
    std::env::var_os("XDG_RUNTIME_DIR")
        .filter(|d| !d.is_empty())
        .map(|dir| PathBuf::from(dir).join(SOCKET_NAME))
}
