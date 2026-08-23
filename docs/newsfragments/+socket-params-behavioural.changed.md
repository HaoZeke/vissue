The checks that hold the control socket to `schema/vissue.capnp` ask a running owner
instead of reading `dispatch.rs` and `rpc.rs` as text. A parameter is present because
the method refuses a value of the wrong type for it, and required because the method
refuses the request without it. Every request is built to fail at decode, so a
mutating method can be asked this without writing anything.
