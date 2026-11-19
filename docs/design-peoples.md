# Design: Peoples of the World

Milestone MP, written 2026-10-18 after C36 merged; decided the same evening (section 15).

The aim is a world as rich in peoples as Dwarf Fortress with a RimWorld modlist of twenty
races. That means:
- a library of dozens of sentient species;
- every world drawing its own mix, which its history then winnows;
- every people with its own body, culture, stories and reactions;
- curses, ghosts and life in the depths.

The project's rules hold throughout:
- deterministic simulation;
- no LLM in the simulation;
- nothing invented (every story, reaction and condition traces to a recorded row);
- pure observation (no orders);
- losing is fun.

The design builds on what exists today. Section 0 lists those facts with file references, so
each later section can say exactly what changes.

## Contents

0. Where we start
1. The species library
2. Three levels of body: plan, species, individual
3. Art as part sets on a rig
4. World selection and the wizard's peoples page
5. Cultures with their own stories
6. Reactions that differ by culture
7. Relations between peoples, and other races in the valley
8. Conditions with recorded causes: curses and ghosts
9. The depths
10. Organic, not planner-driven
11. The depth gate and the new simulated axes
12. The readability gate
13. Performance and size
14. Roadmap
15. Decisions for Dan

---

## 0. Where we start

Paths are relative to `crates/hf-sim/src` unless stated.

Creatures:
- Every creature is one `Agent` struct (`agents.rs:43`).
- `Species` is an enum of 14 variants: Human, Deer, Boar, Wolf, Hare, Fox, Crow, Bear, Duck, Goat,
  Hart, Bat, Crawler, Burrower (`agents.rs:10-31`). The client receives it as an int whose order
  matters (`godot/scripts/world_view.gd:60`).
- "Is a person" is written as `species == Species::Human`, 73 times in 27 files. Needs, health,
  ageing, society, beliefs, learning and cold are all gated on it. Births always produce a Human
  (`scenario.rs:140-150`).

Genome and appearance:
- The genome is 14 loci of one byte each (`genetics.rs:17-32`): skin, hair colour, build, hair
  style, four temperament loci, four aptitudes, hardiness and longevity.
- A child takes each locus from one parent at random; 8% shift by ±24 and 1% re-roll.
- There is no size: build is one of three silhouettes. Lifespan is 50 + longevity × 20 years.

Bodies:
- `body.rs` knows exactly six parts: head, torso, two arms, two legs.
- Wounds (cut, bruise, break, burn, frostbite) are applied by about a dozen callers. Each passes
  a hard-coded human `Part` (`systems/animals.rs:430`, `hazards.rs:375`, `work.rs:378`, and
  others).
- Healing and illness run for humans only.

People art (`tools/art/people.py`):
- Cells are 48 px, drawn on one parametric humanoid skeleton.
- Sheets are 28 columns × 4 directions (W is E mirrored). Frames: idle ×3, glance, walk ×6,
  swing ×3, stoop ×3, tap ×3, hold, carry ×6, sleep, sit. There are no fight or fall frames.
- Layers are body, clothing, hair, bandage and equipment, each for 3 builds × 2 ages. That makes
  168 PNGs, recoloured on the CPU from `swaps.json` against the 92-colour palette.
- Animals have 9 frames at 32 px, with deer, hart and bear larger.

