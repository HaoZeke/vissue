//! Bind, flock, accept, and the initialize / identity/get handshake.

use std::fs::{self, File, OpenOptions};
use std::io::{self, BufReader, Write};
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::os::unix::net::UnixStream as StdUnixStream;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use anyhow::{bail, Context, Result};
use fs2::FileExt;
use serde_json::{json, Value};
use tokio::net::UnixListener;
use vissue_control::frame::{read_message, write_message, FrameError, Framing};
use vissue_control::peercred::accept_socket;
use vissue_control::rpc::IdentityResult;
use vissue_control::{beside_socket, socket_lock_path, socket_pid_path};
use vissue_control::{
    invalid_params, invalid_request, method_not_found, parse_error, parse_initialize_params,
    InitializeResult, JsonRpcId, JsonRpcRequest, JsonRpcResponse, PROTOCOL_VERSION,
};
use vissue_core::events;

use crate::{ServeConfig, LIVE_CAPABILITIES, SERVE_REVISION};

pub(super) struct OwnerState {
    pub layout: vissue_core::config::Layout,
    pub clients: AtomicUsize,
}

pub(super) struct Session {
    pub agent: Option<String>,
}

struct Owner {
    listener: UnixListener,
    lock: File,
    socket: PathBuf,
    pid_path: PathBuf,
    state: Arc<OwnerState>,
}

impl Drop for Owner {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.socket);
        let _ = fs::remove_file(&self.pid_path);
        let _ = self.lock.set_len(0);
    }
}

pub(super) fn run_foreground(cfg: &ServeConfig) -> Result<()> {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .context("start tokio runtime")?;
    rt.block_on(serve(cfg))
}

async fn serve(cfg: &ServeConfig) -> Result<()> {
    let owner = Owner::bind(cfg)?;
    eprintln!("vissue serve: control socket {}", owner.socket.display());
    eprintln!("  root={}", cfg.layout.root().display());
    eprintln!("  prefix={}", cfg.layout.prefix());
    eprintln!("  pid={}", std::process::id());
    tokio::select! {
        result = owner.accept_loop() => result,
        _ = shutdown_signal() => Ok(()),
    }
}

async fn shutdown_signal() {
    let ctrl_c = tokio::signal::ctrl_c();
    let mut term = match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
        Ok(s) => s,
        Err(_) => {
            let _ = ctrl_c.await;
            return;
        }
    };
    tokio::select! {
        _ = ctrl_c => {}
        _ = term.recv() => {}
    }
}

impl Owner {
    fn bind(cfg: &ServeConfig) -> Result<Self> {
        let socket = cfg.socket.clone();
        prepare_socket_dir(&socket)?;
        let lock = acquire_lock(&socket)?;
        takeover_or_fail(&socket)?;
        let listener =
            UnixListener::bind(&socket).with_context(|| format!("bind {}", socket.display()))?;
        chmod_path(&socket, 0o600)?;
        write_pid_file(&socket, std::process::id())?;
        Ok(Self {
            listener,
            lock,
            pid_path: socket_pid_path(&socket),
            socket,
            state: Arc::new(OwnerState {
                layout: cfg.layout.clone(),
                clients: AtomicUsize::new(0),
            }),
        })
    }

    async fn accept_loop(&self) -> Result<()> {
        loop {
            let (stream, _) = self.listener.accept().await.context("accept")?;
            if !accept_socket(&stream) {
                if let Ok(uid) = vissue_control::peercred::peer_uid(&stream) {
                    eprintln!("vissue serve: rejected peer uid={uid}");
                } else {
                    eprintln!("vissue serve: rejected peer");
                }
                continue;
            }
            let std_stream = match stream.into_std() {
                Ok(s) => s,
                Err(err) => {
                    eprintln!("vissue serve: into_std: {err}");
                    continue;
                }
            };
            if let Err(err) = std_stream.set_nonblocking(false) {
                eprintln!("vissue serve: set_nonblocking: {err}");
                continue;
            }
            let state = Arc::clone(&self.state);
            std::thread::spawn(move || {
                state.clients.fetch_add(1, Ordering::Relaxed);
                handle_client(std_stream, &state);
                state.clients.fetch_sub(1, Ordering::Relaxed);
            });
        }
    }
}

pub(super) fn prepare_socket_dir(socket: &Path) -> Result<()> {
    let parent = socket.parent().filter(|p| !p.as_os_str().is_empty());
    if let Some(dir) = parent {
        fs::create_dir_all(dir).with_context(|| format!("create {}", dir.display()))?;
        chmod_path(dir, 0o700)?;
    }
    Ok(())
}

