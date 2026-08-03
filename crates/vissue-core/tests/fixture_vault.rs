//! End-to-end checks against the synthetic fixture tracker in `tests/fixture_vault`.

use std::fs;
use std::path::{Path, PathBuf};

use vissue_core::config::{Layout, DEFAULT_PREFIX};
use vissue_core::mirror::{self, Format};
use vissue_core::report;
use vissue_core::store::{self, IssueDoc};

fn fixture_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixture_vault")
}

fn fixture_layout() -> Layout {
    Layout::new(fixture_root(), DEFAULT_PREFIX)
}

fn copy_tree(src: &Path, dest: &Path) {
    fs::create_dir_all(dest).unwrap();
    for entry in fs::read_dir(src).unwrap() {
        let entry = entry.unwrap();
        let target = dest.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), &target).unwrap();
        }
    }
}

/// Copy the fixture into a temporary directory so a test may write to it.
fn writable_copy() -> (tempfile::TempDir, Layout) {
    let dir = tempfile::tempdir().unwrap();
    copy_tree(
        &fixture_root().join(DEFAULT_PREFIX),
        &dir.path().join(DEFAULT_PREFIX),
    );
    let layout = Layout::new(dir.path().to_path_buf(), DEFAULT_PREFIX);
    (dir, layout)
}

#[test]
fn the_fixture_holds_two_projects_and_six_issues() {
    let layout = fixture_layout();
    assert_eq!(
        store::list_projects(&layout).unwrap(),
        vec!["atlas", "beacon"]
    );
    assert_eq!(store::load_all(&layout).unwrap().len(), 6);
}

#[test]
fn parsing_and_rewriting_reproduces_the_file_byte_for_byte() {
    let (_dir, layout) = writable_copy();
    for project in ["atlas", "beacon"] {
        let path = layout.project_issues_path(project);
        let before = fs::read_to_string(&path).unwrap();
        IssueDoc::parse_file(project, &path)
            .unwrap()
            .write()
            .unwrap();
        let after = fs::read_to_string(&path).unwrap();
        assert_eq!(before, after, "{project} changed on a parse and rewrite");
    }
}

#[test]
fn a_clock_line_survives_the_rewrite() {
    let (_dir, layout) = writable_copy();
    let path = layout.project_issues_path("atlas");
    IssueDoc::parse_file("atlas", &path)
        .unwrap()
        .write()
        .unwrap();
    let text = fs::read_to_string(&path).unwrap();
    assert!(
        text.contains("CLOCK: [2026-01-14 Wed 09:12]--[2026-01-14 Wed 10:42] =>  1:30"),
        "{text}"
    );
}

#[test]
fn export_rows_carry_the_documented_schema() {
    let layout = fixture_layout();
    let jsonl = report::export(&layout, Some("atlas")).unwrap();
    let rows: Vec<serde_json::Value> = jsonl
        .lines()
        .map(|l| serde_json::from_str(l).expect("each line is one JSON object"))
        .collect();
    assert_eq!(rows.len(), 4);

    let parser = rows
        .iter()
        .find(|r| r["id"] == "atlas-1a2b")
        .expect("the parser issue is exported");
    for key in [
        "id",
        "project",
        "title",
        "state",
        "priority",
        "properties",
        "logbook",
        "body",
        "line_start",
        "line_end",
    ] {
        assert!(!parser[key].is_null(), "missing {key} in {parser}");
    }
    assert_eq!(parser["project"], "atlas");
    assert_eq!(parser["state"], "STARTED");
    assert_eq!(parser["priority"], "A");
    assert_eq!(parser["properties"]["TAGS"], "parser,core");
    assert_eq!(parser["logbook"][0]["to"], "STARTED");
    assert_eq!(parser["logbook"][0]["from"], "TODO");
    assert!(parser["body"]
        .as_str()
        .unwrap()
        .starts_with("Scope: read the header block"));
}

#[test]
fn export_covers_every_project_when_unfiltered() {
    let layout = fixture_layout();
    assert_eq!(report::export(&layout, None).unwrap().lines().count(), 6);
}

#[test]
fn ready_hides_blocked_and_closed_work() {
    let layout = fixture_layout();
    let ready = report::ready(&layout, None).unwrap();
    assert!(ready.contains("atlas-1a2b"), "{ready}");
    assert!(ready.contains("atlas-2c3d"), "{ready}");
    assert!(ready.contains("beacon-5j6k"), "{ready}");
    assert!(
        !ready.contains("atlas-3e4f"),
        "blocked issue listed: {ready}"
    );
    assert!(
        !ready.contains("atlas-4g5h"),
        "closed issue listed: {ready}"
    );
    assert_eq!(report::count(&layout, None, None, true).unwrap(), "3\n");
}

