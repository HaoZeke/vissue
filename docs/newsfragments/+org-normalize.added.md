`vissue normalize` rewrites a tracker onto the Org and ELPA
property names the other tools already read: `#+CATEGORY:`, type
as a heading tag, typos (`BLOCKEDBY`, drawer `TAGS`) folded, and
a bare `:BLOCKER:` id list moved to `:BLOCKED_BY:`. `--dry-run`
prints the files that would change. An org-edna condition stays.
vissue does not mint `:BLOCKER: ids(...)`.
