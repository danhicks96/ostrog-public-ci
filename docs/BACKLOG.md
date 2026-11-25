# Ostrog development backlog

This is the public backlog for developers working on Ostrog. `OST-` IDs are stable: use them in public claims and PRs, and keep them if an entry moves sections or changes state. Only link public PRs and published release pages here. Maintain any crosswalk to previous trackers separately; it does not belong in this file or in public PR metadata.

## Claiming and closing work

1. Pick a `Ready` item. Open a small public PR against `main` changing only that entry to `Claimed` and adding your public name or initials. A claim wins **only when its PR lands**. A draft, open PR, or mention is not a reservation. If someone else already has a claim PR open, pick another item rather than starting a duplicate.
2. Once the claim lands, the listed developer is responsible for the implementation. Put the backlog ID in the implementation PR and add a public PR reference to the entry when it goes into review. Do not start or continue a second implementation of claimed work. Transfer or release a claim with a landed backlog update before another developer starts.
3. Keep unrelated work separate. Check existing claims and public PRs for overlapping modules or files before starting. Share implementation results and unresolved checks in the public PR, without private references or data.
4. A verified public implementation PR already in progress when an item is added may be listed directly as `In review`, with its developer and public PR link. This does not bypass claims for new work. Do not duplicate an implementation marked `Claim pending` while its handoff is being reviewed.
5. Move to `Landed` only after the implementation PR merges and its stated acceptance checks are met. Mark `Released` only after verifying the change is included in an actual published release, and link that release. A branch, green CI run, merge or unreleased tag is not a published release.

**States:** `Ready` (available to claim), `Claim pending` (a claim or port is in progress; not a landed claim, but do not duplicate it), `Claimed` (claim landed), `In review` (claimed work has an implementation PR), `Waiting on dependencies` (required code or measurement is not yet available), `Landed` (accepted and merged), `Released` (verified in a published release), `Release-gated` (a named capability must ship before work starts), `Stretch` (not ready for a claim). Record an owner and a public PR link when verified. `None` means no owner or PR; `Pending` means the public record is not yet available.

## Milestones and parallel work

The milestone labels group related work; they do not promise a release date. Each entry names its own acceptance checks. Pending ports and dependent entries are not open for duplicate work.

### OST-001 · Give major personal events more weight in camera selection