#[test]
fn counts_respect_project_and_state_filters() {
    let layout = fixture_layout();
    assert_eq!(report::count(&layout, None, None, false).unwrap(), "6\n");
    assert_eq!(
        report::count(&layout, Some("atlas"), None, false).unwrap(),
        "4\n"
    );
    assert_eq!(
        report::count(&layout, None, Some("TODO"), false).unwrap(),
        "2\n"
    );
}

#[test]
fn list_sorts_by_priority_then_state_then_id() {
    let layout = fixture_layout();
    let listed = report::list(&layout, None, None, false).unwrap();
    let ids: Vec<&str> = listed
        .lines()
        .map(|l| l.split_whitespace().next().unwrap())
        .collect();
    assert_eq!(
        ids,
        vec![
            "atlas-3e4f",  // [#A] BLOCKED
            "atlas-1a2b",  // [#A] STARTED
            "atlas-2c3d",  // [#B] TODO
            "beacon-5j6k", // [#B] TODO
            "beacon-6m7n", // [#C] CANCELLED
            "atlas-4g5h",  // [#C] DONE
        ]
    );
}

#[test]
fn the_tree_shows_children_and_blockers() {
    let layout = fixture_layout();
    let ascii = report::tree(&layout, "atlas-1a2b", "ascii").unwrap();
    assert!(ascii.starts_with("atlas-1a2b STARTED"), "{ascii}");
    assert!(ascii.contains("  atlas-2c3d TODO"), "{ascii}");
    assert!(
        !ascii.contains("atlas-4g5h"),
        "unrelated issue in tree: {ascii}"
    );

    let dot = report::tree(&layout, "atlas-1a2b", "dot").unwrap();
    assert!(dot.starts_with("digraph vissue_tree {"), "{dot}");
    assert!(
        dot.contains("\"atlas-1a2b\" -> \"atlas-2c3d\" [color=\"#00897B\"];"),
        "{dot}"
    );

    assert!(report::tree(&layout, "atlas-1a2b", "svg").is_err());
    assert!(report::tree(&layout, "atlas-nope", "ascii").is_err());
}

#[test]
fn backlinks_name_the_relation() {
    let layout = fixture_layout();
    let text = report::backlinks(&layout, "atlas-1a2b").unwrap();
    assert!(text.contains("atlas-3e4f"), "{text}");
    assert!(text.contains("(blocked-by)"), "{text}");
    assert!(text.contains("atlas-2c3d"), "{text}");
    assert!(text.contains("(parent)"), "{text}");
}

#[test]
fn check_passes_and_counts_the_corpus() {
    let layout = fixture_layout();
    let out = report::check(&layout).unwrap();
    assert_eq!(out.errors, 0, "{}", out.text);
    assert_eq!(out.warnings, 0, "{}", out.text);
    assert!(
        out.text
            .contains("checked 6 issue(s) across 2 project(s): 0 error(s), 0 warning(s)"),
        "{}",
        out.text
    );
}

#[test]
fn check_reports_a_broken_blocker_edge() {
    let (_dir, layout) = writable_copy();
    let path = layout.project_issues_path("atlas");
    let mut doc = IssueDoc::parse_file("atlas", &path).unwrap();
    doc.headings
        .iter_mut()
        .find(|h| h.id == "atlas-3e4f")
        .unwrap()
        .properties
        .insert("BLOCKED_BY".into(), "atlas-gone".into());
    doc.write().unwrap();

    let out = report::check(&layout).unwrap();
    assert_eq!(out.errors, 1, "{}", out.text);
    assert!(
        out.text
            .contains("[err]  atlas-3e4f (in atlas) :BLOCKED_BY: atlas-gone -> not found"),
        "{}",
        out.text
    );
}

#[test]
fn cycles_are_reported_when_the_blocker_graph_closes() {
    let layout = fixture_layout();
    assert_eq!(report::cycles(&layout).unwrap(), "no cycles\n");

    let (_dir, layout) = writable_copy();
    let path = layout.project_issues_path("atlas");
    let mut doc = IssueDoc::parse_file("atlas", &path).unwrap();
    doc.headings
        .iter_mut()
        .find(|h| h.id == "atlas-1a2b")
        .unwrap()
        .properties
        .insert("BLOCKED_BY".into(), "atlas-3e4f".into());
    doc.write().unwrap();
    let text = report::cycles(&layout).unwrap();
    assert!(text.contains("atlas-1a2b -> atlas-3e4f"), "{text}");
}

