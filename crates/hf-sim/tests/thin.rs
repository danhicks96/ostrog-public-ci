//! The thin place (thin.rs, issue #55): the portal stone wakes after two seasons; what happens
//! within its reach is drawn from what is recorded; people study it and choose; wards quiet it;
//! left open, it leads the living lost home.

use hf_sim::events::EventKind;
use hf_sim::mind::techniques::Technique;
use hf_sim::opening::SurvivorState;
use hf_sim::structures::StructureKind;
use hf_sim::time::TICKS_PER_DAY;
use hf_sim::World;

fn run_days(w: &mut World, days: u64, log: &mut Vec<hf_sim::events::Event>) {
    w.ensure_nav();
    for _ in 0..days * TICKS_PER_DAY {
        w.step();
        log.extend(w.last_events.iter().cloned());
    }
}

/// `f` for each item, on threads of their own (the runs are independent; one after another they
/// took minutes), results in the items' order. A panic in any run fails the test. At most
/// `hf_sim::par::threads()` run at once (`HF_SIM_THREADS`), so a CI run capped beside a soak on a
/// shared box stays capped; uncapped, that is the machine's cores up to eight, as before.
fn each<I: Copy + Send, T: Send>(items: &[I], f: impl Fn(I) -> T + Sync) -> Vec<T> {
    let f = &f;
    let mut out = Vec::with_capacity(items.len());
    for batch in items.chunks(hf_sim::par::threads().max(1)) {
        out.extend(std::thread::scope(|sc| {
            let handles: Vec<_> = batch.iter().map(|&s| sc.spawn(move || f(s))).collect();
            handles
                .into_iter()
                .map(|h| h.join().unwrap())
                .collect::<Vec<_>>()
        }));
    }
    out
}

fn each_seed<T: Send>(seeds: &[u64], f: impl Fn(u64) -> T + Sync) -> Vec<T> {
    each(seeds, f)
}

#[test]
fn the_stone_wakes_after_two_seasons() {
    let mut w = World::opening_world(3);
    let s = hf_sim::thin::stone(&w).expect("the stone stands where they came through");
    assert_eq!(s, w.meta.founding_site);
    let mut log = Vec::new();
    run_days(&mut w, 9, &mut log);
    assert!(w.thin.woke.is_none(), "not before two seasons");
    run_days(&mut w, 3, &mut log);
    assert!(w.thin.woke.is_some());
    assert!(log
        .iter()
        .any(|e| matches!(e.kind, EventKind::ThinStirred { .. })));
}

#[test]
fn what_happens_there_is_true_to_the_record() {
    // Four runs: each phenomenon is a daily roll, and which ones land depends on the path
    // (after C45's cultures, the first three runs saw one patch of wrong weather between them).
    // The runs are independent and each runs on its own thread. The 45 days stay: by day 20 the
    // four runs together had only three patches against the two asked for (probe, 2026-10-27).
    let counts = each_seed(&[1, 3, 7, 11], record_run);
    let things: usize = counts.iter().map(|c| c.0).sum();
    let echoes: usize = counts.iter().map(|c| c.1).sum();
    let patches: usize = counts.iter().map(|c| c.2).sum();
    assert!(
        things >= 1 && echoes >= 1 && patches >= 2,
        "{things} {echoes} {patches}"
    );
}

