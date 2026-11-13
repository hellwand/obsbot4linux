// SPDX-License-Identifier: GPL-3.0-or-later

//! Test support, not part of the client API: a scripted daemon. It serves one transcript in
//! the form of `tests/golden/*.jsonl` to the first connection on a socket, so that client and
//! CLI tests run without a daemon (design spec 10.3, track C: before the fake daemon exists).
//!
//! Transcript lines:
//!
//! - `> {…}`: the client must send this line next; compared as JSON values;
//! - `< …`: written to the client unchanged (any text, so a 0.13 reply can be scripted);
//! - `!sleep <ms>`: waits;
//! - `!close`: closes the connection here;
//! - `#` lines and empty lines are skipped.
//!
//! After the last line the server reads until the client closes the connection: any further
//! line from the client is a mismatch. So a test sees every request a command sends, and
//! a command that sends one request too many fails.

use crate::values::same;
use serde_json::Value;
use std::io::{BufRead, BufReader, ErrorKind, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

/// How long the server waits for the client at each step.
const STEP: Duration = Duration::from_secs(10);

/// A scripted daemon on one socket, serving one connection.
pub struct ScriptServer {
    path: PathBuf,
    handle: Option<JoinHandle<Result<Vec<String>, String>>>,
}

impl ScriptServer {
    /// Binds `path` (a stale file there is removed) and serves `transcript` to the first
    /// connection, in a thread.
    pub fn start(path: &Path, transcript: &str) -> ScriptServer {
        let _ = std::fs::remove_file(path);
        let listener = UnixListener::bind(path).expect("bind the script socket");
        let script = transcript.to_string();
        let handle = thread::spawn(move || serve(listener, &script));
        ScriptServer {
            path: path.to_path_buf(),
            handle: Some(handle),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Waits for the script to end: the lines the client sent, or the first mismatch.
    pub fn finish(mut self) -> Result<Vec<String>, String> {
        let handle = self.handle.take().expect("finish runs once");
        handle
            .join()
            .unwrap_or_else(|_| Err("the script thread panicked".into()))
    }
}

impl Drop for ScriptServer {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

fn accept(listener: &UnixListener) -> Result<UnixStream, String> {
    listener.set_nonblocking(true).map_err(|e| e.to_string())?;
    let until = Instant::now() + STEP;
    loop {
        match listener.accept() {
            Ok((stream, _)) => {
                stream.set_nonblocking(false).map_err(|e| e.to_string())?;
                return Ok(stream);
            }
            Err(e) if e.kind() == ErrorKind::WouldBlock && Instant::now() < until => {
                thread::sleep(Duration::from_millis(5));
            }
            Err(e) => return Err(format!("no client connected: {e}")),
        }
    }
}

/// The next line from the client; `None` at the end of the connection.
fn next_line(reader: &mut BufReader<UnixStream>) -> Result<Option<String>, String> {
    let mut line = String::new();
    match reader.read_line(&mut line) {
        Ok(0) => Ok(None),
        Ok(_) => Ok(Some(line.trim_end_matches('\n').to_string())),
        Err(e) => Err(format!("reading the client: {e}")),
    }
}

fn serve(listener: UnixListener, script: &str) -> Result<Vec<String>, String> {
    let stream = accept(&listener)?;
    stream
        .set_read_timeout(Some(STEP))
        .map_err(|e| e.to_string())?;
    let mut writer = stream.try_clone().map_err(|e| e.to_string())?;
    let mut reader = BufReader::new(stream);
    let mut sent = Vec::new();
    for (n, line) in script.lines().enumerate() {
        let at = n + 1;
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(want) = line.strip_prefix("> ") {
            let want: Value = serde_json::from_str(want)
                .map_err(|e| format!("script line {at} is not JSON: {e}"))?;
            let got = next_line(&mut reader)?
                .ok_or_else(|| format!("line {at}: the client closed instead of sending {want}"))?;
            let value: Value = serde_json::from_str(&got)
                .map_err(|e| format!("line {at}: the client sent no JSON ({e}): {got}"))?;
            if !same(&value, &want) {
                return Err(format!("line {at}: expected {want}, got {got}"));
            }
            sent.push(got);
        } else if let Some(text) = line.strip_prefix("< ") {
            writer
                .write_all(format!("{text}\n").as_bytes())
                .map_err(|e| format!("line {at}: writing to the client: {e}"))?;
        } else if let Some(ms) = line.strip_prefix("!sleep ") {
            let ms: u64 = ms.parse().map_err(|_| format!("line {at}: bad !sleep"))?;
            thread::sleep(Duration::from_millis(ms));
        } else if line == "!close" {
            return Ok(sent);
        } else {
            return Err(format!("script line {at} is not '> ', '< ', '!' or '#'"));
        }
    }
    match next_line(&mut reader)? {
        None => Ok(sent),
        Some(extra) => Err(format!("the client sent one line more: {extra}")),
    }
}
