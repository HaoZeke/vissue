`vissue hud` is a summonable task board. It opens on the project list;
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
