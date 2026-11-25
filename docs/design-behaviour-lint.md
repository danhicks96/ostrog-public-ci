# Behaviour lint: a canary for "the world is alive"

Unit tests prove a mechanism works. None of them can tell you that the colony went quiet after the
first house — which is exactly the defect the build server already recorded in STATUS.md and the thing you
see when you watch it. The per-day event histogram that exposed it should be a permanent gate: a
**behaviour lint** that runs the simulation over a fixed window, on fixed seeds, and asserts that
the world stayed alive, purposeful and legible.

This is a design document. The gate described here does not exist yet.

## What it asserts (thresholds provisional, from today's measurements)

Baseline: seed 1, 8 founders, 10 days (288,000 ticks). Productive events =
`built + hauled + mined + harvested + cooked`.

| Gate | Rule | Today (seed 1) | Verdict |
|---|---|---|---|
| Quiet days | no run of ≥ 2 consecutive days with 0 productive events | days 2–3 and 7–9 have **0** | **fails** |
| Burstiness | no single day holds more than 25 % of the window's productive events | days 4–5 hold 89 of 172 = **52 %** | **fails** |
| Legible reasons | `chose` events ≥ 2 per agent-day | 44 over 10 days ≈ **0.4/agent-day** | **fails** |
| Plan churn | `plan_failed` ≤ 2 × productive events per day | 326 vs 301 overall; day 7: 71 vs 0 | **fails** |
| Not starving | `starving` bounded (provisional: ≤ 1 per agent-day) | 49 over 60 days earlier | passes |
| Determinism | digest stable for (seed, ticks) | existing gate | passes |
| Tick budget | ≤ 2 ms mean at 300 inhabitants | existing target | unmeasured |

The first four failing today are precisely the complaint: work arrives in a two-day burst, then
nothing happens for days, and almost nothing says why. That makes this the right canary — it fails
on the symptom a viewer reports, not on an implementation detail.

## Where it lives

One function, two callers (DRY):

- `crates/hf-sim/src/diag.rs` — `pub fn day_histogram(seed: u64, days: u32) -> Vec<DayRow>` building
  per-day counts by stepping the world day by day and tallying `Event::kind_name()` from
  `world.last_events`, exactly as `crates/hf-cli/src/main.rs` builds its `tally` map.
- `crates/hf-sim/tests/behaviour_lint.rs` — the CI gate, calling that function over seeds 1–4.
- `crates/hf-cli` — `ostrog lint --seeds 1-4 --days 10` printing the same table for humans
  (the diagnostic form of the same assertion).

Sketch of the assertion (illustrative; API per `diag.rs` above):

```rust
#[test]
fn the_world_stays_alive() {
    for seed in 1..=4 {
        let days = diag::day_histogram(seed, 10);
        let quiet: usize = days.windows(2)
            .filter(|w| w.iter().all(|d| d.productive == 0))
            .count();
        assert_eq!(quiet, 0, "seed {seed}: {quiet} pairs of silent days");
        let total: usize = days.iter().map(|d| d.productive).sum();
        let worst = days.iter().map(|d| d.productive).max().unwrap_or(0);
        assert!(worst * 100 <= total * 25, "seed {seed}: {worst} of {total} in one day");
    }
}
```

## How it stays honest

- **Thresholds are provisional and written down here.** Changing one is a written change with the
  measurement that justified it, in STATUS.md — the same rule DEFINITION_OF_DONE.md already applies
  to performance targets. Quietly relaxing a gate is the failure mode this is meant to prevent.
- **The per-day table is a golden.** Commit it beside the review
  (`docs/reviews/soak-<date>/perday.csv`); a shape change requires an ITERATION_LOG entry, the same
  convention the project already uses for digest and golden changes.
- **Tolerance bands, not exact numbers.** Different seeds legitimately differ (today: 29–50 alive
  after 60 days across seeds), so gates assert distributions and floors, never equality.
- **Cost**: 10 days × 4 seeds ≈ 70 s of simulation here (a 10-day run is ~17 s). Acceptable in CI;
  if it grows, run 4 seeds only on the checkpoint job and 1 seed per commit.
- **It is not a substitute for unit tests.** Mechanisms get unit tests; this asserts the experience.

## Anti-patterns to avoid

- Over-fitting the thresholds to today's numbers so tightly that legitimate improvements fail.
- Using it to assert "busy" without "meaningful": a world can be busy and incoherent, which is why
  the legibility gate (`chose`) sits alongside it.
- Asserting anything not derived from recorded state — this lint must never read like a script.

## Your 100 edge cases plug in here

This is the right home for them. Each edge case becomes a **named seeded scenario with an
invariant**, so a list of 100 becomes 100 executable checks rather than a document nobody reads:

| Edge case | Scenario | Invariant |
|---|---|---|
| Everyone is a coward | seed a colony with all-cowardly trait mixes | expeditions ≈ 0, but knowledge still arrives via visitors; no crash, no starvation spiral |
| Only knower dies | kill the single technique holder mid-run | technique either retained (scriptorium/taught) or visibly regressed; never silently present |
| Raid takes every tool | remove all tools | capability collapse is bounded; repair/trade/reverse-engineer path exists |
| Tribute payer collapses | destroy the payer | raider switches to trade or banditry; world does not dead-end |
| False rumour | seed a false reputation | corrected by direct observation within N days |
| Specialist exiled | exile the knower before teaching | permanent loss is recorded, not silently avoided |

## Current-direction canaries (E16 and local physical depth)

These are design canaries for the next implementation slices. They are not claims that the
mechanisms already exist:

| Edge case | Scenario | Invariant |
|---|---|---|
| Undercurrent | a swimmer enters a current | movement follows liquid/support state; rescue, exhaustion or drowning is possible and recorded |
| Flooded camp | a rising body reaches homes | routes and shelter choices change; nobody walks on unsupported liquid |
| Depleted harvest | a colony strips a berry patch and woodland | regrowth and individual memories change later choices |
| Open spoilage | equal food is left outside and inside | preservation follows environment/container/technique, not a magic indoor label |
| Porous wall | a dig opens near a water-bearing wall | seepage can precede breach; observers may notice or miss it |
| False lesson | a person misreads an accident | the mistaken belief belongs to that person and can later be corrected by evidence |
| Withheld warning | a rival knows a hazard | sharing, distortion or silence follows the relationship and is recorded |
| Hidden change | an eligible person undergoes a rare transformation | setup does not reveal it; onset and cause appear only in play |

Send the list when it's ready and I'll wire them in as scenarios under this same harness — the ones
that describe behaviour go in `tests/behaviour_lint.rs`, and the ones that describe a mechanism go
in the relevant `tests/*.rs` where they belong.
