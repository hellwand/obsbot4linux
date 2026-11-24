// SPDX-License-Identifier: GPL-3.0-or-later

//! `scripts/check-model.sh` (design spec 9.4) against `fake-a` and `fake-b` with
//! `--assume-pass`, as stage W2's acceptance asks ("the check script passes against fake-a and
//! fake-b in CI"). The daemon runs in-process in fake mode with short poll tiers, so that a
//! lagging readback shows within the test's time; the script runs as it would on a camera,
//! with the command line of this crate. The person's answers of a run without `--assume-pass`
//! come from a file (`CHECK_MODEL_TTY`). Needs bash and jq.

mod common;

use obsbot_device::models::fake::{FakeBus, Op};
use obsbot4linuxd::{Config, Timing};
use serde_json::Value;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

const A: &str = "sn:FAKEA0000001";
const B: &str = "sn:FAKEB0000001";

/// Poll tiers short enough that a value of fake-b's 60 s group shows within a second; the
/// deadline is the spec's.
const FAST: Timing = Timing {
    scan: Duration::from_millis(50),
    request_deadline: obsbot_proto::REQUEST_DEADLINE,
    second: Duration::from_millis(100),
    five_seconds: Duration::from_millis(200),
    minute: Duration::from_millis(400),
};

struct Daemon {
    socket: PathBuf,
    bus: Arc<FakeBus>,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<std::io::Result<()>>>,
}

impl Daemon {
    fn start(name: &str) -> Daemon {
        let socket = common::socket(&format!("m-{name}"));
        let _ = std::fs::remove_file(&socket);
        let bus = Arc::new(FakeBus::new(&["fake-a", "fake-b", "unknown"]).expect("the fakes"));
        let mut config = Config::new(socket.clone());
        config.fake = Some(Arc::clone(&bus));
        let stop = Arc::clone(&config.stop);
        let thread = thread::spawn(move || obsbot4linuxd::run_with(config, FAST));
        let until = Instant::now() + Duration::from_secs(20);
        while UnixStream::connect(&socket).is_err() {
            assert!(!thread.is_finished(), "the fake daemon ended");
            assert!(Instant::now() < until, "the fake daemon does not accept");
            thread::sleep(Duration::from_millis(20));
        }
        Daemon {
            socket,
            bus,
            stop,
            thread: Some(thread),
        }
    }

    /// The full state of one device, as `status --json` prints it.
    fn state(&self, id: &str) -> Value {
        let (code, out, err) =
            common::cli(Some(&self.socket), &["status", "--json", "--device", id]);
        assert_eq!(code, 0, "{err}");
        serde_json::from_str::<Value>(&out).unwrap()["state"].clone()
    }

    /// The script with the given arguments; the results file is returned with the output.
    fn command(&self, results: &Path, cli: &Path, args: &[&str]) -> Command {
        let mut c = Command::new("bash");
        c.arg(script())
            .args(args)
            .arg("--results")
            .arg(results)
            .env("OBSBOT4LINUX", cli)
            .env("OBSBOT4LINUX_SOCKET", &self.socket)
            .env("CHECK_MODEL_WAIT", "5")
            .env("CHECK_MODEL_SLEEP_WAIT", "5")
            .env("CHECK_MODEL_RETURN_WAIT", "10")
            .env("CHECK_MODEL_POLL", "0.05")
            .stdin(Stdio::null());
        c
    }

    fn check(&self, name: &str, args: &[&str]) -> (Output, Vec<Row>) {
        let results = results_file(name);
        let out = self
            .command(
                &results,
                Path::new(env!("CARGO_BIN_EXE_obsbot4linux")),
                args,
            )
            .output()
            .expect("bash runs");
        (out, rows(&results))
    }
}

impl Drop for Daemon {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
        // the daemon keeps its `<socket>.lock` (5.9); the test's daemon is gone, so is it
        let mut lock = self.socket.clone().into_os_string();
        lock.push(".lock");
        let _ = std::fs::remove_file(lock);
    }
}

fn script() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../scripts/check-model.sh")
}

fn results_file(name: &str) -> PathBuf {
    let path = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("check-model-{name}.tsv"));
    let _ = std::fs::remove_file(&path);
    path
}

