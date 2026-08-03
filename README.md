# vissue

Issue tracking in plain orgmode files, one file per project, with a CLI and an
MCP server over the same library.

An issue is a top-level org heading. The file is the database: no daemon, no
SQLite, no server. Every command parses the files it needs and every mutation
rewrites one file under a lock, so the tracker diffs, merges, and greps like the
rest of the repository it lives in.

```
<root>/Software/<project>/issues.org
```

`Software` is the default prefix and is configurable. `<root>` comes from
`--root`, the `VISSUE_ROOT` environment variable, or the current directory.

## Install

```console
$ cargo install --git https://github.com/HaoZeke/vissue vissue-cli
```

The MCP server is a second binary in the same workspace:

```console
$ cargo install --git https://github.com/HaoZeke/vissue vissue-mcp
```

## Tutorial: from an empty directory to a tracked backlog

Everything below runs in a scratch directory and touches nothing else.

**1. Make a tracker and create the first issue.**

```console
$ mkdir -p /tmp/demo && cd /tmp/demo
$ vissue create --project parser "Reject a manifest with no header"
parser-k29f  TODO  [#C]  Reject a manifest with no header
file: /tmp/demo/Software/parser/issues.org
```

The file now exists, with a preamble and one heading:

```console
$ cat Software/parser/issues.org
#+TITLE: parser issues
#+FILETAGS: :issues:parser:
#+DATE: [2026-08-03 Mon]
#+DESCRIPTION: Issue tracking file for parser specs, plans, and implementation tasks.
#+STATUS: Active
#+TODO: TODO STARTED BLOCKED | DONE CANCELLED

* TODO [#C] Reject a manifest with no header
:PROPERTIES:
:ID:         parser-k29f
:CREATED:    [2026-08-03 Mon]
:END:
```

**2. Add a second issue that depends on the first.**

```console
$ vissue create --project parser --priority A \
    --body "Scope: the error message quoted in the release notes." \
    "Publish the release notes"
parser-3xq7  TODO  [#A]  Publish the release notes
file: /tmp/demo/Software/parser/issues.org

$ vissue update parser-3xq7 --block parser-k29f
parser-3xq7: state TODO -> BLOCKED (auto on block), blocked_by += parser-k29f
```

Adding a blocker moved the issue to BLOCKED on its own, and the transition is in
the logbook.

**3. Ask what is actually workable.**

```console
$ vissue ready
parser-k29f            TODO      [#C]  Reject a manifest with no header
```

The blocked issue is gone from the list, which is the whole point of `ready`.

**4. Work the issue and close it.**

```console
$ vissue claim parser-k29f
claimed parser-k29f
parser-k29f: state TODO -> STARTED
ID:       parser-k29f
...
$ vissue update parser-k29f --state DONE
parser-k29f: state STARTED -> DONE
[hint] parser-3xq7 (in parser) lists this as a blocker; clear with `vissue update parser-3xq7 --unblock parser-k29f`
```

Closing a blocker names every issue still waiting on it.

**5. Clear the edge and see the backlog open up.**

```console
$ vissue update parser-3xq7 --unblock parser-k29f
parser-3xq7: state BLOCKED -> TODO (auto on unblock), blocked_by -= parser-k29f
$ vissue ready
parser-3xq7            TODO      [#A]  Publish the release notes
```

**6. Share the backlog with someone who cannot read your tracker.**

```console
$ vissue mirror --project parser --out /tmp/demo/backlog-mirror.org
wrote /tmp/demo/backlog-mirror.org
```

That file is a read-only projection with a banner saying so. Regenerate it after
changes; it is output, not a second source of truth.

## How to

**Filter and count.** `--project` (also `-p` or `-P`) and `--state` apply to
`list`, `count`, `ready`, `graph`, `roadmap`, and `export`.

```console
$ vissue list --project parser --state TODO
$ vissue count --ready
```

**Feed another tool.** `export` writes one JSON object per line, carrying every
property, the logbook, the body, and the file line range.

```console
$ vissue export --project parser | jq -r '.id + " " + .state'
```

**Follow the graph.** `tree` walks children and blockers below an id; `graph`
emits the whole thing as Graphviz DOT; `backlinks` finds everything pointing at
an id; `cycles` reports a blocker loop.

