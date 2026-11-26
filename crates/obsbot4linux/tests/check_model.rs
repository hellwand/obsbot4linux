// SPDX-License-Identifier: GPL-3.0-or-later

//! `scripts/check-model.sh` (design spec 9.4) against `fake-a` and `fake-b` with
//! `--assume-pass`, as stage W2's acceptance asks ("the check script passes against fake-a and
//! fake-b in CI"). The daemon runs in-process in fake mode with short poll tiers, so that a
//! lagging readback shows within the test's time; the script runs as it would on a camera,
//! with the command line of this crate. The person's answers of a run without `--assume-pass`
//! come from a file (`CHECK_MODEL_TTY`). Needs bash and jq; the frame comparison needs python3
//! and numpy.

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
        Daemon::start_with(name, FAST)
    }

    fn start_with(name: &str, timing: Timing) -> Daemon {
        let socket = common::socket(&format!("m-{name}"));
        let _ = std::fs::remove_file(&socket);
        let bus = Arc::new(FakeBus::new(&["fake-a", "fake-b", "unknown"]).expect("the fakes"));
        let mut config = Config::new(socket.clone());
        config.fake = Some(Arc::clone(&bus));
        let stop = Arc::clone(&config.stop);
        let thread = thread::spawn(move || obsbot4linuxd::run_with(config, timing));
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
    // asleep rows only for `asleep: unknown`; zoom.field_of_view is `ignored` and audio.volume
    // `accepted` (measured)
    assert!(
        rows.iter()
            .any(|r| r.kind == "asleep" && r.path == "exposure.anti_flicker")
    );
    assert!(
        !rows
            .iter()
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
            .any(|r| r.kind == "asleep" && r.path == "focus.position" && r.result == "accepted"),
        "focus.position asleep, after its setup: {rows:#?}"
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

/// A start mode that the camera's own app stored and that the model reads but does not write
/// (`close_up`, U-18) cannot be put back after a row: the leaf gets no row and keeps the owner's
/// value, instead of ending at the last value the script wrote (the restore would be refused).
#[test]
fn a_value_that_cannot_be_written_back_is_left_alone() {
    let d = Daemon::start("nr");
    d.bus
        .store(A, "power_on.start.mode", serde_json::json!("close_up"))
        .unwrap();
    let until = Instant::now() + Duration::from_secs(10);
    while d.state(A)["power_on"]["start"]["mode"] != "close_up" {
        assert!(
            Instant::now() < until,
            "the daemon does not show the stored mode"
        );
        thread::sleep(Duration::from_millis(50));
    }
    let from = d.bus.ops(A).len();
    let (out, rows) = d.check("nr", &["--device", "fake-a", "--assume-pass"]);
    assert_eq!(out.status.code(), Some(0), "{}", report(&out));
    assert_eq!(
        row(&rows, "setting", "power_on.start.mode", "\"close_up\"").result,
        "not run: the value it reads cannot be written back"
    );
    let written: Vec<Op> = d.bus.ops(A)[from..]
        .iter()
        .filter(|op| matches!(op, Op::Apply(leaf, _) if leaf == "power_on.start.mode"))
        .cloned()
        .collect();
    assert!(written.is_empty(), "{written:?}");
    assert_eq!(d.state(A)["power_on"]["start"]["mode"], "close_up");
}

/// Under automatic exposure the Tiny 2 Lite read a gain of 71, above the 64 a write may set
/// (measured 2026-11-26, #357): that value is the automatic exposure's, not the owner's, and a
/// write of it is refused. The gain rows still run (in manual exposure, their `requires`); the
/// value is named, not written back, and the run does not fail on it.
#[test]
fn an_automatic_value_outside_the_bounds_is_not_written_back() {
    let d = Daemon::start("auto");
    assert_eq!(d.state(A)["exposure"]["mode"], "auto");
    d.bus
        .store(A, "exposure.gain", serde_json::json!(71))
        .unwrap();
    let until = Instant::now() + Duration::from_secs(10);
    while d.state(A)["exposure"]["gain"] != 71 {
        assert!(
            Instant::now() < until,
            "the daemon does not show the stored gain"
        );
        thread::sleep(Duration::from_millis(50));
    }
    let from = d.bus.ops(A).len();
    let (out, rows) = d.check("auto", &["--device", "fake-a", "--assume-pass"]);
    assert_eq!(out.status.code(), Some(0), "{}", report(&out));
    for value in ["1", "64"] {
        assert_eq!(row(&rows, "setting", "exposure.gain", value).result, "pass");
    }
    assert!(
        !rows.iter().any(|r| r.kind == "restore"),
        "no restore warning: {rows:#?}"
    );
    assert!(
        String::from_utf8_lossy(&out.stdout).contains("not written back: exposure.gain 71"),
        "{}",
        report(&out)
    );
    let gains: Vec<Op> = d.bus.ops(A)[from..]
        .iter()
        .filter(|op| matches!(op, Op::Apply(leaf, _) if leaf == "exposure.gain"))
        .cloned()
        .collect();
    assert!(
        !gains
            .iter()
            .any(|op| matches!(op, Op::Apply(_, v) if *v == serde_json::json!(71))),
        "{gains:?}"
    );
    assert_eq!(
        d.state(A)["exposure"]["mode"],
        "auto",
        "the mode is put back"
    );
}

/// The zoom of the pose is put back after the action rows although the state does not show
/// that a move changed it: `zoom.move` is not followed by a read, the zoom group shows the new
/// factor only at its next 5 s tick. Here that tick does not come during the run, so the state
/// shows the snapshot's zoom to the end; a restore that trusted it would leave the camera
/// zoomed in, and the run would still exit 0. With the spec's tiers the same stale state made
/// the outcome depend on where the tick fell: a run left the camera zoomed in and passed, or
/// named `zoom.factor` as not restored although every row passed (#347).
#[test]
fn the_zoom_a_move_changed_is_put_back_although_the_state_does_not_show_it_yet() {
    const NO_FIVE_SECOND_TICK: Timing = Timing {
        five_seconds: Duration::from_secs(3600),
        minute: Duration::from_secs(3600),
        ..FAST
    };
    let d = Daemon::start_with("zoom", NO_FIVE_SECOND_TICK);
    let snapshot = d.state(A)["zoom"]["factor"]
        .as_f64()
        .expect("a zoom factor");
    let from = d.bus.ops(A).len();
    let (out, _) = d.check("zoom", &["--device", "fake-a", "--assume-pass"]);
    assert_eq!(out.status.code(), Some(0), "{}", report(&out));
    let ops = &d.bus.ops(A)[from..];
    let zoom = |op: &Op| match op {
        Op::Act(a, p) if a == "zoom.move" => p.get("factor").and_then(Value::as_f64),
        Op::Apply(leaf, v) if leaf == "zoom.factor" => v.as_f64(),
        _ => None,
    };
    let moved = ops
        .iter()
        .rposition(|op| matches!(op, Op::Act(a, _) if a == "zoom.move"))
        .expect("the zoom.move row ran");
    assert_ne!(
        zoom(&ops[moved]),
        Some(snapshot),
        "the move changes the zoom"
    );
    assert_eq!(
        ops[moved..].iter().filter_map(zoom).next_back(),
        Some(snapshot),
        "the camera's last zoom after the move is the snapshot's: {:?}",
        ops[moved..]
            .iter()
            .filter(|op| !matches!(op, Op::Read(_)))
            .collect::<Vec<_>>()
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

/// `--observe state`: starting the script is the go for every action and asleep row, so no
/// row is declined although every answer is no. A row whose effect the state shows (a preset
/// saved: the list changed) passes with the measurement in its result. A move is never passed
/// on the state alone (the pose leaves are the daemon's own report, the picture is the
/// evidence): with no picture it is asked, and the answer no fails it. Nothing fails without
/// a person's answer: every failed row is one the person declined to confirm.
#[test]
fn observe_answers_from_the_state_and_asks_the_rest() {
    let d = Daemon::start("obs");
    let results = results_file("obs");
    let out = d
        .command(
            &results,
            Path::new(env!("CARGO_BIN_EXE_obsbot4linux")),
            &["--device", "fake-a", "--asleep", "--observe", "state"],
        )
        .env("CHECK_MODEL_TTY", answers("obs", &"n\n".repeat(400)))
        .env("CHECK_MODEL_SETTLE", "3")
        .env("CHECK_MODEL_QUIET", "1")
        .env("CHECK_MODEL_BASELINE", "0.1")
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1), "{}", report(&out));
    let rows = rows(&results);
    let declined: Vec<&Row> = rows
        .iter()
        .filter(|r| r.result.contains("declined"))
        .collect();
    assert!(declined.is_empty(), "{declined:#?}");
    let save = rows
        .iter()
        .find(|r| r.kind == "action" && r.path == "preset.save")
        .unwrap();
    assert!(
        save.result.starts_with("pass (observed: state: presets"),
        "{save:#?}"
    );
    let moved = rows
        .iter()
        .find(|r| r.kind == "action" && r.path == "gimbal.move")
        .unwrap();
    assert_eq!(moved.result, "fail (not confirmed)", "{}", report(&out));
    assert!(String::from_utf8_lossy(&out.stdout).contains("observed: pose "));
    let unconfirmed_fail = rows
        .iter()
        .filter(|r| r.result.starts_with("fail") && r.result != "fail (not confirmed)")
        .collect::<Vec<_>>();
    assert!(unconfirmed_fail.is_empty(), "{unconfirmed_fail:#?}");
    // sleep and wake are seen in the state
    for action in ["sleep", "wake"] {
        let r = rows
            .iter()
            .find(|r| r.kind == "action" && r.path == action)
            .unwrap();
        assert!(
            r.result.starts_with("pass (observed: state: asleep"),
            "{r:#?}"
        );
    }
}

/// A video node that is not one stops the run before it touches the camera (exit 2), rather
/// than running without the picture it was asked to use.
#[test]
fn observe_with_a_missing_video_node_is_a_usage_error() {
    let d = Daemon::start("obs-bad");
    let (out, rows) = d.check(
        "obs-bad",
        &["--device", "fake-a", "--observe", "/nonexistent/video9"],
    );
    assert_eq!(out.status.code(), Some(2), "{}", report(&out));
    assert!(String::from_utf8_lossy(&out.stderr).contains("is not a video device"));
    assert!(rows.is_empty(), "{rows:#?}");
    assert!(acts(&d, 0).is_empty());
}

/// A character device that is not the device's own video node (another camera, here
/// `/dev/null`) is refused before any row: its picture would show nothing of this camera's
/// moves, and a person moving in it would read as a move.
#[test]
fn observe_with_another_devices_node_is_a_usage_error() {
    let d = Daemon::start("obs-other");
    let (out, rows) = d.check(
        "obs-other",
        &[
            "--device",
            "fake-a",
            "--assume-pass",
            "--observe",
            "/dev/null",
        ],
    );
    assert_eq!(out.status.code(), Some(2), "{}", report(&out));
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("is not the video node of"),
        "{}",
        report(&out)
    );
    assert!(rows.is_empty(), "{rows:#?}");
    assert!(acts(&d, 0).is_empty());
}

/// A smooth random texture (value noise on a 6 px lattice), sampled anywhere, so that a
/// shifted or magnified copy is the same scene seen after a move or a zoom.
fn texture(x: f64, y: f64) -> f64 {
    fn lattice(i: i64, j: i64) -> f64 {
        let mut h = (i.wrapping_mul(73_856_093) ^ j.wrapping_mul(19_349_663)) as u64;
        h ^= h >> 13;
        h = h.wrapping_mul(0x5bd1_e995);
        h ^= h >> 15;
        (h % 1000) as f64 / 1000.0
    }
    let (gx, gy) = (x / 6.0, y / 6.0);
    let (i, j) = (gx.floor(), gy.floor());
    let (fx, fy) = (gx - i, gy - j);
    let (sx, sy) = (fx * fx * (3.0 - 2.0 * fx), fy * fy * (3.0 - 2.0 * fy));
    let (i, j) = (i as i64, j as i64);
    let top = lattice(i, j) * (1.0 - sx) + lattice(i + 1, j) * sx;
    let bottom = lattice(i, j + 1) * (1.0 - sx) + lattice(i + 1, j + 1) * sx;
    40.0 + 160.0 * (top * (1.0 - sy) + bottom * sy)
}

/// A 320x180 binary PGM, as ffmpeg writes the frames of `--observe`.
fn pgm(dir: &Path, name: &str, pixel: impl Fn(f64, f64) -> f64) -> PathBuf {
    let (w, h) = (320, 180);
    let mut data = format!("P5\n{w} {h}\n255\n").into_bytes();
    for y in 0..h {
        for x in 0..w {
            data.push(pixel(f64::from(x), f64::from(y)).round().clamp(0.0, 255.0) as u8);
        }
    }
    let path = dir.join(name);
    std::fs::write(&path, data).unwrap();
    path
}

/// The comparison `--observe` passes a move on. A move must show as a shift or a scale, and
/// sensor noise on an unchanged view must not (a still picture is what a stop passes on and
/// what keeps a noisy scene from passing a move); a black frame, as a sleeping camera's or a
/// dark room's, is never compared at all.
#[test]
fn the_frame_comparison_tells_a_move_from_noise_and_darkness() {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("observe-frames");
    std::fs::create_dir_all(&dir).unwrap();
    let helper =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../scripts/check-model-observe.py");
    let compare = |a: &Path, b: &Path| {
        let out = Command::new("python3")
            .arg("-I")
            .arg(&helper)
            .args([a, b])
            .output()
            .expect("python3 runs (with numpy)");
        assert_eq!(out.status.code(), Some(0), "{}", report(&out));
        String::from_utf8(out.stdout).unwrap()
    };
    let before = pgm(&dir, "before.pgm", texture);
    // the same view with sensor noise (a fixed pseudo-random +-3)
    let noisy = pgm(&dir, "noisy.pgm", |x, y| {
        texture(x, y) + ((x * 12.9898 + y * 78.233).sin() * 43_758.545).fract() * 6.0 - 3.0
    });
    let panned = pgm(&dir, "panned.pgm", |x, y| texture(x + 12.0, y));
    let zoomed = pgm(&dir, "zoomed.pgm", |x, y| {
        texture(159.5 + (x - 159.5) / 1.25, 89.5 + (y - 89.5) / 1.25)
    });
    let black = pgm(&dir, "black.pgm", |_, _| 1.0);

    let still = compare(&before, &noisy);
    assert!(still.starts_with("still "), "{still}");
    let pan = compare(&before, &panned);
    assert!(pan.starts_with("moved "), "{pan}");
    assert!(pan.contains("shift=-12,+0"), "{pan}");
    let zoom = compare(&before, &zoomed);
    assert!(zoom.starts_with("moved "), "{zoom}");
    let scale: f64 = zoom
        .split_whitespace()
        .find_map(|f| f.strip_prefix("scale="))
        .unwrap()
        .parse()
        .unwrap();
    assert!((scale - 1.25).abs() < 0.03, "{zoom}");
    for pair in [(&before, &black), (&black, &before)] {
        let dark = compare(pair.0, pair.1);
        assert!(dark.starts_with("dark "), "{dark}");
    }
}

/// A restart row passes only once the device went away and came back: the camera answers a
/// `restart` write or action before it restarts (measured 2026-11-26: the Tiny 2 Lite went
/// away 2.9 s after the reply to `factory_reset`). A row that took the device that was still
/// listed for the returned one let the next restart row and the final restore reach a camera
/// that was about to go away, and the restore was refused (`unavailable`), exit 1. Here the
/// command line delays every restart write and action by 3 s after its reply; no other write
/// or action may be sent between a reply and the moment its command reached the fake (the
/// order the command line logs, not a timing), and the run must still exit 0 with every
/// restart row `returned` and every leaf it wrote put back.
#[test]
fn a_restart_row_waits_for_the_device_to_go_away_and_return() {
    let d = Daemon::start("late");
    // a value the factory reset changes, so that a reset that lands after the restore shows
    let (code, _, err) = common::cli(
        Some(&d.socket),
        &["set", "audio.volume=37", "--device", "fake-a"],
    );
    assert_eq!(code, 0, "{err}");
    let until = Instant::now() + Duration::from_secs(10);
    while d.state(A)["audio"]["volume"].as_f64() != Some(37.0) {
        assert!(
            Instant::now() < until,
            "the fake does not show audio.volume=37"
        );
        thread::sleep(Duration::from_millis(50));
    }
    let before = writable_values(&d.state(A), "fake-a");
    let wrapper = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/late-restart-cli.sh");
    let results = results_file("late");
    let log = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("check-model-late.log");
    let _ = std::fs::remove_file(&log);
    let out = d
        .command(
            &results,
            &wrapper,
            &["--device", "fake-a", "--assume-pass", "--destructive"],
        )
        .env("LATE_RESTART_REAL_CLI", env!("CARGO_BIN_EXE_obsbot4linux"))
        .env("LATE_RESTART_LOG", &log)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0), "{}", report(&out));
    // the order the command line saw: after the reply to a restart command, nothing writes to
    // the camera until that command reached it (the next row and the restore come later)
    let log = std::fs::read_to_string(&log).unwrap_or_default();
    let mut pending: Option<&str> = None;
    let mut landed = 0;
    for line in log.lines() {
        if let Some(cmd) = line.strip_prefix("reply ") {
            assert!(
                pending.is_none(),
                "{cmd} sent before {pending:?} landed:\n{log}"
            );
            pending = Some(cmd);
        } else if let Some(cmd) = line.strip_prefix("landed ") {
            assert_eq!(pending, Some(cmd), "{log}");
            pending = None;
            landed += 1;
        } else if let Some(cmd) = line.strip_prefix("write ") {
            assert!(
                pending.is_none(),
                "{cmd} sent before {pending:?} landed:\n{log}"
            );
        }
    }
    assert_eq!((pending, landed), (None, 3), "{log}");
    assert!(
        !String::from_utf8_lossy(&out.stdout).contains("restore refused"),
        "{}",
        report(&out)
    );
    let rows = rows(&results);
    for (path, value) in [
        ("audio.enabled", "false"),
        ("audio.enabled", "true"),
        ("factory_reset", "confirm=true"),
    ] {
        let kind = if path == "factory_reset" {
            "action"
        } else {
            "setting"
        };
        let r = row(&rows, kind, path, value);
        assert_eq!(
            (r.result.as_str(), r.seen.as_str()),
            ("pass", "returned"),
            "{r:#?}"
        );
    }
    // the delayed commands reached the fake: a row cannot have passed before its restart
    let restarts = d
        .bus
        .ops(A)
        .iter()
        .filter(|op| {
            matches!(op, Op::Apply(p, _) if p == "audio.enabled")
                || matches!(op, Op::Act(a, _) if a == "factory_reset")
        })
        .count();
    assert_eq!(restarts, 3);
    let after = writable_values(&d.state(A), "fake-a");
    let changed: Vec<_> = before
        .iter()
        .zip(&after)
        .filter(|(b, a)| !same_json(&b.1, &a.1))
        .collect();
    assert!(changed.is_empty(), "not restored: {changed:#?}");
}

