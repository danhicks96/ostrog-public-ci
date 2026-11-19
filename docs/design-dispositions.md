# Dimensional dispositions and surface expression

Status: first implementation slice for #488, 2026-10-31. This is a bounded person-model slice,
not a complete psychology model and not a clinical or diagnostic system.

## Model and assumptions

A new person has eight persistent core values in the inclusive range 0..1. The values come from
the existing genome without consuming another random-stream draw. Four use the temperament loci
already present in saves; the other four use a deterministic mix of the whole genome. A child's
profile therefore comes from the same inherited genome as their existing traits. There is no new
random source and no wall-clock input.

| Dimension | Psychological idea represented | Current behavioral or expressive effect |
|---|---|---|
| sociability | desired frequency of company | social-need decay and talk/gather weight |
| industry | persistence in purposeful work | work weight; fatigue and pain soften its current expression |
| curiosity | interest in exploration | wander weight; stress softens its current expression |
| caution | sensitivity to possible harm | wander weight; stress and pain bring it forward |
| warmth | affiliative expression toward others | talk/gather weight and surface expression |
| cheerfulness | positive versus subdued outward manner | surface expression, interacted with current mood, pain and stress |
| order | preference for putting things in place | hauling fit and surface expression |
| independence | self-directed versus communal outward manner | surface expression only in this slice |

These are simple weighted dimensions, not claims about the structure of human personality. The
pairs of surface words are descriptive endpoints, not good/bad scores. Values in the middle are
not labeled. The current slice deliberately does not infer motives, morality, diagnoses, or a
single worth or happiness value from the profile.

## Core, acute state, and learning are different

`Agent::dispositions` stores only the core profile. `mind/dispositions.rs` computes a temporary
expressed profile on demand from that core plus rest, mood, stress, wound/illness severity, and
the dimension-specific rules in the table. It never writes the result back to the core. Rest or
recovery therefore restores the earlier expression without erasing the person's baseline.

Needs, wounds, illness, feelings, stress, memories, beliefs, skills, relationships, and lessons
remain in their existing models. They are not copied into the disposition profile. Existing
binary traits remain for save compatibility and for mechanics that this slice does not migrate.
An old save with no dimensional profile keeps its legacy trait weights and shows its existing
trait list, still capped to five entries.

Long-term change to a core value is not implemented. When it is added, it needs a recorded cause
and must not be confused with the acute expression calculated here.

## Surface presentation

Setup cards and person inspection call the same surface-trait calculation. They receive no core
values or full internal profile. Candidates below the salience threshold are omitted; the rest
are ordered by salience with a deterministic label tie-break and capped at five. Acute strain can
also bring forward `irritable` or `exhausted`. These are current expressions, not new permanent
traits and not hidden violent tendencies.

There is no word-cloud rendering in this slice. If one is added, it must consume this same bounded
surface list rather than reading `Agent::dispositions` or the legacy full trait list.

## Guards and scope boundary

`crates/hf-sim/tests/dispositions.rs` builds explicit person scenarios rather than relying on a
lucky seed. It guards deterministic bounded genome derivation, temporary surface change and
recovery without core mutation, salience order and the five-label cap. Paired `decide::step`
scenarios require relevant axes to change the committed talk, work, wander and haul/chop outcomes,
and unrelated axes to leave those outcomes identical. A real pre-field v2 fixture checks legacy
fallback, and a snapshot-restored world must match the continuous world's digest after further
ticks. `tools/gd_test.gd` also checks the public setup/inspection boundary.

This slice does not implement observation-driven appraisal, decision traces, durable learned
exceptions, long-term core development, afflictions, or crisis outcomes. Those remain required
to close #488. Additional dimensions should be added only with a specific notice, choice,
learning, memory, or expression effect; this list is not a mandate to accumulate unused labels.
Focused guards and integrated captures do not establish population-scale performance or long-run
behavior; box-ci, integrated review and the required simulation soak remain separate gates.

## Where each disposition is read (#1520)

Written for #1520 at origin/master 59972f57 (2026-11-14). It maps each of the eight dispositions to
every function in `crates/hf-sim/src` that reads its value, found with the acceptance grep of #1520
(`grep -rn "sociability\|industry\|curiosity\|caution\|cheerfulness\|independence\|d\.order\|\.warmth"
crates/hf-sim/src`) plus a sweep for `.order`, which that grep cannot see except as `d.order`
(`self.order` and `current.order` in `mind/dispositions.rs` are reads too). A "read" is a use of the
field of a `Dispositions` value; `needs.warmth` (the warmth need) and the climate `warmth` of world
generation are other fields and are not listed. Line numbers are those of the commit above and drift:
when a reader is added, moved or removed, update both tables in the same change.

