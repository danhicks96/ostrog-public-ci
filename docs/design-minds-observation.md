# Situated minds, observation and problem solving

Status: current design direction, 2026-10-30. This document turns the later psychology and
problem-solving conversations into an implementation contract. It preserves the original
Ostrog principles—autonomy, no omniscience, real history and people who remain themselves—
while making the next layer explicit.

## 1. People are dimensional, not diagnostic labels

Ostrog models observable dispositions and learned responses, not clinical diagnoses. A person
can have a stronger or weaker preference for novelty, solitude, order, risk, beauty, company,
water, heat, cold, noise, darkness, repetition or particular kinds of work. They can enjoy a
task, tolerate it, become bored by it, resent being assigned to it, or become absorbed in it.

These are weighted influences, not rigid switches. A preference changes a score; it does not
remove agency. Hunger, danger, duty, relationships, memory, skill and opportunity can overcome
it. The same situation can therefore produce different but understandable reactions in different
people, and different reactions in the same person at different times.

No label should be used as a shortcut for a real-world diagnosis. The model describes behaviour
and friction in context, including masking or accommodation where the simulation eventually
supports it, without declaring a person cured, broken or morally defective.

The first dimensional core/surface implementation and its explicit scope boundary are recorded in
`docs/design-dispositions.md` (#488).

## 2. Observation is personal state

The world is authoritative, but a person is not given the world state for free. Each person and
animal has an observation ledger:

1. an initial full observation when they first attend to a tile, object, creature or event;
2. later deltas when something changes or they look again;
3. confidence, age, source and interpretation attached to what they think they know.

The ledger is observer-specific. Sight, hearing, smell, touch, prior skill, attention, lighting,
obstruction, distance and stress determine what enters it. A person may notice damp stone and
still misunderstand what it means. Another may already know the pattern from a previous life.
The player can inspect the person's evidence and confidence, not a fabricated omniscient answer.

The same protocol applies to the client: full snapshots establish state, then deltas conserve
work. A delta is not permission to invent a fact; it is only a changed or newly observed fact
that the bridge actually received.

## 3. The problem-solving loop

The canonical chain is:

`problem -> observation -> appraisal -> reaction -> attempted response -> outcome -> interpretation -> memory, belief, skill or trust change`

A problem is local and real: a flood reaches a camp, berries stop returning, food spoils, a wall
weeps water, a person is overheating, a route collapses, or an animal is trapped. The problem
does not automatically become a colony-wide task. People who observe it choose among responses
available to them, weighted by need, personality, skill, known techniques, relationships, trust,
fear, resentment, time and perceived risk.

Responses may be productive, cautious, avoidant, selfish, reckless, curious, resentful or wrong.
An unsuccessful response is still a real outcome and can produce a useful, mistaken or harmful
lesson. The chain remains visible in history and inspection when the relevant records exist.

## 4. Learning and social transfer

Watching an outcome can teach, but it is not automatic. Learning depends on attention, whether
the observer could perceive the relevant cause, prior knowledge, trust in the actor, willingness
to imitate, emotional state and the observer's interpretation. A trusted person may explain a
lesson; a rival may withhold it, distort it, or want the other person to fail. A warning retains
its teller and can be false, stale or misunderstood.

Knowledge belongs to people. It can be taught, copied, forgotten, revised, lost when its last
holder dies or leaves, or preserved in a durable record whose wording may still be wrong. Social
learning is a reason to seek people, teach children, protect specialists and sometimes avoid
those who have harmed or deceived us.

## 5. Physical hazards and liquid truth

Movement and rendering must obey physical state. Nobody walks on unsupported water or lava. A
person can swim, float, wade, climb out, be swept by a current, panic, become exhausted, drown or
be rescued only when the simulation provides the relevant capability and state. Aquatic ability
improves competence or tolerance; it is never immunity.

Flooding, undercurrents, porous walls, cave-ins, spoilage, overharvesting, heat, cold and
darkness are all ordinary problems that can create different reactions and learned lessons.
Liquid appearance may vary with depth, body, flow, temperature, murkiness, pollution or other
properties only after those properties exist as simulation-backed state. The renderer never uses
colour to imply a fact that the simulation does not model.

## 6. Rare changes are discovered, not selected

Ordinary inheritance remains bounded by the species and body-plan rules. Separately, the game may
roll a rare in-play biological, psychic or ancient-linked transformation: the intended design
rate is 2 in 10 eligible opportunities, not a setup option. The setup screen must not reveal who
will change. A transformation needs a recorded cause, visible onset, consequences, and a history
that distinguishes observation from rumour. It may be wondrous or frightening, but it is not a
free power and never bypasses physical rules.

## 7. Acceptance scenarios

The implementation must eventually guard and review these seeded situations:

- a swimmer meets an undercurrent and may be swept, rescued or drowned;
- a flood changes routes and causes people to move, remember and teach differently;
- overharvesting delays regrowth and produces different choices by person;
- exposed food rots while sheltered or preserved food survives;
- desert heat produces different shade, water, work and shelter responses;
- a porous wall leaks or stains before a breach when the simulation supports the clue;
- a person acts on incomplete or false information;
- a trusted teller shares while a rival withholds or distorts;
- no agent traverses unsupported liquid;
- a rare transformation is absent from setup and appears only through play.

The tests must prove personal ownership, provenance, bounded resource use, deterministic seeded
outcomes and truthful inspection. A passing mechanism test is not enough without an integrated
game review.
