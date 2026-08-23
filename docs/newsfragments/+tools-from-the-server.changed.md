The checks that hold the MCP tools to `schema/vissue.capnp` ask the server over
stdio instead of reading `server.rs` as text. One `tools/list` handshake returns the
same answer an agent gets, so an argument is checked by the name and JSON type it is
advertised under, along with whether a caller may omit it.

This catches a class the source scan could not. A `#[serde(rename)]` on an argument
leaves the Rust field name in place, so scanning the struct for it passed while
agents saw a different name.
