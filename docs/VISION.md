# Ostrog — Vision

The **egress point** is the local area chosen on the world map. Its **portal exit** is the tile where the founders emerge. After arrival, use the place name in maps and history (for example, *Follow in Ashcross*). A **valley** is one possible landform alongside a ridge, plateau, plain, coast or marsh.

Ostrog is an autonomous colony-world simulation built to run on a second monitor. The
viewer is a witness and optional caretaker. The inhabitants, creatures, communities and
expeditions create the activity through their own needs, skills, knowledge and relationships.

## North Star

[docs/NORTH-STAR.md](NORTH-STAR.md) (the maintainer, 2026-11-13, D93): every condition a watcher can see
registers in someone, moves them to act, and ripples to others; and a glance shows who they are,
how they feel and what they are doing. It names the three cuts in the living loop, the numbers that
measure progress toward 10/10 against The Sims, RimWorld and Dwarf Fortress, and the rule that the
work is taken one bite-sized slice at a time.

## The intended experience

Glance over from work and see people doing recognisable things for recognisable reasons: a
builder carrying stone to a half-finished wall, a cook at the hearth, two friends talking by
the river, a child following a parent, a mining party lamp-lit three levels down, a caravan
leaving at dawn. Look back an hour later and understand what changed from the event feed and
the history view, without the game ever having asked for a decision.

The world mixes four lineages:

- RimWorld: readable settlements, production chains, individual inhabitants, caravans, emergent
  personal stories.
- Dwarf Fortress: a meaningful vertical world, excavation, underground discovery, persistent
  consequences, historical depth.
- WorldBox: observation of independently acting creatures and communities, many at once.
- Early Sims: competent autonomy. Inhabitants pursue needs without instruction.

The world supports survival, settlement growth, comfortable daily life, exploration, cultural
development, travel, generations, decline and rediscovery, overlapping rather than sequenced.

Things are part of that history, not anonymous tokens. Tools, clothing, weapons, furniture,
workstations, ruins and relics should be assembled from understandable materials and parts, can
be worn, repaired, broken, abandoned or rediscovered, and retain true history where the simulation
knows it. Clicking a person should make their equipment, clothing and carried inventory legible,
then lead into the item's maker, origin and custody story without inventing missing facts.

## Visual direction

Top-down 2.5D pixel art, 32 px tiles, one master palette. The quality target is between Animal
Crossing's approachable material warmth and high-definition RimWorld/DF mods: richer than
RimWorld, at least as readable, and never noisy at second-monitor distance. Distinctive
inhabitants whose clothing and equipment can be recognised at normal viewing distance.
Settlements visibly change through labour and use. Varied terrain, vegetation, water, buildings,
weather, day-night lighting, wear and object condition. Underground spaces look and feel
different from the surface: dark base, lamp light, exposed strata. Clear elevation, entrances,
shafts and routes between z-levels. Layered top-down 2.5D is the current working presentation;
future renderer experiments must preserve its readability and simulation-backed material
language. Mockups may inform direction, but they are not production art or an implementation
checkpoint. The world must be compelling before any information panel is opened; the art bible
(docs/ART_BIBLE.md) records the quality actually achieved and the gaps that remain.

## Defining principles

1. Competent autonomy. Agents are never made inept to create work for the viewer.
2. No omniscience. Agents act on what they have perceived or been told.
3. Memory matters. What an inhabitant remembers changes what it does.
4. Labour is real. Construction and production consume real resources and real work time.
5. History is recorded, never invented. Every chronicle line traces to a recorded event.
6. Continuity over scale. Off-screen people stay the same people.
7. Ambient first. No modal prompts, no mandatory decisions, simulation continues unfocused.
8. Traceable culture. Every custom has an origin event and a behavioural consequence.
9. Ordinary simulation needs no language model.
10. Situated minds. People and animals observe locally, react through weighted personal
    dispositions, and learn from outcomes and other people. The player may inspect their evidence
    and history, but neither the simulation nor the renderer grants them omniscient world truth.

## What would undermine the vision

- Scripted events to keep the screen busy.
- Agents that stall, oscillate, deadlock or wander uselessly.
- Culture as decorative names with no behavioural effect.
- LOD that regenerates or replaces inhabitants when the camera returns.
- A settlement that changes without anyone having done the work.
- A camera director that fixates, switches constantly or shows empty ground.
- Polish as particles and bloom rather than readability and variety.
- Defining success down to a prototype or demo.

Working title. Setting, names and identity are original to this project.
