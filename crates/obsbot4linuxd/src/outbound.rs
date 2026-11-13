// SPDX-License-Identifier: GPL-3.0-or-later

//! The outbound queue of one connection (5.1): at most `OUTBOUND_LINES` lines wait for the
//! connection's writer thread. When the queue is full, the connection is closed and one log
//! line names it by its connection number and the `client` of its `hello`.

use obsbot_proto::{ErrorBody, ErrorCode, OkBody, Reply, ReplyBody, RequestId};
use std::os::unix::net::UnixStream;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, SyncSender, TrySendError, sync_channel};
use std::sync::{Arc, Mutex, PoisonError};

/// The number of one connection, counted from 1 in accept order. 0 is never a connection: it
/// marks the daemon's own jobs (the stop of a continuing action, 5.6).
pub type ConnId = u64;

struct Shared {
    conn: ConnId,
    client: Mutex<String>,
    closed: AtomicBool,
    stream: Option<UnixStream>,
}

/// The sending end of a connection's outbound queue. Clones share the queue.
#[derive(Clone)]
pub struct Outbound {
    tx: SyncSender<String>,
    shared: Arc<Shared>,
}

impl Outbound {
    /// A queue of `capacity` lines for connection `conn`; `stream` (a clone of the
    /// connection's socket) is shut down when the queue overflows or [`Outbound::close`] runs.
    /// The receiver goes to the writer thread.
    pub fn new(
        conn: ConnId,
        stream: Option<UnixStream>,
        capacity: usize,
    ) -> (Outbound, Receiver<String>) {
        let (tx, rx) = sync_channel(capacity);
        let shared = Arc::new(Shared {
            conn,
            client: Mutex::new(String::new()),
            closed: AtomicBool::new(false),
            stream,
        });
        (Outbound { tx, shared }, rx)
    }

    pub fn conn(&self) -> ConnId {
        self.shared.conn
    }

    /// Records the `client` of the connection's `hello`, for the overflow log line.
    pub fn set_client(&self, client: &str) {
        *self
            .shared
            .client
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = client.to_string();
    }

    /// Queues one line. `false` when the connection is closed or the queue was full; a full
    /// queue closes the connection and logs it once.
    pub fn send(&self, line: String) -> bool {
        if self.is_closed() {
            return false;
        }
        match self.tx.try_send(line) {
            Ok(()) => true,
            Err(TrySendError::Full(_)) => {
                let client = self
                    .shared
                    .client
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .clone();
                if !self.shared.closed.swap(true, Ordering::SeqCst) {
                    eprintln!("{}", log_full(self.shared.conn, &client));
                    self.shutdown();
                }
                false
            }
            Err(TrySendError::Disconnected(_)) => {
                self.shared.closed.store(true, Ordering::SeqCst);
                false
            }
        }
    }

    /// Closes the connection: no further line is queued, and the socket is shut down in both
    /// directions, which ends its reader and writer threads.
    pub fn close(&self) {
        self.shared.closed.store(true, Ordering::SeqCst);
        self.shutdown();
    }

    /// Marks the connection closed without touching its socket: no further line is queued,
    /// and a worker skips every request of this connection it has not started, also one it
    /// has already taken into a batch (4.1 rule 9). The writer still sends what is queued, so
    /// a last error line reaches the client.
    pub fn mark_closed(&self) {
        self.shared.closed.store(true, Ordering::SeqCst);
    }

    pub fn is_closed(&self) -> bool {
        self.shared.closed.load(Ordering::SeqCst)
    }

    fn shutdown(&self) {
        if let Some(stream) = &self.shared.stream {
            let _ = stream.shutdown(std::net::Shutdown::Both);
        }
    }
}

/// The line of an `ok` reply.
pub fn ok_line(id: &RequestId, body: OkBody) -> String {
    let reply = Reply {
        id: Some(id.clone()),
        body: ReplyBody::Ok(body),
    };
    serde_json::to_string(&reply).expect("a reply always serialises")
}

/// The line of an `error` reply; `id: null` when the request had no usable id.
pub fn error_line(id: Option<&RequestId>, body: ErrorBody) -> String {
    let reply = Reply {
        id: id.cloned(),
        body: ReplyBody::Error(body),
    };
    serde_json::to_string(&reply).expect("a reply always serialises")
}

/// An error body with only a code and a message.
pub fn error(code: ErrorCode, message: &str) -> ErrorBody {
    ErrorBody::new(code, message)
}

