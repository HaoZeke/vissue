`issue/mirror_check` reports whether a mirror file's stamp is still fresh. It briefly
answered with the corpus digest instead, under the name `issue/mirror`.

Two mistakes in one method. It answered a different question — a hash rather than a
verdict — so a caller asking whether its mirror was current had no way to tell it had
been told something else. And it conflated two operations that the tool list has always
kept apart: `vissue_mirror` renders a mirror, `vissue_mirror_check` judges one.

Rendering has no socket method, and the reason is on the schema row: it writes a file,
and a file the server writes lands on the server's disk rather than the caller's, so a
remote client would get a success and no file.

The reply carries `fresh` beside the report, because the subcommand exits non-zero on a
stale mirror and a client needs that signal without reading prose.
