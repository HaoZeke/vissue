//! The on-disk store: one `issues.org` per project, parsed and rewritten whole.

use anyhow::{Context, anyhow};

use crate::error::Result;
use fs2::FileExt;
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::config::Layout;
use crate::model::{IssueHeading, LogEntry, TODO_HEADER, parse_log_line, today_inactive_bracket};
use crate::org::{
    OrgScan, ensure_org_preamble, is_headline, is_issue_headline, is_planning_line,
    is_top_level_headline, opens_a_drawer, parse_headline_bits, parse_planning_line,
    property_key_and_append, split_statistics_cookies, todo_keywords_from_lines,
};

/// Process-local mutex per path, so concurrent async handlers in one process
/// serialize even where an advisory file lock would not (same process, many
/// descriptors).
static PROCESS_LOCKS: OnceLock<Mutex<HashMap<PathBuf, Arc<Mutex<()>>>>> = OnceLock::new();
static WRITE_TMP_SEQ: AtomicU64 = AtomicU64::new(0);

const ID_ALPHABET: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyz";

struct CrossProcessLock {
    file: fs::File,
}

impl CrossProcessLock {
    fn acquire(path: &Path) -> Result<Self> {
        let lock_path = issues_lock_path(path);
        if let Some(parent) = lock_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("create lock parent {}", parent.display()))?;
        }
        let file = fs::OpenOptions::new()
            .create(true)
            .read(true)
            .append(true)
            .open(&lock_path)
            .with_context(|| format!("open lock {}", lock_path.display()))?;
        file.lock_exclusive()
            .with_context(|| format!("lock {}", lock_path.display()))?;
        Ok(Self { file })
    }
}

impl Drop for CrossProcessLock {
    fn drop(&mut self) {
        let _ = fs2::FileExt::unlock(&self.file);
    }
}

/// Write `bytes` and flush them to the device, so the caller may rename the
/// file knowing the contents are durable.
fn write_synced(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::io::Write as _;
    let mut file = fs::File::create(path)?;
    file.write_all(bytes)?;
    file.sync_all()
}

/// Replace `path` with `body` through a flushed temporary and a rename.
///
/// For generated output a reader shares, which is a mirror: a plain write
/// truncates first, so a reader mid-pull, or a crash, sees a half file where
/// a whole one was.
///
/// # Errors
///
/// Returns an error if the parent directory cannot be created, the temporary
/// cannot be written, or the rename cannot publish it.
pub fn replace_file_atomically(path: &Path, body: &str) -> Result<()> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    fs::create_dir_all(&parent).with_context(|| format!("create {}", parent.display()))?;
    let seq = WRITE_TMP_SEQ.fetch_add(1, Ordering::Relaxed);
    let base = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("output");
    let tmp = parent.join(format!(".{}.tmp.{}-{}", base, std::process::id(), seq));
    if let Err(e) = write_synced(&tmp, body.as_bytes()) {
        let _ = fs::remove_file(&tmp);
        return Err(e)
            .with_context(|| format!("write temp {}", tmp.display()))
            .map_err(crate::error::Error::from);
    }
    if let Err(e) = fs::rename(&tmp, path) {
        let _ = fs::remove_file(&tmp);
        return Err(e)
            .with_context(|| format!("rename {} -> {}", tmp.display(), path.display()))
            .map_err(crate::error::Error::from);
    }
    Ok(())
}

fn issues_lock_path(path: &Path) -> PathBuf {
    let mut s = path.as_os_str().to_owned();
    s.push(".lock");
    PathBuf::from(s)
}

fn process_mutex_for(path: &Path) -> Arc<Mutex<()>> {
    let key = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let mut map = PROCESS_LOCKS
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .unwrap_or_else(|p| p.into_inner());
    map.entry(key)
        .or_insert_with(|| Arc::new(Mutex::new(())))
        .clone()
}

/// Serialize every read-modify-write cycle on one `issues.org`.
///
/// Without this, parallel creates race on the temporary file rename and lose
/// updates: the last writer wins and peers see a vanished temporary.
///
/// # Errors
///
/// Returns an error if the lock file cannot be created or acquired, or if `f`
/// itself fails.
pub fn with_issues_lock<R, F>(path: &Path, f: F) -> Result<R>
where
    F: FnOnce() -> Result<R>,
{
    let mutex = process_mutex_for(path);
    let _proc = mutex.lock().unwrap_or_else(|p| p.into_inner());
    let _cross = CrossProcessLock::acquire(path)?;
    f()
}

/// Lock several files in sorted order, which makes a cross-project move
/// deadlock-free.
///
/// # Errors
///
/// Returns an error if a lock file cannot be created or acquired, or if `f`
/// itself fails.
pub fn with_issues_locks<R, F>(paths: &[&Path], f: F) -> Result<R>
where
    F: FnOnce() -> Result<R>,
{
    let mut keys: Vec<PathBuf> = paths.iter().map(|p| (*p).to_path_buf()).collect();
    keys.sort();
    keys.dedup();
    let mutexes: Vec<Arc<Mutex<()>>> = keys.iter().map(|k| process_mutex_for(k)).collect();
    let mut proc_guards = Vec::with_capacity(mutexes.len());
    let mut cross_guards = Vec::with_capacity(keys.len());
    for (key, mutex) in keys.iter().zip(mutexes.iter()) {
        proc_guards.push(mutex.lock().unwrap_or_else(|p| p.into_inner()));
        cross_guards.push(CrossProcessLock::acquire(key)?);
    }
    f()
}

/// One project's `issues.org`: a preamble followed by top-level headings.
#[derive(Debug, Clone)]
pub struct IssueDoc {
    /// Project directory name this file belongs to.
    pub project: String,
    /// Path of the `issues.org` this document was parsed from or will write to.
    pub path: PathBuf,
    /// File header above the first heading, including `#+TODO:`.
    pub preamble: String,
    /// Top-level issue headings, in file order.
    pub headings: Vec<IssueHeading>,
    /// Org that follows each issue (COMMENT trees, notes headings). Same
    /// length as [`Self::headings`] after a parse; a write pads missing
    /// slots with the usual blank line.
    after: Vec<String>,
}

impl IssueDoc {
    /// An empty document with the house preamble and no headings.
    pub fn empty(project: &str, path: PathBuf) -> Self {
        IssueDoc {
            project: project.to_string(),
            path,
            preamble: default_preamble(project),
            headings: Vec::new(),
            after: Vec::new(),
        }
    }

