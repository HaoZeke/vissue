`vissue hud` execs a separate `vissue-hud` iced palette over ready and
search. First paint reads the files. Unless `--offline`, the palette
attaches to `vissue serve` (starting it when the socket is free) and
falls back to the files when serve is down or bound to another root.
`--toggle` / `--show` / `--hide` talk to a summon socket.
