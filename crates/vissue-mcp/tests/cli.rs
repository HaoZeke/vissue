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
#[test]
fn the_reference_lists_every_tool() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let server = std::fs::read_to_string(root.join("src/server.rs")).expect("server.rs");
    let reference =
        std::fs::read_to_string(root.join("../../docs/orgmode/reference.org")).expect("reference");

    let mut tools: Vec<&str> = server
        .lines()
        .filter_map(|l| l.trim().strip_prefix("async fn "))
        .filter_map(|rest| rest.split('(').next())
        .filter(|name| name.starts_with("vissue_"))
        .collect();
    tools.sort_unstable();
    tools.dedup();
    assert!(tools.len() > 20, "no tools found: {tools:?}");

    let missing: Vec<&str> = tools
        .iter()
        .copied()
        .filter(|name| !reference.contains(&format!("={name}=")))
        .collect();
    assert!(
        missing.is_empty(),
        "not in docs/orgmode/reference.org: {missing:?}"
    );
}
