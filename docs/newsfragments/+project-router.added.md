A user-level `~/.config/vissue/config.toml` can send named projects to
another checkout. A route wins over `--root` and `VISSUE_ROOT`, so a
process whose default root is one tracker can still create and show
issues that live on another. `--no-route` and `VISSUE_NO_ROUTE=1` keep
every verb on the process default. `projects` and `identity` list the
routed names; `show` and `claim` find an id on any configured layout.
`vissue.el` still parses the original `identity` and `projects` lines;
route lines are appended.

`refile` and `reject` route their destination as well, so a bounce onto a
routed name lands on that name's tracker. `serve`, and the TUI and HUD
that talk to it, stay on the single layout the server was started with.
