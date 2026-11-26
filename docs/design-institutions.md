# Roles, institutions and places that communities make (#454)

Design note for issue #454: people take on work and responsibility, form groups and give
places a purpose through what they do, rather than being handed a role list, a government or a
room type. This note says what the code already does toward that, what is missing, the first
slice built on branch `feat-454-institutions`, and what would break the rules.

## What already does part of this

- Leadership is granted, not assigned (C77, `justice.rs`, docs/design-leadership.md). Someone
  who sees a valued deed credits its doer with deference in that domain (feeding and housing,
  keeping safe, the rites, keeping the peace), and it fades. A community follows whoever its
  people defer to most in each domain, so different people lead in different things, and a
  crisis's deeds change who is followed. The one followed in the social domain judges offences.
- A held role with a challenge (#297, `keepers.rs`). The one followed in the spirit is named
  keeper of the observances and the shrine, and keeps it until death, leaving or losing a
  challenge that the grown members decide by standing with one side.
- Organised groups. Secret societies (`societies.rs`) form from shared anger at one person,
  meet apart, can betray and be betrayed. A turned-back challenge leaves a faction
  (`heresy.rs`) that keeps observances apart and can split off as its own community.
- Places with a purpose. A shrine (`shrines.rs`) is proposed by someone who heeds the signs,
  built on the community's ground, kept up by use and lost by neglect. Relics
  (`relics.rs`) make a place where a revered person's thing lies. The stockpile, field, cellar
  and quarry grow from use (`grounds.rs`, design rule 1).
- Customs (`systems/culture.rs`): a rest day for someone who died, food taboos, storing food
  after a famine, the welcome norm, hearth evenings, the way of building. Each comes from an
  event and lapses when it is not kept.
- Visitors with reasons. Newcomers arrive from real places beyond the colony carrying their
  families' tales (`realm/live.rs`, `lore.rs`); raiders come for food they have seen or a
  renowned thing whose fame reached them (D34); caravans trade from real stores (`trade.rs`).

## What is missing

- Work roles are a fixed class list handed out each day. `systems/society.rs` `assign_roles`
  gives each community one forager, builder, farmer, cook, miner, hauler and healer, by best
  skill, and the rest none. Nobody can hold two, share one, drift into one because they did
  the work, or let one go; the healer is picked by cooking skill. The role then makes its
  holder 1.5 times keener on that work and 0.8 times on other work (`systems/decide.rs`), and
  sets their clothes (`view/look.rs`). This is the one place the code still does what #454
  asks it not to do.
- Rooms have no use of their own. A house is a floor and walls (`houses.rs`); the community's
  gathering place is the hearth nearest home (`systems/culture.rs` `hearth_of`), not the one
  people use. There is usually one hearth, so this matters less until there are several.
- Groups other than faith and grudges: no mutual aid, no craft fellowship, no assembly.
- Visitors who come for a place's renown (a shrine's, a well-known hearth's) rather than for
  food or a thing.

## The first slice: a role is the work someone has been doing

Built in `crates/hf-sim/src/duties.rs`; replaces `assign_roles`.

1. Each person keeps a tally of the work they have done lately, by the kind of work: building
   and felling, sowing and harvest, cooking, mining and digging, hauling, tending the hurt.
   Every working minute adds to the kind it belongs to; every day the tally keeps a share of
   itself, so it reflects the last several days.
2. Each day, a person's duties are the kinds they have done at least two hours of lately and
   at least a third as much as their most-done kind. Their role is the most-done one. Several
   people can share a role; one person can hold two duties; someone who stops doing the work
   loses the duty after some days; someone who takes it up in a pinch (the cook is ill and
   someone else cooks) comes to hold it.
3. Duties weigh the choice of work as the role did (keener on work of their duties, a little
   less on the rest), so habits hold, but nobody is given a duty they have not done.
4. The feed records a role taken ("Bren took up cooking work") and a role let go; inspect shows
   the role and the other duties, with the hours behind each, so the player can see why.
5. The healer is whoever has tended the hurt, not the best cook.

## Assumptions (the maintainer can overrule)

- Two hours of a kind of work over the last days (the tally keeps 85 percent a day, so about
  six days' worth) makes it a duty; a third of the top kind makes it a second duty.
- Foraging is not a duty in this slice: it is not a work order, and nobody's role is forager
  until gathering is counted the same way. The forager clothing simply stops appearing.
- The first days have no roles: people choose work by skill and need as before, and roles
  follow from that within two or three days.
- Refusal, rotation by agreement and crisis delegation are left for later slices; habits and
  the tally's fading already let a duty pass from one person to another.

## Later slices

- A duty recognised by others: those who rely on the cook think the worse of them when meals
  are short, and look to them when food is low (the deference model of `justice.rs`).
- Places by use: tally where people eat, drink, talk and work, and let a room become known
  as the community's hall or alehouse by that use; the evening gathering goes to the hearth
  actually used.
- A fellowship of a craft: those who share a duty and think well of each other teach each
  other and pool tools (with design rule 3's teaching).
- Visitors who come for a place's renown, by the same news paths as raiders and newcomers.

## What would break the rules

- A fixed list of classes, or a best-by-skill assignment: that is what this replaces.
- Any duty the person did not do. The tally is of their own work only.
- Knowledge: a duty does not grant a craft; only those who know a craft take its orders
  (design rule 3), as before.
- Rule 7: duties weigh work against work; they do not outrank hunger, shelter, safety or the
  #415 secured gate.
- Determinism: the tally is integer minutes and a fixed daily decay, saved with the person; no
  chance is involved.
