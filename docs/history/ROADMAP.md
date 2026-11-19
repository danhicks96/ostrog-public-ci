# Roadmap

Route to the release contract. Each phase ends runnable, glance-reviewed (from Phase 3),
matrix-updated and checkpointed. "Advances" never means "completes". Issues on GitHub hold the
actionable detail; one milestone per phase.

| # | Phase | Scope | Gates | Advances | State |
|---|---|---|---|---|---|
| 0 | Contract + repo | private repo, active checkout, checkpoint + restore test, records, issues | checkpoint verification + restore pass | 13 | done 2026-10-13 |
| 1 | Foundations + representative scene | 3-crate workspace, ported rng/pipeline/chronicle/snapshot/digest, chunked z-grid + worldgen, protocol + record, save v1 with manifest + migration scaffold, gdext hello (Linux, Windows), palette, staged representative scene reviewed, art bible | digest_deterministic, save_roundtrip_identical, save_manifest_mismatch_rejected, windows hello builds, scene review | 1, 8, 10 | gates pass 2026-10-13; wall-top art owed (ART_BIBLE) |
| 2 | Navigation + movement | 3D A*, regions/portals incl. vertical, reach(), incremental rebuild, movement interpolated at 1× | vertical_path_via_stairs, unreachable_O1, region_rebuild_after_wall, throughput measured | 5 | gates pass 2026-10-13 (0.15 ms/path) |
| 3 | Living inhabitants + first glance | needs/traits/skills/perception/knowledge, eat/sleep/forage/wander, sprite composition, director v1, feed, away-summary v1, inspect, tick time measured at 1×/10× | need_satisfaction_seeded, knowledge_only_targets, first glance review | 2, 6, 7, 10 | gates pass 2026-10-13; glance review done: NOT acceptable yet (empty world) |
| 4 | Work + settlement | planner, leases, recovery, designations, haul/build/mine/craft/cook/farm, stockpiles, recipes, ledger | prereq_chain, contention_no_deadlock, lease_expiry_sweep, interrupted_plan_resumes, target_vanishes_recovers, ledger_zero_drift, budgets re-measured, glance review | 2, 4 | gates pass 2026-10-13; glance review: settlement builds, goes quiet after the house |
| 5 | Memory + social | consolidation, memory bias, relationships, mood, socialize, feed/history polish | memory_changes_choice, memory_cap_bounded, relationship_from_memories, glance review | 3 | gates pass 2026-10-14; cause of memory-driven choices not yet visible in the feed |
| 6 | Persistence + viewer | .hfsave ring + migrations, autosave, speeds, z-nav, follow, bookmarks, settings, audio, sim-while-unfocused | restore_equals_continuous, migration_chain_loads_v1_fixture, interrupted_write_recovers, autosave hitch measured, panel shots | 7, 8 | gates pass 2026-10-14; capture 0.9 ms; controls exercised headlessly only |
| 7 | Communities + economy | roles, shared tasks, cohesion, newcomers, splits, trade | two_communities_trade, split_has_cause_event, newcomer_by_norm, glance review | 4, 5 | gates pass 2026-10-14; default worlds grow by arrivals; splits only under hostility (no quarrels yet) |
| 8 | Generations + culture | genetics, births, lineage, aging/death, beliefs/customs with hooks, taught history | genome_vs_learned, three_generations, custom_origin_traceable, custom_changes_behavior | 3, 5 | gates pass 2026-10-14 |
| 9 | Depth + discovery | strata/caves/ores, excavation, underground construction, discovery, lighting; LOD only if measured need, with D10 continuity tests | dig_reveals_cave_event, underground_cellar_in_use, lod_continuity_same_people (if applicable), z shots reviewed | 5, 10 | gates pass 2026-10-14; wildlife/hunting/quarrels/cellar/cave-ins in; no LOD needed yet |
| 10 | Expeditions | overworld, caravans, travel, encounters, persistent outcomes | caravan_returns_with_outcome, caravan_lost_leaves_history, expedition_memories_consolidate | 5 | |
| 11 | History + director maturity | era summaries, never-invents audit, discovery-respecting mode, director tuning from footage | chronicle_only_real_events, director footage review, feed rate check | 6, 7 | |
| 12 | Hardening + ship | soak 24 h × 4 seeds, perf per speed, onboarding, error handling, Linux + Windows exports, docs, asset records | all targets met, fresh-launch review on both exports, matrix fully evidenced | 1, 9, 11, 12, 13 | |

