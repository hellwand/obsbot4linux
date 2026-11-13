// SPDX-License-Identifier: GPL-3.0-or-later

//! The envelopes of protocol 2 (4.1): requests, replies, events and errors. One JSON object
//! per line; [`Request::from_line`] is the daemon's parser, `serde_json::to_string` gives the
//! canonical line of every type.

use crate::description::Device;
use crate::ids::RequestId;
use crate::state::{Params, Patch, State};
use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fmt;

fn yes() -> bool {
    true
}

/// The `hello` verb.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Hello {
    pub protocol: u32,
    /// `"<name> <version>"` of the client.
    pub client: String,
    /// Events for this connection; `true` when left out on the wire.
    #[serde(default = "yes")]
    pub subscribe: bool,
    /// Live topics (4.1 rule 8); empty when left out.
    #[serde(default)]
    pub live: Vec<String>,
}

impl Hello {
    /// A `hello` for protocol 2 with `subscribe: false` and no live topics.
    pub fn new(client: &str) -> Hello {
        Hello {
            protocol: crate::PROTOCOL,
            client: client.to_string(),
            subscribe: false,
            live: Vec::new(),
        }
    }

    pub fn subscribe(mut self, on: bool) -> Hello {
        self.subscribe = on;
        self
    }

    pub fn live(mut self, topics: &[&str]) -> Hello {
        self.live = topics.iter().map(|t| t.to_string()).collect();
        self
    }
}

/// One request line. Exactly one verb.
#[derive(Debug, Clone, PartialEq)]
pub struct Request {
    pub id: RequestId,
    pub verb: Verb,
}

#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Verb {
    Hello(Hello),
    /// `paths: None` asks for the full state (`"get": null`).
    Get {
        device: String,
        paths: Option<Vec<String>>,
    },
    Set {
        device: String,
        patch: Patch,
    },
    /// `"do"` with its parameters (`"with"`, `{}` when left out).
    Do {
        device: String,
        action: String,
        params: Params,
    },
    Live {
        topics: Vec<String>,
    },
}

impl Verb {
    /// The device id a device request names; `None` for `hello` and `live`.
    pub fn device(&self) -> Option<&str> {
        match self {
            Verb::Get { device, .. } | Verb::Set { device, .. } | Verb::Do { device, .. } => {
                Some(device)
            }
            Verb::Hello(_) | Verb::Live { .. } => None,
        }
    }
}

/// Why a request line was refused before it reached a device: always error code `invalid`
/// (the daemon answers `protocol` instead when this is the first line of a connection).
#[derive(Debug, Clone, PartialEq)]
pub struct RequestError {
    /// The id, when the line had a usable one; the reply carries `id: null` otherwise.
    pub id: Option<RequestId>,
    pub message: String,
}

impl fmt::Display for RequestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for RequestError {}

/// The wire form of a request: every verb optional, so that "no verb" and "two verbs" can be
/// told apart from malformed JSON.
#[derive(Serialize, Deserialize, JsonSchema)]
struct WireRequest {
    #[serde(default)]
    id: Option<RequestId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    device: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    hello: Option<Hello>,
    #[serde(default, skip_serializing_if = "Option::is_none", with = "nullable")]
    #[schemars(with = "Option<Vec<String>>")]
    get: Option<Option<Vec<String>>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    set: Option<Patch>,
    #[serde(default, rename = "do", skip_serializing_if = "Option::is_none")]
    act: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    with: Option<Params>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    live: Option<Vec<String>>,
}

/// `Some(None)` for an explicit `null`, `None` for a missing field.
mod nullable {
    use super::*;

