The control socket now has a method for every verb that changes a file:
`issue/append`, `issue/reject`, `issue/resolve`, `issue/fold` and
`issue/normalize` join the six that were already there.

It stays optional. The advisory lock serialises a direct write just as well, so
this is not a correctness fix and a client mixing the two paths corrupts nothing.
What it buys is a complete surface: `append` used to exist only on the command
line, so a socket client had to shell out for it, those writes went behind the
server's back, and its change stream had a hole exactly where they were.