/// One line of the results file: kind, path, value, result.
#[derive(Debug, Clone)]
struct Row {
    kind: String,
    path: String,
    value: String,
    result: String,
    seen: String,
    how: String,
}

/// The rows written so far. The script may be in the middle of a line (the SIGTERM test reads
/// while it runs): only lines that end in a newline and hold every column are rows.
fn rows(results: &Path) -> Vec<Row> {
    let text = std::fs::read_to_string(results).unwrap_or_default();
    let complete = &text[..text.rfind('\n').map_or(0, |i| i + 1)];
    complete
        .lines()
        .filter(|l| !l.starts_with('#'))
        .filter_map(|l| {
            let f: Vec<&str> = l.split('\t').collect();
            (f.len() == 7).then(|| Row {
                kind: f[0].into(),
                path: f[1].into(),
                value: f[2].into(),
                result: f[3].into(),
                seen: f[5].into(),
                how: f[6].into(),
            })
        })
        .collect()
}

fn row<'a>(rows: &'a [Row], kind: &str, path: &str, value: &str) -> &'a Row {
    rows.iter()
        .find(|r| r.kind == kind && r.path == path && r.value == value)
        .unwrap_or_else(|| panic!("no row {kind} {path} {value} in {rows:#?}"))
}

fn report(out: &Output) -> String {
    format!(
        "exit {:?}\n--- stdout\n{}\n--- stderr\n{}",
        out.status.code(),
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

/// The writable leaves of a device that read back, with their values in `state`.
fn writable_values(state: &Value, model: &str) -> Vec<(String, Value)> {
    let d = obsbot_device::models::fake::description(model).unwrap();
    d.settings
        .iter()
        .filter(|(_, s)| !s.readonly && s.readback)
        .map(|(p, _)| {
            let v = p.split('.').fold(state, |v, k| &v[k]).clone();
            (p.clone(), v)
        })
        .collect()
}

/// Whether two JSON values are the same, numbers compared as numbers (the script prints `5`
/// where a state may hold `5.0`).
fn same_json(a: &Value, b: &Value) -> bool {
    match (a.as_f64(), b.as_f64()) {
        (Some(x), Some(y)) => (x - y).abs() < 1e-9,
        _ => a == b,
    }
}

/// The preset slots and view ids of a device, with their names, as the state shows them.
fn lists(state: &Value) -> (Value, Value) {
    (state["presets"].clone(), state["views"].clone())
}

/// The `Act` operations of a device since `from`, as (action, parameters).
fn acts(d: &Daemon, from: usize) -> Vec<(String, serde_json::Map<String, Value>)> {
    d.bus
        .ops(A)
        .into_iter()
        .skip(from)
        .filter_map(|op| match op {
            Op::Act(name, params) => Some((name, params)),
            _ => None,
        })
        .collect()
}

/// fake-a, the Tiny 2 Lite's shape: every row passes, the `requires` rows included, the
/// restart rows run last and pass when the device is back under its id, and afterwards every
/// leaf the script wrote has its value from before the run.
#[test]
fn fake_a_passes_every_row_and_is_left_as_it_was() {
    let d = Daemon::start("a");
    let before = writable_values(&d.state(A), "fake-a");
    let (out, rows) = d.check(
        "a",
        &[
            "--device",
            "fake-a",
            "--assume-pass",
            "--asleep",
            "--destructive",
        ],
    );
    assert_eq!(out.status.code(), Some(0), "{}", report(&out));
    let failed: Vec<&Row> = rows
        .iter()
        .filter(|r| r.result.starts_with("fail"))
        .collect();
    assert!(failed.is_empty(), "{failed:#?}");
    // every writable enum value, both bools, number bounds, range span (9.4)
    for value in ["\"auto\"", "\"manual\""] {
        assert_eq!(row(&rows, "setting", "exposure.mode", value).result, "pass");
    }
    assert_eq!(row(&rows, "setting", "exposure.time", "100").result, "pass");
    assert_eq!(
        row(&rows, "setting", "exposure.time", "250000").result,
        "pass"
    );
    assert_eq!(
        row(
            &rows,
            "setting",
            "zone.area.pan",
            "{\"min\":-150,\"max\":150}"
        )
        .result,
        "pass"
    );
    assert_eq!(
        row(&rows, "setting", "led.blink", "true").result,
        "pass (confirmed)"
    );
    // the restart rows (spec 9.4: only with --destructive, last)
    let restart = row(&rows, "setting", "audio.enabled", "false");
    assert_eq!(
        (restart.result.as_str(), restart.seen.as_str()),
        ("pass", "returned")
    );
    let reset = rows.iter().find(|r| r.path == "factory_reset").unwrap();
    assert_eq!(
        (reset.result.as_str(), reset.seen.as_str()),
        ("pass", "returned")
    );
    let last_two: Vec<&str> = rows.iter().rev().take(3).map(|r| r.path.as_str()).collect();
    assert_eq!(
        last_two,
        ["factory_reset", "audio.enabled", "audio.enabled"]
    );
    // asleep rows only for `asleep: unknown`; zoom.field_of_view is `ignored` (measured)
    assert!(
        rows.iter()
            .any(|r| r.kind == "asleep" && r.path == "audio.volume")
    );
    assert!(
        !rows
            .iter()
            .any(|r| r.kind == "asleep" && r.path == "zoom.field_of_view")
    );
    // an asleep row writes a value that differs from the snapshot (otherwise `accepted` proves
    // nothing), and a leaf with `requires` gets its setup while awake: no row is answered
    // with a refusal of the setup (`conflict`)
    for r in rows.iter().filter(|r| r.kind == "asleep") {
        let was = before.iter().find(|(p, _)| *p == r.path).map(|(_, v)| v);
        let written = serde_json::from_str::<Value>(&r.value).ok();
        if let (Some(was), Some(written)) = (was, written) {
            assert!(
                !same_json(was, &written),
                "asleep {} writes the value it already shows ({was})",
                r.path
            );
        }
        assert!(!r.result.contains("conflict"), "{r:?}");
    }
    assert!(
        rows.iter()
            .any(|r| r.kind == "asleep" && r.path == "exposure.time" && r.result == "accepted"),
        "exposure.time asleep, after its setup: {rows:#?}"
    );
    // every row says which kind it is (spec 9.4)
    assert!(rows.iter().all(|r| !r.how.is_empty()), "{rows:#?}");
    assert_eq!(
        row(&rows, "setting", "led.blink", "true").how,
        "readback:false (person in view)"
    );
    assert!(
        String::from_utf8_lossy(&out.stdout).contains("not restored by this script"),
        "an action's question says what the script does not restore"
    );
    // a turn is always followed by a stop, awake and asleep
    let acts = acts(&d, 0);
    let turns = acts.iter().filter(|(a, _)| a == "gimbal.turn").count();
    let stops = acts.iter().filter(|(a, _)| a == "gimbal.stop").count();
    assert!(turns >= 2 && stops > turns, "{turns} turns, {stops} stops");
    // left as it was
    let after = writable_values(&d.state(A), "fake-a");
    let changed: Vec<_> = before
        .iter()
        .zip(&after)
        .filter(|(b, a)| b.1 != a.1)
        .map(|(b, a)| (b.0.clone(), b.1.clone(), a.1.clone()))
        .collect();
    assert!(changed.is_empty(), "not restored: {changed:#?}");
}

/// fake-b, of another shape: its own leaves and values, a lagging readback seen at a later
/// read than the reply's, and its 60 s group.
#[test]
fn fake_b_passes_every_row_and_tells_the_reply_from_a_later_read() {
    let d = Daemon::start("b");
    let before = writable_values(&d.state(B), "fake-b");
    let (out, rows) = d.check("b", &["--device", "fake-b", "--assume-pass", "--asleep"]);
    assert_eq!(out.status.code(), Some(0), "{}", report(&out));
    assert!(
        !rows.iter().any(|r| r.result.starts_with("fail")),
        "{rows:#?}"
    );
    for value in ["\"red\"", "\"green\"", "\"blue\""] {
        let r = row(&rows, "setting", "beacon.color", value);
        assert_eq!(r.result, "pass");
    }
    // `seen` follows the rev: the state of the reply's own rev did not show the lagging
    // brightness (250 ms) however slow the command line was to start
    assert_eq!(
        row(&rows, "setting", "led.brightness", "5").seen,
        "later",
        "fake-b's lagging readback shows after the reply: {rows:#?}"
    );
    assert!(
        rows.iter()
            .any(|r| r.kind == "setting" && r.seen == "reply")
    );
    assert!(
        !rows.iter().any(|r| r.path.starts_with("gimbal")),
        "fake-b has no gimbal"
    );
    let after = writable_values(&d.state(B), "fake-b");
    assert_eq!(before, after, "fake-b is left as it was");
}

/// Without `--destructive` the rows that restart the camera are listed and nothing of them
/// reaches the fake.
#[test]
fn without_destructive_no_restart_row_reaches_the_camera() {
    let d = Daemon::start("nd");
    let (out, rows) = d.check("nd", &["--device", "fake-a", "--assume-pass"]);
    assert_eq!(out.status.code(), Some(0), "{}", report(&out));
    assert_eq!(
        row(&rows, "setting", "audio.enabled", "-").result,
        "not run (--destructive)"
    );
    assert_eq!(
        row(&rows, "action", "factory_reset", "-").result,
        "not run (--destructive)"
    );
    let ops = d.bus.ops(A);
    assert!(
        !ops.iter().any(|op| matches!(op,
            Op::Apply(p, _) if p == "audio.enabled")
            || matches!(op, Op::Act(a, _) if a == "factory_reset")),
        "{ops:?}"
    );
}

/// A row fails when the state never shows the written value: here a command line that
/// answers `set led.brightness=…` without sending it. The script exits 1.
#[test]
fn a_value_that_never_shows_fails_its_row() {
    let d = Daemon::start("f");
    let wrapper = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/drop-led-cli.sh");
    let results = results_file("f");
    let out = d
        .command(&results, &wrapper, &["--device", "fake-b", "--assume-pass"])
        .env("DROP_LED_REAL_CLI", env!("CARGO_BIN_EXE_obsbot4linux"))
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1), "{}", report(&out));
    let rows = rows(&results);
    assert_eq!(row(&rows, "setting", "led.brightness", "0").result, "pass");
    assert!(
        row(&rows, "setting", "led.brightness", "5")
            .result
            .starts_with("fail: not shown"),
        "{rows:#?}"
    );
}

