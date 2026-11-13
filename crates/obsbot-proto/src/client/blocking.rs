// SPDX-License-Identifier: GPL-3.0-or-later

//! The blocking client: one request at a time on a connection with `subscribe: false`.

use crate::client::{ClientError, Connection};
use crate::description::Device;
use crate::message::{Hello, HelloOk, OkBody, ReplyBody, ServerMessage, Verb};
use crate::state::{Params, Patch, State};
use serde_json::Value;
use std::path::Path;
use std::time::{Duration, Instant};

/// A connection that blocks per request (spec 6.1).
pub struct Client {
    conn: Connection,
    hello: HelloOk,
}

/// Action parameters as an object: `null` is no parameter, anything but an object is refused.
pub(crate) fn params_of(params: Value) -> Result<Params, ClientError> {
    match params {
        Value::Object(map) => Ok(map),
        Value::Null => Ok(Params::new()),
        _ => Err(ClientError::NotAnObject),
    }
}

impl Client {
    /// Connects to [`socket_path`](crate::socket_path) and sends `hello` with
    /// `subscribe: false`, whatever `hello` says.
    pub fn connect(hello: Hello) -> Result<Client, ClientError> {
        let path = crate::socket_path().ok_or(ClientError::NoSocketPath)?;
        Client::connect_to(&path, hello)
    }

    /// As [`Client::connect`], on the socket at `path`.
    pub fn connect_to(path: &Path, hello: Hello) -> Result<Client, ClientError> {
        let mut conn = Connection::open(path)?;
        let reply = conn.hello(hello.subscribe(false))?;
        Ok(Client {
            conn,
            hello: reply.ok,
        })
    }

    /// The `hello` reply: daemon version, devices and their states at connect time.
    pub fn hello(&self) -> &HelloOk {
        &self.hello
    }

    /// The device list of the `hello` reply.
    pub fn devices(&self) -> &[Device] {
        &self.hello.devices
    }

    /// How long a reply may take; `CLIENT_REPLY_TIMEOUT` unless set.
    pub fn set_reply_timeout(&mut self, timeout: Duration) {
        self.conn.set_reply_timeout(timeout);
    }

    /// `get`: the full state (`paths: None`) or the paths asked for.
    pub fn get(
        &mut self,
        device: &str,
        paths: Option<&[&str]>,
    ) -> Result<(u64, State), ClientError> {
        let verb = Verb::Get {
            device: device.to_string(),
            paths: paths.map(|p| p.iter().map(|s| s.to_string()).collect()),
        };
        match self.call(verb)? {
            OkBody::State(rs) => Ok((rs.rev, rs.state)),
            _ => Err(ClientError::Malformed(
                "the reply to get holds no state".into(),
            )),
        }
    }

    /// `set`: one patch; returns the reply's rev.
    pub fn set(&mut self, device: &str, patch: Patch) -> Result<u64, ClientError> {
        let verb = Verb::Set {
            device: device.to_string(),
            patch,
        };
        match self.call(verb)? {
            OkBody::Rev(r) => Ok(r.rev),
            _ => Err(ClientError::Malformed(
                "the reply to set holds no rev".into(),
            )),
        }
    }

    /// `do`: one action with its parameters (a JSON object, or `null` for none); returns
    /// the reply's rev and the action's result, if it has one.
    pub fn act(
        &mut self,
        device: &str,
        action: &str,
        params: Value,
    ) -> Result<(u64, Option<Params>), ClientError> {
        let verb = Verb::Do {
            device: device.to_string(),
            action: action.to_string(),
            params: params_of(params)?,
        };
        match self.call(verb)? {
            OkBody::Rev(r) => Ok((r.rev, r.result)),
            _ => Err(ClientError::Malformed(
                "the reply to do holds no rev".into(),
            )),
        }
    }

    /// Sends one request and waits for its reply. Events are skipped; a reply without an id
    /// (an error the daemon could not tie to a request) is taken as this request's.
    fn call(&mut self, verb: Verb) -> Result<OkBody, ClientError> {
        let id = self.conn.send(verb)?;
        let deadline = Instant::now() + self.conn.reply_timeout();
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            let line = self.conn.read_line(Some(left))?;
            let message: ServerMessage = serde_json::from_str(&line)
                .map_err(|e| ClientError::Malformed(format!("{e}: {line}")))?;
            let ServerMessage::Reply(reply) = message else {
                continue;
            };
            if reply.id.is_some() && reply.id.as_ref() != Some(&id) {
                return Err(ClientError::Malformed(format!(
                    "a reply to another request: {line}"
                )));
            }
            return match reply.body {
                ReplyBody::Ok(body) => Ok(body),
                ReplyBody::Error(e) => Err(ClientError::Refused(Box::new(e))),
            };
        }
    }
}
