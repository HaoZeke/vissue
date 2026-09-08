The explanation page now says why a working set has no poisoning surface: nothing
an agent reads becomes part of one. An entry is there because an issue declares
`:BLOCKED_BY:`, `:PARENT:`, or `:DISCOVERED_FROM:` naming it, and those are
written only by a tracker mutation under the lock, by a named identity, with a
logbook line. `fold` ingests a file into issues carrying a title and a body and
no edges at all.