/// A run ended by SIGTERM still sets every leaf it wrote back to its snapshot value (9.4: the
/// trap on EXIT, INT and TERM).
#[test]
fn a_terminated_run_restores_what_it_wrote() {
    let d = Daemon::start("t");
    let before = writable_values(&d.state(A), "fake-a");
    let results = results_file("t");
    let mut child: Child = d
        .command(
            &results,
            Path::new(env!("CARGO_BIN_EXE_obsbot4linux")),
            &["--device", "fake-a", "--assume-pass"],
        )
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    // a probe that returns "not yet": wait until a few rows have run
    let until = Instant::now() + Duration::from_secs(60);
    while rows(&results).len() < 12 {
        assert!(Instant::now() < until, "the script wrote no rows");
        assert!(
            child.try_wait().unwrap().is_none(),
            "the script ended early"
        );
        thread::sleep(Duration::from_millis(20));
    }
    nix::sys::signal::kill(
        nix::unistd::Pid::from_raw(child.id() as i32),
        nix::sys::signal::Signal::SIGTERM,
    )
    .unwrap();
    let status = child.wait().unwrap();
    assert_eq!(status.code(), Some(143));
    let after = writable_values(&d.state(A), "fake-a");
    assert_eq!(before, after, "restored after SIGTERM");
}

