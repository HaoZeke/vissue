# Changelog

All notable changes to vissue are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and releases use
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

<!-- towncrier release notes start -->

## [0.4.0](https://github.com/HaoZeke/vissue/releases/tag/v0.4.0) - 2026-08-16

### Changed

- Library crates return typed errors instead of `anyhow::Result`.
  `vissue-core` exposes `Error` and `Result`; `vissue-tui` and
  `vissue-serve` do the same. The CLI, MCP server, and HUD still
  use anyhow at the process edge. Match on the enum; do not parse
  the display text.
- The workspace uses Rust edition 2024. The published MSRV is still 1.89.


## [0.3.0](https://github.com/HaoZeke/vissue/releases/tag/v0.3.0) - 2026-08-15

### Added

- `vissue append <id>` records a dated, attributed report under an issue's
  heading, from `--text`, a `--file`, or stdin. A body could previously only be
  set at `create`, and `note` folds its text to a single line by design, so
  there was no way to write down the result of work. Markdown is safe. The MCP
  tool is `vissue_append`.
- `vissue hud` is a summonable task board. It opens on the project list;
  entering a project shows that project's ready forest, then List / Claims /
  Agenda / Search within it, with show / excerpt / tree / related / notes on
  the selected row. The window execs a separate `vissue-hud` binary, so the
  CLI does not carry the GUI dependencies.

  First paint reads the files. Unless `--offline`, the board attaches to
  `vissue serve`, starting it when the socket is free, and falls back to the
  files when serve is down or bound to another root. `--toggle`, `--show`
  and `--hide` talk to a summon socket, so a keybinding can raise the board
  that is already running rather than starting a second one.

  `--rofi` gives the seat dmenu picker instead, over the set named by
  `--mode`: Return opens the heading in `$EDITOR`, Alt+c claims, Alt+n
  notes. Keys come from a catalog that `~/.config/vissue/keys.toml` or
  `VISSUE_KEYS` remaps; `vissue keys` prints it and `--check` validates an
  overlay.
- `vissue serve` now caches the issue catalog and answers the v1 control
  methods (`issue/list`, `issue/ready`, `issue/claim`, and the rest).
  Attached clients receive `vault/changed` after a rebuild. The files stay
  the store: stopping serve loses nothing.
- `vissue show` prints the body, and `show --org` writes the heading out whole
  (property drawer, logbook, and body) for handing an issue to someone as the
  thing to work from. `IssueDetail` carries the body, so `--json`, the MCP
  tools, and the control protocol answer with what the issue asks for rather
  than a file path and a line range. Prefer `show --org` over `body-excerpt`
  when the text is the specification: the excerpt is a preview and stops at 40
  lines. The MCP tool is `vissue_org`.
- `vissue tui` is an interactive board over ready, list, claims, agenda,
  and search. First paint reads the files. Unless `--offline`, the board
  attaches to `vissue serve` (starting it when the socket is free) and
  falls back to the files when serve is down or bound to another root.

### Changed

- Publication uses crates.io trusted publishing on a `v*` tag, with no registry secret in the repository.
- The minimum supported Rust version is 1.89, which is what the iced board's
  dependencies require.
- `check` only searches the wider tree for `:PARENT:` ids that are not
  issues. It validates parents against any Org id, which includes design
  documents and notes, so on a tracker sharing a root with a notes vault it
  read every `.org` file to answer a question the issues had usually
  already answered. Where every parent is another issue the scan is now
  skipped entirely: 58ms to 23ms on a 35MB corpus. A tracker that does
  point at a design document still pays for the search, which now stops as
  soon as the ids it wants have been found.

### Fixed

- A body containing a line that starts with `* `, which any markdown bullet
  list does, no longer cuts the issue in two. The parser splits issues on that
  line, so such a body used to leave a heading with no `:ID:`, stop the file
  parsing, and drop every issue in that project out of `list`. Those lines are
  now indented by one on the way out; `** Scope` and deeper are left alone,
  being children of the issue rather than the end of it.
- A client of `vissue serve` sees its own write. The catalog was rebuilt only
  by the file watcher, so for up to about 450ms after a mutation the writer was
  answered from the catalog as it stood before: `issue/list` came back one
  short, and `issue/get` on the id `issue/create` had just returned reported
  the issue as missing. The mutation path refreshes before it answers and
  reports the resulting revision.
- An idle tracker no longer rebuilds its catalog and broadcasts `vault/changed`
  several times a second. The generation poll opens the projects directory on
  every tick, inotify reports that open against the watched tree, and a read
  was counted as a change, so the rebuild's own reads raised the next event.
- `check` now reads `:ID:` only from a property drawer under a heading, where
  org defines one. A report appended to an issue body quotes the heading it
  describes, and a quoted id used to count: a `:PARENT:` pointing at nothing
  resolved against the mention, and the check that exists to catch broken links
  went quiet.
- `digest` and `mirror --check` read the corpus once rather than once per
  project. Each project's digest came from a whole-corpus export filtered
  down to that project, which is quadratic in the project count: on a
  tracker with 115 projects and 4781 issues those commands took 6.2s, and
  now take 0.16s. The digest values are unchanged, so mirrors stamped by an
  earlier version still read as fresh.

### Developer

