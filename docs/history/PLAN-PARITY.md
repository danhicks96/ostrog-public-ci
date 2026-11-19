# Parity programme: Dwarf Fortress depth, RimWorld readability, better art than both

This is a dated parity gap record. Current execution is in `docs/history/PLAN-NEXT.md`; current object
breadth and custody rules are in `docs/OBJECT_CATALOG.md` and D57; current visual direction is in
D58, D63, D64 and `docs/ART_BIBLE.md`; situated-minds rules are in
`docs/design-minds-observation.md`. The early counts below describe the Phase 12 baseline and are not a
claim that the later catalog or art overhaul is already shipped.

Written 2026-10-15 after the Phase 12 release soak. Phases 0 to 12 proved the base: a
deterministic valley with z-levels, people with memory and lineage, customs with
recorded origins, caravans, honest history, a hundred-person settlement that runs for thirty
game-years unattended. What is missing against the two reference games is content depth
and visual richness, and this plan adds it in the same way the base was built: one phase
at a time, each with gate tests, a glance review from the integrated simulation, a matrix
update and a verified checkpoint. Nothing here is marked done until it is on screen in an
unscripted run.

## The gap, named

| Area | Dwarf Fortress (Steam) | RimWorld | Ostrog at Phase 12 |
|---|---|---|---|
| Items, materials | hundreds of materials, properties, quality, named artifacts | hundreds of items, quality, apparel layers | 14 item kinds, 1 quality number |
| Work | ~50 labours, workshops, recipes, stockpile rules | ~20 work types, bills | 13 orders, 1 workbench, no recipes |
| Health | body parts, wounds, infection, syndromes, rest | body parts, diseases, prosthetics | injured_until timer only |
| Mind | needs, thoughts, stress, tantrum spiral, personality facets | mood, mental breaks, traits | needs, traits, memory bias, mood number |
| World | fluids, temperature, weather, seasons, cave-ins, caverns, magma, geology | temperature, weather, fire, zones | seasons, weather sprite, cave-ins, strata |
| Society | civilisations, nobility, justice, diplomacy, sieges, legends | factions, raids, trade, ideology | communities, norm, trade, splits, 4 customs |
| Beliefs | religions, spheres, temples | ideoligion precepts, rituals | customs from events |
| History | Legends mode, thousands of figures, artifacts, wars | event log, art describing events | chronicle, eras, custom origins |
| Art | 32 px tiles, 100+ creatures, apparel, animations | hand-drawn, layered apparel, animations, weather | 138 generated tiles, no animation |
| Scale | 200 dwarves, 300 creatures, 7 z-levels common | ~20 colonists | 110 people on 128² |

## Phases 13 to 25

Order is chosen so each phase is visible on the second monitor, and so that systems depend
only on earlier ones. Gates are automated tests over the integrated simulation; "review" is
the glance review with screenshots and footage; "art" means hand-judged pixel art, whether
generated, sourced under licence or pixelled by hand, judged against docs/ART_BIBLE.md.

