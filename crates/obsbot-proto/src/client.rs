// SPDX-License-Identifier: GPL-3.0-or-later

//! The synchronous client (design spec 6.1). A [`Client`] blocks per request on its own
//! connection (`subscribe: false`). State, patches and action parameters are path → JSON
//! value; there are no per-setting methods. There is no built-in retry: when the connection
//! ends, the caller decides whether to reconnect (4.1, "Reconnect").

mod blocking;
mod connection;
mod error;
#[doc(hidden)]
pub mod script;

pub use blocking::Client;
pub use connection::{Connection, HelloReply};
pub use error::ClientError;
