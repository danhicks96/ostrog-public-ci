//! The census behind #415 ("what would a normal person do here?", docs/design-rules.md rule 7):
//! person-hours spent wandering, loitering, talking or idle while (a) the person is hungry or
//! their community holds under two days' food, (b) night is near and they know no bed and are
//! not under a roof, (c) they know of no water. Sampled on the hour, every living person.
//! Cases: the opening soaks (seeds 1 to 4, 60 days), a community that has just moved camp (seed
//! 1, ten days after the move), and a party of eight set down with no food (ten days). Also
//! counts breakdowns, deaths (by cause) and births. Loitering or idle at dusk with no bed is
//! counted as settling for the night, not leisure, when in the night hours or near camp.
//!
//! Run: cargo run --release -p hf-sim --example normal_person [days] [opening|opening=N|moved|hungry]
use hf_sim::events::EventKind as E;
use hf_sim::ids::{CommunityId, EventId, TilePos};
use hf_sim::items::ItemPlace;
use hf_sim::mind::knowledge::Known;
use hf_sim::mind::Activity;
use hf_sim::structures::StructureKind;
use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use hf_sim::World;
use std::collections::BTreeMap;

#[derive(Default)]
struct Tally {
    /// Person-hours under each condition, and of those, at leisure: [wander, loiter, talk, idle].
    cond: [u64; 6],
    idle: [[u64; 4]; 6],
    all_hours: u64,
    all_leisure: [u64; 4],
    broke: u32,
    died: BTreeMap<String, u32>,
    born: u32,
}

/// Days of eating the community holds: food lying within 45 tiles of home and nearer it than
/// any other community's home, plus what its members carry, against 18 eating hours a head a
/// day (settlement::EATING_HOURS_A_DAY).
fn food_days(w: &World, cid: CommunityId) -> f32 {
    let Some(c) = w.communities.iter().find(|c| c.id == cid) else {
        return 0.0;
    };
    let others: Vec<TilePos> = w
        .communities
        .iter()
        .filter(|o| o.id != cid && !o.dissolved)
        .map(|o| o.home)
        .collect();
    let people = c
        .members
        .iter()
        .filter(|m| w.agents.iter().any(|a| a.id == **m && a.alive))
        .count()
        .max(1) as f32;
    let hours: f32 = w
        .items
        .iter()
        .filter(|it| it.kind.is_food())
        .filter(|it| match it.place {
            ItemPlace::Ground(p) => {
                let d = p.dist(c.home);
                d <= 45 && others.iter().all(|o| p.dist(*o) >= d)
            }
            ItemPlace::Carried(id) => c.members.contains(&id),
            _ => false,
        })
        .map(|it| it.count as f32 * it.kind.nutrition_hours())
        .sum();
    hours / (people * 18.0)
}

fn sample(w: &World, t: &mut Tally) {
    let hour = (w.meta.tick % TICKS_PER_DAY) / TICKS_PER_HOUR;
    let dusk = !(6..20).contains(&hour);
    let days: BTreeMap<CommunityId, f32> = w
        .communities
        .iter()
        .filter(|c| !c.dissolved)
        .map(|c| (c.id, food_days(w, c.id)))
        .collect();
    for a in w.agents.iter().filter(|a| {
        a.alive && a.species == hf_sim::agents::Species::Person && !a.activity.is_asleep()
    }) {
        let class = match &a.activity {
            Activity::Wandering { .. } => Some(0),
            Activity::Loitering { .. } => Some(1),
            Activity::Talking { .. } => Some(2),
            Activity::Going { then, .. } if matches!(**then, Activity::Talking { .. }) => Some(2),
            Activity::Idle => Some(3),
            _ => None,
        };
        let short = a
            .community
            .and_then(|c| days.get(&c).copied())
            .is_some_and(|d| d < 2.0);
        let hungry = a.needs.hunger < 0.5 || short;
        let bed = a.knowledge.map.iter().any(|(k, b)| {
            matches!(k, Known::Structure(_))
                && (b.tag == StructureKind::Bed as u16
                    || b.tag == StructureKind::Tent as u16
                    || b.tag == StructureKind::Nest as u16)
        });
        let roofless = dusk && !bed && !hf_sim::systems::hazards::indoors(w, a.pos);
        let dry = !a.knowledge.map.keys().any(|k| matches!(k, Known::Water(_)));
        // Loitering or idle at dusk with no bed, where they will sleep, is not leisure: in the
        // night hours (awake before dawn where they slept), or within eight tiles of camp.
        let camp = a
            .community
            .and_then(|c| w.communities.iter().find(|x| x.id == c))
            .map(|c| c.home);
        let settled = roofless
            && matches!(class, Some(1) | Some(3))
            && (!(6..22).contains(&hour) || camp.is_some_and(|h| h.dist(a.pos) <= 8));
        let class = if settled { None } else { class };
        t.all_hours += 1;
        if let Some(c) = class {
            t.all_leisure[c] += 1;
        }
        let starving = a.needs.hunger < 0.5;
        let banded = hungry && a.band.is_some();
        for (k, on) in [hungry, roofless, dry, starving, short, banded]
            .into_iter()
            .enumerate()
        {
            if on {
                t.cond[k] += 1;
                if let Some(c) = class {
                    t.idle[k][c] += 1;
                }
            }
        }
    }
}

