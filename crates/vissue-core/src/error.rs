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
    /// Any other failure, usually I/O or a parse problem.
    Other(anyhow::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::IssueNotFound { id } => write!(f, "issue {id} not found"),
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