The world:
- The realm (`realm/peoples.rs`) has 4 to 7 peoples. Each has a generated language
  (`realm/language.rs`, a port of the prototype's `word_for`), a homeland, a way of living
  (Farming, Herding, Hunting, Fishing) and convictions (one value per belief topic).
- History runs a year at a time: sites, figures, polities, wars (`realm/sites.rs:442`,
  `figures.rs`, `war.rs`).
- Measured: 1,000 years with 72,700 figures in 2.6 s; an opening world with 500 years in about
  250 ms.

Beliefs:
- A belief has a stance, a confidence and up to 6 sources (`Event`, `Person`, `Ideology`), on 7
  topics: Drink, Violence, Fire, Hunger, Strangers, Death, Trust (`past.rs:29`,
  `mind/beliefs.rs:21-41`).
- Children inherit at reduced confidence. Customs (`culture.rs`) carry an origin event.
- Thoughts carry an optional cause event (`mind/thoughts.rs:135`).

Stories:
- Tales (`lore.rs:132`) hold a claim about an old site, a teller and how it was heard.
- Tales drift on retelling with a fixed map (famine to curse, fever to plague, fire to
  lightning, moved to driven out). `Tale::true_to` checks a tale against the record.

Senses:
- Sight only: radius 12, line of sight on one z-level (`mind/perception.rs:7`).
- Darkness does not reduce sight. In the dark, work is slower and people have the Dark thought
  (`underground.rs:285-375`).
- There is no hearing or smell.

The depths:
- Three strata (`world/gen/strata.rs`), worm caves, and two-level caverns in the middle stratum
  with fungus forests.
- Bats, one crawler and one burrower. No peoples live below.

The planner:
- `systems/settlement.rs` `plan_community` posts most work by rule: stockpiles, hauls, chopping,
  mining, the hearth, houses and beds, fields, hunts, cooking, workshops, a 27-entry recipe wants
  table, brewing, veins, carving, clay, roofs.
- Caravans form by rule in `systems/expedition.rs` `consider_new`.
- Personal undertakings exist only as owned designations: fire, a beast that hurt someone, study,
  tending, lamps, props, fey moods, ward stones.

Budgets in force:
- `history_pass_budget`: an opening world under 2 s.
- 1,000 realm years under 10 s; 20,000 figures under 30 s.
- Soak memory: start + 1 MB per person + 24 MB.
- DoD: 2 ms mean tick at 300 people at 10x, save under 150 MB, chronicle under 500 MB per 100
  game-years.

---

## 1. The species library

### 1.1 What a species is

A species is data, not code. The library is a folder of TOML files that the simulation loads at
start:

```
data/peoples/plans/*.toml      body plans (section 2)
data/peoples/species/*.toml    hand-made species
data/peoples/generator.toml    ranges and weights for generated species (section 1.4)
```

TOML because the project already reads and writes it (`tools/art/provenance*.toml`) and it diffs
cleanly.

The loader:
- validates every file: ids unique, ranges ordered, every named part exists in the plan, every
  colour ramp in the palette;
- builds a `Library` of `SpeciesDef`s indexed by `SpeciesId(u16)`;
- hashes the whole folder (sha256 over sorted file bytes).

The hash goes into the world's setup record. A save made with one library refuses to load
against another unless the missing species are carried in the save itself (generated species
always are; see 1.4).

`Species` the enum becomes `SpeciesId`. The 14 existing creatures keep their numbers 0 to 13 as
fixed entries of a built-in `animals.toml`. Old saves map variant names to those ids in
`save/migrate.rs`, so no old save breaks and the client's int order holds. Every
`species == Species::Human` becomes `a.is_person()`, which reads `SpeciesDef.sentient`.

### 1.2 The format, by example

A hand-made anchor in full (the blind cavern people of section 9):

```toml
id        = "murrow"            # stable across versions; never reused
name      = "Murrow"            # anchors keep a fixed name; generated species take a language word
plural    = "Murrow"
sentient  = true
plan      = "biped"             # the body plan it is locked to
art_set   = "biped/murrow"      # part set and colour ramps (section 3)
rarity    = "uncommon"          # default; the wizard can override

[size]                           # adult, before the individual's genome
height_cm = [120, 145]
mass_kg   = [38, 55]

[life]
mature_years  = 14
lifespan      = [60, 85]         # replaces the human 50 + longevity × 20
litter        = [1, 1]
fertile_years = [16, 45]

[diet]
kind  = "fungivore"              # herbivore, omnivore, carnivore, fungivore
foods = { fungus = 1.4, root = 1.0, raw_meat = 0.6, grain = 0.5 }   # nutrition multipliers

[senses]
sight      = 0                   # 0 none, 1 dim, 2 day, 3 day and dark
dark_sight = 0
hearing    = 3                   # 0 to 3: radius 0, 6, 10, 14 tiles through walls
smell      = 3                   # 0 to 3: radius 0, 3, 6, 9 tiles, no line of sight

[climate]
comfort_c = [6, 18]              # cold stress below, heat stress above
coat      = "bare"               # bare, fur, feathers, scales: warmth factor
light     = "shuns"              # seeks, neutral, shuns: light hurts the light-shy

[habitat]
layers  = ["caverns"]            # surface, upper_caves, caverns, deep_caverns, underworld
biomes  = []                     # surface biomes when layers has "surface"
builds  = "hollows"              # houses, hollows, tents, nests: how they make a home

[social]
structure = "clan"               # band, clan, household, hive
bonding   = "pair"               # pair, group, none
leaders   = "eldest"             # eldest, strongest, chosen, none (M5 respect model reads it)
group     = [8, 40]

[temperament]                    # species leanings, the first source of every belief
convictions = { strangers = 0.5, death = 0.4, fire = -0.2 }
values      = { tradition = 0.6, tranquility = 0.5, craftsmanship = 0.4 }
cycle       = "any"              # diurnal, nocturnal, any

[parts]                          # which variants of the plan's optional parts it may show
head    = ["murrow_round", "murrow_long"]
ears    = ["wide_fan"]
eyes    = ["none"]               # cosmetic only; sight above is what counts
tail    = []
skin    = ["pale_grey", "pale_lilac", "chalk"]   # palette ramps
hair    = ["none", "fine_white"]

[living]                         # ways of living open to them in the world layer
ways = ["fungus_farming", "foraging", "delving"]

[relations]
fertile_with = []                # species ids; empty means only their own (section 7)
```

Every field has a default in `generator.toml` and a validator. The format is the same for
hand-made, generated and, later, player-made species. A player's folder
(`user://peoples/species`) is read after the built-in one; its files count in the library hash
like any other. Decision 3 asks when to open that up.

### 1.3 The anchors

Twelve hand-made species carry the world's identity. They are written in our own voice:
analogues of the elf, goblin and kobold roles, not copies.

| Id | Name | Plan | Size | Where | What makes them themselves |
|---|---|---|---|---|---|
| human | Humans | biped | 150-190 cm | surface | the portal party's kind; the baseline every gate compares against |
| seamfolk | Seamfolk | biped (tall, thin) | 175-205 cm | where the world is thin | born only near thin places; few; long-lived (200-300); sense the echoes of the dead (section 8) |
| hollin | Hollin | biped | 165-195 cm | old forest | long-lived (250-400), slow to breed; cutting a living tree is abhorrent; carvings in living wood |
| grisk | Grisk | small biped | 95-120 cm | hills, upper caves | short lives (25-40), large litters, dark sight; scavengers; strangers are prey or kin, nothing between |
| screeling | Screelings | digitigrade, tail | 80-105 cm | upper caves, scree | egg-laying clutches; trap-makers; sleep in heaps; cold-blooded (comfort 18-32 °C, sluggish below) |
| murrow | Murrow | biped | 120-145 cm | caverns | blind; hear and smell through rock; fungus farmers; light hurts them |
| holm | Holm | huge biped | 300-380 cm | deep caverns | cave giants; solitary households; eat stone-moss and meat; eight-hundred-year lives, very slow |
| skarri | Skarri | winged biped | 140-165 cm | cliffs, mountains | hollow-boned, light (30-45 kg); nests on heights; cold-hardy (feathers); cannot carry much |
| tharn | Tharn | four-armed biped | 170-200 cm | hot dry lands | four hands: craft and carry more; heat-loving; long memory for grudges |
| meer | Meer | biped, webbed | 140-170 cm | lakeshore, rivers | amphibious: swim and need water daily; fish-eaters; aquatic competence reduces risk but does not grant drowning immunity |
| gnarrow | Gnarrow | taur (four legs, two arms) | 200-260 cm long | grassland | fast, carry much, cannot climb ladders or pass narrow doors; herders |
| below | The Below | serpentine | 300-500 cm long | the underworld | only in the deepest layer; never trade; their tale is mostly told by others |

The Seamfolk tie the epic to the game's premise. They are the only people born of the thin
place's influence:
- A Seamfolk settlement's founding row names the thin place that drew it.
- Seamfolk who come to the valley feel its portal stone wake. Each phenomenon gives them a
  thought with that phenomenon's event as its cause.
- They are the only people whose stories of the portal are first-hand rather than drifted.

### 1.4 Generated species

The rest of the library is generated per world seed. The default is 30 generated species, so a
world's library is 12 anchors + 30 = 42 (decision 4).

Generation is a pure function of the seed: `stream(seed, [purpose::SPECIES, i])`. For each
species:
1. Pick a body plan by the weights in `generator.toml` (biped 30%, small biped 15%, digitigrade
   tail 15%, winged 8%, four-armed 8%, taur 8%, huge 6%, serpentine 4%, amphibious 6%).
2. Pick values on the simulated axes of section 11 (diet, climate, senses, lifespan, size, social
   structure, way of living, cycle). Each is drawn from that plan's allowed range.
3. Reject and redraw any species that differs from humans on fewer than four behaviour-changing
   axes. The depth gate is satisfied by construction and checked by a test.
4. Derive what follows from those choices:
   - habitat from climate, senses and size: a species with dark sight and light-shyness lives in
     caves;
   - comfort range from habitat and coat;
   - temperament from way of living and social structure: hunters lean Violence +0.2, clans lean
     Tradition, cave-dwellers lean Strangers +0.3.
5. Choose parts and colour ramps from the plan's variants, avoiding the silhouette and colour of
   any species already drawn (the readability rule, section 12).
6. Name it from a fresh language seeded by the species index: `language.word(seed, "people")`.
   The plural follows the language's ending rules.

Generated species are written into the world save in full (about 2 KB each). A save never
depends on the generator's code version to reproduce its species.

Example, seed 7, generated species 3:
- Draw: digitigrade-tail plan; carnivore; comfort 10-26 °C with scales; dark sight 3; smell 2;
  lifespan 30-45; size 110-130 cm and 30-45 kg; social: band of 6-15; way of living: hunting;
  cycle: nocturnal.
- Against humans it differs on diet, senses, lifespan, size, social structure and cycle (six
  axes); climate is close. It passes.
- Derived: habitat upper caves and hills; builds hollows; leans Violence +0.2, Strangers +0.3.
- Named in its language: "Vashk", plural "Vashka".

---

## 2. Three levels of body: plan, species, individual

### 2.1 Body plan

A plan is the skeleton shared by the art rig and the body simulation. It is data:

```toml
id    = "digitigrade_tail"
cell  = 48                      # sprite cell in px; huge plans use 96 (2 × 2 tiles)
tiles = 1                       # footprint in tiles

[[part]]
id = "head"
attach = "neck"
roles = ["sense", "vital"]
wound_weight = 8
layer = "head"
[[part]]
id = "torso"
attach = "root"
roles = ["vital"]
wound_weight = 30
layer = "body"
[[part]]
id = "arm_l"
attach = "shoulder_l"
roles = ["manipulate"]
wound_weight = 10
layer = "body"
[[part]]
id = "arm_r"
attach = "shoulder_r"
roles = ["manipulate"]
wound_weight = 10
layer = "body"
[[part]]
id = "leg_l"
attach = "hip_l"
roles = ["locomote"]
wound_weight = 12
layer = "body"
form = "digitigrade"
[[part]]
id = "leg_r"
attach = "hip_r"
roles = ["locomote"]
wound_weight = 12
layer = "body"
form = "digitigrade"
[[part]]
id = "tail"
attach = "tailbase"
roles = ["balance"]
wound_weight = 8
layer = "tail"
optional = true

[joints]                         # the rig: parent joint and rest offset in cell px
root       = { parent = "",      at = [24, 30] }
neck       = { parent = "root",  at = [0, -10] }
shoulder_l = { parent = "root",  at = [-6, -8] }
# ...
```

The plans proposed:
- biped (the current human skeleton, made data);
- small biped;
- huge biped;
- digitigrade with tail;
- winged biped (arms plus wings);
- four-armed biped;
- taur (four legs, two arms);
- amphibious biped (biped joints, webbed and gilled variants);
- serpentine (a tail in place of legs).

Every plan has the same core parts (head, torso, at least one manipulator pair) so tools, carrying
and crafting keep working. A plan without hands is left out (decision 8).

`body.rs`'s `Part` enum becomes a `PartId(u8)` into the plan. `Body` gains a per-part state
(whole, scarred, lost, replaced) for M5. Every wound caller stops naming a part and asks the plan
for one: `plan.random_part(r, filter)` weighted by `wound_weight`, with `filter` for "a leg" or
"a hand" where the cause needs one (a fall hurts a locomotor part, frostbite an extremity).

That gives one part list for both the sprite and M5's wounds:
- a wounded part draws its bandage on that part's layer;
- a lost part hides its layer;
- a prosthetic replaces the layer with its own sprite.

### 2.2 Species

A species locks one plan and sets the ranges: size, lifespan, diet, senses, climate, habitat,
temperament, and the allowed variants of each optional part and colour ramp. "If Screeling, then
80-105 cm, digitigrade legs, a tail, two arms, scales in one of three ramps."

A species can switch an optional part of its plan off (a tailless digitigrade people). It can
never add a part the plan does not have.

### 2.3 Individual

The genome gains a species and a body block:

```rust
pub struct Genome {
    pub species: SpeciesId,
    pub loci: [u8; 14],     // as now: temperament, aptitudes, hardiness, longevity
    pub body: [u8; 8],      // SIZE, HEAD, PART_A, PART_B, COLOUR_1, COLOUR_2, PATTERN, MARK
}
```

How the body block is read:
- Each body locus maps into the species' range or variant list: `height = lerp(range, SIZE/255)`,
  `head = species.parts.head[HEAD % n]`.
- Skin and hair loci index the species' ramps, not a global table.
- Longevity maps into the species' lifespan range. A Hollin with longevity 128 lives about 325
  years; a Grisk with the same locus about 32.

Inheritance is unchanged in form. Each locus comes from one parent, with the same drift and
mutation, so children resemble parents within the species. Ordinary mutation never leaves the
species' ranges. Rare in-play transformations are a separate, recorded system governed by
`docs/design-minds-observation.md`; they are not revealed by setup and are not ordinary heredity.

Culture dresses the body: clothing, ornament, hair cut, paint, all layers over the body. It never
changes a part. After birth, parts change only by a recorded wound, scar, loss or prosthetic.
Each has the event that caused it, as now.

### 2.4 What size does

Size is mass and height. Size class follows from height:

| Class | Height | Footprint | Examples |
|---|---|---|---|
| small | under 125 cm | 1 tile | Grisk, Screelings |
| medium | 125-210 cm | 1 tile | Humans, Murrow, Hollin |
| large | 210-280 cm | 1 tile, stoops | Gnarrow |
| huge | over 280 cm | 2 × 2 tiles | Holm, the Below |

The simulation reads it in five places.

Food: hunger runs at `(mass / 65)^0.75` of the human rate (Kleiber's law; 65 kg is the human
mean today). The diet multiplies each food's nutrition.
- A 40 kg Grisk gets hungry at 0.70× the human rate.
- A 340 kg Holm gets hungry at 3.46× and needs three and a half times the store.

Carrying: today a haul takes a whole stack up to `max_stack`. With species it takes up to
`max_stack × carry`, where `carry = clamp((mass / 65)^0.67 × hands / 2, 0.4, 4)` and four arms
count as 3 hands, not 4.
- A Skarri (35 kg) carries 0.66 of a load.
- A Tharn (90 kg, four arms) carries 1.86 loads.
- A Gnarrow (220 kg) carries 2.27 loads.

Doors and beds: a door or bed is built for the builder's size class (`Structure.size`, a new
field).
- Small and medium share doors.
- A large person passes a medium door at half speed.
- A huge person cannot pass one, so a Holm household builds 2-tile doors.
- A bed of a smaller class counts as sleeping rough: thought SleptRough, with the bed's size as
  the cause line.

