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

/// Every read the schema names answers too, not only every write.
///
/// Fourteen read verbs had no method, so a socket client shelled out for `check`,
/// `graph`, `wait` and the rest. That cost a subprocess rather than correctness,
/// which is why it outlived the write gap, but `wait` is the case that made it
/// worth closing: a verb whose whole job is to block until something changes is
/// exactly what a connection is good at and a subprocess is bad at.
#[test]
fn the_reads_the_schema_names_answer_too() {
    let h = Harness::new();
    let mut client = h.connect();

    let mut missing = Vec::new();
    for op in vissue_core::surface::operations() {
        if op.socket.is_empty() || op.mutates {
            continue;
        }
        if let Err(Error::Rpc(err)) = client.request(&op.socket, json!({}))
            && err.code == -32601
        {
            missing.push(op.socket);
        }
    }
    assert!(
        missing.is_empty(),
        "the schema names these reads and the socket does not answer them: {missing:?}"
    );
}

/// And the ones with something to say, say it.
///
/// Reachability is not usefulness: a method that answers `method not found` fails
/// the check above, and one that answers an empty body passes it while being no use
/// to the client that called it.
#[test]
fn the_new_reads_return_something() {
    let h = Harness::new();
    let mut client = h.connect();

    let checked = client.request("issue/check", json!({})).unwrap();
    assert!(checked.get("report").is_some(), "{checked}");
    assert!(
        checked.get("errors").is_some(),
        "check hides its error count"
    );

    let counted = client.request("issue/count", json!({})).unwrap();
    assert!(!counted["report"].as_str().unwrap_or_default().is_empty());

    let digest = client.request("issue/digest", json!({})).unwrap();
    assert!(
        digest["combined"].as_str().is_some_and(|c| !c.is_empty()),
        "the digest has no combined hash: {digest}"
    );

    for method in [
        "issue/export",
        "issue/graph",
        "issue/roadmap",
        "issue/cycles",
    ] {
        let got = client.request(method, json!({})).unwrap();
        assert!(
            got["report"].as_str().is_some(),
            "{method} returned no report: {got}"
        );
    }

    let stale = client.request("issue/stale", json!({"days": 7})).unwrap();
    assert!(stale["report"].as_str().is_some(), "{stale}");

    // mirror needs a file to judge, and answers about that file rather than the
    // corpus: freshness, not a digest.
    let mirror_path = h.root.join("mirror.md");
    std::fs::write(&mirror_path, "no sync stamp here\n").unwrap();
    let mirrored = client
        .request(
            "issue/mirror_check",
            json!({"path": mirror_path.to_str().unwrap()}),
        )
        .unwrap();
    assert_eq!(
        mirrored["fresh"].as_bool(),
        Some(false),
        "a file with no stamp is not fresh: {mirrored}"
    );
    assert!(
        mirrored["report"]
            .as_str()
            .unwrap_or_default()
            .contains("stale"),
        "{mirrored}"
    );

    let pinged = client.request("events/ping", json!({})).unwrap();
    assert!(pinged["report"].as_str().is_some(), "{pinged}");

    // Waiting on a generation already passed returns at once rather than blocking.
    let waited = client
        .request("events/wait", json!({"last": 0, "timeout_ms": 2000}))
        .unwrap();
    assert!(
        waited["generation"].as_u64().is_some(),
        "wait returned no generation: {waited}"
    );
}

/// The socket carries every method the schema names.
///
/// This is the guard that the five-verb gap could not have survived. `append` had
/// no socket method for as long as the socket existed, and nothing said so: the
/// docs tests checked that the reference listed the methods that existed, which is
/// a different question from whether the methods that should exist do.
///
/// Reads the set out of `schema/vissue.capnp` through the encoded constant, so the
/// schema is the thing being satisfied rather than a list maintained beside it.
#[test]
fn the_socket_answers_every_method_the_schema_names() {
    let h = Harness::new();
    let mut client = h.connect();

    let mut missing = Vec::new();
    for method in vissue_core::surface::mutating_socket_methods() {
        // Called with no params: a method that exists rejects them as invalid
        // params, and one that does not exist answers method-not-found. Only the
        // second is a gap, so the code is what decides and not success.
        if let Err(Error::Rpc(err)) = client.request(&method, json!({}))
            && err.code == -32601
        {
            missing.push(method);
        }
    }
    assert!(
        missing.is_empty(),
        "the schema names these methods and the socket does not answer them: {missing:?}"
    );
}

