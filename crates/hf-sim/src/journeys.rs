//! Away and back (#347): someone of a community travels on their own and comes back, still
//! one of them while away. A curious or solitary adult, now and then, sets out alone for a
//! place beyond the valley their community knows of (to see it, to trade a little, to be
//! alone, to look for the seven still missing) for four to twelve days, and returns. While
//! away they are off the map and off the roll of those present (`overworld.away`), their
//! crafts not lost (crafts.rs `knowers_away`); their departure and return are on record.

use crate::events::{EventKind, Severity};
use crate::mind::{Activity, Trait};
use crate::rng::{purpose, stream};
use crate::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use crate::World;
use rand::Rng;

const WHY: [&str; 4] = [
    "to see it for themselves",
    "to trade a little",
    "to be alone a while",
    "to ask after the ones still missing",
];

pub fn step(w: &mut World) {
    let tick = w.meta.tick;
    if tick % TICKS_PER_DAY != 7 * TICKS_PER_HOUR + 23 {
        return;
    }
    let day = tick / TICKS_PER_DAY;
    back(w);
    if w.since_arrival() < 15 * TICKS_PER_DAY {
        return;
    }
    let mut going: Vec<(usize, String, String, u64)> = Vec::new();
    for i in 0..w.agents.len() {
        let a = &w.agents[i];
        let Some(cid) = a.community else { continue };
        if !a.alive
            || a.species != crate::agents::Species::Person
            || a.is_child(tick)
            || a.caravan.is_some()
            || a.body.needs_care()
            || !(a.traits.has(Trait::Curious) || a.traits.has(Trait::Solitary))
        {
            continue;
        }
        // Not the last two of a community: someone has to stay.
        let here = w
            .agents
            .iter()
            .filter(|o| o.alive && o.community == Some(cid))
            .count();
        if here <= 3 {
            continue;
        }
        let mut r = stream(
            w.meta.seed,
            &[purpose::AGENT_SOCIAL, 0x347A, a.id.0 as u64, day],
        );
        if !r.gen_bool(1.0 / 120.0) {
            continue;
        }
        let known: Vec<String> = w
            .overworld
            .sites
            .iter()
            .filter(|s| s.known_by.contains(&cid) && s.people > 0)
            .map(|s| s.name.clone())
            .collect();
        if known.is_empty() {
            continue;
        }
        let to = known[r.gen_range(0..known.len())].clone();
        let why = WHY[r.gen_range(0..WHY.len())].to_string();
        let days = r.gen_range(4..=12);
        going.push((i, to, why, tick + days * TICKS_PER_DAY));
    }
    // Highest index first, so removing one does not move the next.
    going.sort_by_key(|g| std::cmp::Reverse(g.0));
    for (i, to, why, until) in going {
        let id = w.agents[i].id;
        let at = w.agents[i].pos;
        w.reservations.release_all(id);
        for it in w.items.iter_mut() {
            if it.place == crate::items::ItemPlace::Carried(id) {
                it.place = crate::items::ItemPlace::Ground(at);
            }
        }
        let mut a = w.agents.remove(i);
        a.activity = Activity::Idle;
        a.activity_ticks = 0;
        a.path.clear();
        a.journey = Some((until, to.clone(), why.clone()));
        w.overworld.away.push(a);
        w.events.push(
            tick,
            Some(at),
            Severity::Notable,
            EventKind::WentAway { agent: id, to, why },
        );
    }
}

fn back(w: &mut World) {
    let tick = w.meta.tick;
    let due: Vec<crate::ids::AgentId> = w
        .overworld
        .away
        .iter()
        .filter(|a| a.journey.as_ref().is_some_and(|j| j.0 <= tick))
        .map(|a| a.id)
        .collect();
    for id in due {
        let Some(ix) = w.overworld.away.iter().position(|a| a.id == id) else {
            continue;
        };
        let (cid, from) = {
            let a = &w.overworld.away[ix];
            (
                a.community,
                a.journey.as_ref().map(|j| j.1.clone()).unwrap_or_default(),
            )
        };
        let home = cid
            .and_then(|c| w.communities.iter().find(|x| x.id == c && !x.dissolved))
            .map(|c| c.home);
        let edge = w
            .overworld
            .sites
            .iter()
            .find(|s| s.name == from)
            .map(|s| s.edge)
            .zip(home)
            .and_then(|(e, h)| crate::systems::expedition::edge_tile(w, e, h));
        let (Some(home), Some(edge)) = (home, edge) else {
            // Nothing to come back to: they stay where they went.
            continue;
        };
        let mut a = w.overworld.away.remove(ix);
        let days = a
            .journey
            .as_ref()
            .map(|j| ((tick + TICKS_PER_DAY).saturating_sub(j.0)) / TICKS_PER_DAY)
            .unwrap_or(0);
        let _ = days;
        a.journey = None;
        a.pos = edge;
        a.from = edge;
        a.path.clear();
        a.activity = Activity::Idle;
        a.needs.hunger = a.needs.hunger.min(0.5);
        w.agents.push(a);
        let i = w.agents.len() - 1;
        if w.path_agent_to(i, home) {
            w.agents[i].activity = Activity::Wandering { to: home };
        }
        w.events.push(
            tick,
            Some(edge),
            Severity::Notable,
            EventKind::CameBack { agent: id, from },
        );
    }
}
