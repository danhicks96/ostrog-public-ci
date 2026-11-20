# Ostrog development backlog

This is the public backlog for developers working on Ostrog. `OST-` IDs are stable: use them in public claims and PRs, and keep them if an entry moves sections or changes state. Only link public PRs and published release pages here. Maintain any crosswalk to previous trackers separately; it does not belong in this file or in public PR metadata.

## Claiming and closing work

1. Pick a `Ready` item. Open a small public PR against `main` changing only that entry to `Claimed` and adding your public name or initials. A claim wins **only when its PR lands**. A draft, open PR, or mention is not a reservation. If someone else already has a claim PR open, pick another item rather than starting a duplicate.
2. Once the claim lands, the listed developer is responsible for the implementation. Put the backlog ID in the implementation PR and add a public PR reference to the entry when it goes into review. Do not start or continue a second implementation of claimed work. Transfer or release a claim with a landed backlog update before another developer starts.
3. Keep unrelated work separate. Check existing claims and public PRs for overlapping modules or files before starting. Share implementation results and unresolved checks in the public PR, without private references or data.
4. Move to `Landed` only after the implementation PR merges and its stated acceptance checks are met. Mark `Released` only after verifying the change is included in an actual published release, and link that release. A branch, green CI run, merge or unreleased tag is not a published release.

**States:** `Ready` (available to claim), `Claim pending` (a claim or port is in progress; not a landed claim, but do not duplicate it), `Claimed` (claim landed), `In review` (claimed work has an implementation PR), `Waiting on dependencies` (required code or measurement is not yet available), `Landed` (accepted and merged), `Released` (verified in a published release), `Release-gated` (a named capability must ship before work starts), `Stretch` (not ready for a claim). Record an owner and a public PR link when verified. `None` means no owner or PR; `Pending` means the public record is not yet available.

## Milestones and parallel work

The milestone labels group related work; they do not promise a release date. Workstation investigation and camera instrumentation are separate from the drafts already in review. Do not start duplicate work.

### OST-001 · Keep major events visible when nobody is nearby

**Milestone:** Make important events easier to follow · **State:** In review · **Developer:** Ostrog CI (OC) · **Public PR:** [#2](https://github.com/danhicks96/ostrog-public-ci/pull/2)

An unwitnessed major event can be discounted enough that one busy person elsewhere draws the camera instead. Give major deaths, captures, breakdowns and exiles full presence even in an empty cell. Keep the existing weights for routine events, fires and departing caravans.

**Acceptance:** Place a major death in an empty cell 30 tiles from one busy person. The director selects the death. Existing director tests and the generated reference check pass. Simulation state is unchanged. Long-run coverage is tracked separately in OST-003.

### OST-002 · Clarify what the furnace would add

**Milestone:** Clarify workstation roles · **State:** Claim pending · **Developer:** Pending · **Public PR:** Pending

A source review is available for reconciliation; do not start a duplicate investigation. The Smelter and Kiln already have station recipes. `Furnace` exists in the structure model and is read by the world view and house logic, but the current recipe table assigns it no recipes. Document whether it has a distinct use instead of adding a station that duplicates the existing two.

**Acceptance:** Check recipes, building placement and compatibility references. Record the decision with concrete source evidence. If there is a distinct use, describe a separate build slice and its tests. Otherwise propose reserving the unused variant without deleting or renumbering it. No recipe, gameplay or save-format change is needed for this investigation.

### OST-004 · Measure major events the camera misses

**State:** Claim pending · **Owner:** Pending · **Public PR:** Pending

An implementation is being reconciled; do not start another. Add a read-only headless measurement for whether the camera showed a major event, split by event kind. This can be developed independently of the scoring change in OST-001; coordinate if a fixture touches the same test file.

**Acceptance:** Define the sampling interval, time window and same-level distance used to mark an event as shown. Test one major death within view and one outside view, and state how events without a location are counted. Report raw counts and shares by kind without changing simulation state.

### OST-005 · Record what a breakdown was over

**Milestone:** Make important events easier to follow · **State:** In review · **Developer:** Ostrog CI (OC) · **Public PR:** [#3](https://github.com/danhicks96/ostrog-public-ci/pull/3)

The chronicle records the kind of breakdown but not what the person was under strain about. Append a separate notable cause event immediately after the original breakdown, at the same tick. Prefer the strongest fear, grief, anger or shame at strength 0.3 or higher, then the heaviest negative thought, then a need below 0.2. If none qualifies, record that the cause is unknown. Name another person only when the recorded feeling identifies them. Do not invent a subject for a thought.

**Acceptance:** A staged breakdown with a known person as the cause names that person; a staged breakdown without a cause reports it as unknown. Preserve the original event and behavior, and keep existing event discriminants in order. Compare break kinds and counts before and after on matched seeds 1–4 over 60 game-days, and report the number and share with a known cause. Focused tests alone do not establish the long-run comparison.

## Waiting on code or measurement

### OST-003 · Compare long-run camera coverage

**State:** Waiting on dependencies · **Owner:** None · **Public PR:** None

**Dependencies:** A candidate ref with the major-event scoring change (OST-001), and working camera-coverage instrumentation (OST-004) available on both baseline and candidate refs. This comparison can run before merge or release once those refs are available.

**Acceptance:** On matched seeds 1–5 over 700 game-days, report total and missed major deaths before and after the scoring change, with the same camera sampling and missed-event definition. Report both per-seed counts and the pooled share. A run with no qualifying deaths has no share; do not count it as zero misses. Record the compared refs.

## Gated by a published release

No entries yet. Use this section only when work must wait for a named capability to ship. Record the capability and verification criteria; move the item out of this section only after linking the qualifying published release. Code availability, merge and CI prerequisites belong under ordinary dependencies.

## Stretch goals

No entries yet. Stretch ideas are not available to claim until promoted to `Ready` with concrete acceptance criteria.
