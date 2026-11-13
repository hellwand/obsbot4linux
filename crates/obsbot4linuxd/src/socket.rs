// SPDX-License-Identifier: GPL-3.0-or-later

//! The single-instance lock and the listening socket (5.9, 5.10, 11). Every daemon,
//! socket-activated or not, takes an exclusive `flock` on `<socket path>.lock` first. Then it
//! inherits the socket from systemd (`LISTEN_FDS`), or binds it: after checking that nothing
//! accepts connections on the path, it removes a stale socket file and binds under umask 077.
//!
//! Carried from the 0.13 daemon (`crates/obsbot4linux/src/daemon.rs`: `inherited_listener`,
//! `bind_at` and their tests). Changed: the lock is taken for an inherited socket too (5.9),
//! and the checks of `LISTEN_PID`/`LISTEN_FDS` are a pure function.

use nix::errno::Errno;
use nix::fcntl::{Flock, FlockArg};
use nix::sys::stat::{Mode, umask};
use std::fs::{File, OpenOptions};
use std::io;
use std::os::fd::FromRawFd;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};

/// The first file descriptor systemd passes (`SD_LISTEN_FDS_START`).
const LISTEN_FD: i32 = 3;

/// The socket the daemon serves and the lock it holds while it runs.
pub struct Bound {
    pub listener: UnixListener,
    /// The socket file this daemon bound, removed at exit; `None` for an inherited socket.
    pub owned: Option<PathBuf>,
    _lock: Flock<File>,
}

/// `<socket path>.lock`.
pub fn lock_path(socket: &Path) -> PathBuf {
    let mut p = socket.as_os_str().to_owned();
    p.push(".lock");
    PathBuf::from(p)
}

/// Takes the single-instance lock; an error names the lock file (5.9).
pub fn take_lock(socket: &Path) -> io::Result<Flock<File>> {
    let path = lock_path(socket);
    let file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .open(&path)?;
    match Flock::lock(file, FlockArg::LockExclusiveNonblock) {
        Ok(lock) => Ok(lock),
        Err((_, Errno::EWOULDBLOCK)) => Err(io::Error::new(
            io::ErrorKind::AddrInUse,
            format!("another daemon holds {}", path.display()),
        )),
        Err((_, e)) => Err(io::Error::from(e)),
    }
}

/// Whether systemd passed this process a socket: `LISTEN_PID` is our pid and `LISTEN_FDS` is
/// at least 1 (sd_listen_fds(3)).
pub fn activated(listen_pid: Option<&str>, listen_fds: Option<&str>, pid: u32) -> bool {
    let pid_ok = listen_pid.and_then(|p| p.parse::<u32>().ok()) == Some(pid);
    let fds_ok = listen_fds
        .and_then(|n| n.parse::<i32>().ok())
        .is_some_and(|n| n >= 1);
    pid_ok && fds_ok
}

/// Binds `path` (the lock is already held): refuses when a daemon accepts connections there,
/// replaces a stale socket file, binds under umask 077 and sets mode 0600.
pub fn bind_at(path: &Path) -> io::Result<UnixListener> {
    if UnixStream::connect(path).is_ok() {
        return Err(io::Error::new(
            io::ErrorKind::AddrInUse,
            format!("a daemon is already listening on {}", path.display()),
        ));
    }
    let _ = std::fs::remove_file(path);
    let old = umask(Mode::from_bits_truncate(0o077));
    let listener = UnixListener::bind(path);
    umask(old);
    let listener = listener?;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    Ok(listener)
}

/// Lock first, then the inherited socket or a bound one (5.9, 11).
pub fn open(socket: &Path) -> io::Result<Bound> {
    let lock = take_lock(socket)?;
    let env = |k: &str| std::env::var(k).ok();
    if activated(
        env("LISTEN_PID").as_deref(),
        env("LISTEN_FDS").as_deref(),
        std::process::id(),
    ) {
        // SAFETY: systemd passed fd 3 to this process for exactly this use; nothing else in
        // the process owns it, and `activated` is true at most for the process systemd started.
        let listener = unsafe { UnixListener::from_raw_fd(LISTEN_FD) };
        return Ok(Bound {
            listener,
            owned: None,
            _lock: lock,
        });
    }
    let listener = bind_at(socket)?;
    Ok(Bound {
        listener,
        owned: Some(socket.to_path_buf()),
        _lock: lock,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn socket_in(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("o4ld-sock-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let socket = dir.join("obsbot4linux.sock");
        // The daemon adds `.lock` and `.test`; a Unix socket path holds at most 107 bytes.
        assert!(
            socket.as_os_str().len() <= 100,
            "socket path too long for a Unix socket: {}",
            socket.display()
        );
        socket
    }

    /// Carried from 0.13 (`a_second_daemon_does_not_take_over_the_socket`), with the lock
    /// taken by `open`.
    #[test]
    fn a_second_daemon_does_not_take_over_the_socket() {
        let path = socket_in("second");
        let starting = take_lock(&path).unwrap();
        let err = open(&path).err().unwrap().to_string();
        assert!(err.contains("another daemon holds"), "{err}");
        assert!(
            err.contains("obsbot4linux.sock.lock"),
            "names the lock file: {err}"
        );
        assert!(!path.exists(), "the loser creates no socket");
        drop(starting);
        let bound = open(&path).unwrap();
        let mode = std::fs::metadata(&path).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
        assert_eq!(bound.owned.as_deref(), Some(path.as_path()));
        UnixStream::connect(&path).unwrap();
        bound.listener.accept().unwrap();
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    /// Carried from 0.13 (`a_stale_socket_is_replaced`).
    #[test]
    fn a_stale_socket_is_replaced() {
        let path = socket_in("stale");
        drop(open(&path).unwrap());
        assert!(path.exists() && UnixStream::connect(&path).is_err());
        let bound = open(&path).unwrap();
        UnixStream::connect(&path).unwrap();
        bound.listener.accept().unwrap();
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    /// 5.9: a daemon that binds the socket itself first checks whether something already
    /// accepts connections on that path (a daemon without the lock, such as a
    /// socket-activated 0.13 daemon).
    #[test]
    fn a_listening_socket_is_never_replaced() {
        let path = socket_in("listening");
        let _other = UnixListener::bind(&path).unwrap();
        let err = open(&path).err().unwrap().to_string();
        assert!(err.contains("already listening"), "{err}");
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn activation_needs_our_pid_and_at_least_one_fd() {
        assert!(activated(Some("42"), Some("1"), 42));
        assert!(activated(Some("42"), Some("2"), 42));
        assert!(
            !activated(Some("41"), Some("1"), 42),
            "another process's sockets"
        );
        assert!(!activated(Some("42"), Some("0"), 42));
        assert!(!activated(None, Some("1"), 42));
        assert!(!activated(Some("42"), None, 42));
        assert!(!activated(Some("x"), Some("1"), 42));
    }
}
