`vissue consensus <id>` weighs an issue's existing ballots by how much the
group listens to each agent, using DeGroot averaging over a trust graph in
`[consensus.trust]`. It prints the plain count and the weighted position
together, each agent's social power, and the two ways there is no consensus to
report: a trust graph with more than one closed group, or one that never
settles.

`--json` gives the same result as structure: the choice set, each agent's limit
and social power, the settling, and the factions when there are any.

Nothing new has to be cast, and nothing changes on a tracker that configures no
trust: every agent then listens to every other equally and the consensus is the
tally as a fraction.
