Every flag a verb takes has to appear in the schema, which closes the last
one-directional check.

The flag checks ran schema to surface only: a flag the schema names must exist, and a
flag that exists and the schema omits went unseen. That is the same asymmetry as at the
verb level, one level down, and it stayed open after the verb level was closed.

Global flags are declared once as `globalFlags` and subtracted, so a per-verb row stays
about what the verb actually takes rather than repeating `--root` forty times. A
local-only verb is skipped: the HUD's nine flags are window management, and enumerating
them in a schema of operations would bury the flags that describe a surface.

Forty-one flags had to be recorded, generated from the argument structs rather than
written out, and two checks had to stop guessing type names while that happened. The
tool check derived `<Verb>Args` and the socket check `<Verb>Params`, which is wrong
wherever a struct is shared: ancestors and impact both take `DepthArgs`, list and ready
both take `IssueListParams`. Both now read the type from the code that declares it, the
tool's signature and the handler's own body.
