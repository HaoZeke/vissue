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
