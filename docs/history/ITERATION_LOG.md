# Iteration log

## 2026-10-30 — Underground production art slice (#414)
The first underground art pass was reviewed against the previous sheets before changing the
assets. Cave floor now reads as damp earth rather than a second black-rock tile; dark and black
rock carry broad, broken strata; ore families retain their material-specific veins; gem-bearing
rock adds restrained gem-ramp facets without claiming a specific species; and the existing water
palette is used for sparse seepage only where a real neighbouring water tile exists. A transparent
strata-face overlay is used only on exposed natural rock below the surface. No production simulation
behaviour, water classifications, plants, animals or effects were changed; the art-only demo gains
one real gallery water tile for integrated evidence. Evidence:
`docs/reviews/c414-underground-assets-2026-10-30/before.png`, `after.png`, and the integrated
`integrated_after.png` gallery frame. The #519 follow-up guards exposed/buried faces, six-level
peels, water/no-water adjacency and deterministic gallery water in `tools/gd_test.gd`.
Guards: `python tools/art/underground_check.py` (now mandatory in `tools/ci.sh`),
palette/provenance/README checks, Godot 4.6.3 `GD_CHECK 18 scripts, 0 failed` and
`GD_TEST 20 passed, 0 failed`.

Entries for golden changes, re-baselines, and reviews. Newest at the bottom.

## 2026-10-13 — Phase 1 representative scene
Staged scene rendered and reviewed (docs/reviews/2026-10-13-phase1-representative-scene.md).
Art bible written from it. Images kept under docs/reviews/.

## 2026-10-13 — Phase 3 glance review
Integrated-sim footage on seeds 1–3 (docs/reviews/2026-10-13-phase3-glance-review.md).
Findings led to: natural ramps in worldgen (the surface was 10 % reachable before), arrival
provisions and food-aware founding site, hunger-driven searching, sleeping through the night,
hunger slowed during sleep, director weighting toward present busy people, softer night.
Verdict: autonomy gates pass; ambient experience not yet acceptable (empty world) → Phase 4.

## 2026-10-13 — Phase 4 glance review
Work and settlement in: community planner, designations, leases, plans, haul/build/mine/
dig-stairs/sow/harvest/cook, stockpile expansion, conservation ledger. Fixed on the way: a
work order chosen every 40 ticks with no cooldown (thrash), sites placed where nobody could
reach, walls built over standing people, work steps rejecting a worker one level up, plans
ending a tick after their order completed. Review: docs/reviews/2026-10-13-phase4-glance-review.md.

## 2026-10-14 — Phase 5 memory and social
Memories, consolidation, bias, relationships, mood; settlement keeps building after the
house (floors, furniture, workbench, lamps). Review: docs/reviews/2026-10-14-phase5-memory-review.md.

## 2026-10-14 — Phase 6 persistence and viewer
Save prepare/commit split (0.9 ms on the sim thread), immutable loads, ring + autosave,
startup load, bookmarks, settings, procedural sound. Found and fixed: loading opened the
save's chronicle and corrupted it; the first version wrote a temp file on the sim thread
(1 s hitch on the spinning disk). Review: docs/reviews/2026-10-14-phase6-persistence-review.md.

## 2026-10-14 — Phase 7 communities and economy
Community records, per-community planning, roles, arrivals judged by a norm, cohesion and
splits with cause, trade between communities, "Chose" events that show memory-driven
choices in the feed; turned-away wanderers now leave the valley. Review:
docs/reviews/2026-10-14-phase7-society-review.md.

## 2026-10-14 — Phase 8a generations
Genome, inheritance, development, pairing, births, children, aging, death, lineage. A
compressed-lifetime test showed population outrunning food; fields now scale with
population and the gate runs at normal life speed. Review:
docs/reviews/2026-10-14-phase8-generations-review.md.

## 2026-10-14 — Phase 8b culture
Customs with recorded origins, narratives, holders, fading, and hooks; taught-history rows
in the chronicle joined to the real event. Review: docs/reviews/2026-10-14-phase8b-culture-review.md.

