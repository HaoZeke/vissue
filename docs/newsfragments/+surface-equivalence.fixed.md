`issue/graph` and `issue/roadmap` group their output the way the subcommands do, and a
test now compares the socket's report against the subcommand's text.

This is the class of bug the name and type checks cannot see. `issue/mirror` answered
with the corpus digest while the schema, the types and the reference all agreed with
each other, because none of them compares content.

The graph divergence was the same shape and cosmetic: `report::graph` over a whole
layout emits every node and then every edge, while the command line emits each
project's nodes and edges together, because it builds one document out of per-project
bodies. Fourteen identical lines in a different order. Two surfaces answering one
question differently is the kind of thing nobody notices until they diff it.
