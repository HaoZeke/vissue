The schema records each field's Rust type per surface, and both checks verify it.

A name check cannot see a type change: a field going from a number to a string keeps
its name. `update --if-gen` is a `u64` on both remote surfaces and nothing said the
two agreed.

Recorded per surface rather than once, because two of them genuinely differ and
forcing one type over both would be a lie. `priority` is `Option<String>` as a tool
argument and `Option<char>` on the socket, `force` is `Option<bool>` and `bool`. What
this catches is either side changing.

Generated from the structs rather than written out, after two earlier rounds of
schema data written from memory turned out wrong.
