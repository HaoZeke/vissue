`vissue show` prints the body, and `show --org` writes the heading out whole
(property drawer, logbook, and body) for handing an issue to someone as the
thing to work from. `IssueDetail` carries the body, so `--json`, the MCP
tools, and the control protocol answer with what the issue asks for rather
than a file path and a line range. Prefer `show --org` over `body-excerpt`
when the text is the specification: the excerpt is a preview and stops at 40
lines. The MCP tool is `vissue_org`.
