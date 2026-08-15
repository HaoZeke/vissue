A client of `vissue serve` sees its own write. The catalog was rebuilt only
by the file watcher, so for up to about 450ms after a mutation the writer was
answered from the catalog as it stood before: `issue/list` came back one
short, and `issue/get` on the id `issue/create` had just returned reported
the issue as missing. The mutation path refreshes before it answers and
reports the resulting revision.
