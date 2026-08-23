`vissue surface` is documented: a row in the reference's command table, its JSON shape
field by field, why it is hidden and why hidden is not secret, and a how-to for reading
it from a wrapper instead of parsing help text.

The check that holds the reference to the command list now reads the parser's own list
rather than help output, so it covers hidden subcommands too. `--help` omitting a verb
is a choice about what a person reading help wants, and says nothing about whether the
verb needs writing down.
