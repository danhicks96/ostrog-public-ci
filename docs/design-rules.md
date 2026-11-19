# Dan's standing design rules

The owner's rules for how Ostrog works. They bind every checkpoint; where the code does not
yet follow one, the gap is named here and in docs/history/PLAN-NEXT.md. Each rule has its decision
number in DECISIONS.md.

## 1. Everything built after the arrival is grown, not set down (D35)

Dan, 2026-10-21: "Everything built 'after' the colonists arrive should be organic, everything
before they arrive are structures which were built (aka prefabs are okay) before them and thus
artifacts and abandoned or maybe some colony hermit crabbed in them elsewhere, but those are the
only potential prefab buildings."

- No fixed footprint or template for anything people build: a house is the floor its builders
  claimed over the ground there is, sized by who needs it, and walled round whatever was
  claimed; it grows when its household does. A hollow or dugout is dug to the room its
  people need. Walls and gates against raids (M5) are the same.
- Prefabs are allowed only for what stood before the colonists came: ruins, the old hamlets'
  artifacts, and places some other colony has moved into.
- Built in C71 (`grounds.rs`): the stockpile is a patch grown from the ground beside home, a
  tile for every two loose stacks, and grows beside the pile when full; the field is grown
  from a seed near home to the mouths there are and grows as they do (`Community::field_tiles`);
  the cellar is dug outward from the stair's foot to the stores there are, the lamp on the
  first tile by the foot; the quarry is sunk where the stone is, nearest home, and its gallery
  follows the stone as far as it goes while stone is wanted.

## 2. Tile by tile means the whole material chain, as in RimWorld and Dwarf Fortress (D36)

Dan, 2026-10-21: "'Grows tile by tile' means they harvest/mine what is needed, craft the
materials, refine them and then place them ... just like in rim and df."

- People cut, quarry or gather the raw material; refine it at a bench (logs sawn to planks,
  stone dressed to blocks, thatch bundled); make fittings as things at a bench (doors, beds,
  tables, chests); carry them to the place; and set each piece. Nothing is built from raw
  material straight onto the ground.
- Built in C67: mining gives rough stone, dressed at a bench; logs are sawn to planks; fittings
  are made at a bench and set.

## 3. Knowledge belongs to people (D37)

Dan, 2026-10-21: "Knowledge/research belongs to those who learn how to do the things or
experiment on how to do things, if those experts get kidnapped or die or something bad happens
to them, their knowledge goes with them, so it is in the interest of a colony to teach others,
even if some traits/memories may produce pushback from some who would prefer to do other
things."

- A craft is something a person knows or does not; arrivals bring the crafts of their trades,
  and a craft can be worked out by trying, slowly and with failures.
- Only those who know a craft do it. When the last who knows it dies, is taken or leaves, the
  colony has lost it, and the record says so.
- Teaching is deliberate (an apprentice at the bench), urged when few know something the
  colony needs, and refused by some for their traits or memories, with the reason recorded.
- Built in C70: every bench recipe asks a craft (`work/recipes.rs` `craft_of`: carpentry,
  dressing stone, smelting, smithing, weaving, potting, brewing, herb lore), and only those
  who know it take the order (`systems/decide.rs`); arrivals bring the crafts of their
  apprenticeships (`opening.rs`) and of their seasoned skills; when nobody of a community knows
  a craft its best hand may try after a day, and a failed try spoils the materials
  (`systems/crafts.rs` `tried`); when the last who knows a craft dies or leaves, the record
  says so (`CraftLost`); one who alone knows a craft the community's benches use offers it to
  the best other hand, who may refuse for their traits (easygoing, solitary) or their memories
  (distrust, a hardship with the teacher), with the reason recorded (`Taught`, `Chose`).

## Object and history rules (D57)

The object system is broad, but it is not a list of disconnected gameplay tokens. A thing has
an identity assembled from its family and form, material, quality, condition, decoration and
history. The complete breadth contract is `docs/OBJECT_CATALOG.md`.

- Ordinary loose materials may be stacked. A named, crafted, worn, broken or otherwise storied
  object keeps its own stable identity even when it changes hands, location or condition.
- Completed objects may be assembled from visible parts, repaired, disassembled or damaged into
  parts. The part links and state changes are simulation facts, not flavour text.
- A history entry must point to a real creation, transfer, use, repair, loss or discovery event.
  The client must never invent a maker, age, owner or reason because a panel needs an answer;
  unknown history is shown as unknown.
- Clicking a person must expose their equipped, worn and carried objects. The first inspection
  slice exists, but the universal record, custody links and full catalog remain planned work.
