// SPDX-License-Identifier: GPL-3.0-or-later

//! Stage W2 hook-up (design spec 10.3 W2): the command line, the daemon and the Tiny 2 Lite
//! model end to end, on a camera recorded on the owner's device. The model runs on the replay
//! transport of track A with the recorded answers of `obsbot-device`'s test support, behind a
//! bus that lists it once; nothing opens a video node or any other device file.
//!
//! What a request sends is compared with the frames recorded on the owner's device: a request
//! sends exactly what it names and nothing before or after it (spec 2).

mod common;
#[path = "../../obsbot-device/tests/support/mod.rs"]
mod support;

use obsbot_device::contract::camera::Result as CameraResult;
use obsbot_device::contract::{Bus, Camera, CameraError, Found, Frame};
use obsbot_device::models::tiny2lite;
use obsbot_device::transport::replay::{Log, Replay};
use obsbot4linuxd::{Config, Timing};
use serde_json::{Value, json};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

/// The id of the recorded camera: `sn:` and the synthetic serial of the test support.
const ID: &str = "sn:EXAMPLE0000001";

/// A bus with one Tiny 2 Lite on the replay transport. It lists the camera on every scan and
/// opens it once; the camera is the model behind the send-table check, as on the real bus. A
/// second open (a retry or a reopen after an error) is refused with an error and counted: the
/// recorded answers are consumed by the first, and `Hookup` fails the test with the count
/// instead of the daemon dying on a panic in a thread.
struct ReplayBus {
    replay: Mutex<Option<Replay>>,
    opens: Arc<AtomicUsize>,
}

impl Bus for ReplayBus {
    fn scan(&self) -> Vec<Found> {
        vec![Found {
            usb: support::usb(),
            devnum: 4,
            video_node: None,
            model: Some(&tiny2lite::plan::ENTRY),
        }]
    }

    fn open(&self, _found: &Found) -> CameraResult<Box<dyn Camera>> {
        self.opens.fetch_add(1, Ordering::SeqCst);
        let replay = self
            .replay
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take()
            .ok_or_else(|| {
                CameraError::Io("the recorded camera answers one open, not two".into())
            })?;
        let (camera, _) = support::model(replay);
        Ok(Box::new(camera))
    }
}

/// The daemon on the replay bus, and the log of what reached the recorded camera.
struct Hookup {
    socket: PathBuf,
    log: Log,
    opens: Arc<AtomicUsize>,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<std::io::Result<()>>>,
}

impl Hookup {
    /// Starts the daemon with the spec's intervals and waits (at most 20 s) until it accepts.
    fn start(name: &str) -> Hookup {
        let socket = common::socket(&format!("r-{name}"));
        let _ = std::fs::remove_file(&socket);
        let replay = support::recorded();
        let log = replay.log();
        let opens = Arc::new(AtomicUsize::new(0));
        let bus: Arc<dyn Bus> = Arc::new(ReplayBus {
            replay: Mutex::new(Some(replay)),
            opens: Arc::clone(&opens),
        });
        let config = Config::new(socket.clone());
        let stop = Arc::clone(&config.stop);
        let thread = thread::spawn(move || obsbot4linuxd::run_on(config, Timing::SPEC, bus));
        let until = Instant::now() + Duration::from_secs(20);
        while UnixStream::connect(&socket).is_err() {
            if thread.is_finished() {
                panic!("the daemon ended: {:?}", thread.join());
            }
            assert!(Instant::now() < until, "the daemon does not accept");
            thread::sleep(Duration::from_millis(20));
        }
        Hookup {
            socket,
            log,
            opens,
            stop,
            thread: Some(thread),
        }
    }

    fn cli(&self, args: &[&str]) -> (u8, String, String) {
        common::cli(Some(&self.socket), args)
    }

    /// What reached the camera since the last `clear`: commands, buffer writes and UVC
    /// writes in order; queries (reads) left out.
    fn writes(&self) -> Vec<Frame> {
        support::commands(&self.log)
    }

    fn payloads(&self) -> Vec<Vec<u8>> {
        support::payloads(&self.log)
    }
}

impl Drop for Hookup {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(thread) = self.thread.take() {
            let ended = thread.join();
            if !thread::panicking() {
                assert!(matches!(ended, Ok(Ok(()))), "run returned {ended:?}");
                let opens = self.opens.load(Ordering::SeqCst);
                assert_eq!(
                    opens, 1,
                    "the daemon opened the recorded camera {opens} times: a reopen needs a \
                     recording that answers it"
                );
            }
        }
        // the daemon keeps its `<socket>.lock` (5.9); the test's daemon is gone, so is it
        let mut lock = self.socket.clone().into_os_string();
        lock.push(".lock");
        let _ = std::fs::remove_file(lock);
    }
}

fn parse(out: &str) -> Value {
    serde_json::from_str(out.trim_end()).unwrap_or_else(|e| panic!("{e}: {out}"))
}

