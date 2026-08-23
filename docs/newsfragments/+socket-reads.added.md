Thirteen read verbs gained control-socket methods: `issue/check`, `issue/count`,
`issue/cycles`, `issue/digest`, `issue/export`, `issue/graph`, `issue/roadmap`,
`issue/stale`, `issue/hygiene`, `issue/waiting_on`, `issue/mirror`, `events/ping`
and `events/wait`.

A socket client used to shell out for all of them. That cost a subprocess rather
than correctness, which is why it outlived the write gap, but `events/wait` is the
case that made it worth closing: a verb whose whole job is to block until something
changes is exactly what a connection is good at and a subprocess is bad at.

Reads that produce a report answer with `report`, the same text the subcommand
prints, because inventing a structure per report would be a second contract to keep
in step with the first. `issue/check` carries `errors` and `warnings` beside it,
since the subcommand exits non-zero on an error count and a client needs that signal
without parsing prose. `issue/digest` and `events/wait` answer with fields.
