The schema names each method's socket parameters, and the param structs are checked
against it.

The verb check says `issue/append` answers. It does not say the method takes `text`
rather than `body`, and the third spelling of a field is where this drifts next. Read
off the param structs in `rpc.rs`, where serde decides the wire names, so what is
checked is what a client has to send.

Two facts that had never been written down anywhere are now on the fields. The issue
being acted on is positional on the command line, `id` on the socket and `issue_id` as
a tool argument. And `agent` exists only on the socket, because only there is there a
connection whose identity can be overridden; the other surfaces take it from the
environment.
