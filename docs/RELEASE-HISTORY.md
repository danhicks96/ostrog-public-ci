# Ostrog release and development history

Ostrog began with a second-monitor idea: a colony that would live its own life while you watched. The inhabitants, not the player, would decide what to eat, where to build, whom to trust, and whether to leave. Dwarf Fortress and RimWorld were useful comparisons for simulation depth and readability, but Ostrog's central difference was that watching, rather than giving orders, was the point. **Ostrog was designed and directed by the maintainer.** See [NOTICE](../NOTICE) for credits and licensing.

## What was released?

The early dates below are **development milestones**, not public launches. The October 14 records mention Linux and Windows packages made during hardening, but do not establish their distribution. The November records document a published Windows **0.1.0 pre-release build** used for validation by November 13. They also record **0.1.1 and 0.1.2 update test builds**; those tests do not establish separate public releases. The **November 19 public GitHub publication was a source snapshot for CI**, not a game or media release. These records do not establish a general public binary launch.

## October 13, 2026: A world that ran but felt empty

The first milestones established a seeded valley with layers above and below ground, navigation between levels, a save format, an event history, and a Godot view. Inhabitants could look for food, sleep, explore, and act on things they knew rather than on a complete map. An automatic camera, an event feed, and an inspection panel gave the watcher ways to follow them.

The first integrated review passed its autonomy checks but failed as something worth watching. Much of the surface was hard to reach, the camera did not reliably stay with busy people, and the world looked empty. Natural ramps improved access, better founding sites and starting supplies gave people room to act, and the camera paid more attention to what inhabitants were doing.

The next milestone gave communities work they could pursue without a player: gathering, hauling, building, mining, growing food, and cooking. It exposed a second problem: once the first house stood, the settlement went quiet. Being able to build a house was not the same as sustaining a community. [Roadmap](history/ROADMAP.md) · [Iteration log](history/ITERATION_LOG.md)

## October 14–15: From a working settlement to a living one

Inhabitants gained memories that could influence later choices, relationships that could change, children and aging, and customs with recorded origins. Communities could accept or turn away newcomers, split, and trade. Excavation opened cellars, caves, and ore deposits. Caravans could leave the map and return with outcomes that remained part of the valley's history. A History view gave the watcher another way to discover what happened without inventing a story to fill gaps.

Long runs exposed failures a short demonstration missed. Settlements starved at year end. Grain piled up because navigation reported that the cellar store was reachable, but the stairs could only be climbed in one direction. Leaving on a caravan could also take too many hands away from a small founding group. The developers adjusted food production and winter storage, made stair traversal and reachability agree, and put limits on who could leave. A save bug and a visible autosave hitch also prompted changes to how the game read existing saves and prepared new ones.

By October 15, the parity plan described a baseline that could sustain a hundred-person settlement over thirty simulated years. That was a development result, not a promise of survival for every world. Work then added more visible life on screen, workshop chains and wear, bodies with distinct injuries, and stress that could build before it appeared as low mood. The first weather pass produced rain in a cellar, and the first stress tuning produced no breakdowns in a forty-day check. Both were reminders to test whether a system made sense in the world, not just whether its code ran. [Iteration log](history/ITERATION_LOG.md) · [October 15 parity record](history/PLAN-PARITY.md)

## October 16 to early November: A past, a point of view, and consequences

The October 16 design described a world where people arrive with lives already lived, and those lives influence future choices without dictating them. A witness and someone who heard the story secondhand need not know or believe the same thing. [Design note on lived pasts and learning](design-organic.md)

Later work applied that principle in concrete ways. Instead of disasters ending as isolated events, a flood could spoil food, draw vermin, and leave someone with a festering bite. Teaching became a social act: someone could withhold knowledge, and a listener's trust could affect whether they accepted it. The New Game map also exposed a simpler kind of false choice: different regions changed the climate but reused the same terrain. The fix made region selection change the generated land as well as the preview.

Timelapse put a different part of the game to the test. Early captures of the same world at the same simulation time could show different rain, and reopening a world started another record rather than continuing its own. The revised feature samples at fixed in-game times and keeps a bounded series for each world. Even presentation needs a reliable clock when watchers use it to compare what happened. [Fix log](history/FIXLOG.md) · [Timelapse guide](TIMELAPSE.md)

## November 6–14: Packaging and the first-session test

The Windows launcher work introduced verification for release packages and a way to stage updates safely. The published **0.1.0** pre-release build was used for a fresh Windows first-session review by November 13. The review found two things an automated check could easily miss: the help panel covered the settlement, and closing the window before the first autosave lost the session. Help moved to the edge, and normal exits began saving eligible running games.

The **0.1.1 and 0.1.2 update tests** showed that installing one update did not prove the next one worked. A successful update left a backup that blocked the next swap. The fix cleared leftovers after success while preserving the backup if it was the only recoverable copy. The camera had a related lesson about what matters to a watcher: a string of wild-animal deaths could pull it away from the new settlement. The event history still kept those deaths, but they no longer drew the camera on their own. [First-session records](https://github.com/danhicks96/ostrog-public-ci/blob/35c0bff2bad0d5889e2312d056b0871f7f1c13f2/docs/history/FIXLOG.md#L2093-L2134)

## November 19–20: Public source and ongoing development

The public repository started with a source snapshot for continuous integration (CI), the automated checks that run against code changes. It is not a game download. Rendered images and audio are not included; CI creates the image inputs it needs for tests from included recipes. Those temporary assets are not cleared for a game release. [Build notes](../README.md) · [Asset status](ASSETS.md)

The first public CI fix put Windows and Linux digest output in the same byte format without weakening what the checks compare. Development then moved into the public repository with two drafts: keeping major events visible when nobody is nearby and explaining breakdowns in the chronicle. These are development changes under review, not released features. [CI fix](https://github.com/danhicks96/ostrog-public-ci/pull/1) · [Camera draft](https://github.com/danhicks96/ostrog-public-ci/pull/2) · [Breakdown draft](https://github.com/danhicks96/ostrog-public-ci/pull/3)

Ostrog remains pre-release. The source includes the simulation, Godot client, and Windows launcher. Ten people still come through a portal; three begin together, while the others' fates have to be discovered. The valley builds, remembers, and changes without player orders. The [development backlog](BACKLOG.md) shows what developers can take on now and what must wait.
