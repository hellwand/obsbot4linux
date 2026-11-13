// SPDX-License-Identifier: GPL-3.0-or-later

//! The subscribed client: a [`Sender`] for requests and a [`Receiver`] whose reader thread
//! delivers replies and events in arrival order (spec 6.1). A request without a reply after
//! the reply timeout ends the receiver with [`ClientError::Timeout`] (spec 5.8).

use crate::client::blocking::params_of;
use crate::client::connection::{ReadHalf, WriteHalf};
use crate::client::{Client, ClientError, Connection};
use crate::ids::RequestId;
use crate::message::{Event, Hello, HelloOk, ReplyBody, ServerMessage, Verb};
use crate::state::Patch;
use serde_json::Value;
use std::collections::HashMap;
use std::net::Shutdown;
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread;
use std::time::{Duration, Instant};

/// One message for a subscribed client.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Incoming {
    /// A reply. `id` is `None` only for an error the daemon could not tie to a request.
    Reply {
        id: Option<RequestId>,
        body: ReplyBody,
    },
    /// An event. Event types this crate does not know are skipped (spec 4.1 rule 7).
    Event(Event),
}

/// Requests sent and not answered yet: id → the time its reply is due.
type Pending = Arc<Mutex<HashMap<RequestId, Instant>>>;

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Sends requests on a subscribed connection; each call returns the request id its reply
/// will carry.
pub struct Sender {
    w: Arc<Mutex<WriteHalf>>,
    pending: Pending,
    timeout: Duration,
}

impl Sender {
    pub fn get(&self, device: &str, paths: Option<&[&str]>) -> Result<RequestId, ClientError> {
        self.send(Verb::Get {
            device: device.to_string(),
            paths: paths.map(|p| p.iter().map(|s| s.to_string()).collect()),
        })
    }

    pub fn set(&self, device: &str, patch: Patch) -> Result<RequestId, ClientError> {
        self.send(Verb::Set {
            device: device.to_string(),
            patch,
        })
    }

    pub fn act(&self, device: &str, action: &str, params: Value) -> Result<RequestId, ClientError> {
        self.send(Verb::Do {
            device: device.to_string(),
            action: action.to_string(),
            params: params_of(params)?,
        })
    }

    /// Replaces the connection's live topics (spec 4.1 rule 8).
    pub fn live(&self, topics: &[&str]) -> Result<RequestId, ClientError> {
        self.send(Verb::Live {
            topics: topics.iter().map(|t| t.to_string()).collect(),
        })
    }

    /// How long a reply may take; `CLIENT_REPLY_TIMEOUT` unless set.
    pub fn set_reply_timeout(&mut self, timeout: Duration) {
        self.timeout = timeout;
    }

    fn send(&self, verb: Verb) -> Result<RequestId, ClientError> {
        // The pending entry exists before the line leaves, so a fast reply always finds it.
        // `pending` is not held across the write: the reader thread takes it for every reply
        // and must not wait behind a slow write.
        let mut w = lock(&self.w);
        let id = w.next_id();
        lock(&self.pending).insert(id.clone(), Instant::now() + self.timeout);
        w.send(verb).inspect_err(|_| {
            lock(&self.pending).remove(&id);
        })
    }
}

/// Replies and events in arrival order. It ends after an error: the connection ended
/// ([`ClientError::Disconnected`]), a line was unreadable, or a reply was overdue.
pub struct Receiver {
    rx: mpsc::Receiver<Result<Incoming, ClientError>>,
    pending: Pending,
    stream: UnixStream,
    done: bool,
}

impl Receiver {
    fn overdue(&self) -> bool {
        let now = Instant::now();
        lock(&self.pending).values().any(|due| *due <= now)
    }
}

impl Iterator for Receiver {
    type Item = Result<Incoming, ClientError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }
        loop {
            let due = lock(&self.pending).values().min().copied();
            let got = match due {
                None => self.rx.recv().map_err(|_| RecvTimeoutError::Disconnected),
                Some(at) => self
                    .rx
                    .recv_timeout(at.saturating_duration_since(Instant::now())),
            };
            match got {
                Ok(Ok(item)) => return Some(Ok(item)),
                Ok(Err(e)) => {
                    self.done = true;
                    return Some(Err(e));
                }
                Err(RecvTimeoutError::Timeout) if self.overdue() => {
                    self.done = true;
                    let _ = self.stream.shutdown(Shutdown::Both);
                    return Some(Err(ClientError::Timeout));
                }
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => {
                    self.done = true;
                    return None;
                }
            }
        }
    }
}

impl Drop for Receiver {
    /// Closes the connection, which ends the reader thread.
    fn drop(&mut self) {
        let _ = self.stream.shutdown(Shutdown::Both);
    }
}

fn read_loop(mut r: ReadHalf, tx: mpsc::Sender<Result<Incoming, ClientError>>, pending: Pending) {
    loop {
        let item = match r.line(None) {
            Err(e) => {
                let _ = tx.send(Err(e));
                return;
            }
            Ok(line) => match serde_json::from_str::<ServerMessage>(&line) {
                Err(e) => {
                    let _ = tx.send(Err(ClientError::Malformed(format!("{e}: {line}"))));
                    return;
                }
                Ok(ServerMessage::Event(Event::Unknown)) => continue,
                Ok(ServerMessage::Event(event)) => Incoming::Event(event),
                Ok(ServerMessage::Reply(reply)) => {
                    if let Some(id) = &reply.id {
                        lock(&pending).remove(id);
                    }
                    Incoming::Reply {
                        id: reply.id,
                        body: reply.body,
                    }
                }
            },
        };
        if tx.send(Ok(item)).is_err() {
            return;
        }
    }
}

impl Client {
    /// Connects to [`socket_path`](crate::socket_path) and sends `hello` with
    /// `subscribe: true`, whatever `hello` says. Returns the `hello` reply (the caller keeps
    /// its own device list from it and from the `devices` events), the sender and the
    /// receiver.
    pub fn subscribe(hello: Hello) -> Result<(HelloOk, Sender, Receiver), ClientError> {
        let path = crate::socket_path().ok_or(ClientError::NoSocketPath)?;
        Client::subscribe_to(&path, hello)
    }

    /// As [`Client::subscribe`], on the socket at `path`.
    pub fn subscribe_to(
        path: &Path,
        hello: Hello,
    ) -> Result<(HelloOk, Sender, Receiver), ClientError> {
        let mut conn = Connection::open(path)?;
        let reply = conn.hello(hello.subscribe(true))?;
        let timeout = conn.reply_timeout();
        let (w, r) = conn.into_halves();
        let stream = r.stream()?;
        let pending: Pending = Arc::new(Mutex::new(HashMap::new()));
        let (tx, rx) = mpsc::channel();
        let reader_pending = Arc::clone(&pending);
        thread::Builder::new()
            .name("obsbot-client-reader".into())
            .spawn(move || read_loop(r, tx, reader_pending))
            .map_err(|e| ClientError::Disconnected(Some(e)))?;
        let sender = Sender {
            w: Arc::new(Mutex::new(w)),
            pending: Arc::clone(&pending),
            timeout,
        };
        let receiver = Receiver {
            rx,
            pending,
            stream,
            done: false,
        };
        Ok((reply.ok, sender, receiver))
    }
}
