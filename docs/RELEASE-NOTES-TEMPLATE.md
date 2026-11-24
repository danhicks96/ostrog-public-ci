# Writing release notes

Release notes are read by someone who runs Ostrog on a second monitor and glances over now and
then. They want to know one thing: **what will I notice differently?** Everything else is
reference material and lives somewhere else.

The shape borrows from the notes people actually read: Factorio's changelog (one line per change,
grouped, plain verbs), RimWorld and Stardew Valley patch notes (what the player sees, not what
the code did), Hades and Dwarf Fortress (a short human intro, a few headline changes, then the
small stuff).

## Length budget

| Part | Budget |
|---|---|
| Headline | 1 line, 12 words or fewer |
| Intro | 2 to 3 sentences |
| Highlights | 3 to 5 bullets, 1 line each |
| Other sections | 8 bullets each at most, 1 line each (2 lines is a smell) |
| Whole page | about 300 words, never more than one screen without scrolling twice |

If a release is bigger than the budget, the answer is a shorter list of the changes a watcher
would notice, not a longer page. Nothing is lost: the details stay in the fix log and the history.

## Rules

1. **Write what a watcher sees.** "Hungry people now walk to the pantry before dark", not "fixed
   the hunger pathing goal".
2. **One change, one line, one verb first.** Added, Changed, Fixed, Removed.
3. **No ticket numbers, PR numbers, work-package codes, file paths, commit hashes or test
   names.** They mean nothing to the reader and age badly.
4. **No private names, links or download prefixes, and nothing about who or what wrote it.**
5. **Group by what the watcher would notice**, in this order, and drop empty sections:
   *Highlights*, *The valley* (world, weather, plants), *The people* (minds, bodies, bonds,
   beliefs), *Settlements* (building, work, trade, raids), *Watching* (camera, feed, panels,
   sounds), *Fixes*, *Performance*.
6. **Fixes are one line each**: what you would have seen go wrong, in past tense. No cause.
7. **Say what to do about saves.** One line under *Saves and updates*, every release.
8. **Be honest about limits.** A "Known limits" line is allowed; the same limits repeated in
   every release are not (see below).
9. Be specific over vague: numbers and examples beat adjectives.

## What does not go in release notes

Install steps, the updater explanation, uninstall steps, privacy text and reporting steps do not
change between releases. They belong on the download page and in a single standing page, not
copied into every release. Release notes link to them in one line.

## Template

Copy the block below, fill it in, delete what is empty. The comment at the top stays out of the
published text.

```markdown
# Ostrog <version> (build <cNN>): <headline, 12 words or fewer>

<2 to 3 sentences, in plain words, about what this release feels like to watch.>

## Highlights

- <the one change you would tell a friend about>
- <...3 to 5 in all>

## The valley
- Added / Changed / Fixed ...

## The people
- ...

## Settlements
- ...

## Watching
- ...

## Fixes
- <what went wrong, past tense, one line each>

## Performance
- <one line, with a number if you have one>

## Saves and updates
<Your <previous version> saves load. / Starting a new valley is recommended because ...>

New here? See the download page for install, update and uninstall steps.
```

## How to write one

1. List what merged since the last release: `git log <last-tag>..main --merges`. Skim the titles
   and the fix log entries.
2. For each, ask: would someone watching notice? If not, leave it out (it stays in the fix log).
3. Rewrite the survivors in watcher words, one line each, using the rules above.
4. Pick the 3 to 5 that make the release feel different and put them in *Highlights*. A change
   appears once: in *Highlights* or in its section, not both.
5. Write the headline last, from the highlights.
6. Check the budget. Cut from the bottom of the list, not the top.
7. Read it as someone who has never seen the repository. Any word they would not know goes.
8. Run the repository hygiene check before opening the pull request.

## Worked example

Built from the changes that merged in the week of 20 to 24 November 2026. It is an example of the
format, not a published release.

```markdown
# Ostrog 0.1.3: Crows, wounds and wild plants

Plants now care where they grow, and people come home hurt in ways you can read. Leave it
running a few days and the valley looks less even.

## Highlights

- A party that comes back from the deep carries a real wound, not just a lower health number.
- Wild plants grow faster by water and slower in shade or a crowd.
- People read crows as an omen only if they were outside and saw them rise.

## The valley
- Changed: berry bushes on bare scree now grow more slowly than the same bush on wet meadow.
- Changed: a plant under a mature tree, or among three or more neighbours, grows more slowly.

## The people
- Added: bodies track moving, handling and sight separately, so a bad hand and a bad eye differ.
- Changed: a realm story is told only when most of those sitting together have not heard it.
- Fixed: people prayed over the same warning again and again.
- Fixed: a grave could be "read" over and over, even after people had forgotten the story.

## Settlements
- Fixed: a lean-to could be started with no free tile to stand it on.

## Saves and updates
Your 0.1.2 saves load.
```

That is about 230 words. Real releases will have more fixes, which is fine up to the budget.
