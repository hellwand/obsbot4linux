// SPDX-License-Identifier: GPL-3.0-or-later

//! The acceptor and the connections (4.1, 5.1). Written in task b-08-conn.

use crate::lines::{Line, read_line};
use crate::outbound::{ConnId, Outbound, error, error_line, ok_line};
use crate::queue::{Job, Refused, Work};
use crate::table::{Routed, Table};
use crate::{ACCEPT_BACKOFF, MAX_CONNECTIONS, OUTBOUND_LINES, Timing};
use obsbot_proto::{
    Empty, ErrorCode, MAX_REQUEST_LINE, OkBody, PROTOCOL, Request, RequestError, Verb,
};
use std::collections::BTreeMap;
use std::io::{BufReader, Write};
use std::net::Shutdown;
use std::os::unix::net::{UnixListener, UnixStream};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Instant;

/// Every accepted connection, for the limit of 64 and for shutdown.
pub struct Conns {
    streams: Mutex<BTreeMap<ConnId, UnixStream>>,
    next: AtomicU64,
}

impl Conns {
    pub fn new() -> Conns {
        Conns {
            streams: Mutex::new(BTreeMap::new()),
            next: AtomicU64::new(1),
        }
    }

    fn lock(&self) -> MutexGuard<'_, BTreeMap<ConnId, UnixStream>> {
        self.streams.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Registers an accepted stream; `None` (and the stream is closed) when `MAX_CONNECTIONS`
    /// are open already.
    pub fn admit(&self, stream: &UnixStream) -> Option<ConnId> {
        let mut streams = self.lock();
        if streams.len() >= MAX_CONNECTIONS {
            let _ = stream.shutdown(Shutdown::Both);
            return None;
        }
        let kept = stream.try_clone().ok()?;
        let id = self.next.fetch_add(1, Ordering::SeqCst);
        streams.insert(id, kept);
        Some(id)
    }

    pub fn remove(&self, conn: ConnId) {
        self.lock().remove(&conn);
    }

    pub fn len(&self) -> usize {
        self.lock().len()
    }

    /// Shuts every connection down in both directions (5.9 shutdown).
    pub fn close_all(&self) {
        for stream in self.lock().values() {
            let _ = stream.shutdown(Shutdown::Both);
        }
    }
}

/// What every connection thread shares.
pub struct Shared {
    pub table: Arc<Table>,
    pub timing: Timing,
    pub conns: Conns,
}

/// The acceptor (5.1): accepts until `stop` is set; a failed accept waits `ACCEPT_BACKOFF`.
pub fn accept_loop(listener: UnixListener, shared: Arc<Shared>, stop: Arc<AtomicBool>) {
    for stream in listener.incoming() {
        if stop.load(Ordering::SeqCst) {
            return;
        }
        match stream {
            Ok(s) => {
                let Some(id) = shared.conns.admit(&s) else {
                    continue;
                };
                let for_conn = shared.clone();
                let spawned = std::thread::Builder::new()
                    .name(format!("conn {id}"))
                    .spawn(move || serve(s, id, for_conn));
                if spawned.is_err() {
                    shared.conns.remove(id);
                }
            }
            Err(e) => {
                eprintln!("obsbot4linuxd: accept: {e}");
                std::thread::sleep(ACCEPT_BACKOFF);
            }
        }
    }
}

/// Closes one connection when its reader ends, also by a panic (5.9: a panic in a connection
/// thread closes only that connection).
struct CloseGuard {
    conn: ConnId,
    shared: Arc<Shared>,
    out: Outbound,
}

impl Drop for CloseGuard {
    fn drop(&mut self) {
        // First: a worker skips every request of this connection it has not started, also one
        // it has already taken into a batch (4.1 rule 9). The socket is left to the writer, so
        // a queued error line still reaches the client.
        self.out.mark_closed();
        self.shared.table.remove_conn(self.conn);
        let now = Instant::now();
        for q in self.shared.table.queues() {
            q.close_conn(self.conn, now);
        }
        self.shared.conns.remove(self.conn);
    }
}