/// The person's answers come from a file, one per line (`CHECK_MODEL_TTY`).
fn answers(name: &str, text: &str) -> PathBuf {
    let path =
        PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("check-model-{name}.answers"));
    std::fs::write(&path, text).unwrap();
    path
}

/// Every action and every asleep row asks first (9.4: "run one by one after a confirmation").
/// A person who answers no sends nothing but the sleeps and wakes of the asleep rows of
/// settings: no preset, view, power-on or gimbal action reaches the camera, and no action row
/// is recorded as run.
#[test]
fn a_person_who_declines_every_action_sends_none() {
    let d = Daemon::start("no");
    let results = results_file("no");
    let out = d
        .command(
            &results,
            Path::new(env!("CARGO_BIN_EXE_obsbot4linux")),
            &["--device", "fake-a", "--asleep"],
        )
        .env("CHECK_MODEL_TTY", answers("no", &"n\n".repeat(400)))
        .output()
        .unwrap();
    // the rows of `readback:false` leaves are not confirmed: the run fails, as it should
    assert_eq!(out.status.code(), Some(1), "{}", report(&out));
    let rows = rows(&results);
    let run_actions: Vec<&Row> = rows
        .iter()
        .filter(|r| {
            (r.kind == "action" || (r.kind == "asleep" && r.how.starts_with("action")))
                && !r.result.starts_with("not run")
        })
        .collect();
    assert!(run_actions.is_empty(), "{run_actions:#?}");
    assert!(rows.iter().any(|r| r.kind == "asleep"
        && r.how.starts_with("action")
        && r.result == "not run (declined)"));
    let names: Vec<String> = acts(&d, 0).into_iter().map(|(a, _)| a).collect();
    assert!(
        names.iter().all(|a| a == "sleep" || a == "wake"),
        "an action reached the camera after a no: {names:?}"
    );
}

