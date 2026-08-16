//! Serve lifecycle through the built binary. Unix only.

#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{Duration, Instant};

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_vissue")
}

fn copy_fixture(dest: &Path) {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixture_vault");
    copy_dir(&src, dest);
}

fn copy_dir(src: &Path, dest: &Path) {
    fs::create_dir_all(dest).unwrap();
    for entry in fs::read_dir(src).unwrap() {
        let entry = entry.unwrap();
        let to = dest.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &to);
        } else {
            fs::copy(entry.path(), to).unwrap();
        }
    }
}

struct Harness {
    _tmp: tempfile::TempDir,
    root: PathBuf,
    socket: PathBuf,
}

impl Harness {
    fn new() -> Self {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("vault");
        copy_fixture(&root);
        let socket = tmp.path().join("run/control.sock");
        Self {
            _tmp: tmp,
            root,
            socket,
        }
    }

    fn run(&self, args: &[&str]) -> Output {
        Command::new(bin())
            .arg("--root")
            .arg(&self.root)
            .arg("--prefix")
            .arg("Software")
            .args(args)
            .arg("-s")
            .arg(&self.socket)
            .output()
            .expect("run vissue")
    }

    fn start_d(&self) -> Output {
        self.run(&["serve", "-d"])
    }

    fn stop(&self) -> Output {
        self.run(&["serve", "stop"])
    }
}