/// The log line of 5.11 for a connection closed for a full outbound queue.
pub fn log_full(conn: ConnId, client: &str) -> String {
    let client = if client.is_empty() {
        "no hello"
    } else {
        client
    };
    format!("obsbot4linuxd: closed connection {conn} ({client}): its outbound queue was full")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_arrive_in_order() {
        let (out, rx) = Outbound::new(7, None, 4);
        assert!(out.send("a".into()));
        assert!(out.send("b".into()));
        assert_eq!(rx.try_recv().unwrap(), "a");
        assert_eq!(rx.try_recv().unwrap(), "b");
        assert_eq!(out.conn(), 7);
    }

    /// 5.1: a subscriber that does not read is disconnected when its queue is full; it never
    /// blocks the sender (a worker or the device table).
    #[test]
    fn a_full_queue_closes_the_connection_instead_of_blocking() {
        let (out, _rx) = Outbound::new(1, None, 2);
        out.set_client("widget 1.0");
        assert!(out.send("1".into()));
        assert!(out.send("2".into()));
        assert!(!out.send("3".into()), "full: refused at once");
        assert!(out.is_closed());
        assert!(!out.send("4".into()), "closed stays closed");
    }

    #[test]
    fn a_full_queue_shuts_the_socket_down() {
        let (ours, theirs) = UnixStream::pair().unwrap();
        let (out, _rx) = Outbound::new(1, Some(ours), 1);
        assert!(out.send("1".into()));
        assert!(!out.send("2".into()));
        let mut buf = [0u8; 1];
        use std::io::Read;
        // Without a timeout a missing shutdown would hang the test instead of failing it.
        theirs
            .set_read_timeout(Some(std::time::Duration::from_secs(5)))
            .unwrap();
        assert_eq!(
            (&theirs).read(&mut buf).unwrap(),
            0,
            "the peer sees the end"
        );
    }

    #[test]
    fn the_log_line_names_the_connection_and_its_client() {
        assert_eq!(
            log_full(3, "widget 1.0"),
            "obsbot4linuxd: closed connection 3 (widget 1.0): its outbound queue was full"
        );
        assert_eq!(
            log_full(4, ""),
            "obsbot4linuxd: closed connection 4 (no hello): its outbound queue was full"
        );
    }

    /// The golden lines of `errors.jsonl` and `session.jsonl` come out byte for byte.
    #[test]
    fn reply_lines_are_canonical() {
        use obsbot_proto::{Empty, RevOk};
        assert_eq!(
            ok_line(
                &RequestId::Int(4),
                OkBody::Rev(RevOk {
                    rev: 6,
                    result: None
                })
            ),
            r#"{"id":4,"ok":{"rev":6}}"#
        );
        assert_eq!(
            ok_line(&RequestId::Int(5), OkBody::Empty(Empty {})),
            r#"{"id":5,"ok":{}}"#
        );
        let mut protocol = error(ErrorCode::Protocol, "the first message must be hello");
        protocol.supported = Some(vec![2]);
        assert_eq!(
            error_line(None, protocol),
            r#"{"id":null,"error":{"code":"protocol","message":"the first message must be hello","supported":[2]}}"#
        );
        assert_eq!(
            error_line(
                Some(&RequestId::Int(3)),
                error(ErrorCode::UnknownDevice, "no device sn:EXAMPLE0000009")
            ),
            r#"{"id":3,"error":{"code":"unknown_device","message":"no device sn:EXAMPLE0000009"}}"#
        );
    }

    #[test]
    fn a_dropped_receiver_counts_as_closed() {
        let (out, rx) = Outbound::new(1, None, 2);
        drop(rx);
        assert!(!out.send("1".into()));
        assert!(out.is_closed());
    }

    /// 4.1 rule 9: a closed connection's clones (held by queued and batched jobs) see the
    /// close at once; the lines queued before it still reach the writer.
    #[test]
    fn mark_closed_keeps_the_queued_lines_and_refuses_new_ones() {
        let (ours, theirs) = UnixStream::pair().unwrap();
        let (out, rx) = Outbound::new(1, Some(ours), 4);
        let job = out.clone();
        assert!(out.send("last".into()));
        out.mark_closed();
        assert!(job.is_closed(), "every clone sees it");
        assert!(!job.send("late".into()));
        assert_eq!(rx.try_recv().unwrap(), "last");
        assert!(rx.try_recv().is_err());
        theirs.set_nonblocking(true).unwrap();
        let mut buf = [0u8; 1];
        use std::io::Read;
        assert!(
            (&theirs).read(&mut buf).is_err(),
            "the socket is left open for the writer"
        );
    }
}
