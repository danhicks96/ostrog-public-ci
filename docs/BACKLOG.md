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

**Milestone:** Make important events easier to follow · **State:** In review · **Developer:** Ostrog CI (OC) · **Public PR:** [#3](https://github.com/danhicks96/ostrog-public-ci/pull/3)

The chronicle records the kind of breakdown but not what the person was under strain about. Append a separate notable cause event immediately after the original breakdown, at the same tick. Prefer the strongest fear, grief, anger or shame at strength 0.3 or higher, then the heaviest negative thought, then a need below 0.2. If none qualifies, record that the cause is unknown. Name another person only when the recorded feeling identifies them. Do not invent a subject for a thought.

**Acceptance:** A staged breakdown with a known person as the cause names that person; a staged breakdown without a cause reports it as unknown. Preserve the original event and behavior, and keep existing event discriminants in order. Compare break kinds and counts before and after on matched seeds 1–4 over 60 game-days, and report the number and share with a known cause. Focused tests alone do not establish the long-run comparison.

### OST-006 · Reconstruct on-map bodies from saved state

**Milestone:** Preserve what the viewer sees across saves · **State:** Claim pending · **Developer:** Pending · **Public PR:** Pending

The read-model candidate is pending review. Do not open a second implementation. Rebuild local bodies awaiting rites and unrested bodies supported by a recorded death; do not wire the client load path in this slice.

**Acceptance:** Round-trip staged local and off-map deaths with and without surviving rite entries. A stale saved agent tile must not override a recorded death tile; check the unrested timestamp separately. Do not restore bodies for rite-caused conditions, including marred rites; omit ambiguous unrested history. For a pending local rite, distinguish missing history from an invalid or contradictory row. Keep laid-to-rest and undiscovered bodies out.

### OST-008 · Count visible activity in the camera frame

**Milestone:** Make important events easier to follow · **State:** Claim pending · **Developer:** Pending · **Public PR:** Pending

Reconcile the existing read-only frame tally rather than starting another. Count whether visible people are doing something readable or standing still, separate from whether a major event was missed.

**Acceptance:** Stage a frame with working, idle, sleeping and off-frame people. Check the counts and denominators, including an empty frame. The measurement does not modify the simulation.

### OST-010 · Write Catch Up counts in plain language

**Milestone:** Make important events easier to follow · **State:** Claim pending · **Developer:** Pending · **Public PR:** Pending

Reconcile the existing summary change. The opening line should summarize important events in words without filling it with routine labor.

**Acceptance:** Build a summary with two deaths, one birth and many hauling rows. It names the births and deaths with the right counts, omits hauling in the headline, and leaves the raw summary counts available to existing consumers.

## Waiting on code or measurement

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

**Dependencies:** Readable counts in OST-010 and a folded summary that keeps named changes separate from routine rows. Use the named-change read model already available.

**Acceptance:** With two named changes and three folded lines, Catch Up renders the headline and named changes before the folded details. Opening History does not close or replace Catch Up. Check the result after an away period in the running client.

### OST-012 · Open Catch Up without relying on window focus

**Milestone:** Make important events easier to follow · **State:** Waiting on dependencies · **Developer:** None · **Public PR:** None

**Dependencies:** The independent panel in OST-011. Keep idle-return behavior and the manual control in one client lane to avoid conflicting controller edits.

**Acceptance:** Returning after a long idle with new events opens Catch Up; long idle without news and short idle with news do not. Retain the focus trigger. A click can open the panel and select a time span; verify the previous-day span and the five-day season boundary. Confirm on a second-screen client without pausing play.

### OST-013 · Check that Catch Up preserves important events

**Milestone:** Make important events easier to follow · **State:** Waiting on dependencies · **Developer:** None · **Public PR:** None

**Dependencies:** The folded summary must be available for the full census. Fixture design may inform the eventual implementation, but it is not a separate claim: the acceptance fixture must exercise the same folded summary used by the census.

**Acceptance:** Compare each eight-hour window against the chronicle in a matched census of eight seeds, three deities and 120 game-days per run. Every recorded death, birth, pairing, arrival and departure in scope is represented by name. No one event kind takes more than three folded lines. Report missing events and affected windows, not just an aggregate percentage.

## Gated by a published release

No entries yet. Use this section only when work must wait for a named capability to ship. Record the capability and verification criteria; move the item out of this section only after linking the qualifying published release. Code availability, merge and CI prerequisites belong under ordinary dependencies.

## Stretch goals

No entries yet. Stretch ideas are not available to claim until promoted to `Ready` with concrete acceptance criteria.
