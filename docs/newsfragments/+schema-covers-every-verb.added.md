The schema covers every subcommand, not only the mutating ones, and a test refuses a
subcommand it does not mention.

Until now the schema constrained the verbs it already listed. A brand-new subcommand
failed nothing, because nothing asked whether the schema knew about it, which is
exactly how the earlier gaps arrived: `vote` was added to the command line and no test
anywhere had an opinion.

Read verbs are held to the surfaces they claim rather than to all of them, because
fourteen of them are genuinely absent from the socket today and the schema says so
instead of pretending otherwise.

Three things had to be modelled that the mutating-only version never met. Aliases,
because `create` answers to `q` and the new check found it and could not tell it from
a verb. Local-only verbs, because the terminal UI and shell completions have no
remote surface and are not gaps. And a socket method answering for two subcommands,
which `identity/get` does for `identity` and `whoami`, allowed when the rows say so
and refused in silence.

The committed generated file is now checked against the schema text as well. Editing
`vissue.capnp` and forgetting to regenerate left every check validating the previous
schema and passing, so the schema was authoritative in the documentation only.
