# Headless tools and diagnostics

[Root map](../../README.md#architecture-map) · [Simulation](../hf-sim/README.md) · [Tools](../../tools/README.md)

From a fresh source checkout, install the toolchain in [rust-toolchain.toml](../../rust-toolchain.toml) and run:

```sh
cargo run --locked --release -p hf-cli -- run --small --seed 7 --agents 12 --ticks 2400
```

This exercises world generation and stepping and prints a digest. It does not start Godot or verify
the real client's imports, rendering or sound. [Contributing](../../CONTRIBUTING.md) explains the pinned
source-asset bootstrap for full checks; [hosted CI](../../.github/workflows/ci.yml) defines the separate
manual route and cross-platform digest coverage. A green cheap push job is not evidence of a full run.

[CLI commands](src/main.rs) include runs, profiles, soaks, save recovery and probes. The current opt-in
[decision log](../hf-sim/src/debuglog.rs) captures scored choices, plan failures, abandonments and day counters,
not every intermediate state. Keep separate what a run measured and what it merely configured.

## Measure before optimizing

Use fixed seed or save, engine, build mode, tick count and machine class. Record both commit IDs and repeat
the same command. Use the CLI profile for per-system time; soak for longer-tail timing, stalls and memory.
Measure [snapshot copy](../hf-godot/README.md), [Godot frames](../../godot/README.md) and
[save capture/write](../hf-sim/src/save/README.md) independently.
