# Design: homes, shelter, materials and moving camp (milestone MH, C56 to C60)

Written 2026-10-19 at the start of MH, from Dan's questions of 2026-10-19 and the answers checked
against the code in docs/history/PLAN-NEXT.md section MH (what exists, with file and function, is there
and is not repeated here). The rules that bind every part: nothing happens on a schedule or by
order; each home, shelter, trip and move has a proposer and a recorded cause; no LLM; the world
is deterministic; nothing is invented.

## 1. C56: walls keep their material

Today `BuildWall(m)`, `BuildDoor`, `BuildFloor(m)` and `BuildRoof(m)` finish with the
designation's generic `Material::WOOD` or `Material::STONE_BLOCK` (`systems/work.rs`, the
finishing match), while `BuildStructure` already takes the delivered material.

- Finishing: the tile stays `Material::WOOD` or `Material::STONE_BLOCK`, and what it was built
  of (the first requirement's delivered material, `r.material`) goes in `World::built_of`, keyed
  by tile, read through `World::made_of`, which answers only while the tile is still a built
  wall. Setting the tile itself to granite was the first idea and was dropped while building:
  a built granite wall would then read as natural rock to mining (`nearest_rock_face`), the
  roof of the deep (`underground.rs`), house checks and the client. Doors are of the log
  delivered. Roofs already have their own record and keep the poles' wood; thatch stays thatch.
- Requests stay generic: the planner still posts `BuildWall(Material::WOOD)` meaning "timber" and
  `BuildWall(Material::STONE_BLOCK)` meaning "stone"; `make` keeps mapping them to Log or
  StoneBlock with no `want`, so any wood or stone at hand is used.
- Code that compares a tile with the two generic materials stays as it is, since the tile is
  still timber or stone.
- Wood burns by its kind: every wood burns; a denser wood burns longer (fuel from density).
- Art: `tools/art/structures.py` generates wall, wall-under-construction, door and floor sheets
  per material from its colour and family (oak, birch, pine, fungus wood; granite, basalt,
  marble, slate, limestone and sandstone, the six rocks that cut to blocks), with provenance. The bridge
  sends the material's file name for a wall (`items.rs` has the naming); the client's fixed
  `WALL_MAT` table goes. A test fails if a material a wall can be made of has no sheet.
- Inspect: "an oak wall", "a granite house".

Gates: `a_wall_is_of_the_wood_it_was_built_from` (door included), `a_stone_wall_is_of_its_stone`,
`an_oak_wall_burns_longer_than_a_pine_one`, `every_wall_material_has_a_sheet`. Soak digests change only
through fire fuel; the review says by how much.

## 2. C57: homes proposed by people

- `ProjectKind::Home { form, household }` (`project.rs`). Proposed in `systems/projects.rs` by a
  member without a bed of their own, or the eldest of a household whose beds are fewer than its
  people, with the cause: a pairing (`Paired`), a birth, newcomers joined, a home lost (fire,
  collapse), or nights slept cold (a memory). One open home project per household.
- Form from the proposer's kind: `species.rs` `Habitat::builds` becomes an enum
  (`Houses`, `Hollows`, `Tents`, `Nests`) read at load, the TOML strings unchanged.
  - Houses: a rectangle sized to the household (beds needed plus a hearth's room) and to body
    size (the plan's size class: 2-tile doors for huge kinds), not the fixed 7 by 6.
  - Hollows: a room dug into a rock face or hillside within reach of home, behind a door, through
    Mine designations and `underground.rs` shoring rules (C58 adds the reasons to dig in).
  - Tents: a hide-and-pole shelter (new `StructureKind::Tent`, 2 hides and 2 logs), quick to
    raise, struck and carried when the household moves (C60).
  - Nests: a raised woven platform (new `StructureKind::Nest`, logs and fibre) for the Skarri.
- A culture's way of building: a `CustomKind::Building { prefers, door_faces, hearth_inside }`
  spread like other customs (`systems/culture.rs`), its source the first home built that way, so
  two communities of one kind differ, each with a cause. Roofing stays a technique.
- The planner's house rule in `plan_community` goes; the planner lays out what a proposal asks
  (footprint, frame, door last) as it does now.

Gates: `a_home_has_a_proposer_and_a_cause`, `a_hollow_folk_digs_a_hollow`,
`a_household_house_fits_the_household`, `a_way_of_building_is_learned_from_someone`, and the
existing house tests rewritten to go through a proposal.

As built in C57: the proposal, the cause (a night on the ground, the Slept event as source),
the four forms (a hollow-building kind with no rock face near builds a house and the reason
says so), tents and nests as sleeping places. Not built in C57 and carried to C58: houses sized
to the household and to body size (the house is still 7 by 6) and a culture's way of building,
which also answers C56's patchwork of woods.

## 3. C58: dig in, harden a shelter

- `ProjectKind::Shelter { form, because }`: proposed by someone who holds that strangers bring
  harm or caution (beliefs, `mind/beliefs.rs`), or who was hurt, after a recorded cause: a raid
  that took stores or hurt someone (`Raided`, a wound with its cause), a beast seen near home, a
  winter death from cold, a breach (`Breached`). The record names the event.
- Two forms: dig a hollow (rooms, a narrow mouth, a barred door) into the nearest rock face or
  hillside; or harden what exists (a cavern or gallery: props by `prop_first`, a wall across the
  mouth, a door), then beds and stores moved in by the planner.
- A badly shored shelter can fall in (`underground.rs` `collapse`), recorded like any cave-in.
- Cavern settling (the C19 gap): cave timber (`FUNGUS_WOOD`) is felled when surface wood is short,
  and kinds of the caverns (Murrow, Holm) settle there by habitat when they join or arrive.

Gates: `they_dig_in_after_a_raid`, `no_raid_no_shelter`, `a_shelter_is_shored_by_one_who_knows`,
`fungus_wood_when_the_trees_are_gone`.

## 4. C59: the land's own materials, and a trip to fetch them

- An exotic biome as a new family. Options for Dan (one to choose before C59 starts):
  1. Ashwood barrens: grey trees that grow on old burns, black porous cinderstone; arises where
     the world's history has a great fire or war and the climate is dry. Wood light and quick to
     burn, stone light and poor for walls.
  2. Bone forest: pale fungal trees (bonewood) above white milkstone, where an old thin place or a
     deep breach lies under the land; ties to the existing thin place and the deep.
  3. Glass fen: reeds and trees with clear, brittle wood (glasswood) in shallow warm water over
     green glassy stone; arises in warm wet hollows near old volcanic rock.
  Recommended: the bone forest, because it grows out of systems that exist (the thin place, the
  deep) rather than adding a new cause. Each choice is a world-map biome with its own climate
  rule, tree, wood material, stone material with properties, terrain and plant art.
- `expedition.rs` `Purpose::Fetch { material, site }`: someone proposes bringing back a material
  the community lacks for a wanted home or shelter (stone with no rock face within reach; wood in
  a treeless fell), to a place they know of (a past, a rumour, a realm site's stores, a valley
  spot seen on a journey). The caravan comes back with it, and it is used.

Gates: `a_fetch_names_what_was_lacking`, `the_strange_wood_builds_strange_walls`,
`the_biome_arises_from_the_world` (it appears only where its rule holds).

## 5. C60: moving camp

- `ProjectKind::Move { to, because }`: proposed after a recorded cause (hunger across seasons,
  repeated raids, deaths, cave-ins and breaches; floods when M6 brings water) by someone the cause
  touched. Each member decides to follow or stay, by opinion of the proposer, their own memories
  of the cause, and what holds them (graves of kin, a cherished thing, a home they built); every
  choice is recorded with its reason. Many on both sides is a split with this cause.
- Where: a place they know: a spot seen on a journey, a ruin from the history pass, a cavern for a
  hollow-building kind, or off the map to a realm site of their own people or empty land.
- In the valley: they carry what they can, strike tents, leave the old site as a ruin with its
  traces.
- Off the map: they leave by the edge and become a realm site (`realm/sites.rs`), simulated after
  they go (`realm/live.rs`). If everyone leaves, that is the third ending: a recorded event and an
  ending card naming who went, why and where. Dying out gets its card in the same checkpoint.

Gates: `a_move_has_a_proposer_and_a_cause`, `those_who_stay_say_why`,
`a_community_that_leaves_becomes_a_site_of_the_world`, `the_valley_left_is_recorded`,
`dying_out_is_recorded`.

## Current environmental response direction

Homes are not only beds and walls. People judge shelter through their own observations and
tolerances: flooding, dampness, shade, ventilation, thermal mass, fire risk, wind, darkness and
distance to water or stores. A desert household may choose shade, thick walls, a cellar, a night
schedule or a water trip; another may tolerate heat longer or resent the same work. A porous wall
can become a warning before it becomes a breach, and the warning may be noticed by one person and
missed by another. These responses belong to the situated-minds and local-fluids work, not to a
global house optimizer.

## What stays the same

Saves stay compatible (new fields with defaults, appended enum variants); the tester gets a set
per checkpoint; each checkpoint runs the four 300-day soaks and compares with C55.
