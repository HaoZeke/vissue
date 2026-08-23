The closed-pipe test builds its corpus by byte count rather than issue count. It
needs more output than a pipe buffer holds, which twenty-four large bodies reach in
a fraction of the work four hundred short titles took, and it now asserts the
corpus really does exceed the buffer so the test cannot quietly stop testing
anything.
