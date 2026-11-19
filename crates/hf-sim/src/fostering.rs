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
        id.is_some_and(|x| w.agents.iter().any(|a| a.id == x && a.alive))
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
        let carer = (0..w.agents.len())
            .filter(|&k| {
                let o = &w.agents[k];
                o.alive && o.community == Some(cid) && !o.is_child(tick) && o.id != child
            })
            .max_by(|&x, &y| {
                let (ox, oy) = (
                    w.agents[x].opinion_of(child)
                        + if w.agents[x].partner.is_some() {
                            0.2
                        } else {
                            0.0
                        },
                    w.agents[y].opinion_of(child)
                        + if w.agents[y].partner.is_some() {
                            0.2
                        } else {
                            0.0
                        },
                );
                ox.total_cmp(&oy).then(w.agents[y].id.cmp(&w.agents[x].id))
            });
        let Some(k) = carer else { continue };
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
            EventKind::Fostered { child, by },
        );
    }
}
