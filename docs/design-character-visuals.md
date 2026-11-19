# Character visuals: distinctness criteria and style guide

Infrastructure slice of #485 (character visuals). This is the measurable standard later #414 /
#485 character-art work is held to: what "reads apart at 1x" means as a number, the palette
roles body art draws from, the silhouette vocabulary each body plan owns, and the known gaps
between the current art and the standard. It extends the C53 silhouette gate
(`tools/art/silhouette_check.py`, `docs/reviews/readability-2026-10-19/README.md`) from the
average adults to the full drawn roster.

Status: gate defined and wired (CI runs `silhouette_check.py --full`); the gate is a **ratchet over
current master art** — 110 cross-kind appearance pairs score under the threshold, are recorded in
`tools/art/silhouette_baseline.json` (tolerated at their recorded score; any new or worse gap
fails CI, `--write-baseline` regenerates it) and are listed below as the work list for the production art PR (per D63 that PR carries the real assets,
provenance and review evidence; this document invents no art).

## 1. The distinctness criterion

Two figures are distinct when their **outline distance at 1x** is at least **0.30**, measured as
in the C53 check:

- Take each figure's body sheet (`godot/assets/people/body_<set>_<build>_<age>.png`; the human
  set has no infix), cut the idle frame facing S and facing W from the plan's cell
  (`swaps.json` `plan_cells`: 48 px for most plans, 64 for taur, 96 for huge_biped and
  serpentine), and reduce it to its opaque mask, outline included.
- Place both masks on one canvas, feet row at the bottom − 2 and the bounding box's horizontal
  centre at the middle — the way two people standing on adjacent tiles actually meet on screen.
- `d = |A xor B| / |A or B|` per view; the pair's score is `min(dS, dW)`. A figure must read
  apart facing the viewer **and** in profile, not on average.

