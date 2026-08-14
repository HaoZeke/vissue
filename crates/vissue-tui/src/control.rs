//! Socket-backed board. Unix only; clients never bind the control socket.

use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use serde_json::Value;
use vissue_control::client::Client;
use vissue_control::rpc::{
    ClaimParams, Error as RpcError, IdParams, InitializeResult, IssueListParams, IssueListResult,
    MutResult as WireMut, NoteParams, Notification, RelatedParams, Request, SearchParams,
    TreeParams, UpdateParams, CONFLICT, CYCLE, INVALID_STATE, NOT_FOUND,
};
use vissue_control::{InitializeParams, PROTOCOL_VERSION};
use vissue_core::config::Layout;
use vissue_core::error::Error;
use vissue_core::views::{
    AgendaRow, ClaimRow, Excerpt, IssueDetail, ListQuery, RelatedHit, SearchHit, TreeNode,
};

use crate::backend::{BackendKind, BoardBackend, ListPage, MutResult, SinceGate, UpdateReq};

/// JSON-RPC client after a matching `initialize`.
pub struct ControlBackend {
    layout: Layout,
    identity: String,
    client: Mutex<Client>,
    generation: AtomicU64,
    revision: AtomicU64,
    since: SinceGate,
    last_since: Mutex<Option<Option<u64>>>,
}

impl ControlBackend {
    /// Connect, `initialize` with a required agent, and refuse a root/prefix
    /// mismatch so mutations never hit the wrong vault.
    pub fn connect(path: &Path, layout: &Layout, agent: &str) -> Result<Self, ControlAttachError> {
        let mut client = Client::connect(path).map_err(ControlAttachError::Rpc)?;
        let params = InitializeParams {
            protocol_version: PROTOCOL_VERSION,
            client: "vissue-tui".into(),
            agent: agent.to_string(),
        };
        let value = client
            .request_typed(&Request::Initialize(params))
            .map_err(ControlAttachError::Rpc)?;
        let init: InitializeResult = serde_json::from_value(value)
            .map_err(|e| ControlAttachError::Rpc(RpcError::Json(e)))?;
        if !roots_match(layout, &init.root, &init.prefix) {
            return Err(ControlAttachError::Mismatch {
                want_root: layout.root().display().to_string(),
                want_prefix: layout.prefix().to_string(),
                got_root: init.root,
                got_prefix: init.prefix,
            });
        }
        Ok(Self {
            layout: layout.clone(),
            identity: init.identity,
            client: Mutex::new(client),
            generation: AtomicU64::new(init.generation),
            revision: AtomicU64::new(init.revision),
            since: SinceGate::after_attach(),
            last_since: Mutex::new(None),
        })
    }

    fn call(&self, req: &Request) -> Result<Value, Error> {
        let mut client = self.client.lock().expect("control client");
        client.request_typed(req).map_err(map_rpc)
    }

    fn list_params(&self, q: ListQuery) -> IssueListParams {
        let revision = self.revision.load(Ordering::SeqCst);
        let since = self.since.next(revision);
        *self.last_since.lock().expect("since") = Some(since);
        IssueListParams {
            project: q.project,
            state: q.state,
            ready: if q.ready { Some(true) } else { None },
            query: q.query,
            limit: q.limit,
            offset: q.offset,
            since_revision: since,
        }
    }

    fn apply_list(&self, result: IssueListResult) -> ListPage {
        if !result.unchanged {
            self.revision.store(result.revision, Ordering::SeqCst);
            self.generation.store(result.generation, Ordering::SeqCst);
        }
        ListPage {
            issues: result.issues,
            total: result.total,
            matched: result.matched,
            revision: result.revision,
            generation: result.generation,
            unchanged: result.unchanged,
        }
    }

    fn apply_mut(&self, wire: WireMut) -> MutResult {
        self.revision.store(wire.revision, Ordering::SeqCst);
        self.generation.store(wire.generation, Ordering::SeqCst);
        MutResult {
            ok: wire.ok,
            report: wire.report,
            issue: wire.issue,
            revision: wire.revision,
            generation: wire.generation,
        }
    }
}

fn roots_match(layout: &Layout, root: &str, prefix: &str) -> bool {
    let want_root = layout.root().display().to_string();
    (root == want_root || Path::new(root) == layout.root()) && prefix == layout.prefix()
}

/// Why attach refused the live socket.
#[derive(Debug)]
pub enum ControlAttachError {
    Rpc(RpcError),
    Mismatch {
        want_root: String,
        want_prefix: String,
        got_root: String,
        got_prefix: String,
    },
}

impl std::fmt::Display for ControlAttachError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Rpc(err) => write!(f, "{err}"),
            Self::Mismatch {
                want_root,
                want_prefix,
                got_root,
                got_prefix,
            } => write!(
                f,
                "serve root/prefix mismatch: want {want_root} {want_prefix}, got {got_root} {got_prefix}"
            ),
        }
    }
}

