The schema records the parameter each of six methods is actually about. `issue/create`
took `title`, `issue/search` took `query`, and `issue/tree`, `issue/ancestors`,
`issue/impact` and `issue/related` took an id, none of which appeared in any row:
`Field` was written around flags, and these arrive as positional arguments on the
command line. The tool spells the id `issue_id` and the socket spells it `id`, which
is the kind of divergence the schema exists to record and did not.

Fields also carry `omittable`, for a parameter a caller may leave out whatever the
Rust type says. `force`, `dry_run`, `last` and `projects` are plain types behind
serde's `default`, so they read as required while every caller omits them.