A **figure** is one kind × build × age. Two figures of **different kinds** must score ≥ 0.30 in
every build/age pairing: people of different kinds stand side by side in play in every
combination of stature and age, so the gate is all 2,376 cross-kind pairs of the 72-figure
roster (see §2). Two figures of the **same** kind (build/age variants) are measured and
reported but not gated: within a kind, variation is intended to be bounded and coherent
(#485's acceptance — "individuals in the same species/age/body plan can vary in a bounded,
coherent way"), so a slim Grisk passing for a broad Grisk is fine; a Grisk passing for a Meer
is a gap.

The 0.30 threshold is the C53 value, set from the MP review's accept/reject pairs: above every
pair the review found too alike at 1x (0.000–0.282), at the bottom of the pairs it accepted
(0.303+). The full-roster gate keeps it rather than re-deriving it, so the extended check is
strictly the old criterion applied to the whole roster. D58's second-monitor readability
target is the visual intent; this number is the enforceable floor under it.

What the criterion does **not** cover (the mask is the body layer of the idle frame only):

- Clothing, hair, equipment and bandage overlays. They are drawn per body plan/build/age, not
  per kind, and change with season, role and injury, so a body difference a garment can cover
  does not survive in play. Kinds must differ in what is always visible: head, parts, build.
- Colour and faces. Palette roles and face contrast are separate gates
  (`tools/art/palette_check.py`, `tools/art/contrast_report.py`).
- Frames other than idle (walk/work/carry/sleep and the other poses). A kind must also keep
  its identity through the animation cycle; that review stays human (contact sheets,
  `tools/art/lineup.py`) until a pose coverage gate exists.

## 2. The roster the client draws

The client renders every person from `Look` (`crates/hf-sim/src/view/look.rs`) through
`godot/scripts/render/entity_layer.gd`:

- **Kind** (P3): the species' `art.body` names the body-sheet set
  (`data/peoples/species/*.toml`); `art.hair` false draws no hair layer.
- **Build**: `Look.body = appearance.body % BUILDS` → `slim / average / broad`
  (`v.BUILD` in `godot/scripts/world_view.gd`). Every layer sheet is drawn per build and age
  (`tools/art/people.py`), so overlays stay in register with the body.
- **Age**: `child` from the life stage; adults are about 40 px, children about 30 px
  (`docs/ART_SPEC.md`). Elders differ only in hair colour, the injured in a bandage overlay —
  neither changes the silhouette.
- Plans and their parts are data: `data/peoples/plans/*.toml`.

So the drawn roster is **12 kinds × 3 builds × 2 ages = 72 figures** (2556 pairs; 2,376
cross-kind gated, 180 intra-kind reported). A missing figure is a client-visible hole, so the
check asserts every one of the 72 sheets exists at its plan's cell size.

Builds widen and narrow the same skeleton (`DIM[age][build]` in `tools/art/people.py`); age
rescales it (children carry about two thirds of the adult height changes, C53). Neither may
cross the kind boundary: the gate's failures cluster exactly where a small or young figure of
one kind meets a differently-proportioned figure of another (§6).

## 3. Palette roles

From `docs/ART_SPEC.md` and `docs/ART_BIBLE.md` (style bible v2); `tools/art/palette_check.py`
enforces the palette on every sheet.

- One master palette, `godot/assets/palette/ostrog.json` (+ `.png`): eleven hue ramps × 8
  steps (88 colours) + 4 neutrals = 92. Every generated pixel is one of these; sprites may not
  leave the palette (the screen grade may).
- Skin is a **swap role**, not fixed paint: body sheets are drawn in the four swap source tones
  and remapped per individual through `swaps.json` (4 source tones → 26 named target ramps;
  each species' TOML `art.skin` lists its allowed targets, e.g. the human's six tones). A
  difference that only exists in one skin tone is not a difference.
- Hair (8 colours) and clothing (12 colours) are swap roles too; `art.hair` false kinds draw
  no hair layer at all.
- Derived sheets (reference-based work) request each swappable region in one saturated key
  hue (clothing pure green, hair pure magenta, skin natural); a non-role colour must not touch
  a swap source or the runtime swap recolours it.
- Figures carry a 1 px darkest-neutral outline and upper-left light (`docs/ART_BIBLE.md`
  fixed rules); faces must read at 2× on every skin tone (eyes/mouth contrast,
  `tools/art/contrast_report.py`).

## 4. Silhouette vocabulary per body plan

Kinds on the same plan differ only in what the plan leaves free: head shape, ears and fronds,
limb count and posture. That is why the gate admits no same-plan exemption (C53). The plans
are `data/peoples/plans/*.toml`; the per-kind parts below are what the current art draws and
what the 0.30 gate is measured on.

| Kind | Plan (cell) | Sheet set | Hair | Distinguishing parts (current art) |
|---|---|---|---|---|
| Human | biped (48) | `body_<build>_<age>` | yes | The baseline every depth gate compares against. |
| Hollin | biped, own rig (48) | `hollin` | yes | 3 px taller, narrow torso, ears 9 px sweeping up/out (tips 2 px above the crown facing S). |
| Seamfolk | biped, own rig (48) | `seamfolk` | yes | 8 px taller (longer legs and neck, longer face), narrow shoulders and head, thin limbs. |
| Murrow | biped, own rig (48) | `murrow` | no | Head sunk into the shoulders, broad torso, arms to the knees, 5 px forward stoop in profile, 8 px fan ears. |
| Tharn | four_armed (48) | `tharn` | yes | Second arm pair from the waist, elbows out, hands ~9 px wide of the hips; broader chest. |
| Meer | amphibious (48) | `amphibious` | no | Head 6 px wider, flat in profile; three gill fronds a side; crown fin ridge; paddle hands and feet. |
| Grisk | small_biped (48) | `grisk` | yes | Big head (+3 px wide, +2 px tall) on a short trunk; ears 8 px out to a point. |
| Screeling | digitigrade_tail (48) | `screeling` | no | Digitigrade legs, tail; small (80–105 cm). |
| Skarri | winged (48) | `skarri` | no | Folded-wing wrists ~8 px above the shoulders beside the head; bird's legs; taloned spread feet. |
| Holm | huge_biped (96, drawn 48 and scaled ×2) | `holm` | yes | Cave-giant trunk 4 px broader, thicker limbs, the same head reading small on the shoulders. |
| Below | serpentine (96) | `below` | no | Long serpent body, no legs; the hot dark's folk. |
| Gnarrow | taur (64) | `taur` | yes | Four-legged taur body, cannot climb ladders; height in the human range, so the front view is the hard one. |

Rules that follow from the vocabulary:

- Kinds on a shared plan (the four biped-rig kinds) must spend their distinguishing parts where
  the outline actually shows at 1x: C53 measured the Hollin's ears at 9 px because 6 px did
  not move the number. Parts hidden from both gated views buy nothing.
- Children carry the same parts (about two thirds of the adult changes) — a kind must be
  identifiable at every age, which is where most of the current gaps live (§6).
- New kinds reuse a plan before inventing one: the plans carry wound/vital semantics in the
  simulation (`data/peoples/plans/*.toml`), and a new plan is simulation work, not just art.

## 5. The gate and how to run it

```
python3 tools/art/silhouette_check.py            # C53 gate: average adults + the human child
python3 tools/art/silhouette_check.py --full     # #485 gate: the 72-figure roster
```

Both exit non-zero when a gated pair scores under 0.30; `--quiet` prints only the summary.
`tools/ci.sh` runs both next to each other. The C53 gate is green on master. The full gate is
**red on master by measurement**: the art predates it (only average adults were verified).
The wiring is deliberately strict — a tolerant gate would let new art land with the same
gaps — so CI stays red until the production art PR (D63: real assets, generator/runtime
integration, provenance, focused guards, contact-sheet or in-game evidence) closes the gaps.
No art is invented here.

When the art PR closes the gaps it updates this document's gap list in the same commit.

## 6. Known gaps on current master (the art PR's work list)

110 of the 2,376 gated cross-kind appearance pairs score under 0.30, in 21 kind pairs. Each
row: how many of the kind pair's 36 build/age appearance pairs are under, and the worst.
Every one is a place where two people of different kinds can stand side by side at 1x and read
as one kind.

| Kinds | Under 0.30 | Worst |
|---|---:|---:|
| gnarrow × human | 8/36 | 0.181 |
| gnarrow × skarri | 9/36 | 0.210 |
| hollin × seamfolk | 5/36 | 0.212 |
| human × skarri | 3/36 | 0.218 |
| human × tharn | 10/36 | 0.222 |
| human × meer | 10/36 | 0.229 |
| grisk × meer | 7/36 | 0.234 |
| meer × skarri | 9/36 | 0.239 |
| gnarrow × screeling | 8/36 | 0.242 |
| grisk × murrow | 8/36 | 0.244 |
| murrow × skarri | 3/36 | 0.251 |
| human × seamfolk | 5/36 | 0.255 |
| hollin × human | 7/36 | 0.261 |
| human × screeling | 5/36 | 0.266 |
| grisk × human | 3/36 | 0.269 |
| meer × murrow | 3/36 | 0.274 |
| screeling × skarri | 2/36 | 0.280 |
| hollin × meer | 1/36 | 0.283 |
| gnarrow × murrow | 1/36 | 0.288 |
| human × murrow | 2/36 | 0.288 |
| grisk × hollin | 1/36 | 0.296 |

Patterns to fix against, not just rows:

- **Children of one kind against adults of a near-sized kind.** The worst scores are child
  figures against different-kind adults (gnarrow child × human adult 0.181 in the S view; the
  W profile is usually fine). Age must not be the only tell between kinds.
- **The biped family at its build extremes.** Human × tharn and human × meer fail at 10/36
  each, concentrated in broad/slim pairings the average-adult C53 pass never measured.
- **Small kinds against small kinds' children.** Grisk × meer, grisk × murrow, gnarrow ×
  screeling: where two kinds' size ranges overlap, their builds collide first.
- **Front views.** Most failing pairs fail in the S view while the W profile reads fine —
  profile differences (tails, wings, stoops) are easier than frontal head/torso ones.

For reference, the intra-kind range (same kind across builds and ages) runs 0.006–0.611 and
is not gated; that is how similar two individuals of one kind may be.

The full 110-row list is in the appendix and is reproduced by
`python3 tools/art/silhouette_check.py --full`.

## 7. Not covered here (follow-ups, in order)

- **Animals.** `godot/assets/animals/*.png` render through a separate path with no
  build/age/skins (one sheet per kind, idle column × S/W rows). A probe of the same metric
  over the 13 animal kinds found them mostly distinct but not gated-clean: deer × hart score
  0.000 because the White Hart is deliberately the deer's frames recoloured (a design
  identity, not an accident), and deer × wolf reaches 0.139 in the front view. Whether animal
  distinctness gates at the same threshold — and whether the hart's shared outline is exempt —
  is a design call for the follow-up that wires `animals/` into this check.
