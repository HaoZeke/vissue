`vote` is reachable from all three surfaces: `vissue vote` on the command line,
`vissue_vote` over MCP, and `issue/vote` on the control socket.

The feature exists for agents rather than for a person at a prompt, and agents
reach the tracker through MCP and the socket. A vote only on the command line
would have been a tally with nothing able to cast into it.

Socket and MCP ballots name the calling agent rather than the server process, so
one server serving several agents records which of them voted.
