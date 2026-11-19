# Emergence brainstorm: why anyone does anything, and what the neighbours do about it

> This remains a brainstorm and source of examples. The current implementation contract for
> person-specific observation, weighted dispositions, problem/reaction/outcome chains and social
> learning is `docs/design-minds-observation.md`. Do not treat an example here as shipped
> behaviour until it has an issue, guard and integrated review.

Requested as a brainstorm with edge cases. This is a design document, not a plan of record; the
stories that come out of it live in `BACKLOG.md` (E6, E7, E8, E9, E12). Everything here is
constrained by the project's law: no scripted events, no omniscience, no modal prompts, no
language model in the simulation (D9), and history is recorded rather than invented.

## 0. The spine

```
need -> goal -> plan -> action -> event, each carrying its because-line
                    ^
            allowed only if the technique is known
```

```
disposition (trait mix) -> the work they take -> what others see or hear -> reputation
                         -> what the community posts more of -> identity -> customs
```

Both loops are seeded, deterministic and testable. A "reason" is always derived from recorded
state; nothing is invented to make a scene.

## 1. Inter-colony dynamics

Archetypes are not chosen at worldgen — they are labels the world earns from capabilities and
history, and a colony can drift between them:

| What a colony is good at | What it lacks | What it does about it |
|---|---|---|
| Fighting | Crafting | Raids for tools and food; later learns by reverse-engineering what it takes |
| Crafting | Fighting | Sells tools, buys protection, hires caravan guards |
| Farming | Everything else | Trades surplus; vulnerable to tribute demands |
| Trading | Production | Brokers knowledge and goods; vulnerable to blockades |
| Isolation | Contact | Stagnant technique base; safe until it is not |
| Observance | Pragmatism | Spreads customs; hostile to different customs |

Relations are a continuum, each step caused by recorded events, never assigned:
unknown → contacted → wary → trading → allied/kin → tributary → raiding → besieging → vassal →
destroyed → ruin (which later becomes the diggable strata of E9.3).

Loops worth having, each with a natural damper:

- **Deterrence**: visible defence reduces raids; over-defence means nobody farms.
- **Dependency**: a raider living on tribute starves when the payer collapses; it then switches to
  trade or becomes bandits.
- **Arming the neighbour**: selling tools raises their capability — a real consequence of trade.
- **Diffusion**: stolen tools can be reverse-engineered, giving partial technique knowledge
  (worse quality than being taught) — a second path to progress.
- **Refugees**: a destroyed colony pushes survivors outward, carrying skills and trauma.
- **Rumour**: what a colony knows about another comes from travellers and hearsay, and is wrong
  more often the further away it is (E8.6).

**The drama director ("what would Randy Random do?")** — implemented honestly. Randy is a pacing
device in RimWorld; here the equivalent is a **pressure system that changes rates, never injects
events**. A tension metric is computed from real state (unmet needs, time since the last notable
event, population stagnation, wealth concentration, external hostility). High tension raises the
*probability rates* of arrivals, discoveries, wildlife pressure and contact; low tension lowers
them. Guardrails: pressure is telegraphed in-world; there is always a counter-play; it never
punishes the viewer; it never kills a specific individual by fiat; the whole thing is seeded so a
run is reproducible.

## 2. Mental strain (the Dwarf Fortress sanity layer, humanely)

Strain accumulates from recorded conditions only: hunger, sleep debt, cold, danger, grief,
overcrowding, darkness, isolation, humiliation after punishment, broken customs, overwork.

Coping is real and buildable: a private room, social contact, work that fits aptitude, custom
observance, rest days, care from others, and purpose-built rooms (below).

Manifestations are behaviours, not diagnoses: withdrawal, insomnia, hoarding, refusal to work,
aimless wandering, flight, lashing out, error-prone labour. Each leaves a trace — a memory with
negative valence that biases later choice, a reputation change, sometimes a custom born from how
the community responded.

Community response is the interesting part: convalescence and care, restraint, conciliation, or
exile (E6.7). A settlement that cares for its strained members accrues cohesion; one that exiles
them accrues a different kind.

Representation rules: treat it as distress, not comedy. No self-harm mechanics. No "insane"
caricature. Recovery is always possible. Strain must be traceable to recorded state — if a player
cannot ask "why is she like this" and get a real answer, it is not implemented correctly.

