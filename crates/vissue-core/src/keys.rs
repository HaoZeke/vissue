//! action catalog and keys.toml overlay.
//!
//! Defaults live in code. Operator diffs live in `$VISSUE_KEYS` or
//! `~/.config/vissue/keys.toml`. Invalid overlay is refused.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Dotted action id. HUD-shaped, remappable except reserved chords.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ActionId {
    ListDown,
    ListUp,
    ListSelect,
    ListDone,
    PaneReady,
    PaneList,
    PaneClaims,
    PaneAgenda,
    PaneSearch,
    PaneNext,
    DetailCycle,
    ProjectCycle,
    Search,
    Add,
    Claim,
    Note,
    StateCycle,
    ConfirmDone,
    ConfirmCancel,
    Open,
    CopyId,
    Reload,
    Help,
}

impl ActionId {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ListDown => "list.down",
            Self::ListUp => "list.up",
            Self::ListSelect => "list.select",
            Self::ListDone => "list.done",
            Self::PaneReady => "pane.ready",
            Self::PaneList => "pane.list",
            Self::PaneClaims => "pane.claims",
            Self::PaneAgenda => "pane.agenda",
            Self::PaneSearch => "pane.search",
            Self::PaneNext => "pane.next",
            Self::DetailCycle => "detail.cycle",
            Self::ProjectCycle => "project.cycle",
            Self::Search => "board.search",
            Self::Add => "issue.add",
            Self::Claim => "issue.claim",
            Self::Note => "issue.note",
            Self::StateCycle => "issue.state",
            Self::ConfirmDone => "issue.done",
            Self::ConfirmCancel => "issue.cancel",
            Self::Open => "issue.open",
            Self::CopyId => "issue.copy",
            Self::Reload => "board.reload",
            Self::Help => "board.help",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        ALL.iter().find(|a| a.as_str() == raw).copied()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    Global,
    Board,
}

impl Scope {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Global => "global",
            Self::Board => "board",
        }
    }
}

const ALL: &[ActionId] = &[
    ActionId::ListDown,
    ActionId::ListUp,
    ActionId::ListSelect,
    ActionId::ListDone,
    ActionId::PaneReady,
    ActionId::PaneList,
    ActionId::PaneClaims,
    ActionId::PaneAgenda,
    ActionId::PaneSearch,
    ActionId::PaneNext,
    ActionId::DetailCycle,
    ActionId::ProjectCycle,
    ActionId::Search,
    ActionId::Add,
    ActionId::Claim,
    ActionId::Note,
    ActionId::StateCycle,
    ActionId::ConfirmDone,
    ActionId::ConfirmCancel,
    ActionId::Open,
    ActionId::CopyId,
    ActionId::Reload,
    ActionId::Help,
];

/// One catalog row. Defaults stay in this table.
#[derive(Debug, Clone, Copy)]
pub struct ActionRow {
    pub id: ActionId,
    pub scope: Scope,
    pub default: &'static str,
    pub remappable: bool,
}

