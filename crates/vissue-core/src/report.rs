//! Read-only verbs. Every function returns its text instead of printing, so a
//! CLI, an MCP server, and a library caller share one implementation.

use anyhow::{bail, Result};
use chrono::{Local, NaiveDate};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt::Write as _;

use crate::config::Layout;
use crate::model::{IssueHeading, READY_STATES};
use crate::store::{collect_org_ids, find_by_id, list_projects, load_all, IssueDoc};

/// One row per issue: id, state, priority cookie, title.
pub fn list(
    layout: &Layout,
    project_filter: Option<&str>,
    state_filter: Option<&str>,
    ready_only: bool,
) -> Result<String> {
    let projects = match project_filter {
        Some(p) => vec![p.to_string()],
        None => list_projects(layout)?,
    };

    let mut rows: Vec<(String, String, String, char, String)> = Vec::new();

    for project in projects {
        let path = layout.project_issues_path(&project);
        let doc = IssueDoc::parse_file(&project, &path)?;

        let active_blockers: HashSet<String> = if ready_only {
            doc.headings
                .iter()
                .filter(|h| h.state != "DONE" && h.state != "CANCELLED")
                .map(|h| h.id.clone())
                .collect()
        } else {
            HashSet::new()
        };

        for h in &doc.headings {
            if let Some(s) = state_filter {
                if h.state != s {
                    continue;
                }
            }
            if ready_only {
                if !READY_STATES.contains(&h.state.as_str()) {
                    continue;
                }
                if h.blocked_by().iter().any(|b| active_blockers.contains(b)) {
                    continue;
                }
            }
            rows.push((
                project.clone(),
                h.id.clone(),
                h.state.clone(),
                h.priority,
                format!("{}{}", h.title, claim_suffix(h)),
            ));
        }
    }

    rows.sort_by(|a, b| {
        a.3.cmp(&b.3)
            .then_with(|| a.2.cmp(&b.2))
            .then_with(|| a.1.cmp(&b.1))
    });

    let mut out = String::new();
    for (_project, id, state, prio, title) in rows {
        let _ = writeln!(out, "{id:<22} {state:<9} [#{prio}]  {title}");
    }
    Ok(out)
}

/// ` (claimed 3d by <identity>)`, or nothing when no one holds the issue.
/// Only a claimed issue grows the suffix, so an unclaimed corpus renders
/// exactly as it did before claims existed.
pub(crate) fn claim_suffix(h: &IssueHeading) -> String {
    let Some(who) = h.claimed_by() else {
        return String::new();
    };
    match h.claim_age_days(Local::now().date_naive()) {
        Some(days) => format!("  (claimed {days}d by {who})"),
        None => format!("  (claimed by {who})"),
    }
}

/// Actionable issues: TODO or STARTED with no open blocker.
pub fn ready(layout: &Layout, project_filter: Option<&str>) -> Result<String> {
    list(layout, project_filter, None, true)
}

/// One issue's metadata and file range. The body stays in the file: an editor
/// opens the range when the prose is wanted.
pub fn show(layout: &Layout, id: &str) -> Result<String> {
    let (h, path, project) =
        find_by_id(layout, id)?.ok_or_else(|| anyhow::anyhow!("issue {id} not found"))?;
    let mut out = String::new();
    writeln!(out, "ID:       {}", h.id)?;
    writeln!(out, "Project:  {project}")?;
    writeln!(out, "Title:    {}", h.title)?;
    writeln!(out, "State:    {}", h.state)?;
    writeln!(out, "Priority: [#{}]", h.priority)?;
    if let Some(who) = h.claimed_by() {
        match h.claim_age_days(Local::now().date_naive()) {
            Some(days) => writeln!(
                out,
                "Claimed:  {who} since {} ({days}d)",
                h.claimed_at().unwrap_or("?")
            )?,
            None => writeln!(out, "Claimed:  {who}")?,
        }
    }
    if h.properties.iter().any(|(k, _)| k != "ID") {
        writeln!(out, "Properties:")?;
        for (k, v) in &h.properties {
            if k == "ID" {
                continue;
            }
            writeln!(out, "  {k}: {v}")?;
        }
    }
    writeln!(
        out,
        "File:     {}:{}-{}",
        path.display(),
        h.line_start,
        h.line_end
    )?;
    writeln!(out)?;
    writeln!(
        out,
        "(body lives in file; open the range above to read or edit)"
    )?;
    Ok(out)
}

