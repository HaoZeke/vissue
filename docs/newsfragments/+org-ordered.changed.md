A parent with `:ORDERED:` holds later children with the same
`:PARENT:` out of `ready` until every earlier sibling is `DONE` or
`CANCELLED`. `:NOBLOCKING:` on a child skips that wait. `check`
names a heading that started or closed early, and a `DONE` that
still has open children.