const CATALOG: &[ActionRow] = &[
    ActionRow {
        id: ActionId::ListDown,
        scope: Scope::Board,
        default: "j",
        remappable: true,
    },
    ActionRow {
        id: ActionId::ListUp,
        scope: Scope::Board,
        default: "k",
        remappable: true,
    },
    ActionRow {
        id: ActionId::ListSelect,
        scope: Scope::Board,
        default: "enter",
        remappable: false,
    },
    ActionRow {
        id: ActionId::ListDone,
        scope: Scope::Board,
        default: "space",
        remappable: true,
    },
    ActionRow {
        id: ActionId::PaneReady,
        scope: Scope::Board,
        default: "1",
        remappable: true,
    },
    ActionRow {
        id: ActionId::PaneList,
        scope: Scope::Board,
        default: "2",
        remappable: true,
    },
    ActionRow {
        id: ActionId::PaneClaims,
        scope: Scope::Board,
        default: "3",
        remappable: true,
    },
    ActionRow {
        id: ActionId::PaneAgenda,
        scope: Scope::Board,
        default: "4",
        remappable: true,
    },
    ActionRow {
        id: ActionId::PaneSearch,
        scope: Scope::Board,
        default: "5",
        remappable: true,
    },
    ActionRow {
        id: ActionId::PaneNext,
        scope: Scope::Board,
        default: "tab",
        remappable: false,
    },
    ActionRow {
        id: ActionId::DetailCycle,
        scope: Scope::Board,
        default: "enter",
        remappable: false,
    },
    ActionRow {
        id: ActionId::ProjectCycle,
        scope: Scope::Board,
        default: "p",
        remappable: true,
    },
    ActionRow {
        id: ActionId::Search,
        scope: Scope::Board,
        default: "/",
        remappable: true,
    },
    ActionRow {
        id: ActionId::Add,
        scope: Scope::Board,
        default: "a",
        remappable: true,
    },
    ActionRow {
        id: ActionId::Claim,
        scope: Scope::Board,
        default: "c",
        remappable: true,
    },
    ActionRow {
        id: ActionId::Note,
        scope: Scope::Board,
        default: "n",
        remappable: true,
    },
    ActionRow {
        id: ActionId::StateCycle,
        scope: Scope::Board,
        default: "s",
        remappable: true,
    },
    ActionRow {
        id: ActionId::ConfirmDone,
        scope: Scope::Board,
        default: "D",
        remappable: true,
    },
    ActionRow {
        id: ActionId::ConfirmCancel,
        scope: Scope::Board,
        default: "X",
        remappable: true,
    },
    ActionRow {
        id: ActionId::Open,
        scope: Scope::Board,
        default: "o",
        remappable: true,
    },
    ActionRow {
        id: ActionId::CopyId,
        scope: Scope::Board,
        default: "y",
        remappable: true,
    },
    ActionRow {
        id: ActionId::Reload,
        scope: Scope::Board,
        default: "R",
        remappable: true,
    },
    ActionRow {
        id: ActionId::Help,
        scope: Scope::Global,
        default: "?",
        remappable: false,
    },
];

/// Reserved chords the overlay may not steal.
const RESERVED: &[&str] = &["esc", "enter", "tab", "?"];

/// Resolved map: chord -> action (board scope).
#[derive(Debug, Clone)]
pub struct KeyMap {
    by_chord: BTreeMap<String, ActionId>,
    pub leader: Option<char>,
    pub leader_timeout_ms: u64,
    pub overlay_error: Option<String>,
}

impl Default for KeyMap {
    fn default() -> Self {
        Self::from_defaults()
    }
}

impl KeyMap {
    pub fn from_defaults() -> Self {
        let mut by_chord = BTreeMap::new();
        for row in CATALOG {
            by_chord.insert(row.default.to_string(), row.id);
        }
        Self {
            by_chord,
            leader: None,
            leader_timeout_ms: 800,
            overlay_error: None,
        }
    }

    pub fn load() -> Self {
        let path = overlay_path();
        match path {
            Some(p) if p.is_file() => match load_overlay(&p) {
                Ok(map) => map,
                Err(err) => {
                    let mut map = Self::from_defaults();
                    map.overlay_error = Some(err);
                    map
                }
            },
            _ => Self::from_defaults(),
        }
    }

    pub fn get(&self, chord: &str) -> Option<ActionId> {
        self.by_chord.get(chord).copied()
    }

    pub fn help_lines(&self) -> Vec<String> {
        CATALOG
            .iter()
            .map(|row| {
                let chord = self
                    .by_chord
                    .iter()
                    .find(|(_, id)| **id == row.id)
                    .map(|(c, _)| c.as_str())
                    .unwrap_or(row.default);
                format!("{chord:8}  {}", row.id.as_str())
            })
            .collect()
    }

    pub fn occupancy(&self) -> Vec<(String, String)> {
        self.by_chord
            .iter()
            .map(|(c, id)| (c.clone(), id.as_str().to_string()))
            .collect()
    }

    /// One line per action: scope, id, resolved chord.
    pub fn table_lines(&self) -> Vec<String> {
        CATALOG
            .iter()
            .map(|row| {
                let chord = self
                    .by_chord
                    .iter()
                    .find(|(_, id)| **id == row.id)
                    .map(|(c, _)| c.as_str())
                    .unwrap_or(row.default);
                format!("{:<8} {:<16} {chord}", row.scope.as_str(), row.id.as_str())
            })
            .collect()
    }
}

