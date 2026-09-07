`vissue deed <id> --add <accession>` cites the deeds a unit of work produced,
in a `:DEEDS:` property on the heading. A deed is
[deedar](https://github.com/indynull/deedar)'s frozen record of a product, and
the accession is the whole handoff: the next unit runs `deedar get` on it
instead of rereading a transcript. The tracker cites and stores no product
bytes, so a value that is not an accession (`deed-<kind>-<slug>`, or a
`sha256:` of the deed or of one product path) is refused rather than stored as
a citation that resolves to nothing.
