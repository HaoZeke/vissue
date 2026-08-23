//! Protocol tests against an in-process owner over a tempfile fixture copy.

#![cfg(unix)]
#![allow(missing_docs)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use serde_json::{Value, json};
use vissue_control::client::Client;
use vissue_control::{Error, NOTIFY_ISSUE_SELECTED, NOTIFY_VAULT_CHANGED, Notification};
use vissue_core::agent;
use vissue_core::config::Layout;
use vissue_core::ops;
use vissue_serve::{OwnerHandle, ServeConfig};

fn fixture_src() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixture_vault")
}

fn copy_dir(src: &Path, dest: &Path) {
    fs::create_dir_all(dest).unwrap();
    for entry in fs::read_dir(src).unwrap() {
        let entry = entry.unwrap();
        let to = dest.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &to);
        } else {
            fs::copy(entry.path(), to).unwrap();
        }
    }
}

struct Harness {
    _tmp: tempfile::TempDir,
    root: PathBuf,
    owner: OwnerHandle,
}

impl Harness {
    fn new() -> Self {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("vault");
        copy_dir(&fixture_src(), &root);
        let socket = tmp.path().join("run/control.sock");
        let owner = OwnerHandle::spawn(ServeConfig {
            layout: Layout::new(&root, "Software"),
            socket,
            exe: None,
        })
        .expect("spawn owner");
        Self {
            _tmp: tmp,
            root,
            owner,
        }
    }

    fn layout(&self) -> Layout {
        Layout::new(&self.root, "Software")
    }

    fn connect(&self) -> Client {
        let mut client = Client::connect(&self.owner.socket).unwrap();
        let init = client
            .request(
                "initialize",
                json!({"protocolVersion": 1, "client": "protocol", "agent": "tui"}),
            )
            .unwrap();
        assert_eq!(init["identity"], "tui");
        assert!(init["revision"].as_u64().unwrap() >= 1);
        client
    }
}

fn rpc_err(err: Error) -> vissue_control::JsonRpcError {
    match err {
        Error::Rpc(e) => e,
        other => panic!("{other:?}"),
    }
}

#[test]
fn issue_ready_matches_agent_json() {
    let h = Harness::new();
    let expected = agent::issues_json(&h.layout(), None, None, true).unwrap();
    let mut client = h.connect();
    let got = client.request("issue/ready", json!({})).unwrap();
    assert_eq!(got["issues"], expected);
    assert_eq!(got["unchanged"], false);
}

#[test]
fn issue_get_atlas_2c3d_matches_show_json() {
    let h = Harness::new();
    let expected = agent::show_json(&h.layout(), "atlas-2c3d").unwrap();
    let mut client = h.connect();
    let got = client
        .request("issue/get", json!({"id": "atlas-2c3d"}))
        .unwrap();
    let obj = expected.as_object().unwrap();
    for key in obj.keys() {
        assert_eq!(got[key], expected[key], "{key}");
    }
    assert!(got["revision"].as_u64().is_some());
}

#[test]
fn issue_list_since_revision_unchanged() {
    let h = Harness::new();
    let mut client = h.connect();
    let first = client.request("issue/list", json!({})).unwrap();
    let rev = first["revision"].as_u64().unwrap();
    assert!(!first["issues"].as_array().unwrap().is_empty());
    let again = client
        .request("issue/list", json!({"since_revision": rev}))
        .unwrap();
    assert_eq!(again["unchanged"], true);
    assert_eq!(again["revision"], rev);
    assert!(again["issues"].as_array().unwrap().is_empty());
}