Reach in a fight (M5): reach = 1 tile, +1 for large and huge, which strike first in an exchange.
Small people are harder to hit (M5's hit chance reads size class).

Warmth: `cold_step`'s base loss is multiplied by:
- the size factor `(mass / 65)^-0.33` (small bodies lose heat faster);
- the coat (bare 1.0, fur 0.6, feathers 0.7, scales 1.2);
- the clothing factor as now.

Cold starts below the species' comfort minimum instead of a fixed 2 °C. A new `heat_step`
mirrors it above the comfort maximum: thirst runs faster, work slows, heat stroke.
- A Screeling (35 kg, scales, comfort from 18 °C) at 8 °C loses heat at 1.2 × 1.23 = 1.47 times
  the human rate, and begins losing it at a temperature where humans are comfortable. Screelings
  settle in warm caves and huddle; the soak checks they do.

Worked example, one individual. Ysse, a Murrow girl born in the valley's community:
- Mother: SIZE 40, HEAD 1. Father: SIZE 200, HEAD 0.
- Ysse takes SIZE from her father (200), drifted to 208: height = 120 + (145 - 120) × 208/255 =
  140 cm, mass 52 kg.
- She takes HEAD from her mother: `murrow_long`. COLOUR_1 is from the father: `pale_lilac`.
- Hunger runs at 0.85× the human rate. Carry is 0.86 loads.
- She hears through rock to 14 tiles, smells to 9, and sees nothing.
- The inspect panel shows "Ysse, a Murrow, 140 cm. Her father's build, her mother's long head."
  Each line is read from the genome and the parents' ids, which the record holds.

---

## 3. Art as part sets on a rig

### 3.1 What the generator flips

The client draws a person by stacking layers, as now. With plans, the stack becomes:
1. body (per plan and size class, skin ramp swapped);
2. head variant;
3. optional part layers (tail, wings, extra arms, crest, ears, horns);
4. clothing (per plan, per culture, cloth ramp swapped);
5. hair or crest (hair ramp swapped);
6. wound and bandage marks on the wounded part's layer;
7. equipment.

The toggles of layers 2, 3 and 5 are read from the genome's body block within the species'
allowed list. The player never sets them. This is the character-editor model with the generator
in the chair.

### 3.2 A rig, not hand-tuned poses

Today `people.py` is about 1,500 lines for one plan. Its pose tables hold joint offsets per frame,
tuned at 32 px and scaled. Repeating that per plan (nine plans) means hand-tuning about 13,000
lines of pose data. That is not feasible. The generator needs a skeletal layout:
- `tools/art/rig.py` reads a plan's joints from its TOML.
- Each animation is a set of joint angle keyframes per plan, written once: walk, swing, stoop,
  tap, carry, sit, sleep, plus the new fight set.
- Each part is a parametric draw function attached to a joint, drawn at the joint's position and
  angle in each frame.
- A new optional part costs one draw function and, where it moves on its own (a tail swaying, a
  wing folding), a few keyframes. It never costs a new pose table.

The current humanoid poses become the biped plan's keyframes. The port must reproduce today's
sheets to within a pixel-difference threshold. tests/look.rs gains a sheet-diff check against the
committed 168 PNGs, so the port changes nothing that was already right.

### 3.3 Animation cost per option

Frames per direction after this epic: idle 3, glance 1, walk 6, swing 3, stoop 3, tap 3, hold 1,
carry 6, sleep 1, sit 1, and the new fight set: strike 3, hit 1, fall 1, lying 1. That is 34
frames. Three directions are drawn (W mirrors E), so 102 cells per layer per size class per age.

| Option | Moves on its own? | Keyframes to write | Cells per size and age | Notes |
|---|---|---|---|---|
| head variant | no (follows neck) | 0 | 102 | one draw function |
| ears, horns, crest | no | 0 | 102 | one draw function each |
| tail | sways on walk, curls on sit and sleep | 6 | 102 | the tail is its own layer behind the body in S, in front in N |
| wings | fold at rest, half-open on walk, open on fall | 8 | 102 | separate layer; large in the cell, so the winged plan's cell is 48 with wings clipped to it |
| second arm pair | yes, every action | about 20 (each action's lower arms) | 102 | doubles arm drawing; carry and swing use all four hands |
| digitigrade legs | yes, every gait | about 14 (walk, carry, sit, fall) | 102 | a plan-level change, not an option |
| taur body | yes: four-legged walk, carry on the back, lie down instead of sit | about 30 | 102 at a 64 px cell | its own plan; no sit frame, a lie-down frame instead |
| serpentine | yes: a slither cycle replaces walk; coils for sit and sleep | about 30 | 102 at a 96 px cell | its own plan |
| huge (2 × 2) | as biped | 0 (biped keyframes at scale) | 102 at a 96 px cell | the same rig at double size, plus detail passes |

Sheet count: today there are 168 PNGs for one plan. The layered model keeps the count additive
per plan, not multiplied by option combinations:
- body: 3 size classes × 2 ages;
- heads: about 6 per plan;
- options: about 6;
- clothing: about 6 cultures × 3 × 2;
- hair: 10;
- equipment: 8 × 3 × 2.

That is about 150 to 220 PNGs per plan, about 1,700 for nine plans. At 10 to 20 KB each that is
about 30 MB. Generation runs in tools/art, palette-checked and with provenance recorded
(`provenance_people.toml` gains plan and option fields), as every art pass does today.

Cell sizes: the huge and serpentine plans need 96 px cells and the taur 64. The entity layer
already reads cell size from the sheet and offsets 48 px cells onto 32 px tiles
(`_person_offset`). That offset becomes a per-plan anchor read from the plan file.

Portraits and hero art (the local image pipeline) belong to M7. Sprites never depend on them.

---

## 4. World selection and the wizard's peoples page

### 4.1 Presence and rarity

Two separate settings per species.

Presence (in or out) decides whether the species exists in this world's history at all.

Rarity, if present, decides how it enters:

| Rarity | In the world's history | What the valley might see |
|---|---|---|
| common | one or more homelands; founds and spreads | neighbours, trade partners, wars |
| uncommon | one homeland, small capacity | occasional caravans and wanderers |
| rare | no homeland; a few households among other peoples' sites, or none at all after history | a wanderer, a caravan member, a figure in an old tale |
| legendary | a homeland at the start that history is weighted to end (capacity falls each century) | ruins, graves, carvings and tales only, unless the dice keep them alive |

Rarity sets the starting conditions, never the outcome. History decides who survives, spreads
or dies out. A common people can be wiped out in a war, and a legendary one can hold on. Either
way the result is rows: Ended, Taken, Famine, Plague. The ruins and tales follow from them as they
do now.

### 4.2 The draw

`realm::peoples::generate` today picks 4 to 7 peoples with random ways of living. It becomes a
draw from the library:
1. Candidates = library species that are present, sentient, and not excluded by species or by
   plan.
2. The number of peoples comes from the setup (default 5 to 9; range 2 to 16).
3. Draw without replacement, weighted by rarity (common 6, uncommon 3, rare 1, legendary 1). The
   draw stream is `stream(seed, [purpose::PEOPLES, k])`.
4. Each drawn species gets a homeland region its habitat allows. Cave peoples take a cavern
   region of the deep map (section 9). If none fits, the species is drawn as rare instead.
5. A species may be drawn twice as two peoples with different languages and convictions. Humans
   on two continents are two peoples. The chance is 15% for common species when the count
   exceeds the candidates.
6. Rare species are seeded as households inside other peoples' sites at the start (a "minority"
   list on the site).

### 4.3 The setup record

`WorldSetup` gains:

```rust
pub struct PeopleSetup {
    pub library: [u8; 32],                 // the library hash
    pub include: Vec<SpeciesId>,           // explicit includes (empty = all)
    pub exclude: Vec<SpeciesId>,
    pub exclude_plans: Vec<PlanId>,
    pub count: (u8, u8),                   // peoples in the world
    pub rarity: Vec<(SpeciesId, Rarity)>,  // overrides
    pub generated: Generated,              // Off, Some (12), Many (30)
}
```

It is recorded in the `SetupChosen` event, so the setup is part of the world's own history. A test
`same_peoples_same_world` builds the same seed and choices twice and compares digests; another
changes one rarity and expects a different world.

### 4.4 The wizard page

A new page, "The peoples of the world", sits between "The valley" and "Who came through".
- Left: the library as a list of cards. Each card shows:
  - the species' silhouette at 2x;
  - its name and plan;
  - a size bar against a human;
  - lines such as "eats fungus · blind · hears through rock · lives 60-85 years";
  - an include toggle and a rarity selector (common, uncommon, rare, legendary).
- Filters: by plan (toggle whole plans off), by habitat, anchors or generated.
- Top: the count of peoples (a two-handled slider) and generated species (off, some, many).
- Right: Preview. It runs the realm generation to year 0 (the draw and homelands only, about
  20 ms) and shows the drawn peoples on the world map in their colours with a card for each.
  Begin runs the full history as now.
- Re-roll redraws only the generated species. It spends one of the existing rerolls and is
  recorded.

Humans always come through the portal (decision 7). The page decides who else is in the world.

---

## 5. Cultures with their own stories

### 5.1 People and culture

A people is a species with a language and a homeland (as now, plus a species). A culture is a
branch of a people. Every people starts with one culture. History splits it.

A culture splits when:
- a daughter site is founded more than 24 regions from the culture's centre;
- a polity of the people goes to war with another polity of the same people (each side keeps its
  own culture from the first battle);
- a site is cut off by a war for 50 years.

The split is a row (`CultureSplit{from, new, site, because: EventId}`). The new culture copies its
parent's convictions, values, stories and ethics, then drifts on its own.

```rust
pub struct Culture {
    pub id: CultureId,
    pub people: PeopleId,
    pub name: String,                    // "the Hollin of the Ashwood", from the language
    pub split: Option<EventId>,          // the row that made it
    pub ethics: Vec<(Act, f32)>,         // section 6
    pub values: Vec<(Value, f32)>,
    pub stories: Vec<StoryId>,           // what they tell (capped, see 13)
    pub heroes: Vec<FigureId>,
    pub grudges: Vec<Grudge>,
    pub gods: Vec<GodId>,                // if decision 11 folds in the pantheon
    pub arts: Vec<ArtForm>,
}
```

### 5.2 Stories come from rows

A story is a culture's telling of one or more recorded events:

```rust
pub struct Story {
    pub id: StoryId,
    pub about: Vec<EventId>,             // the rows it tells; never empty
    pub culture: CultureId,
    pub kind: StoryKind,                 // Origin, Victory, Defeat, Betrayal, Famine, Plague,
                                         // Migration, Founding, Hero, Grudge, Sign, Lament
    pub hero: Option<FigureId>,
    pub foe: Option<Foe>,                // a people, polity, figure or "the cold"
    pub claim: Claim,                    // what it says happened; may have drifted
    pub version: u16,                    // how many retellings changed it
    pub tellers: Vec<Teller>,            // figures (world) or agents (valley), last 6
}
```

Stories are made in the realm's yearly step, from the year's rows:
- Battle, Taken, WarEnded: each side gets a story. The framing comes from the outcome and the
  culture's convictions (rules in 5.3).
- Famine, Plague, Ended: a Lament for the sufferers. Neighbours of a culture leaning Strangers
  above 0.4 get a "they brought it on themselves" telling.
- Founded, Moved, Led: Migration and Founding stories. The leader becomes a hero if the site
  grows to a village.
- The first site of a people: its Origin story. It names the homeland region and the founding
  figure, all real rows.
- Deity answers (the existing Pattern claims): Sign stories.

Retelling uses the lore model, generalised from `lore.rs`. When a figure or agent tells a story,
it drifts with chance 0.2, or 0.45 for a cautious teller or one leaning Strangers (as now). Drift
moves the claim along the existing map plus three new steps:
- Victory to "Destined": the gods gave it;
- Defeat to Betrayal: someone opened the gate (a named figure of the other side, chosen from
  real figures present at the battle);
- Migration to "Driven out".

`Story::true_to(record)` compares any telling with its rows, as `Tale::true_to` does. The Legends
page shows both: the telling, then "what the record holds".

### 5.3 The same battle, two stories

The rows, from a world on seed 11:
- Year 312: `Battle{war: 41, at: site 220 "Tessen Ford", won: polity 9 (Hollin), lost: polity 14
  (Grisk), dead: 31}`.
- Year 312: `Died{figure: 5521 "Aelith", how: battle}` (the Hollin war leader).
- Year 313: `Taken{site: 220, by: polity 9}`.

The Hollin of the Ashwood (Violence -0.3, Death +0.5) get a Lament, not a Victory. The rule: a
people leaning Violence below 0 tells a battle it won as a Lament if its war leader died.
- Year 312: "Aelith fell at Tessen Ford so the Ashwood could keep its trees." Hero: Aelith.
  Foe: the Grisk.

The Grisk of Karsk (Violence +0.4, Strangers +0.6) lost. They get a Defeat, foe the Hollin.
- Year 312: "At Tessen Ford the tall ones came out of the trees. Thirty-one of Karsk fell."
- Year 340, version 2 (retold by a cautious teller, drifted to Betrayal): "Tessen Ford was sold
  to the tall ones by Vrikka." Vrikka is a real Grisk figure who was present and survived (figure
  6013).
