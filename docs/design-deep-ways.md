# The deep as a living threat and a resource (#455)

Design note for issue #455: dangerous places joined to the colony by a way through, with their
own people, creatures and riches, which the colony can go into, profit from and pay for. In
Ostrog the deep caverns under the egress point already are such a place, and the breach a
community digs into one is its portal. This note says what the code already does, what is
missing, the first slice built on branch `feat-455-portal-economy`, and what would break the
rules.

## What already does part of this

- A persistent other place. Deep caverns are made with the world in the basalt below the
  egress point (`world/gen/caverns.rs`), each with a name, a size, sometimes a pool, sometimes the
  underworld's stone in its walls. Below them, beyond the map, lie the deep's settlements and
  peoples in the realm (`realm/sites.rs`, `deep: true`), with beasts of their own
  (`realm/beasts.rs`, lairs in the deep caverns).
- A way through with a state. A community that mines into a deep cavern breaches it
  (`underground.rs` `breaches`): the breach tile, whose way it is (`gate_of`), whether it is
  walled (`sealed`) and who keeps it (`keeper`) are saved with the cavern. Delving
  (`delving.rs`) sinks shafts for real reasons (ore, tin, harder stone, the gems and gold of
  the deep) and lets a cautious community wall the way or a greedy one dig on (`DeepChosen`).
- Crossing both ways. At a breach, two of the deep's nearest people come up and walk to the
  community as strangers do (`FromBelow`); a beast of the deep that lairs near is stirred
  (`BeastStirred`) and later comes up as a crawler that hunts by night (`Rose`) unless the
  way is walled first (`Barred`). The colony posts a wall at the breach. A society can bargain
  with the deep's people to take a hated person's child (`societies.rs`); a search party goes
  below after it, off the map like a caravan (`taken.rs`), and a child grown below may come up
  again.
- Riches and their price. Gems lie only in the deep basalt and are cut into jewels at a bench;
  touching the underworld's stone brings the Stone-sleep (`conditions.rs`); cave mushrooms
  spread over dark floors (`cave_life.rs`). Digging deeper is slower, darker, wetter and can
  bring the roof down (`underground.rs`, `aquifer.rs`).
- Expeditions. Caravans form from real people with real loads, travel in day-steps off the map
  and come back changed (`systems/expedition.rs`).

## What is missing

- The threat runs on a clock, not a cause. A stirred beast comes up exactly two days after the
  breach unless the wall stands by then (`creatures.rising`, due tick). Nothing the colony does
  in those two days changes it, and once barred or risen nothing more comes. The issue asks
  for escalation that follows world state, with local warning signs.
- No ongoing relationship with the deep's people after the first two come up.
- No dungeon heart and no material whose use is tied to the deep's life; gems are only a
  luxury.
- No expedition into the cavern itself (the search party is the only going below).

## The first slice: what stirs below comes up because of what the colony does near it

Built in `crates/hf-sim/src/stirring.rs`; replaces the two-day clock for new breaches.

1. A breach that wakes a beast of the deep gives that cavern a stir, a count kept with the
   cavern's index and the beast's name.
2. Each hour, the stir rises with the noise the colony makes there: each person of the colony
   in the cavern or within six tiles of the breach on its level adds one, and each of them at
   work (mining, digging, building) adds two more. An hour with nobody there takes one off.
3. When the stir first reaches a warning level, those within earshot hear it: "Something moves
   below, in Ashdeep" goes on record, and each of them keeps a bad memory of the place, so the
   cautious keep off and the hearers can be told of it.
4. When it reaches the rising level, the beast comes up through the way if the way is open,
   and finds it barred if walled, and goes back down, as a barred beast did before. Either is on
   record, after the warning that says where the noise was.
5. The wall the colony posts at the breach is itself noisy work: a slow wall can bring up what
   it was meant to keep down, and the warning comes first.

A save from before this slice keeps its old due-tick entries and they come up as they would
have.

## Assumptions (the maintainer can overrule)

- Warning at 12, rising at 30. With three people working at a breach that is a warning in two
  hours and a rising in about four; one person passing now and then never wakes it.
- Only the beast of the deep is moved off the clock. The two of the deep's people who come up
  at a breach still come at once; their later coming and going is later work.
- Mining anywhere within six tiles of the breach counts, not only in the cavern; sound carries
  through rock.

## Later slices

- The deep's people come and go through an open way by their own needs, as hungry places beyond
  the colony send bands (`threat.rs`), with trade or raids following what they want and what
  the colony has.
- An expedition into the cavern: a party of real people with lamps, food and tools, out of sight
  below, coming back with gems, mushrooms or nothing, hurt or lost (the caravan model).
- A heart and its crystals: a deep cavern whose wall-crystals grow back from a heart while the
  heart stands; a crystal set in a lamp gives light below without oil; cutting out the heart
  stops the growth and leaves the cavern's creatures hungry, so they come up.
- A portal in the proper sense: the thin place (`thin.rs`) as a way to somewhere else, with the
  same stir and crossing rules.

## What would break the rules

- A timer, or a wave that comes whatever the colony does. The stir has only the colony's own
  presence and work as causes.
- Global knowledge. Only those within earshot hear the warning; others know only if told.
- Free riches. Anything taken from the deep is mined, carried and paid for in risk (this slice
  adds the risk to work that already exists: the gem and ore galleries near a breach).
- Determinism. No chance enters the stir; the rising is at a count, and the state is saved with
  the creatures.
- Rule 7: the warning gives the cautious a reason to keep off; it never sends anyone toward the
  breach.
