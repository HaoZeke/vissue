`vissue check` no longer reads the word "rejected" anywhere in a body as an
issue that was rejected, and no longer asks for a `DISCOVERED_FROM` edge between
two issues a `:PARENT:` or blocker edge already connects.

Both were substring rules that do not match how issues get written. Every bug
about input validation says "rejected" — "silently corrupted rather than
rejected", "a hand-written parser is rejected as strictly dominated" — and three
worked-and-closed issues in one corpus were flagged for exactly those sentences.
A rejection is now recognised by the shapes one is written in: the tool's own
phrasing, "superseded by", "rejected in favour of", or a heading that names the
outcome. And a parent naming its child is a stated relation the tracker already
holds, so it is no longer a warning that can only be answered with a wrong edge.

The mention warning is narrower on both sides now: the edge may be `:PARENT:` or
`:BLOCKED_BY:` as well, and the prose near the link has to claim a discovery or a
pivot. A body links other issues for every reason there is, and in one corpus
twenty-two such warnings stood and not one of them was a discovery.
