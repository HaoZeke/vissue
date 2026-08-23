`check` reports a heading whose `:ID:` belongs to an org-gcal event. It counted those
among the parsed headings, and the loader skips them precisely because a calendar sync
owns them, so the count was always zero. Counting the file instead reports what is
actually there, which matters because such a heading sits in the tracker and vissue
will not touch it.
