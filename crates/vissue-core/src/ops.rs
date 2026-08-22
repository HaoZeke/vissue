//! Mutating verbs: create, update, and move issues between projects.

use anyhow::{Context, anyhow};

use crate::error::Result;
use chrono::NaiveDate;
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use crate::config::{Layout, VissueConfig};
use crate::error::Error;
use crate::graph::DependencyGraph;
use crate::model::{IssueHeading, LogEntry, TODO_KEYWORDS, today_inactive_bracket};
use crate::store::{
    IssueDoc, collect_org_ids, detect_project_from_ctx, find_by_id, generate_id, load_all,
    resolve_existing_project_case, with_issues_lock, with_issues_locks,
};

/// Resolve the project to act on. An explicit name wins; otherwise walk up from
/// the current directory for `.project-ctx.toml` and read `[project].name`.
/// Neither available is an error, so nothing is ever guessed silently.
///
/// # Errors
///
/// Returns an error if the explicit project name is empty, no name can be
/// resolved, the current directory cannot be read, or the name matches more
/// than one project directory.
pub fn resolve_project(layout: &Layout, explicit: Option<&str>) -> Result<String> {
    if let Some(p) = explicit {
        if p.is_empty() {
            return Err(anyhow!("--project given but empty").into());
        }
        return resolve_existing_project_case(layout, p);
    }
    let cwd = std::env::current_dir()?;
    let detected = detect_project_from_ctx(&cwd).ok_or_else(|| {
        anyhow!(
            "no --project given and no .project-ctx.toml found walking up from {}",
            cwd.display()
        )
    })?;
    resolve_existing_project_case(layout, &detected)
}

/// Optional fields on a new issue.
#[derive(Debug, Default, Clone, Copy)]
pub struct CreateOpts<'a> {
    /// Priority cookie; the configured default is used when `None`.
    pub priority: Option<char>,
    /// `:TYPE:` property.
    pub issue_type: Option<&'a str>,
    /// Deadline as an org timestamp.
    pub deadline: Option<&'a str>,
    /// Scheduled date as an org timestamp.
    pub scheduled: Option<&'a str>,
    /// Comma- or colon-separated tags.
    pub tags: Option<&'a str>,
    /// `:PARENT:` id; must already exist somewhere under the prefix.
    pub parent: Option<&'a str>,
    /// Print only the new id.
    pub quiet: bool,
    /// Body prose written under the properties drawer.
    pub body: Option<&'a str>,
    /// Extra ids treated as taken when minting, so a twin file on another
    /// layout cannot share a suffix with this create.
    pub extra_ids: &'a [String],
    /// Twin files whose ids are read *inside* the lock and treated as taken.
    ///
    /// [`Self::extra_ids`] is a snapshot the caller took before calling, which
    /// is a read outside the lock that guards the write. Two creates for one
    /// project in two roots each read the other before either writes, hold
    /// different locks because locks are per file, and can mint one suffix
    /// twice. `find_by_id` then reports `DuplicateId` and neither issue is
    /// reachable by id.
    ///
    /// Paths given here are locked alongside the file being written and read
    /// after the lock is held, so a peer's create is either wholly before or
    /// wholly after this one.
    pub extra_id_paths: &'a [PathBuf],
}

/// Append a new TODO issue to the project's file and return the status text.
///
/// The first `[[id:XXX]]` in `body` that names a heading already in the
/// corpus becomes `:DISCOVERED_FROM:`, unless that property is already set.
/// Prose never writes `:BLOCKED_BY:`.
///
/// # Errors
///
/// Returns an error if the priority is not `A`/`B`/`C`, a date does not parse,
/// `parent` is not a known org id, the id space is exhausted, or the file
/// cannot be locked or rewritten.
pub fn create(layout: &Layout, project: &str, title: &str, opts: CreateOpts<'_>) -> Result<String> {
    let project = resolve_existing_project_case(layout, project)?;
    let cfg = VissueConfig::load(layout)?;
    let path = layout.project_issues_path(&project);
    let (spec, named) = match IssueDoc::parse_file(&project, &path) {
        Ok(doc) => (doc.priority_spec(), doc.priorities_are_named()),
        Err(_) => (crate::org::PrioritySpec::default(), false),
    };
    let house_new = !path.exists();
    let priority = opts.priority.unwrap_or(if named || house_new {
        spec.default
    } else {
        cfg.issues.default_priority
    });
    if !spec.contains(priority) {
        return Err(anyhow!(
            "invalid priority {priority:?}; file allows [#{}]..[#{}]",
            spec.highest,
            spec.lowest
        )
        .into());
    }

    // Parent and body [[id:]] both need the corpus id set; scan once.
    let known_ids = if opts.parent.is_some() || opts.body.is_some() {
        collect_org_ids(layout)?
    } else {
        std::collections::HashSet::new()
    };
    if let Some(p) = opts.parent
        && !known_ids.contains(p)
    {
        return Err(anyhow!("--parent {p} does not refer to any known id").into());
    }

    // Every file the mint consults is locked, not only the one it writes, so a
    // twin create in another root cannot land between the read and the write.
    // with_issues_locks sorts and dedups, so the write path appearing in
    // extra_id_paths is normal rather than a self-deadlock.
    let mut lock_paths: Vec<PathBuf> = vec![path.clone()];
    lock_paths.extend(opts.extra_id_paths.iter().cloned());
    let lock_refs: Vec<&Path> = lock_paths.iter().map(PathBuf::as_path).collect();
    with_issues_locks(&lock_refs, || {
        let mut doc = IssueDoc::parse_file(&project, &path)?;
        let mut taken = doc.known_ids();
        taken.extend(opts.extra_ids.iter().cloned());
        for twin in opts.extra_id_paths {
            if twin == &path {
                continue;
            }
            if let Ok(doc) = IssueDoc::parse_file(&project, twin) {
                taken.extend(doc.known_ids());
            }
        }
        let id = generate_id(&project, &taken, cfg.issues.id_length)?;

        let mut props = BTreeMap::new();
        props.insert("ID".into(), id.clone());
        props.insert("CREATED".into(), today_inactive_bracket());
        if crate::props::get(&props, crate::props::DISCOVERED_FROM).is_none()
            && let Some(body) = opts.body
            && let Some(origin) = first_existing_id_link(body, &known_ids)
        {
            crate::props::insert(&mut props, crate::props::DISCOVERED_FROM, origin);
        }
        let mut org_tags: Vec<String> = Vec::new();
        if let Some(t) = opts.issue_type {
            crate::props::insert(&mut props, crate::props::TYPE, t.into());
            // Type is an Org tag when the character class allows it, so
            // agenda tag search and C-c \ see `bug` / `feature` / `task`.
            if t.chars().all(crate::model::is_org_tag_char)
                && !t.is_empty()
                && !org_tags.iter().any(|seen| seen == t)
            {
                org_tags.push(t.to_string());
            }
        }
        if let Some(d) = opts.deadline {
            validate_org_date(d)?;
            props.insert("DEADLINE".into(), d.into());
        }
        if let Some(s) = opts.scheduled {
            validate_org_date(s)?;
            props.insert("SCHEDULED".into(), s.into());
        }
        // A tag Org can hold goes on the heading, where Org's own tag search
        // and agenda read it. One Org would not accept, `needs-review` say,
        // stays in the property so it survives instead of becoming title text.
        if let Some(tags) = opts.tags {
            let mut property_tags: Vec<String> = Vec::new();
            for tag in tags.split([',', ':']).map(str::trim) {
                if tag.is_empty() {
                    continue;
                }
                if tag.chars().all(crate::model::is_org_tag_char) {
                    if !org_tags.iter().any(|seen| seen == tag) {
                        org_tags.push(tag.to_string());
                    }
                } else if !property_tags.iter().any(|seen| seen == tag) {
                    property_tags.push(tag.to_string());
                }
            }
            if !property_tags.is_empty() {
                props.insert(crate::model::TAGS_PROPERTY.into(), property_tags.join(","));
            }
        }
        if let Some(p) = opts.parent {
            crate::props::insert(&mut props, crate::props::PARENT, p.into());
        }

        doc.headings.push(IssueHeading {
            id: id.clone(),
            title: title.to_string(),
            state: "TODO".into(),
            priority,
            properties: props,
            org_tags,
            statistics: None,
            property_order: Vec::new(),
            extra_drawers: Vec::new(),
            body: match opts.body {
                Some(b) if !b.trim().is_empty() => format!("{}\n", b.trim_end()),
                _ => String::new(),
            },
            logbook: Vec::new(),
            line_start: 0,
            line_end: 0,
        });
        doc.write()?;

        if opts.quiet {
            Ok(format!("{id}\n"))
        } else {
            Ok(format!(
                "{id}  TODO  [#{priority}]  {title}\nfile: {}\n",
                path.display()
            ))
        }
    })
}

pub(crate) fn validate_org_date(s: &str) -> Result<()> {
    let inner = s
        .trim_start_matches(['<', '['])
        .trim_end_matches(['>', ']']);
    let token = inner.split_whitespace().next().unwrap_or("");
    NaiveDate::parse_from_str(token, "%Y-%m-%d").with_context(|| {
        format!("expected org date like <YYYY-MM-DD> or [YYYY-MM-DD], got {s:?}")
    })?;
    Ok(())
}

/// Change state, priority, or blocker edges. Adding a blocker to an open issue
/// moves it to BLOCKED; clearing the last blocker moves it back to TODO.
///
/// # Errors
///
/// Returns an error if `id` is not in the corpus, the state or priority is
/// invalid, adding the blocker would cycle, or the file cannot be rewritten.
pub fn update(
    layout: &Layout,
    id: &str,
    new_state: Option<&str>,
    new_priority: Option<char>,
    block_add: Option<&str>,
    block_clear: Option<&str>,
) -> Result<UpdateOutcome> {
    let identity = crate::config::identity(layout);
    update_as(
        layout,
        id,
        new_state,
        new_priority,
        block_add,
        block_clear,
        &identity,
    )
}

/// Last-seen state or generation a write must still match.
///
/// This is the causal context on a PUT: the caller read the heading, then
/// writes only if nothing else closed or rewrote it.
#[derive(Debug, Default, Clone, Copy)]
pub struct UpdatePred<'a> {
    /// Refuse unless the heading is still this state.
    pub if_state: Option<&'a str>,
    /// Refuse unless the corpus generation is still this value.
    pub if_gen: Option<u64>,
}

/// [`update`] with a last-seen predicate.
///
/// # Errors
///
/// Same as [`update`], plus [`Error::StaleWrite`] when the predicate fails.
pub fn update_pred(
    layout: &Layout,
    id: &str,
    new_state: Option<&str>,
    new_priority: Option<char>,
    block_add: Option<&str>,
    block_clear: Option<&str>,
    pred: UpdatePred<'_>,
) -> Result<UpdateOutcome> {
    let identity = crate::config::identity(layout);
    update_as_pred(
        layout,
        id,
        new_state,
        new_priority,
        block_add,
        block_clear,
        &identity,
        pred,
    )
}