    pub fn serialize<S: Serializer>(
        v: &Option<Option<Vec<String>>>,
        s: S,
    ) -> Result<S::Ok, S::Error> {
        match v {
            Some(inner) => inner.serialize(s),
            None => s.serialize_none(),
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        d: D,
    ) -> Result<Option<Option<Vec<String>>>, D::Error> {
        Option::<Vec<String>>::deserialize(d).map(Some)
    }
}

impl Request {
    /// Parses one request line. Errors carry the id when the line had a usable one.
    pub fn from_line(line: &str) -> Result<Request, RequestError> {
        let value: Value = serde_json::from_str(line).map_err(|e| RequestError {
            id: None,
            message: format!("not JSON: {e}"),
        })?;
        let id = value
            .get("id")
            .and_then(|v| serde_json::from_value::<RequestId>(v.clone()).ok());
        let wire: WireRequest = serde_json::from_value(value).map_err(|e| RequestError {
            id: id.clone(),
            message: format!("malformed request: {e}"),
        })?;
        Request::try_from(wire)
    }
}

impl TryFrom<WireRequest> for Request {
    type Error = RequestError;

    fn try_from(w: WireRequest) -> Result<Request, RequestError> {
        let fail = |id: Option<RequestId>, message: &str| RequestError {
            id,
            message: message.to_string(),
        };
        let verbs = [
            w.hello.is_some(),
            w.get.is_some(),
            w.set.is_some(),
            w.act.is_some(),
            w.live.is_some(),
        ];
        let id = w.id.clone();
        match verbs.iter().filter(|v| **v).count() {
            0 => return Err(fail(id, "the request has no verb")),
            1 => {}
            _ => return Err(fail(id, "the request has more than one verb")),
        }
        let Some(id) = w.id else {
            return Err(fail(None, "the request has no id"));
        };
        let device = |d: Option<String>| {
            d.ok_or_else(|| fail(Some(id.clone()), "the request names no device"))
        };
        let verb = if let Some(hello) = w.hello {
            Verb::Hello(hello)
        } else if let Some(paths) = w.get {
            Verb::Get {
                device: device(w.device)?,
                paths,
            }
        } else if let Some(patch) = w.set {
            Verb::Set {
                device: device(w.device)?,
                patch,
            }
        } else if let Some(action) = w.act {
            Verb::Do {
                device: device(w.device)?,
                action,
                params: w.with.unwrap_or_default(),
            }
        } else {
            Verb::Live {
                topics: w.live.unwrap_or_default(),
            }
        };
        Ok(Request { id, verb })
    }
}

impl From<&Request> for WireRequest {
    fn from(r: &Request) -> WireRequest {
        let mut w = WireRequest {
            id: Some(r.id.clone()),
            device: r.verb.device().map(str::to_string),
            hello: None,
            get: None,
            set: None,
            act: None,
            with: None,
            live: None,
        };
        match &r.verb {
            Verb::Hello(h) => w.hello = Some(h.clone()),
            Verb::Get { paths, .. } => w.get = Some(paths.clone()),
            Verb::Set { patch, .. } => w.set = Some(patch.clone()),
            Verb::Do { action, params, .. } => {
                w.act = Some(action.clone());
                w.with = (!params.is_empty()).then(|| params.clone());
            }
            Verb::Live { topics } => w.live = Some(topics.clone()),
        }
        w
    }
}

impl Serialize for Request {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        WireRequest::from(self).serialize(s)
    }
}

impl<'de> Deserialize<'de> for Request {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Request, D::Error> {
        let wire = WireRequest::deserialize(d)?;
        Request::try_from(wire).map_err(serde::de::Error::custom)
    }
}

impl JsonSchema for Request {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "Request".into()
    }

    fn json_schema(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
        let mut schema = WireRequest::json_schema(generator);
        schema.insert(
            "description".into(),
            "One request line: `id` and exactly one verb (`hello`, `get`, `set`, `do`, `live`). \
             `get`, `set` and `do` name a `device`; `with` holds the parameters of `do`."
                .into(),
        );
        schema
    }
}

/// A reply: `{"id":…,"ok":{…}}` or `{"id":…,"error":{…}}`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Reply {
    /// The request's id; `null` only for a `protocol` or `invalid` error without a usable id.
    pub id: Option<RequestId>,
    #[serde(flatten)]
    pub body: ReplyBody,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ReplyBody {
    Ok(OkBody),
    Error(ErrorBody),
}

