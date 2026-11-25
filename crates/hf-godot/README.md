# Native Rust–Godot bridge

[Root map](../../README.md#architecture-map) · [Godot client](../../godot/README.md) · [Simulation](../hf-sim/README.md)

[SimHost](src/lib.rs) is the boundary the desktop calls. The client requests ticks; [the worker](src/sim_thread.rs)
steps the owned session, then copies a read snapshot of the finished world and bodies. The client uses that snapshot
while a later batch runs. Events also become feed entries, sound cues and camera suggestions.
The [generated reference](../../docs/REFERENCE.md) lists the packed agent record layout; the reader is
[world_view.gd](../../godot/scripts/world_view.gd). A layout change must update both sides and the generated table.

The worker records step time before copying the snapshot. Measure the copy and handoff separately before
assuming slow rendering or slow ticks. Changes to data layout should verify actual client consumption
and compare simulation digests. Save capture and disk commit are a [separate boundary](../hf-sim/src/save/README.md).