/// Case-insensitive substring scan over id, title, properties, and body. Linear
/// in the corpus, which is the right cost until the issue count climbs.
pub fn search(layout: &Layout, query: &str, limit: usize) -> Result<String> {
    let needle = query.to_lowercase();
    let mut hits: Vec<(String, IssueHeading)> = Vec::new();

    for (project, h) in load_all(layout)? {
        let mut hay = String::new();
        hay.push_str(&h.id);
        hay.push(' ');
        hay.push_str(&h.title);
        hay.push(' ');
        for (k, v) in &h.properties {
            hay.push_str(k);
            hay.push(':');
            hay.push_str(v);
            hay.push(' ');
        }
        hay.push_str(&h.body);
        if hay.to_lowercase().contains(&needle) {
            hits.push((project, h));
        }
    }

    hits.sort_by(|a, b| {
        a.1.priority
            .cmp(&b.1.priority)
            .then_with(|| a.1.state.cmp(&b.1.state))
            .then_with(|| a.1.id.cmp(&b.1.id))
    });
    hits.truncate(limit);

    let mut out = String::new();
    for (project, h) in hits {
        let _ = writeln!(
            out,
            "{:<22} {:<9} [#{}]  {}  ({})",
            h.id, h.state, h.priority, h.title, project
        );
    }
    Ok(out)
}

/// Issues whose `:PARENT:` points at `parent_id`.
pub fn children(layout: &Layout, parent_id: &str) -> Result<String> {
    let mut rows: Vec<(String, IssueHeading)> = load_all(layout)?
        .into_iter()
        .filter(|(_, h)| h.parent() == Some(parent_id))
        .collect();
    rows.sort_by(|a, b| {
        a.1.priority
            .cmp(&b.1.priority)
            .then_with(|| a.1.state.cmp(&b.1.state))
            .then_with(|| a.1.id.cmp(&b.1.id))
    });
    let mut out = String::new();
    for (project, h) in rows {
        let _ = writeln!(
            out,
            "{:<22} {:<9} [#{}]  {}  ({})",
            h.id, h.state, h.priority, h.title, project
        );
    }
    Ok(out)
}

/// Open issues whose `:CREATED:` is at least `days` old. An issue without a
/// parseable date is never stale, because its age is unknown.
pub fn stale(layout: &Layout, days: i64, project_filter: Option<&str>) -> Result<String> {
    let today = Local::now().date_naive();
    let cutoff = today - chrono::Duration::days(days);
    let mut rows: Vec<(String, IssueHeading, NaiveDate)> = Vec::new();
    for (project, h) in load_all(layout)? {
        if let Some(p) = project_filter {
            if project != p {
                continue;
            }
        }
        if !READY_STATES.contains(&h.state.as_str()) {
            continue;
        }
        let Some(created) = h.properties.get("CREATED") else {
            continue;
        };
        let Some(parsed) = parse_org_date(created) else {
            continue;
        };
        if parsed <= cutoff {
            rows.push((project, h, parsed));
        }
    }
    rows.sort_by_key(|r| r.2);
    let mut out = String::new();
    for (project, h, created) in rows {
        let age = (today - created).num_days();
        let _ = writeln!(
            out,
            "{:<22} {:<9} [#{}]  {} ({}d, {})",
            h.id, h.state, h.priority, h.title, age, project
        );
    }
    Ok(out)
}

