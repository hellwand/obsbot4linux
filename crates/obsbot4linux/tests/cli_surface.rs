// SPDX-License-Identifier: GPL-3.0-or-later

//! CLI surface guard of the groundwork stage (rebuild plan, stage G): the `--help` text of the
//! CLI and of every subcommand, compared with `tests/fixtures/cli-surface.txt`, which was
//! written from the 0.13 code before the workspace move, and `--version`. Only `--help` and
//! `--version` run: nothing opens a device or a socket.
//!
//! `CLI_SURFACE_BLESS=1 cargo test --test cli_surface` writes the file: once when this test is
//! added, and on `main` in a change that alters a help text on purpose. A move never writes it;
//! every other run compares.

use std::process::Command;

const FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/cli-surface.txt"
);

fn run(args: &[&str]) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_obsbot4linux"))
        .args(args)
        .output()
        .expect("obsbot4linux runs");
    assert!(out.status.success(), "obsbot4linux {args:?} failed");
    String::from_utf8(out.stdout).expect("UTF-8 output")
}

/// The names in the "Commands:" section of a help text: the lines indented by exactly two
/// spaces (a longer indent continues a description).
fn subcommands(help: &str) -> Vec<String> {
    help.lines()
        .skip_while(|l| *l != "Commands:")
        .skip(1)
        .take_while(|l| l.starts_with(' '))
        .filter(|l| !l.starts_with("   "))
        .filter_map(|l| l.split_whitespace().next())
        .filter(|name| *name != "help")
        .map(String::from)
        .collect()
}

fn walk(path: &mut Vec<String>, out: &mut String) {
    let mut args: Vec<&str> = path.iter().map(String::as_str).collect();
    args.push("--help");
    let help = run(&args);
    out.push_str(&format!("=== obsbot4linux {}\n{help}", args.join(" ")));
    for sub in subcommands(&help) {
        path.push(sub);
        walk(path, out);
        path.pop();
    }
}

#[test]
fn help_texts_are_unchanged() {
    let mut out = String::new();
    walk(&mut Vec::new(), &mut out);
    // a hidden subcommand is not listed under "Commands:"
    walk(&mut vec!["gimbal".into(), "center".into()], &mut out);
    if std::env::var_os("CLI_SURFACE_BLESS").is_some() {
        std::fs::write(FIXTURE, &out).unwrap();
        return;
    }
    let recorded = std::fs::read_to_string(FIXTURE).expect("tests/fixtures/cli-surface.txt");
    assert_eq!(out, recorded, "a help text of the CLI changed");
}

#[test]
fn version_is_the_package_version() {
    // the workspace move must keep the version the binary reports
    assert_eq!(
        run(&["--version"]),
        format!("obsbot4linux {}\n", env!("CARGO_PKG_VERSION"))
    );
}