/// [`update`] with an explicit identity instead of [`crate::config::identity`].
///
/// # Errors
///
/// Returns an error if `id` is not in the corpus, the state or priority is
/// invalid, adding the blocker would cycle, or the file cannot be rewritten.
pub fn update_as(
    layout: &Layout,
    id: &str,
    new_state: Option<&str>,
    new_priority: Option<char>,
    block_add: Option<&str>,
    block_clear: Option<&str>,
    identity: &str,
) -> Result<UpdateOutcome> {
    update_as_pred(
        layout,
        id,
        new_state,
        new_priority,
        block_add,
        block_clear,
        identity,
        UpdatePred::default(),
    )
}

/// [`update_as`] with a last-seen predicate.
///
/// # Errors
///
/// Same as [`update_as`], plus [`Error::StaleWrite`] when the predicate fails.
#[allow(clippy::too_many_arguments)]
pub fn update_as_pred(
    layout: &Layout,
    id: &str,
    new_state: Option<&str>,
    new_priority: Option<char>,
    block_add: Option<&str>,
    block_clear: Option<&str>,
    identity: &str,
    pred: UpdatePred<'_>,
) -> Result<UpdateOutcome> {
    let (_h0, path, project) =
        find_by_id(layout, id)?.ok_or_else(|| Error::IssueNotFound { id: id.to_string() })?;

    let (transition, changed) = with_issues_lock(&path, || {
        // Read the graph inside the lock. Built before it, the check answers
        // for a corpus a peer may already have moved on from.
        let graph = if block_add.is_some() {
            Some(DependencyGraph::from_issues(&load_all(layout)?)?)
        } else {
            None
        };
        let mut doc = IssueDoc::parse_file(&project, &path)?;
        let spec = doc.priority_spec();
        let h = doc
            .headings
            .iter_mut()
            .find(|x| x.id == id)
            .ok_or_else(|| Error::IssueNotFound { id: id.to_string() })?;

        let original = h.state.clone();
        let mut changed = Vec::new();

        if pred.if_state.is_some() || pred.if_gen.is_some() {
            let seen = crate::events::generation(layout);
            if let Some(want) = pred.if_state {
                if !TODO_KEYWORDS.contains(&want) {
                    return Err(
                        anyhow!("invalid --if-state {want:?}; allowed: {TODO_KEYWORDS:?}").into(),
                    );
                }
                if h.state != want {
                    return Err(Error::StaleWrite {
                        id: id.to_string(),
                        expected_state: Some(want.to_string()),
                        actual_state: h.state.clone(),
                        expected_gen: pred.if_gen,
                        actual_gen: Some(seen),
                    });
                }
            }
            if let Some(want_gen) = pred.if_gen
                && seen != want_gen
            {
                return Err(Error::StaleWrite {
                    id: id.to_string(),
                    expected_state: pred.if_state.map(str::to_string),
                    actual_state: h.state.clone(),
                    expected_gen: Some(want_gen),
                    actual_gen: Some(seen),
                });
            }
        }

        if let Some(s) = new_state {
            if !TODO_KEYWORDS.contains(&s) {
                return Err(anyhow!("invalid state {s:?}; allowed: {TODO_KEYWORDS:?}").into());
            }
            if h.state != s {
                if is_terminal(&h.state) && is_terminal(s) {
                    record_sibling_terminal(h, s);
                    changed.push(format!("sibling terminal {s} (held {})", h.state));
                } else {
                    let from = h.state.clone();
                    h.record_state_change(s);
                    changed.push(format!("state {from} -> {s}"));
                    for note in settle_claim(h, &from, s, identity) {
                        changed.push(note);
                    }
                }
            }
        }

        if let Some(p) = new_priority {
            if !spec.contains(p) {
                return Err(anyhow!(
                    "invalid priority {p:?}; file allows [#{}]..[#{}]",
                    spec.highest,
                    spec.lowest
                )
                .into());
            }
            if h.priority != p {
                h.priority = p;
                changed.push(format!("priority -> [#{p}]"));
            }
        }

        if let Some(blk) = block_add {
            let mut current = h.blocked_by();
            if !current.iter().any(|x| x == blk) {
                if let Some(graph) = &graph {
                    graph.accepts_edge(blk, id)?;
                }
                current.push(blk.to_string());
                crate::props::insert(
                    &mut h.properties,
                    crate::props::BLOCKED_BY,
                    current.join(" "),
                );
                if h.state == "TODO" || h.state == "STARTED" {
                    let from = h.state.clone();
                    h.record_state_change("BLOCKED");
                    changed.push(format!("state {from} -> BLOCKED (auto on block)"));
                }
                changed.push(format!("blocked_by += {blk}"));
            }
        }

        if let Some(blk) = block_clear {
            let mut current = h.blocked_by();
            let before = current.len();
            current.retain(|x| x != blk);
            if current.len() < before {
                if current.is_empty() {
                    crate::props::remove(&mut h.properties, crate::props::BLOCKED_BY);
                    if h.state == "BLOCKED" {
                        let from = h.state.clone();
                        h.record_state_change("TODO");
                        changed.push("state BLOCKED -> TODO (auto on unblock)".to_string());
                        for note in settle_claim(h, &from, "TODO", identity) {
                            changed.push(note);
                        }
                    }
                } else {
                    crate::props::insert(
                        &mut h.properties,
                        crate::props::BLOCKED_BY,
                        current.join(" "),
                    );
                }
                changed.push(format!("blocked_by -= {blk}"));
            }
        }

        if changed.is_empty() {
            return Ok((None, Vec::new()));
        }

        let final_state = h.state.clone();
        doc.write()?;
        let transition = (original != final_state).then_some((original, final_state));
        Ok((transition, changed))
    })?;

    if changed.is_empty() {
        return Ok(UpdateOutcome {
            report: format!("{id}: no change\n"),
            hints: Vec::new(),
        });
    }

    if let Some((from, to)) = &transition {
        let _ = crate::events::emit_state_change(layout, &project, id, from, to);
    }

    let mut hints = Vec::new();
    if matches!(
        transition.as_ref().map(|(_, to)| to.as_str()),
        Some("DONE") | Some("CANCELLED")
    ) {
        for (other_project, other) in load_all(layout)? {
            if !other.blocked_by().iter().any(|b| b == id) {
                continue;
            }
            if other.state == "DONE" || other.state == "CANCELLED" {
                continue;
            }
            hints.push(format!(
                "{} (in {}) lists this as a blocker; clear with `vissue update {} --unblock {}`",
                other.id, other_project, other.id, id
            ));
        }
    }
    Ok(UpdateOutcome {
        report: format!("{id}: {}\n", changed.join(", ")),
        hints,
    })
}

/// States that keep a claim: someone still holds the issue even when it is
/// waiting on something else. Leaving for TODO, DONE, or CANCELLED gives it up.
fn keeps_claim(state: &str) -> bool {
    matches!(state, "STARTED" | "BLOCKED")
}

fn is_terminal(state: &str) -> bool {
    matches!(state, "DONE" | "CANCELLED")
}

fn record_sibling_terminal(h: &mut IssueHeading, attempted: &str) {
    crate::props::insert(
        &mut h.properties,
        crate::props::SIBLING_TERMINAL,
        attempted.to_string(),
    );
}

/// Pick one terminal after a sibling close. Clears `:SIBLING_TERMINAL:`.
///
/// # Errors
///
/// Returns an error if `id` is missing, `state` is not DONE or CANCELLED, or
/// the file cannot be rewritten.
pub fn resolve_terminal(layout: &Layout, id: &str, state: &str) -> Result<String> {
    if !is_terminal(state) {
        return Err(anyhow!("resolve state must be DONE or CANCELLED, got {state:?}").into());
    }
    let identity = crate::config::identity(layout);
    let (_h0, path, project) =
        find_by_id(layout, id)?.ok_or_else(|| Error::IssueNotFound { id: id.to_string() })?;
    let from = with_issues_lock(&path, || {
        let mut doc = IssueDoc::parse_file(&project, &path)?;
        let h = doc
            .headings
            .iter_mut()
            .find(|x| x.id == id)
            .ok_or_else(|| Error::IssueNotFound { id: id.to_string() })?;
        let from = h.state.clone();
        if from != state {
            h.record_state_change(state);
            settle_claim(h, &from, state, &identity);
        }
        crate::props::remove(&mut h.properties, crate::props::SIBLING_TERMINAL);
        doc.write()?;
        Ok(from)
    })?;
    if from != state {
        let _ = crate::events::emit_state_change(layout, &project, id, &from, state);
    }
    Ok(format!("resolved {id} -> {state}\n"))
}

/// Take or give up the claim as the state moves.
///
/// Entering STARTED unclaimed stamps the identity; leaving for a state that
/// holds no claim releases it, and the logbook keeps who held it and since
/// when.
fn settle_claim(h: &mut IssueHeading, from: &str, to: &str, identity: &str) -> Vec<String> {
    let mut notes = Vec::new();
    if to == "STARTED" && h.claimed_by().is_none() {
        h.set_claim(identity);
        notes.push(format!("claimed by {identity}"));
    } else if keeps_claim(from)
        && !keeps_claim(to)
        && let Some((who, _when)) = h.release_claim()
    {
        notes.push(format!("claim released ({who})"));
    }
    notes
}

/// Take an issue: move it to STARTED and stamp the claim.
///
/// A claim held by another identity is refused unless `force`, which records
/// the takeover in the logbook rather than losing it.
///
/// # Errors
///
/// Returns an error if `id` is not in the corpus, the issue is DONE or
/// CANCELLED, another identity holds it and `force` is false, or the file
/// cannot be rewritten.
pub fn claim(layout: &Layout, id: &str, force: bool) -> Result<String> {
    let identity = crate::config::identity(layout);
    claim_as(layout, id, force, &identity)
}

/// [`claim`] with an explicit identity instead of [`crate::config::identity`].
///
/// # Errors
///
/// Returns an error if `id` is not in the corpus, the issue is DONE or
/// CANCELLED, another identity holds it and `force` is false, or the file
/// cannot be rewritten.
pub fn claim_as(layout: &Layout, id: &str, force: bool, identity: &str) -> Result<String> {
    let (_h0, path, project) =
        find_by_id(layout, id)?.ok_or_else(|| Error::IssueNotFound { id: id.to_string() })?;

    let report = with_issues_lock(&path, || {
        let mut doc = IssueDoc::parse_file(&project, &path)?;
        let h = doc
            .headings
            .iter_mut()
            .find(|x| x.id == id)
            .ok_or_else(|| Error::IssueNotFound { id: id.to_string() })?;

        if h.state == "DONE" || h.state == "CANCELLED" {
            return Err(Error::InvalidState {
                id: id.to_string(),
                state: h.state.clone(),
            });
        }
        if let Some(holder) = h.claimed_by() {
            if holder != identity && !force {
                return Err(Error::ClaimConflict {
                    id: id.to_string(),
                    holder: holder.to_string(),
                    claimed_at: h.claimed_at().map(str::to_string),
                });
            }
            if holder != identity {
                let previous = holder.to_string();
                let from = h.state.clone();
                h.release_claim();
                h.set_claim(identity);
                h.record_state_change("STARTED");
                doc.write()?;
                if from != "STARTED" {
                    let _ =
                        crate::events::emit_state_change(layout, &project, id, &from, "STARTED");
                }
                return Ok(format!("claimed {id} (taken over from {previous})\n"));
            }
        }

        let was = h.state.clone();
        h.record_state_change("STARTED");
        if h.claimed_by().is_none() {
            h.set_claim(identity);
        }
        doc.write()?;
        if was != "STARTED" {
            let _ = crate::events::emit_state_change(layout, &project, id, &was, "STARTED");
        }
        if was == "STARTED" {
            Ok(format!("claimed {id} by {identity}\n"))
        } else {
            Ok(format!("claimed {id} by {identity} ({was} -> STARTED)\n"))
        }
    })?;
    Ok(report)
}

