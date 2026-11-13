// SPDX-License-Identifier: GPL-3.0-or-later

//! The command line of `obsbot4linuxd` (5.12): `--fake <model>[,<model>…]` runs the daemon with
//! fake devices instead of the sysfs scan.

use obsbot_device::models::fake::KEYS;

pub const USAGE: &str = "usage: obsbot4linuxd [--fake <model>[,<model>...]]

The obsbot4linux daemon. It is started by the systemd user socket obsbot4linux.socket.

  --fake <models>  run with fake devices instead of the cameras (fake-a, fake-b, unknown);
                   the test-control socket is <socket>.test
  --help           print this help
  --version        print the version";

/// What the command line asks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Args {
    /// Run the daemon; `fake` holds the fake models, in plug order.
    Run {
        fake: Option<Vec<String>>,
    },
    Help,
    Version,
}

/// Parses the arguments after the program name.
pub fn parse(args: &[String]) -> Result<Args, String> {
    let mut fake: Option<Vec<String>> = None;
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        let list = match arg.as_str() {
            "--help" | "-h" => return Ok(Args::Help),
            "--version" | "-V" => return Ok(Args::Version),
            "--fake" => it
                .next()
                .ok_or_else(|| "--fake needs a list of models".to_string())?
                .clone(),
            other => match other.strip_prefix("--fake=") {
                Some(list) => list.to_string(),
                None => return Err(format!("unknown argument {other}")),
            },
        };
        if fake.is_some() {
            return Err("--fake given twice".into());
        }
        let models: Vec<String> = list.split(',').map(|m| m.trim().to_string()).collect();
        if let Some(bad) = models.iter().find(|m| !KEYS.contains(&m.as_str())) {
            return Err(format!(
                "unknown fake model {bad:?}; known: {}",
                KEYS.join(", ")
            ));
        }
        fake = Some(models);
    }
    Ok(Args::Run { fake })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(args: &[&str]) -> Result<Args, String> {
        parse(&args.iter().map(|a| a.to_string()).collect::<Vec<_>>())
    }

    #[test]
    fn no_arguments_run_the_real_devices() {
        assert_eq!(p(&[]), Ok(Args::Run { fake: None }));
    }

    /// 5.12 and 10.3: CI runs `--fake fake-a,fake-b,unknown`.
    #[test]
    fn fake_takes_a_comma_list_in_plug_order() {
        let want = Ok(Args::Run {
            fake: Some(vec!["fake-a".into(), "fake-b".into(), "unknown".into()]),
        });
        assert_eq!(p(&["--fake", "fake-a,fake-b,unknown"]), want);
        assert_eq!(p(&["--fake=fake-a,fake-b,unknown"]), want);
        assert_eq!(
            p(&["--fake", "fake-b,fake-b"]),
            Ok(Args::Run {
                fake: Some(vec!["fake-b".into(), "fake-b".into()])
            }),
            "a model may repeat"
        );
    }

    #[test]
    fn bad_command_lines_are_refused_with_a_reason() {
        assert!(p(&["--fake"]).unwrap_err().contains("needs a list"));
        assert!(
            p(&["--fake", "tiny2lite"])
                .unwrap_err()
                .contains("unknown fake model")
        );
        assert!(
            p(&["--fake", ""])
                .unwrap_err()
                .contains("unknown fake model")
        );
        assert!(
            p(&["--fake", "fake-a", "--fake", "fake-b"])
                .unwrap_err()
                .contains("twice")
        );
        assert!(p(&["daemon"]).unwrap_err().contains("unknown argument"));
        assert_eq!(p(&["--help"]), Ok(Args::Help));
        assert_eq!(p(&["--version"]), Ok(Args::Version));
    }
}
