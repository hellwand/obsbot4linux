// SPDX-License-Identifier: GPL-3.0-or-later

//! `watch` (design spec 6.2): `hello` with `subscribe: true` and the given live topics; the
//! reply as its first line, then every line the daemon sends, unchanged, one JSON object per
//! line. It reads nothing from stdin, takes no `--device`, exits 0 on SIGINT or SIGTERM and
//! 4 when the connection ends.

use crate::Ctx;
use crate::errors::{CliCode, CliError};
use crate::session::{client_error, client_name};
use obsbot_proto::client::{ClientError, Connection};
use obsbot_proto::{ErrorCode, Hello};
use std::io::{self, Write};

/// Writes one line in one call, so that the signal thread never cuts a line.
fn line_out(out: &mut dyn Write, line: &str) -> io::Result<()> {
    out.write_all(format!("{line}\n").as_bytes())?;
    out.flush()
}

/// Blocks SIGINT and SIGTERM in this thread (the only one) and exits 0 when one arrives,
/// after the line being written is complete.
fn exit_on_signal() -> Result<(), CliError> {
    use nix::sys::signal::{SigSet, Signal};
    let mut set = SigSet::empty();
    set.add(Signal::SIGINT);
    set.add(Signal::SIGTERM);
    set.thread_block().map_err(|e| {
        CliError::new(
            CliCode::Proto(ErrorCode::Internal),
            format!("cannot block SIGINT and SIGTERM: {e}"),
        )
    })?;
    std::thread::spawn(move || {
        if set.wait().is_ok() {
            let _whole_lines = io::stdout().lock();
            std::process::exit(0);
        }
    });
    Ok(())
}

pub fn watch(ctx: &mut Ctx, live: &[String]) -> Result<(), CliError> {
    if ctx.device.is_some() {
        return Err(CliError::usage(
            "watch takes no --device: it prints the events of every device",
        ));
    }
    if ctx.exit_on_signal {
        exit_on_signal()?;
    }
    let path = ctx
        .socket
        .as_deref()
        .ok_or_else(|| client_error(ClientError::NoSocketPath))?;
    let mut conn = Connection::open(path).map_err(client_error)?;
    let topics: Vec<&str> = live.iter().map(String::as_str).collect();
    let hello = Hello::new(&client_name()).subscribe(true).live(&topics);
    let reply = conn.hello(hello).map_err(client_error)?;
    if line_out(ctx.out, &reply.line).is_err() {
        return Ok(());
    }
    loop {
        let line = conn.read_line(None).map_err(client_error)?;
        // stdout closed (a pipe whose reader ended): stop like an interrupt.
        if line_out(ctx.out, &line).is_err() {
            return Ok(());
        }
    }
}
