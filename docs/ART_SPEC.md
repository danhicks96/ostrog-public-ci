# Art specification (input to the generators; the ART_BIBLE records what was achieved)

Top-down 2.5D pixel art. 32 px tiles. People are composed in 48×48 cells, with adults about
40 px tall and children about 30 px; feet sit at the cell bottom minus 2 px. Camera is
RimWorld-like: straight down with a slight front-face cheat so walls show a 6 px "face" strip
at their south edge and people show faces. Layered top-down 2.5D is the current working
presentation; a future renderer experiment may change scale only if the object language,
simulation-backed material truth and readability gates remain intact.

## Palette
One master palette `godot/assets/palette/ostrog.json` + `ostrog.png`: eleven hue ramps × 8
steps (88 colours) plus 4 neutrals (black, dark shadow, light, white) = 92. Ramps: earth
(browns), stone (cool greys), foliage (greens), water (blue-teal), warm (reds/oranges for fire,
berries, cloth), cool (purples/blues for cloth, ore glint, night), skin (6 skin tones + 2 hair).
Every generated pixel must be one of these colours (checked by tools/art/palette_check.py).

## Terrain (godot/assets/terrain/<family>.png, normally 8 variants in a row, 256×32)

Grass and autumn grass use 24 variants in a row to reduce repeated ground motifs across large
open areas. The renderer selects variants deterministically from tile coordinates.

High-coverage natural ground materials use fewer isolated flecks and tightly packed motifs,
so large views read as terrain before they read as texture. Stone, soil, gravel, moss, ash,
scree, frost, peat and mud retain distinct material signatures at map zoom.
Families: grass, soil, clay, sand, gravel, pale_stone, warm_stone, grey_stone, dark_stone,
black_stone, ore_iron, ore_copper, ore_coal, water (4 animation frames × 2 variants),
deep_water (4 frames), wood_floor, stone_floor, thatch, peat, snow, mud, cave_floor.
Texture, not noise: grass has tufts and a few flowers (1 in 8), soil has clods and pebbles,
stone has cracks and lighter flecks, ore shows a vein of glinting colour. Variation must tile
seamlessly in any arrangement (no visible repeating edge pattern).

Edge blending (godot/assets/terrain/edges/, tools/art/terrain.py): per blending family a 640×128
sheet of 20 pieces × 4 rows (edges N E S W, diagonal-only corners, round fillets, mid fillets and
chamfers, each NE SE SW NW), cut from the family's own texture with an irregular, dithered
boundary that meets every tile seam 6 px deep. `shore.png` is the wet line under banks on water;
`snow_cover.png` is the union of the pieces for each of the 256 neighbour sets, so falling snow is
laid once over spilled ground. Order (lowest first): water, the stones, gravel, scree, mud,
wet_soil, sand, clay, peat, packed_earth, soil, frozen_soil, mossy, grass, wildflower, ash, snow.
Built floors, cave floor and ores keep hard edges. On flush ground the later family spills; across
a one-level step the upper ground's lip spills over the lower tile whatever the order (ramps take
part as the ground they show), so a family change on a contour is not a square edge. Every ground
family the generator lays must have a place in both BLEND_ORDER lists and an edge sheet;
`crates/hf-sim/tests/biomes.rs` (`every_surface_family_blends`) fails otherwise.

### Which families are ground and which are looks (decided 2026-10-17, issues #44 #52 #58)

The generator chooses ground by place (`crates/hf-sim/src/world/gen/biome.rs`): temperature,
moisture and stoniness fields, each biased by the world's character (wet/dry, warm/cold,
stony/soft, drawn from the seed), classify every column into a biome, and the biome picks the
surface material. A family is a **material** when where it lies is a fact about the place that
lasts; it is a **client look** when it is the weather or the season acting on ground that is
something else underneath. `ostrog describe --seed N` lists each family's count and why it
is present or absent.

