# Events, accounts and evidence (#452)

Design note for issue #452: an event happens once and is kept as it happened; people hold
accounts of it that can differ and spread; later people read what it left behind and may get
it wrong. This note says what the code already does toward that, what is missing, the first
slice built on branch `feat-452-stories`, and what would break the rules.

## What already does part of this

- The record. Every event is a row in the chronicle (`events.rs`, `chronicle/`), written once
  with its tick, place, participants and cause. Nothing rewrites a row. The egress point's
  history before the arrival (`deep.rs`: sites, figures, graves, how each hamlet ended) is the
  same kind of record, written before the epoch.
- Accounts, for the pre-arrival settlements only. `lore.rs` keeps tales: one version of how an old hamlet
  ended, held by a person, with who told it (`teller`) and how they came by it (`how`:
  "their family's memory in Mirewell", "told by Kella", "their own guess"). Tales pass in talk
  (`lore::on_talk`, called from `systems/act.rs`) and drift for a reason: a teller who holds
  that strangers bring harm, or is cautious, tells it darker (a famine becomes a curse, a
  move becomes being driven out). `Tale::true_to` checks a tale against the record without
  changing either. Culture (`systems/culture.rs`) has the keeper of a community tell a tale to
  those who lack it.
- Evidence, read badly. `lore::step` has settlers standing in an old ruin guess how it ended
  (the fearful guess a curse, burned timbers read as fire); `left_sites.rs` does the same for
  camps a community left after the arrival. Engravings (`engrave.rs`) carve a real event's
  words into a wall, so a later reader has the row. Things with a life (`legend.rs`) and
  relics (`relics.rs`) carry their deeds and whose they were.
- Beliefs with sources. `mind/beliefs.rs` keeps a stance on a topic with its source (an
  event, a teller), and faith (`faith.rs`, `omens.rs`) has readings of signs that are later
  borne out or belied on the record.

## What is missing

- Nothing that happens after the arrival becomes an account. Tales are keyed to an old site
  id; a death, a flood or a raid in the colony's own time is in the record and in a few
  memories, but nobody tells it to anyone, so it cannot drift and cannot be wrong.
- The colony's own dead leave no trace. A body is drawn for a few hours (`view/bodies.rs`,
  view only) and then nothing marks where someone was put in the ground, so there is nothing
  for a later person to find and read.
- Inspect shows tales of the pre-arrival settlements but nothing a person believes about their own time,
  and nothing on a tile says "this is what happened here; this is what is said of it".

## The first slice: a death becomes told accounts and leaves a grave

Built in `crates/hf-sim/src/accounts.rs`. One kind of event (a person's death) and one kind
of persistent trace (a grave).

1. Event. The `Died` row is untouched. Its cause words are classed into what a person would
   say (`Said`: sickness, hunger, cold, thirst, wounds, a beast, a fall, drowned, old age, the
   rock, lost, or the words themselves).
2. Witnesses. Those awake, not children, on the same level and within eight tiles of the death
   when it happens hold an account "saw it" with the true cause. Nobody else does.
3. Telling. When two people talk, one may tell the other an account of a death the other has
   no account of. The listener keeps it "told by <teller>". It can drift, by the same rule as
   the pre-arrival settlements' tales: a teller who holds strangers bring harm, or is cautious, drifts it
   more often. A sickness or a famine death becomes a curse, a fall or a fight becomes a
   killing, a beast, the water or the deep becomes "taken". Each account keeps its teller, so
   the chain back to a witness can be followed.
4. Trace. A person buried by their people's rite (`Rite::Bury`, `systems/rites.rs`) where
   they lay gets a grave marker on that tile, recorded with the death row and the rite row.
5. Inference. A grown person who never knew the dead, holds no account of that death and came
   to the colony after it, standing within three tiles of the grave, reads it: "someone was
   put in the ground here". The cautious or those who fear death guess a curse; the rest say
   they cannot tell how. The account records the grave as its evidence. If later told the
   story, they hold both: the reading and the telling, side by side.
6. Acting on it. An account that the death was a curse or a taking makes its holder uneasy
   about the place (a bad memory of that tile, through the existing `Memories::aversion`), so
   only those who saw or were told it keep their distance. Nobody acts on an account they do
   not hold.
7. What the player sees. Inspect lists a person's accounts beside their tales, with how each
   came ("saw it", "told by Kella", "read from a grave"). Hovering a grave says whose it is and
   how they died, from the record, then what people say of it, with who says it. The feed
   tells a telling that changed ("In Kella's telling to Bram, Halgar was cursed").

## Assumptions (the maintainer can overrule)

- Only burial leaves a grave. Burning, laying out under the sky and the hall of the dead leave
  no marker in this slice.
- Drift uses the pre-arrival settlements' tale rule (teller's fear of strangers or caution), not a new
  model of motive. Framing for advantage (blaming a rival) is later work.
- Accounts are capped at 24 a person; the oldest told ones go first, so what someone saw lasts
  longest.
- A grave read never names the dead. Only telling joins the reading to a name.

## Later slices

- Other events: floods into tunnels (`floods.rs`, the issue's example), raids (`siege.rs`),
  fires. The same `Said`-style classing per event kind and the same telling path.
- Other traces: the flood's silt and the collapsed passage, burned timbers, the wall breach,
  each recorded as the trace of a row.
- Communities as holders: when most of a community holds one version, it is "what is said in
  Ashford"; the keeper tells that version at observances (`systems/culture.rs` already does
  this for tales).
- Engraving an account rather than a row, so a carving can be wrong.

## What would break the rules

- Writing an account back into the record, or narrating a drifted account as fact. The record
  stays the only truth; the feed says "in Kella's telling", never "Halgar was cursed".
- Random distortion. Every change has a teller whose fear made it.
- Global knowledge. Nobody holds an account without having seen the death, been told, or
  stood at the grave.
- Determinism. All chance draws from `rng::stream` with the seed, the two people and the tick;
  accounts and graves are saved with the world.
- Rule 7. The unease is a bad memory of one tile that fades over days (`TRAUMA_HALF_LIFE`); it
  weighs choices and never stops anyone going home, eating or sleeping.
