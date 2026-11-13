// SPDX-License-Identifier: GPL-3.0-or-later

//! Usage errors, help and version (design spec 6.2, "Errors"): nothing here needs a daemon.

mod common;

use common::cli;

/// Spec 10.3 C: the `--json` error shape for a usage error.
#[test]
fn a_usage_error_with_json_prints_the_error_object_on_stdout() {
    let (code, out, err) = cli(None, &["--json", "frobnicate"]);
    assert_eq!(code, 2);
    assert_eq!(err, "");
    let v: serde_json::Value = serde_json::from_str(out.trim_end()).unwrap();
    assert_eq!(v["error"]["code"], "usage");
    assert!(
        v["error"]["message"]
            .as_str()
            .unwrap()
            .contains("frobnicate"),
        "{out}"
    );
    assert_eq!(v.as_object().unwrap().len(), 1);
    assert_eq!(v["error"].as_object().unwrap().len(), 2, "code and message");
}

#[test]
fn json_after_the_subcommand_counts_too() {
    let (code, out, _) = cli(None, &["get", "--json"]);
    assert_eq!(code, 2);
    assert!(
        out.starts_with(r#"{"error":{"code":"usage","message":"#),
        "{out}"
    );
}

#[test]
fn a_usage_error_without_json_goes_to_stderr() {
    let (code, out, err) = cli(None, &["set"]);
    assert_eq!(code, 2);
    assert_eq!(out, "");
    assert!(err.starts_with("error: "), "{err}");
    assert!(err.contains("Usage: obsbot4linux set"), "{err}");
}

/// Spec 6.2: help and version stay plain text, also with `--json`.
#[test]
fn help_and_version_are_plain_text_and_exit_0() {
    let (code, out, _) = cli(None, &["--json", "--help"]);
    assert_eq!(code, 0);
    assert!(
        out.contains("Usage: obsbot4linux [OPTIONS] <COMMAND>"),
        "{out}"
    );
    for sub in ["status", "get", "set", "do", "describe", "devices", "watch"] {
        assert!(out.contains(&format!("  {sub}")), "{sub} in {out}");
    }
    let (code, out, _) = cli(None, &["--version"]);
    assert_eq!(code, 0);
    assert_eq!(out, format!("obsbot4linux {}\n", env!("CARGO_PKG_VERSION")));
}

/// Parity exception PX-3: `--direct` is gone.
#[test]
fn the_0_13_direct_flag_is_a_usage_error() {
    let (code, _, err) = cli(None, &["--direct", "status"]);
    assert_eq!(code, 2);
    assert!(err.contains("--direct"), "{err}");
}