pub(crate) fn parse_org_date(s: &str) -> Option<NaiveDate> {
    let inner = s
        .trim_start_matches(['<', '['])
        .trim_end_matches(['>', ']']);
    let token = inner.split_whitespace().next()?;
    NaiveDate::parse_from_str(token, "%Y-%m-%d").ok()
}

/// The matching issue count and nothing else, for shell pipelines.
pub fn count(
    layout: &Layout,
    project_filter: Option<&str>,
    state_filter: Option<&str>,
    ready_only: bool,
) -> Result<String> {
    let all = load_all(layout)?;
    let active_blockers: HashSet<String> = if ready_only {
        all.iter()
            .filter(|(_, h)| h.state != "DONE" && h.state != "CANCELLED")
            .map(|(_, h)| h.id.clone())
            .collect()
    } else {
        HashSet::new()
    };
    let n = all
        .iter()
        .filter(|(project, h)| {
            if let Some(p) = project_filter {
                if project != p {
                    return false;
                }
            }
            if let Some(s) = state_filter {
                if h.state != s {
                    return false;
                }
            }
            if ready_only {
                if !READY_STATES.contains(&h.state.as_str()) {
                    return false;
                }
                if h.blocked_by().iter().any(|b| active_blockers.contains(b)) {
                    return false;
                }
            }
            true
        })
        .count();
    Ok(format!("{n}\n"))
}

/// One JSON object per line: every property, the logbook, the body, and the
/// file line range. Round-trippable, and the seam other tools consume.
pub fn export(layout: &Layout, project_filter: Option<&str>) -> Result<String> {
    let mut out = String::new();
    for (project, h) in load_all(layout)? {
        if let Some(p) = project_filter {
            if project != p {
                continue;
            }
        }
        let logbook: Vec<serde_json::Value> = h
            .logbook
            .iter()
            .map(|e| {
                serde_json::json!({
                    "timestamp": e.timestamp,
                    "from": e.from_state,
                    "to": e.to_state,
                    "note": e.note,
                })
            })
            .collect();
        let row = serde_json::json!({
            "id": h.id,
            "project": project,
            "title": h.title,
            "state": h.state,
            "priority": h.priority.to_string(),
            "properties": h.properties,
            "logbook": logbook,
            "body": h.body,
            "line_start": h.line_start,
            "line_end": h.line_end,
        });
        let _ = writeln!(out, "{}", row);
    }
    Ok(out)
}

/// Children and blockers below `root_id`, as indented text or Graphviz DOT.
pub fn tree(layout: &Layout, root_id: &str, format: &str) -> Result<String> {
    let all = load_all(layout)?;
    let by_id: HashMap<String, &IssueHeading> =
        all.iter().map(|(_, h)| (h.id.clone(), h)).collect();
    if !by_id.contains_key(root_id) {
        bail!("issue {root_id} not found");
    }
    let mut children_map: HashMap<String, Vec<String>> = HashMap::new();
    for (_, h) in &all {
        if let Some(parent) = h.parent() {
            children_map
                .entry(parent.to_string())
                .or_default()
                .push(h.id.clone());
        }
    }
    let mut out = String::new();
    match format {
        "ascii" | "text" => tree_ascii(
            &by_id,
            &children_map,
            root_id,
            0,
            &mut HashSet::new(),
            &mut out,
        ),
        "dot" => tree_dot(&by_id, &children_map, root_id, &mut out),
        _ => bail!("unknown format {format:?}; allowed: ascii, dot"),
    }
    Ok(out)
}

