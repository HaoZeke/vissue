An id's starting suffix is now derived from the project *and the issue's title*,
so minting is a function of its inputs rather than of the clock.

The same project, title and seed ask for the same id every time, which makes a
mint replayable. It also keeps two agents apart without coordination in the
ordinary case: two creates with different titles start from different points in the
space, so neither has to see the other's write to avoid it. Two agents asking for
the same title at the same moment do want one suffix, and the reservation settles
that, the first taking it and the second walking one along.