    /// Parse `path`, or produce an empty document when the file is absent.
    ///
    /// # Errors
    ///
    /// Returns an error if the file exists but cannot be read or parsed.
    pub fn parse_file(project: &str, path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::empty(project, path.to_path_buf()));
        }
        let content =
            fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
        Self::parse(project, path.to_path_buf(), &content)
    }

    /// Parse `content` as an `issues.org` for `project`.
    ///
    /// # Errors
    ///
    /// Returns an error if an issue heading has no `:ID:`. A heading
    /// whose first word is not a TODO keyword is Org around the issues,
    /// not a failed parse.
    pub fn parse(project: &str, path: PathBuf, content: &str) -> Result<Self> {
        let lines: Vec<&str> = content.lines().collect();
        let keywords = todo_keywords_from_lines(&lines);
        let mut nest = OrgScan::new();
        let first_heading = lines
            .iter()
            .position(|line| {
                if nest.observe(line) {
                    return false;
                }
                is_issue_headline(line, &keywords)
            })
            .unwrap_or(lines.len());
        let preamble = if first_heading == 0 {
            default_preamble(project)
        } else {
            lines[..first_heading].join("\n").trim_end().to_string()
        };
        let mut headings = Vec::new();
        let mut after = Vec::new();
        let mut i = first_heading;
        while i < lines.len() {
            if !is_issue_headline(lines[i], &keywords) {
                i += 1;
                continue;
            }
            let (heading, body_end) = parse_heading(&lines, i, &keywords)
                .with_context(|| format!("at {}:{}", path.display(), i + 1))?;
            headings.push(heading);
            i = body_end;
            let inter_start = i;
            let mut nest = OrgScan::new();
            while i < lines.len() {
                if !nest.observe(lines[i]) && is_issue_headline(lines[i], &keywords) {
                    break;
                }
                i += 1;
            }
            let raw = lines[inter_start..i].join("\n");
            after.push(if raw.trim().is_empty() {
                String::new()
            } else {
                raw.trim_start_matches('\n').to_string()
            });
        }
        Ok(IssueDoc {
            project: project.to_string(),
            path,
            preamble,
            headings,
            after,
        })
    }

    /// The file body this document would write.
    pub fn render_string(&self) -> String {
        let mut out = String::new();
        let preamble = if self.preamble.trim().is_empty() {
            default_preamble(&self.project)
        } else {
            ensure_org_preamble(&self.preamble, &self.project)
        };
        out.push_str(preamble.trim_end());
        out.push_str("\n\n");
        for (i, h) in self.headings.iter().enumerate() {
            out.push_str(&h.render());
            out.push('\n');
            if let Some(extra) = self.after.get(i)
                && !extra.is_empty()
            {
                out.push_str(extra);
                if !extra.ends_with('\n') {
                    out.push('\n');
                }
            }
        }
        out
    }

    /// Render and replace the file through a uniquely named temporary. Callers
    /// hold [`with_issues_lock`] around the parse and this write.
    ///
    /// # Errors
    ///
    /// Returns an error if the parent directory cannot be created, the
    /// temporary cannot be written, or the rename cannot publish it.
    pub fn write(&self) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let out = self.render_string();
        // A shared temporary name races: a peer renames it out from under this
        // writer and the rename fails with ENOENT.
        let seq = WRITE_TMP_SEQ.fetch_add(1, Ordering::Relaxed);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let base = self
            .path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("issues.org");
        let tmp = self
            .path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(format!(
                ".{}.tmp.{}-{}-{}",
                base,
                std::process::id(),
                nanos,
                seq
            ));
        // Flush the bytes to the device before the rename publishes them.
        // Rename is atomic against a concurrent reader, not against a crash:
        // an unsynced temporary can land as a truncated issues.org.
        if let Err(e) = write_synced(&tmp, out.as_bytes()) {
            let _ = fs::remove_file(&tmp);
            return Err(e)
                .with_context(|| format!("write temp {}", tmp.display()))
                .map_err(crate::error::Error::from);
        }
        if let Err(e) = fs::rename(&tmp, &self.path) {
            let _ = fs::remove_file(&tmp);
            return Err(e)
                .with_context(|| format!("rename {} -> {}", tmp.display(), self.path.display()))
                .map_err(crate::error::Error::from);
        }
        self.announce_write();
        Ok(())
    }

    /// Tell pollers the file moved. The event files live beside the project
    /// directories, which is this file's grandparent. Failure here is not a
    /// failed write: the issue is already on disk.
    fn announce_write(&self) {
        if !crate::events::enabled() {
            return;
        }
        let Some(dir) = self.path.parent().and_then(|p| p.parent()) else {
            return;
        };
        let _ = crate::events::emit_issues_write(dir, &self.project, &self.path);
        let _ = crate::events::ensure_gitignore_hint(dir);
    }

    /// Every heading id in this document, in file order.
    pub fn known_ids(&self) -> Vec<String> {
        self.headings.iter().map(|h| h.id.clone()).collect()
    }

    /// Replace a heading with the same id, or append if none matches.
    pub fn upsert(&mut self, heading: IssueHeading) {
        if let Some(slot) = self.headings.iter_mut().find(|h| h.id == heading.id) {
            *slot = heading;
        } else {
            self.headings.push(heading);
            self.after.push(String::new());
        }
    }

    /// Remove the heading with `id`, if it is in this document.
    pub fn remove(&mut self, id: &str) -> Option<IssueHeading> {
        let idx = self.headings.iter().position(|h| h.id == id)?;
        let heading = self.headings.remove(idx);
        let extra = if idx < self.after.len() {
            self.after.remove(idx)
        } else {
            String::new()
        };
        if !extra.trim().is_empty() {
            if idx == 0 {
                if !self.preamble.is_empty() && !self.preamble.ends_with('\n') {
                    self.preamble.push('\n');
                }
                if !self.preamble.is_empty() {
                    self.preamble.push('\n');
                }
                self.preamble.push_str(&extra);
            } else if let Some(prev) = self.after.get_mut(idx - 1) {
                if !prev.is_empty() && !prev.ends_with('\n') {
                    prev.push('\n');
                }
                prev.push_str(&extra);
            }
        }
        Some(heading)
    }
}