#[test]
fn issue_claim_conflict_and_identity() {
    let h = Harness::new();
    let mut client = h.connect();
    let err = rpc_err(
        client
            .request("issue/claim", json!({"id": "atlas-1a2b"}))
            .unwrap_err(),
    );
    assert_eq!(err.code, -32009);
    assert_eq!(err.data.as_ref().unwrap()["code"], "conflict");
    assert_eq!(err.data.unwrap()["holder"], "fixture-agent");

    let claimed = client
        .request("issue/claim", json!({"id": "atlas-2c3d"}))
        .unwrap();
    assert_eq!(claimed["ok"], true);
    assert_eq!(claimed["issue"]["claimed_by"], "tui");

    let control = tempfile::tempdir().unwrap();
    let control_root = control.path().join("vault");
    copy_dir(&fixture_src(), &control_root);
    let layout = Layout::new(&control_root, "Software");
    ops::claim_as(&layout, "atlas-2c3d", false, "tui").unwrap();
    let via_rpc = fs::read_to_string(h.root.join("Software/atlas/issues.org")).unwrap();
    let via_ops = fs::read_to_string(control_root.join("Software/atlas/issues.org")).unwrap();
    assert!(via_rpc.contains("CLAIMED_BY") && via_rpc.contains("tui"));
    assert_eq!(
        claimed_by_line(&via_rpc),
        claimed_by_line(&via_ops),
        "rpc claim stamp must match ops::claim_as"
    );

    let err = rpc_err(
        client
            .request("issue/claim", json!({"id": "atlas-2c3d", "agent": "other"}))
            .unwrap_err(),
    );
    assert_eq!(err.code, -32009);
}

fn claimed_by_line(text: &str) -> Option<String> {
    text.lines()
        .find(|l| l.contains("CLAIMED_BY"))
        .map(str::trim)
        .map(str::to_string)
}

#[test]
fn events_since_after_claim_sees_issues_write() {
    let h = Harness::new();
    let mut client = h.connect();
    client
        .request("issue/claim", json!({"id": "atlas-2c3d"}))
        .unwrap();
    let ev = client
        .request("events/since", json!({"since": 0, "limit": 50}))
        .unwrap();
    let kinds: Vec<&str> = ev["events"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|e| e["kind"].as_str())
        .collect();
    assert!(
        kinds.contains(&"issues_write"),
        "events/since after claim: {kinds:?}"
    );
}

#[test]
fn second_client_receives_vault_changed_after_note() {
    let h = Harness::new();
    let mut writer = h.connect();
    let mut reader = h.connect();
    writer
        .request(
            "issue/note",
            json!({"id": "atlas-2c3d", "text": "progress from the protocol test"}),
        )
        .unwrap();
    let note = reader
        .wait_notification(Duration::from_secs(3))
        .expect("vault/changed");
    match note {
        Notification::VaultChanged(body) => {
            assert!(body.revision >= 2);
            assert_eq!(note_method(&body), NOTIFY_VAULT_CHANGED);
        }
        other => panic!("expected vault/changed, got {}", other.method()),
    }
}

fn note_method(_body: &vissue_control::rpc::VaultChanged) -> &'static str {
    NOTIFY_VAULT_CHANGED
}

#[test]
fn issue_open_notifies_selected() {
    let h = Harness::new();
    let mut a = h.connect();
    let mut b = h.connect();
    let got = a
        .request("issue/open", json!({"id": "atlas-2c3d"}))
        .unwrap();
    assert_eq!(got["id"], "atlas-2c3d");
    let note = b
        .wait_notification(Duration::from_secs(2))
        .expect("issue/selected");
    match note {
        Notification::IssueSelected(sel) => {
            assert_eq!(sel.id, "atlas-2c3d");
            assert_eq!(sel.project, "atlas");
        }
        other => panic!("expected issue/selected, got {}", other.method()),
    }
    assert_eq!(NOTIFY_ISSUE_SELECTED, "issue/selected");
}

#[test]
fn typed_errors_map_to_control_codes() {
    let h = Harness::new();
    let mut client = h.connect();
    let err = rpc_err(
        client
            .request("issue/get", json!({"id": "missing-zzzz"}))
            .unwrap_err(),
    );
    assert_eq!(err.code, -32004);
    assert_eq!(err.data.unwrap()["code"], "not_found");

    let err = rpc_err(
        client
            .request("issue/claim", json!({"id": "atlas-4g5h"}))
            .unwrap_err(),
    );
    assert_eq!(err.code, -32010);
    assert_eq!(err.data.unwrap()["code"], "invalid_state");

    let err = rpc_err(
        client
            .request(
                "issue/update",
                json!({"id": "atlas-1a2b", "block": "atlas-3e4f"}),
            )
            .unwrap_err(),
    );
    assert_eq!(err.code, -32022);
    assert_eq!(err.data.unwrap()["code"], "cycle");
}

