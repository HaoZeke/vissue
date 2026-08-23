Every read that answers with a report is compared against its subcommand: `export`,
`graph`, `roadmap`, `cycles`, `count`, `check`, `stale`, `hygiene` and `waiting-on`.

Reading the handlers was not a mechanism. `issue/mirror` answered with the corpus
digest while every name and type check agreed with it, and it was found by eye.

Nine agree byte for byte. `ping` cannot: it appends to the event log, so two calls
report two sequence numbers, and equality is the wrong instrument for a read with a
side effect. Its shape is asserted instead, and the schema row says why it changes
something while remaining a non-mutating verb: it changes no issue.

`stale` defaults to thirty days on the command line and has no default on the socket,
so the test passes the number to both. A default that differs between surfaces is
itself a divergence, and leaving it implicit would have hidden one.
