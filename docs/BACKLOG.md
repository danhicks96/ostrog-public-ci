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

### OST-325 · mind/intentions.rs is 522 dead lines and an earlier item was closed on it: fold its useful parts into aims

**Milestone:** Make the valley feel alive · **State:** Claimed · **Developer:** Ostrog CI (OX) · **Public PR:** Pending

The unused intentions module duplicates part of the planned aims work. Move its bounded slip/reminder chances and tests unchanged into mind/aims, preserve the retired file under to_delete, and remove the active module declaration. Keep forgetting behavior for OST-343. Tracking issue: [#501](https://github.com/danhicks96/ostrog-public-ci/issues/501).

**Acceptance:** Tests `mind::aims::tests::slip_chance_is_bounded` and `mind::aims::tests::remind_chance_is_bounded` (moved from `intentions.rs:453-480`). `cargo build` and `tools/ci.sh` are green, and digests are unchanged. Done when: no `intentions` module remains in `src/` and the two chance functions live in `aims.rs`.

### OST-326 · There is no count of what people want: add an aims census before any want rule lands

**Milestone:** Make the valley feel alive · **State:** Claimed · **Developer:** Ostrog CI (OX) · **Public PR:** Pending

There is no repeatable count of wants. Add a read-only opening census of adults with active wants, kinds, active duration, recorded causes and completed outcomes. Initially report zeros before generation rules land and test a planted want through its helper. Tracking issue: [#502](https://github.com/danhicks96/ostrog-public-ci/issues/502).

**Acceptance:** The example builds in CI (`cargo build --examples`) and prints zeros on seed 1 for 20 days today. Test `crates/hf-sim/tests/aims.rs::aims_census_counts_a_planted_want`: it plants one aim and the census helper counts it. Done when: the census runs and its helper is tested.

### OST-327 · Two nights on the ground and nobody wants a bed of their own: add the daily aims stage and first rule

**Milestone:** Make the valley feel alive · **State:** Claimed · **Developer:** Ostrog CI (OX) · **Public PR:** Pending

Repeated rough nights form no persistent want. Add the daily aims stage and deterministic OwnBed rule, using the existing rough-night counter, a bounded strength and deadline, and a recorded AimTaken cause. Run before personal projects are proposed. Tracking issue: [#503](https://github.com/danhicks96/ostrog-public-ci/issues/503).

**Acceptance:** Test `crates/hf-sim/tests/aims.rs::a_night_on_the_ground_twice_forms_a_want_for_a_bed`: a scenario with no beds, two nights, then `OwnBed` active with a cause event. Test `aims.rs::a_night_in_a_bed_does_not_form_the_want`. `aims_census` on seed 1, 60 days, shows OwnBed wants. Digest change logged in ITERATION_LOG. Done when: rough sleepers form the want and the feed says why.

### OST-328 · Nobody sets their heart on someone or on getting even with them: add the Win and GetEven want rules

**Milestone:** Make the valley feel alive · **State:** Claimed · **Developer:** Ostrog CI (OX) · **Public PR:** Pending

Affection and remembered wrongs form no personal aims. Add Win and GetEven rules from eligible relationships, personal memories and dispositions, with the stated deadlines and reserved AIMS purpose value 63. Exclude a partner or child from revenge. Tracking issue: [#504](https://github.com/danhicks96/ostrog-public-ci/issues/504).

**Acceptance:** Test `crates/hf-sim/tests/aims.rs::a_wrong_remembered_forms_get_even_with_its_doer`. Test `aims.rs::a_warm_person_lets_a_wrong_go_more_often_than_a_cold_one`: over 100 seeded days, the GetEven count for warmth 0.9 is below the count for 0.1. Test `aims.rs::a_liked_unpaired_neighbour_forms_win`. Done when: both rules fire in the census on seeds 1-4 at 60 days.

### OST-329 · Skilled makers never set out to make something fine outside a fey mood: add MakeFine and LearnCraft wants

**Milestone:** Make the valley feel alive · **State:** Claimed · **Developer:** Ostrog CI (OX) · **Public PR:** Pending

Skilled makers have no fine-work or learning wants. Add MakeFine from skill and industry or a recent masterwork. LearnCraft instead requires personal knowledge of a finished community station serving the craft, lack of the craft and curiosity at least 0.5; use the reviewed 0.05 daily chance and twenty-day deadline. Tracking issue: [#505](https://github.com/danhicks96/ostrog-public-ci/issues/505).

**Acceptance:** Test `crates/hf-sim/tests/aims.rs::a_skilled_industrious_carpenter_wants_to_make_something_fine`. Test `aims.rs::one_who_knows_the_bench_and_lacks_its_craft_wants_to_learn`. Done when: both rules fire in `aims_census` on seeds 1-4 at 120 days. A curious person who knows the finished bench and lacks its craft forms LearnCraft within forty seeded days; without that personal knowledge they never do.

### OST-330 · Why someone sets out alone is a dice roll among four phrases: make a lone trip come from a want

**Milestone:** Make the valley feel alive · **State:** Claimed · **Developer:** Ostrog CI (OX) · **Public PR:** Pending

Lone trips have dice-picked reasons. Add SeePlace only from news this person actually heard: record the latest overworld site, hearing tick and teller for awake community people within eight tiles and line of sight of the returner. Travel to that personally heard-of site and name the teller in the cause; retain the existing departure safety guards. Tracking issue: [#506](https://github.com/danhicks96/ostrog-public-ci/issues/506).

**Acceptance:** Test `crates/hf-sim/tests/journeys.rs::a_curious_one_with_a_tale_of_a_place_wants_to_see_it_and_goes_there`. Test `journeys.rs::a_lone_trip_names_the_news_behind_it`. Done when: in the seed 7 census, ≥ half of `went_away` follow a `SeePlace` want. The curious person three tiles from the returner hears the news, forms SeePlace and goes; one thirty tiles away does not. Use personal news rather than a deep-site tale in the curious-traveller case.

### OST-331 · New parents never want a home of their own or to keep their child safe: add HomeForHousehold and KeepSafe

**Milestone:** Make the valley feel alive · **State:** Claimed · **Developer:** Ostrog CI (OX) · **Public PR:** Pending

Parents have no wants for a household home or a child's safety. Add HomeForHousehold for an un-housed family with a child, held by the elder partner, and one KeepSafe aim while the proposed child or injured-partner condition holds. Tracking issue: [#507](https://github.com/danhicks96/ostrog-public-ci/issues/507).

**Acceptance:** Test `crates/hf-sim/tests/aims.rs::a_new_parent_without_a_house_wants_one_for_the_household`. Test `aims.rs::a_warm_parent_keeps_a_small_child_safe_and_the_want_ends_when_they_grow`. Done when: both fire on seeds 1-4 by day 200 in the census.

### OST-332 · Nobody has a lifetime aspiration: give each adult one, drawn from their past and temperament

**Milestone:** Make the valley feel alive · **State:** Claimed · **Developer:** Ostrog CI (OX) · **Public PR:** Pending

Adults have no enduring aspiration. Draw one once at arrival, joining or coming of age from dispositions and recorded past, using the six proposed aspiration families and a stable personal stream. Persist milestones without redrawing on reload. Tracking issue: [#508](https://github.com/danhicks96/ostrog-public-ci/issues/508).

**Acceptance:** Test `crates/hf-sim/tests/aims.rs::every_arriving_adult_has_an_aspiration_drawn_from_their_past`. Test `aims.rs::a_child_coming_of_age_draws_one`. Test `aims.rs::aspiration_draw_is_the_same_at_1_and_6_threads`. Done when: all adults on seeds 1-4 hold one, and no kind exceeds 35%.

### OST-334 · Everyone's day looks the same and nothing measures it per person: add day-spread keys to the alive report

**Milestone:** Make the valley feel alive · **State:** Claimed · **Developer:** Ostrog CI (OX) · **Public PR:** Pending

Existing day-distance keys do not expose each activity's individual spread. Extend the shared alive report with mean, range and p90-p10 shares and aim-lifted hours. Pool seeds 5-8 and people sampled for at least one hundred game-days; use the reviewed pooled comparison rather than seed 5 alone. Tracking issue: [#510](https://github.com/danhicks96/ostrog-public-ci/issues/510).

**Acceptance:** `aims_pursuit.rs::day_spread_counts_hours_by_activity` counts a scripted two-person day correctly. Run seeds 5-8 for 600 days, pool people sampled for at least 100 days, and compute the reported loitering p90-p10 spread. It is within two percentage points of the audit's pooled 6.7. If the source used a different spread, state that and recompute the comparable reference rather than comparing seed 5 alone.

### OST-335 · How far along a want is cannot be read: add a pure progress function per want kind

**Milestone:** Make the valley feel alive · **State:** Claimed · **Developer:** Ostrog CI (OX) · **Public PR:** Pending

A want has no pure progress reading. Add bounded progress per kind from the holder's own state and known outcomes. Win reads their own familiarity divided by 0.8, or pairing, never the target's hidden opinion. GetEven and SeePlace return zero here and complete through recorded events in OST-342. Household progress depends on household ownership. Tracking issue: [#511](https://github.com/danhicks96/ostrog-public-ci/issues/511).

**Acceptance:** Unit tests in `crates/hf-sim/src/aims.rs`: `progress_of_own_bed_is_one_when_owned`; `progress_of_make_fine_tracks_best_quality`; `progress_of_win_is_one_when_paired`; `progress_is_pure_and_does_not_touch_rng`. Add `progress_of_win_reads_only_the_holders_familiarity` and `progress_of_get_even_and_see_place_is_zero`. Done when these six named tests pass; event-only wants return zero and are completed by OST-342.

### OST-336 · The choice has no slot for what a person wants: add a neutral aim factor to choose and the Why? lens

**Milestone:** Make the valley feel alive · **State:** Claimed · **Developer:** Ostrog CI (OX) · **Public PR:** Pending

Decision scores have no place for personal aims. Add a neutral aim factor, initially 1.0 everywhere, to the existing score and Why trace without changing choices or digests. Tracking issue: [#512](https://github.com/danhicks96/ostrog-public-ci/issues/512).

**Acceptance:** Test `crates/hf-sim/tests/aims_pursuit.rs::aim_term_is_one_without_aims`. `crates/hf-sim/tests/why_lens.rs` shows an `aim` column. `tests/pipeline.rs` digests and `tests/threads.rs` are unchanged. Done when: the factor exists, shows in Why?, and nothing else changes.

### OST-337 · Someone who wants a bed of their own spends their spare hours like everyone else: lift work on sleeping places

**Milestone:** Make the valley feel alive · **State:** Claimed · **Developer:** Ostrog CI (OX) · **Public PR:** Pending

Wanting a bed or household home does not lift its work. Weigh relevant construction, crafting and material hauling through the aim factor while preserving the security gate. Household work requires OST-331 and household ownership and must target the holder's own house. Tracking issue: [#513](https://github.com/danhicks96/ostrog-public-ci/issues/513).

**Acceptance:** Test `crates/hf-sim/tests/aims_pursuit.rs::one_who_wants_a_bed_takes_the_bed_order_over_a_field_order`. Test `aims_pursuit.rs::a_hungry_bed_wanter_still_eats_first`. The alive report's day-spread keys (OST-334) show lifted-goal hours > 0. Done when: both tests pass and the digest change is logged. Depend on household ownership and OST-331. `aims_pursuit.rs::a_household_wanter_takes_their_own_houses_wall_order` chooses the holder's own household wall order over an equally distant other house.

### OST-338 · Nobody seeks out the one they are sweet on: Win(x) lifts talk with x and the hearth when x is there

**Milestone:** Make the valley feel alive · **State:** Claimed · **Developer:** Ostrog CI (OX) · **Public PR:** Pending

Wanting someone does not lead to company with them. Lift talk and gathering with the desired person under the existing leisure gate and bound the combined courtship and aim effects. Tracking issue: [#514](https://github.com/danhicks96/ostrog-public-ci/issues/514).

**Acceptance:** Test `crates/hf-sim/tests/aims_pursuit.rs::wanting_someone_seeks_their_company_but_not_while_hungry`. Test `aims_pursuit.rs::wanting_someone_sits_at_the_hearth_they_sit_at`. Done when: both pass, and `Talked` events between Win pairs rise ≥ 2× against matched non-Win pairs on seeds 1-4.

### OST-340 · A want neither ends nor is ever told to have ended: close wants as met or given up, on record

**Milestone:** Make the valley feel alive · **State:** Claimed · **Developer:** Ostrog CI (OX) · **Public PR:** Pending

Wants never finish or give up. End a want as Met at full progress or GivenUp after the stated no-progress interval or deadline, move it to the bounded completed store and record its actual outcome. Tracking issue: [#516](https://github.com/danhicks96/ostrog-public-ci/issues/516).

**Acceptance:** Test `crates/hf-sim/tests/aims_outcomes.rs::a_bed_of_ones_own_ends_the_want_as_met`. Test `aims_outcomes.rs::a_want_with_no_progress_for_five_days_is_given_up`. Done when: both pass, and `aims_census` reports outcomes.

### OST-341 · Getting what you wanted, or losing it, is not felt: add thoughts for wants met, thwarted and given up

**Milestone:** Make the valley feel alive · **State:** Claimed · **Developer:** Ostrog CI (OX) · **Public PR:** Pending

A want's outcome has no feeling. Add strength-scaled thoughts for meeting, losing or giving up a want and for aspiration milestones, using the recorded completion path. Tracking issue: [#517](https://github.com/danhicks96/ostrog-public-ci/issues/517).

**Acceptance:** Test `crates/hf-sim/tests/aims_outcomes.rs::a_met_want_leaves_a_glad_thought_scaled_by_strength`. Test `aims_outcomes.rs::a_given_up_want_leaves_a_small_sour_one`. Done when: both pass.

### OST-342 · When what you wanted becomes impossible, nobody notices: thwart wants from recorded events

**Milestone:** Make the valley feel alive · **State:** Claimed · **Developer:** Ostrog CI (OX) · **Public PR:** Pending

Events do not close impossible or event-completed wants. Add an observation stage for thwarting through pairing, death, departure, house loss or a site ending. Also mark GetEven as Met after the holder's qualifying fight or accusation, and SeePlace on the holder's return, since pure progress cannot read those events. Tracking issue: [#518](https://github.com/danhicks96/ostrog-public-ci/issues/518).

**Acceptance:** Test `crates/hf-sim/tests/aims_outcomes.rs::the_one_wanted_pairing_with_another_thwarts_win`. Test `aims_outcomes.rs::a_kept_child_dying_thwarts_keep_safe`. Done when: both pass. `aims_outcomes.rs::a_fight_won_meets_get_even` verifies a qualifying holder-initiated fight after the want began ends GetEven as Met with AimMet; a return completes SeePlace.

### OST-343 · An earlier item was closed but nobody ever forgets anything they meant to do: let a tired, stressed mind forget a want

**Milestone:** Make the valley feel alive · **State:** Claimed · **Developer:** Ostrog CI (OX) · **Public PR:** Pending

A tired or stressed person never forgets a want. Use the preserved bounded slip chance in the daily aims stage with the reserved stream; a forgotten want ends quietly without a strong thought or event. Tracking issue: [#519](https://github.com/danhicks96/ostrog-public-ci/issues/519).

**Acceptance:** Test `crates/hf-sim/tests/aims_outcomes.rs::a_tired_stressed_person_forgets_more_wants_than_a_rested_one`: two scenario people, 100 days, the tired one's forget count is greater. Done when: it passes, and the earlier item can be closed against this slice.

### OST-344 · Two who make up still carry the grudge as a want: making up lets GetEven go

**Milestone:** Make the valley feel alive · **State:** Claimed · **Developer:** Ostrog CI (OX) · **Public PR:** Pending

Making up does not release an aim to get even. End the matching aims on MadeUp, register the small relief and record LetItGo, retaining the original reconciliation event. Tracking issue: [#520](https://github.com/danhicks96/ostrog-public-ci/issues/520).

**Acceptance:** Test `crates/hf-sim/tests/aims_outcomes.rs::making_up_lets_getting_even_go`. Done when: it passes.

### OST-345 · A family gets a house and only one of them is glad; a rival pairs off and nobody is jealous

**Milestone:** Make the valley feel alive · **State:** Claimed · **Developer:** Ostrog CI (OX) · **Public PR:** Pending

A want's outcome affects only its holder. Share household gladness, bound jealousy when the desired person pairs elsewhere, and let the target of revenge remember the harm. Reuse one appended Jealous thought across the related slices. Tracking issue: [#521](https://github.com/danhicks96/ostrog-public-ci/issues/521).

**Acceptance:** Test `crates/hf-sim/tests/aims_outcomes.rs::the_one_wanted_pairing_with_another_breeds_jealousy_of_the_rival`. Test `aims_outcomes.rs::a_household_shares_the_gladness_of_a_house_built`. Test `aims_outcomes.rs::getting_even_leaves_the_other_a_grudge`. Done when: all three pass.

### OST-346 · Wants that came true or were dashed leave no memory, so the next want ignores them

**Milestone:** Make the valley feel alive · **State:** Claimed · **Developer:** Ostrog CI (OX) · **Public PR:** Pending

Met or thwarted wants leave no memory for later choices. Record the proposed personal work, company, hardship or discovery memories and let recent success or rejection adjust later aim generation. Tracking issue: [#522](https://github.com/danhicks96/ostrog-public-ci/issues/522).

**Acceptance:** Test `crates/hf-sim/tests/aims_outcomes.rs::a_rebuffed_suitor_tries_the_same_person_less`. Done when: it passes.

### OST-556 · A maker who wants to make something fine does not seek out their bench: MakeFine(c) lifts orders whose recipe is craft c

**Milestone:** Make the valley feel alive · **State:** Claimed · **Developer:** Ostrog CI (OX) · **Public PR:** Pending

Wanting fine work does not lead a maker to their craft. Lift matching recipe orders through MakeFine and cap its product with craft liking at 1.6; measure the holder's bench-work share before and after. Tracking issue: [#565](https://github.com/danhicks96/ostrog-public-ci/issues/565).

**Acceptance:** Test `crates/hf-sim/tests/aims_pursuit.rs::a_maker_who_wants_fine_work_takes_their_crafts_order_first`. Two open orders at equal distance, a Craft of `c` and a field order. A holder of `MakeFine(c)` at strength 1.0 gets `lean` 1.5 for the craft order and 1.0 for the field order, and takes the craft order. Measured: share of a `MakeFine` holder's work hours at their craft's bench, seeds 1-4, 300 days, against the same people before the want. development-history entry.

### OST-557 · A maker set on fine work takes the nearest planks, not the best: with MakeFine, a craft plan picks the highest-quality known stack

**Milestone:** Make the valley feel alive · **State:** Claimed · **Developer:** Ostrog CI (OX) · **Public PR:** Pending

A maker aiming at fine work still takes the nearest material. Add an ordering option to the existing known-item search so only a matching MakeFine craft plan prefers quality, then distance, without reserving anything or changing other searches. Tracking issue: [#566](https://github.com/danhicks96/ostrog-public-ci/issues/566).

**Acceptance:** Test `crates/hf-sim/tests/aims_pursuit.rs::a_fine_work_want_picks_the_best_free_wood`. Two known, reachable plank stacks, quality 0.4 at 3 tiles and 0.9 at 10 tiles. The plan of a `MakeFine(Carpentry)` holder for a carpentry order takes the 0.9 stack; the same person without the want takes the 0.4 stack. Measured: mean quality of crafts made by `MakeFine` holders, seeds 1-4, 300 days, against their own crafts before the want. development-history entry.

### OST-558 · A grudge holder tends and chats with the one they mean to get even with: GetEven(x) lowers Tend(x) and Talk(x) to 0.8

**Milestone:** Make the valley feel alive · **State:** Claimed · **Developer:** Ostrog CI (OX) · **Public PR:** Pending

A revenge aim does not affect helping or friendly talk. Apply the bounded 0.8 Tend and Talk factor toward its target while preserving care when no other healer can help. Tracking issue: [#567](https://github.com/danhicks96/ostrog-public-ci/issues/567).

**Acceptance:** Test `crates/hf-sim/tests/aims_pursuit.rs::getting_even_never_tends_the_other_first_when_another_healer_is_free`. X hurt, two healers at equal distance, one holding `GetEven(X)`: the other healer tends X. With the holder as the only healer, X is still tended within the day. Measured: not a census key; the digest change is noted in the development history. development-history entry.

### OST-559 · Two who bear a grudge talk as amiably as anyone: an active GetEven adds to the ill will that decides whether a talk sours into a quarrel

**Milestone:** Make the valley feel alive · **State:** Claimed · **Developer:** Ostrog CI (OX) · **Public PR:** Pending

A revenge aim leaves an ordinary talk equally amiable. Add the proposed strength-weighted ill will before the existing capped quarrel test, keeping its stream and measured comparison. Tracking issue: [#568](https://github.com/danhicks96/ostrog-public-ci/issues/568).

**Acceptance:** Test `crates/hf-sim/tests/aims_pursuit.rs::getting_even_makes_a_quarrel_likelier_when_they_talk`. 200 seeded talks between the same pair at opinion 0 and mood 0: the `Quarrelled` count when one holds `GetEven` at strength 1.0 is at least twice the count without it. Measured: quarrel rate between GetEven pairs against all pairs, seeds 1-4, 300 days (a sibling item's 2× target). development-history entry.

### OST-560 · Wanting to learn a craft changes nothing: LearnCraft(c) lifts study of things that teach c and a leisure walk to a known bench of c

**Milestone:** Make the valley feel alive · **State:** Claimed · **Developer:** Ostrog CI (OX) · **Public PR:** Pending

Wanting a craft changes neither study nor free time. Lift study of known teaching items and offer a secure leisure walk near a personally known station for that craft. Learning still uses the existing observation and teaching rules. Tracking issue: [#569](https://github.com/danhicks96/ostrog-public-ci/issues/569).

**Acceptance:** Test `crates/hf-sim/tests/aims_pursuit.rs::wanting_a_craft_stands_by_its_bench_when_free`. A secure, rested person holding `LearnCraft(Carpentry)` who knows a carpentry bench 15 tiles away stands within 2 tiles of it at least once in 6 game-hours. With stores under two days, they never take that walk. Measured: hours LearnCraft holders spend within 2 tiles of their craft's bench, seeds 1-4, 300 days. development-history entry.

### OST-561 · A parent who wants to keep a small child safe sleeps wherever is nearest: KeepSafe(x) lifts a bed near x's and a walk toward x

**Milestone:** Make the valley feel alive · **State:** Claimed · **Developer:** Ostrog CI (OX) · **Public PR:** Pending

A protective parent ignores the child's sleeping place. Lift a bed near the child's claim, or near the child when actually in sight, and a visible nearby seek target, preserving ordinary choice otherwise. Tracking issue: [#570](https://github.com/danhicks96/ostrog-public-ci/issues/570).

**Acceptance:** Test `crates/hf-sim/tests/aims_pursuit.rs::a_parent_keeping_a_child_safe_sleeps_near_them`. At dusk, a child asleep in a bed, and two free beds equally far from the parent, one 3 tiles from the child and one 15. The parent holding `KeepSafe(child)` takes the near one; without the want, the choice follows the existing tie rule. Measured: not a census key; the digest change is noted in the development history. development-history entry.

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
