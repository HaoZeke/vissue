//! Argument schemas for the MCP tool surface.

use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Deserialize, JsonSchema)]
pub struct ProjectArgs {
    /// Project name. Omit to cover every project.
    pub project: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct ListArgs {
    /// Project name. Omit to cover every project.
    pub project: Option<String>,
    /// State filter: TODO, STARTED, BLOCKED, DONE, or CANCELLED.
    pub state: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct IdArgs {
    /// Issue id.
    pub issue_id: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct CreateArgs {
    /// Project name.
    pub project: String,
    /// One-line title.
    pub title: String,
    /// Priority cookie: A, B, or C.
    pub priority: Option<String>,
    /// Type tag such as feature, bug, task, chore, or plan.
    pub issue_type: Option<String>,
    /// Comma- or colon-separated tags.
    pub tags: Option<String>,
    /// Parent id for a plan or spec hierarchy.
    pub parent: Option<String>,
    /// Body prose written under the heading.
    pub body: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct UpdateArgs {
    /// Issue id to update.
    pub issue_id: String,
    /// New state: TODO, STARTED, BLOCKED, DONE, or CANCELLED.
    pub state: Option<String>,
    /// New priority cookie: A, B, or C.
    pub priority: Option<String>,
    /// Add a blocker edge.
    pub block: Option<String>,
    /// Remove a blocker edge.
    pub unblock: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct CountArgs {
    /// Project name. Omit to cover every project.
    pub project: Option<String>,
    /// State filter.
    pub state: Option<String>,
    /// Count only actionable issues.
    pub ready: Option<bool>,
}

#[derive(Deserialize, JsonSchema)]
pub struct SearchArgs {
    /// Case-insensitive substring.
    pub query: String,
    /// Maximum rows returned.
    pub limit: Option<usize>,
}

#[derive(Deserialize, JsonSchema)]
pub struct TreeArgs {
    /// Root issue id.
    pub issue_id: String,
    /// Output format: ascii or dot.
    pub format: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct MirrorArgs {
    /// Projects to include. Omit to cover every project.
    pub projects: Option<Vec<String>>,
    /// Output format: org or markdown.
    pub format: Option<String>,
    /// Include only this state.
    pub state: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct EventsArgs {
    /// Only events with a sequence above this value.
    pub since: Option<u64>,
    /// Maximum events returned.
    pub limit: Option<usize>,
}

#[derive(Deserialize, JsonSchema)]
pub struct PingArgs {
    /// Note recorded on the event.
    pub detail: Option<String>,
}

/// Parse a single-character priority cookie from the wire representation.
pub fn priority_char(raw: Option<&String>) -> Option<char> {
    raw.and_then(|s| s.trim().chars().next())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn priority_takes_the_first_character() {
        assert_eq!(priority_char(Some(&"A".to_string())), Some('A'));
        assert_eq!(priority_char(Some(&" b".to_string())), Some('b'));
        assert_eq!(priority_char(Some(&String::new())), None);
        assert_eq!(priority_char(None), None);
    }
}
