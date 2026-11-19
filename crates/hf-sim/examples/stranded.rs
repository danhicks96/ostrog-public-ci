//! #447: how people end up somewhere they cannot walk home from. Steps an opening; every 100
//! ticks, for each person of a community, whether a way joins them to home. When it stops, says
//! where they were the step before and now, what they were doing, and what changed round them;
//! when someone dies, says whether they were cut off and since when.
//! Run: cargo run --release -p hf-sim --example stranded -- <seed> <days>
use std::collections::BTreeMap;

use hf_sim::agents::Species;
use hf_sim::ids::{AgentId, TilePos};
use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use hf_sim::World;

fn tile(w: &World, p: TilePos) -> String {
    let t = w.grid.get(p);
    format!(
        "{:?} {} flags {:#x} walk {}",
        t.shape,
        w.grid.materials.get(t.material).name,
        t.flags.0,
        w.grid.walkable(p)
    )
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let seed: u64 = args.get(1).and_then(|a| a.parse().ok()).unwrap_or(3);
    let days: u64 = args.get(2).and_then(|a| a.parse().ok()).unwrap_or(60);
    let mut w = World::from_setup(&hf_sim::setup::WorldSetup::new(seed));
    // Last place each was joined to home, and since when they have not been.
    let mut last_ok: BTreeMap<AgentId, (u64, TilePos, String)> = BTreeMap::new();
    let mut cut: BTreeMap<AgentId, u64> = BTreeMap::new();
    let mut said: BTreeMap<AgentId, Vec<String>> = BTreeMap::new();
    for _ in 0..days * TICKS_PER_DAY {
        w.step();
        let tick = w.meta.tick;
        for e in &w.last_events {
            for who in e.participants() {
                let v = said.entry(who).or_default();
                v.push(format!(
                    "d{} h{:02} {}",
                    tick / TICKS_PER_DAY,
                    (tick % TICKS_PER_DAY) / TICKS_PER_HOUR,
                    hf_sim::view::narrate::line(&w, e)
                ));
                if v.len() > 8 {
                    v.remove(0);
                }
            }
            if let hf_sim::events::EventKind::Died { agent, .. } = &e.kind {
                if let Some(a) = w.agents.iter().find(|a| a.id == *agent) {
                    if a.species == Species::Person {
                        println!(
                            "d{} DIED {} {} at {} cut off since {:?}",
                            tick / TICKS_PER_DAY,
                            a.name,
                            a.death.clone().unwrap_or_default(),
                            a.pos,
                            cut.get(agent).map(|t| t / TICKS_PER_DAY)
                        );
                    }
                }
            }
        }
        if !tick.is_multiple_of(100) {
            continue;
        }
        w.ensure_nav();
        let nav = w.nav.as_ref().unwrap();
        for a in w
            .agents
            .iter()
            .filter(|a| a.alive && a.species == Species::Person)
        {
            let Some(cid) = a.community else { continue };
            let Some(c) = w.communities.iter().find(|c| c.id == cid) else {
                continue;
            };
            let ok = nav.reach(a.pos, c.home);
            if ok {
                cut.remove(&a.id);
                last_ok.insert(a.id, (tick, a.pos, a.activity.describe()));
            } else if !cut.contains_key(&a.id) && w.grid.walkable(a.pos) {
                cut.insert(a.id, tick);
                let (t0, p0, act0) =
                    last_ok
                        .get(&a.id)
                        .cloned()
                        .unwrap_or((0, a.pos, String::new()));
                println!(
                    "d{} h{:02} CUT OFF {} at {} doing {} (home {}); joined at d{} h{:02} at {} doing {}",
                    tick / TICKS_PER_DAY,
                    (tick % TICKS_PER_DAY) / TICKS_PER_HOUR,
                    a.name,
                    a.pos,
                    a.activity.describe(),
                    c.home,
                    t0 / TICKS_PER_DAY,
                    (t0 % TICKS_PER_DAY) / TICKS_PER_HOUR,
                    p0,
                    act0
                );
                println!(
                    "  here {} / under {}",
                    tile(&w, a.pos),
                    tile(&w, a.pos.offset(0, 0, -1))
                );
                println!(
                    "  then {} / under {}",
                    tile(&w, p0),
                    tile(&w, p0.offset(0, 0, -1))
                );
                // The ground round them: each column's top solid level, and what stands on
                // their own level (W built wall, # rock, . walkable, ' ' air, ~ water); @ them, H home.
                for dy in -7..=7 {
                    let mut row = String::new();
                    for dx in -9..=9 {
                        let (x, y) = (a.pos.x + dx, a.pos.y + dy);
                        let q = TilePos::new(x, y, a.pos.z);
                        let c = if (dx, dy) == (0, 0) {
                            '@'
                        } else if w.grid.get(q).is_water() {
                            '~'
                        } else if w.grid.get(q).shape == hf_sim::world::Shape::Wall {
                            if w.built_of.contains_key(&q) {
                                'W'
                            } else {
                                '#'
                            }
                        } else if w.grid.walkable(q) {
                            if nav.reach(a.pos, q) {
                                'o'
                            } else {
                                '.'
                            }
                        } else {
                            ' '
                        };
                        let g = w.grid.ground_z(x, y);
                        row.push(c);
                        row.push(
                            char::from_digit((g - a.pos.z + 5).clamp(0, 9) as u32, 10).unwrap(),
                        );
                    }
                    println!("    {row}");
                }
                for l in said.get(&a.id).into_iter().flatten() {
                    println!("    {l}");
                }
            }
        }
    }
}
