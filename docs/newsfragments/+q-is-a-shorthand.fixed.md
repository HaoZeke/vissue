`q` has its own schema row instead of being recorded as an alias of `create`. It is
a subcommand of its own taking three of create's fields, so an alias row answered
for flags it rejects: `--body` and `--priority` among them. Rows now carry
`shorthandFor`, and a shorthand has to name a verb that exists, reaches the socket
if the shorthand mutates, and takes every field the shorthand takes.