impl Drop for Harness {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

fn stdout(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

fn wait_accepts(socket: &Path, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if std::os::unix::net::UnixStream::connect(socket).is_ok() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    std::os::unix::net::UnixStream::connect(socket).is_ok()
}

#[test]
fn lifecycle_start_status_second_d_stop() {
    let h = Harness::new();
    let out = h.start_d();
    assert!(
        out.status.success(),
        "serve -d failed: {}{}",
        stdout(&out),
        stderr(&out)
    );
    assert!(
        wait_accepts(&h.socket, Duration::from_secs(5)),
        "socket did not accept"
    );

    let status = h.run(&["serve", "status"]);
    assert!(
        status.status.success(),
        "status exit {}: {}{}",
        status.status.code().unwrap_or(-1),
        stdout(&status),
        stderr(&status)
    );
    let text = stdout(&status);
    assert!(text.contains("live: true"), "{text}");
    assert!(
        text.contains(&format!("socket: {}", h.socket.display())),
        "{text}"
    );

    let again = h.start_d();
    assert!(again.status.success(), "{}", stderr(&again));
    let again_text = stdout(&again);
    assert!(
        again_text.contains("already running"),
        "{again_text} {}",
        stderr(&again)
    );
    assert!(again_text.contains("pid="), "{again_text}");
    assert!(
        again_text.contains(&format!("socket={}", h.socket.display())),
        "{again_text}"
    );

    let stop = h.stop();
    assert!(stop.status.success(), "{}", stderr(&stop));

    let down = h.run(&["serve", "status"]);
    assert_eq!(down.status.code(), Some(1), "{}", stdout(&down));
    assert!(stdout(&down).contains("live: false"), "{}", stdout(&down));

    // The socket goes with the owner. One left behind refuses the next bind,
    // and the daemon that finds it has to decide whether it is stale.
    assert!(
        !h.socket.exists(),
        "the socket outlived the owner: {}",
        h.socket.display()
    );
}

#[test]
fn status_json_and_socket_modes() {
    let h = Harness::new();
    assert!(h.start_d().status.success());
    assert!(wait_accepts(&h.socket, Duration::from_secs(5)));

    let dir_mode = fs::metadata(h.socket.parent().unwrap())
        .unwrap()
        .permissions()
        .mode()
        & 0o777;
    assert_eq!(dir_mode, 0o700);
    let sock_mode = fs::metadata(&h.socket).unwrap().permissions().mode() & 0o777;
    assert_eq!(sock_mode, 0o600);

    let out = h.run(&["serve", "status", "--json"]);
    assert!(out.status.success(), "{}", stderr(&out));
    let value: serde_json::Value = serde_json::from_str(&stdout(&out)).unwrap();
    assert_eq!(value["live"], true);
    assert_eq!(value["prefix"], "Software");
    assert!(value["pid"].as_u64().is_some());
    assert!(
        value["revision"].as_u64().unwrap() >= 1,
        "live owner starts catalog revision at 1: {value}"
    );
}

#[test]
fn initialize_requires_agent_over_the_socket() {
    use vissue_control::Error;
    use vissue_control::client::Client;

    let h = Harness::new();
    assert!(h.start_d().status.success());
    assert!(wait_accepts(&h.socket, Duration::from_secs(5)));

    let mut client = Client::connect(&h.socket).unwrap();
    let err = client
        .request(
            "initialize",
            serde_json::json!({"protocolVersion": 1, "client": "cli-test"}),
        )
        .unwrap_err();
    match err {
        Error::Rpc(e) => {
            assert_eq!(e.code, -32602);
            assert_eq!(e.message, "agent is required");
        }
        other => panic!("{other:?}"),
    }

    let ok = client
        .request(
            "initialize",
            serde_json::json!({"protocolVersion": 1, "agent": "cli-test"}),
        )
        .unwrap();
    assert_eq!(ok["identity"], "cli-test");
    let ident = client
        .request("identity/get", serde_json::json!({}))
        .unwrap();
    assert_eq!(ident["identity"], "cli-test");
}

#[test]
fn default_socket_uses_xdg_runtime_dir() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("vault");
    copy_fixture(&root);
    let runtime = tmp.path().join("xdg");
    fs::create_dir_all(&runtime).unwrap();
    let socket = runtime.join("vissue/control.sock");

    let start = Command::new(bin())
        .arg("--root")
        .arg(&root)
        .env("XDG_RUNTIME_DIR", &runtime)
        .env_remove("VISSUE_CONTROL_SOCKET")
        .args(["serve", "-d"])
        .output()
        .unwrap();
    assert!(
        start.status.success(),
        "{}{}",
        String::from_utf8_lossy(&start.stdout),
        String::from_utf8_lossy(&start.stderr)
    );
    assert!(wait_accepts(&socket, Duration::from_secs(5)));

    let status = Command::new(bin())
        .arg("--root")
        .arg(&root)
        .env("XDG_RUNTIME_DIR", &runtime)
        .env_remove("VISSUE_CONTROL_SOCKET")
        .args(["serve", "status"])
        .output()
        .unwrap();
    assert!(status.status.success());

    let _ = Command::new(bin())
        .arg("--root")
        .arg(&root)
        .env("XDG_RUNTIME_DIR", &runtime)
        .env_remove("VISSUE_CONTROL_SOCKET")
        .args(["serve", "stop"])
        .output();
}

#[test]
fn serve_help_hides_foreground() {
    let out = Command::new(bin())
        .args(["serve", "--help"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let text = stdout(&out);
    assert!(text.contains("-d"), "{text}");
    assert!(text.contains("--detach"), "{text}");
    assert!(!text.contains("--foreground"), "{text}");
    assert!(!text.contains("--no-detach"), "{text}");
    assert!(text.contains("stop"), "{text}");
    assert!(text.contains("status"), "{text}");
}

#[test]
fn completions_omit_foreground() {
    // Every shell the CLI offers, not a sample: a generator is per-shell, so
    // one that emits a hidden flag would hide behind the others.
    for shell in ["bash", "elvish", "fish", "powershell", "zsh"] {
        let out = Command::new(bin())
            .args(["completions", shell])
            .output()
            .unwrap();
        assert!(out.status.success(), "{shell}: {}", stderr(&out));
        let text = stdout(&out);
        assert!(
            text.contains("vissue"),
            "{shell} produced nothing that names the binary"
        );
        assert!(
            !text.contains("--foreground"),
            "{shell} completions advertise --foreground"
        );
        assert!(
            !text.contains("--no-detach"),
            "{shell} completions advertise --no-detach"
        );
    }
}

#[test]
fn man_page_has_no_trailing_whitespace() {
    let out = Command::new(bin()).args(["man"]).output().unwrap();
    assert!(out.status.success(), "{}", stderr(&out));
    let text = stdout(&out);
    for (i, line) in text.lines().enumerate() {
        assert_eq!(
            line,
            line.trim_end(),
            "man line {} has trailing space: {line:?}",
            i + 1
        );
    }
    let committed =
        fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../man/vissue.1"))
            .unwrap();
    for (i, line) in committed.lines().enumerate() {
        assert_eq!(
            line,
            line.trim_end(),
            "committed man line {} has trailing space",
            i + 1
        );
    }
}

#[test]
fn committed_completions_omit_foreground() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    for name in [
        "completions/vissue.bash",
        "completions/_vissue",
        "completions/vissue.fish",
    ] {
        let text = fs::read_to_string(root.join(name)).unwrap();
        assert!(
            !text.contains("--foreground"),
            "{name} still contains --foreground"
        );
        assert!(
            !text.contains("--no-detach"),
            "{name} still contains --no-detach"
        );
    }
}

#[test]
fn claim_uses_client_agent_not_process_env() {
    use vissue_control::Error;
    use vissue_control::client::Client;

    let h = Harness::new();
    let start = Command::new(bin())
        .arg("--root")
        .arg(&h.root)
        .arg("--prefix")
        .arg("Software")
        .args(["serve", "-d"])
        .arg("-s")
        .arg(&h.socket)
        .env("VISSUE_AGENT", "daemon")
        .output()
        .unwrap();
    assert!(
        start.status.success(),
        "{}{}",
        stdout(&start),
        stderr(&start)
    );
    assert!(wait_accepts(&h.socket, Duration::from_secs(5)));

    let mut client = Client::connect(&h.socket).unwrap();
    client
        .request(
            "initialize",
            serde_json::json!({"protocolVersion": 1, "agent": "tui"}),
        )
        .unwrap();
    let claimed = client
        .request("issue/claim", serde_json::json!({"id": "atlas-2c3d"}))
        .unwrap();
    assert_eq!(claimed["issue"]["claimed_by"], "tui");
    let text = fs::read_to_string(h.root.join("Software/atlas/issues.org")).unwrap();
    assert!(
        text.contains("tui"),
        "claim stamp must be the client agent: {text}"
    );
    assert!(
        !text.contains(":CLAIMED_BY: daemon") && !text.contains("CLAIMED_BY:         daemon"),
        "process VISSUE_AGENT must not stamp the claim: {text}"
    );

    let err = client
        .request(
            "issue/claim",
            serde_json::json!({"id": "atlas-2c3d", "agent": "other"}),
        )
        .unwrap_err();
    match err {
        Error::Rpc(e) => assert_eq!(e.code, -32009),
        other => panic!("{other:?}"),
    }
}

#[test]
fn restart_brings_a_stopped_owner_back() {
    let h = Harness::new();
    assert!(h.start_d().status.success());
    assert!(wait_accepts(&h.socket, Duration::from_secs(5)));
    let first = stdout(&h.run(&["serve", "status"]))
        .lines()
        .find_map(|l| l.strip_prefix("pid: "))
        .map(|p| p.trim().to_string());
    assert!(first.is_some(), "no pid while it was up");
    assert!(h.stop().status.success());
    let restart = h.run(&["serve", "restart"]);
    assert!(
        restart.status.success(),
        "{}{}",
        stdout(&restart),
        stderr(&restart)
    );
    assert!(wait_accepts(&h.socket, Duration::from_secs(5)));
    let back = h.run(&["serve", "status"]);
    assert!(back.status.success());
    // A restart that kept the old process would satisfy everything above,
    // so the pid has to have moved.
    let pid = |text: &str| -> Option<String> {
        text.lines()
            .find_map(|l| l.strip_prefix("pid: "))
            .map(|p| p.trim().to_string())
    };
    let after = pid(&stdout(&back)).expect("a pid once it is back");
    assert!(!after.is_empty(), "{}", stdout(&back));
    assert_ne!(Some(after), first, "restart kept the same process");
}
