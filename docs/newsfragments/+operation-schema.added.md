`schema/vissue.capnp` states the verb set once, and the command line, the control
socket and the MCP tool list are each tested against it.

Every surface used to declare its own verbs in its own idiom, so a verb could exist
on one and not the others. That kept happening and nothing caught it: `vote` shipped
on the command line alone, `append` had no socket method for as long as the socket
existed, and a test asserting `issue/fold` was an unknown method became wrong the day
fold got one. The reference-completeness tests stayed green through all of it, because
they checked that the docs listed what existed rather than whether what should exist
did.

Three tests now read the schema and fail by name until their surface satisfies it.
Adding a verb to the schema that nothing implements fails all three at once, each
naming the gap on its own surface.

A verb that should not reach a surface leaves that field empty and says why in `note`,
so a deliberate omission reads differently from a forgotten one.

The schema's constant is compiled into a committed Rust file, so the checks read the
schema's own bytes rather than a list maintained beside it. `capnp` is not needed to
build or test; regenerating after a schema edit is a maintainer step, written down in
`schema/README.md`.
