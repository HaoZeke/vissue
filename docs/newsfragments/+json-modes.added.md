Nine reads gained a `--json` mode: `search`, `children`, `ancestors`, `impact`,
`backlinks`, `agenda`, `tree`, `body-excerpt` and `projects`.

They answered in prose on the command line and in structure over the socket, so there
was nothing to compare between the two surfaces and they went unaudited while every
other read was checked.

The modes go through the same `CatalogService` the socket answers from, so the two
surfaces are one computation rather than two that have to be kept in agreement. A test
compares all nine, and they matched on the first run, which is what routing them this
way round was for.
