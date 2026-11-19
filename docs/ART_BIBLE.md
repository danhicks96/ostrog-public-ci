# Art bible (written from the Phase 1 representative scene, 2026-10-13)

The quality actually achieved, which later work must not fall below, and the gaps that are
owed. Review record: docs/reviews/2026-10-13-phase1-representative-scene.md. The current
visual direction is recorded below separately from this historical baseline.

## Current direction (D64, 2026-10-30; quality target D58)

The target is a warm, high-fidelity presentation between Animal Crossing's approachable
material warmth and the high-definition RimWorld/DF mod space: richer silhouettes, material
read, wear, lighting, animation and object variety, while keeping Ostrog's 32 px tile
clarity, palette discipline and second-monitor readability. This is a quality target, not a
claim that the overhaul is complete.

The current 32 px material-aware item sheets, 24-variant ground sheets, irregular thatch pass and
the production slices in the #414 art PRs are real implementation work, not mockups. Layered
top-down 2.5D is the current working presentation; a renderer change may be evaluated later, but
it must preserve the same readability and material truth. A mockup is direction only and never
counts as a production asset checkpoint. The compositional object language in
`docs/OBJECT_CATALOG.md` is the way to reach breadth without a separate sprite for every
material, quality and condition combination.

## Fixed rules
- 32 px tiles; people are composed in 48×48 cells (adults about 40 px tall, children about 30 px);
  item art is 32 px with a 16 px legacy/fallback scale where the client still needs it.
- One current palette: godot/assets/palette/ostrog.json (92 colours, palette v2; the earlier
  60-colour baseline is historical). `tools/art/palette_check.py`
  must pass for every sheet under godot/assets (exempt: light/soft.png, alpha masks).
- Light from the upper-left; 1-px darkest-neutral outline on figures, furniture and trees;
  cast shadow blobs under trees; no bloom, no particles as a substitute for readability.
- Walls: top texture distinct from floors, lit N/W bevel, dark S/E bevel, 12 px south face
  overlapping the tile below. Natural elevation: soil-lip/strata/boulder faces by material.
- Underground: near-black but materially readable base, lamp light radius 4.5 tiles, hearth 6
  tiles, warm tint on lit tiles; darkness should preserve enough rock and floor texture to show
  the shape of an unlit space.
- Day-night: full daylight 08:00–18:00, two-hour ramps, night floor 72 % dark; lamps lit at
  night and underground.
- Light and colour are a full-screen grade (godot/shaders/grade.gdshader, 2026-10-16): smooth
  point-light falloff in world space, warm low sun, blue night that keeps values, earthy dim
  underground. The grade may leave the palette; sprite sheets may not.
- Sun shadows: every tree, bush and standing structure casts its own silhouette, flattened and
  leaned away from the sun (west in the morning, east in the evening), none at night; soft
  blobs under people, animals and items. Canopies and stems sway with the wind (sway.gdshader).
- Weather is particles in world space (rain with splashes, snow, autumn leaves), dimmed by the
  grade like everything else.
- Effects are brief, local and readable: generic contact/wound accents, weapon contacts only
  from explicit combat events, material-tinted work chips, compact ignition/embers, and cool
  resonance around an already-modelled thin place. They may decorate a fact the client receives,
  but never imply damage, a projectile, a psychic power or an environmental change that the
  simulation did not report.
- Entities sort by y; canopies may overlap the tile north; nothing overlaps the HUD.
- Familiar silhouettes orient the player; alien biodiversity supplies discovery. Strange forms
  still need readable ecological roles, material identity and state changes.
- Post-arrival structures should visibly show sourcing, tile-by-tile assembly, substitutions,
  repairs and construction stages. Ruins may show older symmetry, collapse, weathering and
  archaeological layering because they predate the arrival.
- Liquids are not locked to one colour or one tile. Their appearance may vary with simulation-
  backed depth, body, flow, temperature, murkiness, pollution and lighting. Unknown state falls
  back to a neutral treatment; art never implies an unsupported quality.
- Dampness, seepage, staining and pressure marks are observation cues, not x-ray overlays. They
  appear only when the relevant material and liquid state support them, and people may miss or
  misinterpret them.
- The visual state language includes intact, worn, damaged, broken, partial, improvised, ruined,
  repaired and perfect/complete objects. Perfect describes completeness, not workmanship quality.

## Achieved (historical baseline)
Terrain families (22), overlays, 3 wall materials with 16-piece autotiles, doors/windows/
stairs/ladder, 17 furniture pieces, 3 tree species × 3 stages, bushes/reeds/grain/root/
flowers/mushrooms/stump/log, 14 item icons, 6 body sheets, 10 hair, 8 outfits, 7 equipment,
6 animals, palette-swap scheme (6 skin, 8 hair, 12 cloth). All generated, all on palette,
all listed in tools/art/provenance.toml and provenance_people.toml.

## Achieved in Phase 13 (2026-10-15)
Wall tops: logs with bark (wood), fieldstone in mortar (stone), packed straw (thatch).
People: season/role/journey outfits, grey and white hair for elders, bandage overlays per
build and age, tool-in-hand overlays, a second idle frame. Animals: ear flick frame.
Weather: rain and snow particles above ground, snow cover on living ground, seasonal ground
tint, broadleaf trees tinted by season, canopy sway. Doors open when occupied.

