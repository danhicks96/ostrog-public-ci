//! The sharing census (#1113, under #1112): `examples/sharing_census.rs`, included here so the
//! tests call the same helper the example prints from. Each number below is worked out by hand
//! from the scenario that is built, not read back from the code.

#[path = "../examples/sharing_census.rs"]
mod sharing_census;

use hf_sim::agents::Species;
use hf_sim::engine::Engine;
use hf_sim::events::{Event, EventKind, Severity};
use hf_sim::ids::{AgentId, CommunityId, EventId, TilePos};
use hf_sim::items::{ItemKind, ItemPlace};
use hf_sim::time::{TICKS_PER_HOUR, TICKS_PER_YEAR};
use hf_sim::world::gen::GenParams;
use hf_sim::World;
use sharing_census::{parse_args, run, starving_beside, world, Beside, Census};

/// A flat meadow with two communities, A and B, and nobody in it. The tick is on an hour.
fn meadow() -> (World, CommunityId, CommunityId) {
    let mut w = World::generate(1113, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.plants.clear();
    w.meta.tick = TICKS_PER_HOUR * 8;
    w.rebuild_caches();
    let a = w.found_community(TilePos::new(30, 30, 2));
    let b = w.found_community(TilePos::new(50, 50, 2));
    (w, a, b)
}

/// A person of `community` at `(x, y, z)` with the given hunger.
fn person(
    w: &mut World,
    community: Option<CommunityId>,
    x: i32,
    y: i32,
    z: i32,
    hunger: f32,
) -> AgentId {
    let id = w.spawn_agent(Species::Person, TilePos::new(x, y, z));
    let a = w.agents.iter_mut().find(|a| a.id == id).unwrap();
    a.community = community;
    a.needs.hunger = hunger;
    id
}

fn carry(w: &mut World, who: AgentId, kind: ItemKind, n: u32) {
    w.create_item(kind, n, ItemPlace::Carried(who), 0.6, "test");
}

fn row(rows: &[Beside], who: AgentId) -> Option<&Beside> {
    rows.iter().find(|r| r.who == who)
}

/// The scenario. Positions are on one level (z 2) unless said; A's people unless said.
///
/// - `child`, 6 years old, hunger 0.10 (starving), at (30,30), carries 3 berries of their own.
/// - `mother` at (35,30), 5 tiles: carries 2 meals and 3 berries (5 units) and 4 logs (not food).
///   She is the child's mother (kin) and, with opinion 0.8, also their friend: both at once.
/// - `thin` at (28,30), 2 tiles, hunger 0.08 (starving too): carries 5 roots. A starving holder
///   beside another starving person still counts as a holder.
/// - `friend` at (30,40), 10 tiles: carries 4 roots; the child's opinion of them is 0.5 (a friend).
/// - `neighbour` at (40,30), 10 tiles: carries 6 grain sheaves; the child's opinion is 0.49
///   (not a friend), no kin.
/// - `edge_in` at (50,30), exactly 20 tiles: carries 7 raw meat. In.
/// - `edge_out` at (51,30), 21 tiles: carries 9 roots. Out.
/// - `high` at (30,30) but 6 levels up (z 8): distance 0 + 6 * 4 = 24, so out; carries 11 roots.
/// - `stranger` of community B at (31,30), 1 tile: carries 8 meals. The child's father (kin), but
///   of another community. Out.
/// - `dead` of A at (32,30): carries 6 meals but is dead. Out.
/// - berries, 12 of them, on the ground at (30,31). Out (nobody carries them).
///
/// Also starving: `hungry` of A at (5,5), hunger 0.14, no one within 20 tiles; and `b_hungry`
/// of B at (32,31), hunger 0.05, beside `stranger` only. Not starving: `just_fed` of A at (31,31),
/// hunger exactly 0.15 (the rule is "under"); and `no_home`, no community, hunger 0.01, at (30,31).
/// A dead person with hunger 0.0 is `dead`.
///
/// Two of A's people are away (`w.overworld.away`), both within 20 tiles of the child: `away_hungry`
/// at (31,32), hunger 0.02, and `away_cook` at (32,32), hunger 0.9, still marked as the carrier of
/// 13 roots. (Most departures put what the person carried on the ground, but `taken.rs`
/// `down_again` does not, so a carried item can stay with someone in `away`; the carried items
/// are kept here so that a census which walked `away` would show.) Neither counts.
struct Scene {
    w: World,
    child: AgentId,
    hungry: AgentId,
    b_hungry: AgentId,
    just_fed: AgentId,
    no_home: AgentId,
    dead: AgentId,
    thin: AgentId,
    away_hungry: AgentId,
}

/// Take `who` off the map the way the expeditions do: out of `w.agents`, into `w.overworld.away`.
fn send_away(w: &mut World, who: AgentId) {
    let i = w.agents.iter().position(|a| a.id == who).unwrap();
    let a = w.agents.remove(i);
    w.overworld.away.push(a);
}

fn scene() -> Scene {
    let (mut w, a, b) = meadow();
    let tick = w.meta.tick;
    let child = person(&mut w, Some(a), 30, 30, 2, 0.10);
    let mother = person(&mut w, Some(a), 35, 30, 2, 0.9);
    let friend = person(&mut w, Some(a), 30, 40, 2, 0.9);
    let neighbour = person(&mut w, Some(a), 40, 30, 2, 0.9);
    let edge_in = person(&mut w, Some(a), 50, 30, 2, 0.9);
    let edge_out = person(&mut w, Some(a), 51, 30, 2, 0.9);
    let high = person(&mut w, Some(a), 30, 30, 8, 0.9);
    let stranger = person(&mut w, Some(b), 31, 30, 2, 0.9);
    let dead = person(&mut w, Some(a), 32, 30, 2, 0.0);
    let hungry = person(&mut w, Some(a), 5, 5, 2, 0.14);
    let b_hungry = person(&mut w, Some(b), 32, 31, 2, 0.05);
    let just_fed = person(&mut w, Some(a), 31, 31, 2, 0.15);
    let no_home = person(&mut w, None, 30, 31, 2, 0.01);
    let thin = person(&mut w, Some(a), 28, 30, 2, 0.08);
    let away_hungry = person(&mut w, Some(a), 31, 32, 2, 0.02);
    let away_cook = person(&mut w, Some(a), 32, 32, 2, 0.9);

    // The child is six, the mother's, and the father's (the father is of community B). The mother
    // is also a friend: opinion 0.8 is 0.5 or more.
    {
        let c = w.agents.iter_mut().find(|x| x.id == child).unwrap();
        c.born = tick as i64 - (TICKS_PER_YEAR * 6) as i64;
        c.mother = Some(mother);
        c.father = Some(stranger);
        c.relate(mother, 0.8, 0.1, tick);
        c.relate(friend, 0.5, 0.1, tick);
        c.relate(neighbour, 0.49, 0.1, tick);
    }
    w.agents
        .iter_mut()
        .find(|x| x.id == mother)
        .unwrap()
        .children
        .push(child);
    w.agents
        .iter_mut()
        .find(|x| x.id == stranger)
        .unwrap()
        .children
        .push(child);
    w.agents.iter_mut().find(|x| x.id == dead).unwrap().alive = false;

    carry(&mut w, child, ItemKind::Berries, 3);
    carry(&mut w, mother, ItemKind::Meal, 2);
    carry(&mut w, mother, ItemKind::Berries, 3);
    carry(&mut w, mother, ItemKind::Log, 4);
    carry(&mut w, friend, ItemKind::Root, 4);
    carry(&mut w, neighbour, ItemKind::GrainSheaf, 6);
    carry(&mut w, edge_in, ItemKind::RawMeat, 7);
    carry(&mut w, edge_out, ItemKind::Root, 9);
    carry(&mut w, high, ItemKind::Root, 11);
    carry(&mut w, stranger, ItemKind::Meal, 8);
    carry(&mut w, dead, ItemKind::Meal, 6);
    carry(&mut w, thin, ItemKind::Root, 5);
    carry(&mut w, away_cook, ItemKind::Root, 13);
    send_away(&mut w, away_hungry);
    send_away(&mut w, away_cook);
    w.create_item(
        ItemKind::Berries,
        12,
        ItemPlace::Ground(TilePos::new(30, 31, 2)),
        0.6,
        "test",
    );
    Scene {
        w,
        child,
        hungry,
        b_hungry,
        just_fed,
        no_home,
        dead,
        thin,
        away_hungry,
    }
}

#[test]
fn sharing_census_sees_a_starving_child_beside_a_carried_meal() {
    let s = scene();
    let now = s.w.meta.tick;
    let c = s.w.agents.iter().find(|a| a.id == s.child).unwrap();
    assert!(c.is_child(now), "the child is a child");
    let rows = starving_beside(&s.w);

    // The child: the mother's 5 units (2 meals + 3 berries; her 4 logs are not food), the
    // friend's 4 roots, the neighbour's 6 sheaves, the edge holder's 7 raw meat (20 tiles away
    // exactly) and the other starving person's 5 roots. 5 + 4 + 6 + 7 + 5 = 27 units from 5
    // holders. Not counted: their own 3 berries, 9 roots at 21 tiles, 11 roots 6 levels up (24),
    // the father's 8 meals (kin, but of community B), the dead person's 6 meals, the 12 berries
    // on the ground, and the away person's 13 roots. Kin or friend: the mother 5 (kin and
    // friend, counted once, not twice) + the friend 4 = 9; the neighbour (opinion 0.49), the edge
    // holder and `thin` are neither. A double count of the mother would give 14.
    assert_eq!(
        row(&rows, s.child),
        Some(&Beside {
            who: s.child,
            units: 27,
            kin_or_friend_units: 9,
            holders: 5,
        }),
        "the starving child's row"
    );
    // thin (0.08): beside the child's 3 berries, the mother's 5, the friend's 4 (10 tiles) and
    // the neighbour's 6 (12 tiles); the edge holder is 22 tiles off. 3 + 5 + 4 + 6 = 18 units
    // from 4 holders. Not kin to anyone, and thin has no opinion of anyone: 0 kin or friend.
    assert_eq!(
        row(&rows, s.thin),
        Some(&Beside {
            who: s.thin,
            units: 18,
            kin_or_friend_units: 0,
            holders: 4,
        }),
        "a starving holder is also a starving person with a row"
    );
    // hungry (0.14): no one within 20 tiles, so a row with nothing in it.
    assert_eq!(
        row(&rows, s.hungry),
        Some(&Beside {
            who: s.hungry,
            units: 0,
            kin_or_friend_units: 0,
            holders: 0,
        }),
        "starving and alone"
    );
    // b_hungry (B): only B's `stranger` is of their community, with 8 meals; none of A's food
    // counts for them. 8 units, 1 holder, not kin and the opinion 0: 0 kin or friend.
    assert_eq!(
        row(&rows, s.b_hungry),
        Some(&Beside {
            who: s.b_hungry,
            units: 8,
            kin_or_friend_units: 0,
            holders: 1,
        }),
        "the other community's starving person sees only their own"
    );
    // Four starving on the map: child, thin, hungry, b_hungry. The rest are not: hunger exactly
    // 0.15, no community, dead, and the fed. `away_hungry` (0.02) is starving but off the map.
    assert_eq!(rows.len(), 4, "rows: {rows:?}");
    assert!(
        row(&rows, s.away_hungry).is_none(),
        "people away are not sampled"
    );
    for r in &rows {
        assert!(
            r.kin_or_friend_units <= r.units,
            "kin/friend within all: {r:?}"
        );
    }
    assert!(row(&rows, s.just_fed).is_none(), "0.15 is not under 0.15");
    assert!(row(&rows, s.no_home).is_none(), "no community, no census");
    assert!(row(&rows, s.dead).is_none(), "the dead are not counted");
}

#[test]
fn the_census_samples_on_the_hour_and_sums_person_hours() {
    let mut s = scene();
    let mut c = Census::default();
    // Hour 8 exactly: 4 starving person-hours (child, thin, hungry, b_hungry); three of them
    // (child 27, thin 18, b_hungry 8) beside food, one (child) beside kin/friends' food;
    // 27 + 18 + 0 + 8 = 53 units, 9 + 0 + 0 + 0 = 9 of them kin/friends'.
    c.sample(&s.w);
    assert_eq!(
        (
            c.hours,
            c.starving,
            c.beside_food,
            c.beside_kin_food,
            c.units,
            c.kin_or_friend_units
        ),
        (1, 4, 3, 1, 53, 9)
    );
    // One tick past the hour is not a sample, and neither is the half hour (a census that
    // sampled every half hour would count each person-hour twice).
    s.w.meta.tick += 1;
    c.sample(&s.w);
    assert_eq!(c.hours, 1, "off the hour nothing is sampled");
    s.w.meta.tick = TICKS_PER_HOUR * 8 + TICKS_PER_HOUR / 2;
    c.sample(&s.w);
    assert_eq!(c.hours, 1, "the half hour is not a sample either");
    assert_eq!(
        c.starving, 4,
        "and no person-hour was added at the half hour"
    );
    // The next hour, the same scene again: every number is doubled.
    s.w.meta.tick = TICKS_PER_HOUR * 9;
    c.sample(&s.w);
    assert_eq!(
        (
            c.hours,
            c.starving,
            c.beside_food,
            c.beside_kin_food,
            c.units,
            c.kin_or_friend_units
        ),
        (2, 8, 6, 2, 106, 18)
    );
}

fn ev(n: u32, kind: EventKind) -> Event {
    Event {
        id: EventId(n),
        tick: 0,
        at: None,
        severity: Severity::Minor,
        kind,
    }
}

#[test]
fn the_census_counts_the_events_it_names_and_no_others() {
    let a = AgentId(1);
    let b = AgentId(2);
    let mut c = Census::default();
    c.note(&[
        ev(1, EventKind::Starving { agent: a }),
        ev(2, EventKind::Repaid { by: a, to: b }),
        ev(3, EventKind::Starving { agent: b }),
        ev(4, EventKind::Quarrelled { a, b }),
        ev(5, EventKind::Exhausted { agent: a }),
    ]);
    // 2 Starving, 1 Repaid; the quarrel and the exhaustion are neither. GaveFood, Stole and
    // SetAside do not exist as events yet, so the census has no count for them at all.
    assert_eq!((c.starving_lines, c.repaid), (2, 1));
    // A second call adds to the first.
    c.note(&[ev(6, EventKind::Repaid { by: b, to: a })]);
    assert_eq!(c.repaid, 2);
}

#[test]
fn an_empty_world_has_nothing_to_census() {
    let (w, _, _) = meadow();
    assert!(starving_beside(&w).is_empty());
    let mut c = Census::default();
    c.sample(&w);
    // An hour was sampled and nobody was starving: no divide by zero in the printed line.
    assert_eq!((c.hours, c.starving, c.units), (1, 0, 0));
    let line = c.line(1113, 0);
    assert!(
        line.contains("starving (hunger < 0.15) 0 person-hours"),
        "{line}"
    );
    assert!(line.contains("beside carried food 0 (0.0%)"), "{line}");
    // The three events that do not exist yet print as "n/a", never as a 0 that reads as measured.
    for name in ["gave_food", "stole", "set_aside"] {
        assert!(
            line.contains(&format!("{name} n/a (no such event yet)")),
            "{line}"
        );
        assert!(!line.contains(&format!("{name} 0")), "{line}");
    }
}

fn argv(words: &[&str]) -> Vec<String> {
    std::iter::once("sharing_census")
        .chain(words.iter().copied())
        .map(String::from)
        .collect()
}

/// The command line as the example reads it: (days, from, to, engine).
fn read(words: &[&str]) -> (u64, u64, u64, Engine) {
    let a = parse_args(&argv(words)).unwrap();
    (a.days, a.from, a.to, a.engine)
}

#[test]
fn the_engine_flag_is_read_wherever_it_stands_and_the_old_command_lines_still_mean_the_same() {
    // The old forms, no flag: Steady, 600 days, seeds 5 to 8, unless the numbers say otherwise.
    assert_eq!(read(&[]), (600, 5, 8, Engine::Steady));
    assert_eq!(read(&["30"]), (30, 5, 8, Engine::Steady));
    assert_eq!(read(&["30", "6", "6"]), (30, 6, 6, Engine::Steady));
    // The flag after the numbers, before them and between them, by pacing name or deity name.
    assert_eq!(
        read(&["30", "6", "6", "--engine", "relentless"]),
        (30, 6, 6, Engine::Relentless)
    );
    assert_eq!(
        read(&["--engine", "mirre", "30", "7", "7"]),
        (30, 7, 7, Engine::Whimsical)
    );
    assert_eq!(
        read(&["30", "--engine", "Whimsical", "7", "7"]),
        (30, 7, 7, Engine::Whimsical)
    );
    // The flag's value is not mistaken for a number: `--engine steady` leaves the days alone.
    assert_eq!(read(&["--engine", "steady"]), (600, 5, 8, Engine::Steady));
}

/// #1796: food taken from a household's chest with nobody watching. `report_missing_food`
/// stands in for the low-level "food vanished from the chest" primitive #1795 would give a
/// thief; this test is only about what the keeper does once the chest is short, not about the
/// stealing decision itself.
#[test]
fn an_unseen_theft_can_be_laid_on_the_wrong_person() {
    use hf_sim::justice;

    let chest = TilePos::new(30, 30, 2);
    let mut wrongly_blamed = 0;
    for seed in 1..=20u64 {
        let (mut w, cid, _b) = meadow();
        w.meta.seed = seed;
        let keeper = person(&mut w, Some(cid), 30, 30, 2, 0.6);
        let taker = person(&mut w, Some(cid), 31, 30, 2, 0.6);
        let neighbour = person(&mut w, Some(cid), 29, 31, 2, 0.6);

        // A disliked neighbour is near the chest too (per the issue's scenario). Which of the
        // two the keeper likes least varies by seed, so the mechanism sometimes names the
        // taker correctly and sometimes does not, exactly as the issue describes ("sometimes
        // the wrong person").
        let flip = seed.wrapping_mul(2_654_435_761) % 2 == 0;
        let ki = w.agents.iter().position(|a| a.id == keeper).unwrap();
        if flip {
            w.agents[ki].relate(neighbour, -0.9, 0.5, 0);
            w.agents[ki].relate(taker, -0.4, 0.5, 0);
        } else {
            w.agents[ki].relate(taker, -0.9, 0.5, 0);
            w.agents[ki].relate(neighbour, -0.4, 0.5, 0);
        }
        let opinion_of_taker_before = w.agents[ki].opinion_of(taker);

        // Nobody saw it happen: the chest is simply short when the keeper next checks it.
        justice::report_missing_food(&mut w, cid, taker, 2, chest);
        assert!(
            w.communities
                .iter()
                .find(|c| c.id == cid)
                .unwrap()
                .missing_food
                .iter()
                .any(|m| m.truly == taker),
            "seed {seed}: the shortage was not recorded"
        );

        // The keeper (standing at the chest already, awake) finds it short.
        justice::find_missing_food(&mut w);

        let c = w.communities.iter().find(|c| c.id == cid).unwrap();
        assert!(
            c.missing_food.is_empty(),
            "seed {seed}: the shortage was never cleared once found"
        );
        let acc = c
            .accused
            .iter()
            .find(|a| a.what == "missing food")
            .unwrap_or_else(|| panic!("seed {seed}: the keeper never opened an accusation"));
        assert_eq!(
            acc.by, keeper,
            "seed {seed}: someone other than the keeper accused"
        );
        assert_eq!(
            acc.truly, taker,
            "seed {seed}: the record lost who truly took it"
        );

        let ki = w.agents.iter().position(|a| a.id == keeper).unwrap();
        if acc.of != taker {
            wrongly_blamed += 1;
            // The keeper's opinion of the true taker never moved: only the one blamed is
            // held to account.
            assert_eq!(
                w.agents[ki].opinion_of(taker),
                opinion_of_taker_before,
                "seed {seed}: the keeper's opinion of the true taker moved though they were \
                 not the one blamed"
            );
        }
    }
    assert!(
        wrongly_blamed >= 1,
        "over 20 seeded cases, the keeper never once blamed the wrong person"
    );
}

#[test]
fn a_bad_or_missing_engine_name_is_an_error() {
    assert!(parse_args(&argv(&["30", "--engine", "loud"])).is_err());
    assert!(parse_args(&argv(&["30", "--engine"])).is_err());
}

#[test]
fn the_census_makes_its_world_with_the_engine_it_was_asked_for() {
    // Zero days: the world is made and nothing is stepped, so this is only the setup.
    for engine in Engine::ALL {
        assert_eq!(run(7, engine, 0).engine, engine);
        // The world itself carries the engine too: a census that copied the argument into its
        // result while making an engine-less world would pass the line above alone.
        assert_eq!(world(7, engine).meta.engine, engine);
    }
}

#[test]
fn the_sharing_line_names_the_engine() {
    let line_of = |engine| {
        Census {
            engine,
            ..Census::default()
        }
        .line(7, 30)
    };
    assert!(
        line_of(Engine::Whimsical).starts_with("SHARING seed 7 engine Whimsical days 30:"),
        "{}",
        line_of(Engine::Whimsical)
    );
    assert!(line_of(Engine::Relentless).contains(" engine Relentless "));
    assert!(line_of(Engine::Steady).contains(" engine Steady "));
    assert!(!line_of(Engine::Steady).contains("Whimsical"));
}