## 2026-10-14 — Phase 9a wildlife, hunting, quarrels
Animals with their own rules, hunts that chase a moving target, quarrels. Review:
docs/reviews/2026-10-14-phase9a-wildlife-review.md.

## 2026-10-14 — Phase 9b depth and discovery
Cellar under the house, mine lamps, mushrooms, cave and ore discoveries as community
knowledge, cave-ins with injury, shared knowledge of the community store. Review:
docs/reviews/2026-10-14-phase9b-depth-review.md.

## 2026-10-14 — Phase 10 expeditions
Overworld sites, caravans for trade or scouting, day-step travel with encounters, return or
loss with persistent outcomes; people away are moved whole off the map and come back as
themselves. The wildlife gate fell to 10 survivors when a party of three left an
eight-person founding, so a community now needs ten able adults and keeps seven; the client
founds with twelve. Review: docs/reviews/2026-10-14-phase10-expeditions-review.md.

## 2026-10-14 — Phase 11 history and director
Never-invents audit, history view (eras, record with detail, custom origins), discovery
view, director presence weighting. The footage loop had been easing the camera back to the
opening focus between frames since Phase 3; fixed, and the director re-measured with a new
CLI diagnostic (0/30 empty frames on two seeds). Review:
docs/reviews/2026-10-14-phase11-history-director-review.md.

## 2026-10-14 — Phase 12 hardening (in progress)
Soak, profile and director commands. The first 600-day soaks starved whole settlements at
year ends and spent 20 ms every game-hour in the planner; four economy passes (fields,
hunting, cooking, winter larder, farmland placement, staggered starvation, newcomer rate),
an occupancy set in the planner and an array union-find in navigation (both digest-
identical), item stack merging and a growing store. Packages for Linux and Windows,
README, recovery demonstration, issue 17 fixed. Details: docs/soak/README.md and
docs/reviews/2026-10-14-phase12-hardening-review.md.

## 2026-10-14 — Phase 12: the cellar nobody could enter
Soak v5 and v6 kept piling grain on the fields. The reach trace found every failed haul
was a walk down to the cellar store: the dig left an Up stair over an Up stair, which the
step rule climbed but would not descend, while navigation regions union links undirected,
so `reach()` said yes and A* said no, forever. Vertical links are now symmetric (either
tile asserts the link, both directions), with a gate test. Also: field tiles are no longer
construction sites for hauling, open hauls scale with people, stacks merge on a tile, the
store grows with people, A* budget 4,000 → 12,000.

## 2026-10-15 — Phase 13 life on screen
Look resolver (season, job, journey, age, injury, tool), overlays and second idle frames in
the generators, wall tops redrawn, weather particles, snow cover, seasonal tint, tree sway,
occupied doors. First pass found rain falling in the cellar (fixed: weather only where the
sky is overhead) and green broadleaf trees in snow (tinted; bare canopies still owed).
Review: docs/reviews/2026-10-15-phase13-life-on-screen-review.md.

## 2026-10-15 — Phase 14 materials and workshops
Recipes as data, craft orders through the existing material-delivery path, four workshops
the planner builds and feeds from need, shared tools with quality and wear, clothes that
wear out. Unscripted seed 1 had all four workshops and three tools in store by day 30.
Review: docs/reviews/2026-10-15-phase14-materials-review.md.

## 2026-10-15 — Phase 15 bodies and health
Body model, health system, harm from work and hunts and cave-ins, winter fever, a healer
with poultices and tending orders. The tending gate first failed because the test had put
the poultices inside a house wall; the tracing added for it (`HF_TRACE_TEND`) stays.
Review: docs/reviews/2026-10-15-phase15-health-review.md.