/// What an update changed, plus advice about issues left dangling by it.
#[derive(Debug, Clone)]
pub struct UpdateOutcome {
    /// One-line change summary, or `{id}: no change`.
    pub report: String,
    /// Issues that still list this one as a blocker after it closed.
    pub hints: Vec<String>,
}

/// Add a dated note to the top of an issue's logbook. State, claim, and
/// properties stay untouched, so an agent can record progress without owning
/// the issue.
///
/// # Errors
///
/// Returns an error if `text` is empty, `id` is not in the corpus, or the
/// file cannot be rewritten.
pub fn note(layout: &Layout, id: &str, text: &str) -> Result<String> {
    // One line in the drawer: fold internal whitespace, and swap double
    // quotes for singles so the rendered `- Note: "..."` line re-parses.
    let text = text
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .replace('"', "'");
    if text.is_empty() {
        return Err(anyhow!("note text is empty").into());
    }
    let (_h0, path, project) =
        find_by_id(layout, id)?.ok_or_else(|| Error::IssueNotFound { id: id.to_string() })?;
    with_issues_lock(&path, || {
        let mut doc = IssueDoc::parse_file(&project, &path)?;
        let h = doc
            .headings
            .iter_mut()
            .find(|x| x.id == id)
            .ok_or_else(|| Error::IssueNotFound { id: id.to_string() })?;
        // Newest first, matching state transitions and claim releases. A
        // drawer written from both ends reads as sorted by neither.
        h.logbook.insert(
            0,
            LogEntry {
                timestamp: LogEntry::now(),
                from_state: None,
                to_state: None,
                note: Some(text.clone()),
                raw: None,
            },
        );
        doc.write()?;
        Ok(format!("{id}: noted\n"))
    })
}

/// Append prose to an issue's body, stamped with the date and identity.
///
/// The logbook holds one line per event, so a written report does not fit in
/// it: [`note`] folds its text to a single line by design. Work that has been
/// done and needs recording belongs under the heading as prose, which is
/// where a reader looks for what the issue is about.
///
/// The text is kept as given. Lines that would end the issue are indented on
/// the way out, so markdown is safe to append.
///
/// # Errors
///
/// Returns an error if `text` is empty, `id` is not in the corpus, or the
/// file cannot be rewritten.
pub fn append_body(layout: &Layout, id: &str, text: &str) -> Result<String> {
    append_body_as(layout, id, text, &crate::config::identity(layout))
}

/// [`append_body`] with the recorded identity passed in.
///
/// # Errors
///
/// Returns an error if `text` is empty, `id` is not in the corpus, or the
/// file cannot be rewritten.
pub fn append_body_as(layout: &Layout, id: &str, text: &str, identity: &str) -> Result<String> {
    let text = text.trim_end();
    if text.trim().is_empty() {
        return Err(anyhow!("append text is empty").into());
    }
    let (_h0, path, project) =
        find_by_id(layout, id)?.ok_or_else(|| Error::IssueNotFound { id: id.to_string() })?;
    with_issues_lock(&path, || {
        let mut doc = IssueDoc::parse_file(&project, &path)?;
        let h = doc
            .headings
            .iter_mut()
            .find(|x| x.id == id)
            .ok_or_else(|| Error::IssueNotFound { id: id.to_string() })?;
        let stamp = format!("{} {identity}", today_inactive_bracket());
        if !h.body.trim().is_empty() {
            h.body = h.body.trim_end().to_string();
            h.body.push_str("\n\n");
        } else {
            h.body.clear();
        }
        h.body.push_str(&stamp);
        h.body.push('\n');
        h.body.push_str(text);
        h.body.push('\n');
        doc.write()?;
        let lines = text.lines().count();
        Ok(format!("{id}: appended {lines} line(s)\n"))
    })
}

/// Fold an inbox-convention org file into tracked issues.
///
/// Each top-level `* TODO <title>` heading that does not already carry a
/// `:VISSUE_ID:` line becomes an issue in `project` (body = the heading's
/// text up to the next heading). The heading is then flipped to DONE and
/// stamped with the assigned id in place, so a second run is a no-op:
/// stamped headings are skipped, and folding is idempotent.
///
/// # Errors
///
/// Returns an error if the inbox cannot be read or written, `project` cannot
/// be resolved, or creating a folded issue fails. Headings already stamped
/// before a failure stay stamped.
pub fn fold(layout: &Layout, inbox: &std::path::Path, project: &str) -> Result<String> {
    let project = resolve_existing_project_case(layout, project)?;
    let text = std::fs::read_to_string(inbox)
        .with_context(|| format!("read inbox {}", inbox.display()))?;
    let lines: Vec<String> = text.lines().map(str::to_string).collect();

    struct Entry {
        line: usize,
        title: String,
        body: String,
        stamped: bool,
    }
    let mut entries: Vec<Entry> = Vec::new();
    let mut i = 0;
    let mut nest = crate::org::OrgScan::new();
    while i < lines.len() {
        if nest.observe(&lines[i]) {
            i += 1;
            continue;
        }
        if let Some(title) = lines[i].strip_prefix("* TODO ") {
            let start = i + 1;
            let mut end_nest = crate::org::OrgScan::new();
            let end = {
                let mut j = start;
                while j < lines.len() {
                    if !end_nest.observe(&lines[j]) && lines[j].starts_with("* ") {
                        break;
                    }
                    j += 1;
                }
                j
            };
            let stamped = lines[start..end]
                .iter()
                .any(|l| l.trim_start().starts_with(":VISSUE_ID:"));
            let body = lines[start..end].join("\n").trim().to_string();
            entries.push(Entry {
                line: i,
                title: title.trim().to_string(),
                body,
                stamped,
            });
            i = end;
        } else {
            i += 1;
        }
    }

    // Stamping inserts lines, so rewrite from the bottom up to keep the
    // recorded line numbers valid.
    let mut out = lines.clone();
    let mut created: Vec<String> = Vec::new();
    let mut failure = None;
    for e in entries.iter().rev() {
        if e.stamped {
            continue;
        }
        let printed = create(
            layout,
            &project,
            &e.title,
            CreateOpts {
                quiet: true,
                body: if e.body.is_empty() {
                    None
                } else {
                    Some(&e.body)
                },
                ..CreateOpts::default()
            },
        );
        let id = match printed {
            Ok(printed) => printed.trim().to_string(),
            Err(e) => {
                // Stop, but stamp what already exists below. Returning here
                // with the inbox untouched would leave every issue created so
                // far unstamped, and the next run would create them again.
                failure = Some(e);
                break;
            }
        };
        out[e.line] = format!("* DONE {}", e.title);
        out.insert(e.line + 1, format!(":VISSUE_ID: {id}"));
        created.push(id);
    }
    created.reverse();

    if !created.is_empty() {
        let mut rendered = out.join("\n");
        if text.ends_with('\n') {
            rendered.push('\n');
        }
        std::fs::write(inbox, rendered)
            .with_context(|| format!("write inbox {}", inbox.display()))?;
    }
    if let Some(error) = failure {
        return Err(crate::error::Error::Other(
            anyhow::Error::from(error).context(format!(
                "folded {} before failing: {}",
                created.len(),
                created.join(" ")
            )),
        ));
    }
    if created.is_empty() {
        return Ok("folded 0 (nothing unstamped)\n".into());
    }
    Ok(format!("folded {}: {}\n", created.len(), created.join(" ")))
}

/// Move one issue's heading to another project's file. The id is not
/// regenerated, so cross-project blocker edges keep resolving.
///
/// # Errors
///
/// Returns an error if `id` is not in the corpus, `to_project` cannot be
/// resolved, or either file cannot be locked or rewritten.
pub fn refile(layout: &Layout, id: &str, to_project: &str) -> Result<String> {
    refile_to(layout, id, layout, to_project)
}

/// Move one issue's heading onto a destination that may live on another
/// tracker layout. A router resolves the destination project name before
/// calling this, so a routed name lands on its own checkout instead of
/// growing a shadow directory under the source root.
///
/// # Errors
///
/// Same as [`refile`].
pub fn refile_to(
    layout: &Layout,
    id: &str,
    dst_layout: &Layout,
    to_project: &str,
) -> Result<String> {
    let to_project = resolve_existing_project_case(dst_layout, to_project)?;
    let target_path = dst_layout.project_issues_path(&to_project);
    let (_heading, src_path, src_project) =
        find_by_id(layout, id)?.ok_or_else(|| Error::IssueNotFound { id: id.to_string() })?;
    if src_path == target_path {
        return Ok(format!("{id} already in {to_project}; nothing to do\n"));
    }
    with_issues_locks(&[&src_path, &target_path], || {
        let mut src_doc = IssueDoc::parse_file(&src_project, &src_path)?;
        let heading = src_doc
            .remove(id)
            .ok_or_else(|| Error::IssueNotFound { id: id.to_string() })?;

        // Two files cannot be replaced in one atomic step, so choose which
        // half-finished state a failure leaves behind. Writing the target
        // first means a failed source write duplicates the id, which `check`
        // reports and a person can resolve; the other order deletes the issue
        // with nothing left naming it.
        let mut tgt_doc = IssueDoc::parse_file(&to_project, &target_path)?;
        tgt_doc.upsert(heading);
        tgt_doc.write()?;
        src_doc.write()?;
        Ok(())
    })?;
    Ok(format!("{id}: {src_project} -> {to_project}\n"))
}

/// Optional fields on [`reject`].
#[derive(Debug, Default, Clone, Copy)]
pub struct RejectOpts<'a> {
    /// Existing destination id. When set, that heading is the successor.
    pub to: Option<&'a str>,
    /// Project to create the destination in when [`Self::to`] is absent.
    pub project: Option<&'a str>,
    /// Title of a created destination. The source title is used when omitted.
    pub title: Option<&'a str>,
    /// Prose appended to the cancelled source.
    pub reason: Option<&'a str>,
    /// Tracker that holds the destination. `None` keeps the source's.
    pub dst_layout: Option<&'a Layout>,
    /// Twin files read under the lock when minting a successor, so a twin on
    /// another layout cannot share a suffix with it. Paths and not ids, because
    /// ids the caller read before the lock can be stale by the time it is held.
    pub dst_extra_id_paths: &'a [PathBuf],
}

