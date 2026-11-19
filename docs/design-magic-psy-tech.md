# Magic, psy and tech in Ostrog (#308)

Design note for issue #308: what the game already has of magic, psy and tech, what fits, what
would break the simulation, and the one slice built on branch `feat-308-slice`. It follows epic
#508 (hidden arrival changes and ancient resonance) for the psy part.

## What already exists

- Magic with recorded causes (`conditions.rs`). The restless dead, the Beast-hunger, the
  Thirst, the Stone-sleep and the true-dream (#547) each hold the row that gave them and who
  passed them. The living never know who is cursed; they see patterns and come to suspect
  someone, with those rows as the source, and a suspicion can be wrong. The unbinding lifts a
  curse, by a healer who knows it.
- The deity acts through real pressures (`deity.rs` levers: cold, dry storms, harvest, herds,
  sickness, wolf hunger), read aloud by people as signs (`faith.rs`, `omens.rs`) and later
  borne out or belied on the record.
- The thin place (`thin.rs`). The portal stone the ten came through thins two seasons on: two
  people dream the same dream, a thing from home lies by the stone, the echo of one who died
  alone stands at dusk, wrong weather falls on a patch, a hollow spreads. The curious sit by it
  and come to understand it (`ThinLore`); the cautious ward it, the unafraid leave it open and it
  leads one of the living lost back.
- Spells (`spells.rs`) and the underworld's stone (`underground.rs` breaches) are further
  supernatural pressures, each tied to a place and a row.
- Tech: nothing beyond the material chain (design rule 2). Things from home that come through
  the stone are the only objects not made in the valley.

## What fits

- More magic of the same kind: each new curse or mark a cause, a pattern others can see, and a
  wrong-suspicion path (the bag of #283). The true-dream showed the shape.
- Psy, through epic #508: a hidden, deterministic potential carried by some of those who came
  through the portal, woken by something that really happens to them, showing first as
  something ambiguous, with limits and costs. Knowledge it brings is a fragment, and belongs to
  the person (design rule 3).
- Tech only through the portal: things from home, or from the thin place, that are strange,
  limited and cannot be made again. Never a research bench.

## What would break the simulation

- A power that is a free bonus to work or survival. Every change has a cost, and it never
  overrides hunger, shelter or safety (rule 7).
- A list of powers at setup or in inspect (rule 8, epic #508): the player learns from events,
  witnesses and behaviour.
- Omniscience: a sense that tells someone a fact they could not get, rather than a feeling tied
  to a real state of the world.
- Chance without a cause: who carries a potential is drawn once from the seed and the person,
  never rerolled, and what wakes it is a recorded event.
- Building tech from research (rules 1, 2, 6): the survivors could not make it, so it is not
  made.
- Mapping a neurotype, diagnosis or ordinary trait to a power (#508 non-goal).

## The slice: the rock-sense, a hidden potential woken at the stone

Built in `crates/hf-sim/src/resonance.rs`; epic #508 steps 1 to 4 for one potential.

1. Hidden state. When the ten come through, each is drawn a potential once from the seed and
   their place in the party: about two in ten carry one, and in this slice the one they carry
   is the rock-sense. It is saved with the person, never rerolled, and shown nowhere: not at
   setup, not in inspect, not in the feed until it shows itself.
2. Trigger: the portal. A carrier who sits by the woken stone (decide.rs `StudyThin`, the
   curious and those it has touched) for four hours in all, while the air there is thin, wakes
   it. The waking is quiet: they wake the next morning shaken (their rest is lower), and the
   record has "came back from the stone unsettled", with nothing said of why.
3. Manifestation: at a mining face whose cut would leave the roof unheld (`underground.rs`
   `unheld_if_opened`, the same test that brings a roof down), one woken who does not know
   shoring stops and will not cut it. The first time, the feed says they would not cut and
   could not say why. They are tired by it (rest falls), and the place is a bad one to them.
   The cut is left for someone else, who may bring the roof down.
4. Limits: only the roof, only at a face they are cutting, only when it really is unheld. A
   woken carrier who knows shoring props as anyone who knows it does, and senses nothing more.
   A carrier who never sits by the stone never wakes; most of the ten carry nothing.
5. Cost, failure and recovery (WP-07 7.4). Each time it stops them it costs them rest
   (`COST`, 0.1). It does not come to one worn out: below half rest (`FAILS_BELOW`) a carrier
   at the same deadly face feels nothing, records nothing and cuts it as anyone who does not
   know shoring would, with the same chance of bringing the roof down. So it holds them back in
   the morning and may not by evening, or after it has fired twice. Sleep brings it back. Nothing
   tells the player which it was: the record shows a refusal one day and a collapse another.

## Assumptions (Dan can overrule)

- The rate is the epic's: two in ten of the arrivals, drawn per person (so a party may have
  none or several). Newcomers and those born in the valley carry nothing in this slice.
- Four hours at the stone, in all, wakes it; the stone must be woken and not yet warded.
- Half rest is where it fails (7.4): a working person is between about 0.3 (when they turn in)
  and 1, so an afternoon's work or two firings take a carrier below it.
- Witnesses' interpretations (a coward, a seer), testing and teaching are later steps of #508.

## Later slices

- Witnesses: those who see the refusal and then see the roof come down for another hold the
  carrier in deference in keeping them safe (justice.rs); those who see nothing fall think them
  idle or afraid.
- More potentials from #508's families, each with its own trigger (grief, terror, an old
  material) and cost.
- A thing from home that cannot be made again (the tech answer), e.g. a tool of a metal the
  valley has none of, wearing out for good.