## 3. Purpose-built rooms

The answer to "why would I add to a structure" is a trigger, and each room changes a rate:

| Room | Built because | Does |
|---|---|---|
| Bedroom | another person, or a birth | rest quality, privacy, less strain |
| Commons / hall | population, gatherings | social need, cohesion, decisions |
| Kitchen | poor cooking throughput | meal quality — this is the fix for the 30× cooking variance |
| Workshop | craft backlog | craft speed and quality, tool wear |
| Storeroom / cellar | stores full, spoilage | preservation; attracts vermin (E7.8) |
| Shrine | a custom needs a place | observance, cohesion |
| Infirmary | injuries, illness | recovery rate |
| Scriptorium | a technique holder is aging | knowledge survives its knower (E8.3) |
| Teaching space | techniques not spreading | transmission speed |
| Gatehouse / defensible room | raids happened | raid outcomes |
| Bath / latrine | refuse, illness | hygiene |
| Guest room | visitors keep arriving | diplomacy, knowledge exchange |

Rules: rooms need a trigger; they cost space, materials and upkeep; they can be mis-sized
(overcrowding is worse than none); they can be repurposed; their absence degrades something
visibly. Additions are staged (E12.11) so the colony can be seen growing.

## 4. Edge cases, with the damper that keeps each from breaking the world

1. **Everyone is a coward** — no expeditions, stagnation. Damper: cowardice eases after escorted
   travel; knowledge still arrives via visitors and trade.
2. **Everyone is greedy** — hoarding, no commons work. Damper: cohesion falls, ostracism and a
   corrective custom emerge.
3. **The only knower dies** — technique lost, possible regression. Damper: teaching and the
   scriptorium; regression is legitimate and recorded.
4. **The knower is exiled before teaching** — permanent loss. Damper: they can teach first, or
   return; the loss is visible in the chronicle.
5. **A raid takes every tool** — capability collapse. Damper: repair tiers, salvage, trade,
   reverse-engineering.
6. **A tribute payer collapses** — the raider starves. Damper: raiders switch to trade or banditry;
   the collapse leaves a ruin.
7. **Knowledge outruns materials** — permanent frustration. Damper: planning de-prioritises
   techniques it cannot resource.
8. **Monopoly on a technique** — extortion dynamics. Damper: diffusion through travel and teaching;
   rumour eventually spreads it.
9. **False reputation from rumour** — innocent ostracism. Damper: confidence decays and is corrected
   by direct observation.
10. **Two colonies with identical strengths** — no reason to trade. Damper: aptitude drift makes
    them diverge over time.
11. **Refugee flood** — overcrowding strain. Damper: capacity limits and turning people away with
    recorded consequences.
12. **Children inherit aptitude, not knowledge** — slow restart after a loss. Damper: teaching.
13. **A self-defeating trait mix** (greedy + cowardly with no hirelings) — the person simply does
    badly; no crash, no special case; others notice.
14. **The viewer never intervenes** — the world must remain stable and legible forever. This is a
    hard requirement, not a nice-to-have.
15. **Siege with no defence** — capture or exile, but never a dead world: survivors flee, become
    bandits, or found a new site.
16. **Over-militarised colony** — nobody farms. Damper: militia is part-time.
17. **Perfect defence** — nothing happens. Damper: the pressure system raises other pressures
    (disease, weather, internal conflict).
18. **A custom forbids necessary work** — real cost, and the custom can lapse (E8.1).
19. **Bridge or caravan dependency** — isolation collapse. Damper: alternative routes at higher
    cost.
20. **Reverse-engineered technique** — skipping the teaching path. Damper: partial quality until
    properly taught.

## 5. What to build first

The three that change what you can see today: **why-chains** (E12.1), **staged construction**
(E12.11) and **purposeful errands** (E12.12). Then techniques (E12.2–12.5), then disposition, fit
and reputation (E12.6–12.10), then the inter-colony and strain layers above.

## 6. How this is verified

Behaviour rows are never moved to `verified` on prose. Each story carries a seeded test, and the
per-day event histogram that exposed the current slowdown (see `docs/reviews/soak-2026-10-14/`)
becomes the standard before/after instrument: if construction is staged and errands purposeful,
the day-by-day table stops reading "0, 0, 0" for days 7–9.