fn run(w: &mut World, days: u64, t: &mut Tally) {
    for _ in 0..days * TICKS_PER_DAY {
        w.step();
        let person = |id: hf_sim::ids::AgentId| {
            w.agents
                .iter()
                .any(|a| a.id == id && a.species == hf_sim::agents::Species::Person)
        };
        for e in &w.last_events {
            match &e.kind {
                E::Broke { agent, .. } if person(*agent) => t.broke += 1,
                E::Died { agent, cause } if person(*agent) => {
                    // Whose: a community's, a band of strangers', or nobody's yet.
                    let a = w.agents.iter().find(|a| a.id == *agent).unwrap();
                    let whose = if a.community.is_some() {
                        ""
                    } else if a.band.is_some() {
                        " (band)"
                    } else {
                        " (no community)"
                    };
                    *t.died.entry(format!("{cause}{whose}")).or_default() += 1
                }
                E::Born { agent, .. } if person(*agent) => t.born += 1,
                _ => {}
            }
        }
        if w.meta.tick.is_multiple_of(TICKS_PER_HOUR) {
            sample(w, t);
        }
    }
}

fn report(name: &str, t: &Tally) {
    let names = [
        "hungry or stores < 2 days",
        "dusk, no bed or roof",
        "no water known",
        "of (a): hungry",
        "of (a): stores < 2 days",
        "of (a): in a band",
    ];
    let leisure: u64 = t.all_leisure.iter().sum();
    println!(
        "{name}: awake person-hours {}, at leisure {} ({:.1}%) [wander {}, loiter {}, talk {}, idle {}]",
        t.all_hours,
        leisure,
        100.0 * leisure as f64 / t.all_hours.max(1) as f64,
        t.all_leisure[0],
        t.all_leisure[1],
        t.all_leisure[2],
        t.all_leisure[3]
    );
    for k in 0..6 {
        let l: u64 = t.idle[k].iter().sum();
        println!(
            "   ({}) {:<26} hours {:>6}, at leisure {:>6} ({:>5.1}%) [wander {}, loiter {}, talk {}, idle {}]",
            ["a", "b", "c", "a1", "a2", "a3"][k],
            names[k],
            t.cond[k],
            l,
            100.0 * l as f64 / t.cond[k].max(1) as f64,
            t.idle[k][0],
            t.idle[k][1],
            t.idle[k][2],
            t.idle[k][3]
        );
    }
    println!(
        "   breakdowns {}, births {}, deaths {} {:?}",
        t.broke,
        t.born,
        t.died.values().sum::<u32>(),
        t.died
    );
}

fn main() {
    let days: u64 = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(60);
    let only: Option<String> = std::env::args().nth(2);
    let want = |c: &str| only.as_deref().is_none_or(|o| o.starts_with(c));
    // "opening=3" runs one seed of the opening.
    let seeds: Vec<u64> = match only.as_deref().and_then(|o| o.strip_prefix("opening=")) {
        Some(n) => vec![n.parse().unwrap_or(1)],
        None => (1..5).collect(),
    };
    // The four opening soaks.
    if want("opening") {
        let mut total = Tally::default();
        for &seed in &seeds {
            let mut w = World::opening_world(seed);
            let mut t = Tally::default();
            run(&mut w, days, &mut t);
            report(&format!("opening seed {seed}, {days} days"), &t);
            total.all_hours += t.all_hours;
            total.broke += t.broke;
            total.born += t.born;
            for (k, v) in t.died {
                *total.died.entry(k).or_default() += v;
            }
            for k in 0..4 {
                total.all_leisure[k] += t.all_leisure[k];
            }
            for k in 0..6 {
                total.cond[k] += t.cond[k];
                for c in 0..4 {
                    total.idle[k][c] += t.idle[k][c];
                }
            }
        }
        if seeds.len() > 1 {
            report("opening, four seeds", &total);
        }
    }
    // A community ten days after it moved camp: the opening of seed 1 at day 10, given hard
    // times and good regard so it moves (as tests/moving.rs does).
    if want("moved") {
        let mut w = World::opening_world(1);
        for _ in 0..10 * TICKS_PER_DAY {
            w.step();
        }
        let cid = w.communities.iter().find(|c| !c.dissolved).map(|c| c.id);
        if let Some(cid) = cid {
            let tick = w.meta.tick;
            let ids: Vec<_> = w
                .agents
                .iter()
                .filter(|a| a.community == Some(cid))
                .map(|a| a.id)
                .collect();
            for a in w.agents.iter_mut().filter(|a| a.community == Some(cid)) {
                for o in &ids {
                    if *o != a.id {
                        a.relationships.entry(*o).or_default().opinion = 0.6;
                    }
                }
            }
            let c = w.communities.iter_mut().find(|c| c.id == cid).unwrap();
            for k in 0..5u64 {
                c.hardships.push((
                    tick - (5 - k) * TICKS_PER_DAY / 2,
                    "a hard thing".to_string(),
                    EventId(900_000 + k as u32),
                ));
            }
            let before = c.home;
            hf_sim::systems::moving::consider(&mut w, cid);
            let after = w.communities.iter().find(|c| c.id == cid).unwrap().home;
            println!("moved camp: {before:?} -> {after:?}");
            let mut t = Tally::default();
            run(&mut w, 10, &mut t);
            report("moved camp (seed 1, day 10), 10 days", &t);
        }
    }
    // A party of eight set down with no food at all, a little hungry already.
    if want("hungry") {
        let mut w = World::generate(415, hf_sim::world::gen::GenParams::default());
        w.meta.autoplan = true;
        let ids = w.found_party(8);
        w.items.retain(|it| !it.kind.is_food());
        for a in w.agents.iter_mut().filter(|a| ids.contains(&a.id)) {
            a.needs.hunger = 0.35;
        }
        w.ensure_nav();
        let mut t = Tally::default();
        run(&mut w, 10, &mut t);
        report("hungry party of eight, no food, 10 days", &t);
    }
}