## Character everyday-action slice (#485, 2026-10-31)
People sheets now carry 40 frames: three talking poses and three eating poses follow the 34
existing frames. Every body plan, build, age, clothing, hair, equipment and bandage layer is
regenerated on the same skeleton. The bridge selects them only for real `Talking` and `Eating`
activities; movement, combat, fresh wounds and bodies retain priority. Eaters do not hold a spear
in the raised hand. The activity-driven poses add no simulation state, appearance facts or world
digest input. Contact and animation-strip evidence is in
`docs/reviews/character-everyday-485/`; this slice does not close #485's full roster, injury,
background-review or performance acceptance.

## Character everyday-action slice 2 (#485, 2026-11-02)
People sheets now carry 48 frames. Sleep is a cycle of three (at rest, breathing in with the
uppermost side of the chest a pixel fuller, stirring with the head rolled a pixel), staggered per
sleeper so a room does not breathe in step. A healer working a Tend order kneels and dresses with
both hands low and forward (tend1-3) and carries neither the builder's bundle nor a spear. A
person drinking bends to the water, lifts cupped hands and drinks (drink1-3). Each is selected
only by the real activity or order; no item, emotion or injury severity is drawn. Evidence:
`docs/reviews/485/`.

## Historical baseline gaps (tracked as work, not accepted as done)
1. (done 2026-10-15) Wall tops; still owed: open-door rework, roofs seen from above.
2. Lit top edges on natural hill masses. (Edge-blend between ground families done 2026-10-16:
   docs/reviews/edge-blend-2026-10-16; since 2026-10-17 the upper ground also spills over a
   one-level step and ramps take part, because biome families change on level steps.)
3. (done 2026-10-15: head turn, ear flick; people shadows 2026-10-16; 2026-10-16: 6-frame walks
   with contact/passing and a 1 px bob, swing/stoop/tap work cycles, a carry walk, every
   person layer drawn per build and age, animal legs that alternate, the bear at 48x48;
   docs/reviews/animation-2026-10-16/). Still owed: tools held in front hidden when facing N.
4. Surface variety: mud after rain (snow cover done 2026-10-15; mushrooms done Phase 9b;
   wildflower meadows, frost, wet soil, moss by water and scree under rises done 2026-10-16,
   client-side; 2026-10-17 the generator places meadow, moss, scree, packed earth, frozen soil,
   wet soil, peat, mud and snow by biome, and stumps and fallen logs are left where trees are
   felled: docs/reviews/terrain-reachable-2026-10-17/). Bare winter canopies drawn, not tinted.
5. Stone terrain cleanup (crack/fleck motifs only).
6. (particles done 2026-10-15; structure and tree sun shadows done 2026-10-16).
7. Faces readable at 2× (eyes + mouth contrast on dark skin tones).

## Style bible v2 (M2, from 2026-10-18)

Decided from the first pipeline experiments (docs/reviews/art-pipeline-2026-10-18, sources in
tools/art/sources/reference/ with their prompts and provenance).

`tools/art/sources/prompts.json` is the successful-prompt library: exact prompt, model, service,
creation time, source hash and kept sprites for every adopted derived sheet. Start each new
reference-sheet prompt from the closest kept row, following `docs/art/ART-GUIDE.md`;
`tools/art/prompt_library_check.py` keeps the library and derived provenance in agreement.

### What each source is for
- People stay procedural (tools/art/people.py): one skeleton drives every layer, so all frames,
  directions, builds, ages, outfits and hair styles stay in register. A frontier image model asked
  for a 4-direction, 6-frame walk sheet kept the character's identity across 32 frames but drew
  8 columns, faced two rows the same way and barely moved the legs: good reference, not usable
  animation. Generated sheets set the proportions, face construction and shading the procedural
  people are redrawn to at 48 px.
- Things with few frames come from generated reference sheets, cut and quantised by
  tools/art/derive.py: items, furniture, props, structure details, creature key poses, and later
  portraits. One request returned 38 clean item icons (asked for 30). This is where the breadth
  targets in docs/PLAN-FINISH.md M2 are met.
- Terrain and plants stay generated by code (tools/art/terrain.py, plants.py): they tile, blend
  and vary by seed, which a picture cannot.
- A local open-weight image model (offline, on the box) is for bulk variation where a frontier request per
  asset would be wasteful; frontier models (an image-model service, off-box, prompts only) for reference sheets.

### Key colours
A sheet meant for palette swapping is requested with each swappable region in one saturated key
hue: clothing pure green, hair pure magenta, skin natural; derive.py `--keys
cloth=green,hair=magenta,skin=natural` assigns each colour cluster to its role and paints it in
that role's four swap source colours (godot/assets/people/swaps.json), darkest to lightest.
Non-role colours snap to the palette without any swap source colour, or the runtime swap would
recolour them too (a face once turned moss green). Planned for M4: metal parts keyed pure cyan so
one tool sheet becomes iron, copper, bronze and gold tools.

### Fixed rules (v2)
- People: 48x48 cells, adults about 40 px tall, children about 30; faces readable at 2x (eyes and
  mouth contrast on every skin tone); the 1 px darkest-neutral outline and upper-left light stay.
- Items: 32 px icons (16 px before), with material and quality variants by swap and overlay.
- Palette v2: every existing sheet remains valid while the palette gains the ramps the first
  sheets showed missing: gold and copper metals, gem accents (ruby, sapphire, emerald, amethyst)
  and an olive dye. The current palette is 92 colours; the earlier 60-colour count is historical.
  Gold currently snaps to pale orange.
- Every derived sprite carries provenance: source sheet hash, model, service, prompt, licence,
  crop and drawing block size (derive.py writes it beside the output).
- Off-box cost, named: a frontier request sends the prompt text (never project files) to an image-model service
  and the vendor it routes to; about 4,000 to 10,000 credits a sheet.
