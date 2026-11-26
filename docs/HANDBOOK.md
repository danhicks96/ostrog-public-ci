# Ostrog: the handbook

Where the project is, where it is going, how it works, and how to carry it on.

## Current orientation (2026-10-30)

For current work, read `VISION.md`, `DECISIONS.md` D59–D64, `docs/history/PLAN-NEXT.md`,
`docs/design-minds-observation.md`, `docs/ART_BIBLE.md`, `docs/ART_SPEC.md` and
`DEFINITION_OF_DONE.md` first. They preserve the original autonomous, non-omniscient, real-history
intent while adding situated observation, weighted personal responses, problem/reaction/learning,
local fluids, and the production-art acceptance gate. All active work is on D:; C: and P: paths
below are historical records, not working locations. Mockups are direction, not shipped art.

Written 2026-10-18 by the Linux build-server agent for the maintainer's archive, for testers, and for any developer who
has to pick the project up cold. It describes master at commit `8b4f93f` (C33 merged, tag `c33`)
and the two checkpoints built after it on their own branches, `c34-crafts` and `c35-masterworks`
(section 5.3). Where this document and the code disagree, the code is right and this document is
out of date; where this document and an older plan disagree, this document is newer.

Contents

1. What Ostrog is
2. The rules that do not bend
3. The goal: what "finished" means
4. A tour of the game as a player sees it
5. Where we are
6. Architecture
7. Content reference
8. Art and audio
9. Building, running and the tools
10. Testing: how to check that it works
11. Working rules: git, checkpoints, records
12. The road ahead: the M4 review and M5 to M8
13. Known problems, gotchas and stale records
14. If something bad happens: recovery
15. Glossary
16. Where to find things

---

## 1. What Ostrog is

Ostrog is an autonomous colony-world simulation built to run on a second monitor. In the
words of VISION.md: "The viewer is a witness and optional caretaker. The inhabitants, creatures,
communities and expeditions create the activity through their own needs, skills, knowledge and
relationships."

Ten people come through a portal into a valley they have never seen. Three arrive together; the
other seven are scattered somewhere in the valley, alive or dead, until someone goes and looks.
From then on nobody gives them orders. They eat what they know of, build what they need, learn by
watching each other, fall out, pair, have children, grow old and die. Strangers come from the
hungry places beyond the valley. A deity, chosen at the start, leans on chance. The world beyond
the valley has centuries of its own recorded history and goes on after they arrive.

The player watches. The tools for watching are the camera (which a "director" can steer to where
something is happening), the event feed, the inspect panel for any person, the History panel, the
Legends browser for the wider world, and a "while you were away" summary.

### The experience it is built for

VISION.md describes it: glance over from work and see "people doing recognisable things for
recognisable reasons: a builder carrying stone to a half-finished wall, a cook at the hearth, two
friends talking by the river, a child following a parent, a mining party lamp-lit three levels
down, a caravan leaving at dawn." Look back an hour later and "understand what changed from the
event feed and the history view, without the game ever having asked for a decision."

The maintainer's acceptance test (epic #46): leave it running on a second monitor for 15 to 30 minutes and
there should be at least one story worth telling, without forcing it.

### An example of what that means

This is the kind of chain the simulation is built to produce, each step from systems that exist on
master today:

1. A hungry village beyond the valley (a real settlement of the world's history, with a people,
   a language and stores) runs short of food. It sends a band of real people towards the valley.
2. One of the valley's people sees them near home. The feed says strangers were sighted. The
   community now counts as threatened.
3. The settlement planner, which handles logistics only, asks the smith for spears because a threat
   is recent and there are iron bars and planks.
4. That night two strangers raid the store. Three of the valley's people hold, from their own pasts,
   that force answers force. They take up the spears themselves; nobody hands them out. The feed
   says "Nalthi took up the iron spear against the strangers."
5. Nalthi goes after a raider and they fight. The spear makes her blows land more often and cut
   deeper. The spear wears and its story records "drew a stranger's blood". The hurt raider turns
   for home.
6. A season later, with no more trouble, the spears are put away at home.
7. Every step is a row in the chronicle. The History panel can show it, and hovering the spear tells
   who made it, who carried it and what it did.

None of those steps is scripted. If the valley's people had held against violence, the same band
would have been met with an envoy or welcomed, and the store would have been shared or raided.

### Where it comes from

VISION.md names four lineages:
- RimWorld for readable settlements, production chains, individuals, caravans and emergent stories;
- Dwarf Fortress for the vertical world, excavation, underground discovery, persistent consequences
  and historical depth;
- WorldBox for many independently acting creatures and communities;
- early Sims for competent autonomy.

Survival, growth, comfortable life, exploration, culture, travel, generations, decline and
rediscovery overlap rather than happening in sequence.

Visual direction: top-down 2.5D pixel art, 32 px tiles, people in 48 px cells, one master palette.
The target is between Animal Crossing's approachable material warmth and high-definition
RimWorld/DF mods: "richer than RimWorld, at least as readable," with clear silhouettes and
material/condition changes. Underground looks different: dark base, lamp light, exposed strata.
Layered top-down 2.5D is the current working presentation; a future renderer experiment must
preserve readability and simulation-backed material truth. A mockup is direction, not shipped art.
The world must be compelling before any information panel is opened; when a panel is opened, it
must make people, equipment and object history more legible rather than replace what the world
shows.

---

## 2. The rules that do not bend

These come from VISION.md, DECISIONS.md, AGENTS.md and PLAN-FINISH.md. Every change has to respect
them. Each has a test or a CI check behind it where that is possible.

### The ten defining principles (VISION.md)

1. Competent autonomy. People are never made inept to create work for the player.
2. No omniscience. People act only on what they have perceived or been told. A person who has never
   seen the berry bush does not walk to it. (Test: `autonomy.rs`.)
3. Memory matters. What happened to someone changes what they choose. (Tests: `memory.rs`,
   `aversion.rs`.)
4. Labour is real. Every wall was carried and built; every meal was cooked from something that
   was gathered. Nothing appears. The item ledger balances every creation and use, and soaks fail
   on any drift. (Test: `work.rs` `ledger_zero_drift_default_world`.)
5. History is recorded, never invented. Every line of history traces to a recorded event with a
   real chronicle id. (Tests: `history.rs` `chronicle_only_real_events`, the
   `history_only_from_simulation` gates.)
6. Continuity over scale. People off screen stay the same people (D10, D15). Nobody is
   regenerated.
7. Ambient first. No modal prompts, no mandatory decisions, the simulation keeps running unfocused.
8. Traceable culture. Every custom has an origin event and a behavioural consequence. (Test:
   `culture.rs`.)
9. Ordinary simulation needs no language model. (D9.)
10. Situated minds. People and animals act on their own observations, weighted dispositions,
    needs, relationships and memories; they learn from outcomes but are not omniscient. (D60.)

### The non-negotiables restated for the finish programme (PLAN-FINISH.md)

- Deterministic simulation. The same seed and choices produce the same world, tick for tick (D7).
- No LLM in the simulation.
- Nothing invented. Every shown thing traces to a recorded cause.
- Losing is fun. Famine, a burned settlement, a family line ending are outcomes to record and show
  well, not failures. Tests check coherence (determinism, no stalls, no drift, history only from
  rows), never that people live.
- Local only. No telemetry, nothing phoning home. Where an off-box tool is used during development
  (the image service for reference sheets, section 8), the cost is named and only prompts are sent.
- Pure observation. The player never gives orders. People choose. Scope grows in the simulation,
  the world and the presentation, never in player commands.
- Branch per checkpoint, PR with a merge commit, tag, checkpoint to the mirror (section 11).
- The AI tester on the Windows test PC is the hands-on evidence. There is no human test pass.

### What would undermine it (VISION.md)

Scripted events to keep the screen busy; people who stall, oscillate, deadlock or wander; culture
as decorative names; level of detail that regenerates people; a settlement that changes without
anyone having done the work; a camera director that fixates, switches constantly or shows empty
ground; polish as particles and bloom rather than readability and variety; and "defining success
down to a prototype or demo."

### What is deliberately not done (docs/BACKLOG.md "Not doing")

No scripted events, no modal prompts, no LLM in the loop, no omniscience, no slavery, forced
labour or torture mechanics (justice ends in restitution, reconciliation or exile), no breaking
the art scale (32 px tiles, one palette, provenance rows, palette gate in CI). The "organic test":
a feature is organic only if it comes from existing systems (needs, knowledge, memory,
relationships, labour).

### Words the game uses (docs/design-glossary.md)

The maintainer chose the vocabulary. In anything the player reads:
- "survivor", never pawn, colonist, settler or dwarf;
- "the starting survivors" for the three who begin together;
- "the missing" for the other seven;
- the community's own name in the HUD (for example "Oakfolk"), never "the colony";
- why something happened is said in a person's terms (what they saw, remembered or were told),
  never in system terms;
- the deity is never credited in the world's voice.

`grep -rniE 'pawn|colonist|settler|\bdwarf' godot/scripts/ godot/scenes/` must come back empty.
Internal type names and design prose may still say "settler".

---

## 3. The goal: what "finished" means

### The finish bar

On 2026-10-17, after an honest assessment, the maintainer set the bar: Ostrog should exceed Dwarf
Fortress and RimWorld on every axis, with a Dwarf-Fortress-scale world, pure observation and hybrid
art. The assessment at the time was that Ostrog "leads on organic, traceable causation and
history provenance, but trails DF badly on world generation, physical simulation, industry and
society, and trails RimWorld on content breadth, readability and polish."

"Hybrid art" means code generators for the families that suit them (terrain, plants, people) and
image-model reference sheets for hero art (items, furniture, props, creature poses), everything
snapped to one palette and checked into one style, with provenance recorded for every asset.

### The milestone programme (docs/PLAN-FINISH.md)

The work is a sequence of milestones, each made of checkpoints, one checkpoint per working session,
numbered on from C20. Each milestone ends with a tester round, four 600-day soaks, a profile at its
population target, and a review on the mirror with an honest gap table against Dwarf Fortress and
RimWorld.

| Milestone | Goal | State |
|---|---|---|
| M0 | Finish what was in flight: C20 the thin place, QA round 2 | Done |
| M1 | Scale and foundations: disk-backed chronicle, 500 people within budget | Done (C21) |
| M2 | Art pipeline and style bible v2: reference sheets, 48 px people, 32 px items | Pipeline done (C22, C23); art continues alongside every milestone |
| M3 | The Dwarf-Fortress-scale world: region map, peoples, settlements, figures, wars, Legends | Done (C24 to C30) |
| M4 | Industry, materials and economy | Done (C31 to C36; review C37) |
| MP | Peoples of the world: species library, body plans, cultures, stories, the depths, curses | Designed and decided (docs/design-peoples.md); next after the M4 review |
| M5 | Bodies, conflict, war and order | Not started |
| M6 | Living world depth: fluids, farming, animals, the deep at world scale, families | Not started |
| M7 | Presentation and audio to exceed both games | Not started |
| M8 | Ship: Linux and Windows packages, 24-hour soaks, every Done row evidenced | Not started |

Sections 5 and 12 go through each in detail.

### Where we stand against the two games (M3 review gap table, 2026-10-18)

| Axis | Dwarf Fortress | RimWorld | Ostrog now |
|---|---|---|---|
| World generation | regions, rivers, climate, savagery/evil, caves, underworld, volcanoes | planet of tiles, biomes, factions placed | regions, rivers, rain shadows, climate, biomes; no caves or underworld at world scale yet, no volcanoes (M6) |
| Peoples and cultures | civilisations with ethics, values, languages | factions with ideologies (Ideology DLC) | peoples with languages and convictions (the same model individuals hold); fewer kinds of value than DF; no religions per people yet (M5) |
| History depth | thousands of figures, deeds, artifacts, wars, sieges, written works | almost none | tens of thousands of figures, realms, successions, wars, takings; no artifacts, no deeds beyond leading and dying in battle, no written works |
| Legends browsing | Legends mode, exported XML, third-party viewers | none | in-game browser with linked pages and a map through time; no search, no family trees drawn, map not clickable |
| The world after embark | sites change, armies march, caravans, migrants, sieges | raids, quests, caravans | world steps yearly, neighbours follow, news by caravan; no armies or migrants from the world's wars reach the valley yet (M5) |
| Valley detail | z-levels, fluids, materials, industry | single map, rich colonist life | z-levels, caverns, organic lives, beliefs, construction; industry being built (M4), fluids simple (M6) |
| Art | ASCII or tilesets | clear, readable top-down sprites | consistent pixel art, 48 px people with faces, 32 px derived items; below RimWorld in breadth |
| Observation | fortress run by orders | colony run by orders | pure observation by design; the director, Legends and History are the player's tools |

Ostrog already leads both games on one thing: every visible thing traces to a recorded cause,
people have lived pasts and beliefs with sources, and the valley inherits its world's history. It
trails most in industry and materials (M4, being closed now), bodies and war on the ground (M5),
and breadth of art and content (M2, continuing).

### The Definition of Done (DEFINITION_OF_DONE.md)

Release is done only when every one of 13 rows is verified with its listed evidence. "A subsystem
test passing advances a row; it never completes one. Completion requires working behaviour,
integration into the running game, appropriate presentation, and verification." Documentation
updates cannot verify behaviour rows 1 to 9. As of today no row is verified; rows 1 to 11 and 13
are partial and row 12 is incomplete. The rows were last updated in the Phase 12 to 16 era and do
not yet mention the M3 and M4 work.