/// The script with `--observe <node>` against a stand-in stream: the command line names
/// `/dev/null` as the fake's video node, and `ffmpeg` and `v4l2-ctl` on PATH are the
/// fixtures in `tests/fixtures/observe-bin` (an ffmpeg that, as measured on the camera, gets
/// no frame when it opens while the camera is asleep and does not end on SIGTERM then).
fn observe_command(d: &Daemon, name: &str, pidfile: &Path) -> (Command, PathBuf) {
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let results = results_file(name);
    let _ = std::fs::remove_file(pidfile);
    let mut c = d.command(
        &results,
        &fixtures.join("node-cli.sh"),
        &[
            "--device",
            "fake-a",
            "--assume-pass",
            "--observe",
            "/dev/null",
        ],
    );
    let path = std::env::var("PATH").unwrap_or_default();
    c.env(
        "PATH",
        format!("{}:{path}", fixtures.join("observe-bin").display()),
    )
    .env("NODE_CLI_REAL_CLI", env!("CARGO_BIN_EXE_obsbot4linux"))
    .env("NODE_CLI_VIDEO", "/dev/null")
    .env("FAKE_FFMPEG_CLI", env!("CARGO_BIN_EXE_obsbot4linux"))
    .env("FAKE_FFMPEG_PIDFILE", pidfile)
    .env("CHECK_MODEL_SETTLE", "3")
    .env("CHECK_MODEL_QUIET", "1")
    .env("CHECK_MODEL_BASELINE", "0.1");
    (c, results)
}

