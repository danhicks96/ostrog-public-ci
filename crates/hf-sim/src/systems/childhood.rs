//! Childhood (#296): children learn by watching, play together, and come of age.
//!
//! Each hour a child awake near an adult at work (within five tiles) takes a little of that
//! work's skill, and now and then the craft itself, as one who watched; children together play
//! and warm to each other. The day a child comes of age is on record, with what they turned
//! out best at.

use crate::events::{EventKind, Severity};
use crate::mind::techniques::Learned;
use crate::rng::{purpose, stream};
use crate::time::{TICKS_PER_DAY, TICKS_PER_HOUR, TICKS_PER_YEAR};
use crate::world_state::World;
use rand::Rng;

pub fn step(w: &mut World) {
    let tick = w.meta.tick;
    if tick % TICKS_PER_HOUR != 222 {
        return;
    }
    let n = w.agents.len();
    for i in 0..n {
        let a = &w.agents[i];
        if !a.alive
            || a.species != crate::agents::Species::Person
            || !a.is_child(tick)
            || a.activity.is_asleep()
            || a.age_years(tick) < 4
        {
            continue;
        }
        let me = a.pos;
        let cid = a.community;
        // Watching someone work.
        let watched = (0..n).find_map(|k| {
            let o = &w.agents[k];
            if k == i || !o.alive || o.community != cid || o.is_child(tick) {
                return None;
            }
            if o.pos.z != me.z || o.pos.dist(me) > 5 {
                return None;
            }
            match &o.activity {
                crate::mind::Activity::Working { plan } => w
                    .designations
                    .iter()
                    .find(|d| d.id == plan.designation)
                    .map(|d| (k, d.kind)),
                _ => None,
            }
        });
        if let Some((k, kind)) = watched {
            let skill = crate::systems::work::skill_for(kind);
            w.agents[i].skills.practise(skill, 0.15);
            if let crate::work::DesignationKind::Craft(r) = kind {
                if let Some(t) = crate::work::recipes::craft_of(r) {
                    let knows = w.agents[k].techniques.knows(t);
                    let mut rr = stream(
                        w.meta.seed,
                        &[purpose::AGENT_SOCIAL, 0x2960, w.agents[i].id.0 as u64, tick],
                    );
                    if knows && !w.agents[i].techniques.knows(t) && rr.gen_bool(0.01) {
                        w.agents[i].techniques.learn(t, Learned::Watched, 0.1, tick);
                    }
                }
            }
        }
        // Playing with another child near.
        let mate = (0..n).find(|&k| {
            let o = &w.agents[k];
            k != i
                && o.alive
                && o.is_child(tick)
                && o.community == cid
                && o.pos.z == me.z
                && o.pos.dist(me) <= 4
                && !o.activity.is_asleep()
        });
        if let Some(k) = mate {
            let (ai, bk) = (w.agents[i].id, w.agents[k].id);
            w.agents[i].relate(bk, 0.02, 0.02, tick);
            w.agents[k].relate(ai, 0.02, 0.02, tick);
            w.agents[i].met(bk, tick);
            w.agents[k].met(ai, tick);
        }
    }
    // Coming of age, once a day.
    if tick % TICKS_PER_DAY != 9 * TICKS_PER_HOUR + 222 {
        return;
    }
    for i in 0..n {
        let a = &w.agents[i];
        if !a.alive || a.species != crate::agents::Species::Person {
            continue;
        }
        let adult = a.stage_years().0;
        let (now, yesterday) = (
            a.age_years(tick),
            a.age_years(tick.saturating_sub(TICKS_PER_DAY)),
        );
        if now < adult || yesterday >= adult {
            continue;
        }
        let best = crate::mind::skills::ALL
            .iter()
            .copied()
            .max_by(|x, y| a.skills.level(*x).total_cmp(&a.skills.level(*y)))
            .map(|s| format!("{s:?}").to_lowercase())
            .unwrap_or_default();
        let (who, at) = (a.id, a.pos);
        w.events.push(
            tick,
            Some(at),
            Severity::Notable,
            EventKind::CameOfAge { agent: who, best },
        );
    }
    let _ = TICKS_PER_YEAR;
}
