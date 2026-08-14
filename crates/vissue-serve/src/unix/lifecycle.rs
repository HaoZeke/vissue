//! Detach, stop, status, and ensure. The detach parent never starts tokio.

use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::os::unix::net::UnixStream;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use nix::sys::signal::{kill, Signal};
use nix::unistd::Pid;
use serde_json::json;
use vissue_control::client::Client;
use vissue_core::events;

use super::owner::{lock_is_free, log_path, prepare_socket_dir, read_pid_file, remove_pid_file};
use crate::{EnsureResult, ServeConfig, Status, ACCEPT_TIMEOUT_MS, SERVE_REVISION};

const POLL_INTERVAL: Duration = Duration::from_millis(50);
const TERM_WAIT: Duration = Duration::from_secs(5);
const KILL_WAIT: Duration = Duration::from_secs(2);

pub fn socket_accepts(path: &Path) -> bool {
    if !path.exists() {
        return false;
    }
    let deadline = Instant::now() + Duration::from_millis(150);
    let mut delay = Duration::from_millis(20);
    loop {
        match UnixStream::connect(path) {
            Ok(_) => return true,
            Err(err)
                if err.kind() == io::ErrorKind::ConnectionRefused
                    || err.kind() == io::ErrorKind::NotFound =>
            {
                return false;
            }
            Err(err)
                if err.kind() == io::ErrorKind::WouldBlock
                    || err.kind() == io::ErrorKind::Interrupted
                    || err.kind() == io::ErrorKind::TimedOut =>
            {
                if Instant::now() >= deadline {
                    return false;
                }
                thread::sleep(delay);
                delay = delay.saturating_mul(2).min(Duration::from_millis(50));
            }
            Err(_) => return false,
        }
    }
}

pub fn wait_until_accepts(path: &Path, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if socket_accepts(path) {
            return true;
        }
        thread::sleep(POLL_INTERVAL);
    }
    socket_accepts(path)
}

pub fn pid_is_alive(pid: u32) -> bool {
    if pid == 0 {
        return false;
    }
    match kill(Pid::from_raw(pid as i32), None) {
        Ok(()) => true,
        Err(nix::errno::Errno::ESRCH) => false,
        Err(_) => true,
    }
}

fn signal_pid(pid: u32, sig: Signal) -> io::Result<()> {
    let raw = pid as i32;
    let group = kill(Pid::from_raw(-raw), sig);
    if group.is_ok() {
        return Ok(());
    }
    kill(Pid::from_raw(raw), sig).map_err(|e| io::Error::from_raw_os_error(e as i32))
}

fn wait_pid_gone(pid: u32, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if !pid_is_alive(pid) {
            return true;
        }
        thread::sleep(POLL_INTERVAL);
    }
    !pid_is_alive(pid)
}

pub fn status(cfg: &ServeConfig) -> Status {
    let live = socket_accepts(&cfg.socket);
    let pid = read_pid_file(&cfg.socket);
    let mut snap = Status {
        live,
        pid,
        socket: cfg.socket.clone(),
        root: cfg.layout.root().to_path_buf(),
        prefix: cfg.layout.prefix().to_string(),
        generation: events::generation(&cfg.layout),
        revision: SERVE_REVISION,
        clients: 0,
    };
    if live {
        enrich_from_owner(&mut snap);
    }
    snap
}

fn enrich_from_owner(snap: &mut Status) {
    let mut client = match Client::connect(&snap.socket) {
        Ok(c) => c,
        Err(_) => return,
    };
    let result = client.request(
        "initialize",
        json!({
            "protocolVersion": 1,
            "client": "vissue-status",
            "agent": "vissue-status",
        }),
    );
    let Ok(value) = result else {
        return;
    };
    if let Some(root) = value.get("root").and_then(serde_json::Value::as_str) {
        snap.root = Path::new(root).to_path_buf();
    }
    if let Some(prefix) = value.get("prefix").and_then(serde_json::Value::as_str) {
        snap.prefix = prefix.to_string();
    }
    if let Some(generation) = value.get("generation").and_then(serde_json::Value::as_u64) {
        snap.generation = generation;
    }
    if let Some(revision) = value.get("revision").and_then(serde_json::Value::as_u64) {
        snap.revision = revision;
    }
}