/// Cancel `src` and point it at a successor in one graph edit.
///
/// Writes `src` to CANCELLED, sets `:PIVOTED_TO:` to the destination, and
/// settles any claim on `src`. A created destination, or an existing one
/// whose `:DISCOVERED_FROM:` is empty, records `src` as its origin. A
/// non-empty `:DISCOVERED_FROM:` is left alone.
///
/// # Errors
///
/// Returns an error if `src` is not in the corpus, `--to` names no heading,
/// neither a destination nor a create project is given, or a file cannot be
/// rewritten.
pub fn reject(layout: &Layout, src: &str, opts: RejectOpts<'_>) -> Result<String> {
    let identity = crate::config::identity(layout);
    let (src0, src_path, src_project) =
        find_by_id(layout, src)?.ok_or_else(|| Error::IssueNotFound {
            id: src.to_string(),
        })?;

    let dst_layout = opts.dst_layout.unwrap_or(layout);
    let existing_dst = if let Some(to) = opts.to {
        if to == src {
            return Err(anyhow!("reject destination cannot be the source {src}").into());
        }
        Some(
            find_by_id(dst_layout, to)?
                .ok_or_else(|| Error::IssueNotFound { id: to.to_string() })?,
        )
    } else {
        None
    };

    let creating = existing_dst.is_none();
    if creating && opts.project.is_none() {
        return Err(anyhow!("reject needs --to DST or --project to create a successor").into());
    }

    let dst_project = if let Some((_, _, ref project)) = existing_dst {
        project.clone()
    } else {
        resolve_existing_project_case(dst_layout, opts.project.unwrap_or(&src_project))?
    };
    let dst_path = dst_layout.project_issues_path(&dst_project);
    let dst_title = opts.title.unwrap_or(src0.title.as_str());
    let cfg = VissueConfig::load(layout)?;

    // The twins the mint consults are locked too, or the reservation is read
    // outside the lock that guards the write and a peer can mint the same id.
    let mut lock_paths: Vec<PathBuf> = vec![src_path.clone(), dst_path.clone()];
    lock_paths.extend(opts.dst_extra_id_paths.iter().cloned());
    let lock_refs: Vec<&Path> = lock_paths.iter().map(PathBuf::as_path).collect();
    let (dst_id, old_state, new_state) = with_issues_locks(&lock_refs, || {
        if src_path == dst_path {
            let mut doc = IssueDoc::parse_file(&src_project, &src_path)?;
            let dst_id = if creating {
                push_successor(
                    &mut doc,
                    &dst_project,
                    dst_title,
                    src,
                    &cfg,
                    opts.dst_extra_id_paths,
                )?
            } else {
                let to = reject_to(opts)?;
                set_discovered_from_if_empty(&mut doc, to, src)?;
                to.to_string()
            };
            let (old_state, new_state) =
                cancel_and_pivot(&mut doc, src, &dst_id, opts.reason, &identity)?;
            doc.write()?;
            Ok((dst_id, old_state, new_state))
        } else {
            let mut src_doc = IssueDoc::parse_file(&src_project, &src_path)?;
            let mut dst_doc = IssueDoc::parse_file(&dst_project, &dst_path)?;
            let dst_id = if creating {
                push_successor(
                    &mut dst_doc,
                    &dst_project,
                    dst_title,
                    src,
                    &cfg,
                    opts.dst_extra_id_paths,
                )?
            } else {
                let to = reject_to(opts)?;
                set_discovered_from_if_empty(&mut dst_doc, to, src)?;
                to.to_string()
            };
            let (old_state, new_state) =
                cancel_and_pivot(&mut src_doc, src, &dst_id, opts.reason, &identity)?;
            dst_doc.write()?;
            src_doc.write()?;
            Ok((dst_id, old_state, new_state))
        }
    })?;

    if old_state != new_state {
        let _ = crate::events::emit_state_change(layout, &src_project, src, &old_state, &new_state);
    }
    Ok(format!("rejected {src} -> {dst_id}\n"))
}

fn reject_to(opts: RejectOpts<'_>) -> Result<&str> {
    opts.to
        .ok_or_else(|| anyhow!("reject destination missing after --to was required").into())
}

fn push_successor(
    doc: &mut IssueDoc,
    project: &str,
    title: &str,
    src: &str,
    cfg: &VissueConfig,
    extra_id_paths: &[PathBuf],
) -> Result<String> {
    let mut taken = doc.known_ids();
    // Read here rather than by the caller, because here is inside the lock set.
    for twin in extra_id_paths {
        if twin == &doc.path {
            continue;
        }
        if let Ok(other) = IssueDoc::parse_file(project, twin) {
            taken.extend(other.known_ids());
        }
    }
    let id = generate_id(project, &taken, cfg.issues.id_length)?;
    let mut props = BTreeMap::new();
    props.insert("ID".into(), id.clone());
    props.insert("CREATED".into(), today_inactive_bracket());
    crate::props::insert(&mut props, crate::props::DISCOVERED_FROM, src.to_string());
    doc.headings.push(IssueHeading {
        id: id.clone(),
        title: title.to_string(),
        state: "TODO".into(),
        priority: doc.default_create_priority(cfg.issues.default_priority),
        properties: props,
        org_tags: Vec::new(),
        statistics: None,
        property_order: Vec::new(),
        extra_drawers: Vec::new(),
        body: String::new(),
        logbook: Vec::new(),
        line_start: 0,
        line_end: 0,
    });
    Ok(id)
}

fn set_discovered_from_if_empty(doc: &mut IssueDoc, id: &str, src: &str) -> Result<()> {
    let h = doc
        .headings
        .iter_mut()
        .find(|h| h.id == id)
        .ok_or_else(|| Error::IssueNotFound { id: id.to_string() })?;
    let empty = crate::props::get(&h.properties, crate::props::DISCOVERED_FROM)
        .is_none_or(|s| s.trim().is_empty());
    if empty {
        crate::props::insert(
            &mut h.properties,
            crate::props::DISCOVERED_FROM,
            src.to_string(),
        );
    }
    Ok(())
}

fn cancel_and_pivot(
    doc: &mut IssueDoc,
    src: &str,
    dst: &str,
    reason: Option<&str>,
    identity: &str,
) -> Result<(String, String)> {
    let h = doc
        .headings
        .iter_mut()
        .find(|h| h.id == src)
        .ok_or_else(|| Error::IssueNotFound {
            id: src.to_string(),
        })?;
    let old_state = h.state.clone();
    if is_terminal(&old_state) && old_state != "CANCELLED" {
        record_sibling_terminal(h, "CANCELLED");
    } else if old_state != "CANCELLED" {
        h.record_state_change("CANCELLED");
        settle_claim(h, &old_state, "CANCELLED", identity);
    }
    crate::props::insert(&mut h.properties, crate::props::PIVOTED_TO, dst.to_string());
    if let Some(reason) = reason {
        append_reason(h, reason, identity);
    }
    Ok((old_state, h.state.clone()))
}

fn append_reason(h: &mut IssueHeading, text: &str, identity: &str) {
    let text = text.trim_end();
    if text.trim().is_empty() {
        return;
    }
    let stamp = format!("{} {identity}", today_inactive_bracket());
    if !h.body.trim().is_empty() {
        h.body = h.body.trim_end().to_string();
        h.body.push_str("\n\n");
    } else {
        h.body.clear();
    }
    h.body.push_str(&stamp);
    h.body.push('\n');
    h.body.push_str(text);
    h.body.push('\n');
}

/// First `[[id:XXX]]` (optionally `[[id:XXX][label]]`) whose id is in `known`.
fn first_existing_id_link(body: &str, known: &std::collections::HashSet<String>) -> Option<String> {
    let mut rest = body;
    while let Some(start) = rest.find("[[") {
        let after_start = &rest[start + 2..];
        let end = after_start.find("]]")?;
        let raw = &after_start[..end];
        let target = raw.split_once("][").map_or(raw, |(target, _)| target);
        let target = target.trim();
        if let Some(id) = target.strip_prefix("id:") {
            let id = id.trim();
            if known.contains(id) {
                return Some(id.to_string());
            }
        }
        rest = &after_start[end + 2..];
    }
    None
}

