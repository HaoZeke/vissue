//! The operation set, read from the Cap'n Proto schema itself.
//!
//! Every verb reaches a caller through three surfaces: the command line, the
//! control socket, and the MCP tool list. Each was declared in its own file in its
//! own idiom, so a verb could exist on one and not the others. That happened
//! repeatedly and nothing caught it: `vote` shipped on the command line alone,
//! `append` had no socket method for as long as the socket existed, and a test
//! asserting `issue/fold` was an unknown method became wrong the day fold got one.
//!
//! The set now lives in `schema/vissue.capnp` and this module reads it. Not a copy
//! of it and not a parser for it: `capnp compile` encodes the constant into the
//! generated `vissue_capnp.rs`, so the bytes this reads are the schema, and a
//! surface is checked against them.
//!
//! No toolchain at build time. `capnp` the compiler is absent from the machines
//! that build this, so the generated file is committed and regenerating it is a
//! maintainer step; what ships is the encoded constant, which the pure-Rust `capnp`
//! runtime reads anywhere.

use crate::vissue_capnp::{OPERATIONS, operation};

/// One verb, named on each surface that carries it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Operation {
    /// Subcommand name, as clap spells it.
    pub cli: String,
    /// Control-socket method, empty when the verb has none.
    pub socket: String,
    /// MCP tool name, empty when the verb is deliberately not a tool.
    pub mcp: String,
    /// Whether the verb changes a file.
    pub mutates: bool,
    /// Whether the verb only makes sense in the process it is typed into.
    pub local: bool,
    /// Other names the command line answers to for this verb.
    pub aliases: Vec<String>,
    /// Why a surface is empty, when one is.
    pub note: String,
    /// Fields the verb takes, each named per surface.
    pub fields: Vec<Field>,
}

/// One field of one verb, named per surface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field {
    /// Flag name without dashes, empty when absent or positional.
    pub cli: String,
    /// MCP argument name, empty when the tool does not take it.
    pub tool: String,
    /// Control-socket parameter name, empty when the method does not take it.
    pub socket: String,
    /// Why a surface is empty, or why the names differ.
    pub note: String,
    /// Rust type of the tool argument, empty when the tool does not take it.
    pub tool_type: String,
    /// Rust type of the socket parameter, empty when the method does not take it.
    pub socket_type: String,
}

/// The operation set as the schema states it.
///
/// # Panics
///
/// Panics if the encoded constant cannot be read, which would mean the committed
/// generated file is corrupt rather than that a caller did anything wrong.
#[must_use]
pub fn operations() -> Vec<Operation> {
    let list = OPERATIONS
        .get()
        .expect("the encoded operation set in vissue_capnp.rs is unreadable");
    list.iter().map(read_one).collect()
}

fn read_one(row: operation::Reader<'_>) -> Operation {
    let text = |r: ::capnp::Result<::capnp::text::Reader<'_>>| -> String {
        r.ok()
            .and_then(|t| t.to_str().ok().map(str::to_string))
            .unwrap_or_default()
    };
    let fields = row
        .get_fields()
        .map(|list| {
            list.iter()
                .map(|f| Field {
                    cli: text(f.get_cli()),
                    tool: text(f.get_tool()),
                    socket: text(f.get_socket()),
                    note: text(f.get_note()),
                    tool_type: text(f.get_tool_type()),
                    socket_type: text(f.get_socket_type()),
                })
                .collect()
        })
        .unwrap_or_default();
    Operation {
        cli: text(row.get_cli()),
        socket: text(row.get_socket()),
        mcp: text(row.get_mcp()),
        mutates: row.get_mutates(),
        local: row.get_local(),
        aliases: row
            .get_aliases()
            .map(|list| {
                list.iter()
                    .filter_map(|a| a.ok().and_then(|t| t.to_str().ok().map(str::to_string)))
                    .collect()
            })
            .unwrap_or_default(),
        note: text(row.get_note()),
        fields,
    }
}

