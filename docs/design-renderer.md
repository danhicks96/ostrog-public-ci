# The renderer plan (#414 B2)

Status: proposed (box-support the local AI, 2026-11-03). Decision row D74. Dan can overrule the
recommendation; the assumption it rests on is stated in section 3.

This plan builds on #190 (closed into #414 on 2026-11-01) and on D53, which sets the frame-time
gate: 95% of frames within 16.7 ms and none over 33 ms, with 500 people of mixed kinds and
their animals in groups across z-levels, and threads used throughout. The sim does not change
for any of this: the client draws only what the bridge sends.

## 1. What the client does each frame today, and where the time goes

### The frame

Everything below runs on Godot's main thread, in GDScript, on the Compatibility renderer
(`godot/project.godot`, `gl_compatibility`).

- `main.gd` `_process` steps the sim on the main thread: 20 ticks a second at 1x, so a
  `sim.step` call lands on about one frame in three at 60 fps, and every frame at 3x and 10x
  (up to 400 ticks in one call after a stall).
- Every 50 ms (one frame in three) `world_view.gd` `refresh` reads the world through the
  bridge: three levels of tiles (`level_tiles`), `ground`, `plants`, `agents`, `bands`,
  `thin_view`, `band_lines`, `structures`, `items`, `engravings`, `designations`, `roofs`,
  `built_of`, `carried`, `scorched` and the clock and weather. Each comes back as a fresh
  Packed array built in Rust and copied into GDScript. Then GDScript rebuilds its own
  dictionaries from them (occupied tiles, fires, scorched ground) and calls the entity rebuild.
- The entity rebuild (`render/entity_layer.gd` `rebuild`) walks the arrays in GDScript and
  places a pooled `Sprite2D` node per drawn thing: a person is a node with layered child
  sprites (body layers, band, equipment, carried thing). Moving things are redone on every
  refresh; plants, structures, camps and items every half second or when the view moves.
- Every frame `world_view.gd` `_process` animates the sprites, updates effects, weather and
  light, keeps the ground chunks in step (`_sync_chunks`), and queues a redraw. Ground chunks
  draw themselves with immediate `draw_texture_rect` calls in GDScript when they change.
- Then Godot draws: one canvas item per sprite, batched by texture.

### Measured on the box

`tools/perf/frame_prep.gd` (added with this plan) loads a save headless and, for 120 frames,
steps the sim one tick, times each bridge read the view's refresh makes, and times the
client's own preparation: `refresh`, the entity rebuild and `_process`. Headless means Godot's
dummy renderer: nothing is drawn, so these numbers are the CPU work of preparing a frame,
not the GPU's. Software-GL timings from the box's Xvfb display are not representative of a
real GPU and are not used. The draw itself is not measured here; it needs a real GPU
(section 4).

Setup: the release bridge (`target/release/libhf_godot.so`), a save made with
`ostrog run --seed 1 --agents 500 --days 2` (the bridge sent 393 agent rows on its
day 2), the view at the settlement's level with a 60 x 34 tile window (1920 x 1080 at zoom 1).
Box load average 13 to 15 on 32 cores. Milliseconds on the main thread:

| Part | p50 | p95 | max | How often |
|---|---|---|---|---|
| `sim.step(1)` | 2.6 | 3.0 | 15.9 | 1 frame in 3 at 1x, every frame at 3x and 10x |
| bridge `level_tiles` x3 | 0.9 | 1.0 | 1.5 | each refresh |
| bridge `plants` | 0.4 | 0.5 | 0.5 | each refresh |
| bridge `agents` | 0.3 | 0.3 | 0.4 | each refresh |
| bridge `ground` | 0.2 | 0.2 | 0.2 | each refresh |
| all other bridge reads | under 0.1 together | | | each refresh |
| `refresh` (reads, script, moving rebuild) | 19.1 | 21.4 | 5041 (first) | 1 frame in 3 |
| entity rebuild, moving only | 11.5 | 13.0 | 24.8 | inside each refresh |
| entity rebuild, with static | 16.9 | 18.4 | 37.7 | every half second |
| `world_view._process` | 2.9 | 3.7 | 10.0 | every frame |

With the whole map in view (`--whole-map`, nothing culled) the moving rebuild was 8.0 ms
and the static one 21.6 ms, at load 14: about the same, because most of the crowd is near
the settlement and the window already holds it. The cost follows the number of things drawn.

What this says:

- The bridge is cheap: all the reads of a refresh together take about 2 ms of the 19.
- The cost is GDScript: walking the arrays, keeping dictionaries, and placing and setting a
  node per sprite layer. About 17 ms of every refresh is script, on the main thread.
- A frame with a refresh already spends about 19 + 3 = 22 ms on preparation before anything
  is drawn, and one frame in three has a refresh. Add `sim.step` on the same thread and the
  static rebuild every half second. So today at 500 people one frame in three is over
  16.7 ms on the CPU alone, and the half-second frames come near 33 ms: the gate fails by a
  wide margin, before the GPU.
- The sim tick is a smaller share (2.6 ms) but it is on the main thread, and its worst ticks
  (the daily planner, #428) land in a frame whole.

## 2. Three paths, compared

All three keep the sim in Rust, on its own thread, unchanged. All three need the same new
boundary: the sim thread publishes a read-only snapshot of what the view needs after each
tick (or every few ticks), and the renderer reads the latest snapshot. Determinism is
untouched by construction: the renderer never writes to the world and never draws a random
number from the sim's streams, and the sim's results do not depend on when a snapshot is
read (D55's rule for threads, applied to drawing).

### Path 1: Godot with a Rust WorldRenderer (#190's proposal)

A GDExtension class in Rust, `WorldRenderer`, owns the world's drawing through Godot's
`RenderingServer` directly: canvas items or MultiMeshes per ground chunk and per sprite batch,
one texture atlas (or Texture2DArray) per family, palette swaps in a shader. It reads the
snapshot in Rust, with no Packed arrays and no GDScript in the per-entity loops. Its
preparation (culling, animation poses, sprite assembly, the z-level stack) runs on a rayon
pool across cores; the main thread only hands the finished buffers to the RenderingServer.
GDScript keeps the camera, UI, panels, audio and input.

- Frame-time gate: the 17 ms of script per refresh becomes Rust over flat arrays, spread over
  cores. A MultiMesh draw of a few thousand sprites is one call per batch. Reaching the gate
  at 500 people is likely on a mid-range GPU; this has to be shown by the harness in section
  4, not assumed.
- Migration cost: moderate. The new class replaces `world_view.gd`, `render/entity_layer.gd`
  and the ground chunk drawing, family by family behind a flag (B7). Nothing outside the world
  view changes: the wizard, Legends, panels, toasts, the timelapse panel, audio buses and
  music director, particles for fire and weather, export to Windows and Linux all stay.
- Assets: the 2,681 PNGs under `godot/assets` and their named slots stay as they are; the
  atlas is built from them at import or load. The CI tests that fail on a missing slot keep
  working. New art (B5) lands in the same slots.
- Risk: lowest. Godot's 2D batching and the GDExtension API are known quantities here
  (the bridge is already one). The main risk is the RenderingServer API surface from Rust
  (godot-rust 0.5) and keeping draw order right with the GDScript overlays.

### Path 2: a pure-Rust client on wgpu

A new client in Rust: winit for the window, wgpu for drawing, an immediate-mode UI (egui) for
panels, an audio crate (kira) for sound. The sim is linked in directly; no bridge.

- Frame-time gate: the best ceiling. Everything is Rust, there are no engine nodes, and
  the job system can own the whole frame.
- Migration cost: high. Every part of the client has to be rebuilt before parity: about 60
  wizard controls and the six-page wizard, Legends with its clickable links, every panel, the
  history reel, toasts, the four audio buses with underground filters and positional voices,
  the music director, particles, font shaping, input, and Windows and Linux packaging. #190
  estimated 2 to 4 months to parity; the client has grown since.
- Assets: the PNGs and their names carry over; the loader, atlas and the GDScript tests over
  them (`tools/gd_test.gd`, `tools/gd_check.gd`, the shot tools) must be rewritten.
- Risk: high. Two clients would exist for months, and the UI epic (#413) would be redone.

### Path 3: BRUTAL with a C# client

BRUTAL (ahwootech/BRUTAL, released 2026-10-15) is the C# framework under RimWorld's Waylith
engine. Checked against its repository on 2026-11-03: C# on .NET 10; Windows and Linux;
Vulkan 1.3 as the main backend; a job system (`Brutal.Concurrency`); Dear ImGui for UI; "no
editor, no scene graph and no renderer"; a licence based on MIT with an attribution
requirement (credits, a logo before play, NOTICE.txt); building from source not yet fully
supported. Ludeon reported 2 to 4 times RimWorld's performance after moving onto it, but
RimWorld was already C#.

- Frame-time gate: capable in principle (Vulkan, a job system), but BRUTAL has no renderer:
  Ostrog would write its own sprite renderer on top, as in path 2.
- Migration cost: highest. A third language (C#) in the client, calling the Rust sim over a
  C ABI that does not exist yet; the whole client rebuilt as in path 2, with UI on Dear ImGui.
- Assets: as path 2, plus a C# loader.
- Risk: highest. A young framework with a small team, source builds not yet supported, a
  Vulkan 1.3 floor (older laptops lose out), and the attribution terms to meet. The move that
  made sense for RimWorld (C# to C#) is a language change for Ostrog.

### Side by side

| | Godot + Rust WorldRenderer | Pure-Rust wgpu client | BRUTAL + C# client |
|---|---|---|---|
| Reaches the gate | likely, to be shown | likely | likely, after writing a renderer |
| Work before parity | the world view only | the whole client | the whole client, plus a C ABI |
| UI, audio, export | kept | rebuilt | rebuilt (Dear ImGui) |
| Named asset slots | kept as they are | kept, new loader | kept, new loader |
| CI asset and GD tests | kept | rewritten | rewritten |
| Languages | Rust, GDScript | Rust | Rust, C# |
| Determinism | sim thread + snapshot | the same | the same |
| Risk | low | high | highest |

## 3. Recommendation

Path 1: Godot with a Rust WorldRenderer.

The assumption, which Dan can overrule: the measured cost is in GDScript preparation on the
main thread, not in Godot's drawing, so moving that preparation into Rust on worker threads
and the sim onto its own thread is enough to meet the gate on Dan's machine. If the harness
in section 4 shows the Godot draw itself over budget after stages 1 to 3, path 2 is the
fallback, and the snapshot boundary built in stage 1 is exactly what a wgpu client would read,
so none of the work is lost (as #190 argued).

Stages, each shippable, the old renderer kept until the new one passes the gate:

1. **The sim on its own thread, with a snapshot.** The sim ticks on a worker thread; after
   each tick it publishes an immutable snapshot (`Arc`) of what the view reads today; the
   bridge's read calls serve from the latest snapshot. GDScript is unchanged. Gate for the
   stage: no frame waits on a tick (the 15.9 ms tick spikes leave the main thread), digests
   identical to the single-thread run. This alone removes `sim.step` from the frame.
2. **The harness on a real GPU** (section 4), run on Dan's Windows machine and the box, giving
   the baseline per thread with stage 1 in.
3. **WorldRenderer for moving things.** Rust builds the people and animal sprite batches from
   the snapshot on the rayon pool and submits MultiMeshes; behind a setting, default off, the
   GDScript entity layer still there. Gate for the stage: the harness shows the refresh frame
   under 16.7 ms at 500 people with the setting on, and the shot tools show the same picture.
4. **WorldRenderer for static things and ground.** Plants, structures, items, camps and ground
   chunks move to Rust-built batches; same flag, same gate.
5. **Switch the default.** When the whole world view passes the gate (95% within 16.7 ms, none
   over 33 ms, 500 mixed-kind people and animals in groups across z-levels), the setting
   defaults on; the GDScript renderer is moved to `to_delete/` one release later.

B5's new art can start at any stage: it lands in the same named slots either way.

## 4. Next issue: per-thread frame-time measurement

Title: "A frame-time harness that replays a saved world and reports frame time per thread
(#414 B2)".

- Input: a save (the gate's crowd: 500 people of mixed kinds with animals, in groups across
  z-levels), a camera path (a few fixed views and a pan), a duration, a speed.
- Runs the real client (not headless) on a real GPU, windowed at 1920 x 1080, vsync off.
- Records per frame: total frame time (`Performance.TIME_PROCESS`, the render server's frame
  time, and wall time between frames), and per thread: the main thread's script time, the sim
  thread's tick time, and each render-preparation worker's busy time (Rust-side timers in the
  bridge, read once a frame through one call that returns them all).
- Reports p50, p95, p99 and max per thread, the share of frames within 16.7 ms and the count
  over 33 ms, as a CSV and a one-line verdict against D53's gate.
- Runs on Dan's Windows machine (the target) and on the box; the box's software GL figures are
  labelled as not representative of a GPU.
- `tools/perf/frame_prep.gd` from this plan is its headless first half and stays as the
  CPU-only check that CI can run.