/// One connection: its reader runs on the calling thread, its writer on a thread of its own.
pub fn serve(stream: UnixStream, conn: ConnId, shared: Arc<Shared>) {
    let (out, rx) = Outbound::new(conn, stream.try_clone().ok(), OUTBOUND_LINES);
    let _guard = CloseGuard {
        conn,
        shared: shared.clone(),
        out: out.clone(),
    };
    if let Ok(mut write_half) = stream.try_clone() {
        let spawned = std::thread::Builder::new()
            .name(format!("conn {conn} writer"))
            .spawn(move || {
                for line in rx {
                    let written = write_half
                        .write_all(line.as_bytes())
                        .and_then(|()| write_half.write_all(b"\n"));
                    if written.is_err() {
                        let _ = write_half.shutdown(Shutdown::Both);
                        return;
                    }
                }
            });
        if spawned.is_err() {
            return;
        }
    } else {
        return;
    }
    let mut reader = BufReader::new(stream);
    let mut said_hello = false;
    loop {
        let parsed = match read_line(&mut reader, MAX_REQUEST_LINE) {
            Ok(Line::Text(text)) => {
                if text.trim().is_empty() {
                    continue;
                }
                Request::from_line(&text)
            }
            Ok(Line::NotUtf8) => Err(RequestError {
                id: None,
                message: "the line is not UTF-8".to_string(),
            }),
            Ok(Line::Eof | Line::TooLong) | Err(_) => return,
        };
        let arrived = Instant::now();
        if !said_hello {
            if !first_line(&shared, &out, parsed, arrived) {
                return;
            }
            said_hello = true;
        } else {
            later_line(&shared, &out, conn, parsed, arrived);
        }
    }
}

/// A `protocol` error that ends the connection once it is written.
fn refuse_protocol(out: &Outbound, id: Option<&obsbot_proto::RequestId>, message: &str) {
    let mut body = error(ErrorCode::Protocol, message);
    body.supported = Some(vec![PROTOCOL]);
    out.send(error_line(id, body));
}

/// The first line of a connection must be a `hello` of protocol 2. `false`: the connection
/// ends.
fn first_line(
    shared: &Shared,
    out: &Outbound,
    parsed: Result<Request, RequestError>,
    arrived: Instant,
) -> bool {
    match parsed {
        Ok(Request {
            id,
            verb: Verb::Hello(hello),
        }) => {
            if hello.protocol != PROTOCOL {
                refuse_protocol(
                    out,
                    Some(&id),
                    &format!("protocol {} is not supported", hello.protocol),
                );
                return false;
            }
            shared
                .table
                .hello(out, &id, &hello, arrived, shared.timing.request_deadline);
            true
        }
        Ok(request) => {
            refuse_protocol(out, Some(&request.id), "the first message must be hello");
            false
        }
        Err(e) => {
            refuse_protocol(out, e.id.as_ref(), "the first message must be hello");
            false
        }
    }
}