fn chmod_path(path: &Path, mode: u32) -> Result<()> {
    let mut perms = fs::metadata(path)
        .with_context(|| format!("stat {}", path.display()))?
        .permissions();
    perms.set_mode(mode);
    fs::set_permissions(path, perms).with_context(|| format!("chmod {:o} {}", mode, path.display()))
}

fn acquire_lock(socket: &Path) -> Result<File> {
    let lock_path = socket_lock_path(socket);
    if let Some(parent) = lock_path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
        }
    }
    let mut file = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(false)
        .mode(0o600)
        .open(&lock_path)
        .with_context(|| format!("open {}", lock_path.display()))?;
    file.try_lock_exclusive()
        .with_context(|| format!("control socket already in use: {}", socket.display()))?;
    file.set_len(0)?;
    writeln!(file, "{}", std::process::id())?;
    file.sync_all()?;
    Ok(file)
}

/// True when nothing holds the exclusive lock. The lock file is never unlinked.
pub(super) fn lock_is_free(socket: &Path) -> bool {
    let lock_path = socket_lock_path(socket);
    let file = match OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(false)
        .mode(0o600)
        .open(&lock_path)
    {
        Ok(f) => f,
        Err(_) => return false,
    };
    match file.try_lock_exclusive() {
        Ok(()) => {
            let _ = fs2::FileExt::unlock(&file);
            true
        }
        Err(_) => false,
    }
}

fn takeover_or_fail(socket: &Path) -> Result<()> {
    if !socket.exists() {
        return Ok(());
    }
    match StdUnixStream::connect(socket) {
        Ok(_) => bail!("control socket already in use: {}", socket.display()),
        Err(err)
            if err.kind() == io::ErrorKind::ConnectionRefused
                || err.kind() == io::ErrorKind::NotFound =>
        {
            fs::remove_file(socket)
                .with_context(|| format!("unlink stale socket {}", socket.display()))?;
            Ok(())
        }
        Err(err) => {
            bail!(
                "control socket already in use: {} ({err})",
                socket.display()
            )
        }
    }
}

pub(super) fn write_pid_file(socket: &Path, pid: u32) -> Result<PathBuf> {
    let path = socket_pid_path(socket);
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
        }
    }
    fs::write(&path, format!("{pid}\n")).with_context(|| format!("write {}", path.display()))?;
    chmod_path(&path, 0o600)?;
    Ok(path)
}

pub(super) fn read_pid_file(socket: &Path) -> Option<u32> {
    let raw = fs::read_to_string(socket_pid_path(socket)).ok()?;
    raw.split_whitespace().next()?.parse().ok()
}

pub(super) fn remove_pid_file(socket: &Path) {
    let _ = fs::remove_file(socket_pid_path(socket));
}

pub(super) fn log_path(socket: &Path) -> PathBuf {
    match std::env::var(vissue_control::SERVE_LOG_ENV) {
        Ok(value) if !value.trim().is_empty() => PathBuf::from(value),
        _ => beside_socket(socket, "control.log"),
    }
}

fn handle_client(stream: StdUnixStream, state: &OwnerState) {
    let cloned = match stream.try_clone() {
        Ok(s) => s,
        Err(_) => return,
    };
    let mut reader = BufReader::new(cloned);
    let mut writer = stream;
    let mut session = Session { agent: None };
    loop {
        let (payload, framing) = match read_message(&mut reader) {
            Ok(v) => v,
            Err(FrameError::Incomplete) => break,
            Err(FrameError::Io(err)) if err.kind() == io::ErrorKind::UnexpectedEof => break,
            Err(_) => {
                let _ = write_response(
                    &mut writer,
                    &JsonRpcResponse::err(Some(JsonRpcId::Null), parse_error()),
                    Framing::Jsonl,
                );
                break;
            }
        };
        if payload.iter().all(u8::is_ascii_whitespace) {
            continue;
        }
        let value: Value = match serde_json::from_slice(&payload) {
            Ok(v) => v,
            Err(_) => {
                let _ = write_response(
                    &mut writer,
                    &JsonRpcResponse::err(Some(JsonRpcId::Null), parse_error()),
                    framing,
                );
                continue;
            }
        };
        let req: JsonRpcRequest = match serde_json::from_value(value) {
            Ok(r) => r,
            Err(_) => {
                let _ = write_response(
                    &mut writer,
                    &JsonRpcResponse::err(Some(JsonRpcId::Null), invalid_request()),
                    framing,
                );
                continue;
            }
        };
        if let Some(resp) = dispatch(state, &mut session, &req) {
            let _ = write_response(&mut writer, &resp, framing);
        }
    }
}

