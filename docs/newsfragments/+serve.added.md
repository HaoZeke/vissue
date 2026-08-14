`vissue serve` now caches the issue catalog and answers the v1 control
methods (`issue/list`, `issue/ready`, `issue/claim`, and the rest).
Attached clients receive `vault/changed` after a rebuild. The files stay
the store: stopping serve loses nothing.
