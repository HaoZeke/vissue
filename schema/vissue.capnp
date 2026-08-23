@0xa1e7c747dbf99b93;

# The operation set, and the one place it is written down.
#
# Every verb this tracker offers reaches a caller through three surfaces: the
# command line, the control socket, and the MCP tool list. Each was declared in
# its own file in its own idiom, so a verb could exist on one and not the others,
# and repeatedly did: `vote` shipped on the command line alone, `append` had no
# socket method for as long as the socket existed, and a test asserting
# `issue/fold` was an unknown method became wrong the day fold got one.
#
# Nothing detected any of that. The reference-completeness tests covered the docs
# and not the surfaces, so the docs stayed honest about a surface set that was not.
#
# So the set lives here and the surfaces are checked against it. A verb added to
# this file that no socket method serves is a failing test naming the gap, and a
# verb added to a surface but not here is the same. Written in Cap'n Proto because
# a schema is the artefact a non-Rust client can read too; the wire formats are
# unchanged, and this file is not compiled into the build.
#
# The names are the contract. `cli` is the subcommand, `socket` the JSON-RPC
# method, `mcp` the tool. An empty `mcp` means the verb is deliberately absent
# from the tool list, and the reason belongs in `note`.

struct Operation {
  # One verb, named on each surface that carries it.

  cli @0 :Text;
  # Subcommand name, as clap spells it.

  socket @1 :Text;
  # Control-socket method, or empty when the verb has none.

  mcp @2 :Text;
  # MCP tool name, or empty when the verb is deliberately not a tool.

  mutates @3 :Bool;
  # Whether the verb changes a file. A mutating verb must reach every surface,
  # because a caller that has to leave the socket for one write puts a hole in the
  # change stream exactly where that write was.

  note @4 :Text;
  # Why a surface is empty, when one is. Empty otherwise.
}

const operations :List(Operation) = [
  ( cli = "create",    socket = "issue/create",    mcp = "vissue_create",    mutates = true,  note = "" ),
  ( cli = "update",    socket = "issue/update",    mcp = "vissue_update",    mutates = true,  note = "" ),
  ( cli = "claim",     socket = "issue/claim",     mcp = "vissue_claim",     mutates = true,  note = "" ),
  ( cli = "note",      socket = "issue/note",      mcp = "vissue_note",      mutates = true,  note = "" ),
  ( cli = "append",    socket = "issue/append",    mcp = "vissue_append",    mutates = true,  note = "" ),
  ( cli = "refile",    socket = "issue/refile",    mcp = "vissue_refile",    mutates = true,  note = "" ),
  ( cli = "reject",    socket = "issue/reject",    mcp = "vissue_reject",    mutates = true,  note = "" ),
  ( cli = "resolve",   socket = "issue/resolve",   mcp = "vissue_resolve",   mutates = true,  note = "" ),
  ( cli = "vote",      socket = "issue/vote",      mcp = "vissue_vote",      mutates = true,  note = "" ),
  ( cli = "fold",      socket = "issue/fold",      mcp = "vissue_fold",      mutates = true,  note = "" ),
  ( cli = "normalize", socket = "issue/normalize", mcp = "",                 mutates = true,
    note = "no tool: rewriting every heading in a corpus is not a thing to hand an agent" ),
];
