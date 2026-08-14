//! Overlay state: filter, excerpt, claim, note. No iced types.

use std::path::Path;

use vissue_core::config::Layout;
use vissue_core::views::{Excerpt, IssueDetail, IssueRow, SearchHit};
use vissue_tui::attach::{AttachHooks, AttachOutcome, ServeStatus};
use vissue_tui::backend::BoardBackend;
use vissue_tui::CoreBackend;

use crate::attach;
use crate::fuzzy::rank_indices;
use crate::summon::{SummonAction, SummonRequest};

/// Where a row came from before the filter merge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemSource {
    Ready,
    Search,
}

/// One selectable palette row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HudItem {
    pub id: String,
    pub title: String,
    pub project: String,
    pub state: String,
    pub priority: String,
    pub source: ItemSource,
}

impl HudItem {
    fn from_row(row: IssueRow) -> Self {
        Self {
            id: row.id,
            title: row.title,
            project: row.project,
            state: row.state,
            priority: row.priority,
            source: ItemSource::Ready,
        }
    }

    fn from_search(hit: SearchHit) -> Self {
        Self {
            id: hit.id,
            title: hit.title,
            project: hit.project,
            state: hit.state,
            priority: hit.priority,
            source: ItemSource::Search,
        }
    }
}

/// Key the overlay understands. iced maps native keys onto this.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaletteKey {
    Char(char),
    Enter,
    Esc,
    Up,
    Down,
    Backspace,
}

/// Summonable palette. Talks only to [`BoardBackend`].
pub struct Palette {
    backend: Box<dyn BoardBackend>,
    agent: String,
    status: ServeStatus,
    message: String,
    query: String,
    items: Vec<HudItem>,
    filtered: Vec<usize>,
    selected: usize,
    excerpt: Option<Excerpt>,
    detail: Option<IssueDetail>,
    note_draft: Option<String>,
    visible: bool,
}

impl Palette {
    pub fn open_core(layout: Layout, agent: String) -> anyhow::Result<Self> {
        let backend = CoreBackend::open(layout, agent.clone())?;
        Self::with_backend(Box::new(backend), agent, ServeStatus::Offline)
    }

    pub fn with_backend(
        backend: Box<dyn BoardBackend>,
        agent: String,
        status: ServeStatus,
    ) -> anyhow::Result<Self> {
        let mut palette = Self {
            backend,
            agent,
            status,
            message: String::new(),
            query: String::new(),
            items: Vec::new(),
            filtered: Vec::new(),
            selected: 0,
            excerpt: None,
            detail: None,
            note_draft: None,
            visible: true,
        };
        palette.reload()?;
        Ok(palette)
    }

    pub fn serve_status(&self) -> ServeStatus {
        self.status
    }

    pub fn agent(&self) -> &str {
        &self.agent
    }

    pub fn visible(&self) -> bool {
        self.visible
    }