/// The `ok` payload. Its shape follows the verb it answers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
#[non_exhaustive]
pub enum OkBody {
    /// The reply to `hello`.
    Hello(HelloOk),
    /// The reply to `get`.
    State(RevState),
    /// The reply to `set` and `do`.
    Rev(RevOk),
    /// The reply to `live`.
    Empty(Empty),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct HelloOk {
    pub protocol: u32,
    /// The daemon's version.
    pub daemon: String,
    pub devices: Vec<Device>,
    /// Device id → its rev and state.
    pub states: BTreeMap<String, RevState>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct RevState {
    pub rev: u64,
    pub state: State,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct RevOk {
    pub rev: u64,
    /// An action's result, when its description declares one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<Params>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Empty {}

/// The `error` payload (4.1, Errors).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ErrorBody {
    pub code: ErrorCode,
    /// English, for logs.
    pub message: String,
    /// Present whenever the daemon re-read the device before answering.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rev: Option<u64>,
    /// `camera`: the leaf paths written before the failure.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub applied: Option<Vec<String>>,
    /// `camera`: the leaf path or action that failed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failed: Option<String>,
    /// `protocol`: the protocol numbers the daemon speaks.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub supported: Option<Vec<u32>>,
}

impl ErrorBody {
    pub fn new(code: ErrorCode, message: &str) -> ErrorBody {
        ErrorBody {
            code,
            message: message.to_string(),
            rev: None,
            applied: None,
            failed: None,
            supported: None,
        }
    }
}

/// The error codes the daemon sends (4.1). A code this crate does not know parses as
/// `Other`; clients treat it as `internal` (4.1 rule 7).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ErrorCode {
    Protocol,
    Invalid,
    UnknownDevice,
    Unsupported,
    UnsupportedDevice,
    OutOfRange,
    Conflict,
    Unavailable,
    Asleep,
    Busy,
    Camera,
    Internal,
    #[serde(untagged)]
    Other(String),
}

impl ErrorCode {
    /// Every code the daemon may send, in the order of the table in 4.1.
    pub const ALL: [ErrorCode; 12] = [
        ErrorCode::Protocol,
        ErrorCode::Invalid,
        ErrorCode::UnknownDevice,
        ErrorCode::Unsupported,
        ErrorCode::UnsupportedDevice,
        ErrorCode::OutOfRange,
        ErrorCode::Conflict,
        ErrorCode::Unavailable,
        ErrorCode::Asleep,
        ErrorCode::Busy,
        ErrorCode::Camera,
        ErrorCode::Internal,
    ];

    /// The wire name (`"out_of_range"`).
    pub fn as_str(&self) -> &str {
        match self {
            ErrorCode::Protocol => "protocol",
            ErrorCode::Invalid => "invalid",
            ErrorCode::UnknownDevice => "unknown_device",
            ErrorCode::Unsupported => "unsupported",
            ErrorCode::UnsupportedDevice => "unsupported_device",
            ErrorCode::OutOfRange => "out_of_range",
            ErrorCode::Conflict => "conflict",
            ErrorCode::Unavailable => "unavailable",
            ErrorCode::Asleep => "asleep",
            ErrorCode::Busy => "busy",
            ErrorCode::Camera => "camera",
            ErrorCode::Internal => "internal",
            ErrorCode::Other(code) => code,
        }
    }
}

/// An event (only on connections with `subscribe: true`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "event", rename_all = "snake_case")]
#[non_exhaustive]
pub enum Event {
    /// The full state of one device.
    State {
        device: String,
        rev: u64,
        state: State,
    },
    /// The full device list.
    Devices { devices: Vec<Device> },
    /// An event type this crate does not know; clients ignore it (4.1 rule 7).
    #[serde(other, skip_serializing)]
    #[schemars(skip)]
    Unknown,
}

/// One line from the daemon: an event (it has `event`) or a reply.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum ServerMessage {
    Event(Event),
    Reply(Reply),
}
