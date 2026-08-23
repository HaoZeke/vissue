Every mutating socket reply is asserted to have the same shape: a boolean `ok`, a
`report` string, and the affected issue where one is named.

`mut_result` produced that shape by convention rather than contract, so a method
returning a bare field, or omitting `ok`, would have passed every other check. A
client switching on `ok` and printing `report` is the normal way to use this socket,
which makes the shape worth asserting once across all of them instead of trusting
eleven call sites to agree.
