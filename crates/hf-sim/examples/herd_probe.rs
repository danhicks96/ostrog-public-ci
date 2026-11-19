//! #1016: does a kept herd need a pen? Runs a seed to a given day and reports, per day, the
//! kept herd's size, how far the farthest kept animal strays from its community's home, and
//! how many kept animals have been lost (died, by cause) since taming. usage:
//! herd_probe <seed> <days>
use hf_sim::agents::Species;
use hf_sim::Session;
use std::collections::BTreeMap;

fn main() -> anyhow::Result<()> {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let seed: u64 = a[0].parse()?;
    let days: u64 = a[1].parse()?;
    let setup = hf_sim::setup::WorldSetup::new(seed);
    let world = hf_sim::world_state::World::from_setup(&setup);
    let mut s = Session::new(world, hf_sim::chronicle::Chronicle::working()?);

    // ids ever seen kept, so a later death can be attributed to a once-kept animal.
    let mut ever_kept: BTreeMap<hf_sim::ids::AgentId, ()> = BTreeMap::new();
    let mut deaths: Vec<(u64, hf_sim::ids::AgentId, String)> = Vec::new();
    let mut max_stray_ever = 0i32;

    for day in 1..=days {
        for _ in 0..hf_sim::time::TICKS_PER_DAY {
            s.step()?;
            for e in s.world.last_events.iter() {
                if let hf_sim::events::EventKind::Died { agent, cause } = &e.kind {
                    if ever_kept.contains_key(agent) {
                        deaths.push((e.tick, *agent, cause.clone()));
                    }
                }
            }
        }
        let w = &s.world;
        let mut by_home: BTreeMap<hf_sim::ids::CommunityId, hf_sim::ids::TilePos> = BTreeMap::new();
        for c in w.communities.iter().filter(|c| !c.dissolved) {
            by_home.insert(c.id, c.home);
        }
        let mut max_stray = 0i32;
        let mut n_kept = 0u32;
        for ag in w.agents.iter() {
            if ag.species == Species::Goat {
                if let Some(cid) = ag.kept {
                    ever_kept.insert(ag.id, ());
                    if ag.alive {
                        n_kept += 1;
                        if let Some(home) = by_home.get(&cid) {
                            let d = ag.pos.dist(*home);
                            if d > max_stray {
                                max_stray = d;
                            }
                        }
                    }
                }
            }
        }
        if max_stray > max_stray_ever {
            max_stray_ever = max_stray;
        }
        if day % 10 == 0 || day == days {
            println!(
                "day {day}: kept goats alive={n_kept}, max stray from home={max_stray} (ever max={max_stray_ever}), kept deaths so far={}",
                deaths.len()
            );
        }
    }

    println!("--- deaths of once-kept goats, with cause ---");
    for (tick, agent, cause) in &deaths {
        println!("tick {tick}: {agent:?} died: {cause}");
    }
    println!(
        "summary: seed {seed}, {days} days, ever-kept goat ids={}, deaths among them={}, max stray ever={max_stray_ever}",
        ever_kept.len(),
        deaths.len()
    );
    Ok(())
}
