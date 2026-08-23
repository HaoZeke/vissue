#![allow(missing_docs)]

use std::process::Command;

#[test]
fn version_flag_prints_the_binary_version() {
    let output = Command::new(env!("CARGO_BIN_EXE_vissue-mcp"))
        .arg("--version")
        .output()
        .expect("run vissue-mcp --version");

    assert!(output.status.success(), "status: {}", output.status);
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        format!("vissue-mcp {}\n", env!("CARGO_PKG_VERSION"))
    );
    assert!(output.stderr.is_empty());
}

/// The reference documents every tool the server exposes.
///
/// A tool nobody wrote down is one an agent will not find, and the reference
/// is where the surface is published. Reading the source rather than a second
/// list keeps the two from drifting the way a hand-maintained copy does.
/// Tool names the server declares, read off the `#[tool]` functions.
fn tool_names(server: &str) -> Vec<String> {
    let mut tools: Vec<String> = server
        .lines()
        .filter_map(|l| l.trim().strip_prefix("async fn "))
        .filter_map(|rest| rest.split('(').next())
        .filter(|name| name.starts_with("vissue_"))
        .map(str::to_string)
        .collect();
    tools.sort_unstable();
    tools.dedup();
    tools
}

#[test]
fn the_reference_lists_every_tool() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let server = std::fs::read_to_string(root.join("src/server.rs")).expect("server.rs");
    let reference =
        std::fs::read_to_string(root.join("../../docs/orgmode/reference.org")).expect("reference");

    let tools = tool_names(&server);
    assert!(tools.len() > 20, "no tools found: {tools:?}");

    let missing: Vec<&str> = tools
        .iter()
        .map(String::as_str)
        .filter(|name| !reference.contains(&format!("={name}=")))
        .collect();
    assert!(
        missing.is_empty(),
        "not in docs/orgmode/reference.org: {missing:?}"
    );
}

/// The tool list carries every verb the schema names as a tool.
///
/// `vote` shipped on the command line and reached no agent for a whole session
/// because nothing asked this question. A verb the schema deliberately keeps off
/// the tool list carries its reason there, and `mutating_mcp_tools` skips those, so
/// this fails only for an omission nobody wrote down.
#[test]
fn the_tool_list_offers_every_tool_the_schema_names() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let server = std::fs::read_to_string(root.join("src/server.rs")).expect("server.rs");
    let tools = tool_names(&server);
    let missing: Vec<String> = vissue_core::surface::mutating_mcp_tools()
        .into_iter()
        .filter(|name| !tools.iter().any(|t| t == name))
        .collect();
    assert!(
        missing.is_empty(),
        "the schema names these tools and the server does not expose them: {missing:?}"
    );
}

/// Each tool takes the arguments the schema names for it.
///
/// The verb-level check says `vissue_create` exists. It does not say the tool can
/// set what the subcommand can set, and it could not: MCP `create` took neither
/// `deadline` nor `scheduled`, both of which the subcommand has always taken, so an
/// agent could not set a date a person could. Nothing was wrong on either side in
/// isolation, which is why only a check across the pair finds it.
///
/// The schema holds both spellings because some divergence is forced: `--type`
/// cannot be a Rust field named `type` and `--for` cannot be one named `for`. The
/// tool name is what is checked here, and a field the tool deliberately omits leaves
/// it empty and says why.
#[test]
fn each_tool_takes_the_arguments_the_schema_names() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let tools_src = std::fs::read_to_string(root.join("src/tools.rs")).expect("tools.rs");

    let mut wrong = Vec::new();
    for op in vissue_core::surface::operations() {
        // Only verbs whose fields the schema names. A read verb takes shared or no
        // arguments and declares none, so there is nothing here to satisfy and no
        // `<Verb>Args` struct to look for.
        if op.mcp.is_empty() || !op.fields.iter().any(|f| !f.tool.is_empty()) {
            continue;
        }
        // vissue_create -> CreateArgs
        let stem = op.mcp.trim_start_matches("vissue_");
        let mut camel = String::new();
        for part in stem.split('_') {
            let mut c = part.chars();
            if let Some(f) = c.next() {
                camel.push(f.to_ascii_uppercase());
                camel.push_str(c.as_str());
            }
        }
        let struct_name = format!("{camel}Args");
        let Some(at) = tools_src.find(&format!("pub struct {struct_name} {{")) else {
            wrong.push(format!("{}: no {struct_name} to check", op.mcp));
            continue;
        };
        let body_end = tools_src[at..]
            .find("\n}")
            .map_or(tools_src.len(), |e| at + e);
        let body = &tools_src[at..body_end];
        for field in &op.fields {
            if field.tool.is_empty() {
                continue;
            }
            if !body.contains(&format!("pub {}:", field.tool)) {
                wrong.push(format!("{struct_name} has no {}", field.tool));
            }
        }
    }
    assert!(
        wrong.is_empty(),
        "the schema names arguments these tools do not take: {wrong:?}"
    );
}

/// And every tool the server exposes is in the schema.
///
/// Every other check here runs schema to surface: it catches a verb the schema names
/// and the surface lacks. Nothing ran the other way, so a tool that exists and the
/// schema omits was invisible, which is the same asymmetry that let verbs drift in
/// the first place. It had already bitten: the schema claimed `normalize` had no tool
/// while `vissue_normalize` was sitting in the server, and every check passed.
#[test]
fn every_tool_the_server_exposes_is_in_the_schema() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let server = std::fs::read_to_string(root.join("src/server.rs")).expect("server.rs");
    let exposed = tool_names(&server);

    let known: Vec<String> = vissue_core::surface::operations()
        .into_iter()
        .filter(|o| !o.mcp.is_empty())
        .map(|o| o.mcp)
        .collect();

    let unknown: Vec<&String> = exposed
        .iter()
        .filter(|t| !known.iter().any(|k| k == *t))
        .collect();
    assert!(
        unknown.is_empty(),
        "these tools exist and no schema row mentions them: {unknown:?}"
    );
}