fn tree_ascii(
    by_id: &HashMap<String, &IssueHeading>,
    children_map: &HashMap<String, Vec<String>>,
    id: &str,
    depth: usize,
    seen: &mut HashSet<String>,
    out: &mut String,
) {
    if !seen.insert(id.to_string()) {
        let _ = writeln!(out, "{}{id} (cycle, stopping)", "  ".repeat(depth));
        return;
    }
    let Some(h) = by_id.get(id) else {
        let _ = writeln!(out, "{}{id} (missing)", "  ".repeat(depth));
        return;
    };
    let _ = writeln!(
        out,
        "{}{id} {:<9} [#{}]  {}",
        "  ".repeat(depth),
        h.state,
        h.priority,
        h.title
    );
    for blocker in h.blocked_by() {
        if !blocker.is_empty() {
            let _ = writeln!(out, "{}* blocked-by {blocker}", "  ".repeat(depth + 1));
        }
    }
    if let Some(kids) = children_map.get(id) {
        let mut kids = kids.clone();
        kids.sort();
        for k in kids {
            tree_ascii(by_id, children_map, &k, depth + 1, seen, out);
        }
    }
}

fn tree_dot(
    by_id: &HashMap<String, &IssueHeading>,
    children_map: &HashMap<String, Vec<String>>,
    root_id: &str,
    out: &mut String,
) {
    let _ = writeln!(out, "digraph vissue_tree {{");
    let _ = writeln!(out, "  rankdir=LR;");
    let _ = writeln!(
        out,
        "  node [shape=box, fontname=\"Jost\", style=filled, fillcolor=\"#E0F2F1\"];"
    );
    let mut visited: HashSet<String> = HashSet::new();
    let mut stack = vec![root_id.to_string()];
    while let Some(id) = stack.pop() {
        if !visited.insert(id.clone()) {
            continue;
        }
        if let Some(h) = by_id.get(&id) {
            let _ = writeln!(
                out,
                "  \"{}\" [label=\"{}\\n{} [#{}]\"];",
                h.id,
                h.title.replace('"', "\\\""),
                h.state,
                h.priority
            );
            if let Some(kids) = children_map.get(&id) {
                for k in kids {
                    let _ = writeln!(out, "  \"{}\" -> \"{}\" [color=\"#00897B\"];", h.id, k);
                    stack.push(k.clone());
                }
            }
            for b in h.blocked_by() {
                if !b.is_empty() {
                    let _ = writeln!(
                        out,
                        "  \"{}\" -> \"{}\" [style=dashed, color=\"#FF7043\", label=\"blocks\"];",
                        b, h.id
                    );
                    stack.push(b);
                }
            }
        }
    }
    let _ = writeln!(out, "}}");
}

/// Cycles in the blocker graph, one per line, or a line saying there are none.
pub fn cycles(layout: &Layout) -> Result<String> {
    let all = load_all(layout)?;
    let by_id: HashMap<String, &IssueHeading> =
        all.iter().map(|(_, h)| (h.id.clone(), h)).collect();

    let mut found: Vec<Vec<String>> = Vec::new();
    for (_, start) in &all {
        let mut path: Vec<String> = vec![start.id.clone()];
        let mut frontier = start.blocked_by();
        while let Some(b) = frontier.pop() {
            if path.contains(&b) {
                let cycle_start = path.iter().position(|x| x == &b).unwrap();
                let mut cycle = path[cycle_start..].to_vec();
                cycle.push(b.clone());
                cycle.sort();
                if !found.iter().any(|c| c == &cycle) {
                    found.push(cycle);
                }
                break;
            }
            path.push(b.clone());
            if let Some(h) = by_id.get(&b) {
                frontier.extend(h.blocked_by());
            }
        }
    }
    let mut out = String::new();
    if found.is_empty() {
        let _ = writeln!(out, "no cycles");
    } else {
        for cycle in found {
            let _ = writeln!(out, "{}", cycle.join(" -> "));
        }
    }
    Ok(out)
}