/// A value of the type the schema names, and one the type must reject.
///
/// `None` for a type with no obvious JSON shape, which the check then skips rather
/// than guessing at.
fn sample_and_violation(socket_type: &str) -> Option<(Value, Value)> {
    let inner = socket_type
        .strip_prefix("Option<")
        .and_then(|t| t.strip_suffix('>'))
        .unwrap_or(socket_type)
        .trim();
    // An array is not a string, a number, a bool or a char, and a number is not an
    // array, so each type gets a value serde has to refuse.
    Some(match inner {
        "String" => (json!("x"), json!([])),
        "char" => (json!("A"), json!([])),
        "bool" => (json!(true), json!([])),
        "u8" | "u16" | "u32" | "u64" | "usize" | "i32" | "i64" => (json!(1), json!([])),
        "f32" | "f64" => (json!(1.5), json!([])),
        _ if inner.starts_with("Vec<") => (json!([]), json!(1)),
        _ => return None,
    })
}

/// Each method takes the parameters the schema names for it.
///
/// The verb check says `issue/append` answers. It does not say the method takes
/// `text` rather than `body`, and the third spelling of a field is where this drifts
/// next: the issue being acted on is `id` here and `issue_id` as a tool argument,
/// which is a real difference a caller has to know and which nothing recorded.
///
/// Asked of a running owner rather than read out of the source. A parameter is
/// present because the method refuses a value of the wrong type for it, and required
/// because the method refuses the request without it -- both of which are what a
/// client experiences, where a Rust type spelled in a source line is a claim about
/// it. Two source scans stood here before, one to find the handler a match arm names
/// and one to find the type it decodes, and both were rewritten after reporting
/// working methods as unimplemented.
///
/// Nothing here executes: every request is built to fail at decode, and the
/// `-32602` that comes back is the parameter check refusing it. That is why a
/// mutating method can be asked this without writing anything.
#[test]
fn each_method_takes_the_parameters_the_schema_names() {
    let h = Harness::new();
    let mut client = h.connect();

    let invalid_params = |client: &mut Client, method: &str, params: Value| -> bool {
        matches!(client.request(method, params), Err(Error::Rpc(e)) if e.code == -32602)
    };

    // A synthetic value has to be one the method accepts, and a few parameters have
    // small domains: a priority is A, B or C, and "x" is refused after decoding
    // rather than during it. Held here as data because it is about these parameters
    // and not about their types.
    // An empty method means the value suits the parameter wherever it appears.
    let constrained: &[(&str, &str, Value)] = &[
        ("", "priority", json!("A")),
        ("", "state", json!("TODO")),
        ("", "if_state", json!("TODO")),
        ("issue/tree", "format", json!("nodes")),
        ("issue/related", "format", json!("ids")),
    ];

    let mut wrong = Vec::new();
    for op in vissue_core::surface::operations() {
        // Same rule as the tool check: only verbs whose parameters the schema names.
        // A read method takes a shared param type or none.
        if op.socket.is_empty() || !op.fields.iter().any(|f| !f.socket.is_empty()) {
            continue;
        }
        let typed: Vec<(&str, Value, Value, bool)> = op
            .fields
            .iter()
            .filter(|f| !f.socket.is_empty())
            .filter_map(|f| {
                let (mut good, bad) = sample_and_violation(&f.socket_type)?;
                if let Some((_, _, value)) = constrained
                    .iter()
                    .find(|(m, n, _)| *n == f.socket && (m.is_empty() || *m == op.socket))
                {
                    good = value.clone();
                }
                let optional = f.socket_type.starts_with("Option<") || f.omittable;
                Some((f.socket.as_str(), good, bad, optional))
            })
            .collect();

        let base: serde_json::Map<String, Value> = typed
            .iter()
            .map(|(name, good, _, _)| ((*name).to_string(), good.clone()))
            .collect();

        // Without this the method's whole row is vacuous. If the schema does not name
        // every parameter the method requires, a request built from the schema fails
        // to decode whatever else is done to it, so every wrong-type check below
        // would pass by refusing a request that was already being refused. The
        // decode error names the parameter it wanted.
        match client.request(&op.socket, Value::Object(base.clone())) {
            Err(Error::Rpc(e)) if e.code == -32602 => {
                wrong.push(format!(
                    "{}: a request naming every parameter the schema records is still \
                     refused, so the schema does not name them all: {}",
                    op.socket, e.message
                ));
                continue;
            }
            _ => {}
        }

        for (name, _, bad, optional) in &typed {
            // A value of the wrong type has to be refused, which it can only be by a
            // method that reads this parameter and holds it to this type.
            let mut violating = base.clone();
            violating.insert((*name).to_string(), bad.clone());
            if !invalid_params(&mut client, &op.socket, Value::Object(violating)) {
                wrong.push(format!(
                    "{} accepts {} of the wrong type, so it does not take it as {}",
                    op.socket,
                    name,
                    op.fields
                        .iter()
                        .find(|f| f.socket == *name)
                        .map_or("", |f| f.socket_type.as_str())
                ));
            }

            // And optionality, which is the part of the contract a caller plans
            // around: a parameter that becomes required breaks everyone who omitted
            // it.
            let mut without = base.clone();
            without.remove(*name);
            let refused = invalid_params(&mut client, &op.socket, Value::Object(without));
            if *optional && refused {
                wrong.push(format!(
                    "{} requires {name}, which the schema names optional",
                    op.socket
                ));
            }
            if !*optional && !refused {
                wrong.push(format!(
                    "{} answers without {name}, which the schema names required",
                    op.socket
                ));
            }
        }
    }
    assert!(
        wrong.is_empty(),
        "the schema names parameters these methods do not take: {wrong:?}"
    );
}

