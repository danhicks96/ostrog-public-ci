# Fluids: options for #323 (water and lava that move)

Status: proposal for Dan, written 2026-11-09 at master 589dbe5c (WP-05 slice 5.2). Nothing here is
implemented. Issues: #323 (water and lava do not move), #325 (irrigation), #272 and #273 (aquifers
and breaches, partly shipped), #189 and WP-01 (frame budget). Proposed as DEC-7 below.

## What is being decided

#323 says water and lava should both move ("a minimum requirement for DF") and itself warns that
this is an architecture decision, not a feature: the largest single piece of work in the backlog,
with a performance cost that has to be designed against a target from the start. This document
says what the game already does, gives the options, and recommends a first step small enough to
ship and measure. The code read is at the commit named above; I did not run anything new.

## Facts at master

Water already moves more than #323's October text says. What is in the code now:

- **Bodies with catchments** (`floods.rs`, #393): the standing water at the start is surveyed into
  rivers, lakes and ponds, each with the ground whose rain runs into it. Every hour a body rises by
  what its catchment gathers, drains back a share at a time, goes over its banks onto open ground
  lowest first, and draws back as it falls. Snow melt feeds spring floods.
- **One valley-wide ceiling:** `MAX_FLOOD = 600` tiles under flood water at once. The flooded set is
  saved. #323 records the flood event firing 41 times in a 600-day soak.
- **Water under the ground** (`aquifer.rs`, #272, #267): wet rock holds water; a cut into it lets
  water in; standing water beside or over a cut pours down the shaft and the workings joined to
  it, and the source gives up as much (#390). Wells (`wells.rs`, #433) dig until they strike it.
- **Water quality** (`systems/water_quality.rs`, #453 part 2) and a first irrigation effect:
  grain and roots with water within three tiles grow a third faster and a drought's lean does not
  reach them (`plants::watered`, `tests/irrigation.rs`; FIXLOG 2026-10-23 fix-325, "part"). That
  entry says digging channels to bring water to a field "waits on moving surface water (#323's
  further part)": option W0 below is meant to remove that dependency.
- **Not there:** water carried to a field (irrigation proper), water held back or diverted by what
  people build, a per-tile depth or flow, and **lava of any kind** (the only "magma" in
  `crates/hf-sim/src` is prose for legendary beasts in `realm/beasts.rs`).

Constraints any option must respect: deterministic runs (D7: ordered containers, no wall clock,
randomness through `rng::stream`); saves that load (the flooded set and any new fluid state are
saved, enums only appended); the 16 ms worst-tick gate at 500 people (WP-01 is still closing it,
with nav and settlement spikes), and simulation LOD deferred under D50.

## Options for water

**W0. Keep the bodies model and add dug channels.** A channel is a recorded edge between a body
and a destination basin or field (a dug line of open ground that stays lower than the source).
Water "flows" along it by being allowed to stand in the destination, computed when the edge is
built or the body's level changes, not by simulating tiles. Cost: no new per-tick work beyond what
`floods.rs` does. Delivers irrigation (#325's dig-to-water case) and a crude form of diversion.
Cannot do pressure, dams that burst, or water finding its own level in a cave.

**W1. A bounded local flow region.** Per-tile depth (integer, fixed update order) simulated only
where something is happening: near a breach, a channel head, a dam or a wall holding water. Tiles
settle and go to sleep; a sleeping region costs nothing. A hard cap on awake tiles keeps the cost
bounded, like `MAX_FLOOD`. Delivers containment ("a wall holds the water, a mistake floods the
stores") and caves that fill from the bottom. Cost: new state to save, new per-tick work while
awake, and a design for the cap and for what happens at the cap.

**W2. A full-valley fluid grid.** Rejected: a fluid tick over a 3D valley grid is what breaks the
frame budget, and the worst-tick gate is not met today.

## Options for lava

**L0. None.** Deep rock stays inert. Cheapest; leaves "digging into it has consequence" to water
(aquifers, breaches) which already exists.

**L1. Static magma pockets.** A material in the deepest strata that does not flow: breaking into
it burns what is adjacent, and it cools to stone over time. Fits the deep-peoples and breach
stories (#273) and costs a material, a hazard rule and a cooling timer, not a fluid.

**L2. Flowing lava** on the same machinery as W1, with a slower rate and cooling to stone. Only
worth doing after W1 exists and has a measured cost; steam where it meets water is a further step.

## Recommendation

1. **Do W0 first, as slice 5.3** (irrigation, the dig-to-water case): a channel edge recorded
   between a body and a field, guarded by a scenario test (a dug channel moistens the field, a
   field with no channel does not), no new per-tick cost. It is the part players ask for most and
   needs no fluid model.
2. **Prototype W1 on one limited region only with a cost measurement first**, as #323's own
   sequencing advice says, and only after the WP-01 worst-tick work closes (the gate is the same
   budget). Success is a cap on awake tiles that holds the gate in the 500-person profile.
3. **Lava: L0 now, L1 as a later content slice, L2 not until W1 has shipped.** No lava is promised
   for 1.0 unless Dan says so.

## Proposed DEC-7 (default unless Dan objects)

Fluids stay off the critical path to a release: water keeps the bodies-and-catchments model and
gains dug channels (W0); a bounded flow region (W1) is a prototype gated on the frame budget; lava
is deferred (L0, then L1). Agents may proceed on slice 5.3 on this default.

## Questions for Dan (feel, not engineering)

1. **Is lava that flows a requirement for the first release, or a later goal?** The recommendation
   assumes later; if it is a requirement, W1 and L2 move up and the release plan changes.
2. **Is a player mistake that floods the stores a wanted fail state?** It is what makes walls and
   channels matter (and needs W1); without it, water is scenery that can be irrigated with.

If Dan does not answer, the default above stands.

## Not in scope

No code, data or enum changes. The cascade ideas in #319 (steam, fire from lava) and a per-tile
temperature coupling are not covered.
