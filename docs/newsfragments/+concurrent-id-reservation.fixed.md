Two concurrent `create`s for one project in two roots can no longer mint the same
id, and neither can two concurrent `reject`s minting a successor.

The reservation that stops a twin file sharing a suffix was read by the caller
and used by the mint. Locks are per file, so two creates in different roots took
different locks, each read the other before either had written, and both could
choose the same suffix. A duplicate across layouts is not cosmetic: `find_by_id`
reports `DuplicateId` for it and the issue stops being reachable by id.

The reservation is now a list of paths rather than a list of ids. Those files are
locked alongside the one being written and read after the locks are held, so a
peer's create lands wholly before or wholly after. The file being written
appearing in its own reservation list is ordinary, because the routed lookup
returns every layout for the project including this one, and the lock helper
sorts and dedups.
