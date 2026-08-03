//! The issue heading and its logbook: parsing, accessors, and rendering.

use chrono::Local;
use serde::Serialize;
use std::collections::BTreeMap;

/// TODO keywords recognised on a heading, in org declaration order.
pub const TODO_KEYWORDS: &[&str] = &["TODO", "STARTED", "BLOCKED", "DONE", "CANCELLED"];
/// States an issue can be worked from once its blockers clear.
pub const READY_STATES: &[&str] = &["TODO", "STARTED"];
/// The `#+TODO:` line written into a fresh issues.org preamble.
pub const TODO_HEADER: &str = "#+TODO: TODO STARTED BLOCKED | DONE CANCELLED";

const PROPERTY_COLUMN: usize = 13;
const DEFAULT_PROPERTY_ORDER: &[&str] = &[
    "CREATED",
    "TYPE",
    "PARENT",
    "BLOCKED_BY",
    "DEADLINE",
    "SCHEDULED",
    "TAGS",
    "FILES",
    "VERIFY",
];

/// One line of an issue's `:LOGBOOK:` drawer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LogEntry {
    pub timestamp: String,
    pub from_state: Option<String>,
    pub to_state: Option<String>,
    pub note: Option<String>,
    /// Opaque logbook line (an org `CLOCK:` entry, say) preserved verbatim
    /// across rewrites.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw: Option<String>,
}

impl LogEntry {
    pub fn render(&self) -> String {
        if let Some(raw) = &self.raw {
            return raw.clone();
        }
        if let (Some(to), Some(from)) = (&self.to_state, &self.from_state) {
            format!("- State \"{}\" from \"{}\" {}", to, from, self.timestamp)
        } else if let Some(to) = &self.to_state {
            format!("- State \"{}\" {}", to, self.timestamp)
        } else if let Some(note) = &self.note {
            format!("- Note: \"{}\" {}", note, self.timestamp)
        } else {
            format!("- {}", self.timestamp)
        }
    }

    /// An inactive org timestamp for the current local time.
    pub fn now() -> String {
        Local::now().format("[%Y-%m-%d %a %H:%M]").to_string()
    }

    fn raw_line(line: &str) -> Self {
        Self {
            timestamp: String::new(),
            from_state: None,
            to_state: None,
            note: None,
            raw: Some(line.trim_end_matches(['\r', '\n']).to_string()),
        }
    }
}

pub(crate) fn parse_log_line(s: &str) -> LogEntry {
    // Org CLOCK lines and other non-dash drawer content survive as raw text.
    let trimmed = s.trim();
    if trimmed.to_ascii_uppercase().starts_with("CLOCK:")
        || (!trimmed.starts_with('-') && !trimmed.is_empty())
    {
        return LogEntry::raw_line(s);
    }
    let Some(body) = trimmed.strip_prefix('-').map(str::trim_start) else {
        return LogEntry::raw_line(s);
    };
    let Some(bracket_idx) = body.rfind('[') else {
        return LogEntry::raw_line(s);
    };
    let Some(rel_end) = body[bracket_idx..].find(']') else {
        return LogEntry::raw_line(s);
    };
    let end = rel_end + bracket_idx;
    let timestamp = body[bracket_idx..=end].to_string();
    let prefix = body[..bracket_idx].trim().trim_end_matches(',');
    if let Some(rest) = prefix.strip_prefix("State ") {
        let mut chunks = rest.splitn(2, " from ");
        let to_quoted = chunks.next().unwrap_or("").trim();
        let from_quoted = chunks.next().map(str::trim);
        LogEntry {
            timestamp,
            from_state: from_quoted.map(|s| s.trim_matches('"').to_string()),
            to_state: Some(to_quoted.trim_matches('"').to_string()),
            note: None,
            raw: None,
        }
    } else if let Some(rest) = prefix.strip_prefix("Note:") {
        let note = rest.trim().trim_matches('"').to_string();
        LogEntry {
            timestamp,
            from_state: None,
            to_state: None,
            note: if note.is_empty() { None } else { Some(note) },
            raw: None,
        }
    } else {
        LogEntry {
            timestamp,
            from_state: None,
            to_state: None,
            note: if prefix.is_empty() {
                None
            } else {
                Some(prefix.to_string())
            },
            raw: None,
        }
    }
}