- **Faces at 2×** — `tools/art/contrast_report.py` (eyes/mouth contrast per skin tone).
- **Contrast against backgrounds** at normal and close zoom (cold/wet, injured, equipped,
  underground views in #485's acceptance) — no gate yet; the silhouette metric is shape-only.
- **Poses beyond idle** (walk/work/carry/sleep/sit/strike): a kind must keep its identity
  through the cycle; currently reviewed on contact sheets only.
- **Injury states** beyond the bandage overlay (visible wounds, being hurt/treated) — #485
  scope, must come with simulation-backed state per the no-fake-cues rule below.

## 7a. Done since (2026-11-02, #485 slice 2)

- Poses beyond idle: sleep breathes and stirs, a healer tends, a drinker drinks (48-column
  sheets). The silhouette gate still measures idle only.
- Per-person variation for the hairless kinds: a head marking in the hair slot chosen by the
  inherited hair-style gene, in the person's own skin colours (`marks_*` sheets).
  `docs/reviews/485/`.

## 8. Rules for later #414 / #485 character work

- Every rendered cue communicates a real simulation state or authored visual trait (#485
  acceptance): no emotion, injury severity, tool function or social role that the simulation
  does not back; markings and colours derive from authored traits or genome data, never
  arbitrary confetti. Visual state must not enter simulation state or world digests.
- People stay procedural through `tools/art/people.py` (one skeleton drives every layer, so
  frames, directions, builds, ages and outfits stay in register). Generated reference sheets
  set proportions, face construction and shading; they are redrawn to the rig at 48 px.
- New or changed body art must keep `--full` green, and lists its new gaps in this document if
  it cannot yet.
- A production art PR follows D63 and `AGENTS.md`: real runtime assets, the generator or
  runtime integration, provenance rows (`tools/art/provenance_people.toml`,
  `tools/art/prompt_library_check.py`), palette and dimension validation, focused guards, and
  contact-sheet or in-game review evidence. Mockups are direction, never checkpoints.
- Variation inside a kind stays bounded and coherent (§1); distinctness is spent between
  kinds, where the gate measures it.

Sources: `docs/ART_SPEC.md`, `docs/ART_BIBLE.md`, `docs/reviews/readability-2026-10-19/`
(C53), DECISIONS D58 / D63 / D64, #485, `tools/art/silhouette_check.py`,
`crates/hf-sim/src/view/look.rs`, `godot/scripts/render/entity_layer.gd`.

## Appendix: the 110 under-threshold pairs on master (2026-10-31)

Measured by `python3 tools/art/silhouette_check.py --full` on master's sheets; S/W are the
per-view outline distances, min is the pair's gate score.

| Figure A | Figure B | S | W | min |
|---|---|---:|---:|---:|
| gnarrow_average_child | human_average_adult | 0.181 | 0.784 | 0.181 |
| gnarrow_slim_child | human_average_adult | 0.192 | 0.796 | 0.192 |
| gnarrow_broad_child | human_broad_adult | 0.200 | 0.720 | 0.200 |
| gnarrow_broad_child | skarri_slim_adult | 0.210 | 0.615 | 0.210 |
| hollin_slim_child | seamfolk_average_child | 0.212 | 0.572 | 0.212 |
| human_broad_adult | skarri_slim_adult | 0.218 | 0.275 | 0.218 |
| human_average_child | tharn_slim_child | 0.260 | 0.222 | 0.222 |
| gnarrow_average_child | human_broad_adult | 0.224 | 0.725 | 0.224 |
| gnarrow_slim_child | human_slim_adult | 0.225 | 0.797 | 0.225 |
| human_broad_child | tharn_average_child | 0.280 | 0.228 | 0.228 |
| gnarrow_broad_child | human_average_adult | 0.228 | 0.780 | 0.228 |
| human_broad_adult | meer_average_adult | 0.299 | 0.229 | 0.229 |
| human_broad_child | tharn_slim_child | 0.317 | 0.234 | 0.234 |
| grisk_average_adult | meer_average_child | 0.234 | 0.381 | 0.234 |
| human_broad_adult | meer_slim_adult | 0.343 | 0.235 | 0.235 |
| grisk_slim_adult | meer_slim_child | 0.236 | 0.390 | 0.236 |
| meer_broad_child | skarri_slim_child | 0.239 | 0.330 | 0.239 |
| human_broad_child | tharn_broad_child | 0.335 | 0.241 | 0.241 |
| gnarrow_slim_child | screeling_slim_adult | 0.242 | 0.599 | 0.242 |
| gnarrow_average_child | skarri_slim_adult | 0.243 | 0.619 | 0.243 |
| grisk_broad_child | murrow_slim_child | 0.244 | 0.332 | 0.244 |
| meer_broad_adult | skarri_broad_adult | 0.247 | 0.325 | 0.247 |
| meer_broad_adult | skarri_average_adult | 0.249 | 0.307 | 0.249 |
| human_broad_adult | meer_broad_adult | 0.252 | 0.249 | 0.249 |
| hollin_slim_adult | seamfolk_average_adult | 0.250 | 0.569 | 0.250 |
| meer_broad_child | skarri_average_child | 0.250 | 0.319 | 0.250 |
| grisk_slim_adult | meer_average_child | 0.250 | 0.340 | 0.250 |
| murrow_broad_adult | skarri_broad_adult | 0.251 | 0.377 | 0.251 |
| human_slim_child | seamfolk_broad_child | 0.255 | 0.269 | 0.255 |
| human_broad_adult | skarri_average_adult | 0.256 | 0.316 | 0.256 |
| meer_broad_child | skarri_broad_child | 0.257 | 0.335 | 0.257 |
| human_average_adult | seamfolk_broad_child | 0.397 | 0.259 | 0.259 |
| human_average_child | tharn_average_child | 0.348 | 0.259 | 0.259 |
| gnarrow_broad_child | skarri_slim_child | 0.260 | 0.603 | 0.260 |
| gnarrow_average_child | skarri_slim_child | 0.260 | 0.608 | 0.260 |
| hollin_broad_adult | human_broad_adult | 0.261 | 0.483 | 0.261 |
| gnarrow_average_child | human_slim_adult | 0.262 | 0.785 | 0.262 |
| grisk_broad_adult | meer_broad_child | 0.263 | 0.337 | 0.263 |
| gnarrow_broad_child | screeling_broad_adult | 0.264 | 0.513 | 0.264 |
| meer_broad_adult | skarri_slim_adult | 0.265 | 0.300 | 0.265 |
| human_average_child | screeling_average_child | 0.266 | 0.509 | 0.266 |
| human_slim_adult | seamfolk_broad_child | 0.283 | 0.266 | 0.266 |
| gnarrow_average_child | screeling_slim_adult | 0.266 | 0.587 | 0.266 |
| meer_average_child | skarri_slim_child | 0.267 | 0.357 | 0.267 |
| human_broad_child | screeling_average_child | 0.267 | 0.510 | 0.267 |
| human_broad_adult | tharn_average_adult | 0.268 | 0.272 | 0.268 |
| gnarrow_slim_child | screeling_average_adult | 0.268 | 0.579 | 0.268 |
| gnarrow_broad_child | skarri_average_child | 0.268 | 0.578 | 0.268 |
| grisk_average_adult | meer_broad_child | 0.269 | 0.402 | 0.269 |
| grisk_slim_adult | human_average_child | 0.269 | 0.293 | 0.269 |
| grisk_slim_child | murrow_broad_child | 0.462 | 0.270 | 0.270 |
| murrow_broad_child | skarri_broad_child | 0.271 | 0.394 | 0.271 |
| human_average_child | meer_slim_child | 0.271 | 0.342 | 0.271 |
| grisk_average_adult | meer_slim_child | 0.272 | 0.429 | 0.272 |
| human_average_adult | meer_slim_adult | 0.286 | 0.273 | 0.273 |
| human_broad_child | meer_average_child | 0.274 | 0.294 | 0.274 |
| meer_broad_child | murrow_slim_child | 0.274 | 0.441 | 0.274 |
| hollin_broad_child | human_average_child | 0.275 | 0.408 | 0.275 |
| human_broad_adult | tharn_broad_adult | 0.332 | 0.275 | 0.275 |
| human_average_adult | screeling_slim_adult | 0.275 | 0.554 | 0.275 |
| hollin_average_child | seamfolk_broad_child | 0.275 | 0.525 | 0.275 |
| human_slim_adult | meer_slim_adult | 0.307 | 0.276 | 0.276 |
| human_slim_adult | seamfolk_average_child | 0.341 | 0.276 | 0.276 |
| hollin_slim_child | seamfolk_broad_child | 0.276 | 0.554 | 0.276 |
| hollin_average_adult | human_slim_adult | 0.276 | 0.468 | 0.276 |
| human_broad_child | meer_broad_child | 0.277 | 0.314 | 0.277 |
| gnarrow_average_child | screeling_average_adult | 0.279 | 0.567 | 0.279 |
| human_slim_child | screeling_average_child | 0.280 | 0.594 | 0.280 |
| human_slim_adult | tharn_slim_adult | 0.348 | 0.280 | 0.280 |
| gnarrow_broad_child | screeling_average_adult | 0.280 | 0.562 | 0.280 |
| screeling_broad_adult | skarri_average_adult | 0.280 | 0.412 | 0.280 |
| grisk_slim_child | murrow_average_child | 0.433 | 0.281 | 0.281 |
| hollin_broad_child | meer_average_child | 0.283 | 0.433 | 0.283 |
| human_broad_adult | screeling_broad_adult | 0.284 | 0.449 | 0.284 |
| murrow_broad_child | skarri_average_child | 0.284 | 0.362 | 0.284 |
| gnarrow_broad_child | skarri_average_adult | 0.286 | 0.587 | 0.286 |
| gnarrow_slim_child | skarri_slim_child | 0.286 | 0.615 | 0.286 |
| gnarrow_slim_child | human_broad_adult | 0.286 | 0.736 | 0.286 |
| hollin_average_child | human_average_child | 0.288 | 0.377 | 0.288 |
| meer_average_child | skarri_average_child | 0.288 | 0.349 | 0.288 |
| meer_average_child | murrow_slim_child | 0.288 | 0.444 | 0.288 |
| gnarrow_broad_child | murrow_slim_adult | 0.288 | 0.695 | 0.288 |
| human_broad_child | murrow_slim_child | 0.288 | 0.300 | 0.288 |
| grisk_broad_child | murrow_broad_child | 0.334 | 0.288 | 0.288 |
| gnarrow_broad_child | skarri_broad_child | 0.288 | 0.543 | 0.288 |
| hollin_slim_adult | human_slim_adult | 0.289 | 0.550 | 0.289 |
| human_average_child | tharn_broad_child | 0.424 | 0.290 | 0.290 |
| grisk_average_child | murrow_slim_child | 0.290 | 0.369 | 0.290 |
| hollin_slim_child | human_slim_child | 0.290 | 0.550 | 0.290 |
| human_broad_child | skarri_slim_child | 0.293 | 0.370 | 0.293 |
| meer_slim_child | murrow_slim_child | 0.293 | 0.449 | 0.293 |
| human_average_adult | seamfolk_broad_adult | 0.293 | 0.324 | 0.293 |
| grisk_slim_child | human_broad_child | 0.400 | 0.294 | 0.294 |
| human_broad_child | murrow_broad_child | 0.399 | 0.294 | 0.294 |
| grisk_slim_child | human_average_child | 0.351 | 0.295 | 0.295 |
| grisk_slim_child | murrow_slim_child | 0.353 | 0.295 | 0.295 |
| hollin_average_child | human_slim_child | 0.296 | 0.515 | 0.296 |
| gnarrow_broad_child | screeling_slim_adult | 0.296 | 0.582 | 0.296 |
| grisk_broad_adult | meer_average_child | 0.296 | 0.318 | 0.296 |
| grisk_slim_adult | hollin_broad_child | 0.296 | 0.315 | 0.296 |
| human_broad_child | meer_slim_child | 0.296 | 0.303 | 0.296 |
| human_average_child | meer_average_child | 0.297 | 0.332 | 0.297 |
| grisk_broad_adult | murrow_broad_child | 0.297 | 0.426 | 0.297 |
| human_broad_adult | tharn_slim_adult | 0.337 | 0.298 | 0.298 |
| gnarrow_average_child | skarri_average_child | 0.298 | 0.583 | 0.298 |
| grisk_broad_child | murrow_average_child | 0.298 | 0.300 | 0.298 |
| meer_average_adult | skarri_slim_adult | 0.299 | 0.326 | 0.299 |
| screeling_broad_adult | skarri_slim_adult | 0.299 | 0.421 | 0.299 |
| hollin_broad_child | seamfolk_broad_child | 0.299 | 0.546 | 0.299 |
| gnarrow_average_child | screeling_broad_adult | 0.299 | 0.518 | 0.299 |
