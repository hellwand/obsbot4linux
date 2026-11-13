// SPDX-License-Identifier: GPL-3.0-or-later

//! The command-line definition (design spec 6.2). It maps 1:1 onto protocol 2: no
//! per-feature subcommands and no shortcuts. The completions are generated from it
//! ([`command`]) and from the embedded description files.

use clap::{ColorChoice, CommandFactory, Parser, Subcommand};
use std::ffi::OsString;

/// obsbot4linux: control OBSBOT cameras through the obsbot4linux daemon.
#[derive(Parser, Debug, PartialEq)]
#[command(
    name = "obsbot4linux",
    version,
    disable_help_subcommand = true,
    color = ColorChoice::Never,
    after_help = "Values: bool on|off|true|false; enum by name; number 1.5; range min,max; \
pose pan,tilt,zoom; point x,y; string as given."
)]
pub struct Cli {
    /// Print the protocol objects as JSON
    #[arg(long, global = true)]
    pub json: bool,
    /// The device: its full id, a serial suffix of at least four characters, or a model key
    #[arg(long, global = true, value_name = "ID")]
    pub device: Option<String>,
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug, PartialEq)]
pub enum Command {
    /// The state of one device, grouped
    Status,
    /// Read leaves or groups
    Get {
        #[arg(required = true, value_name = "PATH")]
        paths: Vec<String>,
    },
    /// Write one patch
    Set {
        #[arg(required = true, value_name = "PATH=VALUE")]
        assignments: Vec<String>,
    },
    /// Run an action
    Do {
        #[arg(value_name = "ACTION")]
        action: String,
        #[arg(value_name = "PARAM=VALUE")]
        params: Vec<String>,
    },
    /// The device description, or one setting or action of it
    Describe {
        #[arg(value_name = "PATH|ACTION")]
        target: Option<String>,
    },
    /// The device list
    Devices {
        /// Print what an issue report needs (USB facts, no serial)
        #[arg(long)]
        report: bool,
    },
    /// Print the hello reply and every event as JSON lines
    Watch {
        /// Live topics to receive (pose, target, auto, …)
        #[arg(long, value_name = "TOPIC", num_args = 1..)]
        live: Vec<String>,
    },
}

/// The clap command, for the completion generator.
pub fn command() -> clap::Command {
    Cli::command()
}

/// Parses the arguments (the program name first).
pub fn parse(args: &[OsString]) -> Result<Cli, clap::Error> {
    Cli::try_parse_from(args)
}

/// Whether `--json` is among the arguments before a `--`. Used when parsing fails, so that a
/// usage error still prints JSON (spec 6.2, "Errors").
pub fn wants_json(args: &[OsString]) -> bool {
    args.iter()
        .skip(1)
        .take_while(|a| *a != "--")
        .any(|a| a == "--json")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn os(args: &[&str]) -> Vec<OsString> {
        std::iter::once("obsbot4linux")
            .chain(args.iter().copied())
            .map(OsString::from)
            .collect()
    }

    #[test]
    fn the_definition_is_consistent() {
        command().debug_assert();
    }

    #[test]
    fn flags_go_before_or_after_the_subcommand() {
        let a = parse(&os(&["--json", "--device", "fake-a", "status"])).unwrap();
        let b = parse(&os(&["status", "--device", "fake-a", "--json"])).unwrap();
        assert_eq!(a, b);
        assert!(a.json);
        assert_eq!(a.device.as_deref(), Some("fake-a"));
        assert_eq!(a.command, Command::Status);
    }

    /// Spec 6.2: a key=value argument never starts with `-`; free text after `=` may.
    #[test]
    fn values_after_the_equals_sign_may_start_with_a_minus() {
        let cli = parse(&os(&["do", "gimbal.move", "pan=-20", "tilt=-5.5"])).unwrap();
        assert_eq!(
            cli.command,
            Command::Do {
                action: "gimbal.move".into(),
                params: vec!["pan=-20".into(), "tilt=-5.5".into()],
            }
        );
        let cli = parse(&os(&["set", "zone.area.pan=-20,20", "led.brightness=2"])).unwrap();
        assert_eq!(
            cli.command,
            Command::Set {
                assignments: vec!["zone.area.pan=-20,20".into(), "led.brightness=2".into()],
            }
        );
    }

    #[test]
    fn get_and_set_need_at_least_one_argument() {
        assert!(parse(&os(&["get"])).is_err());
        assert!(parse(&os(&["set"])).is_err());
        assert!(parse(&os(&["do"])).is_err());
    }

    #[test]
    fn watch_takes_live_topics() {
        let cli = parse(&os(&["watch", "--live", "pose", "target"])).unwrap();
        assert_eq!(
            cli.command,
            Command::Watch {
                live: vec!["pose".into(), "target".into()],
            }
        );
    }

    #[test]
    fn json_is_found_before_a_double_dash_only() {
        assert!(wants_json(&os(&["set", "--json", "x"])));
        assert!(!wants_json(&os(&["set", "--", "--json"])));
        assert!(!wants_json(&os(&["set", "x"])));
    }
}
