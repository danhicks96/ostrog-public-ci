//! WP-10 (#1489): the alive report reads a colony's mood distribution from the world.

use hf_sim::agents::Species;
use hf_sim::events::{Event, EventKind, Severity};
use hf_sim::ids::{AgentId, CommunityId, ItemId, TilePos};
use hf_sim::mind::Activity;
use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use hf_sim::view::alive::{AliveReport, AliveTally};
use hf_sim::work::designation::DesignationId;
use hf_sim::work::Plan;
use hf_sim::world::gen::GenParams;
use hf_sim::World;

/// The party of `tests/feelings.rs`: a flat site and `n` founders standing in a row.
fn party(n: usize) -> (World, Vec<usize>) {
    let mut w = World::generate(5101, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.meta.tick = TICKS_PER_HOUR * 10;
    w.rebuild_caches();
    let ids = w.found_party(n);
    w.meta.autoplan = false;
    let idx: Vec<usize> = ids
        .iter()
        .map(|id| w.agents.iter().position(|a| a.id == *id).unwrap())
        .collect();
    (w, idx)
}

/// The mood the three people who must not be counted are given: far below everyone who should
/// be, so counting any one of them moves the median, the 10th percentile and the share below 0.
const OUTSIDER_MOOD: f32 = -1.0;

#[test]
fn pair_timing_keys_read_a_stream_without_changing_the_world() {
    let (mut w, people) = party(2);
    let (a, b) = (w.agents[people[0]].id, w.agents[people[1]].id);
    let event = |id, tick, kind| Event {
        id: hf_sim::ids::EventId(id),
        tick,
        at: None,
        severity: Severity::Minor,
        kind,
    };
    let events = [
        event(1, 0, EventKind::Talked { a, b, warmth: 0.5 }),
        event(
            2,
            TICKS_PER_DAY,
            EventKind::Talked {
                a: b,
                b: a,
                warmth: 0.5,
            },
        ),
        event(3, 3 * TICKS_PER_DAY, EventKind::Paired { a, b }),
    ];
    w.meta.tick = 3 * TICKS_PER_DAY;
    let before = hf_sim::digest::digest(&w);
    let mut tally = AliveTally::default();
    tally.observe_feed(&w, &events[..2]);
    tally.observe_feed(&w, &events[2..]);
    let report = tally.report();
    assert_eq!(
        report.get("pair_days_median"),
        Some(3.0),
        "the report includes the measured pairing delay"
    );
    assert_eq!(report.get("pair_talks_median"), Some(2.0));
    assert_eq!(report.get("pair_timing_n"), Some(1.0));
    assert_eq!(report.get("pair_unknown_arrival"), Some(1.0));
    tally.observe_events(&w, &events);
    assert_eq!(
        report,
        tally.report(),
        "the second event reader must not count the same records twice"
    );
    assert_eq!(before, hf_sim::digest::digest(&w));
}

#[test]
fn mood_keys_read_a_built_colony() {
    // Eight people: the six who count, one without a community and one who is dead; below, an
    // animal that does belong to the colony is added to them.
    let (mut w, p) = party(8);
    let moods = [-0.5, 0.0, 0.3, 0.9, 1.0, 1.0];
    for (&i, m) in p[..6].iter().zip(moods) {
        assert!(
            w.agents[i].community.is_some(),
            "a founder belongs to the colony"
        );
        w.agents[i].mood = m;
    }
    // A person who belongs to no community: not in the colony's mood.
    let drifter = p[6];
    w.agents[drifter].community = None;
    w.agents[drifter].mood = OUTSIDER_MOOD;
    // A person who is dead: not alive, so not sampled.
    let dead = p[7];
    assert!(w.agents[dead].community.is_some());
    w.agents[dead].alive = false;
    w.agents[dead].mood = OUTSIDER_MOOD;
    // An animal that is alive and belongs to the colony (founding gives animals no community,
    // so it is given one here): only its species keeps it out of the sample.
    let colony = w.agents[p[0]].community;
    let beast = w
        .agents
        .iter()
        .position(|a| a.species != Species::Person && a.alive)
        .expect("founding populates the valley with animals");
    w.agents[beast].community = colony;
    w.agents[beast].mood = OUTSIDER_MOOD;

    let mut tally = AliveTally::default();
    tally.observe_hour(&w);
    let r = tally.report();
    let get = |k: &str| r.get(k).unwrap_or_else(|| panic!("no key {k}"));

    assert!(
        (get("mood_median") - 0.6).abs() < 0.01,
        "median {}",
        get("mood_median")
    );
    // Nearest rank floor(0.1 * (6 - 1)) = 0 of the six sorted moods: the lowest, -0.5. An
    // off-by-one rank (0.0), the minimum of a wrongly wider sample (-1.0), or a constant 0.0
    // all differ from it.
    assert!(
        (get("mood_p10") - -0.5).abs() < 1e-9,
        "p10 {}",
        get("mood_p10")
    );
    assert!((get("mood_hi") - 0.5).abs() < 1e-9, "hi {}", get("mood_hi"));
    assert!(
        (get("mood_lo") - 1.0 / 6.0).abs() < 1e-9,
        "lo {}",
        get("mood_lo")
    );
    assert!(
        (get("mood_clamped") - 1.0 / 3.0).abs() < 1e-9,
        "clamped {}",
        get("mood_clamped")
    );
    // Six people were sampled, so the hour's s.d. was kept (population s.d. of the six moods).
    assert!(
        (get("mood_spread") - 0.568).abs() < 0.01,
        "spread {}",
        get("mood_spread")
    );

    let mut again = AliveTally::default();
    again.observe_hour(&w);
    assert_eq!(again.report(), r, "the same world gives the same report");
}

#[test]
fn mood_p10_is_the_nearest_rank_over_the_whole_run() {
    // Two observed hours of the same six founders make twelve samples, where the nearest rank
    // floor(0.1 * (12 - 1)) = 1 is no longer the minimum (rank 0) or rank 2.
    let (mut w, p) = party(6);
    let hour_one = [-0.5, 0.0, 0.3, 0.9, 1.0, 1.0];
    for (&i, m) in p.iter().zip(hour_one) {
        w.agents[i].mood = m;
    }
    let mut tally = AliveTally::default();
    tally.observe_hour(&w);
    w.meta.tick += TICKS_PER_HOUR;
    w.agents[p[0]].mood = -0.2;
    tally.observe_hour(&w);
    // Sorted: -0.5 -0.2 0.0 0.0 0.3 0.3 0.9 0.9 1.0 1.0 1.0 1.0
    let p10 = tally.report().get("mood_p10").unwrap();
    assert!((p10 - -0.2).abs() < 1e-6, "p10 {p10}");
}

#[test]
fn an_empty_tally_gives_every_key_as_zero() {
    let r = AliveTally::default().report();
    let keys: Vec<&str> = r.rows.iter().map(|(k, _)| *k).collect();
    assert_eq!(
        keys,
        [
            "mood_median",
            "mood_p10",
            "mood_hi",
            "mood_lo",
            "mood_clamped",
            "mood_spread",
            "mood_p25",
            "mood_p75",
            "mood_below_m03",
            "mood_term_needs",
            "mood_term_recent",
            "mood_term_long",
            "mood_term_thoughts",
            "standing_share",
            "unreadable_share",
            "work_share",
            "talk_share",
            "day_distance",
            "act_entropy_sd",
            "ties_saturated",
            "ties_hostile",
            "ties_frozen",
            "ties_frozen_max_days",
            "ties_to_dead_share",
            "grief_drop_2d",
            "grief_drop_10d",
            "grief_days",
            "grief_n",
            "cause_repeat",
            "lesson_ignored",
            "glance_major_n",
            "glance_missed_major",
            "feed_lines_day",
            "feed_folded_day",
            "glance_missed_major_died",
            "glance_missed_major_captured",
            "glance_missed_major_broke",
            "glance_missed_major_exiled",
            "glance_missed_major_born",
            "glance_missed_major_paired",
            "glance_missed_major_returned"
        ],
        "an empty tally keeps the rows' shape"
    );
    for (k, v) in &r.rows {
        assert_eq!(*v, 0.0, "{k} of an empty tally");
    }
}

/// How far a float key may be from its hand-computed value: the keys are f64 over f32 inputs,
/// so a rounding step is allowed and a wrong rule (a different rank, mean or filter) is not.
const TOL: f64 = 1e-4;

fn working() -> Activity {
    // Any plan will do: the report reads only that the activity is `Working`.
    Activity::Working {
        plan: Plan {
            designation: DesignationId(0),
            steps: vec![],
            cursor: 0,
            label: "building".into(),
        },
    }
}

fn loitering() -> Activity {
    Activity::Loitering { ticks_left: 600 }
}

fn sleeping() -> Activity {
    Activity::Sleeping {
        bed: None,
        until: 0,
    }
}

#[test]
fn activity_keys_count_a_loitering_day() {
    // Eight people and a beast: the six who count (three loitering, two working, one sleeping),
    // a person with no community who is idle, a dead person who is working, and an animal that
    // belongs to the colony and is talking. Each of the three would move a key if it counted:
    // standing_share, work_share and talk_share, and the hours of each would make them qualify
    // for day_distance.
    let (mut w, p) = party(8);
    let acts = [
        loitering(),
        loitering(),
        loitering(),
        working(),
        working(),
        sleeping(),
    ];
    for (&i, a) in p[..6].iter().zip(acts) {
        w.agents[i].activity = a;
    }
    let drifter = p[6];
    w.agents[drifter].community = None;
    w.agents[drifter].activity = Activity::Idle;
    let dead = p[7];
    w.agents[dead].alive = false;
    w.agents[dead].activity = working();
    let colony = w.agents[p[0]].community;
    let beast = w
        .agents
        .iter()
        .position(|a| a.species != Species::Person && a.alive)
        .expect("founding populates the valley with animals");
    w.agents[beast].community = colony;
    w.agents[beast].activity = Activity::Talking {
        with: w.agents[p[0]].id,
        ticks_left: 100,
    };

    let mut tally = AliveTally::with_min_hours(24);
    for _ in 0..24 {
        tally.observe_hour(&w);
        w.meta.tick += TICKS_PER_HOUR;
    }
    let r = tally.report();
    let get = |k: &str| r.get(k).unwrap_or_else(|| panic!("no key {k}"));
    // Waking hours are 5 people * 24; 3 * 24 of them are loitering, 2 * 24 working.
    assert!(
        (get("standing_share") - 0.6).abs() < TOL,
        "standing {}",
        get("standing_share")
    );
    assert!(
        (get("unreadable_share") - 0.6).abs() < TOL,
        "unreadable {}",
        get("unreadable_share")
    );
    assert!(
        (get("work_share") - 0.4).abs() < TOL,
        "work {}",
        get("work_share")
    );
    assert_eq!(get("talk_share"), 0.0, "only the beast talks");
    // Shares over all 24 sampled hours: a loiterer is (1, 0, 0), a worker (0, 1, 0), the sleeper
    // (0, 0, 1) in (loitering, working, sleeping). The mean is (1/2, 1/3, 1/6), so the half-L1
    // distances are 50 (x3), 200/3 (x2) and 250/3: the median is (50 + 200/3) / 2 = 175/3.
    assert!(
        (get("day_distance") - 175.0 / 3.0).abs() < TOL,
        "day_distance {}",
        get("day_distance")
    );
    // Every person spent the whole run in one activity: entropy 0 for all, so no spread.
    assert!(
        get("act_entropy_sd").abs() < TOL,
        "entropy sd {}",
        get("act_entropy_sd")
    );

    // All six loitering for the same 24 hours: everyone's day is the colony's day.
    for &i in &p[3..6] {
        w.agents[i].activity = loitering();
    }
    let mut same = AliveTally::with_min_hours(24);
    for _ in 0..24 {
        same.observe_hour(&w);
        w.meta.tick += TICKS_PER_HOUR;
    }
    let r = same.report();
    assert!(r.get("day_distance").unwrap().abs() < TOL);
    assert!((r.get("standing_share").unwrap() - 1.0).abs() < TOL);
    assert_eq!(r.get("work_share").unwrap(), 0.0);
}

#[test]
fn activity_shares_count_every_waking_activity() {
    // One person in each of the ten activities, but each for a different number of hours: the
    // activity in slot s (the order of `Activity::name()`: idle, going, eating, foraging,
    // sleeping, talking, loitering, wandering, working, drinking) is held for 2^s hours, 1, 2,
    // 4, ... 512, so no two slots have the same total and every sum of slots is unique. A person
    // stops being sampled when their hours are up (their community is taken away), so the
    // counts are exact. Swapping two slots that the keys count differently, or dropping a waking
    // activity from the denominator, moves one of the four shares; swaps within one bucket
    // cannot (see the comment on the sums below).
    let (mut w, p) = party(10);
    let someone = w.agents[p[0]].id;
    let colony = w.agents[p[0]].community;
    let acts = [
        Activity::Idle,
        Activity::Going {
            to: TilePos::new(31, 30, 2),
            then: Box::new(Activity::Idle),
        },
        Activity::Eating {
            item: ItemId(0),
            ticks_left: 10,
        },
        Activity::Foraging {
            plant: TilePos::new(32, 30, 2),
            ticks_left: 10,
        },
        sleeping(),
        Activity::Talking {
            with: someone,
            ticks_left: 10,
        },
        loitering(),
        Activity::Wandering {
            to: TilePos::new(33, 30, 2),
        },
        working(),
        Activity::Drinking {
            ticks_left: 10,
            back: None,
        },
    ];
    for (&i, a) in p.iter().zip(acts) {
        w.agents[i].activity = a;
    }
    let mut tally = AliveTally::default();
    for hour in 0..512u32 {
        for (slot, &i) in p.iter().enumerate() {
            w.agents[i].community = if hour < 1 << slot { colony } else { None };
        }
        tally.observe_hour(&w);
        w.meta.tick += TICKS_PER_HOUR;
    }
    let r = tally.report();
    let get = |k: &str| r.get(k).unwrap_or_else(|| panic!("no key {k}"));
    // Hours per slot: idle 1, going 2, eating 4, foraging 8, sleeping 16, talking 32,
    // loitering 64, wandering 128, working 256, drinking 512. Waking hours are every slot but
    // sleeping: 1023 - 16 = 1007.
    // standing = idle + loitering = 1 + 64 = 65; unreadable = standing + going = 65 + 2 = 67;
    // work = 256; talk = 32. Each is a different sum, so swapping any two slots that the keys
    // count differently moves a share. Each slot is in some of five buckets: standing, unreadable,
    // work, talk and waking (the denominator). Idle and loitering are standing, unreadable and
    // waking; going is unreadable and waking; talking is talk and waking; working is work and
    // waking; sleeping is not waking; eating, foraging, wandering and drinking are waking only.
    // A swap within the same buckets (idle <-> loitering; the 6 pairs among eating, foraging,
    // wandering and drinking) cannot change any reported key: 7 of the 45 pairs. day_distance
    // and act_entropy_sd are also unchanged by any relabelling applied to every person alike. The
    // other 38 pairs move at least one share, so this test fails for them.
    assert!(
        (get("standing_share") - 65.0 / 1007.0).abs() < TOL,
        "standing {}",
        get("standing_share")
    );
    assert!(
        (get("unreadable_share") - 67.0 / 1007.0).abs() < TOL,
        "unreadable {}",
        get("unreadable_share")
    );
    assert!(
        (get("work_share") - 256.0 / 1007.0).abs() < TOL,
        "work {}",
        get("work_share")
    );
    assert!(
        (get("talk_share") - 32.0 / 1007.0).abs() < TOL,
        "talk {}",
        get("talk_share")
    );
    // Nobody was sampled 2,000 hours (the most is 512): below the default minimum, so no
    // person key reads.
    assert_eq!(get("day_distance"), 0.0);
    assert_eq!(get("act_entropy_sd"), 0.0);
}

#[test]
fn a_colony_asleep_has_no_waking_hours_to_share() {
    let (mut w, p) = party(6);
    for &i in &p {
        w.agents[i].activity = sleeping();
    }
    let mut tally = AliveTally::default();
    tally.observe_hour(&w);
    let r = tally.report();
    for k in [
        "standing_share",
        "unreadable_share",
        "work_share",
        "talk_share",
    ] {
        assert_eq!(r.get(k), Some(0.0), "{k} with no waking hours");
    }
}

#[test]
fn day_distance_and_entropy_read_each_persons_own_mix() {
    // Six hours, min_hours 4. The colony has six people, sampled for different numbers of hours
    // (a person is sampled only while they belong to the colony):
    //   people 0, 1 and 5: all six hours, loitering throughout, share (loitering, working) = (1, 0);
    //   person 2: all six hours, loitering for three and working for three, (1/2, 1/2);
    //   person 3: the last four hours, working throughout, (0, 1): the fewest hours that count;
    //   person 4: the last three hours, working, exactly min_hours - 1: out of the person keys,
    //   but still in the shares.
    let (mut w, p) = party(6);
    let colony = w.agents[p[0]].community;
    let mut tally = AliveTally::with_min_hours(4);
    for hour in 0..6 {
        for &i in &[p[0], p[1], p[5]] {
            w.agents[i].activity = loitering();
        }
        w.agents[p[2]].activity = if hour < 3 { loitering() } else { working() };
        w.agents[p[3]].activity = working();
        w.agents[p[4]].activity = working();
        w.agents[p[3]].community = if hour >= 2 { colony } else { None };
        w.agents[p[4]].community = if hour >= 3 { colony } else { None };
        tally.observe_hour(&w);
        w.meta.tick += TICKS_PER_HOUR;
    }
    let r = tally.report();
    let get = |k: &str| r.get(k).unwrap_or_else(|| panic!("no key {k}"));
    // Five people qualify. Each counts once, so the mean day is the mean of their five share
    // vectors: loitering (1 + 1 + 1 + 1/2 + 0) / 5 = 7/10, working 3/10. With only two
    // activities the half-L1 distance is the gap in the loitering share: 3/10 for people 0, 1
    // and 5, 2/10 for person 2, 7/10 for person 3. Sorted: 20, 30, 30, 30, 70 points; the
    // median is 30 (the mean of the five distances, 36, is another number).
    // Wrong versions: a mean weighted by hours (6, 6, 6, 6, 4 of 28: loitering 21/28 = 3/4)
    // gives 25, 25, 25, 25, 75 and a median of 25; a minimum of min_hours - 1 lets person 4 in
    // (six people, mean loitering 3.5/6): distances 41.7 (x3), 8.3, 58.3 (x2), a median of 41.7.
    assert!(
        (get("day_distance") - 30.0).abs() < TOL,
        "day_distance {}",
        get("day_distance")
    );
    // Entropies in bits: 0 for people 0, 1, 3 and 5, 1 for person 2. Mean 1/5; deviations -1/5
    // (four people) and 4/5 (one), so the variance is (4/25 + 16/25) / 5 = 4/25 and the s.d. is
    // 2/5. (With person 4 in, six people, the s.d. is sqrt(5)/6.)
    assert!(
        (get("act_entropy_sd") - 0.4).abs() < TOL,
        "entropy sd {}",
        get("act_entropy_sd")
    );
    // Shares count all six people: loitering 6 + 6 + 6 + 3 = 21, working 3 + 4 + 3 = 10, of 31
    // waking hours.
    assert!((get("standing_share") - 21.0 / 31.0).abs() < TOL);
    assert!((get("work_share") - 10.0 / 31.0).abs() < TOL);
}

// ---- #1491: the tie keys ----

/// A party whose people start with no ties at all, so every tie a tie test counts is one the
/// test made.
fn bare_party(n: usize) -> (World, Vec<usize>) {
    let (mut w, p) = party(n);
    for a in w.agents.iter_mut() {
        a.relationships.clear();
    }
    (w, p)
}

/// Person `from` (an index into `w.agents`) relates to `to` by `opinion` (their first tie, so
/// the opinion is exactly `opinion`), last in contact at `tick`.
fn tie(w: &mut World, from: usize, to: usize, opinion: f32, tick: u64) {
    let other = w.agents[to].id;
    w.agents[from].relate(other, opinion, 0.5, tick);
}

/// The report of a fresh tally that has looked at the ties of `w` once.
fn tie_report(w: &World) -> AliveReport {
    let mut tally = AliveTally::default();
    tally.observe_ties(w);
    tally.report()
}

fn key(r: &AliveReport, k: &str) -> f64 {
    r.get(k).unwrap_or_else(|| panic!("no key {k}"))
}

#[test]
fn frozen_tie_is_counted() {
    // The issue's case. A relates to B at +1.0 at tick t; 101 days later A relates to C at +1.0.
    // A has two ties, B untouched for exactly 101 days and C for none, and nobody else has any.
    let (mut w, p) = bare_party(3);
    let t = w.meta.tick;
    tie(&mut w, p[0], p[1], 1.0, t);
    w.meta.tick = t + 101 * TICKS_PER_DAY;
    let now = w.meta.tick;
    tie(&mut w, p[0], p[2], 1.0, now);
    let r = tie_report(&w);
    // Only the tie to B is at >= 0.95 and 100+ days old; its gap is 101 days.
    assert_eq!(key(&r, "ties_frozen"), 1.0);
    assert_eq!(key(&r, "ties_frozen_max_days"), 101.0);
    // A's top five is both ties, both at +1.00 (>= 0.995): 2 of 2.
    assert_eq!(key(&r, "ties_saturated"), 1.0);
    assert_eq!(key(&r, "ties_hostile"), 0.0);
    assert_eq!(key(&r, "ties_to_dead_share"), 0.0);
    assert_eq!(tie_report(&w), r, "the same world gives the same report");
}

#[test]
fn frozen_ties_read_the_threshold_edges() {
    // One person, A, with eight ties to different people, all last touched at NOW - gap. A tie is
    // frozen when its opinion is >= 0.95 and its gap is >= 100 days = 2,880,000 ticks.
    //   to p1: 0.95   (at the opinion bound)       gap 120 days            frozen, 120 days
    //   to p2: 0.9499 (one step below the bound)   gap 130 days            not frozen
    //   to p3: 0.96                                gap exactly 100 days    frozen, 100 days (at the gap bound)
    //   to p4: 0.96                                gap 100 days - 1 tick   not frozen (99 whole days)
    //   to p5: 1.0                                 gap 187 days + 20,000 ticks   frozen, 187 whole days
    //   to p6: 0.97                                gap 103 days + 5 ticks  frozen, 103 days
    //   to p7: -0.99  (strong but hostile)         gap 400 days            not frozen
    //   to p8: 0.5    (a distant acquaintance)     gap 300 days            not frozen
    // Frozen: p1, p3, p5, p6 = 4. The longest gap of a tie at >= 0.95 is p5's: 187 days (20,000
    // ticks is 0.69 of a day, so rounding would say 188; p1 120, p3 100, p4 99, p6 103). The
    // longest gaps of the ties below 0.95 (p7's 400, p8's 300, p2's 130) are not read.
    let (mut w, p) = bare_party(9);
    const NOW: u64 = 500 * TICKS_PER_DAY;
    w.meta.tick = NOW;
    let d = TICKS_PER_DAY;
    let ties = [
        (0.95, 120 * d),
        (0.9499, 130 * d),
        (0.96, 100 * d),
        (0.96, 100 * d - 1),
        (1.0, 187 * d + 20_000),
        (0.97, 103 * d + 5),
        (-0.99, 400 * d),
        (0.5, 300 * d),
    ];
    for (k, (opinion, gap)) in ties.into_iter().enumerate() {
        tie(&mut w, p[0], p[k + 1], opinion, NOW - gap);
    }
    let r = tie_report(&w);
    assert_eq!(key(&r, "ties_frozen"), 4.0);
    assert_eq!(key(&r, "ties_frozen_max_days"), 187.0);
    // A's top five by strength are p5 (1.0), p7 (-0.99), p6 (0.97) and p3 and p4 (0.96 each);
    // p1 (0.95) is sixth. One of the five is at >= 0.995 (p5) and one below 0 (p7): 1/5 each.
    assert!((key(&r, "ties_saturated") - 0.2).abs() < 1e-9);
    assert!((key(&r, "ties_hostile") - 0.2).abs() < 1e-9);
}

#[test]
fn top_five_ties_are_ranked_by_strength_and_pooled() {
    // Two people have ties, all touched now. Each person's five strongest ties (by |opinion|)
    // count; the keys are shares of the pooled top-five ties of everyone.
    //   A: to p2 +1.0, p3 +0.995 (at the saturation bound), p4 +0.99, p5 -0.8, p6 +0.3,
    //      p7 -0.3, p8 +0.1, p9 +0.05: eight ties, so only five count. p6 and p7 are equally
    //      strong; the lower id (p6, whose opinion is +0.3) takes fifth place.
    //      Top five: p2, p3, p4, p5, p6. Saturated (>= 0.995): p2, p3 = 2. Hostile: p5 = 1.
    //   B: to p2 -1.0, p3 +0.6, p4 -0.5: three ties, all count. Saturated: 0 (-1.0 is hostile,
    //      not saturated). Hostile: p2, p4 = 2.
    // Pooled: 5 + 3 = 8 ties; saturated 2/8 = 0.25; hostile 3/8 = 0.375.
    // Wrong versions: the mean of the two people's shares gives 0.2 and 0.433; all 11 ties give
    // 2/11 and 4/11; the top six put p7 (-0.3) in A's list, so 2/9 and 4/9; the top four give
    // 2/7 and 3/7; ranking by signed opinion gives A the list p2, p3, p4, p6, p8 and B the same
    // three, so 2/8 and 2/8; breaking the tie between p6 and p7 by the higher id puts p7 in
    // fifth place and gives 2/8 and 4/8.
    let (mut w, p) = bare_party(10);
    assert!(
        w.agents[p[6]].id < w.agents[p[7]].id,
        "founders have ascending ids"
    );
    let now = w.meta.tick;
    let a = [1.0, 0.995, 0.99, -0.8, 0.3, -0.3, 0.1, 0.05];
    for (k, o) in a.into_iter().enumerate() {
        tie(&mut w, p[0], p[k + 2], o, now);
    }
    for (k, o) in [-1.0, 0.6, -0.5].into_iter().enumerate() {
        tie(&mut w, p[1], p[k + 2], o, now);
    }
    let r = tie_report(&w);
    assert_eq!(key(&r, "ties_saturated"), 0.25);
    assert_eq!(key(&r, "ties_hostile"), 0.375);
    assert_eq!(key(&r, "ties_frozen"), 0.0);
}

#[test]
fn ties_of_people_outside_the_colony_are_not_counted_and_the_dead_are_a_share() {
    // Nine people and a beast. A (p0), B (p1) and C (p2) have the ties that count. p3 is dead (in
    // `w.agents`, alive false). p6 is a living person with no community (a drifter). p7 is away on
    // a journey and p8 is a captive: both are living people off the map, kept in
    // `w.overworld.away` (journeys.rs, siege.rs) and not in `w.agents`. A beast belongs to the
    // colony. AgentId(900_000) is found nowhere. All ties are touched now except A's to p3.
    //
    // A has eight ties: to p3 +1.0 (dead, 150 days old), the missing id -0.8, p8 0.45, p5 0.4,
    // p7 0.35, p4 0.3, the drifter p6 0.25 and the living beast 0.15.
    // B has seven: p4 0.7, the journey p7 0.65, p2 0.6, p5 0.55, the captive p8 0.5, the drifter
    // p6 0.45 and, weakest, the dead p3 0.2.
    // C has one: p5 0.0 (neither close nor hostile).
    //
    // Top five (by strength): A's are p3 1.0, the missing id 0.8, p8 0.45, p5 0.4, p7 0.35 (p4,
    // p6 and the beast are weaker); B's are p4, p7, p2, p5, p8 (0.7 down to 0.5), so B's tie to
    // the dead p3 is seventh, outside their top five; C's one tie counts. 5 + 5 + 1 = 11 top-five
    // ties. Saturated (>= 0.995): A's +1.0 to the dead p3 = 1, so 1/11. Hostile: A's -0.8 to the
    // missing id = 1, so 1/11 (the 0.0 tie of C is not hostile).
    //
    // Dead share: only an id found with alive == false is dead. Ties to someone the world holds
    // (in `w.agents` or `w.overworld.away`): A's seven (all but the missing id) + B's seven + C's
    // one = 15. Ties to the dead: A's to p3 and B's to p3 = 2. The share is 2/15. (An id found
    // nowhere is neither dead nor in the denominator, but its tie is still a tie: it is in A's
    // top five and in `ties_hostile`; FIXLOG 2026-11-14 #1491.) Wrong versions, each by hand:
    //   the away counted as dead (p7 and p8, each tied to by A and by B: 4 more) 6/15;
    //   the drifter counted as dead (A's and B's tie to p6) 4/15;
    //   the beast counted as dead 3/15;
    //   the missing id counted as dead 3/16; left in the denominator, not dead 2/16;
    //   the away not looked for (not found, so out of the denominator: -4) 2/11;
    //   dead ties counted among the top five only, both parts: A's top five holds four held ties
    //   (p3, p8, p5, p7), B's five and C's one = 10, and only A's tie to p3 is dead: 1/10;
    //   only the dead part counted among the top five: 1/15;
    //   a tie to the missing id dropped before the top five is taken: A's p4 takes fifth place
    //   and the -0.8 is gone, so hostile reads 0 where it must read 1/11.
    // Frozen: A's tie to the dead p3 is +1.0 and 150 days old, so `ties_frozen` is 1 and the
    // longest gap is 150 (a version that leaves the frozen ties to the dead out reads 0 and 0).
    // No other tie is at >= 0.95 among the ties of the population.
    //
    // The dead p3, the drifter p6 and the beast also have ties, each at +-1.0 and 300-400 days
    // old: counting any of them would move saturated, hostile, frozen and the longest gap.
    let (mut w, p) = bare_party(9);
    const NOW: u64 = 500 * TICKS_PER_DAY;
    w.meta.tick = NOW;
    let d = TICKS_PER_DAY;
    let missing = AgentId(900_000);
    assert!(w.agents.iter().all(|a| a.id != missing));
    assert!(w.overworld.away.iter().all(|a| a.id != missing));
    let beast = w
        .agents
        .iter()
        .position(|a| a.species != Species::Person && a.alive)
        .expect("founding populates the valley with animals");
    w.agents[beast].community = w.agents[p[0]].community;

    tie(&mut w, p[0], p[3], 1.0, NOW - 150 * d);
    w.agents[p[0]].relate(missing, -0.8, 0.5, NOW);
    tie(&mut w, p[0], p[4], 0.3, NOW);
    tie(&mut w, p[0], p[5], 0.4, NOW);
    tie(&mut w, p[0], beast, 0.15, NOW);
    tie(&mut w, p[0], p[6], 0.25, NOW);
    tie(&mut w, p[0], p[7], 0.35, NOW);
    tie(&mut w, p[0], p[8], 0.45, NOW);
    tie(&mut w, p[1], p[3], 0.2, NOW);
    tie(&mut w, p[1], p[2], 0.6, NOW);
    tie(&mut w, p[1], p[4], 0.7, NOW);
    tie(&mut w, p[1], p[5], 0.55, NOW);
    tie(&mut w, p[1], p[6], 0.45, NOW);
    tie(&mut w, p[1], p[7], 0.65, NOW);
    tie(&mut w, p[1], p[8], 0.5, NOW);
    tie(&mut w, p[2], p[5], 0.0, NOW);
    // The ones who must not count.
    tie(&mut w, p[3], p[0], 1.0, NOW - 400 * d);
    tie(&mut w, p[3], p[4], -1.0, NOW - 400 * d);
    tie(&mut w, p[6], p[3], 1.0, NOW - 300 * d);
    tie(&mut w, p[6], p[0], 1.0, NOW - 300 * d);
    tie(&mut w, beast, p[3], 1.0, NOW - 350 * d);
    w.agents[p[3]].alive = false;
    w.agents[p[6]].community = None;
    // p7 leaves on a journey and p8 is taken captive: both come off the map whole and alive, as
    // journeys.rs and siege.rs do it. Removed after every index above was used.
    let (journey, captive) = (w.agents[p[7]].id, w.agents[p[8]].id);
    for id in [journey, captive] {
        let at = w.agents.iter().position(|a| a.id == id).unwrap();
        let mut a = w.agents.remove(at);
        assert!(a.alive);
        if id == journey {
            a.journey = Some((NOW + 10 * d, "Greywater".to_string(), "trade".to_string()));
        }
        w.overworld.away.push(a);
    }
    assert!(w.agents.iter().all(|a| a.id != journey && a.id != captive));
    assert!(
        !w.overworld.sites.is_empty(),
        "the valley has overworld sites"
    );
    w.overworld.sites[0].captives.push((captive, NOW));

    let r = tie_report(&w);
    assert!(
        (key(&r, "ties_to_dead_share") - 2.0 / 15.0).abs() < 1e-9,
        "dead share {}",
        key(&r, "ties_to_dead_share")
    );
    assert!(
        (key(&r, "ties_saturated") - 1.0 / 11.0).abs() < 1e-9,
        "saturated {}",
        key(&r, "ties_saturated")
    );
    assert!(
        (key(&r, "ties_hostile") - 1.0 / 11.0).abs() < 1e-9,
        "hostile {}",
        key(&r, "ties_hostile")
    );
    assert_eq!(key(&r, "ties_frozen"), 1.0);
    assert_eq!(key(&r, "ties_frozen_max_days"), 150.0);
}

#[test]
fn a_colonist_away_has_their_own_ties_unread() {
    // Ties are read from the people in `w.agents`, as `observe_hour` samples them; someone away
    // (here a captive, in `w.overworld.away`) is the other end of a tie but their own are not
    // read. A (p0) has one tie, +0.5 to p1, touched now. C (p2) has +1.0 to p1, 300 days old, and
    // -0.9 to A; C is then taken away. Read: A's one tie, 0.5: 1 top-five tie, not saturated, not
    // hostile, not frozen (0 days), longest gap 0, and p1 is alive: dead share 0. A version that
    // read C's ties too would have 3 top-five ties, 1 saturated (1/3), 1 hostile (1/3), 1 frozen
    // and a longest gap of 300.
    let (mut w, p) = bare_party(3);
    const NOW: u64 = 500 * TICKS_PER_DAY;
    w.meta.tick = NOW;
    tie(&mut w, p[0], p[1], 0.5, NOW);
    tie(&mut w, p[2], p[1], 1.0, NOW - 300 * TICKS_PER_DAY);
    tie(&mut w, p[2], p[0], -0.9, NOW);
    let id = w.agents[p[2]].id;
    let at = w.agents.iter().position(|a| a.id == id).unwrap();
    let a = w.agents.remove(at);
    w.overworld.away.push(a);
    let r = tie_report(&w);
    assert_eq!(key(&r, "ties_saturated"), 0.0);
    assert_eq!(key(&r, "ties_hostile"), 0.0);
    assert_eq!(key(&r, "ties_frozen"), 0.0);
    assert_eq!(key(&r, "ties_frozen_max_days"), 0.0);
    assert_eq!(key(&r, "ties_to_dead_share"), 0.0);
}

#[test]
fn the_longest_close_gap_is_read_when_no_tie_is_frozen() {
    // A has three ties, none frozen (every gap is under 100 days = 2,880,000 ticks):
    //   to p1: 0.97   gap 40 days   close (>= 0.95)
    //   to p2: 0.5    gap 90 days   not close
    //   to p3: 0.9499 gap 80 days   not close (one step under the bound)
    // The longest gap of a close tie is 40 days: `ties_frozen_max_days` reads 40 and
    // `ties_frozen` 0. Reading only frozen ties gives 0 for the first; reading the gap of every
    // tie gives 90.
    let (mut w, p) = bare_party(4);
    const NOW: u64 = 500 * TICKS_PER_DAY;
    w.meta.tick = NOW;
    let d = TICKS_PER_DAY;
    tie(&mut w, p[0], p[1], 0.97, NOW - 40 * d);
    tie(&mut w, p[0], p[2], 0.5, NOW - 90 * d);
    tie(&mut w, p[0], p[3], 0.9499, NOW - 80 * d);
    let r = tie_report(&w);
    assert_eq!(key(&r, "ties_frozen"), 0.0);
    assert_eq!(key(&r, "ties_frozen_max_days"), 40.0);
}

#[test]
fn hostile_pools_every_look_and_the_other_keys_read_the_latest() {
    // First look: A has four ties, to p1 -0.5, p2 +0.5, p3 +0.5 and p4 +1.0; all four are in
    // A's top five: 4 top ties, 1 hostile, 1 saturated. Then B gains two hostile ties (to p2 and
    // p3, -0.5 each) and the second look sees 6 top ties (A's four and B's two), 3 hostile and
    // 1 saturated.
    //   ties_hostile (pooled over both looks) = (1 + 3) / (4 + 6) = 0.4. The latest look alone
    //   gives 3/6 = 0.5; the first alone 1/4 = 0.25; the mean of the two looks' shares 0.375.
    //   ties_saturated (latest look) = 1/6; pooled it would be (1 + 1) / (4 + 6) = 0.2.
    // The looks are 30 days apart, as the caller's are: two looks at one tick count once
    // (`a_second_look_at_the_same_tick_is_not_pooled_twice`).
    let (mut w, p) = bare_party(5);
    let now = w.meta.tick;
    for (k, o) in [-0.5, 0.5, 0.5, 1.0].into_iter().enumerate() {
        tie(&mut w, p[0], p[k + 1], o, now);
    }
    let mut tally = AliveTally::default();
    tally.observe_ties(&w);
    w.meta.tick = now + 30 * TICKS_PER_DAY;
    let now = w.meta.tick;
    tie(&mut w, p[1], p[2], -0.5, now);
    tie(&mut w, p[1], p[3], -0.5, now);
    tally.observe_ties(&w);
    let r = tally.report();
    assert!(
        (key(&r, "ties_hostile") - 0.4).abs() < 1e-9,
        "hostile {}",
        key(&r, "ties_hostile")
    );
    assert!(
        (key(&r, "ties_saturated") - 1.0 / 6.0).abs() < 1e-9,
        "saturated {}",
        key(&r, "ties_saturated")
    );
}

/// Two looks 30 days apart, as in `hostile_pools_every_look_and_the_other_keys_read_the_latest`:
/// the first, at tick `t`, sees A's four ties to p1 -0.5, p2 +0.5, p3 +0.5 and p4 +1.0 (4 top-five
/// ties, 1 hostile); B then gains -0.5 ties to p2 and p3 and the second look, 30 days later, sees
/// 6 top-five ties, 3 hostile. Returns the world, the people, the tally and the first tick.
fn two_looks_thirty_days_apart() -> (World, Vec<usize>, AliveTally, u64) {
    let (mut w, p) = bare_party(5);
    let t = w.meta.tick;
    for (k, o) in [-0.5, 0.5, 0.5, 1.0].into_iter().enumerate() {
        tie(&mut w, p[0], p[k + 1], o, t);
    }
    let mut tally = AliveTally::default();
    tally.observe_ties(&w);
    w.meta.tick = t + 30 * TICKS_PER_DAY;
    let now = w.meta.tick;
    tie(&mut w, p[1], p[2], -0.5, now);
    tie(&mut w, p[1], p[3], -0.5, now);
    tally.observe_ties(&w);
    (w, p, tally, t)
}

#[test]
fn a_second_look_at_the_same_tick_is_not_pooled_twice() {
    // A run of 600 days ends on a 30-day look: the caller looks twice at one tick. After the
    // two looks above the pool is (1 + 3) / (4 + 6) = 0.4. A third look at the same tick sees the
    // same world, so it replaces the second look's 3 / 6 with 3 / 6 and the key still reads 0.4.
    // Adding it again gives (1 + 3 + 3) / (4 + 6 + 6) = 7/16 = 0.4375.
    let (w, _, mut tally, _) = two_looks_thirty_days_apart();
    assert!((key(&tally.report(), "ties_hostile") - 0.4).abs() < 1e-9);
    tally.observe_ties(&w);
    let r = tally.report();
    assert!(
        (key(&r, "ties_hostile") - 0.4).abs() < 1e-9,
        "hostile {}",
        key(&r, "ties_hostile")
    );
    assert!(
        (key(&r, "ties_saturated") - 1.0 / 6.0).abs() < 1e-9,
        "the other keys read the latest look"
    );
}

#[test]
fn a_second_look_at_the_same_tick_replaces_the_first_and_an_earlier_one_is_added() {
    // Continue from the two looks (pool 4 / 10): B gains a third hostile tie (to p4, -0.5) and
    // the caller looks again at the same tick. That look sees 7 top-five ties (A's four, B's
    // three), 4 hostile, and replaces the second look's 3 / 6 in the pool:
    //   pool = (1 + 4) / (4 + 7) = 5/11 = 0.4545.
    // Wrong versions: the second call ignored 4/10 = 0.4; added as well 8/17 = 0.4706; the pool
    // reset to the latest look alone 4/7 = 0.5714.
    // Then a look at a tick EARLIER than the last one (10 days after the first), seeing the same
    // 7 / 4, is added like any other: (1 + 4 + 4) / (4 + 7 + 7) = 9/18 = 0.5. Wrong versions: an
    // earlier look that replaces or is ignored reads 5/11 = 0.4545.
    let (mut w, p, mut tally, t) = two_looks_thirty_days_apart();
    let now = w.meta.tick;
    tie(&mut w, p[1], p[4], -0.5, now);
    tally.observe_ties(&w);
    let hostile = |t: &AliveTally| key(&t.report(), "ties_hostile");
    assert!(
        (hostile(&tally) - 5.0 / 11.0).abs() < 1e-9,
        "hostile {}",
        hostile(&tally)
    );
    w.meta.tick = t + 10 * TICKS_PER_DAY;
    tally.observe_ties(&w);
    assert!(
        (hostile(&tally) - 0.5).abs() < 1e-9,
        "hostile {}",
        hostile(&tally)
    );
}

#[test]
fn a_later_look_replaces_the_earlier_one() {
    // A's tie to B is +1.0 and 101 days old at the first look: frozen. A speaks to B again and
    // the second look sees a fresh tie: the keys describe the latest look, they do not add up
    // (`ties_hostile` alone pools the looks; `hostile_pools_every_look_and_the_other_keys_read_the_latest`).
    let (mut w, p) = bare_party(3);
    let t = w.meta.tick;
    tie(&mut w, p[0], p[1], 1.0, t);
    w.meta.tick = t + 101 * TICKS_PER_DAY;
    let mut tally = AliveTally::default();
    tally.observe_ties(&w);
    assert_eq!(tally.report().get("ties_frozen"), Some(1.0));
    let now = w.meta.tick;
    tie(&mut w, p[0], p[1], 0.0, now);
    tally.observe_ties(&w);
    let r = tally.report();
    assert_eq!(key(&r, "ties_frozen"), 0.0);
    assert_eq!(key(&r, "ties_frozen_max_days"), 0.0);
    assert_eq!(key(&r, "ties_saturated"), 1.0, "A's one tie is still +1.00");

    // The hourly sample does not read ties, and looking at ties adds no mood sample.
    let mut hours = AliveTally::default();
    hours.observe_hour(&w);
    for (k, v) in hours.report().rows {
        if k.starts_with("ties_") {
            assert_eq!(v, 0.0, "{k} before any look at ties");
        }
    }
    let r = tally.report();
    assert_eq!(key(&r, "mood_median"), 0.0);
    assert_eq!(key(&r, "standing_share"), 0.0);
}

#[test]
fn a_tie_dated_after_now_is_not_frozen() {
    // A tie whose last contact is later than the tick being looked at (5 days later) has no gap:
    // not frozen, and the look does not underflow.
    let (mut w, p) = bare_party(3);
    let t = w.meta.tick;
    tie(&mut w, p[0], p[1], 1.0, t + 5 * TICKS_PER_DAY);
    let r = tie_report(&w);
    assert_eq!(key(&r, "ties_frozen"), 0.0);
    assert_eq!(key(&r, "ties_frozen_max_days"), 0.0);
    assert_eq!(key(&r, "ties_saturated"), 1.0);
}

#[test]
fn a_colony_with_no_ties_gives_zero_tie_keys() {
    let (w, _) = bare_party(5);
    let r = tie_report(&w);
    for k in [
        "ties_saturated",
        "ties_hostile",
        "ties_frozen",
        "ties_frozen_max_days",
        "ties_to_dead_share",
    ] {
        assert_eq!(key(&r, k), 0.0, "{k} with no ties");
    }
}

// ---- #1492: the grief keys ----

/// Ticks: a game day (28,800) and an hour (1,200), as `i64` offsets from a death.
const DAY: i64 = TICKS_PER_DAY as i64;
const HOUR: i64 = TICKS_PER_HOUR as i64;

/// Make `a` and `b` (indexes into `w.agents`) partners, both ways, as the game does.
fn pair(w: &mut World, a: usize, b: usize) {
    let (ia, ib) = (w.agents[a].id, w.agents[b].id);
    w.agents[a].partner = Some(ib);
    w.agents[b].partner = Some(ia);
}

/// Kill `dead` (an index into `w.agents`) with the game's own `die`, end the tick as the game
/// does (`flush` moves the tick's events into `last_events`) and return that tick's events.
fn death(w: &mut World, dead: usize) -> Vec<Event> {
    let id = w.agents[dead].id;
    hf_sim::systems::life::die(w, id, "fever".into());
    hf_sim::systems::flush::step(w);
    w.last_events.clone()
}

/// A tick's events holding one `Died` row for an id the world does not know.
fn died_event(w: &mut World, id: AgentId) -> Vec<Event> {
    let tick = w.meta.tick;
    let kind = EventKind::Died {
        agent: id,
        cause: "fever".into(),
    };
    w.events.push(tick, None, Severity::Major, kind);
    hf_sim::systems::flush::step(w);
    w.last_events.clone()
}

/// The report's four grief keys: (`grief_drop_2d`, `grief_drop_10d`, `grief_days`, `grief_n`).
fn grief_keys(r: &AliveReport) -> (f64, f64, f64, f64) {
    (
        key(r, "grief_drop_2d"),
        key(r, "grief_drop_10d"),
        key(r, "grief_days"),
        key(r, "grief_n"),
    )
}

/// The issue's case, hour by hour. A party of four: p0 and p1 partners, p2 and p3 flat at mood
/// 0.5, and p1 at 0.5 too, so p1's mood against the rest of their community is 0.0 at the
/// baseline hour. p0 dies (with the game's `die`), the tick's events are fed (when `feed`), and
/// then p1's mood falls by 0.5 at once and recovers linearly over `recover` hours while p2 and p3
/// stay flat; the tally looks every hour for a day longer than that.
fn scripted_death(recover: u32, feed: bool) -> AliveReport {
    let (mut w, p) = party(4);
    pair(&mut w, p[0], p[1]);
    for &i in &p {
        w.agents[i].mood = 0.5;
    }
    let base = w.meta.tick;
    let mut tally = AliveTally::default();
    tally.observe_hour(&w);
    let events = death(&mut w, p[0]);
    if feed {
        tally.observe_events(&w, &events);
    }
    for k in 0..=recover + 24 {
        w.meta.tick = base + u64::from(k) * TICKS_PER_HOUR;
        let drop = if k < recover {
            0.5 * (1.0 - k as f32 / recover as f32)
        } else {
            0.0
        };
        w.agents[p[1]].mood = 0.5 - drop;
        tally.observe_hour(&w);
    }
    tally.report()
}

#[test]
fn grief_curve_reads_a_scripted_death() {
    // The issue's scenario: a partner dies, the survivor's mood falls 0.5 and recovers linearly
    // over 12 days (288 hours) while the two others stay at 0.5.
    //   grief_n 1: one bereaved person (p1, p0's partner).
    //   The rest of p1's community is p2 and p3 (the dead p0 is not sampled), flat at 0.5, so p1's
    //   mood against them is mood - 0.5 = -drop, and the baseline (all four at 0.5) is 0.0.
    //   grief_drop_2d: the largest drop in the first 2 days is at the first look, 0.5 - 0 = 0.5.
    //   grief_drop_10d: the look at hour 240 (day 10): drop 0.5 * (1 - 240/288) = 0.5/6 = 0.0833.
    //   grief_days: back within 0.10 when 0.5 * (1 - k/288) <= 0.10, i.e. k >= 230.4, so at hour
    //   231 = 9.625 days. The issue wrote 12 (+-1) for this script; 12 days is when the mood is
    //   back at its baseline, but the rubric's return is "within 0.10 of baseline", which comes
    //   2.4 days sooner. A recovery over 16 days (384 hours) is back within 0.10 at k >= 307.2,
    //   hour 308 = 12.8333 days (the issue's 12 +- 1 is not a slower recovery: it comes from "back
    //   at baseline", a tolerance of 0, as the FIXLOG says), and its drop at day 10 is
    //   0.5 * (1 - 240/384) = 0.1875.
    let r = scripted_death(288, true);
    let (d2, d10, days, n) = grief_keys(&r);
    assert_eq!(n, 1.0);
    assert!(
        (d2 - 0.5).abs() < 1e-6,
        "drop_2d {d2} (the issue allowed +-0.02)"
    );
    assert!((d10 - 0.5 / 6.0).abs() < 1e-4, "drop_10d {d10}");
    assert!((days - 9.625).abs() < 1e-5, "grief_days {days}");
    let (d2, d10, days, n) = grief_keys(&scripted_death(384, true));
    assert_eq!(n, 1.0);
    assert!((d2 - 0.5).abs() < 1e-6, "drop_2d {d2}");
    assert!((d10 - 0.1875).abs() < 1e-4, "drop_10d {d10}");
    assert!((days - 308.0 / 24.0).abs() < 1e-4, "grief_days {days}");

    // Feeding the events changes only the grief keys: the same run without them has the other
    // rows as they are and the grief rows 0.
    let plain = scripted_death(288, false);
    for (k, v) in &plain.rows {
        if k.starts_with("grief_") {
            assert_eq!(*v, 0.0, "{k} without events");
        } else {
            assert_eq!(
                r.get(k),
                Some(*v),
                "{k} is the same with and without events"
            );
        }
    }
}

/// How a link ties the bereaved X to the dead D in `bereaved_cases` (indexes into `w.agents`).
type Link = fn(&mut World, usize, usize);

fn nop(_: &mut World, _: usize, _: usize) {}

struct Case {
    name: &'static str,
    /// Applied before the hour that sets the baseline.
    link: Link,
    /// Applied after the hour and D's death, before the events are fed.
    after: Link,
    /// Bereavements the tally opens when D dies.
    expect: f64,
}

#[test]
fn only_a_partner_parent_or_child_is_bereaved() {
    // Five people at mood 0.5; D is p0, X is p1. One look to set the baseline, D dies, the
    // tick's events are fed. Each case is run alone, so each expected 1 or 0 is that link's.
    //
    // What `die` leaves of a partnership (the reason a link is read from either end): it clears
    // the survivor's `partner` (`unpair`) and leaves the dead person's own.
    let (mut w, p) = party(2);
    pair(&mut w, p[0], p[1]);
    death(&mut w, p[0]);
    assert_eq!(w.agents[p[0]].partner, Some(w.agents[p[1]].id));
    assert_eq!(w.agents[p[1]].partner, None);
    let cases = [
        Case {
            name: "partner (both ways; `die` clears the survivor's end)",
            link: |w, d, x| pair(w, d, x),
            after: nop,
            expect: 1.0,
        },
        Case {
            name: "partner, named by the survivor alone after the death",
            link: nop,
            after: |w, d, x| w.agents[x].partner = Some(w.agents[d].id),
            expect: 1.0,
        },
        Case {
            name: "mother, named by the dead alone",
            link: |w, d, x| w.agents[d].mother = Some(w.agents[x].id),
            after: nop,
            expect: 1.0,
        },
        Case {
            name: "father, named by the dead alone",
            link: |w, d, x| w.agents[d].father = Some(w.agents[x].id),
            after: nop,
            expect: 1.0,
        },
        Case {
            name: "child, named by the dead's children alone",
            link: |w, d, x| {
                let child = w.agents[x].id;
                w.agents[d].children.push(child);
            },
            after: nop,
            expect: 1.0,
        },
        Case {
            name: "child, whose mother is the dead",
            link: |w, d, x| w.agents[x].mother = Some(w.agents[d].id),
            after: nop,
            expect: 1.0,
        },
        Case {
            name: "child, whose father is the dead",
            link: |w, d, x| w.agents[x].father = Some(w.agents[d].id),
            after: nop,
            expect: 1.0,
        },
        Case {
            name: "parent, whose children name the dead",
            link: |w, d, x| {
                let child = w.agents[d].id;
                w.agents[x].children.push(child);
            },
            after: nop,
            expect: 1.0,
        },
        Case {
            name: "sibling (the same mother, who is in no one's world)",
            link: |w, d, x| {
                w.agents[d].mother = Some(AgentId(900_000));
                w.agents[x].mother = Some(AgentId(900_000));
            },
            after: nop,
            expect: 0.0,
        },
        Case {
            name: "grandparent (parent of the dead's mother, who is in no one's world)",
            link: |w, d, x| {
                w.agents[d].mother = Some(AgentId(900_000));
                w.agents[x].children.push(AgentId(900_000));
            },
            after: nop,
            expect: 0.0,
        },
        Case {
            name: "friend (+1.0 both ways)",
            link: |w, d, x| {
                let t = w.meta.tick;
                tie(w, d, x, 1.0, t);
                tie(w, x, d, 1.0, t);
            },
            after: nop,
            expect: 0.0,
        },
        Case {
            name: "partner with no community (no baseline)",
            link: |w, d, x| {
                pair(w, d, x);
                w.agents[x].community = None;
            },
            after: nop,
            expect: 0.0,
        },
        Case {
            name: "partner already dead before the baseline hour",
            link: |w, d, x| {
                pair(w, d, x);
                w.agents[x].alive = false;
            },
            after: nop,
            expect: 0.0,
        },
        Case {
            name: "partner who dies after the baseline hour, before the events are fed",
            link: |w, d, x| pair(w, d, x),
            after: |w, _, x| w.agents[x].alive = false,
            expect: 0.0,
        },
        Case {
            name: "partner away since before the baseline hour",
            link: |w, d, x| {
                pair(w, d, x);
                let a = w.agents.remove(x);
                w.overworld.away.push(a);
            },
            after: nop,
            expect: 0.0,
        },
        Case {
            name: "partner who goes away after the baseline hour",
            link: |w, d, x| pair(w, d, x),
            after: |w, _, x| {
                let a = w.agents.remove(x);
                w.overworld.away.push(a);
            },
            expect: 0.0,
        },
        Case {
            name: "partner of a dead person who has no community",
            link: |w, d, x| {
                pair(w, d, x);
                w.agents[d].community = None;
            },
            after: nop,
            expect: 1.0,
        },
    ];
    for c in &cases {
        let (mut w, p) = party(5);
        for a in w.agents.iter().filter(|a| a.species == Species::Person) {
            assert!(a.partner.is_none() && a.mother.is_none() && a.father.is_none());
            assert!(a.children.is_empty(), "founders start with no kin");
        }
        for &i in &p {
            w.agents[i].mood = 0.5;
        }
        (c.link)(&mut w, p[0], p[1]);
        let mut tally = AliveTally::default();
        tally.observe_hour(&w);
        let events = death(&mut w, p[0]);
        (c.after)(&mut w, p[0], p[1]);
        tally.observe_events(&w, &events);
        let n = key(&tally.report(), "grief_n");
        assert_eq!(n, c.expect, "{}: grief_n {n}", c.name);
    }
}

#[test]
fn deaths_that_are_not_a_persons_loss_open_nothing() {
    // D (p0) and X (p1) are partners and X is baselined, in each of four worlds.
    let world = || {
        let (mut w, p) = party(5);
        for &i in &p {
            w.agents[i].mood = 0.5;
        }
        pair(&mut w, p[0], p[1]);
        let mut tally = AliveTally::default();
        tally.observe_hour(&w);
        (w, p, tally)
    };
    let n = |t: &AliveTally| key(&t.report(), "grief_n");

    // A tick without a `Died` row opens nothing, though D is dead in the world: the tally reads
    // the events it is given, not the world's dead.
    let (mut w, p, mut tally) = world();
    death(&mut w, p[0]);
    let x = w.agents[p[1]].id;
    let kind = EventKind::Ate {
        agent: x,
        item: None,
        quality: 0.5,
    };
    let tick = w.meta.tick;
    w.events.push(tick, None, Severity::Minor, kind);
    hf_sim::systems::flush::step(&mut w);
    let events = w.last_events.clone();
    assert!(events
        .iter()
        .all(|e| !matches!(e.kind, EventKind::Died { .. })));
    tally.observe_events(&w, &events);
    assert_eq!(n(&tally), 0.0, "no Died row");

    // The death of an animal is nobody's bereavement, even when a person names it: p1 is its
    // partner and its parent in the records (nonsense in the game; it would open one if the
    // species were not read).
    let (mut w, p, mut tally) = world();
    let beast = w
        .agents
        .iter()
        .position(|a| a.species != Species::Person && a.alive)
        .expect("founding populates the valley with animals");
    let (beast_id, x) = (w.agents[beast].id, w.agents[p[1]].id);
    w.agents[beast].partner = Some(x);
    w.agents[beast].children.push(x);
    w.agents[p[1]].children.push(beast_id);
    w.agents[p[1]].partner = Some(beast_id);
    let events = death(&mut w, beast);
    assert!(events
        .iter()
        .any(|e| matches!(&e.kind, EventKind::Died { agent, .. } if *agent == beast_id)));
    tally.observe_events(&w, &events);
    assert_eq!(n(&tally), 0.0, "an animal died");

    // A `Died` row for an id the world does not hold: the dead's own record cannot be read, but a
    // living person whose parent it was is still bereaved (1); a row nobody names opens nothing.
    let (mut w, p, mut tally) = world();
    let ghost = AgentId(900_001);
    w.agents[p[2]].mother = Some(ghost);
    let events = died_event(&mut w, ghost);
    tally.observe_events(&w, &events);
    assert_eq!(n(&tally), 1.0, "the child of an id found nowhere");
    let events = died_event(&mut w, AgentId(900_002));
    tally.observe_events(&w, &events);
    assert_eq!(n(&tally), 1.0, "a death nobody names adds none");
}

#[test]
fn a_death_is_one_bereavement_per_bereaved_person_however_often_it_is_fed() {
    // p1 is p0's partner and p2's child (p1's mother is p2), so p0 and p2 dying on one tick leave
    // p1 with two bereavements; p3 and p4 are unrelated. All five at 0.5, one baseline hour.
    // The first feed opens 2. A second feed of the same events, right away, opens none. Then p1
    // falls to 0.0 (0.5 below the unrelated p3 and p4 at 0.5), is looked at at the death and a
    // day later, and a third feed still opens none; each bereavement's drop within 2 days is 0.5,
    // so the median is 0.5.
    // Wrong versions: one bereavement per person reads 1; a feed that always opens reads 4 and
    // then 6.
    let (mut w, p) = party(5);
    for &i in &p {
        w.agents[i].mood = 0.5;
    }
    pair(&mut w, p[0], p[1]);
    w.agents[p[1]].mother = Some(w.agents[p[2]].id);
    let base = w.meta.tick;
    let mut tally = AliveTally::default();
    tally.observe_hour(&w);
    let mut events = death(&mut w, p[0]);
    events.extend(death(&mut w, p[2]));
    let died = events
        .iter()
        .filter(|e| matches!(e.kind, EventKind::Died { .. }))
        .count();
    assert_eq!(died, 2, "both deaths are in the list");
    let n = |t: &AliveTally| key(&t.report(), "grief_n");
    tally.observe_events(&w, &events);
    assert_eq!(n(&tally), 2.0);
    tally.observe_events(&w, &events);
    assert_eq!(n(&tally), 2.0, "the same events again");
    w.agents[p[1]].mood = 0.0;
    tally.observe_hour(&w);
    w.meta.tick = base + TICKS_PER_DAY;
    tally.observe_hour(&w);
    tally.observe_events(&w, &events);
    let r = tally.report();
    assert_eq!(n(&tally), 2.0, "the same events a day later");
    assert!((key(&r, "grief_drop_2d") - 0.5).abs() < 1e-6);
}

#[test]
fn the_baseline_is_the_latest_hour_before_the_death() {
    // p0 and p1 are partners; p2 and p3 sit at 0.0 throughout.
    //   hour -2: p1 at 0.25, so 0.25 against the rest (p0, p2, p3 at 0.0);
    //   hour -1: p1 at 0.5: baseline 0.5.
    // p0 dies at the base tick. Before the death is fed, p1's mood is already 0.25 (grief that
    // came with the death). The looks after: base 0.25, +1 hour 0.0, +2 hours 0.125, so the drops
    // against 0.5 are 0.25, 0.5 and 0.375: grief_drop_2d is 0.5.
    // Wrong versions: the baseline of the first hour (0.25) reads drops 0, 0.25, 0.125, so 0.25;
    // a baseline taken from the world when the death is fed (0.25) reads the same 0.25; a
    // baseline of 0.0 reads drops of -0.25, 0.0 and -0.125: 0.0.
    let (mut w, p) = party(4);
    pair(&mut w, p[0], p[1]);
    for &i in &p {
        w.agents[i].mood = 0.0;
    }
    let base = w.meta.tick;
    let mut tally = AliveTally::default();
    w.meta.tick = base - 2 * TICKS_PER_HOUR;
    w.agents[p[1]].mood = 0.25;
    tally.observe_hour(&w);
    w.meta.tick = base - TICKS_PER_HOUR;
    w.agents[p[1]].mood = 0.5;
    tally.observe_hour(&w);
    w.meta.tick = base;
    let events = death(&mut w, p[0]);
    w.agents[p[1]].mood = 0.25;
    tally.observe_events(&w, &events);
    for (k, m) in [(0, 0.25), (1, 0.0), (2, 0.125)] {
        w.meta.tick = base + k * TICKS_PER_HOUR;
        w.agents[p[1]].mood = m;
        tally.observe_hour(&w);
    }
    assert_eq!(grief_keys(&tally.report()).0, 0.5);
}

#[test]
fn a_death_with_no_baseline_hour_opens_nothing() {
    // p0 and p1 are partners. A tally that has not looked at an hour has no baseline: the death
    // opens nothing, and an hour looked at afterwards does not open it late (events fed once).
    let (mut w, p) = party(4);
    pair(&mut w, p[0], p[1]);
    let mut tally = AliveTally::default();
    let events = death(&mut w, p[0]);
    tally.observe_events(&w, &events);
    assert_eq!(key(&tally.report(), "grief_n"), 0.0);
    tally.observe_hour(&w);
    assert_eq!(grief_keys(&tally.report()), (0.0, 0.0, 0.0, 0.0));

    // A person with no community at the last hour has none either, even if they belong to one
    // by the time of the death.
    let (mut w, p) = party(4);
    pair(&mut w, p[0], p[1]);
    let colony = w.agents[p[1]].community;
    w.agents[p[1]].community = None;
    let mut tally = AliveTally::default();
    tally.observe_hour(&w);
    w.agents[p[1]].community = colony;
    let events = death(&mut w, p[0]);
    tally.observe_events(&w, &events);
    assert_eq!(key(&tally.report(), "grief_n"), 0.0);
}

#[test]
fn a_look_at_tick_zero_and_a_death_there_do_not_trip_the_tally() {
    // The world's first tick: p0 and p1 are partners at 0.5 with p2 and p3. Baseline look at
    // tick 0, p0 dies at tick 0, p1 falls to 0.0 (0.5 below the others) at tick 0, and is back at
    // 0.5 a day later: n 1, drop 0.5, back at 1.0 days.
    let (mut w, p) = party(4);
    pair(&mut w, p[0], p[1]);
    for &i in &p {
        w.agents[i].mood = 0.5;
    }
    w.meta.tick = 0;
    let mut tally = AliveTally::default();
    tally.observe_hour(&w);
    let events = death(&mut w, p[0]);
    assert!(events
        .iter()
        .any(|e| matches!(e.kind, EventKind::Died { .. }) && e.tick == 0));
    tally.observe_events(&w, &events);
    w.agents[p[1]].mood = 0.0;
    tally.observe_hour(&w);
    w.meta.tick = TICKS_PER_DAY;
    w.agents[p[1]].mood = 0.5;
    tally.observe_hour(&w);
    let (d2, d10, days, n) = grief_keys(&tally.report());
    assert_eq!((n, d10, days), (1.0, 0.0, 1.0));
    assert!((d2 - 0.5).abs() < 1e-6, "drop_2d {d2}");
}

#[test]
fn grief_is_measured_against_the_rest_of_the_community() {
    // Five people: p0 (to die) at 0.0, p1 the partner at 0.5, p2 at 0.25, p3 at 0.0, p4 at -0.25.
    // Baseline: the rest of p1's community is p0, p2, p3, p4, mean 0.0, so the baseline is 0.5.
    // A colony-wide dip follows. Look 1 (at the death): the others fall by 0.375 to -0.125, -0.375
    // and -0.625 (mean -0.375, and the dead p0 is not in it) while p1 falls by 0.5 to 0.0, so p1
    // stands 0.0 + 0.375 = 0.375 above the rest: drop 0.5 - 0.375 = 0.125.
    // Look 2 (day 10): the others at -0.25, -0.5, -0.75 (mean -0.5), p1 at -0.25: 0.25 above the
    // rest, drop 0.25. p1 left (0.125 is over 0.10) and is not back, and the bereavement is only 10
    // days old at the last look: grief_days leaves it out (#1492 second round), so it reads 0.0.
    // Wrong versions: the raw mood drop reads 0.5 and 0.75; a mean that includes p1 gives a
    // baseline of 0.5 - 0.1 = 0.4 (mean 0.1 over the five) and, at look 1, p1 at 0.0 against a mean
    // of (0 - 0.125 - 0.375 - 0.625)/4 = -0.28125: a drop of 0.4 - 0.28125 = 0.11875; a mean
    // that includes the dead p0 (still at mood 0.0 in the world) is (0 - 1.125)/4 = -0.28125 at
    // look 1, so p1 stands 0.28125 above it: a drop of 0.5 - 0.28125 = 0.21875.
    let (mut w, p) = party(5);
    pair(&mut w, p[0], p[1]);
    let base = w.meta.tick;
    for (&i, m) in p.iter().zip([0.0, 0.5, 0.25, 0.0, -0.25]) {
        w.agents[i].mood = m;
    }
    let mut tally = AliveTally::default();
    tally.observe_hour(&w);
    let events = death(&mut w, p[0]);
    tally.observe_events(&w, &events);
    for (day, moods) in [
        (0, [0.0, -0.125, -0.375, -0.625]),
        (10, [-0.25, -0.25, -0.5, -0.75]),
    ] {
        w.meta.tick = base + day * TICKS_PER_DAY;
        for (&i, m) in p[1..].iter().zip(moods) {
            w.agents[i].mood = m;
        }
        tally.observe_hour(&w);
    }
    let (d2, d10, days, n) = grief_keys(&tally.report());
    assert_eq!((n, days), (1.0, 0.0));
    assert!((d2 - 0.125).abs() < 1e-6, "drop_2d {d2}");
    assert!((d10 - 0.25).abs() < 1e-6, "drop_10d {d10}");
}

#[test]
fn each_community_is_measured_against_its_own_mean() {
    // Eight people in two communities: p0 (to die), p1 (the partner), p2 and p3 in the founders'
    // community at 0.0, 0.5, 0.25 and -0.25; p4..p7 in another at 0.875. The rest of p1's
    // community is p0, p2, p3: mean 0.0, baseline 0.5. After the death p1 falls to 0.0 and the
    // rest (p2, p3) have mean 0.0: drop 0.5. A mean over everyone in the valley is
    // (0.0 + 0.25 - 0.25 + 3.5)/7 = 0.5 for the seven others at the baseline (so a baseline of
    // 0.0) and (0.25 - 0.25 + 3.5)/6 = 0.5833 for the six after the death, which puts p1 at
    // -0.5833: a drop of 0.5833.
    let (mut w, p) = party(8);
    pair(&mut w, p[0], p[1]);
    let other = Some(CommunityId(900_001));
    for (&i, m) in p
        .iter()
        .zip([0.0, 0.5, 0.25, -0.25, 0.875, 0.875, 0.875, 0.875])
    {
        w.agents[i].mood = m;
    }
    for &i in &p[4..] {
        w.agents[i].community = other;
    }
    assert!(w.agents[p[1]].community.is_some() && w.agents[p[1]].community != other);
    let mut tally = AliveTally::default();
    tally.observe_hour(&w);
    let events = death(&mut w, p[0]);
    tally.observe_events(&w, &events);
    w.agents[p[1]].mood = 0.0;
    tally.observe_hour(&w);
    let d2 = grief_keys(&tally.report()).0;
    assert!((d2 - 0.5).abs() < 1e-6, "drop_2d {d2}");
}

#[test]
fn a_bereaved_person_alone_in_their_community_has_no_mean_to_be_measured_against() {
    // Two people, p0 at 0.25 and p1 at 0.5, partners. Baseline: p1 is 0.25 above p0. p0 dies and
    // p1 is alone, at 0.0: there is no one to compare with, so no look is added. The bereavement
    // is still counted: n 1, and the other keys read as an empty set, 0.0. A version that
    // measured a lone person against 0.0 would read a drop of 0.25 - 0.0 = 0.25.
    let (mut w, p) = party(2);
    pair(&mut w, p[0], p[1]);
    w.agents[p[0]].mood = 0.25;
    w.agents[p[1]].mood = 0.5;
    let mut tally = AliveTally::default();
    tally.observe_hour(&w);
    let events = death(&mut w, p[0]);
    tally.observe_events(&w, &events);
    w.agents[p[1]].mood = 0.0;
    tally.observe_hour(&w);
    assert_eq!(grief_keys(&tally.report()), (0.0, 0.0, 0.0, 1.0));
}

#[test]
fn the_grief_keys_are_medians_over_bereavements() {
    // Seven people at mood 0.0: p0 (to die), p1 its partner, p2 its mother (p0's `mother`), p3 its
    // child (p3's `mother` is p0) and p4, p5, p6 unrelated. After the death p1, p2 and p3 sit at
    // -0.5, -0.25 and -0.125 and the rest at 0.0, at the death and again a day later at 0.0.
    // Six people are sampled. Each bereaved person is measured against the other five,
    // including the other two bereaved: with S = -0.875 the sum of the six moods, the drop of
    // someone at m is (S - 6m)/5:
    //   p1: (-0.875 + 3)/5 = 0.425;  p2: (-0.875 + 1.5)/5 = 0.125;  p3: (-0.875 + 0.75)/5 = -0.025.
    // A day later all are at 0.0 (a drop of 0). Each one's largest drop within 2 days is then
    // 0.425, 0.125 and, for p3, 0.0 (the later look beats its -0.025): grief_drop_2d is the
    // median, 0.125 (the mean is 0.1833, the greatest 0.425). Measured against the unrelated
    // three alone the first drops would be 0.5, 0.25 and 0.125 and the median 0.25.
    // grief_days: p1 and p2 left (0.425 and 0.125 are over 0.10) and are back at day 1; p3
    // (-0.025) never left: 0.0. The median of 0, 1, 1 is 1.0. There is no look at day 10.
    let (mut w, p) = party(7);
    pair(&mut w, p[0], p[1]);
    w.agents[p[0]].mother = Some(w.agents[p[2]].id);
    w.agents[p[3]].mother = Some(w.agents[p[0]].id);
    for &i in &p {
        w.agents[i].mood = 0.0;
    }
    let base = w.meta.tick;
    let mut tally = AliveTally::default();
    tally.observe_hour(&w);
    let events = death(&mut w, p[0]);
    tally.observe_events(&w, &events);
    for (&i, m) in p[1..4].iter().zip([-0.5, -0.25, -0.125]) {
        w.agents[i].mood = m;
    }
    tally.observe_hour(&w);
    w.meta.tick = base + TICKS_PER_DAY;
    for &i in &p[1..4] {
        w.agents[i].mood = 0.0;
    }
    tally.observe_hour(&w);
    let (d2, d10, days, n) = grief_keys(&tally.report());
    assert_eq!((n, d10, days), (3.0, 0.0, 1.0));
    assert!((d2 - 0.125).abs() < 1e-6, "drop_2d {d2}");
}

#[test]
fn bereavements_are_read_at_their_own_ages() {
    // Eight people in two communities so that each bereaved person's reference stays flat at
    // 0.0: A is p0 (D1), p1 (X1), p4, p5; B is p2 (D2), p3 (X2), p6, p7 (CommunityId 900_001).
    // Everyone is at 0.0 except as scripted, so a person's drop is minus their mood. X1 lost D1 at
    // day 0 and X2 lost D2 at day 5. X1's looks: day 0 -0.5, day 1 -0.25, day 2 -0.375,
    // day 3 -0.0625, then (day 5) 0.0, day 10 -0.125, day 12 0.0. X2's looks: day 5 -0.75,
    // day 6 -0.5, day 7 -0.5, day 10 -0.25, day 12 -0.5.
    //   grief_n 2.
    //   grief_drop_2d: X1's largest in its first 2 days is 0.5 (days 0, 1, 2: 0.5, 0.25, 0.375);
    //   X2's, over days 5, 6, 7 (up to 2 days after the death): 0.75, 0.5, 0.5 is 0.75. The
    //   median of 0.5 and 0.75 is 0.625.
    //   grief_drop_10d: X1's look at day 10 is 0.125. X2 is 7 days old at the last look and has
    //   no day-10 look: only X1's counts, 0.125. (X2's last look, at day 12, is -0.5: a version
    //   that read each person's latest look would give a median of (0.125 + 0.5)/2 = 0.3125.)
    //   grief_days: X1's drops 0.5, 0.25, 0.375, 0.0625: back at day 3. X2's drops 0.75, 0.5, 0.5,
    //   0.25 and 0.5 are all over 0.10: not back, and it is only 7 days old at the last look, so it
    //   is left out until it is 60 days old (#1492 second round): the median is X1's 3.0 alone.
    //   (Reading X2 as 60 gives the median of 3 and 60, 31.5; the dedicated test is
    //   `a_bereavement_not_back_is_left_out_of_grief_days_until_60_days_old`.)
    let (mut w, p) = party(8);
    pair(&mut w, p[0], p[1]);
    pair(&mut w, p[2], p[3]);
    for &i in &p[2..4] {
        w.agents[i].community = Some(CommunityId(900_001));
    }
    for &i in &p[6..] {
        w.agents[i].community = Some(CommunityId(900_001));
    }
    for &i in &p {
        w.agents[i].mood = 0.0;
    }
    let base = w.meta.tick;
    let at = |w: &mut World, day: u64| w.meta.tick = base + day * TICKS_PER_DAY;
    let mut tally = AliveTally::default();
    tally.observe_hour(&w);
    let events = death(&mut w, p[0]);
    tally.observe_events(&w, &events);
    let (x1, x2) = (p[1], p[3]);
    for (day, m) in [(0, -0.5), (1, -0.25), (2, -0.375), (3, -0.0625), (5, 0.0)] {
        at(&mut w, day);
        w.agents[x1].mood = m;
        tally.observe_hour(&w);
    }
    // Day 5, D2 dies: the look just before it is the baseline for X2 (both at 0.0).
    let events = death(&mut w, p[2]);
    tally.observe_events(&w, &events);
    for (day, m1, m2) in [
        (5, 0.0, -0.75),
        (6, 0.0, -0.5),
        (7, 0.0, -0.5),
        (10, -0.125, -0.25),
        (12, 0.0, -0.5),
    ] {
        at(&mut w, day);
        w.agents[x1].mood = m1;
        w.agents[x2].mood = m2;
        tally.observe_hour(&w);
    }
    let (d2, d10, days, n) = grief_keys(&tally.report());
    assert_eq!(n, 2.0);
    assert!((d2 - 0.625).abs() < 1e-6, "drop_2d {d2}");
    assert!((d10 - 0.125).abs() < 1e-6, "drop_10d {d10}");
    assert!((days - 3.0).abs() < 1e-5, "grief_days {days}");
}

/// What happens to the bereaved person at one look of a scripted bereavement.
#[derive(Clone, Copy)]
enum Look {
    Mood(f32),
    Away,
    Dead,
    Unhoused,
}
use Look::{Away, Dead, Mood, Unhoused};

/// A partner's death at the base tick, then looks at the given offsets (ticks from the death,
/// one hour of game time is 1,200, a day 28,800). Four people at mood 0.0: p0 dies, p1 is the
/// bereaved partner, p2 and p3 stay at 0.0, so p1's mood against the rest is their mood and
/// their drop is minus their mood; the baseline is 0.0. A `Mood` look brings p1 home first (from
/// away) and gives their community back.
fn grief_run(looks: &[(i64, Look)]) -> AliveReport {
    let (mut w, p) = party(4);
    pair(&mut w, p[0], p[1]);
    for &i in &p {
        w.agents[i].mood = 0.0;
    }
    let base = w.meta.tick;
    let (x, colony) = (w.agents[p[1]].id, w.agents[p[1]].community);
    let mut tally = AliveTally::default();
    tally.observe_hour(&w);
    let events = death(&mut w, p[0]);
    tally.observe_events(&w, &events);
    for &(offset, look) in looks {
        w.meta.tick = base.checked_add_signed(offset).unwrap();
        place(&mut w, x, colony, look);
        tally.observe_hour(&w);
    }
    tally.report()
}

/// Do `look` to the person `x` of the world: bring them home from away first, then give them a
/// mood (and their community back, `colony`), send them away, kill them or take their community.
fn place(w: &mut World, x: AgentId, colony: Option<CommunityId>, look: Look) {
    if let Some(at) = w.overworld.away.iter().position(|a| a.id == x) {
        let a = w.overworld.away.remove(at);
        w.agents.push(a);
    }
    let i = w.agents.iter().position(|a| a.id == x).unwrap();
    match look {
        Mood(m) => {
            w.agents[i].community = colony;
            w.agents[i].mood = m;
        }
        Away => {
            let a = w.agents.remove(i);
            w.overworld.away.push(a);
        }
        Dead => w.agents[i].alive = false,
        Unhoused => w.agents[i].community = None,
    }
}

/// One death with two bereaved: p0 dies, p1 is its partner (X) and p2 its child (Y, `mother` p0),
/// and `others` more people (p3 on) are unrelated. All are at mood 0.0 at the baseline hour, so
/// both baselines are 0.0. Each look is (ticks from the death, what happens to X, to Y); both are
/// placed before the tally looks. The report after the last look.
fn grief_run2(others: usize, looks: &[(i64, Look, Look)]) -> AliveReport {
    let (mut w, p) = party(3 + others);
    pair(&mut w, p[0], p[1]);
    w.agents[p[2]].mother = Some(w.agents[p[0]].id);
    for &i in &p {
        w.agents[i].mood = 0.0;
    }
    let base = w.meta.tick;
    let colony = w.agents[p[1]].community;
    let (x, y) = (w.agents[p[1]].id, w.agents[p[2]].id);
    let mut tally = AliveTally::default();
    tally.observe_hour(&w);
    let events = death(&mut w, p[0]);
    tally.observe_events(&w, &events);
    assert_eq!(
        key(&tally.report(), "grief_n"),
        2.0,
        "X and Y are both bereaved"
    );
    for &(offset, lx, ly) in looks {
        w.meta.tick = base.checked_add_signed(offset).unwrap();
        place(&mut w, x, colony, lx);
        place(&mut w, y, colony, ly);
        tally.observe_hour(&w);
    }
    tally.report()
}

/// A scripted curve: its name, its looks, and the expected `grief_drop_2d`, `grief_drop_10d` and
/// `grief_days`.
type CurveCase = (&'static str, Vec<(i64, Look)>, f64, f64, f64);

#[test]
fn the_grief_curve_is_read_at_its_edges() {
    // Each case is one bereaved partner and a list of looks; the expected (drop_2d, drop_10d,
    // days) are worked out in the comment on each. The drop at a look is minus the mood. A
    // person is "left" at the first look more than 0.10 below the baseline and "back" at the
    // next look no more than 0.10 below it (so above the baseline is back); never left reads 0.0
    // days and left but never back reads 60.0. The 2-day key is the largest drop over looks at
    // or before 2 days (57,600 ticks) and the 10-day key the drop at the first look from 10 days
    // (288,000) on that comes before 11 days; a key with no look to read is 0.0. A person who
    // left and is not back is left out of `grief_days` until the bereavement is 60 days old at the
    // latest look (1,728,000 ticks; the `Dead` and `Away` looks count as looks), so a lone one
    // that is younger reads 0.0 (a median over nothing); at 60 days it reads 60.0.
    let d = DAY;
    let h = HOUR;
    let cases: Vec<CurveCase> = vec![
        // drops 0.5, 0.1, 0: left at 0, back at day 1 (0.10 exactly counts).
        (
            "exactly 0.10 is back",
            vec![(0, Mood(-0.5)), (d, Mood(-0.1)), (2 * d, Mood(0.0))],
            0.5,
            0.0,
            1.0,
        ),
        // drops 0.5, 0.0999, 0: back at day 1.
        (
            "under 0.10 is back",
            vec![(0, Mood(-0.5)), (d, Mood(-0.0999)), (2 * d, Mood(0.0))],
            0.5,
            0.0,
            1.0,
        ),
        // drops 0.5, 0.1001, 0: not back until day 2.
        (
            "over 0.10 is not back",
            vec![(0, Mood(-0.5)), (d, Mood(-0.1001)), (2 * d, Mood(0.0))],
            0.5,
            0.0,
            2.0,
        ),
        // drops 0.5, -0.3, 0: above the baseline is back at day 1.
        (
            "above the baseline is back",
            vec![(0, Mood(-0.5)), (d, Mood(0.3)), (2 * d, Mood(0.0))],
            0.5,
            0.0,
            1.0,
        ),
        // drops 0.05 and 0.02: never over 0.10; the first look is an hour in, not at the death.
        (
            "never left, first look an hour in",
            vec![(h, Mood(-0.05)), (d, Mood(-0.02))],
            0.05,
            0.0,
            0.0,
        ),
        // drops 0.10 exactly and 0: 0.10 is not "over", so never left.
        (
            "exactly 0.10 has not left",
            vec![(h, Mood(-0.1)), (d, Mood(0.0))],
            0.1,
            0.0,
            0.0,
        ),
        // drops 0.1001 and 0: left at the hour, back at day 1.
        (
            "0.1001 has left",
            vec![(h, Mood(-0.1001)), (d, Mood(0.0))],
            0.1001,
            0.0,
            1.0,
        ),
        // drops 0, 0.5, 0: within 0.10 at the death, left at day 1, back at day 3.
        (
            "a drop that comes later",
            vec![(0, Mood(0.0)), (d, Mood(-0.5)), (3 * d, Mood(0.0))],
            0.5,
            0.0,
            3.0,
        ),
        // drops 0.5, 0.3 (day 10), 0.2 (day 59), 0.2 (day 60): never back, and 60 days old.
        (
            "never back",
            vec![
                (0, Mood(-0.5)),
                (10 * d, Mood(-0.3)),
                (59 * d, Mood(-0.2)),
                (60 * d, Mood(-0.2)),
            ],
            0.5,
            0.3,
            60.0,
        ),
        // the same, one tick short of 60 days at the last look: not 60 days old yet, left out.
        (
            "never back, a tick short of 60 days",
            vec![
                (0, Mood(-0.5)),
                (10 * d, Mood(-0.3)),
                (59 * d, Mood(-0.2)),
                (60 * d - 1, Mood(-0.2)),
            ],
            0.5,
            0.3,
            0.0,
        ),
        // drops 0.05 at an hour and 0.5 at exactly day 60: the point at 60 days is on the curve
        // (it is `<= 60 days`), so the person left at day 60 and is not back: 60.0. The 2-day key
        // reads the hour's 0.05; no point from 10 days to before 11 days: 0.0. (Ending the curve at
        // `< 60 days` drops that point: never left, days 0.0.)
        (
            "the first point over 0.10 is at day 60",
            vec![(h, Mood(-0.05)), (60 * d, Mood(-0.5))],
            0.05,
            0.0,
            60.0,
        ),
        // back one hour before day 60: (60 * 28,800 - 1,200) / 28,800 = 59.9583.
        (
            "back an hour before day 60",
            vec![(0, Mood(-0.5)), (60 * d - h, Mood(0.0))],
            0.5,
            0.0,
            59.958_333,
        ),
        // back exactly at day 60 reads 60 (the same as never); one tick later the look is past
        // the curve's end and is not added: 60.0 exactly (adding it would read 60.0000347).
        (
            "back at day 60",
            vec![(0, Mood(-0.5)), (60 * d, Mood(0.0))],
            0.5,
            0.0,
            60.0,
        ),
        (
            "back a tick after day 60",
            vec![(0, Mood(-0.5)), (60 * d + 1, Mood(0.0))],
            0.5,
            0.0,
            60.0,
        ),
        // drops 0.5, 0, 0.5, 0: the first return, day 1.
        (
            "back, then low again, then back",
            vec![
                (0, Mood(-0.5)),
                (d, Mood(0.0)),
                (2 * d, Mood(-0.5)),
                (3 * d, Mood(0.0)),
            ],
            0.5,
            0.0,
            1.0,
        ),
        // 2-day edges: drops 0.25, 0.625 (exactly at 2 days), 0.875 (one tick after): 0.625.
        (
            "2 days is in the window, a tick later is not",
            vec![
                (0, Mood(-0.25)),
                (2 * d, Mood(-0.625)),
                (2 * d + 1, Mood(-0.875)),
            ],
            0.625,
            0.0,
            0.0,
        ),
        // drops 0.25, 0.75, 0.5: the largest, not the first or the last.
        (
            "the largest drop, not the first or last",
            vec![(0, Mood(-0.25)), (d, Mood(-0.75)), (2 * d, Mood(-0.5))],
            0.75,
            0.0,
            0.0,
        ),
        // drops -0.25 and -0.5: above the baseline; the key is -0.25, not clamped at 0.
        (
            "a rise is a negative drop",
            vec![(0, Mood(0.25)), (d, Mood(0.5))],
            -0.25,
            0.0,
            0.0,
        ),
        // the only look is 2 days and a tick in: nothing in the 2-day window.
        (
            "no look within 2 days",
            vec![(2 * d + 1, Mood(-0.5))],
            0.0,
            0.0,
            0.0,
        ),
        // 10-day edges: drops 0.5, 0.375 (a tick before 10 days), 0.125 (at 10 days), 0.25 (a tick
        // after): the first look from the mark on, 0.125.
        (
            "10 days: the first look from the mark",
            vec![
                (0, Mood(-0.5)),
                (10 * d - 1, Mood(-0.375)),
                (10 * d, Mood(-0.125)),
                (10 * d + 1, Mood(-0.25)),
            ],
            0.5,
            0.125,
            0.0,
        ),
        (
            "no look from 10 days on",
            vec![(0, Mood(-0.5)), (10 * d - 1, Mood(-0.375))],
            0.5,
            0.0,
            0.0,
        ),
        (
            "a look a tick before 11 days is the day-10 look",
            vec![(0, Mood(-0.5)), (11 * d - 1, Mood(-0.25))],
            0.5,
            0.25,
            0.0,
        ),
        (
            "a look at 11 days is not",
            vec![(0, Mood(-0.5)), (11 * d, Mood(-0.25))],
            0.5,
            0.0,
            0.0,
        ),
        // looks at one tick: the later replaces the earlier (drops 0.5 then 0.125 at day 10).
        (
            "a second look at day 10's tick replaces the first",
            vec![
                (0, Mood(-0.5)),
                (10 * d, Mood(-0.5)),
                (10 * d, Mood(-0.125)),
            ],
            0.5,
            0.125,
            0.0,
        ),
        // drops 0.5 then 0.125 at the death's tick: the later replaces the earlier.
        (
            "a second look at the death's tick replaces the first",
            vec![(0, Mood(-0.5)), (0, Mood(-0.125))],
            0.125,
            0.0,
            0.0,
        ),
        // looks at day 0 (0.5), day 2 (0.25), then an earlier one at day 1 (0.75): the earlier
        // look is not added, so the largest within 2 days is 0.5.
        (
            "a look at an earlier tick is not added",
            vec![(0, Mood(-0.5)), (2 * d, Mood(-0.25)), (d, Mood(-0.75))],
            0.5,
            0.0,
            0.0,
        ),
        // a look half an hour before the death (drop 0.875) is not a sample of the grief: the
        // largest drop is the later look's 0.25 (counted as a look at the death it would be 0.875).
        (
            "a look before the death is not added",
            vec![(-h / 2, Mood(-0.875)), (h, Mood(-0.25))],
            0.25,
            0.0,
            0.0,
        ),
        // away from day 2 to day 8: no looks in between; back at day 8; the day-10 look 0.125.
        (
            "away for a while, back at day 8",
            vec![
                (0, Mood(-0.5)),
                (d, Mood(-0.25)),
                (2 * d, Away),
                (8 * d, Mood(0.0)),
                (10 * d, Mood(-0.125)),
            ],
            0.5,
            0.125,
            8.0,
        ),
        // dies at day 2: no looks after (drops 0.5, 0.25): nothing at day 10, never back.
        (
            "dies on day 2",
            vec![
                (0, Mood(-0.5)),
                (d, Mood(-0.25)),
                (2 * d, Dead),
                (10 * d, Dead),
            ],
            0.5,
            0.0,
            0.0,
        ),
        // dies at day 2 and is looked at to day 60 (drops 0.5, 0.25): not back, 60 days old: 60.0.
        (
            "dies on day 2, looked at to day 60",
            vec![
                (0, Mood(-0.5)),
                (d, Mood(-0.25)),
                (2 * d, Dead),
                (60 * d, Dead),
            ],
            0.5,
            0.0,
            60.0,
        ),
        // away from day 2 and still away at day 60 (drops 0.5, 0.25): 60.0, as one who never came
        // back; a day short of it the bereavement is left out.
        (
            "away from day 2 to day 60",
            vec![
                (0, Mood(-0.5)),
                (d, Mood(-0.25)),
                (2 * d, Away),
                (60 * d, Away),
            ],
            0.5,
            0.0,
            60.0,
        ),
        (
            "away from day 2 to day 59",
            vec![
                (0, Mood(-0.5)),
                (d, Mood(-0.25)),
                (2 * d, Away),
                (59 * d, Away),
            ],
            0.5,
            0.0,
            0.0,
        ),
        // back at day 1, dies at day 2: back stays at day 1.
        (
            "back, then dies",
            vec![(0, Mood(-0.5)), (d, Mood(0.0)), (2 * d, Dead)],
            0.5,
            0.0,
            1.0,
        ),
        // leaves the community at day 1 and has no looks until day 2, when back at 0.0.
        (
            "out of the community for a day",
            vec![(0, Mood(-0.5)), (d, Unhoused), (2 * d, Mood(0.0))],
            0.5,
            0.0,
            2.0,
        ),
    ];
    for (name, looks, want_2d, want_10d, want_days) in cases {
        let (d2, d10, days, n) = grief_keys(&grief_run(&looks));
        assert_eq!(n, 1.0, "{name}: grief_n");
        assert!(
            (d2 - want_2d).abs() < 1e-5,
            "{name}: drop_2d {d2}, want {want_2d}"
        );
        assert!(
            (d10 - want_10d).abs() < 1e-5,
            "{name}: drop_10d {d10}, want {want_10d}"
        );
        assert!(
            (days - want_days).abs() < 1e-5,
            "{name}: grief_days {days}, want {want_days}"
        );
    }
}

#[test]
fn a_bereavement_not_back_is_left_out_of_grief_days_until_60_days_old() {
    // One death, two bereaved, p3..p6 unrelated, all at 0.0 at the baseline hour (both baselines
    // 0.0). Look 1 (at the death): X and Y both at -0.5. The five others of X are Y (-0.5) and
    // four at 0.0, mean -0.1, so X stands -0.5 + 0.1 = -0.4 against them: drop 0.4; the same for Y.
    // Look 2 (day 1): X at 0.0, Y still at -0.5: X stands 0.0 + 0.1 = +0.1 against the rest, drop
    // -0.1: back at day 1 (days 1.0). Y stands -0.5 against four at 0.0 and X at 0.0: drop 0.5.
    // Then a last look at T with X at 0.0 and Y at -0.5. grief_n 2; grief_drop_2d is the median of
    // X's largest 0.4 and Y's 0.5: 0.45; no day-10 look: grief_drop_10d 0.0.
    //   T = day 5: Y is 5 days old and still low: left out of grief_days, so it is X's 1.0 alone.
    //   T = day 60 less a tick: still not 60 days old at the last look: 1.0.
    //   T = day 60: Y is 60 days old and still low: it reads 60, so the median of 1 and 60 is 30.5.
    //   T = day 60 and a tick: the look past the curve's end is not added to it, but is the latest
    //   look and Y is over 60 days old: 30.5 too.
    // Wrong versions: Y read as 60 however young (30.5 at day 5 and at day 60 less a tick); Y never
    // read (1.0 at day 60); the age tested with `>` rather than `>=` (1.0 at exactly 60 days); the
    // age taken from Y's last point rather than the latest look (the away check below).
    let at = |t: i64| {
        let looks = [
            (0, Mood(-0.5), Mood(-0.5)),
            (DAY, Mood(0.0), Mood(-0.5)),
            (t, Mood(0.0), Mood(-0.5)),
        ];
        grief_keys(&grief_run2(4, &looks))
    };
    for (t, want_days) in [
        (5 * DAY, 1.0),
        (60 * DAY - 1, 1.0),
        (60 * DAY, 30.5),
        (60 * DAY + 1, 30.5),
    ] {
        let (d2, d10, days, n) = at(t);
        assert_eq!((n, d10), (2.0, 0.0), "T {t}");
        assert!((d2 - 0.45).abs() < 1e-5, "T {t}: drop_2d {d2}");
        assert!(
            (days - want_days).abs() < 1e-5,
            "T {t}: grief_days {days}, want {want_days}"
        );
    }
    // Y is away at the last look, a day short of 60 and then at 60 days: the age is read from the
    // latest look, not from Y's last point (day 1): 1.0, then 30.5.
    for (t, want_days) in [(59 * DAY, 1.0), (60 * DAY, 30.5)] {
        let looks = [
            (0, Mood(-0.5), Mood(-0.5)),
            (DAY, Mood(0.0), Mood(-0.5)),
            (t, Mood(0.0), Away),
        ];
        let days = grief_keys(&grief_run2(4, &looks)).2;
        assert!(
            (days - want_days).abs() < 1e-5,
            "Y away at {t}: grief_days {days}, want {want_days}"
        );
    }
}

#[test]
fn a_bereaved_person_with_no_look_in_two_days_is_not_a_zero_drop() {
    // One death, two bereaved, p3 and p4 unrelated, all at 0.0 at the baseline hour. At the death
    // X is at -0.5 and Y is out of any community (so not sampled and without a point); the rest
    // of X are p3 and p4 at 0.0: X stands -0.5, drop 0.5. Three days later both are back at 0.0
    // (Y gets its community back): X's drop is 0.0 and Y's first point, at 3 days, has drop 0.0.
    //   grief_n 2. grief_drop_2d: only X has a point within 2 days, its largest drop 0.5; Y has
    //   none and is left out: 0.5. (Counting Y as a drop of 0 reads the median of 0.5 and 0.0,
    //   0.25.) grief_drop_10d: no point from 10 days: 0.0. grief_days: X left at the death and is
    //   back at day 3: 3.0; Y never left: 0.0; the median of 3 and 0 is 1.5.
    let looks = [(0, Mood(-0.5), Unhoused), (3 * DAY, Mood(0.0), Mood(0.0))];
    let (d2, d10, days, n) = grief_keys(&grief_run2(2, &looks));
    assert_eq!((n, d10), (2.0, 0.0));
    assert!((d2 - 0.5).abs() < 1e-6, "drop_2d {d2}");
    assert!((days - 1.5).abs() < 1e-6, "grief_days {days}");
}

#[test]
fn a_bereaved_person_never_looked_at_is_not_a_zero_in_grief_days() {
    // One death, two bereaved, p3 and p4 unrelated, all at 0.0 at the baseline hour. Y is out of
    // any community at every look and has no point at all. X is at -0.5 at the death (stands -0.5
    // against p3 and p4 at 0.0: drop 0.5) and back at 0.0 a day later.
    //   grief_n 2 (Y is counted). grief_days: X is back at day 1: 1.0; Y has no curve and is left
    //   out: 1.0. (Reading Y as 0.0 days gives the median of 1 and 0, 0.5.) grief_drop_2d 0.5 and
    //   grief_drop_10d 0.0 rest on X alone.
    let looks = [(0, Mood(-0.5), Unhoused), (DAY, Mood(0.0), Unhoused)];
    let (d2, d10, days, n) = grief_keys(&grief_run2(2, &looks));
    assert_eq!((n, d10), (2.0, 0.0));
    assert!((d2 - 0.5).abs() < 1e-6, "drop_2d {d2}");
    assert!((days - 1.0).abs() < 1e-6, "grief_days {days}");
}

// ---- #1823: cause_repeat and lesson_ignored ----

#[test]
fn death_with_a_held_lesson_is_counted() {
    // A party of 3 in one community. One person holds a strong lesson about water depth, pushed
    // straight into `learning.lessons` (the normal `learn` path starts a lesson at 0.2 or 0.35,
    // so a direct push is the only way to get 0.8 without many confirmations). Both are killed
    // with the game's own `life::die` and the same cause, "drowned through the ice": the first
    // death is nobody's repeat (nothing has died of it yet in this community) and the victim held
    // a matching lesson, so it is `lesson_ignored`; the second death's cause has now killed once
    // before in the same community, so it is `cause_repeat`, and its victim has no lesson at all,
    // so it is not `lesson_ignored`.
    use hf_sim::mind::learning::{Lesson, ObservationKind, OutcomeKind};

    let (mut w, p) = party(3);
    let lesson_holder = p[0];
    w.agents[lesson_holder].learning.lessons.push(Lesson {
        cue: ObservationKind::WaterDepth,
        outcome: OutcomeKind::Death,
        pos: None,
        confidence: 0.8,
        formed: w.meta.tick,
        last_tested: w.meta.tick,
        confirmations: 0,
        contradictions: 0,
        origins: Vec::new(),
    });
    assert!(
        w.agents[p[1]].learning.lessons.is_empty(),
        "the second victim holds no lesson"
    );

    let mut tally = AliveTally::default();
    tally.observe_hour(&w);

    let id = w.agents[lesson_holder].id;
    hf_sim::systems::life::die(&mut w, id, "drowned through the ice".into());
    hf_sim::systems::flush::step(&mut w);
    tally.observe_events(&w, &w.last_events.clone());

    let id = w.agents[p[1]].id;
    hf_sim::systems::life::die(&mut w, id, "drowned through the ice".into());
    hf_sim::systems::flush::step(&mut w);
    tally.observe_events(&w, &w.last_events.clone());

    let r = tally.report();
    assert_eq!(key(&r, "cause_repeat"), 1.0);
    assert_eq!(key(&r, "lesson_ignored"), 1.0);
}

/// OST-867: the table of what the client draws names every activity, in the order of the
/// counters, and marks the activities drawn as plain standing.
#[test]
fn the_readable_table_names_every_activity_and_marks_plain_standing() {
    use hf_sim::view::alive::{drawn, work_drawn};
    use hf_sim::work::DesignationKind;
    let (w, idx) = party(1);
    let _ = (&w, &idx);
    let pos = TilePos::new(1, 1, 2);
    let all = [
        Activity::Idle,
        Activity::Going {
            to: pos,
            then: Box::new(Activity::Idle),
        },
        Activity::Eating {
            item: ItemId(1),
            ticks_left: 1,
        },
        Activity::Foraging {
            plant: pos,
            ticks_left: 1,
        },
        Activity::Sleeping {
            bed: None,
            until: 1,
        },
        Activity::Talking {
            with: AgentId(1),
            ticks_left: 1,
        },
        Activity::Loitering { ticks_left: 1 },
        Activity::Wandering { to: pos },
        Activity::Drinking {
            ticks_left: 1,
            back: None,
        },
    ];
    for a in &all {
        assert_eq!(drawn(a).activity, a.name());
    }
    assert!(drawn(&Activity::Idle).plain_standing);
    assert!(drawn(&Activity::Loitering { ticks_left: 1 }).plain_standing);
    assert!(
        !drawn(&Activity::Talking {
            with: AgentId(1),
            ticks_left: 1
        })
        .plain_standing
    );
    assert_eq!(work_drawn(&DesignationKind::Chop).0, "swing");
    assert_eq!(work_drawn(&DesignationKind::Cook).0, "tap");
}

/// OST-864: the report counts, per settled game-day, the rows that would reach the feed (Notable
/// or above) and the lines left after folding the same news.
#[test]
fn the_feed_repeat_keys_count_lines_before_and_after_folding() {
    use hf_sim::ids::EventId;
    let (w, _) = party(2);
    let a = w.agents[0].id;
    let b = w.agents[1].id;
    let at = |tick: u64, severity: Severity, kind: EventKind| Event {
        id: EventId(0),
        tick,
        at: None,
        severity,
        kind,
    };
    let suspect = |tick: u64| {
        at(
            tick,
            Severity::Notable,
            EventKind::Suspected {
                by: a,
                of: b,
                because: "x".into(),
            },
        )
    };
    let day = TICKS_PER_DAY;
    // Day 0: five suspicions of one person, one death, one Minor row (not in the feed).
    let mut events: Vec<Event> = (1..=5).map(suspect).collect();
    events.push(at(
        6,
        Severity::Major,
        EventKind::Died {
            agent: b,
            cause: "fever".into(),
        },
    ));
    events.push(at(
        7,
        Severity::Minor,
        EventKind::Talked { a, b, warmth: 0.1 },
    ));
    let mut tally = AliveTally::default();
    tally.observe_feed(&w, &events);
    // Day 0 is unfinished: nothing is counted yet.
    assert_eq!(tally.report().get("feed_lines_day"), Some(0.0));
    // The first row of day 1 settles day 0: 6 lines, 2 after folding (one suspicion line, one death).
    tally.observe_feed(&w, &[suspect(day + 1)]);
    let r = tally.report();
    assert_eq!(r.get("feed_lines_day"), Some(6.0));
    assert_eq!(r.get("feed_folded_day"), Some(2.0));
}
