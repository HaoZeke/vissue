A fresh tracker declares Org's tag and publish keywords:
`#+FILETAGS:` includes `noexport`, `#+TAGS:` has the type group
plus `docs` / `perf` / `ignore` / `ARCHIVE`, and
`#+EXCLUDE_TAGS:` / `#+SELECT_TAGS:` are written. `search` matches
inherited FILETAGS and a group tag. An Org-format `mirror` drops a
heading tagged `noexport`. `check` names a file that still lacks
those lines.
