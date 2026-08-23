The operation schema names each verb's command-line flags, and every subcommand's
help has to offer them.

Naming a verb on each surface stops the verb going missing. It does nothing about
the surfaces disagreeing on what to call a field, which is the next way this drifts:
a socket method taking `body` where the subcommand takes `--text` is two spellings of
one idea and no verb-level check sees it.

Read out of each subcommand's own help, so the parser answers rather than a scan of
the source that declares it. Positional arguments are excluded, and the reason is on
the field: the first version of the list claimed `create` and `reject` took `--title`
when both take it positionally, and the check caught that plus a `--blocked-by` that
is really `--block` on its first run.
