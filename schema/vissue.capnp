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

  flags @5 :List(Text);
  # Command-line flags the verb takes, without the leading dashes, and only the
  # ones that name a field rather than a mode. Positional arguments are not in
  # here: `create` and `reject` take their title positionally, and the first
  # version of this list said `title` for both because it was written from memory
  # rather than from the parser. The check found all three mistakes on its first
  # run, which is the argument for having it. Naming the verb on each surface
  # stops a verb going missing; it does not stop the surfaces disagreeing about
  # what to call a field, which is the next way this drifts: a socket method
  # taking `body` where the subcommand takes `--text` is two spellings of one
  # idea and no test would notice.
}

const operations :List(Operation) = [
  ( cli = "create",    socket = "issue/create",    mcp = "vissue_create",    mutates = true,  note = "",
    flags = ["project", "priority", "type", "tags", "parent", "body", "body-file", "deadline", "scheduled"] ),
  ( cli = "update",    socket = "issue/update",    mcp = "vissue_update",    mutates = true,  note = "",
    flags = ["state", "priority", "block", "unblock", "if-state", "if-gen"] ),
  ( cli = "claim",     socket = "issue/claim",     mcp = "vissue_claim",     mutates = true,  note = "",
    flags = ["force"] ),
  ( cli = "note",      socket = "issue/note",      mcp = "vissue_note",      mutates = true,  note = "",
    flags = [] ),
  ( cli = "append",    socket = "issue/append",    mcp = "vissue_append",    mutates = true,  note = "",
    flags = ["text", "file"] ),
  ( cli = "refile",    socket = "issue/refile",    mcp = "vissue_refile",    mutates = true,  note = "",
    flags = ["to"] ),
  ( cli = "reject",    socket = "issue/reject",    mcp = "vissue_reject",    mutates = true,  note = "",
    flags = ["to", "project", "reason"] ),
  ( cli = "resolve",   socket = "issue/resolve",   mcp = "vissue_resolve",   mutates = true,  note = "",
    flags = [] ),
  ( cli = "vote",      socket = "issue/vote",      mcp = "vissue_vote",      mutates = true,  note = "",
    flags = ["for"] ),
  ( cli = "fold",      socket = "issue/fold",      mcp = "vissue_fold",      mutates = true,  note = "",
    flags = ["project"] ),
  ( cli = "normalize", socket = "issue/normalize", mcp = "",                 mutates = true,
    note = "no tool: rewriting every heading in a corpus is not a thing to hand an agent",
    flags = ["project", "dry-run"] ),
];
