`issue/digest` reports the event-log generation, which `digest --json` has always
carried and the method omitted.

A client comparing digests across time needs to know which generation each was taken
at, and had no way to see the field was missing. Found by comparing the structured
reads against the command line's `--json` mode, the same way the report-producing reads
are compared.

One divergence is intended and is now pinned as such. `issue/list` sorts by priority,
then state, then id across every project in the layout, while the `list` subcommand
applies that order within each project and concatenates, because it can span several
layouts and a global sort across them would interleave two trackers. Same rows,
different sequence, and the terminal UI relies on the socket's. The test compares the
rows as sets and says why.