/// The whole blocker and parent graph as Graphviz DOT. Node fill encodes state.
pub fn graph(layout: &Layout, project_filter: Option<&str>) -> Result<String> {
    let all = load_all(layout)?;
    let mut out = String::new();
    writeln!(out, "digraph vissue_graph {{")?;
    writeln!(out, "  rankdir=LR;")?;
    writeln!(out, "  node [shape=box, fontname=\"Jost\", style=filled];")?;
    writeln!(out, "  edge [fontname=\"Jost\"];")?;
    for (project, h) in &all {
        if let Some(p) = project_filter {
            if project != p {
                continue;
            }
        }
        let fill = match h.state.as_str() {
            "DONE" => "#A5D6A7",
            "CANCELLED" => "#CFD8DC",
            "BLOCKED" => "#FFCC80",
            "STARTED" => "#80CBC4",
            _ => "#E0F2F1",
        };
        let _ = writeln!(
            out,
            "  \"{}\" [label=\"{}\\n{} [#{}]\", fillcolor=\"{}\"];",
            h.id,
            h.title.replace('"', "\\\""),
            h.state,
            h.priority,
            fill
        );
    }
    for (project, h) in &all {
        if let Some(p) = project_filter {
            if project != p {
                continue;
            }
        }
        for b in h.blocked_by() {
            if !b.is_empty() {
                writeln!(out, "  \"{}\" -> \"{}\" [color=\"#FF7043\"];", b, h.id)?;
            }
        }
        if let Some(parent) = h.parent() {
            writeln!(
                out,
                "  \"{}\" -> \"{}\" [color=\"#00897B\", style=dashed];",
                parent, h.id
            )?;
        }
    }
    writeln!(out, "}}")?;
    Ok(out)
}

/// A markdown roadmap grouped by project and state. Closed items collapse into
/// one section so the document stays about live work.
pub fn roadmap(layout: &Layout, project_filter: Option<&str>) -> Result<String> {
    let all = load_all(layout)?;
    let mut by_project: BTreeMap<String, Vec<&IssueHeading>> = BTreeMap::new();
    for (project, h) in &all {
        if let Some(p) = project_filter {
            if project != p {
                continue;
            }
        }
        by_project.entry(project.clone()).or_default().push(h);
    }
    let mut out = String::new();
    writeln!(out, "# Roadmap")?;
    writeln!(out)?;
    writeln!(
        out,
        "Generated from `vissue roadmap`. Source of truth lives in the per-project issues.org files."
    )?;
    writeln!(out)?;
    for (project, mut headings) in by_project {
        headings.sort_by(|a, b| {
            a.priority
                .cmp(&b.priority)
                .then_with(|| a.state.cmp(&b.state))
                .then_with(|| a.id.cmp(&b.id))
        });
        let buckets = ["STARTED", "TODO", "BLOCKED"];
        let active: Vec<&&IssueHeading> = headings
            .iter()
            .filter(|h| buckets.contains(&h.state.as_str()))
            .collect();
        let closed: Vec<&&IssueHeading> = headings
            .iter()
            .filter(|h| h.state == "DONE" || h.state == "CANCELLED")
            .collect();
        if active.is_empty() && closed.is_empty() {
            continue;
        }
        writeln!(out, "## {project}")?;
        writeln!(out)?;
        for state in buckets {
            let in_state: Vec<&&IssueHeading> = active
                .iter()
                .copied()
                .filter(|h| h.state == state)
                .collect();
            if in_state.is_empty() {
                continue;
            }
            writeln!(out, "### {state}")?;
            writeln!(out)?;
            for h in in_state {
                let deadline = h
                    .deadline()
                    .map(|d| format!(" :: deadline {d}"))
                    .unwrap_or_default();
                let blockers = h.blocked_by();
                let blocked_by = if blockers.is_empty() {
                    String::new()
                } else {
                    format!(" :: blocked by {}", blockers.join(", "))
                };
                writeln!(
                    out,
                    "- **{}** [#{}] {}{}{}",
                    h.id, h.priority, h.title, deadline, blocked_by
                )?;
            }
            writeln!(out)?;
        }
        if !closed.is_empty() {
            writeln!(out, "### Closed ({} items)", closed.len())?;
            writeln!(out)?;
            for h in closed.iter().take(10) {
                writeln!(
                    out,
                    "- {} [#{}] {} ({})",
                    h.id, h.priority, h.title, h.state
                )?;
            }
            if closed.len() > 10 {
                writeln!(out, "- ... and {} more", closed.len() - 10)?;
            }
            writeln!(out)?;
        }
    }
    Ok(out)
}

