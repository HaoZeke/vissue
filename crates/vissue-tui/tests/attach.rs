//! Attach story: `--offline` never connects; first list after init drops since.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use vissue_core::config::Layout;
use vissue_core::views::ListQuery;
use vissue_serve::ServeConfig;
use vissue_tui::attach::{try_attach, AttachFail, AttachHooks, AttachOutcome, ServeStatus};
use vissue_tui::backend::{BoardBackend, ListPage, MutResult, UpdateReq};
use vissue_tui::{BackendKind, SinceGate};

struct CountingHooks {
    probes: AtomicUsize,
    ensures: AtomicUsize,
    connects: AtomicUsize,
}

impl CountingHooks {
    fn new() -> Self {
        Self {
            probes: AtomicUsize::new(0),
            ensures: AtomicUsize::new(0),
            connects: AtomicUsize::new(0),
        }
    }
}

static COUNTS: Mutex<Option<CountingHooks>> = Mutex::new(None);

fn bump_probe(_: &Path) -> bool {
    COUNTS
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .probes
        .fetch_add(1, Ordering::SeqCst);
    panic!("offline must not probe");
}

fn bump_ensure(_: &ServeConfig) -> Result<vissue_serve::EnsureResult, String> {
    COUNTS
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .ensures
        .fetch_add(1, Ordering::SeqCst);
    panic!("offline must not ensure");
}

fn bump_connect(_: &Path, _: &Layout, _: &str) -> Result<Box<dyn BoardBackend>, AttachFail> {
    COUNTS
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .connects
        .fetch_add(1, Ordering::SeqCst);
    panic!("offline must not connect");
}

#[test]
fn offline_never_touches_the_socket() {
    *COUNTS.lock().unwrap() = Some(CountingHooks::new());
    let layout = Layout::new("/tmp/vissue-tui-offline", "Software");
    let hooks = AttachHooks {
        probe: bump_probe,
        ensure: bump_ensure,
        connect: bump_connect,
    };
    let outcome = try_attach(
        &layout,
        &PathBuf::from("/tmp/does-not-exist.sock"),
        "agent",
        true,
        &hooks,
    );
    match outcome {
        AttachOutcome::Stay {
            status: ServeStatus::Offline,
            ..
        } => {}
        _ => panic!("expected stay offline"),
    }
}

/// Records `since_revision` the way `ControlBackend` does after initialize.
struct RecordingBackend {
    gate: SinceGate,
    last: Mutex<Option<Option<u64>>>,
    revision: u64,
    layout: Layout,
}

impl RecordingBackend {
    fn after_initialize(revision: u64) -> Self {
        Self {
            gate: SinceGate::after_attach(),
            last: Mutex::new(None),
            revision,
            layout: Layout::new("/tmp/vissue-tui-record", "Software"),
        }
    }
}

impl BoardBackend for RecordingBackend {
    fn layout(&self) -> &Layout {
        &self.layout
    }
    fn generation(&self) -> u64 {
        3
    }
    fn revision(&self) -> u64 {
        self.revision
    }
    fn live(&self) -> BackendKind {
        BackendKind::Control
    }
    fn identity(&self) -> &str {
        "tui"
    }
    fn list(&self, _q: ListQuery) -> Result<ListPage, vissue_core::error::Error> {
        let since = self.gate.next(self.revision);
        *self.last.lock().unwrap() = Some(since);
        Ok(ListPage {
            revision: self.revision,
            generation: 3,
            ..ListPage::default()
        })
    }
    fn ready(&self, _project: Option<&str>) -> Result<ListPage, vissue_core::error::Error> {
        self.list(ListQuery {
            ready: true,
            ..ListQuery::default()
        })
    }
    fn get(&self, _id: &str) -> Result<vissue_core::views::IssueDetail, vissue_core::error::Error> {
        unimplemented!()
    }
    fn excerpt(&self, _id: &str) -> Result<vissue_core::views::Excerpt, vissue_core::error::Error> {
        unimplemented!()
    }
    fn search(
        &self,
        _q: &str,
        _n: usize,
    ) -> Result<Vec<vissue_core::views::SearchHit>, vissue_core::error::Error> {
        unimplemented!()
    }
    fn claims(
        &self,
        _h: Option<&str>,
        _p: Option<&str>,
    ) -> Result<Vec<vissue_core::views::ClaimRow>, vissue_core::error::Error> {
        unimplemented!()
    }
    fn agenda(
        &self,
        _d: i64,
        _p: Option<&str>,
    ) -> Result<Vec<vissue_core::views::AgendaRow>, vissue_core::error::Error> {
        unimplemented!()
    }
    fn tree(&self, _id: &str) -> Result<vissue_core::views::TreeNode, vissue_core::error::Error> {
        unimplemented!()
    }
    fn related(
        &self,
        _id: &str,
        _d: usize,
        _n: usize,
    ) -> Result<Vec<vissue_core::views::RelatedHit>, vissue_core::error::Error> {
        unimplemented!()
    }
    fn projects(&self) -> Result<Vec<String>, vissue_core::error::Error> {
        Ok(vec![])
    }
    fn claim(&self, _id: &str, _f: bool) -> Result<MutResult, vissue_core::error::Error> {
        unimplemented!()
    }
    fn note(&self, _id: &str, _t: &str) -> Result<MutResult, vissue_core::error::Error> {
        unimplemented!()
    }
    fn update(&self, _r: UpdateReq) -> Result<MutResult, vissue_core::error::Error> {
        unimplemented!()
    }
    fn open(
        &self,
        _id: &str,
    ) -> Result<vissue_core::views::IssueDetail, vissue_core::error::Error> {
        unimplemented!()
    }
    fn wait(&self, last: u64, _ms: u64) -> Result<u64, vissue_core::error::Error> {
        Ok(last)
    }
    fn last_since_revision(&self) -> Option<Option<u64>> {
        *self.last.lock().unwrap()
    }
}

fn connect_ok(_: &Path, _: &Layout, _: &str) -> Result<Box<dyn BoardBackend>, AttachFail> {
    Ok(Box::new(RecordingBackend::after_initialize(9)))
}

fn probe_yes(_: &Path) -> bool {
    true
}

fn ensure_unused(_: &ServeConfig) -> Result<vissue_serve::EnsureResult, String> {
    panic!("probe already accepted")
}

#[test]
fn accept_then_connect_switches_to_control() {
    let layout = Layout::new("/tmp/vissue-tui-switch", "Software");
    let hooks = AttachHooks {
        probe: probe_yes,
        ensure: ensure_unused,
        connect: connect_ok,
    };
    match try_attach(
        &layout,
        &PathBuf::from("/tmp/vissue-tui-switch.sock"),
        "agent",
        false,
        &hooks,
    ) {
        AttachOutcome::Switch { backend, status } => {
            assert_eq!(status, ServeStatus::Live);
            assert_eq!(backend.live(), BackendKind::Control);
            assert_eq!(backend.revision(), 9);
        }
        _ => panic!("expected switch"),
    }
}

#[test]
fn after_mocked_initialize_next_list_has_no_since_revision() {
    let backend = RecordingBackend::after_initialize(41);
    let page = backend.ready(None).unwrap();
    assert!(!page.unchanged);
    assert_eq!(backend.last_since_revision(), Some(None));
    let _ = backend.list(ListQuery::default()).unwrap();
    assert_eq!(backend.last_since_revision(), Some(Some(41)));
}
