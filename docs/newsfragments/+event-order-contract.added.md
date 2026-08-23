The reference states what the change stream promises: unique sequences, and no
ordering.

An event is emitted after the file lock is released, so two changes to different
project files can reach the log in the opposite order to the one they reached disk in.
A consumer treating a notification as "something moved, go and look" is correct; one
reconstructing history from the log's order is not, and nothing promised it could.

Emitting inside the lock would order the log at the cost of holding a file lock across
another write for every state change. Declined, and the reasoning is written down: the
log exists to wake a poller, and a poller re-reads state.

The cross-file cycle window is recorded the same way. Locking every file an acyclicity
check reads would prevent it and serialise every blocker edit in the corpus against
every other, because the check reads all of it. For blockers added by hand a few at a
time, the after-the-fact report is the better side of that trade.
