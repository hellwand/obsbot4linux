// SPDX-License-Identifier: GPL-3.0-or-later

//! `obsbot4linux`: the command line (design spec 6.2). Everything is in the library, so that
//! tests run it in-process.

use std::ffi::OsString;
use std::io;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<OsString> = std::env::args_os().collect();
    let code = obsbot4linux::run(
        &args,
        obsbot_proto::socket_path(),
        true,
        &mut io::stdout(),
        &mut io::stderr(),
    );
    ExitCode::from(code)
}
