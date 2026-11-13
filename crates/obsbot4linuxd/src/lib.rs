// SPDX-License-Identifier: GPL-3.0-or-later

//! The obsbot4linux daemon (design spec 5) as a library, so that tests run it in-process on a
//! temporary socket (section 11). `Config` and `run` are the entry point fixed in stage W0.
//!
//! The daemon knows devices, groups, tiers, leaves, actions and live topics. It knows no
//! command id, no offset and no selector of any model: everything model-specific comes through
//! `trait Camera` and `trait Bus` (section 5, 8.2).

// Removed by task b-14-tests-hotplug-process, the last task that wires a worker feature;
// until then the stubs of the skeleton and the features still to come leave items unused.
#![allow(dead_code, unused_variables, unused_imports)]

mod batch;
mod conn;
mod control;
mod daemon;
mod hotplug;
mod identity;
mod lines;
mod names;
mod outbound;
mod pending;
mod queue;
mod registry;
mod schedule;
mod socket;
mod table;
mod worker;

#[doc(hidden)]
pub mod args;

use obsbot_device::contract::Tier;
use obsbot_device::models::fake::FakeBus;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Duration;

/// At most this many connections at once; a further one is closed at once (4.1).
pub const MAX_CONNECTIONS: usize = 64;
/// At most this many requests queued per device; one more is answered `busy` (4.1, 5.6).
pub const MAX_QUEUED: usize = 32;
/// The outbound queue of one connection; when it is full the connection is closed (5.1).
pub const OUTBOUND_LINES: usize = 64;
/// The pause after a failed `accept` (5.8 `ACCEPT_BACKOFF`).
pub const ACCEPT_BACKOFF: Duration = Duration::from_millis(100);
/// How often queued requests are checked against their deadline (4.1 rule 9).
pub const SWEEP_EVERY: Duration = Duration::from_millis(50);

/// How one daemon runs.
pub struct Config {
    /// The socket to bind when systemd passes none (`LISTEN_FDS`). The lock file is
    /// `<socket>.lock` (5.9); in fake mode the test-control socket is `<socket>.test` (5.12).
    pub socket: PathBuf,
    /// `None`: the real devices (the sysfs scan). `Some`: fake mode with these fake devices
    /// (`obsbot4linuxd --fake fake-a,fake-b,unknown` builds it with `FakeBus::new`).
    pub fake: Option<Arc<FakeBus>>,
    /// Set to `true` to end `run` as on SIGTERM (5.9).
    pub stop: Arc<AtomicBool>,
}

impl Config {
    /// Real devices on `socket`, not stopped.
    pub fn new(socket: PathBuf) -> Config {
        Config {
            socket,
            fake: None,
            stop: Arc::new(AtomicBool::new(false)),
        }
    }
}

/// The intervals the daemon runs on. `run` always uses [`Timing::SPEC`]; in-process tests
/// may pass shorter tier periods and scan intervals to [`run_with`], so that a 60 s tier or a
/// 5 s identity attempt is seen within a test's time. Not part of the daemon's interface.
#[doc(hidden)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Timing {
    /// The sysfs (or fake bus) scan interval (5.3: once a second).
    pub scan: Duration,
    /// `REQUEST_DEADLINE` (4.1 rule 9, 5.8); also the `hello` bound (4.1) and the quiet time
    /// after a `restart` write (5.6).
    pub request_deadline: Duration,
    /// The periods of the tiers `Second`, `FiveSeconds` and `Minute` (5.7).
    pub second: Duration,
    pub five_seconds: Duration,
    pub minute: Duration,
}

impl Timing {
    /// The values of the design spec: scan 1 s, deadline 15 s, tiers 1 s, 5 s and 60 s.
    pub const SPEC: Timing = Timing {
        scan: Duration::from_secs(1),
        request_deadline: obsbot_proto::REQUEST_DEADLINE,
        second: Duration::from_secs(1),
        five_seconds: Duration::from_secs(5),
        minute: Duration::from_secs(60),
    };

    /// The period of a tier under this timing; `None` for `Touched`.
    pub fn period(&self, tier: Tier) -> Option<Duration> {
        match tier {
            Tier::Second => Some(self.second),
            Tier::FiveSeconds => Some(self.five_seconds),
            Tier::Minute => Some(self.minute),
            Tier::Touched => None,
        }
    }
}

/// Runs the daemon until `config.stop` is set or a signal ends it. Returns once the socket,
/// the lock and every worker are released.
pub fn run(config: Config) -> std::io::Result<()> {
    run_with(config, Timing::SPEC)
}

/// [`run`] with other intervals; for in-process tests only.
#[doc(hidden)]
pub fn run_with(config: Config, timing: Timing) -> std::io::Result<()> {
    daemon::run(config, timing)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `run` must use the spec's intervals: the tier periods of the contract, the 15 s
    /// deadline of `obsbot-proto` and one scan a second (5.3, 5.7, 5.8).
    #[test]
    fn run_uses_the_intervals_of_the_spec() {
        let t = Timing::SPEC;
        for tier in [Tier::Second, Tier::FiveSeconds, Tier::Minute, Tier::Touched] {
            assert_eq!(t.period(tier), tier.period(), "{tier:?}");
        }
        assert_eq!(t.request_deadline, obsbot_proto::REQUEST_DEADLINE);
        assert_eq!(t.scan, Duration::from_secs(1));
    }

    #[test]
    fn the_daemon_limits_are_the_spec_values() {
        assert_eq!(MAX_CONNECTIONS, 64);
        assert_eq!(MAX_QUEUED, 32);
        assert_eq!(OUTBOUND_LINES, 64);
        assert_eq!(ACCEPT_BACKOFF, Duration::from_millis(100));
    }
}
