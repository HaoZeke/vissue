Every `issues.org` carries `#+VISSUE: 1`, the on-disk protocol
number. It is independent of the crate version and of the
control-socket `protocolVersion`. `normalize` writes or upgrades it.
`check` names a missing stamp and errors on a future number.
`identity` prints `protocol: 1`.
