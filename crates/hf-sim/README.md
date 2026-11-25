# Simulation

[Root map](../../README.md#architecture-map) · [People](src/systems/README.md) · [CLI](../hf-cli/README.md) · [Bridge](../hf-godot/README.md)

Start at [Session](src/lib.rs), [World](src/world_state.rs) and the [tick pipeline](src/systems/mod.rs).
A session owns a mutable world and a chronicle. A tick advances the world through a fixed order of systems.
Some stages do work only at their own cadence. The pipeline flushes events and runs observers; the session
writes eligible events and aggregates history. View readers are not a second source of truth.
[Generated tick order](../../docs/REFERENCE.md) and [game time](../../docs/TIME.md) explain the scheduling.

## World and ecology

[Setup](src/setup.rs) takes the seed and settings, builds a [local grid](src/world/), can generate a [wider realm](src/realm/),
runs the valley's [past](src/deep.rs), then introduces [the founding people](src/opening.rs).
[Weather](src/systems/weather.rs), [plants](src/systems/plants.rs), [water](src/aquifer.rs), [hazards](src/systems/hazards.rs),
[wildlife](src/systems/animals.rs) and the [live realm](src/realm/live.rs) update that same world.
World state is saved; runtime indexes are rebuilt. See the [save boundary](src/save/README.md) and
[headless tools](../hf-cli/README.md) for round trips and profiling.

## Bodies and lives

[Agents](src/agents.rs), [species](src/species.rs) and [body](src/body.rs) define who exists and their present condition.
[Life](src/systems/life.rs), [health](src/systems/health.rs), [childhood](src/systems/childhood.rs), and
[rites](src/systems/rites.rs) change it. An agent's current state is not equivalent to every historical event about them;
follow [history](src/chronicle/README.md) and the [Godot boundary](../hf-godot/README.md) for those representations.

## Work and material

[Settlement](src/systems/settlement.rs) posts work. [Plans](src/plan.rs) and [work execution](src/work/)
connect choices to [items](src/items.rs), [structures](src/structures.rs) and [navigation](src/nav/).
[Trade](src/trade.rs) and the [economy](src/economy.rs) carry those effects across people and places.
Start with the [failed-plan tests](tests/failed_plans.rs) and [navigation tests](tests/navigation.rs).
Compare original and candidate with the same world and configuration before optimizing pathfinding or job searches.

## Societies and stories

[Society](src/society.rs), [relationships](src/social.rs), [customs](src/culture.rs), [faith](src/faith.rs)
and [expeditions](src/expedition.rs) operate on the live world. [Events](src/events.rs) feed post-tick observers
and the [chronicle](src/chronicle/README.md); [viewer projections](src/view/) turn evidence into player-facing read models.
For any new feedback path, verify which observers read an event and when; a new screen line is not evidence of a
new simulation effect.