**Milestone:** Make important events easier to follow · **State:** In review · **Developer:** Ostrog CI (OC) · **Public PR:** [#2](https://github.com/danhicks96/ostrog-public-ci/pull/2)

An unwitnessed major event can be discounted enough that one busy person elsewhere draws the camera instead. Give major deaths, captures, breakdowns and exiles full presence even in an empty cell. Keep the existing weights for routine events, fires and departing caravans.

**Acceptance:** Place a major death in an empty cell 30 tiles from one busy person. The director selects the death. Existing director tests and the generated reference check pass. Simulation state is unchanged. Long-run coverage is tracked separately in OST-003.

### OST-002 · Clarify what the furnace would add

**Milestone:** Clarify workstation roles · **State:** Claim pending · **Developer:** Pending · **Public PR:** Pending

A source review is already available for reconciliation. The Smelter and Kiln have station recipes; the current recipe table assigns none to `Furnace`. The variant still has view and house-logic references. This is a documentation decision, not permission to remove a variant or add a station.

**Acceptance:** Check recipes, building placement and compatibility references. Record the decision with concrete source evidence. If there is a distinct use, describe a separate build slice and its tests. Otherwise propose retaining the Furnace variant without new recipes, preserving its existing uses and enum position. No recipe, gameplay or save-format change is needed for this investigation.

### OST-004 · Measure major events the camera misses

**State:** Landed · **Developer:** Wren (WR) · **Public PR:** [#555](https://github.com/danhicks96/ostrog-public-ci/pull/555)

Merged in #555. Not yet in a published release. Add a read-only headless measurement for whether the camera showed a major event, split by event kind. This can be developed independently of the scoring change in OST-001; coordinate if a fixture touches the same test file.

**Acceptance:** Define the sampling interval, time window and same-level distance used to mark an event as shown. Test one major death within view and one outside view, and state how events without a location are counted. Report raw counts and shares by kind without changing simulation state.

### OST-005 · Record what a breakdown was over

**Milestone:** Make important events easier to follow · **State:** Landed · **Developer:** Ostrog CI (OC) · **Public PR:** [#3](https://github.com/danhicks96/ostrog-public-ci/pull/3)

The existing breakdown event now carries an optional explanation, not a second event. Choose the strongest qualifying fear, grief, anger or shame (strength 0.3 or higher), then the heaviest active negative thought other than arrival, then the lowest need below 0.2. A new breakdown with no qualifying cause is unknown; old records can lack the field. Only a recorded feeling can identify another person, and the viewer must already know them for the explanation to name them.

**Acceptance:** Preserve the original headline, event ID, participants and simulation behavior. Focused tests cover known and unknown explanations, undiscovered subjects and old records. The pre-merge comparison for the corrected implementation matched the baseline on seeds 1–4 over 60 game-days, with 59 of 59 breakdowns recording a known cause (100%).

### OST-006 · Reconstruct on-map bodies from saved state

**Milestone:** Preserve what the viewer sees across saves · **State:** Claim pending · **Developer:** Pending · **Public PR:** Pending

The read-model candidate is pending review. Do not open a second implementation. Rebuild local bodies awaiting rites and unrested bodies supported by a recorded death; do not wire the client load path in this slice.

**Acceptance:** Round-trip staged local and off-map deaths with and without surviving rite entries. A stale saved agent tile must not override a recorded death tile; check the unrested timestamp separately. Do not restore bodies for rite-caused conditions, including marred rites; omit ambiguous unrested history. For a pending local rite, distinguish missing history from an invalid or contradictory row. Keep laid-to-rest and undiscovered bodies out.

### OST-008 · Count visible activity in the camera frame

**Milestone:** Make important events easier to follow · **State:** Claimed · **Developer:** Iris (IV) · **Public PR:** Pending

Reconcile the existing read-only frame tally rather than starting another. Count whether visible people are doing something readable or standing still, separate from whether a major event was missed.

**Acceptance:** Stage a frame with working, idle, sleeping and off-frame people. Check the counts and denominators, including an empty frame. The measurement does not modify the simulation.

### OST-010 · Write Catch Up counts in plain language

**Milestone:** Make important events easier to follow · **State:** Landed · **Developer:** Iris (IV) · **Public PR:** [#526](https://github.com/danhicks96/ostrog-public-ci/pull/526)

Reconcile the existing summary change. The opening line should summarize important events in words without filling it with routine labor.

**Acceptance:** Build a summary with two deaths, one birth and many hauling rows. It names the births and deaths with the right counts, omits hauling in the headline, and leaves the raw summary counts available to existing consumers.

### OST-1206 · Repay a good turn with a real meal, and not across a feud

**Milestone:** Make the valley feel alive · **State:** Landed · **Developer:** Sage (SG) · **Public PR:** [#633](https://github.com/danhicks96/ostrog-public-ci/pull/633)

Repaying a good turn moves hunger numbers, not food. Hand over a real meal. Tracking issue: [#610](https://github.com/danhicks96/ostrog-public-ci/issues/610).

**Acceptance:** A carried meal changes hands with no food created or used twice; a stack splits by one; feud, opinion and distance each block the repayment; with no food in reach nothing is repaid. Existing social tests and hosted CI pass; no save-format change.

### OST-014 · Fold repeated Catch Up details before limiting the list

**Milestone:** Make important events easier to follow · **State:** Landed · **Developer:** Mara (MR) · **Public PR:** [#7](https://github.com/danhicks96/ostrog-public-ci/pull/7)

The Catch Up read model folds viewer-visible events of Notable severity or higher by existing repeat keys before applying the detail cap. Named changes, raw counts and timelapse markers stay separate and unchanged. This does not wire the panel (OST-011) or complete the census (OST-013). Measure latency on a large chronicle before wiring the panel.

**Acceptance:** In a summary with many repeated suspicions and several distinct raids, the suspicions group without crowding the raids out before folding. Each folded line keeps the newest history ID as its navigation anchor, and stores its earliest and latest event ticks separately. Verify those ticks with out-of-order input, preserve distinct named changes, and check that the timelapse still receives the same raw markers.

### OST-1100 · Read what a person is seen doing without changing anything

**Milestone:** Make the inner life visible and trustworthy · **State:** Claimed · **Developer:** Tamsin (TS) · **Public PR:** Pending

Add a pure `ActKind` / `act_of` helper in `mind` that classifies a person's current activity as an observer could see it, with no caller wired yet. Check current owners and overlapping public work before starting; do not touch unrelated behaviour.

**Acceptance:** Three focused tests cover an observer who can see the person, one who cannot, and a sleeper; the helper changes no simulation or saved state. It must not duplicate or replace the camera tally in OST-008.

### OST-1101 · Add a Lonely thought when the social need runs low

**Milestone:** Make the inner life visible and trustworthy · **State:** Claimed · **Developer:** Tamsin (TS) · **Public PR:** Pending

Add a `Lonely` thought, appended to the thought list, that appears while an awake person's social need is low and fades when it recovers. Check current owners and overlapping public work before starting; do not touch unrelated behaviour.

**Acceptance:** An awake person with a low social need gets the thought and it clears after recovery; a sleeper does not. The thought enum gains the new variant at the end, old saves load unchanged, and the Catch Up and breakdown-explanation ranking still pass.

### OST-1102 · Draw Gourmet and Tidy traits from the genome

**Milestone:** Make the inner life visible and trustworthy · **State:** Claimed · **Developer:** Tamsin (TS) · **Public PR:** Pending

Draw the Gourmet and Tidy traits from the genome so that people can have them. Check current owners and overlapping public work before starting; do not touch unrelated behaviour.

**Acceptance:** Over 500 genomes each trait appears on at least 10 people and no person carries more than the existing trait cap. Existing trait discriminants and determinism hold, and hauling responds to a matched disposition.

### OST-1103 · Mark skill milestones with a line and a thought

**Milestone:** Make the inner life visible and trustworthy · **State:** Claimed · **Developer:** Tamsin (TS) · **Public PR:** Pending

Add a daily skill-tier check that writes one history line and one positive thought per tier, backed by a saved high-water mark. Check current owners and overlapping public work before starting; do not touch unrelated behaviour.

**Acceptance:** A person crossing a tier gets exactly one line and thought; reloading a save never repeats it, and a loaded old save initialises silently.

### OST-1104 · Let a memory be about an item or a community

**Milestone:** Make the inner life visible and trustworthy · **State:** Claimed · **Developer:** Tamsin (TS) · **Public PR:** Pending

Append `Item` and `Community` subjects to memory without changing the existing ones. Check current owners and overlapping public work before starting; do not touch unrelated behaviour.

**Acceptance:** Existing subjects keep their saved positions, old saves load with the same digest, and a new subject round-trips through save and load. No feeling or gameplay consumer is added here.

### OST-1105 · Parents and grandparents are proud when a child comes of age

**Milestone:** Make the inner life visible and trustworthy · **State:** Claimed · **Developer:** Tamsin (TS) · **Public PR:** Pending

Add a pride thought, appended to the thought list, for living parents and grandparents when a child comes of age. Check current owners and overlapping public work before starting; do not touch unrelated behaviour.

**Acceptance:** A living parent and a living grandparent each get one thought, a person related by two paths gets one, and deceased or unknown kin get none. The enum order and old saves hold.

### OST-1106 · Make a witnessed death require a view of it

**Milestone:** Make the inner life visible and trustworthy · **State:** Claimed · **Developer:** Tamsin (TS) · **Public PR:** Pending

Require a SawDeath witness to be awake with a clear line of sight, not merely within ten tiles. Check current owners and overlapping public work before starting; do not touch unrelated behaviour.

**Acceptance:** A sleeper and a person behind a wall do not witness; a person with a clear view does; the victim is excluded. Keep the existing awake/line-of-sight helper and its API, and keep grief news separate (see public #313).

### OST-471 · Mark unused observation and problem kinds as reserved

**Milestone:** Make the inner life visible and trustworthy · **State:** In review · **Developer:** Tamsin (TS) · **Public PR:** [#644](https://github.com/danhicks96/ostrog-public-ci/pull/644)

This is the reserved-marking slice of the tracking issue on unused problem and observation kinds (public #189). Add a `RESERVED` list and a doc comment on each variant that nothing writes yet, naming what would wire it, plus a live/reserved table in the minds design note. Do not wire any kind, delete or reorder a variant, or change a label, save or simulation behaviour. Verify each listed variant against every current producer and consumer before listing it.

**Acceptance:** A test fails if a kind listed as reserved is also written by perception or learning. Existing tests, the enum-order check and the generated reference check pass. Saved state is unchanged.

### OST-901 · Count births per community, not per valley

**Milestone:** Make the valley feel alive · **State:** Landed · **Developer:** Sol (SL) · **Public PR:** [#634](https://github.com/danhicks96/ostrog-public-ci/pull/634)

Raiders and strangers count toward the crowd rule that halves births. Count per community instead. Tracking issue: [#587](https://github.com/danhicks96/ostrog-public-ci/issues/587).

**Acceptance:** Staged community of 30 with a band of 15 near camp: births use the uncrowded chance. Focused tests and hosted CI pass.

### OST-902 · Retire the unreachable cave-in event

**Milestone:** Make the valley feel alive · **State:** Landed · **Developer:** Sol (SL) · **Public PR:** [#635](https://github.com/danhicks96/ostrog-public-ci/pull/635)

`CaveIn` is never pushed; collapses are `Collapse`. Remove its dead arms and keep the variant for save order. Tracking issue: [#588](https://github.com/danhicks96/ostrog-public-ci/issues/588).

**Acceptance:** Existing tests build `Collapse` rows; only the definition, narration and two tables mention `CaveIn`; no digest change. Focused tests and hosted CI pass.

### OST-903 · Rain wets people outdoors; a roof or hearth dries them

**Milestone:** Make the valley feel alive · **State:** Landed · **Developer:** Sol (SL) · **Public PR:** [#636](https://github.com/danhicks96/ostrog-public-ci/pull/636)

Add a saved wetness state per person. Nothing reads it yet. Tracking issue: [#589](https://github.com/danhicks96/ostrog-public-ci/issues/589).

**Acceptance:** After two game-hours of steady rain an outdoor person is at least 0.6 wet, an indoor one 0, and a soaked one beside a lit hearth returns to 0 within two hours. Save round-trip covers it. Focused tests and hosted CI pass.

### OST-904 · Add mood-term keys to the alive report

**Milestone:** Make the valley feel alive · **State:** Landed · **Developer:** Sol (SL) · **Public PR:** [#638](https://github.com/danhicks96/ostrog-public-ci/pull/638)

Report what mood is made of across a colony. Report only; simulation digest unchanged. Related tracker: [#179](https://github.com/danhicks96/ostrog-public-ci/issues/179). Tracking issue: [#590](https://github.com/danhicks96/ostrog-public-ci/issues/590).

**Acceptance:** A built person's four mood terms clamp to the stored mood after one needs pass; the report prints the new keys. Focused tests and hosted CI pass.

### OST-905 · Let the last knower of a craft carve it

**Milestone:** Make the valley feel alive · **State:** Landed · **Developer:** Sol (SL) · **Public PR:** [#639](https://github.com/danhicks96/ostrog-public-ci/pull/639)

An old or ailing sole knower of a craft their community uses carves the way of the craft before it is lost. Tracking issue: [#591](https://github.com/danhicks96/ostrog-public-ci/issues/591).

**Acceptance:** A staged community with one old smelter carves a smelting engraving within five days; with two smelters, none; old saves load with no craft. Focused tests and hosted CI pass.

### OST-906 · Gate fire sightings on line of sight

**Milestone:** Make the valley feel alive · **State:** Landed · **Developer:** Sol (SL) · **Public PR:** [#640](https://github.com/danhicks96/ostrog-public-ci/pull/640)

A fire behind a hill is known only by sight or by smoke already observed. Tracking issue: [#592](https://github.com/danhicks96/ostrog-public-ci/issues/592).

**Acceptance:** A fire behind a wall 10 tiles off is responded to by someone with a fresh smoke observation and by no one else. This slice is gated on the smell/smoke observation being available on main. Focused tests and hosted CI pass.

### OST-907 · Do not re-form a custom from the same cause

**Milestone:** Make the valley feel alive · **State:** Landed · **Developer:** Sol (SL) · **Public PR:** [#641](https://github.com/danhicks96/ostrog-public-ci/pull/641)

A custom for a dissolved community is formed and lapsed again every dawn. Skip dissolved communities and never re-form from an origin event that already produced one. Tracking issue: [#593](https://github.com/danhicks96/ostrog-public-ci/issues/593).

**Acceptance:** A staged split forms one custom; after everyone leaves there is exactly one formed and one lapsed row over 30 days; a new death can still found a new rest day. Focused tests and hosted CI pass.


### OST-1300 · Hold duties only once grown

**Milestone:** Keep daily life believable · **State:** Landed · **Developer:** Ostrog CI Sim (OS) · **Public PR:** [#625](https://github.com/danhicks96/ostrog-public-ci/pull/625)

`duties::daily` names a role for anyone with enough recorded minutes, so a child who forages can be listed as a forager. Hold no duty for a member who is not of age; keep counting and fading their minutes so the role arrives on their first morning of age. Observance leadership is a separate entry (OST-1301).

**Acceptance:** A six-year-old with forager minutes holds no role, no held duty and no role-taken record. The same person aged sixteen takes the role the next morning. Adults' behaviour and the existing duty tests are unchanged.

### OST-1301 · Let only someone of age lead an observance

**Milestone:** Keep daily life believable · **State:** Landed · **Developer:** Ostrog CI Sim (OS) · **Public PR:** [#626](https://github.com/danhicks96/ostrog-public-ci/pull/626)

An observance leader can currently be a child, whether chosen as the followed leader or as the devout fallback. Require the leader to be of age in both paths. Do not combine with duty gating (OST-1300) or with the burial-observance entry.

**Acceptance:** The youngest eligible age leads; a child-only community's observance lapses; an awake adult leads when present. Existing observance tests pass.

### OST-1302 · Leave a fallen shrine's material on its tile

**Milestone:** Keep daily life believable · **State:** Landed · **Developer:** Ostrog CI Sim (OS) · **Public PR:** [#627](https://github.com/danhicks96/ostrog-public-ci/pull/627)

When a worn shrine is removed the loss is recorded but no material item remains. Leave the stone or timber of the shrine on its tile. Keep the change inside the wear step.

**Acceptance:** Stone and timber shrines each leave matching material and exactly one loss record. A burned shrine leaves none, and repeated steps add no duplicate items. The simulation digest change is explained in the PR.

### OST-1303 · Do not foster a child whose parent is away with a caravan

**Milestone:** Keep daily life believable · **State:** Landed · **Developer:** Ostrog CI Sim (OS) · **Public PR:** [#628](https://github.com/danhicks96/ostrog-public-ci/pull/628)

The fostering check recognises only a parent living on the map, so a parent away with a caravan or journey is treated as gone. Count a living parent or foster who is away on a trip as present. Keep this separate from childhood age gates and from kin-first fostering.

**Acceptance:** A child whose parent is away on a staged caravan is not fostered; a dead or permanently departed parent still is. Existing fostering tests pass. Frequency of wrong fostering is reported separately, not claimed here.

### OST-1304 · Count why a house is waiting for a table

**Milestone:** Measure the simulation · **State:** Landed · **Developer:** Ostrog CI Sim (OS) · **Public PR:** [#629](https://github.com/danhicks96/ostrog-public-ci/pull/629)

Add a read-only example that reports, for each house waiting for a table, the first settlement decision that blocks it (open walls, floor-order pressure, no planks). The probe must mirror the live posting branch and change no simulation state.

**Acceptance:** Test open walls, floor-order pressure and no planks. Each house is counted once per sample, and the probe agrees with the posting branch. A short smoke run is reported before any longer run.

### OST-1305 · Census of wading steps at crossings

**Milestone:** Measure the simulation · **State:** Landed · **Developer:** Ostrog CI Sim (OS) · **Public PR:** [#630](https://github.com/danhicks96/ostrog-public-ci/pull/630)

Add a read-only census of steps taken through water, per tile and per person, using the live stepping rule and an eligible person-day denominator. It informs the crossing decision (tracked separately) and changes no movement.

**Acceptance:** A fixture with one person crossing one tile counts one step. Water-to-water steps are distinguished from entry and exit. Output states its denominator and time window. Simulation state is unchanged.

### OST-1306 · Foster an orphan to kin before whoever likes them best

**Milestone:** Keep daily life believable · **State:** Landed · **Developer:** Ostrog CI Sim (OS) · **Public PR:** [#652](https://github.com/danhicks96/ostrog-public-ci/pull/652)

Fostering picks the adult the orphan likes best. Prefer kin of the same community first: a living grandparent, then a parent's sibling, then an adult sibling. Ties go to opinion, then the lower id. Non-kin keeps the old rule. Deterministic, no new randomness. Keep this separate from the caravan-away check (OST-1303).

**Acceptance:** Each kin tier beats a better-liked non-kin adult, tiers rank in the order above, and with no kin the old choice is unchanged. The simulation digest change, if any, is explained in the PR.

### OST-1307 · Find a free seat

**Milestone:** Make the valley feel alive · **State:** Landed · **Developer:** Ostrog CI Sim (OS) · **Public PR:** [#651](https://github.com/danhicks96/ostrog-public-ci/pull/651)

Stools and chairs are built, but nothing can look for a free one. Add a pure `houses::free_seat` returning the nearest finished stool or chair on the level that nobody else holds or stands on, optionally beside a table, and reachable. It reserves nothing and has no caller yet; using it is tracked separately.

**Acceptance:** Tests cover nearest-first, beside-table, a reserved seat, an occupied seat, and an unreachable seat. No simulation state changes.

### OST-1308 · Census of postings aimed at unseen targets

**Milestone:** Measure the simulation · **State:** Landed · **Developer:** Ostrog CI Sim (OS) · **Public PR:** [#666](https://github.com/danhicks96/ostrog-public-ci/pull/666)

Add a read-only example that watches each hourly planning pass and reports how many hunt and mine postings point at something nobody in the community has seen: a hunted animal not seen by any living member within two days, or a mine cell outside the explored set. It also reports the deepest level mined below home and food per head and ore in store at fixed days. It informs a later planner decision and changes no behaviour.

**Acceptance:** A test flags a hunt for an unseen animal and not for one that was seen. The probe reuses the live planner state and changes nothing. A short smoke run is reported before any long run.

### OST-1309 · Census of each house as a room

**Milestone:** Measure the simulation · **State:** Landed · **Developer:** Ostrog CI Sim (OS) · **Public PR:** [#667](https://github.com/danhicks96/ostrog-public-ci/pull/667)

Add a read-only example that reports, per house at fixed days, floor tiles, beds and sleepers, seats, tables, chests, lamps, hearths, main wall material and loose stacks on the floor, with the median and spread of floor per sleeper and fittings per sleeper. It prints the numbers a later room score will be compared against and does not depend on the room-score work in OST-1402.

**Acceptance:** A test counts a staged house exactly. The example only looks at the world and changes no state.

### OST-1310 · Read a body's moving, manipulation and sight

**Milestone:** Make the valley feel alive · **State:** Landed · **Developer:** Ostrog CI Sim (OS) · **Public PR:** [#672](https://github.com/danhicks96/ostrog-public-ci/pull/672)

Add a pure `Body::capacities` that folds a body's parts and wounds into three numbers: moving, manipulation and sight. It reads the body only and changes nothing. Like the free-seat finder (OST-1307) it has no caller yet; each consumer is tracked as its own slice.

**Acceptance:** Tests cover a whole body, a damaged leg or arm and an injured eye, each lowering only its own number, and values stay in range. Simulation state and digests are unchanged.

### OST-1311 · A party that comes up hurt from the deep carries a real wound

**Milestone:** Keep daily life believable · **State:** Landed · **Developer:** Ostrog CI Sim (OS) · **Public PR:** [#673](https://github.com/danhicks96/ostrog-public-ci/pull/673)

A party that comes back up hurt from below is only marked injured for two days, with nothing to heal, scar or tend. Give each hurt member one deterministic wound (part, kind and severity drawn from a stream keyed by the seed, the taken child and the person), recorded with the cause. Do not change who is hurt or how often.

**Acceptance:** A hurt party member comes up with exactly one wound and a cause naming the child. The same seed always gives the same wounds. Saves written before this change still load. The simulation digest change is explained in the PR.

### OST-1312 · Read crow omens only when someone outdoors sees them rise

**Milestone:** Keep daily life believable · **State:** Landed · **Developer:** Ostrog CI Sim (OS) · **Public PR:** [#674](https://github.com/danhicks96/ostrog-public-ci/pull/674)

A crow omen is read wherever the crows are, whether or not anyone could see them. Read it only when someone outdoors sees the crows rise. Keep this to the crow reading and change no other omen. A behaviour measurement is reported before this merges.

**Acceptance:** Tests show an omen read with a witness outdoors, none with only indoor witnesses or nobody, and unchanged behaviour for other omens. The measurement and any digest change are stated in the PR.

### OST-900 · Count a grown tree's shade in the heat

**Milestone:** Make the valley feel alive · **State:** Landed · **Developer:** Sol (SL) · **Public PR:** [#609](https://github.com/danhicks96/ostrog-public-ci/pull/609)

Heat wears people down the same whether they stand under a grown tree or on bare ground. Halve the excess heat for someone shaded by a mature tree. Tracking issue: [#541](https://github.com/danhicks96/ostrog-public-ci/issues/541).

**Acceptance:** Two awake people outdoors at 38 °C, one beside a mature tree and one on open ground: the shaded one's comfort is at least 0.1 higher and only the other holds `Hot`. Focused hazard tests and hosted CI pass; no save-format change.

### OST-505 · Pray over a warning once

**Milestone:** Make the valley feel alive · **State:** Claimed · **Developer:** Ostrog CI Story (OT) · **Public PR:** Pending

A warning sign stays open five days and the devout set out to pray over it every few hours, so one warning sends the same person to the shrine again and again. Remember the signs each person has prayed over (bounded to 8) and do not send them for the same sign twice. Grief and loss are unchanged.

**Acceptance:** One open bad sign sends each devout member to pray once over five days; a second bad sign sends each a second time. A FIXLOG entry is added.

### OST-506 · Tell a realm story only when most sitters lack it

**Milestone:** Make the valley feel alive · **State:** Claimed · **Developer:** Ostrog CI Story (OT) · **Public PR:** Pending

One newcomer in the circle is enough for the whole hearth to hear the same battle again. Count a realm story as fresh only when at least half of the other sitters lack it, and pick the story the most sitters lack.

**Acceptance:** Eight sitters, seven holding a story: it is not told that evening. With four lacking it, it is told and all four hold it. The existing stranger-tells-their-peoples-story test still passes. A FIXLOG entry is added.

### OST-165 · Read a grave once, even after the account of it is forgotten

**Milestone:** Make the valley feel alive · **State:** Claimed · **Developer:** Ostrog CI Story (OT) · **Public PR:** Pending

A person keeps at most 24 accounts, and a grave reading is the first kept account to go, so the same person reads the same grave again within weeks. Give each person a bounded list (64, oldest out) of the deaths whose graves they have read, and skip those graves in the reading step. The account limit is unchanged.

**Acceptance:** A newcomer beside a grave reads it once; after 30 told accounts push the reading out, ten more hours there give no second reading. The existing "read by one who came after, and the reading can be wrong" test still passes. A FIXLOG entry is added.

### OST-136 · Show the dead in the People panel

**Milestone:** Make the valley feel alive · **State:** Claimed · **Developer:** Ostrog CI Story (OT) · **Public PR:** Pending

The People panel lists only the living, so someone followed for years disappears on death. Add a "Remembered" tab beside "Living", filled from the simulation's list of the dead, with Inspect only and a filter (this season, this year, all).

**Acceptance:** A built list of three dead rows fills the tab with three Inspect buttons and no Follow, and the "this season" filter keeps one. A FIXLOG entry is added if behaviour changes.

### OST-130 · Count the first-days grace from the latest party

**Milestone:** Make the valley feel alive · **State:** Claimed · **Developer:** Ostrog CI Story (OT) · **Public PR:** Pending

Every "first days" grace (newcomer grace, bands not yet knowing of the valley, the deity's first day, fate's settled days) counts from the first portal, so a party arriving years later would get none. Add the tick of the latest party's arrival (zero on old saves, meaning the first portal) and count those four graces from it. Calendar years keep counting from the first portal. Nothing sets the new tick yet.

**Acceptance:** With the latest arrival set to today in a world opened 400 days ago, no band sets out within four days and the calendar year is unchanged. The world digest is unchanged. A FIXLOG entry is added.

### OST-357 · Count consecutive nights on the ground

**Milestone:** Make the valley feel alive · **State:** Claimed · **Developer:** Ostrog CI Story (OT) · **Public PR:** Pending

A week sleeping on the ground weighs the same as one night, because the thought is refreshed rather than stacked and nothing remembers how many nights in a row a person has slept rough. Give each person a count of consecutive rough nights (reset by a night in a bed) and make the slept-on-the-ground thought weigh a little more for each night in a row, up to a cap.

**Acceptance:** Seven rough nights in a row weigh more than one; a night in a bed resets the count. A FIXLOG entry is added and any simulation digest change is explained in the PR.

### OST-354 · Every house gets a table, two stools and a chest unasked: build furniture only when someone takes it up

**Milestone:** Make the valley feel alive · **State:** Claimed · **Developer:** Ostrog CI (OX) · **Public PR:** Pending

The planner posts tables, stools and chests automatically. Instead leave furnishing suggestions that a person with order at least 0.6 adopts. Record a Furnish project with its adopter and retain the real production and placement chain. Tracking issue: [#130](https://github.com/danhicks96/ostrog-public-ci/issues/130). Intake review: Initially adopt for order at least 0.6 alone. The MakeFine and AteWithoutTable alternatives are added only with their defining slices; household ownership retains the stated fallback.

**Acceptance:** Test `crates/hf-sim/tests/suggestions.rs::a_table_is_built_only_when_someone_takes_it_up`. Test `suggestions.rs::an_orderly_householder_takes_up_the_table`. Done when: both pass, and `builders_census` shows furniture ≥ 90% decided by a person.

### OST-355 · A house is the colony's, raised for a figurehead: let a household take up its own, one each at a time

**Milestone:** Make the valley feel alive · **State:** Claimed · **Developer:** Ostrog CI (OX) · **Public PR:** Pending

Home proposals use a community figurehead and one community-wide construction slot. Let a household member with a relevant want propose its own home, size it for the household and record its owners. Allow one active home per household and at most two per community. Tracking issue: [#133](https://github.com/danhicks96/ostrog-public-ci/issues/133).

**Acceptance:** Test `crates/hf-sim/tests/suggestions.rs::the_one_who_took_up_the_house_sleeps_in_it`. Test `suggestions.rs::two_households_may_build_at_once_but_not_three`. Done when: both pass, and seeds 1-4 at 300 days show no rise in rough nights against today's `sleep_rough` census.

### OST-358 · After nights on the ground with no bed to be had, nobody puts up a shelter for themselves: let them

**Milestone:** Make the valley feel alive · **State:** Claimed · **Developer:** Ostrog CI (OX) · **Public PR:** Pending

Someone repeatedly sleeping rough cannot put up a shelter. Offer RaiseShelter after at least two rough nights when no free reachable bed is known, food is secure and no owned sleeping-place order is unfinished. Reuse the existing lean-to site search and post an owned material-backed construction order. Tracking issue: [#144](https://github.com/danhicks96/ostrog-public-ci/issues/144).

**Acceptance:** Test `crates/hf-sim/tests/lean_to.rs::three_nights_on_the_ground_with_logs_and_hides_at_hand_raise_a_lean_to`. Test `lean_to.rs::nobody_raises_a_lean_to_beside_a_free_bed_they_know`. Test `lean_to.rs::a_hungry_camp_gathers_food_before_poles`. Done when: all three pass, and the `sleep_rough` census on seeds 1-4 at 300 days shows rough nights at least halved.

### OST-359 · A lean-to someone built for themselves would go to whoever reaches it first that night: make it theirs

**Milestone:** Make the valley feel alive · **State:** Claimed · **Developer:** Ostrog CI (OX) · **Public PR:** Pending

A personally built lean-to could be used by whoever arrives first. Carry the owner of a completed sleeping-place construction order onto its structure so the own-bed and other-owner preference rules apply. Tracking issue: [#147](https://github.com/danhicks96/ostrog-public-ci/issues/147).

**Acceptance:** Test `crates/hf-sim/tests/lean_to.rs::a_lean_to_is_its_builders_bed_and_others_do_not_take_it`. Done when: it passes.

### OST-361 · Nothing measures whether people sleep in the same bed from night to night: extend the sleep_rough census

**Milestone:** Make the valley feel alive · **State:** Claimed · **Developer:** Ostrog CI (OX) · **Public PR:** Pending

The rough-sleep census does not measure habitual beds or contention. Record each sleeper and bed per night, report the share of bed-nights in each sleeper's most-used bed, distinct beds per person per ten nights, and failures caused by a taken bed. Tracking issue: [#156](https://github.com/danhicks96/ostrog-public-ci/issues/156).

**Acceptance:** Test `crates/hf-sim/tests/sleep_bed.rs::sleep_census_counts_bed_changes`, through a helper on a scripted two-night case. Done when: the census prints the baseline for seeds 1-4 at 120 days.

### OST-362 · People pass their own bed for a nearer free one, and lie in others' beds: prefer one's own and avoid another's

**Milestone:** Make the valley feel alive · **State:** Claimed · **Developer:** Ostrog CI (OX) · **Public PR:** Pending

Sleep choice favors the nearest available bed without respecting ownership. Include the person's own bed among candidates, prefer it by a factor of 1.5, reduce another present living owner's bed to 0.25, and retain normal weight for absent or dead owners. Compose this with room quality and propose the D72 amendment. Tracking issue: [#160](https://github.com/danhicks96/ostrog-public-ci/issues/160).

**Acceptance:** Test `crates/hf-sim/tests/sleep_bed.rs::ones_own_bed_is_chosen_over_a_nearer_free_one`. Test `sleep_bed.rs::anothers_bed_is_taken_only_when_no_other_is_free`. Done when: both pass, and the OST-361 census shows ≥ 85% of bed-nights in one's own bed and "bed was taken" ≤ 1 per 100 bed-nights on seeds 1-4.

### OST-363 · The one whose name is on a house proposal gets no more of the house than anyone: make it their household's

**Milestone:** Make the valley feel alive · **State:** Claimed · **Developer:** Ostrog CI (OX) · **Public PR:** Pending

The proposer of a house has no household priority in it. Record the proposer, partner, children below coming of age and fosterling as the household at marking out, then let them claim the finished house's beds before others through the claims system. Tracking issue: [#164](https://github.com/danhicks96/ostrog-public-ci/issues/164).

**Acceptance:** Test `crates/hf-sim/tests/ownership.rs::the_proposers_household_owns_the_house_and_sleeps_in_it`. Done when: it passes.

### OST-365 · Nothing says how good anyone's home is: add a pure home-worth reading and a census of it

**Milestone:** Make the valley feel alive · **State:** Claimed · **Developer:** Ostrog CI (OX) · **Public PR:** Pending

Fine stone is the only home-quality input. Add a pure bounded personal home_worth reading for an owned sleeping place and roof, using the existing room statistics for space, furnishings, walls, light and carvings. Add a census of per-person values every fifty days without duplicating the room-score implementation. Tracking issue: [#194](https://github.com/danhicks96/ostrog-public-ci/issues/194).

**Acceptance:** Test `crates/hf-sim/tests/home_worth.rs::a_roofed_house_with_table_and_lamps_is_worth_more_than_a_lean_to`. Test `home_worth.rs::home_worth_is_pure`. Done when: both pass and the census prints for seeds 1-4.

### OST-366 · A family in a good stone house and one on bare grass feel the same: add pride and shame of home

**Milestone:** Make the valley feel alive · **State:** Claimed · **Developer:** Ostrog CI (OX) · **Public PR:** Pending

A fine house and bare ground feel alike. Refresh HomeProud for worth at least 0.7, and HomePoor for worth at most 0.25 after ten days since arrival. Replace the FineHouse producer with the personal home-worth reading while retaining its saved enum variant. Tracking issue: [#208](https://github.com/danhicks96/ostrog-public-ci/issues/208).

**Acceptance:** Test `crates/hf-sim/tests/home_worth.rs::a_fine_house_gives_pride_and_a_lean_to_after_ten_days_gives_shame`. Done when: it passes, and on seeds 1-4 after day 100 ≥ 10% hold each.

### OST-367 · Nobody envies a neighbour's better house: add envy, with a slow and bounded souring toward the envied

**Milestone:** Make the valley feel alive · **State:** Claimed · **Developer:** Ostrog CI (OX) · **Public PR:** Pending

A familiar neighbor's better home never causes envy. Add bounded, saved envy toward at most two familiar people when the worth gap reaches 0.35, with disposition-weighted daily onset, a thought and at most -0.4 opinion souring. End it when the gap closes or other causes improve opinion enough. Tracking issue: [#223](https://github.com/danhicks96/ostrog-public-ci/issues/223). Intake review: Use the reserved AIMS purpose value 63, reusing it if already defined and appending exactly that value here otherwise.

**Acceptance:** Test `crates/hf-sim/tests/home_worth.rs::a_poor_sleeper_beside_a_fine_house_comes_to_envy_its_owner`. Test `home_worth.rs::envy_stops_when_the_gap_closes_and_its_souring_is_bounded`. Done when: both pass, and envy affects 2-15% of adults on seeds 1-4 after day 100.

### OST-368 · Envy would only sour people, never move them: let it turn into a want to keep up

**Milestone:** Make the valley feel alive · **State:** Claimed · **Developer:** Ostrog CI (OX) · **Public PR:** Pending

Envy would otherwise only sour people. Let it form OwnBed or HomeForHousehold for someone without a house, or MakeFine(carpentry) for a householder who knows the craft. The recorded cause names the envied home. Tracking issue: [#233](https://github.com/danhicks96/ostrog-public-ci/issues/233).

**Acceptance:** Test `crates/hf-sim/tests/home_worth.rs::envy_of_a_better_house_becomes_a_want_to_keep_up`. Done when: it passes.

### OST-369 · Evenings are spent only at the community hearth: let a proud householder have a friend round

**Milestone:** Make the valley feel alive · **State:** Claimed · **Developer:** Ostrog CI (OX) · **Public PR:** Pending

Evening company gathers only at the community hearth. Offer a proud householder's table as an evening gathering place and let a nearby known friend join. After an hour together, record ordinary talk and at most one HadRound event per host per day, under the existing leisure-security gate. Tracking issue: [#238](https://github.com/danhicks96/ostrog-public-ci/issues/238).

**Acceptance:** Test `crates/hf-sim/tests/home_worth.rs::a_proud_householder_has_a_friend_round_of_an_evening`. Done when: it passes.

### OST-371 · Nothing counts how long people sleep rough or what they do about it: add streaks and answers to the census

**Milestone:** Make the valley feel alive · **State:** Claimed · **Developer:** Ostrog CI (OX) · **Public PR:** Pending

The sleep census lacks consecutive rough-night streaks. Track the per-person streak distribution in bands 1, 2-3, 4-7 and 8+, and the longest streak with its person. Add answer counts once the logged rough-answer chooser is available; this measurement changes no simulation state. Tracking issue: [#244](https://github.com/danhicks96/ostrog-public-ci/issues/244).

**Acceptance:** Test `crates/hf-sim/tests/sleep_bed.rs::sleep_census_counts_rough_streaks`, through the helper on a planted case. Done when: the baseline prints for seeds 1-8 at 300 days.

### OST-372 · Turned out of a bed someone else reached first, a person just gets up: remember who, and mind it a little

**Milestone:** Make the valley feel alive · **State:** Claimed · **Developer:** Ostrog CI (OX) · **Public PR:** Pending

Someone turned out of a taken bed remembers neither the occupant nor the slight. Identify the earlier sleeper, lower opinion of them a little, more when the bed belongs to the rejected sleeper, and name them in the reason without adding to failure counters. Tracking issue: [#247](https://github.com/danhicks96/ostrog-public-ci/issues/247).

**Acceptance:** Test `crates/hf-sim/tests/rough.rs::turned_out_of_a_bed_one_thinks_a_little_less_of_who_was_in_it`. Done when: it passes.

### OST-373 · There is no choice of how to answer a night on the ground: add a pure chooser, logged only

**Milestone:** Make the valley feel alive · **State:** Claimed · **Developer:** Ostrog CI (OX) · **Public PR:** Pending

There is no personal choice of how to answer repeated rough nights. Add a pure deterministic Answer chooser for enduring, building, asking to share, taking a bed, sulking or leaving, using dispositions and known circumstances. Initially record only the selected answer in the Why trace; enactment belongs to later slices. Tracking issue: [#250](https://github.com/danhicks96/ostrog-public-ci/issues/250).

**Acceptance:** Test `crates/hf-sim/tests/rough.rs::opposite_temperaments_answer_a_rough_night_differently`. Test `rough.rs::the_answer_is_the_same_at_1_and_6_threads`. Digests are unchanged. Done when: all pass.

### OST-374 · Nobody ever asks a friend with a spare bed to put them up: add asking to share, with gratitude and a debt

**Milestone:** Make the valley feel alive · **State:** Claimed · **Developer:** Ostrog CI (OX) · **Public PR:** Pending

Nobody asks a friend for a spare bed. Enact AskToShare by walking to a host; on acceptance grant up to five nights of hospitality, gratitude and a debt, and on refusal remember it and lower opinion. Save the guest relationship and record the outcome as AskedToShare. Tracking issue: [#252](https://github.com/danhicks96/ostrog-public-ci/issues/252).

**Acceptance:** Test `crates/hf-sim/tests/rough.rs::a_liked_householder_with_a_free_bed_takes_in_a_friend`. Test `rough.rs::a_refusal_is_remembered`. Done when: both pass.

### OST-375 · Nobody ever sleeps in an empty bed that is someone else's: allow it, with a grievance and an accusation

**Milestone:** Make the valley feel alive · **State:** Claimed · **Developer:** Ostrog CI (OX) · **Public PR:** Pending

Nobody takes another person's empty bed. Enact TakeBed while keeping ordinary ownership preferences elsewhere. When the owner finds out, record the grievance and hardship memory, and make an accusation when the owner's theft belief treats it as wrong. Record the taking event. Tracking issue: [#255](https://github.com/danhicks96/ostrog-public-ci/issues/255). Intake review: An owner reacts only after seeing the taker asleep in their bed while awake, on the same level, within eight tiles and in line of sight, or being told by a real witness at half strength. An empty bed on return supplies no identity or accusation.

**Acceptance:** Test `crates/hf-sim/tests/rough.rs::a_cold_independent_one_sleeps_in_an_away_neighbours_bed_and_is_resented`. Test `rough.rs::an_owner_who_holds_taking_wrong_accuses`. Done when: both pass. Intake-review acceptance: Also test `rough.rs::an_owner_who_never_sees_the_taker_holds_nothing`: a taker gone before the owner returns causes no opinion change or offence.

### OST-376 · However long the nights on the ground, no member ever walks out: allow leaving, with a reason

**Milestone:** Make the valley feel alive · **State:** Claimed · **Developer:** Ostrog CI (OX) · **Public PR:** Pending

Community members never walk out over prolonged rough sleeping. Enact Leave only after eight rough nights, resentment toward at least two householders, no partner or child in the community and no held role. Reuse the existing leaving path, record why, and let close ties miss the departed. Tracking issue: [#257](https://github.com/danhicks96/ostrog-public-ci/issues/257).

**Acceptance:** Test `crates/hf-sim/tests/rough.rs::a_bitter_one_with_no_ties_walks_out_after_a_month_on_the_ground`. Test `rough.rs::a_parent_never_walks_out_over_a_bed`. Done when: both pass, and walk-outs over beds are ≤ 1 per seed per 100 days.

### OST-566 · Partners own beds in different houses and sleep apart wherever a bed is free: move a partner's claim into the other's house and let partners share one bed

**Milestone:** Make the valley feel alive · **State:** Claimed · **Developer:** Ostrog CI (OX) · **Public PR:** Pending

Partners can own beds in different houses and cannot share one. Move a partner's claim to a free bed in the other's house, releasing the previous claim; with no free bed keep existing claims. Prefer a partner's bed and allow exactly the owning couple to share an ordinary Bed. Tracking issue: [#575](https://github.com/danhicks96/ostrog-public-ci/issues/575).

**Acceptance:** Test `crates/hf-sim/tests/ownership.rs::partners_share_a_house_and_its_beds`: partners A and B own beds in two different houses, A's house has a free bed; after the 06:30 claim step both own beds in A's house plan. A second case: A and B both choose A's one `Bed` the same night; neither gets a `PlanFailed` "the bed was taken", and both push `Slept { in_bed: true }`. Measured: not a sim change beyond sleeping places; the sibling item's census shows partners' bed-nights in one house plan. development-history entry.

### OST-567 · Children sleep in whatever free bed is nearest: a child with no bed claims a free bed in a house where a parent owns one

**Milestone:** Make the valley feel alive · **State:** Claimed · **Developer:** Ostrog CI (OX) · **Public PR:** Pending

Children claim whichever free bed is near. Give a child without an owned bed first choice of a free bed in a parent's house, using the lowest structure identifier for ties and stable agent order. Fall back to the existing habit rule when no parent owns a suitable bed. Tracking issue: [#576](https://github.com/danhicks96/ostrog-public-ci/issues/576).

**Acceptance:** Test `crates/hf-sim/tests/ownership.rs::a_child_sleeps_in_a_parents_house`: a child with no bed, the mother owning a bed in house A that has one more free bed, and a nearer free bed outside A; after the 06:30 claim step the child owns house A's free bed and not the nearer one. Measured: not a sim change beyond bed claims. development-history entry.

### OST-568 · An owned bed would stay with the dead, the departed and the long gone: release it on death, leaving, exile, a camp move or ten nights unused

**Milestone:** Make the valley feel alive · **State:** Claimed · **Developer:** Ostrog CI (OX) · **Public PR:** Pending

Bed claims would remain with dead, departed or long-absent owners. Release claims after death, leaving, exile or a camp move, and after ten nights unused while the owner remains in the valley. Time travelling does not count; clear the person's bed cache together with ownership and emit no event. Tracking issue: [#577](https://github.com/danhicks96/ostrog-public-ci/issues/577).

**Acceptance:** Test `crates/hf-sim/tests/ownership.rs::a_bed_left_ten_days_is_let_go_but_not_by_a_traveller`: an owner who sleeps elsewhere in the valley for 10 days has `owner == None` after the next claim step; an owner in `overworld.away` for 20 days still owns it; a dead owner's bed is unowned after the next claim step. Measured: the sibling item's census's "bed was taken" count does not rise against its baseline on seeds 1-4. development-history entry.

### OST-569 · A householder's house would go nowhere when they die: pass it to the partner, else the eldest child of age, with an Inherited line

**Milestone:** Make the valley feel alive · **State:** Claimed · **Developer:** Ostrog CI (OX) · **Public PR:** Pending

A house has no succession when its householder dies or leaves. Pass it to a living partner, including one temporarily away, else the eldest child of age in the valley. With no heir clear ownership; otherwise put the heir first and emit one existing Inherited event naming the house. Tracking issue: [#578](https://github.com/danhicks96/ostrog-public-ci/issues/578).

**Acceptance:** Test `crates/hf-sim/tests/ownership.rs::a_dead_householders_house_passes_to_the_partner`: the householder dies with a living partner: after the claim step `house_owners[ix][0]` is the partner and exactly one `Inherited` with `what == "the house"` is pushed. With no partner and a child of age, the child heads it; with neither, the entry is empty and no event is pushed. Measured: count of `inherited` rows whose `what` is "the house" on seeds 1-4 at 300 days, printed in the PR. development-history entry.

### OST-241 · Make a break's cost last

**Milestone:** Make the valley feel alive · **State:** Claimed · **Developer:** Ostrog CI Inner Life (OT) · **Public PR:** Pending

A breakdown and a quarrel each knock a fixed amount off mood, and the next needs pass rebuilds mood from its terms a game-minute later, so both costs vanish at once. Replace the two direct writes with a thought that decays over days, so the cost of a break is felt the next morning.

**Acceptance:** An hour after a break the person is at least 0.15 lower in mood than the same person without it. A FIXLOG entry is added and any simulation digest change is explained in the PR.

### OST-1500 · Record and query unattended simulation diagnostics

**Priority:** High · **Milestone:** Verify long-running simulations · **State:** Claimed · **Developer:** Ostrog CI (OC) · **Public PR:** Pending

Extend the opt-in decision logger with a Flight Recorder for machine soaks and desktop debugging. Survey provides world summaries and points to anomalies; focused Trace explains actual choices, attempts, failures and outcomes with stable tick and entity references; verified Replay is a follow-up that refuses to claim equivalence on divergence or missing inputs. Local read-only queries should recover useful evidence without opening the game or loading an entire recording. Keep diagnostics separate from narrative history, the Why lens, and the camera director. Any future in-game consumer reads simulation signals independently of the recorder.

**First implementation acceptance:** From an unattended run with no person selected, reopen the output in a fresh process and recover a real decision, attempt, failure and later retry; distinguish missing evidence from a recorded absence. Report capture coverage, dropped output and retention limits. Preserve the build, seed and tick range, bound resource use, leave normal runs without a trace file, and prove equal simulation digests with diagnostics on and off. No new player-facing or gameplay behavior in the first slice.

## Waiting on code or measurement

### OST-1200 · Count a drought as blocking storms as well as rain

**Milestone:** Make the valley feel alive · **State:** Landed · **Developer:** Sage (SG) · **Public PR:** [#632](https://github.com/danhicks96/ostrog-public-ci/pull/632)

A drought clears rain but lets storms through. Make it clear storms too. Tracking issue: [#545](https://github.com/danhicks96/ostrog-public-ci/issues/545).

**Acceptance:** With a drought spell active on an hour the unforced weather would make a storm, the weather reads `Clear` and rainfall is 0. Focused spell tests and hosted CI pass; no save-format change.

### OST-1000 · List what happened in one colony year

**Milestone:** Make important events easier to follow · **State:** Landed · **Developer:** Wren (WN) · **Public PR:** [#585](https://github.com/danhicks96/ostrog-public-ci/pull/585)

Add a read-only history query that returns the notable-or-worse events of one colony year, in the order they happened, that the viewer is allowed to know about. Repeated events are folded the same way the history and Catch Up read models fold them. This is a Rust read model only: no bridge, client panel or media change, and no simulation change. It is separate from the Catch Up folding in OST-014 and the panel in OST-011.

**Acceptance:** Stage deaths across several years plus many repeated suspicions in one year. The year query returns only that year's events, oldest first, with the repeats folded to one line, and pages deterministically. Check the first and last tick of the year, a negative year (empty), and a page past the end. Say in the result whether the total counts folded lines or raw rows, and keep rows about undiscovered people out.

### OST-1001 · Show a cohesion word on the community line

**Milestone:** Make important events easier to follow · **State:** Landed · **Developer:** Wren (WN) · **Public PR:** [#622](https://github.com/danhicks96/ostrog-public-ci/pull/622)

Show a short word for how united a community is on the HUD community line, using the community's existing cohesion number. Keep the raw number available. This is a display of the current state, not a forecast, unless a reading before a split can be shown.

**Acceptance:** Check the exact boundary values between words, and that the HUD still shows the raw state. A client or bridge test shows the word for a stable and a divided community. Do not claim advance warning of a split unless the word is shown to change before it.

### OST-1002 · Take the mood word from the simulation

**Milestone:** Make important events easier to follow · **State:** Landed · **Developer:** Wren (WN) · **Public PR:** [#623](https://github.com/danhicks96/ostrog-public-ci/pull/623)

The inspect panel works out its mood word in client script. Have the simulation supply the existing mood label and the shared strain threshold, and let the client show what it is sent. Keep the simulation's behavior unchanged.

**Acceptance:** Check every threshold edge, the client fallback when no word is sent, and that a run's digest is unchanged. Compare against the landed breakdown explanation (OST-005) before touching the shared strain code.

### OST-1003 · Follow a record line's chain of recorded causes

**Milestone:** Make important events easier to follow · **State:** Landed · **Developer:** Wren (WN) · **Public PR:** [#624](https://github.com/danhicks96/ostrog-public-ci/pull/624)

Add a viewer page that follows a history line back through its recorded causes, one step at a time, beyond the single-step explanation that exists today. Do not duplicate or replace the one-step explanation (OST-005).

**Acceptance:** The walk is bounded, shows only causes the viewer may know, and says plainly when a cause is missing rather than claiming there is none. Staged fixtures cover a long chain, a missing record and a hidden person. Keep screenshots and planning notes out of the source change.

**Known gap:** the History "why?" link and the Legends page have not been run in the Godot client; that check is waived for this change and left as a follow-up.

### OST-003 · Compare long-run camera coverage

**State:** Waiting on dependencies · **Developer:** None · **Public PR:** None

**Dependencies:** A candidate ref with the major-event scoring change (OST-001), and working camera-coverage instrumentation (OST-004) available on both baseline and candidate refs. This comparison can run before merge or release once those refs are available.

**Acceptance:** On matched seeds 1–5 over 700 game-days, report total and missed major deaths before and after the scoring change, with the same camera sampling and missed-event definition. Report both per-seed counts and the pooled share. A run with no qualifying deaths has no share; do not count it as zero misses. Record the compared refs.

### OST-1201 · Keep a feud alive while its only holder is on a journey

**Milestone:** Make the valley feel alive · **State:** Landed · **Developer:** Sage (SG) · **Public PR:** [#621](https://github.com/danhicks96/ostrog-public-ci/pull/621)

Feuds end while their only holder is on a journey. Count people away as still holding it. Tracking issue: [#547](https://github.com/danhicks96/ostrog-public-ci/issues/547).

**Acceptance:** The only holder of one side leaves on a six-day journey: the feud is still running on their return and neither an inheritance nor an end event was recorded. Controls for a dead or disowned holder still end the feud. Focused feud tests and hosted CI pass.

### OST-007 · Keep local bodies visible after reload

**Milestone:** Preserve what the viewer sees across saves · **State:** Waiting on dependencies · **Developer:** None · **Public PR:** None

**Dependencies:** The reviewed read model in OST-006 must be available to the client integration. Use one implementation lane for reload and lifetime handling.

**Acceptance:** Load a seeded scene containing a local unburied body through the running client and verify it remains at the recorded tile. Verify a local unrested body is still visible after ten game-days and disappears when its condition is removed. Off-map and already buried deaths do not reappear; a new world does not inherit bodies.

### OST-1202 · Do not offer a bed in a flood's way to a tired sleeper

**Milestone:** Make the valley feel alive · **State:** Landed · **Developer:** Sage (SG) · **Public PR:** [#613](https://github.com/danhicks96/ostrog-public-ci/pull/613)

Tired people are offered beds in a flood's way and collapse instead. Skip those beds. Tracking issue: [#549](https://github.com/danhicks96/ostrog-public-ci/issues/549).

**Acceptance:** A staged camp of six whose beds lie by a pond in spate, awake since morning and tired in the evening: all six are asleep away from flood water without an exhaustion event. A safe bed is still preferred over the ground. Existing flood tests and hosted CI pass; no save-format change.

### OST-009 · Sample camera coverage during headless runs

**Milestone:** Make important events easier to follow · **State:** Waiting on dependencies · **Developer:** None · **Public PR:** None

**Dependencies:** The frame tally in OST-008 and headless tally/report output must be available. Integrate the missed-event metric in OST-004 without starting a duplicate implementation. Tracking how often a frame contains a notable event is a separate measure from how many major events were missed.

**Acceptance:** At a fixed stride of 600 ticks, a two-day run records 96 camera samples. A staged notable event counts while current and in frame, but not after expiry or outside the frame. Report each metric with its own denominator. Matching runs with and without observation have the same simulation digest.

### OST-1203 · Keep a role until a challenger has led for several mornings

**Milestone:** Make the valley feel alive · **State:** Landed · **Developer:** Sage (SG) · **Public PR:** [#614](https://github.com/danhicks96/ostrog-public-ci/pull/614)

Roles flip daily when two duties are close. Add a margin and a three-morning hold. Tracking issue: [#551](https://github.com/danhicks96/ostrog-public-ci/issues/551).

**Acceptance:** A person with 300 forage minutes and 280 haul minutes who hauls one extra day keeps the forager role. After three days of hauling that put haul minutes 30% ahead, they become a hauler with one role event. Existing duty tests and hosted CI pass; old saves load.

### OST-011 · Give Catch Up its own panel

**Milestone:** Make important events easier to follow · **State:** Landed · **Developer:** Iris (IV) · **Public PR:** [#650](https://github.com/danhicks96/ostrog-public-ci/pull/650)

**Dependencies:** Met. Readable counts landed in OST-010 ([#526](https://github.com/danhicks96/ostrog-public-ci/pull/526)) and folded details in OST-014 ([#7](https://github.com/danhicks96/ostrog-public-ci/pull/7)). Use the named-change read model already available. Keep the raw `counts` unchanged.

**Acceptance:** With two named changes and three folded lines, Catch Up renders the headline and named changes before the folded details. Opening History does not close or replace Catch Up. Check the result after an away period in the running client.

**Follow-up (open):** the headless client tests and the Rust card test cover ordering, placement and History opening beside Catch Up. The check in a running desktop client after a real away period has not been done and stays open until someone does it on a desktop client.

### OST-1204 · Let tools left at an abandoned camp teach their craft

**Milestone:** Make the valley feel alive · **State:** Landed · **Developer:** Sage (SG) · **Public PR:** [#612](https://github.com/danhicks96/ostrog-public-ci/pull/612)

Left-behind tools keep their maker but not their craft. Make them teach it. Tracking issue: [#553](https://github.com/danhicks96/ostrog-public-ci/issues/553).

**Acceptance:** A community with a smith-made iron tool on its floor moves camp: the tool teaches smithing, a curious member of another community can study it and learn smithing, and food left behind teaches nothing. Existing left-site tests and hosted CI pass; no save-format change.

### OST-012 · Open Catch Up without relying on window focus

**Milestone:** Make important events easier to follow · **State:** In review · **Developer:** Iris (IV) · **Public PR:** [#657](https://github.com/danhicks96/ostrog-public-ci/pull/657)

**Dependencies:** Met. The independent panel landed in OST-011 ([#650](https://github.com/danhicks96/ostrog-public-ci/pull/650)). Keep idle-return behavior and the manual control in one client lane to avoid conflicting controller edits.

**Acceptance:** With the proposed 300-second default, test 299, 300 and 301 seconds: the return opens Catch Up only after more than 300 seconds and only with news, meaning a viewer-visible named change or notable line. Retain the focus trigger. A click opens the panel and selects a time span. Place qualifying rows just before, at and after midnight and a five-day season boundary, then assert which rows each span returns. Previous-day ranges exclude the new day; the current day and season include their boundary row. At tick zero, a qualifying row at tick zero is included in the current day and season, with no underflow or invented previous day. Confirm on a second-screen client without pausing play.

### OST-1205 · Post a cooking pot beside a hearth that is cooked at

**Milestone:** Make the valley feel alive · **State:** Landed · **Developer:** Sage (SG) · **Public PR:** [#637](https://github.com/danhicks96/ostrog-public-ci/pull/637)

Hearths are cooked at but never get a pot. Post one from a stored vessel. Tracking issue: [#596](https://github.com/danhicks96/ostrog-public-ci/issues/596).

**Acceptance:** A hearth with a cook order and a vessel in store gets one pot order, once; with no vessel, none; a finished or open pot order within a tile prevents another. Focused tests and hosted CI pass; no save-format change.

### OST-013 · Check that Catch Up preserves important events

**Milestone:** Make important events easier to follow · **State:** Ready · **Developer:** None · **Public PR:** None

**Dependencies:** Met. The folded details landed in OST-014 ([#7](https://github.com/danhicks96/ostrog-public-ci/pull/7)) and the card that shows them in OST-011 ([#650](https://github.com/danhicks96/ostrog-public-ci/pull/650)); the census uses that same card. Fixture design may inform the eventual implementation, but it is not a separate claim: the acceptance fixture must exercise the same folded summary used by the census.

**Acceptance:** Compare each eight-hour window against the chronicle in a matched census of eight seeds, three deities and 120 game-days per run. Compare only qualifying deaths, births, pairings, arrivals and departures the viewer is allowed to know. A qualifying departure is a `left` event, not a caravan setting out. Each qualifying event is represented by name. The census checks whether any event kind takes more than three lines in the folded notable details; report violations rather than adding a second cap to OST-014. Never hide a named change to satisfy this check. Report missing events, violating kinds and affected windows, not just an aggregate percentage.

### OST-1400 · Name three stages of childhood

**Milestone:** Fill in the living world · **State:** Claimed · **Developer:** Marlow (ML) · **Public PR:** Pending

Add an infant, small-child and older-child answer to how far along a child is, with two helpers for later behaviour gates. Decision D101 already records the ages: infant under 2, small under 7, grown at 15 (human years), scaled by each kind's maturity. This is a read-only helper: no caller, no saved field, no behaviour change. The age of authority (16) stays a separate rule.

**Acceptance:** Pin the boundaries exactly for a human (1.99, 2.0, 6.99, 7.0 years) and for a kind that matures at 30, and show that a grown person returns no stage. Check both helpers on each stage. The generated reference and simulation digests are unchanged.

### OST-1401 · Add a threat pacing census example

**Milestone:** Fill in the living world · **State:** Claimed · **Developer:** Marlow (ML) · **Public PR:** Pending

Add a read-only example that prints one CSV row per 100 game-days of band set-outs, hosts besieging, portions raided, endings by kind, deaths not of old age and the longest gap from an ending to the next arrival, so storyteller changes can be measured before and after. No simulation change and no threat tuning.

**Acceptance:** A unit test covers block boundaries, a gap that crosses a block, band set-out versus host besieging labelled apart, and a person who died while away or removed. The first slice does not run the seeds 1-8, three-engine, 600-day baseline; that needs its own authorisation.

### OST-1402 · Score a house as a room

**Milestone:** Fill in the living world · **State:** Claimed · **Developer:** Marlow (ML) · **Public PR:** Pending

Add a pure room statistic for a house plan: space per sleeper, furnishing, light, beauty and clutter, folded into one bounded impressiveness number. Nothing calls it, and dirt stays zero until a dirt source exists. Do not wire a consumer, home valuation or the body provenance work in OST-006.

**Acceptance:** A bare hut scores below a furnished house. Cover laid ground floors and upper floors, an inactive lamp that gives no light, loose clutter and bodies awaiting rites, and a figurine or carving. The generated reference check passes and simulation state is unchanged.

### OST-1403 · Find one tile for a lean-to

**Milestone:** Fill in the living world · **State:** Claimed · **Developer:** Marlow (ML) · **Public PR:** Pending

Add a pure search for the nearest suitable single tile within 12 tiles of home where one person could put up a lean-to. The tile must be flat, walkable, dry, unbuilt, not ordered, outside house plans and the palisade, and not a place the person fears. Nothing calls it. This shares `houses.rs` with OST-1402, so the two land one after the other.

**Acceptance:** Cover each exclusion, the tie order (the same world always gives the same tile), a remembered hurt that moves the site, and no candidate at all. Keep the caller for a later slice. Simulation digests are unchanged.

### OST-1404 · Let wild plants read their ground

**Milestone:** Fill in the living world · **State:** Claimed · **Developer:** Marlow (ML) · **Public PR:** Pending

Wild, non-crop plants currently grow at the weather's rate alone. Make them read ground, nearby water, shade from a mature tree, and crowding the way crops already read ground, reusing the existing crop ground table. This changes plant growth, so it is a gameplay change, not a refactor.

**Acceptance:** A bush on poor ground in shade grows slower than one on wet meadow; trees read ground and crowding only; dead wood does not count as a crowd. State the decision row. The browse and grazer comparison (seeds 1-5, 700 days) is not part of the first slice and is reported as not run unless authorised.

### OST-867 · Find which activities the client draws distinguishably

**Milestone:** Make important events easier to follow · **State:** Claimed · **Developer:** Ostrog CI Watcher (OW) · **Public PR:** None

Read the client's pose and emote choice for people and list, per `Activity` name and per work plan kind, what is drawn. Write the result as a `READABLE` table in `crates/hf-sim/src/view/alive.rs`, with a comment naming the client file. Anything the client draws as plain standing counts as standing whatever the sim calls it. Tracking issue: [#417](https://github.com/danhicks96/ostrog-public-ci/issues/417).

**Acceptance:** The table lists every `Activity::name()` value and every work plan kind with the client's pose. The existing glance tests still pass. Simulation state is unchanged.

### OST-868 · Run the camera director headlessly in the alive soak

**Milestone:** Make important events easier to follow · **State:** Claimed · **Developer:** Ostrog CI Watcher (OW) · **Public PR:** None

Under `soak --alive`, feed the director the same events and suggestions the bridge does, and sample the glance at a fixed stride of 600 ticks (no rng). Tracking issue: [#420](https://github.com/danhicks96/ostrog-public-ci/issues/420). Builds on the existing glance tally in OST-008; do not duplicate it.

**Acceptance:** A two-day opening soak prints the glance keys with 96 samples. A saved ring's digest is the same with and without `--alive`.

### OST-864 · Count how many feed lines a day are the same news again

**Milestone:** Make important events easier to follow · **State:** Ready · **Developer:** None · **Public PR:** None

Extend the alive report with, per game-day, the number of lines that reach the feed at notable or above, the number after folding, and the top fold keys. Tracking issue: [#408](https://github.com/danhicks96/ostrog-public-ci/issues/408). Uses the existing `view::known::told` and `view::fold::fold`.

**Acceptance:** `ostrog soak --alive` prints `feed_lines_day` and `feed_folded_day`. Baseline and folded counts per day are reported for seeds 1-8 and all three deities over 300 days, or the report says which were not run.

### OST-862 · Check that a returning viewer is told every death, birth and pairing

**Milestone:** Make important events easier to follow · **State:** Ready · **Developer:** None · **Public PR:** None

Add `crates/hf-sim/examples/catch_up_census.rs`: for each 8-game-hour window on opening worlds, compare `summary::changes` and the folded lines against the chronicle's rows of the kinds that name who changed. Tracking issue: [#403](https://github.com/danhicks96/ostrog-public-ci/issues/403).

**Acceptance:** Every death, birth, pairing, arrival and departure in a window appears by name, and no kind fills more than 3 of the folded lines. Report seeds and days actually run.

### OST-863 · Check that a biography fits on a page and keeps every life event

**Milestone:** Make important events easier to follow · **State:** Ready · **Developer:** None · **Public PR:** None

Add `crates/hf-sim/examples/biography_census.rs`: for each colonist who lived at least 100 game-days, count `view::biography` lines and check that every child's birth, pairing, partner's death and their own death is present. Tracking issue: [#405](https://github.com/danhicks96/ostrog-public-ci/issues/405).

**Acceptance:** 95% of such lives fit in 60 lines and 100% keep the listed events, or the shortfall is reported with examples. Simulation state is unchanged.

## Repository usability and diagnostics

### OST-1501 · Map where game data lives and how to change it

**Priority:** High · **Milestone:** Make the repo easier to navigate and profile · **State:** Claimed · **Developer:** Ostrog CI (OC) · **Public PR:** Pending

Give contributors a linked, text-first architecture map from the README. Start with world generation and the tick loop, person decisions and feedback, and the boundaries for Godot, the chronicle, saves and debug output. Add a registry of the remaining domains so missing maps are visible. For each map, document the source of truth, writers, readers, cadence, cross-thread and save boundaries, what is retained or rebuilt, and where to test and measure expensive copies, updates or writes.

**First implementation acceptance:** A contributor can trace a person decision and world change from source to screen and save without knowing the directory layout. The same PR updates maps when ownership, timing, shape, storage or consumers change. Verify links and machine-checkable anchors, reuse existing generated reference tooling where practical, and test that one fixture change to a mapped contract detects stale docs without requiring edits for unrelated refactors. Keep the docs source-only and distinguish measured costs from investigation leads.

## Gated by a published release

No entries yet. Use this section only when work must wait for a named capability to ship. Record the capability and verification criteria; move the item out of this section only after linking the qualifying published release. Code availability, merge and CI prerequisites belong under ordinary dependencies.

## Stretch goals

No entries yet. Stretch ideas are not available to claim until promoted to `Ready` with concrete acceptance criteria.
