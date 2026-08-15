`digest` and `mirror --check` read the corpus once rather than once per
project. Each project's digest came from a whole-corpus export filtered
down to that project, which is quadratic in the project count: on a
tracker with 115 projects and 4781 issues those commands took 6.2s, and
now take 0.16s. The digest values are unchanged, so mirrors stamped by an
earlier version still read as fresh.
