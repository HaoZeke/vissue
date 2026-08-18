The `issues.org` parser now follows Org 9.8 on the constructs that
used to fail a file or split an issue: greater and dynamic blocks are
literal, a planning line accepts timestamp ranges and repeaters,
drawers may arrive in any order, `COMMENT` and notes headings stay
Org rather than becoming a missing `:ID:`, and file-local `#+TODO:`
keywords are recognised. See `docs/orgmode/org-syntax.org`.
