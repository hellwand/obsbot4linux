// SPDX-License-Identifier: GPL-3.0-or-later

//! The process (spec 5.9, 5.10, 11): the single-instance lock, socket activation through
//! `LISTEN_FDS`, the exit codes of the binary and a clean stop.

mod common;

use common::*;
use obsbot4linuxd::{Config, run_with};
use std::os::fd::AsRawFd;
use std::os::unix::net::UnixListener;
use std::os::unix::process::CommandExt;
use std::process::{Command, Stdio};
use std::time::Duration;

const T: Duration = Duration::from_secs(5);

/// 5.9: a second daemon on the same socket path exits with a message naming the lock file.
#[test]
fn a_second_daemon_on_the_same_socket_is_refused_by_the_lock() {
    let d = Daemon::start();
    let mut second = Config::new(d.socket.clone());
    second.fake = Some(std::sync::Arc::new(
        obsbot_device::models::fake::FakeBus::new(&["fake-a"]).unwrap(),
    ));
    let err = run_with(second, fast()).expect_err("refused");
    assert!(err.to_string().contains("s.sock.lock"), "{err}");
    let (_c, ok) = d.hello(false, &[]);
    assert_eq!(ids(&ok["devices"]).len(), 3, "the first one goes on");
}

#[test]
fn a_clean_stop_returns_ok_and_removes_the_sockets() {
    let d = Daemon::start();
    let socket = d.socket.clone();
    d.stop().expect("Ok");
    assert!(!socket.exists());
    let mut test = socket.as_os_str().to_owned();
    test.push(".test");
    assert!(!std::path::Path::new(&test).exists());
}

/// 11 and 5.10: under systemd the daemon serves the socket it inherits (fd 3, `LISTEN_PID`
/// and `LISTEN_FDS`), takes the lock beside its path, and leaves the socket file to systemd.
#[test]
fn socket_activation_serves_the_inherited_socket() {
    let dir = temp_dir();
    let socket = dir.join("s.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    let fd = listener.as_raw_fd();
    let mut cmd = Command::new("sh");
    cmd.args([
        "-c",
        "export LISTEN_PID=$$ LISTEN_FDS=1; exec \"$0\" --fake fake-a",
        env!("CARGO_BIN_EXE_obsbot4linuxd"),
    ])
    .env("OBSBOT4LINUX_SOCKET", &socket)
    .stdin(Stdio::null());
    // SAFETY: dup2 is async-signal-safe; it gives the child the listener as fd 3, as systemd
    // does, and clears close-on-exec on the copy.
    unsafe {
        cmd.pre_exec(move || {
            if nix::libc::dup2(fd, 3) < 0 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    let mut child = cmd.spawn().unwrap();
    let mut c = Client::connect(&socket);
    let ok = c.hello(false, &[]);
    assert_eq!(ids(&ok["devices"]), [A]);
    assert!(
        dir.join("s.sock.lock").exists(),
        "the lock is taken for an inherited socket too"
    );
    nix::sys::signal::kill(
        nix::unistd::Pid::from_raw(child.id() as i32),
        nix::sys::signal::Signal::SIGTERM,
    )
    .unwrap();
    assert_eq!(child.wait().unwrap().code(), Some(0));
    assert!(socket.exists(), "an inherited socket file is systemd's");
    drop(listener);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn the_binary_refuses_bad_arguments_and_a_missing_socket_path() {
    let bin = env!("CARGO_BIN_EXE_obsbot4linuxd");
    let out = Command::new(bin)
        .args(["--fake", "tiny2lite"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&out.stderr).contains("unknown fake model"));
    let out = Command::new(bin)
        .args(["--fake", "fake-a"])
        .env_remove("OBSBOT4LINUX_SOCKET")
        .env_remove("XDG_RUNTIME_DIR")
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&out.stderr).contains("XDG_RUNTIME_DIR"));
    let out = Command::new(bin).arg("--version").output().unwrap();
    assert_eq!(out.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&out.stdout).starts_with("obsbot4linuxd "));
}

/// SIGINT ends the daemon like SIGTERM (5.9).
#[test]
fn sigint_ends_the_daemon_with_0() {
    let dir = temp_dir();
    let socket = dir.join("s.sock");
    let mut child = Command::new(env!("CARGO_BIN_EXE_obsbot4linuxd"))
        .args(["--fake", "unknown"])
        .env("OBSBOT4LINUX_SOCKET", &socket)
        .env_remove("LISTEN_PID")
        .env_remove("LISTEN_FDS")
        .spawn()
        .unwrap();
    assert!(eventually(T, || std::os::unix::net::UnixStream::connect(
        &socket
    )
    .is_ok()));
    nix::sys::signal::kill(
        nix::unistd::Pid::from_raw(child.id() as i32),
        nix::sys::signal::Signal::SIGINT,
    )
    .unwrap();
    assert_eq!(child.wait().unwrap().code(), Some(0));
    let _ = std::fs::remove_dir_all(dir);
}

/// Review focus 5: a stop of the daemon returns while a camera operation hangs: polls and
/// identity attempts are cut at their next frame boundary.
#[test]
fn a_stop_returns_while_a_camera_operation_hangs() {
    let d = Daemon::start();
    let (_c, _) = d.hello(false, &[]);
    let skip = d.ops(A).len();
    d.bus.set_delay("fake-a", Duration::from_secs(30));
    assert!(
        eventually(T, || d.ops(A).len() > skip),
        "an operation is in flight"
    );
    let t0 = std::time::Instant::now();
    d.stop().expect("Ok");
    assert!(t0.elapsed() < Duration::from_secs(3), "{:?}", t0.elapsed());
}