/// One issue: a top-level org heading with a properties drawer, an optional
/// logbook, and free-form body prose.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IssueHeading {
    pub id: String,
    pub title: String,
    pub state: String,
    pub priority: char,
    pub properties: BTreeMap<String, String>,
    #[serde(skip_serializing)]
    pub property_order: Vec<String>,
    #[serde(skip_serializing)]
    pub body: String,
    pub logbook: Vec<LogEntry>,
    pub line_start: usize,
    pub line_end: usize,
}

impl IssueHeading {
    /// Ids listed in `:BLOCKED_BY:`. Commas and whitespace both separate, so
    /// values written by hand or by an importer parse the same way.
    pub fn blocked_by(&self) -> Vec<String> {
        self.properties
            .get("BLOCKED_BY")
            .map(|s| {
                s.split(|c: char| c == ',' || c.is_whitespace())
                    .map(|x| x.trim().to_string())
                    .filter(|x| !x.is_empty())
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn tags(&self) -> Vec<String> {
        self.properties
            .get("TAGS")
            .map(|s| {
                s.split([',', ':'])
                    .map(|x| x.trim().to_string())
                    .filter(|x| !x.is_empty())
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn deadline(&self) -> Option<&str> {
        self.properties.get("DEADLINE").map(|s| s.as_str())
    }

    pub fn scheduled(&self) -> Option<&str> {
        self.properties.get("SCHEDULED").map(|s| s.as_str())
    }

    pub fn parent(&self) -> Option<&str> {
        self.properties.get("PARENT").map(|s| s.as_str())
    }

    pub fn render(&self) -> String {
        let mut out = format!("* {} [#{}] {}\n", self.state, self.priority, self.title);
        out.push_str(":PROPERTIES:\n");
        out.push_str(&render_property("ID", &self.id));
        for key in self.ordered_property_keys() {
            if key != "ID" {
                if let Some(val) = self.properties.get(&key) {
                    out.push_str(&render_property(&key, val));
                }
            }
        }
        out.push_str(":END:\n");
        if !self.logbook.is_empty() {
            out.push_str(":LOGBOOK:\n");
            for entry in &self.logbook {
                out.push_str(&entry.render());
                out.push('\n');
            }
            out.push_str(":END:\n");
        }
        if !self.body.is_empty() {
            out.push('\n');
            out.push_str(&self.body);
            if !self.body.ends_with('\n') {
                out.push('\n');
            }
        }
        out
    }

    /// Keys as they appeared on disk first, then the house order, then the rest.
    /// Rewriting an issue therefore leaves a hand-arranged drawer alone.
    fn ordered_property_keys(&self) -> Vec<String> {
        let mut keys = Vec::new();
        for key in &self.property_order {
            if self.properties.contains_key(key) && !keys.contains(key) {
                keys.push(key.clone());
            }
        }
        for key in DEFAULT_PROPERTY_ORDER {
            let key = key.to_string();
            if self.properties.contains_key(&key) && !keys.contains(&key) {
                keys.push(key);
            }
        }
        for key in self.properties.keys() {
            if !keys.contains(key) {
                keys.push(key.clone());
            }
        }
        keys
    }

    /// Move to `new_state` and prepend the transition to the logbook. A
    /// transition to the current state is not recorded.
    pub fn record_state_change(&mut self, new_state: &str) {
        if self.state == new_state {
            return;
        }
        let from = Some(self.state.clone());
        self.logbook.insert(
            0,
            LogEntry {
                timestamp: LogEntry::now(),
                from_state: from,
                to_state: Some(new_state.to_string()),
                note: None,
                raw: None,
            },
        );
        self.state = new_state.to_string();
    }
}

fn render_property(key: &str, val: &str) -> String {
    let key_part = format!(":{}:", key);
    let pad = if key_part.len() < PROPERTY_COLUMN {
        " ".repeat(PROPERTY_COLUMN - key_part.len())
    } else {
        " ".to_string()
    };
    format!("{}{}{}\n", key_part, pad, val)
}

/// Today as an inactive org timestamp.
pub fn today_inactive_bracket() -> String {
    Local::now().format("[%Y-%m-%d %a]").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_heading() -> IssueHeading {
        let mut props = BTreeMap::new();
        props.insert("ID".into(), "sample-abc1".into());
        props.insert("CREATED".into(), "[2026-04-25 Sat]".into());
        props.insert("TYPE".into(), "feature".into());
        IssueHeading {
            id: "sample-abc1".into(),
            title: "Add a thing".into(),
            state: "TODO".into(),
            priority: 'A',
            properties: props,
            property_order: vec!["ID".into(), "CREATED".into(), "TYPE".into()],
            body: "Some body lines.\nWith multiple lines.".into(),
            logbook: Vec::new(),
            line_start: 4,
            line_end: 12,
        }
    }

    #[test]
    fn blocked_by_accepts_commas_spaces_and_both() {
        let mut h = sample_heading();
        for raw in [" A-1, B-2 ,, C-3 ", "A-1 B-2  C-3", "A-1, B-2 C-3"] {
            h.properties.insert("BLOCKED_BY".into(), raw.into());
            assert_eq!(h.blocked_by(), vec!["A-1", "B-2", "C-3"], "raw = {raw:?}");
        }
    }

    #[test]
    fn tags_split_on_commas_and_colons() {
        let mut h = sample_heading();
        h.properties
            .insert("TAGS".into(), "rust:perf, scaling".into());
        assert_eq!(h.tags(), vec!["rust", "perf", "scaling"]);
    }

    #[test]
    fn accessors_read_optional_properties() {
        let mut h = sample_heading();
        assert!(h.parent().is_none());
        h.properties.insert("PARENT".into(), "sample-q3xa".into());
        h.properties
            .insert("DEADLINE".into(), "<2026-05-15 Fri>".into());
        h.properties
            .insert("SCHEDULED".into(), "<2026-04-28 Mon>".into());
        assert_eq!(h.parent(), Some("sample-q3xa"));
        assert_eq!(h.deadline(), Some("<2026-05-15 Fri>"));
        assert_eq!(h.scheduled(), Some("<2026-04-28 Mon>"));
    }

    #[test]
    fn state_change_prepends_and_skips_no_ops() {
        let mut h = sample_heading();
        h.record_state_change("STARTED");
        assert_eq!(h.state, "STARTED");
        assert_eq!(h.logbook[0].from_state.as_deref(), Some("TODO"));
        h.record_state_change("DONE");
        assert_eq!(h.logbook.len(), 2);
        assert_eq!(h.logbook[0].to_state.as_deref(), Some("DONE"));
        h.record_state_change("DONE");
        assert_eq!(h.logbook.len(), 2, "no-op transition is not logged");
    }

    #[test]
    fn logbook_renders_state_transitions() {
        let entry = LogEntry {
            timestamp: "[2026-04-26 Sun 14:22]".into(),
            from_state: Some("STARTED".into()),
            to_state: Some("DONE".into()),
            note: None,
            raw: None,
        };
        assert_eq!(
            entry.render(),
            "- State \"DONE\" from \"STARTED\" [2026-04-26 Sun 14:22]"
        );
    }

    #[test]
    fn clock_lines_survive_a_state_rewrite() {
        let clock = "   CLOCK: [2026-07-26 Sun 17:40]";
        let closed = "   CLOCK: [2026-07-26 Sun 10:00]--[2026-07-26 Sun 11:00] =>  1:00";
        let state = "- State \"STARTED\" from \"TODO\" [2026-07-26 Sun 17:39]";
        assert_eq!(parse_log_line(clock).render(), clock);
        assert_eq!(parse_log_line(closed).render(), closed);
        let parsed_state = parse_log_line(state);
        assert_eq!(parsed_state.to_state.as_deref(), Some("STARTED"));
        assert!(parsed_state.raw.is_none());

        let mut h = sample_heading();
        h.logbook = vec![parsed_state, parse_log_line(clock)];
        h.record_state_change("DONE");
        let rendered = h.render();
        assert!(
            rendered.contains("CLOCK: [2026-07-26 Sun 17:40]"),
            "{rendered}"
        );
        assert!(rendered.contains("State \"DONE\""), "{rendered}");
    }

    #[test]
    fn note_lines_round_trip() {
        let parsed = parse_log_line("- Note: \"picked up after review\" [2026-04-26 Sun 09:15]");
        assert_eq!(parsed.note.as_deref(), Some("picked up after review"));
        assert_eq!(
            parsed.render(),
            "- Note: \"picked up after review\" [2026-04-26 Sun 09:15]"
        );
    }
}