| Family | Kind | Where / when |
|---|---|---|
| grass | material | grassland, meadow edges, woodland, fen tussocks, fell |
| wildflower | material `meadow` (21) | meadow; rare patches in grassland. Shows as grass in autumn and winter |
| mossy | material `moss` (22) | damp woodland floor and stream banks; fen moss |
| packed_earth | material (23) | heath and stony uplands; grassland patches in a dry valley |
| frozen_soil | material (24) **and** look | fell and snowfield; grassland patches in a cold valley. Also the look of bare soil, clay, wet soil and mud in a hard frost |
| wet_soil | material (25) **and** look | fen; grassland patches in a wet valley. Also the look of bare soil and clay in rain |
| scree | material (26) | the foot of rises in stony uplands, fell and heath; stony patches |
| peat, mud | material | fen (mud at the water's edge) |
| sand | material | heath, beside water in grassland and meadow |
| gravel | material | stony uplands, fell; grassland patches in a stony valley |
| snow | material | snowfield (cold heights). Falling snow over other ground stays a look |
| soil | material | bare woodland floor; tilled and burned ground |
| water | flag | streams and lakes on the local terrain's lowest ground; more lake in a wet area |
| deep_water | flag `DEEP` | lake columns two or more tiles from any shore; the outer ring of deep water takes the shallow water at half strength |
| grass_autumn | look | grass in autumn (drawn instead of grass, edges included), not a family of its own |
| ash | look | burned ground for ten days |

### Liquids and wet materials

Water and future liquids are rendered from simulation-backed state, not from a fixed colour per
tile. Depth, body type, flow/current, flood stage, temperature/ice, murkiness, pollution,
underground context and lighting may each affect colour, opacity, animation, glints or shore
treatment only when the corresponding state exists and is sent to the client. Unknown state uses
the neutral family treatment. Salt, foul, brackish or polluted appearance must not be implied by
art until the simulation and inspection contract names that fact.

People occupy solid support, wade, swim, float or fall only when the simulation says so. The
renderer must never show a person walking on unsupported water or lava. Porous materials may show
dampness, seepage, staining, condensation or pressure marks only when adjacency, permeability,
pressure, fluid and temperature state support the cue; the cue is evidence available to an
observer, not an omniscient overlay.

## Overlays (godot/assets/overlays/*.png)
- `cliff_face.png`: 32×32 face strip for an elevation step (south edge), 3 variants.
- `cliff_shadow.png`: soft shadow cast onto lower ground on N/W edges, 4 edge pieces + 4 corners.
- `wall_face.png`: the south face of a built wall (6–8 px), per wall material: wood, stone block, thatch.
- `edge_blend.png`: 4 edge + 4 corner alpha masks (unused since terrain/edges/ replaced it).
- `strata_face.png`: eight transparent 32×16 underground natural-rock face details. The client
  draws these only on an exposed natural face below the surface; the underlying family still
  determines the material and colour.
- `cave_seepage.png`: eight transparent 32×32 damp channels. The client draws these only beside
  a real water tile, as a visual observation cue; it never implies salinity, pollution or water
  quality that the simulation does not model.

## Walls and structures (godot/assets/structures/*.png)
Wall tops for wood, stone block, thatch: 16 autotile pieces (4-bit N/E/S/W adjacency) so walls
connect. Door (closed/open), window. Floors are terrain families. Furniture at 32×32:
bed, table, chair, stool, chest, shelf, hearth (3 frames fire), workbench, loom, anvil,
cooking pot (2 frames), barrel, crate, stockpile marker, well, fence post.

## Plants (godot/assets/plants/*.png)
Trees: oak, birch, pine — 3 growth stages (sapling, young, mature); mature is 64×64 with the
trunk base in the tile and the canopy overlapping north. Bush, berry bush (bare/fruit), reeds,
grain (4 stages), root crop (4 stages), wildflowers, mushrooms (cave), stump, felled log.
Every plant kind the simulation has is placed by the generator by biome (`Biome::flora`):
herbs (heath, meadow; foraged as roots-and-herbs, which the hearth makes into poultices), fern
(damp woodland and fen), vine (the foot of rises, stony uplands), root crops (wild turnips:
grassland and heath, foraged as food), flowering plant and wildflowers (meadows), fallen log
(woodland deadfall). Staged sheets `<name>_0..2.png` are drawn by growth (< 0.35, < 0.8,
grown); reeds grow through `reed_0..2` and a grown stand is `reed_2` or one of the three
clumps in `reeds.png`. A felled tree leaves a stump (`stump.png`) and one in three leaves its
crown beside it as a fallen log (`log.png`). Tree species follow the biome: pines on cold
ground, birch in fen, oak and birch elsewhere.
Seasons: oak and birch have autumn and bare winter canopies, pine is snow-laden while snow
lies; shrubs, berry bushes (bare and fruiting) and ferns have `_autumn` and `_winter` sheets;
reeds stand as snowed straw in winter; flowering plants die back to their first stage and
wildflowers are not drawn in winter. A missing season sheet falls back to the plain one.

## Inhabitants (godot/assets/people/*.png), composed at runtime
Layers share one parametric skeleton and one 48×48 cell (64×64 taur; 96×96 huge/serpentine),
with four directions (S, W, E, N) and 48 frames: idle(4), walk(6), swing/stoop/tap work(3 each),
hold(1), carry(6), sleep/sit(1 each), strike(3), hit/fall/lying(1 each), talk(3), eat(3),
sleep breathing and stirring(2), tend(3), drink(3).
`tools/art/people.py` and `swaps.json` are authoritative for exact column order and plan cells.

- Body: slim/average/broad builds × child/adult, in skin swap colours. Species and body plans
  supply authored heads, parts and proportions; `silhouette_check.py` compares every kind at 1×.
- Hair: 10 bounded styles in hair swap colours; elders use the existing grey/white hair state.
- Clothing: tunic, dress, apron, cloak, coat, work shirt, robe and vest in cloth/trim swap
  colours. Layer order is body, clothing, hair, bandage, equipment; each overlay is redrawn for
  every build, age and plan so it stays in register. Held equipment follows the actual activity,
  and no spear is drawn in an eater's raised hand.
- Variation comes only from the simulation's existing `Appearance` indices, species/body plan,
  age, clothing/equipment and injury state. The renderer adds no random markings, colours,
  personality, emotion, wound severity or social role.
- Faces use the master palette, darkest-neutral outline, upper-left light and a small detail
  budget: eyes, nose, mouth and lower-lid/chin contrast rather than texture noise. Talk frames
  change the mouth and hand only for `Activity::Talking`; eat frames raise and lower the hand
  only for `Activity::Eating`. Neither cycle implies an emotion or social outcome.

Silhouettes must differ clearly between builds and outfits at 1× on a 1080p second monitor.
Face contrast and 1×/2× contact-sheet review are separate gates; a passing silhouette check
does not prove facial readability on every tone or background.

## Animals (godot/assets/animals/*.png)
Deer, boar, wolf, hare, goat, chicken: 4 directions × idle(1), walk(4), sleep(1).
Goat is a registered species (2026-10-17): wild goats live only on stony uplands and fells,
graze, breed and are hunted. Chicken is not registered: it has no wild role, so it waits for
livestock rather than being scattered over the egress point as wildlife.

## Items (godot/assets/items32/*.png, with legacy fallbacks in items/)

The current ground and carried-object art uses 32 px material-aware icons from `items32/`.
The older `items/` sheets remain 16 px fallback icons and must stay palette-checked. Current
families include raw resources, bars and ores, tools, weapons, armour, clothing, food and drink,
containers, furniture and records; the exact simulation mapping is tracked in
`docs/OBJECT_CATALOG.md`, not by adding one hand-drawn sheet for every material or condition.
Material, quality, wear, damage and provenance marks are compositional overlays or palette
variants wherever the silhouette remains readable.

## Lighting and weather
Day-night is a CanvasModulate curve; lamps and hearths are PointLight2D with a 128 px soft
texture (`godot/assets/light/soft.png`). Rain and snow are particle overlays. Underground:
black base, hand-lit.

### Production acceptance

An accepted art change contains real runtime assets or the code that consumes them, provenance,
palette/dimension validation, a focused guard when applicable, and contact-sheet plus integrated-
game review evidence. A reference sheet or renderer mockup alone is not production art. Effects
are inert until an explicit client-visible event exists; unknown effect names must not create
simulation state.

## Review
`tools/art/contact_sheet.py` renders every sheet at 1× and 2× with labels to
`shots/out/contact_*.png` for human-style review. Nothing is accepted from a generator until
it has been looked at.
