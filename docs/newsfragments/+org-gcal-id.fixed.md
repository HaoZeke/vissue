An org-gcal `:ID:` of the form `<event>/<calendar>` is not an issue
id. The heading stays in the file around the real issues.
`find_org_ids` and `collect_org_ids` skip slash ids. `check` errors
if a parsed issue still carries one.