#[test]
fn read_methods_over_the_fixture() {
    let h = Harness::new();
    let mut client = h.connect();
    let projects = client.request("project/list", json!({})).unwrap();
    let names: Vec<&str> = projects["projects"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(Value::as_str)
        .collect();
    assert!(names.contains(&"atlas"));
    assert!(names.contains(&"beacon"));

    let search = client
        .request("issue/search", json!({"query": "manifest", "limit": 5}))
        .unwrap();
    assert!(!search.as_array().unwrap().is_empty());

    let claims = client.request("issue/claims", json!({})).unwrap();
    assert!(
        claims
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["holder"] == "fixture-agent")
    );

    let agenda = client
        .request("issue/agenda", json!({"days": 400}))
        .unwrap();
    assert!(agenda.is_array());

    let excerpt = client
        .request("issue/excerpt", json!({"id": "atlas-2c3d"}))
        .unwrap();
    assert_eq!(excerpt["id"], "atlas-2c3d");
    assert_eq!(excerpt["suppressed"], false);

    let tree = client
        .request("issue/tree", json!({"id": "atlas-1a2b"}))
        .unwrap();
    assert_eq!(tree["id"], "atlas-1a2b");

    let ascii = client
        .request("issue/tree", json!({"id": "atlas-1a2b", "format": "ascii"}))
        .unwrap();
    assert!(ascii["text"].as_str().unwrap().contains("atlas-1a2b"));

    let related = client
        .request("issue/related", json!({"id": "atlas-1a2b"}))
        .unwrap();
    assert!(related.is_array());

    let children = client
        .request("issue/children", json!({"id": "atlas-1a2b"}))
        .unwrap();
    assert!(
        children
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["id"] == "atlas-2c3d")
    );

    let _ = client
        .request("issue/ancestors", json!({"id": "atlas-2c3d"}))
        .unwrap();
    let _ = client
        .request("issue/impact", json!({"id": "atlas-1a2b"}))
        .unwrap();
    let _ = client
        .request("issue/backlinks", json!({"id": "atlas-1a2b"}))
        .unwrap();
    let show = client
        .request("issue/show", json!({"id": "atlas-2c3d"}))
        .unwrap();
    assert_eq!(show["id"], "atlas-2c3d");
    let generation = client.request("events/gen", json!({})).unwrap();
    assert!(generation["revision"].as_u64().unwrap() >= 1);
}

#[test]
fn create_update_refile_roundtrip() {
    let h = Harness::new();
    let mut client = h.connect();
    let created = client
        .request(
            "issue/create",
            json!({"project": "atlas", "title": "Serve protocol extra"}),
        )
        .unwrap();
    assert_eq!(created["ok"], true);
    let id = created["issue"]["id"].as_str().unwrap().to_string();
    let updated = client
        .request("issue/update", json!({"id": id, "priority": "A"}))
        .unwrap();
    assert_eq!(updated["issue"]["priority"], "A");
    let refiled = client
        .request("issue/refile", json!({"id": id, "to": "beacon"}))
        .unwrap();
    assert_eq!(refiled["ok"], true);
    assert_eq!(refiled["issue"]["project"], "beacon");
}