/// Whether the process in `pidfile` is still running (it may need a moment to end).
fn still_running(pidfile: &Path) -> bool {
    let Ok(text) = std::fs::read_to_string(pidfile) else {
        return false;
    };
    let pid = nix::unistd::Pid::from_raw(text.trim().parse().unwrap());
    let until = Instant::now() + Duration::from_secs(2);
    while nix::sys::signal::kill(pid, None).is_ok() {
        if Instant::now() > until {
            nix::sys::signal::kill(pid, nix::sys::signal::Signal::SIGKILL).ok();
            return true;
        }
        thread::sleep(Duration::from_millis(50));
    }
    false
}

/// `--observe <node>` with a sleeping camera wakes it before the stream opens: opened while
/// the camera slept, the stream got no frame within 15 s and the run ended with exit 2
/// (measured 2026-11-26). A run ended by SIGTERM leaves no ffmpeg behind, although this one
/// ignores SIGTERM as the measured one did.
#[test]
fn observe_wakes_a_sleeping_camera_before_the_stream_and_ends_it() {
    let d = Daemon::start("obs-asleep");
    let (code, _, err) = common::cli(Some(&d.socket), &["do", "sleep", "--device", "fake-a"]);
    assert_eq!(code, 0, "{err}");
    let until = Instant::now() + Duration::from_secs(10);
    while d.state(A)["asleep"] != Value::Bool(true) {
        assert!(Instant::now() < until, "the fake does not fall asleep");
        thread::sleep(Duration::from_millis(50));
    }
    let pidfile = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("obs-asleep.ffmpeg.pid");
    let (mut c, results) = observe_command(&d, "obs-asleep", &pidfile);
    // stderr to a file, not a pipe: a stream left running would hold a pipe open
    let errfile = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("obs-asleep.stderr");
    let mut child = c
        .stdout(Stdio::null())
        .stderr(std::fs::File::create(&errfile).unwrap())
        .spawn()
        .unwrap();
    let until = Instant::now() + Duration::from_secs(60);
    while rows(&results).len() < 3 {
        if let Some(status) = child.try_wait().unwrap() {
            let running = still_running(&pidfile);
            panic!(
                "the script ended early ({status}; ffmpeg left running: {running}): {}",
                std::fs::read_to_string(&errfile).unwrap_or_default()
            );
        }
        assert!(Instant::now() < until, "the script wrote no rows");
        thread::sleep(Duration::from_millis(20));
    }
    nix::sys::signal::kill(
        nix::unistd::Pid::from_raw(child.id() as i32),
        nix::sys::signal::Signal::SIGTERM,
    )
    .unwrap();
    let until = Instant::now() + Duration::from_secs(60);
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        assert!(Instant::now() < until, "the script does not end on SIGTERM");
        thread::sleep(Duration::from_millis(50));
    };
    assert_eq!(status.code(), Some(143));
    assert!(!still_running(&pidfile), "ffmpeg left running");
}