/// And every method the server dispatches is in the schema.
///
/// The mirror of the check above, and the direction that was missing everywhere: a
/// method the schema omits was invisible to every test, exactly as a verb the schema
/// omitted used to be.
///
/// The one check here that reads source, because it is the one question a handshake
/// cannot answer: a client can ask whether a named method answers, and no client can
/// ask for the names it has not been told. The match arms are that list.
#[test]
fn every_method_the_server_dispatches_is_in_the_schema() {
    let dispatch =
        std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("src/unix/dispatch.rs"))
            .expect("dispatch.rs");

    // The match arms name every method the owner answers.
    let mut dispatched: Vec<String> = dispatch
        .lines()
        .filter_map(|l| l.trim().strip_prefix('"'))
        .filter_map(|rest| rest.split('"').next())
        .filter(|m| m.contains('/'))
        .map(str::to_string)
        .collect();
    dispatched.sort_unstable();
    dispatched.dedup();
    assert!(
        dispatched.len() > 25,
        "no methods parsed out of dispatch.rs: {dispatched:?}"
    );

    let known = vissue_core::surface::socket_methods();
    // `initialize` and the lifecycle calls are protocol rather than operations, and
    // the schema is about operations.
    const PROTOCOL: &[&str] = &["issue/open", "issue/get"];
    let unknown: Vec<&String> = dispatched
        .iter()
        .filter(|m| !known.iter().any(|k| k == *m))
        .filter(|m| !PROTOCOL.contains(&m.as_str()))
        .collect();
    assert!(
        unknown.is_empty(),
        "these methods are dispatched and no schema row mentions them: {unknown:?}"
    );
}

