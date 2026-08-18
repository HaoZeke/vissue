//! Matchable errors so callers do not parse [`std::fmt::Display`] text.

use std::fmt;

/// Library result using [`Error`].
pub type Result<T> = std::result::Result<T, Error>;

/// Recoverable catalog and mutation failures with a stable shape.
#[derive(Debug)]
pub enum Error {
    /// No heading in the corpus carries this id.
    IssueNotFound {
        /// The id that was looked up.
        id: String,
    },
    /// The same id exists under more than one distinct tracker layout.
    DuplicateId {
        /// The id that was found twice.
        id: String,
        /// The `issues.org` files that define it.
        paths: Vec<std::path::PathBuf>,
    },
    /// Another identity already holds the issue.
    ClaimConflict {
        /// The issue that is already claimed.
        id: String,
        /// Who holds it.
        holder: String,
        /// When the claim was stamped, if the heading recorded it.
        claimed_at: Option<String>,
    },
    /// The edge would close a loop in the blocker graph.
    BlockerCycle {
        /// The prospective prerequisite.
        blocker: String,
        /// The issue that would wait on it.
        issue: String,
    },
    /// The issue is in a state that cannot be claimed.
    InvalidState {
        /// The issue that was refused.
        id: String,
        /// The heading state at the time of the refusal.
        state: String,
    },
    /// A write named a last-seen state or generation that no longer holds.
    StaleWrite {
        /// The issue that was refused.
        id: String,
        /// State the caller required, when `--if-state` was set.
        expected_state: Option<String>,
        /// Heading state at the refusal.
        actual_state: String,
        /// Generation the caller required, when `--if-gen` was set.
        expected_gen: Option<u64>,
        /// Corpus generation at the refusal.
        actual_gen: Option<u64>,
    },
    /// A second terminal disagrees with the one already on the heading.
    TerminalConflict {
        /// The issue that already has a terminal state.
        id: String,
        /// The terminal already written.
        held: String,
        /// The terminal the caller asked for.
        attempted: String,
    },
    /// Any other failure, usually I/O or a parse problem.
    Other(anyhow::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::IssueNotFound { id } => write!(f, "issue {id} not found"),
            Error::DuplicateId { id, paths } => {
                let listed = paths
                    .iter()
                    .map(|p| p.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                write!(f, "id {id} is defined in more than one tracker: {listed}")
            }
            Error::ClaimConflict {
                id,
                holder,
                claimed_at,
            } => write!(
                f,
                "{id} is claimed by {holder} since {}; pass --force to take it over",
                claimed_at.as_deref().unwrap_or("an unknown time")
            ),
            Error::BlockerCycle { blocker, issue } if blocker == issue => {
                write!(f, "issue {issue} cannot block itself")
            }
            Error::BlockerCycle { blocker, issue } => {
                write!(
                    f,
                    "adding {blocker} -> {issue} would create a blocker cycle"
                )
            }
            Error::InvalidState { id, state } => {
                write!(f, "{id} is already {state}; cannot claim")
            }
            Error::StaleWrite {
                id,
                expected_state,
                actual_state,
                expected_gen,
                actual_gen,
            } => match (expected_state, expected_gen) {
                (Some(want), Some(want_gen)) => write!(
                    f,
                    "{id} is {actual_state} at generation {}, not {want} at {want_gen}; write refused",
                    actual_gen.map_or_else(|| "?".into(), |g| g.to_string())
                ),
                (Some(want), None) => {
                    write!(f, "{id} is {actual_state}, not {want}; write refused")
                }
                (None, Some(want_gen)) => write!(
                    f,
                    "{id} generation is {}, not {want_gen}; write refused",
                    actual_gen.map_or_else(|| "?".into(), |g| g.to_string())
                ),
                (None, None) => write!(f, "{id} write refused: stale"),
            },
            Error::TerminalConflict {
                id,
                held,
                attempted,
            } => write!(
                f,
                "{id} already closed as {held}; {attempted} kept as a sibling"
            ),
            Error::Other(err) => write!(f, "{err}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Other(err) => Some(err.as_ref()),
            _ => None,
        }
    }
}

impl From<anyhow::Error> for Error {
    fn from(err: anyhow::Error) -> Self {
        match err.downcast::<Error>() {
            Ok(typed) => typed,
            Err(other) => Error::Other(other),
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::Other(err.into())
    }
}

impl From<fmt::Error> for Error {
    fn from(err: fmt::Error) -> Self {
        Error::Other(err.into())
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Error::Other(err.into())
    }
}
