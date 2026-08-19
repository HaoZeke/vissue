`#+PRIORITIES: highest lowest default` sets the cookie range and the
value used when a heading has no `[#X]`. A fresh file writes
`#+PRIORITIES: A C C`. `create` without `--priority` uses that
default; when the file has no such line it uses
`issues.default_priority`. A cookie outside the range is refused.