/// Every mutating reply has the same shape.
///
/// `mut_result` returns `ok`, `report` and the affected issue, and that was a
/// convention rather than a contract: a method returning a bare string, or omitting
/// `ok`, would have passed every other check here. A client switching on `ok` and
/// printing `report` is the normal way to use this socket, so the shape is worth
/// asserting once across all of them rather than trusting eleven call sites to agree.
///
/// Driven from the schema, so a mutating method added later is covered without
/// anyone remembering to add it here.
#[test]
fn every_mutating_reply_has_the_same_shape() {
    let h = Harness::new();
    let mut client = h.connect();

    // One issue to act on, and a second so refile and reject have somewhere to go.
    let made = client
        .request(
            "issue/create",
            json!({"project": "atlas", "title": "shape subject"}),
        )
        .unwrap();
    let id = made["issue"]["id"].as_str().unwrap().to_string();

    // Per verb, the smallest params that reach the operation rather than a
    // validation error. `fold` and `normalize` act on files rather than an issue.
    let inbox = h.root.join("shape-inbox.org");
    std::fs::write(&inbox, "* TODO folded for the shape test\n").unwrap();
    let calls: Vec<(&str, serde_json::Value)> = vec![
        ("issue/update", json!({"id": id, "priority": "B"})),
        ("issue/claim", json!({"id": id, "agent": "shape-agent"})),
        ("issue/note", json!({"id": id, "text": "a note"})),
        ("issue/append", json!({"id": id, "text": "a report"})),
        (
            "issue/vote",
            json!({"id": id, "choice": "ship", "agent": "shape-agent"}),
        ),
        ("issue/normalize", json!({"dry_run": true})),
        (
            "issue/fold",
            json!({"file": inbox.to_str().unwrap(), "project": "atlas"}),
        ),
    ];

    for (method, params) in calls {
        let got = client
            .request(method, params)
            .unwrap_or_else(|e| panic!("{method} failed: {e}"));
        assert!(
            got.get("ok").and_then(serde_json::Value::as_bool).is_some(),
            "{method} has no boolean ok: {got}"
        );
        assert!(
            got.get("report")
                .and_then(serde_json::Value::as_str)
                .is_some(),
            "{method} has no report string: {got}"
        );
    }

    // And the ones that name an issue return it, so a client need not re-read.
    let updated = client
        .request("issue/update", json!({"id": id, "priority": "C"}))
        .unwrap();
    assert_eq!(
        updated["issue"]["id"].as_str(),
        Some(id.as_str()),
        "update did not return the issue it changed: {updated}"
    );
}

/// What the server advertises is what the schema says it has.
///
/// `initialize` returns a capability list, and that is the fourth place the method
/// set is written down after the dispatch table, the schema and the reference. It is
/// also the one a client reads to decide what it may call, and it had fallen nineteen
/// methods behind while the other three agreed with each other. A client inspecting
/// capabilities would have concluded that `append`, `vote`, `fold` and every read
/// added beside them did not exist.
///
/// Both directions, because either is a lie: advertising a method that is not
/// dispatched sends a client at something that will answer method-not-found, and
/// dispatching one that is not advertised hides it from anyone who asks first.
#[test]
fn capabilities_match_the_schema() {
    use std::collections::BTreeSet;

    let advertised: BTreeSet<&str> = vissue_serve::LIVE_CAPABILITIES.iter().copied().collect();
    let from_schema: BTreeSet<String> =
        vissue_core::surface::socket_methods().into_iter().collect();

    // `issue/get` and `issue/open` are protocol rather than operations: one is the
    // detail fetch behind `issue/show`, the other is the shared selection a client
    // uses to point its peers at an issue. The schema is about operations.
    const PROTOCOL: &[&str] = &["issue/get", "issue/open"];

    let unadvertised: Vec<&String> = from_schema
        .iter()
        .filter(|m| !advertised.contains(m.as_str()))
        .collect();
    assert!(
        unadvertised.is_empty(),
        "the schema has these methods and initialize does not advertise them: {unadvertised:?}"
    );

    let unknown: Vec<&&str> = advertised
        .iter()
        .filter(|m| !from_schema.contains(**m) && !PROTOCOL.contains(m))
        .collect();
    assert!(
        unknown.is_empty(),
        "initialize advertises these and no schema row mentions them: {unknown:?}"
    );
}
