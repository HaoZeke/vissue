`consensus.susceptibility` chooses the opinion model. At `1.0`, the default, an
agent gives up its own starting position entirely and the group converges on one
number: DeGroot, exactly as before. Below it, an agent moves that fraction of the
way toward what it hears and keeps the rest of the ballot it cast, which is
Friedkin and Johnsen's generalisation.

Two things follow. The group settles while still disagreeing, and `consensus`
reports where each agent landed plus the spread between them instead of naming a
position none of them holds. And any anchor makes the step a contraction, so the
periodic trust graph that never settles under DeGroot cannot arise.