fn write_response(
    writer: &mut StdUnixStream,
    resp: &JsonRpcResponse,
    framing: Framing,
) -> io::Result<()> {
    let bytes = serde_json::to_vec(resp)?;
    write_message(writer, &bytes, framing)?;
    writer.flush()
}

pub(super) fn dispatch(
    state: &OwnerState,
    session: &mut Session,
    req: &JsonRpcRequest,
) -> Option<JsonRpcResponse> {
    if req.is_notification() {
        return None;
    }
    let id = req.id.clone();
    let result = match req.method.as_str() {
        "initialize" => dispatch_initialize(state, session, req.params.as_ref()),
        "identity/get" => dispatch_identity(state, session),
        other => Err(method_not_found(other)),
    };
    Some(match result {
        Ok(value) => JsonRpcResponse::ok(id, value),
        Err(err) => JsonRpcResponse::err(id, err),
    })
}

fn dispatch_initialize(
    state: &OwnerState,
    session: &mut Session,
    params: Option<&Value>,
) -> Result<Value, vissue_control::JsonRpcError> {
    let params = parse_initialize_params(params.unwrap_or(&json!({})))?;
    session.agent = Some(params.agent.clone());
    let result = InitializeResult {
        protocol_version: PROTOCOL_VERSION,
        capabilities: LIVE_CAPABILITIES.iter().map(|s| (*s).to_string()).collect(),
        root: state.layout.root().display().to_string(),
        prefix: state.layout.prefix().to_string(),
        generation: events::generation(&state.layout),
        revision: SERVE_REVISION,
        identity: params.agent,
    };
    serde_json::to_value(result).map_err(|e| vissue_control::rpc::internal_error(e.to_string()))
}

