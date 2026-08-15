A body containing a line that starts with `* `, which any markdown bullet
list does, no longer cuts the issue in two. The parser splits issues on that
line, so such a body used to leave a heading with no `:ID:`, stop the file
parsing, and drop every issue in that project out of `list`. Those lines are
now indented by one on the way out; `** Scope` and deeper are left alone,
being children of the issue rather than the end of it.