- The catalog query surface and the typed error enum are covered by tests.
- The seams between the parts have tests of their own. The client and the
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


## [0.2.0](https://github.com/HaoZeke/vissue/releases/tag/v0.2.0) - 2026-08-14

### Added

- A documentation site at [vissue.rgoswami.me](https://vissue.rgoswami.me), written as Org under `docs/orgmode/` and rendered with Sphinx. The pages keep to one Diataxis quadrant each, and the README is a front door rather than the whole manual. The site is a managed Cloudflare Pages target, deployed with `ttech-ops site deploy vissue`.
- Logo mark: a tessellated V of issue nodes with one ready cell, plus a circular crest and wordmark lockup under `assets/`.
- MCP tools for `ancestors`, `impact`, `cycles`, `refile`, `wait`, and `whoami`. `identity` also emits `root=` / `prefix=` tokens.
- `vissue completions <shell>` and `vissue man`, generated from the CLI definition so they cannot drift, with copies committed under `completions/` and `man/`.

### Changed

- **Compatibility**: a tracker written by this version is not readable by 0.1.0, which fails on the planning line with ":ID: property missing". Upgrade every reader of a tracker together. A second tool that writes an `issues.org` also needs to place a `:LOGBOOK:` drawer below the planning line, or it breaks the file for every reader including `vissue`.
- A new project file carries `#+CATEGORY: <project>`. Org otherwise takes the agenda category from the file name, and every project's file is `issues.org`, so a multi-project agenda labelled every row `issues`.
- Dates and tags move to where Org keeps them. `DEADLINE`, `SCHEDULED`, and `CLOSED` render on the planning line under a heading, and a tag Org can hold renders in the heading's own tag run, so `org-agenda`, Org tag search, and `org-lint` all work against a tracker. Both shapes parse, so existing files read the same and settle on the Org shape when next rewritten.
- JSONL export and `show --json` carry `org_tags` and a `tags` union.
- The README is a front door rather than the manual: what vissue is, how to install it, a minute of it working, and links into the documentation site. The tutorial, how-to, reference, and explanation move to their own pages.
- The tag property is `:VISSUE_TAGS:`. `TAGS` is a name Org reserves and `org-lint` reports; a drawer written under the old name is migrated on parse.
- `tests/org_interop.sh` drives Emacs over a tracker in CI: org-lint, org-agenda, tag search, and org-id all have to agree, and Org's own edits have to survive a vissue rewrite.

### Fixed

- A `fold` that fails partway stamps the issues it did create, so rerunning does not create them twice.
- A heading whose priority cookie holds a multi-byte character no longer panics the parser.
- A prefix-scoped `issues.config.toml` overrides `vissue.toml` key by key instead of resetting the keys it does not name.
- A reader that closes the pipe, as `vissue export | head` does, exits 0 instead of panicking with status 101.
- An Org planning line no longer hides the property drawer below it. Writing a deadline, a scheduled date, or a `CLOSED` stamp from Emacs used to make every verb fail with ":ID: property missing" for the whole project file.
- An `issues.org` is flushed to the device before the rename publishes it.
- Auto-unblock to TODO releases the claim.
- Event generation and log append take a file lock.
- JSONL export and digest include CLOCK `raw` logbook lines.
- Text `ready` now uses the corpus-wide open-blocker set, matching `count --ready` and JSON ready.
- The `body-excerpt` secret screen reads credential shapes rather than three substrings, and matches vendor token prefixes on a whole word: the old form would have flagged "making" as an AWS key had that prefix been listed.
- The acyclicity check for `--block` reads the corpus inside the lock, so a peer write that landed first is part of the answer.
- `--project` folds case in `list`, `count`, `ready`, `export`, `graph`, `roadmap`, and the JSON rows, matching `claims`, `agenda`, and every verb that writes.
- `check` counts a duplicate id as an error, and reports a `:PARENT:` cycle. Every id in a parent loop resolves, so the edge checks passed it while the hierarchy stayed unwalkable and `tree` printed "(cycle, stopping)".
- `create` reports a full id space instead of panicking inside the write. `id_length = 2` is 1296 suffixes, which a project can outgrow.
- `events::tail_report` counts log lines rather than sequence numbers, so a debounced burst no longer shortens the tail.
- `graph` and `tree --format dot` escape backslashes and newlines in titles and ids, so issue text cannot become DOT syntax.
- `hygiene` matches issues by id rather than by row prefix.
- `mirror --out` writes through a flushed temporary and a rename, so a reader mid-pull cannot see a half-written projection.
- `note` adds its entry at the top of the logbook, where state transitions and claim releases already go.
- `org-set-tags` in Emacs no longer folds the tag run into the issue title.
- `refile` writes the target file before removing the heading from the source, so a failed write cannot lose the issue.
- `search` and `related` read tags from the heading as well as the drawer.


### Developer

- The Emacs interop check reports the error when Emacs itself fails, instead of exiting with a bare status and no message.


## [0.1.0] - 2026-08-12

### Added

- Org-backed issue storage, queries, graph operations, claims, events, mirrors,
  and JSONL export through the `vissue` CLI.
- MCP server exposing the same issue operations over stdio.
- Release preparation, package validation, and cross-platform archive wiring.

[0.1.0]: https://github.com/HaoZeke/vissue/releases/tag/v0.1.0