fn dispatch_identity(
    state: &OwnerState,
    session: &Session,
) -> Result<Value, vissue_control::JsonRpcError> {
    let identity = session
        .agent
        .clone()
        .ok_or_else(|| invalid_params("initialize required"))?;
    let result = IdentityResult {
        identity,
        root: state.layout.root().display().to_string(),
        prefix: state.layout.prefix().to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    };
    serde_json::to_value(result).map_err(|e| vissue_control::rpc::internal_error(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SERVE_REVISION;
    use serde_json::json;
    use std::os::unix::net::UnixListener as StdUnixListener;
    use vissue_core::config::Layout;

    fn state(dir: &Path) -> OwnerState {
        OwnerState {
            layout: Layout::new(dir, "Software"),
            clients: AtomicUsize::new(0),
        }
    }

    #[test]
    fn initialize_requires_agent() {
        let dir = tempfile::tempdir().unwrap();
        let state = state(dir.path());
        let mut session = Session { agent: None };
        let req = JsonRpcRequest::call(
            JsonRpcId::Number(1),
            "initialize",
            json!({"protocolVersion": 1, "client": "tui"}),
        );
        let resp = dispatch(&state, &mut session, &req).unwrap();
        let err = resp.error.unwrap();
        assert_eq!(err.code, -32602);
        assert_eq!(err.message, "agent is required");
        assert!(session.agent.is_none());
    }

    #[test]
    fn initialize_empty_agent_is_invalid_params() {
        let dir = tempfile::tempdir().unwrap();
        let state = state(dir.path());
        let mut session = Session { agent: None };
        let req = JsonRpcRequest::call(
            JsonRpcId::Number(1),
            "initialize",
            json!({"protocolVersion": 1, "agent": "  "}),
        );
        let err = dispatch(&state, &mut session, &req).unwrap().error.unwrap();
        assert_eq!(err.code, -32602);
    }

    #[test]
    fn initialize_version_2_names_supported() {
        let dir = tempfile::tempdir().unwrap();
        let state = state(dir.path());
        let mut session = Session { agent: None };
        let req = JsonRpcRequest::call(
            JsonRpcId::Number(1),
            "initialize",
            json!({"protocolVersion": 2, "agent": "tui"}),
        );
        let err = dispatch(&state, &mut session, &req).unwrap().error.unwrap();
        assert_eq!(err.code, -32602);
        assert_eq!(err.data, Some(json!({"supported": 1})));
    }

    #[test]
    fn initialize_returns_live_capabilities_and_agent_identity() {
        let dir = tempfile::tempdir().unwrap();
        let state = state(dir.path());
        let mut session = Session { agent: None };
        let req = JsonRpcRequest::call(
            JsonRpcId::Number(1),
            "initialize",
            json!({"protocolVersion": 1, "client": "tui", "agent": "tui-agent"}),
        );
        let resp = dispatch(&state, &mut session, &req).unwrap();
        let result = resp.result.unwrap();
        assert_eq!(result["protocolVersion"], 1);
        assert_eq!(result["identity"], "tui-agent");
        assert_eq!(result["prefix"], "Software");
        assert_eq!(result["revision"], SERVE_REVISION);
        let caps = result["capabilities"].as_array().unwrap();
        assert_eq!(caps, &vec![json!("identity/get")]);
        assert!(!caps.iter().any(|c| c == "issue/list"));
        assert_eq!(session.agent.as_deref(), Some("tui-agent"));
    }

    #[test]
    fn identity_get_requires_initialize() {
        let dir = tempfile::tempdir().unwrap();
        let state = state(dir.path());
        let mut session = Session { agent: None };
        let req = JsonRpcRequest::call(JsonRpcId::Number(2), "identity/get", json!({}));
        let err = dispatch(&state, &mut session, &req).unwrap().error.unwrap();
        assert_eq!(err.code, -32602);
        assert_eq!(err.message, "initialize required");
    }

    #[test]
    fn identity_get_returns_connection_agent() {
        let dir = tempfile::tempdir().unwrap();
        let state = state(dir.path());
        let mut session = Session {
            agent: Some("tui-agent".into()),
        };
        let req = JsonRpcRequest::call(JsonRpcId::Number(2), "identity/get", json!({}));
        let result = dispatch(&state, &mut session, &req)
            .unwrap()
            .result
            .unwrap();
        assert_eq!(result["identity"], "tui-agent");
        assert_eq!(result["prefix"], "Software");
        assert_eq!(result["version"], env!("CARGO_PKG_VERSION"));
        assert_eq!(result["root"], dir.path().display().to_string());
    }

    #[test]
    fn unknown_method_is_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let state = state(dir.path());
        let mut session = Session { agent: None };
        let req = JsonRpcRequest::call(JsonRpcId::Number(3), "issue/list", json!({}));
        let err = dispatch(&state, &mut session, &req).unwrap().error.unwrap();
        assert_eq!(err.code, -32601);
        assert_eq!(err.data, Some(json!({"method": "issue/list"})));
    }

    #[test]
    fn notifications_have_no_reply() {
        let dir = tempfile::tempdir().unwrap();
        let state = state(dir.path());
        let mut session = Session { agent: None };
        let req = JsonRpcRequest::notification("serve/shutting_down", json!({}));
        assert!(dispatch(&state, &mut session, &req).is_none());
    }

    #[test]
    fn pid_file_roundtrip_is_mode_600() {
        let dir = tempfile::tempdir().unwrap();
        let socket = dir.path().join("control.sock");
        write_pid_file(&socket, 4242).unwrap();
        assert_eq!(read_pid_file(&socket), Some(4242));
        let mode = fs::metadata(socket_pid_path(&socket))
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o600);
        remove_pid_file(&socket);
        assert!(read_pid_file(&socket).is_none());
    }

    #[test]
    fn prepare_socket_dir_is_0700() {
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

    #[test]
    fn lock_is_free_when_unlocked_and_held_when_locked() {
        let dir = tempfile::tempdir().unwrap();
        let socket = dir.path().join("control.sock");
        prepare_socket_dir(&socket).unwrap();
        assert!(lock_is_free(&socket));
        let held = acquire_lock(&socket).unwrap();
        assert!(!lock_is_free(&socket));
        drop(held);
        assert!(lock_is_free(&socket));
        assert!(socket_lock_path(&socket).exists());
    }

    #[test]
    fn takeover_unlinks_a_refused_stale_socket() {
        let dir = tempfile::tempdir().unwrap();
        let socket = dir.path().join("control.sock");
        let listener = StdUnixListener::bind(&socket).unwrap();
        drop(listener);
        assert!(socket.exists());
        takeover_or_fail(&socket).unwrap();
        assert!(!socket.exists());
    }

    #[test]
    fn takeover_refuses_a_live_socket() {
        let dir = tempfile::tempdir().unwrap();
        let socket = dir.path().join("control.sock");
        let _listener = StdUnixListener::bind(&socket).unwrap();
        let err = takeover_or_fail(&socket).unwrap_err();
        assert!(err.to_string().contains("already in use"), "{err}");
    }

    #[test]
    fn log_path_sits_beside_the_socket() {
        let socket = Path::new("/run/user/1000/vissue/control.sock");
        assert_eq!(
            log_path(socket),
            PathBuf::from("/run/user/1000/vissue/control.log")
        );
    }

    struct TestOwner {
        shutdown: Option<tokio::sync::oneshot::Sender<()>>,
        thread: Option<std::thread::JoinHandle<()>>,
        socket: PathBuf,
    }

    impl Drop for TestOwner {
        fn drop(&mut self) {
            if let Some(tx) = self.shutdown.take() {
                let _ = tx.send(());
            }
            if let Some(handle) = self.thread.take() {
                let _ = handle.join();
            }
        }
    }

    fn start_test_owner(dir: &Path) -> TestOwner {
        use super::super::lifecycle::wait_until_accepts;
        use std::time::Duration;

        let socket = dir.join("run/control.sock");
        let layout = Layout::new(dir.join("vault"), "Software");
        let _ = fs::create_dir_all(layout.projects_dir());
        let cfg = ServeConfig {
            layout,
            socket: socket.clone(),
            exe: None,
        };
        let (tx, rx) = tokio::sync::oneshot::channel();
        let thread = std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("test runtime");
            rt.block_on(async move {
                let owner = Owner::bind(&cfg).expect("bind test owner");
                tokio::select! {
                    _ = owner.accept_loop() => {}
                    _ = rx => {}
                }
            });
        });
        assert!(
            wait_until_accepts(&socket, Duration::from_secs(2)),
            "test owner did not accept on {}",
            socket.display()
        );
        TestOwner {
            shutdown: Some(tx),
            thread: Some(thread),
            socket,
        }
    }

    #[test]
    fn in_process_owner_answers_initialize_and_identity() {
        use vissue_control::client::Client;

        let dir = tempfile::tempdir().unwrap();
        let owner = start_test_owner(dir.path());
        let meta = fs::metadata(owner.socket.parent().unwrap()).unwrap();
        assert_eq!(meta.permissions().mode() & 0o777, 0o700);
        let meta = fs::metadata(&owner.socket).unwrap();
        assert_eq!(meta.permissions().mode() & 0o777, 0o600);

        let mut client = Client::connect(&owner.socket).unwrap();
        let init = client
            .request(
                "initialize",
                json!({"protocolVersion": 1, "client": "test", "agent": "test-agent"}),
            )
            .unwrap();
        assert_eq!(init["identity"], "test-agent");
        assert_eq!(init["prefix"], "Software");
        let ident = client.request("identity/get", json!({})).unwrap();
        assert_eq!(ident["identity"], "test-agent");

        let err = client.request("issue/list", json!({})).unwrap_err();
        match err {
            vissue_control::Error::Rpc(e) => assert_eq!(e.code, -32601),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn in_process_owner_rejects_initialize_without_agent() {
        use vissue_control::client::Client;

        let dir = tempfile::tempdir().unwrap();
        let owner = start_test_owner(dir.path());
        let mut client = Client::connect(&owner.socket).unwrap();
        let err = client
            .request(
                "initialize",
                json!({"protocolVersion": 1, "client": "test"}),
            )
            .unwrap_err();
        match err {
            vissue_control::Error::Rpc(e) => {
                assert_eq!(e.code, -32602);
                assert_eq!(e.message, "agent is required");
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn header_framing_roundtrips_on_the_owner() {
        use vissue_control::client::Client;
        use vissue_control::Framing;

        let dir = tempfile::tempdir().unwrap();
        let owner = start_test_owner(dir.path());
        let mut client = Client::connect_with_framing(&owner.socket, Framing::Headers).unwrap();
        let init = client
            .request("initialize", json!({"protocolVersion": 1, "agent": "hdr"}))
            .unwrap();
        assert_eq!(init["identity"], "hdr");
    }

    #[test]
    fn second_bind_fails_while_lock_is_held() {
        let dir = tempfile::tempdir().unwrap();
        let owner = start_test_owner(dir.path());
        let cfg = ServeConfig {
            layout: Layout::new(dir.path().join("vault"), "Software"),
            socket: owner.socket.clone(),
            exe: None,
        };
        let err = match Owner::bind(&cfg) {
            Ok(_) => panic!("second bind succeeded"),
            Err(err) => err,
        };
        assert!(err.to_string().contains("already in use"), "{err:#}");
    }
}
