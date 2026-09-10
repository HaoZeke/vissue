`root` lives on the router's own configuration file. Writing it to
`~/.config/vissue/config.toml` used to make the whole file unreadable,
because that file denied unknown keys. The router now accepts `root`,
and `$VISSUE_CONFIG` points at the same file.