/// Rewrite project files onto the Org / ELPA / vissue property split.
///
/// Folds typos (`BLOCKEDBY`, drawer `TAGS`) and a bare `:BLOCKER:` id
/// list into `:BLOCKED_BY:`. A real org-edna condition stays. Puts legal
/// types on the heading and inserts a missing `#+CATEGORY:`. Does not
/// mint `:BLOCKER: ids(...)`.
///
/// # Errors
///
/// Returns an error if a project file cannot be read or rewritten.
pub fn normalize(layout: &Layout, project: Option<&str>, dry_run: bool) -> Result<String> {
    let projects = match project {
        Some(name) => vec![resolve_existing_project_case(layout, name)?],
        None => crate::store::list_projects(layout)?,
    };
    let mut out = String::new();
    let mut files = 0usize;
    let mut headings = 0usize;
    let mut changed = 0usize;
    for project in projects {
        let path = layout.project_issues_path(&project);
        if !path.exists() {
            continue;
        }
        files += 1;
        let before =
            std::fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
        let report = with_issues_lock(&path, || {
            let mut doc = IssueDoc::parse_file(&project, &path)?;
            let mut moved = 0usize;
            for h in &mut doc.headings {
                moved += crate::props::settle(&mut h.org_tags, &mut h.properties);
            }
            let after = doc.render_string();
            if after != before {
                if !dry_run {
                    doc.write()?;
                }
                Ok(Some((moved, after.len())))
            } else {
                Ok(None)
            }
        })?;
        headings += IssueDoc::parse(&project, path.clone(), &before)
            .map(|d| d.headings.len())
            .unwrap_or(0);
        if let Some((moved, _)) = report {
            changed += 1;
            let verb = if dry_run { "would rewrite" } else { "rewrote" };
            writeln!(out, "{verb} {project} ({moved} key move(s))")?;
        }
    }
    let mode = if dry_run { "dry-run" } else { "wrote" };
    writeln!(
        out,
        "normalize {mode}: {changed}/{files} file(s) changed, {headings} heading(s) scanned"
    )?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DEFAULT_PREFIX;
    use std::fs;
    use std::path::Path;

    fn fresh_layout(dir: &Path) -> Layout {
        fs::create_dir_all(dir.join(DEFAULT_PREFIX)).unwrap();
        Layout::new(dir, DEFAULT_PREFIX)
    }

    fn issue_at(layout: &Layout, project: &str, id: &str) -> IssueHeading {
        IssueDoc::parse_file(project, &layout.project_issues_path(project))
            .unwrap()
            .headings
            .into_iter()
            .find(|h| h.id == id)
            .expect("issue not found")
    }

    fn only_id(layout: &Layout, project: &str) -> String {
        IssueDoc::parse_file(project, &layout.project_issues_path(project))
            .unwrap()
            .headings[0]
            .id
            .clone()
    }

    #[test]
    fn create_rejects_a_parent_that_does_not_exist() {
        let dir = tempfile::tempdir().unwrap();
        let layout = fresh_layout(dir.path());
        let err = create(
            &layout,
            "sample",
            "child without parent",
            CreateOpts {
                parent: Some("sample-zzz9"),
                ..Default::default()
            },
        )
        .unwrap_err();
        assert!(err.to_string().contains("does not refer to any known id"));
    }

    #[test]
    fn create_accepts_a_parent_defined_in_a_design_document() {
        let dir = tempfile::tempdir().unwrap();
        let layout = fresh_layout(dir.path());
        let parent_id = "sample-spec-20260615";
        let project_dir = layout.projects_dir().join("sample");
        fs::create_dir_all(&project_dir).unwrap();
        fs::write(
            project_dir.join("design.org"),
            format!("#+TITLE: sample design\n\n* Design\n:PROPERTIES:\n:ID:         {parent_id}\n:END:\n"),
        )
        .unwrap();

        create(
            &layout,
            "sample",
            "child under design",
            CreateOpts {
                parent: Some(parent_id),
                ..Default::default()
            },
        )
        .unwrap();
        assert!(only_id(&layout, "sample").starts_with("sample-"));
        let doc = IssueDoc::parse_file("sample", &layout.project_issues_path("sample")).unwrap();
        assert_eq!(doc.headings[0].parent(), Some(parent_id));
    }

    #[test]
    fn a_state_update_writes_a_logbook_entry() {
        let dir = tempfile::tempdir().unwrap();
        let layout = fresh_layout(dir.path());
        create(&layout, "sample", "first", CreateOpts::default()).unwrap();
        let id = only_id(&layout, "sample");
        update(&layout, &id, Some("STARTED"), None, None, None).unwrap();
        let h = issue_at(&layout, "sample", &id);
        assert_eq!(h.state, "STARTED");
        assert_eq!(h.logbook[0].from_state.as_deref(), Some("TODO"));
        assert_eq!(h.logbook[0].to_state.as_deref(), Some("STARTED"));
    }

    #[test]
    fn blocking_and_unblocking_drive_the_state() {
        let dir = tempfile::tempdir().unwrap();
        let layout = fresh_layout(dir.path());
        create(&layout, "sample", "first", CreateOpts::default()).unwrap();
        create(&layout, "sample", "blocker", CreateOpts::default()).unwrap();
        let doc = IssueDoc::parse_file("sample", &layout.project_issues_path("sample")).unwrap();
        let first = doc.headings[0].id.clone();
        let blocker = doc.headings[1].id.clone();

        update(&layout, &first, None, None, Some(&blocker), None).unwrap();
        let h = issue_at(&layout, "sample", &first);
        assert_eq!(h.state, "BLOCKED");
        assert!(h.blocked_by().contains(&blocker));

        update(&layout, &first, None, None, None, Some(&blocker)).unwrap();
        let h = issue_at(&layout, "sample", &first);
        assert_eq!(h.state, "TODO");
        assert!(h.blocked_by().is_empty());
    }

    #[test]
    fn auto_unblock_to_todo_releases_the_claim() {
        let dir = tempfile::tempdir().unwrap();
        let layout = fresh_layout(dir.path());
        create(&layout, "sample", "first", CreateOpts::default()).unwrap();
        create(&layout, "sample", "blocker", CreateOpts::default()).unwrap();
        let doc = IssueDoc::parse_file("sample", &layout.project_issues_path("sample")).unwrap();
        let first = doc.headings[0].id.clone();
        let blocker = doc.headings[1].id.clone();

        crate::agent::claim(&layout, &first, false).unwrap();
        update(&layout, &first, None, None, Some(&blocker), None).unwrap();
        assert!(issue_at(&layout, "sample", &first).claimed_by().is_some());

        update(&layout, &first, None, None, None, Some(&blocker)).unwrap();
        let h = issue_at(&layout, "sample", &first);
        assert_eq!(h.state, "TODO");
        assert!(h.claimed_by().is_none(), "claim stuck on TODO: {h:?}");
    }

    #[test]
    fn blocker_cycle_is_rejected_before_writing() {
        let dir = tempfile::tempdir().unwrap();
        let layout = fresh_layout(dir.path());
        create(&layout, "sample", "first", CreateOpts::default()).unwrap();
        create(&layout, "sample", "second", CreateOpts::default()).unwrap();
        let doc = IssueDoc::parse_file("sample", &layout.project_issues_path("sample")).unwrap();
        let first = doc.headings[0].id.clone();
        let second = doc.headings[1].id.clone();

        update(&layout, &first, None, None, Some(&second), None).unwrap();
        let err = update(&layout, &second, None, None, Some(&first), None).unwrap_err();
        assert!(err.to_string().contains("blocker cycle"), "{err}");
        assert!(issue_at(&layout, "sample", &second).blocked_by().is_empty());
    }

    #[test]
    fn closing_a_blocker_reports_the_issues_still_pointing_at_it() {
        let dir = tempfile::tempdir().unwrap();
        let layout = fresh_layout(dir.path());
        create(&layout, "sample", "first", CreateOpts::default()).unwrap();
        create(&layout, "sample", "blocker", CreateOpts::default()).unwrap();
        let doc = IssueDoc::parse_file("sample", &layout.project_issues_path("sample")).unwrap();
        let first = doc.headings[0].id.clone();
        let blocker = doc.headings[1].id.clone();
        update(&layout, &first, None, None, Some(&blocker), None).unwrap();

        let outcome = update(&layout, &blocker, Some("DONE"), None, None, None).unwrap();
        assert_eq!(outcome.hints.len(), 1, "{:?}", outcome.hints);
        assert!(outcome.hints[0].contains(&first), "{:?}", outcome.hints);
    }

    #[test]
    fn refile_moves_the_heading_between_projects() {
        let dir = tempfile::tempdir().unwrap();
        let layout = fresh_layout(dir.path());
        create(&layout, "source", "the issue", CreateOpts::default()).unwrap();
        let id = only_id(&layout, "source");
        refile(&layout, &id, "target").unwrap();

        let src = IssueDoc::parse_file("source", &layout.project_issues_path("source")).unwrap();
        let tgt = IssueDoc::parse_file("target", &layout.project_issues_path("target")).unwrap();
        assert!(src.headings.is_empty());
        assert_eq!(tgt.headings[0].id, id);
    }

    #[test]
    fn deadlines_must_parse_as_org_dates() {
        let dir = tempfile::tempdir().unwrap();
        let layout = fresh_layout(dir.path());
        let err = create(
            &layout,
            "sample",
            "bad date",
            CreateOpts {
                deadline: Some("not-a-date"),
                ..Default::default()
            },
        )
        .unwrap_err();
        assert!(err.to_string().contains("expected org date"));

        for (i, d) in ["<2026-05-15 Fri>", "[2026-05-15]"].iter().enumerate() {
            create(
                &layout,
                "sample",
                &format!("issue {i}"),
                CreateOpts {
                    deadline: Some(d),
                    ..Default::default()
                },
            )
            .unwrap();
        }
        let doc = IssueDoc::parse_file("sample", &layout.project_issues_path("sample")).unwrap();
        assert_eq!(doc.headings.len(), 2);
        assert!(doc.headings.iter().all(|h| h.deadline().is_some()));
    }

    #[test]
    fn org_safe_tags_go_on_the_heading_and_the_rest_stay_in_the_property() {
        let dir = tempfile::tempdir().unwrap();
        let layout = fresh_layout(dir.path());
        create(
            &layout,
            "sample",
            "tagged",
            CreateOpts {
                tags: Some("rust: perf ,, scaling, needs-review"),
                ..Default::default()
            },
        )
        .unwrap();
        let doc = IssueDoc::parse_file("sample", &layout.project_issues_path("sample")).unwrap();
        let h = &doc.headings[0];
        assert_eq!(h.org_tags, vec!["rust", "perf", "scaling"]);
        assert_eq!(
            h.properties
                .get(crate::model::TAGS_PROPERTY)
                .map(|s| s.as_str()),
            Some("needs-review"),
            "a tag Org cannot hold keeps the property"
        );
        // Whichever half a tag landed in, a query sees all of them.
        assert_eq!(
            h.tags(),
            vec!["needs-review", "rust", "perf", "scaling"],
            "{h:?}"
        );
    }

    #[test]
    fn create_puts_a_legal_type_on_the_heading() {
        let dir = tempfile::tempdir().unwrap();
        let layout = fresh_layout(dir.path());
        create(
            &layout,
            "sample",
            "a bug",
            CreateOpts {
                issue_type: Some("bug"),
                ..Default::default()
            },
        )
        .unwrap();
        let doc = IssueDoc::parse_file("sample", &layout.project_issues_path("sample")).unwrap();
        let h = &doc.headings[0];
        assert_eq!(
            crate::props::get(&h.properties, crate::props::TYPE),
            Some("bug")
        );
        assert_eq!(h.org_tags, vec!["bug"]);
        let written = std::fs::read_to_string(layout.project_issues_path("sample")).unwrap();
        assert!(written.contains("#+CATEGORY: sample"), "{written}");
        assert!(written.contains(":bug:"), "{written}");
    }

    #[test]
    fn resolve_project_needs_a_name_from_somewhere() {
        let dir = tempfile::tempdir().unwrap();
        let layout = fresh_layout(dir.path());
        assert_eq!(
            resolve_project(&layout, Some("fromcli")).unwrap(),
            "fromcli"
        );
        assert!(
            resolve_project(&layout, Some(""))
                .unwrap_err()
                .to_string()
                .contains("empty")
        );
    }

    /// Parallel creates must not lose headings or fail the temporary rename.
    #[test]
    fn concurrent_creates_preserve_every_heading() {
        use std::sync::Arc;
        use std::thread;

        let dir = tempfile::tempdir().unwrap();
        let layout = Arc::new(fresh_layout(dir.path()));
        let n = 24usize;
        let handles: Vec<_> = (0..n)
            .map(|i| {
                let layout = Arc::clone(&layout);
                thread::spawn(move || {
                    create(
                        &layout,
                        "sample",
                        &format!("parallel title {i}"),
                        CreateOpts {
                            quiet: true,
                            ..Default::default()
                        },
                    )
                })
            })
            .collect();
        let mut ids: Vec<String> = handles
            .into_iter()
            .map(|h| {
                h.join()
                    .expect("thread panicked")
                    .expect("create failed")
                    .trim()
                    .to_string()
            })
            .collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), n, "expected {n} unique ids, got {ids:?}");

        let doc = IssueDoc::parse_file("sample", &layout.project_issues_path("sample")).unwrap();
        let mut on_disk: Vec<String> = doc.headings.iter().map(|h| h.id.clone()).collect();
        on_disk.sort();
        assert_eq!(on_disk, ids);
    }

    #[test]
    fn note_appends_to_the_logbook_and_leaves_state_alone() {
        let dir = tempfile::tempdir().unwrap();
        let layout = fresh_layout(dir.path());
        create(&layout, "sample", "carries a note", CreateOpts::default()).unwrap();
        let id = only_id(&layout, "sample");

        let out = note(&layout, &id, "first pass done,\n  \"quoted\" bit next").unwrap();
        assert_eq!(out, format!("{id}: noted\n"));

        let h = issue_at(&layout, "sample", &id);
        assert_eq!(h.state, "TODO");
        assert!(h.claimed_by().is_none());
        let notes: Vec<&str> = h.logbook.iter().filter_map(|e| e.note.as_deref()).collect();
        // Whitespace collapses to single spaces; double quotes become single.
        assert_eq!(notes, vec!["first pass done, 'quoted' bit next"]);
    }

    #[test]
    fn the_logbook_reads_newest_first_however_an_entry_arrived() {
        let dir = tempfile::tempdir().unwrap();
        let layout = fresh_layout(dir.path());
        create(&layout, "sample", "ordered", CreateOpts::default()).unwrap();
        let id = only_id(&layout, "sample");

        note(&layout, &id, "first note").unwrap();
        update(&layout, &id, Some("STARTED"), None, None, None).unwrap();
        note(&layout, &id, "second note").unwrap();

        let h = issue_at(&layout, "sample", &id);
        let summary: Vec<String> = h
            .logbook
            .iter()
            .map(|e| match (&e.note, &e.to_state) {
                (Some(note), _) => note.clone(),
                (_, Some(to)) => format!("state:{to}"),
                _ => "?".into(),
            })
            .collect();
        assert_eq!(
            summary,
            vec!["second note", "state:STARTED", "first note"],
            "{h:?}"
        );
    }

    #[test]
    fn note_rejects_empty_text_and_unknown_ids() {
        let dir = tempfile::tempdir().unwrap();
        let layout = fresh_layout(dir.path());
        create(&layout, "sample", "target", CreateOpts::default()).unwrap();
        let id = only_id(&layout, "sample");
        assert!(note(&layout, &id, "   ").is_err());
        assert!(note(&layout, "sample-zzz9", "text").is_err());
    }

    #[test]
    fn fold_creates_issues_and_stamps_the_inbox_idempotently() {
        let dir = tempfile::tempdir().unwrap();
        let layout = fresh_layout(dir.path());
        create(&layout, "sample", "seed", CreateOpts::default()).unwrap();

        let inbox = dir.path().join("inbox.org");
        fs::write(
            &inbox,
            "#+TITLE: inbox\n\n\
             * TODO first discovered thing\nSome body line.\nAnother line.\n\
             * DONE already handled elsewhere\n\
             * TODO second discovered thing\n",
        )
        .unwrap();

        let out = fold(&layout, &inbox, "sample").unwrap();
        assert!(out.starts_with("folded 2: "), "got: {out}");

        let doc = IssueDoc::parse_file("sample", &layout.project_issues_path("sample")).unwrap();
        let titles: Vec<&str> = doc.headings.iter().map(|h| h.title.as_str()).collect();
        assert!(titles.contains(&"first discovered thing"));
        assert!(titles.contains(&"second discovered thing"));
        let folded = doc
            .headings
            .iter()
            .find(|h| h.title == "first discovered thing")
            .unwrap();
        assert!(folded.body.contains("Some body line."));

        // Headings flipped to DONE and stamped with the assigned id.
        let stamped = fs::read_to_string(&inbox).unwrap();
        assert_eq!(stamped.matches("* DONE ").count(), 3);
        assert_eq!(stamped.matches(":VISSUE_ID: sample-").count(), 2);
        assert!(!stamped.contains("* TODO "));

        // Second fold finds nothing unstamped and creates nothing.
        let again = fold(&layout, &inbox, "sample").unwrap();
        assert_eq!(again, "folded 0 (nothing unstamped)\n");
        let doc2 = IssueDoc::parse_file("sample", &layout.project_issues_path("sample")).unwrap();
        assert_eq!(doc2.headings.len(), doc.headings.len());
    }

    #[test]
    fn refile_to_moves_across_two_layouts_and_leaves_no_shadow() {
        let src_dir = tempfile::tempdir().unwrap();
        let dst_dir = tempfile::tempdir().unwrap();
        let src_layout = fresh_layout(src_dir.path());
        let dst_layout = fresh_layout(dst_dir.path());
        create(&src_layout, "misc", "wrong board", CreateOpts::default()).unwrap();
        let id = IssueDoc::parse_file("misc", &src_layout.project_issues_path("misc"))
            .unwrap()
            .headings[0]
            .id
            .clone();

        let out = refile_to(&src_layout, &id, &dst_layout, "surf").unwrap();
        assert!(out.contains("misc -> surf"), "{out}");

        // The heading is on the destination tracker, and the source root has
        // no `surf` directory standing in for it.
        let moved = IssueDoc::parse_file("surf", &dst_layout.project_issues_path("surf")).unwrap();
        assert_eq!(moved.headings.len(), 1);
        assert_eq!(moved.headings[0].id, id);
        assert!(!src_layout.project_issues_path("surf").exists());
        let left = IssueDoc::parse_file("misc", &src_layout.project_issues_path("misc")).unwrap();
        assert!(left.headings.is_empty());
    }

    #[test]
    fn reject_creates_the_successor_on_the_destination_layout() {
        let src_dir = tempfile::tempdir().unwrap();
        let dst_dir = tempfile::tempdir().unwrap();
        let src_layout = fresh_layout(src_dir.path());
        let dst_layout = fresh_layout(dst_dir.path());
        create(&src_layout, "misc", "old approach", CreateOpts::default()).unwrap();
        let src = IssueDoc::parse_file("misc", &src_layout.project_issues_path("misc"))
            .unwrap()
            .headings[0]
            .id
            .clone();

        // A twin id the destination file does not hold yet: the successor must
        // not mint it, because the routed board already uses it. Handed over as
        // the file that holds it rather than as the id, so the reservation is
        // read under the lock that guards the write.
        let twin_dir = tempfile::tempdir().unwrap();
        let twin_layout = fresh_layout(twin_dir.path());
        let twin_path = twin_layout.project_issues_path("surf");
        std::fs::create_dir_all(twin_path.parent().unwrap()).unwrap();
        std::fs::write(
            &twin_path,
            "#+TITLE: surf issues\n\n* TODO taken elsewhere\n:PROPERTIES:\n             :ID:         surf-aaaa\n:END:\n",
        )
        .unwrap();
        let twins = vec![twin_path.clone()];
        let out = reject(
            &src_layout,
            &src,
            RejectOpts {
                project: Some("surf"),
                title: Some("new approach"),
                dst_layout: Some(&dst_layout),
                dst_extra_id_paths: &twins,
                ..Default::default()
            },
        )
        .unwrap();

        assert!(!src_layout.project_issues_path("surf").exists());
        let made = IssueDoc::parse_file("surf", &dst_layout.project_issues_path("surf")).unwrap();
        assert_eq!(made.headings.len(), 1);
        assert_ne!(made.headings[0].id, "surf-aaaa");
        assert!(out.contains(&made.headings[0].id), "{out}");
        assert_eq!(issue_at(&src_layout, "misc", &src).state, "CANCELLED");
    }

    #[test]
    fn reject_to_an_existing_issue_cancels_and_wires_the_pair() {
        let dir = tempfile::tempdir().unwrap();
        let layout = fresh_layout(dir.path());
        create(&layout, "sample", "old approach", CreateOpts::default()).unwrap();
        create(&layout, "sample", "new approach", CreateOpts::default()).unwrap();
        let doc = IssueDoc::parse_file("sample", &layout.project_issues_path("sample")).unwrap();
        let src = doc.headings[0].id.clone();
        let dst = doc.headings[1].id.clone();

        let out = reject(
            &layout,
            &src,
            RejectOpts {
                to: Some(&dst),
                ..Default::default()
            },
        )
        .unwrap();
        assert!(out.contains(&src) && out.contains(&dst), "{out}");

        let src_h = issue_at(&layout, "sample", &src);
        assert_eq!(src_h.state, "CANCELLED");
        assert_eq!(
            src_h.properties.get("PIVOTED_TO").map(String::as_str),
            Some(dst.as_str())
        );
        let dst_h = issue_at(&layout, "sample", &dst);
        assert_eq!(
            dst_h.properties.get("DISCOVERED_FROM").map(String::as_str),
            Some(src.as_str())
        );
    }

    #[test]
    fn reject_creates_the_destination_in_another_project() {
        let dir = tempfile::tempdir().unwrap();
        let layout = fresh_layout(dir.path());
        create(&layout, "sample", "old approach", CreateOpts::default()).unwrap();
        let src = only_id(&layout, "sample");

        let out = reject(
            &layout,
            &src,
            RejectOpts {
                project: Some("other"),
                title: Some("new approach"),
                ..Default::default()
            },
        )
        .unwrap();

        let dst_doc = IssueDoc::parse_file("other", &layout.project_issues_path("other")).unwrap();
        assert_eq!(dst_doc.headings.len(), 1);
        let dst = &dst_doc.headings[0];
        assert_eq!(dst.title, "new approach");
        assert_eq!(
            dst.properties.get("DISCOVERED_FROM").map(String::as_str),
            Some(src.as_str())
        );
        assert!(out.contains(&src) && out.contains(&dst.id), "{out}");

        let src_h = issue_at(&layout, "sample", &src);
        assert_eq!(src_h.state, "CANCELLED");
        assert_eq!(
            src_h.properties.get("PIVOTED_TO").map(String::as_str),
            Some(dst.id.as_str())
        );
    }

    #[test]
    fn reject_refuses_an_unknown_source_or_destination() {
        let dir = tempfile::tempdir().unwrap();
        let layout = fresh_layout(dir.path());
        create(&layout, "sample", "only", CreateOpts::default()).unwrap();
        let src = only_id(&layout, "sample");

        let missing_src = reject(
            &layout,
            "sample-zzzz",
            RejectOpts {
                to: Some(&src),
                ..Default::default()
            },
        )
        .unwrap_err();
        assert!(
            matches!(missing_src, Error::IssueNotFound { .. }),
            "{missing_src}"
        );

        let missing_dst = reject(
            &layout,
            &src,
            RejectOpts {
                to: Some("sample-zzzz"),
                ..Default::default()
            },
        )
        .unwrap_err();
        assert!(
            matches!(missing_dst, Error::IssueNotFound { .. }),
            "{missing_dst}"
        );
    }

    #[test]
    fn reject_does_not_overwrite_a_nonempty_discovered_from() {
        let dir = tempfile::tempdir().unwrap();
        let layout = fresh_layout(dir.path());
        create(&layout, "sample", "origin", CreateOpts::default()).unwrap();
        create(&layout, "sample", "old approach", CreateOpts::default()).unwrap();
        create(&layout, "sample", "already sourced", CreateOpts::default()).unwrap();
        let doc = IssueDoc::parse_file("sample", &layout.project_issues_path("sample")).unwrap();
        let origin = doc.headings[0].id.clone();
        let src = doc.headings[1].id.clone();
        let dst = doc.headings[2].id.clone();

        let path = layout.project_issues_path("sample");
        let mut doc = IssueDoc::parse_file("sample", &path).unwrap();
        doc.headings
            .iter_mut()
            .find(|h| h.id == dst)
            .unwrap()
            .properties
            .insert("DISCOVERED_FROM".into(), origin.clone());
        doc.write().unwrap();

        reject(
            &layout,
            &src,
            RejectOpts {
                to: Some(&dst),
                ..Default::default()
            },
        )
        .unwrap();
        let dst_h = issue_at(&layout, "sample", &dst);
        assert_eq!(
            dst_h.properties.get("DISCOVERED_FROM").map(String::as_str),
            Some(origin.as_str()),
            "a filled DISCOVERED_FROM stays put"
        );
    }

    #[test]
    fn create_sets_discovered_from_from_the_first_known_id_link() {
        let dir = tempfile::tempdir().unwrap();
        let layout = fresh_layout(dir.path());
        create(&layout, "sample", "source", CreateOpts::default()).unwrap();
        let known = only_id(&layout, "sample");
        create(
            &layout,
            "sample",
            "fell out of it",
            CreateOpts {
                body: Some(&format!("See [[id:{known}]] for the parent finding.")),
                ..Default::default()
            },
        )
        .unwrap();
        let doc = IssueDoc::parse_file("sample", &layout.project_issues_path("sample")).unwrap();
        let child = doc
            .headings
            .iter()
            .find(|h| h.title == "fell out of it")
            .unwrap();
        assert_eq!(
            child.properties.get("DISCOVERED_FROM").map(String::as_str),
            Some(known.as_str())
        );
    }

    #[test]
    fn create_ignores_an_id_link_that_is_not_in_the_corpus() {
        let dir = tempfile::tempdir().unwrap();
        let layout = fresh_layout(dir.path());
        create(
            &layout,
            "sample",
            "orphan mention",
            CreateOpts {
                body: Some("See [[id:sample-zzzz]] which does not exist."),
                ..Default::default()
            },
        )
        .unwrap();
        let h = issue_at(&layout, "sample", &only_id(&layout, "sample"));
        assert!(
            !h.properties.contains_key("DISCOVERED_FROM"),
            "unknown [[id:]] must not mint DISCOVERED_FROM: {h:?}"
        );
        assert!(
            !h.properties.contains_key("BLOCKED_BY"),
            "prose must not mint BLOCKED_BY: {h:?}"
        );
    }

    #[test]
    fn related_after_reject_names_the_successor_without_a_body_link() {
        let dir = tempfile::tempdir().unwrap();
        let layout = fresh_layout(dir.path());
        create(&layout, "sample", "old approach", CreateOpts::default()).unwrap();
        create(&layout, "sample", "new approach", CreateOpts::default()).unwrap();
        let doc = IssueDoc::parse_file("sample", &layout.project_issues_path("sample")).unwrap();
        let src = doc.headings[0].id.clone();
        let dst = doc.headings[1].id.clone();
        reject(
            &layout,
            &src,
            RejectOpts {
                to: Some(&dst),
                ..Default::default()
            },
        )
        .unwrap();

        assert!(
            !issue_at(&layout, "sample", &src).body.contains(&dst),
            "the pair is wired by PIVOTED_TO, not prose"
        );
        let from_src = crate::related::related(&layout, &src, 1, 10, "text").unwrap();
        assert!(from_src.contains(&dst), "{from_src}");
        assert!(from_src.contains("pivoted_to"), "{from_src}");

        let from_dst = crate::related::related(&layout, &dst, 1, 10, "text").unwrap();
        assert!(from_dst.contains(&src), "{from_dst}");
        assert!(from_dst.contains("successor_of"), "{from_dst}");

        let waiting = crate::report::backlinks(&layout, &dst).unwrap();
        assert!(waiting.contains(&src), "{waiting}");
    }

    #[test]
    fn update_to_cancelled_emits_state_change_with_the_id() {
        let dir = tempfile::tempdir().unwrap();
        let layout = fresh_layout(dir.path());
        create(&layout, "sample", "first", CreateOpts::default()).unwrap();
        let id = only_id(&layout, "sample");
        let before = crate::events::generation(&layout);
        update(&layout, &id, Some("CANCELLED"), None, None, None).unwrap();
        let events = crate::events::since(&layout, before, 50).unwrap();
        assert!(
            events.iter().any(|e| {
                e.kind == "state_change"
                    && e.id.as_deref() == Some(id.as_str())
                    && e.detail.as_deref() == Some("TODO->CANCELLED")
            }),
            "{events:?}"
        );
    }

    #[test]
    fn a_stale_done_after_reject_is_refused_and_the_source_stays_cancelled() {
        let dir = tempfile::tempdir().unwrap();
        let layout = fresh_layout(dir.path());
        create(&layout, "sample", "old plan", CreateOpts::default()).unwrap();
        create(&layout, "sample", "rewrite", CreateOpts::default()).unwrap();
        let doc = IssueDoc::parse_file("sample", &layout.project_issues_path("sample")).unwrap();
        let src = doc.headings[0].id.clone();
        let dst = doc.headings[1].id.clone();
        reject(
            &layout,
            &src,
            RejectOpts {
                to: Some(&dst),
                ..Default::default()
            },
        )
        .unwrap();

        let err = update_pred(
            &layout,
            &src,
            Some("DONE"),
            None,
            None,
            None,
            UpdatePred {
                if_state: Some("STARTED"),
                if_gen: None,
            },
        )
        .unwrap_err();
        assert!(
            matches!(
                err,
                Error::StaleWrite {
                    ref actual_state,
                    ref expected_state,
                    ..
                } if actual_state == "CANCELLED" && expected_state.as_deref() == Some("STARTED")
            ),
            "{err:?}"
        );
        assert_eq!(issue_at(&layout, "sample", &src).state, "CANCELLED");
    }

    #[test]
    fn if_gen_refuses_when_the_corpus_moved() {
        let dir = tempfile::tempdir().unwrap();
        let layout = fresh_layout(dir.path());
        create(&layout, "sample", "first", CreateOpts::default()).unwrap();
        let id = only_id(&layout, "sample");
        let seen = crate::events::generation(&layout);
        update(&layout, &id, Some("STARTED"), None, None, None).unwrap();
        let err = update_pred(
            &layout,
            &id,
            Some("DONE"),
            None,
            None,
            None,
            UpdatePred {
                if_state: None,
                if_gen: Some(seen),
            },
        )
        .unwrap_err();
        assert!(matches!(err, Error::StaleWrite { .. }), "{err:?}");
        assert_eq!(issue_at(&layout, "sample", &id).state, "STARTED");
    }

    #[test]
    fn a_second_terminal_does_not_drop_the_first() {
        let dir = tempfile::tempdir().unwrap();
        let layout = fresh_layout(dir.path());
        create(&layout, "sample", "first", CreateOpts::default()).unwrap();
        let id = only_id(&layout, "sample");
        update(&layout, &id, Some("DONE"), None, None, None).unwrap();
        update(&layout, &id, Some("CANCELLED"), None, None, None).unwrap();
        let h = issue_at(&layout, "sample", &id);
        assert_eq!(h.state, "DONE", "first terminal must stay");
        assert_eq!(
            crate::props::get(&h.properties, crate::props::SIBLING_TERMINAL),
            Some("CANCELLED")
        );

        resolve_terminal(&layout, &id, "CANCELLED").unwrap();
        let h = issue_at(&layout, "sample", &id);
        assert_eq!(h.state, "CANCELLED");
        assert!(crate::props::get(&h.properties, crate::props::SIBLING_TERMINAL).is_none());
    }

    #[test]
    fn check_warns_on_reject_prose_done_and_a_mention_without_an_edge() {
        let dir = tempfile::tempdir().unwrap();
        let layout = fresh_layout(dir.path());
        create(&layout, "sample", "shipped", CreateOpts::default()).unwrap();
        create(&layout, "sample", "other", CreateOpts::default()).unwrap();
        let doc = IssueDoc::parse_file("sample", &layout.project_issues_path("sample")).unwrap();
        let shipped = doc.headings[0].id.clone();
        let other = doc.headings[1].id.clone();
        update(&layout, &shipped, Some("DONE"), None, None, None).unwrap();
        append_body(&layout, &shipped, "superseded by the other one, bounced").unwrap();
        append_body(
            &layout,
            &other,
            &format!("discovered while reading [[id:{shipped}]]"),
        )
        .unwrap();

        let report = crate::report::check(&layout).unwrap();
        assert!(
            report.text.contains(&shipped)
                && report.text.contains("DONE but the body reads as a reject"),
            "{}",
            report.text
        );
        assert!(
            report.text.contains(&other)
                && report
                    .text
                    .contains("as discovered or pivoted with no edge"),
            "{}",
            report.text
        );
        assert!(report.warnings >= 2, "{}", report.text);
    }

    // The word is not the finding. Every bug about input validation says
    // "rejected", and three issues in one corpus were flagged for sentences
    // about what the software does to bad input.
    #[test]
    fn check_is_quiet_about_a_done_issue_that_merely_uses_the_word_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let layout = fresh_layout(dir.path());
        create(&layout, "sample", "validation", CreateOpts::default()).unwrap();
        let doc = IssueDoc::parse_file("sample", &layout.project_issues_path("sample")).unwrap();
        let id = doc.headings[0].id.clone();
        update(&layout, &id, Some("DONE"), None, None, None).unwrap();
        append_body(
            &layout,
            &id,
            "A compound spec is silently corrupted rather than rejected, and the \
             alternative parser was rejected as strictly dominated.",
        )
        .unwrap();

        let report = crate::report::check(&layout).unwrap();
        assert!(
            !report.text.contains("reads as a reject"),
            "the word alone was read as an outcome: {}",
            report.text
        );
    }

    // A "Supersedes" section rolls up issues this one did not close, which is the
    // opposite of being superseded, and the two differ by one letter.
    #[test]
    fn check_reads_supersedes_as_a_roll_up_and_superseded_by_as_an_outcome() {
        let dir = tempfile::tempdir().unwrap();
        let layout = fresh_layout(dir.path());
        create(&layout, "sample", "umbrella", CreateOpts::default()).unwrap();
        create(&layout, "sample", "replaced", CreateOpts::default()).unwrap();
        let doc = IssueDoc::parse_file("sample", &layout.project_issues_path("sample")).unwrap();
        let rollup = doc.headings[0].id.clone();
        let replaced = doc.headings[1].id.clone();
        update(&layout, &rollup, Some("DONE"), None, None, None).unwrap();
        update(&layout, &replaced, Some("DONE"), None, None, None).unwrap();
        append_body(&layout, &rollup, "** Supersedes\nrolls up the pieces").unwrap();
        append_body(&layout, &replaced, "superseded by the umbrella").unwrap();

        let report = crate::report::check(&layout).unwrap();
        let flagged: Vec<&str> = report
            .text
            .lines()
            .filter(|l| l.contains("reads as a reject"))
            .collect();

        assert!(
            flagged.iter().any(|l| l.contains(&replaced)),
            "an issue that says it was superseded was not flagged: {}",
            report.text
        );
        assert!(
            !flagged.iter().any(|l| l.contains(&rollup)),
            "a Supersedes roll-up was read as its own rejection: {}",
            report.text
        );
    }

    // A body links other issues for every reason there is. Only the reason the
    // properties name is a finding.
    #[test]
    fn check_is_quiet_about_a_mention_that_claims_no_relation() {
        let dir = tempfile::tempdir().unwrap();
        let layout = fresh_layout(dir.path());
        create(&layout, "sample", "umbrella", CreateOpts::default()).unwrap();
        create(&layout, "sample", "piece", CreateOpts::default()).unwrap();
        let doc = IssueDoc::parse_file("sample", &layout.project_issues_path("sample")).unwrap();
        let umbrella = doc.headings[0].id.clone();
        let piece = doc.headings[1].id.clone();
        append_body(
            &layout,
            &umbrella,
            &format!("** Supersedes\nRolls up [[id:{piece}]], which it does not close."),
        )
        .unwrap();

        let report = crate::report::check(&layout).unwrap();
        assert!(
            !report.text.contains("as discovered or pivoted"),
            "a roll-up was read as a discovery: {}",
            report.text
        );
    }

    // And the claim has to be near the link: a long issue says many things.
    #[test]
    fn check_reads_a_discovery_claim_only_near_the_link_it_belongs_to() {
        let dir = tempfile::tempdir().unwrap();
        let layout = fresh_layout(dir.path());
        create(&layout, "sample", "long", CreateOpts::default()).unwrap();
        create(&layout, "sample", "elsewhere", CreateOpts::default()).unwrap();
        let doc = IssueDoc::parse_file("sample", &layout.project_issues_path("sample")).unwrap();
        let long = doc.headings[0].id.clone();
        let elsewhere = doc.headings[1].id.clone();
        let filler = "prose ".repeat(120);
        append_body(
            &layout,
            &long,
            &format!("discovered while auditing the loader.\n{filler}\nsee [[id:{elsewhere}]]"),
        )
        .unwrap();

        let report = crate::report::check(&layout).unwrap();
        assert!(
            !report.text.contains("as discovered or pivoted"),
            "a claim in another section was attached to this link: {}",
            report.text
        );
    }

    // A parent naming its child is a stated relation the tracker already holds.
    #[test]
    fn check_is_quiet_about_a_mention_that_a_parent_edge_already_explains() {
        let dir = tempfile::tempdir().unwrap();
        let layout = fresh_layout(dir.path());
        create(&layout, "sample", "umbrella", CreateOpts::default()).unwrap();
        let doc = IssueDoc::parse_file("sample", &layout.project_issues_path("sample")).unwrap();
        let parent = doc.headings[0].id.clone();
        create(
            &layout,
            "sample",
            "piece",
            CreateOpts {
                parent: Some(parent.as_str()),
                ..CreateOpts::default()
            },
        )
        .unwrap();
        let doc = IssueDoc::parse_file("sample", &layout.project_issues_path("sample")).unwrap();
        let child = doc
            .headings
            .iter()
            .find(|h| h.id != parent)
            .map(|h| h.id.clone())
            .unwrap();
        // The prose claims a discovery, so the warning would fire on this pair
        // if the parent edge were not recognised. Without the claim the test
        // would pass whatever edge_connects does, and asserting the absence of
        // the old wording would pass even with the fix reverted.
        append_body(
            &layout,
            &parent,
            &format!("discovered while reading [[id:{child}]]"),
        )
        .unwrap();
        create(&layout, "sample", "unrelated", CreateOpts::default()).unwrap();
        let doc = IssueDoc::parse_file("sample", &layout.project_issues_path("sample")).unwrap();
        let stranger = doc
            .headings
            .iter()
            .find(|h| h.id != parent && h.id != child)
            .map(|h| h.id.clone())
            .unwrap();
        append_body(
            &layout,
            &stranger,
            &format!("discovered while reading [[id:{parent}]]"),
        )
        .unwrap();

        let report = crate::report::check(&layout).unwrap();
        let flagged: Vec<&str> = report
            .text
            .lines()
            .filter(|l| l.contains("as discovered or pivoted"))
            .collect();
        assert!(
            flagged.iter().any(|l| l.contains(&stranger)),
            "the control pair with no edge was not flagged, so this test proves nothing: {}",
            report.text
        );
        assert!(
            !flagged
                .iter()
                .any(|l| l.contains(&parent) && l.contains(&child)),
            "a parent edge did not count as a relation: {}",
            report.text
        );
    }

    #[test]
    fn check_names_a_file_missing_category_and_a_type_not_on_the_heading() {
        let dir = tempfile::tempdir().unwrap();
        let layout = fresh_layout(dir.path());
        let path = layout.project_issues_path("sample");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(
            &path,
            "#+TITLE: sample issues\n#+TODO: TODO STARTED BLOCKED | DONE CANCELLED\n\n* TODO [#A] Untagged type\n:PROPERTIES:\n:ID:         sample-aaaa\n:TYPE:       bug\n:END:\n",
        )
        .unwrap();
        let report = crate::report::check(&layout).unwrap();
        assert!(
            report.text.contains("sample: preamble has no #+CATEGORY:"),
            "{}",
            report.text
        );
        assert!(
            report
                .text
                .contains("have :TYPE: that is a legal Org tag but is not on the heading"),
            "{}",
            report.text
        );
        assert!(
            report
                .text
                .contains("preamble has no #+VISSUE: protocol stamp"),
            "{}",
            report.text
        );
        assert!(
            report.text.contains("preamble has no #+PRIORITIES:"),
            "{}",
            report.text
        );
    }

    #[test]
    fn check_errors_on_a_newer_protocol_stamp() {
        let dir = tempfile::tempdir().unwrap();
        let layout = fresh_layout(dir.path());
        let path = layout.project_issues_path("sample");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(
            &path,
            "#+TITLE: sample issues\n#+VISSUE: 99\n#+CATEGORY: sample\n#+FILETAGS: :issues:sample:noexport:\n#+TAGS: docs\n#+TODO: TODO | DONE\n\n* TODO [#A] Future\n:PROPERTIES:\n:ID:         sample-aaaa\n:END:\n",
        )
        .unwrap();
        let report = crate::report::check(&layout).unwrap();
        assert!(report.errors >= 1, "{}", report.text);
        assert!(
            report
                .text
                .contains("#+VISSUE: 99 is newer than this vissue"),
            "{}",
            report.text
        );
    }

    #[test]
    fn normalize_rewrites_legacy_keys_and_keeps_edna() {
        let dir = tempfile::tempdir().unwrap();
        let layout = fresh_layout(dir.path());
        let path = layout.project_issues_path("sample");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(
            &path,
            "#+TITLE: sample issues\n#+TODO: TODO STARTED BLOCKED | DONE CANCELLED\n\n* TODO [#A] Legacy\n:PROPERTIES:\n:ID:         sample-aaaa\n:TYPE:       bug\n:PARENT:     sample-root\n:BLOCKEDBY:  sample-bbbb\n:END:\n\n* TODO [#A] Edna condition\n:PROPERTIES:\n:ID:         sample-cccc\n:BLOCKER:    prev-sibling\n:END:\n",
        )
        .unwrap();
        let dry = normalize(&layout, Some("sample"), true).unwrap();
        assert!(dry.contains("would rewrite"), "{dry}");
        let on_disk = std::fs::read_to_string(&path).unwrap();
        assert!(on_disk.contains(":TYPE:"), "{on_disk}");
        let wrote = normalize(&layout, Some("sample"), false).unwrap();
        assert!(wrote.contains("rewrote"), "{wrote}");
        let after = std::fs::read_to_string(&path).unwrap();
        assert!(after.contains("#+CATEGORY: sample"), "{after}");
        assert!(after.contains("#+PRIORITIES: A C C"), "{after}");
        assert!(after.contains(":TYPE:       bug"), "{after}");
        assert!(after.contains(":PARENT:"), "{after}");
        assert!(after.contains(":BLOCKED_BY:"), "{after}");
        assert!(
            !after.contains("ids(sample-bbbb)"),
            "normalize must not mint edna ids(): {after}"
        );
        assert!(after.contains("prev-sibling"), "{after}");
    }
    /// The reservation has to be read after the lock is taken, not before.
    ///
    /// Deterministic rather than a stress test, because a stress test has no
    /// power here: the suffix space is 36^n and two racing creates almost never
    /// collide by luck, so a run that passes proves nothing. This forces the
    /// question instead. With `id_length = 2` the space is 1296 suffixes; the
    /// twin layout is handed 1295 of them, so exactly one is free and a mint
    /// that reads the twin has no choice but to return it.
    ///
    /// A mint that trusts a caller's snapshot, which is what `extra_ids` is,
    /// picks from the whole space and returns that one suffix with probability
    /// 1/1296.
    #[test]
    fn the_reservation_is_read_after_the_lock_is_held() {
        let dir = tempfile::tempdir().unwrap();
        let own_root = dir.path().join("own");
        let twin_root = dir.path().join("twin");
        std::fs::create_dir_all(&own_root).unwrap();
        std::fs::create_dir_all(&twin_root).unwrap();
        std::fs::write(own_root.join("vissue.toml"), "[issues]\nid_length = 2\n").unwrap();
        let own = fresh_layout(&own_root);
        let twin = fresh_layout(&twin_root);

        // Every suffix but "zz", written straight to the twin file.
        let mut body = String::from("#+TITLE: sample issues\n\n");
        let alphabet = b"0123456789abcdefghijklmnopqrstuvwxyz";
        for a in alphabet {
            for b in alphabet {
                if *a == b'z' && *b == b'z' {
                    continue;
                }
                let id = format!("sample-{}{}", *a as char, *b as char);
                body.push_str(&format!(
                    "* TODO filler {id}\n:PROPERTIES:\n:ID:         {id}\n:END:\n\n"
                ));
            }
        }
        let twin_path = twin.project_issues_path("sample");
        std::fs::create_dir_all(twin_path.parent().unwrap()).unwrap();
        std::fs::write(&twin_path, body).unwrap();

        let twins = vec![twin_path.clone()];
        let id = create(
            &own,
            "sample",
            "the only suffix left",
            CreateOpts {
                quiet: true,
                extra_id_paths: &twins,
                ..Default::default()
            },
        )
        .expect("create failed")
        .trim()
        .to_string();

        assert_eq!(
            id, "sample-zz",
            "the mint did not treat the twin file as taken, so it read the reservation \
             before the lock rather than after"
        );
    }

    /// And the twin being the file under write is ordinary, not a deadlock.
    /// `extra_id_paths_for` returns every layout for the project including this
    /// one, so the write path arrives in its own reservation list on every
    /// routed create.
    #[test]
    fn the_written_file_appearing_in_its_own_reservation_is_not_a_deadlock() {
        let dir = tempfile::tempdir().unwrap();
        let layout = fresh_layout(dir.path());
        let own_path = layout.project_issues_path("sample");
        let twins = vec![own_path.clone(), own_path.clone()];
        let id = create(
            &layout,
            "sample",
            "self referential reservation",
            CreateOpts {
                quiet: true,
                extra_id_paths: &twins,
                ..Default::default()
            },
        )
        .expect("create deadlocked or failed")
        .trim()
        .to_string();
        assert!(id.starts_with("sample-"), "{id}");
    }
}
