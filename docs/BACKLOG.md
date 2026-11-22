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

**State:** Claim pending · **Developer:** Pending · **Public PR:** Pending

An implementation is being reconciled; do not start another. Add a read-only headless measurement for whether the camera showed a major event, split by event kind. This can be developed independently of the scoring change in OST-001; coordinate if a fixture touches the same test file.

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

**Milestone:** Make important events easier to follow · **State:** Claim pending · **Developer:** Pending · **Public PR:** Pending

Reconcile the existing read-only frame tally rather than starting another. Count whether visible people are doing something readable or standing still, separate from whether a major event was missed.

**Acceptance:** Stage a frame with working, idle, sleeping and off-frame people. Check the counts and denominators, including an empty frame. The measurement does not modify the simulation.

### OST-010 · Write Catch Up counts in plain language

**Milestone:** Make important events easier to follow · **State:** Claimed · **Developer:** Iris (IV) · **Public PR:** Pending

Reconcile the existing summary change. The opening line should summarize important events in words without filling it with routine labor.

**Acceptance:** Build a summary with two deaths, one birth and many hauling rows. It names the births and deaths with the right counts, omits hauling in the headline, and leaves the raw summary counts available to existing consumers.

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

Add an appended `Lonely` thought that appears while an awake person's social need is low and fades when it recovers. Check current owners and overlapping public work before starting; do not touch unrelated behaviour.

**Acceptance:** An awake person with a low social need gets the thought and it clears after recovery; a sleeper does not. The thought enum gains the new variant at the end, old saves load unchanged, and the Catch Up and breakdown-explanation ranking still pass.

### OST-1102 · Draw Gourmet and Tidy traits from the genome

**Milestone:** Make the inner life visible and trustworthy · **State:** Claimed · **Developer:** Tamsin (TS) · **Public PR:** Pending

Add the two traits are drawn from the genome so people can have them. Check current owners and overlapping public work before starting; do not touch unrelated behaviour.

**Acceptance:** Over 500 genomes each trait appears on at least 10 people and no person carries more than the existing trait cap. Existing trait discriminants and determinism hold, and hauling responds to a matched disposition.

### OST-1103 · Mark skill milestones with a line and a thought

**Milestone:** Make the inner life visible and trustworthy · **State:** Claimed · **Developer:** Tamsin (TS) · **Public PR:** Pending

Add a daily skill-tier check that writes one history line and one positive thought per tier, backed by a saved high-water mark. Check current owners and overlapping public work before starting; do not touch unrelated behaviour.

**Acceptance:** A person crossing a tier gets exactly one line and thought; reloading a save never repeats it, and a loaded old save initialises silently.

### OST-1104 · Let a memory be about an item or a community

**Milestone:** Make the inner life visible and trustworthy · **State:** Claimed · **Developer:** Tamsin (TS) · **Public PR:** Pending

Add append `Item` and `Community` subjects to memory without changing existing ones. Check current owners and overlapping public work before starting; do not touch unrelated behaviour.

**Acceptance:** Existing subjects keep their saved positions, old saves load with the same digest, and a new subject round-trips through save and load. No feeling or gameplay consumer is added here.

### OST-1105 · Parents and grandparents are proud when a child comes of age

**Milestone:** Make the inner life visible and trustworthy · **State:** Claimed · **Developer:** Tamsin (TS) · **Public PR:** Pending

Add an appended pride thought for living parents and grandparents when a child comes of age. Check current owners and overlapping public work before starting; do not touch unrelated behaviour.

**Acceptance:** A living parent and a living grandparent each get one thought, a person related by two paths gets one, and deceased or unknown kin get none. The enum order and old saves hold.

### OST-1106 · Make a witnessed death require a view of it

**Milestone:** Make the inner life visible and trustworthy · **State:** Claimed · **Developer:** Tamsin (TS) · **Public PR:** Pending

Add a SawDeath witness must be awake with a clear line of sight, not merely within ten tiles. Check current owners and overlapping public work before starting; do not touch unrelated behaviour.

