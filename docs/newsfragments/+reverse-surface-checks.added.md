Two checks now run surface to schema, not only schema to surface: every tool the
server exposes and every method it dispatches must appear in a schema row.

Every existing check ran one way, catching a verb the schema names and the surface
lacks. Nothing caught the reverse, so a tool that existed and the schema omitted was
invisible to all of them — the same asymmetry that let verbs drift in the first place.

It had already bitten three times. The schema claimed `normalize` had no tool while
`vissue_normalize` sat in the server, and `vissue_org` and `vissue_mirror_check` were
in no row at all. Every check passed throughout.

The last two needed a shape the schema could not express: a tool whose command-line
form is a flag on another verb rather than a verb of its own, `show --org` and
`mirror --check`. Such a row carries no subcommand and says so, and the uniqueness
checks were treating two absent names as a collision.
