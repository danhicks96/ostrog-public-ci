# M3 design: the world the valley is part of

Programme row M3 in docs/PLAN-FINISH.md. Written 2026-10-18 from a survey of the code it sits
on (world/gen, deep.rs, setup.rs, expedition.rs, threat.rs, names.rs, chronicle, save).

## Where we start

- The valley is one generated 128x128 tile map with 24 levels; its climate comes from a seeded
  `Character { wetness, warmth, stoniness }`, not from anywhere.
- The "overworld" is six sites made by `Overworld::generate(seed)` from a fixed list: no
  positions, only an edge and a number of days; two villages and a mine have people.
- The past (`deep::run`) is one valley's hamlets over up to 300 years, one step a year, with
  named figures and traces on the ground; names come from fixed English-flavoured tables.
- The chronicle references only agents (`entity_ref.entity_kind = 'agent'`); `Old*` events
  name no one.

## What M3 builds

A world of a few hundred regions on a side, generated from the seed before anything else; the
valley is one region of it, generated at full detail from that region's data. Peoples with
their own languages and convictions found sites, grow, trade, quarrel, fall and are
remembered; thousands of named figures live, make, rule and die; every one of those things is
an event in the chronicle with references to what it touched. The arrival happens in a world
with a past, and the world goes on after it at coarse grain.

Nothing is decorative: a ruin on the map is a site that ended, with the year and the cause; a
name in a lay is a figure with a life; a raid comes from a real place at a real distance.

## The pieces (module `realm`, crate hf-sim)

1. Region map (`realm/map.rs`). Size 192x192 by default. Per region: elevation, temperature,
   rainfall, drainage, biome, river flow and its direction, lake, rock (the valley's strata
   follow it), soil fertility. Generation, all from purpose streams of the seed:
   - elevation from octave noise plus a few ridge lines (mountain chains), sea level by
     fraction;
   - hydrology: depressions filled by priority flood (lakes where they were deep), flow
     accumulated downhill, rivers where flow passes a threshold; every river ends in the sea
     or a lake (a test);
   - climate: temperature by latitude and height; rain carried by a prevailing wind, lost
     over rising ground (rain shadows behind ranges);
   - biome by temperature and rain (a Whittaker table): ice, tundra, taiga, temperate forest,
     grassland, steppe, desert, savanna, rainforest, swamp, plus mountain and coast.
2. Embarking (`realm/embark.rs`). The valley's `GenParams.character` comes from its region
   (wetness from rain, warmth from temperature, stoniness from height and rock), and its edge
   rivers enter where the region's river does. The wizard picks a region; the default is a
   habitable one chosen by seed. The old `Character::of_seed` stays for tests and
   `--legacy`.
3. Peoples (`realm/peoples.rs`). Four to seven peoples, each with a language (the earlier prototype
   `naming::word_for(seed, culture, concept)`, ported; place, person and polity names are words
   of that language), convictions as belief stances (the same `mind::beliefs` model people
   use), a homeland biome and a way of living (farming, herding, hunting, mining).
4. Sites and polities (`realm/sites.rs`, `realm/polity.rs`). Sites: hamlet, village, town,
   keep, mine, shrine, ruin, lair. A site has a region, a people, a population (an aggregate,
   not individuals), stores, crafts and a founding year. Polities are sites under one ruler.
   Each simulated year: growth toward what the land carries, a daughter site when crowded,
   trade along the cheapest routes, quarrels where borders and convictions meet, war when
   they harden, famine, plague and fire from conditions, and the deity's leaning on them as
   now (deity.rs watches the world pass as it watches the valley's).
5. Figures (`realm/figures.rs`). Named people: rulers, heroes, makers, priests, outlaws, and
   beasts with lairs. Compact records (birth, parents, site, people, craft, deeds as event
   ids, death and cause). Tens of thousands over a thousand years. A figure who comes into the
   valley is promoted to a full agent once, keeping name, age, kin and past (D10).
6. The history pass (`realm/history.rs`). Yearly steps for up to 1,000 years on a worker
   thread, with a loading screen that shows real rows; deep.rs becomes the valley's own
   chapter of it (who lived in this region, and what came to it from where).
7. After the arrival. The realm steps once a game-season; `overworld.sites` becomes the real
   sites near the valley, with distances in days from the map; caravans go to them, bands
   come from them, news of wars and famines arrives as rumours with a teller.
8. The chronicle. `entity_ref` gains kinds `figure`, `site`, `polity`, `people`, `artifact`,
   `region`, written from a method on `EventKind` beside `participants()`. Era rows summarise
   decades. The Legends browser (M3's last checkpoints) reads only these.

## Local liquids, porous materials and observation

World generation supplies bodies and catchments; local simulation must eventually carry the state
that makes a tile wet or dangerous: depth, body type, flow direction and strength, flood stage,
temperature or ice, saturation, murkiness/pollution where implemented, and interactions with
porous materials. A wall or floor may seep, stain, soften, conduct heat or rupture only when the
material and adjacent fluid state support it.

People do not receive these facts globally. A person can notice a damp wall, hear water, smell a
stagnant pool, remember an earlier flood, or miss the clue entirely. Swimming and wading are
explicit capabilities and states; unsupported water and lava are never walkable. The renderer may
vary a liquid's appearance only from fields the simulation and bridge actually provide.

This section is a design boundary, not a claim that every field is implemented today. Missing
fields remain unknown rather than becoming invented visual or inspection facts.

## Saves and size

The region map is regenerated from the seed and its parameters, never saved. Saved: peoples,
sites, polities, figures and the realm's year, as `#[serde(default)]` fields on the snapshot;
history rows go to the chronicle. Target: under 20 MB of snapshot for a 1,000-year world.

## Gates (tests)

- `realm_is_deterministic` (same seed and parameters, same map and history digest).
- `rivers_reach_the_sea_or_a_lake`; `rain_shadows_fall_behind_ranges`.
- `valley_follows_its_region` (a wet, warm region gives a wet, warm valley).
- `history_only_from_simulation` extended: every realm event's references resolve, every
  ruin has an ending event, every named figure in a lay exists.
- `figure_promoted_keeps_identity`.
- Budget: map plus 1,000 years under 60 s on the box (aim 20 s); `ostrog realm` reports it.

## Checkpoints

| # | Work |
|---|---|
| C24 | Region map: elevation, hydrology, climate, biomes; `ostrog realm --png`; embark from a region |
| C25 | Peoples and languages (word_for port), sites, polities, the yearly growth step |
| C26 | Figures, the history pass to 1,000 years, chronicle references, budget |
| C27 | The valley inside it: overworld sites from the map, deep.rs fed by the realm, caravans and bands by distance |
| C28 | Legends browser v1: world map through time, site, figure and people pages |
| C29 | The world goes on after the arrival: seasonal steps, wars, migrations, news as rumours |
| C30 | Review, tester round, four soaks, gap table against DF and RimWorld |

## Ported from the earlier prototype

- `naming::word_for` and the onset, nucleus and coda tables, as is (depends only on rng, which
  is already identical).
- The epoch-anchored roller pattern of `lod::generate_dormant_events` (composable rolls on
  absolute boundaries) for event chances in the history pass; not its event list.
- The aggregate idea of `lod::PopulationAggregate` for site populations (headcount, age and
  craft distributions); not its code, which is tied to the prototype's Individual and Genome.
- Not ported: `fate_scout` (its World and cosmology have no counterpart); the same idea, a
  scratch twin from `Snapshot::capture`, can preview a world for the wizard.