Three functions in `mind/dispositions.rs` read all eight values without choosing anything:
`Dispositions::expressed` (builds the acute value from the core, rules in the first table),
`Dispositions::surface_traits` (turns the acute value into the label shown on setup cards and
person inspection) and `Dispositions::into_bounded` (clamps to 0..1). A disposition whose only
readers are these three is shown but changes no outcome. The other readers are the ones that feed
a weight or a test.

The last column is the correlation between the disposition and what people do. It is the best |r|
that `traits_best` will print as `disposition>behaviour:r` (#1826, split from #1518), a Pearson r of
per-hour rates over people with at least `min_hours` sampled hours, and it is to be measured on the
baseline that #1496 records for seeds 1-8 on all three deities. Neither exists yet (#1826 and #1496
are open), so no number is entered; the audit's earlier probe figures are not on that baseline.

| Disposition | Function that reads it (file:line) | What it changes | #1518 r on the #1496 baseline |
|---|---|---|---|
| sociability | `Dispositions::expressed` at `crates/hf-sim/src/mind/dispositions.rs:51`<br>`Dispositions::surface_traits` at `crates/hf-sim/src/mind/dispositions.rs:71`<br>`Agent::social_rate` at `crates/hf-sim/src/mind/dispositions.rs:144`<br>`Agent::talk_weight` at `crates/hf-sim/src/mind/dispositions.rs:151`<br>`Dispositions::into_bounded` at `crates/hf-sim/src/mind/dispositions.rs:185` | acute value: core minus 0.20 stress, 0.15 pain, 0.10 fatigue (lack of rest)<br>label `sociable` or `solitary`, shown when the acute value is 0.175 or more from the middle (at most five labels)<br>core value: 0.55 + 0.9 x sociability, the rate the social need falls at; only the part above 1.0 adds extra decay (`step` in `systems/needs.rs`)<br>acute value: (0.55 + 0.9 x sociability) x (0.9 + 0.2 x warmth), the weight of talking and gathering and who tells at the hearth (`hearth_evening`)<br>clamp to 0..1 only | not measured yet (#1826, #1496) |
| industry | `Dispositions::expressed` at `crates/hf-sim/src/mind/dispositions.rs:52`<br>`Dispositions::surface_traits` at `crates/hf-sim/src/mind/dispositions.rs:72`<br>`Agent::work_weight` at `crates/hf-sim/src/mind/dispositions.rs:158`<br>`Dispositions::into_bounded` at `crates/hf-sim/src/mind/dispositions.rs:186` | acute value: core minus 0.25 fatigue, 0.15 pain<br>label `industrious` or `easygoing`<br>acute value: 0.7 + 0.6 x industry, the weight of work designations (0 while the person is withdrawn)<br>clamp to 0..1 only | not measured yet (#1826, #1496) |
| curiosity | `Dispositions::expressed` at `crates/hf-sim/src/mind/dispositions.rs:53`<br>`Dispositions::surface_traits` at `crates/hf-sim/src/mind/dispositions.rs:73`<br>`Agent::wander_weight` at `crates/hf-sim/src/mind/dispositions.rs:169`<br>`Dispositions::into_bounded` at `crates/hf-sim/src/mind/dispositions.rs:187` | acute value: core minus 0.25 stress<br>label `curious` or `settled`<br>acute value: (0.6 + 0.8 x curiosity) x (1.1 - 0.2 x caution), the weight of wandering when nothing presses<br>clamp to 0..1 only | not measured yet (#1826, #1496) |
| caution | `Dispositions::expressed` at `crates/hf-sim/src/mind/dispositions.rs:54`<br>`Dispositions::surface_traits` at `crates/hf-sim/src/mind/dispositions.rs:74`<br>`Agent::wander_weight` at `crates/hf-sim/src/mind/dispositions.rs:169`<br>`Dispositions::into_bounded` at `crates/hf-sim/src/mind/dispositions.rs:188`<br>`regret_tendency` at `crates/hf-sim/src/mind/conscience.rs:207`<br>`form` at `crates/hf-sim/src/mind/witnessed.rs:107`<br>`sat_by_stone` at `crates/hf-sim/src/resonance.rs:172` | acute value: core plus 0.25 stress, 0.15 pain<br>label `cautious` or `bold`<br>the factor (1.1 - 0.2 x caution) on the weight of wandering<br>clamp to 0..1 only<br>core value: 0.25 x caution is part of the regret tendency, which makes a person lean to regret rather than rationalising (stance at 0.5 or more)<br>core value below 0.25 sets `shaken`, which stops the person telling what they saw of a carrier's manifestation (`tells` false)<br>core value above 0.75 marks a woken fragment `tells: false`, so the person keeps it to themselves | not measured yet (#1826, #1496) |
| warmth | `Dispositions::expressed` at `crates/hf-sim/src/mind/dispositions.rs:55`<br>`Dispositions::surface_traits` at `crates/hf-sim/src/mind/dispositions.rs:75`<br>`Agent::talk_weight` at `crates/hf-sim/src/mind/dispositions.rs:151`<br>`Dispositions::into_bounded` at `crates/hf-sim/src/mind/dispositions.rs:189`<br>`regret_tendency` at `crates/hf-sim/src/mind/conscience.rs:206`<br>`willing` at `crates/hf-sim/src/mind/telling.rs:85` | acute value: core minus 0.30 pain, 0.20 stress, 0.10 fatigue<br>label `warm` or `reserved`<br>the factor (0.9 + 0.2 x warmth) on the weight of talking and gathering and who tells at the hearth (`hearth_evening`)<br>clamp to 0..1 only<br>core value: 0.45 x warmth is part of the regret tendency<br>core value of 0.75 or more lets a teller tell someone they think ill of (opinion below -0.3) or whose family is at feud with theirs | not measured yet (#1826, #1496) |
| cheerfulness | `Dispositions::expressed` at `crates/hf-sim/src/mind/dispositions.rs:56`<br>`Dispositions::surface_traits` at `crates/hf-sim/src/mind/dispositions.rs:76`<br>`Dispositions::into_bounded` at `crates/hf-sim/src/mind/dispositions.rs:190` | acute value: core plus 0.50 mood, minus 0.25 pain, 0.20 stress<br>label `cheerful` or `downcast`<br>clamp to 0..1 only; no function that chooses or changes anything reads it | not measured yet (#1826, #1496) |
| order | `Dispositions::expressed` at `crates/hf-sim/src/mind/dispositions.rs:57`<br>`Dispositions::surface_traits` at `crates/hf-sim/src/mind/dispositions.rs:77`<br>`Agent::haul_fit` at `crates/hf-sim/src/mind/dispositions.rs:176`<br>`Dispositions::into_bounded` at `crates/hf-sim/src/mind/dispositions.rs:191` | copied unchanged: no acute rule moves it<br>label `orderly` or `flexible`<br>core value: 0.3 x order, 0 to 0.3 added to the score of a haul designation (0.5 + hauling skill / 20 + this)<br>clamp to 0..1 only | not measured yet (#1826, #1496) |
| independence | `Dispositions::expressed` at `crates/hf-sim/src/mind/dispositions.rs:58`<br>`Dispositions::surface_traits` at `crates/hf-sim/src/mind/dispositions.rs:78`<br>`Dispositions::into_bounded` at `crates/hf-sim/src/mind/dispositions.rs:192`<br>`regret_tendency` at `crates/hf-sim/src/mind/conscience.rs:208`<br>`trust` at `crates/hf-sim/src/mind/telling.rs:77` | copied unchanged: no acute rule moves it<br>label `self-reliant` or `communal`<br>clamp to 0..1 only<br>core value: 0.30 x (1 - independence) is part of the regret tendency<br>core value of the listener: trust in a teller's word is multiplied by (1.2 - 0.6 x independence), and trust below `BELIEVE` means the telling is refused | not measured yet (#1826, #1496) |

Not reads, so not in the table: `Genome::dispositions` in `genetics.rs` builds the core from the
genome (a write); `Default for Dispositions` and the struct declaration; comments; and the `needs.warmth`
and climate `warmth` fields named above. `study` in `systems/work.rs` mentions curiosity but reads
the old binary trait `Trait::Curious`, not the disposition. A person from an old save has no
profile (`dispositions` is `None`), and the weight functions then use the old trait weights
(`self.traits.*`); the regret, telling, witness and fragment readers use a middle value or `false`.

Two channels carry the same genome values to choices. They are not reads of `Dispositions`, so they
are outside both tables (found at 59972f57, tests and examples left out). The first table's effect column
near the top of this page describes the #488 slice as built (2026-10-31); the two tables in this
section replace it where they differ: independence is read by `trust` and `regret_tendency`, and the
caution and warmth cells there leave out the regret, telling, witness and fragment readers.

The first channel is `genome.calm()` (`Genome::calm`, `genetics.rs:309`), which returns the calm
locus. That is exactly 1 minus the core caution (`Genome::dispositions` at `genetics.rs:243`).
`git grep -n "calm()" -- crates/hf-sim/src` finds 7 lines: one is the name of a test (`engine.rs:173`)
and 6 read the value. In `mind/feelings.rs`, `gain` at 141 sets how strongly a feeling is felt. In
`systems/strain.rs`, `arrival_shock` at 209 sets the weight and length of the first-day shock, `snap`
at 271 picks who snaps to the survival pole, and `step` at 302 and 311 (a calmer neighbour steadies
them) and 341 (how fast stress builds and eases). `give` at `species.rs:492` reads the same locus
directly, not through `calm()`, to set the age of a non-human arrival.

The second channel is the legacy traits. Every new person carries them, built from four of the same
loci in `Genome::traits` (`genetics.rs:252-271`) and assigned at `opening.rs:417` and, for people, at
`scenario.rs:64` and `scenario.rs:233`: `Sociable` or `Solitary` from the sociability locus,
`Industrious` or `Easygoing` from the drive locus (industry), `Curious` from the curiosity locus and
`Cautious` from the low end of the calm locus (high caution). A trait is set only at an extreme of its
locus (above 0.75 or below 0.25), and a person keeps at most three traits. `git grep -n -E
"Trait::(Curious|Cautious|Industrious|Easygoing|Sociable|Solitary)" -- crates/hf-sim/src` finds 77
lines: 6 in `genetics.rs` build the traits and 21 in `mind/traits.rs` name them and hold the old
weights (used only when a person has no profile); the other 50, in 18 files, read them. They include
`choose` in `systems/decide.rs` at 1816-1817, 2060, 2153 and 2180. There are 5 more lines in
`mind/feelings.rs` and `systems/strain.rs`, where `Trait` is imported as `T`, that the grep does not see.

So the sociability, industry, curiosity and caution loci reach choices through more places than the
tables list, and the tables do not show the whole effect of those four. For warmth, cheerfulness,
order and independence, which are mixes of the whole genome (`extra(0)` to `extra(3)` in
`Genome::dispositions`), there is no such channel: `Genome::traits` sets no trait from them and
`calm()` is not one of their loci.

### Who calls the readers

The functions that return a weight or a test, and the one level of caller above them. A weight is
used only where it is called, so this is where the disposition reaches a choice. Callers in tests
are left out.

| Reader | Called from | file:line |
|---|---|---|
| `social_rate` | `step` | `crates/hf-sim/src/systems/needs.rs:67` |
| `talk_weight` | `choose` | `crates/hf-sim/src/systems/decide.rs:1967` |
| `talk_weight` | `choose` | `crates/hf-sim/src/systems/decide.rs:2045` |
| `talk_weight` | `hearth_evening` | `crates/hf-sim/src/systems/culture.rs:179` |
| `work_weight` | `choose` | `crates/hf-sim/src/systems/decide.rs:2199` |
| `wander_weight` | `choose` | `crates/hf-sim/src/systems/decide.rs:2395` |
| `haul_fit` | `choose` | `crates/hf-sim/src/systems/decide.rs:2274` |
| `surface_traits` | `surface_traits` | `crates/hf-sim/src/mind/dispositions.rs:128` |
| `surface_traits` | `inspect` | `crates/hf-godot/src/lib.rs:1849` |
| `surface_traits` | `survivor_cards` | `crates/hf-godot/src/lib.rs:2545` |
| `regret_tendency` | `stance_of` | `crates/hf-sim/src/mind/conscience.rs:217` |
| `stance_of` | `step` | `crates/hf-sim/src/systems/conscience.rs:58` |
| `willing` | `tell` | `crates/hf-sim/src/social.rs:192` |
| `trust` | `tell` | `crates/hf-sim/src/social.rs:220` |
| `form` | `sat_by_stone` | `crates/hf-sim/src/resonance.rs:151` |
| `form` | `balks` | `crates/hf-sim/src/resonance.rs:260` |
| `sat_by_stone` | `study` | `crates/hf-sim/src/thin.rs:665` |