- The Grisk now hold a Grudge against the Hollin with source event the battle, and a personal
  grudge against Vrikka's line with source this story's version 2.

If a Grisk wanderer reaches the valley in game year 3 and tells this at a hearth, the HearthTale
row names the story, its version and its teller. The Legends page of Tessen Ford then shows:
- "The Hollin say: Aelith fell so the Ashwood could keep its trees."
- "The Grisk say: the ford was sold by Vrikka (a later telling; the record shows Vrikka fought
  and lived)."

Every word is from the rows, the culture's rules and the drift map.

### 5.4 Heroes, grudges, religion and art

Heroes:
- A figure becomes a culture's hero when a story names them in three tellings.
- Heroes appear in carvings (the engraving chooser reads hero stories) and in naming: children
  named after heroes, from the language, with the story as the source.

Grudges:
- `Grudge { against: Foe, since: EventId, strength }`, from Defeat and Betrayal stories.
- Each retelling refreshes a grudge; it fades 1% a year without one.
- Grudges feed war chance in `war.rs` (+0.01 per 0.5 of grudge strength) and the stance toward
  that people (section 7).

Religion:
- If decision 11 folds the E13 pantheon in, each culture's gods come from what struck it, not
  from a list. A people whose Famine stories outnumber the rest worships a god of hunger, named
  in its language, with those famines as the god's sources.
- The chosen deity (Aun, Hask, Mirre) is the one god that really acts. Others are beliefs.
- A Sign story whose pattern matches the deity's real triggers makes a culture believe in it
  more truly. This extends the existing "belief in the deity from patterns".

Art forms (`ArtForm` is carving, song, cloth pattern or figure) set what a culture's carvers and
fey moods depict:
- a culture with Death above 0.4 carves laments;
- one with Violence above 0.3 carves victories;
- one with Tradition above 0.5 carves origins.

The existing engraving chooser gains this weight. The event carved is still a real row.

---

## 6. Reactions that differ by culture

### 6.1 Ethics: a stance on each act

Dwarf Fortress gives each civilisation an ethic per act, from Unthinkable to Required. Here each
act is a belief topic like the seven that exist. A stance in -1..1 reads as:

| Stance | Word | Effect on someone who sees or hears of the act |
|---|---|---|
| below -0.6 | abhorrent | strong bad thought; trust in the actor falls by 0.3; may refuse them shelter |
| -0.6 to -0.2 | wrong | bad thought; trust falls by 0.1 |
| -0.2 to 0.2 | nothing to them | no thought |
| 0.2 to 0.6 | fitting | small good thought |
| above 0.6 | required | not doing it, when it is due, is itself wrong (burial rites, avenging) |

The acts, each with what in the simulation counts as doing it:

| Act | Counts when | Due when (for required) |
|---|---|---|
| Kill (a person, outside a fight) | a `Killed` row with no fight tag | never |
| KillInFight | a death in a recorded fight | a kin's killer is known: Avenge (exists as a goal) |
| EatDead (a sentient corpse) | eating a corpse item of a sentient species | never |
| Theft | taking an item owned by another (ownership exists for storied items and personal goods) | never |
| CutLivingTree | chopping a living tree | never |
| Drink | drinking ale or wine (exists as the Drink topic) | at a hearth night, for some |
| Burial | how the dead are kept: bury, burn, expose, keep | a death in the community |
| Strangers | welcoming strangers (exists as the Strangers topic) | a band arrives |
| OtherRaces | living alongside another species | another species joins |
| DeitySigns | heeding the deity's signs (patterns believed) | a sign is seen |
| DeepDigging | digging below the second cavern layer | never |
| KeepingBeasts | penning or riding animals | never |

