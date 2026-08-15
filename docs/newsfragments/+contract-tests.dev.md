The seams between the parts have tests of their own. The client and the
server now talk to each other over a real socket, which is where
a mutation the server accepted but did not make visible to the next read
used to hide; the Model Context Protocol (MCP) server is driven the way
an agent meets it, a process on the other end of a pipe; `serve -d` runs
as a real daemon; and Org decides whether a markdown body stayed body,
rather than the parser deciding that about itself.

Concurrent writers are covered: with the advisory lock removed, twelve
overlapping `create` calls lose between two and six of themselves while
every writer reports success, and nothing in the suite noticed before.

Three drift guards fail rather than rot: every command and MCP tool must
appear in the reference, every advertised control capability must be
routed, and the release must cover every publishable crate in an order
taken from the manifests.

`cargo test` runs with `--no-fail-fast`, so a red run names every broken
target instead of stopping at the first.
