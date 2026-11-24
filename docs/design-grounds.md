# Grounds: options for #289 (terrain and ground variety)

Status: proposal for the maintainer, written 2026-11-10 at master d5ef6f29 (WP-05 slice 5.5, the #289
half). Nothing here is implemented. Issues: #289 (ground variety; closed as the detail holder,
its list lives on), #84 (desire paths, still open), #268 (tile fertility, WP-05 5.6's
territory). Proposed as DEC-8 below.

## What is being decided

#289 says the tile-level texture is thin: ground families exist and blend at biome edges, but
the ground a person walks across never changes under them. Its list has seven items, and they
are not one feature: some are worldgen-time dressing, some are weather-driven surface changes,
and two need a per-tile state decision that has to be designed against the frame budget and the
save format before anyone writes code. This document says what the game already does, gives
the options per item, and recommends a first implementation slice. The code read is at the
commit named above; I did not run anything new.

## Facts at master

- **Materials are ready**: the surface table already names `SNOW`, `MUD`, `WET_SOIL` and
  `SCREE` (`world/material.rs`), and `FROZEN_SOIL`, `PEAT`, `GRAVEL`, `SAND` alongside them.
  What is missing is not new kinds but rules that put them on tiles and take them off.
- **Ground families exist and blend**: `every_surface_family_blends` and
  `ground_families_follow_biome` (tests/biomes.rs) pin the family layer; families render as
  terrain sheets in the client.
- **Weather is already simulated at valley scale**: `systems/weather.rs` steps a weather state
  (`growth_factor` and the rain that drives "drenched" and "rain_spoiled" events) once per
  tick. Snow exists on roofs (`r.snow`, cleared by the ClearRoof designation) and as a ground
  material, but it does not lie on open ground.
- **Movement is dense**: hundreds of people and animals path across the valley every tick.
  Any per-tile counter they all touch is a hot path; anything derived from movement must be
  computed from already-recorded data, not gathered fresh each tick.
- **Constraints**: determinism (ordered containers, `rng::stream` only); saves that load
  (enums and tile state appended only); the 16 ms worst-tick gate at 500 people (WP-01, now
  green, is the budget any per-tile work is spent against); simulation LOD deferred under D50.

## The seven items, options each

1. **Wear paths (desire paths, #84).** Options: (a) a per-tile step counter saved with the
   world — the honest model, the hottest path, and a save-format change; (b) a periodic
   recompute: at most daily, walk the already-recorded movement of the day (a bounded sample)
   and mark the busiest tiles worn, with wear only ever deepening on family tiles — no
   per-tick cost, deterministic, but a path is a lagging record, not a live reaction; (c)
   worldgen-only: pre-worn routes from the founding site to water and woodland. Recommend
   **(b) with (c) at generation**, and only for the valley's few most-used corridors: a worn
   path should be a readable landmark, not noise. #84 stays open until it ships.
2. **Wetness and mud after rain.** Options: (a) a saved per-tile wetness value — most
   flexible, most expensive; (b) a weather-driven material swap: while the weather is rain
   and for a bounded while after, low-lying `SOIL`/`GRASS` families render and behave as
   `WET_SOIL`/`MUD`, computed from the weather state and each tile's height and family —
   nothing new saved, fully deterministic, reverts on its own; (c) cosmetic only, client-side
   tint. Recommend **(b)**: mud that dries is the point of the item, and the weather state
   plus a tile's surveyed properties are enough to compute it with no new tile state.
3. **Snow accumulation and melt.** Roofs already carry snow with a melt rule; the ground has
   the `SNOW` material. Options: (a) extend the roof mechanism to a ground snow layer with a
   saved depth — pretty, heavy; (b) a weather-driven material swap like mud's: below the snow
   line, family tiles lie under `SNOW` while it is cold and precipitating, and melt back in
   order of exposure. Recommend **(b)**, sharing one mechanism with item 2: *one* "weather
   ground overlay" pass that computes both.
4. **Scree and loose rock on slopes.** Pure worldgen: where the local height gradient is
   steep and the family is bare, seed `SCREE`/`GRAVEL` tiles. No runtime state at all.
   Recommend **doing it**; it is the cheapest item and visible from day one.
5. **Tile-level fertility / drainage.** Belongs with #268 (worldgen nuance, WP-05 5.6), not
   here: it changes what farmers should see, which is a 5.6 decision. Recommend **defer**.
6. **Erosion and sediment.** Heavy simulation with knock-ons (a river that moves its banks
   invalidates nav, fields, memories). Recommend **defer** until fluids' W1 (flow regions,
   design-fluids.md) exists to hang it on.
7. **Broadened families at biome edges.** The blend tests exist; broadening is tuning, not
   architecture. Recommend **folding into item 4's PR** as data tuning.

## Recommendation (DEC-8, proposed)

Adopt: item 4 (scree, worldgen) and item 7 (family tuning) as one small PR; then items 2+3
as one "weather ground overlay" PR that computes mud and lying snow from the weather state
and surveyed tile properties with no new saved per-tile state; item 1 (wear) as a follow-up
proposal once the overlay exists, since it wants the same periodic pass. Defer 5 and 6.

Feel questions for the maintainer:
1. Should worn paths change movement speed at all, or are they looks and story only?
2. Lying snow: should it slow people and stock (a real cost), or is melt-water and the look
   enough for the first slice?

The defaults if unanswered: paths are story-only (speed effects come later if ever); ground
snow slows nobody in the first slice.
