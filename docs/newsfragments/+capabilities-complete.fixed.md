`initialize` advertises every method the server answers. It was nineteen behind.

The capability list is the fourth place the method set is written down, after the
dispatch table, the schema and the reference, and it is the one a client reads to
decide what it may call. While the other three agreed with each other this one fell
behind, so a client inspecting capabilities would have concluded that `append`,
`vote`, `fold` and every read added beside them did not exist.

A test now holds it to the schema in both directions, since either way round is a lie:
advertising a method that is not dispatched sends a client at a method-not-found, and
dispatching one that is not advertised hides it from anyone who asks first.

The `Method` enum grew the same nineteen. Two exhaustive matches then refused to
compile, which is the arrangement working: the typed request and response helpers have
no form for these, and they now say so per method rather than through a wildcard,
because a wildcard would swallow the next one silently.

`control.org` said `issue/check`, `issue/export`, `issue/graph`, `issue/mirror`,
`issue/roadmap`, `issue/hygiene` and `issue/fold` were "not in v1" and stayed on the
command line and MCP. All seven are methods now, and the page lists every one.
