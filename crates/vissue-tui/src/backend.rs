//! Sync board facade shared by the file-backed and socket-backed clients.

use vissue_core::config::Layout;
use vissue_core::error::Error;
use vissue_core::views::{
    AgendaRow, ClaimRow, Excerpt, IssueDetail, ListQuery, RelatedHit, SearchHit, TreeNode,
};

/// Which store the board is talking to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendKind {
    Core,
    Control,
}

/// One page of list/ready rows.
#[derive(Debug, Clone, Default)]
pub struct ListPage {
    pub issues: Vec<vissue_core::views::IssueRow>,
    pub total: u64,
    pub matched: u64,
    pub revision: u64,
    pub generation: u64,
    pub unchanged: bool,
}

/// Outcome of claim, note, or update.
#[derive(Debug, Clone)]
pub struct MutResult {
    pub ok: bool,
    pub report: String,
    pub issue: Option<IssueDetail>,
    pub revision: u64,
    pub generation: u64,
}

/// Fields `issue/update` accepts.
#[derive(Debug, Clone, Default)]
pub struct UpdateReq {
    pub id: String,
    pub state: Option<String>,
    pub priority: Option<char>,
    pub block: Option<String>,
    pub unblock: Option<String>,
}

/// Drops `since_revision` for one fetch after attach.
#[derive(Debug)]
pub struct SinceGate {
    skip_once: std::sync::atomic::AtomicBool,
}

impl SinceGate {
    /// After `initialize`, the next list must not send a core generation.
    pub fn after_attach() -> Self {
        Self {
            skip_once: std::sync::atomic::AtomicBool::new(true),
        }
    }

    pub fn next(&self, revision: u64) -> Option<u64> {
        if self
            .skip_once
            .swap(false, std::sync::atomic::Ordering::SeqCst)
        {
            None
        } else if revision > 0 {
            Some(revision)
        } else {
            None
        }
    }
}

/// Read and mutate the board. Implementations are `CoreBackend` and
/// `ControlBackend`.
pub trait BoardBackend: Send + Sync {
    fn layout(&self) -> &Layout;
    fn generation(&self) -> u64;
    fn revision(&self) -> u64;
    fn live(&self) -> BackendKind;
    fn identity(&self) -> &str;

    fn list(&self, q: ListQuery) -> Result<ListPage, Error>;
    fn ready(&self, project: Option<&str>) -> Result<ListPage, Error>;
    fn get(&self, id: &str) -> Result<IssueDetail, Error>;
    fn excerpt(&self, id: &str) -> Result<Excerpt, Error>;
    fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchHit>, Error>;
    fn claims(&self, holder: Option<&str>, project: Option<&str>) -> Result<Vec<ClaimRow>, Error>;
    fn agenda(&self, days: i64, project: Option<&str>) -> Result<Vec<AgendaRow>, Error>;
    fn tree(&self, id: &str) -> Result<TreeNode, Error>;
    fn related(&self, id: &str, depth: usize, limit: usize) -> Result<Vec<RelatedHit>, Error>;
    fn projects(&self) -> Result<Vec<String>, Error>;
    fn claim(&self, id: &str, force: bool) -> Result<MutResult, Error>;
    fn note(&self, id: &str, text: &str) -> Result<MutResult, Error>;
    fn update(&self, req: UpdateReq) -> Result<MutResult, Error>;
    fn open(&self, id: &str) -> Result<IssueDetail, Error>;

    /// Core: wait on the file generation. Control: wait for `vault/changed`.
    fn wait(&self, last: u64, timeout_ms: u64) -> Result<u64, Error>;

    /// Last `since_revision` sent on list/ready. `None` means the field was
    /// omitted. Default is "not recorded".
    fn last_since_revision(&self) -> Option<Option<u64>> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::SinceGate;

    #[test]
    fn after_attach_first_list_omits_since_revision() {
        let gate = SinceGate::after_attach();
        assert_eq!(gate.next(7), None);
        assert_eq!(gate.next(7), Some(7));
        assert_eq!(gate.next(8), Some(8));
    }

    #[test]
    fn a_zero_revision_never_sends_since() {
        let gate = SinceGate::after_attach();
        assert_eq!(gate.next(0), None);
        assert_eq!(gate.next(0), None);
    }
}
