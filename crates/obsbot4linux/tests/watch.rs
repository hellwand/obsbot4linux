// SPDX-License-Identifier: GPL-3.0-or-later

//! `watch` against a scripted daemon (design spec 6.2): the hello reply as its first line,
//! then every line as received; exit 4 when the connection ends; exit 0 on SIGINT and
//! SIGTERM (the binary, in a child process).

mod common;

use common::{cli, hello_reply_line, socket, three};
use obsbot_proto::client::script::ScriptServer;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};

fn watch_hello(live: &str) -> String {
    format!(
        r#"> {{"id":1,"hello":{{"protocol":2,"client":"obsbot4linux {}","subscribe":true,"live":[{live}]}}}}"#,
        env!("CARGO_PKG_VERSION")
    )
}

const EVENTS: [&str; 3] = [
    r#"{"event":"devices","devices":[]}"#,
    r#"{"event":"frobnicate","from":"a newer daemon"}"#,
    r#"{"event":"state","device":"sn:FAKEA0000001","rev":2,"state":{"available":false,"pending":{},"unread":[],"x":1.50}}"#,
];

#[test]
fn watch_prints_the_hello_reply_and_every_line_as_received_then_exits_4() {
    let reply = hello_reply_line(&three());
    let mut lines = vec![watch_hello(r#""pose","target""#), reply.clone()];
    lines.extend(EVENTS.iter().map(|e| format!("< {e}")));
    lines.push("!close".into());
    let server = ScriptServer::start(&socket("watch"), &lines.join("\n"));
    let (code, out, err) = cli(Some(server.path()), &["watch", "--live", "pose", "target"]);
    assert_eq!(code, 4);
    let mut want = format!("{}\n", reply.strip_prefix("< ").unwrap());
    for e in EVENTS {
        want.push_str(e);
        want.push('\n');
    }
    // Unchanged: an unknown event type and `1.50` stay as the daemon wrote them.
    assert_eq!(out, want);
    assert_eq!(
        err,
        "obsbot4linux: the obsbot4linux daemon closed the connection\n"
    );
    assert_eq!(server.finish().unwrap().len(), 1, "watch sends only hello");
}

#[test]
fn watch_takes_no_device() {
    let (code, out, _) = cli(None, &["--json", "watch", "--device", "fake-a"]);
    assert_eq!(code, 2);
    assert_eq!(
        out,
        "{\"error\":{\"code\":\"usage\",\"message\":\"watch takes no --device: it prints the events of every device\"}}\n"
    );
}

/// Spec 6.2: `watch` exits 0 on SIGINT or SIGTERM.
#[test]
fn watch_exits_0_on_sigint_and_sigterm() {
    for (n, signal) in ["INT", "TERM"].into_iter().enumerate() {
        let script = [watch_hello(""), hello_reply_line(&three())].join("\n");
        let server = ScriptServer::start(&socket(&format!("watch-signal-{n}")), &script);
        let mut child = Command::new(env!("CARGO_BIN_EXE_obsbot4linux"))
            .arg("watch")
            .env("OBSBOT4LINUX_SOCKET", server.path())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let mut first = String::new();
        BufReader::new(child.stdout.take().unwrap())
            .read_line(&mut first)
            .unwrap();
        assert!(
            first.starts_with(r#"{"id":1,"ok":{"protocol":2"#),
            "{first}"
        );
        let killed = Command::new("kill")
            .args([format!("-{signal}"), child.id().to_string()])
            .status()
            .unwrap();
        assert!(killed.success());
        let status = child.wait().unwrap();
        assert_eq!(status.code(), Some(0), "SIG{signal}: {status:?}");
        server.finish().unwrap();
    }
}

/// A writer whose reader went away after `n` lines (`watch | head -1`).
struct Closes(usize);

impl std::io::Write for Closes {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if self.0 == 0 {
            return Err(std::io::ErrorKind::BrokenPipe.into());
        }
        self.0 -= buf.iter().filter(|b| **b == b'\n').count().min(self.0);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Review focus: `watch | head -1` ends quietly with 0 once stdout is gone.
#[test]
fn watch_into_a_closed_pipe_exits_0() {
    // One event only: the server has nothing left to write when the command line closes.
    let lines = [
        watch_hello(""),
        hello_reply_line(&three()),
        format!("< {}", EVENTS[0]),
    ];
    let server = ScriptServer::start(&socket("watch-pipe"), &lines.join("\n"));
    let args: Vec<std::ffi::OsString> = ["obsbot4linux", "watch"].iter().map(Into::into).collect();
    let mut err = Vec::new();
    let code = obsbot4linux::run(
        &args,
        Some(server.path().to_path_buf()),
        false,
        &mut Closes(1),
        &mut err,
    );
    assert_eq!(code, 0);
    assert!(err.is_empty());
    server.finish().unwrap();
}
