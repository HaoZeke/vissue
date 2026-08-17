`vissue update --if-state` / `--if-gen` refuse a write when the heading
or corpus moved since the caller last read it. Disagreeing terminals
keep the first close and record `:SIBLING_TERMINAL:`; `vissue resolve`
picks one. `vissue check` warns on a DONE that reads as a reject and on
a body `[[id:]]` with no successor edge.
