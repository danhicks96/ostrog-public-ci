//! The opening soak's loop with the heap opened up (#431): every `every` days, resident memory
//! beside glibc's own count of the bytes in use and the bytes free inside the heap, and the
//! number of agents (the dead included). Saves to a three-slot ring every five days, as the
//! soak does. usage: soak_probe <seed> <days> <every> <ring dir> [trim] [nosave]
//! With `trim`, `malloc_trim(0)` runs after each report, to show what the allocator holds;
//! with `nosave`, nothing is saved. glibc only (Linux); elsewhere it says so and stops.
#[cfg(all(target_os = "linux", target_env = "gnu"))]
use hf_sim::Session;

#[cfg(not(all(target_os = "linux", target_env = "gnu")))]
fn main() {
    eprintln!("soak_probe reads glibc's heap counts: Linux only");
}

#[cfg(all(target_os = "linux", target_env = "gnu"))]
#[repr(C)]
#[allow(dead_code)] // glibc's layout; only some fields are read
struct MallInfo2 {
    arena: usize,
    ordblks: usize,
    smblks: usize,
    hblks: usize,
    hblkhd: usize,
    usmblks: usize,
    fsmblks: usize,
    uordblks: usize,
    fordblks: usize,
    keepcost: usize,
}

#[cfg(all(target_os = "linux", target_env = "gnu"))]
extern "C" {
    fn mallinfo2() -> MallInfo2;
    fn malloc_trim(pad: usize) -> i32;
}

#[cfg(all(target_os = "linux", target_env = "gnu"))]
fn rss_kb() -> u64 {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|t| {
            t.lines()
                .find(|l| l.starts_with("VmRSS:"))
                .and_then(|l| l.split_whitespace().nth(1))
                .and_then(|v| v.parse().ok())
        })
        .unwrap_or(0)
}

#[cfg(all(target_os = "linux", target_env = "gnu"))]
fn main() -> anyhow::Result<()> {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let seed: u64 = a[0].parse()?;
    let days: u64 = a[1].parse()?;
    let every: u64 = a[2].parse()?;
    let ring = std::path::PathBuf::from(&a[3]);
    let trim = a.iter().skip(4).any(|t| t == "trim");
    let nosave = a.iter().skip(4).any(|t| t == "nosave");
    std::fs::create_dir_all(&ring)?;
    let setup = hf_sim::setup::WorldSetup::new(seed);
    let world = hf_sim::world_state::World::from_setup(&setup);
    let mut s = Session::new(world, hf_sim::chronicle::Chronicle::working()?);
    s.write_pending()?;
    let ring_paths = hf_sim::save::ring_paths(&ring.join("r"), 3);
    println!("day,agents,dead,humans,rss_kb,heap_used_kb,heap_free_kb,mmapped_kb,rss_trimmed_kb");
    for day in 1..=days {
        for _ in 0..hf_sim::time::TICKS_PER_DAY {
            s.step()?;
        }
        if !nosave && day % 5 == 0 {
            let slot = hf_sim::save::next_ring_slot(&ring_paths);
            hf_sim::save::write(&s.world, &mut s.chronicle, &slot)?;
        }
        if day % every == 0 {
            let w = &s.world;
            let dead = w.agents.iter().filter(|a| !a.alive).count();
            let humans = w
                .agents
                .iter()
                .filter(|a| a.alive && a.species == hf_sim::agents::Species::Person)
                .count();
            let m = unsafe { mallinfo2() };
            let rss = rss_kb();
            let trimmed = if trim {
                unsafe { malloc_trim(0) };
                rss_kb().to_string()
            } else {
                String::new()
            };
            println!(
                "{day},{},{dead},{humans},{rss},{},{},{},{trimmed}",
                w.agents.len(),
                m.uordblks / 1024,
                m.fordblks / 1024,
                m.hblkhd / 1024,
            );
        }
    }
    Ok(())
}
