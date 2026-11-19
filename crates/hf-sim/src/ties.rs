//! Divided loyalty (#348): someone who left a community in a split while kin of theirs stayed
//! keeps a tie to it. Now and then, by day, they go across to it; the tie is on record when it
//! is made, and lapses when nobody of their kin is left there.

use crate::events::{EventKind, Severity};
use crate::ids::{AgentId, CommunityId};
use crate::rng::{purpose, stream};
use crate::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use crate::World;
use rand::Rng;

fn kin(w: &World, a: usize, b: AgentId) -> bool {
    let x = &w.agents[a];
    x.partner == Some(b)
        || x.mother == Some(b)
        || x.father == Some(b)
        || x.children.contains(&b)
        || w.agents.iter().any(|o| {
            o.id == b
                && (o.mother.is_some() && (o.mother == x.mother)
                    || o.father.is_some() && o.father == x.father)
        })
}

fn kin_in(w: &World, i: usize, cid: CommunityId) -> bool {
    w.agents
        .iter()
        .any(|o| o.alive && o.community == Some(cid) && o.id != w.agents[i].id && kin(w, i, o.id))
}

pub fn observe(w: &mut World) {
    let tick = w.meta.tick;
    let splits: Vec<(CommunityId, CommunityId)> = w
        .last_events
        .iter()
        .filter_map(|e| match e.kind {
            EventKind::Split { from, to, .. } => Some((from, to)),
            _ => None,
        })
        .collect();
    for (from, to) in splits {
        let leavers: Vec<usize> = (0..w.agents.len())
            .filter(|&i| w.agents[i].alive && w.agents[i].community == Some(to))
            .collect();
        for i in leavers {
            if w.agents[i].tie.is_none() && kin_in(w, i, from) {
                w.agents[i].tie = Some(from);
                let (who, at) = (w.agents[i].id, w.agents[i].pos);
                w.events.push(
                    tick,
                    Some(at),
                    Severity::Notable,
                    EventKind::Torn {
                        agent: who,
                        with: to,
                        and: from,
                    },
                );
            }
        }
    }
}

/// Each day near noon: one with a tie may go across to it (a day in ten), and a tie with no
/// kin left in it lapses.
pub fn step(w: &mut World) {
    let tick = w.meta.tick;
    if tick % TICKS_PER_DAY != 12 * TICKS_PER_HOUR + 7 {
        return;
    }
    for i in 0..w.agents.len() {
        let Some(t) = w.agents[i].tie else { continue };
        if !w.agents[i].alive {
            continue;
        }
        if !kin_in(w, i, t) {
            w.agents[i].tie = None;
            continue;
        }
        let mut r = stream(
            w.meta.seed,
            &[
                purpose::AGENT_SOCIAL,
                0x3480,
                w.agents[i].id.0 as u64,
                tick / TICKS_PER_DAY,
            ],
        );
        if !r.gen_bool(0.1) || w.agents[i].activity.is_asleep() {
            continue;
        }
        if let Some(home) = w
            .communities
            .iter()
            .find(|c| c.id == t && !c.dissolved)
            .map(|c| c.home)
        {
            if w.path_agent_to(i, home) {
                w.agents[i].activity = crate::mind::Activity::Wandering { to: home };
                w.agents[i].activity_ticks = 0;
            }
        }
    }
}
