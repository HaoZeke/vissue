A freshly started `serve` owner is now waited on for 15 seconds rather than
5, and `VISSUE_ACCEPT_TIMEOUT_MS` overrides that. A cold start binds its
socket only after building a runtime and loading the tracker, so a loaded
machine could reach the old deadline and be reported as a spawn failure.
The HUD's detach path reads the same deadline.
