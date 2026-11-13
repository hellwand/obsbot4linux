// SPDX-License-Identifier: GPL-3.0-or-later

//! `obsbot4linux`: the command line (design spec 6.2), a pure client of protocol 2. Every
//! command sends the one request it maps to and chains no other (spec 2): `status` and `get`
//! a `get`, `set` a `set`, `do` a `do`; `describe`, `devices` and `watch` only `hello`.

pub mod args;
pub mod completions;
pub mod describe;
pub mod errors;
pub mod read;
pub mod render;
pub mod select;
pub mod session;
pub mod value;
pub mod watch;
pub mod write;

use args::Command;
use errors::{CliError, report};
use std::ffi::OsString;
use std::io::Write;
use std::path::PathBuf;

/// The version this command line reports and names in its messages.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// What every command gets.
pub struct Ctx<'a> {
    /// The daemon's socket (`None`: neither `OBSBOT4LINUX_SOCKET` nor `XDG_RUNTIME_DIR` set).
    pub socket: Option<PathBuf>,
    /// `--device` as given.
    pub device: Option<String>,
    /// `--json`.
    pub json: bool,
    /// `watch` exits 0 on SIGINT and SIGTERM. Only the binary sets it: in-process tests
    /// must not change the test process's signal handling.
    pub exit_on_signal: bool,
    pub out: &'a mut dyn Write,
    pub err: &'a mut dyn Write,
}

/// Runs one command line (`args` starts with the program name) and returns its exit code.
pub fn run(
    args: &[OsString],
    socket: Option<PathBuf>,
    exit_on_signal: bool,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> u8 {
    let cli = match args::parse(args) {
        Ok(cli) => cli,
        Err(e) => return clap_error(&e, args::wants_json(args), out, err),
    };
    let mut ctx = Ctx {
        socket,
        device: cli.device,
        json: cli.json,
        exit_on_signal,
        out,
        err,
    };
    let result = match &cli.command {
        Command::Status => read::status(&mut ctx),
        Command::Get { paths } => read::get(&mut ctx, paths),
        Command::Describe { target } => read::describe(&mut ctx, target.as_deref()),
        Command::Devices { report } => read::devices(&mut ctx, *report),
        Command::Set { assignments } => write::set(&mut ctx, assignments),
        Command::Do { action, params } => write::act(&mut ctx, action, params),
        Command::Watch { live } => watch::watch(&mut ctx, live),
    };
    match result {
        Ok(()) => 0,
        Err(e) => report(&e, ctx.json, ctx.out, ctx.err),
    }
}

/// Help and version print as plain text and exit 0 (spec 6.2: they stay plain text with
/// `--json`); every other parse error is `usage` (exit 2).
fn clap_error(e: &clap::Error, json: bool, out: &mut dyn Write, err: &mut dyn Write) -> u8 {
    use clap::error::ErrorKind;
    let text = e.render().to_string();
    if matches!(e.kind(), ErrorKind::DisplayHelp | ErrorKind::DisplayVersion) {
        let _ = write!(out, "{text}");
        return 0;
    }
    if json {
        let first = text.lines().next().unwrap_or_default();
        let message = first.strip_prefix("error: ").unwrap_or(first);
        return report(&CliError::usage(message), true, out, err);
    }
    let _ = write!(err, "{text}");
    errors::CliCode::Usage.exit()
}
