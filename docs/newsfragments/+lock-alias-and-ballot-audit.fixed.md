A read-modify-write cycle over several files now counts a file once however its
path was spelled, and four correctness fixes land in `vote`.

The lock helper deduplicated by the path as written while keying its mutex on the
resolved path. Two names for one file therefore took the same non-reentrant mutex
twice and hung, and took a second advisory lock on the same file besides. Latent
while callers passed one or two known-distinct files; reachable now that a mint
locks every twin file for a project, where two configured roots can be links to
one tree.

In `vote`:

- A single ballot is no longer called a consensus. One agent agreeing with itself
  is not agreement, and reporting it as such is how one unreviewed opinion gets
  acted on as though it had been checked.
- An identity containing a colon and a space is refused. A choice may contain one,
  which is why the ballot line splits on the first, so such an identity would have
  been read back as a shorter name with the rest of itself attached to the choice:
  the ballot filed under an agent that never voted, silently.
- Lines the parser does not recognise are kept. The drawer is org a person can
  edit, and a rewrite keeping only recognised lines ate a comment left there.
- Two hand-written lines for one agent collapse to the last, so a duplicate cannot
  make one voter count twice.

The votes drawer is also rewritten in place rather than removed and appended, so a
vote no longer reorders the other drawers on the heading.