impl std::error::Error for ControlAttachError {}

fn map_rpc(err: RpcError) -> Error {
    match err {
        RpcError::Rpc(rpc) => match rpc.code {
            NOT_FOUND => Error::IssueNotFound {
                id: rpc
                    .data
                    .as_ref()
                    .and_then(|d| d.get("id"))
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
            },
            CONFLICT => Error::ClaimConflict {
                id: rpc
                    .data
                    .as_ref()
                    .and_then(|d| d.get("id"))
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
                holder: rpc
                    .data
                    .as_ref()
                    .and_then(|d| d.get("holder"))
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
                claimed_at: None,
            },
            CYCLE => Error::BlockerCycle {
                blocker: rpc
                    .data
                    .as_ref()
                    .and_then(|d| d.get("block"))
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
                issue: rpc
                    .data
                    .as_ref()
                    .and_then(|d| d.get("id"))
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
            },
            INVALID_STATE => Error::InvalidState {
                id: rpc
                    .data
                    .as_ref()
                    .and_then(|d| d.get("id"))
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
                state: rpc
                    .data
                    .as_ref()
                    .and_then(|d| d.get("state"))
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
            },
            _ => Error::Other(anyhow::anyhow!("{}", rpc.message)),
        },
        other => Error::Other(anyhow::anyhow!("{other}")),
    }
}

fn decode<T: serde::de::DeserializeOwned>(value: Value) -> Result<T, Error> {
    serde_json::from_value(value).map_err(|e| Error::Other(e.into()))
}

impl BoardBackend for ControlBackend {
    fn layout(&self) -> &Layout {
        &self.layout
    }

    fn generation(&self) -> u64 {
        self.generation.load(Ordering::SeqCst)
    }

    fn revision(&self) -> u64 {
        self.revision.load(Ordering::SeqCst)
    }

    fn live(&self) -> BackendKind {
        BackendKind::Control
    }

    fn identity(&self) -> &str {
        &self.identity
    }

    fn list(&self, q: ListQuery) -> Result<ListPage, Error> {
        let params = self.list_params(q);
        let value = self.call(&Request::IssueList(params))?;
        Ok(self.apply_list(decode(value)?))
    }

    fn ready(&self, project: Option<&str>) -> Result<ListPage, Error> {
        let params = self.list_params(ListQuery {
            project: project.map(str::to_string),
            ready: true,
            ..ListQuery::default()
        });
        let value = self.call(&Request::IssueReady(params))?;
        Ok(self.apply_list(decode(value)?))
    }

    fn get(&self, id: &str) -> Result<IssueDetail, Error> {
        let value = self.call(&Request::IssueGet(IdParams { id: id.to_string() }))?;
        let row: vissue_control::rpc::IssueGetResult = decode(value)?;
        Ok(row.issue)
    }

    fn excerpt(&self, id: &str) -> Result<Excerpt, Error> {
        let value = self.call(&Request::IssueExcerpt(IdParams { id: id.to_string() }))?;
        decode(value)
    }

    fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchHit>, Error> {
        let value = self.call(&Request::IssueSearch(SearchParams {
            query: query.to_string(),
            limit: Some(limit),
        }))?;
        decode(value)
    }

    fn claims(&self, holder: Option<&str>, project: Option<&str>) -> Result<Vec<ClaimRow>, Error> {
        let value = self.call(&Request::IssueClaims(vissue_control::rpc::ClaimsParams {
            holder: holder.map(str::to_string),
            project: project.map(str::to_string),
        }))?;
        decode(value)
    }

    fn agenda(&self, days: i64, project: Option<&str>) -> Result<Vec<AgendaRow>, Error> {
        let value = self.call(&Request::IssueAgenda(vissue_control::rpc::AgendaParams {
            days: Some(days),
            project: project.map(str::to_string),
        }))?;
        decode(value)
    }

    fn tree(&self, id: &str) -> Result<TreeNode, Error> {
        let value = self.call(&Request::IssueTree(TreeParams {
            id: id.to_string(),
            format: Some("nodes".into()),
        }))?;
        match decode::<vissue_control::rpc::TreeResult>(value)? {
            vissue_control::rpc::TreeResult::Nodes(node) => Ok(node),
            vissue_control::rpc::TreeResult::Text { text } => Err(Error::Other(anyhow::anyhow!(
                "serve returned tree text, not nodes: {text}"
            ))),
        }
    }

    fn related(&self, id: &str, depth: usize, limit: usize) -> Result<Vec<RelatedHit>, Error> {
        let value = self.call(&Request::IssueRelated(RelatedParams {
            id: id.to_string(),
            depth: Some(depth),
            limit: Some(limit),
        }))?;
        decode(value)
    }

