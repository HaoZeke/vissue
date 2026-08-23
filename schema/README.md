# The operation set

`vissue.capnp` is where the verb set is written down, and the three surfaces are
checked against it: the command line, the control socket, and the MCP tool list.

Each surface used to declare its own verbs in its own idiom, so a verb could exist
on one and not the others. That happened repeatedly and nothing caught it. `vote`
shipped on the command line alone. `append` had no socket method for as long as the
socket existed. A test asserting `issue/fold` was an unknown method became wrong the
day fold got one. The docs tests were green throughout, because they checked that
the reference listed what existed rather than whether what should exist did.

Now three tests read this file and fail by name until their surface satisfies it:

| surface | test | crate |
|---|---|---|
| command line | `the_command_line_offers_every_verb_the_schema_names` | `vissue-cli` |
| control socket | `the_socket_answers_every_method_the_schema_names` | `vissue-serve` |
| MCP tools | `the_tool_list_offers_every_tool_the_schema_names` | `vissue-mcp` |

A verb that should not reach a surface leaves that field empty and says why in
`note`; the checks skip those, so a deliberate omission is distinguishable from a
forgotten one.

## Regenerating

The schema's constant is encoded into `crates/vissue-core/src/schema/vissue_capnp.rs`,
which is committed. Rust reads the schema's own bytes rather than a copy of them.

Committed rather than generated during the build because `capnp` the compiler is not
installed on the machines that build this, and a build-time codegen step would turn
the guarantee into a broken build. The pure-Rust `capnp` runtime reads the encoded
constant anywhere.

After editing `vissue.capnp`:

```
capnp compile -o- schema/vissue.capnp > /tmp/cgr.bin
capnpc-rust < /tmp/cgr.bin          # writes schema/vissue_capnp.rs
mv schema/vissue_capnp.rs crates/vissue-core/src/schema/vissue_capnp.rs
```

`capnp` and the `capnpc-rust` plugin are both needed for that step and neither is
needed to build or test. If they are on different machines, the first command only
needs `capnp` and the second only needs the plugin, so the request file can be moved
between them.
