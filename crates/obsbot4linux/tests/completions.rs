// SPDX-License-Identifier: GPL-3.0-or-later

//! The completions (design spec 6.2): the committed files under `packaging/completions/` are
//! current (`scripts/regenerate.sh` rewrites them, `OBSBOT_REGENERATE=1`), and the bash script
//! completes what the spec lists.

use obsbot4linux::completions::{bash, zsh};
use std::path::PathBuf;
use std::process::Command;

fn file(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../packaging/completions")
        .join(name)
}

#[test]
fn the_completion_files_are_current() {
    for (name, want) in [("obsbot4linux.bash", bash()), ("_obsbot4linux", zsh())] {
        let path = file(name);
        if std::env::var_os("OBSBOT_REGENERATE").is_some() {
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, &want).unwrap();
            continue;
        }
        let have = std::fs::read_to_string(&path).unwrap_or_default();
        assert!(
            have == want,
            "packaging/completions/{name} is stale: run scripts/regenerate.sh and commit the result"
        );
    }
}

/// What bash offers for `line` (the cursor at its end), with `=` in COMP_WORDBREAKS (bash's
/// default) or without it.
fn complete(line: &str, equals_breaks: bool) -> Vec<String> {
    let breaks = if equals_breaks {
        " \t\n\"'><=;|&(:"
    } else {
        " \t\n\"'><;|&(:"
    };
    let out = Command::new("bash")
        .arg("-c")
        .arg(
            r#"source "$SCRIPT"; COMP_WORDBREAKS=$BREAKS; COMP_LINE=$LINE; COMP_POINT=${#COMP_LINE};
               _obsbot4linux; printf '%s\n' "${COMPREPLY[@]}""#,
        )
        .env("SCRIPT", file("obsbot4linux.bash"))
        .env("LINE", line)
        .env("BREAKS", breaks)
        .output()
        .expect("bash runs");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout)
        .unwrap()
        .lines()
        .filter(|l| !l.is_empty())
        .map(String::from)
        .collect()
}

#[test]
fn bash_completes_subcommands_and_flags() {
    assert_eq!(complete("obsbot4linux s", true), ["status", "set"]);
    assert_eq!(
        complete("obsbot4linux --device fake-a st", true),
        ["status"]
    );
    assert_eq!(complete("obsbot4linux devices --r", true), ["--report"]);
    assert_eq!(complete("obsbot4linux --j", true), ["--json"]);
    assert!(
        complete("obsbot4linux --device ", true).is_empty(),
        "no --device values"
    );
    assert_eq!(complete("obsbot4linux watch --live p", true), ["pose"]);
}

#[test]
fn bash_completes_paths_values_actions_and_parameters() {
    assert_eq!(
        complete("obsbot4linux get zone.area.p", true),
        ["zone.area.pan"]
    );
    assert_eq!(
        complete("obsbot4linux describe preset.sa", true),
        ["preset.save"]
    );
    assert_eq!(
        complete("obsbot4linux set tracking.mo", true),
        ["tracking.mode="]
    );
    assert_eq!(
        complete("obsbot4linux set tracking.mode=up", true),
        ["upper_body"]
    );
    assert_eq!(
        complete("obsbot4linux set tracking.mode=up", false),
        ["tracking.mode=upper_body"]
    );
    assert_eq!(
        complete("obsbot4linux set image.mirror=o", true),
        ["off", "on"]
    );
    assert_eq!(complete("obsbot4linux do zoom.mo", true), ["zoom.move"]);
    assert_eq!(
        complete("obsbot4linux do zoom.move ", true),
        ["factor=", "speed="]
    );
    assert_eq!(
        complete("obsbot4linux do zoom.move speed=", true),
        ["fast", "medium", "slow"]
    );
    assert_eq!(
        complete("obsbot4linux do gimbal.turn ", true),
        ["pan_speed=", "tilt_speed="],
        "keep is sent by the command line itself"
    );
}

/// The zsh script must at least parse. zsh is not on every CI runner: run with
/// `cargo test -p obsbot4linux --test completions -- --ignored` where zsh is installed.
#[test]
#[ignore = "needs zsh"]
fn the_zsh_script_parses() {
    let status = Command::new("zsh")
        .arg("-n")
        .arg(file("_obsbot4linux"))
        .status()
        .expect("zsh runs");
    assert!(status.success());
}