/// The presets and views of the owner are never touched: the script saves its own preset in a
/// slot the snapshot shows free and its own view under the name `check`, works on those only,
/// and deletes them again, so the lists are as before.
#[test]
fn the_owners_presets_and_views_are_left_alone() {
    let d = Daemon::start("own");
    let cli = |args: &[&str]| {
        let (code, _, err) = common::cli(Some(&d.socket), args);
        assert_eq!(code, 0, "{err}");
    };
    cli(&[
        "do",
        "preset.save",
        "slot=0",
        "name=mine",
        "--device",
        "fake-a",
    ]);
    cli(&["do", "view.save", "name=mine", "--device", "fake-a"]);
    let before = lists(&d.state(A));
    assert_eq!(before.0[0]["name"], "mine");
    assert_eq!(before.1[0]["name"], "mine");
    let from = d.bus.ops(A).len();
    let (out, rows) = d.check("own", &["--device", "fake-a", "--assume-pass", "--asleep"]);
    assert_eq!(out.status.code(), Some(0), "{}", report(&out));
    assert_eq!(
        lists(&d.state(A)),
        before,
        "the owner's lists are as before"
    );
    let on_items: Vec<_> = acts(&d, from)
        .into_iter()
        .filter(|(a, _)| {
            a.starts_with("preset.") || a.starts_with("view.") || a == "power_on.use_preset"
        })
        .collect();
    assert!(on_items.len() >= 8, "{on_items:#?}");
    for (action, params) in &on_items {
        let own = if action.starts_with("view.") {
            params.get("id")
        } else {
            params.get("slot")
        };
        if let Some(own) = own {
            assert_ne!(
                own,
                &Value::from(0),
                "{action} {params:?} works on the owner's item"
            );
        }
    }
    assert_eq!(row(&rows, "action", "preset.save", "slot=1").result, "pass");
}

/// A camera that is asleep at the start is woken first when the person agrees (as the 0.13
/// script did), and the script ends with a no instead of failing every row when they do not.
#[test]
fn a_sleeping_camera_is_woken_first_when_the_person_agrees() {
    let d = Daemon::start("w");
    let (code, _, err) = common::cli(Some(&d.socket), &["do", "sleep", "--device", "fake-a"]);
    assert_eq!(code, 0, "{err}");
    let until = Instant::now() + Duration::from_secs(10);
    while d.state(A)["asleep"] != Value::Bool(true) {
        assert!(Instant::now() < until, "the fake does not fall asleep");
        thread::sleep(Duration::from_millis(50));
    }
    let results = results_file("w-no");
    let out = d
        .command(
            &results,
            Path::new(env!("CARGO_BIN_EXE_obsbot4linux")),
            &["--device", "fake-a"],
        )
        .env("CHECK_MODEL_TTY", answers("w-no", "n\n"))
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(3), "{}", report(&out));
    assert_eq!(d.state(A)["asleep"], Value::Bool(true), "nothing was sent");
    let (out, rows) = d.check("w", &["--device", "fake-a", "--assume-pass"]);
    assert_eq!(out.status.code(), Some(0), "{}", report(&out));
    assert!(
        !rows.iter().any(|r| r.result.starts_with("fail")),
        "{rows:#?}"
    );
}

#[test]
fn without_device_it_is_a_usage_error() {
    let out = Command::new("bash")
        .arg(script())
        .env("OBSBOT4LINUX", env!("CARGO_BIN_EXE_obsbot4linux"))
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&out.stderr).contains("usage: check-model.sh --device"));
}
