`[consensus.susceptibility_of]` sets one agent's susceptibility where it differs
from the `consensus.susceptibility` default. Friedkin and Johnsen's
susceptibility is a diagonal rather than one number, which is the part that
carries the meaning: a maintainer who has read the code for years and a reviewer
seeing it for the first time are not equally movable. Rows merge agent by agent
like the trust rows, and where the agents differ the report puts the value on
each row rather than on the header.
