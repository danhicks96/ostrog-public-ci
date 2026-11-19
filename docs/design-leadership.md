# Emergent leadership: who the Oakfolk follow, and why

Requested as a brainstorm. Design document, not a plan of record. Constrained by the
project's law: no scripted events, no omniscience, no modal prompts, no language model in
the simulation (D9); history is recorded, not invented. Builds on design-emergence.md (the
need -> goal -> action spine) and the observation / feelings / affect work of this session.
Depends on the affect + perception-condition extension (witness -> felt reaction), specced
separately in design-feelings.md.

## 0. The claim this document makes

Leadership in a colony is not assigned and not a trait. It is *granted* by followers and
*emerges* from what is witnessed, then spreads by affective contagion. Two roads lead to the
same "magnet" property the brief asks for, and they end in opposite colony health:

- **Prestige** — charisma. Others *copy* you because you are admired.
- **Dominance** — manipulation. Others *comply* because the cost of not is worse.

Model both. Collapsing them into one "charisma" stat throws away the interesting failure mode,
because a prestige leader steadies a colony and a dominance leader extracts from it.

## 1. The spine

```
witnessed deed / stance -> deference credited by followers -> influence (per domain)
        -> affective contagion to nearby survivors -> soft bias on their choices
                                   ^
                  never a command; the nagging mind does the forcing
```

```
trait(Charisma | Dominance) -> pull on witness -> but only if deference accrues
        -> no deference, no leader, regardless of trait
```

## 2. Influence is granted, not possessed (deference ledger)

A charismatic survivor standing alone has zero leadership. Influence exists only because others
*give* it. Model a **deference ledger** per agent pair: when B copies A's belief, walks where A
walks, or joins A's side in a split, B's credit to A rises; when B diverges, it decays.
Charisma without deference is nothing — this guards against trait-as-magic.

Concretely, reuse the existing `opinions` / `relationships` (familiarity, opinion) and extend
them with a directional `defers_to` weight that the perception pass and the decision scorer
already have hooks to update.

## 3. Influence is a vector, not a crown

One leader is a trap. Humans follow different people per crisis. Give each survivor a
**per-domain influence**:

| Domain | Earned by | Existing hook |
|---|---|---|
| Provisioning | foraging, cooking, building stores | `Role::Forager/Cook/Builder`, designations |
| Protection | fighting off bands, hunting dangers | `threat.rs`, `Avenge` |
| Spiritual | presiding at the thin / portal, rites | `thin.rs`, `deity.rs`, `rites.rs` |
| Social | welcome norms, settling quarrels | `society.rs` welcome, opinions |

Scarcity swaps who is followed. The same person need not lead every domain.

## 4. Visible deeds beat loudness (prestige without extroversion)

Prestige accrues from *observed* competence, not talk. `systems/perceive.rs` already lets
survivors witness each other; extend it so a witnessed valued deed (builds the wall, tends the
sick, feeds the camp) credits the doer with prestige in the watcher's ledger. A quiet expert
who *does* the thing, even between wanders, can lead in their domain. This is the loophole that
keeps wanderers free *and* lets merit surface — it satisfies "do not limit the cloud-gazers."

## 5. How influence is exercised — soft authority only

To honor "do not force things on them / do not limit wanderers": leader influence **biases**,
never commands.

- **Affective contagion** through `perceive.rs` + the affect layer: a leader's mood / feelings
  bleed to nearby followers (emotional contagion is real and measurable). Charismatic survivor
  in good cheer -> morale lift; anxious-dominance survivor -> paranoia spread. This is the
  mechanism by which the "magnet" actually pulls.
- A gentle nudge on `choose()` goal scores (priority of nearby designations) and on
  `society.rs` *framing* of welcome / role, not on hard role assignment.
- Nobody is conscripted. The nagging mind (the affect layer) does the forcing, which is the
  whole point of the observation -> feeling -> reaction chain.

## 6. The dark side is a feature (wire it to the existing split)

A dominance leader in scarcity does recognizable things: redirects stores to allies, scapegoats
the newcomer (interacts with `WelcomeNorm` / `Hospitality` belief and `lore.rs` storytelling),
and captures the narrative so the group cannot easily eject them.

Do not patch this out. Instead **connect influence -> cohesion** so a toxic leader *causes* the
schism rather than being assigned away. `society.rs::cohesion_and_splits` already fires when
cohesion < 0.4 with enough members; let a dominance leader's extractive behavior drive cohesion
down so the split triggers on its own. The colony arc writes itself.

## 7. Ritual as the legitimacy transfer

`rites.rs` is already referenced (in `joined`). Humans do not merely *have* a leader; they
*signal* one through ritual. Let presiding over rites grant durable influence. Cheap, buildable,
and it explains why a nobody becomes focal once the hearth-fire is lit.

## 8. Feed the Engine (closes #280's social blind spot)

`engine::strain` reads only hunger + rest + comfort; warmth and social never reach it (#280).
Collective influence + cohesion is exactly the social signal missing. A steadied colony under a
prestige leader -> fewer `arcs.rs` troubles -> the Engine eases; a manipulated colony -> more
arcs. The storyteller finally feels the social body. `arcs.rs::weight()` already feeds the
music; extend the same read to `engine.rs`.

## 9. Build hooks (verified in code)

| Piece | Where it lives / hooks |
|---|---|
| Traits `Charisma`, `Dominance` | `mind` traits (alongside Curious/Industrious/Tidy/Hearty/Cautious/Sociable/Solitary) |
| Deference ledger, influence vector | new `mind/influence.rs`; persists like `opinions`/`relationships` |
| Visible-deed prestige | extend `systems/perceive.rs` (already witnesses agents/structures) |
| Contagion | `systems/perceive.rs` + affect layer (design-feelings.md) |
| Dominance / narrative | `lore.rs` (storytelling), `arcs.rs` (hardship feed), `society.rs` (`WelcomeNorm`, `Hospitality`, `cohesion_and_splits`) |
| Ritual | `rites.rs` |
| Spiritual domain | `thin.rs`, `deity.rs` |
| Soft bias | `systems/decide.rs` `choose()`; `systems/society.rs` `assign_roles` (skill-only today -> add influence framing) |
| Engine feed | `engine.rs` `strain`; `arcs.rs` `weight()` |

## 10. Open questions

- Deference decay rate and how fast a leader can be unmade.
- Whether dominance may *force* a split or only encourage one.
- Cross-community influence via trade carriers as contagion vectors.
- How strongly the spiritual domain should outrank the others under the Engine.

## 11. Relation to other docs

- design-emergence.md — the need -> goal -> action spine this sits on.
- design-feelings.md — the affect + perception-condition extension this depends on (written 2026-10-23).
- #280 — Engine strain blind spot this helps close.