/// A stream that never delivers a frame ends the run with exit 2 and is stopped (SIGINT,
/// bounded): the SIGTERM the script sent before left the measured ffmpeg running.
#[test]
fn observe_without_a_frame_exits_2_and_leaves_no_stream() {
    let d = Daemon::start("obs-noframe");
    let pidfile = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("obs-noframe.ffmpeg.pid");
    let (mut c, results) = observe_command(&d, "obs-noframe", &pidfile);
    // stderr to a file, not a pipe: a stream left running would hold a pipe open
    let errfile = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("obs-noframe.stderr");
    let mut child = c
        .env("FAKE_FFMPEG_NO_FRAMES", "1")
        .stdout(Stdio::null())
        .stderr(std::fs::File::create(&errfile).unwrap())
        .spawn()
        .unwrap();
    let until = Instant::now() + Duration::from_secs(60);
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        assert!(Instant::now() < until, "the script does not end");
        thread::sleep(Duration::from_millis(50));
    };
    let err = std::fs::read_to_string(&errfile).unwrap();
    assert_eq!(status.code(), Some(2), "{err}");
    assert!(err.contains("no frame from /dev/null within 15 s"), "{err}");
    assert!(rows(&results).is_empty());
    assert!(!still_running(&pidfile), "ffmpeg left running");
}