fn parse_heading(
    lines: &[&str],
    start: usize,
    keywords: &[String],
) -> Result<(IssueHeading, usize)> {
    let header = lines[start];
    let stripped = header
        .strip_prefix("* ")
        .ok_or_else(|| anyhow!("not a heading"))?;
    let bits = parse_headline_bits(stripped, keywords);
    let state = bits
        .keyword
        .ok_or_else(|| anyhow!("not an issue heading"))?
        .to_string();
    let priority = bits.priority.unwrap_or('C');
    let (title_and_cookies, org_tags) = crate::model::split_headline_tags(bits.rest);
    let (title, statistics) = split_statistics_cookies(&title_and_cookies);

    let mut properties = BTreeMap::new();
    let mut property_order = Vec::new();
    let mut logbook: Vec<LogEntry> = Vec::new();
    let mut extra_drawers: Vec<String> = Vec::new();
    let mut i = start + 1;

    // Org writes DEADLINE, SCHEDULED, and CLOSED on a planning line between
    // the heading and the drawer. Several planning lines are legal; a blank
    // or a keyword does not end the drawer site (manual 2.7, 8.1).
    while i < lines.len() {
        let found = parse_planning_line(lines[i]);
        if found.is_empty() {
            break;
        }
        for (key, value) in found {
            if !property_order.contains(&key) {
                property_order.push(key.clone());
            }
            properties.insert(key, value);
        }
        i += 1;
    }

    while i < lines.len() {
        let trimmed = lines[i].trim();
        if trimmed.is_empty() {
            i += 1;
            continue;
        }
        if !opens_a_drawer(trimmed) {
            break;
        }
        if trimmed.eq_ignore_ascii_case(":PROPERTIES:") {
            i += 1;
            while i < lines.len() && !lines[i].trim().eq_ignore_ascii_case(":END:") {
                let line = lines[i].trim();
                if let Some(rest) = line.strip_prefix(':')
                    && let Some(idx) = rest.find(':')
                {
                    let raw_key = &rest[..idx];
                    let (key, append) = property_key_and_append(raw_key);
                    let val = rest[idx + 1..].trim().to_string();
                    if append {
                        properties
                            .entry(key.to_string())
                            .and_modify(|existing| {
                                if !val.is_empty() {
                                    if !existing.is_empty() {
                                        existing.push(' ');
                                    }
                                    existing.push_str(&val);
                                }
                            })
                            .or_insert(val);
                    } else {
                        properties.insert(key.to_string(), val);
                    }
                    if !property_order.iter().any(|k| k == key) {
                        property_order.push(key.to_string());
                    }
                }
                i += 1;
            }
            if i < lines.len() {
                i += 1;
            }
            continue;
        }
        if trimmed.eq_ignore_ascii_case(":LOGBOOK:") {
            i += 1;
            while i < lines.len() && !lines[i].trim().eq_ignore_ascii_case(":END:") {
                if !lines[i].trim().is_empty() {
                    logbook.push(parse_log_line(lines[i]));
                }
                i += 1;
            }
            if i < lines.len() {
                i += 1;
            }
            continue;
        }
        let drawer_start = i;
        i += 1;
        while i < lines.len() && !lines[i].trim().eq_ignore_ascii_case(":END:") {
            i += 1;
        }
        if i < lines.len() {
            i += 1;
        }
        let mut drawer = lines[drawer_start..i].join("\n");
        drawer.push('\n');
        extra_drawers.push(drawer);
    }

    let body_start = i;
    let mut body_end = body_start;
    let mut nest = OrgScan::new();
    while body_end < lines.len() {
        if !nest.observe(lines[body_end]) && is_top_level_headline(lines[body_end]) {
            break;
        }
        body_end += 1;
    }
    let body = lines[body_start..body_end]
        .join("\n")
        .trim_matches('\n')
        .trim_end()
        .to_string();

    // Carry a drawer written under the old name forward, so an existing
    // tracker reads the same and the next rewrite settles on the name Org
    // does not reserve.
    if let Some(legacy) = properties.remove(crate::model::LEGACY_TAGS_PROPERTY) {
        property_order.retain(|key| key != crate::model::LEGACY_TAGS_PROPERTY);
        properties
            .entry(crate::model::TAGS_PROPERTY.to_string())
            .or_insert(legacy);
    }

    let id = properties
        .get("ID")
        .cloned()
        .ok_or_else(|| anyhow!(":ID: property missing"))?;

    Ok((
        IssueHeading {
            id,
            title,
            state,
            priority,
            properties,
            org_tags,
            statistics,
            property_order,
            extra_drawers,
            body,
            logbook,
            line_start: start + 1,
            line_end: if body_end == 0 { 1 } else { body_end },
        },
        body_end,
    ))
}

/// The header a fresh project file gets.
///
/// `#+CATEGORY:` names the project, because Org otherwise takes the category
/// from the file name and every project's file is `issues.org`: an agenda
/// spanning several projects would label every row `issues`.
pub fn default_preamble(project: &str) -> String {
    format!(
        "#+TITLE: {project} issues\n#+CATEGORY: {project}\n#+FILETAGS: :issues:{project}:\n#+DATE: {}\n#+DESCRIPTION: Issue tracking file for {project} specs, plans, and implementation tasks.\n#+STATUS: Active\n{}",
        today_inactive_bracket(),
        TODO_HEADER
    )
}

/// Every project directory under the layout prefix that holds an `issues.org`.
///
/// # Errors
///
/// Returns an error if the projects directory exists but cannot be read.
pub fn list_projects(layout: &Layout) -> Result<Vec<String>> {
    let dir = layout.projects_dir();
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut projects = Vec::new();
    for entry in fs::read_dir(&dir).with_context(|| format!("read dir {}", dir.display()))? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir()
            && path.join("issues.org").exists()
            && let Some(name) = path.file_name().and_then(|n| n.to_str())
        {
            projects.push(name.to_string());
        }
    }
    projects.sort();
    Ok(projects)
}

/// Map a project name onto the directory that already exists, ignoring case.
/// An unmatched name is returned unchanged so `create` can make it.
///
/// # Errors
///
/// Returns an error if more than one project directory matches ignoring case.
pub fn resolve_existing_project_case(layout: &Layout, project: &str) -> Result<String> {
    if project.is_empty() {
        return Ok(project.to_string());
    }
    if layout.project_issues_path(project).exists() {
        return Ok(project.to_string());
    }
    let project_lower = project.to_lowercase();
    let matches: Vec<String> = list_projects(layout)?
        .into_iter()
        .filter(|candidate| candidate.to_lowercase() == project_lower)
        .collect();
    match matches.as_slice() {
        [] => Ok(project.to_string()),
        [canonical] => Ok(canonical.clone()),
        _ => Err(anyhow!(
            "project {project:?} is ambiguous; case-insensitive matches: {}",
            matches.join(", ")
        )
        .into()),
    }
}

/// Whether a project belongs to a `--project` selection.
///
/// Case folds, because the directory on disk is what names a project and
/// [`resolve_existing_project_case`] already folds case for every verb that
/// writes. A query that dropped `-p Atlas` on a tracker holding `atlas` would
/// answer "no issues" to a question that has issues.
pub fn project_selected(project: &str, filter: Option<&str>) -> bool {
    match filter {
        None => true,
        Some(p) => project.eq_ignore_ascii_case(p),
    }
}

/// Locate one issue by id across every project.
///
/// # Errors
///
/// Returns an error if a project file cannot be read or parsed.
pub fn find_by_id(layout: &Layout, id: &str) -> Result<Option<(IssueHeading, PathBuf, String)>> {
    for project in list_projects(layout)? {
        let path = layout.project_issues_path(&project);
        let doc = IssueDoc::parse_file(&project, &path)?;
        for h in doc.headings {
            if h.id == id {
                return Ok(Some((h, path, project)));
            }
        }
    }
    Ok(None)
}

/// Snapshot every heading across every project, tagged with its project.
///
/// # Errors
///
/// Returns an error if a project file cannot be read or parsed.
pub fn load_all(layout: &Layout) -> Result<Vec<(String, IssueHeading)>> {
    let mut all = Vec::new();
    for project in list_projects(layout)? {
        let path = layout.project_issues_path(&project);
        let doc = IssueDoc::parse_file(&project, &path)?;
        for h in doc.headings {
            all.push((project.clone(), h));
        }
    }
    Ok(all)
}