## 2026-10-15 — Phase 16 minds under strain
Thoughts, stress and breaks. First calibration (stress from low mood) gave zero breaks in
forty fed days; stress now builds from hardship directly and eases with calm and good mood,
giving about one break per day and a half among forty people. Grief blocks re-pairing.
Review: docs/reviews/2026-10-15-phase16-strain-review.md.

## 2026-10-17 — Terrain and plant families reachable by the generator (issues #44 #52 #58 #41)
World generation changes for every seed, so digests and every seeded frame change. The
generator now has climate fields, a per-world character drawn from the seed, and eight biomes
that choose the surface ground and plants; lakes fill the valley's lowest ground and deep water
lies only in lake centres. Eight plant kinds, six materials and the goat are appended. Before
and after frames on seeds 1, 7, 10 and 16 in three seasons:
docs/reviews/terrain-reachable-2026-10-17/.

## 2026-10-22 — The day's temperature curve the right way up (#256, C71)
Every digest changes: `systems/weather.rs` had the diurnal term's sign reversed, so the warmest
hour was 02:24 and the coldest 14:24, and in winter people took frostbite at work in the
afternoon while nights on the ground were the mildest hours. Now the afternoon is the warmest
and the hour before dawn the coldest. Soak tables and goldens from before this are of the old
curve; `ostrog weather --seed 7 --days 1` shows the new one.


## 2026-10-22 — Portable float maths: one digest on Linux and Windows (#234, fix/234)
Every digest after tick 0 changes, on both platforms: the sim's `exp`, `ln`, `powf`, `sin`,
`cos` and `atan2` now come from the `libm` crate (`fmath::Det`) instead of the platform's C
library, whose last bits differ between glibc and MSVC. Digests, goldens and soak tables quoted
before this are of the platform maths (and the Linux and Windows values of them differ); from
this commit the two platforms agree (`run --opening --seed 3 --days 3`: `c6e78cbb...`).

## 2026-10-30 — Production shoreline asset pass

The water generator now has broad broken current bands, moving surface glints, and a lower-
contrast deep-water treatment. The simulation contract is unchanged: shallow/deep selection
still comes from the tile state, and the art does not imply unsupported water-quality facts.
Review: `docs/reviews/production-shoreline-2026-10-30/`.

## 2026-10-31 — Limited wild-flora communities (#486, first slice)

Every seed's world generation, initial digest and subsequent simulation can change: each world
now records eight distinct kinds drawn from the 22 wild additions of #286, rather than allowing
the whole catalogue. Core, crop and cave flora are not bag-drawn. Existing habitat filters and
plant assets are unchanged; climate-eligible low-flora density is redistributed among drawn
kinds, with exact full-catalogue site controls over eight seeds. Placement, kind selection and
cosmetic variation use independent streams so the draw cannot shift site occupancy.
`ostrog describe` and the setup preview name both drawn and undrawn kinds from the actual
record; legacy worlds report an unrecorded draw rather than a seed-derived invention. Focused
flora/biome guards pass; full Rust/Godot validation and viewer review are left to box-ci and a
subsequent running-viewer pass; no visual golden review is claimed here.
## 2026-10-30 — Rare hillside cave mouths (#482, PR #501)

World-generation digests and downstream seeded frames change where a hillside mouth is cut. The
review hardening narrows eligible destinations to orthogonally connected cave rooms of at least
eight floors, preserves natural-ramp approaches, and keeps the actual stratum's material in cut
ramps and sills; these corrections can change which mouths survive and their tiles. The 2026-10-31
follow-up keeps the hillside face as a virtual search node but starts the cut on a downward ramp
one level below it, so the existing surface rock remains as the brow instead of adding a new
hilltop. Preserving the original search also retains valid shallow-cave routes. Mouth tiles and
downstream digests change again while surface relief and biome ground stay unchanged. The original
screenshots under `docs/reviews/482-cave-mouths/` were captured at a28d5e31 before this review
hardening. Matched pre-fix d5e7f616 / fixed 0052a2c9 noon frames, two corrected mouths and their
real cave destinations are reviewed in `docs/reviews/482-cave-mouths/post-hardening-0052a2c9/`.
## 2026-10-30 — Production terrain-edge flora and wildlife slice (#414)
The first production art slice after the renderer review: authored procedural plants now have
stronger canopy depth, shrub identity, layered reeds, living alien fungus/bone details, and a
reserved seasonal alien-bloom asset family (not currently spawned or reachable in play).
Familiar animals gain readable species markings while preserving their existing animation-sheet
contract. Before/after contact sheets and the focused asset guard are in
`docs/reviews/c414-terrain-vegetation-2026-10-30/`.

