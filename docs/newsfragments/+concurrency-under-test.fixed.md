Concurrency is now covered by tests that were each checked against the lock they
guard, rather than by a paragraph in the reference.

Three cross-process tests: every mutating verb at once against one file, creates
racing across two roots for one project name, and separate processes emitting
change events. Each was run with its lock removed to confirm it fails. Without the
file lock, three of eleven headings vanish. Without the advisory half of the events
lock, event sequences duplicate heavily.

The events test needs one project per subject to have any power. With every subject
in one file the issues lock already serialises the pipeline and the events lock
never contends, so a first version using one project passed with the advisory lock
removed and proved nothing.

Two of these are labelled in the source as smoke tests rather than guards. The
two-root create test cannot detect a duplicate id at the default id length, because
the suffix space is 36^4 and two racing creates almost never collide by luck; the
guard with power for that is the deterministic reservation test in `vissue-core`.