/// Verbs the schema records as reaching the socket, mutating or not.
#[must_use]
pub fn socket_methods() -> Vec<String> {
    operations()
        .into_iter()
        .filter(|o| !o.socket.is_empty())
        .map(|o| o.socket)
        .collect()
}

/// Every subcommand the schema knows, including the local-only ones.
#[must_use]
pub fn cli_verbs() -> Vec<String> {
    operations()
        .into_iter()
        .filter(|o| !o.cli.is_empty())
        .flat_map(|o| std::iter::once(o.cli).chain(o.aliases))
        .collect()
}

/// Every mutating verb's socket method, skipping any the schema leaves empty.
#[must_use]
pub fn mutating_socket_methods() -> Vec<String> {
    operations()
        .into_iter()
        .filter(|o| o.mutates && !o.socket.is_empty())
        .map(|o| o.socket)
        .collect()
}

/// Every mutating verb's subcommand.
#[must_use]
pub fn mutating_cli_verbs() -> Vec<String> {
    operations()
        .into_iter()
        .filter(|o| o.mutates && !o.cli.is_empty())
        .map(|o| o.cli)
        .collect()
}

/// Every mutating verb's MCP tool, skipping the ones deliberately absent.
#[must_use]
pub fn mutating_mcp_tools() -> Vec<String> {
    operations()
        .into_iter()
        .filter(|o| o.mutates && !o.mcp.is_empty())
        .map(|o| o.mcp)
        .collect()
}

