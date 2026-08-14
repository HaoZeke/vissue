//! Summonable iced palette over ready and search.
//!
//! First paint always goes through [`vissue_tui::CoreBackend`]. Live updates
//! attach to `vissue serve` unless `--offline`. A root/prefix mismatch stays
//! on core so mutations never hit the wrong vault.

pub mod attach;
pub mod cli;
pub mod detach;
pub mod fuzzy;
pub mod log;
pub mod palette;
pub mod summon;
pub mod theme;
pub mod wire;

pub mod app;
pub mod view;

pub use cli::{run_cli, HudCli};
pub use palette::Palette;
pub use summon::{parse_request, sanitize_token, SummonAction, SummonRequest};

#[cfg(test)]
pub(crate) fn env_lock() -> std::sync::MutexGuard<'static, ()> {
    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}