- Ruins, ancient castles and dungeons may be prefab remains from before arrival. They use the
  same object records and materials as new work, so a broken relic or abandoned workstation can
  be understood, repaired or reused without turning post-arrival construction into a prefab.

## 6. The people are the players (D43)

Dan, 2026-10-22 (#266): "The people in the game are the players. They should change the world,
not find it ready-made." Nothing the survivors could have made themselves is pre-placed; what
was there before them is the ancients', readable as a ruin and said to be theirs. The test for
any starting feature: could the survivors have made this? Then it is not pre-placed. Was it
plausibly left by an earlier people? Then it reads as a ruin. The interest comes from the world
reacting to what the people do.

- Built so far: the staged demo scene (`--stage=demo`, art review only) no longer writes the
  autosave ring (C73), so its preset mine shaft never comes back as the game.
- Open (2026-10-22): the underground is uniform (#267), so there is little to discover by
  digging; creatures do not come into the built place (#270); content is mostly in code, not
  data (#269); area generation is thin against DF and RimWorld (#268). Planned under M6.

## 7. What would a normal person do here? (D56)

Dan, 2026-10-27 (#415): "What would a normal human do if...? ... wandering about when they
don't have a ready supply of food or shelter is probably going to ensure death." Every
behaviour is checked against it. A person dropped into a strange land finds food and a place
to sleep before dark, stays near what they found, keeps their group together, and puts off
talk, play and exploring until tomorrow's food and tonight's shelter are secure. Leisure is
what people do once safe, not instead of getting safe.

- The order that holds at every decision, not only after the arrival: safety, then food (and
  tomorrow's), shelter and warmth before dark, then work that secures the next days, and only
  then leisure (talk, wander, loiter, the hearth, study).
- Built (#415, `systems/decide.rs` `choose`): a person is unsecured when hungry (below half),
  when their community holds under two days' food (`settlement::food_days`), or when night is
  two hours off or come and they know no bed and have no roof overhead. While unsecured,
  leisure weighs a twentieth of its usual; a short store sends those who know where food grows
  to gather it for the store (`act.rs` leaves it on the ground when they were not hungry) and
  weighs food work more; dusk with no bed sends them under the community's roof, else weighs
  the work of shelter more; one far from camp walks back to it. Hungry with no food known,
  they walk back to camp, then to where they ate before, then search round the camp, never on
  from wherever they stand. A secure community talks and wanders as before. Systems that call
  people together outside the decision ask `decide::secured` too: a community short of food,
  or anyone hungry, does not gather for an observance (observances.rs).
- Water is now a real need with thirst, drinking, wells and water-quality knowledge. A person may
  only drink from a source they can reach and know or choose to investigate; the client must not
  imply a quality the simulation does not model. Movement never treats unsupported water or lava
  as walkable ground. Swimming, wading, currents, flooding and drowning belong to the physical
  depth work, and aquatic competence never grants immunity.

## 8. Observation is evidence, not omniscience

Every person or animal acts from an observer-specific ledger: an initial observation followed by
later deltas, each with source, confidence and age. Sight, sound, smell, touch, skill, lighting,
obstruction and attention constrain what enters it. A player inspection can show what that person
knows and why, but cannot silently replace it with world truth.

## 9. Problems become lived lessons

The canonical chain is `problem -> observation -> appraisal -> reaction -> attempted response ->
outcome -> interpretation -> memory, belief, skill or trust change`. Reactions may be helpful,
avoidant, resentful, selfish, reckless or mistaken. Watching someone succeed can teach, but only
through attention, interpretation, trust and willingness; another person may share, withhold,
distort or want someone else to fail. See `docs/design-minds-observation.md`.

## 10. Visual clues must have physical causes

Water colour and animation, wetness, seepage, stains, heat shimmer, frost and pressure marks are
presentation of simulation-backed state, never decorative evidence of an unimplemented mechanic.
Porous blocks can reveal a nearby liquid to one observer before another notices it, but the cue
must come from actual adjacency, permeability, pressure, temperature or fluid state.

## 4. Earlier calls

- A thing with a story does not rot; fire can still take it, and its loss is named (D32).
- One raw food makes one meal, of the food the cook fetched (D33).
- Raiders take only food they have seen, and may come for a renowned thing whose fame reached
  them, looking where the telling put it (D34).
- A home is a personal project, proposed by someone who slept on the ground, in the form their
  kind builds (D29). Moving camp is a proposal people follow or refuse (D30).
- You watch; you do not command. Every community, the colonists' own included, decides and
  builds by the same rules (VISION.md).

## 5. Working rules

- Push the working branch to GitHub after every commit, so GitHub is always in sync (Dan,
  2026-10-21).
- Merge on GitHub once checks pass, without waiting for Dan (Dan, 2026-10-20).