pub fn print_status(status: &Status, json: bool) -> Result<()> {
    if json {
        let rendered = serde_json::to_string_pretty(status).context("encode status")?;
        println!("{rendered}");
        return Ok(());
    }
    println!("live: {}", status.live);
    match status.pid {
        Some(pid) => println!("pid: {pid}"),
        None => println!("pid:"),
    }
    println!("socket: {}", status.socket.display());
    println!("root: {}", status.root.display());
    println!("prefix: {}", status.prefix);
    println!("generation: {}", status.generation);
    println!("revision: {}", status.revision);
    println!("clients: {}", status.clients);
    Ok(())
}

pub fn stop(cfg: &ServeConfig) -> Result<i32> {
    let socket = &cfg.socket;
    let pid = read_pid_file(socket);
    let accepts = socket_accepts(socket);

    if lock_is_free(socket) {
        if accepts {
            eprintln!("error: control socket is live but the ownership lock is free; not stopping");
            return Ok(1);
        }
        unlink_stale_socket_if_lock_free(socket);
        remove_pid_file(socket);
        eprintln!("already stopped  socket={}", socket.display());
        return Ok(0);
    }

    if accepts {
        match pid {
            None => {
                eprintln!(
                    "error: control socket is live but no daemon pid file \
                     (owner is not a vissue serve process; not stopping)"
                );
                return Ok(1);
            }
            Some(pid) if !pid_is_alive(pid) => {
                eprintln!(
                    "error: pid {pid} is not running but socket still accepts connections; \
                     not unlinking live socket"
                );
                remove_pid_file(socket);
                return Ok(1);
            }
            Some(_) => {}
        }
    }

    if let Some(pid) = pid.filter(|p| pid_is_alive(*p)) {
        return stop_pid(pid, socket);
    }

    unlink_stale_socket_if_lock_free(socket);
    remove_pid_file(socket);
    eprintln!("already stopped  socket={}", socket.display());
    Ok(0)
}

fn stop_pid(pid: u32, socket: &Path) -> Result<i32> {
    if let Err(err) = signal_pid(pid, Signal::SIGTERM) {
        if err.kind() == io::ErrorKind::NotFound {
            unlink_stale_socket_if_lock_free(socket);
            remove_pid_file(socket);
            eprintln!("already stopped  socket={}", socket.display());
            return Ok(0);
        }
        eprintln!("error: could not signal pid {pid}: {err}");
        return Ok(1);
    }
    if wait_pid_gone(pid, TERM_WAIT) {
        unlink_stale_socket_if_lock_free(socket);
        remove_pid_file(socket);
        eprintln!("stopped pid={pid}");
        return Ok(0);
    }
    let _ = signal_pid(pid, Signal::SIGKILL);
    if wait_pid_gone(pid, KILL_WAIT) {
        unlink_stale_socket_if_lock_free(socket);
        remove_pid_file(socket);
        eprintln!("stopped pid={pid} (SIGKILL)");
        return Ok(0);
    }
    eprintln!(
        "error: pid {pid} did not exit within {}s",
        TERM_WAIT.as_secs()
    );
    Ok(1)
}

fn unlink_stale_socket_if_lock_free(socket: &Path) {
    if socket_accepts(socket) {
        return;
    }
    if !lock_is_free(socket) {
        return;
    }
    if socket.exists() {
        let _ = fs::remove_file(socket);
    }
}