#[test]
fn read_only_protocol_leaves_committed_fixture_clean() {
    let atlas = fs::read_to_string(fixture_src().join("Software/atlas/issues.org")).unwrap();
    assert!(
        atlas.contains("fixture-agent"),
        "source fixture claim stamp must stay fixture-agent"
    );
    assert!(
        !atlas.contains("Serve protocol extra"),
        "source fixture must not receive create/refile writes"
    );
    assert!(
        !atlas.contains("progress from the protocol test"),
        "source fixture must not receive note writes"
    );
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    if !root.join(".git").exists() {
        return;
    }
    let out = Command::new("git")
        .args(["diff", "--exit-code", "--", "tests/fixture_vault"])
        .current_dir(&root)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "fixture vault dirty:\n{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

/// Ballots over the socket name the session's agent, not the server process, so
/// two agents on one server disagree rather than overwrite. This is the surface
/// the feature exists for: agents reach the tracker here and through MCP, so a
/// vote only on the CLI would be a tally nothing can cast.
#[test]
fn votes_over_the_socket_are_per_agent() {
    let h = Harness::new();
    let mut client = h.connect();
    let created = client
        .request(
            "issue/create",
            json!({"project": "atlas", "title": "what to do"}),
        )
        .unwrap();
    let id = created["issue"]["id"].as_str().unwrap().to_string();

    let first = client
        .request(
            "issue/vote",
            json!({"id": id, "choice": "ship", "agent": "agent-a"}),
        )
        .unwrap();
    assert_eq!(first["ok"], true);

    client
        .request(
            "issue/vote",
            json!({"id": id, "choice": "hold", "agent": "agent-b"}),
        )
        .unwrap();

    // A recast replaces only the caster's own ballot.
    let recast = client
        .request(
            "issue/vote",
            json!({"id": id, "choice": "rework", "agent": "agent-a"}),
        )
        .unwrap();
    let report = recast["report"].as_str().unwrap_or_default();
    assert!(report.contains("changed ship to rework"), "{report}");

    let tally = client.request("issue/vote", json!({"id": id})).unwrap();
    let text = tally["report"].as_str().unwrap_or_default();
    assert!(text.contains("2 votes from 2 options"), "{text}");
    assert!(text.contains("agent-b"), "{text}");
    assert!(text.contains("no consensus"), "{text}");
}

/// Every verb that changes a file is reachable over the socket.
///
/// Not for correctness: the advisory lock makes a direct write safe, so a client
/// mixing the two is not corrupting anything. It is so the socket is a complete
/// surface rather than most of one. A client that had to shell out for `append`
/// was writing behind the server's back and its change stream had a hole in it
/// exactly where that write went.
#[test]
fn every_mutating_verb_is_reachable_over_the_socket() {
    let h = Harness::new();
    let mut client = h.connect();

    let created = client
        .request(
            "issue/create",
            json!({"project": "atlas", "title": "socket surface"}),
        )
        .unwrap();
    assert_eq!(created["ok"], true);
    let id = created["issue"]["id"].as_str().unwrap().to_string();

    // append
    let appended = client
        .request(
            "issue/append",
            json!({"id": id, "text": "a report", "agent": "agent-a"}),
        )
        .unwrap();
    assert_eq!(appended["ok"], true, "{appended}");
    assert!(
        appended["report"]
            .as_str()
            .unwrap_or_default()
            .contains("appended"),
        "{appended}"
    );

    // normalize, over every project, as a dry run so it changes nothing
    let normalized = client
        .request("issue/normalize", json!({"dry_run": true}))
        .unwrap();
    assert_eq!(normalized["ok"], true, "{normalized}");

    // reject, creating a successor in the same project
    let rejected = client
        .request(
            "issue/reject",
            json!({"id": id, "project": "atlas", "title": "the better idea",
                   "reason": "superseded by the better idea"}),
        )
        .unwrap();
    assert_eq!(rejected["ok"], true, "{rejected}");

    // resolve, on a fresh issue with a terminal to settle
    let other = client
        .request(
            "issue/create",
            json!({"project": "atlas", "title": "to settle"}),
        )
        .unwrap();
    let other_id = other["issue"]["id"].as_str().unwrap().to_string();
    let resolved = client
        .request("issue/resolve", json!({"id": other_id, "state": "DONE"}))
        .unwrap();
    assert_eq!(resolved["ok"], true, "{resolved}");

    // fold, from an inbox file
    let inbox = h.root.join("inbox.org");
    std::fs::write(&inbox, "* TODO folded in from an inbox\n").unwrap();
    let folded = client
        .request(
            "issue/fold",
            json!({"file": inbox.to_str().unwrap(), "project": "atlas"}),
        )
        .unwrap();
    assert_eq!(folded["ok"], true, "{folded}");

    // Nothing above needed the command line, and the corpus still reads back.
    let listed = client.request("issue/list", json!({})).unwrap();
    assert!(
        listed["issues"].as_array().map(Vec::len).unwrap_or(0) >= 3,
        "{listed}"
    );
}
