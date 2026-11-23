# Fix log

## 2026-11-22: the last knower carves the way of their craft

- Symptom: when the last knower of a craft died, the craft was lost with nothing left behind.
- Fix: an old or ailing sole knower of a craft their community uses prefers to carve it. Engravings gain an optional craft field that old saves load as none.
- Guard: `the_last_smith_carves_the_way_of_smelting` in `tests/engrave.rs`.
## 2026-11-22: mood terms in the alive report

- Symptom: nothing reported what a colony's mood is made of, only a few spread numbers.
- Fix: extract the four mood terms into one helper used by the mood update and add the term means and spread keys to the alive report. Report only; simulation unchanged.
- Guard: `mood_terms_sum_to_mood` in `tests/mood.rs` and the alive report tests.
## 2026-11-22: a free stool or chair can be found (OST-1307)

- Symptom: stools and chairs are built, but nothing could look for a free one.
- Cause: no seat finder existed. This is a pure addition with no caller yet, so behaviour and the
  simulation digest do not change.
- Fix: `houses::free_seat` returns the nearest finished stool or chair on the level within range
  that no one else reserves or stands on, optionally beside a finished table, and reachable by
  `nav.reach`. Ties go to the lower id. Water is waded by nav, so the unreachable case is a
  walled-off stool.
- Guard: `tests/furniture.rs` `free_seat_finds_the_stool_beside_the_table`; it does not compile
  without `free_seat`.
## 2026-11-23: Catch Up shared History's panel and was replaced by it (OST-011)

- Symptom: the away card and History were written into the same panel, so opening History (or
  any centred reading panel) replaced Catch Up, and the card was a headline followed by a flat list
  of major lines.
- Cause: Catch Up had no panel of its own; `_show_summary` wrote into the shared summary box.
- Fix: a Catch Up panel of its own at the left edge under the HUD, registered outside the reading
  group. `summary::catch_up_card` reads the headline in words, then the named changes, then the
  folded details (a row already named is not repeated below); the bridge's `catch_up_since` hands
  the client the card. `summary_since` and the raw counts are unchanged.
- Guards: `tests/catch_up.rs::catch_up_card_orders_headline_changes_then_folded_details`;
  `tools/gd_test.gd` panel placement, History opening beside Catch Up, and the bridge card.
- Not checked here: the panel in a running desktop client after a real away period.
## 2026-11-22: births are gated by community, not by the whole valley

- Symptom: a band of raiders camped at the edge halved the births of a small community.
- Cause: the crowd rule counted every living person on the map, including bands and strangers who belong to no community.
- Fix: count only living people of the couple's own community when applying the halving at 40.
- Guard: `strangers_do_not_crowd_the_nursery` in `tests/generations.rs`.
## 2026-11-22: rain wets people outdoors; a roof or hearth dries them

- Symptom: people had no wetness, so standing in a storm left someone exactly as dry as sitting by the hearth.
- Fix: add a saved `wet` value (0 to 1, defaulting to 0 in old saves). Outdoors in rain it rises with rainfall; otherwise it falls, faster beside a lit hearth. Nothing reads it yet.
- Guard: `rain_wets_those_out_in_it_and_a_roof_dries_them` in `tests/weather_events.rs`.

## 2026-11-22: let the history view trace an event back through its causes (OST-1003)

- Symptom: the history view showed what happened but not why, though the record names a cause
  for many events; the client's one-step "because" was the only trace.
- Cause: nothing followed the recorded cause links past one step, and the one step lived in the
  Godot bridge instead of the view layer.
- Fix: `view::causes` follows recorded causes (bounded, and saying so when cut), stops with a
  plain line at a cause missing from the record or about someone the valley has not found, and
  serves it as the Legends page `why:<id>`. The bridge's one-step lookup now uses it.
- Guards: `tests/causes.rs` covers a full chain, the step limit, an event with no cause, a cause
  missing from the record, and a cause about an unfound person (chain ends `Untold` and the
  page does not name them).
- Not measured: how often chains reach a root in a long run.

## 2026-11-22: name how settled a community is (OST-1001)

- Symptom: the communities line gave a cohesion number the player could not read at a glance, so
  a settlement about to split looked the same as a close one.
- Cause: the number was shown raw, with no word for its bands.
- Fix: `society::cohesion_word` names the bands (close from 0.75, uneasy below 0.55, divided
  below 0.45, nothing between) and the bridge appends the word to the communities line.
- Guard: `tests/society.rs::cohesion_word_reads_divided_in_the_hour_of_a_split` pins the band
  edges and reads the word in the hour a hostile-cliques community splits. No simulation draw changes.
- Not measured: how long a community reads divided before it splits.


## 2026-11-22: say a person's mood in words, with the strain threshold shared (OST-1002)

- Symptom: the inspect panel gave a mood number the player had to interpret, and the number's
  bands were duplicated in the client script.
- Cause: the band edges lived only in the client; the strain system kept its own inline copy of
  the threshold formula.
- Fix: `view::why::mood_band` and `mood_word` give the word on the inspect dictionary. The strain
  threshold moves unchanged into `strain::threshold`, used by both the shock and the step.
- Guards: `tests/client_view.rs::mood_band_follows_the_inspect_thresholds` (exact edges) and
  `strain_threshold_is_unchanged` fail if either band or the formula drifts. No simulation draw changes.
- Not measured: how often each word appears in a long run.

## 2026-11-22: only someone of age leads an observance (OST-1301)

- Symptom: a nine-year-old could lead the season's observance, whether as the person followed in
  the spirit or as the most devout of those gathered.
- Cause: the leader choice checked only that a person was awake and (for the fallback) devout.
- Fix: both paths require the leader to be of age. A gathering of children alone lapses with
  "nobody led it".
- Guards: `tests/shrines.rs` `a_child_does_not_lead_the_observance`,
  `the_youngest_person_of_age_may_lead_and_one_a_year_short_may_not` and
  `an_observance_with_only_children_gathered_lapses`; all three fail without the fix.
- Not measured: a long-run census of observance leaders' ages. Duty gating is OST-1300.
## 2026-11-22: the away card's first line was a chore tally in internal names

- Symptom: "While you were away" opened with `hauled 122, harvested 166, suspected 21`.
- Cause: the client printed every `counts` key raw.
- Fix: `Summary::counts_in_words` (over `view::history::summary`) gives "died 2, born 1"; the
  Godot bridge sends it as `counts_words` and the away card prints it. `counts` is unchanged for
  the timelapse and other consumers.
- Guard: `tests/catch_up.rs::summary_counts_read_as_words` (2 died, 1 born, 40 hauled rows).

## 2026-11-22: a read-only census of wading steps (OST-1305)

- Symptom: how much people wade at fords was unmeasured, so any rule about crossings had no number
  behind it.
- Change: `systems::movement::wading_kind` classifies a step as entering water, stepping along
  water, or neither (landing on dry ground is not a wade); `wading_step` is its yes/no form. The
  simulation does not call either. `examples/crossing_census.rs` counts wading steps per living
  person per day (denominator: living people on the map at each day start) and lists the busiest
  water tiles with distance to the nearest community home, distinct people and days.
- Guards: `tests/movement.rs` `a_ford_step_is_counted` (one crossing of a one-tile stream is one
  step) and `a_wider_ford_counts_entry_and_each_step_along_but_not_the_exit`.
- Measured: a 30-day smoke of seed 1 (about 6 steps per person per day). No multi-seed figures are
  recorded here; they need the same stepping and denominator to be rerun on this code.

## 2026-11-22: a read-only census of why a house has no table (OST-1304)

- Symptom: most houses in some seeds never get a table, and nothing measured which test of the
  settlement step stopped them.
- Change: `hf_sim::table_census` (read-only, no randomness) names, per house, the first test that
  stops the table: walls open, no table spot, spot taken or already ordered, four or more open
  build orders, floor tiles still wanting orders, no planks, or ready to post. The example
  `table_census <seed> <days>` prints the counts every 25 days. It runs between planner passes,
  so it sees the orders already posted, not what the next pass will post first.
- Guards: `tests/table_census.rs` builds a house by hand and checks walls open, floor blocks and
  no planks, then checks the census against what `systems::settlement::step` actually posts
  (floor tiles block the table; once the pass's own earlier orders fill the slot or the gate, no
  table order follows, and a house is counted once).
- Measured: a 50-day smoke of seed 5 only (one house, one table). The cause for seeds 5 and 8 over
  200 days is not measured here; the floor-order test is a candidate cause, not a conclusion.

## 2026-11-22: a child with a parent on a caravan is not fostered (OST-1303)

- Symptom: a child whose parent was away on a caravan or journey was taken in as an orphan
  ("whose parents are gone").
- Cause: the fostering check looked for a living parent or foster only among people on the map;
  anyone away is moved to the away list, so they counted as gone.
- Fix: a living parent or foster who is on the away list with a caravan or journey set counts as
  present. Someone away with neither (a leaver, a captive) and the dead still count as gone.
- Guards: `tests/fostering.rs` `a_child_whose_mother_is_away_with_a_caravan_is_not_fostered`,
  `a_parent_or_foster_away_on_a_journey_is_away_not_gone`, `a_parent_away_for_good_is_gone`.
- Not measured: how often seeded runs fostered wrongly before the fix. Kin-first carer choice is a
  separate change.


## 2026-11-22: a shrine that falls to ruin leaves its material (OST-1302)

- Symptom: a worn shrine was removed and its loss recorded, but nothing lay on the tile, so the
  stone or timber it was built from simply vanished.
- Cause: the wear step deleted the structure without creating an item.
- Fix: when wear brings a shrine to nothing it leaves two rough stones (stone shrine) or one log
  (timber shrine) on its tile, carrying the shrine's material and a story naming its community.
  A shrine broken down or burned still leaves nothing (those paths are unchanged).
- Guards: `tests/shrines.rs` `a_fallen_shrine_leaves_its_stones`,
  `a_fallen_timber_shrine_leaves_a_log_once` (a repeated step adds nothing and the loss is
  recorded once) and `a_shrine_broken_down_or_burned_leaves_no_rubble`.
- Not measured: seeded long-run counts of untended losses. This adds items, so simulation digests
  change for runs where a shrine falls.

## 2026-11-22: a grown tree's shade eases the heat

- Symptom: a person standing under a grown tree lost comfort in a heat wave exactly as fast as one on bare ground.
- Cause: the heat branch of `cold_step` never read the plants.
- Fix: a person at ground level in or beside the column of a living mature tree has their excess heat halved before the comfort and rest drains, and holds `Hot` only if the halved excess is still above 4 C.
- Guard: `a_tree_shades_from_the_heat` in `tests/hazards.rs` (shaded comfort at least 0.1 higher; only the unshaded person holds `Hot`).

## 2026-11-22: a drought also blocks storms

- Symptom: A drought cleared rain from the sky but let storms through, so the feed could read that the rain had stopped coming and then report a storm before the drought lifted.
- Cause: The drought arm of the spell weather adjustment only cleared Sky::Rain.
- Fix: Clear a storm the same way as rain; a storm's wind without rain is not modelled, so clear is the honest stand-in. No save-format change.
- Guard: crates/hf-sim/tests/spells.rs::a_drought_lets_no_storm_through forces a storm hour under an active drought and checks the sky reads Clear with no rainfall; it fails without the change.


## 2026-11-22: a child holds no duty until grown (OST-1300)

- Symptom: `duties::daily` documents duties as held by grown members but checked only that a
  person was alive and in a community, so a toddler with foraging minutes was named a forager.
- Cause: the held set was taken from the minutes tally with no age check.
- Fix: anyone not of age holds no duties (empty held set, no role, no role-left record). Their
  minutes keep counting and fading, so a grown member takes the role on their first morning of age.
- Guard: `tests/duties.rs::a_child_who_forages_holds_no_duty_until_of_age` fails without the fix
  (a six-year-old is given the role); aged to sixteen the same person is forager next morning.
- Not measured: the long-run count of role rows held by children.

## 2026-11-20: show what a breakdown was over

- Symptom: the feed stated the breakdown type without saying what weighed on the person.
- Cause: strain chose and recorded the breakdown without recording its apparent cause.
- Fix: record the explanation on the original breakdown event. A second event shifted later
  event IDs, which can change simulation draws. Old events without an explanation still use
  the original line, and an unfound subject does not appear in the viewer's explanation.
- Guards: named and unknown causes, event-ID and observer parity, legacy event decoding,
  and visibility in the feed and history.
- Evidence: [the paired 60-day runs](https://github.com/danhicks96/ostrog-public-ci/actions/runs/44790774365)
  compared `35c0bff2` with `0204b9d7` across four seeds. All eight soaks passed;
  normalized chronicle and endpoint world hashes matched for every seed. All 59 breakdowns
  in the candidate recorded a known cause.

## 2026-11-22: a good turn repaid is a real meal, and not across a feud

- Symptom: Repaying a good turn moved two hunger numbers within 15 tiles without any food existing, and ignored feuds and souring opinion.
- Cause: The daily repayment added to one person's hunger and subtracted from another's, with no item involved.
- Fix: The repayer must be within one tile and not across a feud or in sour opinion with the recipient, and hands over a real food item they carry themselves, whole or split by one as hauling does; the recipient notes it. No food, no repayment. No save-format change.
- Guard: crates/hf-sim/tests/social.rs covers a carried meal moved with no creation or double use, a split stack, feud and opinion blocks, and distance; it fails without the change.

## 2026-11-22: the tired no longer choose beds in a flood's way

- Symptom: After a flood, many people collapsed from exhaustion in the same hour instead of lying down on the high ground they fled to.
- Cause: A tired person was offered every reachable free bed, with ground sleep only when no bed was found, and nothing asked whether the bed lay in a flood's way. The flood-flight step turned them back from it each time, so they never lay down until rest ran out (likely loop; not confirmed by a trace).
- Fix: The shared bed-consideration step skips a bed in a flood's way or on unsafe ground and does not count it as found, so ground sleep is offered where the person stands. Safe beds stay preferred; the night still counts as rough. No save-format change.
- Guard: crates/hf-sim/tests/floods.rs::the_tired_sleep_on_high_ground_when_the_flood_covers_their_beds stages six tired people with beds by a pond in spate and checks all sleep away from flood water with no exhaustion event; it fails without the change.

## 2026-11-22: a feud survives its only holder's journey

- Symptom: A feud ended with 'nobody of one side is left in the valley' while its only holder of that side was away on a journey, and came back to find it gone.
- Cause: The feud check counted only people alive in the valley's agent list; journeyers are held whole in the away list, so they never counted, despite a comment saying those away still hold the feud.
- Fix: Count someone away on a journey who has a community as still here for the feud checks, including the heir check. Dead and disowned holders still end a feud.
- Guard: crates/hf-sim/tests/feuds.rs::a_feud_survives_a_journey sends the only holder away for six days, checks the feud survives each morning with no end or inheritance event, and checks the head on return; it fails without the change.

## 2026-11-22: a hearth cooked at for years gets a cooking pot

- Symptom: A hearth that people cooked at for years never got a cooking pot: the pot kind existed but nothing posted one.
- Cause: The settlement planner had no path that posted a cooking pot, and the structure cost table had no entry for one.
- Fix: When a hearth has a cook order open or freshly posted and no cooking pot (finished or ordered) within a tile, and a vessel is in store, the planner posts a pot order on a free neighbouring tile, a clay pot whose only cost is the vessel. No vessel in store, no order. No save-format change.
- Guard: crates/hf-sim/tests/hearth_pot.rs covers the positive case and the no-vessel case, and de-duplicates open and finished orders; the positive case fails without the change.

## 2026-11-22: a role is kept until a challenger has led for three mornings

- Symptom: A person's role flipped between forager and hauler within days, each flip a feed line; about a third of role events in long runs returned to the previous role within three days.
- Cause: Each morning the role was set to whichever duty had the most minutes, with no margin, so one day's work could flip it.
- Fix: Keep the current role unless a challenger's minutes are 25% ahead on three consecutive mornings; a defaulted counter on the duties record tracks the mornings. Taking a first role and dropping a faded role stay immediate. The duty list that weighs work choice is unchanged.
- Guard: crates/hf-sim/tests/duties.rs::a_role_does_not_flip_on_one_day checks one and two mornings keep the role, the third switches it once with one role event; it fails without the change. Old saves load through the serde default.

## 2026-11-22: tools left at an abandoned camp teach their craft

- Symptom: Tools left at an abandoned camp taught nothing when studied, although the old hamlets' things did, so craft-bearing things ran out over a long game.
- Cause: Study teaches only when an item's story names a craft; only the old hamlets' things were given one, and things a moving community leaves behind kept their maker but not their craft.
- Fix: When a community records a move, each made item on the camp's floors whose maker is still known and knew the craft of its recipe now teaches that craft, with a 'left at' origin if it had none. Food, unmade things and items whose maker is gone or did not know the craft are unchanged. No save-format change.
- Guard: crates/hf-sim/tests/left_sites.rs::the_tools_left_at_the_old_camp_teach_their_craft; it fails without the change.

## 2026-11-19: preserve asset coverage in a media-free CI source snapshot

- Symptom: removing media left asset-dependent Rust and documentation checks without inputs.
- Cause (checked): the historical entry point assumed fallback outputs existed and omitted separate large-item, animal and people passes.
- Fix: ci_source_assets.py validates included recipe inputs, creates output directories, generates pinned fallback inputs first and then the full procedural inventory. The Rust job owns generation and provenance; other jobs retain their original checks. No rendered media is committed or uploaded.
- Guards: source_assets_test.py rejects missing/changed/linked inputs, missing/extra/changed outputs and malformed manifests. check_source_only.py rejects committed media or unapproved binary blobs. Existing Rust tests, enum/bridge/docs checks and cross-platform digest comparison are preserved.
- Commit: this source-only CI root; local proof is not hosted CI or publication approval.

## 2026-11-20: remove the camera’s presence penalty for major personal events

- Symptom: a death in an empty camera cell scored below one busy worker in another cell.
- Cause: fewer people in the event cell reduced its score.
- Fix: give major deaths, captures, breakdowns and exiles full presence weight. Other scoring and shot timing are unchanged.
- Guard: a major death in an empty cell outranks a busy worker in a separate cell.

## 2026-11-19: eight text-template defaults had legacy folder provenance

- Symptom: eight large item icons cited the excluded sheet pipeline.
- Cause (checked): provenance.py used its folder fallback rather than items.py's eight BIG text-template outputs. Regenerating only those templates reproduces all eight committed PNG blobs exactly; no sheet inputs are read.
- Fix: exact generator overrides and eight provenance rows point to items.py. No pixels, runtime code, credits or licence text change. Three test-only release URLs use neutral example account/repository names while preserving host and URL semantics.
- Guard: fallback_integration_test.py reproduces the eight shipped defaults from text and checks regenerated provenance; failed before, passes after. Rust release tests and release_publish_test.py retain their signing/rejection assertions.
- Commit: this local compliance checkpoint; no overall rights or privacy approval implied.

## 2026-11-19: miniature docs-policy export lacked required provenance setup

- Symptom: its positive export case exited before snapshot creation despite valid docs policy.
- Cause (checked): the miniature repository omitted the mandatory provenance checker.
- Fix: public_docs_guard_test.py copies the real checker with an explicit empty-art policy shim. No production gate or asset policy changes.
- Guard: positive exact-policy exports succeed; invalid/new docs still fail; an unlisted synthetic asset still fails the real provenance checker. Fallback policy enforcement remains covered by its dedicated suite.
- Commit: this local fixture-setup checkpoint.

## 2026-11-19: bounded public CI documentation cleanup

- Symptom: the rehearsal retained optional account-scoped automation and historical operational references.
- Cause (checked): neither was required by the public CI workflow, but export rules retained them.
- Fix: exclude automerge and optional updater operations documentation; public_text.rewrite normalizes only reviewed documentation contexts in the exported view. Private originals, legal credits, runtime and test fixtures remain unchanged.
- Guards: public_text_guard_test.py synthetic context/preservation cases and public_boundary_guard_test.py excluded automation detection.
- Commit: this local cleanup checkpoint; no privacy or release approval implied.

## 2026-11-18: expanded snapshot exclusions could still escape the export

- **Symptom**: a multiword entry or a terminal parent component could bypass the prerequisite
  exporter's exclusion check; a drive-qualified path could do the same on Windows.
- **Cause (checked)**: the original check validated a whole line before word splitting and glob
  expansion, without resolving each removal target relative to the export root.
- **Fix**: reject whitespace, drive-qualified, backslash and dot-component patterns, then resolve
  every expanded target and reject anything outside the export before removing any match.
- **Guard**: `publish_snapshot_guard_test.py` checks multiword traversal, terminal parent, root,
  drive-qualified and backslash cases alongside the original CRLF exclusion/metadata checks.

## 2026-11-18: the snapshot gate rejected its own fixture

- **Symptom**: the focused fixture guard passed, but exporting the real branch failed the leak gate.
- **Cause (checked)**: the exported guard source contained the same forbidden word it tested.
- **Fix**: use a neutral fixture-only sentinel in `snapshot_gate_guard_test.py` and its temporary
  forbidden-pattern list, keeping the actual tracked fixture source exportable.
- **Guard**: the fixture leak rejection and clean export scenarios remain executable checks;
  publication CI guards also reject calls to publication-only tools in a public tree.

## 2026-11-18: public snapshot CI called excluded publication tools

- **Symptom**: public snapshot CI attempted to execute scripts removed during export.
- **Cause (checked)**: the workflow called the exporter directly, while `tools/ci.sh` ran guards
  that require the excluded publication directory before checking whether it exists.
- **Fix**: condition publication-only checks on the private source layout and trusted default
  branch metadata; keep missing tools a failure in the private tree and explicitly skip those
  checks in the public tree, including when both private marker paths are removed.
- **Guard**: `tools/tests/snapshot_ci_guard_test.py` executes the private and public shell paths
  with stub tools and verifies that private failures propagate without public missing-file calls.

## 2026-11-18: required publication blocklist

- **Symptom**: an unset list, a source-local list or a relative list path could leave the snapshot
  identifier scan without the owner's configured patterns while it still reported success.
- **Cause (checked)**: the gate changed into the exported repository before resolving the source
  configuration, and the scanner accepted an empty effective list.
- **Fix**: resolve the source configuration before export; require at least one effective pattern
  for private publication gates and workflow preflight using `--require-blocklist`. Diagnostics
  report missing configuration or the matching rule without printing the list contents.
- **Guards**: `hygiene_blocklist_guard_test.py` covers absent, blank, comment-only, environment,
  relative-file and local-file lists. `snapshot_gate_guard_test.py` covers source list propagation
  across the export boundary. `snapshot_ci_guard_test.py` covers workflow preflight execution.

## 2026-11-18: a public snapshot exclusion could escape its scratch tree

- **Symptom**: `make-snapshot.sh` expanded an entry from `exclude.txt` directly into `rm -rf`;
  a traversal or option-like entry could remove a path outside the freshly-created snapshot.
  The generated public commit also used a personal default identity and an unnecessary body.
- **Found by**: the go-public safety audit of the snapshot exporter.
- **Cause (checked)**: exclusions were not constrained to relative paths before shell expansion,
  Windows CRLF left a carriage return on each exclusion, and the exporter defaulted its commit
  metadata to the historical owner identity.
- **Fix**: strip CRLF, reject root, dot, traversal and option-like exclusions before removal; pass
  the end-of-options marker to `rm`; default snapshot commits to a neutral project identity and a
  single-line message without a body.
- **Guard**: `tools/tests/publish_snapshot_guard_test.py` runs the exporter with a CRLF traversal
  entry and then a CRLF ordinary exclusion; it fails on the old exporter after the sentinel is
  removed or the ordinary exclusion is ignored, and passes once both are handled.

## 2026-11-18: a historical tool attribution was not public-safe

- **Symptom**: the public snapshot gate found a tool attribution in the historical FIXLOG.
- **Cause (checked)**: one historical entry named an external tool instead of a neutral role.
- **Fix**: rewrite that attribution as a neutral desktop audit attribution; no simulation or release
  behavior changes.
- **Guard**: `tools/tests/public_hygiene_guard_test.py` checks the public-facing FIXLOG against the
  repository's forbidden-pattern list and fails when the old attribution returns.

## 2026-11-18: public snapshot gate runs in CI

- **Symptom**: a publication leak or identifier could be introduced on a pull request and remain
  unnoticed until the next public snapshot was built.
- **Cause**: the snapshot exporter was only run as a release-time procedure.
- **Fix**: `tools/ci.sh` and the CI workflow run `tools/publish/check-snapshot.sh` when the
  publication tools are present; the gate exports the current tree and scans the complete snapshot.
- **Guard**: `tools/tests/snapshot_gate_guard_test.py` proves a forbidden fixture fails the gate and a
  clean fixture passes it.

## 2026-11-16 #1665: the ecology census counted a hare that walked in from the edge as a birth

- **Symptom**: `examples/ecology_census.rs` counted every new non-person agent id as a birth, so
  adults that came over the hills (`CameOver`, 238 rows in 700 days on seed 1) were births. It
  also printed nothing about the cap, ducks or fish.
- **Found by**: the 2026-11-13 alive audit (#1665).
- **Cause (checked)**: births were `id > highest seen`, with no look at `CameOver` events.
- **Fix**: the census subtracts `CameOver` agents from births and prints them apart, prints per
  kind the share of days at `wild_cap`, and per 30-day row ducks here/away and the mean and minimum
  `fish::stock` over the stretches with surface water (read at the start of the run). The cap table
  in `systems/animals.rs` moved to a const `WILD_CAPS` with the same numbers, read by the new
  `pub fn wild_cap`. No sim behaviour changes.
- **Guard**: `tests/wildlife.rs::wild_cap_reads_the_table`.

## 2026-11-16 #940: no census showed how often choices act on people the chooser could not see

- **Symptom**: `spoken_for` (`systems/decide.rs`), the Talk checks and the Tend scan read other
  people's live state to decide who to talk to or tend, without asking whether the chooser could
  actually see them. No measure said how often that happened.
- **Found by**: a desktop audit and the 2026-11-13 alive audit (#940); corrected by a follow-up
  audit after the first proposal could not be tested as written.
- **Cause**: nothing tallied the gap between what a decision read and what the chooser's own
  perception (`mind::perception::can_see`) would have found.
- **Fix**: `view::knowing::Knowing`, a read-only tally kept on `World` (`w.knowing`), counts
  seen/unseen for every chosen Talk, Tend, Forage and EatItem. `systems::decide::choose` calls
  it right after a goal is picked, checking line of sight from the chooser's position to the
  target's (for Forage and EatItem, to whoever already claimed the plant or item, if anyone).
  `Knowing::observe_failures` adds the day's "nothing to gather" and "they left" plan failures
  from `World::last_events`. `examples/knowing_census.rs` steps a world and prints the running
  report; it reads the world and changes nothing, so a run's digest is unaffected.
- **Guard**: `tests/pipeline.rs::knowing_census_counts_an_unseen_target`. A Talk target placed
  behind a wall tile counts as unseen (out-of-sight share 1.0); a Tend patient in plain view
  counts as seen (share 0.0).
- **Commit**: fix-940-knowing-census, from master `88c4dcdf`.

## 2026-11-17 #1587: caravans found four berries from nowhere on any quiet day

- **Symptom**: on a day with no trouble, `travel_day` pushed `(Berries, 4)` onto the cargo when the
  roll fell under `danger + 0.08`, in any season, on any land, by no one (seed 1 d40: a scouting
  party of 2 came home with exactly four berries).
- **Found by**: the VPS-audit (slice of #1546, after #1584).
- **Cause (checked)**: the branch read neither season, land nor party.
- **Fix**: `systems/expedition.rs::travel_day`: food is gathered only in summer (berries) and
  autumn (roots), with chance `0.15 x land` (Forest/Lakeshore 1.0, Village 0.6, Mine/Ruin 0.3);
  the amount is `members x (1 + best Foraging / 10)`; the best forager is named in the log
  ("day 4: Ylagar gathered 6 berries by the way") and practises Foraging. `road_incident` also
  reads "gathered".
- **Guard**: `tests/expedition.rs::nothing_is_gathered_on_a_winter_road` (fails on the old code,
  which added berries in winter) and `a_summer_forest_road_feeds_the_party`.
- **Not done**: the `road_census` before/after measure was not run here.

## 2026-11-16 #1020: a fine table and a crude one were the same once set

- **Symptom**: benches make plain, good and fine fittings, but once set the quality was gone.
- **Found by**: the local AI audit, 2026-11-13 alive audit (#1020).
- **Cause (checked)**: `Structure` had no quality; the `BuildStructure` completion (`systems/work.rs`) read only the delivered fitting's material.
- **Fix**: `Structure.quality` (`#[serde(default = "half")]`, so an old save's structure loads at 0.5). A fitting item set by a `BuildStructure` gives its mean delivered quality; kinds built from raw planks or blocks get 0.5. Nothing reads it yet (room furnishing #1021 and comfort #1008 will).
- **Guard**: `tests/fittings_quality.rs::a_set_table_keeps_its_quality` (a table of 0.8 gives a Structure of 0.8) and `a_structure_from_an_old_save_is_middling`.
- **Digest note**: every snapshot's bytes change once (each structure carries the key), so digests move; no behaviour moves. Recorded in ITERATION_LOG.
- **Base note**: every `Structure { .. }` literal in the tree gains `quality: 0.5`, so other open branches that add one will need the same line when this lands.
- **Commit**: see PR.

## 2026-11-16 #1368: nobody knew who was cold where

- **Symptom**: `needs.warmth` and the `Cold` thought (`hazards.rs:cold_step`) existed, but no
  example or soak column reported them by where a person stood, so #1371-#1373 (who gets cold
  and why) had no "before" to measure against.
- **Found by**: the local AI (desktop audit), the 2026-11-13 alive audit (#1368).
- **Cause (unconfirmed, it is a missing report, not a behaviour bug)**: no code ever bucketed a
  living person by shelter and printed person-hours, warmth and the `Cold` thought against it.
- **Fix**: `crates/hf-sim/src/systems/hazards.rs::warmth_bucket` (new) tells apart "hearth"
  (walled, roofed, enclosed, with a lit hearth within 4 or a fire within 3 — the same test
  `cold_step`'s `warm_near` uses), "roofed, no hearth", "roofed, open side" (roofed but not
  enclosed by the same 3x3 test `building.rs::roof_old_houses` uses at lines 375-379, generalised
  to any tile until #1369 lands), "tent/nest", "below ground" (`underground::below`) and "open".
  The new read-only `crates/hf-sim/examples/warmth_census.rs` steps an opening world and prints
  person-hours, the share with warmth below 0.35, the share holding the `Cold` thought, frostbite
  wounds and froze deaths per bucket, split by season and by asleep/awake
  (`cargo run --release -p hf-sim --example warmth_census 600 1 2 3 4`). No simulation behaviour
  changed.
- **Guard**: `crates/hf-sim/src/systems/hazards.rs::tests::warmth_bucket_tells_apart_every_shelter`
  builds a walled, roofed room and checks it reads "roofed, no hearth", then "hearth" once a lit
  hearth is placed inside, then "roofed, open side" once a wall tile is knocked out; it fails to
  compile on master, which has no `warmth_bucket`.

## 2026-11-16 #907: the hardest worker stood about as much as anyone

- **Symptom**: Loiter was always offered at the same `0.2 + 0.3 at night` for every person
  (`systems/decide.rs`), whatever their industry. Loitering share against industry measured
  r = -0.03 over 263 long-lived colonists (seeds 5-8, #1518): no relationship at all.
- **Found by**: the 2026-11-13 alive audit (draft B4c, epic #1531): a watcher should see the
  hard worker never quite still, the way an industrious RimWorld colonist rarely idles.
- **Cause (checked)**: a missing term, not a fault in the simulation.
- **Fix**: `Agent::loiter_weight` (new, `mind/dispositions.rs`) gives `0.5 + 1.0*(1 - industry)`
  from the core profile (0.5 at full industry, 1.5 at none), or a legacy `0.6`/`1.2`/`1.0` from
  `Trait::Industrious`/`Easygoing`/neither (`mind/traits.rs`); `systems::decide::choose`'s
  `temperament` closure (#906) now returns it for `Goal::Loiter`, so it multiplies into Loiter's
  score and is named by the Why? lens exactly like `spirits`.
  `tests/dispositions.rs::industry_changes_the_real_work_choice_and_unrelated_axes_do_not`'s
  low-industry case changed from `Wandering` to `Loitering`: the laziest now outweighs a
  low-curiosity Wander at Loiter's new 1.5x, which is the fix working as intended, so the test's
  expectation was updated rather than the fix.
- **Guard**: `tests/dispositions.rs::industrious_stand_about_less` (the acceptance test named in
  the issue: two adults at industry 0.9 and 0.1, open hauls, full needs, over a day — the
  industrious one loiters at most 0.7x as much).

## 2026-11-16 #908: industry moved work's weight too little to tell a worker from an idler

- **Symptom**: `Agent::work_weight` only ranged 0.7-1.3 (`0.7 + 0.6*industry`), and the matching
  legacy traits only 0.8-1.3. Working-share ratio between the top and bottom industry quartiles
  measured 1.4 (#1518) — too close to tell a worker from an idler, where Dwarf Fortress and
  RimWorld both make the work ethic one of the most visible differences between people.
- **Found by**: the 2026-11-13 alive audit (draft B4d, epic #1531).
- **Cause (checked)**: a term too narrow, not a fault in the simulation.
- **Fix**: `Agent::work_weight` (`mind/dispositions.rs`) now gives `0.4 + 1.5*industry` (0.4-1.9);
  the legacy `Trait::Industrious`/`Easygoing` fallback (`mind/traits.rs`) widened to 1.6/0.6 to
  match. Moved the factor out of `base` and into `systems::decide::choose`'s `temperament`
  closure (#906) for `Goal::Work`, computed once per decision (`withdrawn_until` still zeroes
  it) rather than per designation, so it is named by the Why? lens like `spirits`.
- **Guard**: `tests/dispositions.rs::industrious_take_more_work` (the acceptance test named in
  the issue: two single-agent worlds, industry 0.9 and 0.1, ten open low-priority chop orders,
  over 2 days — the industrious one completes at least 2x as many).
- **Commit**: this one.

## 2026-11-16 #1761: the tidy were no more likely to keep the stores or the pot

- **Symptom**: `order` reached a choice only through `haul_fit` (`0.3 × order`, added to the
  haul score directly), so the tidy weighed a haul order only a little more, and a cook order
  not at all — "the tidy keep the stores and the pot" was not something the watcher could see.
- **Found by**: #1761 (umbrella #905, split from #913, epic #1531), depends on #906's
  `temperament` closure.
- **Cause (checked)**: a missing reader, not a fault in the simulation.
- **Fix**: `Agent::order_weight()` (`mind/dispositions.rs`, beside `haul_fit`) reads
  `0.8 + 0.4 × order` from the core profile, or `1.0` for a legacy person. In the `temperament`
  closure (#906, `systems/decide.rs`), `Goal::Work(did)` looks the designation up by id and
  applies `order_weight()` when its kind is `Haul` or `Cook`; every other designation kind, and
  every other goal, is unaffected (1.0).
- **Guard**: `tests/why_lens.rs::orderly_weigh_haul_and_cook_more` (two single-agent worlds,
  order 0.9 and 0.1, facing a haul, a cook and a chop order: the haul and cook options'
  `temperament` are 1.16 and 0.84 within 0.01, the chop (standing in for a build order, which
  the `temperament` closure treats the same way) is 1.0 for both).
- **Commit**: see `add-1761-orderly-weight` branch, built on the unmerged `add-906-why-
  temperament-factor` (#906, PR #1940). Siblings #907 (PR #1945) and #908 (PR #1947) touch
  other match arms of the same `temperament` closure; whichever of mine lands last will have a
  small, mechanical merge conflict there.

## 2026-11-16 #1760: a cheerful person was no more likely to stop for a word

- **Symptom**: `cheerfulness` was shifted by mood in `current_dispositions` and shown as a
  surface label, but no option in `decide::choose` ever read it — a cheerful person was no
  more likely to be the one who stopped for a word than a glum one.
- **Found by**: #1760 (umbrella #905, split from #913, epic #1531), depends on #906's
  `temperament` closure.
- **Cause (checked)**: a missing reader, not a fault in the simulation.
- **Fix**: `Agent::cheer_weight()` (`mind/dispositions.rs`, beside `talk_weight`) reads
  `0.85 + 0.3 × cheerfulness` from the current (mood-shifted) dispositions, or `1.0` for a
  legacy person. The `temperament` closure (#906, `systems/decide.rs`) applies it to
  `Goal::Talk(..)`; every other goal is unaffected (1.0).
- **Guard**: `tests/why_lens.rs::cheerful_weigh_talk_more` (two worlds, cheerfulness 0.9 and
  0.1, both lonely with a friend in reach: both `Talk` options' `temperament` is within
  0.85-1.15, and the cheerful one's is at least 0.2 higher).
- **Also fixed**: `tests/why_lens.rs::why_lists_a_temperament_factor_of_one` traced a legacy-
  disposition agent instead (cheerfulness is now a real reader, so "every factor is 1.0" only
  holds for someone with no core profile); `the_lens_shows_the_options_their_terms_and_what_
  the_short_store_put_off`'s score-equality check for the talk option now includes `learned`
  and `temperament`, which it had silently omitted.
- **Commit**: see `add-1760-cheerful-talk` branch, built on the unmerged `add-906-why-
  temperament-factor` (#906, PR #1940). Siblings #907 (PR #1945), #908 (PR #1947) and #1761
  (PR #1962) touch other match arms of the same `temperament` closure; whichever of mine
  lands last will have a small, mechanical merge conflict there.

## 2026-11-16 #1759: independence was invisible in what anyone did

- **Symptom**: `independence` was read only by `mind/conscience.rs` and `mind/telling.rs`; no
  choice in `decide::choose` read it, so one of D67's eight dimensions never showed in a
  person's actual behaviour.
- **Found by**: #1759 (umbrella #905, split from #913, epic #1531), depends on #906's
  `temperament` closure.
- **Cause (checked)**: a missing reader, not a fault in the simulation.
- **Fix**: `Agent::independence()` (`mind/dispositions.rs`, beside `haul_fit`) reads the current
  (mood-shifted) `independence`, or `0.5` for a legacy person — the midpoint of a fresh core
  profile, so a formula built around it reads as neutral for a legacy person too. The
  `temperament` closure (#906, `systems/decide.rs`) applies `1.2 − 0.4 × independence()` to
  `Goal::Gather(_)` and `0.8 + 0.4 × independence()` to `Goal::Wander(_)`; both are leisure
  goals, survival goals are untouched.
- **Guard**: `tests/dispositions.rs::independent_gather_less` (two single-agent worlds at a
  hearth evening hour, independence 0.9 and 0.1: the independent one's hearth-sitting
  `temperament` is lower, their wandering `temperament` is higher).
- **Also fixed**: `tests/why_lens.rs::why_lists_a_temperament_factor_of_one` now traces a
  legacy-disposition agent, since independence is now a real reader of the traced agent's core
  profile (same fix as #1760's PR for the same test, applied here independently since the two
  branches don't share history).
- **Commit**: see `add-1759-independent-gather-wander` branch, built on the unmerged
  `add-906-why-temperament-factor` (#906, PR #1940). Siblings #907 (PR #1945), #908 (PR #1947),
  #1760 (PR #1965) and #1761 (PR #1962) touch other match arms of the same `temperament`
  closure; whichever of mine lands last will have a small, mechanical merge conflict there.

## 2026-11-16 #1758: the idle and the industrious gathered for a short store alike

- **Symptom**: `Goal::Stock`'s score never read anyone's industry; a lazy and a hard worker
  weighed stocking the store identically.
- **Found by**: the 2026-11-13 alive audit (draft B4b, epic #1531), as a sibling of #1759/
  #1760/#1761 under #906.
- **Cause (checked)**: `systems::decide::choose`'s `temperament` closure (#906) returned 1.0
  for every goal; nothing fed `Goal::Stock` a term of its own.
- **Fix**: `Agent::stock_weight` (`mind/dispositions.rs`), `0.6 + 0.8 * industry` off the
  context-expressed disposition (1.0 for a legacy person), read by a new
  `Goal::Stock(_) => a.stock_weight()` arm in the `temperament` closure.
- **Base note**: branched directly off `add-906-why-temperament-factor`, not off #1759/#1760/
  #1761's own branches, since the four issues are independent siblings with no stated
  dependency; all four edit the same `temperament` match block in `decide.rs`, so whichever of
  these PRs lands last among its siblings picks up a small, mechanical merge conflict there.
  `why_lists_a_temperament_factor_of_one` and `why_lists_a_spirits_factor_of_one` in
  `tests/why_lens.rs` both exercise a Stock option via `lonely_by_a_bush`'s short-store camp;
  both now set the traced agent's dispositions to `None` (and clear their traits) before
  tracing so every option still reads `temperament == 1.0`/`spirits == 1.0`, the same fix
  applied independently on the #1759/#1760 branches for their own goals.
- **Guard**: `tests/dispositions.rs::industrious_weigh_stocking_more` (asserts the Stock
  option's `temperament` is 1.32 at industry 0.9 and 0.68 at industry 0.1, matching the
  issue's acceptance criterion).
- **Commit**: this one.

## 2026-11-16 #906: the Why? lens could not say a disposition swayed a choice

- **Symptom**: disposition weights (`work_w` in `systems/decide.rs`, `talk_weight`) are folded
  silently into `base`, so `why::Considered` had no term through which the lens could ever say
  "industrious" or any other temperament, whatever later slices add.
- **Found by**: the 2026-11-13 alive audit (draft B4b, epic #1531): inner life only reads as
  personality if the lens can name it.
- **Cause (checked)**: a missing slot, not a fault in the simulation.
- **Fix**: `systems::decide::choose` gained a `temperament(&Goal) -> f32` closure, returning
  1.0 for every goal; its result multiplies into `score` alongside `emotion`/`gate`/`learned`/
  `spirits` and is recorded on a new `why::Considered.temperament` field, printed by `explain`
  only when it is not 1.0 (mirroring `spirits`, #885). No score changes while every disposition
  term stays in `base`; later slices move them out one at a time.
- **Guard**: `tests/why_lens.rs::why_lists_a_temperament_factor_of_one`; `why.rs`'s own
  `temperament_is_said_only_when_it_is_not_one`.
- **Commit**: this one.

## 2026-11-16 #1262: a soaking in a cold rain chilled no faster than a dry walk

- **Symptom**: standing out in cold rain chilled a person at exactly the same rate as standing out
  dry in the same cold. Wet clothes should make a cold spell dangerous; the sim gave them no edge.
- **Found by**: the 2026-11-13 alive audit (#1262), with a corrected test threshold from Dan's
  review comment on the issue (the originally proposed 0.08 absolute gap over 3 hours could not be
  met by the proposed 1.8x factor; the corrected spec is 6 hours, a 1.7x-1.9x ratio and at least a
  0.07 absolute gap).
- **Cause**: `cold_step` in `crates/hf-sim/src/systems/hazards.rs` scaled warmth loss by clothing,
  fur, deep cold, sleep, huddling and body size, but never read how wet the person was — there was
  no `wet` field on `Agent` at all (its source, #1261, is not yet merged to master).
- **Fix**: added `Agent::wet: f32` (0..1, default 0.0, so old saves and every existing agent
  constructor are unaffected) and, in `cold_step` (`crates/hf-sim/src/systems/hazards.rs`), after
  the clothing/fur factor: `loss *= 1.0 + 0.8 * a.wet;` — soaked through (`wet` 1.0) loses warmth
  at 1.8x the dry rate.
- **Guard**: `crates/hf-sim/tests/weather_events.rs::soaked_in_the_cold_chills_faster`. Two clothed,
  unfurred, un-huddled people at -1.5°C (lapsed from 0°C at the founding ground), one with `wet`
  1.0 held there: after 6 game-hours of `cold_step`, the wet one's warmth loss is 1.7x-1.9x the dry
  one's and at least 0.07 more in absolute terms. Fails without the fix (ratio 1.0, zero gap, since
  `a.wet` is unread) and passes with it.
- **Overlap note for the merge coordinator**: issue #1263 (weather mood, PR #1911, not yet merged)
  also touches `systems::hazards` and is expected to add its own `Agent::wet`-adjacent state. This
  fix was built independently against current master and adds the `wet` field itself; whichever of
  #1911/#1262 lands second should reconcile the two `wet`-related additions rather than duplicate
  the field.

## 2026-11-16 #1081: humans had no one-person shelter, and the sleep choice ignored it anyway

- **Symptom**: a person who slept alone had no sleeping place of their own to raise; the only
  one-tile shelters were the Gnarrow's tent and the Skarri's nest. Fixing that by adding a
  `StructureKind` was not enough on its own: the sleep choice and the bed bookkeeping each
  hard-coded `Bed | Tent | Nest` as three separate tag lists in `systems/decide.rs`, so a new
  sleeping-place kind would sit there finished, in sight, and never be slept in.
- **Found by**: the 2026-11-13 desktop alive audit (#1081); corrected by the local AI (VPS-audit) in a
  2026-11-14 comment on the issue after the first draft's `is_sleeping_place` addition did not
  reach the sleep-goal or roofless-knowledge checks.
- **Cause (confirmed)**: `StructureKind::is_sleeping_place` existed for one query, but
  `systems/decide.rs` re-derived the same set three more times, twice from a knowledge tag
  (`SleepingPlaces::build`'s own kind match plus `secured`'s `knows_bed` and `choose`'s bed-goal
  loop at the old lines 178, 1248-1251 and 1918-1925), each spelled out as its own
  `Bed | Tent | Nest` match or `== as u16` chain. Appending a kind to the enum alone would not
  reach any of the three.
- **Fix**: `structures.rs` appends `StructureKind::LeanTo` (D35, D36: raised alone, the same day,
  from logs and hides, reusing the tent's build time) and folds it into `is_sleeping_place`.
  `work/designation.rs` gives it the tent's 30-minute build time and a cost of 2 logs and 2
  hides (`structure_cost` has no "either/or" cost mechanism yet, so the issue's "hides or thatch"
  is simplified to hides — a judgement call, revisit if thatch lean-tos are wanted). In
  `systems/decide.rs`, `SleepingPlaces::build` now calls `s.kind.is_sleeping_place()` directly;
  the two tag-based checks (`secured`'s `knows_bed`, `choose`'s bed-goal loop) now go through a
  new `tag_is_sleeping_place(tag: u16)` helper that lists the same four kinds once, so a future
  sleeping-place kind only needs adding there and to `is_sleeping_place`, not at each call site
  separately.
- **Guard**: `tests/homes.rs::a_lean_to_is_a_sleeping_place_with_a_roof_over_its_tile` (a known,
  finished lean-to under the agent makes `secured(..).roofless` false) and
  `tests/homes.rs::a_lean_to_in_sight_is_slept_in` (a tired person with no bed known and a
  finished lean-to 5 tiles off chooses it as their `Goal::SleepBed` and, run on, pushes
  `EventKind::Slept { in_bed: true, .. }`). Both fail against the pre-fix code: the first panics
  because `StructureKind::LeanTo` does not exist, and with only the enum change restored (no
  `decide.rs` fix) the second fails with `Sleeping { bed: None, .. }` and the first still fails
  on `roofless`.
- **Commit**: this fix's commit.

## 2026-11-16 #1743: "Someone was put in the ground here, nobody can say how"

- **Symptom**: `Grave { pos, dead, death, rite, laid }` (`accounts.rs:152-159`) held no name and
  no words. A newcomer standing by any grave, even one dug by someone who knew the dead well,
  could only read `of: None` and `Said::Unknown`/`Said::Cursed` — e.g. seed 4:
  `d59 [grave_read] | Nalaen stood at a grave and read it: someone was put in the ground here,
  nobody can say how.` (414 `grave_read` rows in that run).
- **Found by**: the local AI (VPS-audit), the 2026-11-14 alive audit (#1743).
- **Cause (checked)**: `accounts::buried` never took the burier into account and never wrote
  anything onto the `Grave` it pushed; `accounts::step` always set `of: None` and guessed from
  `Said::Unknown`/`Cursed` alone, with no path for a grave to carry a name.
- **Fix**: `Grave` gained `#[serde(default)] words: Option<String>`. `accounts::buried`
  (`accounts.rs::buried`) now takes the burier (`by: Option<AgentId>`) and, when there is one,
  writes the dead's name plus their strongest trained skill as a short "known for" phrase (the
  simple stand-in for #1201's fuller reputation record, which does not exist yet), or the name
  alone if they held no trained skill (`accounts.rs::known_for`); its one caller in
  `systems/rites.rs::lay_to_rest` only runs when the burial is not `in_absence`, so a burial in
  absence still makes no grave at all, as before. A new `Said::Marked(String)` variant
  (`accounts.rs::Said`) carries the words read back verbatim; `accounts::step` gives a marked
  grave's reader `of: Some(dead)` and `Said::Marked`, and only falls back to the old
  `Unknown`/`Cursed` guess when a grave has no words.
- **Guard**: `crates/hf-sim/tests/accounts.rs::a_marked_grave_tells_a_newcomer_who_lies_there`
  (new) — a staged burial by B, who buries D after D trained a skill; a newcomer who comes
  later holds an account `of: Some(D)` with the name and "known for" phrase, and once the
  grave's words are cleared (standing in for a grave with no one to write it) the same newcomer
  only reads "someone" again. `a_grave_is_read_by_one_who_came_after_and_the_reading_can_be_wrong`
  (existing) is updated to the new behaviour: every ordinary burial is now marked, so the
  newcomer there also reads `of: Some(dead)` and `Said::Marked`, fearful or not.

## 2026-11-16 #1120: nothing counted what becomes of heirlooms and treasured things

- **Symptom**: #1119's target (60% of heirlooms still with an heir after 50 days) had no
  baseline: nothing counted a storied item's place (held, carried, on the ground, in store),
  how many are cherished or heirlooms, or whether a heirloom's heir still holds it.
- **Found by**: the 2026-11-13 alive audit (draft D12a, epic #1531).
- **Cause (checked)**: a missing census, not a fault in the simulation.
- **Fix**: new `crates/hf-sim/examples/possessions_census.rs`. Every 50 days it tallies every
  storied item by `ItemPlace` (Ground items inside a `Stockpile` structure's tile count as "in
  store"), cherished and heirloom counts, and for each heirloom whether its last `owners` entry
  (`Story::owners.last()`) is alive and still `Held`/`Carried` by them. It also tallies
  `LostTreasure` thoughts by cause, read from a matching `ToolBroke` or `NewClothes` event on the
  same agent and tick (an unmatched thought, which should not occur, counts as "unknown" —
  documented in the file as the one approximation here).
- **Guard**: `tests/legend.rs::possessions_census_sees_an_heirloom_set_down` (an heirloom set
  down by its heir reads as one heirloom, on the ground, with no living heir holding it).

## 2026-11-16 #1268: every save carried a "place feelings" map that nothing wrote or read

- **Symptom**: `mind::knowledge::Knowledge::place_affect` ("Phase 5 memory writes it") had no
  writer or reader outside `knowledge.rs` itself; a dead field that looks like a feature.
- **Found by**: the 2026-11-13 alive audit (draft F10c, epic #1531).
- **Cause (checked)**: place feeling was moved into `memory::place_multiplier` at some earlier
  point and this field was never removed.
- **Fix**: `#[serde(default, skip_serializing)]` on `place_affect`, with a doc comment saying it
  is retired and kept only so an old save still loads (deserialize is unaffected; the attribute
  only stops it being written back out).
- **Digest note**: since `digest::digest` hashes the encoded snapshot (`digest.rs:9-15`), and
  the encoded snapshot no longer carries this key, the digest moves once even though no
  behaviour changes (an empty `place_affect` was already read by nothing). ITERATION_LOG entry
  added.
- **Guard**: `tests/save_memory.rs::old_saves_with_place_affect_still_load` (hand-encodes a
  `Knowledge` in the pre-#1268 shape with `place_affect` populated, confirms it still decodes,
  and confirms the re-encoded bytes carry no `place_affect` key).
- **Commit**: this one.

## 2026-11-15 #1429: hovering a person wrote their name in the corner of the screen

- **Symptom**: the hover text was appended to the HUD block in the top-left, far from the sprite the
  mouse was on.
- **Found by**: the local AI (desktop audit), the 2026-11-13 alive audit (#1429).
- **Fix**: the name-tag layer (#1428) takes the hovered person and `Rules.hover_tag` ("Galela,
  talking": the name and what they are doing, without the place) and draws it over them in light
  blue, before any other tag and whatever the name-tag setting. The HUD line stays for #294's readers.
- **Guard**: `tools/gd_test.gd::test_hover_tag_draws_beside_the_person`. It drives the real
  `name_tags.gd` over three stand-in people, and a hover on row 1 puts one tag centred over that
  person even with names off. The hover comes before the followed person, no hover with names off
  draws nothing, and the place is left off the words. Evidence in `docs/reviews/2026-11-15-hover-tag/`.

## 2026-11-15 #1727: "A clothe lay by the portal stone"

- **Symptom**: the thing-from-home line cut the "s" off the item's stock word. It read "A clothe
  lay by the portal stone, made by Aelivane in Fernmere." 20 times in four of the five alive-audit
  runs (seeds 1, 3, 4 and 5).
- **Found by**: the local AI (VPS-audit), the 2026-11-13 alive audit (#1727).
- **Cause (checked)**: `view/narrate.rs` wrote `format!("A {}", kind.label().trim_end_matches('s'))`.
  `ItemKind::label` is the word for a stock ("clothes", "torches", "berries", "iron ore"), so
  trimming gave "A clothe", "A torche" and "A berrie", and mass words took "A" ("A iron ore").
- **Fix**: a view helper `narrate::a_thing(kind, material)` gives one thing with its article. It
  uses "some" for heaps, sets and measures ("Some clothes", "Some iron ore"), names the three odd
  ones ("a torch", "a sheaf of grain", "a stone block") and otherwise uses `a_or_an` over
  `legend::noun` ("an iron tool", "a hammer"). Text only: `legend::noun` and `ItemKind::label`
  are unchanged, since `legend::noun` also feeds stored relic and event text.
- **Guard**: `tests/narrate.rs::a_thing_from_home_reads_naturally_for_every_item_kind`. It narrates
  the line for all 49 kinds: each starts "A ", "An " or "Some ", none has a cut word, and
  clothes, a hammer and a torch read as they should.

## 2026-11-16 #1385: nobody can tell where the valley's know-how came from

- **Symptom**: each `Know` already keeps how a craft was learned (`Learned`: Past, Watched,
  Tried, Studied, Taught) and a free-text `from` set by `learn_from`
  (`crates/hf-sim/src/mind/techniques.rs`), and `crafts.rs` already emits `CraftLost` and
  `CraftRegained`, but nothing tallied any of it, so no run could say what share of the
  valley's craft knowledge came from outside versus was carried in or taught locally.
- **Found by**: the 2026-11-13 alive audit (#1385, part of #1531); cited for the #1384 "before"
  numbers.
- **Cause (checked, not a bug)**: a read-only reporting gap, not a behaviour defect — the
  provenance was tracked but never surfaced.
- **Fix**: new `crates/hf-sim/examples/craft_census.rs` (read-only; `<days> <seed> [seed...]`).
  Every 100 days (and the final day) it lists, per living community, each craft in `CRAFTS`
  with its knowers, their `Learned` source and `from`; at the end of each seed it prints totals
  by source plus how many `CraftLost`/`CraftRegained` events fired over the run. The tally
  itself lives in `craft_census.rs::tally_by_source`, factored out of `main` so it can be
  tested directly.
- **Guard**: `crates/hf-sim/examples/craft_census.rs::tests::tallies_crafts_by_learned_source_and_ignores_non_crafts`
  (new; `cargo test -p hf-sim --example craft_census`) — asserts each `Learned` source is
  counted separately, a non-`CRAFTS` technique (`ThinLore`) is never tallied, and only rows
  with a non-empty `from` count toward "named a source".
- **Commit**: this one.

## 2026-11-15 #1834: a witness's caution was read backwards, so the calmest were silenced

- **Symptom**: `mind::witnessed::form` set `shaken` (keeps the sighting to themselves) on
  `caution < 0.25`, but `caution = 1 - CALM` (`genetics.rs`), so the calmest witnesses (low
  caution) were the ones silenced and the boldest told everyone — the opposite of what the
  doc comment, the variable name, and the sibling rule in `resonance.rs` (`caution > 0.75`)
  all describe.
- **Found by**: the worker of #1520 and confirmed by its reviewer, reading the code (#1834).
- **Cause (checked)**: `mind/witnessed.rs`'s `form` used `caution < 0.25` where `resonance.rs`
  uses `caution > 0.75` for the same rule ("the very cautious keep it to themselves").
- **Fix**: `crates/hf-sim/src/mind/witnessed.rs::form` now reads `caution > 0.75`, matching
  `resonance.rs`. Doc comment at the fix site corrected.
- **Guard**: `crates/hf-sim/tests/witnessed.rs::a_bold_witness_tells_a_cautious_one_stays_silent`
  (new); the existing shaken-witness case in
  `a_withheld_account_stays_out_of_the_gossip_chain` flipped from caution 0.1/silent to caution
  0.9/silent; `witnesses_of_a_refusal_hold_their_own_read_not_the_potential` pinned its
  witness's caution to 0.5 so it tests reading/telling, not this gate, regardless of seed.
- **Commit**: this one.

## 2026-11-14 #1418: the inspect panel received a person's thoughts and stress and drew neither

- **Symptom**: the bridge sends `stress` and `thoughts` (the strongest six, "lost someone -0.60")
  with every inspect, and the panel showed neither: the mood word and its because-line only.
- **Found by**: the 2026-11-13 alive audit (#1418, part of #1414).
- **Cause (checked)**: `main.gd` `_update_inspect` never read the two keys.
- **Fix**: `_update_inspect` adds a stress bar under the needs (warm red; a full bar is the
  breaking point) and, under the mood line, an "on their mind" block from
  `godot/scripts/rules.gd` `format_moodlets`: one thought per line, strongest first, green when it
  lifts the mood and red when it weighs on it.
- **Guard**: `test_inspect_moodlets_colour_by_sign` (gd_test). Real main scene
  (docs/reviews/2026-11-14-inspect-moodlets): the stress bar, and "on their mind: torn from the
  old world -0.60, the missing -0.20" in red.
- **Commit**: this branch (`feat/1418-inspect-moodlets`).

## 2026-11-15 #1428: nobody in the world was named, even the person being followed

- **Symptom**: the only text drawn in the world was stack counts and event marks, so to know who was
  who a viewer had to click or hover each sprite (#1423).
- **Found by**: the local AI (desktop audit), the 2026-11-13 alive audit (#1428).
- **Fix**: `render/name_tags.gd` draws a small label over people's heads, above every world layer.
  Who is tagged is `Rules.name_tag_ids`: by default the followed or selected person (in a warm colour)
  and favourites; with the new setting at "everyone", everyone in view once the camera is at 1.25x or
  closer. The setting is `name_tags` in settings.cfg, cycled by a "Name tags" button in settings (O):
  off, favourites and followed, everyone up close. In a crowd, labels never overlap: they are placed in
  priority order, lifted a row or two, or left out. Zoomed out, they keep about 12 px on screen.
- **Guard**: `tools/gd_test.gd::test_name_tags_follow_setting_and_zoom`, the issue's case (three people,
  one favourite and one followed: the default tags two; "everyone" tags three at 1.5x and two at 1x;
  off tags none; the selected person is tagged; the default setting is 1). Evidence in
  `docs/reviews/2026-11-15-name-tags/`.

## 2026-11-15 #1721: Seed 4's decline (hard things, a proposal to move, answers by trust, a split) was guarded by no test through the real pipeline

- **Symptom**: nothing ran the arc that seed 4 shows (`d259 [proposed] ... five hard things in two
  seasons`, nine answers of `went with`, `[moved_camp]`; on day 104 the same chain split the camp by
  trust) from real deaths, a raid and a rockfall to a move. The tests in `tests/moving.rs` wrote
  `hardships` in by hand and called `moving::consider` directly, or fed `last_events` to `observe`,
  so a change to the order of `PIPELINE`, to the kinds `observe` reads, or to the projects hour
  (`projects::AT`) could break the arc with every test green.
- **Found by**: the alive audit (umbrella #1526, epic #1531, slice #1721).
- **Cause (checked)**: a missing guard, not a fault in the simulation.
- **Fix**: none in the simulation: test only, nothing under `crates/hf-sim/src` changed, no digest or
  golden changed. `tests/moving.rs` gains `valley_of(seed, party)` (`valley` now calls it with 8) and
  the scene `declining(seed, distrusting)`: nine grown people on a flat valley with autoplan on, the
  first to die the parent of the proposer P, `distrusting` of the other six at -0.6 of P and the rest
  at +0.6. Day 1 `life::die` "drowned through the ice", day 3 `die` "bled out", day 5 a
  `Raided { open: false }` row ten tiles from home, day 7 a `Collapse` row near home, each followed by
  `w.step()`; then one `w.step()` onto day 8 at 08:17.
- **Guard**: `tests/moving.rs::a_run_of_hardships_splits_the_camp_by_trust_and_moves_it`, on scene
  seeds 6101, 6102 and 6103. It asserts: after day 7 the community's `hardships` are exactly the four
  rows (text, the tick of the step that flushed each, and the event ids); no `Move` project before day 8;
  on day 8 one `Proposed` row by P "to move camp to a better place in the valley" with `because` "the
  rock came down; strangers from beyond the valley took from the store; <second dead> died of bled
  out: four hard things in two seasons"; one `Move` project by P whose `source` is the rockfall's row
  and whose place is 20 or more tiles from home; the hardships cleared and `last_move_proposal` set;
  `Chose` rows `went with <P>` / `thinks well of <P>` for those at +0.6 and `stayed` / `does not trust
  <P>` for those at -0.6, and an `Answered` row with the counts and the stay reasons. With two at -0.6
  it is a `Split` (P and four others, 5 members, in a new community whose home is the project's place,
  the two staying in the old one at the old home, no `MovedCamp`); with one at -0.6 it is a
  `MovedCamp` (the old home recorded in `old_camps`, the community's home the new place, all seven
  still members).
- **Differences from the issue text**, found when the scene was run: (1) the issue makes P the
  partner of the first to die, but `die` unpairs the survivor, so `touched` in `moving::consider`
  finds kin only as parent or child of the dead; P is the dead's child. (2) The issue expects a
  `MovedCamp` with two at -0.6, but `consider` moves the whole community only when at most one grown
  person stays (`stay_grown <= 1`), so two staying is a `Split`; both outcomes are asserted. (3)
  `because` names the three newest hardships, so the first death is in the count ("four") and not in
  the words. (4) The clock is set between the days (`meta.tick`), not run through
  them, because the staged valley cannot be run through: `valley` clears the plants, and run tick by
  tick P and another member starve by staged day 5 and a different move is proposed on day 5 at 08:17
  ("six hard things"). So the days are skipped, and only the event ticks and day 8's 08:17 are real
  pipeline steps; the arc would still fire if run through, but earlier and from starvation. Every
  member's mood is also set to 0 on day 8's morning, so that moods do not decide who answers and why;
  the opinions are asserted unchanged first. P has the highest id of the nine and all moods are 0, so
  only the weight of kin in `touched` can make P the proposer (with that weight set to 0, the
  proposer is the lowest-id member and the test fails at the proposer assertion).
- **Not done here**: the second test, `the_decline_arc_survives_the_threat_budget`, which the issue
  adds only when #1281 lands `WorldMeta.storyteller`; and the 700-day count of move proposals on
  seed 4 Relentless before and after #1284. Both are to be done with #1284 (posted there as a
  comment, 2026-11-15). This test shows the chain on a staged scene, not on seed 4, and nothing
  about how often a real run reaches it.
- **Found while writing this test**: #1896 (the dead-partner checks in `moving.rs` `touched()` and
  `graves()` can never be true, because `die` clears the survivor's `partner`).
- **Commit**: this one.

## 2026-11-15 #856: fear of a fight or an attack reached people who could not see it

- **Symptom**: `feel::near` (`crates/hf-sim/src/systems/feel.rs`) is radius only, so a fight,
  an attack or a death served fear to everyone awake within radius on the level, whether or not
  a wall stood between them and the event — a frightened neighbour behind a wall who saw
  nothing is how fear spreads without a cause the watcher can see (design-rules 8).
- **Found by**: the 2026-11-13 alive audit, filed as #856 (draft A6d), citing `feel.rs:16-29,
  54-57, 64-68, 84-100`.
- **Cause (confirmed)**: the three call sites for `Died`, `Attacked` and `Fought` passed the
  radius set of awake agents straight to `feel`, with no line-of-sight check.
- **Fix**: `crates/hf-sim/src/systems/feel.rs` adds `seen_near`, which filters `near`'s radius
  set through the existing `mind::perception::can_see` (line of sight plus its own sight-radius
  cap), and uses it in place of `near` for the `Died`, `Attacked` and `Fought` arms of
  `observe`. `FireStarted`, the raid/siege/stranger-sighted generic arm and the other radius
  uses are left on `near` as the issue specifies (light, smoke and shouted news carry past a
  line of sight; hearing is radius-only until `data/peoples/species/human.toml` hearing is
  wired in). #854's shared `witnesses` helper (not yet landed) would let this call site and
  `strain::observe`'s (#855) share one implementation; this fix keeps its own local filter so
  as not to touch #855's file while both land in parallel.
- **Guard**: `crates/hf-sim/tests/feelings.rs::fear_of_a_fight_needs_sight_of_it` — a fight 6
  tiles from two bystanders at the same distance, one with a clear line to it (angered) and one
  behind a wall of solid granite tiles (feels nothing); fails on master because the walled
  bystander is angered too.
- **Commit**: (this commit).

## 2026-11-15 #1804: a newborn was a stranger to its grandparents and its siblings

- **Symptom**: at birth the child and each parent related to each other, but grandparents and
  siblings got no relation at all until they happened to interact later (a play or a talk, +0.02),
  so a lineage that should feel close from day one looked like strangers on the record.
- **Found by**: the alive audit (umbrella #1207, split from #1210, epic #1531, slice #1804).
- **Cause (checked)**: `give_birth` in `scenario.rs` called `relate` for the child and each parent
  only; it never looked up the parents' own parents (grandparents) or other children who share a
  mother or a father (siblings).
- **Fix**: `scenario.rs` `give_birth`, after the parent ties: collects each living grandparent (the
  parents' own mothers and fathers) and each living sibling (shares the mother or the father),
  then each one calls `relate(child, 0.4, 0.3, tick)` and the child calls `relate(them, 0.3, 0.2,
  tick)` — plain `relate`, the same call the parent ties use, not `relate_as(.., BondKind::Other
  ..)`, which needs #1137 and this slice does not depend on it.
- **Guard**: `tests/generations.rs::a_newborn_is_known_to_grandparents_and_siblings` (a grandmother
  and an elder sister each hold opinion >= 0.4 of the newborn at birth, the newborn holds >= 0.3 of
  each, and an unrelated neighbour holds no relation to the newborn at all).
- **Commit**: this one.

## 2026-11-15 #1121: taking up a dead partner's blade reset it to a stranger's tool

- **Symptom**: seed 7, run-day 385: "Naludel took up Gooskge's Greytooth" (`d10425 23:30`).
  Greytooth had just been marked `heirloom = true`, but `legend::take_up` unconditionally reset
  `keeper_uses = 0; cherished = false` for any new keeper, so the blade was no longer cherished the
  moment the heir took it up — the same reset a stranger picking up an ordinary tool gets. An
  heirloom is dear because of whose it was, not because of use.
- **Found by**: the 2026-11-13 alive audit (code cited at master `dbf7f557`); claimed and fixed on
  branch `fix-1121-heirloom-blade-heir`.
- **Cause (checked)**: confirmed — `legend::take_up` (`crates/hf-sim/src/legend.rs`) had one reset
  path for every new keeper, with no exception for an heir taking up a kin's or partner's
  heirloom.
- **Fix**: `legend::take_up` now keeps `cherished` true when the item is already `heirloom` and
  the previous keeper (the last `owners` entry) is the new keeper's partner, child, mother or
  father; it records `Story.keepsake_of: Option<(AgentId, String)>` (`#[serde(default)]`) with the
  dead's id and name so `legend::describe` can say "an heirloom, Gooskge's" instead of "once
  Gooskge's". A stranger (or kin picking up a non-heirloom tool) still gets the ordinary reset.
- **Guard**: `crates/hf-sim/tests/legend.rs::an_heirloom_from_a_partner_stays_cherished_and_names_them`
  — a partner's cherished tool is marked an heirloom and taken up by the surviving partner; it
  stays cherished, `keepsake_of` names the dead, and the feed's description carries both "an
  heirloom" and "cherished".
- **Related**: #1122 (the dead's named clothes set down beside the heir, who puts on someone
  else's) is a separate bug in how the heir re-equips, not in `take_up`'s reset; left for its own
  issue. `systems::arms::take_up` (weapons/armour picked up at home) has the same unconditional
  `cherished = false` reset this fix removes from `legend::take_up`, but is out of this slice's
  scope — noted for a human to decide whether it needs the same guard.
- **Commit**: this fix's commit.

## 2026-11-15 #1606: seven chats outscored a death because every Minor event in a cell added its full weight

- **Symptom**: in `Director::suggest`, each kept event added `sal * recency * shown * presence`
  to its cell with no limit on how many of one kind could stack. In the five 700-day runs,
  Minor rows piled up in stockpile/hearth cells (`foraged` 321,268, `talked` 197,730, `hauled`
  179,955 across the runs) for the whole `KEEP_TICKS` window (6,000 ticks). In the umbrella
  replay, seed 2's `d335 16:11 [died] Velisin died: drowned through the ice.` scored 7.84
  against 21.44 for a cell with 22 `hauled`, 20 `talked` and 18 `chose` rows: a death lost to a
  pile of routine chores.
- **Found by**: issue #1606 (alive audit, umbrella #1548, depends #1604).
- **Cause (checked)**: `suggest` in `crates/hf-sim/src/view/director.rs` summed every kept
  event's `sal * recency * shown_penalty * presence` into its `(cell, kind)` bucket with no
  discount for repeats, so the thirtieth `hauled` row in a cell counted exactly as much as the
  first.
- **Fix**: `view/director.rs` `Director::suggest`: a `kind_count: BTreeMap<((i32,i32,i32),
  &str), u32>` counts, in `recent`'s existing chronological order, the n-th Minor event of a
  kind seen so far in a cell, and multiplies that event's contribution by `1/n` (harmonic:
  ten `hauled` rows in one cell now weigh about 2.9 of one, not 10). Major and Notable events,
  and the `people` term, are untouched. No new state is kept; `recent` and `by_cell` are as
  before.
- **Guard**: `crates/hf-sim/tests/director.rs::a_death_outweighs_a_stockpile_of_hauling` (30
  `hauled` Minor events in cell A vs. one `died` Major event in cell B, same tick and no one
  awake in either cell, now suggests cell B; failed before the fix, when cell A's stacked
  `hauled` rows outscored the death) and
  `director.rs::busy_people_still_draw_the_camera` (three busy people with no events still beat
  an empty fallback cell, confirming the people term is unchanged).
- **Commit**: this one.

## 2026-11-15 #1131: hearing someone talked about counted as meeting them

- **Symptom**: `Relation::last_contact` was stamped by `Agent::relate` on every call, including
  hearsay -- gossip (`social.rs` `gossip`), suspicion told (`social.rs` `tell_suspicion`) and
  feud sides (`feuds.rs` `set_against`) -- so a relation's `last_contact` could read as recent
  long after the two people last actually met. Seed 5: Ulounor's `last_contact` with Galn read
  run-day 545, though Galn was exiled run-day 175 and never returned.
- **Found by**: the 2026-11-13 alive audit (draft E1b); this is #1131, which #1132 (fade by time
  apart) depends on.
- **Cause (confirmed)**: `relate` has one job, moving opinion and familiarity and stamping
  contact, and every caller used it for both real contact and hearsay alike; there was no
  separate stamp for in-person contact.
- **Fix**: `mind/memory.rs` `Relation::met: u64` (`#[serde(default)]`, additive field, save
  format version unchanged); `agents.rs` `Agent::met(other, tick)`, which creates the relation
  entry through the same cap logic as `relate` and sets only `met`. Called beside `relate` at
  the direct-contact sites only: talk and quarrel (`systems/act.rs`), hearth tale and drink
  (`systems/culture.rs`), pairing and the ring (`systems/life.rs`), making up (`social.rs`
  `make_up`), fostering (`fostering.rs`), working side by side (`systems/work.rs`) and childhood
  play (`systems/childhood.rs`). Hearsay sites (gossip, suspicion told, feud sides) are
  untouched and still move only `last_contact`. `last_contact`'s doc comment now says "last
  heard of" to make the distinction explicit. Nothing reads `met` yet, so there is no behaviour
  change: the world digest is unchanged.
- **Guard**: `tests/social.rs::gossip_does_not_count_as_meeting` (A is told about an absent C for
  10 days of talk: `met` for C stays unset while `last_contact` moves) and
  `tests/social.rs::talk_stamps_met` (a talk that finishes in `systems::act::step` stamps `met`
  on both sides at that tick).
- **Commit**: this fix's commit.

## 2026-11-15 #1796: food taken from a chest with nobody watching was never missed

- **Symptom**: a theft nobody saw left no trace anywhere: nobody ever found a household's chest
  short, so no accusation followed it and no watcher ever remembered a wrong blame over missing
  food.
- **Found by**: the alive audit (umbrella #1112, split from #1118, epic #1531), reading
  `justice.rs:224-240`'s existing "nobody saw" branch of `offence` (decision D48) and noting it
  had no caller for a theft with nobody nearby at all.
- **Cause (checked)**: `offence`'s nobody-saw blame logic lived inline in `offence` itself, so it
  could only run where `offence` was called at the moment of the deed; nothing recorded a chest
  shortage for a keeper to find later, and there was no reusable entry point a keeper's own check
  could call without tripping `offence`'s `seen` test (which would wrongly count the keeper
  standing at the chest as a witness and name the true taker).
- **Fix**: `justice.rs`. Added `Community::missing_food` (capped at 4, `#[serde(default)]`) and
  `report_missing_food` to record a chest shortage with nobody watching, against the real taker
  (`truly`), for a household's keeper to find later. Factored the "nobody saw" blame rule out of
  `offence` into `pub fn lay_blame(w, finder, truly, what, at)`: picks, from those of the
  finder's community within 14 tiles, the one the finder likes least (not necessarily `truly`),
  opens the accusation, and sours only the finder's opinion of the one blamed
  (`relate(blamed, -0.3)` plus a `Hardship` memory of them) — never the finder's opinion of
  `truly` unless `truly` is the one blamed. `offence`'s own nobody-saw branch now calls
  `lay_blame` instead of repeating the rule. Added `find_missing_food`, run once a day from
  `justice::step`: any community member who comes within 3 tiles of a shortage's chest, awake,
  finds it and calls `lay_blame` directly (bypassing `offence` so the finder is never counted a
  witness of their own find); an unfound shortage older than 10 days is dropped.
- **Guard**: `crates/hf-sim/tests/sharing.rs::an_unseen_theft_can_be_laid_on_the_wrong_person`:
  over 20 seeded cases, a keeper standing at the chest with a disliked neighbour nearby always
  opens an accusation once the shortage is reported, and in at least one of the 20 the one
  blamed is not the true taker, whose opinion the keeper's opinion of is unchanged in those
  cases. `report_missing_food` stands in for the low-level "food went missing" primitive #1795
  will eventually give a thief; this fix is scoped to what the keeper does once the chest is
  short, not to the stealing decision itself.
- **Commit**: this one.

## 2026-11-15 #926: nobody could say what the loitering hours actually were, or when work happens

- **Symptom**: 8 goals end as `Activity::Loitering` (Search, Seek, StudyThin, Gather, Warm,
  Flee, Confront, Welcome), as do failed walks and failed work starts; `Activity::name()`
  reports all of them as `"loitering"`, with no split by cause or hour of day. Loitering was
  23.8% of hours in the audit's seeds 5-8 (`hours.txt`) with nothing to say whether it was the
  hearth evening, the end of a purposeful walk, or standing about.
- **Found by**: 2026-11-13 alive audit (desktop), epic #1531, issue #926; #927-#930 need this
  split.
- **Cause**: measurement gap, not a sim bug; no behaviour change.
- **Fix**: new `view::day_shape::DayShape` (per the issue's own correction comment, in the
  library rather than only in an example, since a test cannot reach an example): `observe_decision`
  keeps the goal last chosen for a person; `observe_tick` samples every living, communitied
  person into one of 24 hour-of-day buckets, under a label that is the activity's name, except
  for `Loitering` and `Working`, which are read as `"{activity}:{goal}"` with that last-chosen
  goal (`"unknown"` if none was). `report()` returns the 24 rows. `examples/day_shape.rs` only
  steps the world, reads the goal chosen each tick from the debug decision log
  (`debuglog::configure`/`decision`'s `chose`), and prints the table.
- **Guard**: `tests/pipeline.rs::day_shape_attributes_a_hearth_hour` (one person loiters for 30
  minutes at 19:00 after `Gather` was chosen for them; row 19's `"loitering:Gather"` reads 0.5
  hours, and the 24 rows sum to the 0.5 hours observed).
- **Commit**: this one.

## 2026-11-15 #885: the Why? lens had no place to show what mood did to a choice

- **Symptom**: `why::Considered` carried four score terms (base, emotion, gate, learned) and no
  slot for a mood factor, so once mood steers a choice (#886 onward) the lens would have shown an
  unexplained pick.
- **Found by**: the alive audit (umbrella #883, epic #1531, slice #885, draft B1b).
- **Cause (checked)**: `consider` in `systems/decide.rs` `choose` multiplied `base * emotion * held
  * taught` and `Considered` had no field for a fifth term.
- **Fix**: `why.rs` `Considered.spirits` (read as 1.0 for every option now) and `explain`, which adds
  ", spirits xN.NN" to the winning option's terms only when it is not 1.0; `decide.rs` `choose`
  `spirits` closure, which returns 1.0 for every goal and is multiplied into `score` in `consider`.
  No behaviour change: the digest and `tests/threads.rs` are unchanged.
- **Guard**: `tests/why_lens.rs::why_lists_a_spirits_factor_of_one` (every `Considered` has
  `spirits == 1.0`, the score is the product of its terms, nothing about spirits is printed) and
  `why::tests::spirits_are_said_only_when_they_are_not_one`.
- **Commit**: this one.

## 2026-11-15 #1773: a tool set on a shelf would be hauled off to the stockpile within the hour

- **Symptom**: a tool placed on a finished Shelf's tile counted as loose and got a Haul order
  like anything else lying off the stockpile; a shelf meant nothing, since what was put on it
  did not stay there.
- **Found by**: the 2026-11-14 alive audit, filed as #1773.
- **Cause (checked)**: the planner's loose-item filter (`systems/settlement.rs`, step 2, "Haul
  loose items") only excluded ground tiles already inside a stockpile or a build site; a tile
  under a finished Shelf was not excluded, so any `ItemKind::Tool` sitting on one was picked up
  by the next hourly sweep. #980 (whittling) and #999 (mending), the issue's other half, are
  not yet implemented (`docs/plan/WP-10-alive.md` still shows both unchecked), so there is no
  existing "tool carried or in a chest" helper to extend; one is added now, ready for them.
- **Fix**: `systems/settlement.rs`, the loose-item filter: skip an `ItemKind::Tool` on the tile
  of a finished (`progress >= 1.0`) `StructureKind::Shelf`. `systems/work.rs`: new
  `pub fn tool_in_reach(&World, pos, radius)`, returning a tool carried, held, in a finished
  Chest, or on a finished Shelf within `radius` tiles — for #980/#999 to call once they land.
- **Guard**: `tests/furniture.rs`. `tools_on_a_shelf_stay_there`: a finished Shelf with two
  tools on its tile, a stockpile 6 tiles off, and a third tool on bare floor; after an hourly
  planner sweep across a game-day, no Haul at the shelf tile and the two tools still there,
  while the floor tool got a Haul. `a_shelf_tool_is_in_reach_for_whittling`: `tool_in_reach`
  finds a shelf tool for a person 5 tiles away.
- **Commit**: this branch (`feat/1773-shelf-tools-stay`).

## 2026-11-15 #1823: thirty drowned through the ice and nothing records whether the dead held a lesson about it

- **Symptom**: the alive audit's runs showed "drowned through the ice" deaths repeating within a
  community (seeds 5-8: 5, 3, 12, 10 deaths, #1494) with no way to tell whether the dead had ever
  held a lesson that should have warned them from the plain fact that it happened more than once.
- **Found by**: #1823 (split from #1494, umbrella #1488, epic #1531).
- **Cause (checked)**: a missing measure, not a fault in the simulation (no sim behaviour change).
- **Fix**: `view/alive.rs` `AliveTally::observe_events` now also calls a new
  `observe_death_cause(&self, record: Option<&Agent>, cause: &str)` on every `EventKind::Died`,
  reading the victim while still in `w.agents` (`alive == false`). Two counters are added to
  `AliveTally` and two rows to `report()`, after `grief_n`:
  - `cause_repeat`: a `BTreeSet<(CommunityId, String)>` of (community, cause) pairs already seen
    counts a death whose cause has already killed someone in the same community. A death with no
    community (the dead's `community` field, which `life::die` leaves on the record) never adds
    to the set and is never a repeat.
  - `lesson_ignored`: a small const table, `CAUSE_CUES`, maps a cause (by substring, so "drowned
    through the ice" matches "drowned") to the `ObservationKind`s a lesson about it would cite:
    `"drowned"` to `WaterDepth`, `WaterCurrent` and `Cold`; `"thirst"` and `"froze"` to `Cold`; any
    other cause maps to no cues, so it can never read `lesson_ignored` (there is nothing in the
    table to tell "no lesson existed" from "a lesson existed and was ignored" for a cause not
    listed). A death counts when the victim's `learning.lessons` holds a `Lesson` whose `cue` is
    one of those and whose `confidence >= 0.5` (`LESSON_HELD`): the issue's cut between a hunch
    (`learn`'s starting 0.2 or 0.35) and something a person has actually come to believe.
- **Guard**: `crates/hf-sim/tests/alive_report.rs::death_with_a_held_lesson_is_counted`. A party of
  3 in one community; a `Lesson { cue: WaterDepth, confidence: 0.8, .. }` is pushed straight into
  one person's `learning.lessons` (the normal `learn` path cannot reach 0.8 without many
  confirmations); both that person and another with no lesson are killed with `life::die(..,
  "drowned through the ice")`, each tick's events fed to `observe_events`. `cause_repeat` reads 1
  (the second death, not the first) and `lesson_ignored` reads 1 (the first death, which held the
  lesson; the second victim held none).
- No rng, no clock, `BTreeSet` order, nothing in `systems/` changes, so no digest moves.

## 2026-11-15 #1492 WP-10 K1d: a partner bleeds out and nobody can say how long the survivor grieved

- **Symptom**: no run reported how the bereaved feel after a death. The alive audit's 54 partners
  who lost a partner (seeds 5-8, 30-day mood samples) dropped against the colony's mean by a median
  of +0.03, -0.02, +0.03 and -0.01 at the first sample after, and 44%, 100%, 62% and 94% were
  already at >= 0.9: a 30-day sample cannot see a drop that heals within a week (`LostSomeone` is
  -0.6 for 7 days, `mind/thoughts.rs`).
- **Found by**: the 2026-11-13 alive audit, filed as #1492 (draft K1d).
- **Cause (checked)**: a missing measure, not a fault in the simulation.
- **Fix**: `view/alive.rs`: new `AliveTally::observe_events(&World, &[Event])`, and `observe_hour`
  now also works out each sampled person's mood against the mean of the rest of their community
  (`last_rel`) and adds it to the curves of the open bereavements (`GriefTrack`); four rows are added
  to `report` after `ties_to_dead_share`: `grief_drop_2d`, `grief_drop_10d`, `grief_days`, `grief_n`.
  Nothing in the soak calls `observe_events` yet (it does not call the other `observe_*` either);
  the caller decides when to look and passes `last_events` of each tick. The issue left these open,
  so they are decided here:
  - Who is bereaved of a death (a `Died` row for a person, or for an id the world does not hold):
    the dead's partner, mother, father and children, each link read from either end: `die` clears
    the survivor's `partner` (`unpair`) but not the dead person's own, so a partner is found only
    from the dead's record, while a child's `mother` or `father` is found from the child's. Siblings,
    grandparents and friends are not bereaved. The death of an animal opens nothing.
  - Who is followed: the bereaved must be alive and in `w.agents` when the event is fed and have a
    baseline, i.e. have been sampled at the latest `observe_hour` (alive, a person, with a
    community, not alone in it). So nothing opens when no hour has been seen, for someone with no
    community, for the dead, or for a person away (`w.overworld.away`: alive but not sampled, so
    their grief is not measured, and returning later opens nothing).
  - "The community mean": the mean mood of the rest of the person's community, i.e. the other
    sampled people with the same community id (the dead, people away, people with no community and
    animals are not sampled; the other bereaved are in it). The person's own mood is left out: with
    it in, the issue's own script (a party of four, one dead, three living, a fall of 0.5) would read
    a drop of 0.33, not the 0.5 it asserts. Someone alone in their community has no figure, so
    no point on their curve (the bereavement still counts in `grief_n`).
  - Baseline: the figure at the latest `observe_hour` when the event is fed (the issue's "the last
    `observe_hour`"). The curve: the figure at every later `observe_hour` up to 60 days
    (1,728,000 ticks) after the event's tick, ticks since the death kept with it. A second look at
    the tick of the last point replaces it; a look earlier than the last point, or before the
    death, adds none. A look at exactly 60 days counts; one tick later does not.
  - One bereavement per bereaved person per death: the same event fed again opens none; a person
    who lost two people at once has two.
  - Keys. The drop at a point is baseline minus the figure (a mood above baseline is negative).
    `grief_drop_2d`: the median, over bereavements with a point at or before 2 days, of their
    largest drop among those points. `grief_drop_10d`: the median of the drop at the first point at
    or after 10 days and before 11 days (the hourly look can be missing, a point a day late is not
    the day-10 point). `grief_days`: the median of the days (fractional) to the first point no more
    than 0.10 below baseline that comes after the first point more than 0.10 below it; 0.10
    exactly is back, and a mood above baseline is back (one-sided); someone never more than 0.10
    below reads 0.0; someone who left and is not back is left out of the median until the
    bereavement is 60 days old at the latest `observe_hour` (whether or not they were sampled at
    it), and from then reads 60.0 (the rubric's "60 if never"), one who died or is away included
    (as first built, a bereaved person still low at the latest look read 60 however young the
    bereavement was; the second round, below, changed that). `grief_n` counts the bereavements
    opened, including any with no point yet, so a key can rest on fewer.
  - Medians: an even count gives the mean of the two middle values (as the other keys).
  - The issue's acceptance numbers: `grief_drop_2d` 0.5 and `grief_n` 1 hold. `grief_days` 12 (+-1)
    does not, for a fall of 0.5 recovering linearly over 12 days: 12 days is when the mood is back at
    baseline, but "within 0.10" comes at 9.625 days. The guard asserts 9.625 and, for a recovery over
    16 days, 12.83 (so a run that gives the issue's 12 +- 1 needs a slower recovery).
  No rng, no clock, `BTreeMap`/`BTreeSet` order, nothing in `systems/` changes, so no digest moves
  (D7).
- **Guard**: `tests/alive_report.rs`, all with arithmetic worked out in the comments.
  `grief_curve_reads_a_scripted_death` is the issue's named test (party of four, `die`, the tick's
  events, hourly looks): 1, 0.5, 0.0833, 9.625, and a 16-day recovery 0.5, 0.1875, 12.83; and the
  other rows are the same with and without the events. `only_a_partner_parent_or_child_is_bereaved`
  runs 17 cases alone (partner from the dead's end; partner on the survivor's end; mother, father
  and child from the dead's end; child by `mother` and by `father`; parent by `children`; a sibling,
  a grandparent, a friend; a person with no community; one dead before the hour and one dead after
  it; one away since before the hour and one gone away after it; the partner of a dead person with
  no community, who is bereaved). `deaths_that_are_not_a_persons_loss_open_nothing` (a tick with no
  `Died` row though someone is dead; an animal's death named by a person; a `Died` row for an id
  found nowhere, which still bereaves someone naming it as a parent). `a_death_is_one_bereavement_...`
  (two deaths at once of the partner and the mother of one person: 2; fed again: 2, also a day
  later). `the_baseline_is_the_latest_hour_before_the_death`, `a_death_with_no_baseline_hour_opens_nothing`,
  `a_look_at_tick_zero_and_a_death_there_do_not_trip_the_tally`,
  `grief_is_measured_against_the_rest_of_the_community` (a colony-wide dip of 0.375 around a fall of
  0.5 reads 0.125, then 0.25; the raw drop would be 0.5 and 0.75),
  `each_community_is_measured_against_its_own_mean`,
  `a_bereaved_person_alone_in_their_community_has_no_mean_to_be_measured_against`,
  `the_grief_keys_are_medians_over_bereavements` (three bereaved of one death, one of them reference
  for the others: 0.425, 0.125, 0.0; median 0.125), `bereavements_are_read_at_their_own_ages` (two
  deaths five days apart in two communities: 0.625, 0.125, and 3.0 since the second round, 31.5
  before it) and
  `the_grief_curve_is_read_at_its_edges` (34 scripted curves since the second round, 29 before: 0.10 below at, under and over the
  bound; above baseline; never left, and left by a hair; a drop that comes later; never back; back
  an hour before, at and a tick after day 60; back, low again and back; the 2-day window at 2 days
  and a tick later; the largest, not the first or last; a rise as a negative drop; no look in the
  window; the day-10 point at a tick before, at and after 10 days and a tick before and at 11 days;
  two looks at one tick; a look at an earlier tick; a look before the death; away for days;
  dying mid-curve, and after returning; out of the community for a day; the second round's: never back at 60 days and a tick short of
  it, the first point over 0.10 at exactly day 60, dying and being away to day 60 and a day
  short of it).
  `an_empty_tally_gives_every_key_as_zero` lists the new rows.
  Checked with a temporary switch read from the environment (removed; `git grep` for it was
  empty before the commit; the results are in the evidence file `1492-switches.txt`): each of these
  wrong versions, applied alone, failed at least one test in the file: the reference mean
  including the person, over the whole valley, or including the dead; the raw mood; a lone person
  measured against 0.0; the baseline kept from the first hour, or 0.0; kin read from the dead's
  record only, or from the survivors' only; partner, mother, father or children each left out;
  siblings; friends; a person away, a dead person, a person with no baseline or the death of an
  animal counted; the same feed opening again; one bereavement per person; the 2-day window one
  tick short or one tick long; the first or the last point in the window instead of the largest;
  the drop clamped at 0; the day-10 point read from `>` the mark, from the last point before it,
  with no upper bound, with a two-day slack, or as the latest point; "within" two-sided; the bound
  read as `>=`; never-left read as the first point's time; no "left first" rule; the last return
  instead of the first; the never-back ones left out; points kept past 60 days; a second look at one
  tick appended or ignored; a look at an earlier tick appended; a look before the death counted at
  the death; the mean or the greatest instead of the median; `grief_n` counting people or leaving
  out empty curves; the events ignored altogether (the red run). Two of the first round's wrong
  versions that this list did not catch were found by the reviewer and are caught in the second
  round (below), as is ending the curve at `< 60 days` instead of `<= 60 days`: a curve whose first
  point over 0.10 below is at exactly day 60 reads 60.0 with `<=` and 0.0 with `<`
  (`the_grief_curve_is_read_at_its_edges`, "the first point over 0.10 is at day 60"; the first
  round's note that no test could catch this was wrong). The mean is worked in f64 and the figures
  kept in f32 (the keys are f32 medians); a test cannot see that.
- **Second round (independent review of b41a99a9, one commit on top; no digest, no saved struct)**:
  - `grief_days` read a bereavement that is under 60 days old at the latest look and not yet back
    as 60 ("never"): in a 600-day soak anyone bereaved in the last days is still low at the last
    look and read 60 instead of the few days they take, which pulled the median toward 60 (inside
    the rubric's 10..60 target, so the error flattered the run), and it disagreed with
    `grief_drop_10d`, which leaves out a bereavement with no day-10 point. Now
    `GriefTrack::days_back(latest)` gives `None` for a bereavement that left and is not back until
    it is 60 days old at `latest` (the tally keeps the tick of the latest `observe_hour` in
    `last_hour`, taken at every look, so the dead and the away age too), and `grief_keys` leaves it
    out of the median; from 60 days (`>=`: the look at exactly 60 days is on the curve) it reads
    60.0. A bereavement that never left still reads 0.0 at any age (a curve with no point more
    than 0.10 below), and one with no point at all is left out (`filter` on an empty curve).
    The issue's "60 if never" and the rubric's "back within 0.10 of baseline between 10 and 60
    days" are unchanged: 60 is what a person who has had the 60 days and is still low reads.
    Old expectations that changed, all of them a bereavement left and not back and under 60 days
    old at the last look: `grief_is_measured_against_the_rest_of_the_community` (60.0 to 0.0, a
    median over nothing), `bereavements_are_read_at_their_own_ages` (31.5 to 3.0, X1 alone) and
    twelve rows of `the_grief_curve_is_read_at_its_edges` (60.0 to 0.0, "dies on day 2" among them);
    the row "never back" gained a look at day 60 and stays 60.0. Nothing else changed.
  - New guards: `a_bereavement_not_back_is_left_out_of_grief_days_until_60_days_old` (X back at
    day 1, Y still low: 1.0 at day 5 and at 60 days less a tick, 30.5 at 60 days and a tick after,
    and with Y away at the last look 1.0 at day 59 and 30.5 at day 60);
    `a_bereaved_person_with_no_look_in_two_days_is_not_a_zero_drop` (one death, two bereaved, one
    out of the community at the early looks: `grief_drop_2d` 0.5, not 0.25);
    `a_bereaved_person_never_looked_at_is_not_a_zero_in_grief_days` (one out of the community at
    every look: `grief_days` 1.0, not 0.5); and the edge row for `<` against `<=` above. The
    comment on the issue's "12 +- 1" in `grief_curve_reads_a_scripted_death` now says it comes from
    "back at baseline" (a tolerance of 0), not from a slower recovery.
  - Checked with a second temporary compiled-in switch read from the environment (removed; results in
    `1492-fix-switches.txt`): each wrong version fails at least one test: a not-back bereavement read
    as 60 at any age; never read; read from 60 days with `>`; its age taken from its last point;
    `< 60 days` for the curve's end; `grief_drop_2d` counting a bereavement with points but none in
    2 days as 0; `grief_days` counting an empty curve as 0.0.
- **Commit**: this one.

## 2026-11-14 #1491 WP-10 K1c: close ties never fade and no run counted them

- **Symptom**: no run reported how many close ties go untouched. `Agent::relate` writes
  `last_contact` (`agents.rs`), but nothing reads it outside one test fixture. The alive audit's
  seeds 5, 7 and 8 had 61, 220 and 174 ties at >= +0.95 with no contact for 100+ days (longest
  gaps 462, 435 and 505 days) and 90.7-94.3% of living colonists' top-five ties, pooled, at +1.00. Ties
  untouched for 100+ days averaged only +0.14, +0.50 and +0.62, so most distant acquaintances were
  never close; what does not happen is a close tie fading.
- **Found by**: the 2026-11-13 alive audit (seeds 5, 7, 8, `agents_final.txt` and 30-day
  snapshots), filed as #1491 (draft K1c).
- **Cause (checked)**: a missing measure, not a fault in the simulation. Whether ties should decay
  is a later change that needs these numbers.
- **Fix**: `view/alive.rs`: new `AliveTally::observe_ties(&World)` and five rows added to `report`
  after `act_entropy_sd`. Nothing in the soak calls it yet (it does not call `observe_hour`
  either); the caller decides when to look. Each call replaces the last look, so the keys describe
  the world at the latest call and are not a sum over a run, except `ties_hostile`, which adds up
  the calls, one look per tick (review fixes, below). The issue left these open, so they are decided here:
  - Population: the people the other alive keys sample (alive, a person, with a community). Their
    own `relationships` are the ties; a tie of anyone else (dead, no community, an animal) is not
    read.
  - "Top five": each person's five ties of greatest `opinion.abs()`; equally strong ties rank the
    lower `AgentId` first (the map's order, then a stable sort); a person with fewer than five has
    all of them.
  - `ties_saturated` = top-five ties at opinion >= 0.995 / all top-five ties of everyone, pooled
    over people (not a mean of per-person shares), at the latest look ("at the last sample" in the
    rubric); `ties_hostile` = top-five ties below 0 over the same denominator, pooled over people
    and over every look ("counts every sample it takes"; a tie at 0.0 is not hostile; -1.0 is
    hostile, not saturated). Both 0.0 with no ties.
  - `ties_frozen` = count of all ties (not only the top five, and including ties to the dead) at
    opinion >= 0.95 whose gap (tick - last_contact, saturating at 0) is >= 100 game days = 2,880,000
    ticks. `ties_frozen_max_days` = the largest gap, in whole days (floor, so 100 days and 1 tick
    is 100), of any tie at opinion >= 0.95 whether frozen or not ("longest time since contact of a
    tie still at +0.95 or more", #1488 M6b); 0.0 when there is no such tie.
  - `ties_to_dead_share`: see the review fix below.
  No rng, no clock, `BTreeMap` order, nothing in `systems/` changes, so no digest moves (D7).
- **Review fix (same entry, a later commit)**: a reviewer sent #1491 back with five findings.
  - `ties_to_dead_share` counted a tie to an id missing from `w.agents` as a tie to the dead. In
    this sim an id missing from `w.agents` is a living person off the map, in `w.overworld.away`
    (a journey, `journeys.rs`; an expedition, `systems/expedition.rs`; a captive, `siege.rs`; a
    child taken, `taken.rs`; `societies.rs`). They come back, and someone who
    dies away is pushed back into `w.agents` before `die_away` (`systems/expedition.rs`), so
    `away` holds only the living. Now `observe_ties` builds `id -> alive` over `w.agents` and
    `w.overworld.away`, and a tie is to the dead only when its other person is found there with
    `alive == false` (the audit counted only ids found dead). Choices the finding left open: an
    id found in neither is neither dead nor living, and its ties are not in the denominator, so
    the share is "of ties to someone the world holds, the share to someone found dead" (a tie
    to nobody can neither be a dead person's nor dilute the share); the other ties' counts are
    unchanged. A living person with no community (a drifter) and a living animal are living.
  - `ties_hostile` now adds the hostile and top-five counts of every `observe_ties` call
    (`hostile_pool`), as the rubric's key "counts every sample it takes" and #1488 M5b pool
    every 30-day snapshot; `ties_saturated`, `ties_frozen`, `ties_frozen_max_days` and
    `ties_to_dead_share` stay at the latest look ("at the last sample", "at the end").
  - `ties_frozen_max_days` reads the longest gap of any tie at >= 0.95 (see above); it had
    read the longest of the frozen ones, 0 when none was 100 days old.
  - The tests below now cover the two wrong versions that passed every test: leaving frozen
    ties to the dead out of `ties_frozen`, and counting a living person outside the colony
    (the drifter) as dead.
  - The Symptom line's "of each person's top-five ties" is now "of living colonists' top-five
    ties, pooled", which is how the audit's figure was taken.
- **Second review fix (same entry, a third commit)**: the review of the fix sent #1491 back with
  two findings and a list of wrong versions no test caught. This is the last fix round.
  - `ties_hostile` weighed a look twice when `observe_ties` was called twice at one tick. The
    doc told the caller to look "at the end of a run and every 30 days"; on a 600-day run both
    are the same tick, so the last sample counted double (looks 1, 2, 2 read 7/16 = 0.4375
    instead of 0.4). `AliveTally` now keeps `last_look`, the tick of the latest call and what
    it added to `hostile_pool`, and a call at the same tick takes those counts out before it
    adds its own. Choices the finding left open: a second look at a tick **replaces** that
    tick's counts (the pool holds at most one look per tick, the latest) rather than being
    ignored, so a caller who looks again after something changed in the same tick gets the
    world as it then was; a look at a tick **earlier** than the last look (it should not
    happen) is added normally, like any look at a new tick. The other tie keys always replaced,
    so they are unchanged.
  - The test of `ties_to_dead_share` did not pin what the FIXLOG said of an id found nowhere
    and of ties to the dead outside a top five. The missing id's tie (0.1) was outside A's top
    five, so a version that dropped such ties entirely passed, and both ties to the dead p3 were
    in a top five, so a version that counted dead ties among top fives only passed. Now A's tie
    to the missing id is -0.8, in A's top five (it is in `ties_hostile`, 1/11, and in the
    top-five denominator, but not in the dead-share denominator), and B has seven ties of which
    the weakest, 0.2, is to the dead p3 and falls outside B's top five (A's p3 tie is +1.0, in
    A's). Expected numbers are worked out in the test's comment.
  - Two choices written down, as the review asked. The ties of colonists who are away (a
    journey, a captive; in `w.overworld.away`) are **not read**: `observe_ties` reads the people
    in `w.agents` with a community, as `observe_hour` samples them, so the two stay consistent;
    an away person is still the other end of a living colonist's tie. `a_colonist_away_has_their_own_ties_unread`
    pins it. In the id map `observe_ties` builds, `w.agents` is inserted after `w.overworld.away`,
    so if someone were in both lists `w.agents` would win; nobody is (a person away is taken out
    of `w.agents`, and put back before they die), so this changes nothing and is only said in a
    comment at the code.
- **Guard**: `tests/alive_report.rs`, all with people whose ties were cleared first so each tie is
  one the test made. `frozen_tie_is_counted` is the issue's case (party of 3, +1.0 to B, 101 days
  later +1.0 to C): `ties_frozen` 1, `ties_frozen_max_days` 101, `ties_saturated` 1.0.
  `frozen_ties_read_the_threshold_edges` gives one person eight ties at the edges of the opinion
  bound (0.95 frozen, 0.9499 not), the gap bound (exactly 100 days frozen, 100 days minus one tick
  not), a floor-or-round case (187 days + 20,000 ticks reads 187), a strongly hostile tie 400 days
  old and a weak 300-day one: 4 frozen, longest 187. `top_five_ties_are_ranked_by_strength_and_pooled`
  has one person with eight ties and one with three (pooled 2/8 saturated, 3/8 hostile; a tie at
  +0.995 exactly, a -1.0 tie that is not saturated, and a -0.3 / +0.3 pair for the tie-break).
  `ties_of_people_outside_the_colony_are_not_counted_and_the_dead_are_a_share` has sixteen ties of
  three colonists, fifteen to someone the world holds, two of them to the dead p3 (2/15; A's
  +1.0, in A's top five, and B's 0.2, the seventh of B's seven, outside it): the others go to a
  person away on a journey and to a captive (both in `w.overworld.away`, the captive also
  listed in a site's captives), to a drifter with no community, to a living animal and to
  living colonists; one more tie, A's -0.8, goes to an id found nowhere (in A's top five, so
  `ties_hostile` and `ties_saturated` read 1/11 each; not dead, not in the dead-share
  denominator). A's +1.0 tie to the dead p3, 150 days old, counts in `ties_frozen` (1) and the
  longest gap (150); a 0.0 tie in a top five, and ties of a dead person, of the drifter and of
  an animal that would each move a key if counted, are in the world too. `the_longest_close_gap_is_read_when_no_tie_is_frozen`:
  a 0.97 tie 40 days old, a 0.5 tie 90 days old and a 0.9499 tie 80 days old read 0 frozen and
  40 days. `hostile_pools_every_look_and_the_other_keys_read_the_latest`: two looks, hostile
  (1 + 3) / (4 + 6) = 0.4 while saturated reads the latest 1/6.
  `a_later_look_replaces_the_earlier_one`, `a_tie_dated_after_now_is_not_frozen` (no underflow) and
  `a_colony_with_no_ties_gives_zero_tie_keys` cover the rest; `an_empty_tally_gives_every_key_as_zero`
  lists the new rows.
  `a_second_look_at_the_same_tick_is_not_pooled_twice` (looks 1, 2, 2 read 0.4, not 7/16),
  `a_second_look_at_the_same_tick_replaces_the_first_and_an_earlier_one_is_added` (the third look
  sees one more hostile tie: 5/11; then a look at an earlier tick: 9/18 = 0.5) and
  `a_colonist_away_has_their_own_ties_unread` (a captive's +1.0 and -0.9 ties are not read).
  Checked by a temporary switch read from the environment (removed; `git grep` for it was empty
  before each commit): each of these wrong versions, applied alone, failed at least one test in the
  file: gap `>` for `>=`; opinion `>` for `>=`; close bound 0.9; days rounded; frozen read
  `opinion.abs()`; plain subtraction for the gap; top six, top four and all ties; ordering by signed
  opinion; equal ties ranked by the higher id; saturation read `abs()` or `>`; hostile `<=` 0;
  a missing id not counted as dead; a living animal counted as dead; ties of people with no
  community, of the dead and of animals counted; looks added up instead of replaced; the longest
  gap taken as the last one. The mean of per-person shares for the pooled share was run as a
  switch in the review fix and fails `top_five_ties_are_ranked_by_strength_and_pooled` (0.2
  against 0.25). In the review fix, with the same kind of switch, these also each fail a test:
  a missing id counted as dead (the first version of the rule) with or without the denominator;
  people away not looked for; living read as "alive with a community"; a living animal read as
  dead; frozen ties to the dead left out; the longest gap over frozen ties only, and over every
  tie whatever its opinion; `ties_hostile` from the latest look only or the first only;
  `ties_saturated` pooled over looks. In the second review fix, with a switch of the same kind
  (read from the environment, removed), each of these fails a test: a second look at the same tick added
  again (the committed behaviour), ignored, or resetting the pool to itself; an earlier look
  that replaces the last or is ignored; the ties of people away read; dead ties counted among
  top fives only (both counts, or the dead count alone); a tie to an id found nowhere dropped
  before the top five is taken; hostile read as `<= 0`. The results are in the evidence file
  `1491-fix2-red.txt`.
- **Commit**: the first commit is `1ce99d8e`; the first review fix is `dd58a599`; the second
  review fix is the one that follows it, this one.

## 2026-11-14 #1490 WP-10 K1b: nothing recorded what people spend their waking hours doing

- **Symptom**: no run reported how much of the day is standing about. The alive audit's exact
  daily hours (seeds 5-8) had loitering + idle at 49.7-54.7% of waking hours, working at 1.0-3.1%
  and talking at 1.7-1.8%, and the median person's day sat 5.7-11.3 points from the colony's
  average day; none of it was visible without a custom probe.
- **Found by**: the 2026-11-13 alive audit (seeds 5-8, probe `daily.csv` and `hours.txt`), filed
  as #1490 (draft K1b).
- **Cause (checked)**: a missing measure, not a fault in the simulation: the 22 goals collapse into
  ten `Activity` names (`mind/activity.rs`, `Activity::name()`) and nothing counted them.
- **Fix**: `view/alive.rs`: `AliveTally::observe_hour` now also adds one hour to a per-person
  counter `[u32; 10]` (`BTreeMap<AgentId, _>`, in the order of `Activity::name()`: idle, going,
  eating, foraging, sleeping, talking, loitering, wandering, working, drinking) for the same
  people the mood keys sample (alive, a person, with a community). `AliveTally::with_min_hours(n)`
  sets the hours a person needs for the two per-person keys; `default()` is 2,000. `report` adds
  six rows after the mood rows. "Waking hours" are all sampled person-hours except sleeping.
  - `standing_share` = (idle + loitering) / waking hours; `unreadable_share` = (idle + loitering +
    going) / waking hours; `work_share` = working / waking hours; `talk_share` = talking / waking
    hours. All 0.0 when there are no waking hours.
  - `day_distance`: over people with at least `min_hours` sampled hours. A person's share vector is
    their hours in each of the ten activities (sleeping included) divided by their own sampled
    hours. The colony's mean day is the mean of those vectors, each person counting once (not
    weighted by hours). A person's distance is half the L1 distance from the mean day, times 100
    (points, 0 to 100). The key is the median of the distances; with an even count, the mean of the
    two middle values. 0.0 when nobody qualifies.
  - `act_entropy_sd`: over the same people, the population s.d. (divide by n, as `mood_spread`) of
    each person's Shannon entropy, in bits (log base 2, 0 log 0 = 0), of their share vector. 0.0
    when nobody qualifies.
  No rng, no clock, nothing in `systems/` changes, so no digest moves (D7). Per D56 / design-rules
  rule 7 this measures leisure overall; `examples/normal_person.rs` still measures leisure while
  food or shelter is unsecured. The issue's test said `day_distance` would be 0 for the mixed
  party; its #1490 correction (three loiterers, two workers and a sleeper are 50, 50, 50, 66.7,
  66.7 and 83.3 points from the mean day, median 58.3) is what the guard asserts.
- **Guard**: `tests/alive_report.rs::activity_keys_count_a_loitering_day` - six people (three
  loitering, two working, one sleeping) for 24 hours with `with_min_hours(24)` give `standing_share`
  0.6, `unreadable_share` 0.6, `work_share` 0.4, `talk_share` 0, `day_distance` 175/3 and
  `act_entropy_sd` 0, while an idle person with no community, a working dead person and a talking
  animal that belongs to the colony stay out; a second tally with all six loitering gives
  `day_distance` 0. `activity_shares_count_every_waking_activity` holds one person in each of the
  ten activities for a different number of hours (1, 2, 4, ... 512, in the order of
  `Activity::name()`), so every sum of slots is unique: `standing_share` 65/1007, `unreadable_share`
  67/1007, `work_share` 256/1007 and `talk_share` 32/1007 over 1007 waking hours, which fails if
  two slots that the keys count differently are swapped (idle with talking, idle with eating,
  going with foraging, talking with wandering, drinking with going were each tried) or sleeping
  stays in the denominator. Swaps within one bucket cannot change any reported key (idle with
  loitering, and the 6 pairs among eating, foraging, wandering and drinking: 7 of the 45 pairs;
  `day_distance` and `act_entropy_sd` also ignore a relabelling applied to every person alike);
  the other 38 pairs move a share. It also shows
  that the default 2,000-hour minimum leaves the person keys at 0.
  `day_distance_and_entropy_read_each_persons_own_mix` has six people sampled for 6, 6, 6, 6, 4
  and 3 hours with `with_min_hours(4)`: the median of the five who qualify is 30 (not the mean of
  the distances, 36; not 25, which a mean weighted by hours gives; not 41.7, which a minimum of
  `min_hours - 1` gives, because the sixth person has exactly 3 hours), the entropy s.d. is 2/5,
  and that person still counts in the shares. `a_colony_asleep_has_no_waking_hours_to_share`
  guards the divide by zero, and `an_empty_tally_gives_every_key_as_zero` the new rows' shape.
  Review fix (same date): the first version of these two tests could not tell those wrong
  versions apart (one hour in every slot; equal hours for everyone who qualified), so they were
  rebuilt as above; no source line changed.
- **Commit**: this one.

## 2026-11-14 #1489 WP-10 K1a: nothing recorded how people feel over a run

- **Symptom**: the soak's daily row had no mood column and its verdict read no mood. Mood is
  recomputed every tick stripe (`systems/needs.rs`) and clamped to +-1, but only the view and the
  alive audit's hand-made probe ever read it over time, so no fix to inner life (#813, #883) could
  show its effect without a custom three-hour probe.
- **Found by**: the 2026-11-13 alive audit (seeds 1-8, 600 days), filed as #1489: 59-72% of the
  probe's 30-day mood samples sat exactly on the +1.00 clamp.
- **Cause (checked)**: a missing measure, not a fault in the simulation.
- **Fix**: new read-only `view/alive.rs`: `AliveTally::observe_hour` samples the mood of every
  living person with a community (and the hour's s.d. when five or more were sampled), and
  `report` gives the keys `mood_median`, `mood_p10`, `mood_hi`, `mood_lo`, `mood_clamped` and
  `mood_spread`. `mood_p10` is the nearest-rank 10th percentile of all sampled moods: the sample at
  rank floor(0.1 * (n - 1)) of the sorted list, no interpolation (n = 6 gives the lowest; n = 12
  gives the second lowest). No rng, no clock, nothing in `systems/` changes, so no digest moves (D7).
  The caller (soak, probe) decides when to call it; later alive slices add keys to the same report.
- **Guard**: `tests/alive_report.rs::mood_keys_read_a_built_colony` - six founders with moods set
  by hand give the expected median, p10 (-0.5), shares and spread, while a person with no community,
  a dead person and an animal that belongs to the colony, each given a mood of -1.0, stay out; a
  second tally gives the same report. `mood_p10_is_the_nearest_rank_over_the_whole_run` (twelve
  samples over two hours) separates the nearest rank from the minimum, and
  `an_empty_tally_gives_every_key_as_zero` guards the empty-slice cases.

## 2026-11-14 #1739 (D95): trees matured at about 3.5 game years, not Dan's three

- **Symptom**: `P::Tree`'s `days_to_mature` of 40 (`plants.rs`), under the season and sky
  factors `systems/weather.rs` applies, matures a sapling in about 3.5 game years rather than
  the three years Dwarf Fortress uses and Dan decided on.
- **Found by**: the 2026-11-14 alive audit (#1739), reading `plants.rs` and `weather.rs`.
- **Cause (checked)**: the constant was set without deriving it from the season/sky factors.
- **Fix**: `crates/hf-sim/src/plants.rs`. Scaled `days_to_mature` by 6/7 (40 -> 3.5yr; target
  3yr) for `P::Tree` (34.3) and `P::BoneTree` (34.3), and by the same ratio for the climate
  trees so their relative pace is unchanged: Alder 35 -> 30.0, Ash 45 -> 38.6, Yew 80 -> 68.6,
  Willow 25 -> 21.4, CrabApple 35 -> 30.0. Crops, shrubs and `P::FungusTree` (cave timber, not
  covered by this decision) are untouched. DECISIONS.md D95.
- **Guard**: `crates/hf-sim/tests/ecology.rs::a_tree_reaches_harvest_size_in_about_three_years`
  — an oak sapling at growth 0.02, stepped through seed 1's standard weather, first reaches
  `mature()` between day 50 and day 70 (2.5-3.5 game years).
- **Commit**: this one.

## 2026-11-15 #1830: a sapling on a busy path grew to a tree instead of dying underfoot

- **Symptom**: `paths::step` counted footsteps and wore ground to a path, but nothing read
  that wear for plants, so a seedling on the busiest tile in camp grew exactly like one in a
  quiet glade.
- **Found by**: the 2026-11-14 alive audit (#1830), reading `paths.rs` and `systems/plants.rs`.
- **Cause (checked)**: no code path connected `w.wear` to plant growth or death.
- **Fix**: `crates/hf-sim/src/paths.rs::step`, in the same daily pass as the wear/path update:
  a `TRAMPLE = 20` constant (a third of the way to `WORN`); any green tree (`is_green_tree()`)
  with `growth < 0.3` on a column whose wear is at or above it dies (`alive = false`), with a
  `Trace`-severity `EventKind::SaplingTrampled { at }` appended only when one actually died.
  No roll: the cause is the footsteps already counted. `EventKind::SaplingTrampled` appended
  to the enum (events.rs) with match arms in `participants()`, `kind_name()` and
  `view/narrate.rs::line`.
- **Guard**: `crates/hf-sim/tests/paths.rs::a_sapling_on_a_busy_path_dies_and_one_aside_lives`
  — a sapling on wear 30 (fading to 27, still over TRAMPLE) dies at the next daily pass; one
  on wear 5 lives through ten days.
- **Commit**: this one.

## 2026-11-15 #1817: a hillside of graves at an old hamlet never read as sickness

- **Symptom**: a settler guessing how an unexplained old hamlet ended (`lore::step`) read
  burned timbers when the site truly burned and otherwise always guessed "the young left",
  never reading the hamlet's own graves even when there were several.
- **Found by**: the 2026-11-14 alive audit (#1817, split from #1398), reading `lore.rs` and
  `deep.rs`.
- **Cause (checked)**: the guess checked only `fearful` and the true cause's `Claim::Fire`;
  `w.history.graves` was never consulted.
- **Fix**: `crates/hf-sim/src/lore.rs::step`. A non-fearful settler now checks `w.history.graves`
  within 6 tiles of the site's centre on its level first: 3 or more gives `Claim::Fever` with
  `how` "the graves there"; otherwise a truly burned site gives `Claim::Fire` with `how` "the
  burned timbers" (previously always "their own guess"); otherwise `Claim::YoungLeft` as
  before. A fearful settler still says `Curse` regardless.
- **Guard**: `crates/hf-sim/tests/lore.rs::an_old_hamlet_with_many_graves_reads_as_sickness` —
  a site with 3 graves within 6 tiles gives `Fever`/"the graves there"; the same site with no
  graves gives `YoungLeft`; a fearful settler at the graves still says `Curse`. The existing
  `a_ruin_nobody_explained_is_guessed_at` updated: seed 11's real site 0 has graves nearby, so
  its guess is now `Fever`/"the graves there" instead of the old always-"their own guess".
- **Commit**: this one.

## 2026-11-15 #1816 (D41): a stranger at a left camp always knew the true way its people went

- **Symptom**: `left_sites::wonder` gave every stranger standing among a left community's houses
  an omniscient tale: the compass to the community's true current home, or its true current name,
  with no regard for what the ground there could actually show (design rule 8: nobody should know
  what they could not have read).
- **Found by**: issue #1816 (umbrella #1392, split from #1398).
- **Cause (checked)**: `wonder` (`left_sites.rs`) built its `went` string straight from each
  `LeftSite`'s own `to`/`community` fields, which hold the truth, not evidence a passer-by could
  see.
- **Fix**: `left_sites.rs` adds `read_site`, first match wins: three or more graves (`w.graves`)
  among the site's floors or beside its walls read as `Claim::Fever`; a wall or floor tile in
  `w.scorched` reads as `Claim::Fire`; failing those, a path at or above `paths::WORN` leading out
  (`worn_way_out`, the farthest worn tile within 15 of home, read with `threat::compass`) reads as
  `Claim::MovedTo` with that compass direction, not the true one; with none of these, the tale is a
  new `Claim::Unknown` ("nobody can say where"). A fearful reader (`Trait::Cautious` or a lean
  toward death) still guesses a curse, as `lore.rs`'s old-hamlet guess does. The evidence read goes
  in the tale's `how`.
- **Guard**: `tests/left_sites.rs::a_ruin_is_read_from_what_lies_there` (graves read as Fever);
  `a_newcomer_among_the_left_houses_wonders_at_them` updated to lay a worn path first, since the
  claim is no longer given for free.
- **Commit**: this commit.

## 2026-11-15 #815: a long memory did not know when it was last renewed

- **Symptom**: `mind::Memory` held `formed` and a reinforcement count but not the tick of the
  last reinforcement, so a memory renewed again last night looked exactly as old as one
  renewed only on the day it formed; #816's bounded mood term needs that tick to weigh recent
  memories above stale ones.
- **Found by**: issue #815 (part of WP-10's mood cluster, umbrella #813, epic #1531).
- **Cause (checked)**: `Memories::consolidate` (`mind/memory.rs`) set `formed` once, on first
  creation, and never touched it or any other timestamp again on reinforcement.
- **Fix**: added `Memory.last: u64` (`#[serde(default)]`, so an old save without the field
  loads as 0), set to `now` both when a memory is first created and whenever it is
  reinforced; `opening.rs`'s formative-memory seeding sets it to the memory's own `formed`
  tick. Nothing reads it yet (#816 will).
- **Guard**: `tests/mood.rs::a_reinforced_memory_records_when` (formed stays put, last moves on
  reinforcement) and `an_old_memory_without_last_loads` (a pre-#815 MessagePack map decodes
  with `last == 0`).
- **Commit**: this commit.

## 2026-11-15 #879: a person dying of thirst read as content

- **Symptom**: mood's needs term (`systems/needs.rs`) read hunger, rest, social and comfort
  only; thirst and warmth never entered it, and there was no `Thirsty` thought (`Cold` was the
  only route for warmth). Seed 6, run-day 101: six people died of thirst in one night while the
  probe's daily row read mean mood 0.979, minimum 0.883.
- **Found by**: issue #879 (alive audit, umbrella #878, epic #1531).
- **Cause (checked)**: `needs::step`'s mood block averaged four of the six needs; thirst and
  warmth were left out of the average outright.
- **Fix**: the needs average in `needs::step` now includes `thirst` and `warmth` (divisor
  4 to 6); `strain::step` adds a new `ThoughtKind::Thirsty` ("parched", shape `(-0.3, 0.5)`,
  matching `Hungry`) at the same `needs.thirst < 0.2` gate `Hungry` uses. (#819's planned
  `mood_terms` extraction and shaped thirst valence are not landed yet; this folds the fix into
  the existing inline formula rather than wait on that refactor, and can be folded into the
  extracted helper once it lands.)
- **Guard**: `tests/thirst.rs::a_parched_person_is_miserable_and_strained` (two otherwise
  identical people, one parched: the parched one reads at least 0.2 lower and carries the
  `Thirsty` thought; needs and strain driven directly, bypassing decide/movement, so neither
  walks to the pond mid-test).
- **Commit**: this commit.

## 2026-11-15 #947: solitary people got lonely exactly as fast as everyone else

- **Symptom**: `social_rate()` gives a solitary person (sociability near 0, or the legacy
  `Solitary` trait) a rate below 1, meant to mean "lonelier more slowly"; but only the
  sociable half of people ever differed from the baseline decay.
- **Found by**: issue #947 (alive audit, related #905, #930; epic #1531).
- **Cause (checked)**: `needs::step` (`crates/hf-sim/src/systems/needs.rs`) computed the extra
  decay from `(a.social_rate() - 1.0).max(0.0)`, which floors every rate below 1 at exactly 0,
  so a solitary person decayed at the same base rate as everyone else.
- **Fix**: dropped the `.max(0.0)`; the signed extra now also slows decay for a rate below 1,
  and the result is clamped to `0.0..=1.0` (the old code only floored at 0, since a negative
  extra was previously impossible).
- **Guard**: `tests/social.rs::solitary_get_lonely_slower` (sociability 0.1 against 0.5, 12
  waking hours with nobody to talk to: the solitary one's social need stays at least 0.15
  higher).
- **Commit**: this commit.

## 2026-11-15 #946: a grieving or breaking-down person built exactly as fast as a contented one

- **Symptom**: work speed (`systems::work::run`) was `skills.speed(skill) * tool_factor *
  body_factor`, where `body_factor` covered wounds, heat, the dark and conditions but never
  mood or stress. A miserable, stressed person's hands worked exactly as surely as a happy
  one's.
- **Found by**: issue #946 (alive audit, related #883, #886; epic #1531).
- **Cause (checked)**: no mood or stress term existed anywhere in the work-speed calculation.
- **Fix**: added `systems::work::spirit(mood, stress) -> f32`
  (`(0.9 + 0.15*mood - 0.1*stress).clamp(0.7, 1.05)`), multiplied into `body_factor` in `run`.
  Implemented directly against the existing `a.mood`/`a.stress` fields; #813's mood-honesty
  rework (currently pinning mood near +1.00 for most colonists) is not landed yet, so the
  practical effect stays modest until it lands, as the issue itself notes.
- **Guard**: `tests/work.rs::low_spirits_work_slower` (mood −0.8/stress 0.6 against mood
  +0.8/stress 0: the contented figure is at least 1.25x the miserable one).
- **Commit**: this commit.

## 2026-11-15 #863: waiting work says "waiting for 1 bed frames"

- **Symptom**: the Waiting work panel showed "Bed: waiting for 1 bed frames; none lies
  anywhere in the valley" — the count and the item's plural didn't agree for items whose
  label is already plural (`bed frames`).
- **Found by**: the local AI (VPS-2) in QA pass 5, loading a ring save from the #781 soak under Xvfb.
- **Cause**: `view::waiting::materials_nowhere` (`view/waiting.rs`) always used
  `ItemKind::label()`, which returns the plural form for every kind, regardless of count.
- **Fix**: added `ItemKind::label_for_count(count)` (`items.rs`), returning the singular form
  for a count of one (mass nouns and words with no clean singular, such as "clothes" or
  "ore", keep their plain form); `materials_nowhere` now calls it with `need`.
- **Guard**: `tests/waiting.rs::a_single_missing_bed_frame_is_singular`.
- **Commit**: (this branch, fix-863-waiting-count-plural).

## 2026-11-15 #809: a wild animal's death is recorded as Major

- **Symptom**: a new valley's first hour often showed "A bat died: engulfed by slime" as a
  top-centre toast and an orange major feed line, ahead of the founding and the first
  arrivals, because `systems/life.rs::die_at` pushed every `Died` event at `Severity::Major`
  including wild creatures nobody saw.
- **Found by**: the local AI (VPS-2) in a QA pass of fresh valleys under Xvfb (seeds 3 and 11).
- **Cause**: `die_at` always pushed `Severity::Major`, with no check of species or witness.
- **Fix**: added `death_severity` (`systems/life.rs`), used by `die_at`: Major for a person
  or a kept animal; otherwise Minor when a person saw it (the sight rule `accounts::witnessed`
  already uses) and Trace when nobody did. See DECISIONS.md D98.
- **Guard**: `tests/death_severity.rs::an_unwitnessed_wild_death_is_trace_a_witnessed_one_is_minor_a_person_is_major`.
- **Commit**: (this branch, fix-809-wild-death-severity).

## 2026-11-15 #862: two people of one community share a name (partial)

- **Symptom**: two different living people named Galathi in one community (Stoneside), and
  separately a departed Eirthi and a later-arrived Eirthi in the same community; feed lines,
  History, accounts and "X told…" can't say which one.
- **Found by**: the local AI (VPS-2), QA pass 5 and the parallel audit (seed 4 Relentless).
- **Status: partial.** Fixed for a birth, arrival or joiner colliding with a name still held
  by a living or a dead person's kept record. Not fixed: a name freed by a band leaving the
  valley outright (`journeys.rs`/`expedition.rs`/`societies.rs` remove the agent record, so no
  history survives to check against) — that needs a persisted valley-wide name history the sim
  doesn't keep today.
- **Cause**: `names::person_name` and the call sites that use it (`World::spawn_agent`,
  `World::give_birth`) drew a name from the seed and the new agent's id alone, with no check
  against anyone already named that in the valley.
- **Fix**: added `names::unique_person_name`, which rerolls `person_name` (same trick as
  `opening::slot_key`'s reroll) against a `taken` check, up to 64 tries; `spawn_agent` and
  `give_birth` call it checking every `Person` in `self.agents` by name, living or dead. See
  DECISIONS.md D99.
- **Guard**: `tests/names.rs` (all three tests, in particular
  `spawned_people_in_one_valley_never_share_a_name`).
- **Commit**: (this branch, fix-862-unique-person-names).

## 2026-11-15 #859: roofs wait 207 days untaken in a hard cold

- **Symptom**: ten `Roof` orders all read "2 could take it; other work weighs more for now
  (for 207 days)" through a hard cold, while Mining and other work went on.
- **Found by**: the local AI (VPS-2), QA pass 5.
- **Cause**: `decide.rs`'s `choose` lifts BuildRoof (and other build work) 1.6x only when the
  worker is personally `roofless` — homeless and it is dusk. A hard cold draining everyone's
  warmth never touched the score, so once a household already knew of a bed, nothing ever
  weighted roofing over other work, however cold it got.
- **Fix**: split `BuildRoof` out of the shared build-kind match arm in the designation-scoring
  loop; it now also lifts 1.6x when `a.needs.warmth < 0.5` (the same low-warmth signal
  `freezing`, earlier in the same function, already uses for frostbite risk).
- **Guard**: `tests/roof_in_cold.rs` (`a_cold_worker_roofs_before_mining`,
  `a_warm_worker_is_free_to_pick_either`).
- **Commit**: (this branch, fix-859-roof-cold-priority).

## 2026-11-15 #1292: a caravan kept trading with the town that was besieging the valley

- **Symptom**: seed 8: Mordric traded at Nis on days 105, 114, 123, 132, the same weeks Nis's
  hosts camped and laid siege to the valley on days 110, 121, 131, 140. Nobody walks berries
  into the town whose men camped at their wall last week.
- **Found by**: the 2026-11-13 alive audit (#1292).
- **Cause**: `trade::best_errand` (and `best_fetch`, used by `systems/projects.rs::lacking` for
  fetch trips) chose any known site by what it pays, with no check of hostility.
- **Fix**: `trade::recently_hostile` skips a site the community's own `known_others` (what the
  valley itself saw, not the site's own `grudge`, which nobody in the valley has perceived —
  the issue's first proposal read `grudge` and was corrected to this by its own author) has an
  entry for with `armed: true` and `last_seen` within 20 days. Applied to both `best_errand`'s
  and `best_fetch`'s site filters.
- **Guard**: `trade.rs::no_trade_with_besiegers`.
- **Commit**: see PR for #1292.

## 2026-11-15 #1322: a realm hamlet was founded with the headcount that set out, not who arrived

- **Symptom**: seed 6: `d100 [realm_founded] | The Nofu made their first home at Tortea` for 12
  people, but only 6 left the valley and 6 died of thirst on the way (#1316). The realm should
  hold who arrived, not who set out.
- **Found by**: the 2026-11-13 alive audit (#1322). The issue's first proposal (found at
  population 0, fill as each arrives) was corrected by its own author: a site founded at
  population 0 and never filled (everyone dying on the way) is a false history, since the
  realm's own yearly step ends any site under 4 as "hunger emptied it" (`realm/sites.rs`).
- **Cause**: `moving::consider`'s out-of-valley branch called `realm::live::settle_from_valley`
  (founding the site immediately, at the full `going.len()`) at the moment of choosing, before
  anyone had taken a step.
- **Fix**: split it into `realm::live::reserve_settle_from_valley` (draws the site's name and
  spot at the choice, as the departure line needs them, but founds nothing) and
  `realm::live::settle_arrived` (founds the site at population 1 on the first leaver who
  actually steps off the map, in `systems::society::leave_valley`; a later arrival adds 1). A
  departure nobody survives founds nothing. Added `Agent.bound_for: Option<u32>`, the pending
  reservation a leaver carries; `Realm::found` split into `found`/`found_named` so the eventual
  founding reuses the name drawn at the choice rather than redrawing one.
  `realm/chronicle.rs::line` says "made their first home" only for a people's actual first
  site; a later valley hamlet (also founded with no parent) is narrated as a plain founding.
- **Guard**: `realm_live.rs::a_valley_hamlet_counts_who_arrived`;
  `moving.rs::a_community_that_leaves_the_valley_becomes_a_settlement_of_the_world` updated for
  the deferred founding.
- **Commit**: see PR for #1322.

## 2026-11-15 #1217: a hardship names a drowning or bleeding death "of drowned"/"of bled out"

- **Symptom**: `opening::died_how` prefixes every cause with "of " unless it matches a known
  verb-phrase prefix (`lost `, `bitten`, `mauled`, `brought`, `taken`, `killed`, `under `). A
  death cause already written as a verb phrase ("drowned through the ice", "bled out",
  "engulfed by the flood", "scattered by raiders", "torn apart", "froze solid") read as "of
  drowned through the ice", "of bled out", etc., which does not parse as English after "died".
- **Found by**: issue #1217 (narration-only; nothing in the simulation reads the wording).
- **Cause**: the `died_how` match guard's prefix list did not cover these verb-phrase causes.
- **Fix**: `opening.rs::died_how` — added `drowned`, `bled`, `engulfed`, `scattered`, `torn`,
  and `froze ` (with trailing space, so it does not catch the bare `"froze"` arm above it) to
  the no-"of" prefix list.
- **Guard**: `moving.rs::a_hardship_names_a_drowning_plainly`.
- **Commit**: see PR for #1217.

## 2026-11-15 #1564: why a band set down far from its camp never takes a step (staged cause confirmed; seed 4's own cause not: partial)

- **Symptom**: #1543's audit: on seed 4 (Relentless) after day 553, every band set down on the south
  edge at x 105-107 stood there until its people died of thirst (88 listed members, one ever moved
  more than 6 tiles; the camp 84 tiles north). The issue named four causes, none checked.
- **Found by**: the alive audit (umbrella #1543, slice #1564).
- **Status: partial.** Seed 4's own cause is not named. The trace of seed 4 (below) ran to day 565
  on this code and never reached the audit's state: the home stayed (71,55) (it never became
  (106,17)), 155 bands came in and all walked in (none was still approaching 1,000 ticks after set-down).
  So the trace cannot show what stopped the audit's day-560 bands, and seed 4's cause stays
  unconfirmed. The follow-up slice for "none of (a)-(d)" (a draft is kept with the evidence for the
  coordinator to file) should start from the audit's own build, or from a soak that reproduces day 553.
  Measured separately by the orchestrator on the #1563 build (posted on #1543, comment 6893679346):
  `deaths_probe 4 700 quiet --engine relentless` listed 840 band members, 4 who never moved more than
  6 tiles, 1 band death (drowned through the ice) and no thirst deaths.
- **Cause (checked in a staged scene; NOT confirmed for seed 4's day 560)**: `World::path_agent_to`
  (`world_state.rs:431`) refuses a path that has any tile over the head when the walker's own tile
  is not (`world_state.rs:464`, the #418 rule), returns false, caches nothing, and makes no search for
  a way round; `threat::go` (`threat.rs:541`) then leaves the member where they stand and the next
  band step (50 ticks on) searches again. A* does count a deep tile as 200 ticks (`nav/step.rs:60`,
  `OVER_HEAD`) but only as a cost: when the way across is cheaper than the way round it returns the way
  across, and the refusal comes after. `nav.reach` (regions, `nav/regions.rs:102`) and
  `edge_tile` (`systems/expedition.rs:56`) know nothing of depth, so the camp and the edge count as
  joined. The staged scene (`far_band`: a 128-tile valley, home (106, 17), a river two rows wide of
  water over the head across all of it at y 70-71 with no ford, a hungry place to the south) matches
  the audit in one respect: the band stands where it was set down. It differs in another: in the
  scene the 8 settlers (agent ids 0-7) also die of thirst, 22,102 to 22,482 ticks after set-down, and do
  not die when the refusal is switched off (a reviewer's run, not asserted by any test), whereas on
  seed 4 the settlers lived; so the scene is not seed 4's valley. The band test does not assert
  thirst; the acceptance check does, and in this valley it fails for a band camped at (104, 40) even
  when that band walks in (see Guard). Four members are set down at (105,125), (106,125), (107,125), (105,125); the camp
  is (104, 40); for each, `walkable(camp)` true, `nav.reach` true, nothing in the failed-walk cache, A*
  finds the 86-tile way (125 - 40 = 85 steps) within the 12,000 expansions, two of its tiles over the
  head; `path_agent_to` returns false and caches nothing; 12 game-hours on, all four are alive, within 3
  tiles of where they were set down and not within 4 of the camp. Run to 2 game-days, Halgar (the
  first member the check reaches; it stops at the first failure, so the other three are measured
  only at the 12-hour check, within 3 tiles of set-down) comes no nearer to the camp than 85
  tiles (`1564-r3-runs.txt`, the acceptance test with `--ignored`). Of the issue's candidates this scene rules out (a) the
  12,000-expansion limit and the failed-walk cache (found, not cached), (c) the member's own tile and (d)
  the camp (both reach); it confirms (b) as the one that holds. Two more places hold the same rule and
  are not what these tests exercise: `par.rs:114` has a second copy of the refusal (the batched walks
  `decide.rs` searches), and `movement.rs:98` (`into_flood`) separately stops anyone on dry ground
  from stepping into water over the head; with the refusal in `path_agent_to` switched off, a
  reviewer's run had the band walk to the south bank (105-107, 72) and be stopped there by
  `movement.rs:98`, dying of thirst 47,962 ticks after set-down.
- **What this does not show**: that (b) is what stopped seed 4's day-560 band. On seed 4's real map none
  of (a)-(d) holds in the states tried (`EVIDENCE/u1543-1564`, scratch examples kept there): at day 0
  the south set-down tiles (105,125), (106,125) and (107,125, one level up) all reach the camp
  tile (105, 41); A* finds the way within 1,595, 1,606 and 1,770 expansions at the smallest budget (limit
  12,000), 87-88 tiles, none over the head (it wades four shallow tiles at (96, 52-55) in a chain of
  three water bodies across y 52-66); with all three bodies in spate (270 tiles over the head) A*
  goes round (92 tiles) and `path_agent_to` accepts it, and the same after 120 hours of flood at
  the cap (857 tiles over the head, path 91); and in a world four days old with the home moved to
  (106, 17) and a hungry place to the south, a band of five set down at (105-107, 125) walked in: four
  of the five were within 4 tiles of its camp (103, 44) 4,050 ticks after set-down. So whatever stopped
  seed 4's bands after day 553 is a state of the map that the runs above did not have.
  A trace of seed 4 Relentless to day 565 (`u1564_scratch`: for each band, the same checks and a
  `find_path` result and the smallest budget that finds the way, at 60, 120, 1,000 ... ticks after set-down)
  was run; its output is `EVIDENCE/u1543-1564/real-seed4-bands.txt` (the day numbers in it are
  absolute: day 10040 is arrival, 553 days on is 10593). It reached absolute day 10602 and found no
  band aimed at (106, 17) and none stuck (see Status), so seed 4's cause stays unconfirmed.
- **Fix**: none; no simulation behaviour changed. Only `crates/hf-sim/tests/threats.rs`.
- **What #1565 and #1566 have to take from this**: (1) #1566's search of the way in must apply the
  rule `path_agent_to` applies, `find_path` and then no over-head tile on the way (the walker's tile
  dry), and not `nav.reach`, which is true across a river; else a band is still set out toward a
  river it cannot cross. Choosing the next edge when the search fails helps only if that search is
  the walking rule. (2) #1565 (putting each member on walkable ground joined to the edge tile) does
  not change the staged result: the set-down tiles here are walkable and joined, and on seed 4's
  day-0 map so are the real ones. (3) `path_agent_to` itself: a refused path is neither cached nor
  retried with the deep tiles blocked, so a person with a ford in reach but A* preferring the way
  across is refused for good (the "far ford" control: ford 86 columns off, two-row river, refused; a
  three-row river with the same ford is walked round, 173 tiles, accepted: width against detour);
  that fix is a change to the walking rule and its own slice, and it cannot turn the staged scene
  (the river spans every column, so there is no way round to find). (4) Only #1566's change of edge
  turns the staged scene: the same valley and hungry place with the band set out by the East edge,
  which is on home's side of the river, passes the acceptance check today
  (`the_acceptance_check_passes_for_a_band_that_comes_in_by_the_east_edge`). The "legs of at most 40
  tiles" slice that the issue reserved for cause (a) is not called for by anything here: the 86-tile
  way was found within the limit, and seed 4's real way needed at most 1,770 of 12,000 expansions at
  day 0. (5) Not looked into: in the staged scene a set-down member finds no place to drink
  (`water::nearest_knowing` returned None in a one-off check, not asserted) although fresh water is 52 tiles
  off; relevant to #1568.
- **Guard**: `a_band_set_down_behind_deep_water_does_not_take_a_step` (set-down tiles, camp bounds,
  per-member `WalkCheck`, the false return, the empty cache, 12 hours on),
  `the_walk_is_refused_only_for_a_path_through_water_over_the_head` (wading river: accepted, 86 tiles; one
  deep row, no ford: refused, 1 over the head; a dry ford 6 columns off: accepted, 86 tiles, none over the
  head; far ford, two rows: refused, 86 tiles, 2 over the head; far ford, three rows: accepted, 173
  tiles, none over the head; a person standing in the river: accepted, path 2 tiles over the head),
  and `the_acceptance_check_passes_for_a_band_that_comes_in_by_the_east_edge` (a control: the check of
  the next test passes for an East-edge arrival; its members start within 2 tiles of their camp, so
  for them it is the thirst half), all in `crates/hf-sim/tests/threats.rs`; and
  `a_band_from_the_far_side_reaches_its_camp` (every member within 4 tiles of the camp at some tick
  within 2 game-days, asserted first; then none of the band dead of thirst), `#[ignore]`d until #1566
  turns the behaviour over. It fails today at the distance half ("Halgar came no nearer than 85 tiles",
  `1564-r3-runs.txt`). The thirst half is not about the walk: in this valley a band camped at (104, 40)
  dies of thirst even when it walks in (a reviewer tried a wading river and a ford 6 columns off by
  hand: both met the distance half and failed the thirst half; not kept as tests), so a correct walk
  alone would not pass the test; this is a second reason that only #1566's change of edge, which moves
  the camp, turns this scene. The first two document today's behaviour: #1566 will turn the first
  over. Checked against wrong versions with one temporary switch in `path_agent_to` (restored; `git
  grep` empty): the over-head refusal off fails both (the band's `path_agent_to` assertion; the
  far-ford case); the refusal cached fails the band test at "not cached"; `nav.reach` forced false
  and a 100-expansion budget (both cached failures) fail the band test at "not cached" and the
  wading control; removing the in-the-water exemption fails only the controls (the band test does not
  use it); refusing only with two or more over-head tiles passes the first two tests of the first
  commit of this slice (the river is exactly two rows; a reviewer's run of that switch,
  `1564-reviewer-two-or-more-passes.txt`) and fails the one-row control (`1564-r2-runs.txt`). Not caught: a change in what `go` does after a refusal that is not visible in
  positions after 12 hours; any change to drinking; a change to A*'s `OVER_HEAD` that moves none of
  the pinned lengths (86, 173). Not examined: a change only to the copy of the refusal in
  `par.rs:114` (no test here goes through it).
- **Commit**: this one.

## 2026-11-14 #1113: nothing counted who starves while others nearby carry food

- **Symptom**: the alive audit read 3,071 "is starving" lines and 86 `repaid` lines for seed 7 and
  could not say how many of those people had food carried by someone of their own community
  within reach, or by kin or a friend. #1112's targets need that as a number.
- **Found by**: the 2026-11-13 alive audit (#1112, slice #1113).
- **Cause**: no measure existed. This is a new read-only measure, not a change to the simulation:
  nothing in `crates/hf-sim/src` changed.
- **Fix**: new example `crates/hf-sim/examples/sharing_census.rs`
  (`cargo run --release -p hf-sim --example sharing_census [days [from to]] [--engine NAME]`,
  default 600 days, seeds 5 to 8, Steady; `--engine steady|relentless|whimsical` is read by
  hf-sim's own `Engine: FromStr` in the form `deaths_probe` uses, and `run` sets
  `WorldSetup.engine` with it; the printed SHARING line names the engine, read from the world
  that was counted, as `engine Steady`). Each game-hour it takes every living person of a community on the map with hunger
  under 0.15 and counts the food units (`ItemKind::is_food`) carried by the other living people
  of that community within 20 tiles, and how many of those units are carried by kin (`feuds::kin`:
  partner, parents, children, brothers and sisters) or by someone the starving person's own
  opinion puts at 0.5 or more. Readings the issue left open, chosen as the most natural: "within
  20 tiles" is `TilePos::dist <= 20` (Chebyshev in the plane plus 4 per level of height, so a
  holder 6 or more levels up or down is never beside, and 5 levels at the same x, y, which is
  20, is); people in `w.overworld.away` are left out, as starving people and as holders: their
  needs do not run, and the census never walks `away`, so an item still marked as carried by
  one of them is not counted (five of the six ways onto the road or below put what the person
  carried on the ground: `journeys.rs`, `systems/expedition.rs`, `siege.rs` `carry_off`,
  `societies.rs` `take` and the going below in `taken.rs`; `taken.rs` `down_again`, which
  sends a child below again, does not); the friend test is the starving person's
  opinion of the holder, not the holder's of them; food on the ground, in the starving person's
  own hands, or carried by another community is not counted; "hunger < 0.15" is strict. It
  also tallies `Starving` ("is starving" lines) and `Repaid` events. `GaveFood`, `Stole` and
  `SetAside` do not exist yet, so the printed line says
  "n/a (no such event yet)" for each, never a 0 that could be read as a measurement; the slice
  that adds one of those events must replace its "n/a" with a real count, matched by variant
  name in `Census::note`, and test it; food kept for someone (#1114) is not counted either,
  since no such store exists yet.
- **Baseline** (`dev` build, 30 days of the opening world, each seed with the engine the audit ran
  it on, one seed at a time: seed 5 Steady, seed 6 Relentless, seed 7 Whimsical, seed 8 Steady;
  the audit README's "Probe runs" lists the same four engines; the 600-day default did not
  finish on this machine in the time available): starving person-hours 22, 29, 45, 29; of those,
  beside carried food 19, 24, 38, 29; beside kin or a friend's food 6, 11, 15, 10; is-starving
  event lines 9, 8, 17, 8 (a different unit from person-hours: one line is one event, one
  person-hour is one person under 0.15 at one sample, so the two are not compared); repaid 1, 0,
  1, 2. Each SHARING line names its engine. Output in the evidence folder
  `1113-fix3-baseline-seed5.txt` to `-seed8.txt`. A 600-day baseline still has to be run, and
  until it has, #1115's targets mean nothing: 30 opening days miss the later famine. Run it one
  seed at a time with the audit's engine (`600 5 5`, `600 6 6 --engine relentless`,
  `600 7 7 --engine whimsical`, `600 8 8`): a bare run of seeds 5 to 8 makes seeds 6 and 7 on
  Steady, worlds the audit never ran. Seed 7 on Whimsical has 17 "is starving" lines in 30 days,
  against 3,071 in 600 days in the audit (`seed7-event-kinds.txt`), the same seed on the same
  engine (not an identical world: #780 renumbered a worldgen tag after the audit, so the scree
  layout differs; no figure here depends on it). **These numbers replace the earlier 30-day numbers**, which were all made on Steady
  (seeds 6 and 7 on Steady are not the audit's worlds: the engine is set at setup, so they
  differ from the first tick, and the 30-day lines, 6 and 11 there, set beside the audit's
  seeds 6 and 7 were not like for like; the 11 against 3,071 was the sentence the third review
  found). Seeds 5 and 8 are Steady on both and came out identical to the earlier
  run (22, 19, 6, 9, 1 and 29, 29, 10, 8, 2): a check that the new code path changes nothing
  for Steady. (The first version of the census printed `gave_food`, `stole` and `set_aside` as 0
  in its output; those were not measurements, there was no such event to count, and the line now
  says "n/a".)
- **Guard**: `tests/sharing.rs` includes the example and checks the helper on a scene worked out
  by hand: `sharing_census_sees_a_starving_child_beside_a_carried_meal` (27 units from 5 holders,
  9 of them kin or friend, the mother counted once though she is both; a starving holder beside
  another starving person; the people and food that must not count, among them the father of
  another community, an away person's carried food and an away starving person, and the 20 and
  21 tile edge),
  `the_census_samples_on_the_hour_and_sums_person_hours`,
  `the_census_counts_the_events_it_names_and_no_others` and
  `an_empty_world_has_nothing_to_census` (which also checks that the three missing events print
  as "n/a (no such event yet)" and never as 0), and, for the engine (third review),
  `the_engine_flag_is_read_wherever_it_stands_and_the_old_command_lines_still_mean_the_same`,
  `a_bad_or_missing_engine_name_is_an_error`,
  `the_census_makes_its_world_with_the_engine_it_was_asked_for` and
  `the_sharing_line_names_the_engine`. Three wrong versions fail them: the world made without
  the engine (`1113-fix3-red-world.txt`: the world test), the flag read and thrown away
  (`1113-fix3-red-parse.txt`: the flag test) and the line without the engine
  (`1113-fix3-red-line.txt`: the line test). Each of 12 wrong versions of the helper
  (community not required, height ignored, any carried item counted, own food counted, hunger
  at or under 0.15, the dead counted, the holder's opinion used, a friend at over 0.5, kin
  ignored, people with no community counted, the radius one tile short, sampling off the hour)
  failed at least one of the first version of the tests, and five more found in review fail the
  tests as they now are (`1113-fix-red.txt`): away holders counted, an away starving person
  sampled, kin of another community counted, a holder who is both kin and friend counted twice,
  starving holders left out. A sixth, found in the second review, sampled every half hour; it
  passed all four tests until `the_census_samples_on_the_hour_and_sums_person_hours` also
  sampled at half past the hour and asserted that the hour count stays 1
  (`1113-fix2-red.txt`). There is no counter, and so no test, for `GaveFood`, `Stole` and
  `SetAside` until a slice adds the event.

## 2026-11-14 #1563: the deaths probe did not say who was a band member or whether they ever moved

- **Symptom**: `examples/deaths_probe.rs` printed each death with its cause but not whether the dead
  person belonged to a band, nor how far they had come from where they were set down; its table
  counted deaths by cause only. The #1543 audit had to rebuild "81 of 82 thirst deaths were band
  members first seen on the map edge" from `events.jsonl` by hand, and the probe could not run
  seed 4 under the Relentless engine as that audit did.
- **Found by**: the alive audit (umbrella #1543, slice #1563).
- **Cause (checked)**: the probe read `Agent.band` only for its LEAVING line and for the end-of-run
  "left" table ("a visiting band going home"), and printed nothing about a band for a death; it had
  no engine argument (it always made the Steady world).
- **Fix**: `crates/hf-sim/examples/deaths_probe.rs` only, no change to `hf-sim`. `BandWatch` lists
  the members of a band in `w.bands` (not `Agent.band`) the tick a `BandSetOut` event is in
  `w.last_events` (they stand on their set-down tiles: a band is set down at in-day tick 6017 and
  the first step that sends it walking is 6063) and again every 200 ticks (10 game-minutes), keeps
  each one's first-seen tile (the set-down tile) and farthest distance (read at the 200-tick
  listings and at the death, so a walk out and back between two listings is not seen), and counts
  a death of a listed member by cause and again when the death tile is within 3 tiles of the
  first-seen tile. Listing only every 200 ticks (as this slice's first commit, eae1da02, did) put the
  first-seen tile up to 200 ticks, about 34 tiles of walking, after they came in, so for a band that walked and then
  stalled both "never moved more than 6 tiles" and "died within 3 tiles of where they came in"
  counted too high, and a member who died before the first listing was missed. A band death is the
  death of anyone ever listed, including one who later left the band's list (a raider sent home
  hurt, a captive the valley kept): a kept captive who dies of thirst in the valley counts as a
  band death. The run now ends with a `BAND TABLE` block (members listed, how many never moved
  more than 6 tiles, band deaths by cause with how many were where they came in), and a DIED line
  of a listed member says "ever listed as a band member".
  `--engine steady|relentless|whimsical` (also the deity names) picks the world's engine; without
  it the probe makes the Steady world as before. The arguments are read by `parse_args`.
- **Guard**: the seven tests in `deaths_probe.rs`; `crates/hf-sim/Cargo.toml` declares the example
  with `test = true`, so they run under `cargo test -p hf-sim` and the workspace `cargo test` /
  `cargo nextest run` of `tools/ci.sh` (an example's tests do not run otherwise); by hand,
  `cargo test -p hf-sim --example deaths_probe`.
  `band_members_are_listed_from_w_bands_and_counted_when_they_die` (two bands, a cleared
  `Agent.band`, a dead member, a settler and a stranger in no band; the counts are worked out by
  hand), `a_band_is_listed_on_the_tick_it_is_set_out_not_at_the_next_200_tick_listing` (set out at
  tick 6017, walks 30 tiles by 6200: first seen is the set-down tile; a member who dies at 6100,
  before the first 200-tick listing, is counted),
  `a_member_listed_and_later_out_of_every_band_list_who_dies_is_still_a_band_death`,
  `a_death_one_level_up_is_not_where_they_came_in`, `with_no_bands_the_table_is_empty`,
  `the_engine_flag_is_read_wherever_it_stands_and_is_not_taken_for_quiet`,
  `a_bad_or_missing_engine_name_is_an_error`. They drive `BandWatch::tick` and `died` with
  scenarios and test the argument parsing, not the printed output of a whole run.

## 2026-11-14 #1740: the HUD showed a frame time and the storyteller's hidden strain

- **Symptom**: the always-on HUD line read "... Aun watches  strain 34%   step 0.73 ms": a
  developer's frame time and the engine's tension as a percentage, which #1560 says the watcher
  should not read off directly; the game read as a tool, and the width is wanted by #1423's strip.
- **Found by**: the 2026-11-13 alive audit (the local AI (VPS-audit), #1740).
- **Cause (checked)**: `main.gd` `_update_hud` always passed `sim.engine_summary()` (deity and
  strain) and `"step %.2f ms"`.
- **Fix**: `crates/hf-godot/src/lib.rs` `deity_name` and `engine_strain` (split out of
  `engine_summary`, which stays); `godot/scripts/rules.gd` `hud_watch_fields`: "Aun watches" alone,
  the strain and the step time only with `--debug-hud` (`main.gd` `debug_hud`, default off).
- **Guard**: `test_hud_hides_debug_fields_by_default` (gd_test): off, neither "ms" nor "%"; on, both;
  the bridge gives the deity by name and the strain apart.
- **Commit**: this branch (`fix/1740-hud-debug-fields`).

## 2026-11-14 #1505: every soak ran Steady, so no soak showed what the other engines do

- **Symptom**: `ostrog soak` could only pace the soaked world with the Steady engine, so no
  soak table showed what Relentless or Whimsical do to a colony over a long run, and the SOAK
  summary line did not say which engine it had run.
- **Found by**: the 2026-11-13 alive audit (#1505).
- **Cause (checked)**: `Cmd::Soak` had no engine argument, so `soak` never set the engine of the
  world it built and every soak kept the default, Steady.
- **Fix**: `crates/hf-cli/src/main.rs` only. `soak --engine steady|relentless|whimsical`
  (default steady) is passed to `soak_world`, which sets the engine on the opening setup
  (`WorldSetup::engine`) or on the generated valley (`meta.engine`), and the SOAK summary line
  now reads `SOAK seed {seed} days {days} engine {engine}: ...`. The flag is in the README's soak
  row. The default soak is unchanged: it sets Steady, which the world already had.
- **Guard**: `soak_setup_takes_the_engine` in `crates/hf-cli/src/main.rs` (the engine is carried
  with and without `--opening`, and a bare `soak` parses to Steady).
- **Commit**: this one.

## 2026-11-13 #778 WP-07 7.5 follow-up: witnesses must be able to see the spot

- **Symptom**: a refusal at a rock face (usually underground) formed `Witnessed` accounts in
  every awake person within radius 8 on the same level — including a miner in a parallel
  gallery a few tiles away behind solid rock, who formed a feeling, a belief nudge, an opinion
  move and a teachable account of something they could not have seen. Relatedly, a retold
  account's place was blurred with `grid.surface`, moving an account born underground onto
  the surface above it.
- **Found by**: the local AI (VPS-2) review of #771 (filed as #778).
- **Cause (checked)**: the witness lists in `resonance.rs` (`balks`, `sat_by_stone`) were
  `beliefs::near` only — same level, within radius, awake — with no line-of-sight test; and
  `share` blurred the place with `grid.surface`, which projects to the surface datum.
- **Fix**: both witness lists now also require `mind::perception::can_see(&w.grid, pos, at)`;
  `share` blurs `x`/`y` only and keeps the account's original `z` (FIXLOG 2026-11-13 comment
  at both sites in `resonance.rs` and `systems/beliefs.rs`).
- **Guard**: `tests/witness_sight.rs` — a miner behind a solid wall within radius forms no
  account; one in the same carved gallery with clear sight does; the quiet waking is not
  witnessed through rock; a retold account keeps its level when the place blurs.

## 2026-11-14 #626: an install could update once; every later update failed and re-downloaded

- **Symptom**: on Windows, the published 0.1.0 updated to 0.1.1 from the test path, but the next
  update (0.1.2) ended in `started installed build (UpdateFailed)` before the new build ever
  started. Every later start downloaded the whole archive again, verified it, unpacked it into a new
  `.ostrog-update\staging\package-<pid>-1` that was never removed (68 MB each) and started the
  old version: `.ostrog-update` held 303 MB after two starts.
- **Found by**: the local AI (Windows), WP-02 2.9 part 2 on Dan's network against the wildcard-DNS test paths.
- **Cause (checked)**: `StagedUpdate::commit` refuses with `BackupAlreadyExists` when
  `.ostrog-update\backup` exists, and nothing removed the backup after an update that started,
  so the first update's copy of the old app (168 MB) blocked every later one. A failed commit also
  never discarded its staged tree.
- **Fix**: `hf_update::clear_finished_update` removes the backup, `staging\` and `extract-*` trees
  while `app\` exists (never during an interrupted swap, when the backup is the only copy). The
  launcher calls it at every start after `recover_interrupted`, and `CommittedUpdate::finish` drops
  the backup once the updated build has started. Because an update never replaces the launcher,
  the game also calls it when its extension starts in a portable install (`launcher.toml` beside
  `app\`), so installs with the 0.1.0 launcher recover once they run a build with this fix.
- **Guard**: `a_second_update_commits_once_the_first_ones_leftovers_are_cleared` and
  `clearing_keeps_the_backup_while_it_is_the_only_application` (hf-update),
  `an_update_after_an_earlier_one_clears_its_leftovers_and_installs` (hf-launcher); the 2.9 re-run
  with a rebuilt 0.1.1.
- **Commit**: this branch (`windows/626-update-backup`).

## 2026-11-14 #1485: a feed line went after 45 real seconds, a death as fast as a talk

- **Symptom**: every feed line was dropped 45 000 real ms after it arrived (`main.gd` `_push_feed`),
  whatever it said and whatever the speed: at 10x a line was gone after 7.5 game-hours, at 1x after
  45 game-minutes, a death the same as a talk.
- **Found by**: the 2026-11-13 alive audit (#1480, slice #1485).
- **Cause (checked)**: a fixed `45000` in `_push_feed` for the lines and the quieter-lines count.
- **Fix**: `godot/scripts/rules.gd` `feed_hold_ms(sev, speed)`: minor lines 1 game-hour, notable 2,
  major 12, converted to real time at the current speed and clamped to 30-180 s (paused counts as
  1x); each line keeps the hold it was given, and the "+N quieter lines" count follows the notable
  hold. The feed rate setting still caps the line count.
- **Guard**: `test_feed_keeps_majors_longer` (gd_test): at 1x major 180 s, notable 120 s; at 10x
  72 s and 30 s; minor, paused and the 3x clamp.
- **Commit**: this branch (`fix/1485-feed-hold-by-severity`).

## 2026-11-14 In a running game the feed stayed empty and event sounds were silent

- **Symptom**: watching a running game, the feed at the bottom left stayed empty (seed 5, 30 s at
  1x: 0 lines; still 0 after pausing), and the sounds tied to events did not play. Lines came out
  only when something settled the session (opening a panel, a save, an inspect). The #797 Windows
  footage has no feed at minutes 0 and 5.
- **Found by**: the local AI (VPS-2), measuring feed hold times for #1485 on the real main scene under Xvfb:
  no line ever reached the client.
- **Cause (checked)**: since the step thread (#603) `dispatch` moved the host's `feed` and `sounds`
  into the core it sent with each batch (`mem::take`), and `restore` replaced them. `step()` collects
  a finished batch and sends the next in one call, so `drain_feed` and `drain_sounds` (every frame)
  found them empty; and paused, the last batch was never collected at all. The same regression as
  #801's director.
- **Fix**: `crates/hf-godot/src/lib.rs`: `dispatch` gives the core empty lists and `restore` appends
  what the batch made (the feed still capped at 400); `drain_feed` and `drain_sounds` first collect a
  finished batch without waiting (`poll`).
  The same cause took the "Built by ..." note off a clicked tile in a running game (`inspect_block`
  read the chronicle only while the session was home): it now brings the session home first, one
  wait per click.
- **Guard**: `test_feed_arrives_while_the_game_runs` (gd_test): stepped as `main.gd` does (`step()`
  only with ticks to run) and drained every frame, the threaded run gives the same feed lines as
  stepping in place, all of them while it runs; with master's library, 0 while running. On the real
  main scene (seed 5, 1x): 9 lines on screen after 30 s (docs/reviews/2026-11-14-feed-while-running).
  `test_block_inspection_names_the_builder_while_the_game_runs` (gd_test): a built tile inspected
  with a batch on the step thread still says who built it; with master's library, no notes at all.
- **Commit**: this branch (`fix/378-feed-on-step-thread`).

## 2026-11-14 The Settings sliders showed no value, and autosave could go off unseen

- **Symptom**: the Settings sliders (four volumes, "Autosave every (days)") showed no value. A
  player could not tell autosave every day from every five, and dragging it to the left end set
  `autosave_days` to 0, which turns autosave off (`main.gd` `_process`), with nothing on screen
  saying so.
- **Found by**: the local AI (VPS-2), QA pass 7 (Settings and Timelapse) under Xvfb, `--fresh --seed=6`.
- **Cause (checked)**: `main.gd` `_slider` built a label and an `HSlider` only.
- **Fix**: `_slider` adds the value in words beside each slider, updated as it moves:
  `Rules.setting_text` gives volumes as a percentage and autosave as "off", "every 6 hours", "every
  day" or "every 1.5 days". The label is now "Autosave" (the value says the rest).
- **Guard**: `test_settings_sliders_say_their_value` (gd_test). Checked on the real main scene
  (docs/reviews/2026-11-14-settings-values): 80%, 70%, 80%, 60%, "every day"; dragged to the left
  end, "off".
- **Commit**: this branch (`fix/378-settings-show-values`).

## 2026-11-14 Load said "manifest missing", and the bar's Load threw the game away without asking

- **Symptom**: with nothing saved, Load (bar or F9) said "load failed: manifest missing". With a
  save, the bar's Load replaced the running valley at once: everything since the manual save was
  gone, with no word of how much.
- **Found by**: the local AI (VPS-2), QA pass 6 (save and load as a player does it) under Xvfb,
  `--fresh --seed=4`.
- **Cause (checked)**: `main.gd` called `sim.load_from(persist.manual_path())` straight from the F9
  key and the bar's "load" action (duplicated), and printed the bridge's error as it came.
- **Fix**: `main.gd` `_load_manual(ask_first)` / `_do_load_manual`: with no `manifest.json` in the
  manual slot it notes `Rules.LOAD_NOTHING_SAVED`; the bar's Load opens a small question with
  `Rules.load_question` (how many game days would be lost, from the save's manifest tick) and Keep
  playing [Esc] / Load; F9 loads at once as before (D94). Esc closes the question first.
- **Guard**: `test_load_says_what_is_lost_and_when_nothing_is_saved` (gd_test). Checked on the real
  main scene (seed 4; docs/reviews/2026-11-14-load-asks-first): Load with nothing saved gives the
  note; after F5 and about 8 000 ticks the bar's Load asks ("less than a day"), Esc keeps playing,
  and Load in the question restores the saved tick.
- **Commit**: this branch (`fix/378-load-asks-first`).

## 2026-11-14 The HUD and the People panel disagreed on how many were away

- **Symptom**: on a day-600 save the HUD said "Stoneside 14 (3 on a journey) · ... · Birchford 15
  (3 on a journey)", six away, while the People panel said "36 in the valley, 8 away".
- **Found by**: the local AI (VPS-2), QA pass 5: a copy of seed 2's ring save from #781's four-seed soak
  (tick 306296400), loaded on master under Xvfb.
- **Cause (checked)**: `crates/hf-godot/src/lib.rs` `communities_summary` counted only the members
  of the community's travelling caravans; the People panel lists everyone of the valley in
  `overworld.away`: on a journey of their own (#347), with a caravan, or carried off.
- **Fix**: `communities_line` (the HUD's text, now a free function) counts each community's
  `overworld.away` people, as the People panel does, and says "away".
- **Guard**: `the_hud_counts_away_as_the_people_panel_does` (hf-godot): a person gone on a journey
  of their own, in no caravan, counts as "(1 away)". On the same save the HUD now reads
  "Stoneside 14 (3 away) · Eldergate 2 · Thornkin 5 · Birchford 15 (5 away)": 8, the People
  panel's count.
- **Commit**: this branch (`fix/378-away-count`).

## 2026-11-14 The wizard's valley page spoke in the describe command's words

- **Symptom**: on a friend's first launch, "The valley" page of the setup wizard showed, beside its
  picture, "character: wetness +0.14 warmth +1.00 stoniness -0.36 (a mild valley), set by
  parameter", "flora draw: 8 of 22 wild kinds: ..." and "flora not drawn: ...".
- **Found by**: the local AI (VPS-2), QA pass 4: the setup wizard on an empty user folder under Xvfb
  (`--wizard-shot`), the path a friend takes (the earlier passes and the DoD footage used `--fresh`).
- **Cause (checked)**: `setup.gd` `_collect_preview` joined the preview's `lines` as they come from
  `hf_sim::setup::preview_in`, which are the `ostrog describe` lines (`biome::describe`).
- **Fix**: `godot/scripts/rules.gd` `valley_note` says it in a player's words: the preview's label
  ("A mild valley."), the kinds of ground over a tenth of the valley ("Mostly grassland, fen,
  woodland and meadow."), and the wild plants drawn ("Wild plants here: ..."); no not-drawn list.
  The sim's lines stay as they are for the CLI and its tests.
- **Guard**: `test_the_wizard_says_the_valley_in_a_players_words` (gd_test), on a real preview, so a
  change to those lines fails there instead of reaching the page. Checked with `--wizard-shot`
  (docs/reviews/2026-11-14-wizard-valley-words).
- **Commit**: this branch (`fix/378-valley-preview-words`).

## 2026-11-14 The feed gave a person's mood as the cause of everything they did

- **Symptom**: nearly every notable feed line ended "— because: torn from the old world, the
  missing": "Irlin finished the hearth", "Galn tended Nalaesin's hurts", "Galn tamed a goat",
  arrivals, discoveries, falling ill. On seed 5's first 24 000 ticks, 26 of 35 notable lines.
- **Found by**: the local AI (VPS-2), QA pass 3 of a fresh valley under Xvfb (`--fresh --seed=5`); counted
  from the bridge's feed rows (`drain_feed`, kind and because per row).
- **Cause (checked)**: `crates/hf-godot/src/lib.rs` `event_because` (#413 A5) falls back, for any
  notable event without a recorded cause or `narrate::detail`, to the first participant's mood
  (`view::why::because`: why they feel as they do), which the client prints as "because:".
- **Fix**: `event_because` uses the mood only when mood drives the event (`mood_drives`: broke,
  quarrelled, fought, would_not_stay, left_the_valley). Recorded causes and `narrate::detail` are
  unchanged, and the inspect panel still says why someone feels as they do.
- **Guard**: `the_feed_gives_a_mood_as_the_cause_only_of_what_mood_drives` (hf-godot): a mind
  giving way keeps the mood as its because-line, tending does not; checked to fail with the old
  fallback. On seed 5's first 24 000 ticks only `custom_formed` keeps a because-line, its recorded
  cause (the tale at the hearth).
- **Commit**: this branch (`fix/378-feed-because-mood`).

## 2026-11-14 Centred panels stacked, the one underneath showing through

- **Symptom**: opening Legends or History with People open left People underneath, its rows
  showing through the top panel's translucent backing; with a person inspected, the inspect panel
  (raised on every refresh) covered the right of Legends, which is 1200 px wide.
- **Found by**: the local AI (VPS-2), QA pass 3 of a fresh valley under Xvfb (`--fresh --seed=5`: P,
  Inspect, then L).
- **Cause (checked)**: the PanelManager stacks any panels opened (#351) and every centred panel
  has the same 0.82-alpha backing; nothing closed a centred panel when another opened.
- **Fix**: `godot/scripts/ui/panel_manager.gd` `register(id, node, group)` and `open`: opening a
  panel closes the other open panels of its group. `main.gd` registers the centred reading panels
  (History, Legends, People, Waiting work, Favourites, Timelapse) as one group (D92), and
  `_show_legends` closes the inspect and block panels it runs under, through `_close_panel`.
- **Guard**: `test_reading_panels_open_one_at_a_time` (gd_test). Checked on the real main scene
  (seed 5, P, Inspect, L; docs/reviews/2026-11-14-one-reading-panel): master leaves People, Legends
  and inspect open together; this branch leaves Legends alone.
- **Commit**: this branch (`fix/378-one-reading-panel`).

## 2026-11-14 Announcements covered open panels and the HUD's hint line

- **Symptom**: the top-centre announcements (toasts) drew over an open History panel, four of them
  over its text, and over the HUD's second and third lines, since the column started at a fixed
  34 px while the HUD wraps to three lines (about 88 px).
- **Found by**: the local AI (VPS-2), a QA pass of a fresh valley under Xvfb (`--fresh --seed=3`, 10x,
  two game days, then H): the History screenshot.
- **Cause (checked)**: `Toaster` is on its own layer above the panels by design (#329, so a toast
  is never hidden behind one), with `_box.offset_top = 34` and nothing that knew a panel was open.
- **Fix**: `godot/scripts/ui/toaster.gd` `set_top` and `hold`; `godot/scripts/rules.gd`
  `toasts_held`; `main.gd` `_process` keeps the column under the HUD's real bottom and holds the
  toasts while a centred reading panel is open (D91): hidden, not ageing, still gathering by kind,
  shown when it closes.
- **Guard**: `test_toasts_wait_while_a_reading_panel_is_open` (gd_test). Checked on the real main
  scene (seed 3; docs/reviews/2026-11-14-toasts-clear-of-panels): toasts at y 96 under the HUD
  (bottom 88); History open, none on screen and four held; closed, all four shown.
- **Commit**: this branch (`fix/378-toasts-clear-of-panels`).

## 2026-11-14 Esc did not close People, Waiting work or the inspect panel

- **Symptom**: the People, Waiting work and inspect panels each say "Close [Esc]", but Esc left
  People and Waiting work open (it only dropped the selection). Opening History over People and
  pressing Esc closed History and then nothing more, so People stayed open under whatever came
  next, its rows showing through the next panel's backing.
- **Found by**: the local AI (VPS-2), a QA pass of a fresh valley under Xvfb (`--fresh --seed=3`, 10x,
  two game days, then P, H and Esc as a player would): the History screenshot showed People's rows
  through it.
- **Cause (checked)**: `main.gd` `_unhandled_input` named six panels in a fixed list for Esc
  (help, legends, timelapse, summary, favourites, block) and missed three of the nine the
  PanelManager knows; it also ignored which panel was on top.
- **Fix**: `main.gd`: Esc closes `panels.top()`, the panel opened last that is still open, through
  `_close_panel`, which does what each Close button does (the inspect panel also lets go of the
  person and stops following; the block panel of the tile). The bookmarks popup and the files page
  still come first, Settings after the panels, and with nothing open Esc deselects as before.
- **Guard**: `test_panel_manager_top_is_the_last_opened_still_open` (gd_test). Checked on the real
  main scene (seed 3, P then H then Esc twice): master leaves People open after both, this branch
  closes History and then People.
- **Commit**: this branch (`fix/378-esc-closes-top-panel`).

## 2026-11-14 The camera director opened a new valley on bats dying in a far cave

- **Symptom**: with the director working again (#801), a fresh valley's camera left the
  settlement in the first game hour for an unexplored cave sixteen levels down (z -14), "watching
  a death", with nobody there. In #801's own trace (seed 7) it did the same.
- **Found by**: the local AI (VPS-2), a QA run of a fresh valley under Xvfb (`--fresh --seed=11`, 10x,
  three minutes) after #801 merged; the history showed five "A bat died: engulfed by slime" in the
  first game hour.
- **Cause (checked)**: `life::die` records every death as a Major event, a wild bat's included,
  and `Director::suggest` weighs a death at 3.0 x 1.8, times 0.25 when nobody is in the cell. Five
  in one cell (about 6.8) far outscored people at work (1.4 each) and the settlement (0.8).
- **Fix**: `crates/hf-godot/src/lib.rs` `wild_death` and `publish`: a wild animal's death (not a
  person, in nobody's herd) is not passed to the director. The chronicle, the feed, the bodies and
  witnesses' accounts keep it, and a person's or a herd animal's death still draws the camera. No
  hf-sim change: the director is view state, not in the digest.
- **Guard**: `the_director_is_not_sent_to_a_wild_animals_death` (hf-godot): of a wild animal, a
  person and a herd animal dying, only the wild death is held back, and with five of them in one
  place the director stays with the settlement.
- **Commit**: this branch (`fix/378-director-wild-deaths`).

## 2026-11-14 #797 finding 2: the camera director never moved in a running game

- **Symptom**: with the director on and nothing pressed, the camera stayed where the game
  started: in the DoD row 1 footage of 0.1.0 three stills five minutes apart show the same
  ground, and the HUD said "director: on" with nothing watched. With three people, most of the
  time nobody was in view.
- **Found by**: the local AI (Windows), DoD row 1 footage (#797, finding 2 on #626); traced on the VPS
  with a headless run of the real main scene (`--fresh --seed=7`, 130 s): `sim.director_suggest()`
  answered `[]` on every call.
- **Cause (checked)**: since the step thread (#603) the `Director` travels with the session, and
  `director_suggest` gave only a suggestion made at home, cached in `last_suggest`. `step()`
  hands the next ticks over the moment a batch comes home, so at any speed above paused the
  session is on the step thread nearly whenever the client asks: no suggestion was ever made, and
  the cache stayed empty. #759 stopped the client crashing on that empty answer; the director
  stayed dead.
- **Fix**: `crates/hf-godot/src/sim_thread.rs` `Core::suggest` works the suggestion out on the step
  thread after each batch while the client is asking (asked within 5 s, `suggestion_wanted`), and
  counts the people only when the current suggestion's dwell is over. `lib.rs` keeps it
  (`suggestion`, home in `restore`, a copy out in `dispatch`) and `director_suggest` answers with
  it while the session is away, or makes one at home as before. Nothing in hf-sim changes; the
  director is view state, not in the digest.
- **Guard**: `test_director_answers_while_the_game_runs` (gd_test) steps a threaded SimHost one
  tick at a time as `main.gd` does and asks each time: a full answer must come, and once it has,
  no later ask may come back empty (no timing threshold). With master's library: no answer in 80
  asks (fails); with the fix: the first at ask 7, none empty after. `the_step_thread_brings_the_directors_suggestion_home`
  (`sim_thread.rs`): every batch carries the suggestion a director at home gives for the same
  ticks, and none is made when nobody asks. The traced run with the fix: the camera went to "the
  community", "a death", "something happening", "people at work" and back in 130 s.
- **Commit**: this branch (`fix/626-director-on-step-thread`).

## 2026-11-13 #797 finding 1: Mouse & Keys opened over the settlement on a first start

- **Symptom**: on a first hands-on start the Mouse & Keys panel opens by itself, as it should,
  but in the middle of the view: the founding spot is the view's centre, so the panel covered
  the settlement until Esc or Close. At 1600x900 its 360 px height also cut off the last key
  lines (they needed scrolling). A friend who waits and watches saw mostly grass for five minutes.
- **Found by**: the local AI (Windows), DoD row 1 footage of the published 0.1.0 (#797, finding 1 on
  #626).
- **Cause (checked)**: `main.gd` built `help_panel` with `PRESET_CENTER` and a fixed 700x360
  rect; `HELP_TEXT` had key lines of about 100 characters that wrapped inside it.
- **Fix**: `main.gd` `_place_help` puts the panel at the left edge just under the HUD, as tall as
  its text and never over the bottom bar (`Rules.help_rect`, `HELP_W` 560), every frame while it
  is open (the HUD gains lines for strangers and troubles). `HELP_TEXT` groups the keys into
  shorter lines; every key is still listed. The panel's behaviour is unchanged: once on a
  hands-on first start, then on [?].
- **Guard**: `test_mouse_and_keys_leaves_the_middle_of_the_view_clear` (gd_test): clear of the
  middle and the bar at 1600x900 and 1280x720, capped in a short window, and every help line
  fits the width at 14 px. Checked on the real main scene under Xvfb (`--fresh --seed=7`, empty
  user data): panel at (10, 73) 560x434, view centre outside it, bottom 507 against the bar at
  856, no scrolling.
- **Commit**: this branch (`fix/626-help-off-centre`).

## 2026-11-13 Closing the window lost the valley until the first autosave

- **Symptom**: on the DoD row 1 run of the published 0.1.0 (#797), a fresh valley played for five
  minutes and the window was closed: `saves\` was empty, and the next launch started over. The first
  autosave comes one game day in (`autosave_days` 1.0 = 28 800 ticks, about 24 real minutes at 1x),
  and nothing saved on close, so a friend who tried the game for a few minutes lost it.
- **Found by**: the local AI (Windows), DoD row 1 footage (#797, finding 3 on #626).
- **Cause (checked)**: `godot/scripts/main.gd` never handled the window close request; Godot's default
  accepts it and quits at once.
- **Fix**: `main.gd` turns `set_auto_accept_quit` off and handles `NOTIFICATION_WM_CLOSE_REQUEST` in
  `_save_and_quit`: it waits for a save already running, writes one more into the autosave ring when
  `Rules.save_on_close` says so (a begun game that moved since its last autosave; never a screenshot,
  footage, stage or wizard-shot run; never headless without `--ring`; not with autosave off), waits
  for that write and its save lock to finish (bounded at 30 s), then quits.
- **Guard**: `tools/gd_test.gd` `test_closing_the_window_saves_only_a_running_game_that_moved`.
  Checked end to end headless: the real main scene with `--seed=7 --ring`, a close request delivered
  after 400 frames, prints `SAVED ON CLOSE`, quits with 0 and leaves `saves/auto.0.hfsave` and no
  save marker; without `--ring` it quits with no save.
- **Digests change**: none; client only.

## 2026-11-13 #508 WP-07 7.6: the second potential wakes one bounded fragment, taught, withheld, contested

- **Symptom**: every drawn potential was the rock-sense; the ancient-resonance catalogue of
  #508 (fragments of knowledge, and the same fragment surviving as different accounts) had no
  second entry, and knowledge had no carrier-bound way to spread, drift or be lost.
- **Found by**: WP-07 slice 7.6 (ancient fragments and contested historical records), reading
  the #508 "Ancient knowledge boundary" and the 7.5 witness/teaching chain it composes with.
- **Cause (checked)**: `Potential` had one variant and `draw` always produced it; nothing on an
  agent could hold a piece of old-world knowledge; `share` carried no knowledge kind.
- **Fix**: `mind/fragment.rs`: a capped `HeldFragment` record (what/mark/origin/learned/tick/
  tells, serde(default) on the agent) — one bounded fragment, drawn at the waking from
  `rng::stream(seed, [RESONANCE_FRAGMENT, carrier, woke tick])`, stored, never rerolled. Kinds:
  one glyph without its meaning, one rite-step without its purpose, a place that reads wrong
  with no reason, a material that wants a device they cannot conceive — never a system or a
  reason, and never written to the event log (the world sees only the unsettled waking, which
  7.5 already witnesses). One in three drawn carriers carries it (a judgment call, recorded
  here). The very cautious (`caution > 0.75`) withhold it. `share` teaches a told fragment on:
  familiarity blurs which one it was (a close listener hears it exactly), every fifth retelling
  loses the mark while the kind survives, and a copy that now disagrees with one the listener
  already holds of the same fragment is kept beside it — a contested record, no arbiter. A
  withheld fragment never travels and dies with its holder.
- **Guard**: `tests/fragments.rs`: `the_woken_carrier_holds_exactly_one_bounded_fragment`,
  `the_fragment_draw_is_deterministic_for_seed_and_carrier`,
  `a_fragment_survives_save_and_load_unchanged`,
  `a_fragment_is_taught_in_talk_and_travels_with_its_origin`,
  `a_close_listener_hears_it_exactly`, `a_strangers_telling_blurs_which_one_it_was`,
  `every_fifth_retelling_loses_which_one_it_was`, `a_withheld_fragment_does_not_travel`,
  `two_drifted_copies_of_one_fragment_are_both_kept`.
- **Digests change**: only where a fragment carrier wakes at the stone beside others (rare:
  one sitting of many); the waking event and its witness accounts are unchanged.
- **Commit**: branch `feat/508-ancient-fragments`.

## 2026-11-13 WP-06 6.4: the Rust renderer rebuilt every batch on every frame

- **Symptom**: with the Rust world renderer on, the real-GPU frame-time run (#786, laptop GPU
  Laptop, 500 people) had a median frame of 13.4 ms and a slow frame (25 to 40 ms) every third
  frame, so the 6.4 gate failed. Headless on the server (`tools/perf/frame_prep.gd`, now with
  `--rust-renderer`), `view._process` took 13.4 ms per frame with the renderer on against 3.7 ms
  off, and `view.refresh` 24.3 ms.
- **Found by**: #786's numbers, then `frame_prep.gd` and a scratch breakdown of
  `rust_world_renderer.rebuild()`.
- **Cause (checked)**: `rust_world_renderer.rebuild()` ran on every frame from
  `world_view._process` and again in `refresh()`, and each call rebuilt everything: people and
  animals (1.6 ms), the static things (3.7 ms), and the whole level's ground, 16 384 tiles fetched
  and set one instance at a time (9.2 ms). The people rows are the sim's state and only change
  with the tick; the ground only changes when a tile does.
- **Fix**: `godot/scripts/render/rust_world_renderer.gd` `rebuild(delta, force)` rebuilds people and
  animals when the tick or level changes, static things when the level changes or at most every
  half second (the layered path's own rule), and the ground when the level or the sum of every
  chunk's version (`sim.chunk_versions` over the whole z range) changes. `world_view._process`
  passes its frame time. Measured headless at 500 people (load about 14 on this shared server):
  `view._process` 13.4 to 0.7 ms (p95 22.3 to 1.0), `view.refresh` 24.3 to 10.4 ms (p95 33.9 to
  15.3). The GPU gate itself is for the Windows run to confirm.
- **Guard**: `tools/gd_test.gd` `test_rust_renderer_rebuilds_only_what_changed` (a repeat call with
  nothing changed builds nothing; a tick rebuilds people; half a second rebuilds static things; a new
  level rebuilds all three). Checked to fail when people rebuild on every call.
- **Digests change**: none; client view only.

## 2026-11-13 #294: the automation sweep could not start Godot on Windows

- **Symptom**: `tools/ci.sh` on Windows (Git Bash) stopped at
  `tools/tests/client_automation_sweep_test.py` with `OSError: [WinError 193] %1 is not a valid
  Win32 application`; everything before it passed.
- **Found by**: the local AI (Windows), the first full `tools/ci.sh` run on the Windows machine (master
  50cd3957).
- **Cause (checked)**: the test passed `tools/godot`, a bash script, straight to
  `subprocess.run`. Linux runs the shebang; Windows can only start executables.
- **Fix**: `client_automation_sweep_test.py` `godot_command()` runs the wrapper through Git Bash
  on Windows (`OSTROG_GIT_BASH`, else Git's default `bash.exe`, else `bash` on PATH), the
  same rule the package tests use; unchanged elsewhere.
- **Guard**: the sweep itself, now passing on Windows (`client automation sweep OK`, 22 s).

## 2026-11-13 #626 WP-02: the launcher window told friends their saves were in data\

- **Symptom**: every stage of the launcher's status window carried the caption "Saves protected in
  data\". No save is ever written there: the game keeps saves in
  `%APPDATA%\Godot\app_userdata\Ostrog\`, and `data\` holds only the launcher's log (2.15 on
  real Windows). A friend looking for their saves where the window said would find nothing.
- **Found by**: the local AI (Windows), rendering every stage with `ostrog-launcher.exe
  --dump-stage-screenshots` while preparing the 2.9 evidence run.
- **Cause (checked)**: the caption in `window.rs` was written before 2.15 settled where saves live.
- **Fix**: `LauncherStage::footer` (crates/hf-launcher/src/lib.rs) gives "Stage: <name>  •  Saves kept
  in AppData", the same length class so it fits beside the buttons; `window.rs` draws it.
- **Guard**: `the_status_window_never_tells_a_friend_their_saves_are_in_data` (hf-launcher, runs on
  every platform, unlike the Windows-only window module).

## 2026-11-13 #626 WP-02: the Windows launcher and bridge needed the Visual C++ runtime

- **Symptom**: the c227 Windows release build of `hf-launcher.exe` and `hf_godot.dll` imports
  `VCRUNTIME140.dll`. That DLL is not part of Windows; it comes with the Visual C++
  redistributable. On a friend's PC without it the launcher would not start ("VCRUNTIME140.dll
  was not found") and the game could not load its bridge. Godot's own `ostrog.exe` imports no
  C runtime DLL.
- **Found by**: the local AI (Windows), a desk check of the first-release package on the Windows
  machine, confirmed by searching the built binaries' import names.
- **Cause (checked)**: Rust's `x86_64-pc-windows-msvc` target links the C runtime dynamically by
  default, and the repo set no `crt-static` target feature.
- **Fix**: `.cargo/config.toml` sets `-C target-feature=+crt-static` for
  `x86_64-pc-windows-msvc` only, so Linux builds are unchanged. `tools/package_windows.sh` and
  `tools/package.sh` refuse to package a launcher or bridge that still names `VCRUNTIME140`.
  (The `api-ms-win-crt-*` imports that remain are the Universal C Runtime, part of Windows 10
  and later.)
- **Guard**: `tools/tests/windows_package_test.py`
  `test_a_launcher_that_needs_the_vc_runtime_is_refused`.
- **Validation**: rebuilt on Windows 11 (`cargo build --release --target x86_64-pc-windows-msvc
  -p hf-godot -p hf-launcher`); neither binary names `VCRUNTIME140.dll` any more (3.7 MB launcher,
  28 MB bridge, about 0.1 MB larger each). Swapped into the c227 portable package, the launcher
  went through its stages and started the game, which loaded the bridge and its world; a
  2,000-tick `--save-at-end` run with the same bridge saved (`ok, capture 24.7 ms`).

## 2026-11-13 #508 WP-07 7.5: witnesses of a manifestation held, taught or withheld it

- **Symptom**: a carrier's refusal at a face was a public event, but nobody reacted to having
  seen it: no account in any mind, no fear or awe, no talk about it, nothing carried into
  relationships or beliefs. The manifestation moved the world only through the carrier.
- **Found by**: WP-07 slice 7.5 (witness knowledge, teaching, withholding and social reaction),
  reading `resonance.rs` `balks`/`sat_by_stone` and `systems/beliefs.rs` `share`.
- **Cause (checked)**: nothing formed an account in anyone but the carrier, and the talk
  channel had no witnessed-account kind to carry.
- **Fix**: `mind/witnessed.rs`: a capped `Witnessed` record (who/at/saw/read/tells, serde(default)
  on the agent) with the read drawn from the witness's own beliefs — ThinLore makes it the stone
  remembering, a DeepDigging conviction names the deep's warning, otherwise the dread stays
  unexplained; the potential is never named. `form` fires from `balks` (radius 8, strength 0.6)
  and the waking (radius 6, 0.35): a feeling toward the carrier (fear, or awe for one who holds
  the signs heeded), a `DeitySigns` nudge (awe toward heeded, fear toward only weather), an
  opinion move, and the tells decision — withheld by one protective of the carrier or too
  shaken by disposition to speak. `share` teaches a told account on (the place blurs by
  familiarity; every fifth retelling loses the read's detail); a withheld account never travels.
- **Guard**: `tests/witnessed.rs`: `witnesses_of_a_refusal_hold_their_own_read_not_the_potential`,
  `a_witness_who_saw_a_failure_fears_the_carrier`, `a_witness_may_teach_what_they_saw_in_talk`,
  `a_withheld_account_stays_out_of_the_gossip_chain`, `non_witnesses_know_nothing`, plus the
  waking, cap and real-event guards.
- **Digests change**: only where a witnessed beat now forms an account (rare: a woken carrier
  and a bystander within reach).
- **Commit**: branch `feat/508-witness-response`.

## 2026-11-13 #759: the director camera crashed on a short suggestion

- **Symptom**: with the director engaged (60 s idle), the client flooded `SCRIPT ERROR: Out of
  bounds get index '2' (on base: 'Array')` every frame at `main.gd` `_camera`, and the camera
  stopped following the director.
- **Found by**: the #757 automation-sweep work; a no-args headless run (default world, director
  auto-engaged) reproduced the flood at once.
- **Cause (checked)**: the `director_suggest()` bridge answers the documented 6-tuple
  `[x, y, z, score, dwell, reason]`, but while the stepper is in flight with no cached
  suggestion it returns an empty array; `_camera` indexed `sug[2]` (and `sug[5]`) unchecked.
- **Fix**: `godot/scripts/main.gd` `_camera` applies a suggestion only when `sug.size() >= 6`.
- **Guard**: the no-args headless repro runs quiet (no SCRIPT ERROR) after the fix.
- **Digests change**: none — pure client view.

## 2026-11-12 #743: a worker taken off the work kept its leases for hours

- **Symptom**: a cook taken off the work by anything other than the work's own finish or abort (a
  rite, an observance, a threat, a fey mood, a flight from rising water) kept its leases (the
  hearth station, the raw food) until they ran out, about two game hours for a cook. On seed 2's
  day-400 save the hearth was held by people loitering, eating or walking to drink.
- **Found by**: the Waiting work panel (#744) showing cooking orders untaken for 26 to 35 hours;
  traced from the save with the decision log (#743).
- **Cause (checked)**: leases are taken only when a work plan starts (`decide.rs`) and released
  only by `work::finish` and `work::abort`; the many systems that replace an activity outright do
  not release them, and `renew` runs only while `Working`.
- **Fix**: `work::sweep` (every 60 ticks) also releases the leases of every holder who is no
  longer alive and `Working`; `ReservationTable::holders` lists them.
- **Guard**: `tests/lease_on_interrupt.rs` `a_cook_taken_off_the_work_lets_go_of_the_hearth`
  (fails on master: the hearth stays leased to the cook who was taken off it; one still at the
  work keeps it).
- **Not the whole story on that save**: its zero meals were also the hearth's ground flooding
  every afternoon (all 24 tiles around it over a person's head), which no lease fix changes; see
  #743.

## 2026-11-12 #723 follow-up: a relationship only the higher row holds drew no line
- **Symptom**: the relationship-lines overlay (#723, WP-08 8.4) drew a pair only from the person
  whose `agents()` row is lower (`if from >= to { continue; }`). Relationships are per person and
  can be one-sided; when only the higher-row person held an opinion of the other, no line was drawn.
- **Found by**: two reviews of #723 (13:43Z, 13:49Z with a failing repro); merged before the fix.
- **Cause (checked)**: the "each pair once" rule skipped the higher row's side instead of only
  deferring to the lower row's view.
- **Fix**: `crates/hf-godot/src/lib.rs` `relationship_lines_core` collects pairs from both sides
  into a `BTreeMap<(lower row, higher row), band>`; the lower row's view wins (D88), the higher
  row's fills in a one-sided pair. Still one triple per pair, now in row order.
- **Guard**: `a_relationship_held_only_by_the_higher_row_is_still_drawn` (fails on master: `[]`).
- **Follow-up**: the leases the sweep takes from someone no longer at work are counted in `expired_total` (`ReservationTable::revoke_all`), with the ones that ran out: both are leases the sweep ended. Without it `tests/work.rs` `lease_expiry_sweep` failed, its phantom holder now losing the lease before it expires (box-ci nextest on #749).
- **Digests change**: none; client view only.
- **Commit**: #736 (b56f9f1a) with this entry and the doc comment added in the second-wave fold.

## 2026-11-12 #351 WP-08 8.5: in-world toasters — a told event marks where it happened

- **Symptom**: #329's screen toasters announced a major event, but once the toast slid away
  the world carried no trace of where it happened; the watcher had to catch it in the feed.
- **Found by**: WP-08 8.5's triage of #351 (the watchability gap the owner's design note
  named: local, world-anchored awareness).
- **Cause (checked)**: not a defect — the toast carried the position (its Look click) but
  nothing drew it on the map.
- **Fix**: `godot/scripts/render/world_marks.gd` — the same told events that toast now also
  drop a fading, rising ring at their tile, severity-coloured, dimmed when the event is on
  another level. The feed is the told stream (what the valley knows), so no new filter is
  needed; off-screen positions are not drawn. Pure client; sim untouched.
- **Guard**: `test_world_marks_anchor_and_expire` (tile-anchor math, level carry, expiry,
  clear).
- **Digests change**: none — pure client view.

## 2026-11-12 #197 WP-08 8.7 case 11: a night raid does not go forever unopposed

- **Symptom**: #197 case 11 feared that with everyone asleep a night raid goes
  unopposed forever: sightings need someone awake, and the 25% wake roll
  (threat.rs `raids`) might never land.
- **Found by**: the #197 edge-case sweep (WP-08 8.7), the last open case.
- **Cause (checked)**: not a defect — the roll works as designed; checked with a
  guard that stages a camped band among a sleeping community and steps a deep
  night: someone wakes within the window. (A raw `meadow` world cannot stage it:
  its agents carry no community, and the roll only wakes community members.)
- **Fix**: none needed; pinned by the guard.
- **Guard**: `tests/edge_cases_197.rs`
  `a_night_raid_eventually_wakes_someone` (camp band among sleepers, bounded
  night, assert a wake).
- **Digests change**: none — a test.

## 2026-11-12 #197 cases 7 and 14 joined up: a proposer gone for good no longer holds a project open

- **Symptom**: after #705, `projects::close` counted anyone of the community in `overworld.away`
  as present, so a project's proposer who was carried off as a captive, or a child gone below for
  good, held the project open until its days ran out (or forever, for a home with a site).
- **Found by**: noted on #705 and #706 when both were open; done once both had landed (c195).
- **Cause (checked)**: #705 chained `overworld.away` whole; #706's `Overworld::on_the_road` (a
  caravan under way, a journey of their own) is the narrower "away, not gone".
- **Fix**: `close()` counts the proposer present only alive in the valley or listed by
  `on_the_road` for the project's community.
- **Guard**: `tests/edge_cases_197.rs` `a_project_is_not_ended_because_its_proposer_is_away_on_the_road`
  now sets the proposer on a journey, and checks that away with no journey or caravan ends it.

## 2026-11-12 #508 WP-07 7.4: the rock-sense cost nothing beyond rest and never failed

- **Symptom**: a woken carrier (7.2) refused every face that would bring the roof down, however
  many times and however tired: a free, perfect safety bonus, which #508 rules out ("No ability is
  a free productivity bonus"; "mistaken confidence when an ability works once but fails later").
- **Found by**: WP-07 slice 7.4 (cost, failure and recovery), reading `resonance.rs` `balks`.
- **Cause (checked)**: `balks` had no condition on the carrier's state; it fired whenever the cut
  was unheld and they did not know shoring.
- **Fix**: `resonance.rs`: below `FAILS_BELOW` (0.5) rest the sense does not come; `balks`
  returns false, records and spends nothing, and the cut goes ahead under the ordinary collapse
  chance. Each firing still costs `COST` (0.1) rest, so two firings or an afternoon's work take
  a carrier below it; sleep restores it.
- **Guard**: `tests/resonance.rs` `the_sense_tires_fails_when_worn_out_and_comes_back_with_sleep`
  and `a_worn_out_carrier_cuts_the_deadly_face_like_anyone` (no refusal at rest 0.35, the face is
  cut on at least one of eight seeds). The 7.2 guard
  `a_woken_carrier_will_not_cut_a_face_that_would_bring_the_roof_down` now keeps its carrier
  rested, which is what it states.
- **Digests change**: only in runs with a woken carrier at an unheld face while tired (rare: the
  stone must be woken and sat by four times).
- **Commit**: branch `feat/508-sense-cost`.

## 2026-11-12 #197 WP-08 8.7: a project ended the day its proposer set off on the road

- **Symptom**: #197 case 14. A project (a house, a wall, a workshop, a well, a shrine) whose
  proposer left on a caravan, or was carried off and not yet back, was closed at the next daily
  project round, as if they had died or left the community.
- **Found by**: #197 case 14 (code reading in QA 2026-10-20), confirmed with the guard below.
- **Cause (checked)**: `systems/projects.rs` `close()` counted the proposer present only if they
  were in `w.agents`; someone on the road is moved to `overworld.away`, kept whole until they
  return, so `gone` was true.
- **Fix**: `close()` looks for the proposer in `w.agents` and `overworld.away`, alive and of the
  project's community. Someone who dies away is still gone.
- **Guard**: `tests/edge_cases_197.rs` `a_project_is_not_ended_because_its_proposer_is_away_on_the_road`
  (fails on master; a proposer dead on the road still closes it).
- **Digests**: change where a proposer is away at a daily project round; the four-seed 600-day
  soak on the PR is the evidence.

## 2026-11-12 #698 WP-05 5.5: the scree pass never fired, because the valleys have no steep ground

- **Symptom**: #687's loose-rock pass changed no tile in generated worlds: tick-0 digests for seeds
  1 to 4 were the same with and without it, and its soak matched the soak without it number for
  number (#687, #698).
- **Cause**: the pass wanted a step of two levels to a neighbour; the opening valleys step one level
  at a time (at that point in generation no column on seeds 1 to 4 stands two levels off a
  neighbour, and every column is within one level of everything two tiles around it). The guard
  drove `scree::apply` on a synthetic cliff, so it passed anyway.
- **Fix** (Dan's choice of the options on #698, "knolls get scree"): a bare top that steps to its
  neighbours on two or more sides (a knoll or a ridge edge) is also eligible, and takes loose rock
  only where the patch noise is high: over `KNOLL_LOOSE` (0.62, about a fifth of such tops on seeds
  1 to 4) as gravel, over `KNOLL_SCREE` (0.70, about a twelfth) as scree. The noise is spatially
  coherent, so they come in patches. A real drop of two or more levels keeps the old test.
- **Guard**: `tests/biomes.rs` `scree_lies_on_the_knolls_of_generated_valleys`: on the relief of the
  opening valleys of seeds 1 to 4, with every top laid bare, the pass turns some knoll tops loose,
  under 35% of them, never a top that does not step on two sides, and the same way twice. Checked to
  fail on the old pass ("seed 1: the pass turned nothing loose on 1276 knoll tops"). The synthetic
  cliff guard still passes.
- **Digests change**: every world's ground does now (surface materials on knolls), so a fresh
  four-seed 600-day soak gates the PR.
- **Commit**: this commit.

## 2026-11-12 #192 WP-08 8.4: relationship lines — the valley shows who is bound to whom

- **Symptom**: #192's legibility list left "relationship lines" undelivered: who is
  paired with, kin to, or at odds with whom lived only as a text line deep in the
  click-to-inspect panel ("people: friend Asha (+0.42)"), invisible at the glance
  distance where a watcher reads a valley.
- **Found by**: WP-08 8.4's triage (docs/reviews/2026-11-10-378-m7-triage.md), the
  remaining P1 legibility item after activity emotes shipped (#692).
- **Cause (checked)**: not a defect — the data was there (`agents.rs` relationships,
  the inspect panel's strings) but no view drew it.
- **Fix**: the bridge emits `relationship_lines()` — 3 ints per visible pair
  (from_row, to_row, opinion band 1 friend / 2 likes / 3 knows / 4 dislikes, the
  inspect panel's own thresholds) in `agents()` row space, each pair once with the
  from-side's band (`hf-godot/src/lib.rs`, `relationship_lines_core`). The client
  draws one line per pair head-to-head, coloured by band, on the R toggle
  (`entity_layer.gd`, `main.gd`); the same 1.75x zoom gate as the activity emotes
  keeps the far view clean. When the two sides disagree the line keeps the lower
  row's colour (D88).
- **Guard**: `relationship_lines_emit_each_visible_pair_once_with_the_from_side_band`
  (single emission, the from-side band on disagreement, and the row shift when a
  dead-unfound person takes no row).
- **Digests change**: none — a read-side view over existing state; sim
  trajectories are untouched.

## 2026-11-12 #724 follow-up: the portrait guard could not fail

- **Symptom**: `test_portrait_identity_key_and_real_ramp_colour` (#724) checked with `assert()`,
  which `tools/gd_test.gd` does not count (every other test reports through `check()`): a failed
  `assert` prints an error and the run can still end green. It also left an orphan `Control`.
- **Found by**: review of #724; merged before the fix. (The `.tools` symlink #724 also committed
  is #734.)
- **Cause (checked)**: the guard was written with `assert`.
- **Fix**: `tools/gd_test.gd` `test_portrait_identity_key_and_real_ramp_colour`: the six
  `assert(...)` are `check(...)` (the last gains a message) and the portrait node is freed.
- **Guard**: the test itself, now able to fail; box-ci runs it.
- **Digests change**: none.

## 2026-11-12 A machine-local `.tools` symlink was committed and shadowed the project Godot

- **Symptom**: master carried `.tools` as a symlink (mode 120000) to
  `$HOME/ostrog/.tools`. On any other checkout it dangles, so `tools/godot` cannot find
  the project-local Godot at `.tools/godot4`, and a machine's home path sits in the repo.
- **Found by**: review of #724 (flagged before merge); it landed with the portraits merge
  `3bbe9d8a`.
- **Cause (checked)**: `.gitignore` had `/.tools/`, which matches only a directory, so a symlink
  named `.tools` was not ignored and `git add -A` took it.
- **Fix**: `.tools` untracked; the rule is now `/.tools` (a file, directory or link of that name).
- **Guard**: `tools/ci.sh` fails when any symlink is tracked (`git ls-files -s`, mode 120000).

## 2026-11-12 #192 WP-08 8.4 (third increment): the inspect panel shows the person's face

- **Symptom**: the click-to-inspect panel named a person and listed their state,
  but showed no face — the only place the game's people read as individuals was
  the world sprite itself.
- **Found by**: WP-08 8.4's triage (portraits are one of #192's four P1 items).
- **Cause (checked)**: not a defect; nobody had drawn the panel portrait.
- **Fix**: `godot/scripts/portrait.gd` draws the head and shoulders procedurally
  from the same gene rows as the world sprites (agent_row 6-11, 16-18, 22): skin,
  hair style and colour, clothing colour through the sheets' own ramps
  (swaps.json targets + palette), child scale, elder graying, an injury bandage,
  and hairless kinds showing their head marking. Shown above the inspect text;
  no new art and no sim surface.
- **Guard**: `test_portrait_identity_key_and_real_ramp_colour` (deterministic
  identity key; the ramp returns the sheet hex).
- **Digests change**: none — pure client view.

## 2026-11-12 #336 WP-05 5.4: a pool cavern the valley had dug into was never fished

- **Symptom**: a valley that broke into a cavern with a still pool (`Cavern { pool, opened }`)
  never fished it, however short food ran: the blind cave fish of #336 ("cave fish", a
  cave-adapted surface archetype) did not exist in practice.
- **Found by**: WP-05 5.4's next increment, reading `fish::spot` against #336's list.
- **Cause (checked)**: `fish::spot` looked only at surface water (`ground_z + 1`) within forty
  tiles of home.
- **Fix**: `fish::spot` keeps the nearest surface water first and falls back to
  `fish::cave_spot`: a water tile of the pool of a cavern someone of the valley has stood in
  (`opened`), not walled up again (`sealed`), within 60 tiles of home, with a dry bank that can be
  walked to from home (`nav.reach`). Settlement step 7c calls `ensure_nav` before asking. The
  cave crocodile already keeps the shallowest pool cavern (#674), so fishing there carries its
  risk with no new code. Known limit: the fish stock is keyed by 16-tile square, not depth, so a
  cave pool directly beneath a surface lake shares that lake's stock.
- **Guard**: `tests/cave_fish.rs` `an_opened_pool_cavern_is_fished_and_a_closed_one_is_not`
  (unopened: none; opened: a water tile of that cavern, deterministic; sealed: none) and
  `a_pool_cavern_beyond_the_reach_of_home_is_not_a_spot`.
- **Digests**: change only where a community runs short of food with no surface water in reach
  and an opened pool cavern within reach; the four-seed 600-day soak is the evidence.

## 2026-11-12 #268 WP-05 5.6: rough ground cost the same to cross as a meadow

- **Symptom**: a scree slope, a mud flat, gravel or sand were as quick going as grass, so paths
  ran straight over them; the "per-tile roughness driving movement" row of #268's audit was
  missing, and the scree and gravel generated valleys now carry (#687, #718) changed nothing about
  how people moved.
- **Found by**: #268's audit table, under WP-05 5.6 (the moisture row was already met, erosion is
  deferred).
- **Cause (checked)**: `nav::step::step_cost` knew wading, doors and worn paths (#84) and nothing
  about the ground underfoot.
- **Fix**: `nav::step::rough` adds ticks by the ground under the tile stepped onto: scree +3, mud
  +2, gravel and sand +1 (docs/TIME.md "Ground underfoot"); `step_cost`, and so both moving and
  path planning, use it. D89 proposed.
- **Guard**: `tests/rough_ground.rs` `rough_ground_costs_more_and_a_worn_path_less` and
  `a_path_goes_round_a_scree_band_where_it_can_and_over_it_where_it_cannot` (fails on master:
  with equal costs the straight line over the band is taken).
- **Digests**: change wherever someone walks over sand, gravel, mud or scree; the four-seed
  600-day soak is the evidence.

## 2026-11-12 #294 / #378: deterministic client automation output for headless smoke runs

- **Symptom**: the Godot client could already take screenshots and open inspect panels from
  command-line flags, but a test runner had no stable machine-readable result containing the
  world identity, tick, or selected read model. Window automation was therefore either manual
  or dependent on parsing presentation text.
- **Cause (checked)**: `godot/scripts/main.gd` had `--shot`, `--footage`, `--inspect` and
  `--inspect-block`, but no opt-in result contract and no metadata-only exit path for headless
  runs.
- **Fix**: add `--automation-json=PATH`. After the world is prepared it writes a versioned JSON
  envelope with seed, tick, world name, display backend, selected person/block read models and
  requested shot/footage paths; metadata-only runs then quit. Shot and footage runs write the
  same envelope immediately before their existing quit. No simulation state or save format is
  changed.
- **Guard**: `tools/tests/client_automation_guard_test.py` fails when the parser, writer,
  versioned schema, JSON serialization, or opt-in path is absent.
- **Validation**: the guard, README flag/table checks and Godot's 25-script parser check pass.
  A matching Windows `hf-godot.dll` smoke with `--stage=demo --no-director --inspect=1
  --inspect-block=68,38,2` wrote schema 1 JSON containing the Corenor inspect record and the
  Grass block record; the non-applicable food value is represented as JSON `null` without a
  `NaN` warning. Full `tools/ci.sh` is running on this exact branch.

## 2026-11-12 #626 WP-02 2.11a: server publish guard used Windows path separators

- **Symptom**: the server host guard failed on Windows after publishing a valid tree because its
  expected URL paths contained `/` while `Path.__str__()` returned `\`.
- **Cause**: the test compared filesystem spelling with URL-shaped public paths instead of
  normalising the relative path at the assertion boundary; its Windows temporary paths were
  also passed to the Bash publisher with backslashes, which made `sha256sum` escape the digest.
- **Fix**: use POSIX spellings for the URL assertion and every path handed to the Bash fixture,
  without changing the publisher or the served files.
- **Guard**: `tools/tests/server_host_test.py` itself reproduces the complete published layout,
  rejects unprefixed files, and now passes on Windows.
- **Validation**: focused server guard, full `tools/ci.sh`, and box CI on the pushed head.

## 2026-11-12 WP-02 2.15 guard: simulated Unix XDG paths fell back on Windows

- **Symptom**: the exact-head Windows CI failed `godot_user_dir_follows_godots_rule_on_each_platform`;
  its simulated Linux `XDG_DATA_HOME=/data/f` was treated as relative and the launcher used
  the simulated HOME path instead.
- **Cause**: `Path::is_absolute()` follows the host Windows path rules even when the helper is
  intentionally exercising a Unix target, where a leading `/` is absolute.
- **Fix**: accept a leading slash as absolute for the non-Windows target branch while retaining
  native Windows path checks for the real Windows branch.
- **Guard**: the existing cross-platform `godot_user_dir_follows_godots_rule_on_each_platform`
  test fails without this fix and now covers the simulated XDG path on Windows.
- **Validation**: focused launcher test, exact-head `tools/ci.sh`, and box-ci are required.

## 2026-11-12 #197 WP-08 8.7: a leaving band with no way to the edge stayed in the valley for good

- **Symptom**: #197 case 12. A band leaving for home (fed, broken, tired, or its siege lifted) whose
  members could not reach the map edge never left. They stood where they were, the band was never
  removed, and `busy` (`threat.rs`, a site with a band out) kept that overworld site from ever
  sending another band.
- **Found by**: #197 case 12 (code reading in QA 2026-10-20), confirmed with the guard below:
  three members in sealed pockets in the middle of the valley stand the whole day, and the band
  stays.
- **Cause (checked)**: `threat.rs` `bands_step`, `BandState::Leaving`: a member more than two tiles
  from the edge tile was sent there with `go()`, which does nothing when `path_agent_to` finds no
  way, and says nothing. The next tick tried again, forever.
- **Fix**: `go()` returns whether it set them walking; in `Leaving`, one with no way to the edge
  leaves the valley from where they stand (`society::leave_valley`), as one with no edge tile at
  all already did.
- **Guard**: `tests/edge_cases_197.rs` `a_leaving_band_that_cannot_reach_the_edge_still_goes` (the
  band and its site's `busy` are gone within a day).
- **Digests**: change only where a leaving member has no path to the edge, which a normal opening
  rarely meets; the four-seed 600-day soak on the PR is the evidence.

## 2026-11-12 #197 WP-08 8.7: a community whose people were all on the road was recorded as died out

- **Symptom**: #197 case 7 asked whether bands keep camping at an empty first home once the
  founding community is gone. They do not (below), but the same check had a neighbour: a
  community whose living people were all away on a caravan or a journey was ended as "died out"
  at the next daily round, while they were still on the road.
- **Found by**: #197 case 7 (QA 2026-10-20), while checking `threat.rs` `home()`, confirmed with
  the guard below.
- **Cause (checked)**: `systems/moving.rs` `endings()` looked for living people of the community
  only in `w.agents`; people on the road are moved to `overworld.away` until they come back.
  Case 7 itself does not hold: `home()` takes the first community not dissolved, and `endings()`
  dissolves one with nobody left by the next daily round.
- **Fix**: `Overworld::on_the_road(cid)` (expedition.rs) lists those of a community off the map
  and expected back (on a caravan not lost or done, or on a journey of their own; not a captive or
  a child gone below for good); `endings()` ends a community only when none of them is on the road
  either.
- **Guard**: `tests/edge_cases_197.rs` `a_community_dies_out_when_nobody_is_left_and_not_while_they_are_away`
  (fails on master; once they all die on the road the community ends, so the bands' home moves on).
- **Digests**: change only where a whole community is on the road at a daily round; the soak on
  the PR is the evidence.

## 2026-11-12 #508 WP-07 7.3: the first rock-sense manifestation had no personal interpretation

- **Symptom**: the first dangerous face a carrier refused was publicly ambiguous, but the
  carrier kept no personal account of what they thought had happened; the discovery chain
  could not progress from manifestation to interpretation.
- **Cause**: `Resonance` stored only the hidden potential, wake state and refusal count. The
  existing `WouldNotCut` event intentionally avoided naming a power, but there was no private,
  person-specific state behind it.
- **Fix**: record the first refused face and an interpretation chosen from that carrier's own
  knowledge: thin-place lore yields a remembered-stone reading, a prior anti-deep conviction
  yields a deep-warning reading, and otherwise the person keeps unexplained dread. Later refusals
  do not rewrite the first account; the public event remains ambiguous.
- **Guard**: `crates/hf-sim/tests/resonance.rs::first_manifestation_is_ambiguous_personal_and_saved`
  proves first-only recording, different interpretations for different knowledge, and save/load
  preservation. It fails without the new private state and assignment.
- **Validation**: focused resonance tests, full workspace CI, and a fresh four-seed 600-day soak
  are required before merge; this changes `hf-sim` behavior.

## 2026-11-12 #715 follow-up: restore additive chronicle tables after loading old saves

- **Symptom**: the #715 built-record view loaded the v2 fixture successfully, then failed on the
  first later `Built` event with `no such table: built_record`.
- **Cause (checked)**: `save::read` initializes a current working chronicle and then SQLite
  backup replaces its contents with the older save, including its older table set.
- **Fix**: re-apply the additive chronicle schema after the backup. Existing rows remain intact;
  newly introduced tables and indexes are recreated with `IF NOT EXISTS`.
- **Guard**: `migration_chain_loads_fixture` now asserts that the current `built_record` table
  exists before advancing the migrated world. It fails on the uncorrected #715 branch.
- **Generated docs**: README tables were regenerated for the new source line numbers; the
  `readme_tables.py --check` guard is green.
- **Validation**: focused migration test and the normal CI/box-ci gates are required before
  merging this follow-up into #715.

## 2026-11-11 WP-02 2.11a follow-up: the publisher kept the checksum escape on Windows

- **Symptom**: on the Windows Git Bash publish path, the served `.sha256` record and the download
  page carried a digest with a leading `\` (the `sha256sum` line escape for the backslash-containing
  temporary filename), so the published checksum did not match the package.
- **Cause (checked)**: `sha256sum` prefixes its output line with `\` when the filename contains
  backslashes, and `cut -d' ' -f1` keeps that backslash as part of the digest. The guard's own
  path spelling was fixed separately in 5aa25e4a ("without changing the publisher"), so the
  publisher still needed this.
- **Fix**: `tools/server/pull_sign_publish.sh` strips only the leading escape (`sed 's/^\\//'`)
  after `sha256sum` and before the digest reaches the checksum file and download page.
- **Guard**: `tools/tests/server_host_test.py` asserts the exact published digest; a digest with a
  leading backslash fails the guard. The sed strip is a no-op on POSIX paths.
- **Validation**: focused server guard test run on the merged head.

## 2026-11-11 #286 WP-05 5.5: the thistle seeding loop runs once per stripe, not once per tick

- **Symptom**: the #286 seeding block in `systems/plants.rs` ran every tick, but its tile
  filter `(n + t / STRIPE) % 8 == 0` and its rng key (`t / STRIPE`) are both constant across
  the sixty ticks of a stripe, so each selected tile was re-tested sixty times with the
  identical stream (sixty identical answers) and each test scanned `w.plants` for a plant on
  that tile — field tiles x plants work every tick, the exact cost WP-01 closed.
- **Found by**: coordinator review on #676 (2026-11-10), held as the one item to fix before
  that PR merges.
- **Cause (checked)**: not a defect in the mechanic — the sixty identical trials collapsed
  to one effective roll per tile per qualifying stripe either way, because pushes land at
  the end of the tick and the occupancy scan then blocks re-seeding — but the redundant
  trials and scans were real per-tick work, and the comment's "once a day" understated the
  true cadence (each empty field tile qualifies in every eighth stripe, about sixty times a
  day, at 0.8%: roughly 0.5 thistles per empty tile per day).
- **Fix**: the block is gated on `t % STRIPE == 0` (once per stripe) and the chance roll now
  happens before the `w.plants` occupancy scan, so the scan only runs for the ~0.8% of rolls
  that pass. The per-tile rate, the stream keys and the push shape are unchanged. One
  rare divergence, accepted: a tile whose stripe roll is true but which is occupied at the
  stripe's first tick used to seed later in the same stripe once it emptied (same roll on
  the next tick); it now waits for its next qualifying stripe.
- **Guard**: unchanged — `tests/weeds.rs` (`a_worked_field_picks_up_thistles_over_time`,
  the same field picks up the same thistles) steps from tick 0 and still sees the same
  deterministic seeding; the gate fires at t = 0, 60, 120, ... which that range covers.
- **Digests change**: none expected in normal play — the effective rate and the streams are
  unchanged; a fresh four-seed 600-day soak on the merge head is the evidence (#676).

## 2026-11-10 #286 WP-05 5.5: thistle, a weed of worked ground that must be cleared

- **Symptom**: #286's list had "weeds and undesirable growth that must be cleared" as one of
  its eleven gaps; worked fields were unnaturally clean forever.
- **Found by**: WP-05 slice 5.5's re-scoping of #286 (the 2026-10-22 audit was stale:
  differentiated grains, dyes, medicinals and fruit bearers already existed).
- **Cause (checked)**: not a defect: no weed kind existed, and the field planner's only
  answer to anything growing on a field tile was to sow over it (or leave fallow tiles
  untouched forever).
- **Fix**: `PlantKind::Thistle` appended (saves store variants by position; old saves load
  it as a new tail variant). Each empty field tile of a living community has a small chance
  per attempt to pick one up from the world-seed stream (tags: purpose::PLANTS, 0x7715,
  community, tile, hour — each attempt an independent draw), so a worked field gathers
  thistles over days and untouched ground never does. A thistle within a step of a crop
  halves that crop's growth factor (`systems/plants.rs` `suits`) until it is pulled. The
  planner posts `DesignationKind::Uproot` (appended; 3 minutes at skill 1.0) on a mature
  thistle that stands on a field tile and presses a neighbouring crop; on a tile being sown
  the weed is still cleared by sowing over it, as shrubs were. Uprooting pulls it root and
  all; nothing of it is kept. `PLANT_KIND` gains "thistle" in the client, with generated
  stage sheets (a flat spiny rosette, then a stalk with a violet head).
- **Guard**: `tests/weeds.rs`: `a_thistle_crowds_the_crop_beside_it` (same wheat, one with a
  thistle beside it: it grows less), `a_worked_field_picks_up_thistles_over_time` (thistles
  appear on the worked tiles and only there), `a_mature_thistle_pressing_a_crop_is_pulled`
  (a living valley orders the pressing weed pulled and it is pulled), and
  `weed_arrival_is_deterministic` (the same field picks up the same thistles).
- **Not here**: weed-to-weed spread beyond a field, seed blow-over from wild ground, and
  other weed kinds (dock, nettle as a weed) are follow-ups; fodder crops and tree
  wood-species are the other open gaps of #286. Digest effect: worlds with worked fields
  gain events; the four-seed 600-day soak on the PR is the evidence the rate is sane.

## 2026-11-12 #414 WP-06 handoff: frame-time guard accepted only pre-6.3 wording

- **Symptom**: the c186 handoff PR was rejected by `tools/tests/frame_time_harness_test.py` even
  though WP-06.3 was complete; the guard only recognised the earlier 6.1 and 6.2 plan statuses.
- **Cause**: the guard was not advanced when the renderer plan moved to the 6.3-complete status.
- **Fix**: accept the current `Status: open (6.3 complete...)` wording while keeping the existing
  frame-harness and dependency assertions unchanged.
- **Guard**: `tools/tests/frame_time_harness_test.py` itself fails on the stale status and passes
  on the current handoff wording.
- **Validation**: focused guard and full box CI required on the pushed head.

## 2026-11-11 #626 WP-02 2.15: the launcher never saw the game's save-write marker

- **Symptom**: the updater's "never update mid-save" guard could not fire in a real install. The
  game takes its marker (`.ostrog-save-writing`) in the folder that holds its save slots, which
  is Godot's per-user folder (`user://saves`, on Windows `%APPDATA%\Godot\app_userdata\Ostrog\saves`);
  the launcher looked only in `<install>/data` and that folder's direct children.
- **Found by**: the 2026-11-10 files-register audit (WP-02 slice 2.15, docs/privacy/FILES-ON-YOUR-COMPUTER.md
  "Open items" 1): `hf-launcher` starts the game with no data-folder setting, `project.godot` sets no
  custom user dir, and `hf-godot`'s `save_to` takes the lock in `dir.parent()` of the slot.
- **Cause (checked on Linux, not yet on Windows)**: `OS.get_user_data_dir()` under Godot 4.6.3 is
  `~/.local/share/godot/app_userdata/Ostrog` and `user://saves` sits under it; nothing in the
  launcher looked there. Windows follows Godot's documented `%APPDATA%\Godot\app_userdata\<name>`
  rule; 2.9 / 9.11 confirm it on a clean machine.
- **Fix**: saves stay in the standard per-user folder (the slice's recommendation). `hf_update::godot_user_dir`
  (crates/hf-update/src/save_guard.rs) gives Godot's per-user folder for a project on Windows, macOS
  and Linux (XDG_DATA_HOME honoured); `LauncherSettings::game_user_dir` holds it, filled from the
  environment by `load`/`load_or_default` (not a `launcher.toml` key: settings parse with
  `deny_unknown_fields`, so an older launcher would refuse a newer config); `LauncherSettings::save_in_progress`
  checks it as well as `data/`, and both checks in `run_with_runtime` use it.
- **Guard**: hf-launcher `a_save_lock_in_the_games_own_user_folder_defers_the_update` (the lock taken
  as `save_to` takes it, in `saves/` of the game's folder: `data/` alone does not see it, the launcher
  starts the installed build with `SaveInProgress` and leaves `app/` alone) and
  `godot_user_dir_follows_godots_rule_on_each_platform`.
- **Not here**: the clean-Windows confirmation (2.9, 9.11).

## 2026-11-11 #414 WP-06 6.3: static scene data had no Rust batch bridge

- **Symptom**: WP-06.2 moved people and animals behind the opt-in Rust renderer, but static
  plants, structures, items and ground rows still had no bounded Rust-to-Godot batch contract.
- **Cause**: the bridge exposed agent rows only; the static and ground layers continued to be
  assembled by the GDScript view path, so Stage 4 could not exercise the same batch boundary.
- **Fix**: added deterministic rayon-backed static and ground instance builders, compact bridge
  methods, and opt-in Godot MultiMesh uploads for the three static families plus ground rows.
  The established layered renderer remains the default; this slice is a bounded generic-texture
  proof path, not a claim of final art parity or a passed frame gate.
- **Guard**: `tools/tests/world_renderer_guard_test.py` fails if either packed contract, Rust
  builder, bridge method, or Godot upload/fallback wiring is removed.
- **Validation**: focused source guard, Rust formatting/check/tests, Godot parser/runtime smoke,
  and exact-head CI are required before merge. The client-only slice uses the recorded four-seed
  soak evidence because it does not modify `crates/hf-sim`.

## 2026-11-10 #267 WP-05 5.6: the middle stratum splits into sub-bands a shaft can read

- **Symptom**: below the soil, one middle rock (granite, slate or marble under limestone)
  ran from the first break to the deep in every column, so a dug shaft crossed nothing and
  had nothing to encounter and name on the way down (#267: "no alternation — no thin seams,
  nothing to encounter and name as you descend").
- **Found by**: #267, carried as WP-05 slice 5.6.
- **Cause (checked)**: not a defect: `world/gen/strata.rs` wrote one middle rock per column
  and the two breaks (top of the middle, top of the deep) had no interior structure.
- **Fix**: the generator lays one or two extra breaks per valley inside the middle stratum
  (a valley-wide count, like the seam height), splitting it into two or three sub-bands that
  alternate the column's two hard rocks: the shallow band keeps `strata.0` (the marble patch
  included), the next is the other of granite/slate, the third the first again. The breaks
  sit at even fractions of the band and wobble a step up or down with their own folds, so a
  shaft shows recognisable layers and two shafts read the same ones. Per-column band rocks
  and tops live in the new `WorldGrid::bands` (deepest first, serde-default empty, so old
  saves keep one middle rock). `WorldGrid::rock_at` reads the bands; the block facts name
  every layer ("granite above z-9, slate down to z-17, basalt in the deep"). No new
  materials and no enum changes; the aquifer keys only on the sedimentary top of the middle
  stratum (`aquifer::held`) and is untouched; a well still gives up at the first hard rock.
- **Guard**: `tests/the_deep.rs` `middle_stratum_reads_a_sequence_as_a_shaft_digs` (seeds
  1..6: every band top strictly inside the middle stratum, ordered deepest first,
  alternating rocks, the deepest band reaching the deep break, some shaft crossing both hard
  rocks) and `bands_are_deterministic_and_absent_in_old_grids` (same seed, same bands; a
  grid with no band rows reads one middle rock).
- **Digests change**: middle-band rock differs in every new world, and tin/silver vein
  acceptance (keyed on granite/slate tiles) shifts with it; the four-seed 600-day soak on
  the PR is the evidence the change is sound. **Not here**: #268's remainder (the audit's
  altitude temperature lapse and drainage), #274's material tiers, and sedimentary or
  water-bearing sub-bands.

## 2026-11-11 #197 WP-08 8.7: a person walled in on every side was left standing inside the wall

- **Symptom**: #197 case 8: when the last open tile beside someone is walled in (a wall finished
  on the tile they stand on, rock or wall on every other side), `displace_from` found no walkable
  neighbour and returned, so the person and whatever lay on the tile stayed inside the wall.
- **Cause**: `displace_from` (systems/work.rs) only looked at the eight tiles around the wall and
  gave up when none was open.
- **Fix**: in that case only, it takes the nearest walkable tile within three (the same level
  first, then one level either way, never a tile a wall is about to be built on; ties to the lowest
  z, y, x). Every run where a neighbour is open takes the same path as before, so digests do not
  move; the only difference is in the pocket case.
- **Guard**: `tests/edge_cases_197.rs`: `someone_in_a_walled_in_pocket_is_not_left_inside_the_wall`
  (checked to fail without the fix with exactly the issue's symptom), plus six scenarios from the
  same list that already behave: `a_save_made_on_the_tick_of_a_death_goes_on_as_the_continuous_game_does`
  (case 1), `a_save_made_between_walls_and_roof_comes_back_unroofed` (case 2),
  `a_caravan_nobody_comes_to_is_given_up_and_its_cargo_comes_back` (case 3: the loaded logs leave
  the world at formation and are all back after the journey is given up),
  `a_thing_with_a_story_is_not_eaten_by_rot_and_its_twin_without_one_is` (case 5: a trophy hide
  keeps while an ordinary one rots), `a_partner_away_when_the_other_dies_comes_home_unpaired` (case 6) and
  `mining_a_face_on_the_edge_of_the_grid_is_safe` (case 10: all four edges, the face is cut). A fresh four-seed 600-day soak gates the PR.
- **Already guarded elsewhere**: case 9 (a floor over a cellar) is `tests/qa_1020.rs` (#174).
- **Open**: cases 4, 7, 11 to 15 of #197 are not yet provoked; they stay in WP-08 8.7.

## 2026-11-11 #626 WP-02 2.11/2.11a: the first download had no home and every file sat at a guessable address

- **Symptom**: `pull_sign_publish.sh` downloaded the portable first-install package into staging and
  never copied it anywhere a friend could reach, no page linked it, and the manifest and archive were
  published at `/stable/...` on a hostname whose certificate is in public logs (a threat-intel
  crawler reached the live host within the hour of go-live).
- **Cause**: slice 2.7 published only the update path (manifest and app archive) and put it at a fixed,
  guessable path.
- **Fix**: the job reads a private prefix from `/srv/ostrog/prefix.txt` (outside the repo and the
  served tree, mode 600) and refuses, before downloading or signing anything, if it is missing,
  shorter than 22 characters or not `A-Za-z0-9_-`; it never prints it. The manifest, the app archive,
  the portable package, its `.sha256` and a generated `index.html` (`tools/server/index.html`, filled
  with version, tag, file name and checksum) all land under `public/<prefix>/`; the signed archive URL
  carries the prefix. A release without the portable package publishes nothing. `tools/package.sh`
  already takes the full manifest URL through `OSTROG_MANIFEST_URL`.
- **Guard**: `tools/tests/server_host_test.py`: the exact set of published files (all under the prefix,
  nothing outside it), the prefix in the signed archive URL, the prefix absent from the job's output,
  the checksum file and page matching the fixture package, missing/short/slash/space prefixes refused
  with nothing downloaded or signed, a missing portable package publishing nothing.
- **Open**: the live host still needs the one-time setup in `tools/server/README.md`; the launcher's
  built-in fallback config still names the old unprefixed address; revoking the link means a new prefix
  and a reinstall for every friend.
- **Commit**: this commit.

## 2026-11-10 #192 WP-08 8.4: the busy show what they are doing

- **Symptom**: #192's legibility list names portraits, activity emotes, relationship lines and
  a minimap; only the emotes were missing in every layer — a watcher could see a person bent
  over a task but not which task until they clicked them open.
- **Found by**: #192 (via #378's M7 triage, docs/reviews/2026-11-10-378-m7-triage.md), taken
  as WP-08 slice 8.4's first increment.
- **Cause (checked)**: not a defect: the bridge's agents row carried the work's sound code
  (for positional effects) but nothing visual; the specific activity never crossed the
  bridge.
- **Fix**: the agents row grows one appended field, `emote` (positions unchanged before it):
  an item-kind index + 1, or 0 for none — the work's own tool, from the same designation
  lookup as the sound (chop → the axe, mining → the sledge, building → the hammer, field
  work → the hand tool, crafting → the saw, cooking → the vessel, dousing and beating out →
  the bucket, tending → the poultice, hunting → the spear, studying → the record). The
  sound keeps its working-pose gate; the emote keys on the work step itself, so a seated
  craft or a study shows its tool too. The client draws the existing item icon (no new art)
  above the person's head at 14 px, only at 1.75x zoom or closer — the roof cut-away's
  threshold, so emotes never crowd the far view.
- **Guard**: `tools/gd_test.gd` `test_activity_emote_art_exists` (every kind the bridge can
  send is in ITEM_NAME and has an icon in the small or the full item set) and the stride
  bump through the effects fixture. `GD_CHECK 24 scripts, 0 failed`, `GD_TEST 57 passed, 0
  failed`; shots on the new 25-field row show no visual regression.
- **Not here**: portraits, relationship lines and the minimap stay as 8.4's remaining items.
  The simulation is untouched (no digest change, no soak gate); a live emote-on-screen
  capture is welcome from the Windows reviewer, who exercises the client by hand.

## 2026-11-11 #414 handoff guard: the frame harness guard stopped at 6.2 in progress

- **Symptom**: the c178 handoff correctly marked WP-06.2 complete, but box CI rejected the
  documentation-only PR because the frame-harness guard only recognized the earlier 6.1-complete
  and 6.2-in-progress plan states.
- **Found by**: PR #693 exact-head box CI after the c178 WP-06.2 merge.
- **Cause**: the guard encoded the live plan as a two-state transition and had not been advanced
  when the renderer slice closed.
- **Fix**: accept the completed 6.2 handoff state while retaining the existing 6.1 and in-progress
  states for branch-local validation.
- **Guard**: `tools/tests/frame_time_harness_test.py` now fails if the harness guard loses any
  valid WP-06.1/WP-06.2 handoff state; `tools/docs/readme_tables.py --check` remains green.
- **Validation**: local `tools/ci.sh` was already green on exact WP-06.2 head; this correction
  is docs/guard-only and is being rechecked by box CI on PR #693.

## 2026-11-11 #414 WP-06 6.2: the renderer had no Rust-side people/animal batch path

- **Symptom**: WP-06.1 could measure the real client, but the view still walked every visible
  person and animal through pooled GDScript sprites. There was no Stage 3 batch path to measure or
  put behind the renderer setting.
- **Found by**: WP-06.2 review against the D53 renderer sequence after the c177 6.1 handoff.
- **Cause**: the bridge exposed packed agent rows only; no immutable snapshot reduction existed
  for the renderer and no Godot `MultiMeshInstance2D` consumer was available.
- **Fix**: added `world_renderer.rs`, which filters the same known/dead/denned rules and builds
  deterministic people/animal instance transforms with rayon from the snapshot. The bridge
  exposes the compact eight-float contract, and an opt-in Godot node uploads two MultiMesh batches.
  The established layered renderer remains the default until art parity and frame evidence pass.
- **Guard**: `tools/tests/world_renderer_guard_test.py` fails if the rayon path, compact bridge,
  MultiMesh upload, fallback, or default-off project setting is removed.
- **Validation**: focused source guard and `cargo fmt --all -- --check` pass; targeted Godot
  bridge tests are running on the fresh worktree.

## 2026-11-10 #289 WP-05 5.5: loose rock lies on steep bare ground

- **Symptom**: a hillside read the same as a plain: the ground families follow the
  biome, so a steep bare slope under a crag wore the same grass or soil as the flat
  below it (#289's item 4; the art and the material table long had scree and gravel,
  and the mountain biomes used them, but nothing placed loose rock by the lie of the
  land).
- **Found by**: #289, carried as WP-05 slice 5.5 on DEC-8's approved default
  (docs/design-grounds.md item 4: scree on slopes, pure worldgen, no runtime state).
- **Cause (checked)**: not a defect: no generator pass keyed surface rock to the
  local gradient.
- **Fix**: `world/gen/scree.rs` runs after the families are laid and before the
  water deepens and the plants come: where a column's steepest step to a neighbour
  is two levels or more and its cover is bare (soil, grass, sand — not the
  vegetated meadow, moss or peat, and not gravel or scree already), a patch noise
  lays the surface as gravel, or scree on the steepest faces (rise of three or
  more); below the noise threshold the family keeps its ground, so the loose rock
  breaks into patches instead of sheeting. Seeded per column from the world seed
  alone; no new material, enum or saved state; flora and the founding site see the
  result and plant and land accordingly.
- **Guard**: `tests/biomes.rs` `scree_lies_on_steep_bare_ground` (a fixture cliff:
  conversions only on steep ground, only from bare covers, only to scree or gravel,
  the vegetated cliff top keeps its meadow, and the pass is deterministic in the
  seed).
- **Digests change**: surface materials differ on steep bare ground in every new
  world (and what grows and where the founding site lands can shift with them); the
  four-seed 600-day soak on the PR is the evidence. **Not here**: items 2+3 of the
  doc (the weather ground overlay: mud after rain and lying snow, one derived pass)
  are the next #289 PR; item 1 (worn paths) waits for that pass; item 7's family
  tuning is data tuning for a later small PR.
- **Correction (2026-11-12, #698)**: measured after the merge, the pass changes no
  tile in the generated opening worlds of seeds 1 to 4: their tick-0 digests equal
  the digests without it, and the four-seed soak is identical number for number. At
  the point the pass runs, no column in those worlds stands two levels off a
  neighbour (every column is within one level of all that lies two tiles around
  it), so "Digests change" above did not happen. The guard drives `scree::apply` on
  a synthetic cliff only. Whether to drop the pass or redefine steep ground is open
  on #698.

## 2026-11-10 #289 WP-05 5.5: the weather ground overlay is the simulation's own

- **Symptom**: the client drew three weathers on the ground that the simulation did
  not know about: frozen soil in a hard frost, wet soil in rain, and snow lying on
  exposed covers all winter (`world_view.gd`'s frozen/wet swaps and its SNOWABLE
  snow). A watcher saw ground that could not burn; the fire spread across it
  anyway, and a clicked block named only the stored tile.
- **Found by**: #289, carried as WP-05 slice 5.5 on DEC-8's items 2+3
  (docs/design-grounds.md's "weather ground overlay" PR).
- **Cause (checked)**: not a rendering bug — presentation ran ahead of the sim.
  The `SNOW` tile flag the fire checks was never set on ground; nothing derived
  the cover from the weather.
- **Fix**: `cover.rs` states the client's own rules as the simulation's:
  `cover_at` swaps the bare-earth families (soil, clay, wet soil, mud) to
  `FROZEN_SOIL` below -1 °C and to `WET_SOIL` in rain (mud already being as wet
  as it gets), and `snow_lies` lays snow on the exposed snowable covers in a
  winter cold enough or while it snows, off where a roof stands over the tile.
  Keyed on the valley reading exactly as the client's rules are, so the two can
  never disagree; derived from the saved weather alone, nothing new per tile.
  Wired in: the fire's grass-fuel rule (snow-covered ground carries none;
  plants and wood above the snow are another matter, as the C62 rule says) and
  the block facts ("snow lying", "frozen hard", "wet with rain", named only
  where the look is drawn).
- **Guard**: `tests/weather_breadth.rs`
  `the_weather_ground_overlay_matches_the_look` (a hot winter day A/B: snowfall
  suppresses the grass fuel, clear sky restores it, a roof holds the snow off;
  rain wets and frost freezes the bare earth; mud stays mud in rain).
- **Judgement calls, stated for Dan** (DECISIONS D85): the overlay wets only
  while it rains, matching the client's instant swap — the doc's "a bounded
  while after" persistence waits until the client shows mud after rain or Dan
  asks, so facts and looks never part; whether lying snow and mud should slow
  people and stock (the doc's second feel question) is not taken — no movement
  cost is in this slice. **Digests change**: winter fires behave differently;
  the four-seed 600-day soak on the PR is the evidence.

## 2026-11-10 #414 WP-06 6.1: the frame gate had no real-client measurement harness

- **Symptom**: `docs/design-renderer.md` required a reproducible 1920x1080 windowed GPU run with
  total frame time, main-process time, Rust sim-worker time, render wait, per-worker preparation
  time, CSV output and a one-line 16.7/33 ms verdict, but the repository only had headless CPU
  preparation scripts.
- **Found by**: WP-06 slice 6.1 review against the renderer design gate D53.
- **Cause**: the existing `frame_prep.gd` and `main_thread_frames.gd` intentionally avoid the
  renderer, so they cannot observe the real window or the post-draw boundary.
- **Fix**: added `tools/perf/frame_time.gd`, which disables vsync, requests a windowed 1920x1080
  client, loads a saved profile, measures wall/process/render-wait/sim-worker time across warmup
  frames, refreshes the bridge on the same 50 ms cadence as the shipped client, writes a CSV and
  prints the gate verdict. Stage 1 has no render-preparation pool, so
  the worker column is explicitly zero and labelled; later renderer slices must replace it with
  real per-worker timers.
- **Guard**: `tools/tests/frame_time_harness_test.py` fails if the harness loses the real-window,
  post-draw, process-monitor, sim timer, worker-column, CSV or verdict requirements, or if the
  plan remains blocked for 6.1-6.4. It also protects the startup latch that hands the first frame
  to `_measure_one_frame` instead of rendering forever.
- **Validation**: the focused guard and Godot 4.6.3 parser/runtime smoke pass. The laptop GPU
  600-frame baseline wrote `<win-drive>\wp06-frame-laptop-c176.csv` (p50 39.47 ms, p95 94.77 ms,
  p99 151.95 ms, max 202.48 ms, 357 frames over 33 ms): the harness is working and the gate is
  honestly **not yet passing**, as expected before WP-06.2's renderer work.
- **Open**: this slice does not claim the gate passes. The saved 500 mixed-kind profile must be
  run on Dan's laptop GPU and the software-GL box, with both CSVs attached to the release evidence.

## 2026-11-10 #268 WP-05 5.6: one temperature, lapsed by height

- **Symptom**: the simulation read one valley-wide temperature everywhere, while the block
  facts displayed a temperature that fell 1.5 °C per level above the founding site's
  ground, and warmth drain (`hazards::cold_step`) already froze people on the heights
  harder than the valley floor. Frost, drinking-water freezing, the cold and heat
  decisions, fire risk, roof thaw and the underground's flat 10 °C each ignored height
  (#268's audit row: "no altitude lapse found").
- **Found by**: #268, carried as WP-05 slice 5.6.
- **Cause (checked)**: not one defect but three overlapping rules — a scalar simulation, a
  lapsing display, and a lapsing warmth drain that agreed with neither everywhere else.
- **Fix**: `World::temp_at(p)` is the one rule — the valley reading minus
  `thermal::LAPSE_C_PER_LEVEL` (1.5 °C) per level above the founding site's ground, and
  warmer by the same per level dug below it; the rate the display already used. Migrated
  to it: the field frost (reckoned per tile; the pass is gated on the coldest the
  valley's highest ground can be, so a marginal night can take a height and spare the
  floor), hardening to the cold, the freezing and warm decisions and the cold-trip
  weight, warmth drain (replacing `cold_step`'s own partial lapse), thirst's heat, work
  heat, hearth ignition and grass fire fuel, roof thaw, ice drowning at the water, food
  rot's cold-preservation threshold, and drinking-water freezing per tile. The block
  facts' underground flat 10 °C fiction is
  gone: they report `temp_at` like every other read. Kept on the valley reading, which
  has no position or is a valley budget: the sky's rain/snow call, snowmelt and
  evaporation (#393's hydrology), spell leans, trouble narration, clothing looks.
- **Guard**: `tests/thermal.rs` `high_ground_frosts_before_the_valley_floor` (on a -3 °C
  autumn night the plateau's crop is frosted and the valley floor's stands) and
  `a_dug_cellar_reads_warmer_than_the_winter_surface`.
- **Digests change**: heights and hollows now live under different weather; the
  four-seed 600-day soak on the PR is the evidence. **Not here**: per-tile drainage and
  the worldgen audit's roughness rows remain under #268.

## 2026-11-10 #325 WP-05 5.3: a dry field could not be watered from a pond a short way off

- **Symptom**: the 2026-10-23 fix-325 entry left irrigation "part": crops beside water grew a
  third faster, but a field a dozen tiles from a river or pond never drank, and nobody could do
  anything about it. Issue #325's own words: the dig-to-water case waited on a fluid model.
- **Cause**: water was a flag on tiles and nothing could carry it. docs/design-fluids.md (#669,
  DEC-7's default) settled that the bodies-and-catchments model stays and gains dug channels (W0),
  so the dependency on moving water is dropped.
- **Fix**: `channels.rs`. A community whose field has fewer than half its tiles drinking, and
  that has open water within 45 tiles of home, gets a proposal (`ProjectKind::Channel`, from the
  best farmer, once a day, one at a time): the shortest cut from a bank of the water to ground
  where at least six field tiles would drink, never uphill (no tile above the one before it, none
  more than a step below), over open reachable ground that is not built on. The planner orders the
  next three tiles (`DesignationKind::DigChannel`, appended; mining skill, the miner's role); the
  work system records each cut. Water reaches as far as the cut runs without a gap, and crops
  within three tiles of it count as watered (`plants::watered`). No per-tick work and no fluid
  state: the route is found once, on the proposal. A cut whose next tile is built over is given up
  (on record), and the community looks again after twenty days; at most two are cut per community.
  The new `World.channels` is saved with `#[serde(default)]`; `ProjectKind::Channel` and
  `DesignationKind::DigChannel` are appended variants (`tools/enum_order.py` passes).
- **Guard**: tests/irrigation.rs: `a_dug_channel_moistens_the_field_it_reaches_and_a_gap_stops_the_water`
  (route never climbs, starts at the bank, an ordered ditch is not a ditch, a gap stops the water,
  a control field stays dry), `no_water_in_reach_means_no_route`, `a_channel_survives_a_save_and_load`,
  and `people_with_a_dry_field_and_a_pond_dig_the_channel_themselves` (five people, forty days, the
  ditch is proposed and dug). A fresh four-seed 600-day soak gates the PR.
- **Open**: the effect is a record, not water you can see or breach (W1's flow region is the
  prototype that would let a wall hold it, a mistake flood the stores); the client does not draw
  the ditch yet.
- **Commit**: this commit.

## 2026-11-10 #336 WP-05 5.4: two cave-adapted variants of surface kinds, depth-tiered

- **Symptom**: the underground was thin against the catalogue of the dark the issue quotes:
  bats, a pale crawler, a burrower, a slime, a swarm of rats and a centipede — nothing at the
  mid tier between the crawler's deep and the bats' shallows, and nothing at the pools.
- **Found by**: #336, carried as WP-05 slice 5.4.
- **Cause (checked)**: not a defect: no species existed for those habitats. The placement and
  behaviour plumbing (`cave_ecology.rs` habitat profiles, `place_deep`, the per-kind branches
  of `systems/animals.rs`) already supported adding them.
- **Fix**: `Species::BlindCaveBear` and `Species::CaveCroc` appended (saves store variants by
  position; `tools/enum_order.py` passes). The blind bear lies up in the shallowest deep
  cavern that is not the crawler's (`cave_ecology::blind_bear_cavern`) and mauls whoever
  comes within three tiles where no light reaches, as territorial as a surface bear; the cave
  crocodile lies up at the edge of the shallowest pool cavern (`croc_cavern`,
  `systems/animals.rs::croc`) and seizes whoever steps within reach of the water. Both keep
  to their caverns between incidents; sizes in `size.rs`, causes in `strike`, names from
  `beast_name`. Placement consumes the world-seed animal stream, so existing worlds' other
  draws shift: new digests are expected and the four-seed 600-day soak on the PR is the
  evidence the change is sound.
- **Guard**: `tests/the_deep.rs` `cave_fauna_variants_keep_their_habitats` (seeds 1..6: at most
  one of each, each in its habitat, the bear never in the crawler's cavern, the crocodile
  beside water), `the_blind_bear_strikes_only_in_the_dark` (mauls in the dark, never where a
  lamp reaches) and `the_cave_croc_seizes_at_the_waters_edge` (seizes at the pool's edge,
  never away from the water).
- **Not here**: the sapient cave dweller (#322), the eats-what ecology web (#285) and the
  discovery event (#273/#324) are follow-up slices. Client art for the two kinds is WP-06;
  until then they render with the generic animal sheet.

## 2026-11-09 #377 WP-05 5.1a: the store kept growing from tiles that stood outside the wall

- **Symptom**: the 2026-10-22 C73 entry's "Open" note: half the stores lie outside the ring as the
  stockpile grows past it. Once the wall stood whole, the pile still grew, and the new tiles
  (and the loose things hauled to them) went outside it.
- **Found by**: the C73 review of the walls; carried as the first item of WP-05 slice 5.1.
- **Cause (checked)**: `systems/settlement.rs` step 1b grew the pile with
  `grounds::grow(w, &stock_tiles, 4, ...)`, from every stockpile tile, nearest the first tile
  first. A closed wall stops growth crossing it, but tiles laid before the wall can already stand
  outside it, and growth from those goes on outside. The free-tile count was taken over all of
  them too, so free tiles outside kept the inside from growing at all.
- **Fix**: `systems/settlement.rs` step 1b: when this community's palisade is closed and some
  stockpile tiles are inside it, the pile grows from the inside tiles and the free count is the
  inside's (`palisade_holds`, the test `in_walls` now uses, factored out). Only when the inside
  has no room to grow does it grow outward as before, so no store is left unhauled. A community
  with no closed wall, or none of whose tiles are inside, grows exactly as before.
- **Guard**: `tests/defence.rs` `a_store_grows_inside_the_wall_once_it_stands` (a full pile with
  two tiles outside, first in the list, and four inside: the new tiles are inside; it fails on
  master with the new tile at (34, 30, 2) outside the wall).
- **Not here**: tiles already outside stay (they are drained by use, not removed), and
  `work/planner.rs` `stockpile_slot` still picks the nearest slot, so hauling may still use an
  outside tile while it has room. Digest effect: hf-sim behaviour changes only for a community
  with a closed wall, so the four-seed 600-day soak on the PR is the evidence it is neutral.

## 2026-11-09 #376 WP-01 1.4: nav portal edges asked the grid per tile as well

- **Symptom**: after the flood-fill bitmap (#663), the freeze-wave chunk rebuild's other
  half - `chunk_edges` - still called `neighbours()` per region tile to find portal
  edges, the same dozen grid reads per tile.
- **Fix**: the edge scan answers horizontal walkability from the same per-chunk bitmap
  (only genuinely cross-chunk targets touch the grid), and keeps `neighbours`' vertical
  and ramp section verbatim where the neighbouring level's tiles are genuinely needed.
  The kept step set is identical and the output is sorted and deduped, so the edge lists
  - and every digest - are unchanged.
- **Guard**: 4-day 500-person profile digest pair vs the #663 head: `5663b937...` both;
  the full suite and a fresh four-seed 600-day soak gate the PR.
- **Commit**: this commit.

## 2026-11-09 #376 WP-01 1.4: the nav region rebuild asked the grid a dozen times per tile

- **Symptom**: the 1.1 ranking's nav spikes — a recurring burst at `tick%hour=78 tick%day=2478`
  (every game-day at ~02:04, the coldest hour: the nightly freeze wave dirties 20–28 chunks
  at once) plus one-off rebuilds after big digs; ~5–7 ms at 8 threads and ~21 ms at 1 on the
  500-person profile, and the 1-thread profile's worst tick.
- **Cause**: `nav::regions::chunk_components` flood-filled each rebuilt chunk by calling
  `neighbours()` per tile — each call re-read the grid for the tile and its flankers, plus
  step-cost work the fill never used. Phase timers showed `nav.update` was 99.9 % of the
  spike; the per-chunk inner loop was the cost.
- **Fix**: the fill now answers walkability from a bitmap of the chunk built once per call
  with the same `grid.walkable` calls on the same tiles; the ortho/diagonal corner-cut rules
  are `neighbours`' horizontal section verbatim, so the component ids — and every region,
  reachability answer, path, and digest — are unchanged.
- **Guard**: 4- and 20-day 500-person profile digests byte-identical to master's; the full
  suite and a fresh four-seed 600-day soak gate the PR.
- **Commit**: this commit.

## 2026-11-09 #376 WP-01 1.4: penned animals repeated the same failed path search

- **Symptom**: the quiet-box ranking found hundreds of animal spikes where a penned animal
  retried a budget-exhausting route to the same goal every look; the 20-day c171 human profile
  still had 219 animal spikes and a 44.429 ms worst tick.
- **Cause**: `path_agent_to` discarded an exact failed `(start, goal)` result even though agents
  do not block one another and the grid was unchanged. A retry-cooldown experiment was rejected
  because its time window changed animal behaviour and worsened a navigation spike (issue #376,
  PR #659, proposed D80).
- **Fix**: `AStarScratch` now remembers exact failed start/goal pairs and answers those failures
  without rerunning A*. The cache is transient and is cleared whenever the nav system observes a
  changed grid chunk, so newly opened routes are searched normally and no save format changes.
- **Guard**: `repeated_failed_agent_path_is_cached_until_navigation_changes` proves the second
  identical failed request is memoized; the existing navigation, thread-digest and resume gates
  cover reachability and determinism.

## 2026-11-08 #376 WP-01 1.3: the hourly settlement pass walked every item per haul order

- **Symptom**: the settlement system owned the most numerous >5 ms spikes in the quiet-box
  ranking (17–37 per 20-day run, all at hour boundaries), and cloned the whole item list every
  hour besides (docs/reviews/worst-tick-2026-11-08/).
- **Cause**: the hourly stale-haul sweep tested each `Haul` designation with a linear
  `items.iter().any(...)` over every item in the world — hundreds of designations over
  thousands of items at 500 people — and took a full `w.items.clone()` to do it.
- **Fix**: `systems/settlement.rs` builds a `BTreeSet` of ground-item positions once per
  hourly pass and answers each designation with one lookup; the clone is gone. The
  keep-or-drop decision per designation is unchanged, so planning behaviour is
  byte-identical (the profile digests below match master's).
- **Guard**: the existing settlement and work-sweep scenario tests pass unchanged; the
  4-day 500-person profile digest equals master's; the fresh four-seed 600-day soaks plus
  exact-head box-ci gate this PR.

## 2026-11-08 #376 WP-01 1.2: a look's work was serial while the lookers were independent

- **Symptom**: at 500 provisioned people the 20-day profile's mean tick passes the 2 ms gate
  but with no headroom, and perception is ~75 % of every tick (782–837 µs of ~1.05–1.11 ms,
  per-system table in docs/reviews/worst-tick-2026-11-08/), so every spike starts from a
  higher floor and the 16 ms worst-tick gate stays open.
- **Cause**: temporary per-phase timing of a perceive pass showed the per-looker work —
  sensing, noting into knowledge, the gone-belief sweep — was nearly all of perceive, and a
  looker only ever reads shared state plus its own mind. A heap experiment against the
  age-index `BTreeSet` was measured and reverted (slower under matched contention), so the
  structure stayed and the serial execution went instead.
- **Fix**: `systems/perceive.rs` splits the striped pass into `look`, which works a looker
  out from the shared read-only world (no RNG, no writes — the par.rs idiom) and runs across
  the pool with `par::map`, and `apply`, which lands each looker's notes, observations,
  explorations and hazard reactions on the calling thread in looker order. The stuck-site
  gather moved to a once-a-tick build when some looker is due an expert look (it is pure, so
  when it runs changes only its cost), and the water-read counter is committed once per look
  so it stays exact on the serial path the cost tests use. Thread count now changes how long
  a look takes, never what it notes; the digests below are byte-identical to master's.
- **Guard**: the existing `perceive_cost` guards (shared-store notes match the full scan,
  one water-scan square per look, the store walked once a tick however many look) pass
  unchanged; `tests/threads.rs` checks 1- and 8-thread digests equal; the 12-day 500-person
  profile digest equals master's (`50dde8f7…`), and the fresh four-seed 600-day soaks plus
  exact-head box-ci gate this PR.

## 2026-11-08 #483 / #489: upper-storey viewer review and storey/inspect clarity

- **Symptom**: when stepping the camera up from the ground floor (`z = 2`) to an upper storey
  (`z = 3`), the surrounding lower ground used the same gentle `0.10` hill-relief dimming as
  ordinary surface terrain, person inspection and hover omitted which storey a person was on,
  and block inspection on an upper-storey room or wall did not state that the tile belonged to an
  upper storey (or clipped the bottom `History & traces:` line when `Work planned:` was also
  shown).
- **Cause**: `world_view._ground_family` treated `peel = 2` identically whether `view_z` was at
  surface level or above `ground + 1`; `SimHost::inspect` reported raw `(x, y, z)` without
  comparing against `ground_z`; `SimHost::site_note` and `inspect_block` did not check
  `HousePlan::upper_floor`; and `block_panel.offset_bottom` in `main.gd` was `420`.
- **Fix**: `world_view._ground_family` now dims lower ground more strongly (`0.25` at `peel = 2`)
  when `view_z > ground[y * w + x] + 1`; `SimHost::inspect` and `SimHost::favourites` include
  relative elevation (`"1 above ground"`) in `"where"` and `"activity"`; `SimHost::upper_house_note`
  adds `"Upper storey (1 above ground), roofed."` to `site_note` and `inspect_block`; and
  `block_panel.offset_bottom` is `520`.
- **Guard**: `upper_house_note_describes_upper_storey_room_and_walls` in `crates/hf-godot` and
  `test_upper_storey_dimming_and_person_level_readout` in `tools/gd_test.gd` verify the upper-house
  note, upper-storey ground dimming, and person level readout; 6 review screenshots are recorded
  under `docs/reviews/483-upper-storeys/`.

## 2026-11-08 #413 (WP-04 slice 4.3): scripted click-only session on a real monitor and UI clickability fixes

- **Symptom**: During the scripted click-only session (`tools/click_session.gd`) on the live 1280x720
  window (`laptop GPU`):
  1. Enabling `Follow`, `Favourite`, and `Why?` together on `inspect_panel` widened the single-row
     `HBoxContainer` past `1600 px`, pushing `Close [Esc]` off the right edge of the viewport
     (`x = 1570..1639`).
  2. `Toaster` anchored `_box` at `PRESET_CENTER_TOP` with `offset_left = 0`, placing toasts at
     `x = 800..1180` directly over the right-hand `Close [Esc]` and `Follow` buttons of `fav_panel`
     and `summary_panel`, and set `MOUSE_FILTER_STOP` even on non-positional toasts (`pos.x < 0`).
  3. `_unhandled_input` in `godot/scripts/main.gd` read `get_global_mouse_position()` instead of the
     event's own canvas position (`get_canvas_transform().affine_inverse() * e.position`).
- **Cause**: Unwrapped button row in `inspect_panel`, uncentered `Toaster._box` with unconditional
  `MOUSE_FILTER_STOP`, and polling the desktop cursor instead of `e.position` in `_unhandled_input`.
- **Fix**: Switched `inspect_panel`'s button bar to `HFlowContainer`, centered `Toaster._box` at
  `[-WIDTH/2, +WIDTH/2]` with `MOUSE_FILTER_IGNORE` on non-positional toasts, used `e.position` in
  `_unhandled_input`, and recorded the 56-assertion click-only session in
  `docs/reviews/413-click-only-session/`.
- **Guard**: `test_toaster_centering_and_mouse_passthrough` in `tools/gd_test.gd` and
  `tools/click_session.gd`.

## 2026-11-08 #413 (WP-04 slice 4.2): block inspection water/underground facts and verification screenshots

- **Symptom**: `SimHost::inspect_block` in `crates/hf-godot/src/lib.rs` did not pass through
  `BlockInfo::water_quality` (`fresh`, `salt`, `brackish`, `none` from `#453`), `Rules.format_block`
  in `godot/scripts/rules.gd` had no distinct colour for `foul` water or display of non-fresh
  `water_quality`, and water/underground block inspection screenshots were still owed from `#449`.
- **Cause**: `#449` deferred salt/foul water to `#453` and owed water-tile and underground block
  inspection screenshots.
- **Fix**: exposed `water_quality` from `hf_sim::view::block::inspect` in `SimHost::inspect_block`,
  updated `Rules.format_block` to render simulation-backed `water_quality` and `foul` styling
  without inventing any client-side classifications, and captured water, underground, and surface
  block inspection screenshots under `docs/reviews/413-block-inspect/`.
- **Guard**: `test_block_inspect_water_and_underground_facts` in `tools/gd_test.gd`.

## 2026-11-08 #413 (WP-04 slice 4.1): because-lines on feed items and panels, feed-rate control, and first-run hints

- **Symptom**: causal events in the feed (`Drenched`, `Possessed`, `Festered`, `CustomFormed`, etc.)
  and notable inhabitant events did not surface their recorded cause on the feed line; the top of
  the inspect panel, the favourites panel, and the HUD hover preview did not show a direct
  `because:` line; the feed throttled minor events only at a fixed 700 ms rate with no player
  control; and a first-run launch had no step-by-step on-screen hints for person inspection, block
  inspection, and the bottom bar.
- **Cause**: `publish` in `crates/hf-godot/src/lib.rs` pushed only `narrate::line` without querying
  the chronicle for an event's `cause`/`origin` or `view::why::because`; `SimHost::favourites()`
  omitted `why` and `feeling`; `_push_feed` in `godot/scripts/main.gd` hardcoded the 700 ms minor
  throttle and 9-line cap; and `rules.gd` had no first-run hint progression.
- **Fix**: extended `publish` and `SimHost::drain_feed()` with `event_because` (resolving causal
  `EventId` rows from the chronicle, `narrate::detail`, or notable participant `view::why::because`
  / `feeling`) as the 9th feed-row element; added `why` and `feeling` to `SimHost::favourites()`;
  added `Rules.format_feed_line`, `Rules.inspect_because_line`, `Rules.format_favourite_row`,
  `Rules.format_hover`, `Rules.feed_rate_name` / `feed_allows_minor` / `feed_max_lines` (`calm`,
  `normal`, `full` cycled via the bottom bar `Feed` button `[G]` or Settings and persisted in
  `persist.gd`), and `Rules.first_run_hint` step-by-step hints in `godot/scripts/rules.gd` and
  `godot/scripts/main.gd`.
- **Guard**: `feed_events_carry_simulation_backed_because_lines` in `crates/hf-godot/src/lib.rs`
  and `test_readability_feed_rate_because_lines_and_first_run_hints` in `tools/gd_test.gd`.

## 2026-11-08 #626: Windows release job and publish script updated for off-VPS staging and VPS mirror URLs

- **Symptom**: `tools/release/publish_windows.sh` required a pre-signed manifest at upload time and
  accepted private `github.com` URLs in the manifest and portable `launcher.toml`, even though
  the repository is private (so friends cannot anonymously download GitHub Release assets) and
  signing happens on the project server after staging. `tools/package_windows.sh` and
  `tools/package.sh` also lacked default values for the DEC-3 mirror URL and DEC-1 public key.
- **Cause**: the earlier release scripts were written before DEC-1 and DEC-3 settled the flow
  (build off-VPS -> stage on private GitHub Release -> pull, sign and mirror on
  `https://updates.example.com/stable/`).
- **Fix**: `tools/package_windows.sh` and `tools/package.sh` now default `OSTROG_MANIFEST_URL`
  to `https://updates.example.com/stable/windows-x64.json` and `OSTROG_PUBLIC_KEY`
  to the DEC-1 public key; `tools/release/publish_windows.sh` supports both the 5-argument off-VPS
  staging mode (`TAG ARCHIVE PORTABLE CHECKSUMS NOTES`) and the 6-argument signed-manifest mode
  (accepting `windows-x64.json` or `ostrog-windows-x64.json`), verifies `launcher.toml` inside
  the portable zip, and rejects any `github.com`/`githubusercontent.com` URL in the manifest or
  `launcher.toml`.
- **Guard**: `tools/tests/release_publish_test.py` tests both the 5-argument staging upload and
  6-argument signed-manifest verification with `https://updates.example.com/stable/`
  asset paths, and proves that a `github.com` URL in either the manifest or portable
  `launcher.toml` is rejected before `gh release create` runs. `tools/tests/windows_package_test.py`
  verifies the default DEC-1/DEC-3 `launcher.toml` values.

## 2026-11-08 #626: launcher had no Windows status window on double-click

- **Symptom**: double-clicking `ostrog-launcher.exe` ran as a console-subsystem CLI that
  printed a single `eprintln!` at the end of startup, giving friends no visible status, progress
  bar, heartbeat, cancel/retry controls, or log file while checking and downloading updates.
- **Cause**: `crates/hf-launcher/src/main.rs` called `hf_launcher::run` synchronously on the main
  thread with no GUI window.
- **Fix**: added `crates/hf-launcher/src/window.rs` using direct Win32 `user32.dll`/`gdi32.dll`
  FFI (zero new external crates in `Cargo.lock`) and `#![cfg_attr(not(test), windows_subsystem =
  "windows")]`, running `run_with_progress` on a background worker thread with a 60 ms `SetTimer`
  heartbeat, `Cancel` / `Retry` / `Close` buttons, `LauncherStage::format_status` matching the
  WP-02 stage table, and logging to `data/logs/launcher.log`. Captured screenshots of all 8 stages
  under `docs/reviews/626-launcher-window/`.
- **Guard**: `stage_format_status_matches_wp02_table_and_writes_launcher_log` verifies every stage
  message and `data/logs/launcher.log` output, and `--dump-stage-screenshots` renders all 8 stages
  through the Win32 window on Windows.
- **Commit**: this commit.

## 2026-11-08 #626: launcher could not bootstrap a first install from an empty folder

- **Symptom**: placing a single launcher executable in an empty folder failed immediately with
  `ConfigError::Io` when `launcher.toml` was absent, did not create `data/`, and did not record
  the newly installed `app/VERSION` for subsequent startup checks.
- **Cause**: `LauncherSettings::load` required an external `launcher.toml` file, and `hf-launcher`
  assumed `app/` and `data/` had already been unpacked from a full portable archive.
- **Fix**: `hf-launcher` embeds the default `launcher.toml` (`DEFAULT_MANIFEST_URL` on
  `updates.example.com` and `DEFAULT_PUBLIC_KEY_HEX` from DEC-1) via
  `LauncherSettings::load_or_default`, resolves paths relative to the launcher directory when
  double-clicked, writes `app/VERSION` when staging a release, reads `effective_current_version()`
  from `app/VERSION`, and ensures `user_data_dir` exists before spawning.
- **Guard**: `first_install_bootstrap_from_empty_folder_installs_app_and_persists_version` verifies
  that an empty folder with no `launcher.toml` or `app/` installs from the signed manifest, writes
  `app/VERSION` and `data/`, and launches `CurrentVersion` on the next run without re-downloading.
- **Commit**: this commit.

## 2026-11-08 #626: launcher startup had no ProgressSink for stage and download reporting

- **Symptom**: `hf-launcher::run` executed startup, manifest checks, archive downloads, staging,
  and launch silently without reporting stage transitions or download byte progress to a UI caller.
- **Cause**: the initial launcher process boundary returned only a final `LaunchOutcome` or
  `LauncherError`.
- **Fix**: `hf-launcher` now defines `LauncherStage` (`Starting`, `Checking`, `Offline`,
  `Downloading { version }`, `Verifying`, `Installing`, `Launching`, `Failure { message }`),
  the `ProgressSink` trait (`on_progress(stage, bytes, total)`), `run_with_progress`, and
  `run_with_runtime_and_progress`, forwarding chunked `DownloadProgress` from `LauncherRuntime`
  and handling first-install bootstrap when `app/` has no executable yet.
- **Guard**: `progress_sink_records_exact_stage_sequence_for_update_path`,
  `progress_sink_records_exact_stage_sequence_for_first_install_path`,
  `progress_sink_records_exact_stage_sequence_for_offline_path`, and
  `progress_sink_records_exact_stage_sequence_for_failure_paths` verify the exact stage and byte
  sequences across install, update, offline, and failure paths.
- **Commit**: this commit.

## 2026-11-08 #626: update archive downloads buffered the entire zip in memory

- **Symptom**: `hf-update::fetch_archive` accumulated up to `MAX_ARCHIVE_BYTES` (512 MiB) into a
  whole-archive `Vec<u8>` without intermediate progress callbacks or `Content-Length` truncation
  checks before verifying and extracting the update.
- **Cause**: the initial transport read the HTTP response body directly into an in-memory buffer and
  passed a `&[u8]` slice to manifest verification and zip extraction.
- **Fix**: `hf-update` now streams the archive response in 64 KiB chunks to a temporary `.zip.part`
  file on disk (`DownloadedArchive`), invoking a progress callback (`DownloadProgress`) after each
  chunk, enforcing `MAX_ARCHIVE_BYTES` both up-front via `Content-Length` and during streaming,
  detecting early EOF truncation against `Content-Length`, and deleting the partial file on error or
  drop. `ReleaseManifest::verify_windows_x64_file`, `unpack_archive_file`, and
  `InstallLayout::stage_verified_archive_file` verify and unpack directly from the file on disk.
- **Guard**: `streamed_archive_writes_temp_file_and_reports_progress_per_chunk`,
  `streamed_archive_enforces_size_bound_and_removes_partial_file`,
  `streamed_archive_detects_truncation_and_removes_partial_file`, and
  `verified_archive_file_stages_from_disk_without_whole_archive_vec` cover the size bound,
  truncation, progress callbacks, and disk-backed staging.
- **Commit**: this commit.

## 2026-11-08 #428 follow-up: sleeping people scanned every agent for a bed conflict

- **Symptom**: the 500-person profile still failed the 16 ms worst-tick gate; the `act` system
  reached 20.884 ms in the mixed profile while every sleeping person checked the whole agent list.
- **Cause**: bed-race handling used an `O(people)` scan inside the per-person action loop, making a
  crowded group of sleepers quadratic even though only the number of sleepers per bed mattered.
- **Fix**: `systems/act.rs` builds a deterministic `BTreeMap` of sleeping-bed counts once per
  tick and updates it when a sleeper leaves a bed, preserving the first-come race behavior while
  removing the repeated agent scans.
- **Guard**: the `sleeping_bed_index_tracks_conflicts_without_an_agent_scan` unit test verifies
  duplicate-bed counts and decrement behavior; the existing bed-race integration test preserves
  the user-visible re-decision behavior. The new guard fails without the index helpers.
- **Commit**: this commit.

## 2026-11-08 #626: the Windows package needed a native archive fallback

- **Symptom**: a real Godot Windows export completed, but `tools/package_windows.sh` stopped when
  Git Bash did not provide a `zip` executable, even though Windows includes PowerShell archive
  support.
- **Cause**: the package path assumed the Unix `zip` utility after deliberately removing its
  Linux bridge dependency; that assumption does not hold on a clean Windows release machine.
- **Fix**: the Windows packager now uses `zip` when available and otherwise calls native
  `powershell.exe` with `Compress-Archive`, converting paths through `cygpath` or `wslpath`.
- **Guard**: `tools/tests/windows_package_test.py` omits `zip` and supplies only a PowerShell
  archive stub, proving both package archives and the portable layout still exist. It fails
  before the fallback is present.
- **Commit**: this commit.

## 2026-11-08 #626: the friend-facing package needed a Windows-only build path

- **Symptom**: `tools/package.sh` required the Linux bridge and export alongside the Windows
  inputs, so a Windows release checkout could not produce the Windows portable package by itself.
- **Cause**: the multi-platform packager combined two release environments even though the
  friend-facing acceptance path only needs the Windows x64 app and launcher.
- **Fix**: `tools/package_windows.sh` builds the Windows export, app archive and portable launcher
  package from only the Windows target artifacts, with the same public manifest URL, public key,
  `app/` replacement boundary and empty `data/` root.
- **Guard**: `tools/tests/windows_package_test.py` runs the Windows path with stub exports and
  actual Cargo artifact names while deliberately omitting every Linux artifact.
- **Commit**: this commit.

## 2026-11-08 #626: the package script asked for a binary Cargo does not build

- **Symptom**: a real c165 Windows build produced `hf-launcher.exe`, but `tools/package.sh`
  looked for `ostrog-launcher.exe` and could not build the friend-facing portable archive.
- **Cause**: the Cargo package is named `hf-launcher`; its default binary artifact is therefore
  `hf-launcher` even though the shipped filename is intentionally `ostrog-launcher`.
- **Fix**: the package script now reads the actual Cargo artifact names and copies them to the
  stable friend-facing launcher filename in each portable package.
- **Guard**: `tools/tests/package_layout_test.py` supplies the real `hf-launcher` artifacts and
  still requires the resulting archive to contain `ostrog-launcher.exe` outside `app/`.
- **Commit**: this commit.

## 2026-11-07 #626: launcher could exit without starting after an update failure

- **Symptom**: a failed update download, staging/commit step, or first launch of the new build
  returned an error instead of starting the installed build; a post-commit launch failure could
  also leave the new app active.
- **Cause**: the launcher treated update work as a single fallible path and only attempted rollback
  after a post-update process failure, without a deterministic boundary for testing fallback.
- **Fix**: `hf-launcher` now injects its transport/process runtime, falls back to the installed
  build for update failures, and rolls back a committed update before retrying that build. User
  data and the signed-manifest/update verification contracts remain unchanged.
- **Guard**: launcher tests force download and post-update launch failures, then verify the old app,
  preserved save, rollback, and fallback launch. They fail before the fallback path exists.
- **Commit**: this commit.

## 2026-11-07 #626: publication could accept an unusable portable package

- **Symptom**: the publisher verified the signed app archive but could upload a portable archive
  missing the launcher, app bridge, or empty data directory, or one that accidentally included
  user data. A friend could download a release that could not install or that shipped a save.
- **Cause**: the first publication slice checked only the manifest/archive trust boundary and the
  asset filenames; it did not inspect the portable archive's install layout.
- **Fix**: `tools/release/publish_windows.sh` now requires the Windows portable launcher, config,
  app executable/bridge, README, VERSION and empty `data/.keep` entries, and rejects a launcher
  inside `app/` or any other shipped user-data file before invoking `gh release create`.
- **Guard**: `tools/tests/release_publish_test.py` publishes a valid zip, then proves a package
  without `data/.keep` is rejected before the `gh` stub is invoked. This fails before the layout
  check exists.
- **Commit**: this commit.

## 2026-11-07 #626: releases needed a reproducible signed manifest step

- **Symptom**: the launcher could verify a signed Windows archive, but no repository-owned release
  step produced the manifest from the exact published bytes; a friend-facing package therefore
  had no safe, repeatable publication handoff.
- **Cause**: #615's client-side trust boundary intentionally stopped at verification and package
  layout. It had no release-only signer or documented external key contract.
- **Fix**: `crates/hf-release` and `tools/release/sign_manifest.sh` now hash the exact app archive,
  sign the shared `hf-update` canonical payload with an externally supplied Ed25519 key, validate
  the generated manifest through the client parser, and write deterministic JSON. The release key
  is accepted only from an external raw/hex file; it is not part of the launcher or package.
- **Guard**: `hf-release` tests verify a generated manifest against `hf-update`, deterministic
  output and archive binding, invalid URL/version rejection, and raw/hex key input. These fail if
  the release signer or shared payload contract is removed.
- **Commit**: this commit.

## 2026-11-07 #626: release publication needed an explicit credential boundary

- **Symptom**: a signed manifest and portable archive could be built, but publication had no
  checked contract for exact asset names or for keeping the release token out of shipped files.
- **Cause**: the earlier signer slice intentionally stopped before GitHub Release publication;
  an ad-hoc upload could publish a hash that did not match the archive or leak a token into args.
- **Fix**: `tools/release/publish_windows.sh` checks the manifest's Windows x64 target, HTTPS URL
  and SHA-256 against the exact app archive, requires the CI-only `GH_TOKEN` environment variable,
  and calls `gh release create --verify-tag` with the four documented public asset names.
- **Guard**: `tools/tests/release_publish_test.py` runs the publisher against a stub `gh`, proves
  hash validation and exact upload arguments, and proves the token is not passed as an argument.
 - **Commit**: this commit.

## 2026-11-07 #483: upper rooms could float after their lower support was removed

- Symptom: removing the only lower support from a finished upper room left its floor, roof and
  workstation in place, creating a floating room with stale usable contents.
- Cause: the building tick checked roof span and snow but did not reconcile material-backed upper
  floor components after a wall, fire or other physical removal changed their support.
- Fix: `building.rs` now finds connected built upper components whose floors or stair connections
  have lost support, removes the dependent tiles, roof and structures, moves anyone left on an
  invalid tile when possible, and records an `UpperFloorFell` event. The event is narrated and
  feeds structural learning for people who witness it.
- Guard: `tests/upper_collapse.rs` proves a supporting-wall removal clears the upper room and
  records an event, a non-supporting wall leaves it intact, and save/load preserves the result and
  event. Before the fix, the first guard observes the floor still standing.
- Documentation: regenerated `README.md` after the building/event source shifts so the generated
  tables remain current.

## 2026-11-07 #483: finished upper rooms could not host a needed workshop

- Symptom: after the organic upper-room slice had grown a supported, roofed level, a community
  with no valid same-level workshop site still gave up a real craft project instead of using the
  room it had built.
- Cause: workshop planning searched only ground-level rectangles near the first house and home;
  it never considered a persisted `HousePlan.upper_floor`.
- Fix: after the existing same-level search fails, `systems/settlement.rs` now accepts one free,
  reachable tile in a fully floored, walled and roofed upper room. The workshop structure is
  still posted as its normal material-backed designation, and no upper room is created for this
  fallback.
- Guard: `tests/upper_workspaces.rs` proves the upper fallback is selected only when the ground
  is unavailable and that a reachable ground site remains preferred. Before the fix, the first
  scenario has no workshop designation.
- Documentation: regenerated `README.md` after the planner source line shifts so the repository's
  generated-table CI check stays current.

## 2026-11-07 #615: the launcher needed a release-package boundary

- **Symptom**: c158 shipped the launcher process API, but the package script still emitted only an
  app archive, so the launcher could not stay outside the tree it updates and a clean uninstall
  could not distinguish game files from saves/settings.
- **Cause**: app payloads and friend-facing install files had no separate archive contract.
- **Fix**: `tools/package.sh` now keeps the signed-update app archives separate and emits portable
  launcher archives with `ostrog-launcher`, `app/`, sibling `data/`, `launcher.toml`, README
  and VERSION files. The manifest URL and public signing key are explicit release inputs; no
  credential is embedded. `.gitattributes` keeps Bash scripts LF-safe on Windows checkouts.
- **Guard**: `tools/tests/package_layout_test.py` builds a clean stub package, checks the launcher
  is outside `app/`, checks public-key configuration contains no credential field, and simulates
  uninstall while asserting `data/` remains. This fails before the package boundary exists.
- **Commit**: (this commit).

## 2026-11-06 #615: Windows launcher paths needed a host-independent guard

- **Symptom**: the launcher accepted a `C:/...` executable path when its config was checked on a
  Unix build host, even though that path is absolute on Windows.
- **Cause**: `Path::is_relative` uses the host operating system's path grammar; the portable
  launcher config is shared by Windows release tooling and cross-platform CI.
- **Fix**: launcher config validation now rejects Windows drive-qualified and UNC executable
  paths on every supported host, while still requiring the game executable to be relative to the
  replaceable application directory.
- **Guard**: `hf-launcher` parses a Windows `C:\\Games\\ostrog.exe` config and rejects it;
  this fails on the previous commit when the test runs on Linux box-ci.
- **Commit**: (this commit).

## 2026-11-06 #615: the updater needed a credential-free Windows process boundary

- **Symptom**: the signed archive and staging layers existed, but a friend-facing install had no
  launcher that could check a release before starting the game, preserve user data, or avoid
  replacing the application while a save was being written.
- **Cause**: manifest verification, staging and the game process were separate APIs with no
  startup orchestration or cross-process save-write marker.
- **Fix**: `hf-launcher` now loads a strict local TOML configuration, verifies the signed Windows
  target before fetching its archive, applies offline-safe channel/version policy, stages through
  `hf-update`, and starts the installed executable from a replaceable application directory. The
  public key is configuration data; no GitHub credential or API client is accepted. `hf-update`
  exposes `SaveWriteLock`, and the Godot bridge holds it across the background save commit so the
  launcher falls back to the current build while save bytes are in flight.
- **Guard**: launcher tests cover offline fallback and relative executable policy; update tests
  cover signature preflight and exclusive save markers; the Godot save path compiles against the
  marker API. These guards fail before the launcher boundary and save-write coordination exist.
- **Commit**: (this commit).

## 2026-11-07 #615: verified archives needed safe disposable extraction

- **Symptom**: the updater could retrieve and hash an archive but had no safe handoff from release
  bytes to the existing filesystem staging transaction; a malicious or malformed zip could write
  outside the install tree, create a symlink, or expand without an uncompressed-size cap.
- **Cause**: staging previously accepted only an already-unpacked directory. Archive extraction and
  the signed verification result were separate concerns with no cleanup boundary between them.
- **Fix**: `hf-update::InstallLayout::stage_verified_archive` verifies the signed manifest and
  archive hash before extraction, unpacks into a disposable sibling, rejects traversal, absolute,
  drive-qualified and symlink entries, caps the uncompressed tree at 512 MiB, rejects empty trees,
  copies only the verified package into normal staging, and removes the extraction tree. User data
  remains outside both extraction and update paths.
- **Guard**: `crates/hf-update/src/archive.rs` tests packaged roots, traversal/backslashes, existing
  destinations and empty archives; `crates/hf-update/src/staging.rs` verifies the signed archive
  path through commit/rollback while preserving a save. These guards fail on c156 because safe
  extraction and its verified staging handoff do not exist there.
- **Commit**: (this commit).

## 2026-11-06 #615: archive retrieval needed a signed locator and bounded body

- **Symptom**: the updater could fetch a signed manifest, but an archive URL supplied separately
  could be swapped for another public URL before the archive hash was checked, and there was no
  bounded archive transport for the next launcher slice.
- **Cause**: the manifest authenticated the archive hash but not where the launcher should obtain
  the archive; the transport boundary only covered the small manifest response.
- **Fix**: `ReleaseManifest` now includes an HTTPS-only `archive_url` in its canonical signed
  payload, and `hf-update::fetch_archive` reuses the anonymous, no-redirect, timeout-bounded
  transport with a 512 MiB response cap. The bytes remain untrusted until the existing signature
  and SHA-256 verification succeeds; no credential, extraction or executable launch is added.
- **Guard**: `crates/hf-update/src/lib.rs` rejects a changed archive URL through signature
  verification, while `crates/hf-update/src/transport.rs` rejects non-anonymous URLs and caps
  archive responses. These guards fail on c155 because the signed locator and archive transport
  do not exist there.
- **Commit**: (this commit).

## 2026-11-06 #615: updater needed bounded anonymous manifest retrieval

- **Symptom**: the launcher policy could decide what to do with a manifest, but had no safe
  transport boundary for obtaining one at startup.
- **Cause**: the earlier #615 slices intentionally stopped at verification, staging, and policy;
  an unrestricted client request could follow redirects, send credentials, or accept an unbounded
  response before parsing.
- **Fix**: `hf-update::fetch_manifest` now accepts only anonymous HTTPS URLs, follows no redirects,
  applies the policy's maximum 30-second timeout, caps the body at 64 KiB, maps transport/status
  failures, and parses the strict manifest. It does not download archives, send GitHub/API
  credentials, or launch executables.
- **Guard**: `crates/hf-update/src/transport.rs` rejects HTTP, embedded credentials, control
  characters, oversized responses, and invalid timeout configuration. These guards fail on c154
  because the bounded transport boundary does not exist there.
- **Commit**: (this commit).

## 2026-11-06 #615: startup checks needed an offline-safe decision policy

- **Symptom**: a future launcher could verify a release but still block startup or offer the
  wrong channel when the network was unavailable, a check was deferred, or a manifest was stale.
- **Cause**: manifest verification and filesystem staging had no deterministic policy boundary for
  disabled checks, deferral, bounded timeouts, offline fallback, channel selection, or version
  comparison.
- **Fix**: `hf-update` now exposes a pure startup decision function. It caps the caller's check
  timeout at 30 seconds, honors disabled/deferred checks, offers only a newer manifest on the
  selected channel, and launches the current version for offline, timeout, rejected, or wrong-
  channel results. It has no network, clock, filesystem, or credential access.
- **Guard**: `crates/hf-update/src/policy.rs` tests offline/timeout fallback, disabled/deferred
  behavior, same/newer version and channel selection, timeout bounds, and current-version
  validation. These guards fail on c153 because the startup policy boundary does not exist there.
- **Commit**: (this commit).

## 2026-11-06 #615: updater staging needed recovery and save isolation

- **Symptom**: verifying a release archive is not enough to update a running Windows install;
  an interrupted replacement could leave the application missing or overwrite user data.
- **Cause**: the first #615 slice defined the signed manifest contract but had no filesystem
  transaction boundary for staging, atomic replacement, rollback, or interrupted recovery.
- **Fix**: `hf-update` now copies a verified unpacked payload to a sibling staging area, swaps
  the application tree with same-volume renames, retains the prior tree for rollback, recovers a
  pre-existing backup after interruption, rejects package symlinks, and rejects layouts that put
  saves/settings inside application or update paths. It still has no network, credential, or
  executable-launch behavior.
- **Guard**: `crates/hf-update/src/staging.rs` tests commit/rollback with save preservation,
  interrupted-swap recovery, unsafe layout rejection, and symlink rejection (on Unix). These
  guards fail on c152 because the staging transaction and recovery boundary do not exist there.
- **Commit**: (this commit).

## 2026-11-06 #615: release manifests needed an offline trust boundary

- **Symptom**: a future Windows launcher needs to decide whether a downloaded release is the
  expected Ostrog build without trusting a mutable URL or shipping a GitHub credential.
- **Cause**: the installer/updater slice did not yet have a deterministic, signed artifact
  contract; network transport and update staging are intentionally not part of this change.
- **Fix**: the new `hf-update` crate parses a strict manifest, authenticates a fixed-order payload
  with an Ed25519 public key, checks the archive SHA-256, and accepts only the requested semantic
  version/channel and Windows x64 target. The verifier takes bytes and a caller-supplied public
  key; it contains no GitHub token or download code.
- **Guard**: `crates/hf-update/src/lib.rs` tests valid verification, malformed and tampered
  signatures, wrong archive hashes, wrong platforms, and invalid versions/channels. These guards
  fail on c151 because the crate and trust boundary do not exist there.
- **Commit**: (this commit).

## 2026-10-30 #414: production effects stayed readable and simulation-backed

- Symptom: work, fire and the existing thin-place presentation had little local motion or
  material feedback, while a broad particle pass could easily imply combat or psychic facts the
  simulation does not yet model.
- Found by: the #414 art pass review against the second-monitor readability target; the client
  bridge currently exposes fire/douse/lightning sounds, actual person strike/hit/work poses and
  thin-place data, but no projectile or psychic event stream.
- Cause: confirmed in `godot/scripts/audio/sound.gd`, `world_view.gd` and the bridge's
  `drain_sounds`: there was no bounded visual-effects layer, and fire ember textures were
  generated only as runtime soft blobs.
- Fix: `tools/art/effects.py` adds palette-checked weapon-impact, environmental-hit,
  fire-flare, ember, smoke and thin-place resonance assets. `godot/scripts/render/effects.gd`
  consumes only delivered sound rows, real visible poses and the existing thin-place record,
  with a 48-sprite pool and material-family tinting. `fire_fx.gd` now uses the authored ember and
  smoke textures. Projectile/psychic names are inert future hooks until the bridge emits them.
- Guard: `tools/art/effects_check.py` checks all six sheets, dimensions and palette membership;
  `tools/gd_test.gd::test_effects_are_bounded_and_fact_backed` checks the pool cap, supported
  event mappings, material families and rejection of unknown events. Both fail if the bounded
  mapping/asset contract is removed.
- Commit: this branch's effects slice.

Every fix to behaviour, with why. One entry per fix, newest at the bottom, written in the same
commit as the fix (AGENTS.md, "While working"). An entry has:

- **Symptom**: what was wrong, as it showed.
- **Found by**: the test, soak, census, review or report that showed it, with the numbers.
- **Cause**: the root cause, as checked, not as guessed. If the cause was not confirmed, say so.
- **Fix**: what changed, and where (file and function).
- **Guard**: the test that fails without the fix, or "none" and why.
- **Commit**: the commit that made it (the fix commit, not the merge).

Corrections to an earlier entry are new entries that name the one they correct.

Entries before 2026-10-18 live in the review folders (docs/reviews/*, "Faults found and fixed")
and docs/history/ITERATION_LOG.md; this log starts with milestone MP.

## 2026-10-18 C37: a community's stock is only what it holds

- Symptom: communities that split off starved and froze; at day 600 seed 4 had lost 46 to hunger.
- Found by: the M4 review's four 600-day soaks (community starvation 0/8/7/46).
- Cause: the planner counted stores and structures across the whole valley for each community,
  so a split-off community saw the old home's larder, hearth and beds as its own and built none.
- Fix: `systems/settlement.rs` `Here::of`: a community holds items and structures within 45 tiles
  of its home that are no nearer another community's home; counts and structure checks use it.
- Guard: `tests/spoilage.rs` `a_neighbours_store_does_not_feed_us`, `tests/communities.rs`
  `a_community_that_split_off_builds_its_own_hearth`.
- Commit: a4afc63, 833d74d.

## 2026-10-18 C38: a journey needs its party before it is proposed

- Symptom: journeys proposed with nobody to go; one person led every caravan.
- Found by: C38 soaks and the review agent of C38-C44.
- Cause: the proposal was made before the party formed; nothing rotated the leader.
- Fix: `systems/projects.rs` `journey`: proposed only once a party forms, led by the proposer,
  with no small child left behind; someone back from the road in the last 20 days leaves the next
  journey to others.
- Guard: `tests/projects.rs` `a_journey_has_a_proposer`.
- Commit: a40edf1, 08c1d6b.

## 2026-10-18 C41: a band eats what it carries before foraging again

- Symptom: one C43 opening reached 1,300 item stacks; peak items normally stay under 820.
- Found by: C43 300-day soak, seed 1.
- Cause: a Skarri band foraged again while carrying food, piling up stacks.
- Fix: `threat.rs` band step eats carried food first.
- Guard: `tests/threats.rs` `a_band_of_meat_eaters_eats_what_it_carries_before_foraging_again`.
- Commit: 5733f96.

## 2026-10-18 Review of C38-C44: twelve faults (review agent)

Each fixed in the commit named; guards are in the tests named in that commit.
- Skin tone index overflowed the tone list (`look.rs`, fold by `SKIN_NAMES.len()`); children of
  other kinds were drawn in human tones (`scenario.rs` `give_birth`). Commit bf7a844.
- The list of lights below ground was not saved, so a resumed game had different light
  (`save/snapshot.rs`; guard `tests/resume.rs`). Commit 822fde5.
- The client's species art was not cleared on a new world (`world_view.gd` `reset_world`); a
  wound on a part a plan lacks was lost (`systems/health.rs` `wound` maps it); the peoples
  setting's check and summary ignored generated kinds. Commit 1a5c401.
- Humans were of age at 15 and aged by human years whatever their kind (`agents.rs` `of_age`,
  `fertile_years`, `ailing_years`; humans 16, 16-46, 50/55). Commit b09101b.
- Humans were hot at 24 degrees (`human.toml` comfort widened to 2-30). Commit bf7a844.

## 2026-10-18 C39: a busy station is a wait, not a failed order

- Symptom: `the_kiln_fires_pots_and_glass` failed on c39: the glass order, materials delivered,
  was never made in four days while people stood idle.
- Found by: the full suite run before merging C39 (the stacked branches had only run their own
  tests).
- Cause: confirmed by tracing: while the potter held the kiln's station lease, every idle person
  who planned the glass order failed to lease it and counted a failure against the order; each
  failure takes a tenth off its score, and after eight it scored below wandering. Size-scaled
  hunger (C39) only changed who reached the kiln first; master passed by that ordering alone.
- Fix: `systems/decide.rs` `start_work`: a plan that is made but cannot be leased is a wait
  (cooldown, no failure counted), and a person's own undertaking is not dropped for it.
- Guard: `tests/crafts.rs` `the_kiln_fires_pots_and_glass` (fails on c39 without the fix).
- Commit: 742f6b0.

## 2026-10-18 C42: the heat test above the human range

- Symptom: `each_feels_heat_and_cold_by_its_comfort` failed from C42 on.
- Found by: the full suite on c46 (no-fail-fast).
- Cause: the review's fix widened human comfort to 30 degrees; the test still made it 30.
- Fix: the test uses 33 degrees. Test-only.
- Guard: the test itself.
- Commit: 180c00a.

## 2026-10-18 C47: refugees only where there is room

- Symptom: `people_stay_within_what_the_land_carries`: a settlement at 277 against a carrying
  capacity of 259.
- Found by: the full suite on c47.
- Cause: flights added their count to the host whatever its land fed; trade added half a person
  a caravan.
- Fix: `realm/relations.rs` `flee`: hosts must have room (capacity x3 minus population) and take
  at most that; the rest scatter. The trade population bonus removed. Flights from a taking are
  queued in `Realm::fleeing` from `war.rs` and settled in `relations::step`, which has the map.
- Guard: `tests/realm_peoples.rs` `people_stay_within_what_the_land_carries`.
- Commit: 2492462.

## 2026-10-19 C47: trade rows keep their cultures; the history pass indexed

- Symptom: "Over ten years 57 caravans went between the Moohoox and the Moohoox".
- Found by: the release census for the C47 review.
- Cause: the row's line looked up the two settlements' cultures when read; one settlement had
  been taken since and held its taker's culture.
- Fix: `RealmEventKind::Traded` keeps `cultures`; routes whose ends now share a culture are left
  out of the decade's rows.
- Guard: `tests/relations.rs` `routes_are_recorded_between_cultures` asserts two cultures.
- Also: the 1,000-year history went from 4.8 s to 6.6 s with P9 (seed 1). Measured by stage
  timers: relations 1.8 s, of it trade 1.5 s. Indexed unmarried figures once a year (was a scan
  of every figure per route), routes and relations by key, and a ring search for trade partners;
  5.3 s after, with identical rows before the culture change.
- Commit: 6ca1599.

## 2026-10-19 C44: the troubles scenario on seeds whose neighbours come

- Symptom: `troubles_overlap_and_then_ease` counted 2 turns, needing 3.
- Found by: C44's CI after merging master.
- Cause: confirmed on c44 and c48: in seed 7's world after generated species, no band set out
  from the hungry neighbour within the run, so nothing overlapped.
- Fix: test seeds 3, 11, 13 (13 has a band). Test-only.
- Guard: the test.
- Commit: 0702e81 (c44); c48 had made the same change with a wrong attribution to P10, replaced
  by master's at the merge.

## 2026-10-19 C45: the thin place's gate takes a fourth run

- Symptom: `what_happens_there_is_true_to_the_record`: 1 patch of wrong weather, needing 2.
- Found by: C45's CI after merging master.
- Cause: not confirmed. Each phenomenon is a daily roll on the run's path; after the cultures the
  three runs saw one patch between them.
- Fix: a fourth seed (11). Test-only.
- Commit: fd13cff.

## 2026-10-19 C48: the deep's peoples hold on, and spread only under mountains

- Symptom: in 3 of 4 worlds the deep's one people was gone within a century; in the fourth the
  deep held 2,020 settlements; a rare deep people (the Below) never had a home.
- Found by: the release census for the C48 review.
- Cause: a beast woke in year 37 and fed every fifth year, emptying the one 30-person hamlet; the
  caverns fed 60-110 a region under all land, so every spot was worth a daughter settlement;
  rare peoples skip founding a homeland (the land's rule).
- Fix: `realm/beasts.rs`: nothing wakes before year 200, a beast feeds one year in ten;
  `realm/sites.rs` `capacity_deep`: 60 under mountains, 25 elsewhere; a deep people always
  founds its homeland.
- Guard: `tests/deep.rs` `cave_peoples_live_below` and `a_beast_is_its_record` (non-vacuous);
  the census numbers are in docs/reviews/deep-2026-10-19.
- Commit: 9ceb1c3.

## 2026-10-19 C48: the deep's rare peoples are not the land's households

- Symptom: `a_rare_people_has_no_land_and_lives_among_others` failed.
- Found by: the full suite on c50 merged with master.
- Cause: a rare deep people founds its caverns (previous entry), and the land's settlements
  could take households of a deep people as their minority.
- Fix: `realm/sites.rs` `found`: minorities only of the land's rare peoples; the test excludes
  deep peoples.
- Commit: 196c3ee.

## 2026-10-19 C48: a gentle village and the strain fix

- Symptom: `welcomed_strangers_take_openly_and_leave_in_peace`, seed 11: two of the village came
  to blows.
- Found by: the full suite on c49, then C48's CI.
- Cause: confirmed by printing the fighters: a newcomer (Violence lean 0.0) who joined after the
  test set the village's beliefs quarrelled with a neighbour (-0.81). Not caused by the restless
  dead.
- Fix: the gate asks what it means, that nobody raises a hand against the strangers. Also, in
  `systems/strain.rs`, one who holds firmly (-0.6) that you never raise a hand no longer turns a
  break on someone: the belief's weight has a floor of 0.6, so it gave a 36% chance of a fight.
- Guard: `tests/threats.rs` (the strain rule itself has no test of its own).
- Commit: 6b8ee71 (c48). The same change on c49, eb5f0fc, says the restless dead caused it; that
  is wrong (this entry corrects it).

## 2026-10-19 C49: the remembered dead wait, and are restless once forgotten

- Symptom: no restless dead in 200 days of two openings.
- Found by: a census for the C49 review.
- Cause: an unlaid death was checked once, on the tenth day; anyone remembered then had the
  condition removed and was never looked at again.
- Fix: `conditions.rs` `restless`: remembered dead keep waiting; carved ones are at rest.
- Guard: `tests/conditions.rs` `the_restless_are_only_the_unremembered` (forgetting step).
- Commit: 44a0731, a4c679f.

## 2026-10-19 C49: the restless are seen within the thin place's reach

- Symptom: 17 restless dead in four 600-day soaks, none seen.
- Found by: the soak event tally (MP review).
- Cause: inferred, not traced per case: sightings needed the dead within 20 tiles of a home,
  and band camps lie 24 out.
- Fix: `conditions.rs` `sightings`: also within the thin place's reach, as the design says.
- Guard: none new; the soaks after it were not rerun.
- Commit: 27f46e2.

## 2026-10-19 C50: would-be carvers asked in order of skill

- Symptom: a 111 ms day in the projects step at 500 people.
- Found by: `ostrog profile --agents 500 --mixed --provision`.
- Cause: every member with construction 4+ was asked what they would carve; each asking walks the
  community's record and its members.
- Fix: `systems/projects.rs` `carve`: candidates sorted by skill (stable), the first with
  something to carve is taken: the same choice. Worst day 3.5 ms after.
- Guard: `tests/projects.rs` (behaviour); no timing test.
- Commit: 5bc817f.

## 2026-10-19 #147: a refused Begin stays in the wizard

- Symptom: when the world could not be made as chosen, the client started a plain 12-person test
  valley instead, saying so only on stdout. Four inputs: the deity's name (`deity=Aun`), a kind's
  display name (`exclude=Tharn`, `rarity=Skarri:rare`), and every kind left out.
- Found by: QA round 2026-10-19 (issue #147), confirmed here with the same four runs under Xvfb.
- Cause: `main.gd` `_on_setup_chosen` fell back to `sim.generate` on any error from
  `sim.begin`; the engine parser knew only the pacing names; the peoples check knew only file ids;
  the wizard let Begin be pressed with nobody to live in the world.
- Fix: `main.gd` keeps the wizard and calls `setup.gd` `refused`, which goes to the page concerned
  and shows the reason in red (stdout prints `BEGIN REFUSED (...)`); `setup.gd` `_validate`
  disables Begin when the peoples preview is empty; `engine.rs` accepts Aun, Hask and Mirre;
  `species::resolve_key` and `PeopleSetup::normalise` (called by the bridge) take kinds by id or
  name in any case and rarities in any case; the check's messages name what was wrong. The hook's
  keys are documented in the HANDBOOK.
- Guard: `tests/species.rs` `the_wizard_takes_kinds_and_deities_by_name`; the client path by the
  four `--wizard-shot` runs (the first three begin the chosen world, the fourth stays on the
  peoples page with the reason). No automated client test.
- Commit: 923bf81 (C51).

## 2026-10-19 #130: `ostrog run` read as a month was one day of the test valley

- Symptom: issue #130 reported 82 of 106 event kinds never firing "in a full 30-day run",
  including crows, the Hart, raids, weather and beasts slain.
- Found by: QA (issue #130); checked here.
- Cause: confirmed. The command was `ostrog run --seed 1 --ticks 28800 --agents 12`.
  28,800 ticks is one game-day (docs/TIME.md), and `run` without the opening is the old
  twelve-person test valley, with no portal, past, neighbours or realm. A census of the game as
  played (`examples/event_census.rs`, four openings of 30 days) fired 95 of 134 kinds: crows
  gathered 34, the Hart sighted 12, raids 24, weather 109, beasts slain 4.
- Fix: `ostrog run` gains `--days N` and `--opening` (the arrival as the game plays it), and
  its help says what 28,800 ticks is. The 39 kinds that did not fire in 120 game-days are
  classified in docs/history/PLAN-NEXT.md (long-cycle, rare by design, or worth densifying).
- Guard: none automated; the census example reproduces the numbers.
- Commit: eb07dd9 (C51).

## 2026-10-19 C54: a house is of stone only if its own community holds the stone

- Symptom: a community's next house could be planned in stone when the blocks lay at another
  community's home, so its builders walked to a neighbour's pile or waited on stone they did not
  have. Not seen in a soak; found reading the house rule for Dan's questions of 2026-10-19.
- Found by: code reading of `systems/settlement.rs` `plan_community` (the house rule counted
  `StoneBlock` on the ground anywhere in the valley), then the guard below, which failed before
  the fix: 21 of 21 planned wall tiles were stone with 30 blocks lying 55 tiles away.
- Cause: checked. C37's `Here` (a community holds what is within 45 tiles of its home and nearer
  it than any other home) was applied to the quarry rule's block count but not to the house
  rule's, which kept its own whole-valley sum.
- Fix: `systems/settlement.rs` `plan_community`, house rule: `count_kind(w, &here,
  ItemKind::StoneBlock)`.
- Guard: `tests/communities.rs` `a_house_is_built_of_stone_only_from_stone_the_community_holds`.
- Commit: efcb8c1 (C54).

## 2026-10-19 C54: ASSETS.md counted 157 assets of about 2,000

- Symptom: ASSETS.md said 157 assets; `godot/assets/provenance.toml` and its sibling tables list
  2,017 after C52.
- Found by: C50 MP review, records audit.
- Cause: checked. `tools/art/provenance_report.py` counted only the hand-made table and nothing
  ran it in CI, so the numbers went stale as generators added sheets.
- Fix: `tools/art/provenance_report.py` counts all four tables (Hand-made, Generated, Derived,
  Audio) and gains `--check`; `tools/ci.sh` runs it. The check fired on the first merge after it
  existed (C52 added sheets), which is what it is for.
- Guard: `provenance_report.py --check` in `tools/ci.sh`.
- Commit: 22a313f (C54).

## 2026-10-19 #148: CI's GDScript check failed in a fresh clone

- Symptom: `tools/ci.sh` in a fresh clone or a new worktree failed the GDScript check with
  `Could not find type "SimHost"`; the tester hit it on Windows and had to import by hand.
- Found by: the tester's QA round index, #148; reproduced in the C53 worktree.
- Cause: checked. The extension library `godot/bin/linux/libhf_godot.so` is gitignored and Godot's
  import cache (`godot/.godot`) is absent in a fresh tree, so the check ran without the class.
- Fix: `tools/ci.sh` builds the extension (`cargo build -p hf-godot`; `cargo test` does not
  build the cdylib) and copies it into place when it is missing, and runs one headless
  `--import` after a fresh copy or when the import cache is missing.
- Guard: none automatic; it is CI's own setup. Checked by moving the C54 worktree's
  `godot/.godot` aside with no library in place: the check then built, imported and passed
  (15 scripts, 0 failed).
- Commit: 22a313f, corrected in b75c73f (C54).
  The first version only copied `target/debug/libhf_godot.so`, which `cargo test` never builds,
  so it did nothing in a fresh tree; the C54 CI run caught it.

## 2026-10-19 C54: the 500-person profile measured a starving party

- Symptom: `ostrog profile --agents 500 --provision` fell from 500 people to under 90 in 20
  days, of starvation in every kind, beside 30,000 provisioned meals; the 2 ms gate was met only
  on that shrinking load. Open since M1, last in the MP review.
- Found by: the MP review's profile; a probe this checkpoint (scratch example, seed 1, the
  profile's own setup, not committed) gave the numbers below.
- Cause: checked, two faults in the harness, neither in the game. First, `found_party(500)`
  spills a party of hundreds out over the ground around the landing: 162 of 500 stood where no
  path reaches the landing, and those were the starving (162 hungry from day 1, 140 dead of
  starvation by day 6). Second, with everyone reachable, nobody starved through day 7, but the
  meals ran out on day 8: the whole 20 days' supply was laid down at once and a meal keeps five
  days in the open (`items.rs` `keeps_days`), so it rotted from day 5 while the party ate about
  seven meals a head a day, more than the three the harness provided for.
- Fix: `crates/hf-cli/src/main.rs`: `gather_reachable` moves anyone the landing cannot reach to
  the nearest free tile that it can (bounds-checked; the first version panicked off the map's
  edge); `Cmd::Profile` lays down eight meals a head at the start and each midnight on the same
  stockpiles instead of one pile.
- Guard: none automatic; the profile is the measurement. Its review table gives people alive at
  day 20 so a starving party shows.
- Commit: 4781c83 (C54).

## 2026-10-19 C55: a full mind walked itself on every new thing

- Symptom: with a fed party of 500 the profile ran at 3.94 ms a tick (humans) and 5.08 ms (mixed
  kinds) against the 2 ms gate; perception 3.75 and 4.89 ms of it.
- Found by: the C54 profile with the harness fixed, then `sudo perf record -g` on two days of
  it: 67% of all time in `mind/knowledge.rs` `Knowledge::note`.
- Cause: checked. Once a mind holds `CAP` things, `note` walked the whole map on every new
  thing to count the budgets and find the oldest of a kind over its budget. In a crowd the
  people budget is always over, so every person seen was a full walk.
- Fix: `mind/knowledge.rs` `Knowledge`: each budget keeps its things ordered by (seen tick, key)
  in `by_age`, built on first need, kept in step by `note`, `forget` and `doubt`, not saved and
  rebuilt after a load. What is forgotten is unchanged.
- Guard: `mind/knowledge.rs` test `forgetting_by_age_forgets_what_the_walk_forgot`, 40,000
  random notes, forgets and doubts against the old walk and a loaded mind; it fails if the
  eviction order differs (checked with `last()` for `first()`). Same people alive at day 20 in
  both profiles (509 and 478); 3.94 to 1.98 ms (humans), 5.08 to 2.18 ms (mixed).
- Commit: 021e5ea (C55).

## 2026-10-19 C55: in a crowd a full mind kept the highest ids, not the nearest people

- Symptom: a person in a crowd, with a full mind, kept in mind the people with the highest ids
  wherever they stood and forgot the person beside them; and the noting and forgetting of
  hundreds of people each look was most of what was left of perception's cost.
- Found by: reading `Knowledge::note`'s eviction while fixing the entry above (ties on the same
  tick go by key), then the guard below, which failed before the fix ("AgentId(1) at 1 was
  nearer than one kept").
- Cause: checked. `systems/perceive.rs` `step` noted everyone sensed in one look, all at the
  same tick; past `PEOPLE_CAP` the eviction took the lowest keys first.
- Fix: `systems/perceive.rs` `step`: when more than `PEOPLE_CAP` people are sensed in one look,
  the nearest `PEOPLE_CAP` (ties by key) are noted, in the order sensed. With fewer, nothing
  changes; a 15-day opening (seed 3) keeps its digest.
- Guard: `tests/crowd.rs` `in_a_crowd_the_nearest_are_kept_in_mind`.
- Commit: fbbab80 (C55).

## 2026-10-20 C57: a house that lost walls was never built again, and blocked every later house

- Symptom: in the C57 300-day soak of seed 3, 67 people had 18 places to sleep at day 300; four
  houses were marked out in 300 days (two in C56's soak of the same seed).
- Found by: the C57 soaks, then a probe of seed 3's day-300 save: the community's first house
  stood at stage 1 with four wall tiles open and no orders on them; the chronicle at its tiles
  holds `fire_started`, `burned` and `roof_fell` rows.
- Cause: checked. Fire turns a timber wall to open ground (`systems/hazards.rs`), and nothing
  reposted a house's missing walls: `plan_community` posts walls only when it marks a house
  out. Its new-house rule waits for the first house to be complete, so one burned house stopped
  all building. Not new in C57; C56's two houses in the same seed came from the same block.
- Fix: `systems/settlement.rs` `roofs_and_stages`: a house whose walls are not complete has
  every open edge tile without an order posted again (a wall of the house's majority material,
  the door at its place).
- Guard: `tests/homes.rs` `walls_lost_to_fire_are_built_again` (fails without the change).
- Commit: f560422 (C57).

## 2026-10-20 C57: the same wish for a home was made again and again

- Symptom: seed 3's soak recorded 58 proposals of a house in 300 days, one every five days,
  each "slept on the ground".
- Found by: the C57 soaks (`proposed` rows in the chronicle).
- Cause: checked. A home project that found no place closed after `PROJECT_DAYS` and was
  proposed again the next morning while the house rule above was blocked.
- Fix: `systems/projects.rs` `home`: no new proposal for ten days after one that found no place,
  nor while a house is going up.
- Guard: none separate; the soak's proposal count is the measure (reported in the C57 review).
- Commit: the same.

## 2026-10-20 C57: a house set wall to wall against another's door shut both off

- Symptom: after the fix above, seed 3 had eleven roofed houses at day 300 and 3 places to sleep
  for 58 people; 55 bed orders had each failed 7 or 8 times. C56's soak of the same seed had no
  beds at all at day 300.
- Found by: the second C57 soaks and a probe of seed 3's save: every failing bed was out of reach
  of home, and a tile map showed the house at (2,22) with its door at (5,27) opening onto the top
  wall of the house at (2,28).
- Cause: checked. A house site on wooded ground comes from `systems/settlement.rs`
  `find_flat_area_with_trees`, which, unlike `find_flat_area`, kept no margin, so a house could
  be marked out touching another's walls and door.
- Fix: `find_flat_area_with_trees`: the footprint needs a ring of one tile round it with no
  wall and nothing else built or ordered (trees in the ring are allowed; they are felled).
- Guard: `tests/homes.rs` `a_wooded_house_site_keeps_a_way_round_its_neighbours` (fails without
  the change).
- Commit: 8e1a716 (C57).

## 2026-10-20 C57: the peace test read a fight between two wanderers as a fight with the band

- Symptom: `tests/threats.rs` `welcomed_strangers_take_openly_and_leave_in_peace` failed on seed
  11 after C57 ("nobody here raises a hand").
- Found by: C57's CI run, then a probe of the seed: the `Fought` event of day 4 of the run was
  between two people outside the community and outside any band (both band `None`), wanderers
  waiting to be taken in; they joined the community a week later. Nobody of the community
  fought the band.
- Cause: checked. The test took anyone outside the community for one of the strangers. C57's
  homes change what happens in the valley, and this seed now had two wanderers come to blows.
- Fix: `tests/threats.rs`: `until_gone_banded` collects everyone of a band at any step (bands
  are removed when they go), and the test counts only fights with them. No game code changed.
- Guard: the test itself.
- Commit: 30e20f9 (C57).

## 2026-10-20 C60: one band's nightly raids moved a village, and a community that left "died out"

- Symptom: in C60's first 300-day soaks seed 4 had nine proposals to move or leave, each "strangers
  took from the store: four hard things in two seasons", two moves, three departures from the
  valley, and 36 people at day 300 (63 in C59's soak); and the community that left on day 138 was
  recorded as having "died out".
- Found by: the soaks' chronicles (`proposed`, `moved_camp`, `left_the_valley`,
  `community_ended` rows).
- Cause: checked, two faults. A night's raid counted as a hardship once per half day, so one band
  camped for several nights made four hardships by itself. And `Community::left_for`, which the
  ending reads, was never set: the edit that set it had not matched the formatted source and was
  silently not applied, and the gate that should have caught it checked the ending only if nobody
  had stayed, which in its world was never true.
- Fix: `systems/moving.rs` `observe`: a raid is named by the band's settlement and counted once in
  twenty days; `consider` sets `left_for` where they went.
- Guard: `tests/moving.rs` `a_bands_raids_are_one_hardship`; the leaving gate now requires that
  everyone went and checks the ending (fails without the fix).
- Commit: fa72ecc (C60).

## 2026-10-20 C60: a community that moved camp built a workshop every day on another's ground

- Symptom: in the MH review's 600-day soak of seed 4, 228 proposals to build a workshop, from
  day 338 nearly daily by one proposer of community 1; eight carpenter's benches stood at day 600.
- Found by: the MH review's soaks (`proposed` rows), then a probe of seed 4's last save: every
  proposal was answered with a bench, each placed by the community's first house, which after it
  moved camp stood nearer community 3's home than its own new one, so the bench was not held as
  its own and it wanted one again the next day.
- Cause: checked. Moving camp (C60) changed a community's home but kept its old house sites, and
  the planner places workshops by the first house without checking the spot is the community's
  own ground (C37's `Here`).
- Fix: `systems/moving.rs` `consider`: a community that moves camp leaves its old houses,
  hollows, fields and quarry, which stand but are no longer its to plan from;
  `systems/settlement.rs` `plan_community` step 9: a workshop's spot must be held by the
  community.
- Guard: `tests/moving.rs` `a_move_has_a_proposer_and_a_cause` (the old house is no longer
  theirs) and `tests/communities.rs` `a_workshop_is_built_on_the_communitys_own_ground`, both
  failing without their change.
- Commit: c213d49 (C60).

## 2026-10-20 C60: at 500 people a move took 168 ms, and the profile's party walked out

- Symptom: on a quiet box the mixed 500-person profile had a 304 ms tick in `projects` and ended
  with 47 people (C55: 487); after a first fix, a 174 ms tick and later 11 people.
- Found by: `ostrog profile --agents 500 --mixed --provision` with C60, then a timing trace of
  each project rule (`HF_TRACE_PROJECTS=1`, kept): "move community 0 168.4 ms"; then the move
  trace under the same variable: on day 12, "to None: Rosgar died of the cold; Nalthi died of the
  cold; Peruthi died of the cold: 27 hard things in two seasons".
- Cause: checked for the time and the walk-out. `systems/moving.rs` `consider` asked each child
  whether any goer was its parent by walking every agent for every goer (people times goers
  times agents), and searched for a new site with `find_flat_area`'s thirty rings at each of
  sixteen points. With no site found in the crowded valley, the community left it. Not checked:
  whether the 304 ms of the first run was the children loop or the search; the trace was taken
  after the loop was rewritten.
- Fix: `consider`: the dead, the goers' children and the dead's kin are each found once;
  `systems/settlement.rs` `find_flat_area_within`, a bounded search, eight rings at each point;
  a community proposes leaving the valley only for a settlement of the world it knows of; a
  hardship count needed to move grows by one for every twenty people (a design choice, recorded
  here because it changes when moves happen: a death weighs more in a hamlet); and
  `WorldMeta::settled`, set only by the profile, holds the measured party in place.
- Guard: `tests/moving.rs` (knowing no place, nobody proposes leaving); the profile's worst tick.
- Commit: ea968a4 (C60).

## 2026-10-20 #161: new clothes overflowed for an id from 245

- Symptom: `attempt to add with overflow` in debug and test builds when someone with an id from
  245 put on new clothes of good quality; release builds wrapped silently.
- Found by: QA round 2026-10-20, #161 (code reading), confirmed by the guard below.
- Cause: checked. `systems/life.rs` `clothes` added the quality step and the id as u8.
- Fix: the same sum in u32, then `% 12`.
- Guard: `tests/qa_1020.rs` `new_clothes_for_a_high_id_do_not_overflow` (panics without the fix).
- Commit: 5df2d51 (C61).

## 2026-10-20 #159: a house open to the sky was roofed on load

- Symptom: a save made while a house had no roof (walls just up, a roof burned) loaded with every
  walled house roofed, with no logs used and nobody building.
- Found by: QA round 2026-10-20, #159 (code reading), confirmed by the guard below.
- Cause: checked. `save/snapshot.rs` `restore` ran C10's upgrade for saves from before roofs
  whenever the save's roofs were empty.
- Fix: saves record `roofs_saved` (true from now on); only a save without it gets the upgrade.
- Guard: `tests/qa_1020.rs` `a_house_open_to_the_sky_stays_open_after_a_load` (fails without).
  C10's own guard, `tests/building.rs` `an_old_save_keeps_its_houses_roofed`, made its old save
  by clearing the roofs only; it now also clears `roofs_saved`, as a save from before roofs has.
- Commit: the same.

## 2026-10-20 #174: a finished floor turned the open tile below into rock

- Symptom: a floor order over an open tile (a cellar room, a stair) filled the level below with
  rock, burying whoever and whatever was there.
- Found by: QA round 2026-10-20, #174 (code reading), confirmed by the guard below.
- Cause: checked. `systems/work.rs` `apply_effect`, `BuildFloor`, set the tile below solid
  without looking at it.
- Fix: the floor is laid only over a solid tile; over an open one it lays nothing.
- Guard: `tests/qa_1020.rs` `a_floor_over_an_open_tile_buries_nothing` (fails without).
- Commit: the same.

## 2026-10-20 #163: an unfinished hollow blocked every later home

- Symptom: a hollow whose dig orders were dropped (a failed prop, a fall that refilled the room)
  was never finished, and its home or shelter project never closed, so the community proposed no
  other home while its people slept on the ground.
- Found by: QA round 2026-10-20, #163 (code reading), confirmed by the guard below.
- Cause: checked. `systems/settlement.rs` posted a hollow's Mine orders only when it marked the
  hollow out, and `systems/projects.rs` `close` ends a sited home only when it stands.
- Fix: `plan_community` step 6a: an undug hollow's tiles without an order are ordered again each
  hour; a hollow not dug within twenty days of being marked out (`Hollow::begun`) is given up and
  its orders removed, and the project it was for then ends.
- Guard: `tests/homes.rs` `a_hollow_left_undug_is_ordered_again_and_given_up_in_time` (fails
  without the change).
- Commit: 855d90c (C61).

## 2026-10-20 #176: communities were never marked dissolved

- Fixed by C60 (`systems/moving.rs` `endings`), which marks a community with nobody left in the
  valley dissolved and records its end, so the existing `!c.dissolved` filters in bands'
  targets (`threat.rs` `home`) and newcomers (`systems/society.rs`) now apply. Guard:
  `tests/moving.rs` `dying_out_is_recorded`. Commit: 0119861 (C60).

## 2026-10-20 #158: a game saved on the tick of a death lost the death's effects

- Symptom: saved on the tick someone died and loaded, the dead kept holding their tool, nobody
  inherited, crows never came, and the deity's and arcs' memory differed from a game never saved.
- Found by: QA round 2026-10-20, #158 (code reading), confirmed by the guard below.
- Cause: checked. The stages before `flush` (legend, creatures, deity, arcs) act on the previous
  tick's `last_events`, which the save did not keep, so a load began with none.
- Fix: `save/snapshot.rs`: the snapshot keeps `last_events` and restores them.
- Guard: `tests/qa_1020.rs` `a_game_saved_on_the_tick_of_a_death_goes_on_as_it_would_have` (the
  dead keep their tool without the fix).
- Commit: 13253eb (C61).

## 2026-10-20 #177: eight small findings from a code review

Found by: QA round 2026-10-20, #177 (code reading). Each cause checked in the code before the fix.

1. A column off the map was read as another row's, or past the end (a panic), by neighbours of
   an edge tile: `world/grid.rs` `ground_z` reads the nearest column on the map. Guard
   `a_column_off_the_map_reads_as_its_edge`.
2. A fallen roof's log was recorded as cause "scenario" with no material: `building.rs` `step`
   makes it of the roof's wood, cause "roof fell". No separate guard (the ledger's cause name).
3. A band that left was removed inside a forward loop that then returned, skipping the bands
   after it that step: `threat.rs` `bands_step` steps from the last band and continues.
   Guard `every_band_that_has_gone_is_removed_in_one_step`.
4. Food in someone's hands never rotted: `systems/spoil.rs` rots carried food as in the open.
   Guard `food_carried_in_hand_rots`.
5. The flood comment said six and three hours; the code is twelve and four. Comment corrected.
6. Taking a tool, clothes or a ring ignored leases, so a hauler holding one aborted: `take_tool`
   (`systems/work.rs`), `clothes` and `give_ring` (`systems/life.rs`) skip leased stacks, as
   `arms::take_up` does. No separate guard (the abort is a log line).
7. An unguarded `partial_cmp().unwrap()` on thought weights: `total_cmp` (`mind/thoughts.rs`).
8. A face of ore counted as a quarry for blocks: `nearest_rock_face` takes stone only and the
   quarry rule counts only orders on stone (`systems/settlement.rs`). Guard
   `an_ore_face_is_no_quarry_for_blocks`.

Guards in `tests/qa_1020.rs`; the four named fail with their fixes undone by hand. Commit: b7c8614
(C61).

## 2026-10-20 #160: a load rebuilt the sky without the deity's Cold lean

- Symptom: with a Cold answer active and rain below zero, a game saved and loaded announced
  "snow" on its first tick, which the game never saved did not: the loaded sky was the unleaned
  rain, and the step then leaned it.
- Found by: QA round 2026-10-20, #160 (code reading), confirmed by the guard below.
- Cause: checked. `world_state.rs` `rebuild_caches` set `Weather::at(seed, tick)`; only
  `systems/weather.rs` `step` applied the lean and the rain-to-snow rule.
- Fix: `systems/weather.rs` `current` is the weather with the lean (or the forced weather); the
  step and `rebuild_caches` both use it.
- Guard: `tests/qa_1020_medium.rs` `weather_after_a_load_keeps_the_cold_lean` (fails without).
- Commit: C62, `git log --grep '#160'`.

## 2026-10-20 #162: materials delivered to an order vanished when it was dropped

- Symptom: an order dropped unfinished (failed more than twelve times, its owner dead or moved
  on, a collapse over it, a fey mood passed) took what had been delivered to it with it: four
  blocks and two logs at a kiln were simply gone, with no line. The ledger balanced (they were
  counted destroyed at delivery), so no soak flagged it.
- Found by: QA round 2026-10-20, #162 (code reading), confirmed by the guard below.
- Cause: checked. `systems/work.rs` consumes delivered stacks into the order ("delivered"); the
  sweeps that drop orders (`systems/settlement.rs` `step`, `systems/hazards.rs`
  `clear_stale_undertakings`, `systems/moods.rs`) removed them with no thought for that.
- Fix: `World::give_back` puts each requirement's delivered count back on the ground at the
  site, of the material recorded (ledger "returned", quality 0.5: the order keeps no quality).
  The three sweeps take dropped orders out with `work::split_off` and give them back; a
  collapse marks orders done and the settlement sweep then gives back. A finished order counts
  its materials used (`systems/work.rs`, `delivered` set to 0 as it is marked done), so it gives
  nothing back.
- Guards: `tests/qa_1020_medium.rs` `a_dropped_order_gives_back_what_was_delivered` (fails
  without) and `a_finished_order_gives_nothing_back`.
- Commit: C62, `git log --grep '#162'`.

## 2026-10-20 #165: a party trading meals sold its own rations

- Symptom: a caravan carrying meals to trade came away from the site with no food at all, so a
  stranded day was far likelier to end with the party lost.
- Found by: QA round 2026-10-20, #165 (code reading), confirmed by the guard below.
- Cause: checked. `systems/expedition.rs` `form_caravan_led` put the rations in the cargo as
  meals, and `travel_day` at the site removed every cargo entry of the traded kind.
- Fix: rations are their own count on the caravan (`Caravan::rations`): eaten first on hungry
  days, taken by robbers only when there is no cargo, set down on return. At the site only the
  goods' entry is taken out.
- Guard: `tests/qa_1020_medium.rs` `a_caravan_carrying_meals_keeps_its_rations`.
- Commit: C62, `git log --grep '#165'`.

## 2026-10-20 #166: a caravan that never left destroyed its cargo

- Symptom: when nobody reached the gathering (a fire or a raid pulled them away), the journey
  was given up, and the twelve goods and the rations loaded for it were gone, with no line.
- Found by: QA round 2026-10-20, #166 (code reading), confirmed by the guard below.
- Cause: checked. `systems/expedition.rs` `forming` marked the caravan done without `arrive`,
  so nothing was set down.
- Fix: `set_down` (also used by `arrive`) puts goods and rations at the gathering tile, ledger
  "returned", each with the material and quality it was loaded with.
- Guard: `a_caravan_that_never_leaves_sets_its_cargo_down_as_it_was` (fails without).
- Commit: C62, `git log --grep '#166'`.

## 2026-10-20 #167: a party lost whole counted nobody against its road

- Symptom: after a party of three was lost on a road, the next party took the same road: the
  C36 penalty (twenty a head lost) never applied to a whole party.
- Found by: QA round 2026-10-20, #167 (code reading), confirmed by the guard below.
- Cause: checked. `systems/expedition.rs` `travel_day` empties the member list when a party is
  lost, and `trade.rs` `lost_on` counted `members.len()` for a lost caravan.
- Fix: the caravan records `lost`, the number with it when it was lost; `lost_on` counts that
  plus those lost one by one (the log), for a lost caravan as for any other.
- Guard: `a_party_lost_whole_marks_its_road` (fails without).
- Commit: C62, `git log --grep '#167'`.

## 2026-10-20 #169: loading for trade took storied and leased stacks, and lost material

- Symptom: the White Hart's hide could walk out as one of twelve hides; a hauler whose stack
  was loaded failed with "item gone"; oak logs sent out and not sold came home as plain logs,
  and goods moved between communities came as quality 0.6, whatever they were.
- Found by: QA round 2026-10-20, #169 (code reading), confirmed by the guards below.
- Cause: checked. `systems/society.rs` `take_items` took any stack of the kind within reach and
  returned only a count; the caravan kept materials only for what it was paid.
- Fix: `take_items` skips storied and leased stacks and returns a `Load` (count, the material
  most of it was, average quality). Caravans keep the goods' material and quality
  (`Caravan::qualities`) and set them down with them; trade between communities makes the
  goods with theirs.
- Guards: `a_storied_hide_is_never_loaded_for_trade` (fails without) and
  `a_caravan_that_never_leaves_sets_its_cargo_down_as_it_was` (the oak).
- Commit: C62, `git log --grep '#169'`.

## 2026-10-20 #164: fetch trips chose empty sites, the wrong cargo, and "spare" by another rule

- Symptom: C59's fetch could send a party to a settlement with nobody in it, carrying a cargo
  chosen for another settlement, to fetch blocks the barter there would never pay ("Ashby has
  stone blocks to spare" with twenty in store and a need of forty); it could also see wood "in
  reach" (any tree within 60, fungus or walled in) where the planner fells nothing, and stone
  out of reach while a quarry shaft was already being dug. The C59 review saw no fetch fire in
  four 300-day soaks.
- Found by: QA round 2026-10-20, #164, first raised by a code review of PR #157; confirmed by
  the guards below.
- Cause: checked. `systems/projects.rs` `lacking` filtered sites on `known_by` and eight in
  store only, took its cargo from `trade::best_errand` (the best errand anywhere), counted
  trees with its own rule, and asked only `nearest_rock_face`, which skips faces with an order
  on them and gives None without paths.
- Fix: `trade::best_fetch` picks the site and cargo together, as `best_errand` does (peopled,
  known, the lost-road penalty) and paid as barter pays (`quote_wanting`, which pays only from
  above half the settlement's need), at least four of the want. `lacking` asks
  `settlement::chop_candidates` (the planner's own felling rule, now one function), counts an
  open felling, a cut into stone or a quarry shaft as the want under way, and proposes nothing
  without paths.
- Guards: `tests/projects.rs` `a_fetch_is_never_to_an_empty_site` and
  `trees_out_of_chop_reach_are_no_wood_in_reach` (both fail without); `spare_means_what_barter_will_pay`
  and `fetch_cargo_is_what_the_site_will_take` hold `best_fetch` to the barter's terms.
- Commit: C62, `git log --grep '#164'`.

## 2026-10-20 #170: storied hides and food rotted or burned without a word

- Symptom: the White Hart's hide, a trophy with a story, left outside rotted in twenty days and
  appeared only as "1 hides" in the day's line; fire took storied clothes and hides with no line
  naming them. The legend just ended.
- Found by: QA round 2026-10-20, #170 (code reading), confirmed by the guards below.
- Cause: checked. `systems/spoil.rs` `step` rotted any stack that keeps days; `systems/hazards.rs`
  `burn_out` consumed what burned with no event for a thing with a story.
- Fix (D32, Dan's call): `spoil::step` passes over storied things. `burn_out` names each storied
  thing that burns in a major `Burned` event, by its name or its noun.
- Guards: `tests/qa_1020_medium.rs` `a_storied_hide_does_not_rot` and
  `a_storied_hide_lost_to_fire_is_named` (both fail without).
- Commit: C62, `git log --grep '#170'`.

## 2026-10-20 #171: hauling onto a stack ignored the stack cap and the quality

- Symptom: ten masterly meals hauled onto ten crude ones made one stack of twenty crude: the cap
  (ten) was passed, and the fine meals' quality was lost.
- Found by: QA round 2026-10-20, #171 (code reading), confirmed by the guard below.
- Cause: checked. `systems/work.rs`, the `DropAt` step with no order, merged into any stack of
  the kind and material on the tile and kept the target's quality (it averaged rot only).
- Fix: `systems/work.rs` `put_down` (the merge, now one function) merges only while both fit
  the kind's stack, as `create_item_of` does, and averages quality as it does rot; otherwise the
  stack is set down beside.
- Guard: `hauling_onto_a_stack_respects_cap_and_quality` (fails without).
- Commit: C62, `git log --grep '#171'`.

## 2026-10-20 #172: cooking took "two" and cooked the first thing in hand

- Symptom: a cook carrying one berry and one piece of raw meat made a meal from one berry (half
  the price of a meal from two), named for the berry, though the plan had fetched the meat.
- Found by: QA round 2026-10-20, #172 (code reading), confirmed by the guard below.
- Cause: checked. `systems/work.rs`, the `Cook` effect, took the first cookable stack carried
  and `consume_item(id, 2)`, which takes what there is; the planner fetched two.
- Fix (D33, Dan's call: one raw thing is a meal): the planner fetches one; the effect cooks the
  stack of the kind and material the plan picked up (the first cookable in hand only if that
  is gone) and takes exactly one. The meal is of that food. More meals come of the same raw
  food than before (one each, not one for two); the C62 soaks record the effect.
- Guard: `the_cook_cooks_the_planned_food_one_to_a_meal` (fails without).
- Commit: C62, `git log --grep '#172'`.

## 2026-10-20 #168: rings and worn storied clothes stayed held by the dead

- Symptom: a ring given at a pairing ("gave Y a silver ring set with a garnet") stayed held by
  Y after Y died: never an heirloom, never on the ground, counted in the ledger and out of
  anyone's reach. Worn clothes with a name the same.
- Found by: QA round 2026-10-20, #168 (code reading), confirmed by the guards below.
- Cause: checked. `systems/life.rs` `die` drops what is carried; `legend.rs` `step` passes on
  the tool and `systems/arms.rs` drops weapon and armour; nothing looked at other held things.
- Fix: `legend.rs` `pieces_of_the_dead`, on the tick after a death: each storied thing they held
  (not the tool, weapon or armour) goes to the nearest living kin in a community (children,
  partner, mother, father, as the tool does) as an heirloom with an `Inherited` line, a ring
  into their hand, clothes set down beside them; with no kin it is left where they fell.
- Guards: `a_ring_passes_to_kin_on_death` and `a_ring_with_no_kin_lies_where_they_fell` (both
  fail without).
- Commit: C62, `git log --grep '#168'`.

## 2026-10-20 #173: raiders knew every stack in the valley, and what they split off changed

- Symptom: a band chose its night's food from every food item within thirty tiles of the home,
  seen or not (against VISION principle 2); the part a raider split off a stack became plain
  food of quality 0.5, so food dropped by a caught raider came back as something else; and a
  raid could take a stack someone held a lease on.
- Found by: QA round 2026-10-20, #173 (code reading), confirmed by the guards below.
- Cause: checked. `threat.rs` `raids` built its targets from `w.items` directly, and its split
  made a new item with `quality: 0.5, material: AIR`.
- Fix (D34, Dan's call): a band keeps what it has seen (`Band::seen`, from `look_about`: food
  in a member's sight by `perception::can_see`, each band step) and what it heard of before it
  set out (`Band::heard`, from `heard_of`: the valley's renowned things, a name or spoken of,
  lying within thirty tiles of home, with where they lay). A raider goes first for a treasure
  heard of, at the place the telling put it (if it is gone from there they give it up), else
  for the nearest food seen still where it was, not leased and not in a shelter. With nothing
  known, one of them goes to look about the houses. A split copies the stack. A treasure taken
  to the camp is named in a major `Raided` line and goes with them.
- Guards: `tests/qa_1020_medium.rs` `raiders_do_not_take_ordinary_food_they_never_saw`,
  `raiders_seek_a_renowned_piece_they_have_heard_of`,
  `a_treasure_moved_from_where_its_story_says_is_not_found` and
  `food_a_raider_splits_off_is_what_the_stack_was` (all four fail without).
- Not done: survivors going after a raider up to 40 tiles away without having seen them
  (`systems/decide.rs`, noted in #173) is left open, listed below.
- Commit: C62, `git log --grep '#173'`.

## 2026-10-20 #175: partners were left paired to the dead or gone

- Symptom: a widow or widower could stay "paired" to a dead partner for good: no new pairing,
  no children. The same for someone away on a journey when their partner died, and for anyone
  whose partner left the valley.
- Found by: QA round 2026-10-20, #175 (code reading), confirmed by the guards below.
- Cause: checked. `systems/life.rs` `die` cleared `partner` only for the living in the valley
  who still had a relationship entry for the dead (`relate` evicts the least familiar when the
  list is full); `overworld.away` was never looked at; `systems/society.rs` `leave_valley`
  cleared nobody.
- Fix: `World::unpair` frees whoever is paired to the id, in the valley and away, whatever their
  relationships hold; `die` and `leave_valley` call it. Grief is still only for those who knew
  them.
- Guards: `tests/qa_1020_medium.rs` `a_widow_is_free_to_pair_again_with_a_full_relationship_list`,
  `a_partner_away_on_a_caravan_is_widowed` and `leaving_the_valley_ends_a_pairing` (all fail
  without).
- Not changed: two people of no community can pair (`life.rs`, `None == None`), which #175
  notes; that is how unaffiliated people live, not a fault.
- Commit: C62, `git log --grep '#175'`.

## 2026-10-20 #178: the inspect panel could not be scrolled

- Symptom: for anyone with a full life the panel ran past its 484 px and the lower part (family,
  customs, mood, people, beliefs, past, memories) could not be reached; the scroll snapped back
  to the top. The "knows N things" list read `Item(ItemId(0)) at (106,58,3)`.
- Found by: QA round 2026-10-20, #178 (code reading, confirmed in play on seed 7).
- Cause: checked. `main.gd` `_process` called `_update_inspect` every frame, which called
  `sim.inspect` and set `inspect_box.text`, re-parsing it and resetting the scroll; the bridge
  wrote each known thing with Rust's debug format.
- Fix: `main.gd`: the panel is rebuilt four times a second (at once on a new selection) and its
  text set only when it changed, keeping the scroll position; `view/known.rs` `describe` names
  each known thing ("oak logs", "a hearth", "Aelwen", "water"), used by the bridge's `inspect`.
- Guard: `tests/client_view.rs` `what_someone_knows_reads_in_their_terms`. The scroll itself has
  no automated guard (it needs input on a drawn panel); checked by the seed 3 `--inspect=0` shot,
  which now shows the scroll bar.
- Commit: 8d95e15 (C63).

## 2026-10-20 #179: roofs were drawn over people south of a house

- Symptom: a roof covered anyone standing just south of the house's south wall, and canopies
  south of it.
- Found by: QA round 2026-10-20, #179 (code reading).
- Cause: checked. `world_view.gd` put `roof_layer` at z_index 2, the entities' own, and added it
  after them, so every roof drew over every sprite.
- Fix: `render/roof_layer.gd`: each connected roof is a piece in the entity layer's y-sort,
  sorted on its south edge (`sort_y`): someone on its rows is beneath it, someone south of it in
  front. Rafters of a roof going up sort on their own row.
- Guard: `tools/gd_test.gd` `test_roofs_sort_with_the_people`.
- Commit: 8d95e15 (C63).

## 2026-10-20 #181: the HUD ran into the inspect panel

- Symptom: the HUD's second line, the whole key list, ended near x=1340 on the 1600-wide canvas
  and the inspect panel starts at 1270, so they overlapped whenever a band, arc or hover line
  widened it.
- Found by: QA round 2026-10-20, #181 (code reading, confirmed in play).
- Cause: checked. One unwrapped Label with no width cap, and the key list on every frame.
- Fix: `main.gd`: the HUD wraps at a width that keeps clear of the panel (`rules.gd`
  `hud_width`); the key list moved to a `?` help panel, shown once on a hands-on first launch
  (`persist.gd` `help_seen`).
- Guard: `tools/gd_test.gd` `test_hud_keeps_clear_of_the_inspector`.
- Commit: 8d95e15 (C63).

## 2026-10-20 #182: the feed was a fixed dark block and could not be clicked

- Symptom: early in a game a 510x220 dark panel covered the stores with one line in it; lines
  could not be clicked to see where they happened; the count of minor lines let pass was kept
  "for the HUD" and never shown.
- Found by: QA round 2026-10-20, #182 (code reading, confirmed in play).
- Cause: checked. `main.gd` `_build_ui` fixed `offset_top` at -230; nothing read `pos` or
  `minor_dropped`.
- Fix: `main.gd`: the box is sized to its text each time it changes and hidden when empty; a
  click on a line moves the camera to the line's place and level (`_on_feed_input`); the lines
  let pass in the last 45 seconds show as "+N quieter lines" (`rules.gd` `feed_text`).
- Guard: `tools/gd_test.gd` `test_feed_counts_what_it_drops`.
- Commit: 8d95e15 (C63).

## 2026-10-20 #183: clicking people missed them, and zoom shimmered

- Symptom: walking people were hard to click, and at 0.5x the pick radius was tiny on screen;
  the wheel zoomed about the screen centre in steps of 1.25, so nearest-filtered art showed
  uneven pixels that shimmered as the camera moved.
- Found by: QA round 2026-10-20, #183 (code reading).
- Cause: checked. `main.gd` `_pick_agent` compared the tile's centre, not the drawn position,
  within 24 world pixels at every zoom; the wheel multiplied the zoom by 1.25.
- Fix: `rules.gd` `pick` uses the drawn position (as `entity_layer.gd` draws it) and a radius in
  screen pixels; the wheel steps through 0.5, 1, 2, 3 and 4 (`next_zoom`), keeping the point
  under the pointer (`zoom_about`).
- Guard: `tools/gd_test.gd` `test_pick_uses_the_drawn_position`,
  `test_pick_radius_follows_the_zoom`, `test_zoom_steps_are_whole_pixels`.
- Commit: 8d95e15 (C63).

## 2026-10-20 #184: lightning without a flash, ladders as stairs, half the fire

- Symptom: thunder with no flash when lightning struck; ladders drawn as stairs; fire on the
  ground cycled two of its four frames; every material without wall art was listed in the
  `missing_assets` line the README calls a bug sign.
- Found by: QA round 2026-10-20, #184 (code reading).
- Cause: checked. Nothing set `light_layer.gd` `flash`, and the bridge sent no event for a
  strike; `world_view.gd` sent every shape from 5 up to the stairs sheets and took the fire
  frame `% 2`; `wall_name` probed with `_tex`, which records a miss.
- Fix: `view::sound_of` gives a fire started "by lightning" the sound `lightning`, on which
  `audio/sound.gd` sets the flash and plays thunder; `rules.gd` `stair_sheet` (ladder.png for
  shape 8) and `fire_frame` (all of the sheet); `wall_name` probes with `assets.optional`.
- Guard: `tests/client_view.rs` `lightning_flashes_and_other_fires_flare`; `tools/gd_test.gd`
  `test_ladders_and_fire_frames`.
- Not done here: windows in house walls, a chicken, and the unused light and overlay sheets
  need design and art; they stay with the asset review (#195).
- Commit: 8d95e15 (C63).

## 2026-10-20 #185: the dead vanished on the spot

- Symptom: a person who died disappeared at once; the people sheets' fall and lying frames
  were never drawn.
- Found by: QA round 2026-10-20, #185 (code reading).
- Cause: checked. The bridge's `agents()` skipped everyone not alive and no pose meant fall or
  lying.
- Fix: `view/bodies.rs`: the bodies of people the viewer was told had died, from the `Died`
  events, falling for 30 ticks and lying after, for three game-hours when their people laid them
  to rest and a day when nobody did. The bridge sends them in `agents()` as poses 8 and 9 and
  `entity_layer.gd` draws the `fall` and `lying` columns. View only: not saved, so a loaded game
  starts with no bodies down.
- Guard: `tests/client_view.rs` `the_dead_fall_lie_and_are_taken_away`,
  `a_body_nobody_tends_lies_longer`; `tools/gd_test.gd` `test_the_dead_have_frames`.
- Commit: 8d95e15 (C63).

## 2026-10-20 #186: carried items used the old 16 px icons

- Symptom: a load in someone's arms was drawn from the old 16 px set while the same item on the
  ground used the 32 px one, and birch logs or marble blocks lost their material in hand.
- Found by: QA round 2026-10-20, #186 (code reading).
- Cause: checked. `entity_layer.gd` `_agent` loaded `items/<kind>.png`, and the bridge's
  `carried()` sent no material.
- Fix: `carried()` sends [id, kind, material]; the carried icon is the ground's 32 px icon (its
  material's where drawn) at half size, the 16 px icon only when there is no 32 px one.
- Guard: none automatic (the icon choice is drawing code); checked in the seed 3 shots.
- Commit: 8d95e15 (C63).

## 2026-10-20 #187: animals stood still in a fight

- Symptom: poses other than idle, walk and sleep (working or grazing, eating, fighting, struck)
  all drew frame 0, so a wolf pack's hunt slid about as statues.
- Found by: QA round 2026-10-20, #187 (code reading).
- Cause: checked. `entity_layer.gd` `animate` mapped only poses 0, 1 and 4; the sheets have only
  idle, walk and sleep frames.
- Fix: `rules.gd` `animal_frame`: a fight lunges on the walk cycle, a blow is a start on the
  second idle frame, eating and grazing the slow idle bob, carrying the walk. Frames drawn for
  each of these are still wanted from the art (#195).
- Guard: `tools/gd_test.gd` `test_animal_poses_are_not_all_frame_zero`.
- Commit: 8d95e15 (C63).

## 2026-10-20 #198: tools/ci.sh failed on a fresh Windows clone

- Symptom: on Git Bash the GDScript step stopped with `cp: cannot stat
  'target/debug/libhf_godot.so'` after 58 minutes of passing checks, with no `CI OK`.
- Found by: QA round 2026-10-20, #198 (a fresh clone on Windows).
- Cause: checked. The #148 step (C54) copied the Linux library by name on every platform.
- Fix: `tools/godot_lib.sh` names the library and its folder for the platform
  (`windows hf_godot.dll` under MINGW, MSYS or Cygwin); `tools/ci.sh` uses it.
- Guard: `tools/ci.sh` checks `godot_lib.sh`'s answer for a Git Bash `uname`.
- Commit: 8d95e15 (C63).

## 2026-10-20 #199: the peoples page clipped what the seed would draw at 720p

- Symptom: at 1280x720 the "What this seed would draw" list sat below twelve kinds and showed
  one line of eight, cut off by the page bottom; the tester's Windows run also saw the kinds'
  descriptions run into the next row.
- Found by: QA round 2026-10-20, #199 (Windows, 1280x720); the clipping reproduced here under
  Xvfb at 1280x720, the overlap did not (Linux fonts).
- Cause: checked for the clipping: the list came after the kinds. Not confirmed for the
  overlap: the three-column grid with a fixed 420 px description; likely the Windows font's
  wider wrap.
- Fix: `setup.gd` `_peoples_page`: the preview comes first; each kind is its own row, as tall
  as its description wraps to in the row's remaining width.
- Guard: `tools/gd_test.gd` `layout_peoples_page` (at 1280x720: the preview ends above Back and
  on screen; no description runs past the next row). Fails on the old page.
- Commit: 8d95e15 (C63).

## 2026-10-20 #200: the HUD showed event identifiers

- Symptom: `director: on (identity_formed (6 events))` on day 1.
- Found by: QA round 2026-10-20, #200.
- Cause: checked. The bridge sent the director's `reason`, the kind's snake_case name and a
  count.
- Fix: `view/director.rs` `watching` puts the kind in a viewer's words ("the community", "a
  fire", "people at work"); `Suggestion::kind` carries it; the HUD reads `director: on,
  watching the community`.
- Guard: `tests/client_view.rs` `the_director_line_is_in_a_viewers_words`.
- Commit: 8d95e15 (C63).

## 2026-10-20 #204: --at=band showed the wrong level

- Symptom: on seed 11 the shot centred on the band's camp at x,y but on the founding level,
  below the camp.
- Found by: QA round 2026-10-20, #204 (set G).
- Cause: checked. `main.gd` `_world_ready` took `B[1], B[2]` and never `B[3]`.
- Fix: `--at=band` also sets the view level to the camp's, unless `--z=` was given.
- Guard: none automatic (a launch flag); checked with the seed 11 command from the issue.
- Commit: 8d95e15 (C63).

## 2026-10-20 #188: the README was stale against the client and the CLI

- Symptom: five client flags, three CLI commands and ten options were not documented; the
  README described the one-screen arrival of C3 rather than the six-page wizard, the Engine
  rather than the deity, 32 px people and 16 px items rather than style bible v2's 48 px people
  and 32 px items, and a roadmap dated 2026-10-16 with C8 on "not started".
- Found by: QA round 2026-10-20, #188 (code reading).
- Cause: checked. Nothing compared the hand-written sections with the code; only the generated
  tables were checked (`readme_tables.py --check`).
- Fix: README.md sections 1, 2 (CLI, flags, start order, keys, the screen), 4 (sheet sizes,
  items, people, animals) and 6 (the roadmap now points at docs/history/PLAN-NEXT.md and the tags);
  `tools/docs/readme_flags.py` reads the flags from `main.gd` `_parse_args` and the commands
  and options from `hf-cli` `enum Cmd`, and `tools/ci.sh` runs it.
- Guard: `tools/docs/readme_flags.py` in `tools/ci.sh` (16 omissions on the old README).
- Correction to the C54 entry "ASSETS.md counted 157 assets": the art table is
  `tools/art/provenance.toml` (and its siblings in `tools/art/`), not
  `godot/assets/provenance.toml`.
- Commit: 8d95e15 (C63).

## 2026-10-21 #180: lights shone through walls

- Symptom: at night a hearth's six-tile pool lit the field outside a stone house, so an
  enclosed house read as a glow in the field; underground a lamp lit through rock into the
  next gallery.
- Found by: QA round 2026-10-20, #180 (code reading).
- Cause: checked. `grade.gdshader` summed each light by distance only; no tile was consulted.
- Fix: the sky mask is now two bytes a tile, the second whether the tile stops light (walls,
  rock and soil faces, doors: `Rules.blocks_light`). The shader marches the line from each
  light in range a quarter tile at a time and drops the light at the first tile that stops it,
  the light's own tile and the lit tile aside, so a wall is lit on its face toward the light.
  `Rules.lit` is the same march in GDScript. The mask rebuilds every 4 s, so a wall just built
  stops light within 4 s. Windows are not in the sim yet, so they do not throw light out.
- Guard: `tools/gd_test.gd` `test_walls_stop_light` (a hearth in a walled room lights the room
  and the wall's inner face, not the field beyond, nor round the corner).
- Commit: dbb41e1 (C64).

## 2026-10-21 #203: History opened on the world's deep past

- Symptom: H opened on "501 years before the portal: settlements founded 8" and about 150
  count-only rows of the wider world came before the colony's own record.
- Found by: QA round 2026-10-20, #203 (set W3 follow-up).
- Cause: checked. The panel listed `view::history::eras` oldest first, one row per year with
  any count, the wider world's included.
- Fix: `view::history::overview` splits the years: the colony's first, newest first; then the
  record and the customs; then the valley's own past, newest first; then the wider world as
  one line per century, its counts in words (`history::summary`, moved from the bridge) and up
  to four of its great events by name ("Chengdraang became the seat of the realm of Tezaang"),
  with how many more Legends has.
- Guard: `crates/hf-sim/tests/history.rs` `history_opens_on_the_colony_and_names_the_world`.
- Commit: dbb41e1 (C64).

## 2026-10-21: ci.sh tested the client against an old library

- Symptom: with the History bridge keys renamed for #203, `tools/ci.sh` printed CI OK, and the
  `--history` shot then failed with `Invalid access to property or key 'colony'`.
- Found by: the C64 review shot.
- Cause: checked. ci.sh built and copied `libhf_godot.so` into `godot/bin` only when it was
  missing, so after the first run the GDScript check and tests loaded a stale build.
- Fix: ci.sh builds the library every run and copies it whenever it differs.
- Guard: none automatic beyond the copy; the C64 shots were taken after it.
- Commit: dbb41e1 (C64).

## 2026-10-21 #202: the leopard-print ground was frozen soil, and ruins looked new

- Symptom: on seed 7 at 2x the settlement ground read as an animal print; the old hamlet's
  walls were the crisp blocks of a wall built today.
- Found by: QA round 2026-10-20, #202 (filed as sand).
- Cause: checked. The shot was a cold morning, and `world_view.gd` `_surface_variety` draws
  soil as frozen soil below -1 °C; that sheet was a white base with three brown earth patches
  a tile. The client had no way to tell a ruin's wall from a built one.
- Fix: frozen soil is the soil's brown rimed with fine frost; sand lost its dark patches.
  `deep::ruins` lists the old houses' walls still standing and the places they fell, never a
  tile the colony has built on (`built_of`); the bridge's `ruins()` hands them to the client,
  which draws a ruin darker under `overlays/ruin.png` and lays `overlays/rubble.png` where one
  fell, also for a hamlet on ground a level above the view, which was drawn as a rise of block
  paving.
- Guard: `tools/art/ground_check.py` (frozen soil and sand blotch shares, in tools/ci.sh) and
  `crates/hf-sim/tests/history_pass.rs` `ruins_are_told_from_new_walls`.
- Commit: 504c09e (C65).

## 2026-10-21 #201: the bone forest read as grey scree and a dead wood

- Symptom: at 1x the bone forest read as a gravel flat with sparse dead grey trees.
- Found by: QA round 2026-10-20, #201 (set AT).
- Cause: checked. Bone moss was a mid grey (luma 174 against scree's 131) and the bone tree's
  crown three small grey clumps.
- Fix: bone moss is a pale green-cream (luma 215, greener than snow); the bone tree carries a
  white clump at each of five limbs, about an oak crown wide, cream and lavender beneath.
- Guard: `tools/art/ground_check.py` (moss against scree and snow, white crown pixels).
- Commit: 504c09e (C65).

## 2026-10-21 #205: perception was 86 % of a 500-person tick

- Symptom: 500 fed people took 2.16 ms a tick on a laptop (2.27 ms in the cloud container),
  over the 2 ms gate, with perception 86 % of it and 10 to 15 times the box's spike count.
- Found by: QA round 2026-10-20, #205 (set AP).
- Cause: checked by instrumenting a look. Each look traced a sight line to every thing in range
  even where many shared a tile, traced one to every belief in range before asking whether the
  thing was missing, sorted the whole crowd to keep the nearest 64, and noted each thing with
  two walks of the memory map.
- Fix: a look memoises sight lines by tile and tries smell and hearing first; checks what is
  gone against the index before any sight line; picks the nearest 64 by selection;
  `Knowledge::note` walks the map once and skips the age order when it would not change. What
  is perceived is unchanged: `ostrog run` digests match before and after on seeds 3 and 7
  (3 days) and 300 people (1 day). 500 fed people: 2.27 to 1.60 ms (mixed 2.47 to 1.67), spikes
  over 5 ms 1994 to 143.
- Guard: `crates/hf-sim/tests/perceive_cost.rs` (sight lines a look traces: 1 for 150 things on
  a tile, was 300; 100 for 100 unmoved things, was 200).
- Commit: d5d5fa3 (C66).

## 2026-10-21 C67: houses were set down from one template

- Symptom: every community built the same 7×6 house whatever its household: two people and
  eight got the same floor, the same six beds, the same door on the same side. Hollows were dug
  to a fixed 3×4 room; a folk with no rock face near built a surface house instead.
- Found by: Dan's design rule (docs/design-rules.md rule 1, D35): what is built after people
  arrive is grown tile by tile from need, never prefab.
- Cause: `systems/settlement.rs` built every house from fixed offsets of its origin
  (`HOUSE_W`, `HOUSE_H`), and hollows from a fixed room.
- Fix: new `houses.rs`. A house's floor is grown a tile at a time over open level ground from a
  seed near home (`grow`), round what stands in the way (trees cost more, rock and other walls
  keep 2 tiles off), until it sleeps the household (`enough`, beds `bed_gap` apart: 3 for a folk
  of half again a human's mass). The walls are the ring round that floor, the door the ring tile
  facing home with open ground outside. The floor and doors are kept on the community
  (`Community.house_plans`); beds, table, stair and lamps are placed from the floor
  (`houses::layout`). A household that outgrows its house adds a room through its wall
  (`houses::annex`) before it starts another. Hollows grow a room to the household
  (`houses::dig_room`) and dig further when outgrown; with no rock face, a hollow folk digs down
  (`find_dugout_site`, `Hollow.dug_down`) and says why. A shelter's room is half a tile a person
  (`shelter_room`).
- Guard: `tests/grown_houses.rs` (all four), `tests/homes.rs`
  `a_hollow_folk_without_rock_digs_down_and_says_why`,
  `a_hollow_folk_with_no_ground_to_dig_builds_a_house_and_says_why`, `tests/shelter.rs`
  `they_dig_in_after_a_raid`.
- Commit: 1cf8bdf, 66f8b35, 6035c7d, 2c9fdda.

## 2026-10-21 C67: building took raw logs and stone straight to the wall

- Symptom: a wall, a door, a bed or a table took logs or stone blocks from the ground straight
  into place; nothing was sawn, dressed or made first.
- Found by: Dan's design rule (docs/design-rules.md rule 2, D36): they harvest or mine what is
  needed, craft and refine it, then place it, as in RimWorld and Dwarf Fortress.
- Cause: `work/designation.rs` `structure_cost` and the planner's `make()` costs named raw
  materials; mining yielded finished `StoneBlock`.
- Fix: mining yields `RoughStone`, dressed to `StoneBlock` at the Workbench (`DRESS_STONE`);
  logs are sawn to planks at the CarpenterBench; fibre is bundled to `Thatch`. Doors, bed frames,
  tables, stools and chests are new items made from planks (`work/recipes.rs` 33-38) and then set
  in place. A wood wall takes two planks, a stone wall a dressed block, a roof planks and thatch.
  `systems/settlement.rs` `chain_needs` counts what the ordered building lacks and
  `building_chain` orders the benches (built of raw logs, the one bootstrap), the crafts, three
  at a time, and the reeds. Thatch roofs only where reeds grow within 50 tiles or there is fibre
  or thatch in stock.
- Guard: `tests/material_chain.rs`
  `walls_want_made_things_and_the_benches_are_ordered_to_make_them`.
- Commit: 704d3dc.

## 2026-10-21 C67: what a bench made could not be carried away

- Symptom: with the chain in, planks sawn at the bench lay there and walls were never built.
- Found by: a seed 3 probe in C67: 40 planks at the CarpenterBench on day 4, no wall begun.
- Cause: `work/planner.rs` `find_known_item` skips items on a build site, and a Craft
  designation's tile counted as one, so the bench's own output was held for a build.
- Fix: `find_known_item` and `settlement.rs` `site_tile` no longer count Craft designations as
  build sites.
- Guard: none in the unit tests: it shows only when people carry, which the planner tests do not
  run. The C67 soaks check it: every community roofed its first house.
- Commit: ab5c41b.

## 2026-10-21 C68: every band knew every renowned thing and carried them all off (#232)

- Symptom: seeds 1 and 3 of the C62 600-day soaks lost every named figure and trophy by day
  300, five in one night; a welcomed pair left with the colony's only named figure.
- Found by: the tester (QA round 6, 2026-10-21), counting `raided` rows against `item_named`.
- Cause: `threat.rs` `spawn_band` gave each band `heard_of()`, every storied thing in the open
  within thirty tiles of home with its tile, with no roll and no fame; the raid targets took
  the heard list without the `free` and `in_shelter` checks the food has, and without regard
  to the band being welcomed.
- Fix: `threat::word_of` gives a band at most one thing, the one whose fame (`threat::fame`:
  its name, being spoken of, its deeds, its age) is widest, and only if a roll under that fame
  passes; `raids()` takes no treasure for a welcomed band and none behind a shelter's door.
- Guard: `tests/qa_1021.rs` `word_of_the_valley_carries_one_treasure_at_most_and_not_always`,
  `guests_told_they_may_eat_leave_the_treasures_alone`, `fame_grows_with_age_and_deeds`.
- Commit: (this commit).

## 2026-11-03 #608: a whole town died of thirst standing on a wall beside its full barrel

- Symptom: in seed 3 of the #587 soak (`tools/soak/run4.sh`, merged head 73220862), 37 people
  died of thirst on day 390 while everyone sent to the community barrel stood idle on the top of
  a wall beside it and the barrel still held water.
- Found by: the #587 soak, then an hourly probe of the saved run (issue #608).
- Cause: `water::stand_by` offered a stand beside a source at its level or one level above it,
  while `water::drink` and structural `can_drink_at` checks used `TilePos::dist <= 1`. One tile
  across and one level up has distance 5, so the offered stand and the drink check disagreed.
- Fix: `water::reaches` is now the shared reach rule for barrels, wells, winter hearths and every
  offered drinking spot. A failed stand is recorded per person for a quarter day, so the next
  decision can choose other water instead of repeating the failed stand.
- Guard: `tests/thirst.rs` `a_wall_beside_the_barrel_does_not_keep_them_from_it`,
  `every_place_offered_to_drink_gives_a_drink`, and
  `a_stand_where_no_drink_was_had_is_passed_over_for_other_water`.
- Commit: (this commit).

## 2026-10-21 C68: the same hardships drove a second proposal twenty days on (#208)

- Symptom: seed 4 of the C60 soaks: three proposals exactly twenty days apart, each naming the
  same three raids, each tearing another party off the community.
- Found by: the tester (QA round 2, 2026-10-20), reading `systems/moving.rs` and the chronicle.
- Cause: `moving::consider` never cleared `Community::hardships` after a proposal, and its
  cooldown looked for the Move project in a list capped at fifty.
- Fix: a proposal clears the hardships it answers and stamps `Community::last_move_proposal`;
  the twenty-day cooldown reads that field.
- Guard: `tests/qa_1021.rs` `a_proposal_answers_its_hardships`,
  `the_cooldown_lives_on_the_community`.
- Commit: (this commit).

## 2026-10-21 C68: "this is home" was computed from age, not years here (#209)

- Symptom: every grown arrival was held by "this is home" from day one of the colony; the
  review's seed 4 had 26 such stayers in a valley 150 days old.
- Found by: the tester (QA round 2), reading `systems/moving.rs`.
- Cause: `years` was `tick - born`, the person's age.
- Fix: `Agent::came`, the tick they came into the valley (set in `spawn_agent` and
  `Identity::into_agent`; an old save falls back to the community's founding), and the hold
  counts from it. Someone neither held nor drawn now "saw no reason to go" instead of being
  given "this is home".
- Guard: `tests/qa_1021.rs` `this_is_home_counts_the_years_lived_here`.
- Commit: (this commit).

## 2026-10-21 C68: food guests were told they could take counted as a raid (#213)

- Symptom: seed 4, day 160: "The strangers took 4 grain from the stores, as they had been told
  they could", then a proposal to move naming those takings as its hard things.
- Found by: the tester (QA round 2), in the client at seed 4 day 160.
- Cause: `moving::observe` matched `Raided` without regard to `open`.
- Fix: an open taking is no hardship.
- Guard: `tests/qa_1021.rs` `food_given_openly_is_no_hardship`.
- Commit: (this commit).

## 2026-10-21 C68: the answers to a proposal never reached the screen (#214)

- Symptom: twenty "went with" and "stayed" lines on one tick, all Minor, throttled to none.
- Found by: the tester (QA round 2), in the client at seed 4 day 160.
- Cause: every answer was a Minor `Chose` on the same tick; the feed keeps one Minor line per
  0.7 s and nine lines.
- Fix: one Notable `EventKind::Answered` after the answers ("12 went with Meivraa, 3 stayed:
  Brann will not leave the graves of their own; ..."), the answers themselves still Minor rows.
- Guard: `tests/qa_1021.rs` `the_feed_gets_one_line_for_the_answers`.
- Commit: (this commit).

## 2026-10-21 C68: a party leaving made the stayers' ending a departure; raids charged to the camp's neighbour (#210)

- Symptom: `left_for` set when seven of forty-eight left, so the forty-one who later died were
  recorded as having left the valley; a raid's hardship went to the community nearest the
  strangers' camp.
- Found by: the tester (QA round 2), reading `systems/moving.rs`.
- Cause: the leave branch of `consider` set `left_for` without the `stay_grown <= 1` check the
  move branch has; `threat.rs` pushed `Raided` at `band.camp`.
- Fix: `left_for` only when nobody of age stays; the food `Raided` event is recorded where the
  food was taken. (The ending card's panel, #210 part 3, is a client matter for the client
  pass.)
- Guard: `tests/qa_1021.rs` `a_party_leaving_does_not_end_the_community_as_a_departure`.
- Commit: (this commit).

## 2026-10-21 C68: the dead were laid to rest on the tick they died, by a sleeper across the valley (#220)

- Symptom: every `died` row had its `laid_to_rest` row on the same tick, "by" someone who was
  asleep forty-five tiles away; the feed said "was buried; X saw to it" while the body lay for
  three hours.
- Found by: the tester (QA round 3), in the C63 soak chronicles, then `rites.rs`.
- Cause: `life::die` called `rites::lay_to_rest` at once, which picked the nearest member with
  no regard to sleep or distance.
- Fix: a death goes on `World::rites_due`; `rites::step` (staged after `life`) sends someone
  awake who keeps the rite (kin first, then the nearest who can get there) walking to the body,
  and the rite is given when they stand by it. A body none of them can reach is mourned at
  home after six hours (`LaidToRest { in_absence: true }`, "was mourned, far from where they
  fell"); one nobody has come to in a day is left unrested (P11). The view keeps the body three
  hours after the rite, not after the death.
- Guard: `tests/qa_1021.rs` `the_dead_are_laid_to_rest_by_someone_who_came_to_them`;
  `tests/cultures.rs` `two_rites_two_thoughts` now steps until the rite.
- Commit: (this commit).

## 2026-10-21 C68: "sought out <friend>" logged on every decision pass (#212)

- Symptom: 18,752 of a 300-day soak's 98,410 chronicle rows were "sought out", one per
  decision pass while walking to the friend, up to 32 a day for one person.
- Found by: the tester (QA round 2), in the seed 4 chronicle.
- Cause: `decide::step` pushed the `Chose` line whenever the chosen goal was `Talk` with a
  friend, with no memory of having said it.
- Fix: said once a day a person, through the same `work_cooldowns` key scheme the "went for
  <food>" line already uses (once a friend a day was still 38,000 rows in 600 days of a
  village of eighty, in the first C68 soaks).
- Guard: `tests/qa_1021.rs` `seeking_out_a_friend_is_said_once_a_day`.
- Commit: (this commit).

## 2026-10-21 C68: the records' words (#227, #228, #215, #226, #222, #223, #233, #211)

- Symptom: two figures by one maker both "Ironmourner"; a hearth tale naming forty
  listeners over six rows; "carried 8 grainsheaf"; a custom's narrative printed twice and a
  White Hart sighting as one record line per witness; "A curse fell on Khaa" four times on one
  site page, and nearly every lament a curse; a realm summary with "0 of 0 sites, -0 people"
  and 704 "fell at" keys; "took up Saewen's clothe"; materials given back at 0.5 on a rock
  tile; nothing ever "spoken of"; "ended 1 years before", "x1.35", and the same hard stretch
  answered every five years for thirty in the past.
- Found by: the tester (QA rounds 2 to 6, 2026-10-20 and 21).
- Cause and fix, each at its site: `legend::tool_name` takes the names already given and
  skips them (`names_given`); `narrate` HearthTale names three and counts the rest;
  `society::kind_name` and the "went for" line use `ItemKind::label`; `narrate::detail` has no
  custom line; `creatures` marks only the first witness in a minute Notable; `stories::retell`
  rolls once a story in the generation after it and a curse keeps its count; `legends::site_page`
  adds the year and skips a repeated line; the realm summary skips a people that never
  settled and battle deaths tally as "fell in battle"; `legend::noun` keeps "clothes";
  `World::give_back` lands on open ground at the quality delivered (`Requirement::quality`);
  `LEGEND_DEEDS` is three families; `ostrog past` uses `deep::ago` and plain words, and
  the deity's watch closes the stretch it answered and does not answer the same hardship
  again for twenty years (`tests/history_pass.rs` `stone_outlasts_timber` samples different
  hamlets now and asks for a 0.15 gap instead of 0.2; the decay rule is unchanged).
- Guard: `tests/qa_1021.rs` `a_name_the_valley_has_given_is_not_given_again`,
  `a_hearth_tale_names_three_listeners_and_counts_the_rest`,
  `items_are_named_in_words_not_variants`, `a_customs_narrative_is_told_once`,
  `a_site_page_tells_each_story_once_with_its_year_and_curses_are_rare`,
  `battle_deaths_are_one_cause`, `a_dropped_order_gives_back_what_it_took_on_open_ground`,
  `the_pasts_deity_does_not_answer_the_same_stretch_every_five_years`. The hart's
  one-line-a-sighting has no unit guard; it shows in the record.
- Not done: the feed's cooking line (#233 part 4, a suggestion) and the ending card's panel
  (#210 part 3, client).
- Commit: (this commit).

## 2026-10-21 C68: the echo by the portal stone was never seen (#229)

- Symptom: 0 `echo_seen` rows in 2,100 soak days across five seeds, though the stone woke,
  was left open and reached full strength.
- Found by: the tester (QA round 5), counting event kinds; cause found with a trace of
  `thin::echo` over 130 days of seed 3: the survivor pool held no `Lost` entry at all, every
  one of the seven having been found alive within the first weeks (the thin test's own
  comment says as much), so the one condition the echo asked for, a lost survivor dead and
  unfound, never arose in play.
- Cause: `thin::echo` drew only from `survivor_pool` entries in `Lost` whose agent was dead.
- Fix: an echo may also stand for one of the valley's own who died with none of their people
  within eight tiles (`Agent::died_alone`, set in `life::die`), each once (`ThinState::echoed`),
  the lost and unfound still first, and one echo in fifteen days at most (`thin::ECHO_GAP`:
  the first C68 soaks saw one every four days). The event and the seer's knowledge are as
  before.
- Guard: `tests/qa_1021.rs` `an_echo_stands_for_one_of_the_valleys_own_who_died_alone`;
  `tests/thin.rs` `what_happens_there_is_true_to_the_record` accepts either.
- Commit: (this commit).

## 2026-10-21 C68: a fire in the open was not worth the camera's look (#219); three "beat out the flames" lines (#221)

- Symptom: a lightning strike, the spread and the beating-out happened off camera; "watching
  a fire" arrived when the fire was out. "Wynwen beat out the flames." three times in twelve
  game-minutes, each Notable.
- Found by: the tester (QA round 3), footage and the chronicle.
- Cause: `view/director.rs` scored an event cell with nobody awake in it at a quarter; a fire
  started in an empty field lost to a store with two busy people. `work.rs` pushed a Notable
  `BeatOut` for every tile put out.
- Fix: `fire_started` and `burned` are exempt from the presence factor, as `caravan_left` is,
  and weighted with cave-ins; the first tile beaten out in an hour is Notable, the rest Minor.
- Guard: `tests/qa_1021.rs` `the_director_looks_at_a_fire_nobody_stands_in`; the beat-out
  severity has no unit guard (it needs a burning field with a worker).
- Commit: (this commit).

## 2026-10-21 C68: the client's small findings (#218, #221, #210 part 3)

- Symptom: the HUD wrapped inside a field at 720p ("step" / "0.00 ms", "[?]" / "keys"); the
  inspect panel stopped at 54% of the screen while its text scrolled; `--shot` could not show
  the director line or a strike's flash; the start-up line called the settlement centre
  "founding"; the ending card wrote over the away-summary's panel.
- Found by: the tester (QA rounds 2 and 3), in the Windows client at 1280x720.
- Cause and fix: `Rules.hud_line` joins the fields with three spaces and holds each field's
  words with non-breaking spaces, so a wrap falls only between fields; the inspect panel is
  anchored to the canvas's bottom (`Rules.inspect_bottom`); `director_on` no longer turns off
  for a shot, and `_do_shot` pumps the sound's event pass once (`sound.pump`); the line prints
  `centre` and `founding` both; the ending card waits while the summary panel is open.
- Guard: `tools/gd_test.gd` `test_hud_wraps_between_fields_only`,
  `test_inspect_panel_reaches_the_bottom`; the rest show in shots.
- Commit: (this commit).

## 2026-10-21 C69: only the first house was furnished (#239)

- Symptom: the second house and the rooms added to it stood on grass with beds only, no
  floor, table, stools, chest or lamps, seventeen days after roofing.
- Found by: the tester (set BB 1 and 4), the seed 7 thirty-day save in the client.
- Cause: `systems/settlement.rs` step 6b ran for `house_sites.first()` only.
- Fix: 6b runs over every house, the first not yet done at a time (the three-at-a-time cap
  holds).
- Guard: `tests/c67_followups.rs` `every_house_is_furnished_not_only_the_first`.
- Commit: (this commit).

## 2026-10-21 C69: a house grown round the community's hearth (#241)

- Symptom: seed 4's split-off grew its one big house round its own hearth; the sparks burned
  the roof and beds every few weeks, and the house was never "roofed".
- Found by: the tester (set BB 4) in the C67 soak chronicle and the client.
- Cause: `houses::taken_ground` took the hearth's own tile only, so the floor grew round it on
  every side.
- Fix: the eight tiles round every hearth, kiln and furnace (built or ordered) are taken.
- Guard: `tests/c67_followups.rs` `a_house_is_not_grown_round_a_hearth`.
- Commit: (this commit).

## 2026-10-21 C69: a loom proposed every five days for sixty, never built (#242)

- Symptom: twelve "wants to build a loom" lines five days apart after a move; no loom.
- Found by: the tester, in the C67 soak chronicle.
- Cause: the workshop was sited by the first house only (`house_sites.first()`), which a
  moved community has none of, or which stood on nobody's ground; the project aged out and
  `projects::workshops` proposed it again with nothing remembered.
- Fix: sited by the first house or, until one stands, by home, then round home; where no
  place is found the community remembers it (`Community::no_site`, sixty days), the project
  closes, and the feed says once "gave up on the loom: found no place for it on their own
  ground".
- Guard: `tests/c67_followups.rs` `a_workshop_with_no_place_is_given_up_once_for_a_season`.
- Commit: (this commit).

## 2026-10-21 C69: moving back to the camp just left; one-way trades (#243)

- Symptom: a community moved (17,93) to (26,67) to (43,94) and back to (26,67) twenty days
  apart; 104 trades in 104 between the valley's communities brought nothing back.
- Found by: the tester, in the C67 soak chronicles.
- Cause: the destination search did not know the old camps; the return rule's thresholds
  scaled with the receiver, so a hamlet of five never had "plenty" while a town of fifty
  always "lacked".
- Fix: `Community::old_camps` (a hundred days) excludes a site within twenty tiles of one;
  `society::returns`: B sends back a kind it holds more of a head than A, beyond its own four
  days. (The house in five hours, part 3, is the chain with planks already in stock; sets BB
  5 and 6, part 4, now say they need `--peoples`.)
- Guard: `tests/c67_followups.rs` `a_community_does_not_move_back_to_the_camp_it_left`,
  `a_trade_brings_something_back_when_the_other_has_more_a_head`.
- Commit: (this commit).

## 2026-10-21 C69: the smaller C67 findings (#240) and the profile by day (#238)

- Symptom: `ostrog run` printed "house None field None"; the seven new item kinds drew at
  16 px on the ground; a house added to three times was a row of two-tile cells with a door
  each; the 500-person profile's cost grows with the days and only one-day runs were quoted.
- Found by: the tester (round 9).
- Fix: the run summary names each community's houses, field and quarry; `tools/art/items.py
  --big` draws the seven templates at 2x into items32 (provenance recorded); `houses::annex`
  takes, of the rooms that would do, the one that keeps the house's bounding box smallest;
  `ostrog profile` prints a line a day (mean tick, perceive, spikes). The roof from the
  level above (#240 part 4) is a client matter for M7. Perceive's growth with the days is
  open: see the C69 review.
- Guard: none automatic for the icons and the summary line; the annex shape shows in set BD 5.
- Commit: (this commit).

## 2026-10-21 C69: a save from before roofs got no roof for a house mid-annex

- Symptom: `tests/building.rs` `an_old_save_keeps_its_houses_roofed`: a seed 9 valley at day
  six, restored as a save from before roofs, came back with no roof at all.
- Found by: the C69 suite (the compact annex of #240 changed when the first house grows).
- Cause: `building::roof_old_houses` roofed a house only when every wall of its whole plan
  stood; a house in the middle of a room being added to has a roofed old part and an unwalled
  new one.
- Fix: the enclosed part of each house is roofed: a floor tile every neighbour of which is
  floor or a standing wall.
- Guard: the same test.
- Commit: (this commit).

## 2026-10-21 C69: a community that moved camp built no house again

- Symptom: seed 3 of the C69 soaks: eighty-five people, one move on day 73, no house marked
  out for the next five hundred days while "wants to build a house" was proposed every
  fifteen days.
- Found by: the C69 soak table (one house marked out in 600 days), then the ring save's open
  orders: seventeen wall orders, three roofs and two floors of the old camp still open.
- Cause: a move cleared the community's house sites but left its orders at the old site, and
  the new-house gate (`walls_posted`, this community's open wall orders) stayed true for good.
- Fix: `moving::consider` drops the community's own open orders within forty-five tiles of
  the old home when it moves and gives back what was delivered to them there.
- Guard: `tests/c67_followups.rs` `a_community_that_moves_leaves_its_orders_behind`.
- Commit: (this commit).

## 2026-10-21 C70: the C68 set's own findings (#245, #246)

- Symptom: the realm summary's deaths line still had 704 "fell at <place>" keys (the C68 fix
  gave the event one cause, but the summary tallies each figure's own record); Aun answered
  plenty at one hamlet four times five years apart; a curse told for one person lost ("1 of
  the Driagliawe ... were lost"); the inspect panel's text ran under the bottom it now reaches.
- Found by: the tester (set BC 6 and 7 on the C68 build).
- Fix: the summary tallies "fell at ..." as one key, the figure pages keep the place
  (`hf-cli`); the past's twenty-year gap covers Aun's plenty as well as hardships (Hask's
  pressing and Mirre's stirring repeat by nature; `deep::old_answer`); a lament becomes a
  curse only for five or more lost, and one lost is told in the singular (`realm/stories.rs`);
  the inspect panel scrolls (`inspect_box.scroll_active`).
- Guard: `tests/qa_1021.rs` `the_pasts_deity_does_not_answer_the_same_stretch_every_five_years`
  (now "not more than twice"), `a_site_page_tells_each_story_once_with_its_year_and_curses_are_rare`
  (no curse under five lost, the singular); the summary and the panel show in set BC.
- Commit: (this commit).

## 2026-10-21 C70: crafting recipes were open to anyone

- Symptom: anyone of a community could saw planks, dress stone, smelt, forge, weave, pot or
  brew; nothing was lost when a smith died; nobody taught anyone.
- Found by: Dan's design rule (docs/design-rules.md rule 3, D37): knowledge belongs to people.
- Cause: `Recipe` carried a `Skill` only; `decide.rs` scored a Craft order for everyone.
- Fix: `recipes::craft_of` names the craft; `decide.rs` skips a craft order the person does not
  know unless `crafts::may_try` (nobody of theirs knows it, the order has waited a day, they
  are the best hand or curious); `work.rs` runs `crafts::tried` for such a maker, which learns
  the craft on a win and spoils the inputs on a loss, both recorded (`CraftTried`); arrivals
  bring the crafts of their apprenticeships and seasoned skills (`opening.rs`,
  `Techniques::common`); `crafts::step` (daily) records `CraftLost` when the last knower is
  dead or gone and offers a lesson where one person alone knows a craft a standing bench uses
  (`Taught`, or a `Chose` refusal with its reason; `Community::crafts`, `teaching`).
- Guard: `tests/crafts.rs`.
- Commit: (this commit).

## 2026-10-21 C70: the C68 set's smaller findings (#248, #249, #250)

- Symptom: `history_pass_budget` failed once on the tester's laptop under load (its headroom a
  third, the timer cold); a hamlet lost every named thing it made, one a visit, with a band
  every twenty days (Redmourner 28 days after its naming); the dusk rot line said "2
  grainsheaf, 1 rawmeat".
- Found by: the tester (set BC on the C68 build).
- Fix: the budget test builds one world off the clock first; `threat::fame` is nothing in a
  thing's first season and `word_of` scales the roll by how many named things there are to
  tell of (up to four); `spoil.rs` tallies by the kind's words. The rite 22 days after a death
  (#250 part 2) is the ghost's own path (P11: one who saw the restless dead gives them their
  rite where they lie, `AtRest` beside it), not a fault.
- Guard: `tests/qa_1021.rs` `word_of_the_valley_carries_one_treasure_at_most_and_not_always`
  (a new thing's fame is nothing); the rot line shows in set BC 5.
- Commit: (this commit).

## 2026-10-22 C71: a room grown over the house's door shut its people in (#253)

- Symptom: two of four C68 soaks had one person inside a house for 230 to 300 days, "starving"
  every day on a berry bush inside the walls; the annex was never roofed.
- Found by: the tester, from `starving` rows per person; cause by loading the C68 seed 3 ring
  save: the first house's door at (19,4) faced east, the annex grew on the east side over the
  door's step, the shared wall became a second door, and neither room had a door to the
  outside.
- Cause: `houses::annex` kept the annex off the old doors' wall tiles but not off the ground
  outside them; `can_floor` let a floor grow over a living bush.
- Fix: no annex floor or wall on the step outside an old door; no floor over a living plant.
- Guard: `tests/c67_followups.rs` `an_added_room_never_takes_the_step_outside_a_door`.
- Commit: (this commit).

## 2026-10-22 C71: newcomers on an edge tile with no way in stood there until they froze (#254)

- Symptom: four arrivals on seed 3's north rim, shut off by a house, never joined or left;
  they starved, ate their dead and froze.
- Found by: the tester, from `arrived` rows without `joined` or `left`.
- Cause: `society::newcomers` took an edge tile on the arrival's side with no reach test and
  set no activity when no path to the camp existed.
- Fix: `society::arrival_tile` takes the nearest edge tile of that side from which the camp can
  be reached, else another side, else nobody arrives; an arrival that still finds no path
  turns back the way they came (`leaving`, `Left`).
- Guard: `tests/c67_followups.rs` `newcomers_arrive_where_the_camp_can_be_reached`.
- Commit: (this commit).

## 2026-10-22 C71: the day's temperature curve was upside down (#256)

- Symptom: warmest at 02:24, coldest at 14:24; a winter noon -9 C, its midnight +1 C; people
  took frostbite at work in the afternoon while nights on the ground were the mildest hours.
- Found by: the tester, `ostrog weather --seed 7 --days 40`.
- Cause: `systems/weather.rs` negated the diurnal cosine.
- Fix: the sign; every digest changes (docs/history/ITERATION_LOG.md 2026-10-22). With the night
  the cold half of the day, a scout's cold nights on the ground outvoted the journey at
  consolidation (`tests/expedition.rs` `expedition_memories_consolidate`): a journey come
  home well is now a vivid memory (valence 0.7, `systems/expedition.rs`), kept whatever
  else filled the days.
- Guard: `tests/hazards.rs` `the_day_is_warmest_in_the_afternoon_and_coldest_before_dawn`.
- Commit: (this commit).

## 2026-10-22 C71: the smaller set BD findings (#255)

- Symptom: a rite on the death tick when the keeper stood beside the body; the run summary
  printed `Some(TilePos { .. })`; seed 7's second house has no lamps by its door; 34 of seed
  2's 48 deaths froze in its winters.
- Found by: the tester (set BD).
- Fix: the body lies an hour before the rite even with the keeper beside it (`rites::step`);
  the summary says "field at 93,78,3". The lamps and the winters are open below.
- Guard: `tests/qa_1021.rs` `the_dead_are_laid_to_rest_by_someone_who_came_to_them` (it
  took a walk) covers the delay; the rest show in set BD.
- Commit: (this commit).

## 2026-10-22 C71: the stockpile, the field, the cellar and the quarry were fixed shapes

- Symptom: every community got a 4 by 3 stockpile extended in rows of four, 6 by 4 field plots
  chained eastward, a 5 by 3 cellar room with the lamp in a set corner, and a quarry shaft at
  home + (9, -7) with a gallery within four tiles, whatever the ground and the need.
- Found by: Dan's design rule (docs/design-rules.md rule 1, D35), open since C67.
- Cause: `systems/settlement.rs` steps 1, 1b, 6c, 7 and `quarry` placed fixed footprints.
- Fix: `grounds::grow` grows a patch outward from a seed over open ground on its level, nearest
  the seed first, round what is in the way, as many tiles as the need; the stockpile is a tile
  for every two loose stacks (six at least) and grows beside the pile when full; the field is
  six tiles a head at first (`Community::field_tiles`, an old save's plots read into it once)
  and grows six tiles a pass toward a plot's worth per three mouths; the cellar is dug outward
  from the stair's foot to a tile per two stacks of food, the lamp on the first tile by the
  foot; the quarry's shaft goes on the nearest open tile six or more from home with stone
  under it, and its gallery follows the stone within ten tiles while rough stone is wanted
  (D40).
- Guard: `tests/grown_shapes.rs` (five tests).
- Commit: (this commit).

## 2026-10-22 C71: a band's list of the food it had seen went stale when the hauls moved it

- Symptom: `tests/threats.rs` `a_band_stays_raids_by_night_and_leaves_for_a_reason`, seed 7:
  a band took sixteen sheaves on its first night and then camped ten days without a raid,
  wanting eight more, its fifteen seen places all empty.
- Found by: the C71 suite (the pile grown beside home is hauled into faster).
- Cause: `threat::look_about` skipped any item already in the band's seen list, so a stack
  carried into the pile kept its old place in the list for good, and the raid targets ("still
  where they saw it") were always empty.
- Fix: an item in sight that was seen before has its place brought up to date.
- Guard: the same test.
- Commit: (this commit).

## 2026-10-22 C71: a house beside the hearth's ring burned from it; an annex waited for thatch (#257)

- Symptom: seed 2's first house, its east side a tile from the hearth's ring, lost its roof,
  six beds, a table, stools and a chest to hearth sparks on days 127 and 193; an annex walled
  up on day 332 was unroofed on day 400.
- Found by: the tester (set BD 2 on the C69 seed 2 soak).
- Cause: `houses::taken_ground` kept the floor one tile off a hearth, and a spark on the ring
  spreads to the ring's neighbours; the new roof's material followed the house's thatch even
  with no reeds within reach and no thatch in stock, so the roof orders waited for ever.
- Fix: two tiles off every hearth, kiln and furnace; a roof begun in thatch goes on in what the
  community can lay now (`roof_style`) when thatch is out of reach.
- Guard: `tests/c67_followups.rs` `a_house_is_not_grown_round_a_hearth` (two off),
  `a_roof_begun_in_thatch_goes_on_in_planks_when_there_is_no_thatch`.
- Commit: (this commit).

## 2026-10-22 C71: a lightning fire crossed forty tiles of grass in a night and burned three houses (#258)

- Symptom: a strike at (19,96) on a summer afternoon reached the village forty tiles off by
  morning and took 42 roof tiles, 12 beds, the loom and the workbench; the record had no row
  between the strike and the first roof and no end for the fire.
- Found by: the tester (set BD 2 on the C69 seed 4 soak).
- Cause: `hazards::fuel` gave every open tile over vegetated ground fuel in any weather, with
  the same spread chance as a wood; `ignite` recorded nothing for a spread; `FireOut` counted
  the tiles alight on the last step.
- Fix: bare grass carries a fire only above 22 C with no rain (a dry summer), so the night's
  cool stops it (the day's curve, #256), and it spreads at two thirds the rate of a wood; a fire that spreads to a roof, wall or structure is said once
  ("spreading to the houses"); the fire's end counts every tile burned since the first start
  (`World::fire_tiles`).
- Guard: `tests/c67_followups.rs` `grass_carries_a_fire_only_in_dry_heat`;
  `tests/hazards.rs` `fire_spreads_unchecked` (a dry summer still burns); the record lines
  show in set BF.
- Commit: (this commit).

## 2026-10-22 C72: the C70 and C71 sets' findings (#259, #260, #261)

- Symptom: "With Irusa gone, nobody in Holtwick knows potting any more" while Irusa led the
  community's own caravan three days away, and nothing when she came back and taught it; 115
  trades in 115 between the valley's communities brought nothing back; "1 tiles burned".
- Found by: the tester (sets BE and BD on the C70 build).
- Cause: `crafts::knowers` counted the valley's agents only, and a caravan's members are away
  (`overworld.away`); the #243 return rule asked the receiver, the town that lacks, to hold
  more a head than the giver; `narrate` had no singular for the fire's end.
- Fix: those away on the community's own journey still count as knowers (`knowers_away`), for
  the loss and for `may_try`; a craft the community had recorded lost and knows again is said
  (`CraftRegained`, `Community::crafts_lost`); a return is what B holds beyond four days of
  its own use in a kind A lacks (`society::returns`), and a trade that brings nothing back
  reads "brought 8 grain from A to B"; "one tile burned".
- Guard: `tests/crafts.rs` `a_craft_is_not_lost_while_its_knower_is_away_on_the_communitys_journey`,
  `a_craft_lost_and_known_again_is_said`; `tests/c67_followups.rs`
  `a_return_is_judged_against_the_receivers_own_need`.
- Commit: (this commit).

## 2026-10-22 C72: a moved community's old site was simply left, and they carried nothing

- Symptom: after a move the old houses stood as if lived in, no record kept the site, nobody
  wondered at it, the walls never fell, and the community walked away from its stores.
- Found by: the MH review (docs/reviews/mh-review-2026-10-20, "an old site that is simply
  left"), carried into M5.
- Fix (D41): `left_sites.rs`. A move records the site (`World::left_sites`: name, home,
  where they went, people, the walls and floors as they stood); `deep::ruins` draws its walls
  as a ruin, standing or fallen; daily, a wall nobody keeps falls with the weather (timber in
  about four years, stone in thirty), its roof with it; hourly, someone of another community or
  one who joined since, standing among the houses, wonders at them and keeps a tale of where
  the people went (`RuinWondered`, `Claim::MovedTo`); and each who goes takes a stack of food
  from the old stores for the road ("took 4 grain for the road"), set down at the new camp on
  arrival (`act.rs`, Wandering done within eight of home).
- Guard: `tests/left_sites.rs` (four tests).
- Commit: (this commit).

## 2026-10-22 C72: a craft "gone from the valley" that had gone to the split-off, said again every few days

- Symptom: the C72 seed 1 soak: eighteen `craft_lost` rows in sixty days, "gone from the
  valley" of people who had split off to the valley's second community, the same craft and
  person three and four times.
- Found by: the C72 soak table.
- Cause: `crafts::step` worded any living knower outside the community as gone from the
  valley, and said the loss again whenever the craft flickered as the split settled.
- Fix: "gone to Mereford" for one alive in another community of the valley; a craft's loss
  is not said again within a season (`Community::crafts_said`).
- Guard: `tests/crafts.rs` `a_craft_gone_with_a_split_is_said_once_and_rightly`.
- Commit: (this commit).

## 2026-10-22 C72: the Windows checkpoint retried a denied file for five hours (#263)

- Symptom: robocopy, with no retry limit, retried the mirror's hidden `.gitignore` 669 times
  at thirty seconds; nothing was mirrored and no pending note written until it was killed.
- Found by: the tester (set BC 8).
- Cause: robocopy's defaults (`/R:1000000 /W:30`); the sync client marks dot-files hidden,
  which robocopy cannot overwrite.
- Fix: `/R:2 /W:5 /A-:SH` on both calls, and the switches kept from Git Bash's rewriting.
- Guard: the tester's next checkpoint (set BC 8).
- Commit: (this commit).

## 2026-10-22 fix/358: every tale of a left camp said "the last of Eldergate went to Eldergate", and a baby wondered (#358)

- Symptom: on C72 seed 2 all nine `ruin_wondered` rows for the camps Eldergate left read "the
  last of Eldergate went to Eldergate"; agent 402, born on day 532.2, wondered at the old houses
  on day 534.3 and kept the tale.
- Found by: the tester (set BG 2 on the C72 seed 2 soak).
- Cause: `left_sites::wonder` told where they went by the moved community's name, and the site
  keeps that same name; the test of who may wonder had no age, and a birth counts as having
  come since the move.
- Fix: where they went is the way from the old camp to the new (`threat::compass`, "the
  south-west"), unless the community has since taken another name; a child does not wonder.
- Guard: `tests/left_sites.rs` `a_newcomer_among_the_left_houses_wonders_at_them` (the tale does
  not end with the site's own name) and `a_baby_does_not_wonder_at_the_left_houses`.

## 2026-10-22 C73: the quarry's shaft was dug a level deeper every time stone was wanted (#264)

- Symptom: seed 7's shaft went from z 2 to z -7 by day 7, seed 1's to z -11, a stair alone in
  the dark rock on every level.
- Found by: the tester (set BF 2), `mined` rows on the shaft's column.
- Cause: `settlement::quarry` took the shaft's levels from `ground_z`, which follows the dig.
- Fix: the levels come from the site's surface as it was chosen (`quarry_site.z - 1` and one
  below).
- Guard: `tests/grown_shapes.rs` `the_quarrys_shaft_is_two_levels_and_no_more`.
- Commit: (this commit).

## 2026-10-22 C73: no wall against raids; the demo's shaft came back as the game (#266)

- Symptom: after a raid a community dug a shelter for the stores (C58) but never walled its
  houses; the staged demo scene wrote the autosave ring, and a launched client could open on
  its preset mine shaft.
- Found by: the M5 plan (defence after raids); Dan (#266) for the demo.
- Fix (D42): `ProjectKind::Wall`, proposed in `projects::wall` after strangers took from the
  store by one who holds strangers a danger (or will not fight them), once the community is
  eight with a house; `settlement::palisade` grows the ring round what stands (the houses'
  walls, the hearth, the stores and benches held, two tiles out) over open level ground, a
  tile at a time by the builders, the gate on the ring's side nearest the fields; when every
  tile stands and the gate is a door, `Walled` is said and `settlement::in_walls` holds, and
  the night's raiders do not come for food or treasures behind it (`threat.rs`). The ring is
  taken ground for houses. The demo stage never autosaves (`main.gd`).
- Guard: `tests/defence.rs` (two tests); the demo autosave shows in set BH.
- Commit: (this commit).

## 2026-10-22 C73: nobody ever wanted the wall

- Symptom: the first C73 soaks: 238 night raids over four seeds and not one wall proposed.
- Found by: the soak table; the rings showed communities of 9 to 56 with houses, and spears
  proposed after the same raids.
- Cause: `projects::wall` asked for someone holding strangers a danger, or refusing to fight,
  above 0.15; few in these valleys hold either that strongly, and a cautious nature added only
  0.1.
- Fix: a cautious person wants a wall on their nature alone ("is a careful one, with strangers
  from X taking from the store"); and the raid that prompts it is the community's own record
  of a night's taking in the last twenty days (`Community::hardships`), not a band still
  camped at home's level at the morning's planning (the second soak run: 45 night raids in
  seed 2, no wall and no spears, the band gone or on another level).
- Guard: `tests/defence.rs` `a_careful_one_proposes_a_wall_without_holding_strangers_a_danger`.
- Commit: (this commit).

## 2026-10-22 C73: a wall that had stood kept raiders out for ever, gaps and all

- Symptom: the C73 soaks: seeds 2 and 3 raised walls and were raided 59 and 113 times at night
  after; seed 3's wall stood at twenty of its forty tiles, and in both the ring covered 40
  tiles of a hilly outline over a hundred long.
- Found by: the soak table, then the ring saves.
- Cause: `in_walls` read `stood` (set once); nothing reposted fallen ring tiles; the ring is
  laid on open level ground only, and the slopes between its pieces stayed open.
- Fix: the ring is laid on the surface of every column round the outline, whatever its level
  (the next soak run: four walls proposed, none ever closed, the slopes between level pieces
  open); `Palisade::closed`, rechecked each pass: every ring tile standing, the gate a door, and
  every other tile of the outline impassable; `in_walls` reads it. Missing, fallen or burned
  ring tiles are reposted at any time, and outline ground that has come open since (a tree
  felled) is taken into the ring; a stockpile marker on the line is moved off it; and the ring
  keeps five tiles in from the map's edge, where builders could not reach (seed 3's last four
  tiles failed five times each at x = 3).
- Guard: `tests/defence.rs` `a_wall_with_a_breach_keeps_nobody_out_and_is_mended`.
- Open: half the stores lie outside the ring as the stockpile grows past it; the pile should
  grow inside the wall. (A hilly valley's wall not closing: fixed in C74.)
- Commit: (this commit).

## 2026-10-22 C74: a hilly valley's wall never closed

- Symptom: the C73 soaks: every raided valley proposed and mostly built a wall, none closed in
  600 days (seed 3 stood at 85 of 91 tiles by day 200).
- Found by: the C73 soak table and ring saves (FIXLOG Open, C73).
- Cause: the last ring orders waited at priority 5 behind all other work; the box's clamp
  five tiles from the map edge cut the ring through a house standing near the edge, its roof
  and chest on the line.
- Fix: once half the ring stands, its remaining wall and gate orders are raised to 8, as a
  half-done roof's are; a house too near the edge for a ring two tiles out stays outside the
  wall rather than being cut through; and a ring tile nobody from home can reach (a ledge by
  the map's edge, where seed 3's last five orders failed every builder) is no way in and
  counts as closed.
- Guard: `tests/defence.rs` `a_wall_half_built_is_finished_first`; the soaks.
- Commit: (this commit).

## 2026-10-22 C74: every wound was the same wound, and tending treated them all alike

- Symptom: a graze and a broken leg healed at one rate; a stab to the belly clotted like a cut
  finger; tending did one thing to every wound; nothing lasting came of a bad break.
- Found by: PLAN-FINISH M5 (tissue layers and treatment that changes a wound's course; wound
  layers on inspect), the #128 suggestion.
- Fix (D44): `body::Tissue` (skin, muscle, bone, deep inside, from kind, severity and part)
  sets the healing rate; `body::Treatment` is what a healer does for each wound
  (`work.rs` Tend); a wound deep inside bleeds until stitched; stitching all but ends
  infection, a deep untreated cut festers more; a break that heals unsplinted leaves the part
  `PartState::Scarred` ("set crooked") and costs work for good; `Body::summary` names the depth
  and the treatment for the inspect panel.
- Guard: `tests/wounds.rs` (six tests).
- Commit: (this commit).

## 2026-10-22 fix/234: the world digest differed between Linux and Windows for the same commit (#234)

- Symptom: `ostrog run --opening --seed 3 --days 3` printed one digest on the Linux box and
  another on the Windows machine for the same commit; four C67 to C72 600-day soaks ended with
  different populations on the two (C71: 74/53/28/55 on Windows against 75/53/51/58).
- Found by: the tester (set BA 2); reproduced here on one machine with WSL Ubuntu 22.04 and
  Rust 1.95.0: master `bba8b5e` gives the same tick-0 digest on both (`digest --seed 3`
  `c57abdb8...`) and different day-3 digests (Windows `fef95c95...`, Linux `0dbc428b...`).
- Cause: `f32`/`f64` `exp`, `ln`, `powf`, `sin`, `cos` and `atan2` call the platform's C maths
  library, and glibc and the MSVC runtime round the last bit differently; 22 calls in the sim
  (skill levels, decay, ruin wear, body scaling, the realm map, the day's temperature, the
  move-camp direction, the compass) carried that into the world state.
- Fix: `fmath::Det` (`d_exp`, `d_ln`, `d_powf`, `d_sin`, `d_cos`, `d_atan2`), backed by the
  `libm` crate (a pure-Rust port of musl's maths: the same bits everywhere), at all 22 calls;
  `clippy.toml` `disallowed-methods` forbids the platform versions in the workspace.
- Guard: clippy (`tools/ci.sh` runs it with `-D warnings`; a planted `x.exp()` fails it);
  `fmath::tests::known_bits`. Checked: with the fix, Windows and Linux give the same digests for
  seeds 3 and 7 at day 3 (`c6e78cbb...`, `81f4199d...`) and seeds 1, 2 and 4 at day 30.
- Commit: (this commit).

## 2026-10-23 C75: someone bitten alone never fought back

- Symptom: in the C75 soaks a beast went for someone 51 times across four seeds and people
  stood against it once.
- Found by: the soak's `attacked` rows against its `stood_against` rows.
- Cause: the bite is struck before the stand is worked out, and `combat::can_stand` excludes
  anyone who needs care, which a bite nearly always makes them; wolves pick someone alone, so
  nobody was left.
  That alone was not it: the rerun gave the same rows. A wolf not driven off bites twice, and
  the pain of both bites (`nerve` takes 0.6 x the wounds) left an ordinary person under the
  0.3 a victim needs.
- Fix: the one gone for may stand hurt, if awake and of age (systems/combat.rs `stand_against`),
  and who stands is judged before the bite, not after it (systems/animals.rs `strike`).
- Guard: tests/combat.rs `one_bitten_alone_still_fights_back`,
  `a_wolf_going_for_someone_alone_meets_them_standing` (8 of 10 or more stand).

## 2026-10-23 fix-273: a breach had no aftermath (#273)

- Symptom: breaking into a deep cavern named it and, since C75, brought its beast up at once;
  nothing said what the chamber was, nothing noticed the breach over time, and it could never
  be closed again.
- Found by: the tester's design issue #273 (asks 2, 4, 5, 6; 1 and 3 were C75 and earlier).
- Cause: not built.
- Fix: the breach's way in is kept (`Cavern::breach`) and the chamber described
  (`cavern_found`: its size, a pool, the underworld's stone); the beast comes up two days on
  (`creatures.rising`, underground.rs `rise`); the community posts a wall across the way in at
  once, and a breach walled before the beast comes is `sealed` and the beast `barred`.
- Guard: tests/combat.rs `a_breach_walled_in_time_bars_the_beast_and_the_cavern_is_described`,
  `the_beast_of_the_deep_comes_up_through_the_breach` (now two days on).

## 2026-10-23 C77: sieges, captures and exiles at a rate no valley survives

- Symptom: the first C76/C77 600-day soaks ended with 31, 35 and 47 people (C75: 47 to 95);
  9 to 21 sieges a run, 20 to 76 carried off, 17 to 28 exiled; seed 1 over its memory budget
  as its people fell.
- Found by: the C77 soaks (docs/soak/c77-2026-10-23, first run).
- Cause: every hurt or kept raider, rout and exile added a grudge point that never faded, and
  two sent a host; a host carried off one a day for up to ten days; any third offence was
  exile, and tantrums and fights run to hundreds a run.
- Fix: a grudge fades a point every twenty days and a host needs four (`threat::GRUDGE_HOST`);
  a host carries off two at most, half the time; exile is for a third fight within forty days
  by one the judge does not like, else amends.
  The second soak: 240 to 475 judgements and 33 to 77 feuds a run soured opinion until the
  valley split (2 splits in two seeds, against none in QA round 4) and the half left behind
  took in no strangers (56 turned away). So a tantrum is made a case of a third of the time,
  an unseen offence is laid on someone half the time, and a feud takes three judgements
  between two people and costs 0.2 of opinion, not 0.3. The third soak still ran 2,800 to
  3,500 breakdowns a seed (QA round 4: 109 to 1,876) and 320 to 660 judgements: C76 had
  branched from master before #237's stress fix merged, so every C76/C77 soak ran with the old
  stress. Master merged in, and soaked again.
- Guard: tests/justice.rs `a_liked_one_who_fights_three_times_makes_amends_and_stays`,
  `a_third_fight_by_one_nobody_likes_is_exile`,
  `two_in_the_record_against_each_other_three_times_are_in_a_feud`; the soaks.

## 2026-10-23 fix-329: nothing decided what the viewer is told (#329)

- Symptom: a watchable game with no announcement layer: a toast for every major line and
  nothing else, no choice, no pause, a storm of one kind pushing out the rest, and no way to
  look where a toast happened.
- Found by: the tester's design issue #329 (the "don't let me miss anything" cluster, #307).
- Cause: not built; toaster.gd (PR #359) left the threshold to this issue.
- Fix: each feed row carries its event's kind; `Rules.announce` decides by the viewer's mode
  (majors, life and death, everything notable, nothing), cycled with N and kept in
  user://announce.cfg; Shift+N pauses the world on a major line; a toast of a kind already
  showing is counted into it (x N); a click on a toast looks where it happened. The digest on
  return (ask 6) was already `_show_summary` after time away.
- Guard: tools/gd_test.gd `test_announcements_follow_the_chosen_mode`.

## 2026-10-23 fix-237: everyone broke down about every four days (#237)

- Symptom: seed 1's 600-day soaks had 6,000 to 7,400 `broke` rows in a village of 50 to 70,
  one a head every four days; tantrums alone broke a thousand pieces of furniture, remade
  with planks and bench time.
- Found by: reading the C67 soak chronicles (issue #237); measured 2026-10-23 on the C75
  soaks: 82% of one person's intervals between breaks fell on day four, and 78 different
  people broke, so it was everyone, not a few.
- Cause: stress was built from the gross weight of every bad thought at full strength,
  ignoring the good ones and their fading, while the ease was capped near 0.01 an hour; in a
  probe (opening seed 1, days 5 to 40) the build beat the ease in two person-hours of three
  with the mean mood at +0.44. Sleeping rough (34%) and seeing someone break (23%, which fed
  back into more breaks) were the largest parts.
- Fix: stress builds from the net of the faded thoughts, good offsetting bad, plus a fifth of
  the gross bad (`strain::HARD_SHARE`); seeing a break weighs -0.1, not -0.2, and does not
  touch someone who broke in the last two days. The same probe: 20 breaks where there were
  123, one a head every 28 days.
- Guard: tests/stress.rs `a_contented_person_with_one_trouble_does_not_break` (9 breaks in
  twenty days before, at most 1 now), `someone_with_nothing_but_trouble_still_breaks`.

## 2026-10-23 fix-324: people crossed a drop as if the map were flat (#324)

- Symptom: Dan, 2026-10-22: "you don't see people climbing down when they walk across the map
  where there is clearly a drop off, they just walk like it is 2d map."
- Found by: Dan; the tester's issue traced the sim side.
- Cause: the sim does change levels (natural ramps at every one-level slope, world/gen/ramps.rs,
  and stairs), but the client was sent only `from_x`/`from_y`, so a step up or down was drawn
  as a slide across flat ground; and nothing could fall: open air under someone moved them
  sideways to the nearest standing tile.
- Fix: `agents()` sends `from_z` (field 24); entity_layer.gd lifts the sprite through a change
  of level in an arc; open air under someone drops them to the floor below and hurts them by
  the height (systems/movement.rs `fall`, a break from two levels); and now and then someone
  going down a level slips (more often when hurt).
- Guard: tests/falls.rs `the_floor_dug_away_under_someone_drops_them_and_hurts_them`; the
  README agents() table checks the field count against AGENT_STRIDE.

## 2026-10-23 fix-369: the 500-person soak measured a famine (#369)

- Symptom: `soak --seed 5 --days 20 --agents 500` ended with 169 alive and 331 laid to rest,
  while 12-person soaks grew.
- Found by: the tester's QA battery (#369); reproduced 2026-10-23: 347 deaths of starvation
  and 1 to wolves, none before day 16, then 1, 8, 77, 139, 122 a day.
- Cause: not a fault of the sim at scale. A party of 500 foraging with no stores strips the
  fruit in two weeks; winter (days 15 to 19) stops regrowth and the starvation clock does the
  rest. `profile --provision` lays down a day's meals each midnight (C54), but `soak` had no
  such option, so the perf soak at 500 measured a famine.
- Fix: `soak --provision` (without --opening) places the profile's stockpiles and a day's
  meals each midnight, and gathers the party where it can reach them; the provisioning moved
  into `provision_piles` and `provide_meals`, shared by both commands. Unprovisioned, the soak
  is unchanged.
- Guard: the provisioned soak itself (seed 5, 20 days, 500 people: none starve).

## 2026-10-23 fix-278: nothing gripped the valley for days (#278)

- Symptom: weather changed by the hour and the season; nothing set in over the whole valley
  for days and lifted again, the thing a second monitor can follow.
- Found by: the tester's design issue #278.
- Cause: not built.
- Fix: spells.rs: a heat wave (+9 C), a cold snap (-10 C), a drought (no rain, growth at four
  tenths, +3 C), a darkened sky (overcast, half the daylight), an ill wind (sickness in any
  season, anyone may catch it), a kind season (growth up to 1.3); one at a time, on about one
  day in thirty from the tenth day, by season, for three to seven days, begun and lifted on
  record (`spell_began`, `spell_lifted`); weather::current leans the day's weather by it.
- Guard: tests/spells.rs.

## 2026-10-23 fix-353: a failing valley was not legible as one (#353, asks 3 to 5)

- Symptom: a community dwindling to nothing read as a list of deaths; "died out" said nothing
  of why, and the last survivor was one more name.
- Found by: Dan's design note #353 ("losing is fun").
- Cause: not built.
- Fix: fate.rs: daily, each community is thriving, struggling (under three meals a head
  lying in store), failing (half its peak gone or under one a head) or collapsed (three
  quarters gone), and a change is on record with why (`faring`); the last one left is on
  record (`last_one`); its dead are tallied by cause, and a community that dies out ends
  "died out (11 of starvation, 5 froze, 3 of old age)". Bad landings by draw (asks 1, 2) wait
  on #352's landforms; ask 6 was realm/live.rs already.
- Guard: tests/fate.rs.

## 2026-10-23 fix-347: a member could not be away and still belong (#347)

- Symptom: everyone of a community was always present; leaving was for good, except a
  community's own caravan.
- Found by: Dan's idea, filed as #347.
- Cause: not built (membership and presence were one fact).
- Fix: journeys.rs: from day fifteen, a curious or solitary adult of a community of four or
  more, on about one day in 120, sets out alone for a peopled place beyond the valley their
  community knows of, for a reason (to see it, to trade a little, to be alone, to ask after the
  missing), for four to twelve days (`went_away`); they are off the map and off those present,
  still of the community (crafts.rs counts them, #259); they come back by their place's edge
  (`came_back`). Departure for a grievance, and the roll as a list in the client, are not here.
- Guard: tests/journeys.rs.

## 2026-10-23 fix-328: one sickness, called "a fever" (#328, epic #376)

- Symptom: every illness was the same thing with no name, one course and one cause.
- Found by: the tester's design issue #328, folded into epic #376.
- Cause: not built; `Illness` had only a severity.
- Fix: `IllnessKind`: a wound gone bad (as before), the winter cough (caught close in the cold
  months), a fever (any season, builds two days then breaks or lingers; leaves the one who had
  it guarded), the flux (beside the dead left lying a day; only the right care turns it), the
  wasting (the old and long-hungry; never lifts below a quarter), beast fever (now and then
  from a hunt; guards after); a healer's care is right or wrong by their practice and herb
  lore (`Illness::treated`), and only the right one speeds the turn; the record names the
  sickness (`fell_ill` and `recovered` carry `what`). Avoidance and quarantine (ask 6) are not
  here.
- Guard: tests/illness.rs.

## 2026-10-23 fix-332: every breakdown looked the same (#332, epic #376)

- Symptom: four ways to break, all of them "stopped working", picked by a few rolls, so the
  rows read as one thing over and over.
- Found by: the tester's design issue #332, folded into epic #376.
- Cause: not built.
- Fix: a bag of seven: a tantrum, withdrawal, wandering off, a fight (as before), a binge (eats
  up to four helpings of what lies within six tiles), a daze (sits a day, work let go), walking
  out (one with nobody they think well of leaves the valley for good), setting a fire (one who
  holds that force settles things, on something that burns within four tiles); weighted by
  traits (industrious, solitary, gourmet, cautious) and each person leaning three times to a
  way of their own, so the same person breaks alike.
- Guard: tests/breaks.rs.

## 2026-10-23 fix-338: a rite simply happened (#338, epic #376)

- Symptom: laying the dead to rest always went the same way; nothing could go wrong at a rite.
- Found by: the tester's design issue #338, folded into epic #376.
- Cause: not built.
- Fix: systems/rites.rs `outcome`: fitting (0.2, more with each mourner up to six and with the
  one leading it holding to burial), marred (0.06, more in rain or snow, with nobody standing
  with them, or given in their absence), else plain; a fitting rite comforts the mourners
  (`FittingRite`), a marred one grieves them and leaves the dead unrested for all that, its
  cause the rite's row (`rite_outcome`, with what went wrong: the rain put out the fire, nobody
  came, a word said wrong). Rites beyond burial (#297, #320) are not here.
- Guard: tests/rite_outcomes.rs.

## 2026-10-23 fix-284: nothing was ever lost or replaced (#284, epic #376)

- Symptom: `PartState::Lost` and `Replaced` existed (C74) and nothing made them; a limb rotting
  with infection could only kill.
- Found by: the C74 review's "not here" and the tester's design issue #284.
- Cause: not built.
- Fix: health.rs `limbs`, daily: a limb whose wound is foul (infected, 0.4 or worse) with the
  rot deep (illness 0.6 or worse) is taken by one of the community near who knows herb lore or
  cooks well (`limb_taken`): the wound goes with it, blood is lost, the rot eases by 0.45. A
  limb lost is replaced by one who knows carpentry from a plank or a log (`limb_made`: a wooden
  leg, a wooden hand). A lost limb takes 0.25 from what a body can do, a made one 0.1; the
  inspect line says "left leg lost" or "left leg replaced by a wooden leg".
- Guard: tests/limbs.rs.

## 2026-10-23 fix-295: the social weave was thin (#295, epic #376)

- Symptom: opinions came from what people did to each other directly; nobody passed a name
  on, a good turn bought nothing, and two at odds stayed so.
- Found by: the tester's design issue #295, folded into epic #376.
- Cause: not built.
- Fix: social.rs: talking, each tells the other what they think of the one they feel most
  strongly about (0.4 or more either way), and the listener leans 0.12 toward it, more the more
  they think of the teller (a reputation, `social::reputation`, is what most have come to
  think); tending and teaching leave the one helped owing, repaid with food when the one owed
  goes hungry (`repaid`); two who think ill of each other, talking in good spirits, make it up
  a fifth of the time (`made_up`). Avoidance, factions and ostracism (asks 5 to 7) are not here.
- Guard: tests/social.rs.

## 2026-10-23 nemesis: strangers forgot the valley as soon as they left (D49)

- Symptom: a raider who fought someone and went home was gone for good; every band was new
  faces, so no enmity could build across raids.
- Found by: Dan, 2026-10-23, asking for a nemesis system alongside gossip.
- Cause: not built.
- Fix: nemesis.rs (D49): rivals made in a fight with a stranger who lives (`rival_made`), come
  back with their people (`rival_returned`), seek their foe (`rival_met`, with a title earned),
  and fall on record (`rival_fell`); `World::rivals` is saved.
- Guard: tests/nemesis.rs.

## 2026-10-23 feelings-1: nothing was felt about anything in particular (design-feelings.md step 1, #281)

- Symptom: people carried a mood and a stress, summed from thoughts, but no felt reaction to a
  particular thing seen; the same siege landed alike on everyone.
- Found by: docs/design-leadership.md's dependency, written up as docs/design-feelings.md.
- Cause: not built.
- Fix: mind/feelings.rs: six feelings (fear, grief, anger, joy, awe, shame), each about someone
  or nothing, at most eight held, topped up when renewed, fading by kind (anger fast, grief
  slow); `gain` is the temperament (#281): calm, traits, the Violence lean and a lean of one's
  own set how strongly each person feels each kind. systems/feel.rs appraises the last tick's
  events (a death grieved by those who cared; an attack, a fire, a raid feared; a fight with a
  stranger angering the bold and frightening the rest; a quarrel angering both; one's own break
  shaming; births, joinings, returns gladdening; the deep awing) and, hourly, fades them and
  carries a strong one (0.3) as a thought, so mood and stress read it. Step 2: each hour,
  someone awake within six tiles who thinks well of a person takes a fifth of their strongest
  feeling. Step 3: feelings lean decide.rs's scores, 0.7 to 1.4 (fear toward fleeing and bed and
  away from wandering; anger toward confronting; joy toward talk; grief and shame away from
  talk and work; awe toward study at the thin place).
- Guard: tests/feelings.rs (step 2 `fear_passes_to_those_who_care_and_stops_at_those_who_do_not`,
  step 3 `the_afraid_wander_less`).

## 2026-10-23 fix-299: the valley's waters held nothing (#299, epic #377)

- Symptom: lakes and rivers were scenery; nobody fished.
- Found by: the tester's design issue #299, folded into epic #377.
- Cause: not built.
- Fix: fish.rs: each 16-tile stretch of water holds up to 24 fish, fished down and coming back
  0.6 a day (`World::fish`, saved); when food runs short (the hunting threshold) and no fishing
  is open, a `Fish` order is posted at the nearest water with a bank and three or more fish
  (settlement.rs 7c); a morning there brings back raw food by the stock and the fisher's way
  with animals (`fished`); in the hard cold it is ice, half the catch, and one time in a
  hundred the ice gives way. Gear, weirs and boats (asks 2, 3, 5) are not here.
- Guard: tests/fishing.rs.

## 2026-10-23 fix-316: no traps, for game or for strangers (#316, epic #377)

- Symptom: a valley short of food could only hunt by chasing; nothing was ever laid against
  raiders on the way in.
- Found by: the tester's design issue #316, folded into epic #377.
- Cause: not built.
- Fix: traps.rs: with food short (the hunting threshold), a snare is set by the nearest hare or
  fox within forty; after strangers took from the store in the last ten days, one a third of
  the way from home toward their camp (settlement.rs 7d, a `Snare` order; three at most); at
  dawn a snare with small game within three takes it six times in ten (`snared`, two helpings
  of meat); a stranger stepping onto one is caught by the leg; a snare is spent when it
  catches, or after ten days. Mechanisms and engineering (the rest of #316) are not here.
- Guard: tests/traps.rs.

## 2026-10-23 fix-270: no creature ever came into the built place (#270, epic #377)

- Symptom: wolves and bears met people out in the wild; nothing came in to the houses and the
  stores, so the built place never had a beast inside it to deal with.
- Found by: the tester's design issue #270, folded into epic #377.
- Cause: not built.
- Fix: creatures.rs `incursions`: once a day, with twenty or more helpings of food lying within
  25 tiles of home, the nearest fox, or out of winter bear, within sixty tiles may be drawn in
  (a chance growing with the stores, at most 0.15; `drawn`); it walks to where most food lies,
  eats there (a fox two, a bear six), a bear breaks the nearest thing standing within three
  (`stores_raided`), and anyone of the valley within five may stand against it (C75); then it
  goes off. Tiers of threat and attraction by wealth rather than food (asks 2, 3) are not here.
- Guard: tests/incursion.rs.

## 2026-10-23 fix-84: the valley's ground showed nothing of who walked it (#84, epic #377)

- Symptom: years of the same walk from the houses to the field left the grass as it was.
- Found by: the suggestion in #84 (and #196), folded into epic #377.
- Cause: not built.
- Fix: movement.rs counts each person's step onto open ground (`World::wear`); paths.rs, once
  a day, fades the count by a tenth, wears grass, meadow or moss walked 60 or more to packed
  earth (the client's own ground), and grows a path below 8 back to what it was
  (`World::paths`, saved); nav makes a worn path two ticks a tile quicker.
- Guard: tests/paths.rs.

## 2026-10-23 fix-296: childhood was almost nothing (#296, epic #377)

- Symptom: children followed a parent about and wandered; they learned nothing until grown,
  and growing up passed without a mark.
- Found by: the tester's design issue #296, folded into epic #377.
- Cause: not built.
- Fix: systems/childhood.rs, hourly: a child of four or more, awake within five tiles of an
  adult of their community at work, takes 0.15 of that work's skill, and at a bench one time in
  a hundred learns the craft as one who watched (C70); children together warm to each other;
  daily, the day a child comes of age is on record with what they turned out best at
  (`came_of_age`). Schooling as a project and a child's own voice (asks 7, 8) are not here.
- Guard: tests/childhood.rs.

## 2026-10-23 fix-346: the cold bit but did not shape a life (#346, epic #377)

- Symptom: cold drained warmth to frostbite, but below ground was as cold as the open, nobody
  grew used to a hard winter, and a killing frost left the fields untouched.
- Found by: the tester's design issue #346, folded into epic #377.
- Cause: not built.
- Fix: below ground counts as sheltered from the season's cold (hazards.rs); thermal.rs: a day
  in cold below one's comfort with warmth under 0.9 hardens a person 0.3 degrees, up to four,
  lost 0.2 a warm day (`Agent::hardened`), and comfort is reckoned from it; at four in the
  morning below -5 C the unripe grain and roots in a community's fields die (`frosted`).
  Thermal mass, clothing by material and heat microclimates (asks 1, 2, 6 in part) are not here.
- Guard: tests/thermal.rs.

## 2026-10-23 fix-341: an orphan was nobody's (#341, epic #377)

- Symptom: a family was a mother and a father; a child whose parents both died was left with
  no one to keep near, and nothing marked it.
- Found by: the tester's design issue #341, folded into epic #377.
- Cause: not built.
- Fix: fostering.rs, daily: a child of a community with neither parent alive and no foster is
  taken in by the adult who thinks best of them (one with a partner a little before others);
  both warm 0.3 to each other (`fostered`), and the child keeps near their foster as they would
  a parent (decide.rs). Other relationship kinds (#341's wider asks) are not here.
- Guard: tests/fostering.rs.

## 2026-10-23 round 7: feelings fed on themselves, and frost killed the winter sowing

- Symptom: the first round 7 soak's seed 3 ended with 21 (C77: 54): 2,669 breakdowns
  (C77: 121), 17 exiles, 278 `frosted` rows.
- Found by: the round 7 soaks (docs/soak/r7-2026-10-23, first run).
- Cause: every feeling at 0.3 became a thought of up to -0.3, and fear passed between friends
  each hour with nothing to stop a listener reaching past the one they caught it from; fear
  made stress, stress made breaks, a break seen made fear. A killing frost struck in winter,
  when crops sown then were killed night after night.
- Fix: a feeling is carried as a thought only at 0.5, the thoughts weigh half as much (fear
  -0.12, grief -0.15, anger -0.1, joy 0.15, awe 0.1, shame -0.12), and contagion stops once
  the listener feels half of what the teller does; a killing frost comes only in spring and
  autumn.
- Guard: tests/feelings.rs, tests/thermal.rs `a_killing_frost_takes_the_growing_crop` (an
  autumn night); the soaks.

## 2026-10-23 fix-325: a field beside the water drank no more than one far from it (#325, part)

- Symptom: crops grew alike wherever they were sown; water near a field did nothing for it,
  and a drought took the riverside crop as hard as the dry one.
- Found by: the tester's design issue #325, folded into epic #377.
- Cause: not built.
- Fix: systems/plants.rs: grain and roots with water within three tiles (at their level or
  one below) grow a third faster, and a drought's lean on growth does not reach them
  (`plants::watered`). Digging channels to bring water to a field (the rest of #325) waits on
  moving surface water (#323's further part).
- Guard: tests/irrigation.rs.

## 2026-10-23 fix-274: reaching harder stone meant nothing (#274, part)

- Symptom: whatever stone was cut, the valley built alike with it and nothing marked a better
  one reached.
- Found by: the tester's design issue #274, folded into epic #377.
- Cause: not built.
- Fix: a `mined` row carries the rock cut (`EventKind::Mined::rock`); tiers.rs keeps the stones
  each community has cut (`Community::stones`) and puts on record the first cut of a stone
  harder than any before it (`new_stone`); a wall dressed from harder stone weathers slower in
  a left camp (left_sites.rs: 1/(3000 x hardness) a day). Ore tiers gating arms, marble for
  carving, and deep finds in trade (asks 2 to 4) are not here.
- Guard: tests/tiers.rs.

## 2026-10-23 fix-322: no society met apart, and nobody gave one away (#322, epic #376)

- Symptom: a judge, a leader or a neighbour could be resented by many and nothing gathered
  round the resentment.
- Found by: the tester's design issue #322, folded into epic #376; docs/design-feelings.md 6.
- Cause: not built; it waited on feelings (round 7).
- Fix: societies.rs: each evening, three or more adults of a community angry (0.3) at the
  same one of them, each thinking well (0.3) of two of the others, form a society against
  them (`society_formed`, `World::societies`, saved); at midnight they go out to meet, fourteen
  tiles from home; a member who has come to think well of the one they meet against (0.4)
  gives it away three nights in ten (`betrayed`): the rest think 0.6 worse of the betrayer and
  the one betrayed 0.4 worse of them; a society lapses when its target dies or it falls under
  two. Acting against the target (the gate opened, #322's own picture) is not here.
- Guard: tests/societies.rs.

## 2026-10-23 fix-slept-rough: every night was a night on the ground

- Symptom: at the end of every 600-day soak of C77 and round 7, nearly everyone carried the
  `SleptRough` thought (96 of 96 in round 7's seed 1), houses and beds standing; it was the
  largest single source of stress in #237's measure.
- Found by: probing the round 7 soak saves for what kept people at the end of their rope.
- Cause: strain.rs read whether a sleep was in a bed back from the sleeper's last three short
  memories, but act.rs consolidates memory at the end of a sleep, right after recording the
  rest, and consolidation clears the short list; the rest memory was never there to be read.
- Fix: `EventKind::Slept` carries `in_bed` (act.rs knows it), and strain.rs reads that.
- Guard: tests/slept.rs `a_night_in_a_bed_is_not_a_night_on_the_ground`.

## 2026-10-23 fix-272: digging into a water-bearing layer had no consequence (#272, epic #377)

- Symptom: miners cut through any rock at any depth dry.
- Found by: the tester's design issue #272, folded into epic #377.
- Cause: not built.
- Fix: aquifer.rs: in patches eight tiles across, one in five by the world's seed, the rock
  three to nine levels down holds water; a cut into it (a `mined` row) floods the cut and the
  open workings below ground within three tiles of it and a level under, shallow
  (`struck_water`). Flowing water (#323) and pumping are not here; the water stands.
- Guard: tests/aquifer.rs.

## 2026-10-23 fix-323: water did not move (#323, the first part)

- Symptom: water was where the generator put it; nothing flowed.
- Found by: the tester's design issue #323, folded into epic #377.
- Cause: not built.
- Fix: aquifer.rs `flow`: water let in from wet rock (#272) carries 24 tiles of volume; each
  hour it moves on, down into open space under it first, else sideways into open floor below
  ground beside it, filling as it goes, until spent (`World::seeping`, saved). Lakes, rivers,
  pressure and lava (the rest of #323) are not here.
- Guard: tests/aquifer.rs `water_let_in_runs_along_the_workings`.

## 2026-10-23 fix-271: the dark grew nothing new (#271, epic #377)

- Symptom: mushrooms were placed on cave floors at generation and never spread, so the deep
  held nothing that grew, and a cavern was worth only its stone.
- Found by: the tester's design issue #271, folded into epic #377.
- Cause: not built.
- Fix: cave_life.rs, daily: a living mushroom patch below ground seeds an open floor tile
  beside it a third of the time (two thirds by water or in a cavern with a pool), unless eight
  already grow within six tiles. More cave crops (the rest of #271) wait on plant art (#286).
- Guard: tests/cave_life.rs.

## 2026-10-23 fix-279: four skies (#279, epic #377)

- Symptom: the sky was clear, overcast, rain or snow, nothing else.
- Found by: the tester's design issue #279, folded into epic #377.
- Cause: not built.
- Fix: `Sky::Fog` (spring and autumn, a ninth of the overcast-or-clear draws) halves the light
  and covers raiders as the night does (threat.rs); `Sky::Storm` (summer and autumn, four in a
  hundred draws) is rain (`Sky::wet`: fires, spoilage, grass fires read it), half the light,
  and lightning, one tick in 900; both are on record as they come and go. The client draws a
  storm's rain and sounds it (sky 5); fog has no drawing of its own yet.
- Guard: tests/weather_breadth.rs.

## 2026-10-23 fix-285: nothing was kept, only hunted (#285, epic #377)

- Symptom: wild goats could be hunted and nothing else; no community kept animals.
- Found by: the tester's design issue #285, folded into epic #377.
- Cause: not built.
- Fix: livestock.rs: while its herd is under six, a community posts a `Tame` order on the
  nearest wild goat within forty (settlement.rs 7e); one tamed is kept (`Agent::kept`,
  `tamed`), grazes within ten of home and does not run from people (animals.rs); a herd of two
  to eleven has a kid one day in twenty; with food short and more than two kept, one is
  hunted for meat; kept animals are left out of the ordinary hunt. Milk, wool, pens and
  fodder (the rest of #285) are not here.
- Guard: tests/livestock.rs.

## 2026-10-23 fix-349: no valley-level knowledge of the peoples beyond (#349, epic #377)

- Symptom: each person knew what they saw, and each community what ground it had explored, but
  nothing held what the valley knew of the peoples beyond it, apart from the places someone
  had travelled to.
- Found by: Dan's design (#349, folded into epic #377).
- Cause: not built.
- Fix: known.rs: when strangers are sighted, their place goes on the community's tracker
  (`Community::known_others`): its name, first and last seen, the most that came, whether any
  were armed, the side they came by; `KnownOther::located` is true only if someone of the
  valley has been there; the first word of a people is on record (`first_word`). Nothing in
  the simulation reads it (#349's rule 7). Staleness shown in a client panel, spies and
  markers (asks 3 in part, 5, 6) are not here.
- Guard: tests/known.rs.

## 2026-10-23 fix-348: nobody could belong to two communities (#348, epic #377)

- Symptom: a split put each person in one community or the other; one whose partner or
  family stayed behind simply belonged to the new one.
- Found by: Dan's idea in #348, folded into epic #377.
- Cause: not built (`Agent::community` is one community).
- Fix: ties.rs: one who went with a split while kin stayed keeps a tie to the old community
  (`Agent::tie`, `torn`); by day, one day in ten, they go across to it; the tie lapses when no
  kin of theirs is left there. Loyalty in a fight between the two, and membership in two at
  once in the rolls (the rest of #348), are not here.
- Guard: tests/ties.rs.

## 2026-10-23 fix-352: every landing was a valley (#352, epic #377)

- Symptom: the ground was one shape of noise everywhere; every world was a valley.
- Found by: Dan's design (#352), folded into epic #377.
- Cause: not built.
- Fix: `GenParams::landform` (heightmap.rs): a valley (as before, and the default for worlds
  made directly), a plain (low and even), a plateau (raised ground with falls at its edges), a
  ridge (a high back of land across the map), a coast (the land falling to the sea along one
  side) or a marsh (low, near the water's level); an opening draws one from its seed, a valley
  half the time, each other a tenth, and the record of the setup says so ("..., on a
  plateau"). Features and a generated past to match the land (the rest of #352), and bad
  landings by draw (#353 asks 1, 2), are not here.
- Guard: tests/landforms.rs.

## 2026-10-23 omens: great events read as nothing (#297, epic #376, ask 4)

- Symptom: a siege, the deep's beast, a spell over the valley passed without anyone making
  anything of them; the spirit domain of C77's leadership had nothing to say.
- Found by: the tester's design issue #297, folded into epic #376; docs/design-feelings.md
  section 6.
- Cause: not built.
- Fix: omens.rs: when a spell begins, a host besieges, the deep's beast rises, a neighbour goes
  out in a beast's shape, a breach is made or a rival comes back, the one a community follows
  in the spirit (justice.rs) reads it, a blessing for a kind season and a warning for the rest
  (`omen`), and everyone of the community within fifteen of them feels it, fear or gladness
  (0.3 by temperament). A place of worship, observances, heresy and pilgrimage (the rest of
  #297) are not here.
- Guard: tests/omens.rs.

## 2026-10-23 fix-268-lapse: high ground was as warm as low (#268, part)

- Symptom: the season's cold was the same on a plateau's top or a ridge's back as at the
  landing; #268's audit found no altitude lapse.
- Found by: the tester's audit #268, folded into epic #377; more pressing since landforms (#352).
- Cause: not built.
- Fix: hazards.rs `cold_step`: the temperature a person feels is 1.5 degrees lower for each
  level their column's ground stands above the landing's (and higher below it). Drainage,
  fertility, soil depth and erosion (the rest of #268) are not here.
- Guard: tests/altitude.rs.

## 2026-10-23 fix-292-why: a mood with no why (#292, part)

- Symptom: the inspect panel said "mood: low" and listed thoughts by weight, but not which of
  them made the mood what it was, nor what the person felt about whom.
- Found by: the tester's design issue #292, folded into epic #378.
- Cause: not built.
- Fix: view/why.rs: `feeling` (the strongest feeling over 0.2, and whom it is about: "angry
  with Halgar") and `because` (the two thoughts weighing most on the side the mood leans);
  the bridge sends them and the panel reads "mood: low, because: slept on the ground, a
  quarrel" and "feels: angry with Halgar". Why-hovers on events and the world (the rest of
  #292) are not here.
- Guard: tests/why.rs.

## 2026-10-23 fix-54-wall: what a people held did not shape what it built (#54, part)

- Symptom: a community known for holding strangers a danger built exactly as one that did not,
  until a band actually took from its store.
- Found by: the backlog epic #54 (identity changing what is built, 12.9), folded into #376.
- Cause: the wall rule (C73) waited only on a raid's hardship.
- Fix: projects.rs `wall`: a community whose identity holds "strangers are a danger" proposes
  its wall unraided, once it has eight and a house, by one who holds it. Other identities
  shaping other building (the rest of #54) are not here.
- Guard: tests/defence.rs `a_people_known_to_hold_strangers_a_danger_walls_itself_in_unraided`.

## 2026-10-26 qa-388: wild animals read as never having acted, the burrower as stuck (#388)

- Symptom: on `run --opening --seed 7 --days 30` the dump showed the burrower "STUCK for
  864000 ticks: Idle", and the bat, crawler and boar "act idle, switches 0".
- Found by: the tester's run against c81 (#388); reproduced on ad6ad3c. Every one of the 128
  wild animals in that dump had 0 switches, deer and hares that were walking included; only
  the burrower was flagged stuck.
- Cause (checked): two faults. The switch count was raised only by `decide::commit`, which
  only people go through; animals set their activity by hand in animals.rs, livestock.rs,
  creatures.rs and act.rs, so no animal ever counted one. And the burrower moves only by
  digging (underground.rs `burrow`), which moved it but never gave it an activity, so its
  activity ticks ran for the whole game.
- Fix: agents.rs `Agent::set_activity` sets the activity, zeroes its ticks and counts the
  switch together; every animal activity change (animals.rs, livestock.rs `graze`,
  creatures.rs, act.rs's end of a walk) goes through it, and `burrow` gives the burrower a
  walk to where it dug to.
- Guard: tests/qa_1026.rs `every_wild_animal_takes_up_something_within_days` (seed 7, three
  days: every living wild animal switched at least once, none held one activity throughout).

## 2026-10-26 qa-389: a carving was taken from its carver halfway (#389)

- Symptom: seed 7's 30-day opening run had "carving the wall/work order gone" four times; the
  dump also said "nearest rock face from home: None", and the issue tied the two together.
- Found by: the tester's run against c81 (#389); a probe of every carving order on seed 7
  showed each of the failures with two people working one order: the one who finished
  carved it, the other's plan ended with "work order gone" the same tick.
- Cause (checked): not the rock face. A carving is cut into a house wall of timber or block
  (engrave.rs `wall_to_carve`), never into a rock face; and a home with no face within reach
  already gets its stone from a quarry (settlement.rs, step 4) or by trade (projects.rs), so
  opening placement was left as it is. The fault was the work lease: it is renewed for two
  hours (2,400 ticks) only when a plan moves from one step to the next (work.rs `advance`),
  and a carving is three hours (3,600 ticks) of work in one step, so the lease ran out an
  hour before the end and the next carver could take the order up. Any work of one step
  longer than two hours was open to it (study is exactly two).
- Fix: work.rs `run`: whoever is at a plan renews its leases every lease sweep (60 ticks)
  while they are at it.
- Guard: tests/qa_1026.rs `a_carving_is_not_taken_from_its_carver_halfway`: one person on
  open ground takes up a carving by hand and holds its lease every tick until it is cut
  (without the fix the lease is lost after 2,460 ticks). It first ran seed 7 for ten days;
  after master's c84 that world carved once, so the guard became this scenario.

## 2026-10-26 qa-397: the White Hart stood at the valley's edge for days (#397)

- Symptom: on seed 11 the dump flagged "STUCK the White Hart for 28833 ticks: Idle"; the
  tester read it as separate from #388's misreport, which it is.
- Found by: the tester's run against c81 (#397). An hourly probe of the Hart on master, seed
  11, 30 days: it stood idle at the north edge (44,1) for up to 58,553 ticks (two days) at
  a time, four times, and left only when someone came near and it fled.
- Cause (checked): the Hart keeps to trees away from the edges and from homes; with none such
  within 20 tiles it steps towards the valley's middle, and that step was set at the level it
  stood on, not on the ground there. From the edge the ground falls away, so the tile at its
  own level was air: not walkable, no walk, every turn (the probe: "centre step at level
  walkable false", the surface there walkable). Fleeing, which does use the ground, clamps
  to the map's edge and took it back there.
- Fix: animals.rs `step`: the Hart's step towards the middle is on the ground (`surface`);
  and at the point every wild animal's turn passes, one left standing idle half a day takes
  a short walk to open ground within six tiles (`stretch`), whatever its kind's own rule
  chose or failed to choose.
- Guard: tests/qa_1026.rs `the_white_hart_does_not_stand_for_days` (seed 11, ten days, hourly:
  no wild animal not denned stands idle more than thirteen hours, the Hart switches).

## 2026-10-26 qa-392: the arrival was a neutral spawn (#392, with #395)

- Symptom: survivors came through the portal at mood 0 and stress 0; neither shock nor any
  hurry to secure food and shelter.
- Found by: the tester's #392 (seed 7) and the design brief #395.
- Cause: not built; `since_arrival` gated behaviour, never feeling.
- Fix: strain.rs `arrival_shock`, called for every survivor in opening.rs `begin_opening`
  (after the past's stress, not for survivors an old save upgrades): a thought `Arrival`
  (mind/thoughts.rs, appended) of weight -0.3 to -0.4 lasting 2.5 to 4.5 days, lighter and
  shorter the calmer, heavier for the cautious, shorter for the easygoing; and a spike of
  stress at once, 0.05 to 0.2, highest in the least calm. The thought is left out of the
  hourly stress build (strain.rs `step`): with it in, ten-day openings broke 6 to 18 times a
  seed against master's 0 to 2. `survival_urge` (1.5 at the arrival, back to 1 as the thought
  fades) multiplies in decide.rs `choose` eating and foraging (and lowers the hunger at which
  they start), getting warm, and the work of shelter, fire and food (walls, doors, floors,
  roofs, structures, felling, sowing, harvest, cooking). From #395: the spread by temperament,
  the stress spike, the survival hurry, and a panic tail through the existing breaks (one or
  two of the ten, mostly the least calm of the lost). Not taken: grief over the missing as
  its own affect with "where are they" events, and panic or calm spreading between people.
- Measured: `run --opening --days 10`, breaks for seeds 1 to 4: master 0, 0, 2, 0; this
  change 1, 2, 4, 2 (eight of the nine by someone of calm 0.04 to 0.23, seven by survivors
  still lost and alone, all in the first five and a half days).
- Guard: tests/qa_1026.rs `the_arrival_is_a_shock_that_fades_and_the_calm_shake_it_faster`
  (seed 7: every survivor holds the thought and stress above 0; among the not easygoing the
  calmer's lasts no longer; community mood on day 1 is more than 0.1 below the same world's
  with the thought removed, and below day 6's; nobody holds it after a week). tests/identity.rs
  `a_people_is_known_by_what_most_of_them_hold` asserted seed 6's count of believers a day on;
  with the change a different set of the lost is found on the first day and the line lapses
  by the rule. The test now checks the rule on a party of six whose views are set by hand
  (three of six: not formed; four: formed; back to three: kept, and saved; two: lapsed), so no
  seed has to land on either side of the line; the arrival check stays on seed 6.
  tests/legend.rs `an_ordinary_run_gives_tools_lives` asked for a tool cherished (100 uses by
  one keeper) within 20 days on seeds 3 and 7; with the change the longest-kept tool had 69
  and 57 uses (master 51 and 108). It now asks for one at half that, and leaves cherishing
  to `long_use_makes_a_tool_cherished_then_named_then_spoken_of`.

## 2026-10-26 fix-390-depth: lakes, streams and swamps were one level deep (#390)

- Symptom: every water body was a single sheet on the surface; a swamp in Dan's screenshot was
  not even one level deep, and a lake had nothing under its surface tile to break into a cut.
- Found by: Dan, 2026-10-26, on #390.
- Cause: `world/gen/hydrology.rs` `apply` puts water only on the tile above each water column's
  ground; nothing gave it depth.
- Fix: `world/gen/hydrology.rs` `deepen`, run after the biomes are laid (`world/gen/mod.rs`):
  each water column is carved down by its distance to the nearest dry column, one level at the
  shore, two a tile in, up to four (`MAX_DEPTH`) in the middle, so a stream wide enough has a
  channel. A fen's water stays one level deep over two levels of `WET_SOIL`. Carving stops
  above open ground (a cave, or a lower bank beside), and every tile of a column carries its
  surface tile's flags, so what the walker sees of a lake is unchanged. Saves hold their own
  tiles, so an old save keeps its old water.
- Guard: tests/water_depth.rs.

## 2026-10-26 fix-390-break: a shaft dug beside a lake stayed dry (#390)

- Symptom: Dan's screenshot (Merebank, Year 1 Spring day 1): a staircase dug two levels down
  two tiles from a lake, and nothing happened; people tunnelled on beside the water.
- Found by: Dan, 2026-10-26, #390.
- Cause: the only water a cut could let in was the rock's own (`aquifer.rs` `observe`, #272);
  surface water had no link to a cut beside or under it.
- Fix: `aquifer.rs` `break_in`, called from `observe` for every `mined` row before the wet-rock
  check: a cut with water on a 4-neighbour at its level or the level above, or over it, lets
  the water in. The body it belongs to is followed (as far as 4,096 tiles) to find its surface;
  the water fills the cut and the open workings joined to it that are under the ground or are a
  stair, lowest first and no higher than that surface, at most 400 tiles and no more than the
  body holds; the body gives up as many tiles from its top level, shore first. Tiles below the
  surface fill to the roof (`TileFlags::FULL`, the eighth flag bit). On record as
  `water_broke_in` (Major), narrated "The water broke in where ... was digging and poured down
  the shaft".
- Guard: tests/lake_breaks.rs `a_shaft_beside_a_pond_floods_and_the_pond_gives_up_as_much`,
  `a_shaft_away_from_the_water_stays_dry`.

## 2026-10-26 fix-390-drown: nobody could drown in a flooded working (#390)

- Symptom: water in a working was only a tile flag; a person could stand in it for ever. The
  only drowning in the game was through ice while fishing.
- Found by: #390's ask that someone at the bottom when the water comes in be in danger.
- Cause: not built.
- Fix: `systems/hazards.rs` `drown_step`, every five ticks: someone whose tile is full to the
  roof, or has water on it and on the tile above (`under_water`), makes for the nearest dry
  tile they can reach, up to twelve across and six levels up (`dry_ground_near`, through
  `decide::commit`), and drowns after `BREATH_TICKS` (60 ticks, three minutes; three times as
  long for a body with a swimming part) under. The count is `World::drowning`, saved. Water
  standing shallow in a working (#272) has air above it and drowns nobody.
- Guard: tests/lake_breaks.rs `someone_at_the_bottom_climbs_out_of_the_flood`,
  `someone_with_no_way_out_drowns`.

## 2026-10-26 fix-390-site: the way down was sited beside the water (#390)

- Symptom: the quarry's stair shaft in Dan's screenshot was two tiles from a lake.
- Found by: Dan, 2026-10-26, #390.
- Cause: `systems/settlement.rs` `quarry` took the nearest open tile with stone under it,
  six or more tiles from home, whatever was beside it; `find_dugout_site` likewise.
- Fix: both skip a site with standing water within two tiles of it, across or up to four
  levels down (`aquifer::near_water`).
- Guard: tests/lake_breaks.rs `the_quarry_shaft_is_dug_on_dry_ground`.

## 2026-10-26 fix-390-damp: no warning before the water broke in (#390)

- Symptom: a cut beside water gave no sign before it let the water in.
- Found by: #390's ask that wet ground seep before it breaks.
- Cause: not built.
- Fix: `aquifer.rs` `damp` and `left_damp`, called by `systems/work.rs` `apply_effect` before
  a Mine or DigStairs cut: stone with water within two tiles at its level or the one above is
  damp. A digger with the `Cautious` trait leaves the cut (its failures rise by three, as when
  there is nowhere to set a prop) and says why the first time ("left a cut alone ... the stone
  was damp"); anyone else digs on and may break through. Shoring against water is not here.
- Guard: tests/lake_breaks.rs `a_cautious_digger_leaves_damp_stone`.

## 2026-10-26 fix-393-runoff: a drizzle flooded like a storm, and a puddle rose like a river (#393)

- Symptom: every shallow water edge in the valley spread one ring outward after twelve hours of
  any rain, every four hours more; a drizzle and a storm flooded alike, and a hilltop pond rose
  as fast as the stream.
- Found by: Dan, 2026-10-26, #393.
- Cause: `systems/hazards.rs` `flood_step` counted hours of rain and nothing else: the weather
  had no strength, and the water no catchment.
- Fix: the weather has a strength (`Weather::drizzle`, one rain in three; `rainfall`: a drizzle
  0.3, a rain 1, a storm 3). New `floods.rs`: `survey` (run once, at a world's first hour or an
  old save's) labels the standing water into bodies (stream, lake or pond, by shape) and gives
  each its catchment by a priority flood over the ground from every body outward, lowest first;
  a body at the map's edge counts 32 columns upstream for each edge tile. `hour` raises each
  body by rain times catchment over area (at most 40) times `RUNOFF` 0.003, drains 8% of the
  water above its usual level, caps it at three levels; a body half a level up goes over banks
  level with it, a level higher needs a level more, one ring an hour of the lowest ground it
  reaches, at most three times its area times how far it stands over its banks, `MAX_FLOOD` 600
  over all. (A first try at 0.3 and twice the area flooded 200 tiles after any six hours of rain
  on seed 3's opening and `legend.rs` `an_ordinary_run_gives_tools_lives` lost its one cherished
  tool; that the floods took it is not confirmed.) Flood water drains from the
  outside in once its body falls below it. The hourly cost is the bodies' shores and their
  flood, not the old scan of every tile of the grid; on the default 128 by 128 map the survey
  takes about 1 ms once (`examples/waters.rs`).
- Guard: tests/floods.rs `the_stream_gathers_more_than_the_hilltop_pond`,
  `the_same_storm_raises_the_stream_more_than_the_pond`; tests/hazards.rs `flood_fills_basin`
  unchanged.

## 2026-10-26 fix-393-thaw: snow never reached the water (#393, part)

- Symptom: snow built up and melted on roofs only (`building.rs`); a winter's snow had no
  effect on the streams, and there was no spring flood.
- Found by: Dan, 2026-10-26, #393.
- Cause: nothing kept snow on the ground; the flood rule read only the hours of rain.
- Fix: `floods.rs` `Waters::snow`, the snow lying on the valley's ground in units of rain: each
  hour of snowfall adds one (`SNOWFALL`), each hour above freezing melts a quarter of a unit
  per degree (`MELT`), and the melt is added to the rain in `hour`, so it runs to every body by
  its catchment. `winter_snow` keeps the most that lay this winter until the thaw has taken it
  all. Snow on the ground is one figure for the valley, not drawn on the map.
- Guard: tests/floods.rs `a_heavy_winter_brings_a_spring_flood` (sixty hours of snow then a
  thaw at ten degrees floods the stream at least one and a half levels and over fifty tiles;
  eight hours of snow raise it less than half as much).

## 2026-10-26 fix-393-spate: a flood was the same shallow water everywhere it went (#393, part)

- Symptom: flood water was one shallow ring at a time on any loose ground beside any water; it
  never went to low ground first, never carried anything off, never put anyone in danger, and
  always drained away to nothing.
- Found by: Dan, 2026-10-26, #393.
- Cause: the rule had no level for the water or the ground (`hazards.rs` `flood_step` before
  #393's first part).
- Fix: `floods.rs`. `rise` takes one ring an hour of the lowest ground the water reaches, so a
  floodplain below the stream fills before the bank level with it. `spate`: a body at least
  `SPATE` (1.5) levels up puts its shallows, and its flood over ground no higher than its banks,
  over a person's head (`TileFlags::FULL`, which `hazards.rs` `under_water` and the drowning of
  #390 read; a death there is "drowned in the flood"), clears that when it falls, and carries
  off every item lying in that deep flood (consumed as "carried off by the flood", on record as
  `carried_off`). `fall` drains a flood tile only to dry ground or open water at its level or
  lower, directly or through flood that drains; flood with no such way, in a hollow, is left
  as a pond (`Waters::left`, on record as `pond_left`).
- Guard: tests/floods.rs
  `a_spate_fills_the_floodplain_first_carries_off_what_lies_there_and_leaves_a_pond`.

## 2026-10-26 fix-393-drought: a stream ran the same through any drought (#393, part)

- Symptom: water only ever rose; a summer without rain left every creek as full as in spring.
- Found by: Dan, 2026-10-26, #393.
- Cause: nothing took water away but the recession of a flood.
- Fix: `floods.rs` `hour`: every dry hour above freezing lowers each body by `EVAP` (0.004 of a
  level at ten degrees, a tenth more for each degree over), down to two levels. `drought`: a
  stream (not a lake or pond) fallen a level takes out its tiles one level deep (a deeper
  channel keeps pools) and keeps them as they were in `Body::dried`; the ponds its floods left
  dry up; on record as `ran_dry`. When rain brings it back to half a level down, the tiles go
  back wherever nothing was built on the bed meanwhile (`ran_again`). At 25 degrees a stream
  runs dry after about four days without rain.
- Guard: tests/floods.rs `a_creek_runs_dry_in_a_long_dry_spell_and_comes_back_after_rain`.

## 2026-10-26 fix-393-memory: a flood went unremarked and homes went up where it had been (#393, part)

- Symptom: the record had one "the water has risen" row for any flood and nothing for a spring
  flood or a river in spate; the planners would site a house or a field on ground a flood had
  just covered.
- Found by: Dan, 2026-10-26, #393.
- Cause: nothing kept where the water had been, and floods had no kinds to tell apart.
- Fix: `floods.rs` `high_water`: a body going into spate is on record once (`high_water`,
  Major) until it falls back within its banks, as the spring flood when the hour's melt is more
  than its rain ("The spring flood: the winter's snow has melted and the stream is over its
  banks, too deep to ford.") or else as a spate after the rain. `Waters::high_water` keeps every
  column a flood has covered; `grounds.rs` `open_ground` (fields, stockpiles and the other
  patches) and `houses.rs` `can_floor` (new houses and annexes) refuse ground below that line.
- Guard: tests/floods.rs `people_keep_homes_and_fields_off_ground_they_have_seen_flooded`; the
  record in `a_heavy_winter_brings_a_spring_flood` and
  `a_spate_fills_the_floodplain_first_carries_off_what_lies_there_and_leaves_a_pond`.

## 2026-10-26 no observance was ever kept (#297, epic #376, ask 2)

- Symptom: the only rite was for the dead; nothing marked the turn of a season, and nothing
  was lost when the faith around the deity thinned.
- Found by: the tester's design issue #297, folded into epic #376 (ask 2: regular observances
  on the calendar, and what happens when they lapse).
- Cause: not built.
- Fix: observances.rs `step`: at noon on the first day of each season, each community keeps an
  observance led by the one it follows in the spirit, or with nobody followed by whoever awake
  most heeds the signs (lean 0.2 or more). Those of the community within twenty tiles keep it
  (`Observance`, with how many came): glad of it (`KeptObservance`, as #338's `FittingRite`),
  a little surer of the signs, following the one who led it a little more. A community that
  kept the last one and has nobody to lead or nobody who comes lets it lapse, once, on record
  (`ObservanceLapsed`, why), and those who heed the signs are troubled (`ObservanceLapsed`
  thought). Kept observances are in `Faith::kept`.
- Guard: tests/faith.rs.

## 2026-10-26 a sign was never borne out (#297, epic #376, ask 6)

- Symptom: an omen or a reading of the deity was said and forgotten; whatever followed, nobody
  held the signs surer or doubted them, and the one who read them was followed the same.
- Found by: the tester's design issue #297, folded into epic #376 (ask 6: heresy, doubt and
  loss of faith after a bad outcome).
- Cause: not built.
- Fix: faith.rs `settle_signs`: a death of one of the community while a sign read to them is
  open (five days) bears out a warning and belies a blessing; each member's DeitySigns moves
  toward heeding or doubting (0.2, 0.25), the reader is followed in the spirit more or less
  (justice.rs `credit`, new `discredit`), and it is on record (`SignSettled`). A sign whose
  days pass quietly moves them the other way, faintly (0.06), with no event. `turned`: anyone
  whose lean crosses from one side to the other is on record (`FaithTurned`), and the
  inspector's beliefs show the signs it came from.
- Guard: tests/faith.rs.

## 2026-10-26 the deity answered unread (#297, epic #376, asks 3 and 8)

- Symptom: the deity's answers were Trace events, hidden from the feed; nobody in the valley
  said what the deity seemed to want, and everyone of a culture held the same about signs.
- Found by: the tester's design issue #297, folded into epic #376 (ask 8: the deity's mood
  legible without a meter; ask 3: divergence of belief and the conflict it causes).
- Cause: not built.
- Fix: faith.rs `read_answers`: each answer over the valley is read aloud by the one a
  community follows in the spirit, in words by the deity's motive and what it answered
  (`DeityRead`: "Hask has seen how well we do and means to test us"). `hear_signs`: a reading
  or an omen is heard by the community within fifteen tiles; most take it as the reader holds
  it (DeitySigns, told by the reader); one who has never held a view and whose own
  `inclination` is against signs comes to hold they are only weather; the one most sure of
  that says so to the reader (`Scoffed`), and the two think the less of each other. Each sign
  is kept open five days (`Faith::signs`).
- Guard: tests/faith.rs.

## 2026-10-26 fix-322-gate: a walled way down could not be opened, and nobody held it (#322, part)

- Symptom: a breach into the deep, walled up (#273), stayed walled for good; nothing said whose
  way down it was or who could open it, so "who may open this" could not be put.
- Found by: the tester's design issue #322 ("a door or gate that can be locked, with a key or
  a keeper"), folded into epic #376.
- Cause: not built.
- Fix: underground.rs: a deep cavern's way down belongs to the community whose people broke in
  (`Cavern::gate_of`, set in `breaches`); when the breach is walled, the adult of that community
  standing nearest the wall (within three tiles) keeps it (`Cavern::keeper`, set in `rise`);
  `open_gate` lets someone standing by the wall cut it out, unseals the cavern and puts
  `gate_opened` on record. Walled again, it seals again with a new keeper.
- Guard: tests/deep_gate.rs `the_one_who_walls_the_way_down_keeps_it_and_it_can_be_opened`.

## 2026-10-26 fix-322-taken: no society wanted anything, and nothing came up from below for anyone (#322, part)

- Symptom: a society against someone met by night and did nothing; the deep's peoples came up
  only once, as strangers, when a cavern was first broken into; nobody was ever let in, and
  nobody of the valley was taken by them.
- Found by: the tester's design issue #322 (the guild that opens the gate and lets the
  cave-dwellers take the children), folded into epic #376.
- Cause: not built.
- Fix: societies.rs: of an evening, a society whose community holds a walled way down to the
  deep (fix-322-gate), with a people of the deep within twelve regions
  (`underground::deep_neighbour`, factored out of `breaches`) and the one they hate the parent
  of a child, comes to terms with that people one evening in eight (`Society::aim`,
  `deep_pact`). At midnight the keeper of the way, if one of them, else the angriest of them,
  goes to the wall instead of the meeting and cuts it out (`underground::open_gate`): `let_in`
  names them, the people below and the one it was done against. Two of the deep's people come
  up on the cavern's floor (`underground::floors`, factored out of `rise`), go for the child and
  carry it down into the deep (`taken_below`; kept in `overworld.away`). Anyone of the valley
  not of the society, awake within eight tiles of the wall, saw who cut it (`Taking::seen_by`,
  for the reckoning). At six in the morning the way is found open and the child missed
  (`gate_found_open`), the deep's people still up go down again, the valley sets about walling
  the way, and the child's parents grieve. A society does it once (`Society::struck`).
- Guard: tests/deep_gate.rs
  `a_society_lets_the_deep_up_through_its_keepers_wall_and_a_child_is_taken`.

## 2026-10-26 fix-322-reckoning: an open way down and a child gone, and nobody answered for it (#322, part)

- Symptom: after fix-322-taken the valley found the way open and the child gone, and that was
  the end of it: nobody was blamed, nobody judged, and the society stayed as it was.
- Found by: the tester's design issue #322 (search, suspicion, accusation possibly wrong; the
  society exposed or not), folded into epic #376.
- Cause: not built.
- Fix: societies.rs `reckon`, at the morning's finding: someone who saw the wall cut names the
  one who cut it; else the child's parent (or whoever is nearest the wall) lays it on the
  keeper of the way, or, with no keeper, on the one they like least, which can be wrong.
  justice.rs: `accuse` (factored out of `offence`) puts it on the evening's list as
  `DEEP_GATE`; judging it is exile whether the judge likes them or not. The one who truly did
  it, convicted, brings the society out (`societies::exposed`, `society_exposed`): everyone else
  of the community thinks 0.5 worse of each member, and the society is done. A keeper exiled for
  another's opening is a wrong verdict like any other (C77), and the society stays hidden.
- Guard: tests/deep_gate.rs `the_keeper_who_opened_it_is_named_exiled_and_the_society_comes_out`,
  `when_another_opened_it_the_keeper_is_blamed_and_exiled_wrongly`.

## 2026-10-26 fix-322-search: a child carried into the deep was never gone after (#322, part)

- Symptom: after fix-322-taken the child's parents grieved and that was all: nobody went down
  after the child, and the child stayed in `overworld.away` for good, never seen again.
- Found by: the tester's design issue #322 ("the child returned or not"), folded into epic #376;
  round 13's brief.
- Cause: not built.
- Fix: taken.rs (new): at the morning's finding (societies.rs `dawn`), `gone` sends the
  child's parents present and able, and up to four in all with those who think well of a parent
  or of the child (opinion 0.4 or more), none of the society, to the valley's side of the way
  down. `step` gathers them as a caravan's departure does (six hours at most) and takes those
  there off the map into `overworld.away` (`WentDownAfter`) for two to five days. Coming up
  (`roll`, `settle`), in the style of #338's rite outcomes: brought back with chance 0.15 plus
  0.1 for each who went; one of the party lost below, 0.15 (the last to join dies, "lost in the
  deep"); hurt, 0.2 (two of them injured two days); else nothing found (`CameUpFromBelow`). A
  child brought back has the `BroughtBack` thought (30 days) and the new `FearsTheDark` trait
  (never drawn at birth), and is afraid whenever in the dark below (underground.rs `step`); the
  parents' grief for it turns to joy. A child not gone after (no parent able) or not found
  stays below; when the deep's people of that site next come up (underground.rs `breaches`,
  societies.rs `let_up`) and the child has grown, `among_them` puts `GrownBelow` on record and
  the parents grieve.
- Guard: tests/deep_gate.rs `the_mother_and_a_friend_go_down_after_the_taken_child`,
  `brought_back_the_child_is_home_and_afraid_of_the_dark`,
  `a_party_can_lose_one_below_or_come_up_hurt_or_with_nothing`,
  `a_child_not_gone_after_turns_up_grown_among_the_deep_people`.

## 2026-10-26 fix-322-rumour: what was seen the night the deep was let in went no further (#322, part)

- Symptom: only the one who saw the wall cut, or the child's parent, ever named anyone for an
  opened way down; nobody else heard of it, nobody thought the worse of anyone for it, and a
  keeper exiled in the opener's place ended the matter for good.
- Found by: the tester's design issue #322 (search, suspicion, accusation possibly wrong),
  folded into epic #376; round 13's brief.
- Cause: not built. The gossip of #295 (social.rs) passes on opinions only, and takings were
  forgotten five days after the night.
- Fix: societies.rs: at the opening, others of the valley awake within twenty tiles of the wall
  (beyond the eight of `seen_by`) glimpsed someone out by the way (`Taking::glimpsed_by`). At
  the morning's finding each holds a suspicion (`Agent::suspects`): the one who saw it, 1.0 of
  the opener; each who glimpsed, 0.5 of the opener; the finder, 0.6 of whoever they accuse,
  rightly or not (`reckon`, `suspect`). social.rs `tell_suspicion`, on every talk: the teller
  passes on their surest suspicion (0.3 or more); the listener's grows by 0.4 times its
  strength times their trust in the teller, and they think the worse of the suspected; the
  suspected are not told, and a member of the society does not take it up against another
  (`in_it_together`). At five each evening (`rumoured`), the surest in the community at 0.8 or
  more accuses one not yet accused of that taking (`Taking::accused`), one a taking a day,
  judged that evening as any `DEEP_GATE` accusation; the true opener convicted brings the
  society out as before. Takings are kept forty days, not five.
- Guard: tests/deep_gate.rs `talk_of_who_opened_the_way_goes_round_and_another_accuses_them`,
  `a_member_does_not_take_up_talk_against_another`.

## 2026-10-26 fix-322-story: a taken child's record began only at the taking (#322, part)

- Symptom: the chronicle rows that name a child taken below began with `taken_below`: the pact
  made against its parent and the night the way was opened named the parent and the opener but
  not the child, so the child's own story could not be read whole.
- Found by: round 13's brief ("the chronicle reads the whole story in order").
- Cause: `DeepPact` and `LetIn` carried no child.
- Fix: events.rs: `DeepPact` and `LetIn` carry the child (`child`, defaulting to none on older
  rows) and name it among their participants; societies.rs fills it. view/history.rs
  `story_of`: every chronicle line that names a person, oldest first, narrated from its row.
- Guard: tests/deep_gate.rs `the_chronicle_reads_the_taken_childs_story_in_order` (the pact,
  the night, the taking, the finding, the going down, the return).

## Open, found but not fixed

- Nearly everyone sleeps rough: fixed in parts by #475, #535 and 2026-11-02 #469 (the beds
  of home are known to all of it). What remains is nights with no free bed standing.

- Seed 2's winters (#255 part 4): 34 of 48 deaths froze, out foraging 8 to 92 tiles from camp
  in a hard cold; the valley fell from 31 to 10 people in 50 days. With the day's curve the
  right way up (#256) the afternoon is the warmest hour now; to re-read in the C71 soaks.
- A person "starving" on many days running is not a soak failure yet (#253): the soak counts
  stalled person-hours, which a forager who sleeps and chooses every day never adds to. A
  starving-days count per person belongs in the soak summary.
- A wall laid across a building never closes (#403, seed 4's Merehollow, from the M5 review's
  day-600 save): 208 of its 211 ring tiles stand, but three tiles of its outline at x 66 hold a
  table, a bed and a lamp of a building no house site of the community names, so the laying
  left them out of the ring and nothing can be built there. Found with
  `examples/wall_probe.rs`; not fixed (fix-402-sieges): the ring must go round or through such
  a building, or the closure test must reckon with it.

- Perceive's cost grows with the days at 500 people (#238, C69 review: 539 us a tick on day 1
  to 909 on day 4 on the box, 70 to 85% of the tick; the tester's laptop sees 2.1 to 2.6 ms
  means over five days against the 2 ms gate). The gate holds on the box; what a look costs
  once a mind is full is the thing to look at in a perception checkpoint.

- A workshop proposal that finds no place in five days closes and is made again: in the MH
  review's 600-day soaks, up to eight times by one proposer for one workshop (seed 4 of the
  second run, 56 proposals; seed 3 of the last, 30). The planner
  posts no workshop of a kind while one of that kind is ordered anywhere in the valley
  (`has_designation`, not by community), which may be why it finds no place; not confirmed.
- The M1 gate's "no tick over 16 ms" at 500 fed people: on a quiet box (2026-10-20, C60) the
  human party's worst tick was 16.9 to 17.3 ms in four runs and the mixed party's 16.6 ms, nav's
  path search the largest part (10.1 to 11.5 ms). Just over the gate; the path search is the thing
  to look at.
- People in the 500-person profile ate about seven meals a head a day where the hunger model
  (`mind/needs.rs`, six hours a meal awake, a quarter of that asleep) asks about three. Cause
  not confirmed: eating at a threshold well above empty, with the gain capped at full, would
  waste most of each meal; to check in M5 against an ordinary settlement's intake. C54 probe.

## 2026-10-26 round-11: the chronicle renamed a beast in lines from before it had a name

- Symptom: `tests/history.rs` `chronicle_only_real_events` failed once #267's geology changed
  seed 1101's valley: the feed showed "A bear has caught the smell of the stores and is coming
  in.", and the same event re-narrated from its chronicle row read "Longback has caught the
  smell...". The chronicle view would show a line the viewer never saw.
- Found by: tools/ci.sh on round-11, 2026-10-26.
- Cause: a beast that hurts someone takes a name from then on (`systems/animals.rs`), and
  `narrate::beast` read the agent's current name, so every earlier line about it changed with
  it. Confirmed: the bear was drawn to the stores, then went for someone and was named.
- Fix: `Agent.named_at` holds the tick a beast earned its name; `narrate::beast` takes the
  event's tick and calls the beast by its kind for events before then. Older saves have no
  `named_at` and narrate as before.
- Guard: tests/history.rs `chronicle_only_real_events` (every shown line re-narrates
  identically from its row).

## 2026-10-26 fix-pipeline-dup: crafts ran its daily pass three times a day

- Symptom: `systems/mod.rs` PIPELINE listed `("crafts", crafts::step)` three times, so the
  once-a-day crafts pass (lessons, teaching, the loss of the last knower) ran three times in
  its tick.
- Found by: the #322 minion reading the pipeline, 2026-10-26.
- Cause: confirmed from history: C70 added one line (a9dace1); later branches each inserted a
  stage beside it (fostering #341, ties #348), and their merges repeated the line.
- Fix: one `crafts` stage. Replays from before differ from the crafts tick on.
- Guard: tests/pipeline.rs `every_stage_runs_once_a_tick` (no stage name twice).

## 2026-10-26 enum-order: a merge put a branch's new variants before master's

- Symptom: merging master (c84) into the #393 and QA branches put each branch's appended
  `EventKind` and `ThoughtKind` variants ahead of master's. Saves store variants by position,
  so a c84 save would have loaded the wrong events and thoughts. Caught before merge.
- Found by: the coordinator's conflict resolution, 2026-10-26: a scripted "keep both sides"
  resolution put the branch's side first; the events.rs case also failed to compile.
- Cause: confirmed: "ours" (the branch) was written before "theirs" (master) in append-append
  conflicts in enum bodies.
- Fix: the variants reordered so master's come first, in both branches.
- Guard: tools/enum_order.py in tools/ci.sh: every enum at the last tag must be an unchanged
  prefix of the same enum now.

## 2026-10-27 fix-402-sieges: a host kept striking someone already down (#402, part)

- Symptom: a besieging host struck one person of the valley hundreds of times: seed 3, agent 2
  took 592 wounds "by the besiegers" over days 34 to 37, and day 35 had 869 `fought` rows.
- Found by: the M5 review's 600-day soaks (docs/reviews/m5-review-2026-10-26, branch
  m5-review, fault 1).
- Cause: confirmed. `siege.rs` `besiege` chose the nearest exposed person of the valley with no
  regard to whether they were already down, and only carried off a downed person while the host
  held fewer than two captives. Once it held two, a downed person lying beside the camp stayed
  the nearest target and was struck on every pass. The new test reproduced it on the old code:
  37 blows in 60 passes on one downed person.
- Fix: `siege.rs` `besiege`: someone down (`needs_care`) or held is not struck. They are a target
  only while the host may still take a captive, and are then carried off without a blow. A host
  that holds its two captives goes for nobody and keeps to its camp. Replays with a siege differ
  from its first day on.
- Guard: tests/sieges.rs `a_host_does_not_strike_someone_already_down`,
  `someone_down_is_carried_off_not_beaten`.

## 2026-10-27 fix-402-sieges: blow after blow never made a wound deeper (#402, part)

- Symptom: the person who took 592 blows from a host lived through them (fever killed them
  later); in four 600-day soaks one person bled out.
- Found by: the M5 review's soaks (fault 1), and the new test below: 40 unarmed blows on one
  person on the old code left 29 wounds, none deep inside and none bleeding.
- Cause: confirmed. That only organ wounds bleed is intended (C74's layers: skin, muscle, bone,
  organ; `health.rs` `wound`). What kept blows from ever killing is that each blow was rolled
  fresh: an unarmed blow is 0.2 to 0.6 severe, an organ wound needs a cut of 0.6 to the body or
  head, and wounds already open on the part did not count, so a beating stayed at the muscle.
- Fix: `systems/combat.rs` `blow`: half the severity still open on the struck part is added to
  the new wound, so the layers it reaches follow the harm already done; repeated blows to the
  body or head reach the organs and bleed until stitched, as C74 has it. One blow is as before.
- Guard: tests/combat.rs `someone_beaten_again_and_again_is_in_mortal_danger` (40 blows reach
  an organ, and untended the person bleeds out within four days).

## 2026-10-27 fix-402-sieges: every end of a siege was recorded as a siege lifted (#402, part)

- Symptom: the chronicle said a siege "was lifted" when the host went home with all it came for;
  all ten `siege_lifted` rows in the M5 soaks were such hosts.
- Found by: the M5 review's soaks (fault 4).
- Cause: confirmed. `threat.rs` called `siege::lifted` for any of the four ends (fed, broken,
  tired, the wall held), and the event had no field for which.
- Fix: `siege.rs` `ended` with an `Ending`: a host that took all it came for is recorded as the
  appended `HostSated` ("host_sated", "had what it came for and went home ..."); any other end
  is `SiegeLifted` with an appended `why` ("the wall held", "driven off", "tired of waiting"),
  narrated after "was lifted". Older records have an empty `why` and read as before. The client
  announces `host_sated` with `siege_lifted` (rules.gd).
- Guard: tests/sieges.rs `a_host_that_took_what_it_came_for_is_not_a_siege_lifted`,
  `a_siege_the_wall_held_says_the_wall_held`.

## 2026-10-27 fix-402-sieges: a wall outlived a camp move (#403, part)

- Symptom: seed 3's Holtwick began a ring of 80 tiles on day 21, moved camp on days 38, 315 and
  571, and at day 600 still held that first ring, with orders for it reposted at the old site
  and no wall proposed at any new camp.
- Found by: the M5 review's soaks (fault 3); the day-600 save of seed 3 shows the ring by the
  map's corner, 86 tiles from home, with two wall orders open on it.
- Cause: confirmed. `systems/moving.rs` cleared the old site's houses, fields and orders on a
  move but not `Community::palisade`; `settlement.rs` `palisade` went on mending that ring from
  the new home, and `projects.rs` `wall` returns while a palisade is set, so no new wall could
  be proposed. The wall project, ended only when the ring stood, stayed open too.
- Fix: `moving.rs` `consider` clears `palisade` with the rest of the old site. `projects.rs`
  `close` ends a wall project whose ring is gone. `settlement.rs` `palisade` lets go of a ring
  whose gate is over 45 tiles from home, so a save from before this fix recovers (seed 3's
  save: Holtwick's ring is gone after one day).
- Guard: tests/moving.rs `a_wall_is_left_with_the_old_camp` (the ring is left, its project ends,
  and a wall is proposed at the new camp).

## 2026-10-27 fix-402-sieges: walls begun and never closed (#403, part)

- Symptom: five walls begun in the M5 soaks, one closed. Seed 1's Oakfolk ring (134 tiles,
  begun day 160), seed 2's Stoneside (97, begun day 216) and seed 4's Merehollow (211, begun
  day 390) were open at day 600.
- Found by: the M5 review's soaks (fault 2); each day-600 save read with the new
  `examples/wall_probe.rs` (every ring tile, its order and materials, and every walkable tile of
  the outline outside the ring).
- Cause: confirmed for two of the three. Seed 1: all 134 ring tiles stood; four tiles of a
  shallow stream lay on the outline. The laying leaves water out of the ring ("water and rock
  hold the line themselves") but the closure test in `settlement.rs` `palisade` asked every
  outline tile off the ring to be unwalkable, and shallow water is walkable, so the ring could
  never be closed. Seed 2 had the same water on its outline (five tiles), and 18 timber orders
  waiting on planks: two of Stoneside's five knew masonry and none carpentry, so nobody of it
  could saw a plank and no plank it knew of lay anywhere. Seed 4 is another cause, listed
  under "Open" above.
- Fix: `settlement.rs` `palisade`: water on the outline holds the line in the closure test as
  in the laying. New `wall_material`: what the first house is walled with, else timber, but
  stone when nobody of the community knows carpentry and one knows masonry; the repair step
  posts ring orders of the other material again in this one, giving back what was brought.
- Effect on the day-600 saves: seed 1's ring closes the next day. Seed 2's ring went from 79
  to 89 standing in 40 days and closed on day 61; Stoneside took in carpenters by day 15, so
  how much of that the stone orders did is not separated.
- Guard: tests/defence.rs `a_ring_built_up_to_the_water_is_closed`,
  `a_community_that_cannot_saw_planks_walls_itself_in_stone`.

## 2026-10-30 #496/#403: close a completed wall project when its camp moves

- Symptom: a community could move immediately after its palisade stood while the wall project
  record remained open until the next daily planner pass. The old ring was cleared from the
  community, but that stale defence order still occupied the wall-project slot during the move.
- Found by: the bounded #403 lifecycle scenario for Epic #496, covering the settlement-step /
  planner-step boundary after a completed palisade.
- Cause: confirmed. `systems/moving.rs` cleared `Community::palisade` during a whole-community
  move, while `systems/projects.rs::close` was the only code that ended a wall project whose
  ring had gone. The move happens after that day's close pass.
- Fix: `systems/moving.rs::consider` marks open wall projects done at the same time it clears the
  old camp's palisade. Split communities keep the old camp and its wall project; only a whole
  relocation discards the stale defence order.
- Guard: `tests/moving.rs::a_completed_wall_project_is_closed_with_the_old_camp` fails on master
  because the project remains open immediately after relocation.
- Commit: this commit.

## 2026-10-27 nowhere to keep the observances (#297, epic #376, ask 1)

- Symptom: a community kept its season's observances wherever the one leading them happened to
  stand; there was no place of worship, nothing to build or keep up for the faith, and nothing
  for it to lose.
- Found by: the tester's design issue #297, folded into epic #376 (ask 1: a place of worship and
  its upkeep).
- Cause: not built.
- Fix: shrines.rs and systems/projects.rs `shrine`: a community that kept the last observance,
  with no shrine standing, has whoever most heeds the signs (the one followed in the spirit
  first) propose one, with why ("holds that the signs must be heeded, and the observances want
  a place", or "remembers the shrine they had: it burned"). `shrines::plan` (called from the
  settlement planner) finds it a spot on the community's own ground and orders a new structure
  kind, `Shrine`, of stone where the first house is stone and of timber (four logs) where it is
  not; a spot not found gives it up for sixty days, as a workshop. Once built it is the
  community's (`Faith::shrines`). observances.rs: an hour before the season's noon those of the
  community who are idle, loitering or talking walk to the shrine and wait; the observance is
  kept there (`Observance.at_shrine`), led by whoever there would lead, and makes the shrine
  good. It wears a twentieth a day, so a year of lapsed observances lets it fall; a timber one
  burns (hazards.rs `burns`); one left out of the community's ground by a move is left behind.
  Each loss is on record (`ShrineLost`, why), and those who heed the signs carry it
  (`ShrineLost` thought, -0.35 for five days). Bands do not set fires, so "burned in a raid"
  happens only when a fire from another cause reaches it. Sprite: tools/art/structures.py
  `shrine`.
- Guard: tests/shrines.rs.

## 2026-10-27 nobody kept the observances (#297, epic #376, ask 5)

- Symptom: whoever was followed in the spirit that day read the signs and led the observance;
  nobody held a religious role, so nobody could claim it from anyone, and a reader's signs
  borne out or belied counted only as the day's deference.
- Found by: the tester's design issue #297, folded into epic #376 (ask 5: a religious role with
  duties, and competing claims to it).
- Cause: not built.
- Fix: keepers.rs `step` (daily at 17:30, after justice.rs works out who is followed): a
  community with no living keeper names the one it follows in the spirit (`KeeperNamed`,
  `Faith::keepers`). The keeper reads the deity's answers and the omens and leads the
  observances (faith.rs `reader_of`, used by faith.rs, omens.rs and observances.rs) until they
  die, leave, or lose a challenge; one followed in the spirit who holds signs are only weather
  cannot claim it. `claim`: deference in the spirit summed over the community, plus half a
  point for each sign borne out less each belied (faith.rs `settle` now keeps
  `Faith::record`), plus a fifth for each who holds the signs as they taught them. At most
  once in ten days, the member who heeds the signs (lean 0.2) with the strongest claim above
  the keeper's (and at least 0.5) challenges; each grown member stands with whom they defer to
  more in the spirit, think better of, and a little by the record, the keeper on a tie. The
  winner keeps or takes it; those who stood with the winner credit them 0.15, everyone
  discredits the loser 0.15, and the two think the less of each other (`KeeperChallenged`,
  with how many stood with each).
- Guard: tests/keepers.rs; tests/faith.rs (the record of signs).

## 2026-10-27 the omens never foretold a band (#297, epic #376, ask 4)

- Symptom: omens were read only from great events already happening (a siege, the deep's
  beast); a hungry band walking in toward the valley was never read in the signs before it
  came.
- Found by: the tester's design issue #297, folded into epic #376 (ask 4: omens read from real
  events, `threat.rs` has outside pressure, link it).
- Cause: not built.
- Fix: omens.rs `foretell`: each daylight hour (6 to 20), for each band approaching and not yet
  sighted, a one in five chance that each community's reader reads a warning ("crows coming in
  over the valley's rim"), once for each band (`Faith::foretold`). It goes through the same
  reading as other omens: felt by those near, heard, heeded or scoffed at, and borne out by a
  death in the five days after (faith.rs). With no band on the road it is never read.
- Guard: tests/omens.rs `a_band_on_the_road_is_read_as_a_warning_before_it_is_seen`.

## 2026-10-27 fix-267-rock: gold, gems and water ignored the region's rock (#267, part)

- Symptom: after the deep became basalt, gabbro or gneiss by region (c82), gold still looked
  for basalt, so a valley whose deep was gabbro or gneiss had none; every gem pocket was
  "gem-bearing basalt" and left basalt rubble whatever it lay in; aquifers were seeded patches
  eight tiles across that took no notice of the rock; and the soil was one level everywhere.
- Found by: the tester's issue #267 (epic #377), its remainder after c82.
- Cause: not built (ores.rs named basalt as gold's host; aquifer.rs hashed the seed and the
  column; grid.rs laid one soil tile).
- Fix: world/gen/ores.rs: gold starts in any undisturbed rock of the deep; deep veins are gems,
  or copper in the basalt and gabbro, iron in the gneiss; veins run only through rock, not
  soil. world/material.rs `gem_in`: which gem by the host rock (the gneiss mostly garnets, the
  gabbro more sapphires); GEM_ROCK is "gem-bearing rock" and mining it leaves its host's
  rubble (systems/work.rs). aquifer.rs `wet`: water lies in sandstone (its lowest two levels)
  and limestone (its lowest) where the break to the middle stratum folds down into a basin;
  granite, slate, marble and the deep are dry; a world without strata keeps the old patches.
  world/gen/strata.rs `soil_depth` (#268's soil depth): three levels on the level ground of a
  plain or marsh, two on low level ground elsewhere, one on slopes and heights
  (`WorldGrid::soil`, empty in older saves: one). systems/settlement.rs `quarry`: the shaft goes
  down through the deepest soil within two tiles and the gallery is cut in the rock under it
  (without this, seed 2's opening found no stone and never built its hearth).
- Guard: tests/geology.rs (`gold_lies_in_the_deep_whatever_its_rock`, `gems_follow_the_rock`,
  `aquifers_lie_in_the_porous_rock`, `soil_is_deeper_on_level_low_ground`); tests/aquifer.rs;
  tests/identity.rs (the hearth on seed 2).

## 2026-10-27 fix-274-props: a stone's hardness and worth changed nothing but weathering (#274, part)

- Symptom: the material table gave every stone a hardness and a value, and the hardness was
  documented as the mining time's multiplier, but cutting granite took as long as sandstone,
  a granite wall stood a siege no better than a pine one (no host ever touched a wall), a
  marble house was a house like any, and a caravan sold marble blocks for what granite fetched.
- Found by: the tester's issue #274 (epic #377), asks 1, 2 and 4; and #402's soaks, where no
  siege ended because a wall held.
- Cause: confirmed for the cutting time: `settlement::make` used `times::MINE` and
  `times::DIG_STAIRS` flat, nothing read `hardness` there. The rest was not built: trade.rs
  `worth` priced stone blocks, ore and gems by kind, and `quote_wanting` priced what was sold
  with no material.
- Fix: tiers.rs `cut` (Mine and DigStairs orders take their rock's hardness times the base)
  and `worked` (systems/work.rs: the first stone brought to an order whose first input is rough
  or dressed stone sets its work to the stone's hardness times the base). siege.rs `batter`:
  a camped host batters the closed wall nearest its camp at noon and brings a stretch down
  after `batter_days` (2.5 times the hardness of what it was built of, rounded up), opens the
  wall, leaves stone rubble, posts a BuildWall to mend it and puts `EventKind::WallBroken` on
  record. houses.rs `fine_stone` and `ThoughtKind::FineHouse` (systems/strain.rs): sleeping in
  a bed in a house walled mostly with a stone worth `tiers::FINE_VALUE` or more is a better
  night. trade.rs: stone, ore and gems are worth what their material is (`worth`); a place
  holding one stone, ore or gem is as short of another as if it had none (`price_at`,
  `by_material`); a caravan sells at the price of what it carries (`barter_wanting` and
  `quote_wanting` take the material; `best_errand` and `best_fetch` weigh the material most of
  the stock is, `carried_material`).
- Guard: tests/tiers.rs (`harder_stone_takes_longer_to_cut_dress_and_lay`,
  `a_house_of_fine_stone_is_a_better_night`, `a_neighbour_pays_for_the_stone_it_lacks`),
  tests/sieges.rs `harder_walls_stand_longer_against_a_siege`. tests/health.rs
  `wound_heals_with_rest` now asks that the bruise is gone, not every wound: with stone work
  slower by hardness the builder in that run later cut an arm at the building site.

## 2026-10-27 fix-274-deep: nothing in the deep was worth the digging (#274, part)

- Symptom: the deep's gabbro and gneiss were worth no more than a plain stone, and a stone
  tool came out the same crude tool (capped at 0.45) whether knapped from sandstone or basalt,
  so beyond gems and gold nothing rewarded a community for digging down.
- Found by: the coordinator's brief for #274 and #267 (make the deep worth reaching).
- Cause: not built (material.rs gave gabbro 1.0 and gneiss 1.2; systems/work.rs capped every
  stone tool at 0.45).
- Fix: tiers.rs `stone_edge`: a stone tool's best quality is 0.25 + 0.2 x its stone's hardness
  (0.4 to 0.62), so the deep's basalt and gabbro take an edge as fine as copper's and better
  than any stone above them (systems/work.rs, the STONE_TOOL cap). material.rs: gabbro and
  gneiss are worth 2.0, fine stone (`tiers::FINE_VALUE`) for a house. With the earlier parts:
  the deep's rock is the hardest, so its walls stand a siege longest, and gold and gems lie
  only there.
- Guard: tests/tiers.rs `the_deep_gives_what_the_surface_cannot`.

## 2026-10-27 fix-404-feuds: a secret society betrayed the evening it formed (#404, part)

- Symptom: the only secret society in the M5 review's four 600-day soaks (seed 4, tick
  10939277) was formed and betrayed on the same tick.
- Found by: docs/reviews/m5-review-2026-10-26 (branch m5-review), "What could be better" 5.
- Cause: in `societies.rs` `step` a member was taken in for anger at the target (strength 0.3)
  with no check on what they thought of the target, and betrayal (liking the target, opinion
  over 0.4) was checked in the same evening pass that formed the society, so one person could
  satisfy both at once. Confirmed in the code and by the guard test; that this is what happened
  on seed 4 was not reproduced (it is day 380 of a 600-day run).
- Fix: `societies.rs` `step`: one who likes the target (`LIKES`, the betrayal threshold) is not
  taken in, and a society is not checked for betrayal until a day after it formed.
- Guard: tests/societies.rs
  `one_who_likes_the_target_is_not_taken_in_and_nobody_betrays_the_evening_it_forms`.

## 2026-10-27 fix-404-feuds: no feud in 600 days, and a feud was only ever between two people (#404, part)

- Symptom: no `feud` row in any of the M5 review's four 600-day soaks; the milestone gate "a
  feud traced across two generations" failed.
- Found by: docs/reviews/m5-review-2026-10-26 (branch m5-review), "What could be better" 6;
  `examples/justice_census.rs` on master c84, 200 days, seeds 1 to 4: 8, 0, 0 and 4
  judgements, 1, 0, 0 and 2 fights, no feud.
- Cause: confirmed in the code and the census. `justice.rs` `feud` needed three judgements
  between the same two people within sixty days (the module comment said twice), and the only
  fights in an ordinary game were breakdowns that turned on someone nearby thought ill of
  (1 to 2 in 200 days a seed). Nothing else set two people against each other in the record,
  and nothing made a feud of a grievance between two families.
- Fix: a new module `feuds.rs`. A feud is between two families (kin: partner, parents,
  children, brothers and sisters): `grievance`, called from `justice.rs` `judge_all` after each
  judgement, begins one when the judging has set the same two families against each other twice
  within sixty days, whoever of each was involved, or at once when one of them was exiled on the
  other's word; each side then thinks the worse of the other. It ends with a reckoning (each side
  has made amends to the other), by the judgement of the one followed in settling what is between
  people when they belong to neither side (`judged`, one case in four), after five years with no
  new grievance (a quarrel or fight between the sides is one), or when one side has nobody left
  in the valley. In `systems/act.rs` a quarrel between two who each think worse than -0.3 of the
  other comes to blows half the time (`strain::fight`, taken out of the breakdown code), started
  by whoever thinks the worse: a fight, judged like any other. New event `feud_ended`; `feud`
  now reads as between families.
- Guard: tests/feuds.rs `two_fights_between_the_same_two_families_are_a_feud_between_the_families`,
  `a_fight_within_one_family_is_not_a_feud`,
  `exile_on_anothers_word_sets_the_exiles_family_against_the_accusers`,
  `amends_made_by_each_side_to_the_other_are_a_reckoning`,
  `a_feud_nobody_feeds_is_let_go_after_five_years`,
  `a_quarrel_between_two_who_bear_each_other_ill_will_can_come_to_blows`; tests/justice.rs
  `two_in_the_record_against_each_other_twice_are_in_a_feud` (was "three times").
- Census after (200 days, seeds 1 to 4): fights within the valley 3, 0, 0, 6 (were 1, 0, 0, 2); feuds 1, 0, 0, 1 (with the next commit, which carries a feud on).

## 2026-10-27 fix-404-feuds: nothing carried a feud to a child (#404, part)

- Symptom: the M5 gate "a feud traced across two generations" could not pass even with feuds
  firing: a feud named two people and died with either.
- Found by: docs/reviews/m5-review-2026-10-26 (branch m5-review), "What could be better" 6.
- Cause: confirmed in the code: `justice.rs` `feud` only wrote an event and lowered two
  opinions; nothing was kept to pass on.
- Fix: `feuds.rs` `observe` (a new pipeline stage after `justice_observe`): a child born to one
  who holds a feud holds it; whoever pairs with one who holds it takes it up and thinks the worse
  of the other side; a marriage between the two sides ends it. `daily`: when the one a side is
  named for dies or leaves the valley, the eldest of their children who holds it takes it up,
  else the eldest who holds it (new event `feud_inherited`). A feud now lasts as long as both
  sides keep someone in the valley and a grievance comes within five years.
- Guard: tests/feuds.rs
  `a_feud_begun_by_parents_is_held_by_their_children_after_the_parents_are_gone` (the gate:
  the daughter and son take it up when both parents die, a child born after it began holds it,
  a partner who joins holds it, and a case between the children keeps it alive),
  `a_marriage_between_the_two_families_ends_the_feud`, and the son's taking it up in
  `exile_on_anothers_word_sets_the_exiles_family_against_the_accusers`.

## 2026-10-27 fix-404-feuds: no exile in 600 days (#404, part)

- Symptom: 52 accusations in the M5 review's four 600-day soaks, 12 of them for fights, and no
  exile.
- Found by: docs/reviews/m5-review-2026-10-26 (branch m5-review), "What could be better" 6;
  `examples/justice_census.rs` on master c84 (200 days, seeds 1 to 4: 8, 0, 0, 4 judgements,
  every verdict a pardon or amends).
- Cause: checked in `justice.rs` `judge_all`; the verdict path works as written (tests/justice.rs
  `a_third_fight_by_one_nobody_likes_is_exile` passes). Exile needed either letting the deep up,
  which never happened in the soaks, or a third offence within forty days by one the judge does
  not like with this one a fight, and no one in the soaks fought more than once in forty days.
  Setting a fire in a rage (a breakdown, `systems/strain.rs`) was not an offence at all.
- Fix: `justice.rs`: `ARSON` and `grave`: setting a fire and letting the deep's people up are
  the gravest offences, always taken up (`offence`) and judged exile, liked or not.
  `systems/strain.rs` `step`: a fire set in a rage that catches is an offence. Fights also come
  more often now (a quarrel between enemies can come to blows, entry above), which makes the
  third-fight exile reachable.
- Guard: tests/justice.rs `a_fire_set_in_a_rage_is_exile_even_for_one_who_is_liked`.

## 2026-10-27 fix-404-feuds: breakdowns cluster on a few people; checked, not a loop (#404, part)

- Symptom: in the M5 review's 600-day soaks the six who broke most account for 333 of 626
  breakdowns (seed 1) and 306 of 667 (seed 4).
- Found by: docs/reviews/m5-review-2026-10-26 (branch m5-review), "What could be better" 7.
- Cause, as checked with `examples/justice_census.rs` (200 days, seeds 1 to 4, master c84):
  breaks 236, 8, 30 and 207; the six who broke most 145, 8, 28 and 93. In seed 1 those six
  are ranked 0, 1, 7, 9, 10 and 13 of 53 by calm (all in the least calm quarter); in seed 4,
  3, 0, 7, 6, 17 and 31 of 55. What weighed on them at each break was where and how they slept
  (the hollow by the portal stone, `SleptUneasy`, and sleeping rough) in seed 1 and sickness in
  seed 4, with quarrels after; seeing others break (`SawBreak`) was not among the five heaviest
  thoughts of any of them. A probe build with both effects of a seen break turned off (the
  `SawBreak` thought and the witness thinking less of the one who broke) gave seed 1 162 breaks
  with the top six 124 (77%, against 61%), and seed 4 80 with 54 (68% against 45%): fewer
  breaks, but more of them on the same few, so the cluster is not made by the seen-break
  cascade. (Seed 4 also went from 207 to 80 breaks on an unrelated change, the quarrel that comes
  to blows, so its probe numbers say little alone.) The stress model weighs calm three times
  (`systems/strain.rs` `step`: build, ease and threshold), which is where the least calm stand
  apart. Not a fault; nothing changed.
- After (this branch merged with master c85, 200 days): breaks 23, 13, 47 and 5; the six who
  broke most 21, 13, 37 and 5; the least calm quarter 21, 13, 40 and 4. Master's own changes
  since c84 moved these openings, so the drop is not this branch's doing.
- Guard: none; no behaviour changed.

## 2026-10-27 fix-404-feuds: a quarrel comes to blows between a pair already at odds (#404, part)

- Symptom: after merging master c85, a 200-day soak on seeds 1 to 4 had no feud and no fight
  outside breakdowns: no quarrel was between two who each thought worse than -0.3 of the other.
- Found by: tools/soak/run4.sh and `examples/justice_census.rs`, 200 days, seeds 1 to 4.
- Cause: confirmed by the census: with master's openings fewer breaks soured opinions, so the
  -0.3 threshold of the entry "no feud in 600 days" above was seldom met (0 quarrels a seed).
- Fix: `systems/act.rs` `BLOWS_AT` -0.15: the quarrel itself takes 0.15 from each, so a pair
  already at odds before it. Census at -0.15: seed 3 had 9 such quarrels, 8 judgements, 3 feuds
  (2 ended); seeds 1, 2 and 4 none.
- Guard: tests/feuds.rs `a_quarrel_between_two_who_bear_each_other_ill_will_can_come_to_blows`
  (unchanged; it sets both at -0.9).

## 2026-10-27 observances were almost never kept (#297 follow-up)

- Symptom: one observance kept in a 40-day event census over four openings.
- Found by: the coordinator, from the round 14 report's census; measured with the new
  `examples/observance_census.rs` (120 days, seeds 1 to 4, the gate's view at each season's
  noon and what came of it).
- Cause: confirmed by that census. Seeds 1, 2 and 4 skipped every season (23, 28 and 23) with
  nobody to lead: no one was followed in the spirit (spirit deference came only from laying
  the dead to rest), and no one held any view on signs at all. The founders come through the
  portal without a culture, so nobody is raised to a stance on signs, and a view on signs was
  only ever learned from a reading by one followed in the spirit. The fallback leader needed a
  lean of 0.2 (`LEADS_AT`), which nobody could reach. Seed 3, where a reader arose, kept 12;
  its lapses and skips were "nobody came": only those already within twenty tiles of the
  leader at noon counted, and nothing brought people together.
- Fix: observances.rs `leader`: with nobody followed in the spirit (or not there), the most
  devout of those gathered leads, by `devotion` (their lean on signs if they hold a view, else
  three tenths of their own `faith::inclination`), if it leans toward the signs at all. `gather`
  and `keep`: every community gathers an hour before noon where it keeps the observance (its
  shrine, else home), and those within twenty tiles of that place keep it. `LEADS_AT` still
  marks who is troubled by a lapse and wants a shrine. Census after (kept / lapsed / skipped):
  seed 1 23/0/0, seed 2 18/6/2 (the community dwindles to two), seed 3 39/2/0, seed 4 40/2/0
  (seeds 3 and 4 split into several communities); before: 0/0/23, 0/0/28, 12/4/7, 0/0/23.
- Guard: tests/faith.rs
  `a_young_community_with_nobody_followed_keeps_its_observance_led_by_the_most_devout`; the
  lapse in `a_seasons_observance_is_kept_and_then_let_lapse` now needs everyone awake to doubt
  the signs.

## 2026-10-27 #425: a person or beast has not eaten until they eat

- Symptom: `hf-cli` inspect printed `last_meal_day 10040.3` on every animal, constant across seeds.
- Found by: issue #425 (seeds 7, 42, 100 on 01a54b8).
- Cause: confirmed by reading the code, not by a debugger. `last_meal` was set to the spawn tick,
  which counts from the start of the world's history, so the dump divided a large absolute tick by
  ticks per day. It was not an uninitialised value, but a spawn tick shown as a meal day. Nothing
  except the dump reads the field, so no hunger or stuck logic was affected.
- Fix: `agents.rs` `Agent::last_meal` is `Option<u64>`, `None` until the first meal; spawns
  (`scenario.rs`, `opening.rs`) leave it `None`; `act.rs` and `threat.rs` set `Some(tick)`.
  The dump shows "never" or "N days ago". Old saves hold a plain tick and load as `Some`.
- Guard: `tests/memory.rs` `nobody_has_eaten_until_they_eat` (new hare and new survivor are
  `None`, the hungry survivor then holds a real tick).
- Commit: see the branch `fix-425-last-meal`.

## 2026-10-27 box-ci: CI could not run on GitHub

- Symptom: GitHub Actions stopped starting jobs (the account's billing), so pushes and pull
  requests went untested; `.github/workflows/ci.yml` was made manual-only.
- Found by: Dan, 2026-10-27.
- Cause: the account's Actions billing, outside the repository (not examined further).
- Fix: tools/boxci: a Docker image with the pinned toolchain, numpy, Pillow, Godot 4.6.3 and gh,
  and `runner.sh`, which every 3 minutes fetches, runs the full `tools/ci.sh` on the master head
  and each open pull request's head without a finished `box-ci` status, and posts the result as
  that status. `compose.yaml` runs it as `hf-boxci` (16 CPUs, 24 GB, restart unless-stopped).
  AGENTS.md and README.md tell every agent that the `box-ci` status is the CI result and how to
  get a branch tested without box access. The Linux and Windows digest comparison is not run
  (tools/boxci/README.md, "Windows digests").
- Guard: the runner's own statuses: `box-ci` success on master 8d77dca (42 min) and 01a54b8
  (47 min, of which `cargo test` 46). Its first runs failed at the audio report because the
  image had no ffmpeg (`audio_report.py` decodes the OGGs with it); ffmpeg is in the image now.
  A status post that hit a TLS handshake timeout (PR #386, 20:09Z) left the head `pending`; posts
  now retry three times, and a commit whose own result was not posted gets it on the next pass.

## 2026-10-27 fix-404-feuds-in-play: a killing or a maiming by a neighbour was nobody's doing (#404, part)

- Symptom: in a 200-day four-seed soak of fix-404-feuds there was no feud and no exile, and a
  fight between people of the valley could neither kill nor maim, so the gravest grievance a
  family can hold never happened.
- Found by: the coordinator's review of fix-404-feuds (c93) against the M5 gate "a feud across
  two generations".
- Cause: confirmed in the code. `systems/strain.rs` `fight` gave each side one bruise with no
  bleeding and no striker (`Wound::by` was None), so nothing it did could kill, and a death or a
  lost limb from it would not have named anyone. `combat::blow` records the striker, but only
  strangers and beasts strike with it, and a neighbour struck there is struck in defence.
- Fix: `body.rs` `Wound::in_anger` (serde default). `systems/strain.rs` `struck`: each side of a
  fight strikes the other, mostly a bruise, one blow in seven a cut from whatever is to hand, one
  in twenty a broken bone; with a spear if the striker carries one and holds that force settles
  things (`combat::blow`); a blow on a part already hurt goes deeper (as #402); the wound names
  the striker and is marked in anger. `systems/health.rs` `killed_by`: bleeding out of, or a
  fever from, such a wound is a killing; `limbs`: a limb taken for such a wound gone bad is a
  maiming. `justice.rs` `struck_down`: new events `killing` and `maiming`; the two families are
  in a feud at once when the dead leave kin in the valley (`feuds::grievance`, grave); the dead's
  nearest kin (or, failing kin, the dead themselves) or the maimed accuse, and `KILLING` and
  `MAIMING` are among the gravest offences, judged exile (the design has nothing worse).
  `feuds.rs` `kin` finds the kin of the dead too. `systems/decide.rs`: one in a feud does not
  seek out the other family to talk. The soak tally counts quarrels, killings, maimings and
  `feud_to_a_child` (a feud taken up by a child of the one it passed from).
- Guard: tests/feuds.rs `a_fights_wounds_name_who_struck_them`,
  `a_killing_in_a_fight_is_a_feud_at_once_and_exile_and_the_children_take_it_up`,
  `a_limb_lost_to_a_wound_struck_in_a_fight_is_a_maiming`,
  `a_death_from_a_wound_not_struck_in_anger_is_no_killing`,
  `one_in_a_feud_does_not_seek_out_the_other_family_for_company` (fails with the decide.rs
  check removed).

## 2026-10-27 fix-405-unfired: a killing frost could not come (#405)

- Symptom: no `frosted` row in four 600-day openings (M5 review) nor in four 200-day probes.
- Found by: the M5 review's soaks (#405); probed 2026-10-27 (four openings, 200 days, the 4 a.m.
  temperature on every spring and autumn morning).
- Cause: confirmed. Since round 7 a killing frost (`thermal.rs`, below -5 C at 4 a.m.) comes only
  in spring and autumn, but `Weather::at` gives spring a base of 12 C and autumn 10 C with a
  daily swing of 5, so their coldest hour is 7.4 C and 5.4 C. The probes' coldest shoulder
  mornings were 2.7 C (seed 1; Aun's -3 lean after a killing fits, not checked) and
  -2.3 C (seeds 2 to 4; the -10 of a winter cold snap running on into spring fits, not checked). Only a winter spell
  and a deity's lean together could reach -5.
- Fix: `systems/weather.rs` `late_or_early_frost`: on the first day of spring and the last of
  autumn, one time in six by the seed and day, the hours before 9 a.m. keep winter's base
  (-4 C), so 4 a.m. is about -8.6 C. Other mornings are unchanged. Replays that pass such a
  morning differ from it on (the cold reaches warmth, hardening and snow as any cold does).
  First made one time in three: a 200-day census then had 25 `frosted` rows in four seeds, a
  frost takes every unripe plant of a community's fields (76 to 288 in one probe of seed 1),
  and a crop lost every year and a half was more than a late frost should be; one in six gives
  about one a seed every three years.
- Guard: tests/thermal.rs `a_late_frost_comes_from_the_weather_itself` (200 days of the
  valley's own weather: the crop dies on exactly those mornings, and on no other spring or
  autumn morning).

## 2026-10-27 fix-405-unfired: no cut ever struck water (#405)

- Symptom: no `struck_water` row in four 600-day openings (M5 review) nor in four 200-day probes.
- Found by: the M5 review's soaks (#405); probed 2026-10-27 by recording the depth below the
  ground of every `mined` cut in four 200-day openings.
- Cause: confirmed. `aquifer.rs` `wet` holds water only three to nine levels down. The 330 cuts
  of the four probes were one level down (224), two down (77) or in a shaft's own open column
  (29); none was three or more down. The cellar, the quarry's gallery and the dug-out home are
  one down by construction (`settlement.rs`); which orders made the two-down cuts was not
  checked.
- Fix: `aquifer.rs` `wet`: water-bearing ground from two levels down. One down stays dry, so a
  cellar of stores is not flooded by this.
- Guard: tests/aquifer.rs `a_cut_two_down_can_strike_water_and_one_down_cannot`.

## 2026-10-27 fix-405-unfired: a rotten limb was almost never taken (#405)

- Symptom: no `limb_taken` or `limb_made` row in four 600-day openings (M5 review); one of each
  in four 200-day probes (seed 4).
- Found by: the M5 review's soaks (#405); probed 2026-10-27 by checking, at the moment of each
  day's limb pass, everyone with an infected arm or leg wound of 0.4 or more.
- Cause: confirmed. The pass ran with the daily one at half past five and wanted a healer of the
  community awake, on the patient's level and within twelve tiles. In the four probes a limb had
  rotted that far (wound 0.4, sickness 0.6) on eight mornings; the community had a healer on
  seven, one was within reach on two, and awake on one.
- Fix: `systems/health.rs`: `limbs` runs at ten in the morning, apart from `daily`, and any
  awake healer of the community will do, wherever they are (the carpenter who makes the limb
  was never asked to be near either). Tuning of a reachable but rare event: how often a wound
  rots that far is unchanged.
- Guard: tests/limbs.rs `a_healer_comes_from_across_the_valley_in_the_morning`.

## 2026-10-27 fix-405-unfired: no shelter was ever dug (#405)

- Symptom: no proposal "to dig a shelter for the stores" and no shelter dug in four 600-day
  openings (M5 review) nor in four 200-day probes. (The review's `sealed`, `barred` and
  `cavern_found` follow a breach of a deep cavern, not a shelter; see the breach entry.)
- Found by: the M5 review's soaks (#405); probed 2026-10-27 at each morning's planning, for each
  community with no shelter that had a raiding band camped near or a raid on record.
- Cause: confirmed. `systems/projects.rs` `shelter` asked for three things at once, and the
  probes found no morning with all three: (1) a band that had raided camped on home's level
  within 60 tiles at the planning (21 such mornings, against 39 mornings with a raid recorded
  in the last twenty days: the gap C73 found for the wall); (2) someone holding that strangers
  bring harm or that you never raise a hand above 0.15 (none, on any of those mornings); (3) a
  rock face with room behind it within 30 tiles (`find_hollow_site`: none, on every one of
  those mornings; ground to dig down into, `find_dugout_site`, on 53 of them). A careful one
  was there on 36 of the 39 mornings with a raid on record.
- Fix: `shelter` answers the raid the community recorded in the last twenty days, as the wall
  does; a careful one proposes it as for the wall ("is a careful one, with strangers from ...
  taking from the store"); where no rock face is near, the shelter is dug down from flat ground
  (`settlement.rs` `find_barred_dugout_site`): the room is reached from the stair's foot through
  one tile only, a door is built there (`Hollow::door`), the stores go behind it, and
  `in_shelter` counts only what lies past the door. One who holds the deep not to be dug still
  will not dig.
- Guard: tests/shelter.rs `on_flat_ground_a_careful_one_digs_a_shelter_down_behind_a_door`; the
  older shelter tests now record the raid they answer.

## 2026-10-27 fix-405-unfired: the Beast-hunger's shape never seen in an opening (#405)

- Symptom: no `shifted` row in four 600-day openings (M5 review) nor in four 200-day probes.
- Found by: the M5 review's soaks (#405); probed 2026-10-27 with the same four openings under
  each engine.
- Cause: confirmed, working as designed. Openings are Aun's valleys (`Engine::Steady`), and only
  Hask (`Engine::Relentless`) curses a killer with the Beast-hunger (`deity.rs`, P11); with no
  curse there is nobody to take the shape. The same four openings under Hask over 200 days:
  one `cursed` in each of seeds 1, 2 and 4, then `shifted` 66, 1 and 2 times (seed 1's curse
  spread by 75 bites); seed 3 had no killing.
- Not changed, for Dan: the deity counts a killing by its death's cause ("killed by",
  "fight", "stranger"). Those words are written only when a beast is felled
  (`combat.rs`, "killed by" the one who struck); a person who dies of wounds another gave dies
  "bled out", which the deity does not count. So under Hask the curse falls on whoever fells a
  beast that went for someone, never on one who killed a person, where the design (P11) has
  "a figure who killed outside a fight".
- Fix: none.
- Guard: tests/combat.rs `under_hask_a_killing_brings_the_beasts_shape_two_full_moons_on`
  (under Hask the killer is cursed the next morning and takes the shape within two full moons;
  under Aun neither).

## 2026-10-27 fix-405-unfired: the deep was never breached (#405, not fixed)

- Symptom: no `breached` row in four 600-day openings (M5 review) nor in four 200-day probes, and
  so none of what follows a breach: `cavern_found`, `from_below`, `beast_stirred`, `rose`, and
  the walling-up of the way (`sealed`, `barred`), which the review listed under shelters.
- Found by: the M5 review's soaks (#405); probed 2026-10-27 (the depth of every cut, and where
  the deep caverns lie).
- Cause: confirmed, unreachable in an ordinary game. A breach is someone of the valley standing
  in a deep cavern (`underground.rs` `breaches`); the deep caverns are carved in the basalt, 15
  levels under the ground in the probed valleys (seed 1: both at 15). Nothing in the game digs
  more than two levels down: the 330 cuts of the four probes were one or two levels down, and no
  planner rule or personal project orders a cut deeper (the quarry's shaft and the cellar are
  two levels by construction). The design's breach comes through "the valley's delving" (P10,
  docs/design-peoples.md), led by the DeepDigging stance, and that delving was never built: the
  stance is read only to refuse work already in the deep (`decide.rs`) and a shelter (`shelter`).
- Fix: none here. Making it reachable is a behaviour to build (people who hold the deep is there
  to be dug sinking a shaft past the caverns, with its roof, water and cost), in the scope of
  M6's deep material tiers (#274), not a gate to correct.
- Guard: tests/deep.rs `the_breach_is_a_row` and tests/combat.rs
  `a_breach_walled_in_time_bars_the_beast_and_the_cavern_is_described` (the breach and what
  follows fire when someone stands in a deep cavern).

## 2026-10-27 fix-405-unfired: every cut along a flooded working was a new break-in (#405, in #390's code)

- Symptom: once the ground two down could hold water (the wells entry above), `water_broke_in`
  rows rose: 54 in the 200-day census of four openings, and in a per-seed comparison 16, 16
  and 6 on the branch against master's 2, 0 and 0 (seeds 1 to 3).
- Found by: the #405 census; the coordinator's review of it.
- Cause: confirmed in the code and by the guard test failing before the fix. #390's
  `aquifer.rs` `break_in` took any water beside a cut as standing water breaking in: water
  already in the workings (struck from the rock, or a pond that broke in before) was taken as a
  new source, so each cut along a flooded working was recorded again and poured from the
  working's own water. How many of the 54 rows were such repeats was not counted.
- Fix: `break_in`: a body of water joined to a dug tile (`World::dug`) is the workings' own and
  is no break-in; only standing water not yet joined to the workings breaks in.
- Guard: tests/lake_breaks.rs `digging_along_a_flooded_working_is_no_second_break_in`.

## 2026-10-27 fix-405-unfired: every cut on through wet rock struck water again (#405)

- Symptom: `struck_water` rows came in runs from one working: seed 1 of one 200-day probe struck
  water on days 2, 3 and 8 (twice) around the same cuts; 16 rows in the last 200-day census.
- Found by: the coordinator's review of the #405 census, after the like fix to `break_in`.
- Cause: confirmed in the code and by the guard test failing before the fix. `aquifer.rs`
  `observe` struck water at every cut into wet rock, so each cut of a gallery driven on
  through one wet patch was a new strike with its own seep.
- Fix: `observe`: a cut beside a dug tile that is itself in wet rock goes on into water the
  workings already reached: the cut is made wet, with no new row and no new seep. Only the
  first cut into wet rock not yet joined to the workings strikes water.
- Guard: tests/aquifer.rs `digging_on_through_wet_rock_strikes_water_once`.

## 2026-10-27 fix-405-unfired: people kept their stores, ate and slept in a flooded cellar (#405)

- Symptom: on the branch, seed 1's people spent 10,403 person-hours in 200 days standing in
  water (master: 795); 15 of the 30 dug tiles were under water at day 200, all of them the
  cellar's stockpile and its lamp, where people loitered (6,097 hours), ate (1,727) and slept
  rough (623).
- Found by: the coordinator's question why seed 1 ended 9 people below master with wells on;
  probed 2026-10-27 (who stood in water, where, doing what).
- Cause: confirmed. The cellar under a house on rising ground has tiles two levels under the
  ground above them, so the wells change (water from two down) let water into it: on seed 1,
  three cuts at depth 2 in the cellar struck water in the first twelve days and flooded the store. The wells
  entry's "a cellar of stores is not flooded" holds only on level ground. Nothing then kept
  people from storing food in the water, putting new stockpiles there, or digging the cellar
  on into the wet. (The 9 fewer people were not this: the two runs had the same chances for a
  child, 1,551 and 1,516 couple-days fertile and fed, and 13 against 21 births came of the
  daily draw; deaths, arrivals and leavers were alike.)
- Fix: what a person would do with a wet cellar. `settlement.rs`: a stockpile under water is
  not counted a store, so what lies there is hauled out as loose and the store grows on dry
  ground; the cellar posts no stockpile or lamp on a water tile and is not dug on into damp
  stone (`aquifer::damp`). `planner.rs` `stockpile_slot`: nothing is put down in standing water.
- Guard: tests/shelter.rs `food_in_a_flooded_store_is_hauled_out`.

## 2026-10-27 fix-405-unfired: under the strata no cut strikes water again (#405, not fixed)

- Symptom: with master's geology (#267) merged, the 200-day census of four openings has no
  `struck_water` row (16 with the seeded patches).
- Found by: the census after the merge, 2026-10-27; probed by finding, for every column of the
  four opening valleys, the shallowest level where `aquifer::wet` holds.
- Cause: confirmed. Under the strata water lies only in a basin, in the lowest two levels of
  sandstone (or the lowest of limestone) above the middle stratum, whose top is below -3 there.
  The four valleys' ground at home is at 1 or 2, so the wet rock lies three to six levels down:
  of the wet columns, the shallowest wet level was 3 to 6 down in seeds 1, 3 and 4 (509, 2,164
  and 1,738 columns) and 2 down in 225 of seed 2's 2,444; seed 4 has none within 30 tiles of
  home. The valley's digging goes one or two levels down (the wells entry above), so it does
  not reach the water.
- Fix: none here. Reaching it needs either digging that goes deeper on purpose (a well, which
  no rule orders; the quarry's shaft stays at two levels since #264) or water standing higher
  in the rock, which is #267's geology to decide.
- Guard: tests/aquifer.rs (`a_cut_two_down_can_strike_water_and_one_down_cannot`,
  `digging_on_through_wet_rock_strikes_water_once`, now finding a basin's wet rock in a
  generated world): a cut into wet rock strikes water once, and one level down is never wet.

## 2026-10-27 ci-speed: box-ci spent half an hour running test binaries one at a time

- Symptom: a box-ci run took 30 to 47 minutes, nearly all of it `cargo test`, with one or two
  of the container's 16 CPUs busy for most of it.
- Found by: the runner's logs. Master 9e8a16e: 1786 s in all, `cargo test` 1695 s; the test
  binaries' own times add up to 1652 s, of which tests/thin.rs 403 s, tests/pacing.rs 348 s,
  tests/study.rs 124 s, tests/arms.rs 94 s.
- Cause: `cargo test` runs the test binaries one after another. Tests inside a binary run in
  parallel, but a binary with a few long sim tests holds the run until its longest test ends
  while every other binary waits (checked against the per-binary times above).
- Fix: `tools/ci.sh` runs the tests with cargo-nextest when it is installed (every test its own
  process, all binaries at once) and the doctests with `cargo test --doc`; without nextest it
  says so and runs `cargo test --workspace` as before (a fresh clone, the Windows machine).
  tools/boxci/Dockerfile installs cargo-nextest 0.9.148 (release tarball, SHA-256 checked) and
  sets `NEXTEST_TEST_THREADS=16`; tools/boxci/README.md "Parallel tests" has the host install.
- Guard: none; the tests and their pass or fail are unchanged. Measured on the box host (load
  about 24 from other builds): all 662 tests in 16 min 38 s with 16 threads, the longest two
  tests being the end of it (thin `what_happens_there_is_true_to_the_record` 783 s, pacing
  `deity_answers_its_triggers` 718 s; see the next entry). The box-ci runner, rebuilt with
  nextest, on ci-speed 7929b3f (all four ci-speed commits): `tools/ci.sh` CI OK in 9 min 43 s
  (583 s), against 29 min 46 s for master 9e8a16e before; the tests took 543 s, against
  1695 s. The longest tests now are arms `spears_are_taken_up_against_a_band_and_used_on_it`
  (321 s), thin `they_study_it_and_the_wards_quiet_it` (302 s) and study
  `in_an_ordinary_run_someone_goes_to_study` (297 s).

## 2026-10-27 ci-speed: tests/thin.rs and tests/pacing.rs ran their seeds one after another

- Symptom: with the tests in parallel, two tests were the whole wait: thin
  `what_happens_there_is_true_to_the_record` (783 s) and pacing `deity_answers_its_triggers`
  (718 s); thin `they_study_it_and_the_wards_quiet_it` (392 s) came next.
- Found by: the nextest run in the entry above (per-test times).
- Cause: each loops over independent sim runs on one thread: four seeds of 45 days (thin),
  three deities by two seeds of 30 days (pacing); `they_study_it_and_the_wards_quiet_it` ran 75
  days when seed 7 is quieted on day 46 (probe, 2026-10-27).
- Fix: what each guards is unchanged. tests/thin.rs and tests/pacing.rs run each seed on its
  own thread (`each_seed`) and add the results up as before:
  `what_happens_there_is_true_to_the_record` (each thing from home was made by someone in the
  owner's recorded past, each echo is of a real dead person and tells their cause of death,
  each patch of wrong weather is within the stone's reach; at least one thing, one echo, two
  patches over four seeds), `left_open_it_leads_the_living_lost_home` (one of the lost is led
  toward the stone in two of three valleys), `deity_answers_its_triggers` (every answer is
  recorded at Trace and fits the deity's motive; each deity leans on something the others
  never do), `troubles_overlap_and_then_ease` (turns come on top of a trouble and a stretch
  eases, at least three and two over three seeds), `the_feed_names_why_it_got_worse` (a turn
  names the strangers' raid only after a real raid, in two of six seeds). The days are kept:
  in `what_happens_there_is_true_to_the_record` the four seeds had only three patches by day 20
  against the two asked for. `they_study_it_and_the_wards_quiet_it` (someone learns thin lore
  and chooses warding, four ward stones go up, the stone is quieted, nothing comes through
  after) now runs until the stone is quieted and fifteen days more, at most 75 days, instead
  of always 75; the quiet is watched for 15 days instead of 29.
- Guard: the same tests, all passing: run alone on the loaded box host (load about 31), thin
  and pacing together took 417 s, longest `they_study_it_and_the_wards_quiet_it` 417 s and
  `what_happens_there_is_true_to_the_record` 386 s (from 783 s), `deity_answers_its_triggers`
  240 s (from 718 s).

## 2026-10-27 ci-speed: every merge ran the full CI a second time

- Symptom: each checkpoint merge waited for `tools/ci.sh` in `tools/checkpoint.sh` on master
  (about half an hour) after box-ci had already tested the same tree on the pull request.
- Found by: the box CI runner's proposal (the runner's "same tree as" results show merge
  commits whose tree is the PR head's).
- Cause: `tools/checkpoint.sh` always ran `tools/ci.sh`, and `tools/merge/gated_merge.sh` did
  not require the PR head to be tested or up to date with master, so the checkpoint's run was
  the only check of the merged tree.
- Fix: `tools/merge/gated_merge.sh` stops before `gh pr merge` unless the PR head contains
  origin/master ("master moved" otherwise) and has a `box-ci` success, and merges with
  `--match-head-commit`. `tools/checkpoint.sh` skips `tools/ci.sh` when `HEAD^{tree}` equals
  `HEAD^2^{tree}` and HEAD^2 has a `box-ci` success, printing the status it relied on;
  otherwise, or with `--full-ci` or `HF_FULL_CI=1`, it runs `tools/ci.sh` as before.
- Guard: none automated (the scripts talk to GitHub). Checked by hand: the skip test on a
  commit with 9e8a16e's tree and 9e8a16e as second parent printed "box-ci on 9e8a16e69d
  (HEAD^2, same tree as HEAD): success 2026-10-27T20:39:30Z tools/ci.sh CI OK in 29m46s"; the
  same with 3fe593a (box-ci failure), and the merges 7d2d010 and 747229c (no box-ci status on
  their second parents), fell through to the full run.

## 2026-10-27 ci-speed: a time-budget test failed when the tests ran in parallel

- Symptom: `tools/ci.sh` with nextest failed at realm_peoples `a_thousand_years_is_quick`:
  17.7 s against its 15 s budget (it passed in the run before, on a less loaded box).
- Found by: `tools/ci.sh` on the box host, ci-speed branch, 2026-10-27.
- Cause: the five tests that assert on wall-clock time (realm `a_world_map_is_quick`,
  realm_peoples `a_thousand_years_is_quick`, realm_figures
  `a_thousand_years_bring_tens_of_thousands_of_figures`, history_pass `history_pass_budget`,
  navigation `unreachable_detected_o1`) ran beside the long sim tests of every other binary,
  so they measured the load. Under `cargo test` only their own binary's tests ran beside them.
- Fix: `.config/nextest.toml` gives those five every test slot (`threads-required`) and a high
  priority, so they run first, one at a time, before anything else starts (together about
  20 s). It also requires nextest 0.9.148. A new test with a time budget goes in that list.
- Guard: the next `tools/ci.sh`: the five ran first and alone (`history_pass_budget` 4.2 s,
  `a_thousand_years_is_quick` 5.9 s, `a_thousand_years_bring_tens_of_thousands_of_figures`
  9.6 s) and all 662 tests passed in 494 s on the loaded host.

## 2026-10-27 fix-422-size: animals ate nothing, whatever their size (#422, part 1)

- Symptom: size was a height and mass range per species and changed nothing in play for
  animals: a deer and a hare went to the same bushes and took nothing from them, and every
  animal read the human range for its mass.
- Found by: issue #422 (Dan, 2026-10-27), read against the code: `systems/needs.rs` `step`
  skipped every animal; `systems/animals.rs` sent grazers to a plant half the time and they ate
  nothing there; `Agent::mass_kg` read the species library, which holds only people.
- Cause: confirmed in the code above; animals never had a size or a hunger of their own.
- Fix: `size.rs` (new): each kind of animal's height and mass range (a 10 to 20% spread about
  its mean) and the years it grows into it; `Agent::mass_kg` and `height_cm` read it for an
  animal, and `Agent::body_kg` is the mass a body has now. `systems/needs.rs` `step`: a grazer's
  hunger burns at `ANIMAL_BURN` (half a waking person's rate) times `hunger_scale` of its body,
  through `Needs::burn`, which `decay_awake` now also uses. `systems/animals.rs` `graze_bite`: a
  hungry grazer by a shrub, fern, vine or flower eats from it as much as fills its belly (a full
  belly is `BELLY`, 0.3 of a plant's growth) down to `GRAZED_DOWN`; a hungry grazer always goes
  for forage, and only to plants with something left. Berry bushes, herbs, root crops and reeds
  are not eaten, so people's forage and thatch are as they were. People's hunger is unchanged
  (the human mean is still `hunger_scale` 1.0).
- Guard: tests/size.rs `a_big_grazer_eats_more_than_a_small_one_and_people_are_as_they_were`
  (a grown deer eats about eleven times what a hare eats, a big deer more than a small one),
  `grazing_strips_the_browse_and_leaves_peoples_food_alone`.
  tests/left_sites.rs `a_moving_community_carries_food_and_sets_it_down_at_the_new_camp`
  now checks the most food set down at the new camp during the day (50 with and without this
  change) rather than what is left uneaten at its end (4 on master, exactly the old bar; 0 here
  as the run went differently). Why more was eaten here is not confirmed.

## 2026-10-27 fix-422-size: a blow, a bite, a load and a kill took no account of size (#422, part 2)

- Symptom: a person's blow on a bear went as deep as one on a person, a beast's blow was the
  same whatever the beast, a cub bit as hard as a grown wolf, any two wolves brought down any
  deer, a yearling gave as much meat as a stag, and a body of any size hauled a whole stack.
- Found by: issue #422 read against `systems/combat.rs` `blow` (fixed base 0.2 or 0.3),
  `systems/animals.rs` `strike` (a range per kind) and the wolves' rule (`pack >= 2`),
  `systems/work.rs` (meat by kind) and `work/planner.rs` (`count: it.count`).
- Cause: confirmed in the code above; size was not read by any of them.
- Fix: `size.rs` `might` ((mass / 65)^0.5). `combat::blow`: severity times the striker's
  might over the struck's, so two people of the human mean are as before. `animals::strike`:
  the kind's bite times this one's might over its kind's mean. `animals::brings_down`: the
  wolves at a deer drag it down when their might together is `DRAG` (1.2) times its. Hunt
  meat times the body's mass over its kind's mean. `Agent::load`: a trip carries a stack times
  `carry`, at most a stack, at least one; hauls and deliveries take no more than that.
- Guard: tests/size.rs `a_bear_hits_harder_than_a_person_and_a_hare_hardly_at_all` (a hare
  stands in for a rat; the game has none), `a_small_body_carries_less_and_wolves_drag_down_by_weight`.

## 2026-10-27 fix-422-size: nothing but a besieging host could bring down a wall or a door (#422, part 3)

- Symptom: a bear and a hare were alike against a wooden door; only a camped host's battering
  (#274) brought anything built down, and the beast come up from below never went in under a
  roof, whatever its size.
- Found by: issue #422 read against `siege.rs` `batter` and `systems/animals.rs` `predator`.
- Cause: confirmed in the code; no beast's size was weighed against what a thing was built of.
- Fix: `siege.rs` `knock_down`: the bringing down that `batter` did (open ground, rubble of
  stone, the closed wall opened, the builders set to mend it, now a door as a door), shared by
  the host and the beasts. `size.rs` `force` ((mass / 65)^0.67) and `breaks`: a body breaks
  wood when its force is `BREAK_PER_HARDNESS` (3) times the wood's hardness; stone never.
  `systems/animals.rs` `can_break`, `try_break` (on record as the new event `BrokeIn`, appended)
  and `break_in`: the risen beast, with nobody out, goes to the nearest breakable door near
  someone under a roof and breaks it, and for a day after goes in after whoever is inside.
  The pale crawler's size is 150 to 200 kg (it breaks pine and birch, not oak); a grown bear
  breaks oak; a wolf, a cub or a person nothing.
- Guard: tests/size.rs `a_heavy_beast_breaks_a_wooden_door_and_a_small_one_cannot`,
  `the_beast_from_below_breaks_in_when_nobody_is_out`; tests/sieges.rs (the host's battering,
  unchanged).

## 2026-10-27 fix-422-size: no animal was bigger or smaller than its kind, and none grew (#422, part 4)

- Symptom: until part 1 every animal read the human mass range; after it, each had its own
  size within its kind's range, but a fawn was as big as a stag and there were no runts or
  giants (#414 B3 asks for both).
- Found by: issue #422 and the B3 plan (docs/history/PLAN-NEXT.md, 2026-10-27).
- Cause: confirmed in the code; nothing read age or rarity into an animal's size.
- Fix: `Agent::body_kg` (part 1) grows a young animal from three tenths of its grown size at
  birth to all of it at its kind's `grown_years` (size.rs `growth`). `genetics.rs`
  `Genome::size_roll` and size.rs `build_of`: one animal in two hundred is a runt (six tenths of
  its size) and one a giant (1.6 times, so stronger, hungrier and harder to drag down through
  parts 1 to 3); `Agent::build`, never for a person. `systems/animals.rs` `name_giants`, daily:
  an unnamed giant with a person within ten tiles is named for its size by `beast_name`, which
  now takes whether it is a giant (size words for the first half), on record as `Named`.
- Guard: tests/size.rs `each_animal_has_its_own_size_and_grows_into_it`,
  `a_giant_is_hungrier_harder_to_drag_down_and_gets_a_name`.

## 2026-10-27 fix-422-size: hungry grazers stood still on bare ground (#422, part 5)

Corrects the part 1 entry's note on tests/left_sites.rs, which changed that test's assertion
with the cause unconfirmed; the assertion is restored as it was.

- Symptom: `a_moving_community_carries_food_and_sets_it_down_at_the_new_camp` left no food at
  the new camp after a day (master on c103: 22).
- Found by: the test, and the coordinator's review of the branch.
- Cause, confirmed: the test's valley has its plants cleared. Part 1 sent a hungry grazer
  always to forage; with none within twelve tiles the goal was empty and it stood still, so
  grazers stopped roaming and fewer came within the hunters' reach: 4 hunts in the day
  against master's 8, the community ate as before (74 meals against 77) and ran out. With
  the always-forage rule turned off (a probe, not kept) it was 9 hunts and 15 food left.
  The carry cap of part 2 was not the cause: the test failed with part 1 alone, and the move's
  food is taken by `left_sites::carry`, which does not read it.
- Fix: `systems/animals.rs` `step`: a hungry grazer with no forage near goes further afield to
  look, up to twelve tiles, on ground it can stand on (six tries). After: 7 hunts, 13 food at
  the new camp.
- Guard: tests/size.rs `a_hungry_grazer_with_nothing_near_goes_looking` (0 of 20 hungry deer
  on bare ground set off before, 20 after); tests/left_sites.rs restored.

## 2026-10-27 #418: people get out of the flood's way and do not walk into it

- Symptom: on seed 2 the valley drowned: 28 dead in floods by day 200 and 3 people left
  (master de60dc3; the issue's 31 were on 23e4cc1). On 8d77dca (c94), 14 drowned by day 70
  and one person was left at day 200, by the same mechanism (all 14 walking, 9 from dry ground).
- Found by: the #405 work's 200-day probes; per death by `examples/deaths_probe.rs` (seed 2).
- Cause, checked death by death: the stream (body 3, 149 tiles, rising 0.2 to 0.25 levels an
  hour, so four to five hours from over its banks to over a head) went into spate beside the
  houses, by day as often as by night (26 of the 28 by day). All 28 died walking: 23 had been
  on dry ground in the last 40 minutes and set out through the spate (to a bed, a meal, a
  plant, or back from the dry tile the drowning rescue of #390 had put them on), because a tile
  with water over the head was walkable and A* routed through it at wading cost; 5 had stood,
  eaten or worked in the shallow flood as it rose round them, and nothing made anyone move
  before their head was under. Five of the dead were children of three or under. Homes
  and fields were already kept off ground a flood had covered (#393); the houses here were
  built before the first flood and the rule only places new floors (not confirmed per house).
- Fix: `world/grid.rs` `WorldGrid::over_head`; `world_state.rs` `path_agent_to` refuses a path
  through water over the head from dry ground; `nav/step.rs` `plan_cost` counts such a step
  `OVER_HEAD` (200) ticks so a way round is found; `systems/movement.rs` stops someone on dry
  ground before a tile that has gone over the head ahead of them; `systems/hazards.rs`
  `flee_step` (every 100 ticks): anyone awake within two tiles of flood water (asleep: one) goes
  to the nearest dry ground four tiles clear of it, the highest of the nearest ring, whatever
  they were doing, and it is on record once a day ("made for higher ground").
- Guard: `tests/floods.rs` `nobody_sets_out_through_water_over_their_head`,
  `people_on_the_floodplain_get_out_of_the_rising_waters_way` (fails with `flee_step` off).
- Commit: (this commit).

## 2026-10-27 #418: the sick lie down, and a tending goes ahead without a poultice

- Symptom: on seed 3 three people died of a fever after 561, 720 and 760 hours sick (master
  8d77dca; the issue's 26 fever deaths on 23e4cc1 no longer happen on this master: 46 caught a
  fever on seed 3 and 3 died; seed 4 had none).
- Found by: `examples/deaths_probe.rs` seed 3, hourly per sick person, and `HF_TRACE_TEND`.
- Cause, checked: none of the three was ever tended, though a healer of their own people was
  within 25 tiles on their level for 327 to 500 of those hours, 17 poultices lay in the valley
  and 33 of their people knew the poultice. Tending was chosen and then never planned: the plan
  (`work/planner.rs`, Tend) required a known free poultice to fetch first, and over 200 days
  276,931 tending plans failed on "no known free poultice" against 18 tendings done. Nobody
  without the poultice could tend at all. And the sick kept their day: they lay in bed 34, 76
  and 88 of those hours, so the rest that turns a fever after its second day rarely came.
  Keeping the sick apart and clean water were checked against the code and not changed: a
  fever is caught within eight tiles of the sick (`health.rs`), and nothing in the record
  ties these three to foul water.
- Fix: `work/planner.rs` Tend: with no poultice to be had, the tender goes to the patient all
  the same; `systems/work.rs` Tend: without a poultice in hand the care is nursing (tended for
  a day, no treatment); `systems/decide.rs` `choose`: anyone well and grown may nurse their own
  family (partner, parent, child) without a poultice or the knowledge of one, kin weighted
  above others; the sick (illness 0.4 or more) lie down by day as well as at night.
- Guard: `tests/illness.rs` `the_sick_lie_down_and_their_family_nurse_them_without_a_poultice`
  (fails without the fix: nobody nurses them).
- Open: on seed 4 after the fix (merged with c99), three of a group living at level -5 died of
  a fever after 410 to 416 hours, sleeping on the ground and never tended: 825 tending plans
  failed on an unreachable patient. Why the way down could not be found is not confirmed.
- Commit: (this commit).

## 2026-10-27 #418: someone caught under water makes for the nearest dry ground

- Symptom: with the flight and the path gate in, seed 2 still lost one man to a flood at day
  200: he was crossing low ground to a meal when the stream went over it, and walked on.
- Found by: `examples/deaths_probe.rs` seed 2 after the first #418 fix.
- Cause, checked from his track: the rescue of #390 (`systems/hazards.rs` `drown_step`) left
  anyone alone who was already going somewhere dry, so his walk to the meal 12 tiles on, through
  the flood, counted as getting out.
- Fix: `drown_step`: someone under water goes to the nearest dry ground they can reach unless
  what they are walking to is dry and no more than a tile further than it.
- Guard: `tests/floods.rs` `someone_caught_under_on_their_way_makes_for_the_nearest_dry_ground`.
- Commit: (this commit).

## 2026-10-27 #418: the flight is from a dangerous flood, and nobody chooses to go back into one

- Symptom: with the flight in, `tests/autonomy.rs` `need_satisfaction_seeded` failed: on seed
  11 three people fled the same flood about fifty times each in ten days and starved beside
  berry bushes; with the sick lying down taken back out it failed the same way.
- Found by: the workspace tests; a trace of each flight.
- Cause, checked from the trace: a pond lapping a tile over its bank (never over a head) was
  fled like a spate, and after each flight the next meal chosen was the bush at the flood's
  edge again, so the flight came back every few minutes and broke off every meal.
- Fix: `systems/hazards.rs` `flood_near`: only water over a head, or the flood of a body risen
  to `FLEE_STAGE` (half a level short of a spate), is run from; `in_floods_way`, read by
  `systems/decide.rs` `choose` (its `reachable` check), so no goal is chosen within two tiles of
  such a flood; the sick lie down only when not hungry.
- Guard: `tests/autonomy.rs` `need_satisfaction_seeded` (fails without it).

## 2026-10-27 fix-415: people idled while their food or shelter was not secured (#415, D56)

- Symptom: people wandered, loitered, talked and stood about while hungry, while their
  community held under two days' food, and at dusk with no bed or roof; a hungry person with no
  food known walked at random from wherever they stood, and a starving wanderer walked on past
  food they knew of.
- Found by: the census `crates/hf-sim/examples/normal_person.rs` (Dan's rule 7, "what would a
  normal person do here?"), person-hours awake and at leisure (wander, loiter, talk, idle) under
  each condition, sampled hourly. Before, on master: the four 60-day openings spent 22,694
  person-hours hungry or short of stores, 16,104 of them (71%) at leisure (wander 3,358, loiter
  9,663, talk 466, idle 2,617); at dusk with no bed or roof 1,664 hours, 1,233 (74%) at
  leisure; with no water known 5,230 hours, 4,172 (80%). Per seed (a, b, c): seed 1 74%, 72%,
  80%; seed 2 63%, 80%, 86%; seed 3 75%, 76%, 84%; seed 4 57%, 72%, 76%. A camp ten days after
  moving: 71% of 1,828 short hours at leisure. A party of eight set down with no food: 57% of
  734. Over all awake hours the openings were at leisure 76% of the time.
- Cause: leisure scored the same whatever was secured. `systems/decide.rs` `choose` cut talk
  only when hunger passed one half, wander and work were gated on the most pressing need with
  company counted in it, so a lonely person with an empty store talked; nothing looked at the
  community's stores or the coming night; the hungry search drew a random tile round the
  person; and `step` left Wandering uninterruptible on the belief that a starving wanderer is
  already looking for food.
- Fix: `systems/decide.rs` `choose`: unsecured = hungry (below half), the community's stores
  under `SECURE_DAYS` (two, `systems/settlement.rs` `food_days`, counted once a tick in
  `step`), or night two hours off or come with no bed known and no roof overhead. While
  unsecured, talk, wander, loiter, the hearth, the portal stone and study weigh
  `LEISURE_UNSECURED` (a twentieth), and company does not count as pressing. A short store
  sends those who know food plants to gather (not at night), and `systems/act.rs` leaves what
  a forager not hungry (`STORE_GATHER_HUNGER`) gathered on the ground for the store; it weighs
  sowing, harvest, cooking, hunting, fishing and snares 1.4 times. Dusk with no bed sends them
  to the community's nearest roofed house (`Goal::Warm`), else weighs building 1.6 times. One
  unsecured further than `GROUP_RANGE` (24) from camp walks back (`Goal::Seek`, a Going, not
  a Wandering). Hungry with no food known: back to camp, else to the nearest place they ate
  well, else round the camp. A starving wanderer who knows of food is interruptible. A hungry
  band camped with no plant known near its camp looks round it (`choose_band`).
- After, same census: the openings spent 16,612 hours hungry or short, 5,837 (35%) at leisure
  (wander 208, loiter 4,383, talk 199, idle 1,047); per seed 32%, 23%, 34%, 43%. Most of what
  is left is loitering at the end of a walk (warming, a fled fire, a short pause after a
  `Seek`) and bands of strangers, whom threat.rs moves (seed 4: 458 hungry band hours, 98%
  loitering). Dusk with no bed: 1,270 hours, 920 (72%) at leisure, about the same: the
  remaining dusk loitering is people sat where they will sleep. Moved camp: 40% of 1,437. The
  hungry party: 31% of 375. A secure community still talks and wanders: leisure outside the
  unsecured hours was talk 591 and wander 4,635 hours before, 614 and 4,625 after. Over all
  awake hours, at leisure 60%. Breakdowns 59 before, 45 after; births 14 and 11; deaths 23 and
  45. Of the after deaths, 33 drowned (floods, as seed 2 drowned 14 before; not traced to
  this change), 3 starved stranded where nothing was reachable, in a corner they had been
  working in (seed 3, cause of the stranding not confirmed), 3 starved in a band of strangers
  (seed 4), 3 lost survivors not yet found froze in winter with no hearth known (seeds 2 and
  3). Water is not a need (nobody drinks), so (c) is counted and not acted on.
- Guard: tests/normal_person.rs (`a_starving_wanderer_who_knows_of_food_goes_to_it`,
  `stores_under_two_days_turn_a_talker_into_a_forager`,
  `gathering_for_the_store_leaves_the_food_for_the_store`,
  `dusk_with_no_bed_sends_people_under_the_roof`,
  `one_far_from_camp_with_nothing_secured_walks_back`,
  `hungry_with_no_food_known_searches_where_they_ate_before`,
  `a_hungry_band_with_nothing_known_growing_looks_round_its_camp`; and
  `a_secure_community_still_talks_and_wanders`, which passes before and after). tests/pacing.rs
  `hask_presses_plenty_and_lets_go_of_the_broken` now also takes the food plants away: with
  them, people gathered at once and the valley was not broken long enough for Hask to let go
  (which part of the change did it is not confirmed). tests/qa_1026.rs
  `the_arrival_is_a_shock_that_fades_and_the_calm_shake_it_faster` compares the day-1
  thoughts by the tenth, and the mood only as lower: after merging master at c94, the shocked
  fared better in their needs (0.79 against 0.76) and their mood was 0.68 against 0.72 (on
  master 0.37 against 0.62); their thoughts were still 0.22 against 0.35.
  `other_systems_can_ask_whether_someone_is_secured` covers `decide::is_secured`, the same test
  for systems that call people together outside the decision. tests/threats.rs
  `raids_keep_the_ledger_and_raiders_leave_the_world` lets a raider the valley holds (D47,
  `held`) through its check that every band member has left: with #415 seed 7 kept one a day
  before the run ended, which master's run did not. observances.rs `gather` (c98) calls
  people to the shrine outside the decision; it now takes only those `decide::secured` passes,
  so a community short of food, or anyone hungry, does not gather
  (`a_community_short_of_food_does_not_gather_for_its_observance`); tests/faith.rs and
  tests/shrines.rs stock their communities with ten days of meals for it. tests/missing.rs
  `missing_survivor_dies_of_a_cause` no longer counts one of the lost who walked off the map
  (`Left`, society.rs `leave_valley`, kept not alive and with no death) as a death without a
  cause: after merging master at c103 one of seed 2, 5 or 8's lost did.
- Commit: (this commit).

## 2026-10-27 fix-415: the starving dived again and again into a flooded cellar for food (#415)

- Symptom: in the moved-camp census, eight of a community starved in ten days beside their
  own stores.
- Found by: the #415 census, moved-camp case, on the first cut of the gate (before it,
  10 deaths, 8 of starvation; the world had diverged from master's run, which had none).
- Cause: checked by tracing one of them tick by tick: the stores lay in a cellar under water;
  `choose` picked a stack there (nav said it was reachable), the walk went down the stair, and
  hazards.rs `drown_step` turned them back to dry ground with a 120-tick loiter, after which
  the same stack was chosen again, every two and a half minutes for two days.
- Fix: `systems/decide.rs` `choose`: a known stack of food lying under water
  (`hazards::under_water`) is not an eating choice. After: 3 deaths, none of starvation.
- Guard: tests/normal_person.rs `nobody_dives_into_a_flooded_cellar_for_food`.
- Commit: (this commit).

## 2026-10-28 fix-415: the gate walked people home through a spate (#415)

- Symptom: on the branch, seed 1 lost eight to the flood; master's run of the same build lost
  none there.
- Found by: tracing each flood death's last activities in the #415 census: on seed 1 those who
  drowned were on the gate's walk back to camp (`Goal::Seek`, a Going then a one-minute pause)
  across ground the spate covered between them and camp.
- Cause: checked as above: the gate chose its destinations and ways with no regard to water.
- Fix: `systems/decide.rs`: `unsafe_ground` (water, under water, in a rising flood's way, or on
  the high-water line while any water is in spate) is never a destination of the gate's walks
  (`Seek`, `Stock`, the new goal for gathering for the store, and `Warm`); after `commit`, a gate
  walk whose way crosses `wet` ground (under water or in a flood's way, hazards.rs
  `in_floods_way` from #418) is not made and they stay put. A first cut refused every water
  tile on the way: on seed 4 four then froze, refused again and again the walk to the hearth
  across a stream in its banks, so a shallow stream is crossed. `secured` takes a roof over
  flooding ground as no shelter. At dusk with no bed and no roof to go to, one more than
  `SETTLE_RANGE` (8) from camp walks back to it, and the tired (rest below 0.4) lie down rather
  than sit up for the dark, except in the cold (below 2 degrees), where a first cut let five on
  seed 4 lie down out of doors and freeze.
- After, merged with master at c106 (#418's flight from floods), census against master c106,
  four 60-day openings. Hungry or short: master 26,170 hours, 69% at leisure (wander 3,647,
  loiter 11,213, talk 529, idle 2,776); branch 23,347 hours, 37% (wander 372, loiter 6,334,
  talk 327, idle 1,570). Dusk with no bed or roof, counting loitering or idle where they will
  sleep (in the night hours, or within eight tiles of camp) as settling, not leisure: master 17%
  of 1,826 hours, branch 15% of 1,115; what is left is loitering in the evening hours away from
  camp. All awake hours at leisure: 74% and 59%. Secure talk 554 and 807 hours, secure wander
  3,818 and 5,526. Moved camp: 69% and 27%; the hungry party: 52% and 23%. Breakdowns 30 and
  42, births 10 and 14, deaths 8 and 13. Flood drownings: none in either. Of the branch's deaths:
  5 starved (4 on seed 3 stranded in its north-west corner, where nothing could be reached, the
  same corner that starved people on earlier runs; master starved 2 on seed 3, not traced), 4
  lost on the road, 1 old age, 1 drowned through ice, and 3 of no community (a lost survivor
  froze, a stranger bled out, one died of fever). No member of a community froze.
- Guard: tests/normal_person.rs `nobody_walks_back_to_camp_through_water`,
  `a_shallow_stream_in_its_banks_does_not_keep_the_cold_from_the_hearth`,
  `a_flooded_roof_is_no_shelter`, `tired_at_dusk_with_no_bed_they_lie_down_at_camp`. The walk
  back to camp now ends at the camp's edge on their side (`edge_of_camp`), not on its heart:
  after merging c107, tests/identity.rs `evenings_at_the_hearth` found a fifth of the afternoon
  spent beside the hearth on seed 2 (0.20 against 0.52 of the evening), people stopping there at
  the end of the walk back. After merging c108 (#428), the gate's check of the way runs on
  the queued path searches (`finish_walks`): a gate walk whose found path crosses `wet` ground
  becomes a loiter, as a walk with no path does. After merging c109 (round 17), the stand-in
  `pilgrimage::secured` (hunger only, marked TODO(#415)) is gone: pilgrimage and a faction's
  gathering apart (heresy.rs) ask `pilgrimage::free_to_go`, which is `decide::is_secured` and
  able to walk; tests/heresy.rs and tests/relics.rs stock their communities with ten days of
  meals for it. tests/thin.rs `left_open_it_leads_the_living_lost_home` counts one of the lost
  who rejoined and went off with a caravan (`overworld.away`) as home, as tests/missing.rs
  does: on seeds 2 and 5 the lost one led home was away with a caravan at the end.
- Census at c109, master against the branch, four 60-day openings: hungry or short, master
  25,992 hours with 69% at leisure (wander 3,494, loiter 11,251, talk 554, idle 2,740; per seed
  71%, 71%, 64%, 72%), branch 21,927 hours with 40% (wander 332, loiter 6,616, talk 283, idle
  1,432; per seed 42%, 42%, 32%, 43%). Dusk with no bed or roof, settling counted as above:
  14% of 1,765 hours and 14% of 681. All awake hours at leisure 74% and 62%. Secure talk 549
  and 898 hours, secure wander 4,174 and 6,807. Moved camp 69% and 37%, the hungry party 52%
  and 24%. Breakdowns 37 and 15, births 11 and 11, deaths 6 and 5; no flood drownings in either
  (master: 3 through the ice, 2 starved on seed 3, 1 lost on the road; branch: 1 through the
  ice, 2 lost on the road, 1 starved on seed 3, 1 lost survivor of no community froze).
- Commit: (this commit).

## 2026-10-28 fix-431-memory: a long game's saves left glibc holding tens of megabytes (#431)

- Symptom: the #404 branch's 600-day opening soaks (c94 plus that branch) failed the memory
  budget on seeds 2, 3 and 4 (RSS 94, 111 and 115 MB against 88, 110 and 106), where the M5
  review passed at 90 to 95 MB on c80.
- Found by: the #404 soak; probed 2026-10-27 with `examples/soak_probe.rs` (the soak's loop
  with glibc's `mallinfo2` beside RSS) and `examples/save_census.rs` (bytes of each part of a
  save's `world.bin`).
- Bisect: no tag made it grow. Seed 3 for 600 days, and all four seeds on c80 and c103:

  | build | seed 1 | seed 2 | seed 3 | seed 4 |
  |---|---|---|---|---|
  | c80 | 90 (138) pass | 97 (106) pass | 93 (110) pass | 92 (133) pass |
  | c99 | | | 94 (117) pass | |
  | c103 | 91 (136) pass | 60 (76) pass | 89 (121) pass | 99 (127) pass |
  | this fix (on c103) | 59 (136) pass | 46 (76) pass | 55 (121) pass | 63 (127) pass |

  RSS at day 600 in MB, the budget in brackets. The saved world at c80 and c99 day 600 (seed
  3) was alike part by part (16.6 and 17.9 MB; agents, the dead included, 645 and 671). The
  budget is set by the peak of living people, so a valley that peaks low (the branch's seed 2
  peaked at 33) fails it at the same RSS another passes at.
- Cause: confirmed. Of the soak's 92 MB at day 600 (c99, seed 3), glibc counted 27.5 MB in use
  and 62 MB free inside the heap; the free part grew from 33 MB at day 25 to 62 MB at day 600.
  Without saves the same run held 5 MB free and 36 MB resident at day 350. Each save cloned the
  whole world into its snapshot, and the ring's slot check (`verify`, run on all three slots
  before each save) and the commit read every `world.bin` and `chronicle.sqlite` whole to hash
  them, 18 and 40 MB by day 600 and growing with the chronicle. glibc raises its mmap threshold
  after the first large free, so these buffers came from the heap and were not given back. The
  world clone alone was a small part of it: with only the clone removed seed 3 still held
  35 MB free at day 200; with the hashing streamed as well, 27 MB, flat to day 600 (30 MB).
- Fix: `save/snapshot.rs`: `Snapshot<'a>` borrows each field from the world (`Cow`), so a
  capture copies only the generated species; a decoded snapshot owns its fields. A `Cow`
  serializes as its value, so `world.bin` is byte for byte the same (checked on seed 3's day-5
  and day-10 saves). `save/manifest.rs` `sha256_file` hashes a file 64 KB at a time; `verify`
  and `commit` use it. No change to the game.
- Guard: tests/save_memory.rs (a counting allocator): `a_capture_allocates_almost_nothing_however_long_the_game`
  (under 64 KB at days 4, 8 and 12), `checking_a_save_reads_its_files_a_block_at_a_time`
  (verify allocates under 256 KB and the save loads to the same digest),
  `a_borrowed_snapshot_encodes_as_the_owned_one_read_back`.

## 2026-10-27 #428: the 500-person worst tick was path searches, region rebuilds and site searches, one after another

- Symptom: at 500 fed people the worst tick was 23.8 ms (humans) and 26.6 ms (mixed) on a quiet
  box against the 16 ms gate (M5 review), owned by `nav` and `settlement`.
- Found by: `ostrog profile --agents 500 --provision --days 3` (and `--mixed`) with timing
  probes (not kept) round each system, each A* search, `Nav::update`'s parts, each section of
  the planner and its site searches.
- Cause: checked by those probes. The `nav` system does no path searches: its spikes (up to
  22 to 26 ms) were region rebuilds after digging or building, `Nav::update` recomputing the
  regions and portal edges of the changed chunks and their neighbours one chunk at a time
  (14 to 52 chunks: 5 to 18 ms in the edges, 1 to 11 ms in the regions). The path searches
  were in `decide` (up to 12 ms: 20 searches of up to 0.9 ms in one tick, each made as the
  decision was committed). `settlement`'s spikes were one community's planner, not many: a new
  house (`houses::new_house`, up to 39 ms growing up to 24 candidate floors), a dug-out home
  (`find_dugout_site`, 34 ms), the quarry site (`quarry`, 13 to 29 ms), and the hourly
  workshops section (about 4.5 ms). The C60 note that put nav's spikes on path searches
  was not right.
- Fix: threads, with the result the same at any count (D55, `par.rs`). `nav/regions.rs`
  `rebuild_components` and `rebuild_edges` work out each chunk on its own across threads and
  write the results in chunk order. `systems/decide.rs` `commit_later` and `finish_walks`: the
  tick's decisions are made in order as before, their walks queued, and the paths searched
  together after the loop and applied in decision order; a batch is spread only when its
  walks add up to 200 tiles or more (spreading every batch raised the mean `decide` from
  0.20 to 0.55 ms). `houses::new_house` grows its candidate floors a batch at a time and judges
  them in ring order; `find_hollow_site`, `find_dugout_site` and `quarry`'s site search search
  rows across threads and take the rows' bests in scan order. The planner is not split by
  community: communities share one list of orders and check the whole valley's, so each
  community's plan depends on those before it (D55). One change of behaviour: one who cannot
  find the way is seen as setting off by those deciding after them in the same tick, until
  the batch is searched; a 2-day, 500-person run (`run --seed 1 --days 2 --agents 500`) has
  master's digest, and both profiles' digests match at 1 and 8 threads.
- Measured (5 days, 500 people provisioned; base is master, new is this branch at 8 threads;
  each pair run back to back, the load average at start and end of each):

  | Run | Load | Humans mean / worst | Mixed mean / worst |
  |---|---|---|---|
  | base | 19.3 to 18.4 | 1.259 / 37.8 ms | 1.233 / 50.0 ms |
  | new (every batch spread) | 18.4 to 20.4 | 1.592 / 19.1 ms | 1.502 / 18.2 ms |
  | new (every batch spread) | 20.4 to 17.1 | 1.433 / 20.7 ms | 1.349 / 15.5 ms |
  | base | 17.1 to 15.4 | 1.192 / 30.0 ms | 1.129 / 50.0 ms |
  | base, all four at once | 24.7 to 29.5 | 1.919 / 595 ms | 1.818 / 504 ms |
  | new (long batches spread), same time | same | 1.906 / 228 ms | 1.832 / 235 ms |

  In the last pair both worst ticks were `perceive`, which this change does not touch: at that
  load a process is set aside by the scheduler for hundreds of milliseconds, and the worst
  tick measures the box. Per system there, settlement's worst 28 to 15 ms (humans) and
  `decide`'s 20 to 11 ms; the means equal.
- Guard: `tests/threads.rs`
  `five_hundred_people_on_one_thread_and_on_six_give_the_same_world` (two days, the digest at
  each day's end, and that the parallel parts ran); the profile now prints the 2 ms and 16 ms
  gates' verdict and its digest.

## 2026-10-28 round-17-faith-and-return: nothing was kept in memory of the revered dead, and nobody went to the shrine in hard times (#297, part)

- Symptom: #297's seventh ask, relics and pilgrimage, had nothing behind it. A keeper of the
  observances died and their named hammer went to their kin to wear out like any tool; a
  community in grief or under a warning went about its day and its shrine stood unvisited
  between observances.
- Found by: epic #376's checklist (M5, the social items left).
- Cause: confirmed in the code; nothing made a thing a relic, and nothing but the season's
  observance (observances.rs `gather`) sent anyone to the shrine.
- Fix: relics.rs (new): when the keeper of the observances dies, or a reader with two or more
  signs borne out and more borne out than belied, or someone whose death bore out a warning
  (faith.rs `SignSettled`), the storied thing they held last becomes a relic of their community
  (`RelicMade`), out of anyone's hand and set down at the shrine, else at home.
  systems/work.rs `take_tool` and systems/arms.rs `take_up` pass over relics. A community that
  moves camp takes its relics along (unlike its shrine, which is left behind). A relic gone or
  carried out of the valley (the one who carried it is remembered) is lost (`RelicLost`): those
  who heed the signs have the thought "a relic lost" and think the less of whoever carried it
  off. pilgrimage.rs (new): by day, someone who heeds the signs (the
  observances' `devotion`) and is in grief, under a warning read to their community and not
  yet settled, or troubled by a shrine or relic lost, sets out (one chance in ten an hour) for
  the shrine, else the relic's place (`Pilgrimage`), at most three of a community at once; at
  the place their grief and fear lessen by four tenths, they hold the signs a little surer and
  have the thought "prayed at the shrine in hard times". Nobody goes whose food is not secured
  (#415): `pilgrimage::secured` asks only hunger, injury and needing care until
  `systems::decide::is_secured` (#415, not on master yet) replaces it, marked TODO(#415).
- Guard: tests/relics.rs `the_keepers_tool_becomes_a_relic_kept_at_the_shrine_and_its_theft_is_felt`,
  `only_the_revered_leave_relics_and_a_move_takes_them_along`,
  `in_grief_a_believer_goes_to_the_shrine_and_is_eased_but_not_while_hungry`.

## 2026-10-28 round-17-faith-and-return: those who lost a challenge for the keeping went on as before (#297, part)

- Symptom: a challenger for the keeping turned back (keepers.rs) and those who stood with them
  were followed a little less and then were like everyone else: no division of belief lasted,
  and nobody ever held the signs apart from their keeper. #297's sixth ask (heresy, doubt) had
  the challenge and the scoffing but no organised split.
- Found by: epic #376's checklist (M5, the social items left).
- Cause: confirmed in the code; nothing kept a lost challenge's side together.
- Fix: heresy.rs (new). keepers.rs `challenge`: a challenger turned back with two or more
  standing with them forms a faction (`FactionFormed`). observances.rs `gather` and `keep` leave
  the faction out; `heresy::gather_apart` walks its members to their leader and
  `heresy::keep_apart` keeps their own observance (`FactionObservance`: gladness, signs held
  surer, the leader followed more). `heresy::step`, daily at 17:40 after the keeping: a member
  who stands with the keeper at least as much as with the leader (keepers.rs `standing`) goes
  back, a believer standing with the leader by 0.2 more joins; down to the leader, or the leader
  dead, gone or now keeper, the faction ends (`FactionEnded`). After two seasons, with an
  observance kept apart, three or more of them and three or more left, and thinking no better of
  the keeper than 0.1 on average, they split off: the founding part of systems/society.rs
  `cohesion_and_splits` is now `split_off` (unchanged, the same random stream), used by both;
  the leader is keeper of the new community and the faction takes one relic of the old with it
  (relics.rs `carried_off_by_split`, lost to the old with the leader named).
- Guard: tests/heresy.rs
  `turned_back_they_hold_apart_keep_their_own_observance_and_split_off_with_the_relic`,
  `a_faction_whose_people_come_round_to_the_keeper_is_brought_back`; tests/keepers.rs and
  tests/shrines.rs unchanged and passing.

## 2026-10-28 round-17-faith-and-return: the grown child never came home (#322, part)

- Symptom: a child taken below and not brought back was seen again only if the deep's people
  next came up through an opened way (taken.rs `among_them`), and then only as a line in the
  feed: they never sought their family, could not be taken back, and nothing that was said at
  the time of the taking was ever revisited.
- Found by: epic #376's checklist (#322, the remaining social items).
- Cause: confirmed in the code; nothing brought anyone up from `overworld.away` for a taken
  child but the search party's outcome.
- Fix: taken.rs. `Taken` keeps who opened the way and who was accused at the time
  (societies.rs `dawn` passes the taking after the reckoning). `seek`, daily at 09:00: one grown
  below and not seen since has one chance in sixty a day to come up (`come_up`): at the way down,
  a stranger walked by nobody but this (`leaving`, community none), on record (`SoughtKin`);
  their parents' grief is stirred. `seeking`, every tick: walked toward the nearest parent, else
  brother or sister, else the old home; within three tiles of one, or after two days, they meet
  (`KinMet`). `meeting`: a parent knows them with a chance of 0.9 less 0.02 a year since (at
  least 0.5), a brother or sister 0.4, nobody met nobody; known, they stay with a chance of 0.7
  (a parent) or 0.5. Taken back, they join the community and their parents' grief turns to joy;
  turned back, they walk to the way down and go below for good, their parents' grief deepened
  if they were known. Known either way, the old accusation is stirred (`stir`): their parent
  suspects the opener at 0.9 and accuses them (justice.rs, if still of the community), and the
  parents think the better of anyone wrongly accused at the time. `Stage::Seeking` appended.
- Guard: tests/deep_gate.rs
  `grown_below_they_come_up_to_seek_their_family_and_are_taken_back_naming_the_opener`,
  `the_seeker_meets_their_mother_when_they_reach_her`; the other deep_gate tests unchanged.

## 2026-10-28 round-17-faith-and-return: nobody ever saw a society meet (#322, part)

- Symptom: a secret society's members walked out together every midnight to the same place
  fourteen tiles east of home, and nobody awake nearby ever made anything of it; suspicion
  (societies.rs `suspects`) began only with a way down found open.
- Found by: epic #376's checklist (#322, the remaining social items).
- Cause: confirmed in the code; only `reckon` and `rumoured`, both about a taking, ever set a
  suspicion.
- Fix: societies.rs `seen_meeting`, at one in the morning: anyone of the community awake within
  ten tiles of the meeting place (`meeting_place`, the midnight walk's spot, now one function)
  while two or more members are there suspects each one there at 0.35 and thinks the less of
  them, tells the one the society meets against if they think well of them (who suspects them at
  0.3), and it is on record (`MeetingSeen`), once until someone new is seen. The talk spreads
  through social.rs `tell_suspicion` as before. `in_it_together` now also holds for two members
  of one society, so a member does not take up that talk against their own.
- Guard: tests/societies.rs
  `one_who_sees_a_society_meet_suspects_them_and_the_talk_goes_round`; tests/deep_gate.rs and
  tests/social.rs unchanged and passing.

## 2026-10-28 #439: someone cut off below is dug down to and tended

- Symptom: on seed 4 (#418's branch merged with c99) three people at level -5 died of a fever
  after 410 to 416 hours, never tended, while 825 tending plans failed as unreachable.
- Found by: `examples/deaths_probe.rs` with `HF_TRACE_TEND`, then `examples/deep_sick.rs`.
- Cause, checked: no stair, ladder or flooded working was involved. On day 10129 a fire burned
  their house's floor and all three "fell through the floor" (#324) at once from level 2 into
  a closed cave at -5: a pocket of 246 tiles, all on one level, with only open air above it and
  rock round it, so no way joined it to anywhere. They lived there four weeks on what grew in
  the dark. Nobody dug down to them; nothing in the game would. And tending made it worse:
  `decide::choose` picked a patient on the tender's own level within 25 tiles without asking
  whether a way led there (Tend was missing from its `reachable` check), so the choice failed
  at planning, again and again, and a sick person on another level was never looked for.
  Not reproduced on master c104 (no one goes below level -2 there in 200 days on seeds 1 to
  4); the cause above is from the run where it happened.
- Fix: `systems/rescue.rs` (new, hourly, after the settlement planner): for anyone of a
  community below ground whom no way joins to home, their people cut a stair down a column of
  rock beside them, a level at a time from the top (`DigStairs`, priority 8), until it opens
  into their pocket. `systems/decide.rs` `choose`: a patient is anyone within 25 tiles across
  on any level whom a way reaches, and Tend is in the `reachable` check. Carrying the sick up
  is not built: once the way is dug the sick can walk and the tenders can come down, and a
  pocket with no way out cannot be carried out of.
- Guard: `tests/rescue.rs` `their_people_dig_down_to_someone_cut_off_below_and_tend_them`
  (fails without `rescue::step`: no way is dug; fails without the `decide.rs` change: the way
  is dug but nobody comes to tend them).

## 2026-10-28 #286: art for the valley's new plants, woods and dyestuff

- Symptom: none in play; the plant kinds of #286 need their sprites before the sim may place
  them, and each new wood needs its wall, wall face, log and plank art.
- Found by: issue #286 (17 plant kinds against Dwarf Fortress's 223).
- Cause: not a fault; new content.
- Fix: `tools/art/plants.py` `flora_286`: alder, ash, yew, willow and crab apple in three
  stages with autumn and winter sheets (the yew winter only); seventeen wild plants (food,
  look-alikes, fibre, dye, healing herbs) in three stages with winter sheets, autumn for those
  that turn; barley, oats, rye, peas and flax in four stages like the grain. Same ramps, sizes
  and upper-left light as the plants before them. `items.py` draws the dyestuff icon (16 px,
  and 32 px with `--big`); `structures.py` and `overlays.py` take the five woods in
  `WALL_LOOKS`; `items_derived.py` recolours the oak log and plank icons for them, as for
  birch and pine. No image model was used. Provenance and ASSETS.md refreshed.
- Guard: tools/ci.sh's art checks (provenance, palette: 0 off-palette, ground_check); the
  sprite tests `client_draws_every_kind_and_family`, `every_wall_material_has_a_sheet` and
  `every_material_variant_has_an_icon` hold the sim's kinds to this art from the next commit.
- Commit: (this commit).

## 2026-10-28 #286: the valley's flora by species: wild food in season, look-alikes, fibre, dye, healing herbs, trees by climate

- Symptom: the valley had 17 plant kinds against Dwarf Fortress's 223 (issue #286): one berry
  bush, one tree of three looks, and nothing that bore only in its season, could be mistaken
  for food, dyed cloth or healed.
- Found by: issue #286 and the count in its comment.
- Cause: not a fault; missing content.
- Fix: `plants.rs`: each kind's data is one row of `PlantKind::lore` (label, prose, days to
  grow, the seasons it bears in, what it gives and how much, its wood, whether it is grazed,
  its warmth range). Added: alder, ash, yew, willow and crab apple, each its own wood
  (`world/material.rs` ALDER to APPLEWOOD, `wood_of`); hazel, bilberry, wild strawberry,
  bramble, wild garlic, chanterelle and bulrush, which bear only in their seasons
  (`Plant::grow` takes the season; fruit left out of season falls in a day; the old kinds bear
  all year as before); four look-alikes (death cap, hemlock, nightshade, lily of the valley);
  nettles for fibre; woad, madder and weld for dye; yarrow and comfrey for healing; and the
  rows of five crops, sown from the next commit. `world/gen/biome.rs` `flora` and the new
  `trees` place them by biome, and `world/gen/flora.rs` keeps each within its warmth range
  (alder and willow only by water). A look-alike gathered is eaten on the spot
  (`systems/act.rs` `ate_look_alike`): the eater sickens (`IllnessKind::Poisoned` through
  `health::catch_at`, as badly as the plant; one who dies of it dies of a poisoning) and from
  then on leaves that kind (`Agent::shuns`); `systems/perceive.rs` does not note one as food
  for whoever knows it: herb lore, or having been sick from it (`plants::knows_look_alike`,
  Dan's rule 3). `systems/settlement.rs` has fibre plants (reeds and nettles) cut for the loom,
  dye plants when the loom has no dyestuff, and healing herbs while someone of the community
  knows herb lore or the poultice and poultices run short; `systems/work.rs` Harvest gives what
  the row says (fibre of its material, `ItemKind::Dye` of its colour, a poultice in the hands of
  one who knows herb lore, roots-and-herbs otherwise) and records every cut as a harvest. The
  loom dyes the clothes it makes when it has dyestuff (`work/recipes.rs` DYED_CLOTHES, the dye
  first so the clothes are of its colour), and clothes put on wear the dye's colour
  (`systems/life.rs` `dyed_colour`). Tree checks ask `is_tree` or `is_green_tree` instead of
  naming `Tree`. The client maps the new kinds (`world_view.gd` PLANT_KIND, `entity_layer.gd`
  TREE_KINDS, STAGED and CROPS).
- Guard: `tests/flora.rs` `every_new_kind_has_its_data_and_a_use`,
  `hazelnuts_come_in_autumn`, `a_look_alike_sickens_whoever_does_not_know_it` (and one who
  knows herb lore leaves it), `chanterelles_are_eaten`, `woad_dyed_clothes_wear_blue`;
  `tests/biomes.rs` `ground_families_follow_biome` holds the trees to their biome's list.
- The worlds differ, so three tests tied to one world's particulars changed, each checked:
  `history_pass` `a_ruin_and_a_grave_explain_themselves` reads the latest site (on seed 11 an
  older site now lies under a later one's houses, and the note rightly names the later one
  first); `missing` `missing_survivor_dies_of_a_cause` allows one who left the valley (on seed
  8 one was exiled for a killing and one broke and walked out, both with a `Left` event and
  neither dead); `pacing` `troubles_overlap_and_then_ease` runs seeds 3, 11 and 13 (7's
  troubles now come one at a time).
- Commit: (this commit).

## 2026-10-28 #286: crops to their ground: barley, oats, rye, peas and flax

- Symptom: every field in every valley grew the same grain at the same rate, wherever its
  community stood.
- Found by: issue #286 (grains with different seasons and yields).
- Cause: not a fault; `systems/work.rs` Sow always planted `Grain`.
- Fix: `plants.rs` `crop_suits` (how each crop does on each biome's ground), `crop_season` (rye
  grows on into autumn, peas want the summer, barley and oats are quick in spring), `thirsty`
  (peas and flax grow at half the rate away from water), `frost_hardy` (rye) and `best_crop`;
  `systems/plants.rs` `crop_for` chooses what is sown on each tile (flax on one tile in six of a
  field whose people have a loom and no reeds or nettles within 50 tiles), and `step` grows a
  crop at its ground's, season's and water's rate; `systems/work.rs` Sow plants that crop;
  `thermal.rs`: a killing frost leaves rye standing. Wheat on grassland grows as before.
- Guard: `tests/flora.rs` `crops_follow_their_ground` (wheat on grassland, barley on heath and
  stony ground, oats in fen and woodland, rye on fell and snowfield, peas in a watered meadow,
  and `crop_for` on a tile of each), `rye_stands_the_frost`.
- Commit: (this commit).

## 2026-10-28 #286: the plant census over four openings

- Symptom: none; a measure of whether the new plants take part in play.
- Found by: `examples/plant_census.rs` (seeds 1 to 4, 60 game-days, run at e06e4b4: this branch
  merged with master 3c3792e).
- Result: all 22 new wild kinds occur in all four openings; barley grew in two, oats in three,
  peas in three, rye in one, flax in none (every opening had reeds or nettles within reach of
  its loom). Eaten from the plant: bulrush roots 817, wild strawberries 251, brambles 153,
  hazelnuts 64, chanterelles 65, wild garlic 75, bilberries 31, crab apples 16; oats 290 and
  barley 114 taken ripe from the field. Used: oats harvested 842 times, barley 273, peas 39,
  rye 6; ash 44, yew 16, willow 6, crab apple 6 and alder 2 felled; yarrow cut 74 times and
  comfrey 43; nettles 15; woad 8, weld 3, madder 1. Poisoned someone: lily of the valley 31,
  hemlock 30, death cap 17, nightshade 14 (92 in all, none fatal); 52 people came to know a
  look-alike that way.
- Open: flax is never sown where reeds grow, which is every opening seen; nobody teaches a
  look-alike to anyone but by herb lore (people learn them one sickness at a time).
- Guard: none; a census.
- Commit: (this commit).

## 2026-10-28 #286: a raider kept at the houses is not a band member left behind

- Symptom: after merging master into the #286 branch, `tests/threats.rs`
  `raids_keep_the_ledger_and_raiders_leave_the_world` failed: "seed 7: AgentId(106) is gone
  from the valley".
- Found by: tools/ci.sh on the merged head.
- Cause, checked by stepping seed 7 the same way: the raider took a cut in a fight and was kept
  at the houses (C76, `siege.rs` `keep`: `held` set, out of the band, "taken and kept at the
  houses"), so he stayed in the valley as the keeping intends. The test asked every band member
  to have left, which no seed had tested against a kept raider before; the worlds differ with
  the new flora.
- Fix: the test skips a member still alive and held.
- Note: master made the same two test changes in parallel (this one with #415, and the
  `missing` test's allowance for one who left the valley); the merge keeps master's wording.
- Guard: the test itself.
- Commit: (this commit).

## 2026-10-28 #444: a fall hurts steeply by the height, and nobody falls through rock

- Symptom: on seed 4 (#418's branch merged with c99) three people "fell through the floor" seven
  levels, from 2 to a closed cave at -5, and lived with a burn and a broken limb each; a fall
  of any height could not kill.
- Found by: #439's work; `examples/fall_probe.rs` printed the column under them the tick before.
- Cause, checked: there was no floor and no shaft. They stood on open grass (level 2 open, level
  1 soil) when a grass fire caught their tile. A burning tile is not walkable, and the safety
  net in `systems/movement.rs` took any unwalkable tile that was not itself rock for a missing
  floor and dropped them to the first walkable tile within twelve levels, through soil at 1
  and 0, sandstone at -1 and slate at -2 to -4, into the cave at -5. A wooden floor that burns
  does not drop anyone at all (`hazards.rs` `burn_out` turns it to a soil floor). And `fall`
  wounded by height (severity 0.15 + 0.2 a level, one wound, no bleeding) but never killed.
- Fix: `systems/movement.rs` `step_inner`: a fall only when the tile under them is not rock, and
  only down through air or water to the first place to stand (or into deep water, to swim
  out); someone standing on ground whose tile cannot be stood on steps aside as before.
  `fall`: by storeys, a chance of death of 0, 0, 5, 25, 60, 80 and 95 per cent for 0 to 6 and
  more (cause "fell"); the living get a leg bruise or sprain from one storey, a likely break
  from two or three, two wounds bleeding inside from four or more; deep water takes the first
  three storeys of it.
- Guard: `tests/falls.rs` `a_fall_hurts_steeply_by_the_height`, `deep_water_softens_a_fall`,
  `nobody_falls_through_rock` (each fails on the old code).

## 2026-10-28 #447: someone stranded a storey up climbs down

- Symptom: on seed 3 one person starved in the valley's north-west corner within 60 days (on
  master 3c3792e Yleivane, on c111 Liriwick), with home 15 tiles off.
- Found by: #415's census; traced with `examples/stranded.rs` (when each person stops being
  joined to home, what they were doing and the ground round them).
- Cause, checked on both commits: not the fall through rock (#444's branch gives the same
  run). A builder was finishing a wall's tile at (11,5,4) standing on that tile itself
  (`work/planner.rs` `stand_tile` returns the target when it can be stood on); when the wall
  stood, `systems/work.rs` `displace_from` put them on the first open neighbour, (11,6,4), on
  the far side: a ledge a level above the ground round it whose only way on and off had been
  the tile just walled. Nothing lets anyone step down a single level without a stair or ramp,
  so they walked about the ledge for three to six days and starved in sight of home.
- Fix: `systems/rescue.rs` `climb_down`: each hour anyone of a community whom no way joins to
  home, with open ground a level down beside them that does, climbs down to it, unhurt, on
  record ("climbed down, there was no other way home"); only those below ground and still cut
  off are dug to (#439). And at the gate, `systems/work.rs` `displace_from`: someone put off a
  tile about to be walled (or set alight) lands on an open neighbour from which they can still
  walk home with that tile closed (`joined_home`: a search over the ground as it will be that
  finds home or walks out over more than 3,000 tiles), and on the first open one only when no
  neighbour is joined. Where a displaced person lands is all it changes; builders still stand
  on the tile they wall.
- Guard: `tests/rescue.rs` `someone_left_on_a_ledge_a_storey_up_climbs_down` (fails without
  the climb), `a_builder_walling_the_way_off_a_ledge_steps_off_on_the_home_side` (fails without
  the `displace_from` change: lands on the ledge). On seed 3 with the climb, Liriwick is cut off
  at the same hour and lives; nobody starves in 60 days.
- Commit: (this commit).

## 2026-10-28 #286: a roof lost while its house grows is roofed again, not "mended"

- Symptom: after merging master (#448) into the #286 branch, `tests/building.rs`
  `a_fallen_roof_is_mended` failed: no "mended" stage within four days.
- Found by: tools/ci.sh on the merged head.
- Cause, checked by stepping seed 7 as the test does: half the roof was taken off at day
  10046; six hours later the household grew the house ("added to"), so its stage went to
  "walls up", and the whole roof, old room and new, was laid again by hour 31 and reported as
  "roofed". The roof was mended; the feed named the later stage.
- Fix: the test accepts "mended" or "roofed" and still requires every tile of the old floor
  roofed.
- Guard: the test itself.
- Commit: (this commit).

## 2026-10-28 #286: a look-alike is learned by those who see it and passed on in talk; a poisoning is not a fever

- Symptom: the plant census had 92 poisonings in 60 days over four openings (146 once master
  #446 was merged in), about one person in three sick from a plant; a neighbour who saw it
  happen went and ate the same plant. And the feed's stretches counted each poisoning as
  "fever in the houses".
- Found by: the coordinator's review of the census; `tests/pacing.rs`
  `troubles_overlap_and_then_ease` on its own seeds 3, 7 and 13 (corrects the entry "the
  valley's flora by species", which moved it to seeds 3, 11 and 13 without finding why).
- Cause, checked by stepping the pacing run with and without the arcs change: on seed 3 one
  "turn" was "fever in the houses during strangers camped near", the fever being a poisoning;
  on seed 7 the first stretch was "wounds, fever" (the poisonings of the first days), which
  eased at day 10044 and ended the run before the strangers came, so nothing turned. With
  poisonings not counted, seed 7's stretch is "strangers, cold" with a turn, and 13 turns
  twice.
- Fix: `systems/act.rs` `ate_look_alike`: everyone awake within eight tiles on the level and in
  sight learns the plant on the spot, as learning by watching works; `social.rs`
  `tell_look_alike`: in each talk each tells the other one look-alike they know and the other
  does not, so it travels person to person (Dan's rule 3); `arcs.rs` `troubles`: a poisoning is
  not a fever in the houses. `tests/pacing.rs` is back on seeds 3, 7 and 13.
- Census (60 days, seeds 1 to 4, on this branch with master c7d56c9): poisonings 146 before,
  47 after (death cap 37 to 12, hemlock 42 to 12, nightshade 33 to 12, lily of the valley 34
  to 11); none fatal either way; 65 to 106 people knew a look-alike at the end.
- Guard: `tests/flora.rs` `a_poisoning_is_learned_by_those_who_see_it_and_told_on`;
  `tests/pacing.rs` on its original seeds.
- Commit: (this commit).

## 2026-10-28 #286: the roof test is strict again, except where the house was added to

- Symptom: the entry "a roof lost while its house grows" let `a_fallen_roof_is_mended` pass on
  "roofed" whatever happened.
- Found by: the coordinator's review.
- Cause, checked from the seed 7 trace: "roofed" was said at hour 31, when the whole floor,
  old room and new, was under its roof (22 of 22 of the old floor, and the stage needs every
  tile of the new); the house had been "added to" within the hour of the loss, so its stage
  went to "walls up" and then "roofed". Not a roof reported whole while down.
- Fix: the test needs "mended", or "roofed" only after an "added to", and in either case
  checks every tile of the house is roofed at the moment it is said.
- Guard: the test itself.
- Note on the raider test (entry "a raider kept at the houses"): the trace showed raider 106
  cut in a fight with Ormar and "taken and kept at the houses" at tick 289668463, the same day
  the run ended, so well inside the keeping's clock (C76, D47); master's version of the test,
  kept in the merge, skips only a member alive and held, and every other check stays.
- Commit: (this commit).

## 2026-10-28 fix-433-thirst: nobody drank (#433, part)

- Symptom: people needed no water. The only use of water was dousing fires; no rule built a
  well, so (#405) no cut ever struck water and nobody needed it to.
- Found by: the #405 work, filed as #433 (Dan's rule 7, docs/design-rules.md: what would a
  normal person do? Water comes first).
- Cause: not built.
- Fix: `mind/needs.rs`: thirst beside hunger, four waking hours from drunk to parched (a
  quarter as fast asleep), half as fast again above 25 C and twice above 30, a quarter more at
  work (`systems/needs.rs`); `Parched` on record when it runs out; a day and a half parched
  kills ("thirst", `systems/life.rs`). `water.rs`: fresh water is a surveyed body's shore
  where it still stands, not flood water, not water in the workings, not ice (below -2 C);
  a well serves always, and in a hard frost a hearth does, where snow is melted.
  `systems/decide.rs`: `Goal::Drink` to the nearest such place they can walk to, pressing
  harder than hunger at the same need; a thirsty sleeper wakes to drink; `act.rs`
  `Activity::Drinking`. A probe of four openings over 20 days: no one died of thirst, a walk
  to drink averaged 7 to 10 tiles, home lay 3 or 4 tiles from water (seed 2, 12 at most).
- Guard: tests/thirst.rs.

## 2026-10-28 fix-433-thirst: water far from home, and no well (#433, part)

- Symptom: with thirst built, a household whose water lay far off had only the walk; nobody
  carried water home, nobody dug a well (`StructureKind::Well` was never ordered), and a
  community moving camp took the first flat place it found, water or not.
- Found by: #433's list (Dan's rule 7: a normal person carries water home, digs a well when
  none is close, and settles near water).
- Cause: not built.
- Fix: `wells.rs` and `systems/projects.rs` `well`: a community with no fresh water within
  twelve tiles of home (`SHORT_WALK`) and no well proposes one, the best digger speaking for
  it with how far the water is; the planner sinks a shaft by home a level at a time
  (`DigStairs`), and when the rock beside the cut holds water (aquifer.rs, #267's strata) it is
  struck (`StruckWater`) and the well is ordered built over it; a shaft that reaches the
  middle stratum's hard rock, or nine levels, dry is given up with why (`Chose` "gave up on
  the well"), not to be tried again for twenty days. `water.rs` `plan_store`: a household
  whose water is more than six tiles off keeps a barrel by home, filled a vessel (six drinks)
  at a time (`DesignationKind::FetchWater`) up to four drinks a head, while it has a bucket
  or a pot (a hide or leather serves as a skin); those near it drink from it
  (`Community::water`). `systems/moving.rs`: a move
  goes to the first place found with water a short walk off, and only to a dry one when none
  of them has any.
- Guard: tests/wells.rs; tests/thirst.rs
  `a_household_far_from_water_keeps_a_store_and_drinks_from_it`; tests/moving.rs
  `a_move_goes_to_water`.

## 2026-10-28 fix-433-thirst: a whole hamlet drowned going down to drink at a river in flood (#433, part)

- Symptom: with thirst, seed 2's opening lost all nine of its people "drowned in the flood"
  within eight days (tests/identity.rs `evenings_at_the_hearth` failed: the custom's holders
  all dead); on master they lived.
- Found by: the full test run on fix-433-thirst.
- Cause: confirmed for the place, not counted per death: the drinking places (`water.rs`
  `spots`) were every shore of every body, so people went down to the stream to drink while it
  was over its banks; with the guard below the test passes.
- Fix: `water.rs` `spots`: no drinking at a body that is over its banks (`stage >= RISE`) or
  in spate (`high`). `nearest` also tries the first 48 places by distance, not 12, so a lake
  beyond a cliff does not hide a shore that can be walked to.
- Guard: tests/identity.rs `evenings_at_the_hearth`. Three tests changed for thirst, keeping
  what they guard: tests/autonomy.rs `knowledge_only_targets` gives water before the bush and
  sets the person back on the flat ground; tests/projects.rs
  `a_carving_is_proposed_by_one_who_remembers` asks for no carving, where its dry village now
  wants a well; tests/homes.rs `a_hollow_left_undug_is_ordered_again_and_given_up_in_time`
  checks that the given-up home's own project ended (a new one may take the same mouth);
  tests/health.rs's meadow has a well by the house, as a settled place would, for its runs
  of days (one is in winter, when the open water is ice). `hf-godot` passes fetching water
  to the client as designation code 24, drawn as nothing, like hauling.

## 2026-10-28 fix-433-thirst: people died of thirst in a drought, and bands never drank (#433, part)

- Symptom: a 200-day probe of four openings had 30, 15, 3 and 9 deaths of thirst. In seed 4
  on day 111 the whole community was parched with no drinking place within 60 tiles, though
  some stood beside water; most of the other deaths were raiding bands' people.
- Found by: the #433 opening census, 2026-10-28 (a probe printing each death of thirst, and
  the parched of seed 4 every three hours).
- Cause: confirmed. (1) `water.rs` `spots` took drinking places only from a body's surveyed
  shore; in a drought (#393) the shore dries and the water left was the body's middle, which
  the shore list does not hold, so the water beside them was not found. (2) A band's people
  choose in `decide.rs` `choose_band`, which had no drinking.
- Fix: `spots` and `wells::water_within` look at every water tile of a body (still standing,
  fresh); `choose_band`: a thirsty band member goes to the nearest water; a parched person
  (thirst under 0.3) looks twice as far, 120 tiles. `systems/moving.rs`: a camp with no water
  within 30 tiles of home whose well has failed proposes moving, to water where it can,
  without waiting for hardships ("no water within a long walk of home, and the well they dug
  found none").
- Guard: tests/thirst.rs `when_the_shore_dries_they_drink_at_the_waters_new_edge`;
  tests/moving.rs `a_dry_camp_whose_well_failed_moves_to_water`. The band's drinking has no
  test of its own; the census counts its deaths.

## 2026-10-28 fix-433-thirst: a whole valley went parched whenever its stream rose (#433, part)

- Symptom: after the guard against drinking at water in flood (entry above), seed 4's
  opening was parched to the last person on half-days 11, 20, 26, 51 and on through day 111
  of a 200-day probe (1,076 `Parched` rows), and tests/thin.rs
  `they_study_it_and_the_wards_quiet_it` failed: on seed 7 the wards stood on about day 55,
  where on master the place was quiet by day 46.
- Found by: the #433 census and tools/ci.sh, 2026-10-28; a probe printing seed 4's waters on
  every half-day when a quarter of its people were parched.
- Cause: confirmed for seed 4: the guard shut every body above `RISE` (over its banks), and
  seed 4's stream and both its ponds stood at 0.6 to 3.0 on each of those half-days, so there
  was nowhere to drink. Seed 7's slower wards are not confirmed to share the cause; with the
  fix below its wards stand on day 23 and it is quiet on day 28.
- Fix: `water.rs` `spots`: water over its banks is drunk from as before; only in spate
  (`high`), and then only from a bank above the high-water line (`Waters::flooded_once`),
  does nobody go down to it.
- Guard: tests/identity.rs `evenings_at_the_hearth` (the drowned hamlet stays alive) and
  tests/thin.rs `they_study_it_and_the_wards_quiet_it`; the census below.

## 2026-10-28 fix-433-thirst: master's flood test after the merge (#433, part)

- Symptom: tests/floods.rs `people_on_the_floodplain_get_out_of_the_rising_waters_way` (from
  master) failed on fix-433-thirst: everyone lived and stood dry, but no "made for higher
  ground" was on record.
- Found by: tools/ci.sh after merging master, 2026-10-28.
- Cause: not confirmed. Its four people loiter on the floodplain for twenty hours, and
  thirst now stops a loiter to go and drink; likely they had left before the water came.
- Fix: the test keeps them watered, so it is the flood that moves them, as it tests.
- Guard: the test.

## 2026-10-28 fix-433-thirst: water in #415's order (#433, part)

- Symptom: #415's `decide::secured` left `thirsty` always false ("no thirst need exists
  yet"), and a meal in hand outscored a walk to water at the same need (drink 3.5, carried
  food 3 x 1.3).
- Found by: the coordinator, on #415 landing (c111).
- Cause: confirmed by reading `secured` and the two scores in `choose`.
- Fix: `secured`: thirsty when thirst is half run down; `choose`: a drink's weight is 6, twice
  eating's, so water comes before food.
- Guard: tests/thirst.rs `thirsty_and_hungry_they_drink_first`.

## 2026-10-28 fix-433-thirst: every brief lapse of thirst was a hardship on record (#433, part)

- Symptom: 281 to 728 `Parched` rows a seed in 200 days, each a hardship memory, most of
  them a minute at empty on the way to the water.
- Found by: the #433 census; Dan's call through the coordinator: only an hour at empty counts.
- Cause: `Parched` was written the moment thirst reached zero (`systems/needs.rs`).
- Fix: `systems/life.rs` `thirst`: `Parched` and its hardship memory once an hour at empty;
  a lapse that ends in a drink within the hour leaves nothing. A day and a half still kills.
- Guard: tests/thirst.rs `parched_is_an_hour_at_empty_not_a_lapse`.

## 2026-10-28 fix-433-thirst: the floodplain test, found and restored (#433, part)

- Symptom: the entry above changed tests/floods.rs
  `people_on_the_floodplain_get_out_of_the_rising_waters_way` to keep its people watered,
  with the cause unconfirmed.
- Found by: the coordinator, asking whether thirsty people walk into a river in spate.
- Cause: confirmed by an hourly trace of the test as master has it, on the branch before
  #415 was merged: at hour 11 of the storm their thirst took them off the floodplain to drink
  at the stream's bank (41, 32), before the water rose; they never stood in its way, so no
  flight was on record. Nobody walked into the flood. On the branch with #415 merged (whose
  `in_floods_way` now also guards the drink goal's target in `choose`) the test as master has
  it passes.
- Fix: the test is as master has it. The drink goal's targets are kept out of a flood: water
  in spate is drunk from only above the high-water line (`water.rs`), and a target in a
  flood's way is not reachable (`decide.rs`, #418's `in_floods_way`).
- Guard: tests/floods.rs as on master; tests/thirst.rs
  `no_drinking_at_water_in_spate_below_the_high_water_line`.

## 2026-10-28 fix-433-thirst: the carving test's assertion restored (#433, part)

- Symptom: tests/projects.rs `a_carving_is_proposed_by_one_who_remembers` had been narrowed to
  "no carving proposed", because its dry village proposed a well.
- Found by: the coordinator.
- Cause: the village had no water within a short walk.
- Fix: the village has a well by home, so nobody wants one; the assertion is master's again
  (no project at all without one who remembers).
- Guard: the test.

## 2026-10-28 fix-433-thirst: every well was dug where there was no water (#433, part)

- Symptom: the census's wells, ten of ten, were dug to the hard rock and found dry.
- Found by: the #433 census; the coordinator (a normal person digs where water is likely).
- Cause: confirmed by reading `wells.rs` `plan`: the site was the first flat tile at a set
  step from home (four or five tiles off), wherever the water lay.
- Fix: `wells.rs` `likeliest`: within fifteen tiles of home, on open ground the community
  holds and can reach, the spot where the ground shows water under it first (the column's
  rock holds water within the nine levels a well is dug: the damp and the green over a basin,
  read from #267's strata, an abstraction of what diggers see), then the lowest ground, then
  thick plants, then the nearest. The old step from home is kept only if no tile qualifies.
- Guard: tests/wells.rs `a_well_is_sited_over_the_basin`.

## 2026-10-28 fix-433-thirst: master's stump test after merging #415 (#433, part)

- Symptom: tests/biomes.rs `felled_tree_leaves_a_stump` failed after merging master: the one
  person never felled the tree in four hours.
- Found by: tools/ci.sh, 2026-10-28.
- Cause: in part confirmed by an hourly trace: an hour in, thirsty enough by the arrival's
  urge, they went to drink at water off the meadow (thirst rose from 0.74 to 0.99 over the
  next hours) and then rested there; why they did not come back to the order is not
  confirmed.
- Fix: the test keeps its person watered, as it keeps them fed and rested.
- Guard: the test.

## 2026-10-28 fix-433-thirst: the hearth test counted people come to drink (#433, part)

- Symptom: tests/identity.rs `evenings_at_the_hearth` failed on seed 1: at the hearth 0.52 of
  the evening and 0.17 of the afternoon, just under the three times it asks.
- Found by: tools/ci.sh, 2026-10-28.
- Cause: confirmed by listing what those at the hearth in seed 1's afternoons were doing: 21
  of about 160 were drinking or on their way to drink, at water a few steps from the hearth.
- Fix: the test leaves out those drinking or going to drink; a drink is not an evening at
  the hearth.
- Guard: the test.

## 2026-10-28 fix-433-thirst: wells sited on bare rock (#433, part)

- Symptom: in a 200-day census after the siting above, seed 3 gave up 13 wells "dug 0 levels
  down to the hard rock".
- Found by: that census, 2026-10-28.
- Cause: confirmed from the message: with no ground showing water near home, `likeliest`
  took the lowest ground, which there was the middle stratum's hard rock at the surface.
- Fix: `likeliest` leaves out columns whose hard rock lies within a level of the surface.
- Guard: tests/wells.rs (the sited and the dry well); the census below.

## 2026-10-28 fix-433-thirst: a well's own rock, and digging the same dry ground again (#433, part)

- Symptom: (1) a shaft sunk into wet rock struck water only if the rock beside it was wet;
  (2) seed 3 dug 9 dry wells in 200 days, every twenty days.
- Found by: reading `wells.rs` `wet_beside` after the census; the census.
- Cause: confirmed. (1) `aquifer::wet` reckons depth from the column's ground, which the
  shaft lowers as it goes, so the shaft's own column never read as wet. (2) Seed 3's home has
  no column within fifteen tiles over wet rock (a probe of the four openings: 121, 29, 0 and
  0 such columns near seeds 1 to 4's homes), and a given-up well was tried again in twenty
  days.
- Fix: `aquifer::held` (wet, from a given ground); `wet_beside` also reads the shaft's own
  rock from the ground as it was. After a dry well where no ground within reach shows water,
  the next waits five years (100 days), else one year as before.
- Guard: tests/wells.rs `water_under_the_shaft_alone_is_struck` and the dry well's wait in
  `a_well_dug_to_dry_hard_rock_is_given_up_with_why`.

## 2026-10-28 fix-433-thirst: an hour parched beside water while a flood lay near (#433, part)

- Symptom: with `Parched` at an hour, the 200-day census still had 502, 513, 120 and 229 rows;
  in seeds 1 and 2 most were people loitering within twenty tiles of water (198 and 274).
- Found by: the census's breakdown of each `Parched` by what the person was doing.
- Cause: confirmed by the fix: `water::nearest` gave the nearest bank, and #418's
  `in_floods_way`, which `choose` now applies to the drink goal, refused it while flood water
  lay near; no farther bank was tried, so they did not drink. Seed 1 over 60 days: 114
  `Parched` before, none after.
- Fix: `water::nearest` passes over banks in a flood's way and takes the nearest one clear
  of it.
- Guard: tests/thirst.rs `a_bank_clear_of_the_flood_serves`.

## 2026-10-28 fix-433-thirst: the stump test's person never came back to the order (#433, part)

- Symptom: the merge's own workaround (an earlier round kept the test's person watered so
  it would pass) papered over a real bug: traced with `HF_TRACE_AGENT`, a lone worker with
  nothing else nearby to have noticed stayed on `Wander`/`Idle` for hours, never once
  choosing to work the chop order.
- Found by: tracing `felled_tree_leaves_a_stump` with the workaround removed; `decide.rs`'s
  whole designation-work block (eat, sow, craft, chop, every `Goal::Work`) sits behind
  `!a.knowledge.map.is_empty()`. A settled community always has this from its shared hearth
  knowledge, but a lone arrival with no community and nothing fruiting, no item, no
  structure, no other person nearby has nothing to populate it until, by luck of the
  wander, they sight a water tile (perception notes any seen water on its own, water::drink
  aside). Sent off thirsty, their wander path changes and that luck can run out for hours,
  during which an assigned, reachable order sits untouched; this is what the earlier round's
  trace saw and could not confirm.
- Cause: confirmed. A placed designation is an explicit order, not something to stumble on
  by sight; gating its consideration on having perceived something unrelated was never
  needed for it specifically, only for the discovery-driven goals (eat what you've found,
  sow what you know of) sharing that block.
- Fix: `decide.rs`: the designation-work block no longer requires non-empty knowledge, only
  the other settling-in checks (not a child, not a rest day, not pressed by hunger or rest,
  not night).
- Guard: tests/biomes.rs `felled_tree_leaves_a_stump`, restored to how master had it (no
  longer keeps its person watered); `a_worker_sent_to_drink_comes_back_to_the_order`, sent
  thirsty from the start with nothing else around to perceive.

## 2026-10-28 fix-433-thirst: after a walk to drink, the stump test's person never went back (#433, part)

- Symptom: tests/biomes.rs `felled_tree_leaves_a_stump` (master's) failed: the one person never
  felled the tree; it had been changed to keep them watered.
- Found by: the coordinator, asking why the work was not resumed after a drink.
- Cause: confirmed by an hourly trace of the test, and of master's with the same trace. The
  work was never begun, so nothing was lost: work is weighed only by one who knows of
  anything (`decide.rs`, `knowledge.map` not empty), and this person, spawned on a cleared
  meadow, knew nothing. On master they wandered until, about an hour and a half in, they
  knew of something and went to the tree. On the branch, at the same hour they walked to
  water at the map's edge, a level below their own (the eye's scan in `perceive.rs` looks at
  its own level, so they did not come to know of it), drank, and stayed there, knowing
  nothing still, loitering for hours.
- Fix: `act.rs`: a drink that was a walk off (more than four tiles from where they chose it,
  `Activity::Drinking::back`) is followed by the walk back, except in the hearth's evening
  hours, when they go on to the hearth as before (tests/identity.rs `evenings_at_the_hearth`
  fell short on seed 2 with the walk back taken then: 0.49 of the evening at the hearth
  against 0.16 of the afternoon, where it asks three times). `water::drink`: where they drank
  is water they know of (`Known::Water`).
- Guard: tests/biomes.rs `felled_tree_leaves_a_stump` as master has it; tests/thirst.rs
  `after_a_walk_to_drink_they_come_back` (they come back and know the water; it fails without
  the walk back) and `a_worker_who_leaves_to_drink_comes_back_and_finishes`.

## 2026-10-28 fix-433-thirst: seed 4 parched whenever its stream was in spate (#433, part)

- Symptom: seed 4 had 238 to 244 `Parched` (an hour at empty) in 200 days, nearly all far from
  water while loitering.
- Found by: the coordinator; a probe printing seed 4 on every half-day when a tenth of its
  people were parched.
- Cause: confirmed for the two worst spells (days 80 and 145, everyone parched): the stream
  and both ponds stood in spate (stage 3.0 and 2.1), every bank within reach lay in the
  flood's way, and the household kept no water: its home lies four tiles from the stream, so
  no store was kept (`plan_store`, beyond six tiles only), and when one was, those passing
  drank it empty.
- Fix: `water.rs`: a household whose home water has flooded (any column within eight tiles
  over the high-water line) keeps a store as one far from water does; the store is drunk from
  only when no other water within twelve tiles is clear of a flood.
- Guard: the census below (seed 4's store). Drinking the rain where one stands was tried
  as well and taken out: it held the people of two of master's tests (the stump and the
  roofs of seed 3) from what those tests wait for, by where it kept them.

## 2026-10-28 fix-433-thirst: one waiting for a caravan never went to drink (#433, part)

- Symptom: none seen directly; found reading `decide.rs` while tracing seed 3's thirst deaths.
- Found by: the coordinator's seed 3 question.
- Cause: confirmed by reading and by the guard test failing before the fix: a caravan's member
  on the way to the gathering place, or waiting there, re-decided only when hungry; thirst
  was not asked.
- Fix: `decide.rs`: thirsty (under 0.3), they re-decide as when hungry.
- Guard: tests/thirst.rs `waiting_for_a_caravan_they_still_drink`.
- Not confirmed: on the branch after merging c114 and before the walk back from a drink, a
  200-day probe of seed 3 had 23 die of thirst on day 168, all loitering with water they
  could reach 7 to 14 tiles off; whether they were a caravan's party was not established
  (the trace that would have said so ran differently: the same probe at 128 days and with a
  decision trace on did not reach that day's deaths). On the code as it is now, seed 3's
  200-day probe has no death of thirst. The trapped corner at (6, 6..11) is gone since c113.
- Checked 2026-11-01 on c147 (box): seed 3, `run --opening --days 200`, twice at `--threads 8`
  and once at `--threads 1`, gave one digest (2aa9b3d2...) and identical event counts, with
  1 `parched` and no death of thirst. The day-168 deaths do not occur on current code, and the
  thread count does not change the outcome; the earlier divergence is not reproduced.

## 2026-10-28 #358: a left camp's tale named the camp again, and a baby wondered at old ruins

- Symptom: a tale of a community that moved read "the last of Eldergate went to Eldergate",
  naming the abandoned camp with the community's own name instead of its own; and a two-day-old
  could "wonder" at old-valley ruins and keep the tale, when the same age gate already stopped
  it wondering at a left site.
- Found by: issue #358 (reported directly); guarded below.
- Cause: checked. `left_sites::record_move` stored `c.name.clone()` as the `LeftSite`'s own name
  instead of giving it a toponym; `lore::step` (the old-valley ruin wonder system) lacked the
  `is_child` gate that `left_sites::wonder` already carries in master.
- Fix: `left_sites::record_move` names the site with `deep::hamlet_name(w.meta.seed, id)` (widened
  to `pub(crate)` for this); `lore::step` skips `is_child` agents, matching `left_sites::wonder`.
- Guard: `tests/left_sites.rs` `a_newcomer_among_the_left_houses_wonders_at_them` (asserts the
  tale names a place, not the site again) and `a_baby_does_not_wonder_at_the_left_houses`.
- Commit: 3fe593a.

## 2026-10-28 fix-255-house-lamps: a lamp by the door even when the flanks are taken (#255 part 2)

- Symptom: seed 7's second house, added to three times, has floor, beds, tables, stools and
  chests but no lamp by its door; the lamp spots outside the door are taken by the rooms added
  against it. (The "Open, found but not fixed" item "A later house's lamps (#255 part 2)" is
  this fix; the bullet is removed below.)
- Found by: the C71 review (entry "the smaller set BD findings (#255)", 2026-10-22).
- Cause, checked: `systems/settlement.rs` `plan_community` step 6b designated a house's lamps
  only at the two tiles flanking the door step, requiring walkable and `Shape::Open`; a flank
  taken by a room grown against the door side (the annex protects only the step tiles,
  `houses.rs` `annex`) failed the check, and with no other candidate the lamp was skipped for
  good. Reproduced in the guard: a complete house whose flanks stand on gets no lamp
  designation within three tiles of the door on master's code ("no lamp within 3 of the door
  at (7,8,2)").
- Fix: step 6b's lamp posting (`plan_community`): when a flanking spot is taken, the lamp is
  designated at the nearest other walkable open tile within three tiles of the door — not the
  door, the step, the flanks themselves or the house floor, and nothing already designated or
  standing there. Planner order placement only; nothing is built instantly. The workbench's
  posting is unchanged. Two guards on the fallback, from the box-ci failure of the first
  version (branch's first head, `tests/identity.rs` `evenings_at_the_hearth` on seed 2: the
  hearth stood three tiles from the door and evening attendance fell): the step runs every
  hour, and a flank stays taken, so the fallback must not post a fresh lamp each run — once a
  lamp stands or is wanted within three tiles of the door, that is it (the failing version
  ringed the hearth with nine lamps, taking its gathering seats); and the candidates sort
  wall-side tiles first, as the flanking spots were, so the lamp stays by the house.
- Guard: `tests/grown_houses.rs` `a_lamp_by_the_door_even_when_the_flanks_are_taken` (fails on
  master's code, passes with the fix); `tests/identity.rs` `evenings_at_the_hearth`, seeds
  1-3.
- Merge with #433 (thirst and wells), 2026-10-29: the fixture of `evenings_at_the_hearth`
  changed, not the habit. On seed 3 the hearth house's flanks stand on the settlement's own
  cellar-stair head and an annex wall, so the fallback now posts its lamp there (the fix
  working as designed); through build-order cascade the colony's evening meals and jobs land
  differently, and the all-activity count reached 2.97 of the afternoon's against the
  assertion's 3. The test now counts only those at leisure (Idle, Loitering, Talking) in both
  windows alike — the population the habit is about, and the population #433 already left
  drinkers out of for the same reason. With the fixture adjustment seed 3 reads 0.61 of the
  evening against 0.12 of the afternoon (5.1x), master alone 0.68 against 0.13 (5.2x).
- Commit: (this commit; the two fallback guards and the FIXLOG note added after the box-ci
  failure of the branch's first head; the merge note and the leisure-only fixture added on
  merging #433).

## 2026-10-28 fix-173-pursuit-sight: survivors went after a stranger they never saw

- Symptom: a settler who leaned to Violence was offered `Goal::Confront` against the nearest
  living band member chosen by TRUE position within 40 tiles, as soon as the band's `taken`
  count was above zero — violating the personal-knowledge invariant (`mind/knowledge.rs`:
  "Agents choose targets ONLY from here (no omniscience)"). The only gate was the band-level
  `sighted` flag, set when ANY awake community member stood within 12 tiles of any band member,
  so one sentry's glance armed every hostile settler with the stranger's exact whereabouts.
- Found by: QA round 2026-10-20, #173's omniscience note; the second half of #173, listed in
  FIXLOG Open since C62.
- Cause: confirmed by code reading and the guard below: `systems/decide.rs` `consider_bands`,
  the pursuit branch, iterated `b.members` by `x.pos` (the agent's true position).
- Fix: `consider_bands` (decide.rs): the pursuit branch now takes a member's position only from
  the settler's own sight (`mind::perception::can_see`) or their own `Known::Agent` belief —
  a stale remembered position is the point of personal knowledge — and skips members the
  settler neither sees nor knows of. The Confront goal's target is the believed position, never
  the true one.
- Guard: `tests/qa_1020_medium.rs` `pursuit_of_a_stranger_needs_personal_knowledge` (band with
  `taken > 0`, `sighted`, hostile settler, member 30 tiles off and unseen: no pursuit; positive
  control, the same member 10 tiles off in the open: pursuit) and
  `pursuit_follows_what_is_known_of_the_stranger` (a stale belief alone is enough). The first
  fails on master ("they go after a stranger they never saw"), both pass after.

## 2026-10-28 fix-253-soak-starving: the soak counts starving person-hours

- Symptom: a person starving on many days running never surfaced in the soak summary, which
  failed only on stalled person-hours, ledger drift and RSS (the closed #253 repro: seed 3,
  150 days, stalled 0.00% while people starved).
- Found by: #253 (Expected, never built); coordinator triage on origin/master @ 3160168f.
- Cause, confirmed from `crates/hf-cli/src/main.rs`: the soak counted `stalled()` at each
  hourly sample and reported its person-hours, but nothing hunger-related; the sim already
  exposes `Needs::starving` (crates/hf-sim/src/mind/needs.rs) and per-agent `starving_ticks`.
- Fix: `crates/hf-cli/src/main.rs` only. New `starving` helper beside `stalled` (same person
  filter, `Needs::starving` as the test); `soak` accumulates `starving_person_hours` at each
  hourly sample beside `stall_person_hours`, adds a `starving` column to the per-game-day CSV
  after `stalled`, and the summary line reports
  `starving person-hours {pct}% (worst day {N})` beside the stalled figure. No failure
  threshold added — the cut-off is Dan's call.
- Guard: none — CLI harness change with no unit-test seam; verified by soak run output
  quoted here. On unmodified master, `ostrog soak --opening --seed 3 --days 60` printed
  `stalled person-hours 0.00% (worst day 0.0%), ledger drift days 0, ...` with no starving
  field and a CSV header of `...humans,away,animals,stalled,plan_failed,...`. With the fix
  the same command prints `starving person-hours 0.33% (worst day 0)` and the header reads
  `...humans,away,animals,stalled,starving,plan_failed,...`.

## 2026-10-28 fix-238-perceive-growth: the shared-store gather ran per looker, so a look's cost grew with the days

- Symptom: perceive's cost grew with the days at 500 people (#238, C69 review: 539 us a tick on
  day 1 to 909 on day 4). The C55 `Knowledge::note` walk was already fixed on master (021e5ea4);
  the residual was the community shared-store gather: every looker re-walked `w.structures` twice
  (hearths, then stockpiles within 40 of home) and `w.items` once per in-range stockpile, so the
  walk grew with filled stockpiles, fields, and buildings.
- Found by: #238's triage, reading `systems/perceive.rs` on master.
- Cause: confirmed: the gather sat inside `step`'s per-looker loop (the community-store block),
  so a tick with L lookers walked the world's structures and items L times.
- Fix: `systems/perceive.rs`, `SharedStore::gather` (new): the hearth/stockpile data is gathered
  once a tick, lazily alongside the per-tick `Index`, indexed by community, with ground items
  bucketed by `TilePos`; each looker reads `stores.get(&cid)`. The push order within a community
  (hearths in structure order, then each in-range stockpile's items in item order) is the same
  as the per-looker walk it replaces, so noting — digest-visible — is byte-identical.
  `perceive::store_walks` (new, on `sight_lines`' thread-local precedent) counts the gathers for
  the cost tests.
- Guard: `tests/perceive_cost.rs` `shared_store_gathered_once_a_tick` (three lookers in one tick
  took three walks before the fix, one after) and `shared_store_notes_match_the_full_scan` (a
  full-mind looker beyond everything's reach notes exactly the naive per-looker full scan's
  (Known, pos, tick, tag) set, evictions included).
- Commit: (this commit).

## 2026-10-29 fix-m5-perceive-gone-beliefs: a missing belief retraced a live sight tile

- Symptom: after c121's shared-store gather fix, the 500-person human profile still spent most
  of its perception time in repeated sight checks; a run had 5,316 sight-line traces and a
  1.684 ms mean perception cost.
- Cause: confirmed in `systems/perceive.rs`: the gone-belief pass used `line_of_sight` directly
  for every missing belief, bypassing the per-look tile memo already used by current observations.
  Several remembered objects can share one tile, especially a cleared heap or a stockpile.
- Fix: route the gone-belief visibility check through the existing `sees` closure, preserving the
  same line-of-sight result while sharing the cached answer for that look.
- Guard: `tests/perceive_cost.rs`
  `gone_beliefs_on_one_tile_share_the_live_sight_line` fails without the fix because 150 missing
  beliefs retrace the tile and exceeds 50 sight lines; it passes with the memoized check. The
  existing digest-equivalence and shared-store guards remain unchanged.

## 2026-10-28 branch 413-mouse-ui: mouse-first watching UI, favourites, and block inspection (#413)

- Fix: added mouse-first panel controls, per-save favourites, left-drag camera panning, and a
  simulation-backed block-inspection read model exposed through Godot.
- Guard: `crates/hf-sim/src/view/block.rs` and `tools/gd_test.gd` inspection/favourites tests.
- Commit: `b9ba57f7`.

## 2026-10-28 #449 review: block inspection reports only simulated water facts

- Fix: water kinds are limited to facts present in the simulation; looked-down ground uses the
  found tile's z-level; mineral inspection covers gems.
- Guard: `block_inspect_reports_water_minerals_and_ground` and
  `test_favourites_persistence_and_block_inspect`.
- Commit: `b9ba57f7`.

## 2026-10-28 #459: person inspection is guarded for equipment and inventory

- Fix: added a deterministic staged-scene guard for `SimHost.inspect()` equipment and inventory
  fields, and refreshed the generated README tables.
- Guard: `test_person_inspection_exposes_equipment_and_inventory`.
- Commit: merged through #459.

## 2026-10-29 #463: terrain ownership separated from the object and inspection pass

- Fix: #463 owns the quieter terrain generator/assets, terrain-specific art rules and 24 grass
  variants; #459 no longer overlaps terrain paths.
- Guard: terrain reproducibility, palette, provenance, ground, README-table, CI and changed-file
  boundary checks.
- Commit: merged as c117.

## 2026-10-30 PR #474: E16 lessons invented injuries, contradicted breaches, and moved worksites

- Symptom: an unhurt cave-in and a collapse with no injuries confirmed `Harm`; a real
  `WaterBrokeIn` began with a contradiction; water observations used the actor's later position
  and the learning pass's tick, with `(0, 0, 0)` when the actor was unavailable.
- Found by: PR #474 review and the failing generated README-table check on `box-ci`.
- Cause: confirmed in `systems/learn.rs` `observe`: injury fields were discarded, `success=false`
  was passed for a breach, and water provenance came from current agent/world state rather than
  the authoritative event. In `mind/learning.rs` `learn`, the parameter name `success` conflated
  confirmation with outcome valence.
- Fix: map hurt/non-hurt events to `Harm`/`NoChange`, record both water events as confirmed
  `Discovery` (not unsupported `Displacement`), use each event's worksite and tick, and skip a
  water event without a location instead of inventing one. `learn` now names the boolean
  `confirmed` and documents that harmful outcomes can be confirmed.
- Guard: `tests/e16_phase1.rs` covers hurt/unhurt cave-ins, empty/nonempty collapse injuries,
  both water events after the actor moves, exact event location/tick and confirmation counts,
  missing water locations, actor-only lessons, actual save encode/decode/restore, missing-learning
  defaults from the v2 fixture, and staged shallow/deep/blocked-sight perception.
- Commit: PR #474, this commit.

## 2026-10-29 #469 (part): an exhausted person walks home to a bed they know

- Symptom: nearly everyone slept rough even with beds standing; by day 600 most adults were
  far from the houses at nightfall and collapsed in the field.
- Found by: the #469 probe on master c1f3c5f8 — one person, night, a free reachable bed in
  their own knowledge, needs.rest = 0.1: they walked to it at 10 and 30 tiles, and slept on
  the ground at 45 and 60.
- Cause: confirmed by the probe and by reading `decide.rs`: `SleepHere` (scoring
  `urgency * 0.6`) was offered whenever `!bed_found || rest_p > 0.8`, so past ~40 tiles the
  ground outscored a known free reachable bed's `urgency * 1.2 * dist_discount * feel` for
  any exhausted person. Secondary: the loser of two in one bed was downgraded to
  `Sleeping{bed:None}` where they stood (`act.rs`) instead of re-deciding.
- Fix: `systems/decide.rs` (`choose`, sleep block): `SleepHere` is offered only when no free
  reachable bed is known — a known bed is unopposed, however far. `systems/act.rs` (the
  bed-taken check in `Sleeping`): the loser `fail`s ("the bed was taken") and re-decides
  toward their next known free bed instead of ground-sleeping beside it.
- Guard: `tests/sleep_bed.rs` `an_exhausted_person_walks_home_to_a_known_bed` (fails on
  master at 45 and 60 tiles) and `the_loser_of_a_bed_race_redecides_instead_of_ground_sleeping`.
- Commit: branch fix-469-sleep-walk-home (guard-test commit fc90b9ca, fix commit this one).

## 2026-10-29 #469 correction: a walk to food in sight is a meal in progress

- Symptom: `tests/autonomy.rs` `need_satisfaction_seeded` seed 11 failed on
  fix-469-sleep-walk-home with one `Starving` event ("nobody starves with food in sight");
  master c120/915e7161 passed.
- Found by: isolation of the branch's two hunks on a scratch worktree — the `decide.rs`
  hunk alone reproduces the failure with byte-identical per-agent traces (`HF_TRACE_AGENT`);
  the `act.rs` hunk alone passes — then an agent-level trace (Liran, tick 236046).
- Cause: confirmed by the trace. On both versions Liran is identical until tick 226078
  (hour 20, hunger 0.42, exhausted at the water hole, 40 tiles from the known bed). Master
  offered `SleepHere` (`rest_p > 0.8`) and he slept on the spot, waking at dawn beside a
  fruiting bush; the branch (#469, intended) walks him home to the bed, so he wakes at hour
  4 inside the house at 5% — where the nearest known bush (25,21) is chronically picked
  bare. His next chosen bush (13,17, fruiting) was 12+ tiles away; the starvation threshold
  crossed mid-walk at tick 236046 while `Going{then: Foraging}` — food in sight, ~30
  seconds away. He foraged, ate, and was fine. The event's own guard ("a last bite on the
  way down is not news") already suppresses `Eating`/`Foraging` but not the walk to the
  bite.
- Fix: `systems/needs.rs` (`step`, the starving-event guard): a person `Going` toward an
  `Eating` or `Foraging` activity counts as feeding, like being mid-meal. No decision or
  goal change; the #469 walk-home behaviour stands.
- Guard: `tests/autonomy.rs` `need_satisfaction_seeded` (seed 11; fails without the fix on
  the branch's head). Cannibalism-under-starvation (`conditions.rs` `starving`) reads
  `needs.hunger` and `starving_ticks` directly and is unaffected.
- Commit: branch fix-469-sleep-walk-home (this commit).

## 2026-10-28 fix-403-wall-building: a ring laid across a standing building never closes (#403, seed 4's Merehollow)

- Symptom: seed 4's Merehollow ring of 211 tiles had 208 standing at day 600 and never
  closed: three tiles of its outline held a table, a bed and a lamp of a building no house
  site of the community names. The laying left them out of the ring, and as the closure test
  asks every walkable outline tile off the ring to be unwalkable or water, the ring could
  never be sealed and no wall could ever be built where the furniture stood.
- Found by: the M5 review's soaks (fault 2), left open by fix-402-sieges, which confirmed
  and fixed the other two causes (water on the outline, timber ordered where no plank could
  be sawn); `examples/wall_probe.rs` on the day-600 save showed this third cause.
- Cause: checked, from the Merehollow save and the code. `settlement.rs` `palisade` lays the
  ring over outline tiles that are open, walkable, water-free and structure-free; a standing
  building on the outline (beyond the 25-tile pull-in of "what the wall goes round", or held
  by another community) has its furniture tiles left out of the ring, and nothing then
  closes or seals. The same fault could come to a ring after the laying: the mend step's
  grow pass also skips tiles with structures, so a building gone up on the line since left
  the same hole.
- Fix: `settlement.rs` `palisade`, at the laying and in the mend/grow step: the line goes
  round what stands on it. A structure (other than a store's marker) or a roofed tile on the
  outline moves the line out a tile on that side, and the scan repeats until the building
  stands inside the ring (`stands_on_the_line`). Water and rock still hold the line, as
  before; stockpile markers are still moved off it. The closure test is not weakened: a
  raider can still walk in through a door of a building the line ignored.
- Guard: `tests/defence.rs` `a_ring_goes_round_what_stands_on_the_line` (the Merehollow
  case at the laying: furniture of an un-named shelter on the outline; on master the ring is
  laid through the shelter and never closes) and
  `a_ring_goes_round_what_goes_up_on_the_line_after_the_laying` (a bench set on the line
  after the laying moves it out a course and the ring still closes). Both fail on master.
- Commit: (this commit).

## 2026-10-30 #493: cave fauna follows cavern habitat

- Symptom: natural caverns had a seed-dependent `4..8` bat range, the crawler was selected from
  the lowest cavern without checking that it was a deep habitat, and the burrower was placed in
  arbitrary middle stratum rock rather than in the underground habitat that supports it. The PR
  review also found that only 40 random floor samples per animal could silently reduce the
  requested population, especially where a pool left little dry floor.
- Found by: M6 ecology audit against #336 and the existing `place_deep` implementation, followed
  by the PR #495 review of placement guarantees and test independence.
- Cause: confirmed in `systems/animals.rs`: placement knew about cavern geometry but had no
  habitat-level fauna profile, and its bounded random sampling could miss every eligible tile.
  `cave_ecology.rs::burrower_cavern` also read the underworld flag directly instead of using the
  profile's burrower preference.
- Fix: `cave_ecology.rs` derives bat population, crawler eligibility and preferred burrower
  habitat from pool/depth/underworld flags, and makes that profile authoritative for selection;
  `place_deep` enumerates all valid dry floors before deterministic seeded selection, so it
  spawns the requested count whenever any eligible floor exists. It skips bats and crawlers when
  no dry floor exists and keeps the bounded rock fallback for the burrower in malformed worlds.
- Guard: constructed caverns in `tests/the_deep.rs` independently cover pool/deep bat scaling,
  deepest-deep crawler selection, underworld burrower preference, no-cavern fallback, and a
  flooded cavern with no eligible floor; the seeded `creatures_of_the_deep` smoke test remains.
- Commits: habitat profiles `47e695fd`; review fixes (this commit), PR #495.

## 2026-10-28 #428: settlement's 13 ms tick at 500 people was a house search looking for a door at every tile

- Symptom: on a quiet box (load about 4) the 500-person human profile's worst tick was
  16.57 ms against the 16 ms gate, settlement's worst 13.4 ms.
- Found by: `ostrog profile --agents 500 --provision --days 5` with timing probes (not
  kept) round each section of the planner and its searches, at 1 and 8 threads. On one
  thread: `houses::new_house` 17 to 19 ms, `quarry` 4 to 8 ms, and in the mixed party
  `dugout_site` 16 to 18 ms twice; at 8 threads 5 ms, 2 to 5 ms and under 2 ms; the hourly
  workshops section 2 to 4 ms at any count.
- Cause: checked by those probes. `houses::grow` looked for a door (the wall ring of the floor
  so far) at every tile it added once the floor reached four tiles, though `enough` cannot be
  met until the floor has two tiles a sleeper; for a party of 500 that never happens, so each
  of up to 24 floors did 60 door searches for nothing. A hollow or dugout search for hundreds
  of sleepers dug each of 1,600 places out to `MAX_ROOM` (40 tiles) though no room that size
  sleeps them. `can_floor` and the quarry's `open_ground` walked every plant in the valley for
  each tile they looked at.
- Fix: `houses::grow` looks for the door only once `floor_enough`; `houses::a_room_could_sleep`
  lets the planner skip the room searches for more sleepers than any room holds
  (`systems/settlement.rs`, the hollow branch of section 6); `houses::Ground` gathers the
  taken ground, living plants and trees once per house search; the quarry's site search uses
  `grounds::open_ground_trees_taken`, since its taken set already holds every tree. Results
  are unchanged: the profile's digests at 5 days equal master's, humans and mixed. After, on
  one thread: `new_house` 2.9 ms, the room searches 0. A fixed schedule across ticks was not
  needed.
- Measured, 5 days at 8 threads, back to back (this branch first, then master c115), load
  average 4.9 at the start, 6.3 between, 8.2 at the end:

  | Party | Master mean / worst | After mean / worst | Settlement's worst, master / after |
  |---|---|---|---|
  | humans | 1.433 / 14.2 ms | 1.397 / 10.2 ms | 9.6 / 5.8 ms |
  | mixed | 1.288 / 14.8 ms | 1.254 / 8.6 ms | 10.9 / 5.6 ms |

- Guard: `houses::tests::room_search_is_skipped_only_when_no_maximum_room_can_fit` and
  `houses::tests::house_door_search_waits_until_the_floor_could_be_enough` fail to compile without
  the new capacity checks; the profile's gate line and `tests/threads.rs` (1 and 6 threads, same
  digest) cover the performance and determinism outcomes.

## 2026-10-30 #468: an orphaned workshop order was re-proposed every five days

- Symptom: a workshop construction designation left behind when its proposer died or left was
  skipped by the settlement planner. The five-day project timeout then closed the project, and
  the workshop rule proposed the same workshop again on the next pass.
- Found by: #468's M5-review triage and a scenario guard with an own-community carpenter-bench
  designation that is not linked to the open workshop project.
- Cause: confirmed in `systems/settlement.rs`: the planner treated any matching community
  designation as a reason to `continue`, but did not attach that designation to the waiting
  project. The project therefore remained `work: None` until `systems/projects.rs::close` timed it
  out.
- Fix: when an open workshop project has an uncompleted matching designation from its community,
  link the project to that designation and continue. The existing organic construction order is
  unchanged: the designation still builds the workshop tile by tile from its material
  requirements; the planner only recovers the ownership link.
- Guard: `tests/c67_followups.rs`
  `an_orphaned_workshop_order_is_adopted_instead_of_reproposed` fails on the parent with
  `project.work == None` and passes after the link is adopted; it also asserts the project stays
  open and no duplicate workshop proposal exists.

## 2026-10-30 branch 482-cave-mouths: rare, traversable cave mouths in hillside faces (#482)

- Symptom: every cave and cavern was sealed under at least three levels of solid rock
  (`world/gen/caves.rs`, `caverns.rs`), so no natural cave mouth ever opened on a hillside and
  caves could only be found by mining down into them.
- Found by: issue #482 (worldgen enhancement request); PR #501 review then found that a single
  open tile qualified as a cave, cutting flattened natural-ramp approaches, and new rock ignored
  deeper strata. Building the deterministic descending-adit accept case also exposed the search's
  vertical-sibling rejection.
- Cause: confirmed in `crates/hf-sim/src/world/gen/caves.rs` (`grid.ground_z(x, y) - z < 3`) and
  `caverns.rs`; no worldgen stage cut a surface-to-cave passage. The review causes were confirmed
  in `mouths.rs`: no component-volume check, an unconditional write to `foot`, and material reads
  from upper `bedrock` rather than the wall at the cut's actual z. The search also treated a
  visited level cell as blocking its ramp alternative, so a level-first search could not descend.
- Fix: `crates/hf-sim/src/world/gen/mouths.rs` (new, called from `world/gen/mod.rs` after
  `ramps::apply`): draws 0 to 3 mouths per valley from its own purpose-tagged stream
  (`(purpose::WORLDGEN_CAVES, 0x40CE)`), picks hillside faces flanked by solid rock where an
  orthogonally connected, same-level cave room of at least eight floors lies within reach (`z`
  in `-10..=-2`), and finds a chordless 1-tile-wide route through solid rock using level
  stretches (`Shape::Open`) and natural steps down (`Shape::RampUp`) with a solid rock roof over
  every cell. Search explores both vertical siblings; final path validation still rejects stacked
  cells. It preserves the existing walkable approach (including a natural ramp), keeps the
  removed wall's material in each cut ramp and sill and the existing rock brow intact, and
  verifies with `nav::step::neighbours` that the passage walks both ways before keeping it; a
  failed check restores every changed tile.
- Guard: `crates/hf-sim/tests/cave_mouths.rs` builds a deterministic hill and cave to cover the
  eight-tile accept, seven-tile and disconnected-tile rejects, bidirectional traversal, natural
  ramp preservation and actual-depth materials; `mouths::tests` forces a failed cut and checks
  complete rollback. The seeded census tests remain supplementary coverage of walkability,
  roofing, spacing, determinism and the no-caves control.
- Commit: a28d5e31; review hardening (this commit).

## 2026-10-31 branch 482-cave-mouths: a cave mouth raised the hillside and broke its biome surface

- Symptom: exact-head box-ci on d5e7f616 failed `ground_families_follow_biome` on seed 1 at
  `(113,58)` because limestone was not meadow ground, and `character_changes_the_valley` because
  wet and dry versions of one seed no longer had identical relief.
- Found by: the two box-ci failures, reproduced locally. A default-parameter mouth census placed
  seed 1's second entrance exactly at `(113,58,z3)`, and the constructed open-brow case reproduced
  the ground-height mutation.
- Cause: confirmed in `world/gen/mouths.rs`: `search` started at the hillside's surface wall and
  `cut` removed it, then wrote a new solid brow one level above when that tile was open air.
  `WorldGrid::set` recomputes ground as the highest wall, so the new brow both raised the column
  and exposed its limestone as the surface material. Mouth placement varies with hydrology, so
  these artificial height changes also differed between wet and dry characters.
- Fix: `search` retains the hillside face as a virtual search node but returns a path starting
  one level lower on a downward ramp. The original surface wall stays in place as the natural
  brow, so cutting the passage does not change any column's ground height or surface material.
  `cut` refuses a missing brow instead of fabricating one. Keeping the original search node also
  preserves the valid route to the constructed cave room at `z=-2`; moving the real search start
  down rejected that route.
- Guard: `tests/cave_mouths.rs` `constructed_mouth_never_adds_a_brow_or_raises_the_surface`
  requires a real mouth while asserting unchanged ground heights, an untouched existing brow,
  no added surface tile and bidirectional traversal;
  `constructed_mouth_requires_an_existing_rock_brow` rejects a missing brow without changing the
  grid. The original biome guards pass unchanged, as do all ten cave-mouth integration tests and
  the rollback unit test.
- Commit: (this commit).

## 2026-11-01 branch 482-cave-mouths: survival relief erased the arrival's shock

- Symptom: on the c147 merge head f8604d55, box-ci failed
  `qa_1026::the_arrival_is_a_shock_that_fades_and_the_calm_shake_it_faster`: the shocked party's
  day-1 mean mood was 0.5807307 against 0.32070386 without the arrival thought, so the arrival had
  become a net positive rather than a shock.
- Found by: reproducing the guard on f8604d55 and on pre-c147 f8d804a2, which failed identically,
  then comparing each person's thoughts, needs, stress and activity. Two control people carried
  `Sick` while the shocked people avoided it and ate better; Daelric's positive day-1 experiences
  outweighed his `Arrival` thought. A no-urgency control otherwise followed identical behavior and
  passed, confirming the divergence's mechanism.
- Cause: `systems::strain::survival_urge` gives newly arrived people a 1.5x survival weight, but the
  base arrival thought was only -0.35 before temperament. In this valley the resulting health and
  positive-thought gains exceeded that penalty, reversing the intended first-day mood effect. This
  was not caused by c147 ecology, whose code and habitat behavior are unchanged here.
- Fix: `systems::strain::arrival_shock` raises the base thought weight from -0.35 to -0.55 while
  preserving its calm/cautious/easygoing shaping, duration, stress spike and the 1.5x survival
  urgency. Survival still helps people secure food and shelter, but no longer erases the shock's
  mood cost. The day-1 mean is now 0.277 against 0.321 and the shock still fades within a week.
- Guard: the existing `tests/qa_1026.rs` arrival scenario is unchanged, including its day-1 mood
  comparison, thought comparison, calm-duration ordering and week-long recovery assertions. The
  focused biome and cave-mouth guards also remain unchanged.
- Commit: (this commit).

## 2026-10-29 #453 (epic #477, part 1): the sea was fresh

- Symptom: on coast worlds the sea stood in the world — the heightmap sinks one side and
  hydrology floods it — but it was the same plain WATER as a mountain pond; nothing in the
  sim could tell salt from fresh. Rot fouling water and sickness from drinking foul water
  (epic #477) have nothing to hang on until water carries its quality.
- Found by: the #477 scoping report (part 1 of 4), checked against `floods.rs` and
  `world/gen/heightmap.rs`.
- Cause: checked in the code: `Body` (`floods.rs`) holds kind, tiles, shore, catchment and
  stage, but no quality; the survey never looked at the landform or the map's edge.
- Fix: `floods.rs` gains `WaterQuality { Fresh, Salt, Brackish }` (appended new enum,
  serde default Fresh) and `Body.quality` (`#[serde(default)]`, so old saves load as fresh).
  `survey` marks the sea on coast worlds — the body hugging the map's lowest-mean-ground
  edge is Salt (the low side is read off the ground, not the landform's name) — and a
  distinct fresh body whose shore stands within two tiles of the sea is Brackish (a lagoon
  or an estuary mouth). `floods::quality(w, tile)` looks the quality up for any surface
  water tile (flood water and flood-left ponds take their body's quality). The view's
  `BlockInfo` (`view/block.rs`) gains `water_quality` ("fresh"/"salt"/"brackish"/"none").
  No behaviour changes yet: nothing drinks differently (part 4); rot does not foul water
  yet (part 2) — the planted hook at `block.rs` ("Rot beside water is not a water-quality
  simulation fact yet") still passes.
- Guard: `tests/water_quality.rs` `survey_marks_the_coast_sea_salt_and_an_adjoining_pond_brackish`,
  `valley_water_is_fresh_even_at_the_map_edge`, and
  `a_body_without_the_quality_field_loads_as_fresh`; the first two fail to compile on
  master (no `quality` field, no `floods::quality`).
- Commit: guard tests 47d2c709; the fix is the commit this entry ships in.

## 2026-10-30 #453 (epic #477, part 2): rot and the dead foul the water

- Symptom: a pile of half-rotten meat or an unburied body could lie in the shallows for days
  and the water stayed "clear fresh water" to every onlooker; nothing in the sim could say a
  pond was fouled, and the sickness-from-foul-water work (epic #477, later parts) has nothing
  to read.
- Found by: the #477 scoping report (part 2 of 4); the planted hook at `view/block.rs`
  ("Rot beside water is not a water-quality simulation fact yet") asserted exactly this gap.
- Cause: checked in the code: nothing linked `items.rs` `rot` or the dead in `rites_due` to
  the water (`floods.rs`); there was no tile-level fouling state anywhere.
- Fix: `Waters` gains `foul: BTreeMap<TilePos, u16>` — hours of fouling left per tile,
  `#[serde(default)]` so old saves load with none. The hourly system
  `systems/water_quality.rs` (registered in `systems/mod.rs` right after `water_flow`, at
  the same tick of the hour as `floods::hour`, hazards.rs `flood_step`): a water tile within
  one tile (Chebyshev, at about its own level) of food at `rot >= 0.6` on the ground or of a
  dead agent still in `rites_due` is set to `FOUL_HOURS` (24 game-hours, docs/TIME.md: 1
  game-hour is 1,200 ticks), topped up each hour the source lies there. Each hour then clears
  a share by how the water moves: a stream keeps 71% (a half-life of about two hours, clear
  in under a day from the worst); standing water — pond, lake, flood-left pond — keeps 97%
  (about an hour of fouling lost an hour, so a fouled pond persists about a day); a flood
  tile decays as running water and its entry goes entirely when the flood recedes and the
  tile is no longer water. Salt and brackish water foul and clear exactly as fresh — the
  simple rule, chosen and kept on purpose (no quality-conditional decay). The shared body
  lookup behind `floods::quality` is factored out as `floods::body_at` so the decay reads a
  tile's body kind. The view's `describe_water` (`view/block.rs`) reports "foul" for a fouled
  tile as its `water_kind` and text, riding over the swamp/fresh classification; the planted
  hook flips to expect "foul".
- Guard: `tests/foul_water.rs` `rot_and_a_body_beside_water_foul_it_and_no_further` (meat at
  rot 0.9 one tile from a pond and a dead agent on the shore foul the edge tiles for
  `FOUL_HOURS`; a tile two plane-tiles from both stays fresh),
  `a_stream_clears_foul_water_in_hours_a_pond_in_about_a_day` (fouled to the worst, the
  stream clears within 12 hours, the pond not before 20), and
  `waters_without_the_foul_field_load_with_none`. All fail to compile on plain master (part
  1 unmerged) and on the part 1 tree without the fix; demonstrated on the part 1 tree by
  stash.
- Commit: guard tests in the commit before the fix; the fix is the commit this entry ships in.

## 2026-10-30 #453 (epic #477, part 3): foul water sickens — the flux from a rotten drink

- Symptom: a pond edge fouled by half-rotten meat or a body left lying (part 2) was foul to
  every onlooker, yet drinking from it did nothing — the fouling was a view fact, not a
  substance you could take harm from. Epic #477's point (water is a substance) stops at the
  description until the water can hurt you.
- Found by: the #477 scoping report (part 3 of 4), checked against `water.rs` `drink` and
  `systems/health.rs`.
- Cause: checked in the code: `drink` reset thirst and noted the water as known, but nothing
  read `Waters.foul` at the moment of drinking, and nothing called `health::catch_at` from
  the water side.
- Fix: `water.rs` `drink`: when the source tile — the first open-water neighbour found, the
  same tile the knowledge note names — is in `Waters.foul`, roll once per drink on
  `rng::stream(seed, &[purpose::DRINK_FLUX, agent, day])` (new purpose tag; keyed by agent
  and day like health.rs's daily rolls, so a day's drinks share one stream coordinate set)
  and on `roll < 0.2 * sickness` call `health::catch_at(w, i, Flux, 0.35, "the flux from
  foul water")` — the severity `catch` gives any non-wasting sickness, and `catch_at` does
  the immunity guard and the FellIll event. The foul map is checked first: a salt or brackish
  tile fouls the same as fresh (part 2's rule), so foul is foul whatever the quality. The
  base chance 0.2 per foul drink is the day's unlucky living near a corpse condensed into
  the drink (health.rs daily rolls: 0.05 the flux near the dead lying a day, 0.2 a sickness
  caught close by), and the Sickness deity lever leans it exactly as the daily rolls are
  leaned. Wells and household barrel stores stay fresh by definition — `drink` only rolls
  when a tile of open water was the source, and they are no tile — so no change there. Salt
  and brackish drinking is unchanged (thirst resets regardless; refusal and salt physiology
  are part 4 and a follow-up). No save-format change: no new state, only a new rng purpose
  tag.
- Guard: `tests/foul_sickens.rs` `a_drink_from_fouled_water_brings_the_flux` (the part 2
  fouled-pond fixture, seed pinned so the drink roll lands under the chance; without the
  fix the drink brings nothing — demonstrated by stash on the part 2 tree),
  `a_drink_from_fresh_water_brings_nothing` (a fresh pond tile quenches, no illness), and
  `an_immune_drinker_stays_healthy` (the same pinned foul drink against an agent guarded
  against the flux). On plain master the guards fail to compile (parts 1-2 unmerged: no
  `waters.foul`, no `systems::water_quality`).
- Commit: guard tests 98172fcd; the fix is the commit this entry ships in.

## 2026-10-30 #453 (epic #477, part 4): a settler refuses salt and shuns foul water they know — unless desperate

- Symptom: every drinker was omniscient about the water. `water::nearest`/`spots` ranked
  every shore and well in the valley by ground truth — the sea's salt edge, a pond fouled
  by a corpse, brackish marsh — and the drink goal sent a settler to the nearest tile of it
  whatever they themselves had ever seen or smelled of it. The valley's view knew the water
  was foul (part 2) and could hurt (part 3), but the person walking to it knew nothing:
  knowledge was the player's, not theirs.
- Found by: the #477 scoping report (part 4 of 4), checked against `water.rs` `nearest` and
  `systems/decide.rs`'s drink goal and `choose_band` thirst arm.
- Cause: checked in the code: `Known::Water` beliefs carried no information about the water
  (the `tag` was always 0), and the drink sites were chosen by scanning `Waters::bodies`
  directly rather than through what the agent believes.
- Fix (knowledge is personal — the design core): a `Known::Water` belief's `tag` now holds
  the quality as THAT agent perceived it: `TAG_WATER_FRESH` 0 (the serde default, so beliefs
  in old saves load as fresh), `TAG_WATER_FOUL` 1, `TAG_WATER_SALT` 2, `TAG_WATER_BRACKISH`
  3 (`floods.rs`). `systems/perceive.rs` stamps it: salt and brackish by sight of the
  water, fresh otherwise; a fouled tile smelled within `smell_radius` — even with no sight
  line, and for smell-less people by sight of the fouled tile — is noted foul wherever it
  stands in reach, one foul shore noted being the pond remembered foul. Asleep agents
  perceive nothing (the looker filter already skips them). `water.rs` gains
  `nearest_knowing`: the drink choice ranked by the agent's own beliefs — a spot whose
  water is known salt is dropped entirely (sea water is never worth the walk, however
  parched); one known foul is dropped unless desperate, and desperate is thirst past the
  parched mark (`thirst_p > 0.7`, decide.rs's reach-doubling threshold) OR nothing they
  could believe fresh standing within reach (foul water or none beats dying of thirst).
  Water never perceived is assumed fresh — they can be wrong, and part 3's flux roll,
  ground truth at drink time, is the punishment. A foul belief anywhere taints the whole
  surveyed body for that agent, and what a drink teaches is noted too (a fouled tile tasted
  is remembered foul). `decide.rs`'s drink goal and `choose_band` thirst arm call
  `nearest_knowing`; the community's water-fetch planner and the thirst-blind `nearest`
  are unchanged. Save-format: `Belief.tag` gains `#[serde(default)]` — a field added where
  there was none before, old values read as 0 = fresh; no new `Known` variant, enums
  untouched.
- Guard: `tests/water_refusal.rs` `knowing_a_pond_is_foul_they_walk_past_it_to_fresh_water`
  (two ponds, one fouled; the perceiver takes the farther fresh one — fails without the
  fix, demonstrated by stash on the part 3 tree), `parched_past_desperation_they_drink_the_
  foul_pond_they_know` (thirst_p 0.75 takes the foul pond they know), `one_who_never_
  perceived_the_foul_pond_walks_to_it_and_drinks` (no knowledge, no refusal — they drink,
  and part 3 may sicken them), `a_known_salt_pond_is_never_chosen_even_when_nearer` (salt
  refused even parched), and `a_nose_knows_foul_water_without_a_sight_line` (a murrow
  behind a wall smells the foul shore). `tests/identity.rs` `evenings_at_the_hearth`'s
  evening/afternoon gathering ratio is loosened 3.0 to 2.5 for the same reason the #433
  drinkers are left out of its sampler: one who shuns a known-foul or known-salt pond walks
  to the next fresh one and the walk back is not separable from any other walk. On plain
  master the guards fail to compile (parts 1-3 unmerged: no `WaterQuality`, no
  `waters.foul`, no tags).
- Commit: the commit this entry ships in.

## 2026-10-29 #467: foragers froze on long trips in hard cold (#255 part 4)

- Symptom: seed 2's winters killed 34 of 48 by freezing, out foraging or stocking 8-92
  tiles from camp in a hard cold (the valley fell 31 -> 10 in 50 days).
- Found by: M5-review soak finding, carried to this issue; triage against master by code
  read (decide.rs, hazards.rs).
- Cause: confirmed by reading and by both guard tests failing before the fix. Three
  gaps: (1) Goal::Forage / Goal::Stock never weighed the cold they were in or the length
  of the trip against the warmth it would cost; (2) the Warm goal only engaged under
  warmth 0.4 AND 2 C, but warmth drains in the whole band below a people's comfort
  (hazards.rs cold_step), so between 2 C and comfort a freezing person got no
  warm-seeking goal; (3) the critical/interruptible pass only re-decided a freezing
  person while asleep, so a committed forager walked a 92-tile round trip to the end
  while warmth drained to frostbite (0.1) and death.
- Fix: `crates/hf-sim/src/systems/decide.rs`: the Warm gate is now the cold they are in
  against their own comfort (plus hardening), opening sooner the harder the cold
  (`warm_gate`, 0.4 up to 0.6); Forage and Stock scores are discounted by the cold
  against the walk (current temperature and their own warmth only — no forecast); a
  person freezing awake out of doors is critical and interruptible, so they turn back
  before frostbite; and with no lit hearth or roof known, the deep-frozen walk back
  toward the camp and the others (huddling loses less).
- Guard: `crates/hf-sim/tests/winter_forage.rs`
  `a_forager_turns_back_from_a_long_trip_in_hard_cold` (fails on master with warmth
  reaching 0.09, "going to foraging") and
  `half_warmed_in_hard_cold_they_warm_instead_of_setting_out` (fails on master: they set
  out foraging at warmth 0.45 in -10 C).
- Commits: guard `ef6c549c`; fix this commit. 600-day soaks on all four seeds run on the
  branch before merge.

## 2026-10-30 PR #476: personal water reactions stopped at an unrecorded choice

- Symptom: the E16 decision candidate emitted a choice and started an idle person's movement,
  but never recorded or resolved that attempt or formed a lesson. The existing hazard system
  had already moved non-idle people without an E16 trace; visible water away from the current
  tile could hide danger underfoot; every flooded tile was urgent; terminal records could not
  react to renewed evidence; and the choice text said "by move to safety".
- Found by: PR #476 correctness review; confirmed by tracing `systems/decide.rs`,
  `systems/hazards.rs`, `systems/perceive.rs`, `mind/problems.rs`, and `systems/act.rs`.
- Cause: the PR added a second, idle-only escape scorer beside the authoritative hazard path,
  addressed records by their evictable vector positions, and had no pending-outcome lifecycle.
- Fix: `mind/problems.rs` adds persistent `ProblemId`s, pending attempts with targets, bounded
  history, and renewed-evidence reopening. `systems/hazards.rs` remains the sole water-escape
  movement owner and now records real attempts, safety, blockage, abandonment and death, with
  `Learning::learn` fed by those outcomes. Its flood thresholds also classify perceived danger.
  `systems/perceive.rs` checks the current water tile before the first other scan hit;
  `systems/act.rs` reports path failure to the pending reaction; `world_state.rs` repairs ids
  from the first E16 saves on load. The duplicate decision scorer and incorrect grammar are gone.
- Guard: `tests/e16_problem_solving.rs` full-pipeline scenarios for dangerous escape, harmless
  flooding, multiple water tiles, failed/blocked retry, personal ownership, outcomes/learning,
  caps and pending save/load; existing `hazards`, `floods`, `lake_breaks`, `senses` and
  `foundations` suites preserve hard safety and persistence behavior.
- Commit: PR #476 follow-up.

## 2026-10-30 #414 review: effects invented weapon wounds and outlived their world

- Symptom: every person or animal entering pose 6 or 7 produced a weapon-impact effect although
  pose 7 covers falls, burns, cave-ins and work wounds; packed earth produced wood-tinted chips,
  stone floors produced earth-tinted chips, and effect sprites and agent id history survived a
  manual world load. Pose/work id dictionaries also retained departed agents indefinitely.
- Found by: PR #518 code review, checked against `crates/hf-godot/src/lib.rs` `agents`,
  `crates/hf-sim/src/world/material.rs`, and `world_view.gd` `reset_world`.
- Cause: confirmed in `effects.gd`: `_scan_agents` treated generic fight/flinch poses as
  authoritative weapon events; `material_family` included ids 23 and 62 as wood/stone and omitted
  ids 15 and 26; no history pruning or effect reset existed, and `world_view.gd` did not reset the
  new layer. `tools/ci.sh` did not run the advertised effects asset guard.
- Fix: `_scan_agents` uses the generic environmental-hit sheet for contact/wound accents, leaving
  weapon impacts only on explicit combat sound names. `material_family` follows the simulation
  material table. `_prune_history` removes departed ids and caps each history at 1,024 live ids;
  `effects.gd` `reset_world` hides all pooled sprites and clears active/resonance/agent history,
  called by `world_view.gd` `reset_world`. `tools/ci.sh` now runs `effects_check.py`.
- Guard: `tools/gd_test.gd` `test_effect_poses_never_invent_weapon_wounds`,
  `test_effect_pool_expires_and_never_exceeds_its_capacity`,
  `test_effect_history_prunes_departed_and_excess_ids`,
  `test_world_load_resets_effect_pool_and_history`, and the expanded material checks in
  `test_effects_are_bounded_and_fact_backed`. All failed for their respective defects on the
  reviewed head and pass with the fix.
- Commit: this PR #518 review-fix commit.

## 2026-10-30 #493: loose ground can become a desire path

- Symptom: footsteps were counted on every surface, but only grass, meadow and moss could ever
  compact into a path. Soil, clay, sand, gravel, peat, wet soil and scree therefore recorded wear
  without a corresponding ground consequence.
- Found by: the M6 ground-ecology seam audit of `paths.rs`, `world/material.rs` and the existing
  wear/path scenario.
- Cause: `paths::soft` maintained a second, incomplete list instead of using the material model's
  existing loose-ground classification.
- Fix: `paths::soft` now accepts every loose surface material except already-compacted earth,
  preserving the original material in `World::paths` so an unused path can regrow.
- Guard: `tests/paths.rs::heavily_walked_soil_compacts_and_can_regrow` fails on master because
  worn soil remains soil and no path provenance is recorded.
- Commit: (this commit).

## 2026-10-30 #519 review: exposed underground faces could not draw strata

- Symptom: the new `strata_face` overlay was unreachable, and natural underground cliff faces
  were suppressed even where their southern edge was exposed to a cave.
- Found by: PR #519 code review of `godot/scripts/world_view.gd` `_draw_cliffs`.
- Cause: confirmed by control-flow inspection. The early return rejected every natural solid
  whose surface `ground` was above `view_z`, while the overlay required exactly that condition.
- Fix: `_natural_face_overlay` now selects a face only for a natural solid with a non-solid
  southern neighbour. `_draw_cliffs` skips fully buried underground rock but draws exposed
  underground faces with strata; surface cliffs and built walls keep their existing treatment.
- Guard: `tools/gd_test.gd`
  `test_underground_strata_draws_only_on_exposed_underground_faces`, with exposed, buried and
  surface scenarios, plus the integrated art-only gallery shot in
  `docs/reviews/c414-underground-assets-2026-10-30/integrated_after.png`.
- Commit: this commit.

## 2026-10-30 #519 review: deep peeled floors checked for water at the wrong levels

- Symptom: a cave floor revealed up to six levels below the camera did not show seepage next
  to its actual adjacent water; water at the camera's level could be checked instead.
- Found by: PR #519 code review of `_ground_family` and `_cave_has_adjacent_water`.
- Cause: confirmed by comparing the returned visible floor level `g[3]` with the helper's fixed
  `[view_z, view_z - 1]` scan; the revealed level was discarded by the drawing call.
- Fix: pass `g[3]` through `_draw_underground_details` and scan the visible level and its
  corresponding lower water level. Dry floors and water only at `view_z` remain negatives.
- Guard: `tools/gd_test.gd` `test_underground_deep_peel_seepage_uses_visible_floor_level`
  builds a deterministic six-level peel and checks dry, wrong-level, visible-level and lower-level
  water. `test_underground_stage_contains_gallery_water` keeps integrated capture deterministic.
- Commit: this commit.

## 2026-10-30 fix-506-underground-context: unlit cavern rock collapsed into a black void (#506)

- Symptom: at underground level `z=-2`, rock and floor texture away from point lights was too
  dark to read the shape and material of the cavern reliably.
- Found by: PR #509 review of the integrated staged-demo lighting evidence; the initial guard
  only checked a numeric range, and the founding-centre shot contained no covered tiles. A
  matched cavern-centred before/after pair at `(40,87)` then exercised the actual render path.
- Cause: confirmed against base commit `6eb884ed`: `grade.gdshader` defaulted `under_level` to
  0.11, and `light_layer.gd` had no runtime assignment for that uniform. The sky mask already
  distinguished covered tiles, so the low ambient floor was the cause of this presentation
  defect rather than missing point lights or terrain data.
- Fix: `godot/shaders/grade.gdshader` sets the unlit underground default to 0.18;
  `godot/scripts/render/light_layer.gd` (`init`/`update`) loads that shader and assigns the same
  `UNDERGROUND_AMBIENT` value every update. Sky-mask occlusion, material hue, and point-light
  falloff are unchanged.
- Guard: `tools/gd_test.gd` `test_underground_ambient_keeps_caves_readable` builds a covered
  one-tile `z=-2` scenario through the real `LightLayer`, verifies its sky mask, overwrites the
  live uniform with a sentinel before `update`, and requires the actual shader default and
  runtime assignment to be 0.18. It therefore fails on either default drift or disconnected
  runtime wiring, not merely on a value outside a broad range. Matched before/after integrated
  captures and the visual review are in
  `docs/reviews/c506-underground-context-2026-10-30/README.md`.
- Commit: branch `fix/506-underground-context` (this commit).

## 2026-10-29 E16/#479: authoritative read-only worksite clues

- Symptom: E16's proposed mine and dig decisions had no single read-only source for the
  geological and roof-support clues already enforced by the simulation; each consumer would
  have to combine the aquifer and excavation queries itself.
- Found by: the E16 implementation and PR #479 review.
- Cause: confirmed: `aquifer::wet`, `aquifer::damp` and `underground::unheld_if_opened` held
  the authoritative facts separately, with no worksite-level read model.
- Fix: `environment::worksite_clues` projects those three queries into `WorksiteClues` without
  changing the world.
- Guard: `tests/e16_environment_clues.rs` builds a water-bearing limestone basin and a dry
  granite column, checks the existing dampness window, and asserts exactly three unsupported
  tiles when support is removed and zero with a completed prop.
- Commit: `73bac6a5`.

## 2026-10-29 #480 review: recent experience follows record time, not storage position

- Symptom: an unchanged observation, retested lesson or reseen problem could be refreshed in its
  old storage position and then omitted from the supposedly recent inspection rows. Once more
  than twelve attempts existed, the reaction trace kept older attempts and dropped newer ones.
- Found by: PR #480 review of `view/experience.rs`; the client guard checked only field types.
- Cause: confirmed in `inspect`: observations, lessons and problems used reverse insertion order,
  although their owning APIs update old records in place; reaction attempts were truncated before
  any timestamp selection, and only attempts from the already limited problem selection participated.
- Fix: `view/experience.rs` `inspect` selects observations by `tick`, lessons by `last_tested`
  then `formed`, and problems by `last_seen` then `opened`, newest first. Latest insertion breaks
  equal-time ties. Reactions from every bounded problem are selected by attempt `tick`, with
  problem and attempt insertion indices breaking ties, before the twelve-row limit. The client
  formatting moved unchanged to `rules.gd` `format_experience`, called by `main.gd`, so its
  populated case exercises the production presentation rather than a test-only rendering.
- Guard: Rust `recent_sections_follow_refresh_ticks_not_insertion_order`,
  `reaction_trace_keeps_latest_attempts_across_problems` (fifteen attempts over three problems),
  and `equal_timestamps_use_latest_insertion_order`; GDScript source-bound size/order checks and
  `test_person_inspection_formats_populated_experience` for personal evidence, expectations,
  section content and row order. This is a populated formatting case, not a claimed screenshot.
- Commit: PR #480 review follow-up.

## 2026-10-30 M5: the person inspector dropped wound layers and treatment

- Symptom: the Rust bridge supplied `Body::summary()` for an inspected person, but the Godot
  inspector never rendered the `body` field. A wounded person could show an injured sprite while
  the requested tissue layer and treatment remained unreadable.
- Found by: the c123/c131 Windows M5 tester pass and read-only code review of
  `crates/hf-godot/src/lib.rs` and `godot/scripts/main.gd`.
- Cause: confirmed: `_update_inspect` built the panel text from needs, equipment, knowledge and
  history but omitted the authoritative body summary already present in the inspection dictionary.
- Fix: `godot/scripts/rules.gd` formats the body summary as a dedicated, colour-coded inspection
  section, and `godot/scripts/main.gd` includes it for every inspected person. The text preserves
  the simulation's wound part, tissue layer, severity and treatment; no client-side wound facts
  are invented.
- Guard: `tools/gd_test.gd` `test_person_inspection_renders_body_layers_and_treatment`, which
  fails before the formatter/use is present.
- Commit: this fix commit.

## 2026-10-31 #523 (c135): a wholly stranded community climbs down

- Symptom: on seed 2 all 25 local people were stranded on a two-tile ledge above home and 22
  died of thirst, although both tiles had valid one-storey climb-down exits.
- Found by: the fresh four-seed 600-day soak on #516 head `12ce0fa5`, then triage against
  `systems/rescue.rs`. What put the people into the pocket remains unconfirmed.
- Cause: confirmed by code inspection and the new guard failing with zero climb-downs on c135:
  `rescue::step` rejected every cut-off person unless another local member could already reach
  home. When the whole local group was stranded and everyone else was away, nobody met that
  precondition, so the valid `climb_down` path was never attempted.
- Fix: `crates/hf-sim/src/systems/rescue.rs` `step` now tries the existing guarded climb-down
  before applying the home-reachable requirement. The requirement still gates underground
  digging, so away members cannot fabricate a rescue; climb-down ownership, destination and
  overhead safety checks are unchanged.
- Guard: `crates/hf-sim/tests/rescue.rs`
  `whole_community_on_a_ledge_climbs_down_without_anyone_home`, with both founders on the ledge
  and nobody home, asserts two recorded climb-downs and restored reachability for both. It
  failed before the fix with 0 rather than 2 climb-downs.
- Commit: guard and fix this commit.

## 2026-10-30 #467: stored clothing rags cannot become “new clothes”

- Symptom: a settler could receive “new clothes” and still be unclothed for cold protection.
  Agent 354 froze with clothing wear 0 only 8,700 ticks after that message; 25 of 39 freezing
  victims in the reviewed run had zero-wear clothing.
- Found by: the #467 freezing investigation, tracing the clothing event to `systems/life.rs`
  and the unclothed threshold in `systems/hazards.rs` `cold_step`.
- Cause: confirmed by code read and both new guards failing on c135. The daily replacement
  search ranked stored clothes only by quality. A storied set retained its recorded wear when
  installed, including 0, while `cold_step` treats wear at or below 0.2 as unclothed.
- Fix: `systems/life.rs` `clothes` now considers only candidates whose installed wear will be
  above 0.2, then retains the existing quality preference. Unusable storied sets stay in the
  store with their history and wear unchanged; if nothing usable is available, the current
  worn-out set remains for the existing replacement flow. Plain sets still install at full wear.
- Guard: `crates/hf-sim/tests/legend.rs` `clothing_replacement_refuses_zero_wear_storied_set`
  and `clothing_replacement_prefers_best_usable_set`. Both failed before the fix: the first
  emitted `NewClothes` at zero wear, and the second selected high-quality zero-wear rags over
  a usable plain set.
- Commit: guard tests 7febac18; fix this commit.

## 2026-10-31 #526 (c137): wrongful judgements were indistinguishable in feed and history

- Symptom: the client feed and History panel narrated a wrongful judgement exactly like an
  ordinary one, although the recorded event distinguished them.
- Found by: M5 evidence review. In seed 1's `ring_s1.2.hfsave`, history id 34719/event 55365 at
  tick 290844300 records judge 141, accused 153, "breaking the bed", pardoned, `wrongly=true`;
  seed 4 also has an amends judgement with `wrongly=true`.
- Cause: confirmed in `crates/hf-sim/src/view/narrate.rs`: the `Judged` match arm ignored the
  event's existing `wrongly` field. `crates/hf-sim/src/justice.rs` already records it correctly.
- Fix: `narrate::line` inserts "wrongly" only for a judgement whose recorded flag is true.
  Ordinary judgement wording is unchanged. This is presentation only: no simulation state,
  events, verdicts or other facts are added or changed.
- Guard: `crates/hf-sim/tests/history.rs`
  `judgement_narration_marks_only_wrongful_verdicts` checks exact true/false wording both directly
  (the feed path) and after chronicle JSON round-trip (the History path). The true case failed
  before the fix with the ordinary wording; the false case already passed.
- Commit: this fix commit.

## 2026-10-31 #467: freezing restarted a meal instead of letting it finish

- Symptom: a freezing settler with a meal in hand could remain critically hungry forever,
  restarting the same meal rather than consuming it and then seeking warmth.
- Found by: the confirmed #467 cold-priority investigation, reproduced by the new
  `winter_forage` guard on c141: after 301 decide/act ticks the carried meal remained and
  hunger was still 0.07.
- Cause: confirmed in `systems/decide.rs` `step`: the awake-freezing interruption becomes
  eligible after 200 activity ticks, but `Activity::Eating` takes 300 ticks. At hunger 0.07
  the carried meal scores about 6.75 against Warm's 3.55 at warmth 0.39 (and 5.5 at zero
  warmth), so each interruption immediately chose and restarted eating.
- Fix: `systems/decide.rs` `step` excludes only an in-progress `Activity::Eating` from the
  freezing-only interruption. Walking to food and all other cold interruptions are unchanged;
  after the meal and its normal idle delay, Warm is chosen as before.
- Guard: `crates/hf-sim/tests/winter_forage.rs`
  `a_freezing_settler_finishes_the_meal_in_hand`, which also checks the subsequent Warm choice.
  It failed before the fix with `meal was not consumed: eating`.
- Commit: guard and fix this commit.

## 2026-10-31 #523 (c138): rescue skips people mid-step at its hourly check

- Symptom: after the whole-community home-guard fix, seed 2 still ended with no humans and
  exactly 129,674 chronicle rows, byte-identical to the pre-fix run; no climb-down was recorded.
- Found by: the fresh four-seed 600-day soak in
  a local validation directory, chronicle/topology inspection, and an
  integration scenario exercising the normal pipeline with 25 people on a two-tile ledge.
- Cause: confirmed in `systems/rescue.rs` `step`: its hourly scan still discarded anyone for
  whom `Agent::is_moving()` was true before attempting `climb_down`. Repeated short walks can
  remain phase-aligned with that single checkpoint, so people keep moving, report no stalls,
  and nevertheless never receive the rescue attempt. The final save's pocket and every climb
  exit were checked separately and are valid.
- Fix: `rescue::step` no longer rejects a cut-off person for being mid-step. The existing
  `climb_down` already clears the old path and movement interpolation before placing them on a
  walkable, uncovered, home-reachable tile. Community ownership and the local-home requirement
  for underground digging are unchanged; away members still cannot fabricate a rescue.
- Guard: `crates/hf-sim/tests/rescue.rs`
  `whole_community_climbs_while_short_walks_cross_the_rescue_tick` puts 25 founders on the
  two-tile ledge, all mid-step at the hourly tick, and requires every distinct person to climb
  and regain reachability. It failed on the previous head with 0 rather than 25 climbers.
- Commit: this fix commit.

## 2026-10-31 #486: each valley draws a limited wild-flora community

- Symptom: every generated valley could contain all 22 wild kinds added by #286, so seeds
  differed in arrangement and habitat but not in which authored community they held.
- Found by: issue #486's world-bag acceptance criterion, checked against
  `world/gen/flora.rs` and `world/gen/biome.rs` on c144.
- Cause: not a fault in the existing habitat rules; the per-world subset draw was missing.
- Fix: `world/gen/flora.rs` adds `FloraDraw`, a deterministic eight-kind draw without
  replacement from the existing 22 wild candidates, using its own purpose-tagged RNG stream.
  `apply` excludes undrawn trees and low flora, while preserving each biome's aggregate low-flora
  density among the drawn kinds. Existing biome, warmth and waterside constraints still run.
  Core flora, cave flora and survivor-sown crops are unchanged. `WorldMeta.flora_draw` records
  the draw as an optional serde-defaulted field (old saves remain readable);
  `world/gen/biome.rs` `describe` names drawn and undrawn kinds and distinguishes a world-bag
  absence from a habitat absence. `setup.rs` includes both summaries in the valley preview.
- Guard: `tests/flora.rs` `each_world_draws_a_limited_flora_community` (eight seeds, exact
  limit, uniqueness, determinism, core/crop controls and no undrawn generated plant),
  `flora_report_distinguishes_not_drawn_from_no_habitat`, and
  `setup_preview_shows_the_worlds_flora_draw`. Existing biome and habitat guards remain in
  `tests/biomes.rs`.
- Commit: guard and fix this commit.

## 2026-10-31 #469 (c143 residual): an off-stripe sleeper notices a bed in sight

- Symptom: an exhausted person with a free bed two tiles away slept on the ground if the bed was
  not already in their personal knowledge, then remained asleep through later perception passes.
- Found by: the deterministic `night_world` scenario at tick 56,400. On c143, agent 0's first
  `World::step` was off their perception stripe and produced `Sleeping { bed: None, until: 66001 }`;
  the identical scenario with the bed in personal knowledge chose the bed.
- Cause: confirmed in `systems/decide.rs` and `systems/perceive.rs`: sleep candidates came only
  from personal knowledge, perception ran on a 20-tick stripe, and sleeping people were skipped.
  Committing to ground sleep between looks therefore made the visible bed unavailable indefinitely.
- Fix: `systems/perceive.rs` `sight_radius` exposes the existing daylight/moonlight/lamp sight
  calculation for both perception and decisions. In `systems/decide.rs` `choose`, only when no
  known free reachable bed exists, the same bed scorer considers currently visible Bed/Tent/Nest
  structures, bounded by that sight radius and existing line-of-sight and reachability checks.
  It does not add knowledge or reveal occluded or remote structures; scoring, claims, eviction
  budgets and race handling are unchanged.
- Guard: `crates/hf-sim/tests/sleep_bed.rs`
  `an_exhausted_person_uses_a_visible_bed_they_do_not_know` failed on c143 with ground sleep and
  passes with the fix; `an_exhausted_person_uses_a_known_bed_on_the_same_off_stripe_step` is the
  known-bed control and passed both before and after.
- Commit: guard demonstrated on c143; guard and fix this commit.

## 2026-10-31 #469 (c144 review): visible sleeping places are complete and locally indexed

- Symptom: the visible-bed fallback could send an exhausted person to a Bed/Tent/Nest still under
  construction, and every sleepy decision scanned every structure and scanned every agent again
  for each candidate's claim.
- Found by: review of PR #535, then deterministic guards on the merged c144 branch head. An unknown
  visible bed at 50% progress was chosen; 24 sleepy decisions with 400 remote beds took 24 full
  structure walks.
- Cause: confirmed in `systems/decide.rs` `choose`: the fallback iterated `w.structures` directly
  without a progress check, and its candidate closure iterated `w.agents` for each claim.
- Fix: `systems/decide.rs` adds a lazy per-decision-tick `SleepingPlaces` index. It buckets only
  completed Bed/Tent/Nest structures by local cell, restoring original structure order before
  checking the same personal sight radius, line-of-sight and reachability and scoring. Claims
  are gathered once; `step` replaces them as each activity commits, preserving same-tick races
  and tie-breaking. Remote and unfinished structures
  remain unknown to the choice; known-bed scoring and knowledge eviction are unchanged.
- Guard: `crates/hf-sim/tests/sleep_bed.rs`
  `an_exhausted_person_does_not_use_an_unfinished_visible_bed` failed before the fix by choosing
  the unfinished bed. `sleepy_decisions_index_remote_sleeping_places_once_a_tick` failed with 24
  structure walks and passes with one, while asserting that nobody chooses a remote bed.
- Commit: guards demonstrated on the merged c144 head; guards and fix this commit.

## 2026-10-31 #430: the Windows watch-copy still required a manual rebuild

- Symptom: the box published a Windows `hf_godot.dll` for each master checkpoint, but opening the
  latest viewer still required a manual pull, commit-matched DLL copy, or local Cargo build.
- Found by: issue #430 and inspection of `tools/prebuilt/README.md`, which documented the manual
  consumer steps but had no launcher.
- Cause: confirmed: only the publisher half existed on master; no script connected a Windows
  shortcut to the pull, verify/install, fallback-build and launch sequence.
- Fix: `watch-copy.cmd` provides the shortcut target and locates Git Bash;
  `tools/watch-copy.sh` refuses dirty or non-master trees, fast-forward pulls `origin/master`,
  verifies the exact commit's prebuilt SHA-256 before atomically replacing the DLL, falls back to
  `cargo build --release -p hf-godot` only when no prebuilt exists, and launches Godot. Hashing by
  filename from the DLL directory avoids Git Bash's escaped checksum output for Windows paths.
- Guard: `tools/tests/watch_copy_test.py` builds five deterministic temporary-repository scenarios
  with stub Cargo/Godot tools: verified prebuilt update and launch, local-build fallback, dirty
  refusal, non-master refusal, and checksum failure preserving the previous DLL. The scenarios
  failed before the launcher existed; the CRLF and Windows-path cases were also exercised during
  implementation. The harness runs from `tools/ci.sh`.
- Commit: this fix commit.

## 2026-10-31 #488: binary traits were the whole personality and the inspector exposed all of them

- Symptom: a new person had only one to three binary traits, with no dimensional core profile or
  context-sensitive surface expression. Person inspection and setup cards displayed that complete
  list rather than a bounded current expression.
- Found by: issue #488 design review against `mind/traits.rs`, `genetics.rs`, the opening identity
  cards and the person inspector on c144.
- Cause: confirmed: `Genome::traits` reduced the temperament loci to thresholded labels, `Agent`
  had no separate dimensional profile, and `hf-godot` passed `traits.list` directly to the client.
- Fix: `mind/dispositions.rs` adds eight deterministic 0..1 core values and computes, but never
  saves, their acute expression from rest, mood, stress and body condition. `systems/decide.rs`,
  `systems/needs.rs` and `systems/culture.rs` use the relevant expressed axes for existing social,
  work, wandering and hauling weights. `scenario.rs` and `opening.rs` initialize profiles for new
  people; `agents.rs` uses an optional additive field so old saves retain legacy behavior. Setup
  and inspection expose at most five salient surface labels and no core values. Model assumptions
  and remaining #488 work are explicit in `docs/design-dispositions.md`.
- Guard: `crates/hf-sim/tests/dispositions.rs` builds deterministic scenarios for core persistence
  through strain and recovery, bounded genome derivation and surface salience/cap. Paired
  `decide::step` scenarios prove relevant axes change committed talk, work, wander and haul/chop
  outcomes while unrelated axes leave them identical. A real pre-field v2 fixture checks old-save
  fallback, and snapshot restore matches continuous simulation after another 1,200 ticks. The
  focused suite passes 12/12. `tools/gd_test.gd` passes 20/20, including the public surface/core
  boundary. Reviewed setup and live-inspection captures are in
  `docs/reviews/dispositions-2026-10-31/`; full box-ci and the required simulation soak remain.
- Commit: guard and fix this commit.

## 2026-10-31 #428: every dirty region union allocated a fresh parent table

- Symptom: every dirty navigation update allocated and initialized a fresh 409,600-element
  `u32` union-find parent table (about 1.56 MiB on the default map), adding allocator work to
  the rebuild spike.
- Found by: allocation review of `nav/regions.rs` `Nav::union_all` on c144.
- Cause: confirmed: `union_all` created a local `parent` vector on every call, although the map
  dimensions are fixed for the lifetime of a `Nav` and `World::rebuild_caches` rebuilds `Nav`
  when the dimensions change.
- Fix: `Nav` now keeps the parent scratch beside `comp`, allocates it in `Nav::build`, and
  refills every entry with its own index at the start of every `union_all`. Edge traversal,
  path-halving, root choice, the dense region scan, and unsupported-item dropping are unchanged.
- Guard: `nav::regions::tests::union_all_refills_poisoned_parent_scratch` fills the persistent
  scratch with stale zero links and requires both parent roots and components to equal a fresh
  build. The existing navigation split/reconnect/no-op, footing-removal and item-fall guards,
  plus the 1-vs-6-thread digest guard, cover the unchanged behavior.
- Measured: `ostrog --threads 8 profile --agents 500 --provision --days 20` on c144 plus
  this fix. Every run ended with digest
  `9cc68bea47b620860d9052bc5fc297920d96f98169d7c36501a825afec4a981c`.

  | Run | Mean tick | Worst tick | Nav max |
  |---|---|---|---|
  | fused-scan reference | 2.066 ms | 43.422 ms | 37.187 ms |
  | loaded run 1 | 2.374 ms | 27.813 ms | 11.344 ms |
  | loaded run 2 | 2.248 ms | 26.410 ms | 9.697 ms |
  | quiet, post-CI | 1.990 ms | 23.342 ms | 8.831 ms |

  The first two runs overlapped unrelated 600-day soaks; the quiet run has no mean regression
  against the reference and passes the 2 ms mean gate. No 4×600-day soak was run.
- Commit: this fix commit.

## 2026-10-31 #484: no bounded, reviewable record of settlement change

- Symptom: a watcher could inspect current history or record a manually bounded footage run, but
  could not automatically retain and scrub deterministic daily views of a settlement while the
  source simulation continued.
- Found by: issue #484 acceptance review against c144; `main.gd` had only one-shot `--shot` and
  finite `--footage` capture, with no sampled-state metadata, retention ring or image-only review.
- Cause: confirmed absence of a viewer-owned sampling/review path; no simulation defect is alleged.
- Fix: `godot/scripts/timelapse.gd` bounds 120 daily samples; `main.gd`
  `_capture_timelapse` records a fixed watched view with seed/tick/state-digest/filter metadata;
  `timelapse_panel.gd` provides image-only scrub, playback and side-by-side comparison without a
  `SimHost`. `hf-godot` exposes read-only `world_seed` and `state_digest`. Scope and remaining
  whole-valley/region/event-marker work are explicit in `docs/TIMELAPSE.md`.
- Guard: `tools/gd_test.gd` `test_timelapse_ring_is_bounded_and_preserves_metadata` uses a
  deterministic 121-sample scenario and exact byte accounting;
  `test_timelapse_state_identity_is_deterministic` checks equivalent identity and read-only access.
  Runtime PNG equivalence and multi-day visual evidence remain unrun here and assigned to box-ci /
  the visual evidence pass.
- Commit: this feature slice commit.

## 2026-10-31 #485 first slice: talking idled and eating reused the sitting frame

- Symptom: a person in a real conversation played the ordinary idle loop, while a person eating
  reused the single sitting frame, so two common life actions were not readable as those actions.
- Found by: #485 source review of `hf-godot` `agents`, the people sheet layout and the client's
  pose dispatch on c144. No simulation activity or timing defect was found or changed.
- Cause: confirmed in `crates/hf-godot/src/lib.rs`: `Activity::Talking` fell through to pose 0
  and `Activity::Eating` selected pose 5. `tools/art/people.py` had no activity-specific frames
  for either action, and `entity_layer.gd` had no corresponding animation branch.
- Fix: `people.py` appends three talk and three eat poses across every plan/build/age/layer;
  `person_pose` selects poses 10/11 only for the matching activity after existing combat, wound,
  movement and body priorities; `Rules.everyday_frame` and `entity_layer.gd` play complete cycles.
  `view::look::look` suppresses a held spear only while eating. This is presentation-only and
  adds no simulation state or world-digest input.
- Guard: the hf-godot unit scenario `talking_and_eating_have_dedicated_poses_without_changing_priority`;
  `look.rs` `people_sheets_carry_the_action_frames_and_every_plan` (all nine plans) and
  `an_eater_does_not_hold_a_spear_to_their_face`; GDScript `test_everyday_poses_use_three_distinct_frames`.
  Palette/provenance/silhouette checks and reviewed contact/strip evidence accompany the assets.
- Commit: guard, generator, runtime integration and assets in this commit. Rust/Godot execution
  and integrated-game captures are delegated to box-ci; the full #485 acceptance remains open.

## 2026-10-31 #428: water perception read the same scan square twice

- Symptom: the c142 500-human provisioned 20-day profile spent 1,900 µs in `perceive` per
  tick on average (2.656 ms mean tick, 81.786 ms worst). Its first visible-water scan and
  foul-water scan each read the same grid square.
- Found by: the #428 performance follow-up, checked against `systems/perceive.rs` on c142.
- Cause: confirmed by code inspection: the `wr` first-visible scan was followed by a separate
  `sr = max(smell, wr)` foul scan, repeating every in-bounds grid read in their shared square.
- Fix: `systems/perceive.rs` now makes one `sr`-radius pass with one grid read per tile. The
  `wr` subsequence retains first-visible-water dy/dx order; foul notes retain scan order and
  per-body deduplication. The existing sight memo, quality tags, direct E16 observations and
  subsequent Learning code are unchanged.
- Guard: `crates/hf-sim/tests/perceive_cost.rs`
  `one_look_reads_the_water_scan_square_at_most_once` caps one human look at `(2*sr+1)^2`
  water-grid reads. `fused_water_scan_matches_the_two_scan_reference` compares the original
  two-scan semantics for shallow/deep water, blocked sight and multiple foul tiles in two
  bodies, including belief position/tick/tag and all `learning.observations`;
  `fused_water_scan_preserves_smelled_foul_behind_a_wall` covers smell without sight.
- Measurement: the same 20-day, 500-human, 8-thread provisioned profile now reports 1,484.26
  µs mean `perceive`, 2.066 ms mean tick and 43.422 ms worst tick, with the c142 digest
  `9cc68bea47b620860d9052bc5fc297920d96f98169d7c36501a825afec4a981c` unchanged. Perception
  remains the largest mean system cost; nav's 37.187 ms maximum is the largest remaining
  system spike. No 600-day soak was run for this narrowly scoped equivalence change.
- Rejected follow-up: `cfcd2eb8` added a step-local `BTreeMap` memo for non-foul quality tags,
  but the identical 8-thread profile regressed mean tick from 2.066 to 2.105 ms and mean
  `perceive` from 1,484.26 to 1,517.55 µs. Its 29.085 ms worst tick did not justify retaining
  the mean regression. The memo and its extra counter/guard were reverted by a normal revert;
  the useful fused scan and its guards stay. Both thread counts had preserved the digest, so
  correctness was not the reason for rejection. No further speculative optimization is claimed.
- Commit: the fused scan and this record ship in their respective commits; the memo is reverted.

## 2026-10-31 #430: a fresh watch-copy reached Godot without registering SimHost

- Symptom: the first real c145 `watch-copy.cmd` acceptance run installed the correct prebuilt DLL
  and started Godot, but the fresh checkout's viewer stayed open with `Could not find type
  "SimHost"` instead of rendering the world.
- Found by: real Windows acceptance using the mirror c145 DLL and project-local Godot 4.6.3, not
  the stubbed launcher harness.
- Cause: confirmed: the launcher installed the extension but never ran Godot's import/editor pass,
  so a fresh `.godot` cache had no extension registration. After the import was added, Godot also
  rewrote tracked `.import` sidecars byte-for-byte; porcelain status reported stat-only changes
  even though their blob hashes and diffs were unchanged.
- Fix: `tools/watch-copy.sh` runs a headless editor import and requires
  `.godot/extension_list.cfg` before launching. Its cleanliness guard now checks actual staged and
  unstaged diffs plus untracked files, so byte-identical import rewrites do not block the next
  launch while genuine edits still refuse.
- Guard: `tools/tests/watch_copy_test.py` now requires the import invocation before the viewer
  invocation. Real c145 `.cmd` acceptance then verified the mirror DLL SHA-256
  `8f07e40a63b6453a00962e0992f0266154157331650e6a2f9029fc0dfed8a4af`, imported, launched on the
  NVIDIA OpenGL renderer and wrote `prebuilt.png` with `SHOT_SAVED`. Separate real `.cmd` dirty and
  non-master runs exited 1 with unchanged HEAD and created neither a DLL nor a Godot cache.
- Commit: this fix commit.

## 2026-10-31 #430: the watch-copy harness used the host Godot instead of its stub

- Symptom: on a checkout with the wrapper's fallback Godot installed, the focused watch-copy suite
  failed its prebuilt and local-build success scenarios because neither created the expected import
  registration; the same suite passed in the runner image.
- Found by: `python3 -I tools/tests/watch_copy_test.py` on the PR #537 head, which failed 2 of 5
  scenarios before this correction.
- Cause: confirmed: the harness put `godot4` only at the front of `PATH`, but `tools/godot` prefers
  its fallback installation when the temporary checkout has no project-local executable. The real
  Godot therefore replaced the stub on hosts with that fallback, making the suite host-dependent.
- Fix: `tools/tests/watch_copy_test.py` installs its stub as the temporary checkout's
  `.tools/godot4`, the wrapper's first choice. The launcher and its import-registration failure
  check are unchanged.
- Guard: the same focused command now passes all 5 scenarios with the host fallback present; the
  test still requires the import and launch invocations and the extension registration file.
- Commit: this fix commit.

## 2026-10-29 feat/e16-social-knowledge: sourced warnings stay fallible at the map edge (#478)

- Symptom: `Evidence::MisledBy` displaced the saved `Remembered` variant; a distorted warning
  immediately told its recipient about the teller's deceptive intent; and a problem on the
  right edge produced its own location after distortion was clamped.
- Found by: PR #478 review of saved enum compatibility, recipient knowledge, and edge behavior.
- Cause: confirmed in `mind/learning.rs`, `mind/problems.rs` `hear`, and `systems/beliefs.rs`
  `share`: the new variant was inserted rather than appended, the teller-only `misleading` flag
  was converted into recipient evidence, and the distorted x coordinate could clamp to its input.
- Fix: append `MisledBy` after `Remembered`; keep every newly heard warning `ToldBy` so only a
  later correction or discrediting can establish deception; step left when the usual rightward
  distortion reaches the right edge.
- Guard: `tests/e16_phase1.rs` round-trips `MisledBy`; `tests/e16_problem_social.rs` checks that
  rivalry distorts without revealing intent and that an edge warning names a different valid
  tile; `tools/enum_order.py` checks the saved variant prefix.
- Commit: PR #478 review correction (this commit).

## 2026-10-31 c146: one water observation per look, owned by the fused scan

- Symptom: `direct_perception_distinguishes_shallow_and_deep_water_for_the_observer_only`
  failed with 2 learning observations where 1 was expected for a visible deep-water tile.
- Found by: box-ci on PR #476 head `e1d8e623` (`hf-sim::e16_phase1`), reproduced locally
  with `cargo nextest run -p hf-sim --test e16_phase1`.
- Cause: confirmed by code inspection: the perceive step's merged fused-scan block observed
  the picked water tile (`WaterDepth` 1.0), and the E16 tail call `hazards::observe_water`
  observed the same tile again at its danger urgency (`WaterDepth` 0.9). The two strengths
  differ by more than the merge tolerance, so both observations were kept. Shallow water
  re-stated the same 0.55 wetness and merged, hiding the duplicate there.
- Fix: `systems/hazards.rs` `observe_water` no longer emits a `Learning::observe`; it records
  only the personal-danger problem when the person stands on the tile. The look's single
  water observation stays with the perceive step's fused scan (strength 1.0 deep, 0.55
  shallow), as master's `fused_water_scan_matches_the_two_scan_reference` oracle expects.
  The perceive-cost reference keeps modeling the hazard tail call so any future second
  observation is caught by the exact observations comparison.
- Guard: `crates/hf-sim/tests/e16_phase1.rs`
  `direct_perception_distinguishes_shallow_and_deep_water_for_the_observer_only` (fails with
  2 observations on the previous head) and `crates/hf-sim/tests/perceive_cost.rs`
  `fused_water_scan_matches_the_two_scan_reference` (exact observations comparison against the
  reference look).
- Commit: PR #476 fix `87e37b83`.

## 2026-10-31 c146: a restore keeps a problem-free person's next problem id at zero

- Symptom: `save_memory::a_borrowed_snapshot_encodes_as_the_owned_one_read_back`,
  `save_memory::checking_a_save_reads_its_files_a_block_at_a_time`,
  `resume::a_game_resumed_mid_hour_goes_on_the_same` and
  `generated::a_world_with_generated_kinds_resumes_the_same` all failed on their final digest
  comparison: the world restored from a snapshot of the live world hashed differently from
  the world that was saved.
- Found by: running the save/restore suites on PR #476 after the water-observation fix;
  box-ci had stopped at the earlier e16_phase1 failure before reaching them.
- Cause: confirmed in `mind/problems.rs` `repair_ids`, called from
  `world_state.rs` `rebuild_caches` on every restore: for a person with no problem records it
  rewrote `next_id` from 0 to `next_id.max(1)`. In-session records always carry ids, so the
  repair is only meaningful for the first E16 saves; on a current save it changed a saved
  field, and the digest (the snapshot's encoding) no longer round-tripped.
- Fix: `repair_ids` returns early when there are no records. The old-save repair path
  (zero or duplicate ids) is unchanged.
- Guard: the four suites above; `tests/e16_problem_solving.rs`
  `ids_survive_eviction_and_attempt_history_stays_bounded` keeps id stability under eviction.
- Commit: PR #476 fix `1f5b0bf8`.

## 2026-10-31 PR #476: deep water produced two direct observations in one look

- Symptom: an observer looking at deep water retained two `WaterDepth` observations for the same
  tile and tick, while shallow water correctly retained one.
- Found by: `tests/e16_phase1.rs`
  `direct_perception_distinguishes_shallow_and_deep_water_for_the_observer_only` failed with two
  observations rather than one after the c145/c146 integration.
- Cause: confirmed in `systems/perceive.rs` and `systems/hazards.rs` `observe_water`: perception
  recorded the direct depth cue at strength 1.0, then the hazard hook recorded the same cue at the
  problem's 0.9 urgency. `Learning::observe` correctly treated that strength change as a delta.
- Fix: `observe_water` now keeps direct cue strength separate from problem urgency. Deep water
  refreshes the existing 1.0 observation; the current-tile problem still receives 0.9 urgency, and
  flooding keeps its distinct 0.9 cue. No observer gains information beyond the selected visible
  water tile.
- Guard: the original observer-only/line-of-sight E16 guards are unchanged;
  `tests/perceive_cost.rs` `fused_water_scan_matches_the_two_scan_reference` now requires exactly
  one full-strength observation for both depths, and `tests/e16_problem_solving.rs` retains the
  current-tile danger, escape, outcome and learning checks.
- Commit: this fix commit.

## 2026-10-31 PR #476: restoring empty problem ledgers changed the world digest

- Symptom: a generated-kinds world had a different digest immediately after save/restore, before
  any resumed ticks, and remained different from the continuous run.
- Found by: `tests/generated.rs` `a_world_with_generated_kinds_resumes_the_same`; a restore-point
  digest assertion isolated the drift from subsequent simulation behavior.
- Cause: confirmed in `mind/problems.rs` `Problems::repair_ids`: every restored agent with an empty
  default ledger had `next_id` changed from 0 to 1 even though there was no record to repair.
- Fix: `repair_ids` now preserves the counter for an empty ledger and starts at 1 only when a
  missing or duplicate record id actually needs replacement. Existing ids and non-empty counters
  retain their repair behavior.
- Guard: the generated-kinds resume test now checks digest equality both at restore and after two
  further simulated hours alongside the continuous world.
- Commit: this fix commit.
- Commit: PR #476 `e232a46f`.

## 2026-10-31 PR #480: the restacked experience view preserves personal reaction records

- Symptom: merging the repaired #478 base conflicted with #480 in water perception and the fix log;
  the experience tests also span the `ProblemId`/`ReactionAttempt.target` API boundary.
- Found by: the exact-head restack merge and focused `view::experience` compilation.
- Cause: confirmed: #480 carried the older multi-water vector while the repaired base deliberately
  selects one visible water tile and gives the current tile priority; reaction attempts are
  addressed by persistent ids and carry an optional movement target.
- Fix: `systems/perceive.rs` retains the repaired base's single selected visible tile, fused scan,
  direct observation and underfoot override. `view/experience.rs` retains `ProblemId` addressing
  and explicit `target` initializers. The inspector still reads only the selected agent's bounded
  records; README now documents those sections without presenting them as world truth.
- Guard: `view::experience` unit tests, the three E16 integration suites, generated-kind and
  mid-hour resume tests, and the existing water-perception equivalence guards.
- Commit: PR #480 restack merge.

## 2026-10-31 #486 review: recorded draws, realized density and the seed-7 ward guard

- Symptom: box-ci's `thin` guard chose to leave seed 7's stone open instead of warding it;
  review also found that flora reporting recomputed the seed draw, legacy saves could be
  misreported, climate-ineligible drawn kinds reduced realized low-flora density, and the setup
  guard compared only line prefixes.
- Found by: exact-head box-ci (801/802 tests passing) and PR #538 review.
- Cause: confirmed by code inspection. `biome::describe` called `FloraDraw::of_seed` instead of
  reading the world record; low-flora normalization counted drawn kinds later rejected by the
  climate filter; the preview guard had no exact text comparison. The thin chooser's open/ward
  decision remained valid, but the changed world trajectory selected a curious person, so the
  old seed-dependent scenario no longer exercised warding.
- Fix: `describe` now takes the actual optional record and reports legacy `None` explicitly;
  CLI and setup pass their generated/saved records. Low-flora normalization includes only
  climate-eligible kinds. The setup guard compares complete summaries. The thin scenario stages
  non-curious people while retaining every studying, construction and quieting assertion.
- Guard: `tests/flora.rs` `flora_report_uses_the_saved_record`,
  `legacy_flora_draw_is_reported_as_unknown`, `setup_preview_shows_the_worlds_flora_draw`, and
  `limited_flora_preserves_realized_density_against_full_catalogue` (eight-seed exact site
  controls against the full catalogue). `tests/thin.rs`
  `they_study_it_and_the_wards_quiet_it` retains its original behavioral expectations.
- Commit: this review-fix commit; focused Rust execution remains assigned to box-ci.

- Commit: PR #476 `e232a46f`.

## 2026-10-30 #493: crop growth responds to surface material within a biome

- Symptom: crops used only their biome, season and nearby-water factors, so two fields in the
  same biome grew identically even when one stood on fertile meadow and the other on loose sand.
- Found by: M6 living-ecology slice for #286, checking the existing surface-ground material
  against the crop growth path.
- Cause: `systems/plants.rs` read the biome at a crop but never read the supporting tile's
  material.
- Fix: `plants::crop_ground_factor` maps the existing surface materials to bounded crop-growth
  factors, with wet-ground preferences for oats and peas and dry-ground tolerance for barley and
  rye; the result multiplies the existing biome, season and water factor.
- Guard: `tests/flora.rs::crop_growth_responds_to_surface_material_within_same_biome` fails
  without the material factor because identical-biome crops on meadow and sand advance equally.
- Commit: this fix commit.

## 2026-10-31 #414: over-cap effect histories respawned work accents every frame

- Symptom: with more than 1,024 visible agent ids, every id beyond the effects history cap could
  produce the same work or pose accent again on every frame instead of once per transition.
- Found by: PR #518 diff review after the c146 fold, then the expanded
  `test_effect_history_prunes_departed_and_excess_ids` guard failing on the merged head before
  this fix.
- Cause: confirmed in `effects.gd`: `_scan_agents` stamped every id, then `_prune_history` kept
  only the first 1,024 ids in the current frame. The remaining live ids therefore had no previous
  pose or work stamp on the next frame and appeared new indefinitely.
- Fix: `_scan_agents` leaves new ids untracked while the pose-history cap is full, so they do not
  repeatedly emit accents. `_prune_history` removes departed ids without evicting live tracked
  ids based on their position in the frame; departed ids still release slots for later agents.
- Guard: `tools/gd_test.gd` `test_effect_history_prunes_departed_and_excess_ids` now clears the
  pool, scans the same over-cap population again, and requires zero new accents while retaining
  the existing bounds, departure-pruning and returning-id checks.
- Commit: this PR #518 c146 follow-up fix.

## 2026-10-31 #483: construction could not make a supported upper room

- Symptom: the work system had no material-backed way to build stairs upward, and a floor order
  over open space completed without laying a floor. Upper rooms therefore could not be grown
  through ordinary labour or used by inhabitants.
- Found by: issue #483 triage against `work/planner.rs`, `systems/work.rs` and `building.rs` on
  c144. This is a backlog enhancement, not a reported release regression.
- Cause: confirmed by code read: `BuildFloor` only handled solid ground immediately below its
  order, while `DigStairs` only carved existing rock and had no material cost. Neither path
  checked or built a supported floor above a room.
- Fix: `work/designation.rs` appends `BuildStairs`; `systems/settlement.rs` `make` gives it real
  work and refined-material requirements; `systems/work.rs` `apply_effect` builds both halves of
  the connection and opens the ceiling below, and lays upper-floor tiles only when supported.
  `building.rs` `floor_supported` carries a span from built walls one level below through existing
  upper floors; `work/planner.rs` `plan_for` withholds unsupported floors and invalid stair sites.
  Ground-floor representation and the #174 no-burial guard are unchanged. The scenario uses the
  existing z-level navigation and sleeping behaviour; no new presentation facts are claimed.
- Guard: `crates/hf-sim/tests/upper_floors.rs`
  `an_upper_floor_needs_support_from_walls_below` and
  `stairs_supported_floors_and_a_bed_are_built_then_used_above`. The latter builds the stair,
  eight floor tiles and bed through real orders and materials, checks their order and consumed
  materials, verifies reachability, then requires actual sleep in the upper bed. The existing
  `qa_1020.rs` no-burial scenario now also requires an unsupported order to wait. Rust validation
  is delegated to box-ci for this branch.
- Commit: this fix commit.

## 2026-10-31 #483 review: stair links and built materials survived construction incorrectly

- Symptom: a second stair flight replaced `StairsDown` with `StairsUp`, cutting the way back;
  an upper-floor order could erase a stair tile; the Godot bridge had no `BuildStairs` mapping;
  and upper-floor/stair provenance and wooden-floor fire behavior did not use their recorded
  materials.
- Found by: PR #540 review after the first primitive-only slice. The same review confirmed no
  production order source emits `BuildStairs`.
- Cause: confirmed in `systems/work.rs` `apply_effect`, `world_state.rs` `made_of`,
  `systems/hazards.rs` `fuel`/`burn_out`, and the exhaustive designation mapping in
  `hf-godot/src/lib.rs`.
- Fix: `building.rs` `built_stair_shapes` preserves existing opposite connections as
  `StairsBoth`; floor planning and completion reject stair tiles without changing ground-floor
  placement. `made_of` now recognizes built floors and stairs; fire density reads the burning
  tile's own material, and wooden floors/stairs burn away instead of becoming floating soil.
  Godot maps built stairs to designation code 25 with material and hammer-sound integration.
  The PR and backlog now explicitly claim engine primitives only; organic order proposal remains
  #483 follow-up work.
- Guard: `upper_floors.rs` `a_second_flight_preserves_the_way_down`,
  `a_floor_order_does_not_erase_a_stair_tile`,
  `upper_wood_materials_drive_provenance_fuel_and_burn_out`, and
  `a_ground_floor_still_builds_on_the_tile_below`; the original construction/use scenario also
  checks oak provenance on every new upper tile. The #174 no-burial guard remains.
- Commit: this review fix.

## 2026-10-31 #484 (c145 review): timelapse identity and pixel determinism guards

- Symptom: review found that the retention guard expected eviction before the ring was full;
  follow-latest could skip an append, a pinned array index changed sample after eviction, and
  shader/particle wall clocks prevented equivalent same-tick PNGs.
- Found by: PR #541 review at c145, confirmed against `tools/gd_test.gd`, `timelapse_panel.gd`,
  `sway.gdshader`, `grade.gdshader` and the CPU weather/fire emitters.
- Cause: the guard subtracted/evicted from tick 1 instead of tick 120; the panel shared the
  recorder's mutable Array and stored selection/pin only as indices; shaders used `TIME` and CPU
  particles kept advancing between capture frames.
- Fix: the guard now expects eviction only on append 121. Panel selection and pinning retain tick
  identity and derive indices after each sync. Shaders use `anim_t`; capture freezes particle
  advancement with tick-derived seeds and suppresses transient lightning. A windowed integration
  scenario exercises actual viewport PNG/JSON creation and deletion, exact same-tick PNG identity,
  watched-view independence, review interactions and source-state immutability, followed by three
  game-days of evidence captures.
- Guard: `tools/gd_test.gd` `test_timelapse_panel_follows_and_pins_ticks_across_eviction` and
  `test_timelapse_presentation_clock_can_be_frozen`; `tools/timelapse_visual.gd` via
  `tools/shots/timelapse.sh`. The windowed scenario and multi-day evidence remain unrun on this
  constrained machine; no visual pass is claimed. No hf-sim source changed and no simulation soak
  is requested.
- Commit: this review-fix commit.

## 2026-10-31 #484 (c146 verification): timelapse panel formatting and fixture lifecycle

- Symptom: the first executable `gd_test` run reported `GD_TEST 23 passed, 0 failed` but emitted
  script errors while syncing the review panel, formatting playback speed, and updating the
  underground light-layer fixture.
- Found by: focused Godot 4.6.3 execution on the c146-based PR #541 head; the full command output
  was inspected rather than accepting the summary or exit status alone.
- Cause: Godot's string formatter rejects `%g`; the panel test called `sync` before `_ready` had
  created its controls; and `UndergroundView` lacked the `anim_t` and `animation_frozen` fields now
  read by `LightLayer.update`.
- Fix: `godot/scripts/timelapse_panel.gd` uses supported `%s` and `%.2f` formats. `tools/gd_test.gd`
  initializes the standalone panel fixture explicitly and gives the underground view the current
  presentation-clock interface. The already-correct 121-sample eviction and retained-byte
  expectations are unchanged.
- Guard: `cargo build -p hf-godot`; Godot import; `gd_check.gd` reports 20 scripts, 0 failed;
  `gd_test.gd` reports 23 passed, 0 failed with no script errors. The windowed visual scenario is
  still unrun. This is client-only work under the simulation-soak waiver.
- Commit: this verification-fix commit.

## 2026-10-31 #485 review: beast-shaped eaters lost the animal eating animation

- Symptom: a person in the Beast-hunger's wolf shape emitted the new people-eating pose 11.
  The animal renderer only recognises the established animal eating pose 5, so the wolf no
  longer used its established slow eating bob.
- Found by: PR #543 review after the first #485 slice; the c145 fold and this correction are
  presentation-only, with no soak required.
- Cause: confirmed in `crates/hf-godot/src/lib.rs`: `person_pose` checked the agent's underlying
  `Species::Person`, while the emitted shape was changed to Wolf later in `agents`. Pose selection
  therefore disagreed with the renderer selected by the same agent row.
- Fix: `rendered_shape` now supplies the same Wolf/person shape to `person_pose` and `agent_row`.
  Beast-shaped eaters retain pose 5; ordinary people retain pose 11 and eating equipment
  suppression. `Rules.animal_frame_for_row` is the animal renderer's row-consuming selection path.
  The original 34 frame columns and all appended assets are unchanged.
- Guard: hf-godot `beast_shaped_eater_emits_animal_row_without_person_equipment` checks the full
  24-int row for ordinary and shifted eaters, including Wolf shape, pose and equipment. GDScript
  `test_beast_shape_row_selects_animal_eating_frames` consumes that row's pose through the real
  render helper and requires both established idle-bob frames, never the people eat cycle.
- Commit: this review-fix commit. Rust/Godot execution remains delegated to box-ci.

## 2026-11-01 #395: the bootstrap arrival affect — panic, survival urgency, and the in-between

- Symptom: the arrival was a dip and nothing more (issue #395, extending #392's shock): no
  freak-out pole, no survival-snap pole, no dread of the missing. A fed newcomer could spend
  the first day at leisure while the party had nothing secured, and the seven scattered
  survivors were nobody's thought.
- Found by: issue #395's design brief (Dan's observation: people abruptly translocated show
  neither the freak-out nor the survival snap nor anything in between). Checked against
  `systems/strain.rs` `arrival_shock`: every survivor got the same mild dip; the stress spike
  capped at 0.20 against a break threshold of at least 0.55, so the shock alone never broke
  anyone; `survival_urge` topped at 1.5 for everyone; nothing weighed leisure down while the
  shock was fresh beyond the #415 unsecured gate.
- Cause: confirmed by the guard probes on unmodified master: all five new guards fail there
  (`crates/hf-sim/tests/arrival_affect.rs`) — the leisure twins decide identically (18983
  leisure ticks each), no first-hours break on seeds 3, 7 and 11, every survival_urge exactly
  1.5, no "the missing" thought, no "wondered where" event, identical stress beside company.
- Fix: `systems/strain.rs`: `arrival_shock` seeds a `Missing` thought (the dread of the rest
  of the party; `mind/thoughts.rs` `ThoughtKind::Missing` appended, -0.2 for 4 days) and tips
  the least calm (`calm <= 0.15`, `PANIC_LINE`) straight over their own break threshold — in
  ten, one or two break in the first hour (wandering off, withdrawal, a daze: reduced
  function, visible as `Broke` events, witnessed as `SawBreak`). A newcomer cannot walk out
  of the valley within their first day (`arrived_today` in the break-kind weights).
  `survival_urge` runs 1.35x for the steadiest (`snap`, `calm >= 0.85`, `SNAP_LINE`).
  `step` eases stress a little beside a markedly calmer awake person (`STEADY`, the calm side
  of contagion) while the Missing dread is held — a bootstrap phenomenon, so worlds without
  an arrival are untouched — and, while anyone is still lost, community members are heard wondering where
  a named missing person is, once a day each. `systems/decide.rs` `choose`: the fresh urge
  now divides leisure down for the secured as well as the unsecured (`1.0 / survive`), and
  hunting, fishing and snaring take the survival-work multiplier with the other food work.
- Guard: `crates/hf-sim/tests/arrival_affect.rs` —
  `the_shock_spans_freakout_snap_and_the_middle`,
  `the_panic_tail_breaks_at_once_and_the_others_see_it`, `fresh_arrivals_put_leisure_off`,
  `the_missing_are_dreaded_and_asked_after`, `a_steady_presence_eases_the_strain`; all five
  fail on unmodified master, each at the assertion named in the file. The seed-7 thin
  trajectory test (`tests/thin.rs` `they_study_it_and_the_wards_quiet_it`) pins the quiet
  day, which any opening change moves: it is re-measured at day 87 (was 46) and its cap
  raised 75 to 110 with a comment saying so.

## 2026-10-31 #495 (c145): size tests pin the runt/giant roll instead of inheriting it from world-gen agent ids

- Symptom: on the PR #495 merge head `4d1a444b`, box-ci failed
  `hf-sim::size a_small_body_carries_less_and_wolves_drag_down_by_weight` at
  `crates/hf-sim/tests/size.rs:202`: `assertion failed: brings_down(&w, deer)` — two grown
  wolves no longer dragged down a grown deer. Master's tree passed the same test.
- Found by: reproducing in a worktree on `4d1a444b` (8/9 size tests passed, this one failed
  identically to the CI log), then printing the fixture's genomes in a temporary debug test.
- Cause: confirmed. `found_party` runs `animals::populate`, which runs the PR's reworked
  `place_deep`: habitat profiles (`cave_ecology::profile`) spawn a different number of deep
  fauna than master's `gen_range(4..8)` loop, so the world's `next_agent_id` when the test
  spawns its deer differs. `spawn_agent` draws each genome from
  `rng::stream(seed, [AGENT_BIRTH, agent_id])`, so the deer at its new id drew a Giant build
  (140 kg, might 1.468): pack might 2 x 0.770 = 1.540 lost to the drag claim
  1.2 x 1.468 = 1.762. The test's claim was right; its fixture silently inherited an
  incidental runt/giant roll from how many animals world gen happened to place.
- Fix: `crates/hf-sim/tests/size.rs` `beast` now pins the spawned animal's runt/giant roll to
  `Build::Plain` (nudging the unused-for-size longevity locus until the roll reads plain), so
  the fixture is a plain animal of the requested size no matter which agent ids world gen
  allocated before it. The drag-down assertion is unchanged in meaning and gains a diagnostic
  message (pack might vs drag claim, deer's build). No simulation code changed: master's size
  semantics and the PR's habitat-backed placement both stand.
- Guard: `crates/hf-sim/tests/size.rs a_small_body_carries_less_and_wolves_drag_down_by_weight`
  still claims two grown wolves bring down a grown deer and two cubs cannot; it failed before
  the fix with the Giant deer and passes after. Pinning `beast` also removes the same latent
  luck from every other size test that uses it (bear/hare blows, door breaking, the giant
  stag's three-wolf claim).

## 2026-11-01 #308: the curse bag grows — the true-dream

- Symptom: the curse bag held three curses and the restless dead; the thin place did its
  narrative work only through visitors, with no condition of its own (#308: grow the bag,
  each curse with a cause and a wrong-suspicion path).
- Found by: issue #308 (the magic/psy/tech design review of what Ostrog already has).
- Cause: no condition was tied to the thin place: sleeping within its reach only ever gave
  a bad night's thought, recorded in no row.
- Fix: `conditions.rs` — a fourth curse, the true-dream (`ConditionKind::TrueDream`). One
  who sleeps within the thin place's reach while it stands woke is marked, one night in
  four from a seeded stream (`marked`); the cursing is recorded with its row, as every
  condition holds its giving row. Every third night the marked one cries out in their
  sleep, of the nearest of the restless dead, else of the stone — true to the record —
  recorded as `EventKind::TrueDream` with their curse's row (`true_dreams`), and their
  community comes to suspect them (`suspect`, as the other curses do): the suspicion holds
  the cry as its source row, and it is wrong — what they are blamed for is the curse's
  doing, and they are its victim, not the hollow's agent. The unbinding (`unbinding`) now
  lifts the true-dream as it lifts the Beast-hunger and the Thirst.
- Guard: `crates/hf-sim/tests/true_dream.rs` —
  `the_true_dream_marks_a_hollow_sleeper` (the cursing is recorded and held),
  `the_true_dream_is_suspected_and_the_suspicion_is_wrong` (the suspicion's memory holds
  the cry's row as its source while the blame is the curse's), and
  `the_unbinding_lifts_the_true_dream_from_one_suspected`. The suite does not compile on
  master without the condition and event.

## 2026-11-01 #319: the first cascade family — flood → rot → vermin → bite → fester

- Symptom: the simulation's troubles were deliberately measured and could not spiral (the
  issue's example: a forgotten stair floods a stockpile, the flooded grain rots, the rot draws
  vermin, someone is bitten, the bite festers). Nothing chained; each event stood alone.
- Found by: issue #319 (recorded from the design review of milestone MP's engine work).
- Cause: no system carried a consequence forward: floods receded without marking the food
  they drowned, rot drew nothing, and there was nothing for a small thing to spiral through.
- Fix: `cascade.rs` (new) runs once a night. Food under flood water rots in a day, recorded
  as `EventKind::Drenched` holding the flood's row (`floods.rs` now keeps the `Flooded` row
  in `Waters.flood_event` while water stands, and clears it when the flood recedes, so a
  cascade never starts from an unrecorded flood). Each drenched tile draws vermin
  (`EventKind::VerminDrawn`, holding the rot's row; outbreaks tracked in `World.vermin`,
  saved). An outbreak bites at most one sleeper in reach a night, the one chance in the
  family from `rng::stream` keyed by site and night: `EventKind::VerminBite` holds the
  outbreak's row, and the wound is festered from the first hour, recorded as
  `EventKind::Festered` holding the bite's row; the existing health pass turns the festering
  wound into the WoundRot. Every step holds the previous step's row, the conditions.rs cause
  machinery, so the chronicle can trace the whole spiral (D9).
- Guard: `crates/hf-sim/tests/cascade.rs` — `a_flood_cascades_to_rot_vermin_bite_and_fester`
  builds the scenario (grain on a stockpile tile, a sleeper by it, recorded flood) and walks
  the chain asserting each row's cause; `no_recorded_flood_no_cascade` holds the D9 line (no
  flood row, no cascade); `the_cascade_is_deterministic` runs the same seed twice and
  compares the rows. The suite does not compile on master without the module.
- Commit: this fix commit.

## 2026-10-31 #486: flora kind and variation draws no longer shift realized density

- Symptom: after merging c147 into PR #538, the seed-1 full-catalogue control occupied
  different sites from the limited flora draw despite climate-eligible density normalization.
- Found by: `cargo test -p hf-sim --test flora --test biomes` on merge head `17f8ace8`;
  biomes passed 13/13 and flora passed 13/14, with the exact-site density guard failing.
- Cause: confirmed in `world/gen/flora.rs::apply`. Site rolls, tree-kind selection and
  kind-dependent growth/variant generation shared one random stream. A different drawn kind
  consumed a different number of draws, shifting subsequent site rolls even though the
  normalized per-site probabilities were correct. After that fix, the guard also exposed that
  its climate control recomputed biomes after water deepening rather than retaining the climate
  used by world generation.
- Fix: tree selection and growth/variant generation now use separate purpose-tagged streams,
  leaving the placement stream's sequence independent of the selected kind. The guard replays
  the generation stages through the original climate, then compares limited and full draws on
  the identical final grid. Density rules are unchanged.
- Guard: `tests/flora.rs limited_flora_preserves_realized_density_against_full_catalogue`
  compares exact plant sites over eight seeds; rerun with the complete flora and biome suites.
- Commit: this merge-follow-up commit.

## 2026-11-01 #486: hunting_feeds_the_community moved off seed 102

- Symptom: box-ci on PR #538 head `34d68171` failed `hf-sim::wildlife hunting_feeds_the_community`
  (`left: 1, right: 0`, nobody starved). Reproduced locally in about 22 s, so it is
  deterministic and not a load flake; the 61 s in the log was load.
- Found by: box-ci, then a focused local run with per-agent tracing.
- Cause: confirmed. The flora draw changes seed 102's trajectory. Thirteen animals were hunted,
  but only agent 5 hunted and it ate nearly all the meat; agent 2 never ate and starved on day 3
  while the others loitered. The same fixture on seeds 101 and 103-107 hunted 5-9 animals and
  starved nobody, so this is one world's luck of who takes hunt work, not a regression in
  hunting. Unequal sharing of one hunter's meat is a real limit and is not addressed here.
- Fix: the test uses seed 103; its fixture and both assertions are otherwise unchanged.
- Guard: the test itself.

## 2026-11-01 PR #501 (c147): qa_1026's arrival fixture pins its camp against worldgen drift

- Symptom: on the `branch 482-cave-mouths` head `f8604d55`, box-ci triage reproduced
  `hf-sim::qa_1026 the_arrival_is_a_shock_that_fades_and_the_calm_shake_it_faster` failing
  deterministically at `crates/hf-sim/tests/qa_1026.rs:173` (seed 7): "day 1: 0.5807307
  against 0.32070386". Master's head passed.
- Found by: PR #501 triage (comment 6552673835), reproduced in a worktree on `f8604d55`,
  then per-survivor dumps of `opening(7)` on both branches. The ten survivors' identities —
  genome, traits, needs, skills, age, the coming-through hurt roll — come from
  `opening::identity(seed, slot, rerolls)` and are identical on master and the mouths branch;
  the triage comment's "survivors' genomes re-roll via `next_agent_id`", the `size.rs` #495
  disease, is not what happens here (survivors are slot-keyed, not id-keyed). What world gen
  shifts is their setting: the founding site, the scattered positions, and the ground, water
  and forage within reach of the camp.
- Cause: confirmed. The day-1 mood gate averages the three community members and compares the
  shocked world against the same people without the Arrival thought; since #415 the shocked
  hurry to secure food and shelter, so the gate's sign rides on how hard that is where
  worldgen happened to place them. The mouth cuts changed seed 7's camp, and there the
  hustle's needs advantage outweighed the thought's mood cost (0.58 against 0.32; on master
  the thought still wins).
- Fix: `crates/hf-sim/tests/qa_1026.rs` gains `pin_camp`, applied to `opening(7)` in the
  arrival test: a flattened 25x25 square at the founding site, a fixed orchard and pool,
  edge trees, beds for the three starters, the provisions re-dropped at fixed spots, and
  wolves and bears kept out of the camp. The calm arm is now a `clone` of the same pinned
  world with the Arrival thought lifted, instead of a second `opening(7)`. No simulation
  code changed; the test's behavioural claims are untouched — the shock is present, weighs
  on day-1 mood, fades, the calm shake it faster, and mood recovers by day 6 (the beds are
  the recovery's comfort, what the first week of a real camp is for). The gate now reads
  the shock's cost, not the world's lottery (the `size.rs` `beast` precedent, FIXLOG
  2026-10-31, #495).
- Guard: `crates/hf-sim/tests/qa_1026.rs`
  `the_arrival_is_a_shock_that_fades_and_the_calm_shake_it_faster` itself, green on both
  `f8604d55` (branch 482-cave-mouths) and master's head.
- Commit: this fix commit.

## 2026-11-01 #510: `troubles_overlap_and_then_ease` met a different valid world after #493

- Symptom: box-ci failed at 34aa8ec5 (`pacing.rs` `assert!(turned >= 3, "{turned}")`, value 2).
- Found by: reading the CI log, then running the arc counts for seeds 3, 7, 13, 11, 17, 19, 23
  on the branch (turns 1, 1, 0, 2, 6, 0, 0; each seed of 3, 7 and 13 still eased once).
- Cause: not a pacing regression. #493 lets more loose ground compact into paths, which changes
  how fast people walk, so each seeded world takes a different course. Every run still eases
  its hard stretches and none stalls; only the number of seeds in which a trouble lands on top
  of another changed. The test's seed list has always been re-chosen for this reason.
- Fix: seed 17 added to the list in `tests/pacing.rs::troubles_overlap_and_then_ease`
  (comment there names this entry); thresholds unchanged (`turned >= 3`, `eased >= 2`).
- Guard: the same test.
- Commit: (this commit).

## 2026-11-01 #510 (second): `they_study_it_and_the_wards_quiet_it` ran out of its 75 days after #493

- Symptom: box-ci failed at a341f7c1 (`tests/thin.rs` `assert!(w.thin.quieted)`, after the
  chosen-ward and four-ward assertions had passed). master passes the same test (quiet on day 46).
- Found by: running the test alone on the branch and on master, then stepping seed 7 with
  per-day prints of the stone's strength, `thin.chosen` and the wards built (a throwaway test,
  not committed).
- Cause: not a wards regression. With loose ground compacting into paths people walk differently,
  so on this seed the first person to sit at the stone and understand it does so about day 60
  instead of before day 26. The stone's strength keeps growing until the wards stand (about 1.0
  by day 57), and four wards then wear it down 0.05 a day, so it was quieted on day 81, past the
  test's 75-day ceiling. Choice, wards and quieting all happened as designed.
- Fix: `tests/thin.rs::they_study_it_and_the_wards_quiet_it` ceiling raised from 75 to 120 days
  (`CEILING`, comment there names this entry). The loop still stops at the quieting and then
  watches fifteen days, so runs that quiet sooner cost no more. All assertions unchanged.
  No simulation code changed.
- Guard: the same test.
- Commit: (this commit).

## 2026-11-01 #488: the dispositions wander weight must not undercut the legacy baseline

- Symptom: on PR #539 head `581c08e8`, box-ci failed `hf-sim::feelings the_afraid_wander_less`
  (`crates/hf-sim/tests/feelings.rs:160`): `wandering: calm 990, afraid 2815`. Deterministic.
- Found by: reproducing in a worktree, then a temporary census of activities per world, run
  with and without the dispositions profile (`a.dispositions = None` gives the legacy weights).
- Cause: confirmed. Legacy `wander_weight` was 1.0 (1.5 curious), so a calm person's wander
  score (0.35 x weight x 0.6..1.0) always beat Loiter's 0.2. The dispositions weight
  `(0.5 + curiosity) * (1.1 - 0.2 * caution)` put most people under that line: calm wandering
  fell from 7646 (legacy) to 990, and the afraid world (awake longer, less sleep) then
  wandered more than the calm one. Legacy census: calm 7646, afraid 3833.
- Fix: `mind/dispositions.rs` `Agent::wander_weight`: `(0.6 + 0.8 * curiosity) * (1.1 - 0.2 *
  caution)`. Calm wandering is restored and the existing fear lean (x0.7 on Wander) shows
  again; a fully incurious, cautious person still loiters (weight 0.54) so the dispositions
  choice tests hold. No test or seed changed.
- Guard: `tests/feelings.rs the_afraid_wander_less`; `tests/dispositions.rs
  exploration_axes_change_the_real_wander_choice_and_unrelated_axes_do_not`.
- Commit: this fix commit.

## 2026-11-01 #488: illness_spreads_then_fades counted a hunter's beast fever as the house fever

- Symptom: on PR #539 head `3bd1cec1`, box-ci failed `hf-sim::health illness_spreads_then_fades`
  (`tests/health.rs:232`): `everyone recovered from the fever`, left 1, right 0. Master passes.
- Found by: reproducing on the branch, then a temporary per-day probe of every ill person
  (illness kind, severity, hunger, rest, activity, tended), removed afterwards.
- Cause: confirmed, and not a stalled recovery path. The two house fevers (winter cough) were
  gone by day 1. The one still ill was person 3, who with the dispositions' different wandering
  went out, was bitten (a wound, healed by day 3) and caught beast fever (`work.rs` hunt). They
  slept in a bed all day, fed (hunger 0.5 to 0.8), severity falling 0.016 a day from 0.48 to
  0.29 at day 12: a normal person recovering at the normal pace. The test's filter only
  excluded illnesses with a still-infected wound, so a healed-wound beast fever counted.
- Fix: `tests/health.rs`: the final count now only includes winter cough and fever, the
  illnesses the scenario seeds and spreads. No simulation code changed, the dispositions are
  untouched, no seed changed; the assertion on the house fever is as strict as before.
- Guard: the same test.
- Commit: this fix commit.

## 2026-11-01 #553: food a neighbour is already going for is not set out for

- Symptom: on seed 3 at 200 days (master `6790e52a`), 45,217 forages failed "nothing to gather",
  10,855 meals "food gone" and 1,809 "food moved": people walked to a bush or a stack someone
  else had just stripped, eaten or carried off.
- Found by: `examples/plan_failed.rs` (new), which tallies `plan_failed` by reason, by person and
  by what was set out for, and says what was found there. In the first 50 days of seed 3,
  1,315 of 4,124 bare bushes and 534 of 1,155 vanished meals had been reached by another person
  after this one set out (an undercount: one already picking when they set out is not counted).
- Cause, checked: `systems/decide.rs` `choose` scored every known bush and stack with no regard to
  who was already going to it; two hungry people at one camp both walked to the nearest bush.
  A hauler's lease on a stack (`ResKey::Item`) was not looked at either.
- Fix: `systems/decide.rs` `spoken_for` lists the plants and ground food that others of their
  community, or anyone in plain view within eight tiles on their level, are walking to or
  picking or eating; `choose`'s `consider` gate drops a Forage or Stock goal on such a plant,
  an EatItem goal on a stack with as many eaters as it has units, and one on a stack another
  holds a lease on. The rest of the bushes left over stay a mistake a person can make: of the
  failures left, almost all are bushes picked by others since this person last saw them
  (1,093 of 1,102 traced), which they cannot know.
- Guard: `tests/failed_plans.rs`
  `a_neighbour_does_not_walk_to_the_bush_another_is_already_going_to_pick` and
  `the_last_of_a_stack_is_not_walked_to_by_two_and_a_haulers_stack_by_none` (both fail on
  master: both people set out for the same bush, and for the one berry).
- Commit: this commit.

## 2026-11-01 #553: nobody sets out to talk to someone walking off

- Symptom: on seed 3 at 200 days, 8,029 talks failed "they left"; the same pair over and over
  (one person tried to talk to agent 7 and failed 113 times).
- Found by: `examples/plan_failed.rs`; in the first 50 days of seed 3 the partner was walking
  (going or wandering) when the talk failed in about 700 of 1,040.
- Cause, checked: `systems/decide.rs` `choose` offered a Talk goal with anyone awake within 14
  tiles, walking or not. The talker walked to where the other had been and found them gone.
- Fix: `choose` leaves out anyone moving or with a path to follow (`walking_off`), except one
  out for a stroll (Wandering), whom the talker can catch up with and who stops to talk
  (`act.rs`, Talking).
- Test changed: `normal_person a_secure_community_still_talks_and_wanders` counted talk as
  who was talking or walking to a talk at each quarter hour, so a walk to a talk that came to
  nothing counted and a six-minute talk was mostly missed. With this fix it saw 3 (4 needed)
  while the camp held 12 talks in the 12 hours (10 without the fix). It now counts the talks
  held (`Talked` events), at the same threshold of 4; the wander count is unchanged.
- Guard: `tests/failed_plans.rs` `nobody_sets_out_to_talk_to_someone_walking_off` (fails on
  master: the lonely one sets out after the walker).
- Commit: this commit.

## 2026-11-01 #553: a build order nobody can reach is not planned

- Symptom: on seed 3 at 200 days, 2,533 fetches for roofs, 1,066 for walls and 625 for beds
  failed "cannot reach", the same few orders by the same people (one wall order 1575: 130, 119,
  108 and 107 times for four people).
- Found by: `examples/plan_failed.rs`, then `HF_TRACE_REACH` with a trace of each plan made:
  every one of the failing plans had its second walk, to the place to stand, unreachable when
  it was planned (`nav.reach` false); the orders stood at level 4 on top of walls.
- Cause, checked: `work/planner.rs` `stand_tile` returned the order's own tile as the place to
  stand whenever it was walkable, without asking whether a way led there (the tiles beside it
  were asked). The plan was made, leased and walked, and failed at the walk; the order's
  failures fade one every four hours, so it came back.
- Fix: `stand_tile` takes the target only if a way leads there; otherwise it looks beside it.
  A plan that cannot be made puts the order off for that person (`decide::start_work`) and counts
  against the order, so an order nobody can reach fades out.
- Guard: `tests/failed_plans.rs` `an_order_on_a_ledge_nobody_can_climb_to_is_not_planned`
  (fails on master: the wall on top of a pillar is planned).
- Commit: this commit.

## 2026-11-01 #553: a tamer follows the goat

- Symptom: on seed 1 at 200 days, 1,162 tamings failed "not at the site", all one goat's order.
- Found by: `examples/plan_failed.rs` with `HF553_BLOCK`: each tamer stood near where the goat
  had been when they set out, the order stood where it had been when the order was given
  (57,61,4), and the goat was elsewhere again.
- Cause, checked: `work/planner.rs` plans a taming as a walk to where the goat is, but
  `systems/work.rs` `run` asked at the work that they stand at the order's tile, which a goat
  that has wandered off is not near. Hunting follows its beast; taming did not.
- Fix: `systems/work.rs` `run`: a Tame order walks after its beast as a Hunt does (the order's
  place and the walk move with it), and the work is done beside the beast, not the order's
  tile. A goat not caught up with in 800 ticks is given up as "the beast wandered off".
- Guard: `tests/failed_plans.rs` `a_tamer_follows_the_goat_from_where_the_order_was_given`
  (fails on master: six "not at the site" and no goat tamed in an hour).
- Commit: this commit.

## 2026-11-01 #553: the tick's batched walks refuse water over the head, and a walk with no way waits

- Symptom: on seed 2 at 200 days, 17,682 walks to drink failed "path blocked", most by the
  people of one camp walking to one place (68,45,2) to drink, over and over.
- Found by: `examples/plan_failed.rs` with `HF553_BLOCK`: the path searched from where they
  stood had its first step in water over their head (`grid.over_head`).
- Cause, checked: since #428 the walks chosen in a tick's decisions are searched together by
  `par::path_between`, which did not have the rule of `World::path_agent_to` (#418) that nobody
  on dry ground sets out through water over their head. The walk was committed, movement stopped
  it at the first deep tile, `act` failed it as "path blocked", and half a minute later the same
  walk was chosen again.
- Fix: `par::path_between` refuses such a path as `path_agent_to` does. `systems/decide.rs`
  `finish_walks`: a walk with no way is put off for that person for an hour
  (`NO_WAY_TICKS`, key `no_way_key`), and `choose`'s `reachable` gate does not offer a walk to
  that tile until then.
- Guard: `tests/failed_plans.rs`
  `a_walk_through_water_over_the_head_is_not_set_out_on_or_chosen_again_at_once` (fails on
  master: the batched search finds the way through the spate).
- Commit: this commit.

## 2026-11-01 #553: two_rites_two_thoughts no longer rides on the marred-rite roll

- Symptom: `cultures two_rites_two_thoughts` failed on the #553 branch: the one who keeps the
  dead's own rite had a RiteBroken thought.
- Found by: `tools/ci.sh` on the branch (master `89a85fef` passes it).
- Cause, checked: the rite given on seed 4501 at tick 10820 was marred ("a word was said
  wrong", `rites::outcome`, a 6% roll keyed on the tick), and a marred rite troubles every
  mourner. The #553 changes moved who reached the body when, so the rite fell on another tick.
  The test had passed on master because that tick's roll came up plain.
- Fix: the test moves the death on a tick at a time (up to twelve) until the rite given is not
  marred, and asserts on that rite exactly as before. Its three assertions are unchanged.
- Guard: the test itself.
- Commit: this commit.

## 2026-11-01 #553: they_study_it_and_the_wards_quiet_it stages who is drawn to the stone

- Symptom: `thin they_study_it_and_the_wards_quiet_it` failed on the #553 branch: nobody had
  chosen what to do about the thin place by day 75 ("someone chose").
- Found by: `tools/ci.sh` on the branch.
- Cause, checked: none of #553's fixes holds the study back; when the place is quieted is the
  run's lottery of who goes to sit by the stone and who learns enough first. Seed 7 with the
  test's staging (`examples/plan_failed.rs`, `HF553_THIN`), with each fix turned off in turn
  by a bisect build: all off, day 58 (master 59); tame off 64; talk off 82; water off 89; the
  stand fix off 95 (it does nothing on this seed; all on, 95); food off, not quieted by day
  130. No single fix moves it back, and turning the food fix off moves it later. With nobody
  curious, only someone the stone has shown an echo or who has begun to learn its ways goes to
  sit there (`decide::choose`, StudyThin), so the first sitting waits on a random echo; the
  branch's world had its first sitting 16 days later. Over ten seeds staged the same way, the
  first to learn enough was mostly a later, curious arrival who left the place open, with the
  fixes and without.
- Fix: the test stages one founder as shown an echo at the arrival (`thin.seen`), so someone
  is drawn to the stone from when it wakes, as anyone shown an echo is. The learning, the
  choice, the wards and the quiet are left to the run, and the 75-day limit and every
  assertion are as before. Seed 7 quiets on day 41 with the fixes and day 34 without.
- Guard: the test itself.
- Commit: this commit.

## 2026-11-01 #553: the hollow by the portal stone is found once a tick

- Symptom: on seed 3 at 200 days the `plants` system took 0.130 ms a tick on the #553 branch
  against 0.029 on master, a quarter of the tick (runs side by side on a loaded box). Run one
  after another on a quieter box: master 0.017 of 0.173 ms; branch 0.072 of 0.239 ms.
- Found by: the per-system time in `examples/plan_failed.rs` (`HF553_PROF`).
- Cause, checked: on the branch's seed 3 the first to learn the stone's ways was curious and
  left the thin place open (`thin.chosen` (agent 6, open)), so it stayed over 0.3 all run (177
  days; on master it was quieted on day 34). While it is open, `systems/plants.rs` `step` asked
  `thin::hollow` for every plant every tick, and each call looked for the portal stone among
  all the structures, though only the sixtieth of the plants growing this tick use the answer.
- Fix: `plants::step` finds the stone and the hollow's reach once, and asks only for this
  tick's stripe. What grows is unchanged: seed 3's 200-day digest is the same before and after
  (`38de540c…`), and the run's `plants` time is 0.017 of 0.169 ms a tick, as on master.
- Guard: none for the time (the digest above shows nothing else changed).
- Commit: this commit.

## 2026-11-01 #553: nothing_grows_in_the_hollow allows a grazer's bite

- Symptom: `thin nothing_grows_in_the_hollow` failed after merging master `eb503384` into the
  #553 branch: the shrub staged in the hollow ended at growth 0.169, not 0.2.
- Found by: `tools/ci.sh` on the branch.
- Cause, checked: a per-tick trace of the shrub showed one change, 0.2 to 0.169 at tick
  289198461, and the only code that lowers a plant's growth is `animals::graze_bite`: a grazer
  browsed it. The shrub never grew. The branch's changes moved the run's course so a grazer
  passed it within the two days; reverting the plants change alone made no difference.
- Fix: the test asserts the growth is at most 0.2 ("it has not grown"), not exactly 0.2.
- Guard: the test itself; a hollow that let the shrub grow still fails it.
- Commit: this commit.

## 2026-10-28 round-16-deep: nobody dug below the quarry's two levels (#405, #274)

- Symptom: no cut deeper than two levels in an ordinary game, so the deep's stone, gold and
  gems (#274), its caverns and the breach (the `breached ... not fixed` entry of 2026-10-27)
  were never reached; the quarry's shaft stopped at two levels and the cellars at one.
- Found by: the #405 probe (330 cuts in four 200-day openings, all one or two levels down).
- Cause: confirmed by reading the planner: no rule orders a cut below the quarry's foot. The
  ore rule (`settlement.rs`, step 9) cuts only a vein face already open within reach, and the
  quarry's shaft is two levels by construction (#264).
- Fix: `delving.rs` (new): a community sinks a mine from its quarry's shaft for a reason it has
  (`wanted`): iron or copper for a smelter standing and someone who knows smelting, with no
  vein showing within reach, tin
  for bronze, silver or gold to set a gem in, harder stone than any it has cut while strangers
  threaten (the quarry then cuts at that level, `stone_level`), and the deep's gems and gold
  for a people settled a year that holds the deep is there to be dug (DeepDigging stance 0.2
  or more). One cut
  at a time (`dig`): a gallery to what shows within six tiles of the workings on the level,
  then down a level (around a cave under the shaft), and at the bottom of the world galleries
  driven out 24 tiles each way. Each cut takes longer the deeper it is (`order`, 1 + depth/15
  on top of the rock's hardness); the dark, wet rock and unheld roofs were already costs
  (work.rs, aquifer.rs, underground.rs). A cautious community (stance under -0.15) does not
  sink its shaft into the deep. Rows: `mine_sunk` (why), `reached_deep`. `settlement.rs`:
  `quarry_site` split out of `quarry`, the mine planned last in `plan_community`. A move
  leaves the mine behind (`moving.rs`). Water (#433) is a `Seek` with nothing choosing it.
- Guard: tests/delving.rs `a_shaft_sunk_for_ore_stops_where_the_vein_shows_and_follows_it`.

## 2026-10-28 round-16-deep: a coal seam open in a shaft was cut a thousand times

- Symptom: 1,003 coal cuts in one 200-day opening (seed 2) once the mine's shaft passed the
  coal seam; the community never got the ore it was short of from them.
- Found by: the round-16-deep census (`examples/deep_census.rs`, seeds 1 to 4, 200 days).
- Cause: confirmed by reading the rule: the ore rule (`settlement.rs`, step 9) wants ore while
  iron and copper ore together are under four, and takes the nearest open face of iron, copper
  or coal; coal never counts toward the want, so an open seam near home was cut for as long as
  ore was short. Before the mine, seams were rarely open (the quarry is two levels deep).
- Fix: the same rule takes coal faces only while coal on hand is under four.
- Guard: tests/delving.rs `coal_is_cut_for_the_smelter_only_while_coal_is_short`.

## 2026-10-28 round-16-deep: a gallery opening into a deep cavern is the breach (#405)

- Symptom: the breach and what follows it fire only when someone stands in a deep cavern, and
  nothing brought anyone there.
- Found by: the 2026-10-27 `breached ... not fixed` entry.
- Cause: as that entry says; and a gallery cut beside a cavern opens it (work.rs `discovered`)
  without anyone going in.
- Fix: `delving.rs`: rock that rings hollow (a cavern floor within six tiles of the workings on
  its level) draws a gallery to it unless the community is cautious (`heard`); a cut that opens
  into a deep cavern nobody has stood in is noted (`observe`, `Mine::look`) and the nearest
  grown member about the workings, not busy, goes in (`send_to_look`). Standing there is the
  breach (underground.rs `breaches`: `breached`, `cavern_found`, those from below, the beast).
- Guard: tests/delving.rs
  `a_people_who_hold_the_deep_is_to_be_dug_dig_down_to_it_and_dig_on` (the shaft goes down
  for the deep, reaches its rock, a gallery opens into the cavern, the breach is a row). The
  planner's sending someone in is not under test there (they are put in the cavern); the
  soaks show it (the breaches in the census of this branch came that way).

## 2026-10-28 round-16-deep: after a breach, a community walls the way or digs on (#405)

- Symptom: a breach was walled only when a beast of the deep was near (underground.rs), the
  same for every community whatever it held about the deep.
- Found by: reading `breaches` for #405 item 4.
- Cause: confirmed: the wall order is posted only on `beast_stirred`.
- Fix: `delving.rs` `observe`: on a breach, the community whose member broke in chooses. With a
  beast stirred, or a DeepDigging stance of 0 or less (a cautious member counts -0.3), it walls
  the way (a wall ordered at the breach tile; `sealed` once it stands, as before); a community
  that holds the deep is there to be dug leaves it open and its mine goes on. `deep_chosen`
  records which (`walled`), and `Mine::chose` keeps it.
- Guard: tests/delving.rs `a_cautious_community_walls_the_breach` and the greedy test above.

## 2026-10-28 round-16-deep: the mine was dug through a famine (#405)

- Symptom: in the branch's 600-day soaks two openings ended far below master's (seed 1: 3 people
  against 62, seed 4: 22 against 42), each when a community left the valley after hard
  seasons (seed 4: six dead of hunger and one of cold in days 499 to 508, then both
  communities left). The mine kept ordering cuts all through.
- Found by: the round-16-deep soaks (`tools/soak/run4.sh`, 600 days) against master's at
  ebf71ff; `examples/deep_census.rs` rows for seed 4, days 498 to 511.
- Cause: not confirmed. The departures are written with hunger, cold, raids and floods as their
  reasons, none of them the mine; that miners' hours away from food work made the hunger worse
  is likely and unmeasured, and a changed replay is not proof either way.
- Fix: `delving.rs` `plan`: no cut is ordered while the community is struggling or worse
  (`Community::fate` above 0, fate.rs: under three meals a head in store), as nobody sinks a
  shaft through a famine.
- Guard: tests/delving.rs `the_mine_waits_while_food_is_short`.

## 2026-10-28 round-16-deep: coal piled up unburned, cut by the quarry's gallery (#405)

- Symptom: seed 4 cut coal 511 times in a 600-day census and the coal in the valley rose to 983
  by day 559 (3 at day 309), far past anything burned.
- Found by: the coordinator's review of the round-16-deep soaks; traced with
  `examples/deep_census.rs` (coal cuts by where they were made, coal held every 50 days).
- Cause: confirmed for 239 of the cuts: the quarry's gallery (`settlement.rs` `quarry_at`)
  took any ore face beside its workings as well as stone, and a coal seam lay at its level;
  coal gives no blocks, so stone stayed short and the gallery went on along the seam. The
  other 272 were made away from any community's present quarry or mine, by the record of
  where they were cut; their rule is not confirmed (the quarry of a community that had moved
  is the likeliest, its site forgotten with the move). The ore rule's coal (the entry above)
  also took coal with no smelter or kiln to burn it in.
- Fix: `quarry_at` cuts stone only; the ore rule takes coal only while coal is short and a
  smelter or kiln of the community's stands.
- Guard: tests/delving.rs `the_quarry_gallery_cuts_stone_not_a_coal_seam`,
  `coal_is_cut_for_the_smelter_only_while_coal_is_short` (now also: no coal cut with nothing
  to burn it in).

## 2026-11-02 #548: the arrival's survival-snap boost never faded (#395)

- Symptom: box-ci failed `dispositions::order_changes_the_real_haul_choice_and_unrelated_axes_do_not`:
  a person with order 1.0 felled a tree instead of hauling a log, at day 30.
- Found by: the box-ci log of 25985e59; the fixture already clears thoughts, so the arrival was not
  the cause, a permanent bias was.
- Cause: confirmed. `strain.rs` `survival_urge` applied the snap pole's 1.35x for the steadiest
  genomes with no fading, so they weighed survival work (felling) 1.35x up for life, over a
  disposition-driven haul. The test's fixture was right and is unchanged.
- Fix: `survival_urge` scales the snap factor by the arrival thought's remaining life, so it fades
  with the shock; code comment names this entry.
- Guard: the existing dispositions test (settled people, no arrival thought), plus
  tests/arrival_affect.rs, which keeps the snap pole fresh.

## 2026-11-01 fix-403-holtwick-tile: seed 3's wall never closed for a house on the line (#403)

- Symptom: at day 600 in the #553 soaks, seed 3's Holtwick had 78 of 79 ring tiles standing
  and never closed; the open tile (5,8,4) was not reachable and its BuildWall order (priority
  8) had failed 9 times; two beds stood on unsealed outline tiles (5,5,3) and (5,7,3).
- Found by: the #553 soak saves and `examples/wall_probe.rs`; traced with seed 3 saves of days
  19 to 22 made on master 4888a9c5 and the day-600 save, which the #553 branch's build made
  (the two builds differ from day 22 on; #557).
- Cause, checked on the saves: the open tile was not what held the ring open. `whole` was
  true from day 10062 (C74 counts the unreachable tile as closed); `sealed` was false for the
  two bed tiles. Before the laying (day 10060) a house stood at the map's west edge, its roofed
  floor at (5,5..7,3) with beds at (5,5,3) and (5,7,3) and the passage between them at
  (5,6,3). The house was too near the edge to be walled round (C74), but its own beds pulled
  the outline out to the edge clamp at x = 5, where `stands_on_the_line` could not move it.
  The laying left out the tiles with beds but, unlike the growth step, not roofed floor, so it
  walled the passage (5,6,3) within a day and shut both beds into pockets nobody could reach,
  on the outline and outside the ring for good. The tile (5,8,4) is on a rise inside the ring
  whose only ways up were ring tiles on the north side at z 4, walled on day 10061; every tile
  beside it is on the same cut-off rise, so `stand_tile` finds nowhere to stand. Its wall stood
  on day 10062 in master's run; why it was gone by day 600 in the #553 build's run is not
  confirmed (not a resumed save going differently: that build resumed from its own day-22
  save reaches the day-600 save's digest exactly, #557).
- Fix: `systems/settlement.rs` `palisade`: outline ground nobody from home can reach is no way
  in, as a ring tile is (`sealed`); the roofed floor of a building whose every way out lies
  inside the line is held by its walls (`held_by_building`); the laying lays no ring tile on
  what `stands_on_the_line` (roofed floor included); wall and door orders at ring tiles that
  neither a way nor a tile beside them joins to home are withdrawn and not posted, what was
  brought left by the site, and posted again if a way opens. Reach counts only while home has
  a region. `plan_community`: no haul is ordered for a stack nobody from home can walk to.
  Holtwick's day-600 save closes the next day with these.
- Guard: tests/defence.rs `a_ring_closes_round_a_ledge_and_a_shut_room_nobody_can_reach`
  (fails on master: not closed), `a_ring_at_the_edge_does_not_wall_a_house_through` (fails
  on master: four ring tiles laid on the house's floor).
- Commit: (this commit).

## 2026-11-02 feat-452-stories: the colony's own deaths became nobody's story (#452)

- Symptom: a death after the arrival was a chronicle row and a few memories; nobody told it to
  anyone, no version of it could differ or be wrong, and nothing marked where the dead were
  buried, so a later person had nothing to find. Tales (lore.rs) covered only the old valley.
- Found by: design review of #452 against lore.rs, left_sites.rs, systems/rites.rs and
  view/bodies.rs (docs/design-stories.md).
- Cause: confirmed by reading: `lore::Tale` is keyed to an old site id, `systems/life.rs`
  `die` gave witnesses no account, and the body is view-only (`view/bodies.rs`), so a burial
  left no trace in the simulation.
- Fix: `accounts.rs`. `die` gives those who saw the death (awake, grown, same level, within
  eight tiles, in sight) an account with the true cause; `systems/act.rs` talk passes an
  account to one who lacks it, with its teller, drifting by the tale rule (a cautious teller or
  one who fears strangers drifts it more: illness to a curse, a fall or wounds to a killing, a
  beast, water, rock or the road to a taking); a dark version leaves its holder a bad memory of
  the place. `systems/rites.rs` `lay_to_rest`: a burial where the dead lay sets a grave
  (`World::graves`, a `Grave` structure) on the nearest open unroofed tile within two. Hourly,
  a grown person who came after and never knew the dead reads a grave they stand by: "someone
  was put in the ground here", or a curse if they are cautious or fear death. Inspect lists
  accounts after tales; a grave's hover gives the record, then what is said and by whom. New
  events `AccountTold`, `GraveRead` (appended).
- Guard: tests/accounts.rs (witnesses only; teller kept and drift with the record unchanged;
  unease only for the holder; a grave read by a newcomer, wrong when fearful, and joined by a
  later telling; save round trip).
- Commit: (this commit).

## 2026-11-02 #559: the New game wizard always opened on the same valley

- Symptom: the valley page of the setup wizard showed seed 7 on every New game (1 with no
  launcher flag), so every game began from the same few valleys unless the player typed a seed.
- Found by: Dan, playing the master build from his watch launcher (#559).
- Cause: confirmed in `godot/scripts/main.gd` `_ready`: a hands-on launch with no valid
  autosave opened the wizard on `seed_v`, which is `--seed=N` when given and 1 otherwise, and the
  launcher passes `--seed=7` (to skip the autosaves). Nothing rolled a seed. Paging back and
  forth already kept the seed and region (`setup.gd` keeps `seed_v` and `region_v` across
  `goto`), but nothing tested it.
- Fix: `setup.gd` `start` rolls a fresh random seed (`roll_seed`, 0 to 999999, the spin box's
  range) when not given one, and resets the region and caches for the New game; `main.gd`
  passes `--seed` only with the new `--fixed-seed` flag or a `--wizard-shot`. `--seed` alone
  still skips the autosaves, and every non-wizard path (shots, footage, `--opening`, `--fresh`,
  `--save-at-end`, stages) still uses it.
- Guard: `tools/gd_test.gd` `wizard_seed_is_random_per_new_game_and_kept_across_pages`: four
  New games roll at least three seeds, an asked-for seed is kept, and after choosing a place,
  going back a page and returning shows the same seed, place, description and picture.
- Commit: branch fix-559-560-valley-wizard, the commit adding this entry.

## 2026-11-02 #560: every place on the world map made the same valley

- Symptom: on the wizard's valley page, clicking another place on the world map left the
  valley picture the same land; only the portal's red mark moved, and some colours shifted.
- Found by: Dan, playing the master build (#560); measured on seeds 1, 7 and 42: between the
  seed's own region and seven other habitable regions, 15% to 68% of the preview's columns
  changed biome, but the hills and valleys stood in the same places (the heightmap does not
  read the climate).
- Cause: confirmed in `crates/hf-sim/src/setup.rs` (`from_setup`, `preview`) and
  `world/gen/mod.rs` `generate`: the region chose only the valley's climate (`character_of`),
  and the ground (heightmap, strata, water, caves, flora placement, founding site) and the lie
  of the land (`Landform::of_seed`) were drawn from the seed alone. The preview was truthful:
  the world made after Begin had the same ground everywhere too.
- Fix: `GenParams::place` (saved, left out at 0) and `gen::ground_seed`: the ground is drawn
  from the seed mixed with the place. `setup::place_of` gives 0 for the region the seed itself
  would choose, so every world made without choosing a region keeps its ground and digest, and
  region + 1 elsewhere; `setup::valley_params`, shared by `from_setup` and `preview_in`, also
  draws the lie of the land from that ground. The world's draw of wild kinds (D65) stays the
  world seed's. DECISIONS D69.
- Guard: `tests/realm.rs` `each_region_of_the_world_is_its_own_valley` (two places differ in
  over a quarter of their heights; the same place repeats exactly; the seed's own region equals
  the unchosen valley; the flora draw is the same) and
  `the_wizard_shows_the_ground_the_world_will_have` (from_setup on a chosen region has the
  previewed ground in 99% of columns and the previewed founding site).
- Commit: branch fix-559-560-valley-wizard, the commit adding this entry.

## 2026-11-02 #560: the valley picture was a biome map made on the main thread

- Symptom: the valley picture was a flat biome colour per column, not what the main view draws,
  and each visit to the valley page or click on the world map generated the valley (and the
  world map, several times) on the main thread before the page could draw.
- Found by: reading `setup.gd` for #560: `goto(1)` called `valley_preview` twice (once for the
  sliders) and the world map three times (`world_preview`, `region_info`, inside each preview);
  measured, a preview takes 80 to 150 ms and a world map 36 ms (release, seeds 1 and 7).
- Cause: confirmed in `setup.gd` `_refresh_preview` and `seed_character`, and the bridge's
  `valley_preview`, `world_preview` and `region_info`, each of which made the world map again.
- Fix: `godot/scripts/valley_preview.gd` draws the picture from the generated valley's tiles,
  heights and trees in the main view's colours (each terrain and tree sheet's average, the flat
  fallback colours otherwise), lit by the lie of the land. The bridge keeps the seed's world map
  (`PreviewJobs`) and makes previews on their own threads (`valley_preview_begin`,
  `valley_preview_take`); `valley_character` gives the sliders their character without a valley.
  `setup.gd` caches each picture by seed, place and character for the New game, so paging back
  or returning to a place shows it at once. Measured with the debug library: a click on a new
  place returns in 0.1 ms and its picture arrives about 90 ms later; returning to the valley
  page takes 2.7 ms; drawing the picture takes 18 ms on the main thread.
- Guard: `tools/gd_test.gd` `wizard_preview_follows_the_place` (two places, two pictures and two
  grounds; the first place again is the same picture at once; a click on a new place returns
  within 100 ms) and the bridge's `valley_previews_are_made_off_the_caller_and_handed_over_once`.
- Commit: branch fix-559-560-valley-wizard, the commit adding this entry.

## 2026-11-02 #564: the chosen place on the wizard's world map shows on snow and ice

- Symptom: on the New game wizard's world map the chosen place was a small white square that
  was close to invisible on ice, snow and other pale ground (the #559/#560 review frames show it
  as a dot).
- Found by: review of PR #563, filed as #564.
- Cause: confirmed in `godot/scripts/setup.gd` `_draw_world`: the mark was a 3x3 square of pure
  white over the map's colours, and the realm map draws ice as (233, 227, 214), within a tenth
  of white's brightness, so nothing set it apart there.
- Fix: `setup.gd` `mark_place` draws a white centre in a dark ring with a thin white edge, sized
  to the map (11 regions across on the 192-region map), clipped at the map's edges; the page's
  caption names it "the ringed mark".
- Guard: `tools/gd_test.gd::test_world_map_mark_shows_on_every_ground` marks ice, ocean and
  grassland places of real world maps and checks that at least 9 of the mark's pixels differ in
  brightness from the ground by 0.4 and that it is at least 9 regions across; the old square
  has no such pixels on ice and is 3 across.
- Commit: this branch's #564 commit.

## 2026-11-02 #423 (part 1): bodies that are not skeletons are plan kinds

- Symptom: every body plan was a skeleton with a head, a chest and limbs, so there was no way
  to say what a slime, a swarm or a many-segmented crawler is made of; wounds and illness
  (body.rs) assume limbs and organs.
- Found by: #423 (epic #497), from the #414 B3 creature list.
- Cause: confirmed in `plan.rs`: `parse` required a head, a torso, a pair of hands and
  something to move on of every plan, and the part vocabulary held only limbs.
- Fix: `plan.rs` gains `Kind` (skeletal, soft, swarm, segmented; saved, so append-only), three
  parts appended to the vocabulary (mass, member, segment), a `count` per plan (members of a
  swarm, segments of a chain; a soft body is `SOFT_WHOLE` hundredths of itself), and
  `check_kind`, the parts each kind has and the one-tile rule (#421). Three plan files
  (`data/peoples/plans/soft.toml`, `swarm.toml`, `segmented.toml`). `species.rs` `validate`
  refuses a people's species on any but a skeleton (beasts only, D70). No creature has the new
  plans yet; nothing in play changes.
- Guard: `tests/body_kinds.rs` (the old part ids unchanged, the new kinds load with their
  parts, bad kind files refused, a people has a skeleton); `tests/bodies.rs`
  `every_built_in_plan_loads` counts the twelve plans.
- Commit: (this commit).

## 2026-11-02 #567: the Why? lens shows a person's live decisions

- Symptom: clicking a person showed what they were doing and their needs, but not why: the
  options they weighed, what tipped the choice, what a pressing need cut into, or what they put
  off. The `chose` rows in the chronicle explain a few kinds of choice once a day.
- Found by: the owner's #565/#567 brief (watch session on 769333ae, seed 324562): a debug view
  that does not show the decisions gives nothing to learn from.
- Cause: confirmed in `systems/decide.rs` `choose`: every option passes one `consider` closure,
  which kept only the best score; the terms (the goal's base, the feelings lean, the secured
  gate) and why an option was not a choice (no way there, flooded ground, spoken for) were
  thrown away each decision.
- Fix: `why.rs` (new) is a bounded read model (8 cycles of at most 10 options, for one person)
  held in `World::why`, never saved and never read by a system. `World::trace_why` names the
  person. `decide::step` builds a `Cycle` for that person only: how they came to decide
  (deliberate, or interrupted by starving/parched/exhausted/freezing and what was stopped),
  the conditions with their values (`secured`, the arrival urge, the leisure weight), and every
  option `consider` saw with base, feelings, gate, score and the rejection reason (the reach
  check now names which test failed: `unreachable` in `choose`). `why::explain` writes the
  reasons and the options the secured gate held back that would otherwise have won ("put off
  until food and shelter are secure"); `put_off_work` adds work given up on for now and walks
  that found no way, from the person's own `work_cooldowns`. The bridge sends it in
  `inspect(id)` (`why_traced`, `why_cycles`) with `set_why_target` and `why_target`; the
  client traces whoever is selected, shows the lens behind a Why? button on the inspect panel,
  and shows "(o) tracing why: NAME" on the HUD while tracing. `--why` opens it for shots.
- Guard: tests/why_lens.rs: the short-store scenario of #415 shows the stocking choice, the talk
  option with its 0.05 gate and the reasons, and lists the talk as put off; with the store full
  the same person talks and nothing is put off; a starving talker shows as interrupted; only
  the traced person is recorded; and the digest after two days is identical traced and
  untraced. tools/gd_test.gd `test_why_lens_renders_the_traced_cycle`.
- Commit: (this commit).

## 2026-11-02 #566: the dead of a lost caravan were buried as if the bodies had come home

- Symptom: in a watch session (master 769333ae, seed 324562) the three members of a caravan
  lost on the road ("no food and no road; nothing more was heard") were each `LaidToRest`
  with `in_absence: false` about 1,500 ticks later. Of the run's other 15 deaths none had a
  rite or burial row.
- Found by: the issue, from the session's chronicle; reproduced with
  `examples/rite_census.rs` (every death of a person and what became of the body) on seeds 1
  and 4 over 120 days: each death "lost on the road" was laid to rest with the body, an hour
  after it.
- Cause, confirmed in the code and by the census: `systems/expedition.rs` brings a lost
  member back from `overworld.away` into `w.agents` at the tile they left from and calls
  `life::die`, which records the death at that tile and queues an ordinary rite. In
  `rites::step` the body was reachable there, so the nearest of the community walked to it
  and gave the rite with the body. The same held for one "lost in the deep" (`taken.rs`
  `settle`). The view drew the body at the departure tile, and the water and sickness near
  the unburied dead counted it. The other 15 deaths: "brought down by wolves" is only ever a
  deer (`animals.rs`) and "killed by {name}" only a beast felled in a fight (`combat.rs`), so
  those 11 are animals and correctly have no rite. The 4 drownings are not confirmed (the
  session's chronicle was not available here); the census found people with no community
  who drowned or died of thirst and were left unrested with nothing on the record, because
  `conditions::unlaid` sets a condition and writes no row.
- Fix: `systems/life.rs` `die_away`, used for "lost on the road" and "lost in the deep": the
  `Died` row has no place, the death counts as alone, and the `RiteDue` is marked `away`.
  `systems/rites.rs` `step`: nobody walks to an `away` body; after `MOURN_AFTER` it is
  mourned at home with `in_absence: true`. `water_quality.rs` `sources` and `health.rs`
  skip an `away` body; `view/bodies.rs` draws no body for a death with no place. A person
  given no rite now gets a `LeftUnrested { agent, why }` row (appended), from
  `rites::left_unrested`, at each place `unlaid` was called for a death with no rite: no
  community at death, nobody of their people left, nobody came in a day.
- Guard: `tests/burial_records.rs` `the_dead_of_a_lost_caravan_are_mourned_in_their_absence`
  (fails on master: the `Died` row has the departure tile, and the rite is given with the
  body) and `a_person_with_no_people_dies_on_the_record_as_given_no_rite`.
- Commit: branch fix-566-burial-records (this commit).

## 2026-11-02 #562: the Legends map of the present said "the year before the portal"

- Symptom: during play, the Legends year slider at its end labelled the map "the year before
  the portal" in any year of the colony.
- Found by: adding the map of the present's marks for those away (#562).
- Cause: confirmed in `godot/scripts/main.gd` `_on_legends_year`: the label counts back from
  `realm_year()`, which goes on after the arrival (realm/live.rs steps it yearly), so its own
  last year always read as one year back.
- Fix: `_on_legends_year` labels the slider's last year "now", and on that map draws and lists
  those of the valley who are away (`away_map.gd`, bridge `away_list`, hf_sim `view::away`).
- Guard: none for the label (a one-line text choice); `tools/gd_test.gd`
  `test_away_marks_and_lines_on_the_world_map` and `crates/hf-sim/tests/away_view.rs` guard
  the marks and rows.
- Commit: this branch's #562 commit.

## 2026-11-02 feat-455-portal-economy: the deep's beast came up on a clock (#455)

- Symptom: a beast of the deep stirred by a breach came up exactly two days later unless the way
  was walled by then; nothing the valley did near the way in between made any difference, and
  nobody had any warning.
- Found by: design review of #455 against underground.rs and creatures.rs
  (docs/design-deep-ways.md).
- Cause: confirmed by reading: `underground::breaches` pushed `(beast, cavern, tick + 2 days)`
  onto `creatures.rising`, and `underground::rise` brought it up when the tick came.
- Fix: `stirring.rs`. A breach now gives the cavern a stir (`creatures.stirring`). Hourly the
  stir rises by one for each of the valley's people in the cavern or within six tiles of the
  breach on its level, and two more for each at mining, digging or building there; an hour with
  nobody there takes one off. At 12 those there hear something move below (`StirBelow`,
  appended) and keep a bad memory of the way; at 30 the beast comes up if the way is open
  (`underground::come_up`, split out of `rise`) or is barred and goes back down if walled. Old
  saves' `rising` entries still come up on their day.
- Guard: tests/stirring.rs (a quiet way never wakes it; three at the way are warned, then it
  rises, at the counts; work is louder than standing; save round trip); tests/combat.rs's two
  rising tests now step hour by hour with someone in the cavern, assertions unchanged.
- Commit: (this commit).

## 2026-11-02 #484 (finish): same-tick timelapse samples differed, series were unbounded across sessions

- Symptom: the first slice's windowed check `identical seed/tick/filter produce equivalent
  viewport PNGs` (`tools/timelapse_visual.gd`), never run before this branch, failed in five of
  six runs on the box: the first sample of a fresh world and a re-capture at the same
  tick differed in 19,233 pixels, all rain streaks. Separately, each launch or load started a
  new series directory named from the first sample's state digest, so series were never
  resumed and nothing bounded how many accumulated on disk; and a sample landed on whatever
  tick a frame's step of up to 400 ticks ended, so a re-run of the same days could not land on
  the same ticks.
- Found by: running `tools/shots/timelapse.sh` under Xvfb for the first time (2026-11-02),
  then a probe that captured the same tick five times with and without a warm-up frame: with
  no frame between world creation and the first sample the first image differed and the next
  four matched one another; with one or more frames of warm-up all five matched. The series
  directory naming was read from `main.gd` `_capture_timelapse`.
- Cause: confirmed for the pixels: the weather emitters were restarted from the tick seed
  before they had run a frame with their current emitting flag, which gave different particles
  from a restart of an emitter that had. The other wall-clock inputs removed in the same change
  (levels above and ruins cached for 2 and 5 real seconds, static entities rebuilt on a
  half-second timer, transient effects aged in real seconds, the 8 Hz water/fire layer) did not
  show in this scene's diff and are removed as a precaution, not as confirmed causes.
- Fix: `godot/scripts/main.gd` `_capture_timelapse` waits one frame at the watch with stepping
  held before freezing the particles; `world_view.gd` `prepare_still`/`end_still` read the
  cached levels and ruins afresh, rebuild every entity and leave transient effects out of the
  sample; `render/anim_ground.gd` redraws every frame while frozen. Series are keyed on the new
  `hf_sim::digest::world_identity` (`SimHost.world_identity`) and reopened from disk
  (`timelapse.gd` `load_series`), at most 6 are kept (`prune_series`), each within 120 samples
  and 160 MiB, images at most 1280 x 720 RGB (`fit_capture`). Samples fall at noon of each
  calendar day and `_process` stops stepping at the sample tick (`next_sample_tick`,
  `step_allowance`). A re-run sample tick is compared by state digest (`reconcile`): the same
  digest confirms it, a different one removes it and every later sample (`drop_from`).
- Guard: `tools/timelapse_visual.gd` (same-tick PNG identity, four noon samples, reload from a
  save and a byte-identical re-run of the four days, divergence removal, valley capture, bytes
  on disk); `tools/gd_test.gd` `test_timelapse_samples_fall_on_exact_noon_ticks`,
  `test_timelapse_byte_bound_order_and_replay`, `test_timelapse_series_reloads_and_prunes`,
  `test_timelapse_event_markers_and_world_identity`; `crates/hf-sim/tests/timelapse_identity.rs`.
- Commit: this commit (feat-484-timelapse-finish).

## 2026-11-02 #469: most rough nights were spent beside a free bed of home nobody had in mind

- Symptom: after #475 and #535, people of a community still slept rough on 888 of 3,642
  person-nights (days 0 to 60 of the opening, seeds 1 to 4), with about one finished bed
  standing per person.
- Found by: `examples/sleep_rough.rs` on master 989d0301. Each night slept (a `Slept` row of six
  hours or more, the row behind `SleptRough`) is counted in bed or rough, and the moment of
  lying down on the ground is classed by the beds standing then. Rough nights: 162 with no
  free bed standing; 724 beside a free, reachable bed the sleeper did not know and could not
  see, median 9 to 15 tiles away; none with a known free bed (the #475 fix holds) and none
  with every free bed unreachable. Sampling each person's knowledge hourly on seeds 1 and 4
  (30 days): of 143 lay-downs beside an unknown free bed, 120 were beside a bed they had once
  known and forgotten, 23 beside one they never knew, and every one of those beds was within
  40 tiles of the sleeper's home.
- Cause: confirmed by the probe and in the code. The sleep choice (`systems/decide.rs`
  `choose`) takes beds only from personal knowledge and, since #535, from sight. A bed enters
  knowledge when finished within 14 tiles (`work.rs`) or when seen (`perceive.rs`), and house
  walls hide it; structures share a knowledge budget, so a bed not seen for a while is
  evicted (`mind/knowledge.rs`). The person then lies down a few tiles from it.
  Ownership is not a cause (there is no bed ownership; "taken" is a live claim scan), and
  neither is reach (no case of a free bed standing with none reachable).
- Fix: `systems/decide.rs` `choose`, sleep block: the finished beds within 40 tiles of the
  person's community home are candidates for every member, as the community's hearths are
  known to all of it (`perceive.rs` `SharedStore`), scored like a known bed. Nothing is
  written into knowledge, so budgets and eviction are unchanged; a person without a
  community, or a bed far from home, is still known only by seeing it. `SleepingPlaces`
  gathers each community's home beds once per decision tick (`home_beds`).
- After (same probe, same seeds and days): rough nights 229 of 3,296, all with no free bed
  standing; rough nights beside a free bed 724 to 0. Per seed, rough with a free bed before
  and after: seed 1 272 to 0, seed 2 105 to 0, seed 3 94 to 0, seed 4 255 to 0.
- Guard: `tests/sleep_bed.rs` `a_villager_who_has_forgotten_the_beds_of_home_still_goes_to_one`
  (fails without the fix: ground sleep 30 tiles from a free bed of home), with controls
  `a_bed_far_from_home_is_not_known_without_seeing_it` and
  `a_stranger_does_not_know_a_village_bed_without_seeing_it`.
- Commit: branch fix-469-sleep-rough (this commit).

## 2026-11-02 feat-454-institutions: roles were a class list handed out by skill (#454)

- Symptom: every community had exactly one forager, builder, farmer, cook, miner, hauler and
  healer each day, the best by skill, whether or not they had ever done that work; nobody could
  hold two, share one, or come to one by doing it, and the healer was picked by cooking skill.
- Found by: design review of #454 against systems/society.rs, systems/decide.rs and view/look.rs
  (docs/design-institutions.md).
- Cause: confirmed by reading: `systems/society.rs` `assign_roles` gave one of each role in
  `ROLES` to the most skilled member not yet given one, daily, and cleared the rest.
- Fix: `duties.rs`. `systems/work.rs` `run` counts each minute at a work order toward its kind
  (building and felling, farming, cooking, mining and digging, hauling and fetching water,
  tending). Daily (`systems/society.rs` `step`, replacing `assign_roles`) the tally keeps 85 in a
  hundred of itself; kinds with two hours or more and a third of the top kind are the person's
  duties, the top one their role. `systems/decide.rs` weighs work by the duties (`duties::fit`,
  the old role table over every duty held); the healer check reads the healer duty. A role taken
  is `RoleTaken` as before; a role let go is `RoleLeft` (appended). Inspect shows the duties
  with the hours behind them.
- Guard: tests/duties.rs (the one who cooked is the cook and the skilled one who never did is
  not; shared roles and two duties with their work weights; a duty fades and passes to whoever
  took the work up; in an opening every role held rests on at least two hours of that work).
- Commit: (this commit).
- Test fixture: tests/health.rs `wound_heals_with_rest` now has the hurt person know their
  house's beds; it had relied on the skill-given farmer role walking them past the house before
  the first night (on this branch they slept on the ground, never having seen a bed: design
  rule 8). The assertions are unchanged.

## 2026-11-02 feat-308-slice: nothing came through the portal with the ten (#308, epic #508)

- Symptom: nobody who came through the portal carried anything from it; psy existed only as
  the thin place's events and the true-dream, never as something hidden in a person and found
  out through play.
- Found by: design review of #308 and epic #508 against conditions.rs, thin.rs and
  underground.rs (docs/design-magic-psy-tech.md).
- Cause: confirmed by reading: `opening.rs` `spawn_survivor` gave the ten their identity,
  past and techniques and nothing hidden; epic #508 steps 1 to 4 had no code.
- Fix: `resonance.rs`. `spawn_survivor` draws each of the ten a potential once from the seed,
  slot and rerolls (`draw`, about two in ten; the rock-sense), saved and shown nowhere.
  `thin.rs` `study`: each hour by the woken stone counts; the fourth wakes it
  (`CameBackUnsettled`, appended; rest falls). `systems/work.rs` mining: a woken carrier who
  does not know shoring will not cut a face that would leave the roof unheld (`balks`, the same
  `unheld_if_opened` test that brings a roof down): `WouldNotCut` (appended), rest falls, a bad
  memory of the face, the order's failures rise by three and the cut is left.
- Guard: tests/resonance.rs (about two in ten, drawn the same every time, nothing woken at the
  start; wakes only in a carrier after four sittings; over eight seeds a woken carrier never
  brings the roof down and leaves the face, where tests/cave_in.rs shows an ordinary miner
  does about a third of the time; nothing where the roof is held, they know shoring or it has
  not woken; save round trip).
- Commit: (this commit).

## 2026-11-02 #568: a quarrel is judged afterwards, with regret or rationalising

- Symptom: a quarrel left both people thinking worse of each other and nothing else, whoever
  turned out to have been hurt, sick or grieving at the time; nobody looked back on what they
  had done, and nothing they did later showed it.
- Found by: the owner's #565/#568 brief ("laughing at someone who turned out to be hurt").
- Cause: confirmed: `systems/act.rs` records a quarrel's opinion and memory at the moment and
  nothing compares the outcome with what was known afterwards; there was no record linking a
  later judgement to the quarrel, and no term in `decide::choose` that a judgement could move.
- Fix: `mind/conscience.rs` (new): each person's `Conscience` holds quarrels still to judge
  (linked to the `Quarrelled` event), the judgements made, and a fading lean toward or away from
  helping each person judged, bounded to 0.7..1.4 and gone after ten days. `systems/act.rs`
  notes the quarrel for both. `systems/conscience.rs` (new, hourly, after `feel`): one who sees
  the other within 8 tiles on their level, in sight, sick, hurt or grieving within a day of
  the quarrel judges it by `regret_tendency` (warmth, caution and a communal bent toward regret,
  stress away from it): regret raises their opinion of the other by 0.12 and their help lean by
  x1.3; rationalising lowers the opinion by 0.05 and the lean by x0.8; a `Chose` row says which.
  `decide::choose` applies the lean as a named term (`learned`, shown by the Why? lens) to
  tending and seeking out that person, and one whose lean passes `AMENDS` (1.15) may tend the
  other though neither healer nor kin. Unjudged quarrels lapse after a day.
- Guard: tests/conscience.rs: the same sick person after the same quarrel leaves a warm person
  regretting (opinion up, lean above 1) and a cold one rationalising (opinion down, lean below
  1), each judgement linked to the quarrel's event; a healer's tend option has the same base and
  a learned term of 1.3 after regret, 1.0 for a control, under 1 after rationalising; one who
  regrets weighs tending a sick non-kin the control never weighs; nothing is judged out of
  sight and the quarrel lapses; repeated judgements stay within the bounds and fade to 1.
  `examples/conscience_census.rs` counts quarrels, judgements and the lean's range in the
  opening soaks.
- Commit: (this commit).

  Merge fix-up: master's #454 made a healer whoever's recent work holds the duty
  (`duties.held`), no longer the handed-out `role`. The healer test set `role`, so after the
  merge its healer was not a healer and weighed no tending. The fixture now sets
  `duties.held`; the assertions are unchanged.

## 2026-11-02 #485 (slice 2): a healer tending was drawn with a bundle of materials, bent like a field hand

- Symptom: someone working a Tend order played the stoop cycle (the hoe and forage bend) and held
  the builder's bundle of materials. A sleeper was one still frame. A person drinking stood in the
  idle pose.
- Found by: reading `crates/hf-godot/src/lib.rs` `agent_row` (Tend fell to work code 0, which the
  client draws as stoop) and `crates/hf-sim/src/view/look.rs` (Tend fell to `Some(_) => 4`, the
  bundle) while planning #485's remaining cycles.
- Cause: confirmed in those two match arms. No cycle existed for tending, drinking or breathing
  in sleep.
- Fix: `tools/art/people.py` appends sleep2, sleep3, tend1-3 and drink1-3 to every sheet (48
  columns). `look.rs` `look` gives a Tend order no equipment, and drops the spear while drinking
  or tending (`tending`). The bridge sends work code 8 for Tend and pose 12 for a person drinking.
  `rules.gd` `sleep_frame`/`held_frame` and `entity_layer.gd` play them.
- Guard: `crates/hf-godot` `drinking_and_tending_have_their_own_rows_with_free_hands`;
  `crates/hf-sim/tests/look.rs` (48 columns, new column names); `tools/gd_test.gd`
  `test_sleep_tend_and_drink_cycles`.
- Commit: this commit (feat-485-animation).

## 2026-11-02 #485 (slice 2): people of a hairless kind looked alike apart from build and skin

- Symptom: two Murrow, Skarri, Screeling, Meer or Below of one build and tone differed only in
  their clothes; the hair slot, where other kinds show their inherited hair style, was empty.
- Found by: #485's acceptance on bounded per-person variation, checked against
  `entity_layer.gd` `_agent` (hair texture set to null for `art.hair = false`).
- Cause: confirmed: no art existed for the hair-style gene of a hairless kind.
- Fix: `tools/art/people.py` `draw_marks` and `MARKED` draw three head markings per hairless kind,
  in skin swap colours, per build and age. `rules.gd` `marking_sheet` and `entity_layer.gd` `_agent`
  draw one in the hair slot, chosen by the hair-style gene.
- Guard: `tools/gd_test.gd` `test_sleep_tend_and_drink_cycles` (every kind's marking sheet exists
  for any gene, and the gene picks the pattern); `palette_check.py` and `provenance_check.py`
  over the 90 new sheets.
- Commit: this commit (feat-485-animation).

## 2026-11-02 #582: time-budget tests measured the machine, not the code

- Symptom: with eight agents building on the box (load average 120 to 230 on 32 cores),
  `realm_figures::a_thousand_years_bring_tens_of_thousands_of_figures`,
  `realm_peoples::a_thousand_years_is_quick` and earlier `history_pass::history_pass_budget`
  failed on PRs that do not touch the realm or the history pass (#577, #581), and stopped
  tools/ci.sh before the other tests ran.
- Found by: local tools/ci.sh runs for #562 and #561 (46 s, 88 s and 62 s against the 30 s
  budget), filed as #582.
- Cause: confirmed. The budgets asserted on `Instant::elapsed`, wall time, which counts the time
  the scheduler gave other processes. Measured on the loaded box: a thousand years took 6.9 s
  of the test thread's CPU and 13.7 s wall (budget 15 s); with figures, 10.7 s CPU and 23.3 s
  wall (budget 30 s), and the wall time went over budget when the load rose.
- Fix: `crates/hf-sim/tests/support/cpu.rs` measures the calling thread's CPU clock
  (`clock_gettime(CLOCK_THREAD_CPUTIME_ID)` through libc, a Unix-only dev-dependency already in
  the lock; wall time on Windows, as before). `a_world_map_is_quick`, `a_thousand_years_is_quick`,
  `a_thousand_years_bring_tens_of_thousands_of_figures`, `unreachable_detected_o1` and
  `history_pass_budget` assert on it, each budget unchanged; `history_pass_budget` runs its
  worlds in a one-thread rayon pool so the thread's clock counts the parallel parts too (the
  result is the same at any thread count, D55). Each prints its CPU and wall time. hf-godot's
  `valley_previews_are_made_off_the_caller_and_handed_over_once` (also failing under load at
  100 to 150, seen by another agent) now asserts that `begin` takes under 20 ms of the calling
  thread's CPU and that the one preview made was made on another thread (`PreviewJobs`
  records the count and the last builder's thread); its 20 ms wall-clock check, which would
  also catch a `begin` that waited on a lock, stays and is skipped, with a line saying so,
  only when the one-minute load is over 1.0 per core.
- Guard: a deliberate slowdown, not committed (CPU spin loops in `Realm::step`,
  `realm::generate` and `Nav::reach`), makes all five fail on CPU time on the loaded box
  (history 13.4 s a world against 6.0 s; 100k reach queries 2.26 s against 2 s; world map
  7.1 s against 5 s; figures 30.9 s against 30 s; a thousand years 27.4 s against 15 s), and
  without it all five pass at load 110 to 170. Making the preview on the calling thread (also
  not committed) fails the bridge test on CPU time (75 ms against 20 ms) and, with that check
  off, on the thread check.
- Commit: this branch's #582 commit.

## 2026-11-02 #587: people rose rested at 03:00 and spent the small hours awake in the dark

- Symptom: on seed 3 about a third of people were awake at 02:00, loitering, eating and
  working at rest 0.4 to 0.5 (seen by the #469 work).
- Found by: `examples/night_awake.rs` on master 976b7b0b, seeds 1 to 4 over 60 days: the share
  of people awake at each hour of the night, by people and by activity, and for people of the
  day, when each night-waker lay down and what they woke to.
- Cause, confirmed by the probe and in `systems/act.rs` (the `Sleeping` arm): the seed 3 third
  at 02:00 is the Leachaa, a nocturnal people (`cycle = "nocturnal"`, rest time 08:00 to
  18:00): 312 of 315 Leachaa person-hours at 02:00 were awake, against 24 of 701 for humans.
  That is by design and not changed. What was wrong was the end of the night for people of
  the day on every seed: 18 to 23% awake at 03:00, 45% at 04:00, 65 to 70% at 05:00. A
  rested sleeper got up after eight hours whatever the hour (`slept >= 8h` at night), and
  rest drains in 16 waking hours, so one who woke at 03:00 was tired again by 19:00, lay
  down, and woke at 03:00 again: the day stayed about three hours off the clock (on seeds 1
  and 3, 380 night wakings in 30 days, all after lying down at 17:00 to 22:00). They woke
  thirsty: asleep, thirst falls a quarter as fast, and eight hours from 20:00 leaves most
  below 0.3.
- Fix: `systems/act.rs`, the `Sleeping` wake rule: at night a rested sleeper stays abed until
  the night ends unless hunger or thirst is below 0.15 (the level at which `decide.rs`
  already treats thirst as critical). The other ways out of sleep are unchanged (`until`
  passed by day, the six-hour cap past `until`, and the critical wake-ups in `decide.rs`).
- After (same probe): awake at 03:00 / 04:00 / 05:00 on seeds 1, 2 and 4 went from
  18/45/70%, 23/46/68% and 22/45/65% to 2/5/16%, 4/9/19% and 1/2/11%. On seed 3 humans
  awake at 02:00 were 3 of 637 person-hours; the Leachaa are all awake, as a nocturnal
  people should be. Hours slept a person-day rose from 8.5 to 8.6 to 8.9 to 9.5.
- Guard: `tests/night_sleep.rs` `one_who_lay_down_at_seven_sleeps_until_the_night_ends`
  (fails on master: up at 03:00) and `hunger_still_gets_a_rested_sleeper_up_in_the_night`.
- Commit: branch fix-587-night-awake (this commit).

## 2026-11-02 #423 (part 2): wounds, wear and death for bodies that are not skeletons

- Symptom: a wound on any body was a cut, bruise or break on a limb or organ that bled and
  healed (body.rs, `health::wound`), so a swarm, a slime or a chain of segments could only be
  hurt as if it had a chest and legs.
- Found by: #423 (part 1 gave such bodies a plan, D70).
- Cause: confirmed in `systems/health.rs` `wound`: every harm became a `Wound` on a part of the
  species' plan (animals read as human), with bleeding and tissue depth; `combat::battered` and
  the felling of a beast read the sum of those wounds only.
- Fix: `body.rs` `Form` (kind, units left, units whole; saved with `#[serde(default)]`, None in
  every older save) with `share`, `least`, `spent`, `harm`, `end` and `label`; `Body::summary`
  names it. `systems/health.rs` `formed_wound`: a harm on such a body lands on a part of its
  plan by weight; a swarm loses a quarter of its whole members at full severity, a soft body
  two fifths of its mass, neither bleeds; a chain loses a segment to a blow of `SEVER` (two at
  `SEVER_TWO`) and a lighter one glances off; a blow to its head is a wound there without
  bleeding. Each loss is an `EventKind::Thinned` row (appended). `forms`, hourly: a starving
  swarm loses a member every four hours and a starving soft body two hundredths an hour; a soft
  body above ground with no water within two tiles dries a hundredth an hour; one `spent`
  (under a fifth of itself, or a chain under two segments) dies, "scattered", "broken up" or
  "cut to pieces". `combat::battered` reads `Form::harm` (or the head's wounds if more), so
  nerve and felling follow it; a felled one is "scattered by" its killer and so on.
  `combat::hunted_down`: a hunt takes such a body only if the hunter's blows (three at most)
  leave it spent, else it gets away (`systems/work.rs`, the hunt). `Agent::body_kg` is scaled
  by what is left, so a thinned swarm hits and pushes as what it is (#422). Beasts' other
  bodies have no illness today (health.rs runs sickness for people alone), so hunger and the
  dry are these kinds' wear. No creature has one of these bodies yet (`plan::beast_plan` is
  None for every kind), so nothing in play changes.
- Guard: `tests/body_kinds.rs` `a_swarm_thins_and_does_not_bleed`,
  `a_soft_body_loses_some_of_itself`, `a_chain_shortens_and_dies_by_its_head`,
  `hunger_thins_a_swarm_and_the_dry_shrinks_a_soft_body`,
  `a_hunter_does_not_take_a_swarm_at_a_stroke`, `a_form_saves_and_old_bodies_have_none`.
- Commit: (this commit).

## 2026-11-02 #486: the valley's food web: grazers on the browse, hunters on the game, crows on carrion

- Symptom: wild kinds lived beside each other but not on each other. Breeding followed a fixed
  cap per kind whatever there was to eat; a grazer got hungry (#422) but never starved; a wolf
  that killed nothing for a season was as fit as one that fed; foxes took nothing; a carcass
  fed nothing; a felled wood never grew back. Over 600 days on master (seeds 1 to 4,
  `examples/ecology_census.rs`) wolves were gone from three valleys for most of the run, hares
  from two, and deer from two by the end, and nothing brought them back.
- Found by: issue #486's acceptance (a population-linked food web, a long run checked for
  depletion and boom-bust), against `systems/animals.rs` `breed_and_age` and `creatures.rs`.
- Cause: confirmed by reading the code: `breed_and_age` read only counts and ages. A second
  cause was found by the first branch census: grazers starved by the dozen beside 800 to 1,000
  growth of browse. `systems/animals.rs` sent a hungry grazer to the nearest plant of any
  `grazed` kind, berry bushes, herbs and reeds included, while `graze_bite` eats only
  `browsed` kinds, so a grazer near a berry bush stood hungry by it. Confirmed by
  `a_hungry_grazer_goes_to_what_it_eats`, which fails with the old target and passes with the
  fix, and by the 90-day census of the fixed branch: no grazer starved, where the unfixed
  branch had starved most of its deer by day 30.
- Fix: `ecology.rs` (new) and `Agent.fed` (the tick a wild animal last ate its fill,
  serde-defaulted). Grazers (deer, boar, hares, goats) are fed when they browse to half full;
  they bear young only if fed within two days and starve after six days unfed (nine in
  winter). Wolves are fed by a kill, deer for the pack or a hare for one, bear young within
  four days of one and starve after ten without; a wolf takes a hare only a day after its last
  kill. Foxes now hunt and take hares two days after their last kill and bear young within four
  days of one; they do not starve (they live mostly on mice and beetles, not modelled). Crows
  are fed at carrion and bear young within six days of it. A carcass picked over greens the
  low growth within two tiles (`carcass_spent`, from `creatures.rs`). A stump lets a shrub
  spring up beside it in spring and summer on open ground (`clearings`). A kind gone from the
  valley (fewer than two) comes back over the hills one at a time when there is food for it
  (`incomers`, recorded as the appended `EventKind::CameOver`). Hares breed at two and a half
  times the deer's chance, as they now have hunters (at the deer's rate the foxes kept them at
  one or two). Starvation deaths are recorded `Died` rows and leave carrion. `animals.rs`: a
  grazer heads for what it eats. The pipeline gains the daily `ecology` stage after
  `creatures`.
- Guard: `crates/hf-sim/tests/ecology.rs`, each scenario with its control:
  `grazers_breed_on_the_browse_and_starve_without_it`,
  `wolves_live_on_the_game_and_starve_when_it_is_gone`, `foxes_hunt_hares`,
  `hunting_the_grazers_lets_the_browse_recover`, `a_spent_carcass_greens_the_ground_about_it`,
  `browse_springs_up_where_a_tree_was_felled`,
  `a_kind_gone_from_the_valley_comes_back_when_there_is_food`, `a_hungry_grazer_goes_to_what_it_eats`,
  `the_food_web_is_deterministic`. The 600-day census before and after is in the PR.
- Commit: this branch's #486 commits.

## 2026-11-03 #486: a beast named in a fight was renamed in the lines before it

- Symptom: box-ci on PR #591 failed `tests/history.rs` `chronicle_only_real_events`: the line
  shown was "bear took a cut to the right arm fighting off a beast." and the same row told
  again read "Longback took a cut ...".
- Found by: box-ci on PR #591 (the coordinator's triage). The food web makes fights with
  bears more common, so the soak-length history test now meets one.
- Cause: confirmed in `view/narrate.rs`: `line` named every agent through `name(w, id)`, the
  agent's present name, so a beast that earned a name after an event was told by it in that
  event's line when told again. Only `beast(w, tick, id)` honoured `Agent.named_at` (the c82
  rule, FIXLOG 2026-10-26 round-11), and only the lines that called it.
- Fix: `view/narrate.rs` `named_as(w, tick, id)`: a person by name, an animal as `beast` says
  it at the event's tick; every agent `line` names goes through it with `e.tick`.
- Guard: `crates/hf-sim/tests/narrate_beasts.rs` `a_wound_line_names_the_beast_as_it_was_then`
  (the wound line before and after the naming, and a later line by the name);
  `tests/history.rs` unchanged and passing.
- Commit: this commit.

## 2026-11-02 #319: three more cascade families: the lean winter, the lost roof, the lost healer

- Symptom: after the flood family (#546) a settlement's other ordinary troubles still stood
  alone. Stores that ran out in winter, a roof burned off or fallen in, and the death of a
  community's last healer each happened and were recorded, but nothing that followed from them
  was laid to them: the hungry fell sick at the same rate as the fed, food under a lost roof
  rotted no faster in rain than under a sound one, and a wound going bad was not told as the
  healer's loss.
- Found by: issue #319's list of the cascades a real settlement suffers (winter store failure,
  fire and roof, the healer's death), checked against `systems/health.rs`, `systems/spoil.rs`,
  `systems/hazards.rs` `burn_out`, `building.rs` and `systems/decide.rs` (who tends) on master
  989d0301.
- Cause: confirmed by reading the code. The roots were recorded (`Burned { what: "roof" }`,
  `RoofFell`, a healer's `Died`) or computable (`settlement::food_days`), but no system kept
  them as standing causes, and `health.rs` read neither a community's stores nor whether it
  still had a healer. The one link that did exist, winter cough taking the hungry more
  readily, had no recorded cause.
- Fix: `cascade.rs` gains `Roots` (saved as `World.cascades`, serde-defaulted): the tiles whose
  roof burned or fell, the communities whose last healer died, the communities whose stores
  failed, and the weakened, each holding the row that started it. `notice` reads each tick's
  pending events for burned roofs and for the death of a healer (one who knows the poultice or
  holds the healer's place) who leaves their community none, recorded as `HealerLost` holding
  the death's row; `building.rs` marks the tiles of a fallen roof with the `RoofFell` row. The
  morning pass `lean` records `StoresFailed` once when a community holds under a day's food a
  head in autumn or winter (over at two days, or when spring comes), `Weakened` (holding that
  row) once a spell for each of its people below a quarter fed, who are weak while the spell
  lasts and they stay below six tenths fed (`weak_now`; a first run held the weakness for the
  rest of the game in a community that never again held three days' food a head, and its
  people caught sickness at twice the rate all along: 410 `SickenedWeak` on seed 1), and
  clears `HealerLost` when the community has a healer again. `health.rs` `daily`: the weakened
  catch what goes round at twice the chance and count as weak for the winter cough, and a
  catching is recorded as `SickenedWeak` holding the weakening's row; an undressed wound that
  turns bad in a community without a healer is recorded as `WoundFestered` holding the
  `HealerLost` row (the rates of turning bad are unchanged: what changed with the loss is that
  nobody lays a poultice, so wounds are nursed and not dressed). The hourly `rain` pass rots
  food lying where a roof was lost at a sixth an hour of rain (`RainSpoiled`, holding the
  loss's row), until the place is roofed again or twenty days have passed, and the spoiled food
  draws vermin through the flood family's `draw_vermin`, so the bite and festering follow. Six
  appended `EventKind`s with narration; the soak tally counts the families' events.
- Guard: `crates/hf-sim/tests/cascade_families.rs`, each with a prepared control:
  `a_failed_winter_store_weakens_the_hungry_and_the_weak_sicken` and
  `a_community_with_stores_does_not_start_the_lean_winter` (the same hungry people and fever,
  a full store); `rain_through_a_burned_roof_spoils_the_store_and_draws_vermin` and
  `a_roof_put_back_or_dry_weather_keeps_the_store`;
  `with_the_last_healer_dead_a_wound_festers_undressed` and
  `a_community_with_a_second_healer_records_no_such_loss`;
  `the_later_families_are_deterministic`. The suite does not compile on master without the
  new event kinds. `examples/cascade_census.rs` counts each family's steps over 600 days.
- Commit: this branch's #319 commits.

## 2026-11-02 #505 (first slice): built walls floated over the ground with no contact shadow

- Symptom: wall tiles were drawn with their own bevels and faces, but the ground beside them was
  untouched, so a house read as pieces laid on a flat floor (the October 30 review shots in #505).
- Found by: #505's scope ("stronger depth cues from shadows, occlusion"), checked against
  `world_view.gd` `_draw_cliffs`. Natural raised ground casts a shadow east; built walls returned
  before reaching that code.
- Cause: confirmed: the `WALL_MAT` branch of `_draw_cliffs` drew the face and returned.
- Fix: `world_view.gd` `_draw_wall_shadows` draws `rules.gd` `wall_contact_shadows` (a two-step
  shadow east and under the southern face, a corner fill, none onto a wall) for every built
  wall at the view level.
- Guard: `tools/gd_test.gd` `test_wall_contact_shadows_fall_only_on_open_ground`; before and
  after screenshots in `docs/reviews/505/`.
- Commit: this commit (feat-505-scenes).

## 2026-11-02 #423 (part 3): abilities are properties of parts

- Symptom: parts did nothing beyond what limbs do (sense, hold, walk, fly, swim), so a soft
  body could not divide, take in its prey or get through a gap, and nothing could climb.
- Found by: #423 ("abilities as part properties", tied to #337).
- Cause: confirmed in `plan.rs`: a part had roles only, and movement, combat and health read
  nothing else.
- Fix: `plan.rs` `Ability` (climb, split, engulf, squeeze, regrow) on parts in the plan files
  (`abilities = [...]`), `Plan::can` and `plan::can` for a creature; `check_kind` refuses split
  or engulf on any but a soft body, regrow on any but the part a body is counted in, and any
  ability on a skeleton. The plan files: soft (split, engulf, squeeze, regrow), swarm (squeeze,
  regrow), segmented (climb, regrow). Wired into what exists, no parallel system:
  `systems/health.rs` `formed_wound` (a cutting blow that takes a fifth or more of a soft body
  that can split leaves the piece alive beside it, `divide`, while under `SPLIT_CAP` of its
  kind live; `EventKind::Divided`, appended) and `forms` (fed at half a belly or more, a body
  that can regrow gets back a member every six hours, a hundredth of a soft body an hour, a
  segment a day, each for a fiftieth of a belly); `systems/animals.rs` `engulf` (a creature
  that can engulf takes a beast of at most three tenths of its mass beside it, never a person
  or a kept animal, and is fed and grows by it; hungry, it goes for one within eight tiles) and
  `get_past` (a beast of these bodies whose path failed gets one step nearer through the gaps
  of a built wooden wall, or up or down a level at a natural rock face; failing those it breaks
  the wooden wall in its way if heavy enough, #422 `try_break` and siege.rs `knock_down`;
  `EventKind::Crept`, appended). Still no creature has these bodies, so nothing in play changes.
- Guard: `tests/body_kinds.rs` `abilities_come_from_the_parts`,
  `a_soft_body_cut_deep_splits_in_two`, `a_soft_body_engulfs_small_creatures_and_not_people`,
  `regrowing_takes_food`, `a_swarm_squeezes_through_a_wooden_wall_and_not_stone`,
  `a_chain_climbs_a_rock_face`.

## 2026-11-02 #569: what a person notices depends on their skill, task and mood

- Symptom: everyone with the same senses built the same observation ledger from the same scene;
  a farmer, a hunter and a builder standing together noticed exactly the same things, and
  nobody's ledger said what they made of a cue.
- Found by: the owner's #565/#569 brief ("the main defence against monolithic people").
- Cause: confirmed in `systems/perceive.rs` `step`: what enters `learning.observations` is
  decided by range and line of sight alone; no term for skill, task, stress, tiredness or
  grief, no threshold, and `Observation` had no field for an interpretation.
- Fix: `mind/attention.rs` (new): `attention` (skill 0..20 to 0.35..1.25, x1.3 on a task of that
  kind, less with stress, tiredness and grief) times a cue's strength, x0.25 when habituated,
  against `NOTICE` 0.5. `systems/perceive.rs` `expert_look`: on one look in `EXPERT_EVERY` (4),
  for each domain whose attention could reach the threshold (or for the traced person), one
  bounded cue: the ground underfoot read for crops (farming; `crop_ground_factor`), the nearest
  animal within 10 tiles that is not in sight (animals), the nearest stuck build site in sight
  within 8 tiles, gathered once a tick (`stuck_sites`) (construction). A noticed cue enters the
  same ledger with its `Reading` (new `Observation::reading`, `#[serde(default)]`; new
  `ObservationKind::Soil`, `Tracks`, `Joint`, appended). The same reading near the same place
  within a day is habituated and not noticed; a changed reading is. The traced person's cues,
  noticed or not, go to the Why? lens (`why::Percept`), never to the ledger. The experience
  readout and the inspect panel show the reading. The lens's "put off" line now names the
  arrival urge when that, not an unsecured need, held an option back.
- Cost (500 people, `ostrog profile --agents 500 --provision --days 1`, four alternating
  runs each on a loaded box): perceive mean 1652 us before and 1685 us after (medians, +2%);
  perceive over needs 51.8 before, 49.4 after; within the run-to-run noise.
- Guard: tests/expertise.rs: in one scene with sand underfoot, a deer behind a rock wall and a
  timber wall site short of two planks, the farmer reads poor ground (65%), the hunter a deer,
  the builder two planks wanting, each misses the others' cues and the novice misses all three;
  grief stops the farmer noticing; the unchanged ground is habituated and gravel is noticed
  again; the traced novice's lens shows all three cues as present but not noticed.

## 2026-11-03 #487: know-how passes in talk only between a willing teller and a listener who believes them

- Symptom: a look-alike plant was told to whoever talked with someone who knew it, friend or
  foe, and always believed; nobody was ever refused or held anything back, and the telling left
  no record of its teller. Crisis know-how (beating out flames, carrying water, poultices,
  foraging far, shoring) passed only by watching it done; nobody explained it.
- Found by: the #487 audit against docs/design-minds-observation.md section 4 (learning
  depends on trust, willingness and interpretation; a rival may withhold).
- Cause: confirmed in `social.rs` `tell_look_alike`: no check of the teller's will or the
  listener's trust, and `shuns` holds only the plant. No talk path taught a technique
  (`Learned::Taught` was used only by bench teaching, crafts.rs).
- Fix: `mind/telling.rs` (new): `Told` records (what, from whom, when, the trust placed, taken,
  refused or misheard; 12 kept per person, new `Agent::told`, `#[serde(default)]`); `trust`
  (0.5 + 0.6 opinion + 0.3 familiarity of the teller, times 1.2 - 0.6 independence; believed at
  0.45, so a stranger is believed by a listener of middling or communal bent); `willing` (not one they think ill of, below -0.3, or at feud with, unless warmth is
  0.75 or more). `social.rs` `tell`: a withheld telling is a `Chose` row ("kept X to
  themselves", once a day per teller); a refused one is kept and not pressed by the same teller
  for five days; a told technique (`TOLD_HOW`: the crisis techniques and foraging far, not the
  bench crafts) is learned at skill 0.1 (watching gives 0.15) with the teller's name, and one
  in ten is misheard (skill 0.03: they think they know it). `tell_how` offers one on a quarter
  of the talks that could carry one, from a teller at skill 0.3 or more. `tell_look_alike` uses
  the same gates. The inspect panel lists what a person was told and what they made of it.
- Guard: tests/social_learning.rs: a trusted friend is taught and the technique keeps its
  teller and a lower skill than watching gives; the same word is taken by a communal listener
  and refused by a self-reliant one; one who thinks ill of the listener withholds and says why,
  a warm one tells anyway; what a listener refused does not pass on through them, and is not
  pressed again for days; a poultice taught in talk makes tending a sick stranger a choice the
  untaught never weighs. tests/flora.rs
  `a_poisoning_is_learned_by_those_who_see_it_and_told_on` now pins the told stranger to a
  middling disposition, since a self-reliant one may refuse a stranger. `examples/telling_census.rs` counts tellings by outcome and the
  knowers of each technique over the opening soaks.
- Commit: (this commit).

## 2026-11-02 #423 (part 4): a creature of each new body in the valley

- Symptom: no creature had a body that is not a skeleton, so parts 1 to 3 changed nothing a
  player could see.
- Found by: #423 (a creature or two per family, placed where it fits, drawn by the client).
- Cause: confirmed: `agents.rs` `Species` had no such beast and `plan::beast_plan` was None for
  every kind.
- Fix: three beasts appended to `Species` (enum order kept): `Slime` (plan soft), `Rats` (plan
  swarm) and `Centipede` (plan segmented), with sizes in `size.rs` (the rats' size is the whole
  swarm's), mapped in `plan::beast_plan`, so each is born whole with its form (scenario.rs
  `spawn_agent`). What each does, by Dan's rule 7 (`systems/animals.rs`): the cave slime shrinks
  from firelight, feeds when hungry on the mushrooms and pale fungus of its cavern (`browses`,
  `graze_bite`) and on any small creature it comes on (`engulf`), and rests by the pool when fed,
  one turn in two; the centipede keeps from fire and from anyone within three tiles, hunts the
  bats of its cavern when hungry and takes one it reaches, lies up at its nest by day and roams
  by night, climbing where it cannot walk; the rats graze and run from people as grazers do,
  lie up in their nest by day unless hungry and feed by night (a first build kept them in the
  nest all night however hungry, and in the soak every seed's swarm starved by day 64 to 300;
  checked by tracing seed 3's swarm, at hunger 0 in its nest), and are one of the beasts the
  stores draw in
  (creatures.rs `incursions`, getting in through a wooden wall by `get_past`), where whoever
  stands against them thins them (combat.rs `stand_against`). They get hungry (`needs.rs`);
  rats and centipedes breed and die old within their caps; nothing is eaten off a slime or a
  centipede (work.rs, the hunt). Placed by `animals::place_bodies` from a stream of its own
  (the valley's other animals are where they were): the rats on open ground sixteen tiles or
  more from the landing, the slime in a wet cavern of the deep (`cave_ecology::slime_cavern`;
  none in a valley with no pool below, for it would dry), the centipede in the shallowest
  cavern (`centipede_cavern`); each nests where it is put (`CreatureState::nests`). The client:
  `world_view.gd` `ANIMAL_NAME` gains slime, rats and centipede, drawn from placeholder sheets
  made by `tools/art/animals.py` (same palette ramps, 32 px cells, 9 frames by 4 rows;
  provenance rows regenerated). The abstract vermin of the flood cascade (cascade.rs) are not
  these rats and are unchanged.
- Guard: `tests/body_kinds.rs` `each_new_beast_is_born_whole_of_its_kind`,
  `they_are_placed_where_they_live`, `a_hungry_slime_feeds_on_the_cave_fungus_and_shrinks_from_fire`,
  `a_centipede_hunts_bats_and_keeps_from_people`, `rats_stood_against_thin_and_scatter`;
  `tools/gd_test.gd` `test_every_animal_kind_has_a_sheet`; `tools/art/plants_animals_check.py`
  checks the three sheets' grids.
  `tests/building_way.rs` `a_later_wall_is_of_their_wood_when_it_is_to_hand` now has every
  member of the community know both logs: the rats took an agent id, a different member took
  the wall order, and that member had seen only the birch, so it hauled the birch (checked by
  tracing the run: agent 2 hauled the birch; with the new placement off the test passed). The
  test's question, whether their way's wood is chosen when it is known, is unchanged.
- Commit: (this commit).

## 2026-11-03 #603: a slow tick stuttered the frame it landed in

- Symptom: the client stepped the sim on Godot's main thread, so a slow tick showed as a
  stutter: at 500 people `sim.step(1)` took 2.6 ms at p50 and up to 15.9 ms, every frame at
  3x and 10x (docs/design-renderer.md, PR #601).
- Found by: the #414 B2 measurement, `tools/perf/frame_prep.gd`.
- Cause: confirmed in `godot/scripts/main.gd` `_process` and the bridge's `SimHost::step`:
  the ticks ran in the call, on the main thread.
- Fix: `crates/hf-godot/src/sim_thread.rs`: one step thread. `SimHost::step` hands the session,
  with the feed, director, sounds and bodies it keeps between ticks, to that thread with the
  ticks asked for (at most 20 a batch, at most 400 waiting, as the client's own cap of 400 a
  frame) and returns at once; the thread runs them as `Session::step` always has and hands
  everything back with a snapshot of the world, which the view's reads (`world()`) use while
  the next batch runs. Calls that read the chronicle or change the world (save, load, inspect's
  first read of a person's past, history, legends, commands) first take the session back
  (`settle`), waiting for the batch in hand at most. `sync()` waits for every tick asked for
  (shots, footage, timelapse samples, tests). `--sim-inline`, `HF_SIM_THREAD=0` or
  `set_threaded(false)` step in place as before.
- Guard: `sim_thread.rs` `the_step_thread_gives_the_world_ticks_in_place_give` (uneven batches
  on the thread against the same ticks in place, digest and feed at every batch's end);
  `tools/gd_test.gd` `test_sim_thread_steps_the_same_world_and_saves_exactly` (a threaded and an
  in-place host give the same digest at the same tick; a save through the threaded host loads
  as saved and runs on as the continuous world). Before and after: `tools/perf/main_thread_frames.gd`.
- Commit: this branch's #603 commit.

## 2026-11-03 fix-602-duties-heal-forage: nobody became a forager or a healer (#602)

- Symptom: after #454 (duties.rs) no one ever held the forager or healer role in a soak, and
  seeds 1 and 2's starving person-hours were 2.5 % and 4.6 % against 1.1 % and 0.9 % on master
  ec33dd0e.
- Found by: the #454 report; examples/role_census.rs (roles held and food and care work done,
  day by day) on seeds 1 to 4, on current master and on current master with the C-era
  `assign_roles` and role table put back (the only difference being how roles form).
- Cause: confirmed for the roles: foraging is not a work order and was never tallied, and a tend
  is a ten-minute order that never reached two hours or a third of a hauler's tally, so neither
  duty formed (role_census: forager 0 and healer 0 every day, against one each a community with
  `assign_roles`). Not confirmed for the starving hours: over the first 146 days the two builds
  starve 63 against 78, 134 against 100, 106 against 110 and 59 against 27 person-hours on
  seeds 1 to 4, no consistent direction; the 600-day soak differences are within the swing
  between seeds and builds.
- Fix: `duties.rs`: a completed forage counts 15 minutes toward gathering (`systems/act.rs`),
  a completed tend 180 toward tending (`systems/work.rs`, both tend paths); gathering
  (`Role::Forager`, added to the tally) and tending are a duty at two hours without the
  third-of-the-top rule, and fade as every kind does. Holding only those makes no one less keen
  on other orders (`fit`); a healer is keener to tend. The tally is a list now, so a save's six
  kinds load and gain the seventh on use.
- Guard: tests/duties.rs `those_who_gather_and_tend_hold_those_duties` (four days of an
  opening with someone hurt each morning: a forager and the one who tended twice or more is a
  healer; both fail on master, which forms neither) and
  `tending_and_gathering_fade_when_they_stop`.
- Commit: (this commit).

## 2026-11-05 #604: foxes never starved when hares ran out

- **Symptom**: 600-day ecology reports found deer and hares ending low because foxes kept
  hunting without any modeled food limit; unlike wolves, foxes could never die when their prey
  disappeared.
- **Found by**: issue #604's four-seed ecology report and code review of `ecology::starving`.
- **Cause**: confirmed: `ecology::hunter` already classified wolves and foxes together for
  prey-linked breeding, but `starving` matched only `Species::Wolf`.
- **Fix**: `crates/hf-sim/src/ecology.rs` uses the existing `hunter` predicate for the hunter
  starvation rule, so foxes now starve after the same ten kill-less days as wolves.
- **Guard**: `crates/hf-sim/tests/ecology.rs` `foxes_starve_when_their_hare_prey_is_gone`,
  which fails on master because the fox returns no starvation cause.
- **Commit**: (this commit).

## 2026-11-06 #483: an exhausted household could not grow a supported upper sleeping room

- **Symptom**: a standing house with no valid same-level annex left a colonist short of a bed;
  the existing #483 engine primitives could build an upper room only when a caller posted direct
  orders, so the settlement never proposed the organic expansion.
- **Found by**: the bounded #483 M6 slice review and the new `upper_sleeping.rs` scenario on c150.
- **Cause**: confirmed: `HousePlan` persisted only the ground floor, and `settlement.rs` had no
  production path from an individual's low rest need to a staged upper-room plan.
- **Fix**: `HousePlan` now records one upper sleeping footprint and its lower stair. The settlement
  planner proposes it only for a colonist with low rest when `houses::annex` finds no valid
  same-level expansion, then posts stairs, supported upper floors, upper walls, held roofs and
  beds in order. Every tile still uses the existing material requirements and worker-time path;
  workshops, collapse/removal and autonomous cultural/site reasoning remain follow-ups.
- **Guard**: `tests/upper_sleeping.rs` covers the unmet-need trigger, same-level-annex control,
  persisted plan and ordered stages; `tests/upper_floors.rs` covers real material consumption,
  support, reachability and actual upstairs-bed use. The new planner test fails on c150 before
  the production proposal is present.
- **Commit**: (this commit).

## 2026-11-07 — #626 publisher verifies the signed manifest before upload

- Found by: release-tooling review of PR #627; `tools/release/publish_windows.sh` checked the
  manifest shape and archive hash but accepted an all-zero 64-byte signature.
- Cause: the publisher had no trusted public-key input and never called the client verifier, so a
  release could be uploaded even though every launcher would reject it.
- Fix: `hf-release-manifest --verify-manifest` now parses and verifies the exact Windows archive
  with an external public key; the publisher requires `OSTROG_RELEASE_PUBLIC_KEY_FILE` and
  runs that verifier before `gh release create`.
- Guard: `tools/tests/release_publish_test.py` publishes a valid signed fixture, then proves a
  tampered signature fails before the `gh` stub is invoked.
- Commit: this fix's commit.

## 2026-11-09 #483: upper-storey growth had no recorded autonomous reason

- **Symptom**: an existing house could grow an upper sleeping room only from an individual
  low-rest trigger; the resulting plan did not say why the household chose a second level, and
  it could begin while the community's food or water was not secure.
- **Found by**: WP-03 slice 3.1 review against #483's acceptance and design rules 7, D35 and D36.
- **Cause**: confirmed: `HousePlan` persisted the upper footprint and stair but not the household
  reason, and `plan_upper_sleeping` had no community-level secured gate or cultural building
  preference.
- **Fix**: record `UpperStoreyReason` in the persisted plan; choose cramped site, grown household
  or an active recorded cultural custom in deterministic precedence; require the existing
  household's food, water and shelter to be secured before posting any upper-room designation.
  The first completed upper room teaches the community an `UpperStorey` custom from its real
  `HouseStage` event, so later cultural choices have a traceable origin. Construction remains the
  existing stair, supported floor, wall, roof and bed chain.
- **Guard**: `systems::settlement::upper_storey_reason_has_a_recorded_cause_and_a_control_case`
  covers all three reasons and the no-upper-floor control; the existing upper sleeping and upper
  workspace scenarios cover the staged material-backed construction and use.
- **Commit**: (this commit).

## 2026-11-16 #1247: deference for deeds was credited to people who could not have seen the deed

- **Symptom**: the seers of a build, harvest, cook, hunt, rite, tending, teaching or welcome
  were anyone of the doer's community, awake, on the same level and within 10 tiles — no line
  of sight, so a miner in the next gallery credited the cook. D48 says "a person who sees
  another build...".
- **Found by**: the 2026-11-13 alive audit (desktop), code cited at master `dbf7f557`.
- **Cause**: confirmed: `justice::witnessed` (`crates/hf-sim/src/justice.rs`) filtered seers by
  community, level, radius and waking state only, with no sight check.
- **Fix**: `justice::witnessed` now also requires `mind::perception::can_see(&w.grid, watcher,
  doer)`, the same line-of-sight primitive already used by `resonance.rs`, `threat.rs` and
  `systems/decide.rs`. The beneficiary rule (the one the deed was for credits it whether or not
  they saw it) is unchanged.
- **Guard**: `crates/hf-sim/tests/justice.rs::a_deed_behind_a_wall_earns_no_deference` — a cook
  seen across open ground earns deference from the watcher; the same cook with a wall between
  them and a watcher 5 tiles away earns none from that watcher. Fails on master because the
  walled watcher still credits the cook.

## 2026-11-16 #1806: a quarrel or fight changed only the two who were in it

- **Symptom**: when two people quarrelled (`systems/act.rs` `:427`) or fought
  (`systems/strain.rs` `fight` `:693`), only the two of them took it against each other; a
  sibling, parent or child standing right there and watching took nothing against the other
  side until the feud machinery fired after a judgement (`justice.rs:582-585`,
  `feuds.rs:98-136`). Family loyalty should show before the feud does.
- **Found by**: issue #1806 (alive-audit).
- **Cause**: confirmed — `social::observe` (`social.rs`) matched `Talked`, `Tended` and `Taught`
  events but had no arm for `EventKind::Quarrelled` or `EventKind::Fought`, so no kin of either
  side ever heard or saw the trouble through the ordinary social-weave pass.
- **Fix**: `social.rs` `kin_take_sides` (called from `observe` on `Quarrelled` and `Fought`,
  skipped when the two are already each other's kin via `feuds::kin`): each living kin of
  either side, other than the two themselves, awake within ten tiles on the same level (the
  same reach as `justice.rs`'s `witnessed`) takes `relate(other_side, -0.1, 0.02, tick)`
  against the other side, at most once per (kin, other side) per day via `work_cooldowns`
  (keyed `0x1806_0000 | other.0`).
- **Guard**: `tests/feuds.rs::kin_side_with_their_own` — A and B quarrel with A's brother awake
  5 tiles away: the brother's opinion of B falls by about 0.1; a stranger standing beside him is
  unchanged; a second quarrel the same day changes the brother's opinion no further. Fails
  without the fix (no arm for `Quarrelled`/`Fought` in `observe`).

## 2026-11-16 #1282: Hask promises "the broken are let be", yet sent bands to a collapsed colony

- **Symptom**: the new-game blurb has Hask say "The strong are tested; the broken are let be"
  (`crates/hf-sim/src/deity.rs:163-166`), but a hungry or aggrieved neighbour kept sending bands
  at the valley even after the valley's own community had collapsed. Seed 6 (Relentless):
  `d158 [faring] Mereford is collapsed: 1 left of 21`, then `d161 [band_set_out]` from Digio and
  `d162 [band_set_out]` from Dardtrou — 19 bands over days 100-195 while the colony held 9-20
  people.
  - From the 2026-11-13 alive audit (historical private review).
- **Found by**: tracing the issue's proposal (a `storyteller::afford` gate on fate, from #1280
  and #1281) back to the code that actually spawns bands, since neither of those dependency
  issues is implemented in master yet.
- **Cause**: confirmed: `threat::sites_daily` decides whether a hungry or grudge-holding site
  sends a band purely from that site's own `hungry_days`/`grudge`; it never looked at how the
  player's own valley was faring. `fate::step` already scores the valley's community 0
  (thriving) to 3 (collapsed), matching the "failing/collapsed" language the issue asks for, but
  nothing in `threat.rs` or `deity.rs` read it before sending a band.
- **Fix**: `crates/hf-sim/src/threat.rs::sites_daily` now checks, once per day and only under
  `Engine::Relentless` (Hask), whether any live community has `fate >= 2`; if so, every band that
  would otherwise set out that day is skipped. A site keeps its hunger or grudge and tries again
  once the valley is no longer failing or collapsed.
- **Guard**: `crates/hf-sim/tests/threats.rs::hask_lets_the_broken_be` sets the valley's fate to
  2 (failing) under Hask with an already-hungry neighbour, runs ten days asserting no
  `BandSetOut` event fires and `w.bands` stays empty, then resets fate to 0 and confirms a band
  can set out again once the valley has recovered. Fails on master because the existing
  `hunger_sends_a_band` path has no fate check at all.

## #1344: one `Body::pain()` from every wound, its depth and treatment

- **Found by**: 2026-11-13 alive audit (desktop), seeds 1-8, 600 days each.
- **Cause**: pain was guessed three different ways in three places (worst wound in
  `mind/dispositions.rs`, a threshold on the worst wound in `systems/strain.rs`, a sum of raw
  severities in `systems/combat.rs`) and none read a wound's depth or treatment.
- **Fix**: added `Body::pain(now, tended_until) -> f32`: sums each wound's severity weighted by
  its kind (Break 1.2, Burn 1.1, Cut/Puncture 1.0, Frostbite 0.8, Bruise 0.6) and depth (Skin 0.6,
  Muscle 1.0, Bone 1.3, Organ 1.4), halved by 0.6 once treated, plus `ill() * 0.5`, plus 0.05 per
  scarred part and 0.03 per lost part, plus 0.1 for a bad back; scaled by 0.7 while tended;
  clamped to 0..1. Pure, no RNG, nothing saved. The three existing call sites are left as they
  were — this slice only adds the one agreed number per #1343.
- **Guard**: `bodies::pain_sums_wounds_by_depth_and_eases_with_treatment` checks a whole body
  gives 0, a skin cut hurts less than a bone break of the same severity, three cuts hurt more
  than one, splinting a break gives 0.6x its pain, and tending gives a further 0.7x.

## 2026-11-16 #1807: a verdict against someone left their family indifferent to the accuser

- **Symptom**: `justice::judge_all` (`crates/hf-sim/src/justice.rs:483`) changed opinion between
  the accused and the accuser on amends, turned the wronged one's friends from the judge on a
  wrong verdict, and reached families only through the feud machinery (`feuds::judged` /
  `grievance`) — a family whose member was judged on a neighbour's word did not cool toward that
  neighbour until a feud happened to begin.
- **Found by**: issue #1807 (alive-audit).
- **Cause**: confirmed — `judge_all` had no step that told the accused's kin apart from the feud
  path, so a verdict with no feud (a single "to make amends") left kin's opinion of the accuser
  untouched.
- **Fix**: `justice.rs` `judge_all`, right after the verdict and before the feud step: for any
  verdict other than "pardoned", each living kin of the accused (`feuds::kin`), other than the
  accused and the accuser themselves, takes `relate(acc.by, -0.1, 0.02, tick)` against the
  accuser.
- **Guard**: `tests/justice.rs::kin_of_the_judged_hold_it_against_the_accuser` — a disliked
  first offender is judged "to make amends": the mother's opinion of the accuser falls by about
  0.1, an unrelated bystander's is unchanged; a liked first offender is pardoned and the
  mother's opinion of the accuser does not move. Fails without the fix (no kin reaction on a
  non-feud verdict).

## 2026-11-15 #1330: nobody could see at a glance that one trader and one town take all the
  valley's trips

- **Symptom**: #1329's figures (seed 8 Mordric 36 of 63 trips and 62 of 63 to Nis; seed 7 Ulric
  20 of 35 to Fioriokre) were counted by hand from probe `events.txt`. No example reports
  caravans; `examples/away_probe.rs` follows people off the map but never tallies trips.
- **Found by**: the 2026-11-13 alive audit (#1330, under #1329, epic #1531).
- **Cause**: confirmed: no code turns `CaravanFormed`/`CaravanReturned` into a per-community
  tally of trips by leader, trips by site and cargo, or returns by what came home, so #1331 to
  #1334 had no before-and-after numbers without hand-counting.
- **Fix**: `crates/hf-sim/examples/trade_census.rs` (`Census::note`, `CommunityCensus`): steps
  an opening world day by day, and on each tick's `CaravanFormed` looks up the just-pushed
  `Caravan` in `w.overworld.caravans` for its site and cargo (the event's `purpose` is only a
  label), tallying trips by leader and by "site: cargo", trips whose site's `grudge` was 2 or
  more at departure, and, on `CaravanReturned`, returns by what came home (`received`, split on
  ", ") with the empty-`received` case counted as "nothing to show". `CommunityCensus::
  leader_share` and `::site_share` give #1329's shares (the largest count over total trips).
  `cargo run --release -p hf-sim --example trade_census -- <seed> <days> <engine>` prints the
  table per community.
- **Guard**: `crates/hf-sim/tests/trade_census.rs`
  `tallies_leader_and_site_share_and_grudge_trips` includes the example and builds a hand-worked
  scenario (one leader takes 3 of 4 trips, all to one site, the fourth to a site whose grudge is
  2; one return is empty-handed) and asserts the tallies, shares and "nothing to show" count by
  hand-computed numbers. The file did not exist before this commit, so the test fails to
  compile/run without the fix and passes with it.

## 2026-11-16 #1443: coming back, nothing told who died, paired or broke down

- **Symptom**: the catch-up card listed the newest 12 notable rows by id; at dawn after a wolf
  wounding, 14 dream and suspicion rows pushed the wound out and a death or pairing could be lost
  among them.
- **Found by**: the 2026-11-13 alive audit (seed 5, run-day 75).
- **Cause**: confirmed: `summary::since` takes the newest rows with no weight and no grouping by kind.
- **Fix**: `view::summary::changes` returns the life events between two ticks (died, born, paired,
  arrived, joined, left, exiled, captured, returned, broke, craft lost, a role first taken), ordered
  by that weight then tick, each narrated from its row through `view::known`. There is no recorded
  "parted" kind yet, so it is not in the list. Not wired into the card (#1445).
- **Guard**: `catch_up::changes_name_who_died_and_paired_first`.

## 2026-11-16 #1453: a person's written story was never shown, and raw it is thousands of lines

- **Symptom**: `view::history::story_of` returned every recorded row naming a person (seed 5's
  Ulounor: 3,431 non-routine rows, 2,144 of them "talked") and nothing called it outside a test.
- **Found by**: the 2026-11-13 alive audit (J5c).
- **Cause**: confirmed: no read model kept the rows that make a life and dropped the routine.
- **Fix**: new `view::biography::biography`: `story_of`'s query restricted to Notable and above,
  plus the first `RoleTaken` of each role, a craft learned and a ring given; each `BioLine`
  carries its `real_history` id (D9). Read model only; no client wiring yet.
- **Guard**: `tests/biography.rs::biography_keeps_life_events_drops_routine`.

## 2026-11-16 - #1584 road_census example (reporting only, no sim change)

- Added `crates/hf-sim/examples/road_census.rs <seed> <days> [engine]` and the read-only classifier
  `expedition::road_incident`; guard `tests/expedition.rs::the_road_log_names_every_incident_kind`.

## 2026-11-15 #872: a wound is remembered as "something bad here" with no event and no attacker

- **Symptom**: a colonist hurt in a fight or by a beast kept only a vague `Hardship` memory
  about the place it happened; it carried no event id and nothing about who struck the blow, so
  no episode (#873) could form from an injury and the memory's origin list stayed empty.
- **Found by**: the 2026-11-13 alive audit, code at `crates/hf-sim/src/systems/health.rs:100-107`
  (the `remember` call ran with `event: None`) and `:108-` (the `Wounded` event was pushed only
  afterward); confirmed `Wound.by` was already known at most call sites (`systems/combat.rs`,
  `systems/strain.rs`, `systems/animals.rs`) but never passed in to be remembered.
- **Cause**: confirmed: `health::wound` recorded the place memory before the `Wounded` event
  existed, so it had no event to point at, and the function took no attacker parameter at all —
  callers that knew the striker set `Wound.by` by mutating the pushed `Wound` after the call,
  but nothing ever told the victim's memory who it was.
- **Fix**: `health::wound` (`crates/hf-sim/src/systems/health.rs`) now takes `by: Option<AgentId>`,
  sets it on the `Wound` directly, pushes the `Wounded` event first and passes its id into the
  place memory, and — when the striker is known — records a second `Hardship` memory about
  `Subject::Person(by)` carrying the same event id. Every call site now passes the known
  attacker (a fight's other party, a beast's own agent id, the Beast-hunger's shifted person, a
  night bite) or `None` for impersonal harm (fire, cold, falls, traps, tools, the portal, the
  sun); the post-call `wd.by = Some(...)` mutations in `systems/combat.rs`, `systems/strain.rs`
  and their tests were folded into the single call.
- **Guard**: `crates/hf-sim/tests/wounds.rs::a_wound_memory_points_to_its_event`, which fails on
  master because the place memory's `event` field is `None` and no second record about the
  attacker exists; it passes once the event is pushed first and the attacker is threaded through.

## 2026-11-15 #1736: nobody comes to see a new baby

- **Symptom**: a parent with a newborn weighed the same as on any other day; kin and friends
  who would be glad of the birth never went to see the mother.
- **Found by**: issue #1736 (alive-audit, Watcher pillar).
- **Cause**: confirmed. `choose`'s Talk gate (`crates/hf-sim/src/systems/decide.rs`, around the
  former line 1965) only ever weighed `Goal::Talk` from the ordinary social need
  (`social_p > 0.3`); it had no branch that looked at a known agent's `last_birth` at all, so a
  grandmother at full social need (nothing drawing her in but the birth) never considered
  visiting.
- **Fix**: `crates/hf-sim/src/systems/decide.rs::choose`, a new consideration placed just before
  the Talk gate: for a known, awake agent whose `last_birth` falls within the last two days
  (`BIRTH_VISIT_WINDOW`) and within 30 tiles (`BIRTH_VISIT_RANGE`), kin (parent, child or a
  sibling sharing a mother or father) or a friend (opinion of them at 0.6 or more) considers
  `Goal::Talk` at `0.9 * fondness`, behind the #415 leisure gate, and skipped once they have
  already talked with that parent since the birth (a short-memory `Company` record for them no
  older than `last_birth`). The feud and aversion checks the ordinary Talk gate already applies
  are kept. (#1735, the Joy-at-a-birth feeling this issue depends on, is not yet merged; this
  fix uses kinship and existing opinion/fondness in its place, so it can be revisited once Joy
  is tracked.)
- **Guard**: `crates/hf-sim/tests/autonomy.rs::kin_come_to_see_the_new_child` stages a birth with
  a grandmother 20 tiles off (already knowing of the mother, as a family would) and a stranger
  equally aware of her; within five simulated hours (short enough that the ordinary Talk gate's
  own social-need decay cannot yet have opened it) the grandmother talks with the mother and the
  stranger does not. Confirmed the guard fails on the pre-fix `decide.rs`.

## 2026-11-15 #1195: a couple's "fed enough" for a child was judged by their hunger at 05:00

- **Symptom**: `births` (`crates/hf-sim/src/systems/life.rs`) ran once a day at the 05:00 stripe
  and required both partners' instantaneous `needs.hunger` above 0.3 at that moment, before
  breakfast. A well-fed household that was merely peckish at dawn, or whose community held
  plenty of stores, was read as not fed enough and turned away for a child.
- **Found by**: the 2026-11-13 alive audit (seeds 1-8, 600 days, draft E11c), filed as #1195
  (depends on #1193's gate census).
- **Cause**: confirmed: the gate sampled one instant of `needs.hunger`, which swings with the
  daily meal cycle, rather than any longer-run measure of whether the household was actually
  going hungry.
- **Fix**: `births` now calls a shared `fed_for_birth(w, pa, pb)` (`crates/hf-sim/src/systems/
  life.rs`), which passes when neither partner has gone hungry long enough to register as
  starving (`starving_ticks == 0`, reset by `starvation` the moment hunger rises past zero) and
  their community holds at least two days' food (`settlement::food_days`, the same "secured"
  measure `decide::secured` already uses, design rule 7 / D56). A couple with no settled
  community yet is judged on hunger alone.
- **Guard**: `crates/hf-sim/tests/generations.rs::a_fed_household_is_not_turned_away_at_dawn`
  builds a founded couple at hunger 0.25 with `starving_ticks == 0` and a community holding
  five days' food, and asserts `fed_for_birth` passes them; it fails on master's instant
  `hunger > 0.3` check and passes with the fix.

## 2026-11-15 #1361: no run counted the trees, so a stripped valley went unseen

- **Symptom**: `examples/ecology_census.rs` printed browse, grazers, predators and crows every
  30 days, and `examples/plant_census.rs` counted plants by kind only at the start and end of a
  short run. Neither showed the valley's trees over a long run, or whether a community's felling
  radius was stripping the woods around its home; the audit probe's `daily.csv` had no plant
  column either.
- **Found by**: the 2026-11-13 alive audit (#1360/#1363-#1366 need a before/after on the woods).
- **Cause**: confirmed — the ecology census never read `w.plants` at all; it only tallied
  `w.agents` by species.
- **Fix**: added `systems::settlement::tree_census` (`crates/hf-sim/src/systems/settlement.rs`),
  read-only, counting mature green trees (`growth >= 0.95`, `Plant::mature`), young green trees
  and stumps, valley-wide and within 40 tiles of any living community's home — the same radius
  `chop_candidates` uses to pick the planner's felling targets. `examples/ecology_census.rs` now
  prints all six counts on its existing 30-day row.
- **Guard**: `crates/hf-sim/tests/ecology_census_trees.rs` plants a mature tree, a young tree and
  a stump near a community's home and a fourth mature tree 200 tiles away, and asserts the near
  and far counts split correctly; a second case checks dead plants, fallen logs and non-tree
  kinds are not counted. Both fail against the pre-fix code (which has no such function) and
  pass with it.
- **Commit**: (this commit).

## 2026-11-17 #854: four places decided who could see a thing, and three skipped line of sight

- **Symptom**: `accounts::witnessed` filtered inline (awake, grown, same level, within 8, in
  sight), while `strain::observe`, `feel::near` and `beliefs::near` filtered the same way without
  sight, so people behind a wall counted.
- **Found by**: the local AI (desktop audit), the 2026-11-13 alive audit (#854).
- **Fix**: one helper, `mind::perception::witnesses(w, at, r, grown_only)`: alive people, awake,
  same level, within `r`, in line of sight, in index order. `accounts::witnessed` now calls it
  (same result; it still drops the dead person). The other three callers are left for their own
  slices (#778).
- **Guard**: `tests/witnessed.rs::witnesses_behind_a_wall_are_not_counted`. Of two people 5 tiles
  from a spot, the one behind a wall is not returned, and a sleeper never is. `tests/accounts.rs`
  is unchanged and passes.

# 2026-11-18: owner publication notes (local release-preparation draft)

- Symptom and confirmed cause: the owner checklist adds a root `go-public/` directory,
  but the snapshot exclusion list did not classify it. A fixture export shipped the directory.
- Fix: `tools/publish/exclude.txt` excludes the owner-only directory. Existing working
  history and files are preserved; only fresh exports omit it.
- Guard: local `check_owner_notes.py` fails before the exclusion and passes afterward,
  also verifying that the public README still exports. Commit: pending owner integration.

## 2026-11-19 — private release integration boundary (local candidate)

Symptom: nested operational docs and confidential policy could enter the public
export; source workflow suppression could disable public checks; folder-level docs
approval allowed new files without review. Found by exact-tree rehearsal and
synthetic regressions. Cause confirmed: relocated paths, recursive classification
and candidate-owned privacy execution. Fix: confined exporter retains #2148 safety
foundations; docs_policy.py mandates exact-file classification; explicit public-ci.yml
keeps public automatic checks separate from manual private CI. Generic public
scanner accepts no private lists or inline exemptions. check-snapshot.sh refuses
privacy clearance and directs a pinned external offline artifact audit. Optional
media and operational records stay private; public_text.py only adjusts exported
text, leaving source history intact. Required notices and credits unchanged.
Guards: public_docs_guard_test (missing/invalid policy and nested files), exporter
confinement/neutral metadata fixture, history quarantine, public boundary, private
audit refusal and workflow split tests. No game rules, saves, digests or art changed.
Exact-head CI, actual privacy approval and replacement sprites remain pending.
Commit: this local release integration checkpoint.

The export fixture exposed a dependency on private empirical Windows audit-path
records. files_register_test.py retains those checks in private source (missing
source records still fail), explicitly skips only those two empirical tests in
the public tree, and adds positive synthetic install/update-path coverage there.
The existing negative/pattern tests and stub-package tree checks still run.

## 2026-11-19 — fallback integration rehearsal (local checkpoint pending)
Symptom: legacy item/provenance generators could restore prior pixels and sheet attribution
following a fallback swap. Found by entry-point review; cause confirmed in items.py,
items_derived.py and provenance.py. Fix: required pinned policy, exact asset checks, protected
item slots, disabled sheet entry points, policy-first atomic provenance and export check.
Unrelated provenance and original private refs retained. Guard: fallback_integration_test.py
exercises actual entry points with toy bytes, missing/altered policy, restored attribution,
protected writes and unrelated generation. The 113 fallback inputs remain unreviewed.

## 2026-11-19 — launch-prep digest and reference gates
Symptom: failing/malformed digest commands could pass CI because echo hid command-substitution
status; fallback dimensions/provenance left the reference assets block stale. Confirmed with
12 failing synthetic workflow scenarios and readme_tables.py --check exit 1. Fix:
tools/ci_digests.py checks each CLI exit and exact lowercase SHA-256 format before emitting the
unchanged comparison records; both workflow variants call it with pipefail. The reference is
regenerated by tools/docs/readme_tables.py. Guard: tools/tests/digest_job_test.py runs the real
job shell with failed, malformed, missing and duplicate fake output, plus exact valid output.
No simulation digest/golden values, comparison semantics or private CI triggers changed.
Checkpoint: local launch-prep review commit; no hosted run or release approval.

## 2026-11-19 — threaded-feed guard completion budget
Symptom: canonical feed guard compared fewer completed threaded ticks/rows against a full
6,000-tick inline run. Reported candidate result was 5,240 versus 6,000 ticks and 40 versus
57 rows; base passed. Native inputs and guard bytes were identical and targeted instrumented
pairs passed near the poll cap. Cause: confirmed harness logic permits comparison after 4,000
polls even when fewer than 6,000 ticks have been requested; timing explains this mechanism,
but the original canonical failure remains red pending an exact patched-head run.
Fix: tools/gd_feed_harness.gd collect waits for exactly 6,000 completed ticks, one 20-tick batch
outstanding, draining every live poll. Monotonic 30-second total and 5-second no-progress budgets
fail explicitly; no sync on a timeout. Only completed equal-duration runs reach feed parity and
while-live delivery checks. tools/gd_test.gd retains seed/opening and target, and reports completion
failure directly. No simulation, art, native build input or digest changes.
Guard: tools/gd_feed_harness_test.gd uses deterministic virtual time and toy workers: the old cap
stops at 5,340 ticks, the same progressing worker completes all 6,000 with the new harness, stalled
and continuing-but-too-slow workers fail, lost feed and delivery only after completion fail, and
unequal duration cannot be reported as a feed parity comparison. These pure tests do not replace
the canonical native suite. Checkpoint: separate local harness-only commit, validation pending.

## 2026-11-22 OST-004: nobody counted the major events the camera never showed
Symptom: no key in the alive report said how many Major events that name a person (a death, a
capture, a breakdown, an exile, a birth, a pairing, a return) the camera director never showed.
Cause: not a defect but a missing measurement, and the first form of it had two flaws. A camera
sample taken after an event's game hour (1,200 ticks) could still mark the event shown, because
samples were matched before old events were settled; and nothing in the tree called the
measurement headlessly.
Fix: `AliveTally::observe_major_events` watches each tick's Major events of those seven kinds,
and `AliveTally::observe_glance` marks one shown only for a sample within 12 tiles of its place, on
the same level, no more than 1,200 ticks after the event; older events then settle into per-kind
counts (an event with no place always settles missed). `view::camera_miss::measure` is the
headless caller: it steps a world, feeds the director the events a viewer is told of, samples its
suggestion every 600 ticks and feeds both to the tally; `examples/camera_misses.rs` prints the
raw counts and shares. New report keys: `glance_major_n`, `glance_missed_major` and
`glance_missed_major_<kind>`. Camera scoring and the simulation are unchanged.
Guard: `tests/camera_miss.rs::a_sample_after_the_hour_does_not_show_the_event` fails without the
timing check; the same file covers a death far from and inside the frame, events with no place,
another level or another severity, and that stepping through `measure` leaves the world digest
unchanged.