## Dependencies

1 → 2 → 3 → 4 → 5; 6 after 4 (save must cover plans and leases); 7 after 5; 8 after 7;
9 after 4 (needs mine/build); 10 after 7 and 9; 11 after 8 and 10; 12 last.
Phases 6 and 9 can overlap with 5 and 7.

## Part II: parity programme (Phases 13 to 25)

See docs/history/PLAN-PARITY.md for the gap table, the thirteen phases and their gates. Written
2026-10-15; Phase 13 (life on screen) starts first.

## Current priority

The active execution order is `docs/history/PLAN-NEXT.md`. Finish the remaining M5 evidence, then M6
essentials, the mouse-first UI (#413), and the graphics/object overhaul (#414). The next design
layer is E16, situated minds and problem solving, which crosses M6 physical depth, E8 knowledge,
E12 motive and M7 presentation. Observation uses a full initial snapshot followed by deltas so
the world remains detailed without giving agents omniscience. The object breadth and inspection
direction is governed by D57 and `docs/OBJECT_CATALOG.md`; the current art quality and PR gate are
governed by D58, D63, D64 and `docs/ART_BIBLE.md`/`docs/ART_SPEC.md`.

The older parity label for this work is Phase 17: water, fire, cold (streams that flow and flood,
wells and buckets, fire that spreads and is fought, cold that needs clothing and shelter,
heatwaves). It remains useful as history, but it is not the current ordering authority.

Phase 16 gate passed 2026-10-15 (tantrum_from_stacked_thoughts, grief_changes_a_week, break_is_remembered_by_witnesses; review docs/reviews/2026-10-15-phase16-strain-review.md).

Phase 15 gate passed 2026-10-15 (wound_heals_with_rest, infection_can_kill, illness_spreads_then_fades; review docs/reviews/2026-10-15-phase15-health-review.md).

Phase 14 gate passed 2026-10-15 (recipe_chain_ore_to_tool, quality_affects_use, worn_out_replaced; review docs/reviews/2026-10-15-phase14-materials-review.md). Mason and wood kinds carried as owed.

Phase 13 gate passed 2026-10-15 (sprite_layers_cover_all_states; review docs/reviews/2026-10-15-phase13-life-on-screen-review.md). Roofs from above and drawn winter canopies carried as owed art. Phase 12's open items (release-blockers 18 and 19, the 300-inhabitant workload) move into Phases 24 and 25 of the parity programme.

Phase 12 (hardening): Done: soak/profile/director commands, five soak passes with economy fixes (docs/soak/README.md), planner and nav hitch fixes, packages for Linux and Windows with checksums, README, recovery path demonstrated, phase gate issues closed. Open: release-blockers 18 and 19 (Windows launch, hand exercise), release-day record review, 300-inhabitant workload.

Phase 11 gates passed 2026-10-14 (chronicle_only_real_events, history_view_reads_the_record; director footage review docs/reviews/2026-10-14-phase11-history-director-review.md).

Phase 10 gates passed 2026-10-14 (caravan_returns_with_outcome, caravan_lost_leaves_history, expedition_memories_consolidate; review docs/reviews/2026-10-14-phase10-expeditions-review.md). Relocation and seeking-a-relative purposes are not built; see the review.

The Windows agent (branch `the test agent`, D15) runs the same phase in parallel: 10a overworld + knowledge, 10b caravans + outcomes, 10c lost/relocated/stranded + custom hooks, 10d client overworld view. See STATUS.md.

## Deferrals from the release contract

None. (Any deferral is recorded here explicitly with the reason and does not count as done.)

## Blockers

None recorded. (Repository sync, toolchain or hardware blockers go here with exact detail.)
