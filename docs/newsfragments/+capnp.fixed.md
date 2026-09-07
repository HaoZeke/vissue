`vissue-core` now builds against `capnp` 0.27. Versions before 0.24 let safe
Rust code trigger undefined behaviour through the generated schema readers
(capnproto-rust#605), and the operation set this crate reads is one of those
readers. Nothing about the schema or the verbs changed; regenerate
`vissue_capnp.rs` with `capnpc` 0.27 if you carry a local edit to
`schema/vissue.capnp`.
