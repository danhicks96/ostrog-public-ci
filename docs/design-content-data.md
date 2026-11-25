# Content as data and composable parts: options for DEC-5

Status: proposal for the maintainer, written 2026-11-08 at c169 (WP-05 slice 5.7). Nothing here is implemented.
Issues: #269 (content is data-driven), #337 (build content from composable parts). Related: #267,
#280, #284, #328, #333, M9 (#380) and the renderer art slices that wait on this (WP-06 6.6).

## What is being decided

DEC-5 carries a default: the dev writes an options document with a recommendation and proceeds. Most
of this is an engineering choice, so the recommendation below can start without waiting. The maintainer is
asked only about how it changes what the game feels like (the two questions at the end).

## Facts at c169 (measured, commands at the bottom)

| Thing | Count | Where it lives |
|---|---|---|
| Data files today | 24 (12 species, 12 body plans) | `data/peoples/species/`, `data/peoples/plans/` |
| `ItemKind` variants | 49 | `crates/hf-sim/src/items.rs` |
| `PlantKind` variants | 44 | `crates/hf-sim/src/plants.rs` |
| Material constants | 75 | `crates/hf-sim/src/world/material.rs` |
| `ThoughtKind` variants | 52 | `crates/hf-sim/src/mind/thoughts.rs` |
| `EventKind` variants | 262 | `crates/hf-sim/src/events.rs` |
| `ConditionKind` / `WoundKind` | 5 / 6 | `conditions.rs`, `body.rs` |

Issue #269 says 21 data files; plans have since grown from 9 to 12, so it is 24 now.

The proven pattern is in `species.rs` and `plan.rs`: each TOML file is pulled in with `include_str!`,
parsed by `parse(name, text)`, checked by a `validate` function, and held in a `OnceLock` library.
It is compiled into the single binary, so there is no runtime file loading and no wall clock (D7).

## The constraints any option must respect

1. **Saves store enum variants by position.** `tools/enum_order.py` guards this: each enum at the
   last tag must be an unchanged prefix of the same enum now. Data-defined content cannot take its
   save identity from file order or from the order entries happen to load. It needs a stable id.
2. **Determinism (D7, #234).** All randomness through `rng::stream`, no `HashMap` iteration in the
   simulation. A content registry must iterate in a fixed order and feed the world digest the same
   way on every machine.
3. **Single binary, no runtime mods.** Reading files at run time would risk digest divergence.
4. **Digest stability when converting.** Moving an existing domain from an enum to data must not
   change any behaviour. The proof is identical digests on the four-seed 600-day soak.

## Options for #269 (content as data)

**A. Parameters in data, kinds stay enums.** Keep `ItemKind` and the rest; move only numbers and
text into TOML keyed by variant name. Smallest and safest. It does not let anyone add a new thing
without editing Rust, so it does not deliver the variety goal. Useful as a first step only.

**B. Entries defined in data, with stable string ids and an append-only order manifest.** Each entry
has an `id` string. A manifest (`data/<domain>/_order.toml`) lists ids in the order they were first
added, seeded from today's enum order so existing saves keep their meaning. The loader assigns each
entry its index from the manifest, never from the file system. A CI check, modelled on
`tools/enum_order.py`, fails if an id is removed, renamed or reordered. Loading still uses
`include_str!` and a `OnceLock` library, with validation like `species::validate`.

**C. Runtime-loaded external files.** Rejected: conflicts with constraints 2 and 3.

## Options for #337 (composable parts)

The issue's own measurement found no composable system in `hf-sim` (zero `CompProperties`-style
patterns), and that appearance is already composed by the renderer from layered sheets
(`agents.rs`). So the open question is which axis gets composition first.

**1. Do not compose; stay with flat kinds plus data (option B alone).** Cheapest. New content is
numbers on a fixed set of behaviours, so variety comes only from re-parameterising.

**2. Compose conditions and wounds first.** Express a condition as a short list of parts (changes a
need, ends on its own, can be discovered, damages a body part, causes a mental state, spreads). The
domain is small today (5 conditions, 6 wound kinds), so the part set can be designed and tested
cheaply, and it is where RimWorld's depth comes from.

**3. Compose items first** (the suggestion on #337). `ItemKind` has 49 kinds and `legend.rs` already
gives items a story and quality, so it is a natural place. It is a larger domain with more save
surface.

## Recommendation

Take B for #269 and option 2 for #337, in this order, each its own slice with its own soak:

1. **Registry slice.** Build the id plus order-manifest registry and its CI guard, and convert one
   low-risk domain with no behaviour change. Pick `ThoughtKind` (52 entries; #280 wants far more mood
   inputs and each one is already just a `shape()` of (weight, days) plus a `label()`, so it is numbers and text). Gate: identical four-seed digests before and after.
2. **Parts slice on conditions and wounds.** Define the part set, express the existing 5 conditions
   and 6 wound kinds with it, same digest gate, then add one new condition made only of existing
   parts as the proof.
3. **Materials, plants, items** follow the same recipe, one per slice. Materials last, because world
   generation touches the digest most directly and #267 depends on it.
4. M9 content lists (WP-08 8.3) are sequenced after step 2 proves the parts work.

Why not items first: the larger save surface makes it the wrong place to learn the pattern. Why not
all domains at once: each conversion needs its own digest proof and soak, and merges are serial.

## Questions for the maintainer (feel, not engineering)

1. **Assembled variety.** Content built from parts can be produced in large numbers, but an
   assembled entry has no hand-written character unless someone gives it a name, a line and a
   story. Is it acceptable for the breadth milestone (M9) to ship a mix of authored and assembled
   entries, with authored ones reserved for what players meet most?
2. **Frozen ids.** Once an id has been in a release, a save may depend on it, so it can be retired
   but never renamed or reused. Are you comfortable with that rule for content, including names you
   may later want to change?

If the maintainer does not answer, the default stands: proceed with the recommendation, treating the first
answer as "mixed authored and assembled" and the second as "yes".

## Not in scope

No code, data or enum changes in this slice. Behaviour trees (#333), the renderer parts system
(WP-06), and the `EventKind` enum (262 variants, a separate and larger conversion) are not covered.

## How the numbers above were measured

```text
find data -type f | sed 's#/[^/]*$##' | sort | uniq -c        # data files per folder
grep -n 'include_str!' -r crates/hf-sim/src                    # pattern in species.rs and plan.rs
# variant counts: a small script that counts top-level variants of each `pub enum` listed
# (same parsing approach as tools/enum_order.py)
grep -c 'pub const [A-Z_0-9]*: Material' crates/hf-sim/src/world/material.rs
```
