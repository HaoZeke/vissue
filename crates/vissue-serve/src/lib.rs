//! Control-socket owner: bind, detach, stop, and the initialize handshake.
//!
//! This crate does not cache a catalog. Methods other than `initialize` and
//! `identity/get` return method-not-found.

use std::path::PathBuf;

use serde::Serialize;
use vissue_core::config::Layout;

#[cfg(not(unix))]
mod stub;
#[cfg(unix)]
mod unix;

#[cfg(not(unix))]
pub use stub::{ensure_serve, invoke, socket_accepts};
#[cfg(unix)]
pub use unix::{ensure_serve, invoke, socket_accepts};

/// How the CLI (or a later TUI/HUD) wants to talk to the owner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Bind and serve until SIGINT/SIGTERM.
    Foreground,
    /// Spawn `serve --foreground` and return when the socket accepts.
    Detach,
    /// SIGTERM the owner, then SIGKILL if it stays up.
    Stop,
    /// [`Action::Stop`] then [`Action::Detach`].
    Restart,
    /// Print a snapshot. Exit 0 when live, 1 otherwise.
    Status { json: bool },
}

/// Layout plus the socket this process should own or probe.
#[derive(Debug, Clone)]
pub struct ServeConfig {
    pub layout: Layout,
    pub socket: PathBuf,
    /// Binary used by [`Action::Detach`]. `None` means `current_exe()`.
    pub exe: Option<PathBuf>,
}

/// `vissue serve status` snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Status {
    pub live: bool,
    pub pid: Option<u32>,
    pub socket: PathBuf,
    pub root: PathBuf,
    pub prefix: String,
    pub generation: u64,
    pub revision: u64,
    pub clients: u64,
}

/// Outcome of [`ensure_serve`] / a detach start.
#[derive(Debug, Clone)]
pub struct EnsureResult {
    pub ok: bool,
    pub already_running: bool,
    pub spawned: bool,
    pub pid: Option<u32>,
    pub socket: PathBuf,
    pub error: Option<String>,
}

impl EnsureResult {
    #[must_use]
    pub fn live(&self) -> bool {
        self.ok && socket_accepts(&self.socket)
    }
}

/// Methods this release actually answers.
pub const LIVE_CAPABILITIES: &[&str] = &["identity/get"];

/// Serve-local catalog revision. Stays 0 until the catalog owner lands.
pub const SERVE_REVISION: u64 = 0;

/// How long a detach parent waits for the child to accept.
pub const ACCEPT_TIMEOUT_MS: u64 = 5_000;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn live_capabilities_are_the_implemented_methods() {
        assert_eq!(LIVE_CAPABILITIES, ["identity/get"]);
        assert_eq!(SERVE_REVISION, 0);
    }

    #[test]
    fn status_json_uses_snake_case_keys() {
        let status = Status {
            live: false,
            pid: None,
            socket: PathBuf::from("/tmp/control.sock"),
            root: PathBuf::from("/tmp/tracker"),
            prefix: "Software".into(),
            generation: 0,
            revision: 0,
            clients: 0,
        };
        let value = serde_json::to_value(&status).unwrap();
        assert_eq!(value["live"], false);
        assert_eq!(value["socket"], "/tmp/control.sock");
        assert!(value.get("socket_path").is_none());
        assert_eq!(value["clients"], 0);
    }
}