/// One run of `what_happens_there_is_true_to_the_record`: things from home, echoes and patches
/// of wrong weather seen in 45 days, each checked against the record as it happens.
fn record_run(seed: u64) -> (usize, usize, usize) {
    let (mut things, mut echoes, mut patches) = (0, 0, 0);
    let mut w = World::opening_world(seed);
    let s = hf_sim::thin::stone(&w).unwrap();
    w.ensure_nav();
    for t in 0..45 * TICKS_PER_DAY {
        // Staged: one of the lost dies of the cold on the second day, through the ordinary
        // death path, so there is a recorded death for an echo to tell once the stone
        // wakes (whether a lost survivor dies on their own depends on the run's path, and
        // most are found within two or three weeks).
        if t == 2 * TICKS_PER_DAY {
            let lost = w.survivor_pool.iter().find_map(|sl| match sl.state {
                SurvivorState::Lost {
                    agent,
                    found: false,
                } if w.agents.iter().any(|a| a.id == agent && a.alive) => Some(agent),
                _ => None,
            });
            if let Some(id) = lost {
                hf_sim::systems::life::die(&mut w, id, "froze".into());
            }
        }
        w.step();
        for e in w.last_events.clone() {
            match &e.kind {
                EventKind::WrongWeather { at, .. } => {
                    patches += 1;
                    assert!(at.dist(s) <= 15, "inside the stone's reach");
                }
                EventKind::ThingFromHome { item, maker, .. } => {
                    things += 1;
                    // Made by someone named in the recorded past of the one it came for.
                    let owner = w
                        .thin
                        .from_home
                        .iter()
                        .find(|x| x.0 == *item)
                        .map(|x| x.1)
                        .expect("it came for someone");
                    let a = w.agents.iter().find(|a| a.id == owner).unwrap();
                    let sl = w
                        .survivor_pool
                        .iter()
                        .find(|sl| hf_sim::opening::slot_name(seed, sl.index, sl.rerolls) == a.name)
                        .expect("one of those who came through");
                    let past = hf_sim::past::life(seed, sl.index, sl.rerolls, &a.traits);
                    assert!(
                        format!("{:?}", past.events).contains(maker.as_str()),
                        "{maker} is in {}'s recorded past",
                        a.name
                    );
                }
                EventKind::EchoSeen { of, cause, .. } => {
                    echoes += 1;
                    let d = w
                        .agents
                        .iter()
                        .find(|a| &a.name == of)
                        .expect("a real person");
                    assert!(!d.alive, "an echo is only ever of the dead");
                    assert_eq!(
                        d.death.as_deref(),
                        Some(cause.as_str()),
                        "and tells the truth"
                    );
                    // One of the lost, or (C68, #229) one of the valley's own who died
                    // with none of their people near.
                    assert!(
                        d.died_alone
                            || w.survivor_pool.iter().any(|sl| matches!(
                                sl.state,
                                SurvivorState::Lost { agent, .. } if agent == d.id
                            ))
                    );
                }
                _ => {}
            }
        }
    }
    (things, echoes, patches)
}