| # | Phase | Scope | Gates and review |
|---|---|---|---|
| 13 | Life on screen | People as layered sprites: body, hair, clothing by season and role, carried item, age stage, injury state; two-frame idle and work animation; door, wall top, roof and floor variety; trees sway; rain and snow; ground palette per season; wall and water shading on peels | review of 6 scenes at 1× and 2× against RimWorld reference shots; `sprite_layers_cover_all_states`; art bible revised from the result |
| 14 | Materials and workshops | Stone kinds (granite, limestone, slate…), wood kinds, copper and iron from ore via a smelter, quality tiers 0–5 that show in the sprite and in use; carpenter, mason, smith, loom, kitchen workshops with recipes and bills the planner posts from need; tools and clothes wear out and are replaced | `recipe_chain_ore_to_tool`, `quality_affects_use`, `worn_out_replaced`; review of a workshop row at work |
| 15 | Bodies and health | Body parts with wounds (cuts, bruises, breaks), bleeding, infection chance, bed rest and herb poultices, scars that show, chronic ailments with age, death causes from the body model; a healer role; illness that spreads in close quarters in winter | `wound_heals_with_rest`, `infection_can_kill`, `illness_spreads_then_fades`; review of the sick-house |
| 16 | Minds under strain | Thoughts with weight and decay (DF style) feeding mood; stress; breaks: tantrum (breaks furniture), withdrawal (stops working), wandering off, a fight; grief and joy events that others remember; personality facets from genome | `tantrum_from_stacked_thoughts`, `grief_changes_a_week`, `break_is_remembered_by_witnesses`; review of a bad week |
| 17 | Water, fire, cold | Streams flow downhill and fill lakes (cellular, bounded), wells and buckets, mud and flooding; fire from hearths and lightning spreads over grass and wood, burns buildings, is fought with water; cold exposure needs clothing and shelter, frostbite; heatwaves | `flood_fills_basin`, `fire_spreads_and_is_put_out`, `cold_without_clothes_hurts`; review of a fire |
| 17b | Lived lives | Survivors arrive with pasts generated year by year and recorded as history before tick 0; beliefs with sources weight choices without deciding them; techniques learned by trying and watching replace crisis orders; the missing seven are away agents with real fates; rumours are memories (docs/design-organic.md) | `past_is_recorded_history`, `siblings_diverge`, `past_shapes_but_does_not_decide`, `fire_fought_without_orders`, `missing_survivor_dies_of_a_cause`, `rumour_needs_a_teller`; review of one survivor's history panel and a fire nobody ordered fought |
| 17c | A world with a past | Before the portal opens, the valley and the land around it are simulated for decades to centuries at a coarse grain (sites and named figures stepped by season), as DF world generation does: peoples settle, grow, trade, quarrel, migrate and die out. What remains is real and traceable: ruins, graves, roads, abandoned mines, objects left where someone dropped them or died, named places, and living neighbours with memories, beliefs and lore. Real history and taught history stay separate, so lore drifts from the truth with a teller for every version. Pulls the cores of Neighbours (19) and Legends (23) forward; first candidate for reuse is the prototype's deep-time tier. The missing seven land among these peoples, and word of them comes from locals who met them (docs/design-organic.md sections 7 and 8) | `history_only_from_simulation` (every ruin, grave, found object and legend traces to rows), `lore_drifts_with_tellers`, `found_object_has_provenance`, `neighbours_remember`, `history_pass_budget` (300 years in under a minute, measured); review of a ruin found and the History panel answering why it is there |
| 17d | The wizard | A new-game wizard grown from the test agent's arrival screen, in the spirit of DF embark and RimWorld scenario setup. Deity: the player picks one deity who watches over the world (the storyteller; the Engine renamed and given a name, face and temperament); each deity has motives and triggers (wealth and idleness, a killing, neglect of the dead, a long quiet, a famine) and answers them by leaning the world's own mechanisms at the cause, recorded with the trigger, never by firing an event directly; it watches every community on and off the map, including through the history pass, so it is chosen before history is generated. History: how long the world beyond the portal is simulated (17c). World: seed, valley size, relief, water, caves, forest, winter, rainfall, starting season, life pace; preview map; landing spot picked on it. Party: the scenario sets how many of the ten come through together (three by default); cards show each past (17b); reroll; choose. Gear: a carrying budget spent on what came through with them. Ideology: shared beliefs seeded with their source in the old village, weighting choices and free to drift. Presets fill every page and stay editable. Every choice is stored in the save and shown in History | `deity_answers_its_triggers`, `deities_measurably_differ` (each deity moves its own measures, the check the Engine modes failed in review), `same_choices_same_world`, `wizard_choices_travel_with_save`, `preview_matches_generated`, `ideology_is_beliefs_not_rules`, `gear_budget_holds`; review of every page at 1080p and a harsh deity against a gentle one |
| 18 | The deep | Three strata of named rock, gem clusters, caverns with fungus forests, underground water, cave creatures (blind things, bats, a burrower), deep mining shafts with supports and real cave-in propagation, lamps and darkness as a need | `cavern_discovered_and_settled`, `cave_in_propagates`, `creature_of_the_deep_fought`; review two levels down |
| 19 | Neighbours | A second community founded on the same valley by default (or by a split), borders that matter, raids when hungry, feuds from deaths, alliances from marriages, refugees after a fire or famine, negotiated trade with value | `raid_has_cause_and_memory`, `alliance_from_marriage`, `refugees_taken_in_by_norm`; review of two settlements |
| 20 | Fighting | Weapons and shields from the smith, hunting as fighting, wounds from Phase 15, deaths with weapon and killer recorded, grudges and revenge, justice customs (exile, restitution) traceable to the first killing | `fight_wounds_both`, `revenge_traces_to_grudge`, `exile_custom_from_killing`; review of a feud |
| 21 | Gods and festivals | Spirits invented from unexplained events (a comet, a plague, a cave-in survival), shrines built, offerings, festivals on the calendar, taboo places, teaching to children with drift across generations, schisms | `spirit_invented_from_event`, `festival_held_yearly`, `belief_drifts_across_generations`; review of a festival |
| 22 | Masterworks | Named items with history (who made it, from what, what it saw), masterwork chance by skill, heirlooms passed on, art objects depicting recorded events, theft as a raid goal | `masterwork_named_and_tracked`, `heirloom_inherited`, `art_depicts_real_event`; review of an heirloom's record |
| 23 | Legends | Overworld sites become real communities simulated at low rate with D10 continuity (named people, births, deaths, wars between sites), migrations into the valley, a Legends browser (figures, places, items, events, lineages) with "why" links back to rows | `offsite_people_keep_identity`, `legends_only_from_rows`; review of the Legends browser |
| 24 | Three hundred | 256×256 valley, 300 people and 100 animals in budget (perception and planning striped and bounded, LOD per D10 only if measured), 24 h soak at 10× on four seeds, memory curve | all DoD performance targets measured and met or revised in writing |
| 25 | Ship | Windows launch recorded, hand review on the second monitor recorded, onboarding on a clean user, release-day review of every record, Phase 12 gate (issue 16) and DoD matrix with no row incomplete | matrix fully evidenced |

## Rules carried forward

- Every phase ends with a glance review from an unscripted run; staged scenes are art
  evidence only (D13).
- Nothing in the simulation calls a model; history is recorded, never invented (D9);
  `chronicle_only_real_events` runs in CI and must keep passing as event kinds grow.
- Off-map and off-screen people keep their identity (D10, D15).
- Saves migrate (D8, D14): every phase that changes world data adds a migration step and a
  fixture.
- Art is judged by eye against docs/ART_BIBLE.md and reference shots; generators are a tool,
  not the standard.
- The soak (`ostrog soak`) runs at the end of every phase from 14 on; a regression in
  stalls, drift, determinism or tick cost blocks the checkpoint. Famine and collapse are
  reported, not blocked: losing is fun (docs/design-organic.md).
- Each phase updates the matrix honestly. "Partial" with evidence is the normal state;
  "done" needs behaviour, integration, presentation and verification.

## Sizing

Phases 0 to 12 took one long session with the base architecture in place. Each phase
above is at least that size; some (15, 17, 19, 23) are larger. This is a programme of many
sessions. The checkpoint discipline means any session can stop anywhere and the next picks
up from STATUS.md.