fn floats(values: &[f32]) -> Vec<u8> {
    values.iter().flat_map(|v| v.to_le_bytes()).collect()
}

/// The recorded camera is listed with the model's identity and its USB facts, and every
/// value the command line shows was decoded by the model from a recorded answer.
#[test]
fn the_recorded_camera_is_listed_and_read_through_the_daemon() {
    let h = Hookup::start("list");
    let (code, out, err) = h.cli(&["devices", "--json"]);
    assert_eq!(code, 0, "{err}");
    let devices = parse(&out)["devices"].clone();
    assert_eq!(devices.as_array().map(Vec::len), Some(1), "{devices}");
    let d = &devices[0];
    assert_eq!(d["id"], ID, "the serial query answered at plug-in (5.5)");
    assert_eq!(d["model"], "tiny2lite");
    assert_eq!(d["support"], "tested", "spec 8.8: tested from W3 on");
    assert_eq!(d["firmware"], "6.2.7.4", "capture 01");
    assert_eq!(d["usb"]["vendor"], "3564");
    assert_eq!(d["usb"]["product"], "fef9");
    assert_eq!(d["usb"]["serial_present"], false);

    let (code, out, err) = h.cli(&[
        "get",
        "--json",
        "zoom.factor",
        "audio.volume",
        "image.mirror",
        "tracking.mode",
        "asleep",
    ]);
    assert_eq!(code, 0, "{err}");
    let state = &parse(&out)["state"];
    assert_eq!(state["zoom"]["factor"], json!(1.6), "capture 04");
    assert_eq!(state["audio"]["volume"], json!(55), "measured 2026-10-31");
    assert_eq!(state["image"]["mirror"], json!(false), "the status block");
    assert_eq!(
        state["tracking"]["mode"],
        json!("static"),
        "the status block"
    );
    assert_eq!(state["asleep"], json!(false), "the status block");
}

/// Reads send no command. The one write of the first read is the rewind of the preset list
/// (spec 8.8: the one exception of section 2, until CAM-19); a `status` after it writes
/// nothing at all.
#[test]
fn reading_writes_only_the_preset_list_rewind() {
    let h = Hookup::start("reads");
    let (code, _, err) = h.cli(&["status"]);
    assert_eq!(code, 0, "{err}");
    assert_eq!(h.writes(), [Frame::Block { selector: 0x0c }]);
    h.log.clear();
    let (code, out, err) = h.cli(&["status"]);
    assert_eq!(code, 0, "{err}");
    assert!(out.contains("tiny2lite"), "{out}");
    assert!(
        !out.contains("untested"),
        "human output names no level for a tested model (8.6): {out}"
    );
    assert!(out.contains("…0001"), "the shortened id: {out}");
    assert_eq!(h.writes(), [], "a status reads and writes nothing");
}

/// `set image.mirror=on` reaches the camera as the short command recorded in capture 05,
/// `14 01 01`, and as nothing else.
#[test]
fn a_set_reaches_the_camera_as_its_recorded_frame_only() {
    let h = Hookup::start("set");
    let (code, _, err) = h.cli(&["get", "image.mirror"]);
    assert_eq!(code, 0, "{err}");
    h.log.clear();
    let (code, out, err) = h.cli(&["set", "image.mirror=on"]);
    assert_eq!((code, out.as_str()), (0, ""), "{err}");
    assert_eq!(h.writes(), [support::short(0x14)]);
    assert_eq!(h.payloads(), [vec![0x14, 0x01, 0x01]]);
}

/// `do gimbal.move` sends the one move frame and no tracking switch before it (spec 2, 15.1
/// row 88): the pan and tilt in the camera's angles, the roll kept.
#[test]
fn a_gimbal_move_reaches_the_camera_as_one_move_and_no_tracking_frame() {
    let h = Hookup::start("move");
    let (code, _, err) = h.cli(&["get", "tracking.mode"]);
    assert_eq!(code, 0, "{err}");
    h.log.clear();
    let (code, _, err) = h.cli(&["do", "gimbal.move", "pan=100", "tilt=-30"]);
    assert_eq!(code, 0, "{err}");
    assert_eq!(h.writes(), [support::framed(0x6444)]);
    assert_eq!(h.payloads(), [floats(&[-1000.0, 30.0, 100.0])]);
    let (_, out, _) = h.cli(&["get", "--json", "tracking.mode"]);
    assert_eq!(parse(&out)["state"]["tracking"]["mode"], json!("static"));
}

/// A value the description refuses is answered by the daemon's validator and reaches the
/// camera as nothing.
#[test]
fn a_refused_value_reaches_the_camera_as_nothing() {
    let h = Hookup::start("refused");
    let (code, _, err) = h.cli(&["get", "led.brightness"]);
    assert_eq!(code, 0, "{err}");
    h.log.clear();
    let (code, _, err) = h.cli(&["set", "led.brightness=9"]);
    assert_eq!(code, 2, "out_of_range: {err}");
    assert_eq!(h.writes(), []);
}
