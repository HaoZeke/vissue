`vissue reject` closes a heading as CANCELLED and writes a successor
edge (`PIVOTED_TO` / `DISCOVERED_FROM`) in one edit, so a bounce is a
walkable wire rather than a body mention. `vissue wait --id --until-terminal`
blocks until that id is DONE or CANCELLED (exit 2 on timeout). State
changes emit a `state_change` event that names the id.