    fn projects(&self) -> Result<Vec<String>, Error> {
        let value = self.call(&Request::ProjectList)?;
        let row: vissue_control::rpc::ProjectListResult = decode(value)?;
        Ok(row.projects)
    }

    fn claim(&self, id: &str, force: bool) -> Result<MutResult, Error> {
        let value = self.call(&Request::IssueClaim(ClaimParams {
            id: id.to_string(),
            force,
            agent: None,
        }))?;
        Ok(self.apply_mut(decode(value)?))
    }

    fn note(&self, id: &str, text: &str) -> Result<MutResult, Error> {
        let value = self.call(&Request::IssueNote(NoteParams {
            id: id.to_string(),
            text: text.to_string(),
        }))?;
        Ok(self.apply_mut(decode(value)?))
    }

    fn update(&self, req: UpdateReq) -> Result<MutResult, Error> {
        let value = self.call(&Request::IssueUpdate(UpdateParams {
            id: req.id,
            state: req.state,
            priority: req.priority.map(|c| c.to_string()),
            block: req.block,
            unblock: req.unblock,
            agent: None,
        }))?;
        Ok(self.apply_mut(decode(value)?))
    }

    fn open(&self, id: &str) -> Result<IssueDetail, Error> {
        let value = self.call(&Request::IssueOpen(IdParams { id: id.to_string() }))?;
        let row: vissue_control::rpc::IssueGetResult = decode(value)?;
        Ok(row.issue)
    }

    fn wait(&self, last: u64, timeout_ms: u64) -> Result<u64, Error> {
        let mut client = self.client.lock().expect("control client");
        match client.wait_notification(Duration::from_millis(timeout_ms.max(1))) {
            Ok(Notification::VaultChanged(changed)) => {
                self.revision.store(changed.revision, Ordering::SeqCst);
                self.generation.store(changed.generation, Ordering::SeqCst);
                Ok(changed.revision)
            }
            Ok(_) => Ok(self.revision.load(Ordering::SeqCst)),
            Err(_) => Ok(last),
        }
    }

    fn last_since_revision(&self) -> Option<Option<u64>> {
        *self.last_since.lock().expect("since")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::{BoardBackend, UpdateReq};
    use serde_json::json;
    use std::io::{BufReader, Write};
    use std::os::unix::net::UnixListener;
    use std::sync::{Arc, Mutex};
    use std::thread;
    use vissue_control::frame::{read_message, write_message};
    use vissue_control::rpc::JsonRpcRequest;
    use vissue_core::views::ListQuery;

    #[test]
    fn after_initialize_the_next_list_omits_since_revision() {
        let dir = tempfile::tempdir().unwrap();
        let sock = dir.path().join("control.sock");
        let layout = Layout::new(dir.path().join("vault"), "Software");
        let seen = Arc::new(Mutex::new(Vec::new()));
        let seen_cb = Arc::clone(&seen);
        let root = layout.root().display().to_string();
        let listener = UnixListener::bind(&sock).unwrap();
        thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut writer = stream;
            while let Ok((payload, framing)) = read_message(&mut reader) {
                let req: JsonRpcRequest = serde_json::from_slice(&payload).unwrap();
                let body = if req.method == "initialize" {
                    json!({
                        "jsonrpc": "2.0",
                        "id": req.id,
                        "result": {
                            "protocolVersion": 1,
                            "capabilities": [],
                            "root": root,
                            "prefix": "Software",
                            "generation": 9,
                            "revision": 41,
                            "identity": "tui"
                        }
                    })
                } else {
                    let since = req
                        .params
                        .as_ref()
                        .and_then(|p| p.get("since_revision"))
                        .cloned();
                    seen_cb.lock().unwrap().push(since);
                    json!({
                        "jsonrpc": "2.0",
                        "id": req.id,
                        "result": {
                            "issues": [],
                            "total": 0,
                            "matched": 0,
                            "revision": 41,
                            "generation": 9,
                            "unchanged": false
                        }
                    })
                };
                write_message(&mut writer, &serde_json::to_vec(&body).unwrap(), framing).unwrap();
                writer.flush().unwrap();
            }
        });

