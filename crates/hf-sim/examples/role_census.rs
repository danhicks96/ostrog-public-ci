//! #602: who holds which role each day of an opening, and the food and care work done, beside
//! the soak's starving count. Built on any commit with `Agent::role` (before and after #454's
//! duties), so the two can be compared on the same seed.
//! usage: role_census <seed> <days>
//! Prints one CSV line a day: day, humans, starving person-hours that day, the number holding
//! each role, and that day's forages, meals cooked, hauls, harvests and tends.
use hf_sim::events::EventKind;
use hf_sim::society::Role;
use hf_sim::Session;

fn main() -> anyhow::Result<()> {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let seed: u64 = a[0].parse()?;
    let days: u64 = a[1].parse()?;
    let setup = hf_sim::setup::WorldSetup::new(seed);
    let world = hf_sim::world_state::World::from_setup(&setup);
    let mut s = Session::new(world, hf_sim::chronicle::Chronicle::working()?);
    s.write_pending()?;
    println!("day,humans,starving_ph,forager,builder,farmer,cook,miner,hauler,healer,none,foraged,cooked,hauled,harvested,tended");
    for day in 1..=days {
        let (mut starving, mut work) = (0u64, [0u32; 5]);
        for t in 0..hf_sim::time::TICKS_PER_DAY {
            s.step()?;
            for e in &s.world.last_events {
                let k = match e.kind {
                    EventKind::Foraged { .. } => 0,
                    EventKind::Cooked { .. } => 1,
                    EventKind::Hauled { .. } => 2,
                    EventKind::Harvested { .. } => 3,
                    EventKind::Tended { .. } => 4,
                    _ => continue,
                };
                work[k] += 1;
            }
            if t % hf_sim::time::TICKS_PER_HOUR == 0 {
                starving += s
                    .world
                    .agents
                    .iter()
                    .filter(|a| {
                        a.alive
                            && a.species == hf_sim::agents::Species::Person
                            && a.needs.starving()
                    })
                    .count() as u64;
            }
        }
        let people: Vec<_> = s
            .world
            .agents
            .iter()
            .filter(|a| {
                a.alive && a.species == hf_sim::agents::Species::Person && a.community.is_some()
            })
            .collect();
        let n = |r: Option<Role>| people.iter().filter(|a| a.role == r).count();
        println!(
            "{day},{},{starving},{},{},{},{},{},{},{},{},{},{},{},{},{}",
            people.len(),
            n(Some(Role::Forager)),
            n(Some(Role::Builder)),
            n(Some(Role::Farmer)),
            n(Some(Role::Cook)),
            n(Some(Role::Miner)),
            n(Some(Role::Hauler)),
            n(Some(Role::Healer)),
            n(None),
            work[0],
            work[1],
            work[2],
            work[3],
            work[4]
        );
    }
    Ok(())
}