#[test]
fn they_study_it_and_the_wards_quiet_it() {
    let mut w = World::opening_world(7);
    // #486 changed the seed's trajectory and produced a curious chooser who left the stone
    // open. Stage non-curious people so this guard always exercises its studying/warding case;
    // every construction and quieting assertion below remains unchanged.
    for a in &mut w.agents {
        a.traits.list.retain(|t| *t != hf_sim::mind::Trait::Curious);
    }
    let mut log = Vec::new();
    // Who goes to sit by the stone is the run's lottery: with nobody curious, only those it
    // has shown an echo (thin.rs `seen`) or who have begun to learn its ways. Which of them
    // learns enough first, and when, moved with every change to how the day is spent (#553,
    // FIXLOG 2026-11-01: quieted on day 58, 64, 82, 89 or 95, or not by day 130, as each of
    // #553's five fixes was turned off in turn, against 59 on master `89a85fef`). So one of the founders
    // has been shown an echo at the arrival, and goes to sit there from when it wakes, as
    // anyone shown one does. Everything after, the learning, the choice, the wards and the
    // quiet, is the run's own.
    let first = w
        .agents
        .iter()
        .find(|a| a.species == hf_sim::agents::Species::Person && a.alive)
        .map(|a| a.id)
        .expect("a founder");
    let now = w.meta.tick;
    w.thin.seen.push((first, now));
    // Until it is quieted, then fifteen days more to see that nothing comes through after.
    // The cap is a ceiling, not a schedule: the run stops as soon as the stone is quiet.
    // When it is quieted depends on when someone first goes and sits by the stone (a chosen
    // errand, so it follows how people walk), then on the thin strength (which keeps growing
    // meanwhile, up to 1.0) wearing down 0.05 a day once four wards stand. It was day 46 on
    // this seed (2026-10-27); with #493's desire paths (FIXLOG 2026-11-01 #510 second entry)
    // the same camp first studies about day 60, the wards stand that day, and the stone is
    // quiet on day 81; #395's arrival affect then reshaped the opening again (day 87
    // on the branch before this merge, 2026-11-01). Nothing about the mechanic changed (four wards within reach still
    // quiet it), so the ceiling is 120 days, which costs nothing when it is quiet sooner.
    const CEILING: u64 = 120;
    let mut day = 0;
    while day < CEILING && !w.thin.quieted {
        run_days(&mut w, 1, &mut log);
        day += 1;
    }
    run_days(&mut w, 15.min(CEILING - day), &mut log);
    assert!(w
        .agents
        .iter()
        .any(|a| a.techniques.knows(Technique::ThinLore)));
    let (who, ward) = w.thin.chosen.expect("someone chose");
    assert!(ward);
    assert!(log.iter().any(|e| matches!(&e.kind,
        EventKind::Chose { agent, what, .. } if *agent == who && what.contains("warding"))));
    let wards = w
        .structures
        .iter()
        .filter(|s| s.kind == StructureKind::WardStone)
        .count();
    assert_eq!(wards, 4);
    assert!(w.thin.quieted);
    let quiet = log
        .iter()
        .find(|e| matches!(e.kind, EventKind::ThinQuieted { .. }))
        .unwrap()
        .tick;
    assert!(
        !log.iter().any(|e| e.tick > quiet
            && matches!(
                e.kind,
                EventKind::SharedDream { .. }
                    | EventKind::ThingFromHome { .. }
                    | EventKind::EchoSeen { .. }
                    | EventKind::WrongWeather { .. }
            )),
        "nothing comes through after"
    );
}

#[test]
fn left_open_it_leads_the_living_lost_home() {
    let led = each_seed(&[2, 5, 9], led_home)
        .into_iter()
        .filter(|&l| l)
        .count();
    assert!(
        led >= 2,
        "{led} valleys saw one of the lost led toward the stone"
    );
}

/// One run of `left_open_it_leads_the_living_lost_home`: whether one of the lost was led home
/// or is being led.
fn led_home(seed: u64) -> bool {
    let mut w = World::opening_world(seed);
    let Some(lost) = w.survivor_pool.iter().find_map(|sl| match sl.state {
        SurvivorState::Lost {
            agent,
            found: false,
        } => Some(agent),
        _ => None,
    }) else {
        return false;
    };
    // Someone has already chosen to leave it open, and it has grown thin.
    w.thin.woke = Some(w.meta.tick);
    w.thin.strength = 0.6;
    w.thin.chosen = Some((w.agents[0].id, false));
    let mut log = Vec::new();
    run_days(&mut w, 12, &mut log);
    let home = w.survivor_pool.iter().any(|sl| {
        matches!(sl.state, SurvivorState::Starting)
            && w.agents
                .iter()
                // Rejoined and gone off with a caravan is home too (as in tests/missing.rs).
                .chain(w.overworld.away.iter())
                .any(|a| a.id == lost && a.community.is_some())
    });
    home || w.thin.guiding.is_some()
}

#[test]
fn nothing_grows_in_the_hollow() {
    let mut w = World::opening_world(3);
    let s = hf_sim::thin::stone(&w).unwrap();
    w.thin.woke = Some(w.meta.tick);
    w.thin.strength = 0.9;
    // A young plant beside the stone.
    let p = s.offset(1, 1, 0);
    w.plants.retain(|q| q.pos != p);
    w.plants.push(hf_sim::plants::Plant::new(
        p,
        hf_sim::plants::PlantKind::Shrub,
        0.2,
        1,
    ));
    assert!(hf_sim::thin::hollow(&w, p));
    let mut log = Vec::new();
    run_days(&mut w, 2, &mut log);
    // Not grown: a grazer passing may still browse it down (animals.rs `graze_bite`, the only
    // thing that lowers growth), as one did here after #553 moved the run's course (FIXLOG
    // 2026-11-01), so the growth is at most what it was.
    let g = w.plants.iter().find(|q| q.pos == p).map(|q| q.growth);
    assert!(g.is_some_and(|g| g <= 0.2), "it has not grown: {g:?}");
}

