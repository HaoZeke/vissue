`vissue normalize` rewrites a tracker onto the Org and ELPA
property names the other tools already read: `#+CATEGORY:`, type
as a heading tag, `:BLOCKED_BY:` mirrored to org-edna
`:BLOCKER: ids(...)`, and typos (`BLOCKEDBY`, drawer `TAGS`)
folded. `--dry-run` prints the files that would change. An edna
condition that is not `ids(...)` is left alone.
