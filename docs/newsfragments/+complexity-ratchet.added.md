`clippy::cognitive_complexity` is enabled with the threshold held at today's ceiling in
`clippy.toml`. A ratchet rather than a target: it blocks a function growing past the
worst one already here, and lowering it is the work of splitting those.

The comment there records what the measure does not see. The control socket dispatches
thirty-seven methods from one match and scores under clippy's default, because the
metric counts nesting and a wide flat match has none. Drift between the surfaces lives
in exactly that breadth, which is why the schema checks catch it and this never would.
