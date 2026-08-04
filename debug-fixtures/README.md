# Debug fixtures

This directory contains small, reviewable history exports or export-shaped
cases for reproducing diagnostics without running the full game.

The browser download uses `pystral-gate-replay-v1`: it contains the stable demo
entrypoint and ordered player action inputs, not the expanded world snapshot.
Save downloaded JSON here when sharing a failure. `scripts/analyze_history.py`
accepts both replay downloads and reduced history fixtures.

The movement fixture captures the important distinction for the initial
playthrough bug: a batch may contain multiple transition-bearing events even
though replay advances one event at a time.

Run the programmatic check with:

```sh
make debug-fixture-check
```

`control_input_flood_case.json` is reduced from a user-exported 71 MB capture
that stopped in `Simulating`. Its renderer-to-worker input sequence was 1401
while worker output was 82 and history had reached sequence barrier 13. That
signature identifies repeated control input, rather than a missing history
transition. Keep reduced, sanitized cases here for fast stable tests even when
the original export is retained as an archival debugging artifact.