## 2026-10-31 — Dimensional core dispositions and bounded surface traits (#488)

New-game social, work, wandering and hauling weights now use continuous core dispositions and
relevant acute state, so post-arrival digests and downstream presentation are expected to change.
No new random-stream draws or enum variants were added. Old saves without profiles retain their
legacy binary-trait weights. The deterministic scenario guards are in `tests/dispositions.rs`;
box-ci and integrated review must establish the new behavior and performance evidence.

## 2026-10-30 — Habitat-backed cave fauna (#493, PR #495)

Opening-world generation and subsequent digests change: cavern bat roosts now scale with pools
and deep habitat, the crawler selects the deepest eligible deep cavern, and the burrower prefers
an underworld cavern. Dry cavern floors are enumerated before seeded selection rather than found
by 40 random samples, guaranteeing the requested cavern population whenever eligible floor exists;
a flooded cavern has no bats or crawler and the burrower uses its rock fallback.

## 2026-11-01 — Arrival survival relief no longer erases the shock (#501)

Opening and downstream simulation digests change: the base arrival-thought weight moves from -0.35
to -0.55 so the intended 1.5x survival urgency can improve health without making the first day's
mood net positive. Temperament shaping, shock duration and survival urgency are unchanged; the
`qa_1026` arrival guard's expectations were not changed.

## 2026-11-10 — Middle-strata sub-bands (#267, WP-05 5.6)

World generation and every downstream digest change: the middle stratum generates one or two
interior breaks per valley (a valley-wide count) and splits into sub-bands alternating the
column's two hard rocks, shallow band unchanged (marble patch included). The rock below the
first break is no longer one material to the deep, so tin and silver vein acceptance (keyed on
granite/slate tiles) shifts with the banding. Old saves load unchanged (`bands` is
serde-default empty and reads as one middle rock). No materials or enums were added; the
aquifer and well rules key on the stratum breaks, which keep their meanings.

## 2026-11-10 — One temperature, lapsed by height (#268, WP-05 5.6)

Simulation digests change: temperature reads are per-position now — the valley reading
minus 1.5 °C per level above the founding site's ground, warmer per level dug below it.
A field on a height frosts on a night the valley floor survives; a high tarn freezes
earlier; a dug cellar reads warmer than the winter surface. The block facts' flat 10 °C
underground display is replaced by the same rule (the rain/snow sky call, snowmelt,
evaporation, spell leans and narration stay valley-wide).

## 2026-11-10 — Loose rock on steep bare ground (#289, WP-05 5.5)

World generation and every downstream digest change: columns whose steepest step to
a neighbour is two levels or more, with a bare cover (soil, grass, sand), lie as
gravel or scree by a seeded patch noise — scree on the steepest faces. No new
materials, enums or saved state; flora placement and the founding site selection run
after the pass and can shift with it. The weather ground overlay (mud and lying
snow, doc items 2+3) is the next #289 PR.

## 2026-11-10 — The weather ground overlay, sim-side (#289, WP-05 5.5)

Simulation digests change: `cover.rs` derives the ground's weather wear from the
saved weather alone, mirroring the client's long-standing looks — frozen soil
below -1 °C, wet soil in rain, snow lying on exposed covers in a cold winter.
Fire spreads no more across ground the watcher sees under snow, and block facts
name the snow, the frost and the wet where the look exists. No rendering change
(the client drew these looks already), no new materials or saved fields.
Movement costs for snow and mud are deliberately not in this slice (Dan's open
question in docs/design-grounds.md).

