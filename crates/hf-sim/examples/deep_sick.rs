//! #439: why the sick deep below are not reached. Steps an opening to the first person of a
//! community who is sick below level -2, then says what lies between them and the others:
//! whether the nav regions join them, the tiles of the shaft or stair above them, and how the
//! sick got there. Run: cargo run --release -p hf-sim --example deep_sick -- <seed> <days>
use hf_sim::agents::Species;
use hf_sim::time::TICKS_PER_DAY;
use hf_sim::World;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let seed: u64 = args.get(1).and_then(|a| a.parse().ok()).unwrap_or(4);
    let days: u64 = args.get(2).and_then(|a| a.parse().ok()).unwrap_or(200);
    let mut w = World::from_setup(&hf_sim::setup::WorldSetup::new(seed));
    let mut deep_since: std::collections::BTreeMap<hf_sim::ids::AgentId, u64> = Default::default();
    let mut said: std::collections::BTreeMap<hf_sim::ids::AgentId, Vec<String>> =
        Default::default();
    for _ in 0..days * TICKS_PER_DAY {
        w.step();
        let tick = w.meta.tick;
        for e in &w.last_events {
            for who in e.participants() {
                let v = said.entry(who).or_default();
                v.push(format!(
                    "d{} {}",
                    tick / TICKS_PER_DAY,
                    hf_sim::view::narrate::line(&w, e)
                ));
                if v.len() > 6 {
                    v.remove(0);
                }
            }
        }
        if !tick.is_multiple_of(1200) {
            continue;
        }
        for a in w
            .agents
            .iter()
            .filter(|a| a.alive && a.species == Species::Person)
        {
            if a.pos.z <= -2 && a.community.is_some() {
                deep_since.entry(a.id).or_insert_with(|| {
                    println!(
                        "d{} {} first below at {} doing {} (z of home ground {})",
                        tick / TICKS_PER_DAY,
                        a.name,
                        a.pos,
                        a.activity.describe(),
                        w.meta.founding_site.z
                    );
                    for l in said.get(&a.id).into_iter().flatten() {
                        println!("    {l}");
                    }
                    tick
                });
            }
        }
        let sick = w.agents.iter().position(|a| {
            a.alive
                && a.species == Species::Person
                && a.community.is_some()
                && a.pos.z <= -2
                && a.body.ill() >= 0.6
        });
        let Some(i) = sick else { continue };
        w.ensure_nav();
        let a = &w.agents[i];
        let nav = w.nav.as_ref().unwrap();
        let p = a.pos;
        println!(
            "d{} SICK {} at {} ill {:.2} doing {} walkable {}",
            tick / TICKS_PER_DAY,
            a.name,
            p,
            a.body.ill(),
            a.activity.describe(),
            w.grid.walkable(p)
        );
        let others: Vec<_> = w
            .agents
            .iter()
            .filter(|o| o.alive && o.community == a.community && o.id != a.id)
            .collect();
        let reach = others.iter().filter(|o| nav.reach(o.pos, p)).count();
        let reach_back = others.iter().filter(|o| nav.reach(p, o.pos)).count();
        println!(
            "  of {} others of their community: {} can reach them, they can reach {}",
            others.len(),
            reach,
            reach_back
        );
        for o in others.iter().filter(|o| o.pos.z <= -2) {
            println!(
                "  also below: {} at {} reach {}",
                o.name,
                o.pos,
                nav.reach(o.pos, p)
            );
        }
        // Flood-fill from the sick person's tile over walkable tiles, by nav neighbours, to see
        // the extent of where they can go.
        let mut seen = std::collections::BTreeSet::new();
        let mut stack = vec![p];
        let mut nb = Vec::new();
        let mut top = p.z;
        while let Some(q) = stack.pop() {
            if !seen.insert(q) || seen.len() > 20000 {
                continue;
            }
            top = top.max(q.z);
            hf_sim::nav::step::neighbours(&w.grid, q, &mut nb);
            for s in nb.clone() {
                if !seen.contains(&s.to) {
                    stack.push(s.to);
                }
            }
        }
        println!("  their pocket: {} tiles, highest z {}", seen.len(), top);
        // The edge of the pocket: tiles beside it, a level up or the same, that are not walkable
        // or not joined, and what they are.
        let mut edges: std::collections::BTreeMap<String, u32> = Default::default();
        let mut example: std::collections::BTreeMap<String, hf_sim::ids::TilePos> =
            Default::default();
        for q in &seen {
            for (dx, dy, dz) in [
                (1, 0, 0),
                (-1, 0, 0),
                (0, 1, 0),
                (0, -1, 0),
                (0, 0, 1),
                (0, 0, -1),
            ] {
                let r = q.offset(dx, dy, dz);
                if seen.contains(&r) {
                    continue;
                }
                let t = w.grid.get(r);
                if t.is_solid() {
                    continue;
                }
                let k = format!(
                    "{:?} {:?} flags {:#x} dz {} walkable {}",
                    t.shape,
                    t.material,
                    t.flags.0,
                    dz,
                    w.grid.walkable(r)
                );
                *edges.entry(k.clone()).or_default() += 1;
                example.entry(k).or_insert(r);
            }
        }
        for (k, n) in &edges {
            println!("  edge {n}x {k} e.g. {}", example[k]);
        }
        return;
    }
    println!("nobody sick below");
}