Burial is not one axis. Each culture holds one rite: bury, burn, expose or keep in a hall. The
stance measures how much the rite matters. A death not given the rite is a broken taboo when the
stance is required (example 6.4.5).

### 6.2 Values

Of Dwarf Fortress's 33 values, eleven change something this simulation can act on:

| Value | What it moves |
|---|---|
| Tradition | keeps customs longer; resists new techniques; carves origins |
| Family | weight of kin's deaths and births; Avenge; fey causes from kin |
| Craftsmanship | weight of making; fey mood chance |
| Knowledge | Study goals; studying the thin place |
| Commerce | proposes caravans (section 10); prices it accepts |
| Nature | CutLivingTree stance; hunting restraint |
| Peace | Violence stance; war chance in the world layer |
| MartialProwess | takes up arms sooner; war chance |
| Merriment | hearth nights, drink, feasts |
| Tranquility | dislikes noise (works with hearing, section 11); settles apart |
| Hospitality | Welcome norm toward strangers and other races |

Values are held exactly like beliefs: `Belief { topic: Topic::Value(v), stance, confidence,
sources }`. That means the existing machinery works on them unchanged: share, hear, inherit,
experience, phrase.

`Topic` grows from 7 to 28: ten new acts (Drink and Strangers already exist) and eleven
values. Beliefs are stored sparsely (only topics a
person holds with confidence above 0.05), so the cost is only what is held. Section 13 has the
numbers.

### 6.3 Where a person's stance comes from

Four levels, each a recorded `Source`:
1. Species temperament: `Source::Species(SpeciesId)`. The species file's leanings, at confidence
   0.15.
2. Culture: `Source::Culture(CultureId, EventId)`. The culture's stance, taught in childhood. The
   event is the row that last changed the culture's stance (a split, a war, a famine story), or
   its founding row.
3. Parents and talk: `Source::Person(AgentId)`. As now: inherit at trust 0.5 + 0.4 × confidence,
   then share in conversation.
4. Experience: `Source::Event(EventId)`. As now: what happened to them.

The inspect panel lists a belief's sources in that order with their words. For example:
- "Holds that cutting living trees is abhorrent: the Hollin way (the Ashwood split from the
  Hollin over the Burning of Lessar, year 288);"
- "taught by her mother Ilsa;"
- "saw the grove cut at Fernmoot (spring, year 2)."

### 6.4 Worked examples, with full cause chains

Each example shows the rows, the beliefs read, the thought, and the lines a player sees.

#### 6.4.1 A death

Rows: `Died{agent: 88 "Orrin", cause: "fever"}` (event 9120). Orrin was human; the community has
Murrow and human members.
- Pell (human, Death +0.5 from his culture, confidence 0.6) gets LostSomeone, source 9120. Its
  weight is ×1.2 from the Death stance (`Beliefs::weight`, as now).
  - Feed: "Pell grieves for Orrin."
  - Inspect: "Grieving for Orrin (died of fever). Holds the living close: his people's way, his
    father Hann."
- Ysse (Murrow, Burial rite "keep in a hall", required 0.7) is in the same community. The human
  rite is burial. The next day the body is buried (a `Buried{agent: 88, rite: Bury}` row, event
  9131).
  - Ysse gets a new thought `RiteBroken`, source 9131, weighted by her stance.
  - Feed: "Ysse is troubled: Orrin was put in the ground, not kept with his people."
  - Inspect: "Troubled by Orrin's burial (the Murrow keep their dead in a hall: the Murrow way;
    her mother Teth)."

Nothing was invented. Two cultures' rites met a real death, and each person's thought names its
row and its belief's sources.

#### 6.4.2 A stranger

Rows: `StrangersSighted{band: 4, people: Grisk of Karsk}` (event 10240). The Grisk band carries
its culture's stories, including the Tessen Ford Betrayal (section 5.3).
- Hollin members: Strangers +0.1 (nearly neutral), OtherRaces -0.1, and a culture grudge against
  the Grisk with source the battle row of year 312, refreshed by the story.
  - Thought `OldEnemy`, source the grudge's battle event.
  - Feed: "The Hollin among them remember Tessen Ford."
  - Inspect: "Wary of the Grisk: the Hollin tell of Aelith, who fell at Tessen Ford, year 312."
- Human members have no grudge. Their Strangers stance decides as now (welcome or wary).
- When the band leaves it may have traded or fought. Either way it is a row, and the grudge
  strengthens or fades by that row.

#### 6.4.3 A fey mood

Rows: `Possessed{agent: 140 "Saelis", because: "remembering Aelith", cause: 702}`. Saelis is a
Hollin in the valley. Event 702 is a `Lived` row of her past: she is of Aelith's line, recorded
in her past (`opening::formatives` gains `HeroAncestor{figure}`).
- Her culture's art form is carving in living wood, and CutLivingTree is abhorrent to her. The
  material search (`moods::materials`) therefore excludes logs. She takes a standing tree in reach
  as the workplace: a new `FeyWork::CarveTree` designation on that tree.
- On completion: an engraving on the tree, `Engraving{event: the Tessen Ford battle row, ...}`,
  with words from the Hollin story's current version.
  - Feed: "Saelis has carved the tale of Aelith into the old oak."
  - Hover: "Carved by Saelis in spring of year 4, as the Hollin tell it: Aelith fell at Tessen
    Ford so the Ashwood could keep its trees."
- A human fey artisan under the same cause makes a figure, as now.

#### 6.4.4 A carving

Rows: an `Engrave` designation is finished by Kerrow (Grisk, culture Violence +0.4, art form:
victories).
- The chooser (`engrave::choose`) weighs events as now, then ×1.5 for kinds matching the culture's
  art form. Kerrow carves `Fought{a: 33, b: 61}` (the valley's last fight) over an older birth.
- Hover: "Carved by Kerrow in autumn of year 5: Tamsin fought Orl by the well."
- A human carver in the same house would have carved the birth. Both events are rows, and the
  choice is explained on the carving's hover ("a Grisk carves fights").

#### 6.4.5 A broken taboo

Rows: winter, famine. `Starved{agent: 77}` (event 11003). Nell (human, Hunger +0.5, EatDead 0.0
in her culture) eats from the corpse, the new act `AteDead{agent: Nell, of: 77}` (event 11012).
The act is possible only for people whose EatDead stance is above -0.2 and who are starving, and
it is rare.
- Hollin and Murrow members (EatDead -0.9) get `Abhorred{act: EatDead, by: Nell}`, source 11012.
  Trust in Nell falls by 0.3. The relation becomes `Shunned` if trust drops below -0.5.
  - Feed: "The Hollin will not sit with Nell."
  - Inspect: "Shuns Nell for eating the dead (abhorrent: the Hollin way; saw it, winter of year
    6)."
- Humans of Nell's culture: no thought for the act, but SawDeath as now.
- The community's cohesion falls by the shunning (cohesion and splits exist in society.rs). The
  split that may follow is a recorded row with this event as its cause.

#### 6.4.6 Another race joins

Rows: `Joined{agent: 301 "Teth", species: Murrow, community: 0}` (event 12550). Teth came as a
caravan member and stayed; the path is described in section 7.
- Each member's OtherRaces stance and their culture's stance toward the Murrow decide:
  - Humans of a culture with Hospitality +0.5: thought `Welcomed`, source 12550. Feed: "Pell
    welcomes Teth among them."
  - A human whose brother died in a cave collapse attributed to the deep (their tale says
    "something in the dark") holds Strangers +0.6 with the tale as a source. Thought `Uneasy`,
    source 12550; inspect "Uneasy about Teth: the tale of the dark at Coldmouth, told by his
    uncle."
- Teth's own thoughts: no hall for the dead (a culture absence, not an event, so no thought until
  a death); the light hurts (light shuns: a new `Glare` thought by day above ground, cause the
  light level).
- Ten days on, Teth asks to dig a hollow under the hill. The personal project of section 10 is
  proposed by Teth, reason "the Murrow live below", and the planner only finds the tiles.

---

## 7. Relations between peoples, and other races in the valley

### 7.1 In the world layer

Relations are kept per pair of cultures, built only from rows:

```rust
pub struct Relation {
    pub trade: u32,                      // trades in the last 50 years
    pub marriages: u32,                  // figures married across (fertile pairs only)
    pub grudge: f32,                     // from Defeat and Betrayal stories (5.4)
    pub alliance: Option<EventId>,       // the row that made it
    pub last_war: Option<EventId>,
}
```

What makes each change:

Trade between world settlements:
- C36's barter (`trade.rs`) runs between sites as well as with the valley.
- Each year every site with a surplus sends one caravan to the best partner within 12 regions
  whose culture's stance toward its people is not hostile, by the same quote logic.
- This also closes C36's gap that the world's prices move only by the valley.
- Rows: `RealmTrade{from, to, goods}`, aggregated per pair per decade so the chronicle does not
  fill (section 13).