#[test]
fn stale_reports_open_issues_with_old_creation_dates() {
    let layout = fixture_layout();
    let text = report::stale(&layout, 30, None).unwrap();
    // Every open fixture issue was created well over 30 days ago.
    assert!(text.contains("atlas-1a2b"), "{text}");
    assert!(text.contains("beacon-5j6k"), "{text}");
    assert!(
        !text.contains("atlas-4g5h"),
        "closed issue reported: {text}"
    );
    assert!(report::stale(&layout, 30, Some("beacon"))
        .unwrap()
        .lines()
        .all(|l| l.starts_with("beacon-")));
}

#[test]
fn the_roadmap_groups_by_project_and_state() {
    let layout = fixture_layout();
    let md = report::roadmap(&layout, None).unwrap();
    assert!(md.starts_with("# Roadmap\n"), "{md}");
    assert!(md.contains("## atlas"), "{md}");
    assert!(md.contains("### STARTED"), "{md}");
    assert!(
        md.contains("- **atlas-3e4f** [#A] Publish the release notes :: deadline <2026-03-01 Sun> :: blocked by atlas-1a2b"),
        "{md}"
    );
    assert!(md.contains("### Closed (1 items)"), "{md}");
}

#[test]
fn the_mirror_projects_selected_projects_only() {
    let layout = fixture_layout();
    let org = mirror::render(&layout, &["atlas".to_string()], Format::Org, None).unwrap();
    assert!(
        org.contains("# MIRROR: generated by `vissue mirror`"),
        "{org}"
    );
    assert!(org.contains("# Projects: atlas"), "{org}");
    assert!(org.contains("* atlas"), "{org}");
    assert!(!org.contains("* beacon"), "{org}");
    assert!(
        org.contains("** BLOCKED [#A] Publish the release notes"),
        "{org}"
    );
    assert!(org.contains(":ID:         atlas-3e4f"), "{org}");
    assert!(org.contains(":BLOCKED_BY: atlas-1a2b"), "{org}");
    assert!(
        org.contains("Waits on the parser landing, because the notes quote its error text."),
        "{org}"
    );
}

#[test]
fn the_mirror_covers_both_projects_by_default() {
    let layout = fixture_layout();
    let org = mirror::render(&layout, &[], Format::Org, None).unwrap();
    assert!(org.contains("# Projects: atlas, beacon"), "{org}");
    assert!(org.contains("* atlas"), "{org}");
    assert!(org.contains("* beacon"), "{org}");
    for id in [
        "atlas-1a2b",
        "atlas-2c3d",
        "atlas-3e4f",
        "atlas-4g5h",
        "beacon-5j6k",
        "beacon-6m7n",
    ] {
        assert!(org.contains(id), "missing {id} in the mirror");
    }
}

#[test]
fn the_mirror_is_deterministic() {
    let layout = fixture_layout();
    let once = mirror::render(&layout, &[], Format::Org, None).unwrap();
    let twice = mirror::render(&layout, &[], Format::Org, None).unwrap();
    assert_eq!(once, twice);
}

#[test]
fn a_search_finds_body_and_property_text() {
    let layout = fixture_layout();
    assert!(report::search(&layout, "backoff table", 10)
        .unwrap()
        .contains("beacon-5j6k"));
    assert!(report::search(&layout, "PARSER,CORE", 10)
        .unwrap()
        .contains("atlas-1a2b"));
    assert_eq!(
        report::search(&layout, "nothing matches this", 10).unwrap(),
        ""
    );
}

#[test]
fn children_lists_issues_under_a_parent() {
    let layout = fixture_layout();
    let text = report::children(&layout, "atlas-1a2b").unwrap();
    assert!(text.contains("atlas-2c3d"), "{text}");
    assert_eq!(text.lines().count(), 1, "{text}");
    // A design document may also be a parent.
    assert!(report::children(&layout, "beacon-design-0001")
        .unwrap()
        .contains("beacon-5j6k"));
}

#[test]
fn show_reports_the_file_range_without_the_body() {
    let layout = fixture_layout();
    let text = report::show(&layout, "atlas-2c3d").unwrap();
    assert!(text.contains("ID:       atlas-2c3d"), "{text}");
    assert!(text.contains("Project:  atlas"), "{text}");
    assert!(text.contains("State:    TODO"), "{text}");
    assert!(text.contains("Priority: [#B]"), "{text}");
    assert!(text.contains("atlas/issues.org:"), "{text}");
    assert!(
        !text.contains("Scope: one row per parsed record"),
        "show must not print the body: {text}"
    );
}

#[test]
fn a_configured_prefix_finds_no_projects_where_there_are_none() {
    let layout = Layout::new(fixture_root(), "Elsewhere");
    assert!(store::list_projects(&layout).unwrap().is_empty());
    assert_eq!(report::count(&layout, None, None, false).unwrap(), "0\n");
}
