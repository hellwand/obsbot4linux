// SPDX-License-Identifier: GPL-3.0-or-later

//! One connection to the daemon: JSON lines over the Unix socket (spec 4.1). Lines from the
//! daemon have no length limit and are not capped here. Every wait for a reply is bounded by
//! the reply timeout (`CLIENT_REPLY_TIMEOUT`, 30 s, spec 5.8).

use crate::client::ClientError;
use crate::ids::RequestId;
use crate::message::{ErrorCode, Hello, HelloOk, OkBody, Reply, ReplyBody, Request, Verb};
use std::io::{BufRead, BufReader, ErrorKind, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::time::{Duration, Instant};

/// The reply to `hello`: parsed, and the line exactly as the daemon sent it (`watch` prints
/// it unchanged, spec 6.2).
#[derive(Debug, Clone)]
pub struct HelloReply {
    pub ok: HelloOk,
    pub line: String,
}

/// The sending side: request ids count up from 1 per connection.
pub(crate) struct WriteHalf {
    stream: UnixStream,
    next_id: i64,
}

impl WriteHalf {
    /// The id the next request gets.
    pub(crate) fn next_id(&self) -> RequestId {
        RequestId::Int(self.next_id)
    }

    /// Sends one request with the next id and returns that id.
    pub(crate) fn send(&mut self, verb: Verb) -> Result<RequestId, ClientError> {
        let id = self.next_id();
        self.next_id += 1;
        let request = Request {
            id: id.clone(),
            verb,
        };
        let mut line = serde_json::to_string(&request).expect("a request always serialises");
        line.push('\n');
        self.stream
            .write_all(line.as_bytes())
            .map_err(|e| ClientError::Disconnected(Some(e)))?;
        Ok(id)
    }
}

/// The receiving side. Keeps a partial line across timeouts.
pub(crate) struct ReadHalf {
    reader: BufReader<UnixStream>,
    buf: Vec<u8>,
}

impl ReadHalf {
    /// The next line without its newline. `deadline: None` waits as long as it takes.
    pub(crate) fn line(&mut self, deadline: Option<Instant>) -> Result<String, ClientError> {
        loop {
            let timeout = match deadline {
                None => None,
                Some(at) => {
                    let left = at.saturating_duration_since(Instant::now());
                    if left.is_zero() {
                        return Err(ClientError::Timeout);
                    }
                    Some(left)
                }
            };
            self.reader
                .get_ref()
                .set_read_timeout(timeout)
                .map_err(|e| ClientError::Disconnected(Some(e)))?;
            match self.reader.read_until(b'\n', &mut self.buf) {
                Ok(_) if self.buf.last() == Some(&b'\n') => {
                    let mut bytes = std::mem::take(&mut self.buf);
                    bytes.pop();
                    return String::from_utf8(bytes)
                        .map_err(|_| ClientError::Malformed("a line is not UTF-8".into()));
                }
                // End of stream, also inside a line.
                Ok(_) => return Err(ClientError::Disconnected(None)),
                Err(e)
                    if matches!(
                        e.kind(),
                        ErrorKind::WouldBlock | ErrorKind::TimedOut | ErrorKind::Interrupted
                    ) => {}
                Err(e) => return Err(ClientError::Disconnected(Some(e))),
            }
        }
    }

    /// A second handle on the socket, to shut it down from another thread.
    pub(crate) fn stream(&self) -> Result<UnixStream, ClientError> {
        self.reader
            .get_ref()
            .try_clone()
            .map_err(|e| ClientError::Disconnected(Some(e)))
    }
}

/// One connection to the daemon. [`Client`](crate::client::Client) builds on it; `watch`
/// uses it directly to print lines as received.
pub struct Connection {
    w: WriteHalf,
    r: ReadHalf,
    reply_timeout: Duration,
}

impl Connection {
    /// Connects to the socket at `path`.
    pub fn open(path: &Path) -> Result<Connection, ClientError> {
        let stream = UnixStream::connect(path).map_err(|source| ClientError::Connect {
            path: path.to_path_buf(),
            source,
        })?;
        let read = stream
            .try_clone()
            .map_err(|e| ClientError::Disconnected(Some(e)))?;
        Ok(Connection {
            w: WriteHalf { stream, next_id: 1 },
            r: ReadHalf {
                reader: BufReader::new(read),
                buf: Vec::new(),
            },
            reply_timeout: crate::CLIENT_REPLY_TIMEOUT,
        })
    }

    /// Connects to [`socket_path`](crate::socket_path).
    pub fn open_default() -> Result<Connection, ClientError> {
        let path = crate::socket_path().ok_or(ClientError::NoSocketPath)?;
        Connection::open(&path)
    }

    /// How long a reply may take; `CLIENT_REPLY_TIMEOUT` unless set (tests shorten it).
    pub fn set_reply_timeout(&mut self, timeout: Duration) {
        self.reply_timeout = timeout;
    }

    pub fn reply_timeout(&self) -> Duration {
        self.reply_timeout
    }

    /// Sends one request with the next id (1 for the first request of a connection) and
    /// returns that id.
    pub fn send(&mut self, verb: Verb) -> Result<RequestId, ClientError> {
        self.w.send(verb)
    }

    /// The next line from the daemon, without its newline. `wait: None` waits as long as it
    /// takes; otherwise [`ClientError::Timeout`] once `wait` has passed. The end of the
    /// connection is [`ClientError::Disconnected`].
    pub fn read_line(&mut self, wait: Option<Duration>) -> Result<String, ClientError> {
        self.r.line(wait.map(|w| Instant::now() + w))
    }

    /// Sends `hello` and reads its reply within the reply timeout. A reply that is not a
    /// protocol-2 `hello` reply is [`ClientError::OldDaemon`] (spec 5.10); the error
    /// `protocol`, or a `hello` reply of another protocol, is [`ClientError::Protocol`].
    pub fn hello(&mut self, hello: Hello) -> Result<HelloReply, ClientError> {
        let id = self.send(Verb::Hello(hello))?;
        let line = self.read_line(Some(self.reply_timeout))?;
        let ok = hello_reply(&line, &id)?;
        Ok(HelloReply { ok, line })
    }

    pub(crate) fn into_halves(self) -> (WriteHalf, ReadHalf) {
        (self.w, self.r)
    }
}

/// Classifies the first line a daemon sent in answer to `hello` with the id `id`.
pub(crate) fn hello_reply(line: &str, id: &RequestId) -> Result<HelloOk, ClientError> {
    let Ok(reply) = serde_json::from_str::<Reply>(line) else {
        return Err(ClientError::OldDaemon);
    };
    match reply.body {
        ReplyBody::Ok(OkBody::Hello(h)) if h.protocol != crate::PROTOCOL => {
            Err(ClientError::Protocol {
                supported: vec![h.protocol],
                message: format!("the daemon answered hello with protocol {}", h.protocol),
            })
        }
        ReplyBody::Ok(OkBody::Hello(h)) if reply.id.as_ref() == Some(id) => Ok(h),
        ReplyBody::Ok(OkBody::Hello(_)) => Err(ClientError::Malformed(format!(
            "the reply to hello carries another id: {:?}",
            reply.id
        ))),
        ReplyBody::Error(e) if e.code == ErrorCode::Protocol => Err(ClientError::Protocol {
            supported: e.supported.clone().unwrap_or_default(),
            message: e.message,
        }),
        ReplyBody::Error(e) => Err(ClientError::Refused(Box::new(e))),
        ReplyBody::Ok(_) => Err(ClientError::OldDaemon),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn one() -> RequestId {
        RequestId::Int(1)
    }

    #[test]
    fn a_protocol_2_hello_reply_is_accepted() {
        let line = r#"{"id":1,"ok":{"protocol":2,"daemon":"1.0.0","devices":[],"states":{}}}"#;
        let ok = hello_reply(line, &one()).unwrap();
        assert_eq!(ok.daemon, "1.0.0");
        assert!(ok.devices.is_empty());
    }

    /// Spec 5.10: 0.13 answers a hello line with `ok:false` and a string `error`.
    #[test]
    fn the_reply_of_a_0_13_daemon_is_an_old_daemon() {
        for line in [
            r#"{"id":null,"ok":false,"error":"bad request: missing field `args` at line 1 column 83"}"#,
            r#"{"id":1,"ok":true,"result":{},"text":"ok"}"#,
            r#"{"event":"state","state":{"zoom":1.0}}"#,
            "not json",
            r#"{"id":1,"ok":{"rev":3}}"#,
        ] {
            assert!(
                matches!(hello_reply(line, &one()), Err(ClientError::OldDaemon)),
                "{line}"
            );
        }
    }

    #[test]
    fn the_error_protocol_names_the_daemons_protocols() {
        let line = r#"{"id":null,"error":{"code":"protocol","message":"protocol 2 is not supported","supported":[3]}}"#;
        match hello_reply(line, &one()) {
            Err(ClientError::Protocol { supported, message }) => {
                assert_eq!(supported, vec![3]);
                assert_eq!(message, "protocol 2 is not supported");
            }
            other => panic!("{other:?}"),
        }
        let newer = r#"{"id":1,"ok":{"protocol":3,"daemon":"2.0.0","devices":[],"states":{}}}"#;
        assert!(matches!(
            hello_reply(newer, &one()),
            Err(ClientError::Protocol { supported, .. }) if supported == vec![3]
        ));
    }

    #[test]
    fn another_error_to_hello_is_a_refusal() {
        let line = r#"{"id":1,"error":{"code":"invalid","message":"a second hello"}}"#;
        assert!(matches!(
            hello_reply(line, &one()),
            Err(ClientError::Refused(e)) if e.code == ErrorCode::Invalid
        ));
    }
}