Intermarriage:
- Figures of two cultures marry when their sites trade or share a polity, both cultures' stance
  on OtherRaces is above 0, and, for children, the species are fertile together (decision 1).

Alliances:
- Two polities at war with the same third polity, with trade between them and grudge below 0.2,
  ally with chance 0.1 a year. The row is `Allied{a, b, against}`.
- Allies join each other's battles in `war.rs`.

Wars:
- `war.rs`'s chance gains the grudge term (5.4), a species term (OtherRaces stance below -0.4
  toward the other people adds 0.006), and a habitat term.
- Peoples who cannot live in each other's land (cave and surface) fight only over the border
  regions where both can live.
- A war's cause text stops being one of three fixed strings. It names the grudge's story or the
  contested region.

### 7.2 Another race in the valley

Other species reach the valley by paths that already exist:
- Caravans to and from the valley: C36's trade partners. A caravan from a Murrow site brings
  Murrow.
- Wandering bands (`StrangersSighted`): bands are drawn from the realm's nearby sites, so they
  carry that site's species and culture.
- Refugees after a famine, a plague or a site taken in war: a band of that site's people (new
  `Fled` rows; the realm already records the cause).
- Promoted figures (D10, planned in M6): a world figure who walks into the valley keeps their
  identity and becomes an agent of their species.

Joining a community follows the existing Welcome path (welcome norms, cohesion). Two additions:
- The joiner's own stance toward the community's species matters as much as theirs toward the
  joiner.
- After joining, a person of another species keeps their culture's beliefs and rites. They teach
  them by the existing share in talk, so a community's culture can become mixed over years.

A mixed community is where cultures meet most: rites at a death (6.4.1), food taboos (customs
already exist), and noise and light. The Murrow shun light, the Skarri want heights, and the Holm
cannot use the doors. Each friction is a thought with a cause, never a scripted event.

### 7.3 Hybrids (decision 1)

Three options:
1. No children across species. The simplest, and a real constraint people live with.
2. One parent's species whole (recommended if allowed):
   - Only species with each other's id in `fertile_with` have children together.
   - The child takes one parent's species whole, drawn 50/50 by the birth stream: its plan, its
     ranges, its body block from that parent.
   - The temperament and aptitude loci mix as usual.
   - The child is a Murrow or a human, never a mix of parts. It carries both parents' cultures
     by inheritance.
   - The record says "born of a Murrow mother and a human father".
   - This keeps every sprite a valid plan and every body list a valid list.
3. Mixed bodies. Not proposed: it breaks the plan model and the art rig, and doubles every
   variant table.

---

## 8. Conditions with recorded causes: curses and ghosts

A condition changes a person and always has a recorded cause:

```rust
pub struct Condition {
    pub kind: ConditionKind,
    pub since: u64,
    pub cause: EventId,                 // the row that gave it: a bite, an act, a sign
    pub from: Option<AgentId>,          // who passed it, if anyone
    pub stage: u8,
}
```

Rules that apply to every curse:
- It spreads only by a recorded contact: a bite row, a shared meal row, a curse row.
- Nobody is cursed by chance alone.
- People never know someone is cursed. They see patterns (drained bodies, someone gone at full
  moon) and form suspicions as beliefs with those events as sources, the way belief in the deity
  forms from patterns.
- A suspicion can be wrong, and the record shows when it was.

### 8.1 The moon

Werebeasts need a moon. The calendar gains a deterministic lunar phase: a 10-day cycle, so two
full moons a year in a 20-day year, from the world's start tick. Night light is brighter at full
moon. That also gives the dark sight axis something to do on the surface.

### 8.2 Curses

The Beast-hunger (werebeast):
- Origin in the world layer: a figure cursed by the deity. The deity's Killing trigger, fired
  against a figure who killed outside a fight at a shrine site, records `Cursed{figure, by:
  deity, for: EventId}`. Only deities whose motives include Test (Hask) curse this way.
- Spread: a bite in a fight with a werebeast in beast form (`Bitten{by, of}`). The bitten gain
  the condition at stage 0.
- Effect: at full moon, from 22:00 to 04:00, the person takes beast form. That is a creature of
  their plan's beast variant (a new Species entry per plan, drawn from the animal art). It hunts
  as a wolf does in `systems/animals.rs`. They remember nothing. Stage rises each moon, and at
  stage 3 the beast form lasts the whole night.
- Signs people see: someone absent at full moon (a row, `Missing{agent, night}`); animal kills
  near where they slept; a bite wound on someone who was "attacked by a wolf".
- Ending: none, except death. A Hollin culture's healer tradition may hold a cure. That is a
  later technique (decision 6).

The Thirst (blood-drinker, vampire-like):
- Origin: drinking the blood of the dead of one's own kin. This is an act (`AteDead` with kin) in
  a culture where EatDead is abhorrent, answered by a new deity trigger, Taboo, which only a
  deity whose motive is Test (Hask) acts on: `Cursed{agent, for: 11012}`.
- Spread: the drinker's bite on a sleeper (`Bitten`) passes stage 0.
- Effect: no hunger for food, a need for blood every 3 days (a sleeper bitten, who wakes weak with
  blood loss on the existing `Body.blood`). Sunlight burns: the Glare thought, then a Burn wound
  by day. They do not age.
- Signs: sleepers waking with blood loss and two small wounds (a new wound kind, `Puncture`);
  someone never seen at meals or by day.
- Ending: death, or a ritual a culture may hold (later).

The Stone-sleep:
- Origin: a delver who breaks into the underworld layer (section 9) and touches its stone. The
  row is `Breached{agent, at}`.
- Effect: over 40 days the body hardens. Work rises, then every part stiffens and the person
  dies, standing, as a statue. The statue is a storied item with the breach row as its origin.
- Spread: none. It warns others away from the deep, and the warning is a story with the breach as
  its row.

### 8.3 Ghosts

Ghosts tie into the thin place's echoes. Today an echo is one of the seven who died alone,
seen at dusk near the portal stone (`thin.rs:474`).

