# Desktop scripts

[Client map](../README.md) · [Root map](../../README.md#architecture-map)

[main.gd](main.gd) runs the scene, handles input and requests ticks from the native bridge.
[setup.gd](setup.gd) handles new-world choices. [world_view.gd](world_view.gd) reads a Rust
snapshot and coordinates the [renderers](render/). [Audio](audio/) consumes sound and music cues;
[UI helpers](ui/) are client code. [persist.gd](persist.gd) owns preferences and names save slots;
the [Rust save module](../../crates/hf-sim/src/save/README.md) owns the package format.

A screen regression needs a client check at a known camera and simulation tick.
A simulation digest alone does not prove what the renderer, feed, or inspector displayed.
