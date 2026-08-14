//! Rofi dmenu over the tracker. This is the default `vissue hud`.
//!
//! The seat theme (font, colours, window chrome) comes from the user's
//! rofi config. This module only feeds rows and reads the exit code.

use anyhow::{bail, Context, Result};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use vissue_core::config::Layout;
use vissue_core::ops::{self, CreateOpts};
use vissue_core::{agent, report};

/// How the picker is populated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Ready,
    List,
    Claims,
    Stale,
    New,
}

impl Mode {
    pub fn parse(raw: &str) -> Result<Self> {
        match raw {
            "ready" => Ok(Self::Ready),
            "list" | "all" => Ok(Self::List),
            "claims" => Ok(Self::Claims),
            "stale" => Ok(Self::Stale),
            "new" => Ok(Self::New),
            other => bail!("unknown hud mode {other:?}; use ready, list, claims, stale, or new"),
        }
    }

    fn prompt(self) -> &'static str {
        match self {
            Self::Ready => "vissue ready",
            Self::List => "vissue list",
            Self::Claims => "vissue claims",
            Self::Stale => "vissue stale",
            Self::New => "vissue new",
        }
    }
}

/// What the user asked for with Return or a custom key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Open,
    Claim,
    Note,
}

/// Map rofi's dmenu exit status. 0 is Return. 10/11 are kb-custom-1/2.
pub fn action_from_status(code: i32) -> Option<Action> {
    match code {
        0 => Some(Action::Open),
        10 => Some(Action::Claim),
        11 => Some(Action::Note),
        _ => None,
    }
}

/// First whitespace-separated field is the issue id.
pub fn id_from_row(line: &str) -> Option<&str> {
    let id = line.split_whitespace().next()?;
    if id.is_empty() {
        return None;
    }
    Some(id)
}

/// How to spawn rofi. Tests inject a fake binary.
pub struct RofiOpts {
    pub layout: Layout,
    pub mode: Mode,
    pub bin: PathBuf,
}

impl RofiOpts {
    pub fn from_env(layout: Layout, mode: Mode) -> Result<Self> {
        Ok(Self {
            layout,
            mode,
            bin: resolve_rofi_bin()?,
        })
    }
}

/// Run the picker and apply the chosen action.
pub fn run(opts: RofiOpts) -> Result<i32> {
    if opts.mode == Mode::New {
        return run_new(&opts);
    }
    let listing = listing(&opts.layout, opts.mode)?;
    if listing.trim().is_empty() {
        notify(
            &format!("vissue ({})", opts.mode.prompt()),
            "no issues found",
        );
        return Ok(0);
    }
    let (chosen, status) = dmenu(&opts.bin, opts.mode.prompt(), &listing, true)?;
    let Some(action) = action_from_status(status) else {
        return Ok(0);
    };
    let Some(id) = chosen.as_deref().and_then(id_from_row) else {
        return Ok(0);
    };
    match action {
        Action::Open => open_heading(&opts.layout, id)?,
        Action::Claim => {
            let out = ops::claim(&opts.layout, id, false)?;
            notify("vissue", out.trim());
        }
        Action::Note => {
            let (text, note_status) = dmenu(&opts.bin, "note", "", false)?;
            if action_from_status(note_status).is_none() {
                return Ok(0);
            }
            let text = text.unwrap_or_default();
            if text.trim().is_empty() {
                return Ok(0);
            }
            let out = ops::note(&opts.layout, id, text.trim())?;
            notify("vissue", out.trim());
        }
    }
    Ok(0)
}

fn listing(layout: &Layout, mode: Mode) -> Result<String> {
    match mode {
        Mode::Ready => report::ready(layout, None),
        Mode::List => report::list(layout, None, None, false),
        Mode::Claims => report::claims(layout, None, None, false),
        Mode::Stale => report::stale(layout, 30, None),
        Mode::New => Ok(String::new()),
    }
}

fn run_new(opts: &RofiOpts) -> Result<i32> {
    let projects = vissue_core::store::list_projects(&opts.layout)?;
    if projects.is_empty() {
        notify("vissue", "no projects under the tracker root");
        return Ok(0);
    }
    let (project, status) = dmenu(&opts.bin, "project", &projects.join("\n"), false)?;
    if action_from_status(status).is_none() {
        return Ok(0);
    }
    let Some(project) = project.filter(|p| !p.trim().is_empty()) else {
        return Ok(0);
    };
    let project = project.trim().to_string();
    let (title, status) = dmenu(&opts.bin, "title", "", false)?;
    if action_from_status(status).is_none() {
        return Ok(0);
    }
    let Some(title) = title.filter(|t| !t.trim().is_empty()) else {
        return Ok(0);
    };
    let out = ops::create(
        &opts.layout,
        &project,
        title.trim(),
        CreateOpts {
            quiet: true,
            ..Default::default()
        },
    )?;
    notify("vissue", &format!("created {}", out.trim()));
    Ok(0)
}

