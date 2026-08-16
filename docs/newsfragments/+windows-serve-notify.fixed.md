`vissue-serve` compiles on Windows again. The `notify` crate is a
Unix-only dependency; converting `notify::Error` is now gated the
same way.
