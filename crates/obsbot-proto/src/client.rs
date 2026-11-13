// SPDX-License-Identifier: GPL-3.0-or-later

//! The synchronous client (design spec 6.1). A [`Client`] blocks per request on its own
//! connection (`subscribe: false`). [`Client::subscribe`] returns the `hello` reply, a
//! [`Sender`] and a [`Receiver`]; a reader thread delivers replies (with their request ids)
//! and events in arrival order. State, patches and action parameters are path → JSON value;
//! there are no per-setting methods. There is no built-in retry: when the connection ends,
//! the caller decides whether to reconnect (4.1, "Reconnect").

mod blocking;
mod connection;
mod error;
#[doc(hidden)]
pub mod script;
mod subscribe;

pub use blocking::Client;
pub use connection::{Connection, HelloReply};
pub use error::ClientError;
pub use subscribe::{Incoming, Receiver, Sender};