## 2026-11-16 — Industry weighs on standing about (#906, #907, alive audit draft B4b/B4c)

Simulation digests change: `systems::decide::choose` gained a `temperament(&Goal)` term
(#906), multiplied into every option's score and named by the Why? lens exactly like
`spirits`; it returns 1.0 for every goal except `Goal::Loiter` (#907), where it now
returns `Agent::loiter_weight()` — `0.5 + 1.0*(1 - industry)` from the core disposition
profile, or a legacy `0.6`/`1.2`/`1.0` from `Trait::Industrious`/`Easygoing`/neither. A
person at the core profile's default industry (0.5) or with no relevant legacy trait is
unaffected (the factor is 1.0); anyone with industry away from 0.5 now loiters more or
less. No new saved state.

## 2026-11-16 — Industry weighs on taking up work (#906, #908, alive audit draft B4b/B4d)

Simulation digests change: `systems::decide::choose` gained a `temperament(&Goal)` term
(#906), multiplied into every option's score and named by the Why? lens exactly like
`spirits`; it returns 1.0 for every goal except `Goal::Work`, where it now returns
`Agent::work_weight()`. That weight itself widened from `0.7 + 0.6*industry` (0.7-1.3) to
`0.4 + 1.5*industry` (0.4-1.9) from the core disposition profile, with the legacy
`Trait::Industrious`/`Easygoing` fallback widened from 1.3/0.8 to 1.6/0.6 to match (#908).
A person at the core profile's default industry (0.5) is unaffected (0.4+1.5*0.5=1.15 vs
the old 0.7+0.6*0.5=1.0 — see below); anyone away from 0.5, or with the legacy trait, now
takes up or avoids open work more than before. No new saved state.

Note: 0.5 industry is no longer exactly weight 1.0 under the new formula (it is 1.15); this
was accepted as part of widening the range rather than held fixed, since the issue's own
proposed constants (`0.4 + 1.5*industry`) do not pass through 1.0 at the profile's default.

## 2026-11-16 — Order weighs a haul or a cook order (#906, #1761, alive audit draft B4b/B4e)

Simulation digests change: `systems/decide::choose`'s `temperament` closure (#906) now reads
`Agent::order_weight()` (`0.8 + 0.4 × order`, 1.0 for a legacy person) for `Goal::Work(did)`
when the designation is a haul or a cook order, instead of always 1.0. Every other designation
kind, and every other goal, is unaffected. `haul_fit`'s own `0.3 × order` term (added directly
to the haul score, unrelated to `temperament`) is untouched, so a very orderly person's pull
toward hauling is now stronger than before (the two terms stack); a cook order now carries any
pull from order at all, where before it carried none.

## 2026-11-16 — Cheerfulness weighs talk (#906, #1760, alive audit draft B4b/B4c)

Simulation digests change: `systems/decide::choose`'s `temperament` closure (#906) now reads
`Agent::cheer_weight()` (`0.85 + 0.3 × cheerfulness`, 1.0 for a legacy person) for
`Goal::Talk(..)`, instead of always 1.0. Every other goal is unaffected. A
cheerful person is somewhat more likely to be the one who stops for a word; a glum one
somewhat less.

## 2026-11-16 — Independence weighs hearth-sitting and wandering (#906, #1759, alive audit draft B4b)

Simulation digests change: `systems/decide::choose`'s `temperament` closure (#906) now reads
`Agent::independence()` (current, mood-shifted `independence`, or 0.5 for a legacy person) for
two leisure goals: `Goal::Gather(_)` (sitting at the hearth) gets `1.2 − 0.4 × independence()`,
`Goal::Wander(_)` gets `0.8 + 0.4 × independence()`. Every other goal, including every
survival goal, is unaffected. A self-reliant person is somewhat less likely to sit at the
hearth and somewhat more likely to wander off alone; a communal one the reverse.

## 2026-11-16 — Industry weighs stocking the store (#906, #1758, alive audit draft B4b)

Simulation digests change: `systems::decide::choose`'s `temperament` closure now reads
`Agent::stock_weight()` (`0.6 + 0.8 * industry`, 1.0 for a legacy person) for
`Goal::Stock(_)`, instead of always 1.0. An industrious person is now somewhat more
likely to go gather for the store, and an idle person somewhat less, when the store
runs short; every other goal is unaffected.

## 2026-11-16 — The retired `place_affect` key no longer appears in a save (#1268)

Every world's digest moves once: `Knowledge::place_affect` is now
`#[serde(default, skip_serializing)]`, so the encoded snapshot no longer carries
this key (it was already written and read by nothing; place feeling has lived
in `memory::place_multiplier` for some time). An old save with a non-empty
`place_affect` still loads; re-saving it drops the key. No other behaviour
changes.

## 2026-11-15 — A wild creature's death is Major only when a person saw it (#809, D98)

Simulation digest changes: `systems/life.rs::die_at` no longer pushes every `Died`
event at `Severity::Major`. A person's or a kept (herd) animal's death stays Major;
an unowned wild creature's is Minor when a person saw it (the same sight rule
`accounts::witnessed` uses: awake, grown, same level, within `WITNESS_WITHIN`
tiles, in line of sight) and Trace otherwise. Because `deity.rs::step` resets
`deity_watch.last_major` from any Major event, a valley whose wildlife dies
unseen (a cave of bats, for instance) now reads as quiet to the deity sooner
than before, and may act on a quiet valley earlier than it would have.
Chronicle, feed and toast content for wild deaths is unchanged in kind, only in
how often they reach Major severity. No new fields, materials or saved state.

## 2026-11-15 — A new person's name never matches one already in the valley (#862, D99)

Simulation digest changes: `names::person_name`'s draws for a birth, an arrival or a
joiner (`World::spawn_agent`, `World::give_birth`) are now rerolled, deterministically
from the same seed and id, when the plain roll matches a name already held by anyone
in `w.agents`, living or dead — up to 64 tries, keeping the last roll if it never
clears. Every person born or arriving after the first collision in a valley's history
can get a different name than before this change. A name a departed band took with it
is not checked against (the agent record is removed outright, so nothing of it
remains to collide with).

## 2026-11-15 — Cold lifts roofing's priority, not only a homeless dusk (#859)

Simulation digest changes: `decide.rs`'s designation-scoring loop now gives
`BuildRoof` its own 1.6x lift, triggered by `roofless` (as before) **or** by
`a.needs.warmth < 0.5` — the same threshold `freezing` already checks. Before
this, a hard cold did nothing to a roof order's score once everyone already
knew of a bed, so roofs could sit untaken for hundreds of days while other
work won out; now a cold worker weighs an open roof order well above equally
fit, equally distant work. Every other build kind's weighting (`roofless`
only) is unchanged.

## 2026-11-15 — A valley hamlet founds on arrival, not on departure (#1322)

Simulation digests change for any save where a community leaves the valley: the realm site a
departure founds is no longer created at the moment of the choice, with the full headcount
that set out, but on the first leaver who actually steps off the map, starting at population 1
and growing by 1 with each later arrival. A departure nobody survives founds nothing at all
(previously it founded a hamlet at population 0, which the realm's own yearly step would then
end as starved). The region and name are still drawn at the choice (so the departure line can
say where), carried on the new `Agent.bound_for` field and each leaver, and reused unchanged
when the site is actually founded. A people's later valley hamlet is narrated as a plain
founding, not "made their first home" (that wording is now kept for a people's actual first
site).

## 2026-11-16 #1020: set fittings keep their quality

`Structure` gains `quality` (0.5 for raw-built kinds and old saves). Nothing reads it yet, so
behaviour is unchanged, but the new encoded key moves every digest once.

