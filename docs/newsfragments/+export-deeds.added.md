Each `export` row carries a typed `deeds` array beside `properties`, in the order
the citations were made. The socket already handed that field over typed and the
JSONL did not, so a consumer had to split a drawer string on whichever separator
the author used. `:DEEDS:` stays in `properties` verbatim for a reader that wants
the drawer.
