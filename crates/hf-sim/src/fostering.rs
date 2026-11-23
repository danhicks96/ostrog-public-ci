//! Families beyond mother and father (#341): a child whose parents are both gone is taken in by
//! an adult of their community, the one who thinks best of them (a partner of theirs with them,
//! if any), and keeps near them as they would a parent.

use crate::events::{EventKind, Severity};
use crate::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use crate::World;

pub fn step(w: &mut World) {
    let tick = w.meta.tick;
    if tick % TICKS_PER_DAY != 10 * TICKS_PER_HOUR + 99 {
        return;
    }
    let alive = |w: &World, id: Option<crate::ids::AgentId>| {
        id.is_some_and(|x| {
            w.agents.iter().any(|a| a.id == x && a.alive)
                // OST-1303: on a caravan or a journey is away, not gone.
                || w.overworld.away.iter().any(|a| {
                    a.id == x && a.alive && (a.caravan.is_some() || a.journey.is_some())
                })
        })
    };
    for i in 0..w.agents.len() {
        let a = &w.agents[i];
        if !a.alive
            || a.species != crate::agents::Species::Person
            || !a.is_child(tick)
            || alive(w, a.mother)
            || alive(w, a.father)
            || alive(w, a.foster)
        {
            continue;
        }
        let Some(cid) = a.community else { continue };
        let child = a.id;
        // Kin first (#1213): a grandparent, then a parent's sibling, then an adult sibling.
        let parents = [w.agents[i].mother, w.agents[i].father];
        let (mut grand, mut sibs_of_parents) = (Vec::new(), Vec::new());
        for pid in parents.into_iter().flatten() {
            if let Some(p) = w.agents.iter().find(|x| x.id == pid) {
                grand.extend([p.mother, p.father].into_iter().flatten());
                sibs_of_parents.push((pid, [p.mother, p.father]));
            }
        }
        let (cm, cf) = (w.agents[i].mother, w.agents[i].father);
        let kin_rank = |o: &crate::agents::Agent| -> u8 {
            if grand.contains(&o.id) {
                return 1;
            }
            let shares = |a: Option<crate::ids::AgentId>, b: Option<crate::ids::AgentId>| {
                a.is_some() && a == b
            };
            if sibs_of_parents.iter().any(|(pid, gp)| {
                o.id != *pid && (shares(o.mother, gp[0]) || shares(o.father, gp[1]))
            }) {
                return 2;
            }
            if shares(o.mother, cm) || shares(o.father, cf) {
                return 3;
            }
            0
        };
        let carer = (0..w.agents.len())
            .filter(|&k| {
                let o = &w.agents[k];
                o.alive && o.community == Some(cid) && !o.is_child(tick) && o.id != child
            })
            .max_by(|&x, &y| {
                let rank = |k: usize| match kin_rank(&w.agents[k]) {
                    0 => 0,
                    r => 4 - r,
                };
                rank(x).cmp(&rank(y)).then_with(|| {
                    let sc = |k: usize| {
                        let o = &w.agents[k];
                        o.opinion_of(child) + if o.partner.is_some() { 0.2 } else { 0.0 }
                    };
                    sc(x)
                        .total_cmp(&sc(y))
                        .then(w.agents[y].id.cmp(&w.agents[x].id))
                })
            });
        let Some(k) = carer else { continue };
        let kin = kin_rank(&w.agents[k]);
        let by = w.agents[k].id;
        w.agents[i].foster = Some(by);
        w.agents[i].relate(by, 0.3, 0.2, tick);
        w.agents[k].relate(child, 0.3, 0.2, tick);
        w.agents[i].met(by, tick);
        w.agents[k].met(child, tick);
        let at = w.agents[k].pos;
        w.events.push(
            tick,
            Some(at),
            Severity::Notable,
            EventKind::Fostered { child, by, kin },
        );
    }
}