The ghost rule: a person dead for 10 days whose death has none of
- a grave or rite (a `Buried` row, by any culture's rite);
- a carving of their death;
- a living person who remembers them (a long memory whose origin is their death);

becomes restless. A restless dead person is a record entry (`Restless{agent, since}`), not an
agent.

Within the thin place's reach, or where they died if that is within 20 tiles of the community,
the restless are seen:
- at dusk, as echoes are now, by someone awake within 12 tiles;
- the sighting is a row (`GhostSeen{of, by, at}`) and tells how they died, truly, as an echo does;
- those who see one get `Uneasy`, and sleep near the place gives `SleptUneasy` (both exist).

Laying to rest: any of the three things above, done later, ends it with `LaidToRest{of, by:
EventId}`. That might be the grave dug, the carving made, or the tale told at a hearth that gives
someone the memory.
- People who believe in ghosts (a Death stance source) propose the grave or carving as a personal
  project (section 10). The planner never does.
- A culture's rite decides what counts. To the Murrow only the hall counts, so a Murrow ghost is
  not laid by a human grave.

The Seamfolk see ghosts anywhere within 12 tiles, not only within the thin place's reach. Their
`GhostSeen` rows are how a community without a Seamfolk member hears of its restless dead.

### 8.4 Necromancy and undead (decision 2)

Recommendation: drop necromancy and walking undead.

Justification:
- Raising the dead needs a cause the record can hold. The only honest one would be a person who
  studies the dead (an existing Study goal) and learns a technique.
- That technique would have to come from somewhere: a teacher, a book, a tale. Nothing in the
  world's history produces such a source unless we seed one, and a seeded villain is invented
  history.
- Undead armies also duplicate what M5's sieges will do with real armies.

Ghosts (8.3) and the curses (8.2) give the same unease from recorded causes. If Dan wants undead,
the narrowest honest form is the Below's dead: things that died in the underworld and do not
stay down, confined to that layer and recorded as its nature in the species file. They would
never be raised by anyone.

---

## 9. The depths

The deep today has three strata, worm caves and two-level caverns in the middle stratum, with
bats, one crawler and one burrower. M6's layered deep (PLAN-FINISH M6) is the foundation:
cavern layers down to an underworld. This epic says who lives there.

| Layer | Depth (z below surface) | Rock | Light | Peoples (anchors) | Generated beasts | Other life |
|---|---|---|---|---|---|---|
| upper caves | 5-15 | sedimentary | none | Screelings, Grisk | none | bats, crawlers |
| caverns | 15-35 | granite, slate, marble | fungus glow | Murrow; generated cave peoples | one per world | fungus forests, cave fish, the burrower |
| deep caverns | 35-60 | basalt | fungus glow, magma light | Holm; beast-folk (generated taur and huge plans) | one or two | giant fungi, blind beasts |
| the underworld | 60 and below | strange stone (`Material::UNDERSTONE`, new) | magma | the Below | the deep's eldest | none that grows |

The world layer: cave peoples have homelands in the deep map. A deep region sits under each
surface region, and capacity comes from cavern area and fungus, generated with the region. They
found sites, grow and fight in the yearly step as surface peoples do. A site can have both a
surface and a deep presence (a Grisk town over a Screeling warren). Contact between them is
trade or war in the same way.

Generated deep beasts (Dwarf Fortress's forgotten beasts, recorded):
- A beast is a generated creature: a body plan from the generator (any plan, including four-armed
  and serpentine, with no hands needed because beasts do not craft), a material (made of basalt,
  fungus, magma), a size of huge, and one attack.
- It is born in the history pass with a `BeastWoke{beast, layer, region}` row in some century.
  From then on its attacks on deep sites are rows (`BeastAttacked{beast, site, dead}`), so its
  legend is its record.
- It reaches the valley only if the valley's delving opens into its layer. The breach is a row,
  and the beast comes up by the opened path.
- Its Legends page lists every site it has ruined.

What the valley sees:
- Digging deeper meets each layer's life and peoples. The first contact with a cave people is a
  `StrangersSighted` underground: a Murrow party at the edge of the dig, with their own culture's
  stance toward surface people.
- The delving culture's DeepDigging stance decides whether people want to go on digging. A
  community whose tales say "something in the dark" stops, and it stops by persons' choices.

---

## 10. Organic, not planner-driven

With one species everything could run on the settlement planner's quotas. With many peoples that
fails: a Murrow-and-human community planned by `plan_community` builds the same houses, fields
and workshops as a human one. The difference has to come from persons.

The planner's rule-driven orders today (`systems/settlement.rs` `plan_community` unless noted):

| Order | Rule today | Move to a personal project? |
|---|---|---|
| stockpile, haul | tiles and haulers per head | no: logistics |
| chop when logs are short | stock rule | no: logistics; but CutLivingTree people refuse to do it |
| mine a face for blocks | stock rule | no: logistics |
| hearth | one per community | no |
| houses and beds | a new house once all are roofed; 6 beds | yes, later: who builds what home (hollow, house, nest) is a culture's way |
| floor, table, workbench, lamps | per house | no |
| cellar | food rule | no |
| fields (sowing by the store) | plots per 3 people | no, but the crop follows the culture's diet |
| hunt | one per 6 people when short | yes: a hunter's project, refused by Nature-holders |
| cook | rule | no |
| workshops (bench, loom, smelter, anvil, kiln) | at 8 people, by stock gates | yes, first wave: a crafter proposes the workshop of their craft |
| recipe wants (27 entries) | table | partly: a maker proposes what they want to make; the table stays as the fallback |
| brewing | food ≥ 10 days a head and Drink lean > -0.1 | yes, first wave: someone who values Drink or Merriment proposes it |
| veins (prospecting) | rule | no |
| carving | carver with skill 4, 5 days apart, 3 per house | yes, first wave: a carver with a story to carve proposes it |
| clay and sand | rule | no |
| roofs, quarry | rule | no |
| caravans (`expedition.rs` `consider_new`) | 0.12 a day if able and fed | yes, first wave: someone with Commerce (or kin at the other site) proposes the journey |

The first wave moves four orders to personal projects before any species work: carving, brewing,
workshops and caravans. The kind proposed in the original organic plan comes back as:

```rust
pub struct Project {
    pub by: AgentId,
    pub kind: ProjectKind,              // Carve{event}, Brew, Workshop(StructureKind), Journey{site, carry}, Grave{of}, Hollow, ...
    pub reason: Reason,                 // the belief or event behind it: Source::Event, Source::Culture, ...
    pub proposed: u64,
}
```

A person proposes a project when their beliefs and skills give it a score above their other goals
(decide.rs's existing scoring). The planner only finds tiles, materials and helpers; it never
decides that a project exists. The order shows "Kerrow wants to carve the fight by the well (a
Grisk carves fights)".

This removes the quota for those four. Soaks gate it: a community with no carvers carves nothing,
and one whose people lean Drink -0.5 never brews.

---

## 11. The depth gate and the new simulated axes

### 11.1 The gate

Every generated species must differ from humans on at least four of these axes. Each must change
what a person does, not only how they look:

| Axis | Human value | How it changes behaviour | Exists today? |
|---|---|---|---|
| diet | omnivore | what they gather, hunt, farm and eat; what famine means to them | partly (food kinds exist; no per-species nutrition) |
| climate | comfort 2-24 °C, bare | where they settle; cold and heat stress; seasonal migration | cold only, fixed threshold |
| senses | sight 2, no dark sight, no hearing or smell | what they perceive, and where they can live and work | sight only, darkness ignored |
| lifespan | 50-70 | generations per century; how much history one life holds; elders | yes (genome) |
| size | 150-190 cm, 65 kg | food, carrying, doors, beds, reach, warmth (2.4) | no |
| social structure | household, pair | who lives with whom, who raises children, group size | household and pair only |
| way of living | farming and others | world-layer production and trade; valley work preferences | world layer only |
| cycle | diurnal | when they work and sleep | no |

Generation guarantees four by redrawing. The test `generated_species_differ_four_ways` checks
every generated species in 100 seeds.

The soak proves it. For each anchor, and for 3 generated species per checked seed, a species soak
(`ostrog soak --opening --species ID`) settles a community of that species in a valley
suited to its habitat. It reports:
- where they live (share of sleep hours on the surface, upper caves, caverns);
- what they eat (by food kind);
- when they sleep (hours);
- cold and heat stress hours;
- deaths by cause;
- reaction counts by thought kind.

The gate is a per-axis expectation in the species file, checked against the soak:
- a nocturnal species sleeps mostly 06:00-18:00;
- a fungivore eats fungus for over half of its meals;
- a cave species sleeps over 80% of nights below ground.

### 11.2 New axes and what they cost

Darkness and dark sight (needed first):
- Sight radius becomes `SIGHT_RADIUS × light_factor`, where `light_factor` comes from
  `underground::light_at` below ground and from the hour and moon above it.
- Species with dark sight 3 ignore it; sight 1 halves the radius; sight 0 sees nothing.
- Cost: one light lookup per perception scan. Perception scans each person once a game minute
  (`STRIPE = 20`). At 500 people that is 25 lookups a tick against a grid already rebuilt hourly.
  Estimated under 0.02 ms a tick.

Hearing:
- Acts that make noise emit a noise row in a per-tick buffer: work by kind (mining 14, chopping
  10, talk 4), fights 12, falls 6.
- On each perception scan, a person with hearing r collects buffered noises within r tiles
  through walls, from the existing 16-tile spatial index. They perceive the source as a known
  location, not a seen thing.
- Cost: the buffer holds at most a few hundred entries a tick. A cell query per scan is at most 9
  cells. Estimated under 0.1 ms at 500 people.
- Hearing gives the Murrow their world, makes Tranquility mean something, and lets people hear a
  fight or a collapse they cannot see.

Smell:
- A person with smell r knows food items, corpses and creatures within r tiles without line of
  sight, from the same index.
- Cost: as hearing without the buffer, under 0.1 ms.

Cycle:
- `decide`'s rest rules read the species cycle instead of `is_night`.
- Cost: none.

Heat stress:
- `heat_step` mirrors `cold_step`.
- Cost: as cold, already within budget.

Diet:
- Nutrition multipliers per species on `nutrition_hours`. Food kinds `Fungus` and `CaveFish` are
  added (fungus trees exist and fish are on the M6 list).
- Cost: none.

Social structure:
- Clans share children's care across households. Hives (none in the anchors; possible for
  generated species) share stores.
- This touches `society.rs` and `life.rs`. Cost: none per tick.

Section 13 gives the combined perception budget.

---

## 12. The readability gate

Silhouette and colour at 1x:
- `tools/art/lineup.py` renders every species of a world side by side at 1x and 2x, in idle,
  walk and work frames, on grass, on cave floor and at night.
- `tools/art/silhouette_check.py` cuts each kind's body mask at the idle frames facing S and W
  and fails any pair whose smaller per-view outline distance (xor over union, feet aligned)
  is under 0.30 (C53, docs/reviews/readability-2026-10-19). `--full` (added for #485) gates
  the whole drawn roster — every kind x build x age figure, not only the average adults; the
  criteria and the current known gaps are written down in
  docs/design-character-visuals.md.
- The generator uses the same check when it picks parts (1.4, step 5), so generated species pass
  by construction. The check stays in CI for hand-made and player-made files.

The tester rubric (a new tester set per checkpoint):
- At 1x in the game, can you tell each species apart in a crowd?
- Name each from the inspect panel.
- Find its page in Legends.
- Frames: the line-up sheet and a crowd scene at the hearth with three species.

Names and explanations, which can land early (with P3, section 14):
- Inspect: "Ysse, a Murrow (blind cave folk: hear and smell through rock; eat fungus; light hurts
  them; live 60-85 years)", then her culture: "of the Murrow of Deepmere: keep their dead in a
  hall; strangers are a danger (0.5)".
- Feed lines name species where it matters: "A band of Grisk is sighted."
- Legends gains a species page (`species:N`) and a culture page (`culture:N`), with plan, size,
  senses, diet, life, peoples, homelands, history, stories and heroes. All of it is text and links,
  which the browser already renders.

What depends on M7:
- portraits per person;
- species icons in the feed and lists;
- filters ("show only Murrow");
- the colour-blind check of species ramps;
- an overlay of who is where by species.

These improve readability but none of them is needed to pass the gate.

---

## 13. Performance and size

The history pass:
- Today 4-7 peoples run 1,000 years in about 3.3 s with figures and wars.
- With 5-9 peoples (up to 16), cultures, stories and inter-site trade, the work per year grows
  with sites (bounded by capacity, as now) and with story events.
- Story events: about 3,000 battles in 1,000 years, giving about 6,000 stories, plus laments and
  migrations, about 10,000 in all.
- Caps: 200 stories per culture (oldest of the least-told dropped), 40 heroes, 20 grudges.
- Inter-site trade is aggregated to one row per pair per decade.
- Gates:
  - `history_pass_budget` stays (an opening world under 2 s);
  - a new `a_thousand_years_of_many_peoples` runs 16 peoples for 1,000 years under 10 s.

The deep map:
- One deep region per surface region doubles the map's region count, but deep regions are plain
  (cavern area, rock, depth).
- Gate: `a_world_map_is_quick` stays under 5 s with the deep map.

Saves:
- Generated species: about 2 KB each, 60 KB for 30.
- Cultures: about 1 KB each plus stories.
- Stories: about 120 bytes each, so about 1.2 MB for 10,000.
- Relations: about 30 bytes per culture pair, so 30 KB for 30 cultures.
- Per-agent body state: 8 body loci and part states, under 64 bytes.
- Beliefs: sparse. A person holds about 12 of the 28 topics on average, each about 70 bytes with
  sources, so under 1 KB per person.
- Gate: `save_growth_with_peoples`. A 1,000-year world with 16 peoples saves under 4 MB more than
  the same seed with today's peoples, and loads under 1 s.

The chronicle:
- New per-event rows are stories made (realm), splits, conditions, ghost sightings and rites.
- Retellings are Trace rows, aggregated as now. Inter-site trade is aggregated per decade.
- Gate: chronicle rows per game-year in the 600-day soak grow under 15% over the M4 review's
  baseline.

The tick:
- Perception gains light, hearing and smell (11.2): together estimated under 0.25 ms at 500
  people.
- Belief sparseness keeps share and hear cheap.
- Gate: `ostrog profile --agents 500 --mixed` (a mixed-species community) stays within 2 ms
  mean with no tick over 16 ms. The profile reports perception separately so the senses' cost is
  measured, not estimated.

The library:
- Gate: `library_loads_quickly`. Loading and validating 42 species and 9 plans takes under 50 ms.
- `library_rejects_bad_files` covers every validator rule with a bad file.

---

## 14. Roadmap

This is its own milestone, MP (Peoples). It runs now, after the M4 review and before M5, so M5's
wounds, prosthetics and fighting are built on P1's part list from day one, not retrofitted
(decision 5).

The order of work: the four planner quotas first (they are needed to make cultures differ), then
bodies, then species in the valley, then the world draw, then stories, then the deep.

| # | Checkpoint | Builds | Gates (tests) | Art | Soak | Tester set |
|---|---|---|---|---|---|---|
| P0 | Personal projects | `Project`; carving, brewing, workshops and caravans proposed by persons | `no_carver_no_carving`, `teetotal_never_brew`, `a_journey_has_a_proposer` | none | 4 × 300 days; carvings, ale, caravans per seed against C36 | AB: who proposed each carving and caravan |
| P1 | Body plans | plan files, `PartId`, `Body` part states, every wound caller asks the plan; genome body block; size effects (food, carry, warmth) for humans unchanged | `human_plan_matches_old_parts`, `wounds_land_on_plan_parts`, `size_scales_hunger`, `same_genome_same_body` | none (data) | 4 × 300; nothing should change for humans: digests compared per system | none |
| P2 | The rig | `rig.py`, biped keyframes; today's sheets reproduced; fight frames; second plan (digitigrade tail) | `rig_reproduces_people_sheets` (pixel diff), palette and provenance | biped port, fight set, digitigrade plan | none | AC: fight frames, tail plan line-up |
| P3 | Species in the valley | library loader and format; `SpeciesId`; `is_person()` for the 73 gates; save migration; six surface anchors (human, Hollin, Grisk, Screelings, Skarri, Tharn) as joiners and bands; inspect and feed lines; Legends species page | `library_loads_quickly`, `library_rejects_bad_files`, `old_saves_keep_their_creatures`, `a_grisk_band_is_grisk`, `species_named_in_inspect` | heads, ramps, wings, four arms | species soaks for each anchor | AD: line-up at 1x; name each species |
| P4 | Senses and cycles | darkness in sight; dark sight; hearing; smell; cycles; heat stress; diets | `dark_sight_sees_in_the_dark`, `blind_hear_through_rock`, `nocturnal_sleep_by_day`, `fungivores_eat_fungus`, profile gate | none | species soaks with the axis expectations | AE: night and cave scenes |
| P5 | The world draw | presence and rarity, the draw, homelands by habitat, minorities, setup record, wizard page with preview | `same_peoples_same_world`, `rarity_changes_the_world`, `legendary_leaves_ruins`, history budget gates | species cards (silhouettes from sheets) | 4 × 300 openings with mixed worlds | AF: the wizard page, preview, re-roll |
| P6 | Generated species | generator, four-axes rule, silhouette check, language names, saved in the world; library 42 | `generated_species_differ_four_ways`, `generated_silhouettes_differ`, `generated_species_survive_a_save` | rig options complete for every plan (taur, huge, amphibious, serpentine) | species soaks: 3 generated per seed | AG: line-up of a generated world |
| P7 | Cultures and ethics | cultures and splits; acts and values as topics; sources from species and culture; reactions and thoughts with causes; rites at death | `stance_traces_to_a_source`, `two_rites_two_thoughts`, `taboo_broken_is_a_row`, the six examples of 6.4 as scenario tests | clothing per culture | 4 × 600 days | AH: read the reasons in inspect |
| P8 | Stories | stories from rows, two sides, drift, heroes, grudges, art forms in carving, religion if decided | `every_story_has_rows`, `two_sides_of_one_battle`, `drift_is_on_the_map`, `story_true_to_record` | none | history passes over 4 seeds, counts and samples in the review | AI: Legends battle pages |
| P9 | Relations | inter-site trade, intermarriage, alliances, grudges in war, refugees, joining across species, hybrids by decision | `trade_between_sites_balances`, `allies_join_battles`, `refugees_carry_their_site`, `hybrid_takes_one_plan` | none | 4 × 600 | AJ: a mixed community |
| P10 | The deep | deep map and cave homelands (needs M6's layers), Murrow, Holm, the Below, generated deep beasts with records | `cave_peoples_live_below`, `a_beast_is_its_record`, `the_breach_is_a_row` | Murrow, Holm, serpentine, beasts | species soaks below; a delving valley | AK: first contact underground |
| P11 | Conditions | moon; ghosts and laying to rest; the Beast-hunger; the Thirst; the Stone-sleep | `ghosts_only_of_the_unremembered`, `laid_to_rest_by_rite`, `curse_spreads_only_by_contact`, `suspicion_has_sources` | beast forms per plan, ghost tint | 4 × 600 with curses on | AL: a haunting and its end |
| P12 | MP review | four 600-day soaks with 16-people worlds, profile with mixed species, readability rubric, gap table against DF and RimWorld races | all gates green | contact sheets | 4 × 600 | the whole round |

That is thirteen checkpoints, c38 (P0) to c50 (P12). M6 has not run, so P10 brings forward the
part of M6's layered deep it needs: deep caverns in the basalt stratum and an underworld layer of
new stone, in `world/gen/caverns.rs` and `strata.rs`. M6 then builds fluids and the rest on it.

---

## 15. Decided 2026-10-18

Dan's answer: take every recommended option and ship it; put the options I recommended against in
the backlog, because he wants them later (docs/BACKLOG.md, E14).

| # | Decision | Shipped in this milestone | Backlog (E14) |
|---|---|---|---|
| 1 | Hybrids | the child takes one parent's species whole, only between species that list each other in `fertile_with` (7.3) | mixed bodies |
| 2 | Necromancy | ghosts and the three curses (8.2, 8.3) | necromancy, raised undead, the Below's restless dead |
| 3 | Player-made species | the TOML format; the loader also reads `user://peoples/species` from P3, counted in the library hash and the setup record | a mod browser and sharing UI |
| 4 | Library size | 12 anchors and 30 generated per world, 42 in all | more hand-made anchors |
| 5 | Placement | milestone MP now, right after the M4 review; M5 builds on P1's part list | none |
| 6 | Curses | the Beast-hunger, the Thirst and the Stone-sleep, with a moon; cures as a healing technique a culture can hold, learned from a recorded source | none |
| 7 | The portal party | humans always come through | a non-human party in the wizard |
| 8 | Handless peoples | excluded | sentient handless peoples |
| 9 | Huge peoples | in the valley, with 2 x 2 sprites and 2-tile doors | none |
| 10 | Acts | all twelve, as in 6.1 | none |
| 11 | Pantheon | folded into P8: gods from what struck a culture; the chosen deity is the only one that acts | E13 stays, for a storyteller of several deities |
| 12 | Names | anchors keep fixed names; generated species take language words | language names for every species in every world |
| 13 | Order | P0 first | none |
| 14 | The M4 review | before P0, as C37 | none |

Checkpoint numbers: the M4 review is c37, and P0 to P12 are c38 to c50.
