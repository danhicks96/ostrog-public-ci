//! A night in a bed is a night in a bed (FIXLOG 2026-10-23: every night read as one on the
//! ground, because the bed was read back from a short memory that sleep's own consolidation
//! had just cleared).

use hf_sim::events::{EventKind, Severity};
use hf_sim::ids::TilePos;
use hf_sim::mind::ThoughtKind;
use hf_sim::world::gen::GenParams;
use hf_sim::World;

#[test]
fn a_night_in_a_bed_is_not_a_night_on_the_ground() {
    let mut w = World::generate(1701, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.rebuild_caches();
    let ids = w.found_party(2);
    let tick = w.meta.tick;
    for (who, bed) in [(ids[0], true), (ids[1], false)] {
        let id = w.events.push(
            tick,
            None,
            Severity::Minor,
            EventKind::Slept {
                agent: who,
                hours: 8.0,
                in_bed: bed,
            },
        );
        let e = w
            .events
            .pending
            .iter()
            .find(|e| e.id == id)
            .unwrap()
            .clone();
        w.last_events = vec![e];
        hf_sim::systems::strain::observe(&mut w);
    }
    let has = |w: &World, id, k| {
        w.agents
            .iter()
            .find(|a| a.id == id)
            .unwrap()
            .thoughts
            .list
            .iter()
            .any(|t| t.kind == k)
    };
    assert!(has(&w, ids[0], ThoughtKind::SleptInBed));
    assert!(!has(&w, ids[0], ThoughtKind::SleptRough));
    assert!(has(&w, ids[1], ThoughtKind::SleptRough));
}

fn night(w: &mut World, who: hf_sim::ids::AgentId, in_bed: bool) {
    let tick = w.meta.tick;
    let id = w.events.push(
        tick,
        None,
        Severity::Minor,
        EventKind::Slept {
            agent: who,
            hours: 8.0,
            in_bed,
        },
    );
    let e = w
        .events
        .pending
        .iter()
        .find(|e| e.id == id)
        .unwrap()
        .clone();
    w.last_events = vec![e];
    hf_sim::systems::strain::observe(w);
}

fn rough_weight(w: &World, id: hf_sim::ids::AgentId) -> f32 {
    w.agents
        .iter()
        .find(|a| a.id == id)
        .unwrap()
        .thoughts
        .list
        .iter()
        .find(|t| t.kind == ThoughtKind::SleptRough)
        .map(|t| t.weight)
        .unwrap_or(0.0)
}

fn party() -> (World, hf_sim::ids::AgentId) {
    let mut w = World::generate(1702, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.rebuild_caches();
    let ids = w.found_party(1);
    (w, ids[0])
}

/// OST-357: a week on the ground weighs more than one night.
#[test]
fn a_week_on_the_ground_weighs_more_than_one_night() {
    let (mut w, who) = party();
    night(&mut w, who, false);
    let one = rough_weight(&w, who);
    for _ in 0..6 {
        w.meta.tick += hf_sim::time::TICKS_PER_DAY;
        night(&mut w, who, false);
    }
    let seven = rough_weight(&w, who);
    assert!(
        seven < one,
        "seven nights ({seven}) weigh more than one ({one})"
    );
    assert!(
        (seven - one).abs() > 0.2,
        "the seventh night is clearly heavier"
    );
}

/// OST-357: one night in a bed starts the count again.
#[test]
fn a_night_in_a_bed_resets_the_rough_count() {
    let (mut w, who) = party();
    for _ in 0..4 {
        night(&mut w, who, false);
    }
    assert_eq!(
        w.agents.iter().find(|a| a.id == who).unwrap().rough_nights,
        4
    );
    night(&mut w, who, true);
    assert_eq!(
        w.agents.iter().find(|a| a.id == who).unwrap().rough_nights,
        0
    );
    night(&mut w, who, false);
    assert!(
        (rough_weight(&w, who) + 0.2).abs() < 1e-6,
        "back to one night's weight"
    );
}

/// OST-242: a night cut short is felt; a nap soon after a full night is not.
fn short_sleep(seed: u64, awake_hours: u64) -> f32 {
    use hf_sim::agents::Species;
    use hf_sim::mind::Activity;
    use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
    let mut p = GenParams::small();
    p.width = 64;
    p.height = 64;
    let mut w = World::generate(seed, p);
    w.flatten(2, 2, 60, 60, 1);
    w.meta.tick = TICKS_PER_DAY + 10 * TICKS_PER_HOUR;
    w.rebuild_caches();
    let at = w.grid.surface(30, 30);
    let id = w.spawn_agent(Species::Person, at);
    let i = w.agents.iter().position(|a| a.id == id).unwrap();
    let now = w.meta.tick;
    let a = &mut w.agents[i];
    a.needs.rest = 0.05;
    a.needs.hunger = 1.0;
    a.needs.thirst = 1.0;
    a.needs.warmth = 1.0;
    a.needs.social = 1.0;
    a.last_sleep = now - awake_hours * TICKS_PER_HOUR;
    a.activity = Activity::Sleeping {
        bed: None,
        until: now + 3 * TICKS_PER_HOUR,
    };
    a.activity_ticks = 0;
    for _ in 0..4 * TICKS_PER_HOUR {
        w.step();
        if !w.agents[i].activity.is_asleep() {
            break;
        }
    }
    assert!(!w.agents[i].activity.is_asleep(), "the sleep ended");
    let t = w.meta.tick;
    w.agents[i].thoughts.sum_of(ThoughtKind::SleptBadly, t)
}

#[test]
fn a_broken_night_is_felt() {
    let after_a_day = short_sleep(1702, 14);
    assert!(
        after_a_day < -0.05,
        "a short sleep after a day awake left {after_a_day}"
    );
    let nap = short_sleep(1702, 3);
    assert!(
        nap.abs() < 1e-6,
        "a nap soon after a night left {nap}, not nothing"
    );
}
