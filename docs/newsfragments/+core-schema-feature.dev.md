`vissue-core` gains an optional `schema` feature deriving JSON Schema on the
view types, and `agent` gains typed accessors beside the JSON ones so a caller
that wants the type does not go through a `Value` to get back to it.
