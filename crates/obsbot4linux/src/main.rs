// SPDX-License-Identifier: GPL-3.0-or-later

//! `obsbot4linux`: the command line (design spec 6.2), a pure client of protocol 2. Track C
//! writes it; the 0.13 command line stays on `main` for 0.14.x.

use std::process::ExitCode;

fn main() -> ExitCode {
    eprintln!("obsbot4linux: the 1.0 command line is not built yet (stage W0 of the 1.0 rebuild)");
    ExitCode::FAILURE
}
