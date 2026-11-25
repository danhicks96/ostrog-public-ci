# Organic people: lived pasts, beliefs with sources, learning by trying

Written 2026-10-16 from the maintainer's direction: people figure things out and try things; fire and
famine are resolved by what individuals know and choose, not by a settlement planner; adults
arrive with lived lives that contribute to how they act without deciding it; losing is fun.

This replaces three things that exist today: founders with empty memories (`opening::identity`),
scripted settlement responses (Douse orders posted by `hazards::respond`, rationing and work
orders posted by the planner), and survival gates that treat a famine as a failure.

## 1. A life before the arrival

When the ten are made, each survivor's past is generated year by year from birth to their
starting age, from their own seeded stream, before tick 0. It is history, not flavour: every
formative event is written to the chronicle at a tick below the arrival epoch (D18) with a real id, so the
never-invents audit still holds and the History panel can answer "why is she like this".

A past has a home (a named village of the old world), parents and siblings with their own
traits, and formative events drawn by age and circumstance: a parent who drank or beat them,
a kind grandparent or teacher, a famine winter, a house fire, a death in the family, an
apprenticeship, a love, a child, a betrayal, a flight. Each event becomes a long memory whose
strength fades with years but never to nothing, and whose origin is that chronicle row.

The same event can take two people opposite ways. A real family is the reference case: two sons
of an alcoholic father, one drinks and one never does. So an event does not set a belief
directly; it opens a question ("what is drink to me"), and the answer is drawn from temperament
(genome), from whether a counter-model existed (a grandparent, a teacher, a friend who lived
differently), and from chance. The result is recorded with its sources. Siblings share events
and diverge in answers.

## 2. Beliefs with sources

