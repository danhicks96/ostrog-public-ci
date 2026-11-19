# North Star: people who feel alive

Dan, 2026-11-13 (D93): the findings of the alive audit are the project's North Star. Every work
package is judged against it, and the alive work is broken into bite-sized slices so that each one is
built, tested and committed before the next is taken.

Sources: the audit (docs/reviews/alive-audit-desktop-2026-11-13 (not included in this source snapshot)
on branch `docs/alive-audit-desktop-2026-11-13`), the parallel interim audit on
`docs/alive-audit-2026-11-13`, the tracking epic #1531 and work package WP-10 (not included in this source snapshot).

## The sentence

**Every condition a watcher can see registers in someone, moves them to act, and ripples to
others; and a glance shows who they are, how they feel and what they are doing.**

## Where we stand (2026-11-13)

| Pillar | Bar | Score | 10/10 means, in mechanics (never content) |
|---|---|---|---|
| Inner life | The Sims | 3 | Mood swings with what happens and stays down after real loss; temperament and feeling visibly change choices; people want things for themselves, pursue them over days, and fear what hurt them |
| Story engine | RimWorld | 4 | Pressure is paced against the colony's strength, with build-ups, climaxes and quiet; consequences chain through people; a colony's end is a chapter, not a dead screen |
| Living world | Dwarf Fortress | 5 | Bodies limit, land regrows, temperature has place, other peoples live their own lives and walk in; ruins hold knowledge |
| Watcher | (the product) | 3 | A glance shows mood, intent and identity; a returning watcher learns what changed and for whom; nothing important is missed for ever |

The detailed rubric with 3/7/10 anchors is K2 (#1497) under the epic; future audits score against it.

### The three cuts in the living loop

The loop is sense, remember, feel, decide, act, and be sensed. Today it is cut in three places, and
nothing else matters as much:

1. **Nobody stays unhappy.** Mood adds up every long-term memory (`systems/needs.rs:159-177`), so
   routine meals and talk pin it at +1. Across seeds 5-8, 64-85% of mood samples are at 0.9 or
   above. A partner's death moves the survivor's mood by about zero against the colony's mean.
2. **Feelings do not decide.** `systems/decide.rs` never reads mood. Curiosity and sociability
   show in behaviour; the other six dispositions change nothing a watcher can see.
3. **Ties never fade.** `Relation.last_contact` is written and never read. Of each person's five
   strongest ties, 91-94% sit at exactly +1.00.

And around them: building is decided by a colony planner rather than by people (against design
rule 6, "the people are the players"); half of waking hours are standing about; nothing paces
threats against the colony; a wiped colony leaves a dead screen for ever; and rules 8 and 9
("observation is evidence", "problems become lived lessons") are written but not yet true in
the build.

## The numbers that say we are getting there

Measured by `ostrog soak --alive` once K1 (#1488) lands (until then, by the audit probe). Each fix
names the key it moves. A change that makes a key worse needs a DECISIONS row that says why.

| Key | Meaning | Today (seeds 5-8) | Alive target |
|---|---|---|---|
| `mood_hi` | Mood samples at 0.9 or above | 64-85% | 15% or less |
| `mood_clamped` | Mood samples at exactly +1.00 | 59-72% | 2% or less |
| `grief_drop_10d` | Survivor's mood drop after a partner dies, against the colony mean | about 0 | 0.4 or more |
| `standing_share` | Loitering plus idle, share of waking hours | 50-55% | 20% or less |
| `work_share` | Working, share of waking hours | 1-3% | 15% or more |
| `ties_saturated` | Top-5 ties at +1.00 | 91-94% | 25% or less |
| `ties_frozen` | Close ties untouched for 100+ days and still +0.95 or more | 61-220 | 0 |
| `repeat7` | Events repeating the same kind and people within 7 days | 22-39% | 10% or less |
| `routine_memory` | Six strongest memories that are meals or company | 73-96% | 60% or less |
| `births_per_joiner` | Births per person who joined | 0.22-0.69 | 1.0 or more |
| `traits_shown` | Dispositions that predict a behaviour share | 2 of 8 | 8 of 8 |
| `glance_readable` | People in the camera's frame visibly doing something distinguishable | not measured | 80% or more |

Seeds 1-8, all three deities. A short subset gates CI (K4 (#1511)), and each improvement tightens its own bound
in the same pull request.

## The order of work

The watcher-first five, from the audit's roadmap. Each is an umbrella of slices under the epic:

1. **Make mood honest, then show it.** Bounded, recency-weighted memory term; habituation; grief
   that lasts; a mood cue and the strongest thought on the sprite.
2. **Let mood and feelings choose.** Mood and the strongest feeling, with its subject, into `choose`;
   breaks aimed at their cause; every disposition visible in a day.
3. **Turn standing about into visible, chosen leisure and small chores.** Split `Loitering`; a
   leisure catalogue chosen by temperament.
4. **Wants and possessions.** A personal aims layer; owned beds, homes and tools; the planner
   suggests and people adopt.
5. **Make missed events and endings findable.** A catch-up on a timer; a roster of the dead with
   biographies; a wiped colony becomes a ruin that a later party finds.

Then: relationships that fade and have kinds and verbs; killing the repetition loops; lessons that
change choices; a storyteller over the existing generators; perception of ordinary acts; bodies
that limit; the wider world walking in. Measurement (K1 (#1488), K3 (#1504), K4 (#1511)) comes first of all, so every slice
can show its number.

## Rules for everyone working here

- **One slice, one test, one commit.** Take one slice issue (size XS or S). Build it, add its named guard
  test, run its gate, commit, open the pull request, and tick it on its umbrella. Only then take the next.
  An umbrella issue (label `tracking`) is never worked as a whole.
- **Autonomy has no ceiling.** Anything that waits on a player, idles for want of a reason, or is
  dumbed down to keep it simple is a gap, not a design.
- **The consequence check.** For any visible condition you add or touch, say how it registers (mood,
  feeling, memory, body, tie), what it moves someone to do, and how it reaches others. If it does none,
  it is cosplay: wire it or don't draw it.
- **The loop check.** For any `hf-sim` change, name the link of the living loop it strengthens and the
  alive key it moves, with before and after numbers from a soak.
- **Visible or it didn't happen.** A new inner state that a watcher cannot see at a glance or in one
  click needs its client slice filed before it merges.
- **Repetition is a bug.** The same line about the same people night after night is machinery
  showing; compress it in the feed and fix the loop in the sim.
- **Mechanics, not content.** Compare against The Sims, RimWorld and Dwarf Fortress by how their
  systems work; never copy their characters, art, names or text.
- Standing rules still hold: "what would a normal person do here?" (rule 7) bounds every leisure and
  want; determinism, append-only enums, FIXLOG entries and soaks as in AGENTS.md.
