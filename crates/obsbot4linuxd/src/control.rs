// SPDX-License-Identifier: GPL-3.0-or-later

//! The test-control socket of fake mode (5.12): `<socket path>.test`, not part of protocol 2.
//! One JSON line in, one JSON line out; the lines are the fake bus's (`FakeBus::handle_line`):
//! plug, unplug, sleep, wake and fail_next.

use crate::lines::{Line, read_line};
use crate::socket;
use obsbot_device::models::fake::FakeBus;
use obsbot_proto::MAX_REQUEST_LINE;
use std::io::{self, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;

/// `<socket path>.test`.
pub fn control_path(socket: &Path) -> PathBuf {
    let mut p = socket.as_os_str().to_owned();
    p.push(".test");
    PathBuf::from(p)
}

/// Binds the test-control socket (mode 0600, a stale file replaced) and serves it on a thread
/// until `stop` is set; the daemon wakes the accept by connecting once. Each connection has a
/// thread of its own that answers every line until the end of the stream.
pub fn serve(
    socket: &Path,
    bus: Arc<FakeBus>,
    stop: Arc<AtomicBool>,
) -> io::Result<JoinHandle<()>> {
    let path = control_path(socket);
    let listener = socket::bind_at(&path)?;
    std::thread::Builder::new()
        .name("test-control".into())
        .spawn(move || {
            for stream in listener.incoming() {
                if stop.load(Ordering::SeqCst) {
                    return;
                }
                let Ok(stream) = stream else { continue };
                let bus = bus.clone();
                let _ = std::thread::Builder::new()
                    .name("test-control conn".into())
                    .spawn(move || answer(stream, &bus));
            }
        })
}

/// Answers the lines of one control connection with the fake bus's replies.
fn answer(stream: UnixStream, bus: &FakeBus) {
    let Ok(mut write_half) = stream.try_clone() else {
        return;
    };
    let mut reader = BufReader::new(stream);
    while let Ok(Line::Text(line)) = read_line(&mut reader, MAX_REQUEST_LINE) {
        let reply = bus.handle_line(&line);
        if writeln!(write_half, "{reply}").is_err() {
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use obsbot_device::models::fake::KEYS;
    use std::io::{BufRead, BufReader, Write};
    use std::os::unix::net::UnixStream;
    use std::sync::atomic::Ordering;

    #[test]
    fn control_lines_reach_the_fake_bus() {
        let dir = std::env::temp_dir().join(format!("o4ld-ctl-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let socket = dir.join("s.sock");
        // The daemon adds `.lock` and `.test`; a Unix socket path holds at most 107 bytes.
        assert!(
            socket.as_os_str().len() <= 100,
            "socket path too long for a Unix socket: {}",
            socket.display()
        );
        let bus = Arc::new(FakeBus::new(&KEYS).unwrap());
        let stop = Arc::new(AtomicBool::new(false));
        let h = serve(&socket, bus.clone(), stop.clone()).unwrap();
        let path = control_path(&socket);
        assert!(path.ends_with("s.sock.test"));
        let mut s = UnixStream::connect(&path).unwrap();
        writeln!(s, "{{\"plug\":\"fake-b\"}}").unwrap();
        writeln!(s, "nonsense").unwrap();
        let mut r = BufReader::new(s.try_clone().unwrap());
        let mut a = String::new();
        r.read_line(&mut a).unwrap();
        assert_eq!(a.trim_end(), r#"{"ok":"sn:FAKEB0000002"}"#);
        let mut b = String::new();
        r.read_line(&mut b).unwrap();
        assert!(b.starts_with(r#"{"error":"not JSON"#));
        stop.store(true, Ordering::SeqCst);
        let _ = UnixStream::connect(&path);
        h.join().unwrap();
        let _ = std::fs::remove_dir_all(dir);
    }
}
