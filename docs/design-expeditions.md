# Phase 10 design — expeditions (the test agent branch)

Working spec for the Windows agent. The build server owns the same phase on master; where this spec
and the build server diverge, the merged branch keeps the better evidence (matrix rows, not prose).
Conventions in force: ticks only (docs/TIME.md), `rng::stream` purpose tags (D7), ledger for
every item (zero drift), history recorded never invented (D9), `#[serde(default)]` for new
fields (D14, no format bump).

## 10a — Overworld and knowledge

- New module `crates/hf-sim/src/overworld.rs`.
  - `SiteKind`: `Market, Village, Ruins, Shrine, BanditCamp, Pasture, Quarry, Spring`.
  - `Site { id: SiteId, pos: (i32, i32), kind, name, hostilities: f32, stocks: Vec<(ItemKind, u32)> }`
    — `pos` is in overworld hops beyond the map edge; the ring holds ~20 sites at 1–6 hops:
    exactly one Market, 1–2 Village, 2–4 Ruins/Shrine, 1–3 BanditCamp, 1–2 Pasture/Quarry/Spring.
  - Generated once at world creation from `stream(seed, &[purpose::OVERWORLD_SITES, i])`;
    names from a new `names::site_name` on the `NAMES` family.
- `World` gains `overworld: Overworld` + `next_site_id: u32`, both `#[serde(default)]`;
  snapshot.rs lists them; v1 fixtures must still load unchanged (test-verified).
- Knowledge: every site starts unknown to every community (no omniscience, principle 2).
  A site becomes known to a community by (a) a scout's return or (b) a trade contact at a
  known market naming a neighbour ("the market folk talk of X"). Community knowledge, not
  world truth: two communities can know different site sets.
- New events: `SiteDiscovered { community, site, by: Option<AgentId> }` (Notable),
  narrated "<community> heard of <site>."
- Gates: `sites_stable_across_seeds` (same seed ⇒ identical sites; different seed ⇒ differs),
  `save_roundtrip_identical` still green, knowledge initially empty.

## 10b — Caravans

- `CaravanPurpose`: `Trade { destination: SiteId, want: ItemKind, offer: ItemKind }`,
  `Scout { destination: SiteId }`, `Search { target: AgentId, lead: Option<SiteId> }`.
  A caravan always has a purpose; formation is a community decision recorded as
  `CaravanFormed { community, leader, members, purpose }` (Notable).
- Formation trigger (community pass, daily, `autoplan`-gated like the planner):
  surplus of `offer` past the store threshold + known Market stocking `want` (Trade);
  a known site of interest + a need or curiosity trigger (Scout); a missing relative (Search).
  Members 3–8, chosen by skill and `memories.bias` (a site's valence steers who volunteers).
- Preparation: cargo leaves the store as `ItemPlace::Caravan(caravan_id)` (new variant —
  verify the v1 fixtures and roundtrip still pass; external tagging keeps old values' wire
  shape, so expect no format bump; if a test disagrees, that is a D14 format event, record it).
- Departure at the dawn tick-of-day: members get `on_caravan: Some(id)` (new agent field,
  default None) and are excluded from local decide/act/perceive; ledger is untouched —
  goods change place, they are not destroyed.
- Travel: one site-hop per game-day. Each day-step draws
  `stream(seed, &[purpose::ENCOUNTER, caravan_id.0, leg, day])` over a table weighted by
  route hostilities and season: Quiet 0.55 / Bandit 0.10+0.25·hostilities / Beast 0.15 /
  Storm 0.10 / Sickness 0.05. Outcomes mutate cargo, injuries and (rarely) life; every
  outcome is an event with a template line and a real cause (no invention).
- Return: cargo lands at home as items (`ledger.created("expedition")`), members return
  alive, `CaravanReturned { community, site, leader, brought, lost }` (Major), members write
  short-term `Expedition` memories keyed to the site; sleep consolidation carries them long
  (gate 10c), and the site's valence then biases future formation and volunteering.
- Gates: `caravan_returns_with_outcome`; `ledger_zero_drift_default_world` stays green;
  `digest_deterministic` stays green (new purpose tags must not shift old draws — D7).

## 10c — Lost, relocated, stranded; culture hooks

- Lost: a severe encounter resolves the caravan `Lost` (no return). The chronicle keeps the
  whole arc; after the absence the record reads "The caravan for <site> never returned."
  — derived from the departure plus the absence, never embellished (D9). Families keep
  Hardship memories; the community marks the site dangerous and trade formation to it stops
  (a behavioural consequence a test can see). A second community that never lost anyone
  there may still trade it — differing interpretations, matrix row 5.
- Customs: two or more losses form a travel taboo via the existing culture pass
  (origin = the CaravanLost event id); formation checks taboos before acting.
- Stranded: an injury can halt the caravan mid-route; if the community cannot spare a
  rescue party, the stragglers settle into a new remote community at that site (names,
  roster, status in the overworld record; local agents drop, continuity preserved per D10).
  A later caravan visiting that site can find them: `FoundAgain` (Major) with family
  memories; a relocated community's births and deaths continue in the chronicle as
  overworld events.
- Gates: `caravan_lost_leaves_history`, `expedition_memories_consolidate`, and a seeded
  30-day run (3 seeds) showing expeditions alongside trade, splits and customs — the row 5
  evidence. Soak metrics recorded in STATUS.md.

## 10d — Presentation (the Windows edge)

- Client overworld layer: the hop-map as a toggleable view; sites as named markers,
  caravans as moving dots with route lines; site inspection popup; feed lines as designed.
- Director: `CaravanDeparted` / `CaravanReturned` are Major — the camera goes to the map
  edge where the caravan last stood (the overworld itself stays a widget, not a scene
  change, until footage says otherwise).
- This machine runs the client on a real display with the gl_compatibility renderer: the
  Windows half of matrix rows 1/7/12 is only evidenceable here (the build server renders via Xvfb
  software GL and its GPU refuses frames). Golden shots + a glance review land in
  docs/reviews/ with seed and tick recorded (D13).
