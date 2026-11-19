//! Livestock (#285): wild goats tamed into a community's herd graze near home, do not run
//! from people, and slowly breed; when food runs short one is killed for meat (an ordinary
//! hunt of a kept beast). A herd is tamed up to six by taming; it may grow to twelve.

use crate::agents::Species;
use crate::events::{EventKind, Severity};
use crate::ids::{AgentId, CommunityId, TilePos};
use crate::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use crate::work::DesignationKind;
use crate::World;
use rand::Rng;

fn herd(w: &World, cid: CommunityId) -> Vec<usize> {
    (0..w.agents.len())
        .filter(|&k| w.agents[k].alive && w.agents[k].kept == Some(cid))
        .collect()
}

/// A kept animal's turn: back toward home if it has strayed, else a few steps about.
pub fn graze(w: &mut World, i: usize, cid: CommunityId, r: &mut impl Rng) {
    let Some(home) = w.communities.iter().find(|c| c.id == cid).map(|c| c.home) else {
        w.agents[i].kept = None;
        return;
    };
    let me = w.agents[i].pos;
    let to = if me.dist(home) > 10 {
        home.offset(r.gen_range(-4..=4), r.gen_range(-4..=4), 0)
    } else {
        me.offset(r.gen_range(-3..=3), r.gen_range(-3..=3), 0)
    };
    let to = w.grid.surface(
        to.x.clamp(1, w.grid.width - 2),
        to.y.clamp(1, w.grid.height - 2),
    );
    if w.grid.walkable(to) && w.path_agent_to(i, to) {
        w.agents[i].set_activity(crate::mind::Activity::Wandering { to });
    }
}

/// The settlement's say (settlement.rs 7e): tame the nearest wild goat while the herd is under
/// six; when `short` of food and the herd is more than two, kill one.
pub fn plan(w: &mut World, home: TilePos, short: bool) {
    let Some(cid) = w
        .communities
        .iter()
        .find(|c| !c.dissolved && c.home == home)
        .map(|c| c.id)
    else {
        return;
    };
    let open =
        |w: &World, f: &dyn Fn(&DesignationKind) -> bool| w.designations.iter().any(|d| f(&d.kind));
    let kept = herd(w, cid);
    if kept.len() < 6 && !open(w, &|k| matches!(k, DesignationKind::Tame(_))) {
        if let Some((id, at)) = w
            .agents
            .iter()
            .filter(|a| {
                a.alive && a.species == Species::Goat && a.kept.is_none() && a.pos.dist(home) <= 40
            })
            .min_by_key(|a| (a.pos.dist(home), a.id))
            .map(|a| (a.id, a.pos))
        {
            let mut d = crate::systems::settlement::make(w, DesignationKind::Tame(id), at, 5);
            d.requirements = vec![];
            w.designations.push(d);
        }
    }
    let hunting_kept = w.designations.iter().any(|d| match d.kind {
        DesignationKind::Hunt(id) => w.agents.iter().any(|a| a.id == id && a.kept.is_some()),
        _ => false,
    });
    if short && kept.len() > 2 && !hunting_kept {
        let k = kept[0];
        let (id, at) = (w.agents[k].id, w.agents[k].pos);
        let mut d = crate::systems::settlement::make(w, DesignationKind::Hunt(id), at, 8);
        d.requirements = vec![];
        w.designations.push(d);
    }
}

/// `i` has tamed `beast` into their community's herd.
pub fn tamed(w: &mut World, i: usize, beast: AgentId) {
    let tick = w.meta.tick;
    let Some(cid) = w.agents[i].community else {
        return;
    };
    let Some(b) = w.agents.iter().position(|a| a.id == beast && a.alive) else {
        return;
    };
    w.agents[b].kept = Some(cid);
    w.agents[i]
        .skills
        .practise(crate::mind::Skill::Animals, 1.0);
    let who = w.agents[i].id;
    let at = w.agents[b].pos;
    w.events.push(
        tick,
        Some(at),
        Severity::Notable,
        EventKind::Tamed {
            agent: who,
            animal: beast,
        },
    );
}

/// Once a day: a herd of two or more, under twelve, has a kid one day in twenty.
pub fn step(w: &mut World) {
    let tick = w.meta.tick;
    if tick % TICKS_PER_DAY != 11 * TICKS_PER_HOUR + 321 {
        return;
    }
    let cids: Vec<CommunityId> = w
        .communities
        .iter()
        .filter(|c| !c.dissolved)
        .map(|c| c.id)
        .collect();
    for cid in cids {
        let h = herd(w, cid);
        if h.len() < 2 || h.len() >= 12 {
            continue;
        }
        let mut r = crate::rng::stream(
            w.meta.seed,
            &[
                crate::rng::purpose::ANIMALS,
                0x2850,
                cid.0 as u64,
                tick / TICKS_PER_DAY,
            ],
        );
        if !r.gen_bool(0.05) {
            continue;
        }
        let at = w.agents[h[0]].pos;
        let id = w.spawn_agent(Species::Goat, at);
        if let Some(a) = w.agents.iter_mut().find(|a| a.id == id) {
            a.kept = Some(cid);
        }
    }
}
