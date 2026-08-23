`schema/regen.sh` regenerates the schema's Rust in one command.

The step needs the Cap'n Proto compiler and the `capnpc-rust` plugin, and neither is
needed to build or test. Where both are present the script is one command; where only
the compiler is, it writes the request file and prints what to run on a machine with
the plugin, because the two halves can live on different machines and the request
moves between them.

It finds the generated file rather than assuming its name, since the plugin lays its
output out by the source path recorded in the request and can put it in a `schema/`
subdirectory rather than flat.
