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

  fields @5 :List(Field);
  # The fields the verb takes, each named on the surfaces that carry it.
  #
  # Naming a verb per surface stops the verb going missing. It says nothing about
  # the surfaces disagreeing on what to call a field, which is the next way this
  # drifts, and it had already happened: `vote` takes `--for` on the command line
  # and `choice` as a tool argument, and MCP `create` could set neither a deadline
  # nor a scheduled date that the subcommand has always taken.
  #
  # Positional arguments are not in here. `create` and `reject` take their title
  # positionally, and the first version of this list said `title` for both because
  # it was written from memory rather than from the parser.
}

struct Field {
  # One field of one verb, named per surface.
  #
  # Two names rather than one, because some divergence is forced rather than
  # sloppy: `--type` cannot be a Rust field called `type`, and `--for` cannot be a
  # field called `for`, since both are keywords. Recording the pair is honest about
  # that, where a single canonical name would either lie or forbid the flag.

  cli @0 :Text;
  # Flag name without the leading dashes, empty when the surface does not take it
  # or takes it positionally.

  tool @1 :Text;
  # MCP argument name, empty when the tool deliberately does not take it.

  socket @2 :Text;
  # Control-socket parameter name, empty when the method does not take it.

  note @3 :Text;
  # Why a surface is empty, or why the names differ. Empty when they agree.
}


const operations :List(Operation) = [
  ( cli = "create", socket = "issue/create", mcp = "vissue_create", mutates = true,
    note = "",
    fields = [
      ( cli = "project", tool = "project", socket = "project", note = "" ),
      ( cli = "priority", tool = "priority", socket = "priority", note = "" ),
      ( cli = "type", tool = "issue_type", socket = "issue_type", note = "the flag cannot be a Rust field of that name, which is a keyword" ),
      ( cli = "tags", tool = "tags", socket = "tags", note = "" ),
      ( cli = "parent", tool = "parent", socket = "parent", note = "" ),
      ( cli = "body", tool = "body", socket = "body", note = "" ),
      ( cli = "deadline", tool = "deadline", socket = "deadline", note = "" ),
      ( cli = "scheduled", tool = "scheduled", socket = "scheduled", note = "" ),
      ( cli = "body-file", tool = "", socket = "", note = "no tool argument: a path resolves on the host running the server, not the caller's" ),
      ( cli = "", tool = "", socket = "agent", note = "socket only: it overrides the identity the connection was opened with, which the other surfaces take from the environment" )
    ] ),
  ( cli = "update", socket = "issue/update", mcp = "vissue_update", mutates = true,
    note = "",
    fields = [
      ( cli = "", tool = "issue_id", socket = "id", note = "the issue being acted on: positional on the command line, and the two remote surfaces spell it differently" ),
      ( cli = "state", tool = "state", socket = "state", note = "" ),
      ( cli = "priority", tool = "priority", socket = "priority", note = "" ),
      ( cli = "block", tool = "block", socket = "block", note = "" ),
      ( cli = "unblock", tool = "unblock", socket = "unblock", note = "" ),
      ( cli = "if-state", tool = "if_state", socket = "if_state", note = "" ),
      ( cli = "if-gen", tool = "if_gen", socket = "if_gen", note = "" ),
      ( cli = "", tool = "", socket = "agent", note = "socket only: it overrides the identity the connection was opened with, which the other surfaces take from the environment" )
    ] ),
  ( cli = "claim", socket = "issue/claim", mcp = "vissue_claim", mutates = true,
    note = "",
    fields = [
      ( cli = "", tool = "issue_id", socket = "id", note = "the issue being acted on: positional on the command line, and the two remote surfaces spell it differently" ),
      ( cli = "force", tool = "force", socket = "force", note = "" ),
      ( cli = "", tool = "", socket = "agent", note = "socket only: it overrides the identity the connection was opened with, which the other surfaces take from the environment" )
    ] ),
  ( cli = "note", socket = "issue/note", mcp = "vissue_note", mutates = true,
    note = "",
    fields = [
      ( cli = "", tool = "issue_id", socket = "id", note = "the issue being acted on: positional on the command line, and the two remote surfaces spell it differently" ),
      ( cli = "", tool = "text", socket = "text", note = "the issue being acted on: positional on the command line, and the two remote surfaces spell it differently" )
    ] ),
  ( cli = "append", socket = "issue/append", mcp = "vissue_append", mutates = true,
    note = "",
    fields = [
      ( cli = "", tool = "issue_id", socket = "id", note = "the issue being acted on: positional on the command line, and the two remote surfaces spell it differently" ),
      ( cli = "text", tool = "text", socket = "text", note = "" ),
      ( cli = "file", tool = "", socket = "", note = "no tool argument: a path resolves on the host running the server, not the caller's" ),
      ( cli = "", tool = "", socket = "agent", note = "socket only: it overrides the identity the connection was opened with, which the other surfaces take from the environment" )
    ] ),
  ( cli = "refile", socket = "issue/refile", mcp = "vissue_refile", mutates = true,
    note = "",
    fields = [
      ( cli = "", tool = "issue_id", socket = "id", note = "the issue being acted on: positional on the command line, and the two remote surfaces spell it differently" ),
      ( cli = "to", tool = "to", socket = "to", note = "" )
    ] ),
  ( cli = "reject", socket = "issue/reject", mcp = "vissue_reject", mutates = true,
    note = "",
    fields = [
      ( cli = "", tool = "issue_id", socket = "id", note = "the issue being acted on: positional on the command line, and the two remote surfaces spell it differently" ),
      ( cli = "to", tool = "to", socket = "to", note = "" ),
      ( cli = "project", tool = "project", socket = "project", note = "" ),
      ( cli = "reason", tool = "reason", socket = "reason", note = "" )
    ] ),
  ( cli = "resolve", socket = "issue/resolve", mcp = "vissue_resolve", mutates = true,
    note = "",
    fields = [
      ( cli = "", tool = "issue_id", socket = "id", note = "the issue being acted on: positional on the command line, and the two remote surfaces spell it differently" ),
      ( cli = "", tool = "state", socket = "state", note = "the issue being acted on: positional on the command line, and the two remote surfaces spell it differently" )
    ] ),
  ( cli = "vote", socket = "issue/vote", mcp = "vissue_vote", mutates = true,
    note = "",
    fields = [
      ( cli = "", tool = "issue_id", socket = "id", note = "the issue being acted on: positional on the command line, and the two remote surfaces spell it differently" ),
      ( cli = "for", tool = "choice", socket = "choice", note = "the flag cannot be a Rust field of that name, which is a keyword" ),
      ( cli = "", tool = "", socket = "agent", note = "socket only: it overrides the identity the connection was opened with, which the other surfaces take from the environment" )
    ] ),
  ( cli = "fold", socket = "issue/fold", mcp = "vissue_fold", mutates = true,
    note = "",
    fields = [
      ( cli = "", tool = "file", socket = "file", note = "the issue being acted on: positional on the command line, and the two remote surfaces spell it differently" ),
      ( cli = "project", tool = "project", socket = "project", note = "" )
    ] ),
  ( cli = "normalize", socket = "issue/normalize", mcp = "", mutates = true,
    note = "no tool: rewriting every heading in a corpus is not a thing to hand an agent",
    fields = [
      ( cli = "project", tool = "", socket = "project", note = "the verb has no tool" ),
      ( cli = "dry-run", tool = "", socket = "dry_run", note = "the verb has no tool" )
    ] ),
];