```console
$ vissue tree parser-3xq7
$ vissue graph --project parser | dot -Tsvg > backlog.svg
```

**Keep the corpus honest.** `check` validates every parent and blocker edge,
every date, and the uniqueness of ids, and exits non-zero on an error.
`hygiene` adds the claims that are not actually workable.

```console
$ vissue check
$ vissue hygiene
```

**Tell whether a copy of the backlog is current.** `digest` hashes the corpus,
combined and per project, so a consumer can compare two points in time without
reading every issue:

```console
$ vissue digest -P ebstack -P surf
combined=7f91ad67512010d0 issues=109 generation=3167 projects=2
6cdab6af46e1c979      12  ebstack
671d99c6181c1494      97  surf
$ vissue digest -P ebstack --quiet
d2ee07c7f585330b
```

The per-project lines are the point: a changed combined digest says something
moved, and the sub-digests say which project. The hash is xxh3 over the JSONL
export, so it tracks issue content and ignores formatting that changes nothing.
`--json` gives the same data as an object.

Every mirror carries that digest in its header as a SYNC stamp:

```
# SYNC: digest=d2ee07c7f585330b generation=3167 issues=12 at=2026-08-03T09:53 projects=ebstack:6cdab6af46e1c979
```

Which makes freshness a single command, exiting 0 when current and 1 when not:

```console
$ vissue mirror --check Software/ebstack/issues-mirror.org
fresh: digest=d2ee07c7f585330b issues=12 generation=3167 (stamped 2026-08-03T09:53)

$ vissue mirror --check stale-copy.org
stale: stale-copy.org
  stamped digest=0000000000000000 at=2026-08-03T09:53 issues=12
  current digest=d2ee07c7f585330b issues=12 generation=3167
  moved: ebstack 1111111111111111 -> 6cdab6af46e1c979
```

The check reads the projects from the stamp, so a caller need not repeat them,
and a mirror with no stamp reads as stale. This is what lets a collaborator who
cannot reach the tracker still know whether the copy they hold is behind: run
the check where the tracker lives and share the verdict, or compare stamps
across pulls on their side.

**See who is holding what.** Claiming an issue stamps an identity and a
timestamp onto it, so a backlog worked by several agents shows who took what
and when:

```console
$ vissue whoami
rgoswami@workstation
$ VISSUE_AGENT=grind-worker-3 vissue claim parser-k29f
claimed parser-k29f by grind-worker-3 (TODO -> STARTED)
$ vissue list --state STARTED
parser-k29f            STARTED   [#C]  Reject a manifest…  (claimed 0d by grind-worker-3)
```

The identity comes from `VISSUE_AGENT`, then `agent` in `vissue.toml`, then
`user@host`. It is an opaque string: an agent should set `VISSUE_AGENT` to
something stable enough to recognise across sessions, such as a model and
session tag.

Moving to STARTED by any route takes the claim if no one holds it, so
`vissue update <id> --state STARTED` stamps it too. BLOCKED keeps the claim,
because the holder is still on the issue. Returning to TODO, or closing as
DONE or CANCELLED, gives it up and writes a logbook note naming who held it
and since when, so the history outlives the properties. A claim held by
another identity is refused unless you pass `--force`, which records the
takeover rather than losing it.

`hygiene` reports claims held longer than `stale_claim_days` (default 7,
settable in `vissue.toml` or per run with `--stale-days`), and STARTED issues
that nobody has claimed.

**Watch for changes without re-reading everything.** A write advances a
generation counter and appends to a log, both beside the project directories.
A poller compares the counter, then reads only what is new:

```console
$ vissue gen
3167
$ vissue events --since 3155 -n 5
$ vissue wait --last 3167 --timeout-ms 30000   # exits 2 on timeout
$ vissue ping --detail "external change"       # wake pollers by hand
```

Set `VISSUE_EVENTS=0` to suppress emission when the tracker must stay
untouched.

**Point at a different layout.** Use `--prefix`, `VISSUE_PREFIX`, or a
`vissue.toml` at the root:

```toml
prefix = "projects"

[issues]
default_priority = "B"
id_length = 5
```

**Detect the project from the working directory.** With a `.project-ctx.toml`
carrying `[project] name = "..."` in or above the current directory, `--project`
may be omitted.

