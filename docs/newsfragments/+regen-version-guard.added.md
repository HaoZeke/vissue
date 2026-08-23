`schema/regen.sh` refuses to run when the `capnpc-rust` plugin's version does not
match the `capnp` runtime crate the generated file has to compile against, and will
run the plugin over ssh when `VISSUE_CAPNPC_SSH` names a host. The compiler is a
distro package and the plugin is a cargo install, so they land on different machines
often enough that the round trip is the common case.