/// Spawn rofi -dmenu. `custom_keys` installs Alt+c / Alt+n.
fn dmenu(
    bin: &Path,
    prompt: &str,
    stdin_text: &str,
    custom_keys: bool,
) -> Result<(Option<String>, i32)> {
    let mut cmd = Command::new(bin);
    cmd.arg("-dmenu")
        .arg("-i")
        .arg("-p")
        .arg(prompt)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit());
    if custom_keys {
        cmd.arg("-no-custom")
            .arg("-kb-custom-1")
            .arg("Alt+c")
            .arg("-kb-custom-2")
            .arg("Alt+n")
            .arg("-mesg")
            .arg("Ret open   Alt+c claim   Alt+n note");
    }
    let mut child = cmd
        .spawn()
        .with_context(|| format!("spawn {}", bin.display()))?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(stdin_text.as_bytes())?;
        if !stdin_text.is_empty() && !stdin_text.ends_with('\n') {
            stdin.write_all(b"\n")?;
        }
    }
    let out = child.wait_with_output().context("wait for rofi")?;
    let status = out.status.code().unwrap_or(1);
    let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
    let chosen = if text.is_empty() { None } else { Some(text) };
    Ok((chosen, status))
}

fn open_heading(layout: &Layout, id: &str) -> Result<()> {
    let detail = agent::show_json(layout, id)?;
    let file = detail
        .get("file")
        .and_then(|v| v.as_str())
        .with_context(|| format!("show {id} had no file"))?;
    // `file` is `path:start-end`.
    let (path, start) =
        split_file_range(file).with_context(|| format!("cannot parse file range {file}"))?;
    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".into());
    let term = std::env::var("TERMINAL")
        .ok()
        .filter(|s| !s.trim().is_empty());
    let mut cmd = if let Some(term) = term {
        let mut c = Command::new(term);
        c.arg("-e").arg(&editor).arg(format!("+{start}")).arg(path);
        c
    } else {
        let mut c = Command::new(&editor);
        c.arg(format!("+{start}")).arg(path);
        c
    };
    cmd.spawn()
        .with_context(|| format!("open {id} in {editor}"))?;
    Ok(())
}

/// Split `path:12-40` into (`path`, 12). The path may contain colons.
pub fn split_file_range(file: &str) -> Option<(&str, usize)> {
    let dash = file.rfind('-')?;
    let colon = file[..dash].rfind(':')?;
    let start = file[colon + 1..dash].parse().ok()?;
    Some((&file[..colon], start))
}

fn resolve_rofi_bin() -> Result<PathBuf> {
    for var in ["VISSUE_ROFI", "ROFI"] {
        if let Ok(raw) = std::env::var(var) {
            let t = raw.trim();
            if !t.is_empty() {
                return Ok(PathBuf::from(t));
            }
        }
    }
    if let Some(path) = which("rofi") {
        return Ok(path);
    }
    bail!("rofi is not installed")
}

fn which(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join(name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

fn notify(title: &str, body: &str) {
    let _ = Command::new("notify-send").arg(title).arg(body).status();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_is_the_first_field() {
        assert_eq!(
            id_from_row("atlas-1a2b             STARTED   [#A]  Parse"),
            Some("atlas-1a2b")
        );
        assert_eq!(id_from_row("   "), None);
    }

    #[test]
    fn return_and_custom_keys_map() {
        assert_eq!(action_from_status(0), Some(Action::Open));
        assert_eq!(action_from_status(10), Some(Action::Claim));
        assert_eq!(action_from_status(11), Some(Action::Note));
        assert_eq!(action_from_status(1), None);
        assert_eq!(action_from_status(255), None);
    }

    #[test]
    fn file_range_keeps_colons_in_the_path() {
        let (path, start) = split_file_range("/tmp/foo:bar/issues.org:12-40").unwrap();
        assert_eq!(path, "/tmp/foo:bar/issues.org");
        assert_eq!(start, 12);
    }

    #[test]
    fn mode_aliases() {
        assert_eq!(Mode::parse("all").unwrap(), Mode::List);
        assert_eq!(Mode::parse("ready").unwrap(), Mode::Ready);
        assert!(Mode::parse("nope").is_err());
    }
}