fn overlay_path() -> Option<PathBuf> {
    if let Ok(raw) = std::env::var("VISSUE_KEYS") {
        let t = raw.trim();
        if !t.is_empty() {
            return Some(PathBuf::from(t));
        }
    }
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
    Some(base.join("vissue/keys.toml"))
}

fn load_overlay(path: &Path) -> Result<KeyMap, String> {
    let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let value: toml::Value = text.parse().map_err(|e| format!("keys.toml: {e}"))?;
    let mut map = KeyMap::from_defaults();
    if let Some(leader) = value.get("leader").and_then(|v| v.as_str()) {
        let mut chars = leader.chars();
        let ch = chars.next();
        if ch.is_none() || chars.next().is_some() {
            return Err("leader must be one character".into());
        }
        map.leader = ch;
    }
    if let Some(ms) = value.get("leader_timeout_ms").and_then(|v| v.as_integer()) {
        if ms > 0 {
            map.leader_timeout_ms = ms as u64;
        }
    }
    let table = value.get("board").and_then(|v| v.as_table());
    if let Some(table) = table {
        let mut pending: Vec<(ActionId, String)> = Vec::new();
        for (id, chord) in table {
            let Some(action) = ActionId::parse(id) else {
                return Err(format!("unknown action {id}"));
            };
            let Some(row) = CATALOG.iter().find(|r| r.id == action) else {
                return Err(format!("unknown action {id}"));
            };
            let Some(chord) = chord.as_str() else {
                return Err(format!("{id} chord must be a string"));
            };
            if !row.remappable {
                return Err(format!("{id} is not remappable"));
            }
            if RESERVED.contains(&chord.to_ascii_lowercase().as_str()) {
                return Err(format!("cannot steal reserved chord {chord}"));
            }
            pending.push((action, chord.to_string()));
        }
        for (action, _) in &pending {
            map.by_chord.retain(|_, id| id != action);
        }
        for (action, chord) in pending {
            if let Some(prev) = map.by_chord.insert(chord.clone(), action) {
                return Err(format!("chord {chord} already bound to {}", prev.as_str()));
            }
        }
    }
    Ok(map)
}

/// Map a typed character (no modifiers) or a named key to a chord token.
pub fn chord_from_char(c: char) -> String {
    c.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_action_has_a_unique_id() {
        let mut seen = BTreeMap::new();
        for row in CATALOG {
            assert!(
                seen.insert(row.id.as_str(), row.id).is_none(),
                "duplicate {}",
                row.id.as_str()
            );
            assert_eq!(ActionId::parse(row.id.as_str()), Some(row.id));
        }
        assert_eq!(seen.len(), ALL.len());
    }

    #[test]
    fn defaults_resolve_j_and_n() {
        let map = KeyMap::from_defaults();
        assert_eq!(map.get("j"), Some(ActionId::ListDown));
        assert_eq!(map.get("n"), Some(ActionId::Note));
        assert_eq!(map.get("?"), Some(ActionId::Help));
    }

    #[test]
    fn overlay_rejects_reserved_and_unknown() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("keys.toml");
        std::fs::write(&path, "[board]\n\"issue.note\" = \"esc\"\n").unwrap();
        let err = load_overlay(&path).unwrap_err();
        assert!(err.contains("reserved"), "{err}");
        std::fs::write(&path, "[board]\n\"no.such\" = \"z\"\n").unwrap();
        let err = load_overlay(&path).unwrap_err();
        assert!(err.contains("unknown"), "{err}");
    }

    #[test]
    fn overlay_remaps_list_down() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("keys.toml");
        std::fs::write(
            &path,
            "leader = \";\"\n[board]\n\"list.down\" = \"n\"\n\"issue.note\" = \"leader+n\"\n",
        )
        .unwrap();
        let map = load_overlay(&path).unwrap();
        assert_eq!(map.leader, Some(';'));
        assert_eq!(map.get("n"), Some(ActionId::ListDown));
        assert_eq!(map.get("j"), None);
    }
}