**Acceptance:** A sleeper and a person behind a wall do not witness; a person with a clear view does; the victim is excluded. Keep the existing awake/line-of-sight helper and its API, and keep grief news separate (see public #313).

## Waiting on code or measurement

### OST-1000 · List what happened in one colony year

**Milestone:** Make important events easier to follow · **State:** Claimed · **Developer:** Wren (WN) · **Public PR:** Pending

Add a read-only history query that returns the notable-or-worse events of one colony year, in the order they happened, that the viewer is allowed to know about. Repeated events are folded the same way the history and Catch Up read models fold them. This is a Rust read model only: no bridge, client panel or media change, and no simulation change. It is separate from the Catch Up folding in OST-014 and the panel in OST-011.

**Acceptance:** Stage deaths across several years plus many repeated suspicions in one year. The year query returns only that year's events, oldest first, with the repeats folded to one line, and pages deterministically. Check the first and last tick of the year, a negative year (empty), and a page past the end. Say in the result whether the total counts folded lines or raw rows, and keep rows about undiscovered people out.

### OST-003 · Compare long-run camera coverage

**State:** Waiting on dependencies · **Developer:** None · **Public PR:** None

**Dependencies:** A candidate ref with the major-event scoring change (OST-001), and working camera-coverage instrumentation (OST-004) available on both baseline and candidate refs. This comparison can run before merge or release once those refs are available.

**Acceptance:** On matched seeds 1–5 over 700 game-days, report total and missed major deaths before and after the scoring change, with the same camera sampling and missed-event definition. Report both per-seed counts and the pooled share. A run with no qualifying deaths has no share; do not count it as zero misses. Record the compared refs.

### OST-007 · Keep local bodies visible after reload

**Milestone:** Preserve what the viewer sees across saves · **State:** Waiting on dependencies · **Developer:** None · **Public PR:** None

**Dependencies:** The reviewed read model in OST-006 must be available to the client integration. Use one implementation lane for reload and lifetime handling.

**Acceptance:** Load a seeded scene containing a local unburied body through the running client and verify it remains at the recorded tile. Verify a local unrested body is still visible after ten game-days and disappears when its condition is removed. Off-map and already buried deaths do not reappear; a new world does not inherit bodies.

### OST-009 · Sample camera coverage during headless runs

**Milestone:** Make important events easier to follow · **State:** Waiting on dependencies · **Developer:** None · **Public PR:** None

**Dependencies:** The frame tally in OST-008 and headless tally/report output must be available. Integrate the missed-event metric in OST-004 without starting a duplicate implementation. Tracking how often a frame contains a notable event is a separate measure from how many major events were missed.

**Acceptance:** At a fixed stride of 600 ticks, a two-day run records 96 camera samples. A staged notable event counts while current and in frame, but not after expiry or outside the frame. Report each metric with its own denominator. Matching runs with and without observation have the same simulation digest.

### OST-011 · Give Catch Up its own panel

**Milestone:** Make important events easier to follow · **State:** Waiting on dependencies · **Developer:** None · **Public PR:** None

**Dependencies:** Readable counts in OST-010 and the folded details in OST-014. Use the named-change read model already available.

**Acceptance:** With two named changes and three folded lines, Catch Up renders the headline and named changes before the folded details. Opening History does not close or replace Catch Up. Check the result after an away period in the running client.

### OST-012 · Open Catch Up without relying on window focus

**Milestone:** Make important events easier to follow · **State:** Waiting on dependencies · **Developer:** None · **Public PR:** None

**Dependencies:** The independent panel in OST-011. Keep idle-return behavior and the manual control in one client lane to avoid conflicting controller edits.

**Acceptance:** With the proposed 300-second default, test 299, 300 and 301 seconds: the return opens Catch Up only after more than 300 seconds and only with news, meaning a viewer-visible named change or notable line. Retain the focus trigger. A click opens the panel and selects a time span. Place qualifying rows just before, at and after midnight and a five-day season boundary, then assert which rows each span returns. Previous-day ranges exclude the new day; the current day and season include their boundary row. At tick zero, a qualifying row at tick zero is included in the current day and season, with no underflow or invented previous day. Confirm on a second-screen client without pausing play.

### OST-013 · Check that Catch Up preserves important events

**Milestone:** Make important events easier to follow · **State:** Waiting on dependencies · **Developer:** None · **Public PR:** None

**Dependencies:** The folded details in OST-014 must be available for the full census. Fixture design may inform the eventual implementation, but it is not a separate claim: the acceptance fixture must exercise the same folded summary used by the census.

**Acceptance:** Compare each eight-hour window against the chronicle in a matched census of eight seeds, three deities and 120 game-days per run. Compare only qualifying deaths, births, pairings, arrivals and departures the viewer is allowed to know. A qualifying departure is a `left` event, not a caravan setting out. Each qualifying event is represented by name. The census checks whether any event kind takes more than three lines in the folded notable details; report violations rather than adding a second cap to OST-014. Never hide a named change to satisfy this check. Report missing events, violating kinds and affected windows, not just an aggregate percentage.

## Gated by a published release

No entries yet. Use this section only when work must wait for a named capability to ship. Record the capability and verification criteria; move the item out of this section only after linking the qualifying published release. Code availability, merge and CI prerequisites belong under ordinary dependencies.

## Stretch goals

No entries yet. Stretch ideas are not available to claim until promoted to `Ready` with concrete acceptance criteria.
