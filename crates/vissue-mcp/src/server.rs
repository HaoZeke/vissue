//! The MCP tool surface, calling vissue-core in process.

use rmcp::{
    handler::server::wrapper::Parameters, handler::server::ServerHandler, model::*, tool,
    tool_handler, tool_router, ErrorData as McpError,
};

use vissue_core::config::Layout;
use vissue_core::mirror::{self, Format};
use vissue_core::ops::{self, CreateOpts};
use vissue_core::store;
use vissue_core::{agent, report};

use crate::tools::*;

/// The tool router is built by `#[tool_handler]` through `Self::tool_router()`,
/// so the server carries only the layout it acts on.
#[derive(Clone)]
pub struct VissueServer {
    layout: Layout,
}

fn text(result: anyhow::Result<String>) -> Result<CallToolResult, McpError> {
    match result {
        Ok(s) => Ok(CallToolResult::success(vec![Content::text(s)])),
        Err(e) => Err(McpError::internal_error(format!("{e:#}"), None)),
    }
}

fn json(result: anyhow::Result<serde_json::Value>) -> Result<CallToolResult, McpError> {
    match result {
        Ok(v) => {
            let rendered = serde_json::to_string_pretty(&v).unwrap_or_else(|_| "null".to_string());
            Ok(CallToolResult::success(vec![Content::text(rendered)]))
        }
        Err(e) => Err(McpError::internal_error(format!("{e:#}"), None)),
    }
}

#[tool_router]
impl VissueServer {
    /// Resolve the layout from `VISSUE_ROOT` and `VISSUE_PREFIX`, or the
    /// current directory.
    pub fn from_env() -> anyhow::Result<Self> {
        Ok(Self::with_layout(Layout::resolve(None, None)?))
    }

    pub fn with_layout(layout: Layout) -> Self {
        Self { layout }
    }

    #[tool(description = "List the projects that hold an issues.org under the tracker root.")]
    async fn vissue_projects(&self) -> Result<CallToolResult, McpError> {
        text(store::list_projects(&self.layout).map(|ps| format!("{}\n", ps.join("\n"))))
    }

    #[tool(description = "List issues, optionally filtered by project and state.")]
    async fn vissue_list(
        &self,
        Parameters(args): Parameters<ListArgs>,
    ) -> Result<CallToolResult, McpError> {
        json(agent::issues_json(
            &self.layout,
            args.project.as_deref(),
            args.state.as_deref(),
            false,
        ))
    }

    #[tool(description = "List actionable issues: TODO or STARTED with no open blocker.")]
    async fn vissue_ready(
        &self,
        Parameters(args): Parameters<ProjectArgs>,
    ) -> Result<CallToolResult, McpError> {
        json(agent::issues_json(
            &self.layout,
            args.project.as_deref(),
            None,
            true,
        ))
    }

    #[tool(description = "Show one issue's metadata and file range. Never returns body prose.")]
    async fn vissue_show(
        &self,
        Parameters(args): Parameters<IdArgs>,
    ) -> Result<CallToolResult, McpError> {
        json(agent::show_json(&self.layout, &args.issue_id))
    }

    #[tool(description = "Create an issue in a project's issues.org.")]
    async fn vissue_create(
        &self,
        Parameters(args): Parameters<CreateArgs>,
    ) -> Result<CallToolResult, McpError> {
        text(ops::create(
            &self.layout,
            &args.project,
            &args.title,
            CreateOpts {
                priority: priority_char(args.priority.as_ref()),
                issue_type: args.issue_type.as_deref(),
                tags: args.tags.as_deref(),
                parent: args.parent.as_deref(),
                body: args.body.as_deref(),
                ..Default::default()
            },
        ))
    }

    #[tool(description = "Update an issue's state, priority, or blocker edges.")]
    async fn vissue_update(
        &self,
        Parameters(args): Parameters<UpdateArgs>,
    ) -> Result<CallToolResult, McpError> {
        let outcome = ops::update(
            &self.layout,
            &args.issue_id,
            args.state.as_deref(),
            priority_char(args.priority.as_ref()),
            args.block.as_deref(),
            args.unblock.as_deref(),
        );
        text(outcome.map(|o| {
            let mut s = o.report;
            for hint in o.hints {
                s.push_str(&format!("[hint] {hint}\n"));
            }
            s
        }))
    }

    #[tool(description = "Claim an issue: move it to STARTED. Fails on a closed issue.")]
    async fn vissue_claim(
        &self,
        Parameters(args): Parameters<IdArgs>,
    ) -> Result<CallToolResult, McpError> {
        text(agent::claim(&self.layout, &args.issue_id))
    }

    #[tool(description = "Count issues, optionally filtered by project, state, or readiness.")]
    async fn vissue_count(
        &self,
        Parameters(args): Parameters<CountArgs>,
    ) -> Result<CallToolResult, McpError> {
        text(report::count(
            &self.layout,
            args.project.as_deref(),
            args.state.as_deref(),
            args.ready.unwrap_or(false),
        ))
    }

    #[tool(description = "Substring search over ids, titles, properties, and bodies.")]
    async fn vissue_search(
        &self,
        Parameters(args): Parameters<SearchArgs>,
    ) -> Result<CallToolResult, McpError> {
        text(report::search(
            &self.layout,
            &args.query,
            args.limit.unwrap_or(20),
        ))
    }