pub fn start_detached(cfg: &ServeConfig) -> Result<EnsureResult> {
    if socket_accepts(&cfg.socket) {
        return Ok(EnsureResult {
            ok: true,
            already_running: true,
            spawned: false,
            pid: read_pid_file(&cfg.socket),
            socket: cfg.socket.clone(),
            error: None,
        });
    }

    prepare_socket_dir(&cfg.socket)?;
    let log = log_path(&cfg.socket);
    if let Some(parent) = log.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
        }
    }
    let mut log_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log)
        .with_context(|| format!("open {}", log.display()))?;
    let _ = writeln!(
        log_file,
        "\n--- vissue serve --foreground spawn parent={} ---",
        std::process::id()
    );

    let exe = match &cfg.exe {
        Some(path) => path.clone(),
        None => std::env::current_exe().context("resolve current executable")?,
    };
    let stdout = log_file.try_clone().context("clone serve log for stdout")?;
    let mut cmd = Command::new(&exe);
    cmd.arg("serve")
        .arg("--foreground")
        .arg("--root")
        .arg(cfg.layout.root())
        .arg("--prefix")
        .arg(cfg.layout.prefix())
        .arg("--socket")
        .arg(&cfg.socket)
        .stdin(Stdio::null())
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(log_file))
        .process_group(0);

    let child = match cmd.spawn() {
        Ok(child) => child,
        Err(err) => {
            return Ok(EnsureResult {
                ok: false,
                already_running: false,
                spawned: false,
                pid: None,
                socket: cfg.socket.clone(),
                error: Some(format!("spawn failed: {err}")),
            });
        }
    };

    let timeout = Duration::from_millis(ACCEPT_TIMEOUT_MS);
    if wait_until_accepts(&cfg.socket, timeout) {
        return Ok(EnsureResult {
            ok: true,
            already_running: false,
            spawned: true,
            pid: read_pid_file(&cfg.socket).or(Some(child.id())),
            socket: cfg.socket.clone(),
            error: None,
        });
    }

    let _ = signal_pid(child.id(), Signal::SIGTERM);
    let mut err = format!(
        "control socket did not accept within {}s",
        timeout.as_secs()
    );
    if let Ok(tail) = fs::read_to_string(&log) {
        let snippet = if tail.len() > 800 {
            &tail[tail.len() - 800..]
        } else {
            &tail
        };
        if !snippet.trim().is_empty() {
            err = format!("{err}\nlog tail:\n{snippet}");
        }
    }
    Ok(EnsureResult {
        ok: false,
        already_running: false,
        spawned: true,
        pid: Some(child.id()),
        socket: cfg.socket.clone(),
        error: Some(err),
    })
}

pub fn ensure_serve(cfg: &ServeConfig) -> Result<EnsureResult> {
    if socket_accepts(&cfg.socket) {
        return Ok(EnsureResult {
            ok: true,
            already_running: true,
            spawned: false,
            pid: read_pid_file(&cfg.socket),
            socket: cfg.socket.clone(),
            error: None,
        });
    }
    start_detached(cfg)
}

#[cfg(test)]
mod tests {
    use super::super::owner::write_pid_file;
    use super::*;
    use crate::Action;
    use std::os::unix::fs::PermissionsExt;
    use std::os::unix::net::UnixListener;
    use vissue_core::config::Layout;

    fn cfg(dir: &Path) -> ServeConfig {
        ServeConfig {
            layout: Layout::new(dir.join("vault"), "Software"),
            socket: dir.join("run/control.sock"),
            exe: None,
        }
    }

    #[test]
    fn missing_socket_does_not_accept() {
        let dir = tempfile::tempdir().unwrap();
        assert!(!socket_accepts(&dir.path().join("nope.sock")));
    }

    #[test]
    fn bound_socket_accepts() {
        let dir = tempfile::tempdir().unwrap();
        let socket = dir.path().join("control.sock");
        let _listener = UnixListener::bind(&socket).unwrap();
        assert!(socket_accepts(&socket));
    }