/// A line after `hello`: answered here or queued for its device.
fn later_line(
    shared: &Shared,
    out: &Outbound,
    conn: ConnId,
    parsed: Result<Request, RequestError>,
    arrived: Instant,
) {
    let request = match parsed {
        Ok(request) => request,
        Err(e) => {
            out.send(error_line(
                e.id.as_ref(),
                error(ErrorCode::Invalid, &e.message),
            ));
            return;
        }
    };
    let id = request.id;
    let (device, work) = match request.verb {
        Verb::Hello(_) => {
            out.send(error_line(
                Some(&id),
                error(ErrorCode::Invalid, "hello was already sent"),
            ));
            return;
        }
        Verb::Live { topics } => {
            shared.table.set_live(conn, &topics);
            out.send(ok_line(&id, OkBody::Empty(Empty {})));
            return;
        }
        Verb::Get { device, paths } => (device, Work::Get(paths)),
        Verb::Set { device, patch } => (device, Work::Set(patch)),
        Verb::Do {
            device,
            action,
            params,
        } => (device, Work::Do { action, params }),
        _ => {
            out.send(error_line(
                Some(&id),
                error(ErrorCode::Invalid, "unknown request"),
            ));
            return;
        }
    };
    let mut job = Job {
        id,
        out: Some(out.clone()),
        conn,
        device,
        work,
        arrived,
        stop: false,
    };
    match shared.table.route(&job.device) {
        Routed::Worker(q) => {
            if let Work::Do { action, .. } = &job.work {
                job.stop = q.is_stop(action);
            }
            match q.push(job) {
                Ok(()) => {}
                Err(Refused::Busy(j)) => j.fail(
                    ErrorCode::Busy,
                    &format!("{} requests are queued for {}", crate::MAX_QUEUED, j.device),
                ),
                Err(Refused::Closed(j)) => shared.table.route(&j.device).answer(&j),
            }
        }
        other => other.answer(&job),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::PortReason;
    use crate::names;
    use crate::queue::{DeviceQueue, Work};
    use obsbot_device::contract::{Bus, Motion};
    use obsbot_device::models::fake::{FakeBus, KEYS};
    use obsbot_proto::State;
    use serde_json::{Value, json};
    use std::io::{BufRead, BufReader, Read, Write};
    use std::time::{Duration, Instant};

    /// A table with fake-a listed (queue returned) and the unknown device.
    fn shared() -> (Arc<Shared>, Arc<DeviceQueue>) {
        let found = FakeBus::new(&KEYS).unwrap().scan();
        let table = Arc::new(Table::new("test"));
        let q = DeviceQueue::new();
        q.set_stops(vec!["gimbal.stop".into(), "zoom.stop".into()]);
        let model = found[0].model.unwrap();
        let device = names::supported_device(
            &found[0],
            model,
            obsbot_device::models::fake::description("fake-a").unwrap(),
            "port:9-1",
            None,
            None,
        );
        table.plug_hidden(1, model, device, q.clone());
        table.announce(
            1,
            Some("FAKEA0000001"),
            PortReason::NoSerial,
            None,
            State::new(true),
        );
        table.plug_unsupported(3, names::unsupported_device(&found[2]));
        let timing = crate::Timing::SPEC;
        (
            Arc::new(Shared {
                table,
                timing,
                conns: Conns::new(),
            }),
            q,
        )
    }

    fn client(shared: &Arc<Shared>) -> (UnixStream, BufReader<UnixStream>) {
        let (ours, theirs) = UnixStream::pair().unwrap();
        let id = shared.conns.admit(&theirs).unwrap();
        let s = shared.clone();
        std::thread::spawn(move || serve(theirs, id, s));
        ours.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
        let reader = BufReader::new(ours.try_clone().unwrap());
        (ours, reader)
    }

    fn send(s: &mut UnixStream, v: Value) {
        writeln!(s, "{v}").unwrap();
    }

    fn recv(r: &mut BufReader<UnixStream>) -> Value {
        let mut line = String::new();
        r.read_line(&mut line).unwrap();
        serde_json::from_str(&line).unwrap()
    }

    fn eof(r: &mut BufReader<UnixStream>) -> bool {
        let mut rest = Vec::new();
        r.read_to_end(&mut rest).is_ok()
    }

    fn hello(s: &mut UnixStream, r: &mut BufReader<UnixStream>) -> Value {
        send(
            s,
            json!({"id": 1, "hello": {"protocol": 2, "client": "t 1", "subscribe": false}}),
        );
        recv(r)
    }

    #[test]
    fn hello_then_requests_are_routed() {
        let (sh, q) = shared();
        let (mut s, mut r) = client(&sh);
        assert_eq!(
            hello(&mut s, &mut r)["ok"]["devices"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
        send(
            &mut s,
            json!({"id": 2, "device": "sn:FAKEA0000001", "set": {"led.brightness": 1}}),
        );
        send(
            &mut s,
            json!({"id": 3, "device": "port:9-3", "do": "sleep"}),
        );
        assert_eq!(
            recv(&mut r)["error"]["code"],
            "unsupported_device",
            "answered at once"
        );
        assert!(crate::schedule::PERIODIC.len() == 3);
        let jobs = q.take_all();
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].device, "sn:FAKEA0000001");
        assert!(matches!(&jobs[0].work, Work::Set(_)));
        send(
            &mut s,
            json!({"id": 4, "device": "sn:FAKEA0000001", "do": "gimbal.stop"}),
        );
        send(
            &mut s,
            json!({"id": 5, "hello": {"protocol": 2, "client": "again"}}),
        );
        assert_eq!(recv(&mut r)["error"]["code"], "invalid");
        assert!(q.take_all()[0].stop, "the queue knows the model's stops");
        send(&mut s, json!({"id": 6, "live": ["pose"]}));
        assert_eq!(recv(&mut r), json!({"id": 6, "ok": {}}));
        assert_eq!(sh.table.topics(), Default::default(), "subscribe was false");
    }

    #[test]
    fn the_first_line_must_be_hello_and_the_connection_closes() {
        let (sh, _) = shared();
        let (mut s, mut r) = client(&sh);
        send(&mut s, json!({"id": 9, "device": "port:9-3", "get": null}));
        let v = recv(&mut r);
        assert_eq!(v["id"], 9);
        assert_eq!(v["error"]["code"], "protocol");
        assert_eq!(v["error"]["supported"], json!([2]));
        assert!(eof(&mut r));
    }

    #[test]
    fn busy_is_answered_by_the_reader() {
        let (sh, _q) = shared();
        let (mut s, mut r) = client(&sh);
        hello(&mut s, &mut r);
        for i in 0..crate::MAX_QUEUED {
            send(
                &mut s,
                json!({"id": 100 + i, "device": "sn:FAKEA0000001", "get": null}),
            );
        }
        send(
            &mut s,
            json!({"id": 99, "device": "sn:FAKEA0000001", "get": null}),
        );
        let v = recv(&mut r);
        assert_eq!(
            (v["id"].clone(), v["error"]["code"].clone()),
            (json!(99), json!("busy"))
        );
    }

    /// 4.1 rule 9 and 5.6: when a connection closes, its queued requests are dropped and the
    /// stop of its continuing action is queued.
    #[test]
    fn a_closing_connection_drops_its_requests_and_stops_its_turn() {
        let (sh, q) = shared();
        let (mut s, mut r) = client(&sh);
        hello(&mut s, &mut r);
        send(
            &mut s,
            json!({"id": 2, "device": "sn:FAKEA0000001", "set": {"led.brightness": 1}}),
        );
        let until = Instant::now() + Duration::from_secs(5);
        while q.len() == 0 && Instant::now() < until {
            std::thread::sleep(Duration::from_millis(5));
        }
        q.record_act(
            "gimbal.turn",
            Motion::Continuing {
                stop: "gimbal.stop",
            },
            1,
            false,
        );
        drop(r);
        s.shutdown(std::net::Shutdown::Both).unwrap();
        let until = Instant::now() + Duration::from_secs(5);
        while sh.conns.len() > 0 && Instant::now() < until {
            std::thread::sleep(Duration::from_millis(5));
        }
        let jobs = q.take_all();
        assert_eq!(jobs.len(), 1);
        assert!(
            jobs[0].out.is_none() && jobs[0].stop,
            "only the daemon's own stop"
        );
    }

    /// 4.1 rule 9: a request the worker has already taken into a batch is no longer in the
    /// queue; the close marks its connection closed, so the batch skips it (b-07 batch step
    /// 2.1).
    #[test]
    fn a_closing_connection_marks_requests_already_taken_closed() {
        let (sh, q) = shared();
        let (mut s, mut r) = client(&sh);
        hello(&mut s, &mut r);
        send(
            &mut s,
            json!({"id": 2, "device": "sn:FAKEA0000001", "set": {"led.brightness": 1}}),
        );
        let until = Instant::now() + Duration::from_secs(5);
        while q.len() == 0 && Instant::now() < until {
            std::thread::sleep(Duration::from_millis(5));
        }
        let taken = q.take_all();
        let out = taken[0].out.clone().expect("a client's request");
        assert!(!out.is_closed());
        drop(r);
        s.shutdown(std::net::Shutdown::Both).unwrap();
        let until = Instant::now() + Duration::from_secs(5);
        while sh.conns.len() > 0 && Instant::now() < until {
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(out.is_closed(), "the batch skips it");
    }

    #[test]
    fn a_line_over_64_kib_closes_the_connection() {
        let (sh, _) = shared();
        let (mut s, mut r) = client(&sh);
        hello(&mut s, &mut r);
        s.write_all(&vec![b'x'; obsbot_proto::MAX_REQUEST_LINE + 1])
            .unwrap();
        assert!(eof(&mut r));
    }

    #[test]
    fn at_most_64_connections_are_admitted() {
        let conns = Conns::new();
        let mut keep = Vec::new();
        for _ in 0..crate::MAX_CONNECTIONS {
            let (a, b) = UnixStream::pair().unwrap();
            assert!(conns.admit(&b).is_some());
            keep.push((a, b));
        }
        let (mut a, b) = UnixStream::pair().unwrap();
        assert_eq!(conns.admit(&b), None);
        drop(b);
        assert_eq!(a.read(&mut [0u8; 1]).unwrap(), 0, "closed at once");
        conns.remove(1);
        assert_eq!(conns.len(), crate::MAX_CONNECTIONS - 1);
        let (_c, d) = UnixStream::pair().unwrap();
        assert!(conns.admit(&d).is_some());
    }
}
