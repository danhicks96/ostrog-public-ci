//! Finds a day on which someone of the valley is in another place the world map shows (#562),
//! and saves the game there for a review shot. Prints each day's view::away rows when they
//! change. usage: away_probe <seed> <max days> <save path> [days more after the first such day]
use hf_sim::Session;

fn main() -> anyhow::Result<()> {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let seed: u64 = a[0].parse()?;
    let days: u64 = a[1].parse()?;
    let out = std::path::PathBuf::from(&a[2]);
    let mut more: Option<u64> = a.get(3).map(|m| m.parse()).transpose()?;
    let setup = hf_sim::setup::WorldSetup::new(seed);
    let world = hf_sim::world_state::World::from_setup(&setup);
    let mut s = Session::new(world, hf_sim::chronicle::Chronicle::working()?);
    s.write_pending()?;
    let mut last = String::new();
    for day in 1..=days {
        for _ in 0..hf_sim::time::TICKS_PER_DAY {
            s.step()?;
        }
        let rows = hf_sim::view::away::list(&s.world);
        let now = format!("{rows:?}");
        if now != last {
            for r in &rows {
                println!(
                    "day {day}: {} [{}] region {:?}: {}",
                    r.name,
                    r.reaction.word(),
                    r.region,
                    r.line
                );
            }
            last = now;
        }
        if rows.iter().any(|r| r.region.is_some()) && more.is_none_or(|m| m == 0) {
            hf_sim::save::write(&s.world, &mut s.chronicle, &out)?;
            println!("saved {} on day {day}", out.display());
            return Ok(());
        }
        if rows.iter().any(|r| r.region.is_some()) {
            more = more.map(|m| m - 1);
        }
    }
    println!("nobody away at a place on the map in {days} days");
    Ok(())
}