A belief is a stance on a topic (drink, fire, water, strangers, authority, sharing food,
hunting, the dark, the missing, the Engine's world) with a confidence and a list of sources:
the chronicle events and the people it came from. Beliefs weight choices; they never decide
them. The weight is capped (a belief can move a choice's utility by at most about 40 percent),
so a frightened person can still be brave on a good day and a careful one can still be reckless.

Beliefs move. Talking spreads them, more from people one trusts. Outcomes confirm or shake them:
someone who believed water beats fire and watched it work holds that harder. Trauma aversion,
which came from the test agent branch, is the first instance and becomes one belief kind among
others.

## 3. Techniques: knowing how comes from trying and watching

A technique is a way of doing something (beat out flames, carry water to fire, cut a firebreak,
ration the store, eat the seed grain, forage further out, smoke meat, leave). Nobody knows all
of them. Survivors start with what their past taught them (the one who grew up in a fire-prone
village knows firebreaks). In a crisis a person picks among the techniques they know, weighted
by beliefs and past outcomes; a curious or desperate person may try something untried. Watching
someone succeed teaches it; watching someone die doing it teaches something too. Children learn
from whoever raised them, which is how upbringing carries forward.

Watching is not automatic instruction. The witness must attend, perceive the relevant cause,
interpret the result and be willing to learn. Trust, resentment, personality, skill and the
witness's prior beliefs change the chance and shape of the lesson. A teacher can explain, refuse,
withhold or distort; an accident can teach a useful, mistaken or fearful lesson. The record keeps
the actor, observer, outcome and source distinct.

The planner keeps logistics (hauling, building what someone decided to build). It stops issuing
responses to crises. `hazards::respond` and its Douse orders go; fire is fought by whoever sees
it, using what they know, or not at all.

## 4. The missing seven are people, not coin flips

Today each missing survivor is a 55 percent roll frozen at world creation, and one found dead is
spawned and killed on the day of discovery. Instead the seven exist from tick 0 as away agents
(the expedition model, D15) at their scatter points. They forage, shelter and get hurt at rates
against the real weather and season where they are; a death happens when it happens, with a
cause, and is recorded then. Searching finds what is actually there: a person, a grave, or an
empty camp with signs of which way they went.

## 5. Word travels in heads

Rumours move from the global `w.rumours` list into memories held by whoever heard them. A
newcomer can only carry word of a missing survivor if their route passed near one. Survivors
pass word on by talking, with distortion in where. Only someone who holds a rumour can act on it.

## 6. Losing is fun

Gates test that the world is coherent (determinism, no stalls, no drift, history only from
rows), not that people live. Starvation, a burned settlement, a dead line are outcomes to record
and show well. The parity plan's rule that a famine regression blocks a checkpoint is replaced:
famine is reported in the soak, and only stalls, drift, determinism and tick cost block.

## 7. The wizard: the player tunes the starting point, not the outcome

The new-game wizard (Phase 17c) is where the player sets up what the organic systems start from,
as DF does with world history and embark and RimWorld does with scenario, landing and ideology.

How long the old world is simulated before the portal opens sets how much history the pasts in
section 1 are drawn from: a decade gives one village and a few hard winters, centuries give
feuds, migrations and family lines the survivors know of. A second depth simulates the valley
beyond the portal before arrival, which is what later leaves ruins and neighbours to find.

Ideology is not a rule set. Each shared belief the player picks is seeded into every survivor as
a belief from section 2, with its source recorded as their old village, so it weights choices
like any other belief and can strengthen, split or die out once the world starts. A group that
arrives sure drink is ruin can still produce a drinker.

Gear is what came through with them, bought from a carrying budget. The scenario sets how many of
the ten come through together. Every choice is stored in the save and appears in History, so a
later reader can see what the world started from.

## 7. A world with a past

The world beyond the portal is not empty land. Before the party arrives, it is simulated for
decades to centuries at a coarse grain, as DF does at world generation. Its peoples have
memories and lore, and found things exist because someone left them there or died there. Real
history and taught history stay separate in the chronicle, so a local may say the hill fort fell
to a curse while History shows a famine winter and a raid, with a teller for every version
between them. When survivors find a ruin, a grave or a rusted axe, the find becomes a memory with
a source, and feeds beliefs like anything else.

The missing seven land among these peoples. Some are taken in, some die, some are never found.
Word of them comes from locals who met them.

## 8. The wizard and the deity

The player tunes the starting point, never the outcome. The wizard sets how long the world is
simulated before arrival, the valley, the landing spot, who comes through, what they carry, and
the beliefs they share. Shared beliefs are seeded as section 2 beliefs with their source in the
old village, so they weight choices and can strengthen, split or die out; a group sure that drink
is ruin can still produce a drinker.

The player also picks one deity who watches over the world. The deity is the storyteller: it
sets how hard chance leans (how often hardship comes, how bunched, how long the quiet lasts) and
never fires an event itself. A fire still starts from lightning or a hearth; a harsh deity makes
dry storms likelier. Each deity must measurably differ, which the first Engine modes did not.

A deity has motives and triggers, as every RimWorld storyteller does. A motive is what it wants
from the world: to test the strong, humble the proud, reward care for the dead, keep things in
balance, or simply be amused. A trigger is something it notices in the real state of the world:
a settlement grown rich and idle, a killing, a broken promise, a grave left untended, a festival,
a long quiet, a famine. When a trigger fires, the deity answers through the world's own
mechanisms, aimed and for a while: lightning likelier over the proudest roof, a winter leaning
colder after a murder, a neighbour's harvest failing so that hunger sends raiders, herds
straying close after a funeral done well. A raid still comes from people who are hungry; the
deity only made them hungry. Every answer is recorded with the trigger that caused it, kept out
of the discovery-respecting view, so History can always explain it. Repeated answers are
patterns people can notice, which is how belief in the deity can arise.

Unlike a RimWorld storyteller, the deity watches every people in the world, not only the
player's party. The same triggers are checked per community: on the map at full detail, and
off it at the coarse grain of section 7, where an answer is a multiplier on that site's rates
for harvest, winter, sickness or herds. Neighbours' fortunes rise and fall under the same deity,
which is where many raids, refugees and alliances will come from. The deity also watches during
the centuries simulated before arrival, so the world's peoples already have lore about it, and
may fear, worship or doubt it, before the party steps through. The wizard therefore asks for the
deity before the history pass runs.

Open question for the maintainer: whether people inside the world can come to sense the deity from patterns
(three hard winters, a fire after a quarrel) and believe in, fear or bargain with it, through the
Phase 21 invention of gods from unexplained events. Recommended, because it ties the storyteller
into the lore. A pantheon of several deities is backlog E13.

## Tests (each over the integrated simulation, not a staged scene)

| Test | Holds when |
|---|---|
| `past_is_recorded_history` | every formative event of every survivor has a chronicle row before tick 0 and a memory whose origin is that row |
| `siblings_diverge` | over many seeds, siblings sharing a formative event take opposite answers in a meaningful share of cases |
| `past_shapes_but_does_not_decide` | a belief shifts the rate of a choice measurably and never to 0 or 100 percent |
| `belief_has_sources` | every belief names at least one event or person |
| `technique_spreads_by_watching` | a technique known by one person can be learned by an attentive witness within days of a success, with seeded cases covering trust, inattention and refusal |
| `fire_fought_without_orders` | with no Douse orders anywhere, some fires are fought and some are not, by who knew how |
| `missing_survivor_dies_of_a_cause` | every death of a missing survivor has a cause and a tick when it happened, not when found |
| `rumour_needs_a_teller` | nobody acts on word of someone without a memory of being told |
| `ideology_is_beliefs_not_rules` | a chosen shared belief shifts choice rates and can be lost by some survivors within a few years |
| `deity_answers_its_triggers` | each deity's answers follow its own triggers in every community, on and off the map, each answer is recorded with its trigger, and none creates an event without a world cause |
| `found_object_has_provenance` | every object found in the world names who made it and why it lies where it does |
| `ideology_is_beliefs_not_rules` | a chosen shared belief shifts choice rates and can be lost by some survivors within a few years |

## Order

Phase 17b (lived pasts), then 17c (a world with a past), then 17d (the wizard), all before
the deep (18). Every later phase (neighbours, fighting, gods) is richer when people and places
carry pasts into it.