/// The schema as its text states it, for comparison with the encoded constant.
///
/// This exists to close a hole in the arrangement rather than to be used at
/// runtime. The constant the checks read is compiled into `vissue_capnp.rs`, which
/// is committed and regenerated by hand. Edit `vissue.capnp`, forget to regenerate,
/// and every check happily validates against the previous schema and passes. The
/// schema would be authoritative in the documentation and not in fact.
///
/// So the text is read too, and a test compares the two. A deliberately small
/// reader for one list of flat records, not a Cap'n Proto parser: it needs to run
/// where `capnp` is not installed, which is everywhere this is built.
///
/// Returns one entry per operation: `(cli, socket, mcp, field-count)`.
#[must_use]
pub fn parse_schema_text(text: &str) -> Vec<(String, String, String, usize)> {
    let Some(start) = text.find("const operations") else {
        return Vec::new();
    };
    let body = &text[start..];
    let mut out: Vec<(String, String, String, usize)> = Vec::new();
    for line in body.lines() {
        let trimmed = line.trim();
        let quoted = |name: &str| -> Option<String> {
            let at = trimmed.find(&format!("{name} = "))?;
            let rest = &trimmed[at..];
            let open = rest.find('"')?;
            let close = rest[open + 1..].find('"')?;
            Some(rest[open + 1..open + 1 + close].to_string())
        };
        // An operation row opens with its three surface names on one line.
        if trimmed.starts_with("( cli = ")
            && trimmed.contains("mutates = ")
            && let (Some(cli), Some(socket), Some(mcp)) =
                (quoted("cli"), quoted("socket"), quoted("mcp"))
        {
            out.push((cli, socket, mcp, 0));
            continue;
        }
        // A field row also opens with `( cli = `, so it is told apart by carrying a
        // `tool =` and no `socket = "issue/`. Counting them catches a field added to
        // the text without a regeneration.
        if trimmed.starts_with("( cli = ")
            && trimmed.contains("tool = ")
            && !trimmed.contains("mutates = ")
            && let Some(last) = out.last_mut()
        {
            last.3 += 1;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn schema_text() -> String {
        let path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../schema/vissue.capnp");
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
    }

    /// The committed generated file and the schema text say the same thing.
    ///
    /// Without this the schema is authoritative only in the documentation.
    /// `vissue_capnp.rs` is regenerated by hand, so an edit to the schema that
    /// nobody regenerated leaves every other check validating the previous schema
    /// and passing.
    #[test]
    fn the_generated_constant_matches_the_schema_text() {
        let from_text = parse_schema_text(&schema_text());
        assert!(
            !from_text.is_empty(),
            "no operations parsed from the schema text; the reader and the file have diverged"
        );
        let from_bytes: Vec<(String, String, String, usize)> = operations()
            .into_iter()
            .map(|o| (o.cli, o.socket, o.mcp, o.fields.len()))
            .collect();
        assert_eq!(
            from_text, from_bytes,
            "schema/vissue.capnp and the committed vissue_capnp.rs disagree; \
             regenerate it, see schema/README.md"
        );
    }

    #[test]
    fn the_schema_constant_reads_back() {
        let ops = operations();
        assert!(
            ops.len() >= 10,
            "the encoded operation set looks truncated: {ops:?}"
        );
        assert!(ops.iter().any(|o| o.cli == "create"));
        assert!(ops.iter().any(|o| o.cli == "vote"), "vote is missing");
    }

    /// Every mutating verb has a socket method. This was false for five verbs at
    /// once, and for `append` the whole time the socket existed.
    #[test]
    fn every_mutating_verb_has_a_socket_method() {
        let missing: Vec<String> = operations()
            .into_iter()
            .filter(|o| o.mutates && o.socket.is_empty())
            .map(|o| o.cli)
            .collect();
        assert!(
            missing.is_empty(),
            "these change a file and have no socket method: {missing:?}"
        );
    }

    /// A surface left empty says why, so a deliberate omission cannot pass for an
    /// oversight or the other way round.
    #[test]
    fn a_missing_surface_carries_its_reason() {
        for o in operations() {
            if o.socket.is_empty() || o.mcp.is_empty() {
                assert!(
                    !o.note.is_empty(),
                    "{} leaves a surface empty and says nothing about why",
                    o.cli
                );
            }
        }
    }

    /// Names are not blank and not accidentally duplicated.
    #[test]
    fn the_names_are_distinct_and_present() {
        let ops = operations();
        // A row may have no subcommand, when the command line reaches the operation
        // through a flag on another verb: `vissue_org` is `show --org`. It has to
        // reach *some* surface and say why the others are empty, which the note
        // check enforces, so the requirement here is a surface rather than a
        // subcommand.
        for o in &ops {
            assert!(
                !(o.cli.is_empty() && o.socket.is_empty() && o.mcp.is_empty()),
                "an operation reaches no surface at all: {o:?}"
            );
        }
        // Empty is not a name. Two rows without a subcommand are two operations the
        // command line reaches through a flag, not a collision.
        let mut clis: Vec<&str> = ops
            .iter()
            .map(|o| o.cli.as_str())
            .filter(|c| !c.is_empty())
            .collect();
        clis.sort_unstable();
        let before = clis.len();
        clis.dedup();
        assert_eq!(before, clis.len(), "two operations share a subcommand");

        // Two subcommands may answer from one method, and one pair does: `identity`
        // and `whoami` both reach `identity/get`. That is a fact about the surfaces
        // rather than a mistake, so it is allowed when the rows say so. Silence is
        // what is refused, because an undocumented duplicate is the shape a
        // copy-paste error takes.
        let mut by_method: std::collections::BTreeMap<&str, Vec<&Operation>> =
            std::collections::BTreeMap::new();
        for o in &ops {
            if !o.socket.is_empty() {
                by_method.entry(o.socket.as_str()).or_default().push(o);
            }
        }
        for (method, sharers) in by_method {
            if sharers.len() < 2 {
                continue;
            }
            let silent: Vec<&str> = sharers
                .iter()
                .filter(|o| o.note.is_empty())
                .map(|o| o.cli.as_str())
                .collect();
            assert!(
                silent.is_empty(),
                "{method} answers for {silent:?} and none of them says why"
            );
        }
    }
}