/// `--observe`: a pose change in the row of an action that does not move the camera is not
/// its effect. Measured 2026-11-26: in the `preset.save` row `zoom.factor` read 1.47 -> 1
/// while the picture stayed still, so the row was asked instead of passed on its preset
/// list. Here the fake's zoom changes the moment `preset.save` reaches it; the row passes on
/// the list and names the pose change.
#[test]
fn observe_does_not_take_a_pose_change_for_the_effect_of_a_save() {
    let d = Daemon::start("obs-drift");
    let bus = Arc::clone(&d.bus);
    let from = bus.ops(A).len();
    let done = Arc::new(AtomicBool::new(false));
    let stop = Arc::clone(&done);
    let drift = thread::spawn(move || {
        let mut seen = from;
        while !stop.load(Ordering::SeqCst) {
            let ops = bus.ops(A);
            if ops[seen..]
                .iter()
                .any(|op| matches!(op, Op::Act(a, _) if a == "preset.save"))
            {
                bus.store(A, "zoom.factor", serde_json::json!(1.5)).unwrap();
                return true;
            }
            seen = ops.len();
            thread::sleep(Duration::from_millis(5));
        }
        false
    });
    let results = results_file("obs-drift");
    let out = d
        .command(
            &results,
            Path::new(env!("CARGO_BIN_EXE_obsbot4linux")),
            &["--device", "fake-a", "--observe", "state"],
        )
        .env("CHECK_MODEL_TTY", answers("obs-drift", &"n\n".repeat(400)))
        // quiet for 2 s: the zoom the fake changes shows at the next 0.2 s read, also on a
        // loaded machine
        .env("CHECK_MODEL_SETTLE", "4")
        .env("CHECK_MODEL_QUIET", "2")
        .env("CHECK_MODEL_BASELINE", "0.1")
        .output()
        .unwrap();
    done.store(true, Ordering::SeqCst);
    assert!(drift.join().unwrap(), "preset.save did not run");
    let rows = rows(&results);
    let save = row(&rows, "action", "preset.save", "slot=0");
    assert!(
        save.result.starts_with("pass (observed: state: presets")
            && save.result.contains("not a move"),
        "{save:#?}\n{}",
        report(&out)
    );
}