        let backend = ControlBackend::connect(&sock, &layout, "tui").unwrap();
        assert_eq!(backend.revision(), 41);
        assert_eq!(backend.live(), BackendKind::Control);
        backend.ready(None).unwrap();
        assert_eq!(backend.last_since_revision(), Some(None));
        backend.list(ListQuery::default()).unwrap();
        assert_eq!(backend.last_since_revision(), Some(Some(41)));
        let seen = seen.lock().unwrap();
        assert_eq!(seen.len(), 2);
        assert_eq!(seen[0], None);
        assert_eq!(seen[1], Some(json!(41)));
    }

    fn serve_methods(path: &std::path::Path, root: String) {
        let listener = UnixListener::bind(path).unwrap();
        thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut writer = stream;
            while let Ok((payload, framing)) = read_message(&mut reader) {
                let req: JsonRpcRequest = serde_json::from_slice(&payload).unwrap();
                let result = match req.method.as_str() {
                    "initialize" => json!({
                        "protocolVersion":1,"capabilities":[],"root":root,
                        "prefix":"Software","generation":2,"revision":3,"identity":"tui"
                    }),
                    "issue/get" | "issue/show" | "issue/open" => json!({
                        "id":"atlas-1a2b","project":"atlas","title":"t","state":"TODO",
                        "priority":"B","properties":{},"org_tags":[],"tags":[],
                        "blocked_by":[],"parent":null,"claimed_by":null,"claimed_at":null,
                        "file":"f","line_start":1,"line_end":2,"revision":3
                    }),
                    "issue/excerpt" => json!({
                        "id":"atlas-1a2b","file":"f","line_start":1,"line_end":2,
                        "text":"body","suppressed":false
                    }),
                    "issue/search" | "issue/claims" | "issue/agenda" | "issue/related" => {
                        json!([])
                    }
                    "issue/tree" => json!({
                        "id":"atlas-1a2b","state":"TODO","title":"t",
                        "children":[],"blocked_by":[]
                    }),
                    "project/list" => json!({"projects":["atlas"],"revision":3}),
                    "issue/claim" | "issue/note" | "issue/update" => json!({
                        "ok":true,"report":"ok","issue":null,"revision":4,"generation":3
                    }),
                    "issue/list" | "issue/ready" => json!({
                        "issues":[],"total":0,"matched":0,"revision":3,
                        "generation":2,"unchanged":false
                    }),
                    other => panic!("unexpected {other}"),
                };
                let body = json!({"jsonrpc":"2.0","id":req.id,"result":result});
                write_message(&mut writer, &serde_json::to_vec(&body).unwrap(), framing).unwrap();
                writer.flush().unwrap();
            }
        });
    }

    #[test]
    fn control_verbs_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let sock = dir.path().join("control.sock");
        let layout = Layout::new(dir.path().join("vault"), "Software");
        serve_methods(&sock, layout.root().display().to_string());
        let backend = ControlBackend::connect(&sock, &layout, "tui").unwrap();
        assert_eq!(backend.get("atlas-1a2b").unwrap().id, "atlas-1a2b");
        assert_eq!(backend.excerpt("atlas-1a2b").unwrap().text, "body");
        assert!(backend.search("x", 5).unwrap().is_empty());
        assert!(backend.claims(None, None).unwrap().is_empty());
        assert!(backend.agenda(14, None).unwrap().is_empty());
        assert_eq!(backend.tree("atlas-1a2b").unwrap().id, "atlas-1a2b");
        assert!(backend.related("atlas-1a2b", 2, 5).unwrap().is_empty());
        assert_eq!(backend.projects().unwrap(), ["atlas"]);
        assert!(backend.claim("atlas-1a2b", false).unwrap().ok);
        assert!(backend.note("atlas-1a2b", "hi").unwrap().ok);
        assert!(
            backend
                .update(UpdateReq {
                    id: "atlas-1a2b".into(),
                    state: Some("STARTED".into()),
                    ..UpdateReq::default()
                })
                .unwrap()
                .ok
        );
        assert_eq!(backend.open("atlas-1a2b").unwrap().id, "atlas-1a2b");
        assert_eq!(backend.wait(3, 5).unwrap(), 3);
    }

    #[test]
    fn root_mismatch_refuses_the_socket() {
        let dir = tempfile::tempdir().unwrap();
        let sock = dir.path().join("control.sock");
        let layout = Layout::new(dir.path().join("vault"), "Software");
        let listener = UnixListener::bind(&sock).unwrap();
        thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut writer = stream;
            let (payload, framing) = read_message(&mut reader).unwrap();
            let req: JsonRpcRequest = serde_json::from_slice(&payload).unwrap();
            let body = json!({
                "jsonrpc": "2.0",
                "id": req.id,
                "result": {
                    "protocolVersion": 1,
                    "capabilities": [],
                    "root": "/other/vault",
                    "prefix": "Software",
                    "generation": 1,
                    "revision": 1,
                    "identity": "tui"
                }
            });
            write_message(&mut writer, &serde_json::to_vec(&body).unwrap(), framing).unwrap();
            writer.flush().unwrap();
        });
        let err = match ControlBackend::connect(&sock, &layout, "tui") {
            Ok(_) => panic!("expected root mismatch"),
            Err(err) => err,
        };
        match err {
            ControlAttachError::Mismatch { got_root, .. } => {
                assert_eq!(got_root, "/other/vault");
            }
            other => panic!("{other:?}"),
        }
    }
}
