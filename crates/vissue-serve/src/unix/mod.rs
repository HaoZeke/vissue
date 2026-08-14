//! Unix owner: bind, accept, detach, stop, status.

mod bus;
mod catalog;
mod dispatch;
mod lifecycle;
mod owner;
mod watcher;

use anyhow::Result;

use crate::{Action, ServeConfig};

pub use lifecycle::{ensure_serve, socket_accepts};
pub use owner::OwnerHandle;

pub fn invoke(action: Action, cfg: &ServeConfig) -> Result<i32> {
    match action {
        Action::Foreground => {
            owner::run_foreground(cfg)?;
            Ok(0)
        }
        Action::Detach => {
            let result = lifecycle::start_detached(cfg)?;
            if !result.ok {
                if let Some(err) = &result.error {
                    eprintln!("vissue: {err}");
                }
                return Ok(1);
            }
            if result.already_running {
                let pid = result
                    .pid
                    .map(|p| p.to_string())
                    .unwrap_or_else(|| "-".into());
                println!(
                    "already running pid={pid} socket={}",
                    result.socket.display()
                );
            }
            Ok(0)
        }
        Action::Stop => lifecycle::stop(cfg),
        Action::Restart => {
            let code = lifecycle::stop(cfg)?;
            if code != 0 {
                return Ok(code);
            }
            invoke(Action::Detach, cfg)
        }
        Action::Status { json } => {
            let status = lifecycle::status(cfg);
            lifecycle::print_status(&status, json)?;
            Ok(i32::from(!status.live))
        }
    }
}