/// `<project>-<base36 suffix>`, retried until it does not collide.
///
/// Fails rather than looping forever when the suffix space is full. That is
/// reachable, not hypothetical: `id_length = 2` is 1296 suffixes, so a
/// project can outgrow it, and the answer is a longer id rather than a
/// crash inside a write.
///
/// # Errors
///
/// Returns an error if every suffix of `length` is already taken.
pub fn generate_id(project: &str, existing: &[String], length: usize) -> Result<String> {
    let len = length.max(2);
    let taken: std::collections::HashSet<&str> = existing.iter().map(String::as_str).collect();
    let base = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(1);
    // Bounded by the size of the space, so a full space is reported instead
    // of spun on. 36^len saturates well before it could overflow.
    let attempts = 36usize
        .checked_pow(len as u32)
        .map(|space| space.saturating_mul(2))
        .unwrap_or(usize::MAX)
        .min(2_000_000);
    for counter in 0..attempts as u128 {
        let mut n = base
            .wrapping_add(counter.wrapping_mul(17))
            .wrapping_mul(2654435761);
        let mut suffix = String::new();
        for _ in 0..len {
            suffix.push(ID_ALPHABET[(n as usize) % 36] as char);
            n /= 36;
        }
        let id = format!("{}-{}", project, suffix);
        if !taken.contains(id.as_str()) {
            return Ok(id);
        }
    }
    Err(anyhow!(
        "no free id left for {project:?} at id_length = {len}; \
         raise `id_length` under [issues] in vissue.toml"
    )
    .into())
}

/// Walk up from `start` for a `.project-ctx.toml` and read `[project].name`.
pub fn detect_project_from_ctx(start: &Path) -> Option<String> {
    let mut dir = start.canonicalize().ok()?;
    loop {
        let candidate = dir.join(".project-ctx.toml");
        if candidate.exists()
            && let Ok(text) = fs::read_to_string(&candidate)
            && let Ok(value) = text.parse::<toml::Value>()
            && let Some(name) = value
                .get("project")
                .and_then(|p| p.get("name"))
                .and_then(|n| n.as_str())
        {
            return Some(name.to_string());
        }
        if !dir.pop() {
            break;
        }
    }
    None
}

/// Look for `wanted` among every `:ID:` under the layout prefix, including
/// design documents and notes, and stop once every requested id has been
/// seen. Returns the subset that exists.
///
/// `check` uses this because a `:PARENT:` may point at a note rather than
/// at another issue.
///
/// # Errors
///
/// Returns an error if an org file under the prefix cannot be read.
pub fn find_org_ids(
    layout: &Layout,
    wanted: &std::collections::HashSet<String>,
) -> Result<std::collections::HashSet<String>> {
    let mut found = std::collections::HashSet::new();
    if wanted.is_empty() {
        return Ok(found);
    }
    let dir = layout.projects_dir();
    if !dir.exists() {
        return Ok(found);
    }
    for entry in walkdir::WalkDir::new(&dir)
        .into_iter()
        .filter_entry(|e| !is_skipped_dir(e))
    {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        if !entry.file_type().is_file()
            || entry.path().extension().and_then(|s| s.to_str()) != Some("org")
        {
            continue;
        }
        let content = fs::read_to_string(entry.path())
            .with_context(|| format!("read {}", entry.path().display()))?;
        for id in org_ids(&content) {
            if wanted.contains(id) {
                found.insert(id.to_string());
                if found.len() == wanted.len() {
                    return Ok(found);
                }
            }
        }
    }
    Ok(found)
}

/// Every `:ID:` value in any org file under the layout prefix.
///
/// # Errors
///
/// Returns an error if an org file under the prefix cannot be read.
pub fn collect_org_ids(layout: &Layout) -> Result<std::collections::HashSet<String>> {
    let mut ids = std::collections::HashSet::new();
    let dir = layout.projects_dir();
    if !dir.exists() {
        return Ok(ids);
    }
    for entry in walkdir::WalkDir::new(&dir)
        .into_iter()
        .filter_entry(|e| !is_skipped_dir(e))
    {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        if !entry.file_type().is_file()
            || entry.path().extension().and_then(|s| s.to_str()) != Some("org")
        {
            continue;
        }
        let content = fs::read_to_string(entry.path())
            .with_context(|| format!("read {}", entry.path().display()))?;
        for id in org_ids(&content) {
            ids.insert(id.to_string());
        }
    }
    Ok(ids)
}

fn is_skipped_dir(entry: &walkdir::DirEntry) -> bool {
    if !entry.file_type().is_dir() {
        return false;
    }
    let name = entry.file_name().to_string_lossy();
    matches!(
        name.as_ref(),
        "node_modules" | "target" | ".git" | ".cache" | "build"
    )
}

fn org_id_property_value(line: &str) -> Option<&str> {
    let value = line.trim_start().strip_prefix(":ID:")?.trim();
    if value.is_empty() { None } else { Some(value) }
}

