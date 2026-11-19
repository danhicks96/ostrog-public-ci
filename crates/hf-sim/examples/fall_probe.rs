//! #444: what was under someone who fell more than a level. Steps an opening and, for every fall
//! of two levels or more, prints the column under where they stood the tick before: each level's
//! shape and material, with any fire, roof or structure there.
//! Run: cargo run --release -p hf-sim --example fall_probe -- <seed> <days>
use std::collections::BTreeMap;

use hf_sim::agents::Species;
use hf_sim::ids::{AgentId, TilePos};
use hf_sim::time::TICKS_PER_DAY;
use hf_sim::World;

fn column(w: &World, p: TilePos) -> Vec<String> {
    (p.z - 9..=p.z + 1)
        .rev()
        .map(|z| {
            let q = TilePos::new(p.x, p.y, z);
            let t = w.grid.get(q);
            let fire = w.fires.iter().any(|f| f.pos == q);
            let s: Vec<String> = w
                .structures
                .iter()
                .filter(|s| s.pos == q)
                .map(|s| format!("{:?}", s.kind))
                .collect();
            format!(
                "    z{z}: {:?} {} flags {:#x}{}{}{} {}",
                t.shape,
                w.grid.materials.get(t.material).name,
                t.flags.0,
                if fire { " FIRE" } else { "" },
                if w.roofs.contains_key(&q) {
                    " roof"
                } else {
                    ""
                },
                if w.built_of.contains_key(&q) {
                    " built"
                } else {
                    ""
                },
                s.join(",")
            )
        })
        .collect()
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let seed: u64 = args.get(1).and_then(|a| a.parse().ok()).unwrap_or(4);
    let days: u64 = args.get(2).and_then(|a| a.parse().ok()).unwrap_or(200);
    let mut w = World::from_setup(&hf_sim::setup::WorldSetup::new(seed));
    let mut before: BTreeMap<AgentId, (TilePos, Vec<String>)> = BTreeMap::new();
    for _ in 0..days * TICKS_PER_DAY {
        let watch: Vec<(AgentId, TilePos)> = w
            .agents
            .iter()
            .filter(|a| a.alive && a.species == Species::Person)
            .map(|a| (a.id, a.pos))
            .collect();
        // The column is costly; keep it only for someone standing over open air or on wood.
        for (id, p) in &watch {
            let under = w.grid.get(p.offset(0, 0, -1));
            let risky = !under.is_solid() || w.fires.iter().any(|f| f.pos.dist(*p) <= 2);
            if risky {
                before.insert(*id, (*p, column(&w, *p)));
            } else {
                before.remove(id);
            }
        }
        w.step();
        for (id, p) in watch {
            let Some(a) = w.agents.iter().find(|a| a.id == id) else {
                continue;
            };
            if a.pos.z <= p.z - 2 {
                println!(
                    "d{} {} fell from {} to {} ({} levels), alive {} {:?}",
                    w.meta.tick / TICKS_PER_DAY,
                    a.name,
                    p,
                    a.pos,
                    p.z - a.pos.z,
                    a.alive,
                    a.body
                        .wounds
                        .iter()
                        .map(|wd| (wd.kind, wd.severity))
                        .collect::<Vec<_>>()
                );
                if let Some((at, col)) = before.get(&id) {
                    println!("  the column at {at} the tick before:");
                    for l in col {
                        println!("{l}");
                    }
                }
                println!("  and now:");
                for l in column(&w, p) {
                    println!("{l}");
                }
            }
        }
    }
}