    #[test]
    fn status_when_down_exits_1() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = cfg(dir.path());
        let snap = status(&cfg);
        assert!(!snap.live);
        assert_eq!(snap.revision, SERVE_REVISION);
        assert_eq!(snap.prefix, "Software");
        assert_eq!(
            crate::invoke(Action::Status { json: false }, &cfg).unwrap(),
            1
        );
    }

    #[test]
    fn stop_when_already_stopped_is_ok() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = cfg(dir.path());
        prepare_socket_dir(&cfg.socket).unwrap();
        assert_eq!(stop(&cfg).unwrap(), 0);
        assert!(
            vissue_control::socket_lock_path(&cfg.socket).exists() || lock_is_free(&cfg.socket)
        );
    }

    #[test]
    fn stop_unlinks_stale_socket_only_when_lock_is_free() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = cfg(dir.path());
        prepare_socket_dir(&cfg.socket).unwrap();
        let listener = UnixListener::bind(&cfg.socket).unwrap();
        drop(listener);
        assert!(cfg.socket.exists());
        assert_eq!(stop(&cfg).unwrap(), 0);
        assert!(!cfg.socket.exists());
        assert!(
            vissue_control::socket_lock_path(&cfg.socket).exists(),
            "lock file must not be unlinked"
        );
    }

    #[test]
    fn stop_does_not_unlink_a_live_socket_without_pid() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = cfg(dir.path());
        prepare_socket_dir(&cfg.socket).unwrap();
        let _listener = UnixListener::bind(&cfg.socket).unwrap();
        assert_eq!(stop(&cfg).unwrap(), 1);
        assert!(cfg.socket.exists());
    }

    #[test]
    fn stop_does_not_signal_a_reused_pid_when_lock_is_free() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = cfg(dir.path());
        prepare_socket_dir(&cfg.socket).unwrap();
        let us = std::process::id();
        write_pid_file(&cfg.socket, us).unwrap();
        assert!(lock_is_free(&cfg.socket));
        assert_eq!(stop(&cfg).unwrap(), 0);
        assert!(
            pid_is_alive(us),
            "stop must not signal a pid-file number when the lock is free"
        );
        assert!(read_pid_file(&cfg.socket).is_none());
    }

    #[test]
    fn stop_refuses_a_live_socket_when_lock_is_free() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = cfg(dir.path());
        prepare_socket_dir(&cfg.socket).unwrap();
        let _listener = UnixListener::bind(&cfg.socket).unwrap();
        let us = std::process::id();
        write_pid_file(&cfg.socket, us).unwrap();
        assert!(lock_is_free(&cfg.socket));
        assert_eq!(stop(&cfg).unwrap(), 1);
        assert!(pid_is_alive(us));
        assert!(cfg.socket.exists());
        assert_eq!(read_pid_file(&cfg.socket), Some(us));
    }

    #[test]
    fn detach_reports_already_running_when_socket_accepts() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = cfg(dir.path());
        prepare_socket_dir(&cfg.socket).unwrap();
        let _listener = UnixListener::bind(&cfg.socket).unwrap();
        write_pid_file(&cfg.socket, 99).unwrap();
        let result = start_detached(&cfg).unwrap();
        assert!(result.ok);
        assert!(result.already_running);
        assert!(!result.spawned);
        assert_eq!(result.pid, Some(99));
        let ensured = ensure_serve(&cfg).unwrap();
        assert!(ensured.already_running);
        assert!(ensured.live());
    }

    #[test]
    fn detach_missing_exe_fails_without_binding() {
        let dir = tempfile::tempdir().unwrap();
        let mut cfg = cfg(dir.path());
        cfg.exe = Some(dir.path().join("no-such-vissue"));
        let result = start_detached(&cfg).unwrap();
        assert!(!result.ok);
        assert!(result.error.as_deref().unwrap().contains("spawn failed"));
        assert!(!cfg.socket.exists());
    }

    #[test]
    fn pid_zero_is_not_alive() {
        assert!(!pid_is_alive(0));
    }

    #[test]
    fn this_process_is_alive() {
        assert!(pid_is_alive(std::process::id()));
    }

    #[test]
    fn print_status_json_mentions_live() {
        let status = Status {
            live: false,
            pid: None,
            socket: "/tmp/control.sock".into(),
            root: "/tmp/vault".into(),
            prefix: "Software".into(),
            generation: 0,
            revision: 0,
            clients: 0,
        };
        print_status(&status, true).unwrap();
    }

    #[test]
    fn socket_dir_mode_is_0700_after_prepare() {
        let dir = tempfile::tempdir().unwrap();
        let socket = dir.path().join("vissue/control.sock");
        prepare_socket_dir(&socket).unwrap();
        let mode = fs::metadata(socket.parent().unwrap())
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o700);
    }
}