/// The ids a file defines, which are the ones org would read.
///
/// A property drawer counts where org lets one start: under a headline, under
/// that headline's planning line, or beside the other drawers clustered
/// there. A `:PROPERTIES:` block further down the entry is an ordinary drawer
/// and the `:ID:` inside it is prose.
///
/// The distinction is the difference between a working `check` and a silent
/// one. Agents write their reports into issue bodies, those reports quote org,
/// and taking every `:ID:` line makes quoted text define an id: a `:PARENT:`
/// pointing at nothing resolves against a report that merely mentions it.
fn org_ids(content: &str) -> impl Iterator<Item = &str> {
    // The top of a file is a drawer site: org reads a file-level drawer there.
    let mut at_drawer_site = true;
    let mut in_drawer = false;
    let mut drawer_is_properties = false;
    // Org takes a planning line on the line under the headline and nowhere
    // else, so prose opening on `DEADLINE:` further down is prose.
    let mut under_headline = false;
    let mut nest = OrgScan::new();

    content.lines().filter_map(move |line| {
        let trimmed = line.trim();

        if in_drawer {
            if trimmed.eq_ignore_ascii_case(":END:") {
                in_drawer = false;
                drawer_is_properties = false;
                // The drawers under a headline sit together, so the next one
                // is still in a place org reads.
                at_drawer_site = true;
                return None;
            }
            if drawer_is_properties {
                return org_id_property_value(line);
            }
            return None;
        }

        // Greater blocks and Babel results are literal. A quoted headline
        // or a #+RESULTS: payload does not define an id (manual 2.8, 16).
        if nest.observe(line) {
            at_drawer_site = false;
            under_headline = false;
            return None;
        }

        if is_headline(line) {
            at_drawer_site = true;
            under_headline = true;
            return None;
        }
        let planning_may_start_here = under_headline;
        under_headline = false;

        // Neither a blank line nor a keyword is content, so neither one moves
        // the entry past the place its drawers live.
        if trimmed.is_empty() || trimmed.starts_with("#+") {
            return None;
        }
        if at_drawer_site {
            if opens_a_drawer(trimmed) {
                in_drawer = true;
                drawer_is_properties = trimmed.eq_ignore_ascii_case(":PROPERTIES:");
                return None;
            }
            if planning_may_start_here && is_planning_line(trimmed) {
                return None;
            }
        }
        // Body text: everything below it is the body.
        at_drawer_site = false;
        None
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DEFAULT_PREFIX;

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
            org_tags: Vec::new(),
            statistics: None,
            property_order: vec!["ID".into(), "CREATED".into(), "TYPE".into()],
            extra_drawers: Vec::new(),
            body: "Some body lines.\nWith multiple lines.".into(),
            logbook: Vec::new(),
            line_start: 4,
            line_end: 12,
        }
    }

    #[test]
    fn render_then_parse_preserves_the_heading() {
        let mut content = String::from("#+TITLE: sample issues\n");
        content.push_str(TODO_HEADER);
        content.push_str("\n\n");
        content.push_str(&sample_heading().render());
        let parsed = IssueDoc::parse("sample", PathBuf::from("/tmp/x.org"), &content).unwrap();
        let h = &parsed.headings[0];
        let original = sample_heading();
        assert_eq!(h.id, original.id);
        assert_eq!(h.title, original.title);
        assert_eq!(h.state, original.state);
        assert_eq!(h.priority, original.priority);
        assert_eq!(h.body, original.body);
        assert_eq!(
            crate::props::get(&h.properties, crate::props::TYPE),
            crate::props::get(&original.properties, crate::props::TYPE)
        );
    }

    #[test]
    fn heading_without_a_priority_cookie_defaults_to_c() {
        let content =
            "#+TITLE: x issues\n\n* TODO Just a title\n:PROPERTIES:\n:ID:         x-aaaa\n:END:\n";
        let doc = IssueDoc::parse("x", PathBuf::from("/tmp/x.org"), content).unwrap();
        assert_eq!(doc.headings[0].priority, 'C');
        assert_eq!(doc.headings[0].title, "Just a title");
    }

    #[test]
    fn a_multibyte_priority_cookie_parses_instead_of_panicking() {
        let content = "#+TITLE: x issues\n\n* TODO [#\u{2192}] Hand edited\n:PROPERTIES:\n:ID:         x-aaaa\n:END:\n";
        let doc = IssueDoc::parse("x", PathBuf::from("/tmp/x.org"), content).unwrap();
        assert_eq!(doc.headings[0].priority, '\u{2192}');
        assert_eq!(doc.headings[0].title, "Hand edited");
    }

    #[test]
    fn a_title_opening_with_a_bracket_keeps_its_text() {
        let content = "#+TITLE: x issues\n\n* TODO [#not a cookie] stays\n:PROPERTIES:\n:ID:         x-bbbb\n:END:\n";
        let doc = IssueDoc::parse("x", PathBuf::from("/tmp/x.org"), content).unwrap();
        assert_eq!(doc.headings[0].priority, 'C');
        assert_eq!(doc.headings[0].title, "[#not a cookie] stays");
    }

    #[test]
    fn an_org_planning_line_parses_instead_of_hiding_the_drawer() {
        // What `C-c C-d`, `C-c C-s`, and `org-log-done` write in Emacs. Before
        // the planning line was read, the drawer below it went unseen and the
        // whole file failed with ":ID: property missing".
        let content = "#+TITLE: x issues\n\n* DONE [#A] Ship it\nCLOSED: [2026-08-14 Fri 03:33] SCHEDULED: <2026-09-05 Sat> DEADLINE: <2026-09-01 Tue>\n:PROPERTIES:\n:ID:         x-aaaa\n:END:\n\nBody stays body.\n";
        let doc = IssueDoc::parse("x", PathBuf::from("/tmp/x.org"), content).unwrap();
        let h = &doc.headings[0];
        assert_eq!(h.id, "x-aaaa");
        assert_eq!(h.deadline(), Some("<2026-09-01 Tue>"));
        assert_eq!(h.scheduled(), Some("<2026-09-05 Sat>"));
        assert_eq!(
            h.properties.get("CLOSED").map(String::as_str),
            Some("[2026-08-14 Fri 03:33]")
        );
        assert_eq!(h.body, "Body stays body.");
    }

    #[test]
    fn a_planning_line_round_trips_in_orgs_own_order() {
        let content = "#+TITLE: x issues\n\n* DONE [#A] Ship it\nCLOSED: [2026-08-14 Fri 03:33] SCHEDULED: <2026-09-05 Sat> DEADLINE: <2026-09-01 Tue>\n:PROPERTIES:\n:ID:         x-aaaa\n:END:\n";
        let doc = IssueDoc::parse("x", PathBuf::from("/tmp/x.org"), content).unwrap();
        let rendered = doc.headings[0].render();
        assert!(
            rendered.contains(
                "\nCLOSED: [2026-08-14 Fri 03:33] SCHEDULED: <2026-09-05 Sat> DEADLINE: <2026-09-01 Tue>\n"
            ),
            "{rendered}"
        );
        assert!(!rendered.contains(":DEADLINE:"), "{rendered}");
    }

    #[test]
    fn a_legacy_date_property_is_promoted_to_a_planning_line() {
        // Trackers written before dates moved out of the drawer still parse,
        // and the next rewrite puts them where Org's agenda reads them.
        let content = "#+TITLE: x issues\n\n* TODO [#A] Ship it\n:PROPERTIES:\n:ID:         x-aaaa\n:DEADLINE:   <2026-09-01 Tue>\n:END:\n";
        let doc = IssueDoc::parse("x", PathBuf::from("/tmp/x.org"), content).unwrap();
        assert_eq!(doc.headings[0].deadline(), Some("<2026-09-01 Tue>"));
        let rendered = doc.headings[0].render();
        assert!(
            rendered.contains("\nDEADLINE: <2026-09-01 Tue>\n"),
            "{rendered}"
        );
        assert!(!rendered.contains(":DEADLINE:"), "{rendered}");
    }

    #[test]
    fn a_line_that_only_looks_like_planning_is_left_as_body() {
        let content = "#+TITLE: x issues\n\n* TODO [#A] Ship it\n:PROPERTIES:\n:ID:         x-aaaa\n:END:\n\nDEADLINE: is discussed in the design note.\n";
        let doc = IssueDoc::parse("x", PathBuf::from("/tmp/x.org"), content).unwrap();
        assert_eq!(doc.headings[0].deadline(), None);
        assert_eq!(
            doc.headings[0].body,
            "DEADLINE: is discussed in the design note."
        );
    }

    #[test]
    fn a_legacy_tags_property_moves_to_the_name_org_leaves_alone() {
        // `TAGS` is one of Org's own special property names, so a drawer that
        // claims it is what `org-lint` reports. Existing trackers still read.
        let content = "#+TITLE: x issues\n\n* TODO [#A] Ship it\n:PROPERTIES:\n:ID:         x-aaaa\n:TAGS:       needs-review,perf\n:END:\n";
        let doc = IssueDoc::parse("x", PathBuf::from("/tmp/x.org"), content).unwrap();
        let h = &doc.headings[0];
        assert_eq!(h.tags(), vec!["needs-review", "perf"]);
        let rendered = h.render();
        assert!(
            rendered.contains(":VISSUE_TAGS: needs-review"),
            "{rendered}"
        );
        assert!(rendered.contains(":perf:"), "{rendered}");
        assert!(!rendered.contains(":TAGS:"), "{rendered}");
    }

    #[test]
    fn bodies_end_at_the_next_heading() {
        let content = "#+TITLE: x issues\n\n* TODO [#A] First\n:PROPERTIES:\n:ID:         x-1111\n:END:\n\nFirst body.\n\n* DONE [#C] Second\n:PROPERTIES:\n:ID:         x-2222\n:END:\n\nSecond body.\nMulti.\n";
        let doc = IssueDoc::parse("x", PathBuf::from("/tmp/x.org"), content).unwrap();
        assert_eq!(doc.headings.len(), 2);
        assert_eq!(doc.headings[0].body, "First body.");
        assert_eq!(doc.headings[1].body, "Second body.\nMulti.");
    }

    #[test]
    fn logbook_survives_a_document_round_trip() {
        let mut h = sample_heading();
        h.logbook = vec![LogEntry {
            timestamp: "[2026-04-26 Sun 09:15]".into(),
            from_state: Some("TODO".into()),
            to_state: Some("STARTED".into()),
            note: None,
            raw: None,
        }];
        let mut content = String::from("#+TITLE: sample issues\n\n");
        content.push_str(&h.render());
        let parsed = IssueDoc::parse("sample", PathBuf::from("/tmp/x.org"), &content).unwrap();
        assert_eq!(parsed.headings[0].logbook.len(), 1);
        assert_eq!(
            parsed.headings[0].logbook[0].from_state.as_deref(),
            Some("TODO")
        );
    }

    #[test]
    fn write_then_reread_finds_the_heading() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Software/sample/issues.org");
        IssueDoc {
            project: "sample".into(),
            path: path.clone(),
            preamble: default_preamble("sample"),
            headings: vec![sample_heading()],
            after: vec![String::new()],
        }
        .write()
        .unwrap();
        let parsed = IssueDoc::parse_file("sample", &path).unwrap();
        assert_eq!(parsed.headings[0].id, "sample-abc1");
    }

    #[test]
    fn write_preserves_the_existing_preamble_and_property_order() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Software/sample/issues.org");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(
            &path,
            "#+TITLE: sample issues\n#+FILETAGS: :issues:sample:\n#+STATUS: Active\n#+TODO: TODO STARTED BLOCKED | DONE CANCELLED\n\n* TODO [#A] Existing issue\n:PROPERTIES:\n:ID:         sample-abc1\n:CREATED:    [2026-04-26 Sun]\n:TYPE:       spec\n:PARENT:     sample-root\n:END:\n",
        )
        .unwrap();

        let mut doc = IssueDoc::parse_file("sample", &path).unwrap();
        doc.headings[0].priority = 'B';
        doc.write().unwrap();
        let written = fs::read_to_string(&path).unwrap();

        assert!(written.contains("#+FILETAGS: :issues:sample:"), "{written}");
        assert!(written.contains("#+CATEGORY: sample"), "{written}");
        assert!(written.contains("#+STATUS: Active"), "{written}");
        assert!(
            written.find(":TYPE:").unwrap() < written.find(":PARENT:").unwrap(),
            "{written}"
        );
    }

    #[test]
    fn an_empty_document_writes_the_house_preamble() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Software/sample/issues.org");
        IssueDoc::empty("sample", path.clone()).write().unwrap();
        let written = fs::read_to_string(&path).unwrap();
        for expected in [
            "#+TITLE: sample issues",
            // Org takes the category from the file name otherwise, and every
            // project's file is issues.org.
            "#+CATEGORY: sample",
            "#+FILETAGS: :issues:sample:",
            "#+DATE:",
            "#+STATUS: Active",
            TODO_HEADER,
        ] {
            assert!(written.contains(expected), "missing {expected}: {written}");
        }
    }

    #[test]
    fn generated_ids_are_unique_and_sized() {
        let existing = vec!["p-aaaa".to_string()];
        let id = generate_id("p", &existing, 4).unwrap();
        assert!(id.starts_with("p-"));
        assert!(!existing.contains(&id));
        assert_eq!(id.len(), 1 + 1 + 4);
        assert_eq!(generate_id("q", &[], 6).unwrap().len(), 1 + 1 + 6);
    }

    #[test]
    fn a_full_suffix_space_is_an_error_and_not_a_panic() {
        // id_length 2 is 36^2 suffixes. Hand it every one of them and it has
        // to say so rather than spin or abort inside a write.
        let mut existing = Vec::new();
        for a in ID_ALPHABET {
            for b in ID_ALPHABET {
                existing.push(format!("p-{}{}", *a as char, *b as char));
            }
        }
        let err = generate_id("p", &existing, 2).unwrap_err();
        assert!(err.to_string().contains("id_length"), "{err}");
        // One free suffix is still found.
        existing.pop();
        assert!(generate_id("p", &existing, 2).is_ok());
    }

    #[test]
    fn projects_are_discovered_under_the_configured_prefix() {
        let dir = tempfile::tempdir().unwrap();
        let layout = Layout::new(dir.path(), "tracker");
        for project in ["beta", "alpha"] {
            IssueDoc::empty(project, layout.project_issues_path(project))
                .write()
                .unwrap();
        }
        assert_eq!(list_projects(&layout).unwrap(), vec!["alpha", "beta"]);
        assert!(
            list_projects(&Layout::new(dir.path(), DEFAULT_PREFIX))
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn project_case_resolves_to_the_directory_on_disk() {
        let dir = tempfile::tempdir().unwrap();
        let layout = Layout::new(dir.path(), DEFAULT_PREFIX);
        IssueDoc::empty("MixedCase", layout.project_issues_path("MixedCase"))
            .write()
            .unwrap();
        assert_eq!(
            resolve_existing_project_case(&layout, "mixedcase").unwrap(),
            "MixedCase"
        );
        assert_eq!(
            resolve_existing_project_case(&layout, "brand-new").unwrap(),
            "brand-new"
        );
    }

    #[test]
    fn project_context_file_is_found_by_walking_up() {
        let dir = tempfile::tempdir().unwrap();
        let nested = dir.path().join("a/b/c");
        fs::create_dir_all(&nested).unwrap();
        fs::write(
            dir.path().join(".project-ctx.toml"),
            "[project]\nname = \"demoproj\"\n",
        )
        .unwrap();
        assert_eq!(
            detect_project_from_ctx(&nested).as_deref(),
            Some("demoproj")
        );
        let empty = tempfile::tempdir().unwrap();
        assert!(detect_project_from_ctx(empty.path()).is_none());
    }

    fn ids(content: &str) -> Vec<&str> {
        org_ids(content).collect()
    }

    #[test]
    fn a_drawer_under_a_headline_defines_an_id() {
        assert_eq!(
            ids("* TODO [#B] a title\n:PROPERTIES:\n:ID:         atlas-1a2b\n:END:\n"),
            ["atlas-1a2b"]
        );
    }

    #[test]
    fn a_drawer_under_the_planning_line_defines_an_id() {
        let text = concat!(
            "* TODO a title\n",
            "DEADLINE: <2026-05-15 Fri>\n",
            ":PROPERTIES:\n",
            ":ID:         atlas-1a2b\n",
            ":END:\n",
        );
        assert_eq!(ids(text), ["atlas-1a2b"]);
    }

    #[test]
    fn a_logbook_beside_the_properties_does_not_hide_it() {
        let text = concat!(
            "* TODO a title\n",
            ":LOGBOOK:\n",
            "- claimed by worker-1\n",
            ":END:\n",
            ":PROPERTIES:\n",
            ":ID:         atlas-1a2b\n",
            ":END:\n",
        );
        assert_eq!(ids(text), ["atlas-1a2b"]);
    }

    #[test]
    fn an_id_quoted_in_a_body_defines_nothing() {
        // What an agent writes back when its report quotes the tracker.
        let text = concat!(
            "* TODO a title\n",
            ":PROPERTIES:\n",
            ":ID:         atlas-1a2b\n",
            ":END:\n",
            "\n",
            "The heading I was handed reads:\n",
            ":PROPERTIES:\n",
            ":ID: ghost-9999\n",
            ":END:\n",
        );
        assert_eq!(
            ids(text),
            ["atlas-1a2b"],
            "a report that quotes an id defined it"
        );
    }

    #[test]
    fn a_bare_id_line_in_a_body_defines_nothing() {
        let text = concat!(
            "* TODO a title\n",
            ":PROPERTIES:\n",
            ":ID:         atlas-1a2b\n",
            ":END:\n",
            "\n",
            "Compare with :ID: ghost-9999 in the other file.\n",
            ":ID: ghost-8888\n",
        );
        assert_eq!(ids(text), ["atlas-1a2b"]);
    }

    #[test]
    fn a_file_level_drawer_defines_an_id() {
        // Org reads one at the top, above every headline.
        let text = concat!(
            "#+TITLE: atlas issues\n",
            "\n",
            ":PROPERTIES:\n",
            ":ID: the-file-itself\n",
            ":END:\n",
            "\n",
            "* TODO a title\n",
            ":PROPERTIES:\n",
            ":ID:         atlas-1a2b\n",
            ":END:\n",
        );
        assert_eq!(ids(text), ["the-file-itself", "atlas-1a2b"]);
    }

    #[test]
    fn every_headline_depth_opens_a_drawer_site() {
        let text = concat!(
            "* TODO a title\n",
            ":PROPERTIES:\n",
            ":ID:         atlas-1a2b\n",
            ":END:\n",
            "** A sub-heading someone wrote by hand\n",
            ":PROPERTIES:\n",
            ":ID:         atlas-3c4d\n",
            ":END:\n",
        );
        assert_eq!(ids(text), ["atlas-1a2b", "atlas-3c4d"]);
    }

    #[test]
    fn a_planning_keyword_needs_its_colon() {
        assert!(is_planning_line("DEADLINE: <2026-05-15 Fri>"));
        assert!(is_planning_line("CLOSED: [2026-05-15 Fri]"));
        // Prose that opens on the same word is prose.
        assert!(!is_planning_line("DEADLINES slipped again"));
        assert!(!is_planning_line("SCHEDULED work for the week"));
    }

    #[test]
    fn prose_that_opens_like_a_planning_line_still_ends_the_drawer_site() {
        // Org reads a planning line directly under the headline and nowhere
        // else, so this one is body text and the quoted drawer below it is
        // body text too.
        let text = concat!(
            "* TODO a title\n",
            ":PROPERTIES:\n",
            ":ID:         atlas-1a2b\n",
            ":END:\n",
            "\n",
            "DEADLINE: is discussed in the design note.\n",
            ":PROPERTIES:\n",
            ":ID: ghost-9999\n",
            ":END:\n",
        );
        assert_eq!(ids(text), ["atlas-1a2b"]);
    }

    #[test]
    fn a_headline_needs_a_space_after_its_stars() {
        assert!(is_headline("* TODO a title"));
        assert!(is_headline("*** deeper"));
        assert!(!is_headline("**bold** at the start of a line"));
        assert!(!is_headline("not a headline"));
    }

    #[test]
    fn a_source_block_does_not_split_an_issue() {
        let content = concat!(
            "#+TITLE: x issues\n\n",
            "* TODO [#A] Real issue\n",
            ":PROPERTIES:\n",
            ":ID:         x-aaaa\n",
            ":END:\n\n",
            "Quoted tracker:\n",
            "#+BEGIN_SRC org\n",
            "* TODO quoted\n",
            ":PROPERTIES:\n",
            ":ID:         ghost-9999\n",
            ":END:\n",
            "#+END_SRC\n\n",
            "Still the same issue.\n",
        );
        let doc = IssueDoc::parse("x", PathBuf::from("/tmp/x.org"), content).unwrap();
        assert_eq!(
            doc.headings.len(),
            1,
            "{:?}",
            doc.headings.iter().map(|h| &h.id).collect::<Vec<_>>()
        );
        assert_eq!(doc.headings[0].id, "x-aaaa");
        assert!(
            doc.headings[0].body.contains("* TODO quoted"),
            "{}",
            doc.headings[0].body
        );
        assert!(doc.headings[0].body.contains("Still the same issue."));
    }

    #[test]
    fn org_ids_ignore_a_drawer_inside_a_block() {
        let text = concat!(
            "#+BEGIN_SRC org\n",
            "* TODO quoted\n",
            ":PROPERTIES:\n",
            ":ID:         ghost-9999\n",
            ":END:\n",
            "#+END_SRC\n",
            "* TODO a title\n",
            ":PROPERTIES:\n",
            ":ID:         atlas-1a2b\n",
            ":END:\n",
        );
        assert_eq!(ids(text), ["atlas-1a2b"]);
    }

    #[test]
    fn a_timestamp_range_on_the_planning_line_parses() {
        let content = concat!(
            "#+TITLE: x issues\n\n",
            "* TODO [#A] Sprint\n",
            "SCHEDULED: <2026-09-01 Tue>--<2026-09-08 Tue> DEADLINE: <2026-09-15 Mon +1w>\n",
            ":PROPERTIES:\n",
            ":ID:         x-aaaa\n",
            ":END:\n",
        );
        let doc = IssueDoc::parse("x", PathBuf::from("/tmp/x.org"), content).unwrap();
        assert_eq!(doc.headings[0].id, "x-aaaa");
        assert_eq!(
            doc.headings[0].scheduled(),
            Some("<2026-09-01 Tue>--<2026-09-08 Tue>")
        );
        assert_eq!(doc.headings[0].deadline(), Some("<2026-09-15 Mon +1w>"));
    }

    #[test]
    fn a_repeater_and_warning_on_the_planning_line_parse() {
        let content = concat!(
            "#+TITLE: x issues\n\n",
            "* TODO [#A] Weekly\n",
            "DEADLINE: <2026-09-01 Tue +1w -2d>\n",
            ":PROPERTIES:\n",
            ":ID:         x-aaaa\n",
            ":END:\n",
        );
        let doc = IssueDoc::parse("x", PathBuf::from("/tmp/x.org"), content).unwrap();
        assert_eq!(doc.headings[0].deadline(), Some("<2026-09-01 Tue +1w -2d>"));
        let rendered = doc.headings[0].render();
        assert!(
            rendered.contains("DEADLINE: <2026-09-01 Tue +1w -2d>"),
            "{rendered}"
        );
    }

    #[test]
    fn a_logbook_before_properties_still_parses() {
        let content = concat!(
            "#+TITLE: x issues\n\n",
            "* TODO [#A] Clocked\n",
            ":LOGBOOK:\n",
            "CLOCK: [2026-08-18 Tue 10:00]--[2026-08-18 Tue 11:00] =>  1:00\n",
            ":END:\n",
            ":PROPERTIES:\n",
            ":ID:         x-aaaa\n",
            ":END:\n",
        );
        let doc = IssueDoc::parse("x", PathBuf::from("/tmp/x.org"), content).unwrap();
        assert_eq!(doc.headings[0].id, "x-aaaa");
        assert_eq!(doc.headings[0].logbook.len(), 1);
        assert!(doc.headings[0].logbook[0].raw.is_some());
    }

    #[test]
    fn other_drawers_at_the_drawer_site_do_not_hide_the_id() {
        let content = concat!(
            "#+TITLE: x issues\n\n",
            "* TODO [#A] Notes drawer\n",
            ":NOTES:\n",
            "hand written\n",
            ":END:\n",
            ":PROPERTIES:\n",
            ":ID:         x-aaaa\n",
            ":END:\n\n",
            "Body stays.\n",
        );
        let doc = IssueDoc::parse("x", PathBuf::from("/tmp/x.org"), content).unwrap();
        assert_eq!(doc.headings[0].id, "x-aaaa");
        assert_eq!(doc.headings[0].body, "Body stays.");
        let rendered = doc.headings[0].render();
        assert!(rendered.contains(":NOTES:"), "{rendered}");
        assert!(rendered.contains("hand written"), "{rendered}");
    }

    #[test]
    fn a_comment_heading_is_not_an_issue() {
        let content = concat!(
            "#+TITLE: x issues\n\n",
            "* COMMENT Archived discussion\n",
            ":PROPERTIES:\n",
            ":ID:         ghost-old\n",
            ":END:\n\n",
            "* TODO [#A] Live\n",
            ":PROPERTIES:\n",
            ":ID:         x-aaaa\n",
            ":END:\n",
        );
        let doc = IssueDoc::parse("x", PathBuf::from("/tmp/x.org"), content).unwrap();
        assert_eq!(doc.headings.len(), 1);
        assert_eq!(doc.headings[0].id, "x-aaaa");
        assert!(doc.preamble.contains("COMMENT Archived"));
    }

    #[test]
    fn a_section_heading_round_trips() {
        let content = concat!(
            "#+TITLE: x issues\n\n",
            "* TODO [#A] First\n",
            ":PROPERTIES:\n",
            ":ID:         x-aaaa\n",
            ":END:\n\n",
            "First body.\n\n",
            "* Notes\n",
            "Hand-written section.\n\n",
            "* TODO [#B] Second\n",
            ":PROPERTIES:\n",
            ":ID:         x-bbbb\n",
            ":END:\n",
        );
        let doc = IssueDoc::parse("x", PathBuf::from("/tmp/x.org"), content).unwrap();
        assert_eq!(doc.headings.len(), 2);
        assert_eq!(doc.headings[0].body, "First body.");
        assert!(doc.after[0].contains("* Notes"));
        assert!(doc.after[0].contains("Hand-written section."));
        let rendered = doc.headings[0].render();
        let mut file = String::from("#+TITLE: x issues\n\n");
        file.push_str(&rendered);
        file.push('\n');
        file.push_str(&doc.after[0]);
        file.push_str(&doc.headings[1].render());
        let again = IssueDoc::parse("x", PathBuf::from("/tmp/x.org"), &file).unwrap();
        assert_eq!(again.headings.len(), 2);
        assert!(again.after[0].contains("* Notes"));
    }

    #[test]
    fn a_file_local_todo_keyword_is_an_issue() {
        let content = concat!(
            "#+TITLE: x issues\n",
            "#+TODO: TODO WAIT | DONE\n\n",
            "* WAIT [#B] Parked\n",
            ":PROPERTIES:\n",
            ":ID:         x-aaaa\n",
            ":END:\n",
        );
        let doc = IssueDoc::parse("x", PathBuf::from("/tmp/x.org"), content).unwrap();
        assert_eq!(doc.headings.len(), 1);
        assert_eq!(doc.headings[0].state, "WAIT");
        assert_eq!(doc.headings[0].id, "x-aaaa");
    }

    #[test]
    fn a_statistics_cookie_is_not_the_title() {
        let content = concat!(
            "#+TITLE: x issues\n\n",
            "* TODO [#A] Break it down [2/5]           :plan:\n",
            ":PROPERTIES:\n",
            ":ID:         x-aaaa\n",
            ":END:\n",
        );
        let doc = IssueDoc::parse("x", PathBuf::from("/tmp/x.org"), content).unwrap();
        assert_eq!(doc.headings[0].title, "Break it down");
        assert_eq!(doc.headings[0].statistics.as_deref(), Some("[2/5]"));
        assert_eq!(doc.headings[0].org_tags, vec!["plan"]);
        let line = doc.headings[0].render().lines().next().unwrap().to_string();
        assert!(line.contains("[2/5]"), "{line:?}");
        assert!(line.contains(":plan:"), "{line:?}");
    }

    #[test]
    fn a_property_plus_appends() {
        let content = concat!(
            "#+TITLE: x issues\n\n",
            "* TODO [#A] Blocked\n",
            ":PROPERTIES:\n",
            ":ID:         x-aaaa\n",
            ":BLOCKED_BY: x-bbbb\n",
            ":BLOCKED_BY+: x-cccc\n",
            ":END:\n",
        );
        let doc = IssueDoc::parse("x", PathBuf::from("/tmp/x.org"), content).unwrap();
        assert_eq!(doc.headings[0].blocked_by(), vec!["x-bbbb", "x-cccc"]);
    }

    #[test]
    fn babel_results_do_not_split_an_issue_or_define_an_id() {
        let content = concat!(
            "#+TITLE: x issues\n\n",
            "* TODO [#A] Real issue\n",
            ":PROPERTIES:\n",
            ":ID:         x-aaaa\n",
            ":END:\n\n",
            "#+NAME: dump\n",
            "#+HEADER: :results raw\n",
            "#+BEGIN_SRC python :results raw\n",
            "print('* TODO dumped')\n",
            "#+END_SRC\n\n",
            "#+RESULTS:\n",
            "* TODO dumped\n",
            ":PROPERTIES:\n",
            ":ID:         ghost-9999\n",
            ":END:\n\n",
            "Still the same issue.\n\n",
            "* TODO [#B] Next\n",
            ":PROPERTIES:\n",
            ":ID:         x-bbbb\n",
            ":END:\n",
        );
        let doc = IssueDoc::parse("x", PathBuf::from("/tmp/x.org"), content).unwrap();
        assert_eq!(
            doc.headings
                .iter()
                .map(|h| h.id.as_str())
                .collect::<Vec<_>>(),
            ["x-aaaa", "x-bbbb"]
        );
        assert!(
            doc.headings[0].body.contains("#+RESULTS:"),
            "{}",
            doc.headings[0].body
        );
        assert!(
            doc.headings[0].body.contains("* TODO dumped"),
            "{}",
            doc.headings[0].body
        );
        assert!(doc.headings[0].body.contains("Still the same issue."));
        assert_eq!(ids(content), ["x-aaaa", "x-bbbb"]);
    }

    #[test]
    fn a_babel_call_with_results_drawer_stays_in_the_body() {
        let content = concat!(
            "#+TITLE: x issues\n\n",
            "* TODO [#A] Calls a named block\n",
            ":PROPERTIES:\n",
            ":ID:         x-aaaa\n",
            ":END:\n\n",
            "#+CALL: plot(x=1) :results drawer\n",
            "#+RESULTS:\n",
            ":RESULTS:\n",
            "* TODO not an issue\n",
            ":END:\n",
        );
        let doc = IssueDoc::parse("x", PathBuf::from("/tmp/x.org"), content).unwrap();
        assert_eq!(doc.headings.len(), 1);
        assert!(doc.headings[0].body.contains("#+CALL: plot"));
        assert!(doc.headings[0].body.contains("* TODO not an issue"));
        assert_eq!(ids(content), ["x-aaaa"]);
    }
}