#[test]
fn the_thin_place_is_kept_in_the_save() {
    let mut w = World::opening_world(3);
    let mut log = Vec::new();
    run_days(&mut w, 14, &mut log);
    let back = hf_sim::save::snapshot::Snapshot::capture(&w).restore();
    assert_eq!(back.thin.woke, w.thin.woke);
    assert_eq!(back.thin.strength, w.thin.strength);
    assert_eq!(back.thin.seen, w.thin.seen);
}

/// A thing from home (staged) for the survivor `owner`, made by `maker`, at `place`.
fn from_home(
    w: &mut World,
    owner: hf_sim::ids::AgentId,
    maker: &str,
    place: hf_sim::items::ItemPlace,
) -> hf_sim::ids::ItemId {
    let id = w.create_lone_item(hf_sim::items::ItemKind::Hammer, 1, place, 0.6, "thin place");
    w.items.iter_mut().find(|it| it.id == id).unwrap().story =
        Some(Box::new(hf_sim::legend::Story {
            maker_name: format!("{maker}, the smith of home"),
            wear: 0.7,
            origin: "from home, come through the thin place".into(),
            ..Default::default()
        }));
    w.thin.from_home.push((id, owner, maker.to_string()));
    id
}

fn recognised(w: &World, item: hf_sim::ids::ItemId) -> bool {
    w.events
        .pending
        .iter()
        .any(|e| matches!(e.kind, EventKind::Recognised { item: x, .. } if x == item))
}

/// The hourly look for things from home, at ten in the morning on day `day`.
fn look_hourly(w: &mut World, day: u64) {
    w.meta.tick = w.meta.epoch + day * TICKS_PER_DAY + 10 * hf_sim::time::TICKS_PER_HOUR;
    hf_sim::thin::step(w);
}

#[test]
fn a_thing_from_home_is_known_in_someone_s_hands_too() {
    use hf_sim::items::ItemPlace;
    use hf_sim::mind::Activity;
    let mut w = World::opening_world(3);
    let people: Vec<usize> = (0..w.agents.len())
        .filter(|&i| w.agents[i].alive && w.agents[i].community.is_some())
        .collect();
    let (oi, hi) = (people[0], people[1]);
    let (owner, holder) = (w.agents[oi].id, w.agents[hi].id);
    w.agents[oi].activity = Activity::Idle;
    // Picked up by someone else, who carries it past the one it came for.
    let carried = from_home(&mut w, owner, "Hadric", ItemPlace::Carried(holder));
    w.agents[hi].pos = w.agents[oi].pos;
    look_hourly(&mut w, 1);
    assert!(recognised(&w, carried), "known in another's arms");
    assert!(w.thin.from_home.iter().all(|x| x.0 != carried));
    let s = w.items.iter().find(|it| it.id == carried).unwrap();
    assert!(s.story.as_ref().unwrap().deeds[0]
        .1
        .contains("known again by"));
    // Held by someone far off: not seen, so not known (yet).
    let far = from_home(&mut w, owner, "Hadric", ItemPlace::Held(holder));
    let p = w.agents[oi].pos;
    w.agents[hi].pos = hf_sim::ids::TilePos::new(p.x + 20, p.y, p.z);
    look_hourly(&mut w, 2);
    assert!(!recognised(&w, far), "nobody near has it");
    assert!(w.thin.from_home.iter().any(|x| x.0 == far));
    // In their own hands, it is known wherever they are.
    let own = from_home(&mut w, owner, "Hadric", ItemPlace::Held(owner));
    look_hourly(&mut w, 3);
    assert!(recognised(&w, own), "known in their own hands");
}