    #[tool(description = "List issues whose PARENT property matches this id.")]
    async fn vissue_children(
        &self,
        Parameters(args): Parameters<IdArgs>,
    ) -> Result<CallToolResult, McpError> {
        text(report::children(&self.layout, &args.issue_id))
    }

    #[tool(description = "List issues that refer to this id through any relation.")]
    async fn vissue_backlinks(
        &self,
        Parameters(args): Parameters<IdArgs>,
    ) -> Result<CallToolResult, McpError> {
        text(report::backlinks(&self.layout, &args.issue_id))
    }

    #[tool(description = "Issues waiting on this id. Dependency hygiene alias for backlinks.")]
    async fn vissue_waiting_on(
        &self,
        Parameters(args): Parameters<IdArgs>,
    ) -> Result<CallToolResult, McpError> {
        text(agent::waiting_on(&self.layout, &args.issue_id))
    }

    #[tool(description = "The first lines of an issue's file range, screened for secrets.")]
    async fn vissue_body_excerpt(
        &self,
        Parameters(args): Parameters<IdArgs>,
    ) -> Result<CallToolResult, McpError> {
        text(agent::body_excerpt(&self.layout, &args.issue_id))
    }

    #[tool(description = "Children and blockers below an id, as ascii indent or Graphviz DOT.")]
    async fn vissue_tree(
        &self,
        Parameters(args): Parameters<TreeArgs>,
    ) -> Result<CallToolResult, McpError> {
        text(report::tree(
            &self.layout,
            &args.issue_id,
            args.format.as_deref().unwrap_or("ascii"),
        ))
    }

    #[tool(description = "The blocker and parent graph as Graphviz DOT.")]
    async fn vissue_graph(
        &self,
        Parameters(args): Parameters<ProjectArgs>,
    ) -> Result<CallToolResult, McpError> {
        text(report::graph(&self.layout, args.project.as_deref()))
    }

    #[tool(description = "A markdown roadmap of active and closed work.")]
    async fn vissue_roadmap(
        &self,
        Parameters(args): Parameters<ProjectArgs>,
    ) -> Result<CallToolResult, McpError> {
        text(report::roadmap(&self.layout, args.project.as_deref()))
    }

    #[tool(description = "One JSON object per issue per line.")]
    async fn vissue_export(
        &self,
        Parameters(args): Parameters<ProjectArgs>,
    ) -> Result<CallToolResult, McpError> {
        text(report::export(&self.layout, args.project.as_deref()))
    }

    #[tool(description = "Validate the corpus: dangling edges, bad dates, duplicate ids.")]
    async fn vissue_check(&self) -> Result<CallToolResult, McpError> {
        text(report::check(&self.layout).map(|r| r.text))
    }

    #[tool(description = "Checklist for agents and CI: stalled claims plus corpus validation.")]
    async fn vissue_hygiene(&self) -> Result<CallToolResult, McpError> {
        text(agent::hygiene(&self.layout))
    }

    #[tool(description = "Render a read-only projection of selected projects.")]
    async fn vissue_mirror(
        &self,
        Parameters(args): Parameters<MirrorArgs>,
    ) -> Result<CallToolResult, McpError> {
        let format = match Format::parse(args.format.as_deref().unwrap_or("org")) {
            Ok(f) => f,
            Err(e) => return Err(McpError::invalid_params(format!("{e:#}"), None)),
        };
        text(mirror::render(
            &self.layout,
            &args.projects.unwrap_or_default(),
            format,
            args.state.as_deref(),
        ))
    }

    #[tool(description = "Report the server version and the resolved root and prefix.")]
    async fn vissue_identity(&self) -> Result<CallToolResult, McpError> {
        text(Ok(format!(
            "vissue-mcp {}\nroot:   {}\nprefix: {}\n",
            env!("CARGO_PKG_VERSION"),
            self.layout.root().display(),
            self.layout.prefix()
        )))
    }
}

#[tool_handler]
impl ServerHandler for VissueServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("vissue", env!("CARGO_PKG_VERSION")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vissue_core::config::DEFAULT_PREFIX;

    #[tokio::test]
    async fn tools_answer_against_a_temporary_layout() {
        let dir = tempfile::tempdir().unwrap();
        let layout = Layout::new(dir.path(), DEFAULT_PREFIX);
        std::fs::create_dir_all(layout.projects_dir()).unwrap();
        ops::create(&layout, "sample", "first", CreateOpts::default()).unwrap();

        let server = VissueServer::with_layout(layout);
        let listed = server
            .vissue_list(Parameters(ListArgs {
                project: None,
                state: None,
            }))
            .await
            .unwrap();
        assert_eq!(listed.is_error, Some(false));

        let counted = server
            .vissue_count(Parameters(CountArgs {
                project: None,
                state: None,
                ready: Some(true),
            }))
            .await
            .unwrap();
        assert_eq!(counted.is_error, Some(false));
    }

    #[tokio::test]
    async fn an_unknown_mirror_format_is_an_invalid_parameter() {
        let dir = tempfile::tempdir().unwrap();
        let layout = Layout::new(dir.path(), DEFAULT_PREFIX);
        let server = VissueServer::with_layout(layout);
        let err = server
            .vissue_mirror(Parameters(MirrorArgs {
                projects: None,
                format: Some("pdf".into()),
                state: None,
            }))
            .await
            .unwrap_err();
        assert!(format!("{err:?}").contains("pdf"), "{err:?}");
    }
}
