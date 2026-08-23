The checks that hold the command line to `schema/vissue.capnp` ask the parser what
it accepts instead of parsing what it prints. A hidden `vissue surface` walks the
built `clap::Command` and emits every subcommand with its aliases and long flags as
JSON, so a flag reaches a check because the parser takes it rather than because a
line of help spelled it in a recognisable way.
