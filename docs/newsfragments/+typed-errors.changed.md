Library crates return typed errors instead of `anyhow::Result`.
`vissue-core` exposes `Error` and `Result`; `vissue-tui` and
`vissue-serve` do the same. The CLI, MCP server, and HUD still
use anyhow at the process edge. Match on the enum; do not parse
the display text.
