# Feelings: what is witnessed, what is felt, and what follows

Status: design, 2026-10-23. Unblocks docs/design-leadership.md sections 5 to 8 and, in epic
#376, #281 (personality), #297 (the religious life), #322 (secret societies and betrayal), and
the rest of #360 (prestige against dominance). Built under docs/design-rules.md: the player
only watches, and nothing here commands anyone (rule 4).

## 1. The claim

People in Ostrog already *know* things (perception, memory, beliefs) and *carry* things
(thoughts summed into mood, stress, breakdowns). What they lack is the step between: a
specific felt reaction to a specific thing seen, coloured by who they are, that fades, passes
to those near them, and leans what they choose next. Mood says "I am low"; a feeling says
"I am afraid of the dark since the beast came up through the cellar".

```
witnessed event --appraisal (traits, beliefs, relation)--> feeling (kind, strength, object)
      feeling --decay--> fades;  --contagion--> those near who care about the feeler
      feelings --soft bias--> goal scores in decide.rs; --sum--> thoughts, mood, stress
```

## 2. Feelings are few, named and about something

Six kinds, each with an object (a person, a place, a kind of thing, or nothing):

| Feeling | Rises from | Leans toward |
|---|---|---|
| fear | a beast, a raid, a fall, the dark, a siege seen | home, company, staying in; away from its object |
| grief | a death of someone held dear, a lost craft, a marred rite | the hearth, rites, quiet work; less talk |
| anger | a wrong done to oneself or one's own, a verdict thought unjust | confronting its object; quarrels; feuds |
| joy | a birth, a wedding, a fitting rite, a good harvest, a return | talk, gathering, generosity, teaching |
| awe | the thin place, the deep, a spell lifting, a deed past the ordinary | the spirit domain: rites, the one who did it |
| shame | one's own break, a verdict against oneself, a failure seen by others | withdrawal, amends, avoiding those who saw |

Stored per person as at most eight live feelings (kind, object, strength 0..1, since,
source event). The strongest of a kind wins when there are more than eight.

## 3. Appraisal is the personality (#281)

The same event lands differently on different people. Personality is the set of gains on
appraisal, drawn at birth from the genome (genetics.rs already gives calm) and the trait list,
and nudged by life:

- **temper** (anger gain), **nerve** (fear gain, the inverse of C75's nerve), **warmth** (joy
  and grief gains for others' fortunes), **reverence** (awe gain), **pride** (shame gain);
- each 0.5 to 1.5, so the same siege makes one person afraid and another angry;
- beliefs already lean appraisal: Violence leans fear toward anger; Strangers leans fear of
  raiders; Burial leans grief at a marred rite.

This is #281's "disposition as a modelled layer": not new traits, but how strongly each
person feels each thing.

### Stable preferences and tolerance

Feelings are episodic. People also have slower-changing dimensions that shape whether an action
is satisfying or wearing: enjoyment, tedium, curiosity, novelty-seeking, risk appetite, solitude,
need for order, heat/cold tolerance, noise tolerance, attachment and resentment. These are
weighted inputs to choice, not diagnoses or rigid identities. A person can mask discomfort,
accommodate someone they love, push through a disliked task for a reason, or become more or less
tolerant after repeated outcomes. The source of a change remains a memory, relationship,
condition or recorded event.

## 4. Fading and spreading

- A feeling fades by a fixed share an hour, slower for grief, faster for anger; one renewed by
  its object (the beast seen again) is topped up.
- **Contagion**: each hour, a person within six tiles and awake who thinks well of the feeler
  (opinion above 0.2) takes a fifth of the feeler's strongest feeling, with the same object,
  scaled by their own gain for it. Followers (C77) take more from the one they follow, which
  is design-leadership.md section 5's soft authority. A dominance leader's anger and fear
  spread; a prestige leader's joy does.
- Feelings feed thoughts (one per kind, weighted by strength), so mood, stress and C75's nerve
  read them without change.

## 5. What feelings lean

Never a command (rule 4). Each is a multiplier, 0.7 to 1.4, on the scores decide.rs already
computes: fear on Flee and on going near its object; anger on Confront and Avenge against its
object; grief on Rest and the hearth; joy on Talk and teaching; awe on study at the thin place
and on rites; shame on Loiter and on keeping away from those who saw.

## 6. What it unlocks

- **Leadership (#360)**: prestige is deference given from joy and awe at deeds; dominance is
  deference given from fear. A dominance leader's fear contagion lowers opinion between
  followers, which lowers cohesion into the split that already exists (society.rs).
- **The religious life (#297)**: awe gathers at the thin place and the deep; omens are events
  appraised with awe or fear by those who hold the spirit domain and passed on; a marred rite
  spreads grief and fear; observances that lapse leave the reverent ashamed.
- **Secret societies and betrayal (#322)**: people who share a strong feeling about the same
  object (anger at the judge, awe at the deep) and think well of each other meet apart; a
  society with an object and a grievance can act against it.
- **Breakdowns (#332)**: the strongest feeling at a break picks its kind: fear, flight; anger,
  a fight or a fire; grief, a daze; shame, walking out.

## 7. Build order

1. Feelings stored, appraisal from the events perceive already notes, fading, thoughts fed
   (no behaviour change beyond mood). Guard: the same event, two temperaments, two feelings.
2. Contagion, with followers taking more. Guard: fear spreads through a crowd and stops at
   those who dislike the feeler.
3. Soft biases in decide.rs. Guard: an afraid person keeps nearer home; soak rates within 10%.
4. Prestige and dominance, then omens and observances, then societies, each its own
   checkpoint under epic #376.

Measures for every step: the 600-day soaks' populations, breakdown and judgement counts stay
within a fifth of the last round unless the step is meant to move them (FIXLOG 2026-10-23
C77: a new social pressure soured a colony into splitting before it was tuned).
