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
    /// Why a surface is empty, when one is.
    pub note: String,
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
    Operation {
        cli: text(row.get_cli()),
        socket: text(row.get_socket()),
        mcp: text(row.get_mcp()),
        mutates: row.get_mutates(),
        note: text(row.get_note()),
    }
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

#[cfg(test)]
mod tests {
    use super::*;

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
        for o in &ops {
            assert!(!o.cli.is_empty(), "an operation has no subcommand: {o:?}");
        }
        let mut clis: Vec<&str> = ops.iter().map(|o| o.cli.as_str()).collect();
        clis.sort_unstable();
        let before = clis.len();
        clis.dedup();
        assert_eq!(before, clis.len(), "two operations share a subcommand");

        let mut sockets: Vec<&str> = ops
            .iter()
            .map(|o| o.socket.as_str())
            .filter(|s| !s.is_empty())
            .collect();
        sockets.sort_unstable();
        let before = sockets.len();
        sockets.dedup();
        assert_eq!(
            before,
            sockets.len(),
            "two operations share a socket method"
        );
    }
}
