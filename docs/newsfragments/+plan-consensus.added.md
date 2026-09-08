`vissue consensus <plan> --children` rolls up over a plan's children, which is
the "can this epic close" question rather than the "what does this issue hold"
one.

It reports the children row by row and does not average them. No weighting over
children can be picked without a judgement the tracker has no basis for, and an
equal-weight one lets an epic split finely outvote one split coarsely; a child
that settled split has no single position to fold in; and a child nobody voted on
is absent rather than neutral, which matters because unvoted is the common case.
So the report says how many children carry ballots, what each holds, whether the
voted ones point the same way, which settled split, and how many carry none.