**Know which tracker you are writing to.** `create`, `update`, `claim`, and
`refile` all write, and claiming counts as a write because it stamps the
holder onto the issue. When a wrapper or the environment sets `VISSUE_ROOT`,
a bare `vissue` writes to that tracker from any directory. Pass `--root`
explicitly for a scratch tracker, and `vissue identity` reports which binary
and which root are in play before you commit to a mutation.

## Reference

### Commands

| Command | Purpose |
|---|---|
| `create`, `q` | Add an issue; `q` prints only the new id |
| `list`, `show` | Rows of issues; one issue's metadata and file range |
| `update`, `claim`, `refile` | Change state, priority, or blockers; take an issue; move it between projects |
| `whoami` | The identity a claim would record |
| `ready`, `count`, `search`, `children`, `stale` | Query the corpus |
| `export` | JSONL, one object per issue |
| `tree`, `graph`, `cycles`, `backlinks` | Relationships |
| `roadmap`, `mirror` | Markdown roadmap; read-only org or markdown projection |
| `digest`, `mirror --check` | Corpus digest; whether a mirror is still current |
| `check`, `hygiene` | Validation |
| `gen`, `events`, `wait`, `ping` | Change stream for pollers |
| `projects`, `identity` | Layout introspection |

### States and priorities

States are `TODO`, `STARTED`, `BLOCKED`, `DONE`, and `CANCELLED`. An issue is
*ready* when it is `TODO` or `STARTED` and no id in its `:BLOCKED_BY:` is still
open. Priorities are the org cookies `[#A]`, `[#B]`, and `[#C]`, defaulting
to `C`.

### Properties

`:ID:` is required and generated. `:CREATED:` is set on create. `:PARENT:`,
`:BLOCKED_BY:`, `:TAGS:`, `:TYPE:`, `:DEADLINE:`, `:SCHEDULED:`,
`:CLAIMED_BY:`, `:CLAIMED_AT:`, and `:DISCOVERED_FROM:` are read by the query
verbs; any other property is preserved
untouched, in the order it appears on disk. `:BLOCKED_BY:` accepts commas,
whitespace, or both. `:PARENT:` may name another issue or any org heading with
an `:ID:` under the tracker prefix, so a design document can head a work
hierarchy.

### JSONL export schema

Each line is an object with `id`, `project`, `title`, `state`, `priority`,
`properties`, `logbook`, `body`, `line_start`, and `line_end`. Logbook entries
carry `timestamp`, `from`, `to`, and `note`.

### MCP server

`vissue-mcp` speaks MCP over stdio and calls the library in process. It resolves
its root from `VISSUE_ROOT` and `VISSUE_PREFIX`. Tools mirror the CLI verbs:
`vissue_list`, `vissue_ready`, `vissue_show`, `vissue_create`, `vissue_update`,
`vissue_claim`, `vissue_count`, `vissue_search`, `vissue_children`,
`vissue_backlinks`, `vissue_waiting_on`, `vissue_body_excerpt`, `vissue_tree`,
`vissue_graph`, `vissue_roadmap`, `vissue_export`, `vissue_check`,
`vissue_hygiene`, `vissue_mirror`, `vissue_projects`, and `vissue_identity`.

## Explanation

### Why the file is the database

A tracker that owns its own store forces a sync problem on everyone who also
wants the data in git. Keeping the org file authoritative removes that problem:
review happens in the diff, history comes from the log, and any editor that
speaks orgmode is a client. The cost is a linear scan per command, which is the
right trade until an issue count gets large enough to notice.

### Why `show` does not print the body

`show` returns metadata and `file:line_start-line_end`. An editor or an agent
opens that range when it wants the prose. Keeping prose out of the command
output is what stops a status check from turning into a wall of text.

### Concurrency

Every read-modify-write cycle takes a process-local mutex and an advisory lock
on `issues.org.lock`, then writes through a uniquely named temporary and
renames. Concurrent creates from several processes therefore neither lose
headings nor collide on the temporary file.

## Contributing

Conventional commits, `cargo fmt`, and `cargo clippy -- -D warnings`. Run
`prek install` for the hooks. The minimum supported Rust version is 1.88.

## Citation

See [CITATION.cff](CITATION.cff).

## License

MIT. See [LICENSE](LICENSE).
