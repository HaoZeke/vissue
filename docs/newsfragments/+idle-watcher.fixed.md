An idle tracker no longer rebuilds its catalog and broadcasts `vault/changed`
several times a second. The generation poll opens the projects directory on
every tick, inotify reports that open against the watched tree, and a read
was counted as a change, so the rebuild's own reads raised the next event.
