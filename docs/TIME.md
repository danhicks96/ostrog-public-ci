# Time model

The tick is the only simulation unit. Every other quantity is derived from it and every
system quotes this table. Nothing in the simulation reads the wall clock.

| Quantity | Value | At 1× (real time) |
|---|---|---|
| 1 tick | 3 game-seconds | 50 ms (20 ticks per real second) |
| 1 game-minute | 20 ticks | 1 s |
| 1 game-hour | 1,200 ticks | 1 min |
| 1 game-day | 28,800 ticks (24 h) | 24 min |
| 1 season | 5 days = 144,000 ticks | 2 h |
| 1 year | 4 seasons = 20 days = 576,000 ticks | 8 h |

## Speeds

| Setting | Ticks per real second | A day takes |
|---|---|---|
| Pause | 0 | — |
| 1× (default) | 20 | 24 min |
| 3× ("workday") | 60 | 8 min |
| 10× | 200 | 2.4 min |
| Fast | uncapped; HUD shows achieved rate | as fast as the build server allows |

The client interpolates entity positions between ticks so 1× looks smooth at 60 fps.

## Movement

| Mover | Ticks per tile | Tiles per real second at 1× |
|---|---|---|
| Walking inhabitant | 4 | 5 |
| Carrying heavy load | 6 | 3.3 |
| Running (danger) | 3 | 6.7 |
| Stairs/ladder level change | 8 | — |
| Small animal | 3 | 6.7 |

Crossing a 40-tile settlement takes 8 real seconds. A 256-tile map edge-to-edge takes about a
real minute.

Ground underfoot (#268, WP-05 5.6, `nav::step::rough`): extra ticks per tile on top of the mover's
own, on the ground under the tile stepped onto. A worn path (packed earth) is still 2 ticks quicker.

| Ground underfoot | Extra ticks per tile | Walking, ticks per tile |
|---|---|---|
| Scree | +3 | 7 |
| Mud | +2 | 6 |
| Gravel, sand | +1 | 5 |
| Anything else | 0 | 4 |

The table reads the tile's stored material. The weather's surface cover does not change it:
rain lays a wet-soil look over soil, clay and wet soil, and a hard frost freezes them to
frozen soil (`cover::cover_at`, #289's overlay, DEC-8 items 2+3), but `step_cost` never reads
the cover, so rain-wet ground is no slower to cross than dry. Real `Mud` above is the terrain
material, found in the world, not the rain look. Whether rain or frost underfoot should slow a
walker is a feel question left open for the maintainer (WP-05 5.5); D90 records the current call.

## Needs and daily rhythm

| Need | Rate | Meaning |
|---|---|---|
| Hunger | full → empty in ~6 game-hours of activity | about four meals a day, each a visible event |
| Rest | ~16 h awake, ~8 h asleep | night is an 8-real-minute lull at 1× |
| Warmth | driven by season, shelter, fire | |
| Safety | event-driven | |
| Social | decays over a day | |
| Comfort / purpose | slow, hours to days | |

## Work durations (base, before skill)

| Job | Game time | Real time at 1× |
|---|---|---|
| Carry one load across the settlement | ~2 min | ~8 s walking |
| Build one wall segment | ~20 min | 20 s |
| Mine one rock tile | ~15 min | 15 s |
| Cook one meal | ~15 min | 15 s |
| Craft a tool | ~2 h | 2 min |
| Plant/harvest one plot | ~5 min | 5 s |
| Build a small house (10 walls, door, roof, bed) | ~6 h with hauling | ~6 min |

## Growth and aging

| Stage | Age | Real time at 1× / 3× |
|---|---|---|
| Crop growth | 3–8 days | 1.2–3.2 h / 24–64 min |
| Child → adult | 15 years | 5 days / 1.7 days |
| Typical lifespan | ~60 years | 20 days / 7 days |
| Generation turnover | ~25 years | 8 days / 2.7 days |

Founders remain recognisable for weeks of ambient running; generational change happens over a
month at the default pace and within a week at 3×.

## Provisional CPU budgets (derived, revised by measurement in STATUS.md)

| Viewing mode | Tick budget at 300 inhabitants |
|---|---|
| 1× smooth | ≤ 50 ms per tick |
| 3× usable | ≤ 16 ms per tick |
| 10× | mean ≤ 2 ms, p99 ≤ 10 ms (400 ms CPU per real second, leaving headroom for rendering) |
| Planner | ≤ 32 plans per tick, amortised ≤ 50 µs each |

The experience requirements (1× smooth, 3× usable, 10× available) are fixed; the numbers are
measurements to be recorded, not assumptions to be defended.