    pub fn query(&self) -> &str {
        &self.query
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn excerpt(&self) -> Option<&Excerpt> {
        self.excerpt.as_ref()
    }

    pub fn detail(&self) -> Option<&IssueDetail> {
        self.detail.as_ref()
    }

    pub fn note_draft(&self) -> Option<&str> {
        self.note_draft.as_deref()
    }

    pub fn backend(&self) -> &dyn BoardBackend {
        self.backend.as_ref()
    }

    pub fn generation(&self) -> u64 {
        self.backend.generation()
    }

    pub fn revision(&self) -> u64 {
        self.backend.revision()
    }

    pub fn filtered_items(&self) -> Vec<&HudItem> {
        self.filtered
            .iter()
            .filter_map(|&i| self.items.get(i))
            .collect()
    }

    pub fn selected_item(&self) -> Option<&HudItem> {
        self.filtered
            .get(self.selected)
            .and_then(|&i| self.items.get(i))
    }

    pub fn selected_id(&self) -> Option<&str> {
        self.selected_item().map(|i| i.id.as_str())
    }

    pub fn selected_index(&self) -> usize {
        self.selected
    }

    pub fn status_line(&self) -> String {
        let kind = match self.status {
            ServeStatus::Live => "live",
            ServeStatus::Offline => "offline",
            ServeStatus::Mismatch => "mismatch",
        };
        let mut line = format!(
            "serve:{kind} gen={} rev={} agent={}",
            self.backend.generation(),
            self.backend.revision(),
            self.agent
        );
        if !self.message.is_empty() {
            line.push_str("  ");
            line.push_str(&self.message);
        }
        line
    }

    /// Post-paint attach. `--offline` never probes the socket.
    pub fn attach(
        &mut self,
        socket: &Path,
        offline: bool,
        hooks: &AttachHooks,
    ) -> anyhow::Result<()> {
        let layout = self.backend.layout().clone();
        let agent = self.agent.clone();
        match attach::attach(&layout, socket, &agent, offline, hooks) {
            AttachOutcome::Switch { backend, status } => {
                self.backend = backend;
                self.status = status;
                self.agent = self.backend.identity().to_string();
                self.message.clear();
            }
            AttachOutcome::Stay { status, message } => {
                self.status = status;
                self.message = message;
            }
        }
        self.reload()
    }

    pub fn set_query(&mut self, query: impl Into<String>) {
        self.query = query.into();
        self.excerpt = None;
        let _ = self.reload();
    }

    pub fn show(&mut self) {
        self.visible = true;
    }

    pub fn hide(&mut self) {
        self.note_draft = None;
        self.visible = false;
    }

    pub fn toggle(&mut self) {
        if self.visible {
            self.hide();
        } else {
            self.show();
        }
    }

    pub fn apply_summon(&mut self, req: &SummonRequest) {
        match req.action {
            SummonAction::Show => self.show(),
            SummonAction::Hide => self.hide(),
            SummonAction::Toggle => self.toggle(),
        }
    }

    pub fn handle_key(&mut self, key: PaletteKey) {
        if !self.visible {
            return;
        }
        if self.note_draft.is_some() {
            self.handle_note_key(key);
            return;
        }
        match key {
            PaletteKey::Esc => {
                if self.excerpt.is_some() {
                    self.excerpt = None;
                } else {
                    self.hide();
                }
            }
            PaletteKey::Enter => self.show_excerpt(),
            PaletteKey::Up => self.move_sel(-1),
            PaletteKey::Down => self.move_sel(1),
            PaletteKey::Backspace => {
                self.query.pop();
                let _ = self.reload();
            }
            PaletteKey::Char('c') => self.claim_selected(),
            PaletteKey::Char('n') => {
                if self.selected_id().is_some() {
                    self.note_draft = Some(String::new());
                }
            }
            PaletteKey::Char(c) => {
                self.query.push(c);
                self.excerpt = None;
                let _ = self.reload();
            }
        }
    }

    fn handle_note_key(&mut self, key: PaletteKey) {
        let Some(mut text) = self.note_draft.take() else {
            return;
        };
        match key {
            PaletteKey::Esc => {
                self.message.clear();
            }
            PaletteKey::Enter => {
                if let Some(id) = self.selected_id().map(str::to_string) {
                    match self.backend.note(&id, &text) {
                        Ok(result) => {
                            self.message = result.report.trim().to_string();
                            let _ = self.reload();
                        }
                        Err(err) => self.message = err.to_string(),
                    }
                }
            }
            PaletteKey::Backspace => {
                text.pop();
                self.note_draft = Some(text);
            }
            PaletteKey::Char(c) => {
                text.push(c);
                self.note_draft = Some(text);
            }
            PaletteKey::Up | PaletteKey::Down => {
                self.note_draft = Some(text);
            }
        }
    }

    pub fn show_excerpt(&mut self) {
        let Some(id) = self.selected_id().map(str::to_string) else {
            return;
        };
        match self.backend.excerpt(&id) {
            Ok(excerpt) => {
                self.excerpt = Some(excerpt);
                self.detail = self.backend.get(&id).ok();
            }
            Err(err) => self.message = err.to_string(),
        }
    }

    pub fn claim_selected(&mut self) {
        let Some(id) = self.selected_id().map(str::to_string) else {
            return;
        };
        match self.backend.claim(&id, false) {
            Ok(result) => {
                self.message = result.report.trim().to_string();
                let _ = self.reload();
            }
            Err(err) => self.message = err.to_string(),
        }
    }

    pub fn poll_updates(&mut self) {
        let last = match self.backend.live() {
            vissue_tui::BackendKind::Control => self.backend.revision(),
            vissue_tui::BackendKind::Core => self.backend.generation(),
        };
        if let Ok(next) = self.backend.wait(last, 1) {
            if next > last {
                let _ = self.reload();
            }
        }
    }

    pub fn reload(&mut self) -> anyhow::Result<()> {
        // Last full ready page stays when serve answers `{unchanged: true,
        // issues: []}`. Search extras are rebuilt from the current query.
        self.items.retain(|item| item.source == ItemSource::Ready);
        if let Ok(page) = self.backend.ready(None) {
            if !page.unchanged {
                self.items = page.issues.into_iter().map(HudItem::from_row).collect();
            }
        }
        if !self.query.is_empty() {
            if let Ok(hits) = self.backend.search(&self.query, 50) {
                for hit in hits {
                    if !self.items.iter().any(|i| i.id == hit.id) {
                        self.items.push(HudItem::from_search(hit));
                    }
                }
            }
        }
        self.refilter();
        Ok(())
    }

    fn refilter(&mut self) {
        self.filtered = rank_indices(&self.query, &self.items);
        if self.selected >= self.filtered.len() {
            self.selected = self.filtered.len().saturating_sub(1);
        }
    }

    fn move_sel(&mut self, delta: i32) {
        if self.filtered.is_empty() {
            return;
        }
        let len = self.filtered.len() as i32;
        let next = (self.selected as i32 + delta).clamp(0, len - 1) as usize;
        if next != self.selected {
            self.selected = next;
            self.excerpt = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicBool, Ordering};
    use vissue_core::config::DEFAULT_PREFIX;
    use vissue_tui::attach::AttachFail;
    use vissue_tui::backend::BackendKind;

    static TOUCHED: AtomicBool = AtomicBool::new(false);

    fn fixture_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixture_vault")
    }

    fn copy_tree(src: &Path, dest: &Path) {
        std::fs::create_dir_all(dest).unwrap();
        for entry in std::fs::read_dir(src).unwrap() {
            let entry = entry.unwrap();
            let target = dest.join(entry.file_name());
            if entry.file_type().unwrap().is_dir() {
                copy_tree(&entry.path(), &target);
            } else {
                std::fs::copy(entry.path(), &target).unwrap();
            }
        }
    }

    fn writable() -> (tempfile::TempDir, Layout) {
        let dir = tempfile::tempdir().unwrap();
        copy_tree(
            &fixture_root().join(DEFAULT_PREFIX),
            &dir.path().join(DEFAULT_PREFIX),
        );
        let layout = Layout::new(dir.path(), DEFAULT_PREFIX);
        (dir, layout)
    }

    fn panic_probe(_: &Path) -> bool {
        TOUCHED.store(true, Ordering::SeqCst);
        panic!("offline must not probe");
    }
    fn panic_ensure(_: &vissue_serve::ServeConfig) -> Result<vissue_serve::EnsureResult, String> {
        TOUCHED.store(true, Ordering::SeqCst);
        panic!("offline must not ensure");
    }
    fn panic_connect(_: &Path, _: &Layout, _: &str) -> Result<Box<dyn BoardBackend>, AttachFail> {
        TOUCHED.store(true, Ordering::SeqCst);
        panic!("offline must not connect");
    }
    fn yes_probe(_: &Path) -> bool {
        true
    }
    fn connect_mismatch(
        _: &Path,
        _: &Layout,
        _: &str,
    ) -> Result<Box<dyn BoardBackend>, AttachFail> {
        Err(AttachFail::Mismatch("other root".into()))
    }

    /// Second `ready` returns `{unchanged: true, issues: []}`, like serve
    /// when `since_revision` matches the catalog head.
    struct UnchangedAfterFirst {
        inner: CoreBackend,
        ready_calls: std::sync::atomic::AtomicUsize,
    }

    impl UnchangedAfterFirst {
        fn new(inner: CoreBackend) -> Self {
            Self {
                inner,
                ready_calls: std::sync::atomic::AtomicUsize::new(0),
            }
        }
    }

    impl BoardBackend for UnchangedAfterFirst {
        fn layout(&self) -> &Layout {
            self.inner.layout()
        }
        fn generation(&self) -> u64 {
            self.inner.generation()
        }
        fn revision(&self) -> u64 {
            5
        }
        fn live(&self) -> BackendKind {
            BackendKind::Control
        }
        fn identity(&self) -> &str {
            self.inner.identity()
        }
        fn list(
            &self,
            q: vissue_core::views::ListQuery,
        ) -> Result<vissue_tui::ListPage, vissue_core::error::Error> {
            self.inner.list(q)
        }
        fn ready(
            &self,
            project: Option<&str>,
        ) -> Result<vissue_tui::ListPage, vissue_core::error::Error> {
            let n = self
                .ready_calls
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            if n == 0 {
                self.inner.ready(project)
            } else {
                Ok(vissue_tui::ListPage {
                    unchanged: true,
                    revision: 5,
                    ..vissue_tui::ListPage::default()
                })
            }
        }
        fn get(
            &self,
            id: &str,
        ) -> Result<vissue_core::views::IssueDetail, vissue_core::error::Error> {
            self.inner.get(id)
        }
        fn excerpt(
            &self,
            id: &str,
        ) -> Result<vissue_core::views::Excerpt, vissue_core::error::Error> {
            self.inner.excerpt(id)
        }
        fn search(
            &self,
            q: &str,
            n: usize,
        ) -> Result<Vec<vissue_core::views::SearchHit>, vissue_core::error::Error> {
            self.inner.search(q, n)
        }
        fn claims(
            &self,
            h: Option<&str>,
            p: Option<&str>,
        ) -> Result<Vec<vissue_core::views::ClaimRow>, vissue_core::error::Error> {
            self.inner.claims(h, p)
        }
        fn agenda(
            &self,
            d: i64,
            p: Option<&str>,
        ) -> Result<Vec<vissue_core::views::AgendaRow>, vissue_core::error::Error> {
            self.inner.agenda(d, p)
        }
        fn tree(
            &self,
            id: &str,
        ) -> Result<vissue_core::views::TreeNode, vissue_core::error::Error> {
            self.inner.tree(id)
        }
        fn related(
            &self,
            id: &str,
            d: usize,
            n: usize,
        ) -> Result<Vec<vissue_core::views::RelatedHit>, vissue_core::error::Error> {
            self.inner.related(id, d, n)
        }
        fn projects(&self) -> Result<Vec<String>, vissue_core::error::Error> {
            self.inner.projects()
        }
        fn claim(
            &self,
            id: &str,
            f: bool,
        ) -> Result<vissue_tui::MutResult, vissue_core::error::Error> {
            self.inner.claim(id, f)
        }
        fn note(
            &self,
            id: &str,
            t: &str,
        ) -> Result<vissue_tui::MutResult, vissue_core::error::Error> {
            self.inner.note(id, t)
        }
        fn update(
            &self,
            r: vissue_tui::UpdateReq,
        ) -> Result<vissue_tui::MutResult, vissue_core::error::Error> {
            self.inner.update(r)
        }
        fn open(
            &self,
            id: &str,
        ) -> Result<vissue_core::views::IssueDetail, vissue_core::error::Error> {
            self.inner.open(id)
        }
        fn wait(&self, last: u64, ms: u64) -> Result<u64, vissue_core::error::Error> {
            self.inner.wait(last, ms)
        }
    }

    #[test]
    fn first_paint_lists_ready_ids() {
        let palette =
            Palette::open_core(Layout::new(fixture_root(), DEFAULT_PREFIX), "snap".into()).unwrap();
        let ids: Vec<_> = palette
            .filtered_items()
            .into_iter()
            .map(|i| i.id.as_str())
            .collect();
        assert_eq!(ids, ["atlas-1a2b", "atlas-2c3d", "beacon-5j6k"]);
        assert!(palette.status_line().contains("serve:offline"));
        assert_eq!(palette.revision(), 0);
    }

    #[test]
    fn enter_shows_excerpt() {
        let (_dir, layout) = writable();
        let mut palette = Palette::open_core(layout, "hud-test".into()).unwrap();
        palette.handle_key(PaletteKey::Down);
        assert_eq!(palette.selected_id(), Some("atlas-2c3d"));
        palette.handle_key(PaletteKey::Enter);
        let text = palette.excerpt().map(|e| e.text.as_str()).unwrap_or("");
        assert!(text.contains("summary"), "{text}");
    }

    #[test]
    fn esc_hides_and_process_state_stays() {
        let palette_layout = Layout::new(fixture_root(), DEFAULT_PREFIX);
        let mut palette = Palette::open_core(palette_layout, "snap".into()).unwrap();
        assert!(palette.visible());
        palette.handle_key(PaletteKey::Esc);
        assert!(!palette.visible());
        assert_eq!(palette.selected_id(), Some("atlas-1a2b"));
    }

    #[test]
    fn offline_attach_never_touches_socket() {
        TOUCHED.store(false, Ordering::SeqCst);
        let mut palette =
            Palette::open_core(Layout::new(fixture_root(), DEFAULT_PREFIX), "snap".into()).unwrap();
        let hooks = AttachHooks {
            probe: panic_probe,
            ensure: panic_ensure,
            connect: panic_connect,
        };
        palette
            .attach(Path::new("/tmp/vissue-hud-offline.sock"), true, &hooks)
            .unwrap();
        assert_eq!(palette.serve_status(), ServeStatus::Offline);
        assert_eq!(palette.backend().live(), BackendKind::Core);
        assert!(!TOUCHED.load(Ordering::SeqCst));
    }

    #[test]
    fn mismatch_stays_core_and_claims_via_ops() {
        let (_dir, layout) = writable();
        let mut palette = Palette::open_core(layout, "hud-test".into()).unwrap();
        let hooks = AttachHooks {
            probe: yes_probe,
            ensure: panic_ensure,
            connect: connect_mismatch,
        };
        palette
            .attach(Path::new("/tmp/vissue-hud-mis.sock"), false, &hooks)
            .unwrap();
        assert_eq!(palette.serve_status(), ServeStatus::Mismatch);
        assert_eq!(palette.backend().live(), BackendKind::Core);
        palette.set_query("atlas-2c3d");
        assert_eq!(palette.selected_id(), Some("atlas-2c3d"));
        palette.claim_selected();
        assert_eq!(
            palette
                .backend()
                .get("atlas-2c3d")
                .unwrap()
                .claimed_by
                .as_deref(),
            Some("hud-test")
        );
        assert_eq!(
            palette.backend().get("atlas-2c3d").unwrap().state,
            "STARTED"
        );
    }

    #[test]
    fn c_claims_and_n_notes_selected() {
        let (_dir, layout) = writable();
        let mut palette = Palette::open_core(layout, "hud-test".into()).unwrap();
        palette.set_query("atlas-2c3d");
        palette.handle_key(PaletteKey::Char('c'));
        assert_eq!(
            palette
                .backend()
                .get("atlas-2c3d")
                .unwrap()
                .claimed_by
                .as_deref(),
            Some("hud-test")
        );
        palette.handle_key(PaletteKey::Char('n'));
        assert!(palette.note_draft().is_some());
        palette.handle_key(PaletteKey::Char('h'));
        palette.handle_key(PaletteKey::Char('i'));
        palette.handle_key(PaletteKey::Enter);
        assert!(palette.note_draft().is_none());
    }

    #[test]
    fn summon_toggle_hides() {
        let mut palette =
            Palette::open_core(Layout::new(fixture_root(), DEFAULT_PREFIX), "snap".into()).unwrap();
        palette.apply_summon(&SummonRequest::new(SummonAction::Toggle));
        assert!(!palette.visible());
        palette.apply_summon(&SummonRequest::new(SummonAction::Show));
        assert!(palette.visible());
        palette.apply_summon(&SummonRequest::new(SummonAction::Hide));
        assert!(!palette.visible());
        palette.handle_key(PaletteKey::Char('c'));
        assert!(!palette.visible());
    }

    #[test]
    fn keys_move_filter_and_backspace() {
        let mut palette =
            Palette::open_core(Layout::new(fixture_root(), DEFAULT_PREFIX), "snap".into()).unwrap();
        assert_eq!(palette.agent(), "snap");
        assert_eq!(palette.generation(), 0);
        assert!(palette.message().is_empty());
        assert!(palette.note_draft().is_none());
        palette.handle_key(PaletteKey::Down);
        assert_eq!(palette.selected_id(), Some("atlas-2c3d"));
        palette.handle_key(PaletteKey::Up);
        assert_eq!(palette.selected_id(), Some("atlas-1a2b"));
        palette.handle_key(PaletteKey::Char('j'));
        assert_eq!(palette.query(), "j");
        palette.handle_key(PaletteKey::Backspace);
        palette.handle_key(PaletteKey::Char('k'));
        assert_eq!(palette.query(), "k");
        palette.handle_key(PaletteKey::Backspace);
        assert_eq!(palette.query(), "");
        assert_eq!(palette.selected_id(), Some("atlas-1a2b"));
        palette.handle_key(PaletteKey::Char('z'));
        assert_eq!(palette.query(), "z");
        assert!(palette.filtered_items().is_empty());
        palette.handle_key(PaletteKey::Backspace);
        assert_eq!(palette.query(), "");
        palette.handle_key(PaletteKey::Enter);
        assert!(palette.excerpt().is_some());
        palette.handle_key(PaletteKey::Esc);
        assert!(palette.excerpt().is_none());
        assert!(palette.visible());
        palette.poll_updates();
        let _ = palette.status_line();
    }

    #[test]
    fn note_prompt_esc_and_empty_claim_are_noops() {
        let (_dir, layout) = writable();
        let mut palette = Palette::open_core(layout, "hud-test".into()).unwrap();
        palette.handle_key(PaletteKey::Char('n'));
        assert!(palette.note_draft().is_some());
        palette.handle_key(PaletteKey::Up);
        palette.handle_key(PaletteKey::Down);
        palette.handle_key(PaletteKey::Esc);
        assert!(palette.note_draft().is_none());
        palette.set_query("no-such-id");
        assert!(palette.selected_id().is_none());
        palette.claim_selected();
        palette.show_excerpt();
        palette.handle_key(PaletteKey::Char('n'));
        assert!(palette.note_draft().is_none());
    }

    #[test]
    fn unchanged_ready_does_not_wipe_rows() {
        let backend = UnchangedAfterFirst::new(
            CoreBackend::open(Layout::new(fixture_root(), DEFAULT_PREFIX), "snap").unwrap(),
        );
        let mut palette =
            Palette::with_backend(Box::new(backend), "snap".into(), ServeStatus::Live).unwrap();
        let first: Vec<_> = palette
            .filtered_items()
            .into_iter()
            .map(|i| i.id.clone())
            .collect();
        assert_eq!(first, ["atlas-1a2b", "atlas-2c3d", "beacon-5j6k"]);
        palette.reload().unwrap();
        let second: Vec<_> = palette
            .filtered_items()
            .into_iter()
            .map(|i| i.id.clone())
            .collect();
        assert_eq!(first, second);
        palette.handle_key(PaletteKey::Char('z'));
        palette.handle_key(PaletteKey::Backspace);
        assert_eq!(palette.query(), "");
        let after_backspace: Vec<_> = palette
            .filtered_items()
            .into_iter()
            .map(|i| i.id.clone())
            .collect();
        assert_eq!(first, after_backspace);
    }
}
