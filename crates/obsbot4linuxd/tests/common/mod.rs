// SPDX-License-Identifier: GPL-3.0-or-later

//! The harness of the daemon's integration tests (design spec 11): an in-process daemon on a
//! temporary socket with the fake bus `fake-a,fake-b,unknown`, a line client that speaks
//! protocol 2 by hand (the client library is track C's), and the test-control socket.
//! Each test file uses the parts it needs.

#![allow(dead_code)]

use obsbot_device::models::fake::{FakeBus, KEYS, Op};
use obsbot4linuxd::{Config, Timing, run_with};
use serde_json::{Value, json};
use std::io::{BufRead, BufReader, ErrorKind, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

/// Short tier periods and scans, so that the 5 s and 60 s tiers and hotplug happen within a
/// test; the request deadline stays the spec's 15 s.
pub fn fast() -> Timing {
    Timing {
        scan: Duration::from_millis(100),
        request_deadline: obsbot_proto::REQUEST_DEADLINE,
        second: Duration::from_millis(100),
        five_seconds: Duration::from_millis(500),
        minute: Duration::from_secs(3),
    }
}

/// The ids the fake bus gives the three devices of `KEYS`.
pub const A: &str = "sn:FAKEA0000001";
pub const B: &str = "sn:FAKEB0000001";
pub const UNKNOWN: &str = "port:9-3";

static NEXT: AtomicU32 = AtomicU32::new(0);

/// A fresh, short directory for one daemon's sockets (a Unix socket path has at most 107
/// bytes, and the daemon adds `.lock` and `.test`): `$TMPDIR/o4ld-<pid>-<n>`. Asserts that
/// `s.sock` in it is at most 100 bytes, so that a deep `TMPDIR` fails loudly.
pub fn temp_dir() -> PathBuf {
    let n = NEXT.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!("o4ld-{}-{n}", std::process::id()));
    let socket = dir.join("s.sock");
    assert!(
        socket.as_os_str().len() <= 100,
        "socket path too long for a Unix socket: {}",
        socket.display()
    );
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// One in-process daemon in fake mode.
pub struct Daemon {
    pub socket: PathBuf,
    pub bus: Arc<FakeBus>,
    pub stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<std::io::Result<()>>>,
    dir: PathBuf,
}

impl Daemon {
    /// `fake-a,fake-b,unknown` with [`fast`] timing.
    pub fn start() -> Daemon {
        Daemon::start_with(&KEYS, fast(), |_| {})
    }

    /// `models` with `timing`; `setup` runs on the fake bus before the daemon starts (a delay,
    /// a `fail_next`).
    pub fn start_with(models: &[&str], timing: Timing, setup: impl FnOnce(&FakeBus)) -> Daemon {
        let dir = temp_dir();
        let socket = dir.join("s.sock");
        let bus = Arc::new(FakeBus::new(models).expect("fake models"));
        setup(&bus);
        let stop = Arc::new(AtomicBool::new(false));
        let config = Config {
            socket: socket.clone(),
            fake: Some(bus.clone()),
            stop: stop.clone(),
        };
        let handle = std::thread::spawn(move || run_with(config, timing));
        let until = Instant::now() + Duration::from_secs(5);
        while UnixStream::connect(&socket).is_err() {
            assert!(
                Instant::now() < until,
                "the daemon did not bind {}",
                socket.display()
            );
            assert!(!handle.is_finished(), "the daemon ended at start-up");
            std::thread::sleep(Duration::from_millis(10));
        }
        Daemon {
            socket,
            bus,
            stop,
            handle: Some(handle),
            dir,
        }
    }

    /// A raw connection.
    pub fn connect(&self) -> Client {
        Client::connect(&self.socket)
    }

    /// A connection after `hello`; returns it and the `ok` payload of the reply.
    pub fn hello(&self, subscribe: bool, live: &[&str]) -> (Client, Value) {
        let mut c = self.connect();
        let ok = c.hello(subscribe, live);
        (c, ok)
    }

    /// One line on the test-control socket (5.12); returns the reply.
    pub fn control(&self, line: &str) -> Value {
        let mut path = self.socket.as_os_str().to_owned();
        path.push(".test");
        let mut s = UnixStream::connect(PathBuf::from(path)).expect("the test-control socket");
        s.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
        writeln!(s, "{line}").unwrap();
        let mut reply = String::new();
        BufReader::new(s).read_line(&mut reply).unwrap();
        serde_json::from_str(&reply).expect("a JSON reply")
    }

    /// The operations the fake device `id` received, oldest first.
    pub fn ops(&self, id: &str) -> Vec<Op> {
        self.bus.ops(id)
    }

    /// Stops the daemon as SIGTERM would and returns what `run` returned.
    pub fn stop(mut self) -> std::io::Result<()> {
        self.finish().expect("the daemon was running")
    }

    /// Whether `run` has returned (a fatal error ends it on its own).
    pub fn ended(&self) -> bool {
        self.handle.as_ref().is_none_or(|h| h.is_finished())
    }

    fn finish(&mut self) -> Option<std::io::Result<()>> {
        let handle = self.handle.take()?;
        self.stop.store(true, Ordering::SeqCst);
        let until = Instant::now() + Duration::from_secs(20);
        while !handle.is_finished() {
            assert!(
                Instant::now() < until,
                "run did not return within 20 s of stop"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
        Some(handle.join().expect("run panicked"))
    }
}

impl Drop for Daemon {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            let _ = self.finish();
        } else {
            self.stop.store(true, Ordering::SeqCst);
        }
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// A protocol-2 connection that keeps the events it reads while it waits for a reply.
pub struct Client {
    stream: UnixStream,
    reader: BufReader<UnixStream>,
    /// Events read and not yet taken, oldest first.
    pub events: Vec<Value>,
    next: i64,
}

impl Client {
    pub fn connect(path: &std::path::Path) -> Client {
        let stream = UnixStream::connect(path).expect("connect");
        let reader = BufReader::new(stream.try_clone().unwrap());
        Client {
            stream,
            reader,
            events: Vec::new(),
            next: 100,
        }
    }

    /// A fresh request id.
    pub fn id(&mut self) -> i64 {
        self.next += 1;
        self.next
    }

    pub fn send(&mut self, v: &Value) {
        self.send_raw(format!("{v}\n").as_bytes());
    }

    pub fn send_raw(&mut self, bytes: &[u8]) {
        self.stream.write_all(bytes).expect("write");
    }

    /// The next line from the daemon; `None` at the end of the stream or after `timeout`.
    pub fn line(&mut self, timeout: Duration) -> Option<Value> {
        self.stream.set_read_timeout(Some(timeout)).unwrap();
        let mut text = String::new();
        match self.reader.read_line(&mut text) {
            Ok(0) => None,
            Ok(_) => Some(serde_json::from_str(&text).expect("the daemon sends JSON lines")),
            Err(e) if matches!(e.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => None,
            Err(e) if e.kind() == ErrorKind::ConnectionReset => None,
            Err(e) => panic!("read: {e}"),
        }
    }

    /// Reads until the reply with `id`; events on the way go to `events`. Panics after 20 s.
    pub fn reply(&mut self, id: i64) -> Value {
        let until = Instant::now() + Duration::from_secs(20);
        loop {
            let left = until.saturating_duration_since(Instant::now());
            let line = self
                .line(left.max(Duration::from_millis(1)))
                .unwrap_or_else(|| panic!("no reply to {id}"));
            if line.get("event").is_some() {
                self.events.push(line);
            } else if line["id"] == json!(id) {
                return line;
            } else {
                panic!("a reply to another id: {line}");
            }
        }
    }

    /// Sends `hello` and returns the `ok` payload.
    pub fn hello(&mut self, subscribe: bool, live: &[&str]) -> Value {
        let id = self.id();
        self.send(
            &json!({"id": id, "hello": {"protocol": 2, "client": "test 1.0",
            "subscribe": subscribe, "live": live}}),
        );
        let r = self.reply(id);
        r.get("ok")
            .cloned()
            .unwrap_or_else(|| panic!("hello refused: {r}"))
    }

    pub fn get(&mut self, device: &str, paths: Option<&[&str]>) -> Value {
        let id = self.id();
        self.send(&json!({"id": id, "device": device, "get": paths}));
        self.reply(id)
    }

    pub fn set(&mut self, device: &str, patch: Value) -> Value {
        let id = self.id();
        self.send(&json!({"id": id, "device": device, "set": patch}));
        self.reply(id)
    }

    pub fn act(&mut self, device: &str, action: &str, with: Value) -> Value {
        let id = self.id();
        self.send(&json!({"id": id, "device": device, "do": action, "with": with}));
        self.reply(id)
    }

    /// The first event (already read or read now) for which `pred` holds; the events before it
    /// are dropped. `None` after `timeout`.
    pub fn event(&mut self, timeout: Duration, pred: impl Fn(&Value) -> bool) -> Option<Value> {
        if let Some(i) = self.events.iter().position(&pred) {
            let ev = self.events.remove(i);
            self.events.drain(..i);
            return Some(ev);
        }
        self.events.clear();
        let until = Instant::now() + timeout;
        loop {
            let left = until.checked_duration_since(Instant::now())?;
            let line = self.line(left)?;
            if line.get("event").is_some() && pred(&line) {
                return Some(line);
            }
        }
    }

    /// Whether the daemon closes the connection within `timeout` (lines before the end are
    /// read and dropped).
    pub fn closed_within(&mut self, timeout: Duration) -> bool {
        let until = Instant::now() + timeout;
        self.stream
            .set_read_timeout(Some(Duration::from_millis(50)))
            .unwrap();
        let mut buf = Vec::new();
        loop {
            if Instant::now() >= until {
                return false;
            }
            buf.clear();
            match self.reader.read_until(b'\n', &mut buf) {
                Ok(0) => return true,
                Ok(_) => continue,
                Err(e) if matches!(e.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {}
                Err(_) => return true,
            }
        }
    }

    /// Closes this end.
    pub fn close(self) {
        let _ = self.stream.shutdown(std::net::Shutdown::Both);
    }
}

/// A state event of `device`.
pub fn state_of(device: &'static str) -> impl Fn(&Value) -> bool {
    move |e: &Value| e["event"] == "state" && e["device"] == device
}

/// A devices event.
pub fn devices(e: &Value) -> bool {
    e["event"] == "devices"
}

/// The ids of a devices list, with `!` for an unavailable entry.
pub fn ids(devices: &Value) -> Vec<String> {
    devices
        .as_array()
        .expect("a device list")
        .iter()
        .map(|d| {
            let mut id = d["id"].as_str().unwrap().to_string();
            if d["available"] == json!(false) {
                id.push('!');
            }
            id
        })
        .collect()
}

/// The value at a dotted path of a state object.
pub fn at<'a>(state: &'a Value, path: &str) -> &'a Value {
    path.split('.').fold(state, |v, k| &v[k])
}

/// The ops of `id` after the first `skip`, without reads.
pub fn writes_after(daemon: &Daemon, id: &str, skip: usize) -> Vec<Op> {
    daemon.ops(id)[skip..]
        .iter()
        .filter(|op| !matches!(op, Op::Read(_)))
        .cloned()
        .collect()
}

/// Polls `cond` every 20 ms until it holds or `timeout` passes.
pub fn eventually(timeout: Duration, cond: impl Fn() -> bool) -> bool {
    let until = Instant::now() + timeout;
    while Instant::now() < until {
        if cond() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    cond()
}
