# Godot desktop client

[Root map](../README.md#architecture-map) · [Native bridge](../crates/hf-godot/README.md) · [Build reference](../docs/REFERENCE.md)

The [script guide](scripts/README.md) lists client entry points. The desktop [entry](scripts/main.gd) handles controls, speed, feed, selectors and save/load requests.
[World view](scripts/world_view.gd) consumes Rust data and passes it to [render layers](scripts/render/).
[Audio scripts](scripts/audio/) play cues and music; [persistence](scripts/persist.gd) handles client settings and save slots.
Camera and inspector paths present the simulation; the simulation's authoritative state lives in Rust.

For a visual or frame regression, use a fixed seed, tick, camera and build; [performance probes](../tools/perf/)
measure client work separately from the simulation worker. Use the source-asset bootstrap described in
[Contributing](../CONTRIBUTING.md) for the relevant tests. A headless sim pass does not certify the rendered client.