| Row | What must be true | Evidence required | Where it stands |
|---|---|---|---|
| 1 | A fresh launch gives an understandable, polished default world, no dev tools | fresh launch on Linux and Windows exports, first-five-minutes footage, onboarding, no console | Linux package launched from the archive headlessly; Windows package never launched on Windows (issue #18) |
| 2 | People sustain ordinary life and pursue goals without routine orders | seeded 10-day runs on 3 seeds with 50+ people, no starvation from inaction, stalled under 1 % | proven by gates and soaks |
| 3 | Memories, traits, inherited variation, relationships, knowledge visibly change behaviour | named scenarios, the inspect panel showing the cause, footage | proven by gates; footage missing |
| 4 | Settlements develop through actual labour and resources | ledger zero drift, time-lapse footage, a build record for every structure | ledger and records pass; no time-lapse footage |
| 5 | Several communities, z-levels, expeditions, caravans, beliefs, generations work together | one seeded 30-day run showing all, plus named tests | tests pass; the single all-in-one run not done |
| 6 | Enjoyable over long unattended sessions, with useful history | 24 h soak on 4 seeds, director footage reviews at hours 1, 6 and 24, away-summary review | director re-measured (0 of 30 frames empty); no 24 h soak |
| 7 | Camera and interface support glances and deeper inspection | an exercised control checklist with screenshots | all controls exist; exercised headlessly only (issue #19) |
| 8 | Save and load preserve state; autosave survives use | restore equals continuous, migration chain, interrupted write, autosave during a soak | all pass; 120 ring saves per 600-day soak verify |
| 9 | Workloads meet performance targets without unbounded memory | soak metrics against targets, RSS curve | 500 people at 1.06 ms mean (C21); disk chronicle |
| 10 | Assets, animation, UI, sound and controls meet the style | art bible review, real runtime assets, goldens, contrast gate, provenance, sound check, integrated-game evidence and object-family coverage | current art is consistent and provenance-checked, but the D58 quality target, layered apparel/animation and object breadth remain partial; a renderer mock-up alone is not acceptance evidence |
| 11 | Onboarding, settings, packaging, startup, errors complete | README quick start followed by someone else, settings, crash recovery | packages build, settings and recovery work; Windows never launched |
| 12 | No known release-blocking defects | zero open issues labelled release-blocker | #18 and #19 open |
| 13 | Repository, records and status describe the game accurately | release-day review | kept current at checkpoints; release-day review not done |

### Performance and stability targets (provisional)

From DEFINITION_OF_DONE.md and docs/TIME.md. They are revised only in writing in STATUS.md with the
measurement that justifies the change.

- Map: local 256x256 with 24 z-levels; overworld 64x64 sites. (The valley is 128x128 today.)
- Population: 300 active people and 100 animals.
- Tick at 300 people: mean at most 2 ms and p99 at most 10 ms at 10x; at most 16 ms at 3x; at most
  50 ms at 1x.
- Planner: at most 32 plans a tick, amortised at most 50 microseconds each; A* at most 2,000
  expansions, hierarchical beyond 64 tiles.
- Client: 60 fps at 1920x1080, frame apply at most 4 ms.
- Memory: simulation RSS at most 2 GB after 600 game-days (24 real hours at 10x); chronicle at most
  500 MB per 100 game-years.
- Soak: 24 h on 4 seeds at 10x with no deadlocks, stalled under 1 %, no oscillation, no save
  corruption.
- Save: at most 150 MB, written in at most 1 s off the simulation thread, loaded in at most 3 s.

---

## 4. A tour of the game as a player sees it

This section is what a tester or new developer sees when they run the client, with the systems
behind each thing named so the code can be found.

### Starting a game: the wizard

A hands-on start with no saves opens the new-game wizard (`godot/scripts/setup.gd`, five pages):

1. Who watches. Choose the deity: Aun (keeps balance), Hask (tests the strong) or Mirre (is
   amused). Internally these are the old Engine modes Steady, Relentless and Whimsical.
2. The valley. A live map of the world (192x192 regions) and of the valley the chosen region makes,
   the depth of the valley's own past (0, 50, 150 or 300 years; default 150) and the age of the
   wider world (200, 500 or 1,000 years; default 500).
3. Who came through. Ten survivor cards, each with a lived past; reroll any one card; choose three
   to begin together.
4. What they hold. Shared convictions (drink, violence, fire, hunger, strangers, death, trust). A
   chosen conviction is seeded into every survivor as a belief whose source is their old village,
   so it can strengthen, split or die. "A group that arrives sure drink is ruin can still produce a
   drinker."
5. Begin. A summary, then `SimHost.begin(setup)`.

What happens on Begin is `World::from_setup` (`crates/hf-sim/src/setup.rs`):
1. Generate the world's region map from the seed.
2. Resolve the chosen region and derive the valley's character from it (wetness from rain, warmth
   from temperature, stoniness from height and rock).
3. Set the arrival epoch far enough after tick 0 to hold both histories.
4. Run the wider world's history (peoples, settlements, figures, realms, wars) and record it.
5. Build the overworld neighbours from the nearest real settlements.
6. Run the valley's own past (old hamlets, ruins, graves, roads, old tools).
7. Set the clock to the epoch plus seven hours, a spring morning.
8. Record the setup as an event, begin the arrival, place the White Hart and the portal stone, seed
   the chosen convictions, and compute the people's identity.

An opening world with 500 years of history builds in about 250 ms.

### The arrival

Ten survivors came through. Three begin at the landing site; the other seven are real people placed
somewhere in the valley, foraging and sheltering against real weather (`opening.rs`, D15). A rumour
of one of them can reach the community (in someone's head, with a teller), and someone may go and
search: they find a person, a grave, or an empty camp with signs. All seven are currently found
within 15 to 24 days on the tested seeds, which the M3 review flags as too quick (an M6 question).

Each survivor's past was simulated year by year from birth (`past.rs`): a home village, parents and
siblings, and formative events such as a parent who drank, a famine winter, a house fire, an
apprenticeship, a marriage, a child who died. Each formative event is a chronicle row dated before
the arrival. An event does not set a belief; it opens a question ("what is drink to me") that is
answered from temperament, whether someone else showed another way, and chance. The reference case
is the maintainer's: two sons of a father who drank, one who drinks and one who never does.

### Daily life

People have five needs (hunger, rest, warmth, social, comfort), ten traits, nine skills, a
genome, memories, relationships, beliefs with sources, techniques and thoughts. Each tick up to 24
of them decide what to do next (`systems/decide.rs`). They choose among eating something they know
of, foraging, sleeping, talking, wandering, warming up, fleeing a fire, fighting it with a technique
they know, tending the sick, going after a raider, welcoming strangers, studying an old record,
gathering at the hearth in the evening, or taking up a job.

Jobs come from the community planner (`systems/settlement.rs`), which runs hourly and only posts
work: a stockpile, hauling, felling, quarrying, a hearth, houses with beds, then floors, tables,
lamps and a cellar, fields, hunting parties, cooking, workshops and craft orders. It never makes
materials appear and never sends anyone to a crisis. Fire is fought by whoever sees it, using what
they know, or not at all.

Evenings: people who are not too hungry or tired gather at the hearth (18:00 to 21:00). Someone
tells a tale or how they came to hold a belief; listeners grow closer, beliefs spread, and the
habit can become a custom. On the C34 branch, those who hold with drink share ale or wine there.

Over time people pair, have children (a 14-locus genome mixes the parents' looks and temperament),
age (adult at 15, elder at 50) and die. A child takes on some of what the people who raised them
believe and know. Generations pass in about a month of real time at the default speed.

When stress passes a person's threshold (lower for the less calm), they break: a fight, a tantrum
that smashes furniture, withdrawal, or wandering off. Witnesses remember it.

### Building

Houses go up in visible stages (C10): marked, walls, roofed, and roofed then damaged. A roof is held
by walls; snow loads it according to the builder's skill; a roof falls when a wall goes, and is
mended. Underground, a dug space is held by a wall or a prop within a span that depends on the rock
(soft rock 2 tiles, granite and slate 3, basalt 4); collapses spread and sink the ground. Shoring is
a technique people learn by watching, not an order.

### Food, materials and crafts (M4)

Food spoils by kind and storage, and fields are sown by what the store holds (C31). Everything is
made of something: marble blocks, birch logs, garnets, wolf pelts, bronze bars (C32). Metals are
smelted, bronze is alloyed, spears and armour are forged and taken up when strangers threaten
(C33). On the branches: pots, glass, leather, ale and wine, meals that remember their ingredient,
rings given at a pairing (C34); quality marks, carvings of remembered events, fey moods (C35).
Section 5 explains each.

### Trouble from outside

Hungry places beyond the valley send bands of real people (`threat.rs`, C8). A band approaches,
camps, raids the store at night, and leaves for a reason the feed can name: it has enough, it has
lost too many, or it waited too long. The valley's people respond by what they believe: confront,
send an envoy, or welcome. Wolves go for someone alone in a hungry winter; a surprised bear attacks;
a beast that hurts someone gets a name, and people may go after it.

### The deity

The chosen deity (`deity.rs`, C9) never fires an event. It notices things in the real world state
(plenty, hardship, a killing, a long quiet, a hungry or a fat neighbouring settlement) and answers
by leaning the world's own mechanisms for a while (harvest, cold, dry storms, sickness, herds,
wolf hunger), in the valley or at one site. "A raid still comes from people who are hungry; the
deity only made them hungry." Every answer is recorded as a trace with its trigger, hidden from the
player's view, so the History can explain it. `arcs.rs` tells overlapping troubles as one story.

### The deep, the thin place and creatures

Below ground are three named rock strata, ore veins that follow the rock, gems in the deep basalt,
named caverns with fungus forests and pools, bats, a pale crawler that hunts in the dark, and a
burrower that cuts its own tunnels (C19). Darkness slows work, and people set lamps.

The portal stone wears thin 10 days after the arrival (C20, `thin.rs`). Shared dreams drawn from
real pasts, things from home with real makers, echoes at dusk of those of the seven who really
died, patches of wrong weather, and a hollow where nothing grows follow. The curious study it and
then choose: ward it with four stones, or leave it open, which can lead one of the living lost home.

Creatures (C15, `creatures.rs`): bears den in winter, ducks migrate, crows scavenge, the wolf pack
hunts together, the White Hart (one in most valleys), and feuds against beasts that hurt people.

### The wider world and the Legends browser

The world is a 192x192 region map with rivers, lakes, rain shadows and 13 biomes, 4 to 7 peoples
with their own languages and convictions, hundreds of settlements, tens of thousands of named
figures, realms with rulers and successions, and wars with battles and takings (C24 to C29). After
the arrival it goes on: once a game year the realm steps forward, and the valley hears of it only
when a caravan comes back with news.

Press L for the Legends browser: linked pages for the world, each people, settlement, figure, realm
and war, with a map through time on a year slider. On seed 1 a 1,000-year world has about 72,700
named figures, 426 wars, 3,015 battles and 642 settlements taken, generated in about 3.3 s.

### What the screen shows

- HUD (top): world name, date, view level, population, communities, speed, director state, deity,
  step time; a key help line; warning lines for bands and ongoing troubles; and a hover line for
  whatever is under the mouse ("Lying here: 12 oak logs", "A carpenter bench of oak", a house's
  state, a storied tool).
- Feed (bottom left): the last nine lines within 45 seconds. Minor lines are throttled to one per
  700 ms; colours grey (minor), pale yellow (notable), salmon (major).
- Inspect panel (right, click a person): name, age, activity, community and role, need bars, traits,
  skills, tool, arms, techniques, tales heard, family, customs, mood, relationships, beliefs, past,
  memories, what they know.
- History (H): eras, the record, and why people do what they do (customs with their origins).
- While you were away: opens when the window regains focus after more than 1,200 ticks.

Controls (from `godot/scripts/main.gd`):

| Key | Does |
|---|---|
| Space | pause; resume at 1x |
| 1, 2, 3 | speed 1x, 3x, 10x |
| , or PageDown / . or PageUp | view one level down / up |
| WASD or arrows | pan |
| mouse wheel | zoom (0.5 to 4) |
| right or middle drag | pan |
| left click | inspect a person |
| C | camera director on or off |
| F | follow the selected person |
| H | History |
| L | Legends (Backspace goes back) |
| M | open the colony map, follow a sworn resident or watch a caravan |
| V | show or hide remembered terrain outside current sight |
| F5 / F9 | save / load |
| Ctrl+F1 to F8 / F1 to F8 | set / go to a bookmark |
| O | settings |
| Esc | close panels, clear selection |

---

## 5. Where we are

### 5.1 How we got here

Phases 0 to 12 (2026-10-13 to 10-14, ROADMAP.md) built the base:
- 0: contract and repo;
- 1: three-crate workspace, deterministic core, chunked z-grid, save format, Godot bridge on Linux
  and Windows, palette;
- 2: navigation (3D A*, regions and portals);
- 3: living people and the first glance review;
- 4: work and settlement (planner, leases, designations, ledger);
- 5: memory and social;
- 6: saves and the viewer;
- 7: communities and economy;
- 8: generations and culture;
- 9: depth and discovery;
- 10: expeditions;
- 11: history and director maturity;
- 12: hardening, packaging, the first soaks.

The parity programme, Phases 13 to 17 (2026-10-15 to 10-16, PLAN-PARITY.md):
- 13: life on screen (layered sprites, dress, animation, wall tops, weather);
- 14: materials and workshops (quality tiers, recipes, wear);
- 15: bodies and health (wounds, bleeding, infection, poultices, illness);
- 16: minds under strain (thoughts, stress, breaks);
- 17: water, fire and cold;
- 17b: lived lives (pasts, beliefs with sources, techniques, the missing seven as people);
- 17c: a world with a past;
- 17d: the wizard.

The C-series checkpoints (from PLAN-SHIP.md, then PLAN-FINISH.md):

| Checkpoint | Delivered |
|---|---|
| C1 | render layers, light radii, blob shadows, unused terrain wired |
| C2 | audio foundation (OGG, buses, sound effects) |
| C3 | the arrival epoch (D18) and lived pasts |
| C4 | beliefs with sources, trauma aversion folded in |
| C5 | techniques replace crisis orders; personal projects |
| C6 | the missing seven as people; rumours as memories |
| C7 | music v1 (no tag exists for c7) |
| C8 | unprompted threats: bands, wolves, bears, named beasts |
| C9 | pacing: deities and arcs |
| C10 | construction as a visible process |
| C11 | item legends: tools with makers, wear, deeds, names, heirlooms |
| C12 | the valley's past (150 years of hamlets before the portal) |
| C13 | study: old records teach crafts |
| C14 | lore: tales that drift as they pass |
| C15 | creatures |
| C16 | reachable terrain: biomes and character per seed |
| C17, C18 | the wizard (simulation side, then screens); hearth evenings; people's identity |
| C19 | the deep: strata, caverns, collapses, cavern creatures |
| C20 | the thin place (PR #103); QA round 2 (PR #102) |
| C21 | M1 scale: disk chronicle, 500 people within budget (PR #109); QA round 3 (PR #110) |
| C22 | the art pipeline (PR #111) |
| C23 | people at 48 px with faces (PR #112) |
| C24 | the world's region map (PR #113) |
| C25 | peoples, languages, settlements (PR #114) |
| C26 | figures, realms and chronicle references (PR #115) |
| C27 | the valley inside its world (PR #116) |
| C28 | the Legends browser (PR #117) |
| C29 | wars and the world after the arrival (PR #118) |
| C30 | M3 review: opening soaks, wizard world map and age, gap table (PR #119) |
| C31 | food that spoils, sowing by the store (PR #120) |
| C32 | materials with properties (PR #121) |
| C33 | chains and arms (PR #122) |

Each checkpoint has a review folder under `docs/reviews/` with its measurements and frames.

### 5.2 M4 so far, on master

M4's design is `docs/design-industry.md`. Its six pieces are materials, production chains,
quality, engravings of recorded history, food that spoils, and trade with the world. Its gates (as
test names) are `ore_becomes_a_weapon_that_is_worn_and_used`, `prices_follow_scarcity`,
`ledger_holds_through_trade_and_rot`, `sowing_follows_the_store`, `an_engraving_shows_a_real_row`
and `a_mood_has_a_cause`.

#### C31: food that spoils, and sowing by what the store holds

Why: the C21 review found that a harvest nobody hauled piled up for years (84,756 sheaves on one
seed), food never spoiled, and fields were sown for the peak population.

What it does (`systems/spoil.rs`, hourly): each food kind keeps for its own number of days in the
open in mild weather.

| Food | Days in the open |
|---|---|
| raw meat | 3 |
| berries | 4 |
| meals | 5 |
| roots | 20 |
| sheaves | 30 |

Under a roof it lasts about three times as long, in a barrel, crate or chest about seven times, in
frost about three times, and rain shortens it by a third. `Item::rot` (saved) records how far gone
a stack is; merging stacks averages it. A stack that has gone off loses a quarter of itself at a
time, recorded in the ledger as rotted, and the feed says once a day at dusk what went off
(`EventKind::Rotted`).

The planner sows one plot when the store holds 40 portions of food per person, half the plots
with 20, all of them when low. These are portions, not days: 40 is about four days' eating (C31's
review first said "forty days"; corrected in C34).

Measured (300-day opening soaks, four seeds): unhauled stacks fell from 376, 568, 1,239 and 322 to
122, 101, 197 and 158. People came out about 10% fewer on average, mostly from different paths;
starvation stayed minor (1 to 6 per seed). Tests: `tests/spoilage.rs` (4).

#### C32: materials with properties, fed from every source

The material table (`world/material.rs`) gives every material:
- a density in kg/m³;
- a melting point where it melts;
- a value, where a day's food for one person is about 1;
- a palette colour;
- what working it turns it into (an ore's metal).

27 materials were appended after the 28 tile materials, so old ids keep their meaning: marble; tin,
silver and gold ores; iron, copper, tin, bronze, silver, gold; oak, birch, pine and fungus wood;
reed; deer, boar, goat hides and wolf, bear, fox, hare pelts and the white hart's hide; garnet,
sapphire, emerald, amethyst. `MaterialTable::refresh` runs on every load: definitions with the same
id and name take the code's properties, new ones are appended, and one the code names differently
is left as saved.

Every source gives its material:
- Mined blocks are of the rock they were cut from.
- Logs are of the tree. The species is `Plant::variant % 3` (oak, birch, pine), which is also what
  the client draws.
- Gems are of their pocket, fixed by position: garnets common, amethysts less so, emeralds and
  sapphires rare.
- Reed fibre is reed.
- Trophy hides are of their beast.

A delivery records the material on the order's requirement, so planks keep their logs' wood, bars
take their ore's metal, and a built table is of the timber brought for it. Stacks of different
materials never merge.

Geology: under limestone the middle stratum is marble in patches (its own noise). Tin veins lie in
the granite where the copper is, silver in the slate, and at most one short gold vein in the deep
basalt, each from a random stream of its own so every existing valley keeps the rest of its rock. A
new `ItemKind::Ore` carries tin, silver and gold ore.

Tiles per valley at the default size, seeds 1 to 6:

| Seed | Marble | Tin | Silver | Gold | Iron | Copper |
|---|---|---|---|---|---|---|
| 1 | 13,952 | 60 | 18 | 5 | 75 | 229 |
| 2 | 10,544 | 56 | 49 | 9 | 174 | 88 |
| 3 | 6,824 | 55 | 46 | 7 | 58 | 85 |
| 4 | 35,803 | 34 | 50 | 9 | 107 | 179 |
| 5 | 21,628 | 66 | 41 | 0 | 196 | 79 |
| 6 | 14,052 | 46 | 9 | 7 | 125 | 56 |

Shown: hovering a stack reads "Lying here: 12 oak logs, 8 marble blocks"; 26 material icons
(`items32/<kind>_<material>.png`) recoloured from the derived ones; terrain sheets for marble and
the three ores. Tests: `tests/materials.rs` (10).

#### C33: chains and arms

Recipes can name the material each input must be (`Recipe::wants`). Every place that matches an
item to an order honours it: the hauler's search (`work/planner.rs` `find_known_item`), the check
that a job has materials somewhere (`systems/decide.rs`), and the delivery (`systems/work.rs`
`DropAt`). What the output is made of is part of the recipe (`Makes::FromInput(fallback)` or
`Makes::Fixed(material)` for an alloy). New generic kinds `Bar`, `Weapon` and `Armour` take their
nature from their metal. Copper takes no fine edge (quality capped at 0.6), bronze a better one
(0.85).

The arms system (`systems/arms.rs`, hourly) works like this:
- A community is threatened when strangers are seen near it, its store is raided, or one of its
  people fights one (`Community::threatened`, the tick).
- The planner makes spears (about one for every third person) and armour (one for every sixth)
  only while a threat is under 40 days old.
- Within 10 days of a threat, those who would go after a raider anyway take up the best spear and
  armour lying within 45 tiles of home. "Would go after a raider" means grown (16 or older), not
  hurt, and leaning on violence above -0.2. Nobody hands arms out. `EventKind::TookUpArms`.
- In a fight a spear makes a blow more likely to land (from 65% to about 80% with a fine iron
  spear), makes it a cut, and makes it worse (by up to half). Armour turns up to about a third of
  blows and blunts the rest (by up to half). Both scale with the metal's hardness against iron's and
  with quality. Against wolves a spear makes someone stand their ground more often; in a hunt it keeps
  a cornered beast at its length.
- Every blow wears the piece: 6% a blow struck at a stranger, 2% a hunt, 8% a blow taken on armour.
  The piece's story counts blows and game and records deeds at 1, 5 and 10 ("drew a stranger's
  blood", "held off raiders ten times"). A broken piece is gone, with a feed line.
- More than 30 days after the last threat, arms are put away at home. The dead leave theirs where
  they fell.

Shown: inspect reads "armed with" and names the pieces with their stories; an armed person carries
the spear beside them (equipment index 7 in the people sheets); built things name their material
on hover ("A carpenter bench of oak"). Tests: `tests/arms.rs` (8), including the design gate and an
end-to-end test that spears are taken up and used in real raids on seeds 3, 7 and 11.

Measured: in 300 days arms are rare. Two of four valleys forged spears (3 and 1), the spears were
put away before the next raid, and none struck a blow in those runs. No valley reached tin, silver
or gold, because nobody went looking for those veins. Both gaps go to the M4 review; C34 closes the
prospecting one.

### 5.3 Built after master: C34 and C35 (branches, not yet merged)

These are committed and pushed to GitHub on their own branches. Neither has a PR yet. C34's full
CI was running when this was written; C35 has passed its own tests only.

#### C34: the other crafts (branch `c34-crafts`)

New item kinds: `Clay`, `Sand`, `Vessel`, `Leather`, `Ale`, `Wine`, `Jewel`. New materials 55 to
59: glass, and four food materials (meat, roots, grain, berries) so that a meal can remember what
went into it. New order kind `DesignationKind::DigEarth` (client code 19).

- Clay and sand: clay is dug from wet banks (wet soil, mud or clay ground) and sand from sand bars,
  three at a time; the ground stays. The planner digs clay while the store holds fewer than 6, and
  sand once there is a kiln and coal.
- The kiln: built at offset (9, 6) from the first house once clay has been dug (cost 4 stone blocks
  and 2 clay). It fires clay pots (2 clay and a log) and glass bottles (3 sand and coal). Food on a
  tile where pots stand keeps as it does in a barrel.
- Hides and leather: every hunt now leaves the beast's hide (deer, boar, goat, wolf, bear, fox,
  hare). The workbench tans a hide into leather of that beast and sews two leathers into clothes.
  Leather and fur keep the cold out better (warmth loss factor 0.3 instead of cloth's 0.5,
  `Agent::furred`). An untanned hide rots within 20 days.
- Drink: every survivor's past already held a view on drink ("drink ruins people" or "a drink eases
  a hard life"), but until C34 it changed nothing. Now:
  - A community brews only when the average of its people's drink beliefs is above -0.1, and only
    from a full store: ten days' eating a person, counted as the hours of hunger the food
    satisfies against about 18 a person a day (`EATING_HOURS_A_DAY`).
  - Ale is 4 sheaves for 3 ale; berry wine is 5 berries for 2 wine. Both are brewed at the hearth
    over two hours.
  - At the hearth in the evening, those who do not hold against drink (above -0.2) and are grown
    share it, fetched from anywhere within reach of home, once a day each.
  - Drinking gives a small good thought for a day and brings the drinkers a little closer.
  - The feed reads "Over ale at the hearth: A, B and C." (`EventKind::DrankTogether`). Those set
    against drink go without.
- Meals that remember: a meal's material is its ingredient, so the ground reads "meals of stew",
  "meals of mashed roots", "meals of porridge" or "meals of berry cake".
- Rings: a silver or gold bar and a gem make a ring at the anvil; the ring's story keeps the gem
  ("set with a garnet"). When two people pair, the one who thinks more of the other gives a ring
  from the store if one exists ("X gave Y a silver ring set with a garnet", `EventKind::GaveRing`),
  and both feel it for ten days.
- Prospecting: the planner now seeks an exposed tin vein once there are copper bars for bronze, and
  silver or gold once there is a gem to set.
- Icons: a third reference sheet (`tools/art/sources/reference/items_c.jpg`) gave the pot, bottle,
  leather roll, ale mug, jug, ring, stew, bread and more. The ring's interior kept the sheet's grey
  background and is cleared by `clear_holes` in `items_derived.py`. Carried 16 px icons are reduced
  from the 32 px ones.

The C34 soaks found three faults, all fixed on the branch (details in
`docs/reviews/crafts-2026-10-18`):
- Ale was brewed but never drunk: it had been hauled out of the hearth's reach. Drink now comes
  from anywhere within 45 tiles of home.
- Grain went to ale on a thin store: the guard counted portions, not days. It now counts hours of
  hunger the store satisfies.
- Nothing stands on air. On seed 4, 38 people froze over three winters. A meal lay on a timber
  wall that burned; the navigation regions recomputed only the wall's own level, so the tile above
  stayed "reachable" though no path led there. People chose that meal every forty ticks for days
  and froze beside the hearth. Now a changed chunk recomputes the chunk above it, things where the
  floor has gone fall, and every walking goal is checked for reachability where goals are chosen.
  This bug predates M4.

Final soaks (300 days, opening, seeds 1 to 4): all pass.

| Seed | People at day 300 (C33) | C34 | Kiln built | Pots | Shared drink at the hearth | Mean drink belief |
|---|---|---|---|---|---|---|
| 1 | 57 | 57 | yes | 21 | 0 | -0.55 |
| 2 | 47 | 56 | yes | 21 | 0 | +0.35 |
| 3 | 55 | 53 | yes | 20 | 4 times | 0.00 |
| 4 | 70 | 82 | yes | 24 | 0 | -0.02 |

With the store measured honestly, drink is rare. Seed 2 starves in every build (8 here, 10 in the
M3 review), so that is the valley, not a regression.
Tests: `tests/crafts.rs` (10).

#### C35: masterworks (branch `c35-masterworks`)

- Quality marks: the items bridge now sends a quality tier (eight ints per item). A fine piece
  (tier 4) shows a gold pip on the ground and a masterwork (tier 5) a four-pointed star.
- Carvings (`crates/hf-sim/src/engrave.rs`, order `DesignationKind::Engrave`, client code 20, three
  hours). Someone who builds well (construction skill 4 or more) cuts into a timber or block wall of
  a roofed house an event their people remember.
  - Only these kinds of event can be carved: a death, a birth, a pairing, an arrival, strangers
    seen, a fight, a raid, a fire, a flood, a discovery, a hearth tale, a ring given, arms taken up.
  - The event comes only from `World::culture_log`, the world's record of its last 200 major
    events. It must be one the viewer may be told of (`view::known::told`), not already carved, and
    about this community or remembered by someone in it.
  - Preference goes to events the carver remembers (+3), then the number of people who remember it
    (+1 each), then events about the community (+1). Ties go to the older event.
  - The carving keeps the event's id and the record's own words (`view::narrate::line`), so
    hovering the wall reads "Carved by Halvor in autumn of year 2: ...".
  - Carved walls are drawn with grooves. The planner posts one carving at a time, five days apart,
    three at most for each house. A carving goes with its wall.
- Fey moods (`systems/moods.rs`, daily): Dwarf Fortress's strange mood, kept to what is recorded.
  - Who can be taken: a grown artisan (crafting or construction skill 3 or more), with a chance of
    1 in 240 a day, and only for a cause in their own record:
    - the craft a master taught them before the portal;
    - a child, sibling or parent lost before it (read from their past through
      `opening::formatives`);
    - kin who died in the valley in the last two years (from `culture_log`).
  - The work: they claim a bench and the finest material in the store that they know of (marble before granite and
    the other stones, then oak, birch, pine and fungus wood; for a smith, gold, silver or bronze
    bars, then iron).
  - The piece: a figure of the one they lost, or a blade. It is made at quality 1.0, gets a name,
    and its story says why ("a figure of Ada, made in a fey mood, remembering Ada, their child").
  - The feed says "X is taken by a fey mood, remembering ..." (`EventKind::Possessed`, which keeps
    the cause's record id).
  - If the work is not done in ten days, the mood passes and leaves them low.
- New item kind `Figurine`, with icons in every stone and wood.
- Tests: `tests/masterworks.rs` (8), including `an_engraving_shows_a_real_row` (the carving's words
  equal the record's words for its event id) and `a_mood_has_a_cause` (the cause is a row of that
  person's own past).
- Faults the soaks found, fixed on the branch (review `docs/reviews/masterworks-2026-10-18`):
  - carvings were mostly quarrels, a hundred on one seed (now weighted, five days apart, three per
    house);
  - no fey work was ever finished (moods now take only known material, and only the mood ends
    its work);
  - whole bands froze in their winter camps (a camped band's fire now warms);
  - carvings stayed on bare ground after their walls went (now removed with the wall).
- Soaks: all pass; 1 to 6 fey pieces and 6 to 51 carvings per seed in 300 days.

#### C36: trade with the world (branch `c36-trade`)

- `crates/hf-sim/src/trade.rs`. Each overworld site (`expedition::Site`) now keeps `stores`
  (kind to amount), the material each is held in (`store_material`) and its last twelve
  `trades` with the valley. All three are saved (`serde(default)`).
- `produces`: what a site makes per person a day, by its people's way of living
  (`realm::peoples::Living`) and its kind:
  - farmers: grain, berries, cloth, ale;
  - herders: goat hides, meat, cloth, goat leather;
  - hunters: deer hides, meat, pine logs;
  - fishers: meat, reed fibre, clay pots;
  - a mine: iron and copper ore, coal, granite blocks, tin ore;
  - empty forest and lakeshore: logs and hides, or meat, at a flat rate;
  - a ruin: ten granite blocks to start with, nothing more.
- `daily` (06:00, before the day's travel): every site adds what it makes. A site new to trade
  starts with ten days of making. Each day food loses 4% and everything else 1%.
- Prices. `worth(kind, material)` is in days of food: a material's value where it has one,
  otherwise by kind (a log 1, cloth 2, a spear 18). `need(site, kind)` is what a site wants
  to hold: people times a per-household amount, halved for what it makes, doubled for a mine's
  food. Scarcity is `(need + 1) / (stock + 1)`, clamped to 0.3 to 3. `price_at` is worth times
  scarcity at the site. `price_home` is the same for the valley, from what lies within 45 tiles
  of home.
- `quote` and `barter`:
  - A site takes a kind only up to half again its need.
  - It takes only as many as it can pay for from its surplus (what it holds beyond half its
    need).
  - It pays first in the goods with the best ratio of the valley's price to its own cost, up to
    ten per member of the party.
  - `barter` applies the quote to the site's stores and records the trade. Every unit that
    leaves one ledger enters the other.
- `best_errand`: the valley carries its surplus (anything above `16 + 2 x people` within 18
  tiles of home, at most 12) only to a peopled site a party of theirs has visited. The choice
  is the quote with the largest gain to the valley (what comes back, at the valley's prices,
  less what goes out), discounted by the length of the road, less 20 for each of the
  community's people that road took in the last 120 days (named lost in a caravan's log, or a
  whole caravan never heard from). With no gain, nobody goes, and the community may send scouts
  instead.
- Caravans (`systems/expedition.rs`):
  - goods come home in the site's materials (`Caravan.materials`, `create_item_of`);
  - unsold goods come back;
  - a party with nothing to carry does not form;
  - a party that arrives with nothing (robbed, or eaten on the road) says so;
  - on short rations a party eats its meals before its goods.
- Legends: the world map of the present year draws a pale gold line from the valley to every
  site it has traded with. A known site's page lists "They hold for trade: ..." and the last three trades with
  their dates.
- Tests: `tests/trade.rs` (7), including `prices_follow_scarcity` and
  `barter_moves_goods_and_makes_none` (the ledger gate: stores before plus goods taken, less
  goods paid, equals stores after; the soak's ledger-drift check covers the valley side).
- Faults the soaks found, fixed on the branch (review `docs/reviews/trade-2026-10-18`):
  - a farming village bought the valley's grain every week (sites now buy only what they lack);
  - a third of trading parties walked out with nothing, because the errand counted grain 40
    tiles out and parties load from 18;
  - parties ate the grain they were carrying before their rations;
  - one valley lost eight people on the road to a mine and kept trading there.

### 5.3b Milestone MP on stacked branches: C39 to C44

C37 (the M4 review) and C38 (personal projects, P0; review `docs/reviews/projects-2026-10-18`) are
in review. C39 to C44 are committed and pushed on branches stacked on them, each merged forward
into the next (`c39-bodies`, `c40-rig`, `c41-species`, `c42-senses`, `c43-world-draw`,
`c44-generated`). None has a PR. Each has a review folder with its tests, faults and soaks. The
design is `docs/design-peoples.md`.

#### C39: body plans (P1, branch `c39-bodies`)

- Nine plans as data in `data/peoples/plans` (amphibious, biped, digitigrade_tail, four_armed,
  huge_biped, serpentine, small_biped, taur, winged), compiled in and checked by `plan::parse`
  (`crates/hf-sim/src/plan.rs`): a head and a torso, at least two hands, something to move on,
  known part ids each once, every part hurtable, a cell of 32, 48, 64 or 96 px, a footprint of 1
  or 2 tiles, an id equal to the file name.
- `body::Part` is `plan::PartId`, an index into a fixed, append-only list of fourteen parts; the six
  human parts keep ids 0 to 5. A part saves as one byte; pre-C39 saves' names ("LeftLeg") load.
  `Plan::hands` (each pair past the first counts half), `Plan::random_part` (by wound weight,
  filtered by role or extremity). `Body::parts` holds `PartState` (scarred, lost, replaced), empty
  until M5.
- The genome's body block (`genetics.rs`, `BODY` = 8 loci), drawn from a hash of the other loci so
  no other draw moved; inherited like the rest; old genomes read 128. Height 150 to 190 cm, mass 51
  to 79 kg. `hunger_scale` = (mass / 65)^0.75 in `systems/needs.rs`; `heat_loss` = (mass /
  65)^-0.33 in the cold step of `systems/hazards.rs`; `carry` defined but not read.
- Wound callers name parts by id but still pass fixed human parts. Since 1a5c401 (on
  `c44-generated`) `systems::health::wound` maps a part the body lacks to one with the same role
  by `random_part`.
- `tools/docs/readme_tables.py` reads part labels from `plan.rs` (36c1b32).
- Tests: `tests/bodies.rs` (7), including `human_plan_matches_old_parts`,
  `wounds_land_on_plan_parts` and `size_scales_hunger_and_cold`.
- Soaks (build 36c1b32): all pass; 68, 55, 61 and 72 people at day 300. The branch base was C38's
  first commit, so C38's soak fixes are not in these runs.

#### C40: the rig (P2, branch `c40-rig`)

- `tools/art/people.py` draws per plan (`PLANS`: biped, digitigrade_tail). The digitigrade plan
  bends the knee back, keeps the heel up and adds a tail that sways with the walk. Sheets for a plan
  other than the biped carry `_<plan>` before the build and age.
- Every sheet has 34 columns: the 28 before, then `strike1` to `strike3`, `hit`, `fall`, `lying`
  (`swaps.json` `layout.columns`). People sheets: 199 to 367 PNGs.
- `Agent::fought_at` (saved, `serde(default)`) is set for both sides of a fight with strangers
  (`threat.rs`) and for a hunter's killing blow (`systems/work.rs`). The bridge sends pose 6 for
  60 ticks after it and pose 7 for 40 ticks after a wound; `entity_layer.gd` draws the strike
  frames (column 28 on) and `hit` (31). `fall` and `lying` are not sent.
- Tests: `tests/look.rs` `people_sheets_carry_the_fight_frames_and_every_plan` (sheet sizes and
  layers). No pixel test holds the biped's old columns.
- Soaks (build 11ed957): all pass; people and chronicle rows identical to C39 on every seed.

#### C41: species in the valley (P3, branch `c41-species`)

- `crates/hf-sim/src/species.rs`: six species files in `data/peoples/species` (human, hollin,
  grisk, screeling, skarri, tharn), compiled in, append-only; the index is `SpeciesId` (0 human).
  `validate` checks plan, ranges, maturity before the shortest life, comfort order, known words,
  senses 0 to 3, temperament topics, skin names. `library_hash` (16 hex characters) is computed
  but not recorded.
- `Species::Human` is `Species::Person` (serde alias "Human"); `Agent::people` names the species.
  Mass, height and lifespan come from the genome within the species' ranges; heat loss also by coat
  (fur 0.6, feathers 0.7, scales 1.2).
- Realm peoples draw a surface species (`realm/peoples.rs` `species_of`; common 6, uncommon 3,
  rarer 1, doubled when its ways suit the land) and add its leanings to their convictions. Bands
  (`threat.rs`) and newcomers (`systems/society.rs`) from a settlement are of its species, named in
  its language (`species::give`). A child is of its mother's species.
- Art: five plans (biped, small_biped, digitigrade_tail, winged, four_armed), a body sheet per
  species with its own head (`draw_features`), 14 skin tones after the six human ones
  (`SKIN_NAMES`, 20). People sheets: 901 PNGs. The bridge's 23rd agent int is the species;
  `SimHost::species_art` gives each species' body set, plan and hair.
- Inspect names the species and adds `SpeciesDef::describe` with height and mass; the feed names
  strangers by kind; Legends has `species:<id>` pages linked from peoples and the world page.
- Tests: `tests/species.rs` (7 on the branch), `tests/look.rs` `every_species_skin_is_drawn`.
- Faults: life stages were human for every kind (fixed in f30eb59 and b09101b on
  `c43-world-draw`); the client's `ANIMAL_NAME[0]` renamed "person" (95a6c58); `view/look.rs`
  folded every skin into the six human tones and children of another kind took human tones
  (bf7a844, which also raised the humans' comfort to 2 to 30 °C).
- Soaks (build a9688a8): all pass; 67, 58, 57 and 72 people at day 300.

#### C42: senses and cycles (P4, branch `c42-senses`)

- `systems/perceive.rs` per species. Sight radius = 12 x eyes (0, 0.5, 1, 1.15) x max(light,
  dark-sight floor 0.25, 0.5, 0.8, 1), with light 1 by day, 0.4 at night above ground, the lamps'
  below (the commit message says a fifth; the code says 0.4). Hearing 0, 6, 10, 14 tiles: anyone
  awake on the level, through walls. Smell 0, 3, 6, 9 tiles: food on the ground and creatures. A
  human sees 12 tiles by day, 5 at night, 3 unlit below; a Grisk 12 everywhere.
- `cycle` in the species file (Grisk nocturnal). `Calendar::rest_time(tick, nocturnal)`: 08:00 to
  18:00 for the nocturnal. Read by `systems/decide.rs`, `systems/act.rs` and camped bands.
- `SpeciesDef::diet_factor` multiplies a food's worth when eaten: carnivores raw meat 1.4, meals
  1.0, else 0.5; herbivores 0.3, 0.9, 1.1; fungivores roots 1.0, meals 0.8, else 0.6.
- Comfort ranges in `systems/hazards.rs`: cold below the low end (humans 2 °C), frostbite 7 below;
  above the high end, outside and awake, comfort and (8 over) rest fall and the new
  `ThoughtKind::Hot` (-0.15 for 0.3 days) is added. Humans are hot above 30 °C since bf7a844.
- The light list below ground is saved (822fde5), so a game resumed mid-hour sees what the
  uninterrupted one saw; `tests/resume.rs` (2).
- Tests: `tests/senses.rs` (5), including `the_dark_shortens_sight_unless_the_eyes_are_made_for_it`
  and `hearing_and_smell_reach_through_walls`.
- Soaks (build 7fd640c, before 822fde5 and bf7a844): all pass; 54, 66, 55 and 71 people at day
  300.

#### C43: the world draw (P5, branch `c43-world-draw`)

- `species::PeopleSetup` in `WorldSetup::peoples`: `exclude`, `exclude_plans`, `count` (default 4
  to 7, at most 16), `rarity` overrides. `check` refuses bad counts, unknown keys and a world with
  no surface species; `summary` goes into `EventKind::SetupChosen` ("In this world: ...").
- `realm::peoples::peoples_with` draws from the species left in, weighted by the setup's rarity,
  no kind twice until every open kind is drawn. `People::rarity` is recorded.
- `realm/sites.rs`: a rare people founds nothing; one settlement in five (by hash) houses a rare
  people's households (`RSite::minority`), and a quarter of newcomers from it are of that people.
  A legendary people's settlement capacity falls 15% per century, to a tenth.
- `Realm::unique_name` runs two words together after 40 tries instead of reusing a name.
- Life stages by kind (f30eb59, b09101b): `Agent::stage_years` (humans 15 and 50; others
  maturity and the short end of the lifespan); `of_age` (16 for humans, maturity otherwise) for
  arms, drink, moods and projects; `fertile_years` (humans 16 to 45; others maturity to three
  quarters of the way to old age); `ailing_years` (the kind's old age and a tenth past);
  `species::give` sets a stranger's age within their kind's grown years.
- The wizard's third page, "The peoples" (`godot/scripts/setup.gd`): counts, a tick box and rarity
  per species, and a live preview from `SimHost::peoples_preview` (map and draw only).
  `--wizard-choices` takes `exclude=<key>` and `rarity=<key>:<rarity>`. Frame:
  `docs/reviews/peoples-draw-2026-10-18/wizard-peoples-page.png`.
- Tests: `tests/world_draw.rs` (6), including `same_peoples_same_world`,
  `a_rare_people_has_no_land_and_lives_among_others` and `a_legendary_peoples_land_fails_it`.
- Soaks (build 59ea3d5, before b09101b): seeds 2 to 4 pass (57, 55, 67 people). Seed 1 stopped at
  day 275 without a summary after items on the map rose past 5,000 and the median tick past 1.3 ms;
  not diagnosed (review folder).

#### C44: generated species (P6, branch `c44-generated`)

- `species::generate(seed, count)`: off 0, some 12 (default), many 30 (`PeopleSetup::generated`).
  Ids from 1000 (`GENERATED`). Plans weighted biped 30, small_biped 18, digitigrade_tail 22, winged
  14, four_armed 16, on existing body sheets; every axis drawn; redrawn until `axes_from_human` >=
  4 (of diet, climate, senses, lifespan, size, social, ways, cycle; up to 60 attempts); first skin
  tone and body sheet unique in the world; `validate`d; named by `Language::of(seed, 5000 + k)`;
  described from its values.
- `World::generated`, `World::kind`, `World::kinds`; saved whole in `Snapshot::generated`;
  `Agent::kind` (not saved) restored on load. Legends, inspect, the feed and
  `SimHost::species_art` (now a dictionary by id) read the world's kinds. The wizard adds "kinds of
  its own: off, some, many".
- Fixes from the review of C38 to C44 (1a5c401): wounds land on parts the body has; the client
  drops the last world's generated kinds on a load or new game; `check` refuses a `generated`
  value other than off, some or many, and the summary records it.
- Tests: `tests/generated.rs` (6): `generated_species_differ_four_ways_and_pass_every_check`,
  `generated_species_look_unlike_each_other`, `worlds_draw_generated_peoples_and_off_means_none`,
  `generated_species_survive_a_save`, `a_wound_lands_on_a_part_the_body_has`,
  `a_world_with_generated_kinds_resumes_the_same`.
- Not done from the roadmap: silhouettes compared (only sheet and colour), the huge, taur,
  amphibious and serpentine plans (left for P10), 12 anchors.
- Soaks (build a0e13fb, before 1a5c401): all pass; 70, 52, 57 and 62 people at day 300.

### 5.4 Branches and worktrees today

On GitHub (private repository):
- `master` at 8b4f93f;
- the merged checkpoint branches (kept, not deleted);
- `c34-crafts` and `c35-masterworks` (no PRs yet);
- this handbook on `handbook`.

Tags: `c1` to `c33` (no `c7`) and `phase-17b`.

On the build server, each checkpoint has a git worktree at `<src>/ostrog-<branch>` (C22 to
C35 plus `ostrog-qa3*` and `ostrog-bisect`). The main checkout is
`<src>/ostrog` on master. The old worktrees should eventually be moved to
`<src>/ostrog_to_delete/` (nothing is deleted outright) and `git worktree prune`
run.

Open GitHub issues:

| # | What |
|---|---|
| 16 | Phase 12 release gate (24 h soaks, perf per speed, onboarding, exports, docs, asset records, evidenced matrix) |
| 18 | Release blocker: the Windows package has never been launched on Windows |
| 19 | Release blocker: the viewer has never been exercised by hand on a real monitor |
| 46 | Epic: make Ostrog feel alive as a watchable second-monitor drama machine |
| 54 | Backlog: colony identity, rituals and ideology should shape how the world feels |
| 84 | Suggestion: repeated journeys leave visible desire paths (planned for M6) |

---

## 6. Architecture

### 6.1 Overview

Ostrog is a Rust workspace plus a Godot client (D3, D12).

| Part | Path | Role |
|---|---|---|
| hf-sim | `crates/hf-sim` | the whole simulation: deterministic, headless, engine-agnostic, about 30,000 lines |
| hf-cli | `crates/hf-cli` | the `ostrog` command-line tool: runs, soaks, profiles, saves, the world map, legends |
| hf-godot | `crates/hf-godot` | a GDExtension (`cdylib`) exposing the class `SimHost` to Godot, about 75 functions |
| client | `godot/` | Godot 4.6.3 project: rendering, sound, UI, the wizard |
| tools | `tools/` | CI, checkpoint, screenshots, packaging, art and audio generators, README tables |

Toolchain: Rust 1.95.0 (pinned in `rust-toolchain.toml`), edition 2021, Godot 4.6.3 through the
`tools/godot` wrapper, cargo-xwin for the Windows library, Python 3.11 with Pillow and numpy for
the art tools, ffmpeg for audio. Workspace dependencies: rand 0.8, rand_chacha 0.3, serde,
serde_json, bincode, rmp-serde 1.3, anyhow, rusqlite 0.31 (bundled), sha2, hex, clap 4, godot
0.5.5 (api-4-6).

The client runs the simulation in process: `SimHost` owns a `Session` (the `World` plus its
chronicle), steps it from `_process` at 20 ticks a second times the speed, and pulls flat integer
arrays describing the world every 0.05 s.

### 6.2 The simulation's module map

Top level of `crates/hf-sim/src`:

| Module | Owns |
|---|---|
| `lib.rs` | `Session { world, chronicle, written }`: steps the world and writes its events to the chronicle |
| `world_state.rs` | `World`, `WorldMeta`, id allocators, item creation and use, `step` |
| `agents.rs` | `Agent`, `Species` (14), `Appearance`, life stages (adult 15, elder 50) |
| `ids.rs` | id types and `TilePos {x, y, z}` (distance: Chebyshev in the plane plus 4 per level) |
| `time.rs` | tick constants and the `Calendar` |
| `rng.rs`, `digest.rs` | determinism (section 6.5) |
| `events.rs` | `Severity`, `EventKind` (114 variants on master), `Event`, `EventSink` |
| `items.rs` | `ItemKind`, `ItemPlace`, `Item`, food and spoil tables, quality tiers, stack names |
| `structures.rs` | `StructureKind` (25), `Structure`, the material hover note |
| `society.rs` | `Community`, `Role`, `WelcomeNorm` |
| `economy.rs` | the item `Ledger` (created and destroyed, by cause) |
| `body.rs` | wounds, bleeding, illness, ailments |
| `genetics.rs` | the genome (looks and temperament) |
| `names.rs` | name generation |
| `plants.rs` | plants and their kinds |
| `engine.rs`, `deity.rs` | the deity (formerly the Engine): notices and leans, fires nothing |
| `opening.rs` | the arrival, the survivor pool, rumours, searches, reunions |
| `setup.rs` | `WorldSetup` and `World::from_setup` |
| `past.rs` | lived pasts and the belief topics |
| `deep.rs`, `lore.rs` | the valley's own past and the tales told of it |
| `legend.rs` | things with a life: makers, owners, wear, deeds, names, heirlooms |
| `culture.rs` | customs (data) |
| `expedition.rs` | the overworld, sites and caravans |
| `threat.rs` | bands of strangers, raids, pursuits and fights |
| `creatures.rs`, `underground.rs`, `thin.rs`, `building.rs`, `arcs.rs` | as section 4 |
| `scenario.rs` | small worlds for tests and the staged demo scene (art review only, D13) |
| `engrave.rs` | carvings of recorded events (C35 branch) |

Subfolders:
- `systems/`: one system per file (section 6.3).
- `mind/`: needs, traits, skills, perception, knowledge, memory, activity, thoughts, beliefs,
  techniques.
- `nav/`: bounded 3D A*, 32x32 region graph with portals for an O(1) reachability check, one
  definition of which tiles connect.
- `work/`: designations, plans and steps, the planner, recipes, reservations.
- `realm/`: the wider world (map, embark, peoples, language, sites, figures, war, chronicle, live).
- `save/`: encode, decode, write, read, verify, ring helpers, snapshot, manifest, migrations.
- `chronicle/`: the SQLite chronicle.
- `view/`: read models only (narrate, known, director, history, legends, look, summary).
- `world/`: tiles, chunks, the grid, materials, and `gen/` (heightmap, noise, strata, hydrology,
  caves, caverns, ores, biome, ramps, flora).

### 6.3 The tick pipeline

`World::step` adds one to the tick and runs the stages of `systems/mod.rs` `PIPELINE` in this fixed
order (D7). A stage that only runs at certain times guards itself (for example `tick % 1200 == 0`
for hourly).

| # | Stage | Purpose and cadence |
|---|---|---|
| 1 | deity | what the deity notices; daily at 04:00 |
| 2 | weather | temperature and sky from seed and tick; every tick |
| 3 | plants | growth; each plant once per 60 ticks |
| 4 | hazards | fire every 40 ticks; flood and rain hourly |
| 5 | building | snow on roofs, roofs that lose a wall; hourly |
| 6 | nav | rebuild navigation regions for changed chunks |
| 7 | settlement | the community planner; hourly |
| 8 | society | leavers, newcomers, roles, cohesion, splits, trade |
| 9 | expedition | caravans; hourly and daily |
| 10 | threats | hungry places and bands; daily |
| 11 | work_sweep | expire leases; every 60 ticks |
| 12 | perceive | perception into knowledge; each person once a game-minute |
| 13 | needs | needs decay; each person once a game-minute |
| 14 | decide | up to 24 decisions a tick, round robin |
| 15 | act | carry out activities, run work plans |
| 16 | animals | wildlife; striped |
| 17 | movement | follow paths; every tick |
| 18 | searches | resolve searches for the missing |
| 19 | reunions | the lost and the party meeting by sight |
| 20 | life | pairing, births, aging, death; daily, striped |
| 21 | health | wounds, infection, illness; hourly and daily |
| 22 | strain | thoughts, stress, breaks; hourly |
| 23 | culture | hearth evenings (18:00 to 21:00); customs daily |
| 24 | legend | the dead leave their tools: heirlooms to kin |
| 25 | arms | spears and armour taken up and laid by; hourly (C33) |
| 26 | moods | fey moods; daily (C35 branch) |
| 27 | creatures | dens, migrations, carrion, the Hart |
| 28 | underground | light below, the dark, the burrower |
| 29 | thin | the portal stone |
| 30 | lore | ruins wondered at, tales |
| 31 | spoil | food goes off; hourly, daily feed line at 18:37 |
| 32 | realm | the world goes on; once a year after the arrival |
| 33 | arcs | troubles told as one story; hourly |
| 34 | flush | pending events become `last_events` |
| 35 to 38 | culture_observe, strain_observe, beliefs_observe, learn_observe | read this tick's events |

Adding a system means adding one line here, in the position the design calls for, and one line in
`systems/mod.rs`'s module list. `run_pipeline_timed` is the same list timed, used by
`ostrog profile`.

### 6.4 The data model

`World` (`world_state.rs`), grouped:
- Meta and grid: `meta: WorldMeta` (seed, tick, generation parameters, founding site, name, autoplan,
  life speed, deity, epoch, setup) and `grid: WorldGrid`.
- Entities: `plants`, `agents`, `structures`, `items`, `designations`, `reservations`,
  `communities`, `customs`, `bands`, with id counters that only grow.
- The arrival: `survivor_pool`, `rumours`, `searches`.
- Terrain state: `dug`, `fires`, `flooded`, `scorched`, `roofs`, rain and dry hours, and on the C35
  branch `engravings`.
- The wider world: `overworld`, `realm`, `realm_map` (never saved; regenerated from the seed),
  `history` (the valley's past), `creatures`, `thin`, `arc`, deity answers and watch.
- Bookkeeping: `decide_cursor`, `culture_log` (last 200 major events), `ledger`, `rotted_today`,
  `events`.
- Not saved, rebuilt: `last_events`, `weather`, `nav`, `astar`, `lights`.

`Agent` (`agents.rs`):
- identity: id, name, species, born, appearance, alive, death;
- motion: position, facing, path;
- mind: needs, traits, skills, knowledge, activity, memories, relationships, mood, thoughts, stress,
  beliefs, techniques, tales, past (chronicle ids of their formative events), home village;
- kin: community, role, genome, mother, father, partner, children, band;
- body: wounds, illness, starvation clock, clothes wear, `furred` (C34);
- gear: tool (quality, wear, the item), weapon and armour (C33), caravan;
- `fey` (C35).

Almost every field after the original ones has `#[serde(default)]`, so old saves load.

Items: `Item { id, kind, count, place, quality, material, story, rot }`.
- `ItemPlace` is `Ground(tile)`, `Carried(agent)` or `Held(agent)`.
- Ground stacks of the same kind and material on one tile merge up to the kind's maximum, unless
  they have a story.
- A storied item is always its own item, never part of a stack. Storied kinds are tools, saws,
  hammers, sledgehammers, chisels, needles, buckets, clothes, records, weapons and armour, plus
  rings and figures on the branches.

Designations (`work/designation.rs`):
`Designation { id, kind, pos, requirements, work_left, work_total, max_workers, priority, created,
failures, done, community, owner }`.
- `owner` makes it someone's own undertaking, which only they take: a fire they chose to fight, a
  fey work.
- `Requirement { kind, needed, delivered, material, want }`: `material` records what was delivered;
  `want` is the exact material required.

Communities: home, members, house sites and stages, fields, quarry, welcome norm, cohesion,
discoveries, explored cells, identity, the hearth evening, and `threatened`.

Tiles: `Tile { material, shape, flags }`, packed in 32 bits as material | shape << 16 | flags << 24.
- Shapes: open, floor, wall, ramp up, ramp top, stairs up, down and both, ladder.
- Flags: water, deep, sky, door, designated, snow, fire.
- Chunks are 32x32 and carry a version for redraws.
- The default valley is 128x128 with z from -16 to 8; tests use 64x64 with z from -8 to 6.

### 6.5 Determinism

- `rng::stream(seed, &[purpose, ids...])` gives a ChaCha8 generator seeded by a hash of the seed and
  the parts, so every random draw is keyed by what it is for (a purpose constant), whom it is for
  and when. Decisions draw with `stream(seed, &[purpose::AGENT_DECISION, agent, tick])`. Draws are
  independent, so adding a draw in one place does not shift another.
- State lives in `BTreeMap`, `BTreeSet` or ordered `Vec`s. `clippy.toml` bans `HashMap` and
  `HashSet`.
- No wall clock in the simulation (the only use is the timed profiling runner) and no network.
- `digest::digest(&World)` is SHA-256 of the saved snapshot, so any new saved field is covered
  automatically. `ostrog run --seed 7 --ticks 200000 --agents 12` run twice must print the same
  digest. `tests/foundations.rs` gates this.
- A digest change on purpose needs an entry in `docs/history/ITERATION_LOG.md`.

How to add something random without breaking old worlds: give it its own stream (a new purpose or
an extra part), as the tin, silver and gold veins do in C32, so existing draws do not move.

### 6.6 How people decide, plan and work

Deciding (`systems/decide.rs`): up to 24 people a tick, in turn, who are idle or have a critical need
during a long activity. Each candidate goal gets a score.
- Eating, for example, scores the square of hunger, raised at night, times how sure they are of the
  food, a distance discount, its nutrition, and what their memories and culture say about it.
- Beliefs move a score by at most about 40% (`Beliefs::weight`, 0.6 to 1.4). They weight choices;
  they never decide them.
- Work is considered only for adults not on a rest day, not pressed by a need and not at night.
- Each open order scores priority times role fit times skill fit times traits times distance times
  a fire belief, less a little for each past failure.

Planning (`work/planner.rs`): `plan_for` turns an order into steps (go to, pick up, drop at, work)
using only items the person knows of and that nobody holds a lease on.

Leases (`work/reservation.rs`): all of a plan's keys (items, the order, a station, a tile, a plant)
are acquired together or not at all, so nobody holds and waits; leases expire.

Executing (`systems/work.rs`): `run` walks the steps; a finished order's world change happens in
`apply_effect`, one arm per order kind. A failed plan puts carried things down, counts a failure,
releases leases and leaves a mild bad memory.

The settlement planner (`systems/settlement.rs`, hourly, per community) is the only thing that posts
community orders, in this order:
1. a stockpile, expanded when nearly full;
2. hauling loose items;
3. felling when logs run short;
4. quarrying;
5. a hearth;
6. houses with beds, then interiors, then a cellar;
7. a field when food is low, sown by the store;
8. hunting parties when food runs short;
9. cooking;
10. workshops (carpenter bench, loom, smelter, anvil, and on the branch a kiln) and craft orders;
11. reeds for the loom;
12. ore prospecting;
13. on the branches: clay and sand pits, carvings.

Needs: hunger empties in about 6 game hours of activity, rest in 16 hours awake, social in a day,
comfort in a day and a half; sleep restores rest in 8 hours.

Knowledge (`mind/knowledge.rs`): people know items, food plants, structures, other people and water
by what they have seen or been told, with a confidence. Memory is bounded: items 128, people 64,
places 192, at most 384 in all; when full, the oldest memory of a kind over its budget is dropped.
Community hearths are shared knowledge.

### 6.7 Events, the chronicle and what the player may be told

Every change worth knowing becomes an `Event { id, tick, at, severity, kind }`.
- Severity is trace (never stored or shown), minor (the feed, throttled), notable or major (also the
  History panel and the camera director).
- `EventKind` has 114 variants on master; the branches add DrankTogether, GaveRing (C34), and
  Engraved and Possessed (C35).
- `participants()` lists the people involved.

The chronicle (`chronicle/mod.rs`) is SQLite, schema version 1:
- `real_history` stores every non-trace event with its kind and fields as JSON. Triggers make it
  immutable: no update, no delete.
- `entity_ref` links rows to the people, sites, figures, polities and wars they are about.
- `taught_history` records what a community was taught (customs, tales), joined to real rows.
- `era_summary` holds per-year counts.

The game runs on a temporary file-backed chronicle; a save copies it.

Text (`view/narrate.rs`): `line(world, event)` makes a sentence only from the event's fields and the
names it refers to. There is no embellishment.

What the player may be told (`view/known.rs`): nothing about the unfound missing, and nothing of the
wider world after the arrival until news comes by caravan. Every read model must ask `told` or use
`rows_told_sql`.

### 6.8 Saves

- A save is a directory `NAME.hfsave` holding `world.bin` (named-field MessagePack of the
  `Snapshot`), `chronicle.sqlite`, and `manifest.json` (magic `OSTROG-SAVE`, format version,
  game version, seed, tick, SHA-256 of both files).
- Writes are atomic: capture on the simulation thread, write a temp directory on a worker, rename
  over the slot. `verify` checks the manifest and both hashes.
- `FORMAT_VERSION` is 2 (D14). Format 1 is refused with a message.
- The rule (top of `save/snapshot.rs`): a new field with `#[serde(default)]` is backward compatible.
  Removing, renaming or changing the meaning of a field needs a format bump, a migration step in
  `save/migrate.rs` and a fixture save. The fixture is `crates/hf-sim/tests/fixtures/saves/v2.hfsave`.
- On restore: the material table is refreshed, old houses get roofs, caches are rebuilt, and old
  opening data is upgraded.
- The client keeps a three-slot autosave ring (`user://saves/auto.{0,1,2}.hfsave`). On start it loads
  the newest valid slot, and falls back to an older one if the newest is corrupt.
- Measured: a 600-day save is 103 to 118 MB, verifies in 0.08 s and loads in 0.15 s; the save pause
  is about 30 ms.

### 6.9 The bridge and the client

`SimHost` (`crates/hf-godot/src/lib.rs`) is a Godot `Node`. Its functions fall into groups.

Lifecycle:
- `generate`, `opening`, `begin`, `stage_demo`;
- `step`, `save_to`, `load_from`;
- `newest_valid`, `next_ring_slot`.

Wizard:
- `survivor_cards`, `deities`, `history_choices`, `realm_year_choices`, `ideology_topics`;
- `valley_preview`, `world_preview`, `region_info`.

Feeds and text:
- `drain_feed` (each line is tick, x, y, z, severity, text);
- `drain_sounds`, `director_suggest`, `summary_since`, `history`, `inspect`, `site_note`;
- `legends_page` (BBCode with links), `legends_map`.

World arrays, flat integers with these layouts:

| Function | Ints per entry | Layout |
|---|---|---|
| `agents()` | 22 | id, x, y, z, species, facing, body, skin, hair, hair colour, clothing, clothing colour, from x, from y, progress x1000, pose, child, injured, elder, equipment, work sound, band |
| `plants()` | 6 | x, y, z, kind, growth x100, variant |
| `structures()` | 8 | id, x, y, z, kind, material, active, progress x100 |
| `items()` | 7 on master, 8 on C35 | id, x, y, z, kind, count, material, (quality tier) |
| `designations()` | 8 | id, x, y, z, kind code, progress x100, materials complete, material |
| `roofs(z)` | 5 | x, y, material, quality x100, snow hours |
| `bands()` | 9 | id, camp x, y, z, state, sighted, welcomed, members here, taken |
| `engravings()` (C35) | 4 | x, y, z, quality tier |
| `level_tiles(z)` | 1 per tile | the packed tile |

Scalars: tick, hour, sky, season, temperature, daylight, tension (read by the music), population,
calendar.

Designation kind codes: 0 wall, 1 door, 2 floor, 3 structure (+kind x100), 4 chop, 5 mine, 6 sow,
7 harvest, 8 haul, 9 cook, 10 stairs, 11 hunt, 12 craft, 13 tend, 14 douse, 15 beat out, 16 roof,
17 clear roof, 18 study, 19 dig clay or sand (C34), 20 carve a wall (C35).

Client scripts (`godot/scripts/`):
- `main.gd` (the root: flags, start path, stepping, HUD, panels, input, saves);
- `world_view.gd` (ground chunks, walls, edges, designations; holds the name arrays that must match
  the Rust enums);
- `setup.gd` (the wizard);
- `persist.gd` (settings, autosave ring, bookmarks);
- `assets.gd` (loads sheets; falls back to the PNG if an import is missing);
- `render/` (entity, ground, light, overlay, roof, weather, fire layers);
- `audio/` (sound and the music director);
- shaders for the colour grade, palette swaps and wind sway.

The client's name arrays (`ITEM_NAME`, `STRUCT_NAME`, `PLANT_KIND`, `ANIMAL_NAME`, `EQUIPMENT` and
others) are indexed by the Rust enum values. The README generator checks that they match.

---

## 7. Content reference

### 7.1 Item kinds

On master, 33 kinds: log, stone block, iron ore, copper ore, coal, berries, grain sheaf, root, raw
meat, meal, tool, cloth, hide, firewood, plank, copper bar, iron bar, fibre, clothes, poultice,
saw, hammer, sledgehammer, chisel, needle, bucket, torch, record, gem, ore, bar, weapon, armour.

C34 adds clay, sand, vessel, leather, ale, wine and jewel; C35 adds figurine.

New kinds are always appended, never inserted, because the enum's position is saved and used by the
client's arrays.

### 7.2 Recipes

All recipes are data in `work/recipes.rs`. A recipe id is its index in `RECIPES`, so new recipes
are appended in id order. The planner posts a craft order when its want condition holds
(`settlement.rs`), at most one open order per recipe.

| Id | Recipe | Station | Inputs | Output | Since |
|---|---|---|---|---|---|
| 0 | planks | carpenter bench | 1 log | 2 planks | Phase 14 |
| 1 | copper bar | smelter | 2 copper ore, 1 coal | 1 copper bar | Phase 14 |
| 2 | iron bar | smelter | 2 iron ore, 1 coal | 1 iron bar | Phase 14 |
| 3 | iron tool | anvil | 1 iron bar, 1 plank | 1 tool | Phase 14 |
| 4 | copper tool | anvil | 1 copper bar, 1 plank | 1 tool | Phase 14 |
| 5 | cloth | loom | 3 fibre | 1 cloth | Phase 14 |
| 6 | clothes | loom | 2 cloth | 1 clothes | Phase 14 |
| 7 | poultice | hearth | 2 roots | 1 poultice | Phase 15 |
| 8 | stone tool | workbench | 1 stone block, 1 log | 1 tool | C11 |
| 9 | tin bar | smelter | 2 ore (tin ore), 1 coal | 1 bar | C33 |
| 10 | silver bar | smelter | 2 ore (silver ore), 1 coal | 1 bar | C33 |
| 11 | gold bar | smelter | 2 ore (gold ore), 1 coal | 1 bar | C33 |
| 12 | bronze | smelter | 2 copper bars, 1 bar (tin), 1 coal | 3 bars (bronze) | C33 |
| 13 | iron spear | anvil | 1 iron bar, 1 plank | 1 weapon | C33 |
| 14 | bronze spear | anvil | 1 bar (bronze), 1 plank | 1 weapon | C33 |
| 15 | copper spear | anvil | 1 copper bar, 1 plank | 1 weapon | C33 |
| 16 | coat of iron mail | anvil | 2 iron bars, 1 cloth | 1 armour | C33 |
| 17 | bronze scale coat | anvil | 2 bars (bronze), 1 cloth | 1 armour | C33 |
| 18 | bronze tool | anvil | 1 bar (bronze), 1 plank | 1 tool | C33 |
| 19 | clay pot | kiln | 2 clay, 1 log | 1 vessel (clay) | C34 branch |
| 20 | glass bottle | kiln | 3 sand, 1 coal | 1 vessel (glass) | C34 branch |
| 21 | leather | workbench | 1 hide | 1 leather (of the beast) | C34 branch |
| 22 | leather clothes | workbench | 2 leather | 1 clothes | C34 branch |
| 23 | ale | hearth | 4 sheaves | 3 ale | C34 branch |
| 24 | berry wine | hearth | 5 berries | 2 wine | C34 branch |
| 25 | silver ring | anvil | 1 bar (silver), 1 gem | 1 jewel | C34 branch |
| 26 | gold ring | anvil | 1 bar (gold), 1 gem | 1 jewel | C34 branch |
| 27 | a figure in stone | workbench | 2 stone blocks | 1 figurine | C35 branch, fey moods only |
| 28 | a figure in wood | workbench | 2 logs | 1 figurine | C35 branch, fey moods only |
| 29 | a blade | anvil | 2 bars | 1 weapon | C35 branch, fey moods only |
| 30 | an iron blade | anvil | 2 iron bars | 1 weapon | C35 branch, fey moods only |

Quality of a craft: 0.25 + crafting skill / 24, plus a tenth of the tool's quality, plus 0.2 for a
known technique (smithing, weaving, herb lore), capped by the metal (copper 0.6, bronze 0.85,
stone 0.45). Fey work is always 1.0. Tiers: 0 crude, 1 poor, 2 plain, 3 good, 4 fine, 5 masterly. A
storied piece at 0.85 or better gets a name when made.

### 7.3 Materials

Ids 0 to 27 are tile materials:
- 0 air, 1 soil, 2 clay, 3 sand, 4 gravel;
- 5 limestone, 6 sandstone, 7 granite, 8 slate, 9 basalt;
- 10 iron ore, 11 copper ore, 12 coal;
- 13 water, 14 wood, 15 stone block, 16 thatch;
- 17 grass, 18 peat, 19 snow, 20 mud, 21 meadow, 22 moss, 23 packed earth, 24 frozen soil, 25 wet
  soil, 26 scree;
- 27 gem-bearing rock (basalt, gabbro or gneiss of the deep).

C32 appended 28 to 54:
- 28 marble;
- 29 tin ore, 30 silver ore, 31 gold ore;
- 32 iron, 33 copper, 34 tin, 35 bronze, 36 silver, 37 gold;
- 38 oak, 39 birch, 40 pine, 41 fungus wood, 42 reed;
- 43 deer hide, 44 boar hide, 45 wolf pelt, 46 bear pelt, 47 goat hide, 48 fox pelt, 49 hare pelt,
  50 white hart's hide;
- 51 garnet, 52 sapphire, 53 emerald, 54 amethyst.

C34 appends 55 glass, 56 meat, 57 roots, 58 grain, 59 berries.

Ids are append-only. A material's kind is one of air, soil, stone, ore, liquid, built, plant,
metal, wood, fibre, hide, gem, and on C34 glass and food.

### 7.4 Beliefs, techniques, thoughts

Belief topics (`past.rs` `Topic`): drink, violence, fire, hunger, strangers, death, trust. A belief
has a stance, a confidence, up to six sources (events and people) and a date.

Techniques: beat out flames, carry water, poultice, forage far, smithing, roofing, weaving, herb
lore, hearthcraft, shoring, thin lore. Each is learned from a past, by watching someone succeed, or
by studying an old record.

Thoughts each carry a weight and a duration, for example:
- losing someone: -0.6 for 7 days;
- a new child: +0.5 for 5 days;
- eating well: +0.15 for a day;
- on the branches, sharing a drink +0.15 for a day, a ring +0.3 for 10 days, a masterwork +0.5 for
  15 days, a lost fey mood -0.4 for 5 days.

Mood combines needs, recent memories and thoughts; stress rises with hardship and breaks a person
past their threshold.

### 7.5 Time

| Unit | Ticks | Real time at 1x |
|---|---|---|
| tick | 1 | 50 ms (3 game seconds) |
| game minute | 20 | 1 s |
| game hour | 1,200 | 1 min |
| game day | 28,800 | 24 min |
| season (5 days) | 144,000 | 2 h |
| year (20 days) | 576,000 | 8 h |

Speeds: 1x is 20 ticks a second, 3x 60, 10x 200. Walking takes 4 ticks a tile. A child becomes an
adult in 15 years, which is 5 days at 1x; a life is about 60 years. Times in code are always ticks,
quoted against `docs/TIME.md`.

---

## 8. Art and audio

### 8.1 The style

From `docs/ART_BIBLE.md`:
- 32 px tiles.
- People in 48x48 cells (adults about 40 px tall, children about 30) with faces readable at 2x.
- Items at 16 px in hands and 32 px on the ground.
- Light from the upper left.
- A one-pixel darkest-neutral outline on figures, furniture and trees.
- Walls with a lit north and west bevel, a dark south and east bevel and a 12 px south face.
- Lamp light radius 4.5 tiles, hearth 6.
- Full day 08:00 to 18:00 with two-hour ramps; night 72% dark.
- The colour grade may leave the palette; sprites may not. `tools/art/palette_check.py` fails on
  any opaque pixel off the palette.

The palette (`godot/assets/palette/ostrog.json`) has 92 colours: eleven ramps of eight steps
(earth, stone, foliage, water, warm, cool, skin, gold, copper, olive, gem) and four neutrals.

### 8.2 Where the art comes from

Code generators in `tools/art/`:

| Generator | Makes |
|---|---|
| `terrain.py` | ground tiles, 8 variants a family, edge blends, shores, snow cover |
| `overlays.py` | cliffs and wall faces |
| `structures.py` | wall tops, doors, furniture |
| `plants.py` | trees in three stages with seasons, crops, reeds, stumps |
| `items.py` | 16 px icons |
| `people.py` | layered 48 px people: 28 frames x 4 directions, bodies, clothing, hair, equipment, bandage, plus `swaps.json` |
| `animals.py` | 14 creature sheets |
| `light.py` | the soft light sprite |
| `palette.py` | the palette |

Hero art from reference sheets (the hybrid half, from C22):
1. `onemin.py` (a private script, not in the public repository) asks a hosted image model for a sheet of
   items from a text prompt. This is off-box; only the prompt is sent, never project files.
2. `derive.py` finds the sheet's pixel grid, takes the median of each block, clusters the colours,
   snaps them to the palette in OKLab, cuts the sprites and writes their provenance.
3. `items_derived.py` maps sprites to item names (`tools/art/sources/items.json`).
4. It then makes material variants by moving some palette ramps by brightness (birch logs, marble
   blocks, garnets, bronze bars) and fixes holes.

Three sheets exist: `items_a`, `items_b`, and `items_c` (C33 and C34). A local image pipeline with commercially
licensed weights was the original plan and remains an option.

Provenance: every PNG, WAV and OGG under `godot/assets` must have a row in `tools/art/provenance.toml`,
`tools/art/provenance_people.toml`, `tools/audio/provenance_audio.toml` or
`tools/audio/provenance_music.toml`. A row records path, kind, source, generator and licence; a
derived row also records sheet, hash, model, service, prompt file and crop. `provenance_check.py`
is a CI gate.

After regenerating art, run `python3 tools/art/provenance.py`, then the Godot import
(`tools/godot --path godot --headless --editor --quit`) before taking screenshots.

### 8.3 Sound and music

Everything is synthesised offline by code in `tools/audio/`, seeded by name, so rebuilds are
byte-identical:
- `synth.py`: plucked strings, struck objects, drones, pads, flute, reverb;
- `sfx.py`: about 30 sounds, from chopping to wolves;
- `music.py`: nine cues, each 32 bars in four looping stems. The cues are spring, summer, autumn
  and winter by day and by night, plus the deep. The modes are spring Mixolydian or Lydian, summer
  Ionian, autumn Dorian and winter sparse Aeolian.

The client's buses are Master with Music, Ambience, SFX and UI. Ambience is low-passed and SFX
reverbed underground. The music director plays a cue for 2 to 4 minutes, then 1 to 3 minutes of
ambience. It chooses by season, day or night, and depth, and raises the tension stem with the
community's strain. `tools/audio/audio_report.py` fails CI on clipping or a click at a loop seam.

---

## 9. Building, running and the tools

### 9.1 Build and run

```
export PATH="$HOME/.cargo/bin:$PATH"
cargo build --release -p hf-cli -p hf-godot
mkdir -p godot/bin/linux && cp target/release/libhf_godot.so godot/bin/linux/
tools/godot --path godot --headless --editor --quit     # import assets and register the extension
tools/godot --path godot                                 # play
```

`godot/bin/` is not in git; a fresh checkout or worktree needs the library copied and the import
run once, or the GDScript check fails with "Could not find type SimHost".

### 9.2 The command-line tool (`target/release/ostrog`)

| Command | Does |
|---|---|
| `run --seed N --ticks T [--agents A] [--engine steady|relentless|whimsical] [--save P]` | run a world and print a summary and the digest |
| `soak --seed N --days D [--opening] --csv P [--ring P --save-days N]` | long run with a CSV row a day and gates (section 10.3) |
| `profile --seed N --days D --agents A [--spike-ms 5] [--provision]` | per-system tick timings |
| `realm --seed N [--years Y] [--ppm file]` | the world map and its history |
| `legends --seed N --page site:3` | a Legends page as text |
| `describe --seed N [--wetness --warmth --stoniness]` | what a valley is like |
| `past --seed N --years Y` | the valley's past |
| `resume PATH --ticks T`, `verify PATH`, `fixture PATH` | saves |
| `digest`, `stage`, `bench-save`, `weather`, `director`, `debug-plan`, `tile X Y` | diagnostics |

### 9.3 Client flags

These are passed after `--`, for example `tools/godot --path godot -- --seed=3 --opening=steady`.
- `--seed=N`, `--opening=steady|relentless|whimsical`, `--stage=demo`, `--load=PATH`, `--fresh`,
  `--ring`;
- `--steps=N` (also `--ticks=N`), `--speed=N`, `--zoom=Z`, `--z=Z`, `--at=x,y` or `--at=band`;
- `--shot=OUT.png`, `--footage=DIR --every=N --frames=N`, `--wizard-shot=OUT`,
  `--wizard-choices=...`: comma-separated `key=value` pairs the wizard is set to before each of
  its pages is shot and Begin pressed: `deity=<Aun|Hask|Mirre>` (or the pacing names steady,
  relentless, whimsical), `years=<N>` (the valley's past), `held=<topic>:<-1|0|1>`,
  `exclude=<kind>` (one pair per kind: `exclude=tharn,exclude=skarri`) and
  `rarity=<kind>:<common|uncommon|rare|legendary>`, kinds by id or name in any case (`tharn`,
  `Tharn`). If the world cannot be made as chosen, Begin is refused: the
  wizard stays on the page concerned with the reason, stdout prints `BEGIN REFUSED (...)`, and
  `_started.png` shows the wizard (#147);
- `--history`, `--legends=TARGET`, `--known`, `--inspect=ID`, `--no-director`, `--music-fast`,
  `--audio-log=PATH`;
- `--save-at-end=PATH`;
- `--ignite=x,y --after-ignite=N`, `--hungry-neighbour`.

Unknown flags are reported, never silently ignored.

### 9.4 Screenshots

`tools/shots/shot.sh OUT.png [client flags]` draws on an Xvfb display (`HF_DISPLAY`, default `:96`),
because the build server's real GPU display refuses frames from a session that is not its own. It reimports
assets when needed (checking every file, because Godot's import crashes about once in twenty runs,
issue 42), runs with the OpenGL renderer, and fails unless the log says `SHOT_SAVED`. Example used
for review frames:

```
HF_SHOT_TIMEOUT=600 tools/shots/shot.sh /tmp/demo.png --stage=demo --steps=1 --zoom=4 --at=60,45
```

### 9.5 CI (`tools/ci.sh`)

In order:
1. `cargo fmt --check`;
2. `cargo clippy --workspace --all-targets -D warnings`;
3. `cargo test --workspace`;
4. the provenance check;
5. the audio report;
6. `tools/docs/readme_tables.py --check`;
7. the GDScript compile check (`tools/gd_check.gd`, all scripts must load);
8. prints `CI OK`.

On GitHub only the Rust half ran, and Actions is now manual-only. The suite runs on the build server in
the `hf-boxci` container (tools/boxci/README.md), which posts a `box-ci` status on the master
head and every open pull request's head.

`cargo test` stops at the first failing test binary, so after a failure run
`cargo test --workspace --no-fail-fast` to see them all. The full suite takes 25 to 35 minutes on
the build server.

The README's generated tables (every string the game shows, every asset, the bridge layout, the
pipeline) are rewritten by `python3 tools/docs/readme_tables.py`. Run it after adding an item kind,
a recipe, a material, a string or an asset.

### 9.6 Packaging

```
cargo build --release -p hf-godot
cargo xwin build --release -p hf-godot --target x86_64-pc-windows-msvc
tools/package.sh [VERSION]
```

This needs the Godot 4.6.3 export templates. It writes `releases/ostrog-linux-x86_64.tar.gz`,
`releases/ostrog-windows-x64.zip` and `SHA256SUMS`. The packages in `releases/` today are from
2026-10-14 and are stale.

---

## 10. Testing: how to check that it works

### 10.1 The automated tests

There are 284 `#[test]` functions on master (about 56 files in `crates/hf-sim/tests/`, one per
subject, plus unit tests in the source). The branches add 10 (crafts) and 4 (masterworks). Most
tests build a small world with `World::generate(seed, GenParams::small())`, flatten it, found a
party, place what they need, and step until something happens. Examples worth reading first:
- `work.rs`: prerequisite chains, contention without deadlock, ledger zero drift;
- `foundations.rs`: determinism, save round trip, tampered manifest rejected;
- `history.rs`: `chronicle_only_real_events`;
- `arms.rs`: `ore_becomes_a_weapon_that_is_worn_and_used`;
- `threats.rs`: bands sent by hunger;
- `realm.rs`: rivers reach the sea, rain shadows fall behind ranges.

### 10.2 What is and is not a bug (from the tester sets)

Not bugs: people dying of hunger, cold, fire, infection or old age, nobody fighting a fire,
communities collapsing. Losing is fun.

File these:
- a death with no visible cause in the History panel or the inspect panel;
- anything appearing that nobody made;
- a person stuck in one place;
- a line of history that cannot be traced back;
- the game asking the player for a decision;
- a crash;
- the same seed producing a different world.

### 10.3 Soaks

```
ostrog soak --opening --seed N --days 600 --csv out.csv --ring DIR/sN --save-days 5
```

This runs the real game (the arrival into a valley of its world) for 600 game days, about 30 to 120
real minutes. Each day writes a CSV row: tick times, humans, animals, stalled people, plan failures,
leases expired, ledger drift, memories, chronicle rows, items, structures, caravans, births, deaths,
RSS and save time.

A run passes when stalled person-hours are under 1%, the ledger never drifts, no save fails, and
memory stays within the budget for the peak population. Famine is reported, never a failure.

Four seeds are run per milestone. Running four at once inflates tick times.

To count human deaths by cause, read the save, not the chronicle's "died" rows. The chronicle counts
animals too: the M3 review once wrongly blamed wolves for deaths that were deer killed by the pack.

### 10.4 The AI tester and the test sets

The hands-on tester is an AI on the Windows test PC. It is called the test agent in the docs; the 2026-10-17 QA
round was filed by the local AI under the maintainer's GitHub account. It tests tagged master builds only and files
one GitHub issue per finding, labelled `test-agent-test`, with commit, tag, machine, command, expected
and actual, and a screenshot or log.

The sets are in `docs/history/TEST-SETS.md`:

| Set | Covers |
|---|---|
| A | build and the automated gate |
| B | the arrival and lived pasts |
| C | beliefs and techniques |
| D | the picture |
| E | sound |
| F | long runs overnight |
| G | outside threats |
| H | pacing and hard times |
| I | building |
| J | tools with a life |
| K | the valley's past |
| L | study and old crafts |
| M | tales |
| N | creatures |
| O | the deity and the past |
| P | the wizard and hearth evenings |
| Q | the deep |
| R | QA round 2 fixes on Windows |
| S | the thin place |
| T | scale and memory |
| U | art |
| V | the world |
| W | the Legends browser |
| X | food, materials and arms (C31 to C33) |
| Y | the other crafts (C34, on its branch) |
| Z | masterworks (C35) |
| AA | trade with the world (C36) |
| AB | work someone wished for (C38) |
| AC | fight frames and the digitigrade art (C40, C41) |
| AD | the species line-up (C41) |
| AE | night and cave senses (C42) |
| AF | the wizard's peoples page (C43) |
| AG | kinds of the world's own (C44) |

Sets T to AG are waiting for the tester. AB to AG test checkpoints still on branches; run them
once those are merged to master.

### 10.5 Checks a person can do in ten minutes

1. Start a game (`tools/godot --path godot -- --fresh --seed=3`), run at 10x for a game day, press H.
   Every line in the record should make sense, and clicking a person should show why they are doing
   what they do.
2. Hover a stockpile. Stacks are named with their materials.
3. Press L, click from the world to a people, to a settlement, to a figure, to their parent, and use
   Backspace to come back.
4. Save with F5, keep playing, load with F9. The world returns to the save.
5. Run `ostrog run --seed 7 --ticks 200000 --agents 12` twice. The digests match.

---

## 11. Working rules: git, checkpoints, records

- One checkpoint per working session, numbered C<n>, on its own branch (`c33-chains` style), in its
  own worktree `<src>/ostrog-<branch>`.
- Push every green step. Commit messages are a plain-sentence subject, a blank line, the reason,
  then a co-author line.
- Before merging: run `tools/ci.sh` to `CI OK`, write the review under
  `docs/reviews/<topic>-<date>/README.md` with measurements and frames, update STATUS.md and the
  design doc, add a tester set, and regenerate the README tables.
- Merging: open a PR against master, wait for MERGEABLE, `gh pr merge N --merge` (always a merge
  commit), check that the head commit's subject is `Merge pull request #N from <owner>/<branch>`,
  then tag `cN` on it and push the tag.
- Checkpoint from master: `tools/checkpoint.sh`. It runs CI, pushes, makes a git bundle of the whole
  history, mirrors the source and releases to `<mirror>`, verifies the
  mirror (no differences, bundle verifies, records match, release sums check, rclone lists the
  folder), and writes `CHECKPOINT.txt`. On any failure it leaves `MIRROR_PENDING.md`. The mirror
  sometimes still shows partial uploads after five minutes; wait, rerun the verify steps, and write
  `CHECKPOINT.txt` once they pass. On master the checkpoint then cross-builds the Windows
  `hf_godot.dll` for the commit and publishes it under `prebuilt/windows/` on the mirror
  (`tools/prebuilt/README.md`, #430); a failed build there leaves `PREBUILT_PENDING.md` and does
  not fail the checkpoint.
- Never run git or cargo on the mirror mount (D2: `git status` took 44 s).
- Delete nothing. Move it to a `to_delete/` folder.
- Retarget dependent PRs before deleting a base branch: deleting a merged base branch closes PRs
  stacked on it.
- Background tasks die at their timeout. Give CI and checkpoints up to two hours.
- The records at the top of the repo: VISION.md (why), DEFINITION_OF_DONE.md (when it is done),
  ROADMAP.md (phases), DECISIONS.md (D1 to D18; there are two entries numbered D16), STATUS.md
Internal maintenance and backup instructions are maintained separately from this public documentation.
  (the mirrors).

---

## 12. The road ahead

### 12.1 Finishing M4

- C34 and C35 are merged and tagged; C36 (trade) is built (section 5.2).
- The M4 review: four 600-day soaks, the profile, a tester round, and the gap table again. Gaps
  already known for it:
  - arms are put away before the next raid (people should keep them while a band is camped
    nearby);
  - prospecting covers only exposed veins;
  - walls of each stone need wall art (moved to M7);
  - armour is not drawn on people;
  - raiders carry no arms (M5);
  - whether drink should need ten days' eating in store.

### 12.1b MP: peoples of the world (thirteen checkpoints, c38 to c50)

Decided 2026-10-18; the design is `docs/design-peoples.md`, the options not taken first are
backlog E14. C39 to C44 are built on stacked branches (section 5.3b) and wait, with C37 and C38,
to be merged in order. In order:
- P0 personal projects (carving, brewing, workshops, caravans proposed by persons): C38, built,
  in review;
- P1 body plans (one part list for art and wounds; size effects): C39, built;
- P2 the art rig (fight frames, the digitigrade plan): C40, built;
- P3 species in the valley (library, `SpeciesId`, `Species::Person`, six surface anchors): C41,
  built;
- P4 senses and cycles (darkness, dark sight, hearing, smell, nocturnal, heat, diets): C42, built;
- P5 the world draw and the wizard's peoples page: C43, built;
- P6 generated species (four axes, distinct looks, language names): C44, built;
- P7 cultures, ethics and values with sources: C45, under way on `c45-cultures`;
- P8 stories from rows and gods per culture;
- P9 relations, intermarriage, refugees, hybrids;
- P10 the deep (deep caverns, an underworld, cave peoples, deep beasts);
- P11 conditions (moon, ghosts, three curses, cures);
- P12 the MP review.

### 12.2 M5: bodies, conflict, war and order (about six checkpoints)

- Bodies with tissue layers and organs, wounds per layer, bleeding, infection, scars, prosthetics,
  medicine with surgery, and disease that spreads.
- Combat with the M4 weapons and armour, skills, morale, fleeing, cover from terrain, grappling.
- Militias that form from belief and need, never from orders.
- War from the world layer: raids, sieges with real armies from the realms, prisoners, ransom,
  occupation, legendary beasts.
- Society: leadership from respect, disputes and justice by the people's beliefs, crime and
  punishment ending in restitution, reconciliation or exile, feuds across generations, roles with
  friction (#54), festivals and rituals, religions per people.
- Gates: a siege fought without orders and recorded from the first sighting to the aftermath; a
  feud traced across two generations; a wound's layers on inspect.

### 12.3 M6: living world depth (about six checkpoints)

- Fluids (water and magma flow, wells, floods, irrigation), temperature by tile, and weather at
  region scale.
- Farming by climate and soil, with irrigation and blight.
- Domestication, herding, riding, pack animals, and 80 or more creatures with roles in their
  biomes.
- Caverns layered down to an underworld with cavern peoples, and the thin place tied to the world's
  portal lore.
- Children, schooling and apprenticeship; romance and families; breaks and inspirations with
  causes.
- Desire paths (#84), visible wear, settling the caverns.
- Making the missing seven harder to find.
- Promoting world figures into the valley when they arrive (D10, keeping identity), and migrants
  and armies from the world's wars reaching the valley.

### 12.4 M7: presentation and audio (about four checkpoints plus continuous art)

- A UI built for watching: an overlay of what matters now, timelines, charts of population, stores
  and beliefs, a world view, composed portraits, a polished Legends browser with search and family
  trees, and accessibility (scaling, a colour-blind check, readable fonts).
- Water and weather shaders, wind sway, rim light, roofs from above.
- Music per culture and site, combat and industry sound, ambience by biome and depth.
- The art breadth targets: 60 terrain families, 50 plants, 80 creatures, 150 structures and pieces
  of furniture, 300 items, 100 clothing pieces.
- Gates: the tester's readability and polish rubric against RimWorld screenshots, the contrast and
  palette reports, and frame time with every effect on the tester's GPU.

### 12.5 M8: ship (about three checkpoints)

- Linux and Windows packages launched by the tester with sound.
- A 24-hour soak at 10x on four seeds with every gate green.
- Every Definition of Done row evidenced.
- Issues #18 and #19 closed by full end-to-end Windows passes with screenshots and logs.
- Release notes.

### 12.6 Backlog epics (docs/BACKLOG.md)

| Epic | Theme |
|---|---|
| E0 | evidence and fairness harness |
| E1 | ship gate |
| E2 | endurance and performance |
| E3 | onboarding, errors, packaging |
| E4 | records and provenance |
| E5 | liveliness |
| E6 | personal depth (RimWorld) |
| E7 | physical and economic depth (Dwarf Fortress) |
| E8 | faith, power and knowledge |
| E9 | civilisation beyond the settlement |
| E10 | z-levels first-class |
| E11 | presentation |
| E12 | motive: why anyone does anything |
| E13 | a pantheon |

`docs/design-emergence.md` is a brainstorm (not the plan of record) of loops with dampers and edge
cases, including the hard requirement that the world must stay stable and legible forever without
the viewer.

---

## 13. Known problems, gotchas and stale records

Behaviour:
- In 300-day runs arms are rare and put away before the next raid (C33).
- Nobody reached tin, silver or gold before C34's prospecting.
- Seed 2 is a hungry valley in every build (8 to 10 starvation deaths in 300 to 600 days).
- Drink is rare now that brewing waits for ten days' eating; whether that bar is right is for the
  M4 review.
- The missing seven are found too quickly (15 to 24 days).
- About 400 wars in 1,000 years on seed 1, so peace rarely lasts.
- Settlement populations in the wider world are aggregates, and figures are not yet promoted into
  the valley.
- People not drawn with armour; raiders unarmed.

Environment:
- A new worktree has no `godot/bin/linux/libhf_godot.so` and no import cache, so the GDScript check
  fails until the library is copied and the import run (section 9.1).
- Godot's import crashes about once in twenty runs and can leave a half-written cache (issue 42);
  `shot.sh` checks every file and retries.
- Screenshots need the Xvfb display `:96`; the real display refuses frames.
- `cargo test` stops at the first failing binary.
- Background tasks are killed at their timeout.
- The backup mirror can still show partial uploads after the checkpoint's five-minute wait.

Records that are stale:
- STATUS.md says C33 is "in review"; it is merged.
- DEFINITION_OF_DONE.md's evidence text predates M3 and M4.
- The README's controls list omits L, its CI section lists fewer steps than `ci.sh` runs, and it
  says the palette has 60 colours (92 since v2); this is a historical-record note, and the
  current palette is 92 colours as documented in `docs/ART_BIBLE.md` and `docs/ART_SPEC.md`.
- ASSETS.md's counts are old.
- `ci.sh` refers to `tools/shots/canonical.sh`, which does not exist (the step is skipped).
- `releases/` holds 2026-10-14 packages.
- `thin.rs` says the stone wakes after "two seasons" but the constant is 10 days.
- DECISIONS.md has two entries numbered D16.

---

## 14. If something bad happens: recovery

- Historical private development records are retained separately.
- A full copy with the whole git history is on the mirror at
  `<mirror>/ostrog.bundle` (from the Windows PC, the mirror
  drive's `<mirror>` folder). `CHECKPOINT.txt` there names the last verified commit.
  To restore:

  ```
  git clone /path/to/<mirror>/ostrog.bundle ostrog
  cd ostrog
  tools/ci.sh
  ```

  `tools/restore_test.sh` does exactly this into a scratch folder and prints `RESTORE OK`.
- The source tree as plain files is in `<mirror>/src/`, and the release packages in
  `<mirror>/releases/` with their checksums.
- The Windows side has its own mirror, `<mirror>_Windows` (D16b).
- To carry on: read this handbook, then STATUS.md, then the design doc of the current milestone
  (`docs/design-industry.md` for M4). Run `tools/ci.sh` and do not start on a red build. Then do the
  next checkpoint the way section 11 describes.
- `tools/godot` looks for a Godot 4.6.3 binary at `.tools/godot4` in the repo, then in a sibling checkout
  checkout, then on the PATH; on a new machine put one at `.tools/godot4`. The Rust toolchain installs itself from `rust-toolchain.toml` through
  rustup.

---

## 15. Glossary

| Term | Meaning |
|---|---|
| survivor | one of the ten who came through the portal |
| the starting survivors | the three who begin together |
| the missing | the other seven, scattered, until found |
| the arrival / epoch | the tick the portal opened; history before it is the past (D18) |
| community | a group living together with a home, members and a name |
| deity | Aun, Hask or Mirre: notices and leans, never fires an event |
| designation (order) | a piece of work the planner posted: build, mine, craft, haul |
| requirement | an item an order needs delivered, with the material it must be |
| lease | a time-limited claim on an item, order, station, tile or plant |
| the planner | `systems/settlement.rs`: posts work, never conjures materials or sends anyone to a crisis |
| technique | a way of doing something, learned from a past, by watching, or from a record |
| belief | a stance on a topic with a confidence and sources; weights choices by up to 40% |
| storied item | an item with a maker, owners, wear, deeds and perhaps a name (`legend.rs`) |
| chronicle | the SQLite record of everything non-trace that happened; immutable |
| real history / taught history | what happened, and what a community was taught (tales and customs) |
| told | what the player may know (`view/known.rs`) |
| culture log | the last 200 major events kept in the world (used by customs, carvings, moods) |
| realm | the wider world: peoples, settlements, figures, polities, wars |
| overworld | the valley's neighbours and sites, reached by caravans |
| band | real people sent by a hungry place; they camp, raid and leave |
| the thin place | the portal stone wearing thin after the arrival |
| fey mood | an artisan taken by a work for a recorded cause (C35) |
| soak | a long automated run with gates on stalls, drift, memory and saves |
| checkpoint | a numbered step of work, merged, tagged and mirrored offsite |
| the test agent | the name used in the docs for the Windows-side agent and tester |

---

## 16. Where to find things

| Looking for | Go to |
|---|---|
| Why the game exists | `VISION.md` |
| What is next | `STATUS.md` "Current" and this handbook's section 12 |
| The milestone plan | `docs/PLAN-FINISH.md` |
| The current milestone's design | `docs/design-industry.md` (M4), `docs/design-world.md` (M3), `docs/design-organic.md` (people) |
| Every decision | `DECISIONS.md` |
| Measurements per checkpoint | `docs/reviews/<topic>-<date>/README.md` |
| Test sets for the tester | `docs/history/TEST-SETS.md` |
| Soak records | `docs/soak/README.md` |
| Every string, asset, bridge field | `README.md` (generated tables) |
| Time units | `docs/TIME.md` |
| The tick order | `crates/hf-sim/src/systems/mod.rs` |
| How people choose | `crates/hf-sim/src/systems/decide.rs` |
| What the planner posts | `crates/hf-sim/src/systems/settlement.rs` |
| What finished work does | `crates/hf-sim/src/systems/work.rs` `apply_effect` |
| Recipes | `crates/hf-sim/src/work/recipes.rs` |
| Materials | `crates/hf-sim/src/world/material.rs` |
| Event text | `crates/hf-sim/src/view/narrate.rs` |
| The bridge | `crates/hf-godot/src/lib.rs` |
| The client's root | `godot/scripts/main.gd` |
| Art generators | `tools/art/` |
| Sound and music | `tools/audio/` |
| CI, checkpoint, screenshots, packaging | `tools/ci.sh`, `tools/checkpoint.sh`, `tools/shots/shot.sh`, `tools/package.sh` |
