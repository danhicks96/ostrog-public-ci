# Flight Recorder

Local, opt-in diagnostics for headless or desktop testing. This first slice records goal scores,
route attempts and their outcomes, plan failures and abandonments, and daily people and plan
counters. It is not a full world replay, a trace of every subsystem, or the player-facing Why lens.

## Record and inspect

```sh
cargo run --locked --release -p hf-cli -- --flight-recorder capture run --small --seed 77 --days 1 --agents 12
python3 -I tools/soak/read_flight.py capture --summary
python3 -I tools/soak/read_flight.py capture --agent 1 --limit 30
```

Use a new directory for each capture. The CLI option also works with `soak`. For the desktop,
set `OSTROG_FLIGHT_RECORDER` to a new directory before launching the Godot client.
Nothing is recorded in this mode unless explicitly enabled. The ordinary decision log and the
world save keep their existing format. Captures are local files; inspect them before sharing.

A second process can read a running capture. Query by tick range, agent, record kind or route
endpoint. The latest buffered data may not yet appear. Records identify what was observed at
the relevant hooks; adjacent records are not a claim that one necessarily caused the other.
An empty result means no matching retained evidence, not that no action took place.

## Retention and limits

- `survey.jsonl` has the session header, daily counters and coverage notices. It is separate
  from raw traces and has its own 8 MiB cap.
- Raw JSONL uses up to four 8 MiB segments. Rotation evicts only the recorder's own oldest
  trace and records that segment's tick span in the survey.
- If the writer or rotation fails, the recorder reports incomplete coverage instead of changing
  the simulation. A missing close record can mean the capture is still running or stopped
  uncleanly. Verify expected coverage rather than assuming a short result is complete.
- No game save, chronicle, or existing destination is pruned or overwritten by the recorder.
  Checkpoints, pinned incidents, full-system tracing and verified replay are separate work.

If a world stalls, check daily failure counts first. Choose a tick span around the spike,
look at one affected person, and compare scored choices, attempted endpoints, and recorded failures.
Where causality is not captured, use a fixed seed or disposable save and reproduce it.

A clean close records the retained raw sequence interval (first included, end excluded). Raw queries compare the whole retained interval before applying display filters, so partial copies missing the beginning, end, or all raw records report incomplete coverage. Summary queries do not inspect raw files. Old captures without a close receipt cannot certify raw coverage.