/// Outcome of [`check`]: the findings, and how many were errors.
#[derive(Debug, Clone)]
pub struct CheckReport {
    pub text: String,
    pub errors: usize,
    pub warnings: usize,
}

/// Validate the corpus: every parent and blocker id resolves, dates parse, open
/// issues carry a creation date, and ids are unique across projects.
pub fn check(layout: &Layout) -> Result<CheckReport> {
    let all = load_all(layout)?;
    let org_ids = collect_org_ids(layout)?;
    let mut out = String::new();

    let mut by_id: HashMap<String, (String, &IssueHeading)> = HashMap::new();
    for (project, h) in &all {
        if let Some(prev) = by_id.insert(h.id.clone(), (project.clone(), h)) {
            writeln!(
                out,
                "[dup]  duplicate id: {} appears in {} and {}",
                h.id, prev.0, project
            )?;
        }
    }

    let mut errors = 0usize;
    let mut warnings = 0usize;

    for (project, h) in &all {
        if let Some(parent) = h.parent() {
            if !org_ids.contains(parent) {
                writeln!(
                    out,
                    "[err]  {} (in {}) :PARENT: {} -> not found",
                    h.id, project, parent
                )?;
                errors += 1;
            }
        }
        for blk in h.blocked_by() {
            if !by_id.contains_key(&blk) {
                writeln!(
                    out,
                    "[err]  {} (in {}) :BLOCKED_BY: {} -> not found",
                    h.id, project, blk
                )?;
                errors += 1;
            }
        }
        if let Some(d) = h.deadline() {
            if parse_org_date(d).is_none() {
                writeln!(
                    out,
                    "[err]  {} (in {}) :DEADLINE: {} -> unparseable",
                    h.id, project, d
                )?;
                errors += 1;
            }
        }
        if let Some(s) = h.scheduled() {
            if parse_org_date(s).is_none() {
                writeln!(
                    out,
                    "[err]  {} (in {}) :SCHEDULED: {} -> unparseable",
                    h.id, project, s
                )?;
                errors += 1;
            }
        }
        if matches!(h.state.as_str(), "TODO" | "STARTED") && !h.properties.contains_key("CREATED") {
            writeln!(
                out,
                "[warn] {} (in {}) state={} but :CREATED: is missing",
                h.id, project, h.state
            )?;
            warnings += 1;
        }
    }

    writeln!(out)?;
    writeln!(
        out,
        "checked {} issue(s) across {} project(s): {} error(s), {} warning(s)",
        all.len(),
        list_projects(layout)?.len(),
        errors,
        warnings
    )?;
    Ok(CheckReport {
        text: out,
        errors,
        warnings,
    })
}

/// Every issue referring to `target_id` through a blocker edge, a parent link,
/// a discovered-from link, or a body mention. The relation is named on the row.
pub fn backlinks(layout: &Layout, target_id: &str) -> Result<String> {
    let all = load_all(layout)?;
    let mut out = String::new();
    for (project, h) in &all {
        if h.id == target_id {
            continue;
        }
        let mut hit = false;
        if h.blocked_by().iter().any(|b| b == target_id) {
            let _ = writeln!(out, "{:<22} (blocked-by) ({})", h.id, project);
            hit = true;
        }
        if h.parent() == Some(target_id) {
            let _ = writeln!(out, "{:<22} (parent) ({})", h.id, project);
            hit = true;
        }
        if h.properties.get("DISCOVERED_FROM").map(|s| s.as_str()) == Some(target_id) {
            let _ = writeln!(out, "{:<22} (discovered-from) ({})", h.id, project);
            hit = true;
        }
        if !hit && h.body.contains(target_id) {
            let _ = writeln!(out, "{:<22} (body mention) ({})", h.id, project);
        }
    }
    Ok(out)
}
