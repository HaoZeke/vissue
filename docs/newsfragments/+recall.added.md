`vissue recall <id>` prints the working set for an issue: the plan it sits in,
the deeds produced by what blocks it, the issue it was bounced from, and what
it has produced itself. Read it before starting work on a node.

It walks `:PARENT:`, `:BLOCKED_BY:`, and `:DISCOVERED_FROM:` rather than
ranking the corpus by resemblance, so it is what the plan says the work stands
on. `related` still answers the resemblance question. `--deeds-only` prints the
accessions one per line, for `deedar get $(vissue recall <id> --deeds-only)`.
A `:PARENT:` that names a design document rather than an issue is named in the
plan too, marked as a heading outside the tracker, because that document is what
the reader should open.

The blocker walk is one hop by default, because a deed records its own sources
and `deedar trail` walks them; `--depth` widens it.
