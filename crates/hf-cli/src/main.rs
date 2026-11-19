//! ostrog: headless CLI for running, digesting, saving and soaking worlds.

use std::path::PathBuf;
use std::time::Instant;

use anyhow::Result;
use clap::{Parser, Subcommand};
use hf_sim::chronicle::Chronicle;
use hf_sim::world::gen::GenParams;
use hf_sim::{digest, save, Session, World};

#[derive(Parser)]
#[command(name = "ostrog", version, about = "Ostrog simulation core")]
struct Cli {
    /// Threads the sim spreads its path and site searches over (#428). Default: the
    /// HF_SIM_THREADS variable, else the machine's cores up to 8. The result is the same at
    /// any count; only the time a tick takes changes.
    #[arg(long, global = true)]
    threads: Option<usize>,
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Generate a world and run it for N ticks, printing a digest and timing.
    Run {
        #[arg(long, default_value_t = 1)]
        seed: u64,
        /// Ticks to run (28,800 ticks is one game-day; see docs/TIME.md).
        #[arg(long, default_value_t = 28_800)]
        ticks: u64,
        /// Game-days to run instead of --ticks (#130: 28,800 ticks read as a month).
        #[arg(long)]
        days: Option<u64>,
        /// The game as it is played: the arrival through the portal into a valley of its
        /// world, with the world's history before it (--agents and --small are then ignored).
        /// Without it, the old twelve-person test valley with no portal and no past.
        #[arg(long)]
        opening: bool,
        #[arg(long)]
        small: bool,
        /// Write a save at the end.
        #[arg(long)]
        save: Option<PathBuf>,
        /// Spawn N wandering inhabitants near the founding site (performance runs).
        #[arg(long, default_value_t = 0)]
        agents: u32,
        /// Which Engine paces this world: steady | relentless | whimsical.
        #[arg(long, default_value_t = hf_sim::engine::Engine::Steady)]
        engine: hf_sim::engine::Engine,
        /// Write every decision (who chose what, with the score of each option) to this file,
        /// headed by the seed, git commit and settings (#565). Capped at 32 MiB, then moved to
        /// `<file>.1`. Env `OSTROG_DEBUG_LOG` does the same when the flag is absent.
        #[arg(long)]
        debug_log: Option<PathBuf>,
    },
    /// Long run with metrics per game-day: tick time p50/p99, stalled and starving
    /// people, plan
    /// failures, leases expired, ledger drift, memories, chronicle rows, RSS. Writes a CSV
    /// and a final threshold report. Saves to a ring every `save_days` days.
    Soak {
        #[arg(long, default_value_t = 1)]
        seed: u64,
        /// Game-days to run.
        #[arg(long, default_value_t = 600)]
        days: u64,
        #[arg(long, default_value_t = 12)]
        agents: u32,
        #[arg(long)]
        csv: Option<PathBuf>,
        /// Save ring base path: three slots are written beside it as PATH.0.hfsave to
        /// PATH.2.hfsave; none = no saves.
        #[arg(long)]
        ring: Option<PathBuf>,
        #[arg(long, default_value_t = 5)]
        save_days: u64,
        /// Soak the game as it is played: the arrival through the portal into a valley of its
        /// world, with the world's history before it (--agents is then ignored).
        #[arg(long)]
        opening: bool,
        /// With --opening: draw this many peoples of the land (the wizard's count; the MP
        /// review soaks worlds of sixteen).
        #[arg(long)]
        peoples: Option<u8>,
        /// Without --opening: lay down a day's meals by the landing at the start and each
        /// midnight, as `profile --provision` does (#369: a 500-person party foraging alone
        /// strips the land and starves in its first winter, so the soak measured a famine).
        #[arg(long)]
        provision: bool,
        /// Which Engine paces the soaked world: steady | relentless | whimsical.
        #[arg(long, default_value_t = hf_sim::engine::Engine::Steady)]
        engine: hf_sim::engine::Engine,
    },
    /// Print the sky and season changes over N days for a seed (to find a rainy or snowy hour).
    Weather {
        #[arg(long, default_value_t = 1)]
        seed: u64,
        #[arg(long, default_value_t = 30)]
        days: u64,
    },
    /// Per-system tick timings over a span: mean, max, and which systems own the spikes.
    Profile {
        #[arg(long, default_value_t = 1)]
        seed: u64,
        #[arg(long, default_value_t = 60)]
        days: u64,
        #[arg(long, default_value_t = 12)]
        agents: u32,
        /// Report ticks slower than this many ms.
        #[arg(long, default_value_t = 5.0)]
        spike_ms: f64,
        /// Stock the landing with meals for the whole party and run, so the load stays at
        /// the party's size (a fresh party of hundreds otherwise starves and the load falls).
        #[arg(long)]
        provision: bool,
        /// Make the party of every kind in the library, in turn (the MP review's mixed
        /// profile): their senses, cycles, diets and bodies all at once.
        #[arg(long)]
        mixed: bool,
    },
    /// Run the director over a span and print each suggestion with who is in frame.
    Director {
        #[arg(long, default_value_t = 7)]
        seed: u64,
        #[arg(long, default_value_t = 288_000)]
        start: u64,
        #[arg(long, default_value_t = 1200)]
        every: u64,
        #[arg(long, default_value_t = 30)]
        frames: u32,
        #[arg(long, default_value_t = 12)]
        agents: u32,
    },
    /// Print the digest of a freshly generated world (tick 0).
    Digest {
        #[arg(long, default_value_t = 1)]
        seed: u64,
        #[arg(long)]
        small: bool,
    },
    /// Load a save, run N more ticks, print digest.
    Resume {
        path: PathBuf,
        #[arg(long, default_value_t = 0)]
        ticks: u64,
    },
    /// Verify a save's manifest and hashes.
    Verify { path: PathBuf },
    /// Digest of the staged demo scene (art review only).
    Stage {
        #[arg(long, default_value_t = 1)]
        seed: u64,
    },
    /// Time the parts of a save after N ticks of the default world.
    BenchSave {
        #[arg(long, default_value_t = 1)]
        seed: u64,
        #[arg(long, default_value_t = 160000)]
        ticks: u64,
    },
    /// Write a small save fixture (for the migration test) to the given directory.
    Fixture { path: PathBuf },
    /// Debug: run N ticks then try to plan every open designation for every person.
    DebugPlan {
        #[arg(long, default_value_t = 1)]
        seed: u64,
        #[arg(long, default_value_t = 57600)]
        ticks: u64,
    },
    /// Dump the tiles of one column (debugging worldgen/rendering).
    Tile {
        #[arg(long, default_value_t = 1)]
        seed: u64,
        x: i32,
        y: i32,
    },
    /// Print a page of the Legends browser for a new opening world (M3): world, people:N,
    /// site:N, figure:N or polity:N.
    Legends {
        #[arg(long, default_value_t = 1)]
        seed: u64,
        #[arg(long, default_value = "world:0")]
        page: String,
    },
    /// Generate the world's region map (M3): print its biome mix, rivers, lakes and time, and
    /// optionally write it as a PPM picture (biome colours, shaded by height, rivers in blue).
    Realm {
        #[arg(long, default_value_t = 1)]
        seed: u64,
        #[arg(long, default_value_t = 192)]
        size: usize,
        #[arg(long)]
        ppm: Option<std::path::PathBuf>,
        /// Run the peoples and their settlements this many years (0 = the map alone).
        #[arg(long, default_value_t = 0)]
        years: i64,
    },
    /// Summarize a generated world: character, biome mix, ground and plant families (with why
    /// each is present or absent), terrain, water, founding site.
    Describe {
        #[arg(long, default_value_t = 1)]
        seed: u64,
        #[arg(long)]
        small: bool,
        /// Override the world's character, -1 (dry) .. 1 (wet); the others keep the seed's.
        #[arg(long, allow_hyphen_values = true)]
        wetness: Option<f32>,
        /// Override warmth, -1 (cold) .. 1 (mild).
        #[arg(long, allow_hyphen_values = true)]
        warmth: Option<f32>,
        /// Override stoniness, -1 (soft) .. 1 (stony).
        #[arg(long, allow_hyphen_values = true)]
        stoniness: Option<f32>,
    },
    /// Make an opening world from new-game choices and print its past: the old hamlets, each
    /// time the deity leaned on one, and the patterns the valley's people noticed.
    Past {
        #[arg(long, default_value_t = 1)]
        seed: u64,
        /// Who watches: steady (Aun) | relentless (Hask) | whimsical (Mirre).
        #[arg(long, default_value_t = hf_sim::engine::Engine::Steady)]
        engine: hf_sim::engine::Engine,
        /// Years of history before the portal (0, 50, 150 or 300 are offered in the game).
        #[arg(long, default_value_t = hf_sim::setup::HISTORY_DEFAULT)]
        years: u64,
    },
}

fn params(small: bool) -> GenParams {
    if small {
        GenParams::small()
    } else {
        GenParams::default()
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    if let Some(n) = cli.threads {
        hf_sim::par::set_threads(n);
    }
    match cli.cmd {
        Cmd::Run {
            seed,
            ticks,
            days,
            opening,
            small,
            save: save_to,
            agents,
            engine,
            debug_log,
        } => {
            let debug_log = debug_log.or_else(|| {
                std::env::var_os("OSTROG_DEBUG_LOG")
                    .filter(|v| !v.is_empty())
                    .map(PathBuf::from)
            });
            if let Some(path) = debug_log {
                hf_sim::debuglog::configure(
                    path,
                    vec![
                        ("command".into(), "run".into()),
                        ("seed".into(), seed.to_string()),
                        (
                            "ticks".into(),
                            days.map_or(ticks, |d| d * hf_sim::time::TICKS_PER_DAY)
                                .to_string(),
                        ),
                        ("opening".into(), opening.to_string()),
                        ("small".into(), small.to_string()),
                        ("agents".into(), agents.to_string()),
                        ("engine".into(), engine.to_string()),
                    ],
                    std::env::args().collect::<Vec<_>>().join(" "),
                );
            }
            let t0 = Instant::now();
            let ticks = days.map_or(ticks, |d| d * hf_sim::time::TICKS_PER_DAY);
            let mut world = if opening {
                let mut setup = hf_sim::setup::WorldSetup::new(seed);
                setup.engine = engine;
                World::from_setup(&setup)
            } else {
                let mut w = World::generate(seed, params(small));
                w.meta.engine = engine;
                w
            };
            if agents > 0 && !opening {
                world.found_party(agents as usize);
            }
            let gen_ms = t0.elapsed().as_secs_f64() * 1000.0;
            let mut s = Session::new(world, Chronicle::working()?);
            let t1 = Instant::now();
            let mut tally: std::collections::BTreeMap<&'static str, u64> =
                std::collections::BTreeMap::new();
            let mut fails: std::collections::BTreeMap<String, u64> =
                std::collections::BTreeMap::new();
            for _ in 0..ticks {
                s.step()?;
                for e in &s.world.last_events {
                    *tally.entry(e.kind_name()).or_default() += 1;
                    if let hf_sim::events::EventKind::PlanFailed { goal, reason, .. } = &e.kind {
                        *fails.entry(format!("{goal}/{reason}")).or_default() += 1;
                    }
                    if let hf_sim::events::EventKind::Slept { hours, .. } = &e.kind {
                        *fails
                            .entry(format!("slept {}h", hours.round() as i32))
                            .or_default() += 1;
                    }
                }
            }
            hf_sim::debuglog::flush();
            println!("plan failures {:?}", fails);
            let mut dk: std::collections::BTreeMap<String, (u32, u32)> =
                std::collections::BTreeMap::new();
            for d in &s.world.designations {
                let e = dk.entry(d.label()).or_default();
                e.0 += 1;
                e.1 += d.failures;
            }
            println!("open designations (count, failures) {:?}", dk);
            let mut sk: std::collections::BTreeMap<String, u32> = std::collections::BTreeMap::new();
            for st in &s.world.structures {
                *sk.entry(format!("{:?}", st.kind)).or_default() += 1;
            }
            println!("structures {:?}", sk);
            let home = s.world.meta.founding_site;
            println!(
                "nearest rock face from home: {:?}",
                hf_sim::systems::settlement::nearest_rock_face(&s.world, home, 45)
            );
            // The communities' own sites (#240: `meta.house_site` is written nowhere).
            for c in s.world.communities.iter().filter(|c| !c.dissolved) {
                let at = |p: Option<hf_sim::ids::TilePos>| {
                    p.map(|p| format!("at {},{},{}", p.x, p.y, p.z))
                        .unwrap_or_else(|| "none".into())
                };
                println!(
                    "{}: houses {} field {} quarry {}",
                    c.name,
                    c.house_sites.len(),
                    at(c.field_site),
                    at(c.quarry_site)
                );
            }
            println!(
                "leases {} expired_total {}",
                s.world.reservations.leases.len(),
                s.world.reservations.expired_total
            );
            let live = hf_sim::economy::live_counts(&s.world.items);
            let expected = s.world.ledger.expected();
            let drift: Vec<String> = expected
                .iter()
                .filter(|(k, v)| live.get(*k).copied().unwrap_or(0) != **v)
                .map(|(k, v)| {
                    format!(
                        "{k}: live {} expected {v}",
                        live.get(k).copied().unwrap_or(0)
                    )
                })
                .collect();
            println!("ledger drift {:?}", drift);
            for a in &s.world.agents {
                if a.activity_ticks > 20 * 60 * 5 {
                    let detail = format!("{:?}", a.activity);
                    println!(
                        "  STUCK {} for {} ticks: {}",
                        a.name,
                        a.activity_ticks,
                        &detail[..detail.len().min(300)]
                    );
                }
                println!(
                    "  {:<10} hunger {:.2} rest {:.2} mood {:+.2} stress {:.2} act {:<10} switches {} known {} last_meal {}",
                    a.name, a.needs.hunger, a.needs.rest, a.mood, a.stress, a.activity.name(), a.switches, a.knowledge.map.len(),
                    match a.last_meal {
                        None => "never".to_string(),
                        Some(t) => format!(
                            "{:.1} days ago",
                            s.world.meta.tick.saturating_sub(t) as f64
                                / hf_sim::time::TICKS_PER_DAY as f64
                        ),
                    }
                );
            }
            let run = t1.elapsed();
            println!(
                "engine {} strain {:.0}% rates {:?}",
                s.world.meta.engine,
                hf_sim::engine::tension(&s.world) * 100.0,
                s.world.engine_rates()
            );
            let alive = s.world.agents.iter().filter(|a| a.alive).count();
            println!("alive {alive} events {:?}", tally);
            println!(
                "world {} seed {} ticks {} gen {:.1} ms run {:.1} ms ({:.3} ms/tick)",
                s.world.meta.name,
                seed,
                ticks,
                gen_ms,
                run.as_secs_f64() * 1000.0,
                run.as_secs_f64() * 1000.0 / ticks.max(1) as f64
            );
            println!("chronicle rows {}", s.chronicle.count()?);
            {
                let stock: Vec<hf_sim::ids::TilePos> = s
                    .world
                    .structures
                    .iter()
                    .filter(|st| st.kind == hf_sim::structures::StructureKind::Stockpile)
                    .map(|st| st.pos)
                    .collect();
                let mut by: std::collections::BTreeMap<(String, bool), (u32, u32)> =
                    std::collections::BTreeMap::new();
                for it in &s.world.items {
                    if let hf_sim::items::ItemPlace::Ground(p) = it.place {
                        let e = by
                            .entry((format!("{:?}", it.kind), stock.contains(&p)))
                            .or_default();
                        e.0 += 1;
                        e.1 += it.count;
                    }
                }
                println!("items (kind, on stockpile) -> (stacks, units): {by:?}");
                println!("stockpile tiles {}", stock.len());
                let home = s.world.meta.founding_site;
                let nav = s.world.nav.as_ref();
                let mut unreachable = 0;
                let mut unwalkable = 0;
                let mut shown = 0;
                for it in &s.world.items {
                    if let hf_sim::items::ItemPlace::Ground(p) = it.place {
                        if stock.contains(&p) {
                            continue;
                        }
                        let walk = s.world.grid.walkable(p);
                        let reach = nav.map(|n| n.reach(home, p)).unwrap_or(false);
                        if !walk {
                            unwalkable += 1;
                        }
                        if !reach {
                            unreachable += 1;
                        }
                        if (!walk || !reach) && shown < 5 {
                            shown += 1;
                            let t = s.world.grid.get(p);
                            let plant = s
                                .world
                                .plants
                                .iter()
                                .find(|pl| pl.pos == p)
                                .map(|pl| format!("{:?}", pl.kind));
                            println!(
                                "  loose {:?} x{} at {:?}: walkable {walk} reach {reach} shape {:?} plant {:?}",
                                it.kind, it.count, p, t.shape, plant
                            );
                        }
                    }
                }
                println!(
                    "loose stacks unwalkable {unwalkable} unreachable-from-home {unreachable}"
                );
                for st in stock.iter().take(3) {
                    println!(
                        "  stock tile {:?} walkable {} reach {}",
                        st,
                        s.world.grid.walkable(*st),
                        nav.map(|n| n.reach(home, *st)).unwrap_or(false)
                    );
                }
            }
            for c in &s.world.overworld.caravans {
                println!(
                    "caravan {} {:?} {:?} formed {} departed {} finished {} at {:?} members {} cargo {:?}: {}",
                    c.id,
                    c.purpose,
                    c.state,
                    c.formed,
                    c.departed,
                    c.finished,
                    c.depart_tile,
                    c.members.len(),
                    c.cargo,
                    c.log.join("; ")
                );
            }
            println!("digest {}", digest::digest(&s.world));
            if let Some(p) = save_to {
                save::write(&s.world, &mut s.chronicle, &p)?;
                println!("saved {}", p.display());
            }
        }
        Cmd::Soak {
            seed,
            days,
            agents,
            csv,
            ring,
            save_days,
            opening,
            peoples,
            provision,
            engine,
        } => soak(
            seed, days, agents, csv, ring, save_days, opening, peoples, provision, engine,
        )?,
        Cmd::Weather { seed, days } => {
            let mut last = None;
            for h in 0..days * 24 {
                let tick = h * hf_sim::time::TICKS_PER_HOUR;
                let wx = hf_sim::systems::weather::Weather::at(seed, tick);
                let key = format!("{:?}", wx.sky);
                if last.as_ref() != Some(&key) {
                    let cal = hf_sim::time::Calendar::from_tick(tick);
                    println!(
                        "tick {tick} day {} {:02}:00 {:?} {} {:.0}C",
                        cal.day_of_year + 1,
                        cal.hour,
                        cal.season,
                        key,
                        wx.temperature_c
                    );
                    last = Some(key);
                }
            }
        }
        Cmd::Profile {
            seed,
            days,
            agents,
            spike_ms,
            provision,
            mixed,
        } => {
            use hf_sim::systems::PIPELINE;
            let mut world = World::generate(seed, params(false));
            let party = world.found_party(agents as usize);
            gather_reachable(&mut world, &party);
            // The profile measures this party: it may die, but it does not move camp (C60: a
            // mixed party leaving the valley for the cold on day 12 left 9 people to measure).
            world.meta.settled = true;
            if mixed {
                let kinds = hf_sim::species::library().len();
                for (k, id) in party.iter().enumerate() {
                    hf_sim::species::give(
                        &mut world,
                        *id,
                        hf_sim::species::SpeciesId((k % kinds) as u16),
                        None,
                    );
                }
            }
            // Provisions: stockpiles by the landing, everyone knows of them, as in a real
            // settlement. A meal keeps five days in the open (`ItemKind::keeps_days`), so a
            // twenty-day pile laid down at once rotted from day five and the party starved
            // from day eight (C54); a day's meals are laid down at the start and each midnight, eight a
            // head, which is more than the party was seen to eat (about seven).
            let piles = provision_piles(&mut world, provision);
            let provide = |world: &mut World| provide_meals(world, &piles, agents);
            let n = PIPELINE.len();
            let mut sum = vec![0.0f64; n];
            let mut max = vec![0.0f64; n];
            let mut spike_owner = vec![0u64; n];
            let mut spikes = 0u64;
            let mut spike_phase: std::collections::BTreeMap<String, u64> =
                std::collections::BTreeMap::new();
            let mut t = vec![0.0f64; n];
            let ticks = days * hf_sim::time::TICKS_PER_DAY;
            let mut worst = 0.0f64;
            // Per day (#238): the cost of a look grows with what there is to note, so a
            // five-day run's slope shows here without five runs.
            let perceive_ix = PIPELINE.iter().position(|p| p.0 == "perceive");
            let (mut day_sum, mut day_perceive, mut day_spikes) = (0.0f64, 0.0f64, 0u64);
            if provision && !piles.is_empty() {
                provide(&mut world);
            }
            for k in 0..ticks {
                if provision
                    && !piles.is_empty()
                    && world.tick().is_multiple_of(hf_sim::time::TICKS_PER_DAY)
                {
                    provide(&mut world);
                }
                world.step_timed(&mut t);
                let total: f64 = t.iter().sum();
                worst = worst.max(total);
                day_sum += total;
                day_perceive += perceive_ix.map(|i| t[i]).unwrap_or(0.0);
                if total > spike_ms {
                    day_spikes += 1;
                }
                if (k + 1) % hf_sim::time::TICKS_PER_DAY == 0 {
                    let d = hf_sim::time::TICKS_PER_DAY as f64;
                    println!(
                        "day {}: mean tick {:.3} ms, perceive {:.0} us, spikes over {spike_ms} ms: {day_spikes}",
                        (k + 1) / hf_sim::time::TICKS_PER_DAY,
                        day_sum / d,
                        day_perceive / d * 1000.0
                    );
                    day_sum = 0.0;
                    day_perceive = 0.0;
                    day_spikes = 0;
                }
                for i in 0..n {
                    sum[i] += t[i];
                    if t[i] > max[i] {
                        max[i] = t[i];
                    }
                }
                if total > spike_ms {
                    spikes += 1;
                    let (imax, _) = t
                        .iter()
                        .enumerate()
                        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
                        .unwrap();
                    spike_owner[imax] += 1;
                    let tick = world.tick();
                    let key = format!(
                        "{} tick%hour={} tick%day={}",
                        PIPELINE[imax].0,
                        tick % hf_sim::time::TICKS_PER_HOUR,
                        tick % hf_sim::time::TICKS_PER_DAY
                    );
                    *spike_phase.entry(key).or_default() += 1;
                }
                world.last_events.clear();
            }
            println!(
                "{:<16} {:>10} {:>10} {:>8}",
                "system", "mean us", "max ms", "spikes"
            );
            for i in 0..n {
                println!(
                    "{:<16} {:>10.2} {:>10.3} {:>8}",
                    PIPELINE[i].0,
                    sum[i] / ticks as f64 * 1000.0,
                    max[i],
                    spike_owner[i]
                );
            }
            let humans = world
                .agents
                .iter()
                .filter(|a| a.alive && a.species == hf_sim::agents::Species::Person)
                .count();
            println!(
                "ticks {ticks} spikes over {spike_ms} ms: {spikes}; people at end {humans}; mean tick {:.3} ms; worst tick {worst:.3} ms",
                sum.iter().sum::<f64>() / ticks as f64
            );
            let mut phases: Vec<(String, u64)> = spike_phase.into_iter().collect();
            phases.sort_by_key(|a| std::cmp::Reverse(a.1));
            for (k, v) in phases.iter().take(12) {
                println!("  {v:>5}  {k}");
            }
            // The M1/M5 gates at 500 fed people (D27, the M5 review): 2 ms mean, no tick over 16.
            let mean = sum.iter().sum::<f64>() / ticks as f64;
            let verdict = |ok: bool| if ok { "pass" } else { "FAIL" };
            println!(
                "gates: mean {mean:.3} ms against 2 ms {}; worst {worst:.3} ms against 16 ms {}",
                verdict(mean <= 2.0),
                verdict(worst <= 16.0)
            );
            // The same at any thread count (#428): compare runs with `--threads`.
            println!(
                "threads {}; digest {}",
                hf_sim::par::threads(),
                digest::digest(&world)
            );
        }
        Cmd::Director {
            seed,
            start,
            every,
            frames,
            agents,
        } => {
            let mut world = World::generate(seed, params(false));
            world.found_party(agents as usize);
            let mut s = Session::new(world, Chronicle::working()?);
            let mut dir = hf_sim::view::director::Director::default();
            while s.world.tick() < start {
                s.step()?;
                dir.observe(s.world.tick(), &s.world.last_events);
            }
            let mut nobody = 0u32;
            let mut awake_frames = 0u32;
            for f in 0..frames {
                for _ in 0..every {
                    s.step()?;
                    dir.observe(s.world.tick(), &s.world.last_events);
                }
                let w = &s.world;
                let people = hf_sim::view::director::people(w);
                let fallback = hf_sim::view::director::settlement_centre(w);
                let sug = dir.suggest(w.tick(), fallback, &people);
                let humans_near = w
                    .agents
                    .iter()
                    .filter(|a| {
                        a.alive
                            && a.species == hf_sim::agents::Species::Person
                            && a.pos.z == sug.at.z
                            && (a.pos.x - sug.at.x).abs() <= 24
                            && (a.pos.y - sug.at.y).abs() <= 13
                    })
                    .map(|a| format!("{}@{},{} {}", a.name, a.pos.x, a.pos.y, a.activity.name()))
                    .collect::<Vec<_>>();
                if sug.reason.starts_with("caravan") {
                    for c in &w.overworld.caravans {
                        for m in &c.members {
                            if let Some(a) = w.agents.iter().find(|a| a.id == *m) {
                                println!(
                                    "  member {} at {:?} alive {} {}",
                                    a.name,
                                    a.pos,
                                    a.alive,
                                    a.activity.name()
                                );
                            } else {
                                println!("  member {:?} not on map", m);
                            }
                        }
                        println!(
                            "  caravan {:?} depart {:?} finished {}",
                            c.state, c.depart_tile, c.finished
                        );
                    }
                }
                let awake_in_frame = w
                    .agents
                    .iter()
                    .filter(|a| {
                        a.alive
                            && a.species == hf_sim::agents::Species::Person
                            && !a.activity.is_asleep()
                            && a.pos.z == sug.at.z
                            && (a.pos.x - sug.at.x).abs() <= 24
                            && (a.pos.y - sug.at.y).abs() <= 13
                    })
                    .count();
                if humans_near.is_empty() {
                    nobody += 1;
                }
                if awake_in_frame >= 1 {
                    awake_frames += 1;
                }
                let cal = hf_sim::time::Calendar::from_tick(w.tick());
                println!(
                    "f{f:02} day {} {:02}:{:02} at {:?} reason {:?} dwell {} in frame: {}",
                    cal.day_of_year + 1,
                    cal.hour,
                    cal.minute,
                    sug.at,
                    sug.reason,
                    sug.dwell,
                    if humans_near.is_empty() {
                        "NOBODY".to_string()
                    } else {
                        humans_near.join("; ")
                    }
                );
            }
            println!(
                "frames {frames}: nobody in frame {nobody}, someone awake in frame {awake_frames}"
            );
        }
        Cmd::Digest { seed, small } => {
            let world = World::generate(seed, params(small));
            println!("{}", digest::digest(&world));
        }
        Cmd::Resume { path, ticks } => {
            let (world, chronicle) = save::read(&path)?;
            let mut s = Session::new(world, chronicle);
            s.run(ticks)?;
            println!(
                "tick {} digest {}",
                s.world.tick(),
                digest::digest(&s.world)
            );
        }
        Cmd::Verify { path } => {
            let m = save::verify(&path)?;
            println!(
                "ok format {} game {} seed {} tick {}",
                m.format_version, m.game_version, m.seed, m.tick
            );
        }
        Cmd::Stage { seed } => {
            let w = World::stage_demo(seed);
            println!(
                "staged {} agents {} structures {} items {} digest {}",
                w.meta.name,
                w.agents.len(),
                w.structures.len(),
                w.items.len(),
                digest::digest(&w)
            );
        }
        Cmd::BenchSave { seed, ticks } => {
            let mut world = World::generate(seed, params(false));
            world.found_party(8);
            let mut s = Session::new(world, Chronicle::working()?);
            s.run(ticks)?;
            let t = Instant::now();
            let snap = hf_sim::save::snapshot::Snapshot::capture(&s.world);
            let t_capture = t.elapsed();
            let t = Instant::now();
            let bytes = bincode::serialize(&snap)?;
            let t_ser = t.elapsed();
            let t = Instant::now();
            s.chronicle.checkpoint()?;
            let t_ckpt = t.elapsed();
            let tmp = std::env::temp_dir().join("hf_bench_chron.sqlite");
            let _ = std::fs::remove_file(&tmp);
            let t = Instant::now();
            {
                let mut dst = rusqlite::Connection::open(&tmp)?;
                let backup = rusqlite::backup::Backup::new(&s.chronicle.conn, &mut dst)?;
                backup.run_to_completion(1024, std::time::Duration::from_millis(0), None)?;
            }
            let t_backup = t.elapsed();
            let chron_len = std::fs::metadata(&tmp)?.len();
            println!(
                "chunks {} agents {} rows {} | capture {:.1} ms, serialize {:.1} ms ({} KB), checkpoint {:.1} ms, backup {:.1} ms ({} KB)",
                s.world.grid.chunks.len(), s.world.agents.len(), s.chronicle.count()?,
                t_capture.as_secs_f64() * 1000.0, t_ser.as_secs_f64() * 1000.0, bytes.len() / 1024,
                t_ckpt.as_secs_f64() * 1000.0, t_backup.as_secs_f64() * 1000.0, chron_len / 1024
            );
        }
        Cmd::Fixture { path } => {
            let mut world = World::generate(77, GenParams::small());
            world.found_party(4);
            let mut s = Session::new(world, Chronicle::working()?);
            s.run(20 * 60 * 6)?;
            save::write(&s.world, &mut s.chronicle, &path)?;
            println!(
                "fixture {} tick {} digest {}",
                path.display(),
                s.world.tick(),
                digest::digest(&s.world)
            );
        }
        Cmd::DebugPlan { seed, ticks } => {
            let mut world = World::generate(seed, params(false));
            world.found_party(8);
            world.run(ticks);
            world.ensure_nav();
            {
                use hf_sim::items::{ItemKind, ItemPlace};
                let a0 = world
                    .agents
                    .iter()
                    .find(|a| a.species == hf_sim::agents::Species::Person)
                    .unwrap();
                for it in world.items.iter().filter(|it| {
                    matches!(
                        it.kind,
                        ItemKind::RawMeat
                            | ItemKind::Root
                            | ItemKind::GrainSheaf
                            | ItemKind::Berries
                    )
                }) {
                    if let ItemPlace::Ground(p) = it.place {
                        let known = a0.knowledge.get(hf_sim::mind::Known::Item(it.id)).is_some();
                        let reach = world.nav.as_ref().unwrap().reach(a0.pos, p);
                        let site = world.designations.iter().any(|d| {
                            d.pos == p
                                && !d.done
                                && !matches!(d.kind, hf_sim::work::DesignationKind::Haul)
                        });
                        println!(
                            "raw {:?} x{} at {} known {} reach {} on_site {}",
                            it.kind, it.count, p, known, reach, site
                        );
                    }
                }
            }
            for d in &world.designations {
                if d.done {
                    continue;
                }
                let mut ok = 0;
                let mut none = 0;
                for a in &world.agents {
                    match hf_sim::work::planner::plan_for(&world, a.id, d) {
                        Some(_) => ok += 1,
                        None => none += 1,
                    }
                }
                let t = world.grid.get(d.pos);
                let above = world.grid.get(d.pos.offset(0, 0, 1));
                let items_here: Vec<String> = world
                    .items
                    .iter()
                    .filter(|it| it.place == hf_sim::items::ItemPlace::Ground(d.pos))
                    .map(|it| format!("{:?}x{}", it.kind, it.count))
                    .collect();
                let stock_here = world.structures.iter().any(|s| {
                    s.pos == d.pos && s.kind == hf_sim::structures::StructureKind::Stockpile
                });
                println!(
                    "{:<10} at {} fail {} plans ok {ok} none {none}  tile {:?}/{:?} above {:?} walkable {} reach_home {} items {:?} stockpile_tile {}",
                    d.label(), d.pos, d.failures, t.shape, world.grid.materials.get(t.material).name, above.shape,
                    world.grid.walkable(d.pos),
                    world.nav.as_ref().unwrap().reach(world.meta.founding_site, d.pos),
                    items_here, stock_here
                );
            }
        }
        Cmd::Tile { seed, x, y } => {
            let w = World::generate(seed, params(false));
            let g = &w.grid;
            println!("column ({x},{y}) ground {}", g.ground_z(x, y));
            for z in (g.z_min..=g.z_max).rev() {
                let t = g.get(hf_sim::ids::TilePos::new(x, y, z));
                println!(
                    "  z {z:>3}: {:?} {:?} flags {:#04x} walkable {}",
                    t.shape,
                    g.materials.get(t.material).name,
                    t.flags.0,
                    g.walkable(hf_sim::ids::TilePos::new(x, y, z))
                );
            }
            let plants: Vec<_> = w
                .plants
                .iter()
                .filter(|p| p.pos.x == x && p.pos.y == y)
                .map(|p| format!("{:?}", p.kind))
                .collect();
            println!("plants {:?}", plants);
        }
        Cmd::Past {
            seed,
            engine,
            years,
        } => {
            let mut setup = hf_sim::setup::WorldSetup::new(seed);
            setup.engine = engine;
            setup.history_years = years;
            let t0 = Instant::now();
            let w = World::from_setup(&setup);
            let d = hf_sim::deity::of(engine);
            println!(
                "{} seed {seed}: {} years of past, watched by {}, {} ({:.0} ms)",
                w.meta.name,
                w.history.years,
                d.name,
                d.title,
                t0.elapsed().as_secs_f64() * 1000.0
            );
            for s in &w.history.sites {
                let end = s
                    .ended
                    .as_ref()
                    .map(|(y, why)| format!("ended {}: {why}", hf_sim::deep::ago(*y)))
                    .unwrap_or_else(|| "still stood".into());
                println!(
                    "  {} founded {} years before, peak {} people, {}{}",
                    s.name,
                    s.founded,
                    s.peak,
                    end,
                    s.well
                        .map(|p| format!(", drew water from a well at {},{},{}", p.x, p.y, p.z))
                        .unwrap_or_default()
                );
            }
            for a in &w.history.answers {
                let site = w
                    .history
                    .sites
                    .iter()
                    .find(|s| s.id == a.site)
                    .map(|s| s.name.as_str())
                    .unwrap_or("?");
                // In a person's terms (#211: "x1.35" was the engine's lean factor).
                let how = if a.strength < 1.0 {
                    "held back"
                } else {
                    "leaned on"
                };
                println!(
                    "  {}: {} answered {} at {site} and {how} {}{}",
                    hf_sim::deep::ago(a.year),
                    d.name,
                    a.trigger.label(),
                    a.lever.label(),
                    if a.followed {
                        ", and it came to pass"
                    } else {
                        ""
                    }
                );
            }
            for (site, claim) in hf_sim::lore::sensed(&w) {
                let name = w
                    .history
                    .sites
                    .iter()
                    .find(|s| s.id == site)
                    .map(|s| s.name.clone())
                    .unwrap_or_default();
                println!("  people say: {}", claim.telling(&name));
            }
        }
        Cmd::Legends { seed, page } => {
            let w = World::from_setup(&hf_sim::setup::WorldSetup::new(seed));
            let mut s = Session::new(w, hf_sim::chronicle::Chronicle::in_memory()?);
            s.write_pending()?;
            println!(
                "{}",
                hf_sim::view::legends::page(&s.world, &s.chronicle, &page)
            );
        }
        Cmd::Realm {
            seed,
            size,
            ppm,
            years,
        } => {
            use hf_sim::realm::{self, RBiome};
            let p = realm::RealmParams {
                size,
                ..Default::default()
            };
            let t0 = std::time::Instant::now();
            let m = realm::generate(seed, &p);
            let ms = t0.elapsed().as_secs_f64() * 1000.0;
            let n = m.size * m.size;
            let mut counts: std::collections::BTreeMap<RBiome, usize> = Default::default();
            for b in &m.biome {
                *counts.entry(*b).or_default() += 1;
            }
            println!(
                "realm seed {seed} size {size}: {ms:.0} ms, digest {:016x}",
                m.digest()
            );
            for (b, c) in &counts {
                println!(
                    "  {:<17} {:>6}  {:>5.1}%",
                    b.label(),
                    c,
                    *c as f64 * 100.0 / n as f64
                );
            }
            let mut temps: Vec<f32> = (0..n)
                .filter(|&i| m.land(i))
                .map(|i| m.temperature[i])
                .collect();
            temps.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            if !temps.is_empty() {
                let q = |f: f32| temps[((temps.len() - 1) as f32 * f) as usize];
                println!(
                    "  land temperature C: 10% {:.0}, median {:.0}, 90% {:.0}",
                    q(0.1),
                    q(0.5),
                    q(0.9)
                );
            }
            let rivers = m.river.iter().filter(|r| **r).count();
            println!("  river regions {rivers}");
            let e = realm::embark::default_region(seed, &m);
            let (ex, ey) = m.xy(e);
            let c = realm::embark::character_of(&m, e);
            println!(
                "  default embark ({ex}, {ey}) {} {:.0} C: wetness {:.2} warmth {:.2} stoniness {:.2}",
                m.biome[e].label(),
                m.temperature[e],
                c.wetness,
                c.warmth,
                c.stoniness
            );
            let history = if years > 0 {
                let t1 = std::time::Instant::now();
                let (rl, events) = realm::sites::run(seed, &m, years);
                println!(
                    "  {} years in {:.0} ms: {} sites founded, {} alive, {} events",
                    years,
                    t1.elapsed().as_secs_f64() * 1000.0,
                    rl.sites.len(),
                    rl.sites.iter().filter(|s| s.alive()).count(),
                    events.len()
                );
                for p in &rl.peoples {
                    let mine: Vec<&realm::sites::RSite> =
                        rl.sites.iter().filter(|s| s.people == p.id).collect();
                    // A people that never settled has no line (#223: "0 of 0 sites, -0").
                    if mine.is_empty() {
                        println!("  {} ({}): never settled", p.name, p.living.label());
                        continue;
                    }
                    let alive = mine.iter().filter(|s| s.alive()).count();
                    let pop: f32 = mine.iter().map(|s| s.population).sum::<f32>().max(0.0);
                    let (hx, hy) = m.xy(p.homeland);
                    let names: Vec<&str> = mine.iter().take(4).map(|s| s.name.as_str()).collect();
                    println!(
                        "  {} ({}, homeland {} at {hx},{hy}): {alive} of {} sites alive, {:.0} people; {}",
                        p.name,
                        p.living.label(),
                        m.biome[p.homeland].label(),
                        mine.len(),
                        pop,
                        names.join(", ")
                    );
                }
                let mut kinds: std::collections::BTreeMap<String, usize> = Default::default();
                for e in &events {
                    let k = format!("{:?}", e.kind);
                    *kinds
                        .entry(k.split([' ', '{']).next().unwrap_or("").to_string())
                        .or_default() += 1;
                }
                println!("  events: {kinds:?}");
                let alive = rl.figures.iter().filter(|f| f.alive()).count();
                println!("  figures: {} named, {} alive", rl.figures.len(), alive);
                let mut causes: std::collections::BTreeMap<&str, usize> = Default::default();
                for f in &rl.figures {
                    if let Some((_, c)) = &f.died {
                        // The figure's record keeps the place; the tally has one key (#245: 704
                        // "fell at" keys still).
                        let key = if c.starts_with("fell at ") {
                            "fell in battle"
                        } else {
                            c.as_str()
                        };
                        *causes.entry(key).or_default() += 1;
                    }
                }
                println!("  deaths: {causes:?}");
                for p in rl.polities.iter().take(12) {
                    let ruler = p
                        .ruler
                        .map(|f| rl.figures[f as usize].name.clone())
                        .unwrap_or_else(|| "nobody".into());
                    println!(
                        "  realm {} of the {}: seat {}, founded {}, {} settlements, ruled by {}{}",
                        p.name,
                        rl.peoples[p.people as usize].name,
                        rl.sites[p.seat as usize].name,
                        p.founded,
                        p.members.len(),
                        ruler,
                        p.ended
                            .as_ref()
                            .map(|(y, c)| format!(" (ended {y}: {c})"))
                            .unwrap_or_default()
                    );
                }
                Some(rl)
            } else {
                None
            };
            if let Some(path) = ppm {
                let mut out = format!("P6\n{} {}\n255\n", m.size, m.size).into_bytes();
                for i in 0..n {
                    let base: [u8; 3] = match m.biome[i] {
                        RBiome::Ocean => [34, 75, 112],
                        RBiome::Lake => [65, 128, 159],
                        RBiome::Ice => [233, 227, 214],
                        RBiome::Tundra => [158, 161, 167],
                        RBiome::Taiga => [42, 86, 52],
                        RBiome::TemperateForest => [54, 110, 50],
                        RBiome::Grassland => [126, 169, 79],
                        RBiome::Steppe => [172, 160, 98],
                        RBiome::Desert => [206, 167, 115],
                        RBiome::Savanna => [170, 170, 85],
                        RBiome::Rainforest => [31, 90, 40],
                        RBiome::Swamp => [70, 95, 70],
                        RBiome::Mountain => [123, 129, 144],
                    };
                    let shade = if m.biome[i] == RBiome::Ocean {
                        0.7 + 0.6 * (m.elevation[i] / m.sea_level.max(1e-6))
                    } else {
                        0.85 + 0.35 * m.height(i)
                    };
                    let mut c = base.map(|v| (v as f32 * shade).min(255.0) as u8);
                    if m.river[i] {
                        c = [64, 128, 200];
                    }
                    if let Some(rl) = &history {
                        if let Some(&sid) = rl.claimed.get(&i) {
                            c = rl.peoples[rl.sites[sid as usize].people as usize].colour;
                        } else if rl.sites.iter().any(|s| s.region == i && !s.alive()) {
                            c = [20, 18, 24];
                        }
                    }
                    out.extend_from_slice(&c);
                }
                std::fs::write(&path, out)?;
                println!("  wrote {}", path.display());
            }
        }
        Cmd::Describe {
            seed,
            small,
            wetness,
            warmth,
            stoniness,
        } => {
            let mut p = params(small);
            if wetness.is_some() || warmth.is_some() || stoniness.is_some() {
                let mut c = hf_sim::world::gen::biome::Character::of_seed(seed);
                c.wetness = wetness.unwrap_or(c.wetness);
                c.warmth = warmth.unwrap_or(c.warmth);
                c.stoniness = stoniness.unwrap_or(c.stoniness);
                p.character = Some(c);
            }
            let w = World::generate(seed, p);
            let g = &w.grid;
            let mut water = 0;
            let mut hist = vec![0usize; (g.z_max - g.z_min + 1) as usize];
            for y in 0..g.height {
                for x in 0..g.width {
                    let gz = g.ground_z(x, y);
                    hist[(gz - g.z_min) as usize] += 1;
                    if g.get(hf_sim::ids::TilePos::new(x, y, gz + 1)).is_water() {
                        water += 1;
                    }
                }
            }
            let caves = g
                .chunks
                .iter()
                .filter(|(k, _)| k.2 < 0)
                .map(|(_, c)| {
                    c.tiles
                        .iter()
                        .filter(|t| t.shape == hf_sim::world::Shape::Open)
                        .count()
                })
                .sum::<usize>();
            println!(
                "{} ({}x{} z {}..{}) seed {}",
                w.meta.name, g.width, g.height, g.z_min, g.z_max, seed
            );
            println!(
                "water columns {}  plants {}  cave tiles {}  chunks {}",
                water,
                w.plants.len(),
                caves,
                g.chunks.len()
            );
            for (i, n) in hist.iter().enumerate() {
                if *n > 0 {
                    println!("ground z {:>3}: {}", i as i32 + g.z_min, n);
                }
            }
            println!(
                "founding site {} ({})",
                w.meta.founding_site,
                g.biome_at(w.meta.founding_site.x, w.meta.founding_site.y)
                    .map_or("no biome", |b| b.label())
            );
            for line in hf_sim::world::gen::biome::describe(
                seed,
                &w.meta.params,
                g,
                &w.plants,
                w.meta.flora_draw.as_ref(),
            ) {
                println!("{line}");
            }
            for c in &g.caverns {
                let fungus = w
                    .plants
                    .iter()
                    .filter(|p| {
                        p.kind == hf_sim::plants::PlantKind::FungusTree && c.contains(p.pos)
                    })
                    .count();
                println!(
                    "cavern {} at {},{},{} ({}x{} tiles, {fungus} fungus trees{})",
                    c.name,
                    c.centre.x,
                    c.centre.y,
                    c.centre.z,
                    c.rx * 2 + 1,
                    c.ry * 2 + 1,
                    if c.pool { ", a pool" } else { "" }
                );
            }
            let mut bad = 0;
            let mut neg = 0;
            for y in 0..g.height {
                for x in 0..g.width {
                    let gz = g.ground_z(x, y);
                    if gz < 0 {
                        neg += 1;
                    }
                    let t = g.get(hf_sim::ids::TilePos::new(x, y, gz));
                    if t.shape != hf_sim::world::Shape::Wall {
                        bad += 1;
                        if bad <= 3 {
                            println!(
                                "  inconsistent column ({x},{y}) ground {gz} top {:?}",
                                t.shape
                            );
                        }
                    }
                }
            }
            println!("columns with negative ground {neg}, with non-solid ground tile {bad}");
            // Renderer check: at the founding view z, columns with nothing to draw within 3 peels.
            for vz in (w.meta.founding_site.z - 1)..=(w.meta.founding_site.z + 2) {
                let mut black = Vec::new();
                for y in 0..g.height {
                    for x in 0..g.width {
                        let t = g.get(hf_sim::ids::TilePos::new(x, y, vz));
                        if t.shape == hf_sim::world::Shape::Wall
                            || t.is_water()
                            || t.shape != hf_sim::world::Shape::Open
                        {
                            continue;
                        }
                        let mut found = false;
                        for peel in 1..4 {
                            let b = g.get(hf_sim::ids::TilePos::new(x, y, vz - peel));
                            if b.shape != hf_sim::world::Shape::Open || b.is_water() {
                                found = true;
                                break;
                            }
                        }
                        if !found {
                            black.push((x, y, g.ground_z(x, y)));
                        }
                    }
                }
                println!(
                    "black columns at view z {vz}: {} e.g. {:?}",
                    black.len(),
                    black.iter().take(5).collect::<Vec<_>>()
                );
            }
            let fs = w.meta.founding_site;
            let nav = hf_sim::nav::Nav::build(g);
            let food_near = w
                .plants
                .iter()
                .filter(|p| {
                    p.kind.yields_food()
                        && (p.pos.x - fs.x).abs() <= 25
                        && (p.pos.y - fs.y).abs() <= 25
                })
                .count();
            let food_reach = w
                .plants
                .iter()
                .filter(|p| {
                    p.kind.yields_food()
                        && (p.pos.x - fs.x).abs() <= 25
                        && (p.pos.y - fs.y).abs() <= 25
                        && nav.reach(fs, p.pos)
                })
                .count();
            let reach_all: usize = (0..g.height)
                .flat_map(|y| (0..g.width).map(move |x| (x, y)))
                .filter(|(x, y)| nav.reach(fs, g.surface(*x, *y)))
                .count();
            println!("food plants within 25: {food_near}, reachable {food_reach}; reachable surface columns {reach_all} of {}", g.width * g.height);
        }
    }
    Ok(())
}

/// Resident memory in KiB, or None where it cannot be read, so a platform without a reader
/// reports "unavailable" instead of a measured 0 MB and a flat trend (issue 87).
#[cfg(not(windows))]
fn rss_kb() -> Option<u64> {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|t| {
            t.lines()
                .find(|l| l.starts_with("VmRSS:"))
                .and_then(|l| l.split_whitespace().nth(1))
                .and_then(|v| v.parse().ok())
        })
}

/// Windows: the working set from K32GetProcessMemoryInfo (kernel32, Windows 7 and later), the
/// counterpart of VmRSS. Declared here rather than through a bindings crate.
#[cfg(windows)]
fn rss_kb() -> Option<u64> {
    #[repr(C)]
    #[derive(Default)]
    struct ProcessMemoryCounters {
        cb: u32,
        page_fault_count: u32,
        peak_working_set_size: usize,
        working_set_size: usize,
        quota_peak_paged_pool_usage: usize,
        quota_paged_pool_usage: usize,
        quota_peak_non_paged_pool_usage: usize,
        quota_non_paged_pool_usage: usize,
        pagefile_usage: usize,
        peak_pagefile_usage: usize,
    }
    #[link(name = "kernel32")]
    extern "system" {
        fn GetCurrentProcess() -> *mut core::ffi::c_void;
        fn K32GetProcessMemoryInfo(
            process: *mut core::ffi::c_void,
            counters: *mut ProcessMemoryCounters,
            cb: u32,
        ) -> i32;
    }
    let mut c = ProcessMemoryCounters {
        cb: std::mem::size_of::<ProcessMemoryCounters>() as u32,
        ..Default::default()
    };
    // SAFETY: GetCurrentProcess returns a pseudo-handle that needs no closing, and `c` is a
    // properly sized, writable PROCESS_MEMORY_COUNTERS whose size is passed as cb.
    let ok = unsafe { K32GetProcessMemoryInfo(GetCurrentProcess(), &mut c, c.cb) };
    (ok != 0).then_some(c.working_set_size as u64 / 1024)
}

/// Stalled: awake, not asleep, in one activity for more than five game-hours.
fn stalled(w: &World) -> usize {
    w.agents
        .iter()
        .filter(|a| {
            a.alive
                && a.species == hf_sim::agents::Species::Person
                && !a.activity.is_asleep()
                && a.activity_ticks > (hf_sim::time::TICKS_PER_HOUR * 5) as u32
        })
        .count()
}

// #253: the soak must surface starving days, not only stalls (FIXLOG 2026-10-28
// fix-253-soak-starving). Starving: hunger at or below the starving line, the same
// test the needs system ticks (Needs::starving).
fn starving(w: &World) -> usize {
    w.agents
        .iter()
        .filter(|a| a.alive && a.species == hf_sim::agents::Species::Person && a.needs.starving())
        .count()
}

#[allow(clippy::too_many_arguments)]
/// A party of hundreds spills out of the landing's ground: in the MP review's profile 162 of 500
/// stood about 90 tiles off, across ground with no way to the landing, and starved beside 30,000
/// provisioned meals, so the profile measured a party shrinking to under 90 (C54). Everyone who
/// cannot reach the landing is moved to the nearest free tile that can.
fn gather_reachable(world: &mut World, party: &[hf_sim::ids::AgentId]) {
    world.ensure_nav();
    let fs = world.meta.founding_site;
    let mut taken: std::collections::BTreeSet<hf_sim::ids::TilePos> = world
        .agents
        .iter()
        .filter(|a| party.contains(&a.id))
        .map(|a| a.pos)
        .collect();
    let mut spots = Vec::new();
    for r in 0..60i32 {
        for dy in -r..=r {
            for dx in -r..=r {
                if dx.abs().max(dy.abs()) != r {
                    continue;
                }
                let (x, y) = (fs.x + dx, fs.y + dy);
                if x < 1 || y < 1 || x >= world.grid.width - 1 || y >= world.grid.height - 1 {
                    continue;
                }
                let p = world.grid.surface(x, y);
                let nav = world.nav.as_ref().unwrap();
                if world.grid.walkable(p) && nav.reach(fs, p) && !taken.contains(&p) {
                    spots.push(p);
                }
            }
        }
    }
    let mut next = spots.into_iter();
    for i in 0..world.agents.len() {
        if !party.contains(&world.agents[i].id) {
            continue;
        }
        let here = world.agents[i].pos;
        if world.nav.as_ref().unwrap().reach(fs, here) {
            continue;
        }
        if let Some(p) = next.next() {
            taken.insert(p);
            world.agents[i].pos = p;
            world.agents[i].from = p;
        }
    }
}

/// Event kinds the soak counts for its report: the peoples milestone's, in the valley.
const TALLIED: [&str; 46] = [
    "proposed",
    "laid_to_rest",
    "tree_felled",
    "hearth_tale",
    "refugees",
    "would_not_stay",
    "joined",
    "breached",
    // #405: the way down, for the delving soak table.
    "mine_sunk",
    "reached_deep",
    "cavern_found",
    "deep_chosen",
    "sealed",
    "barred",
    "from_below",
    "beast_stirred",
    "restless",
    "ghost_seen",
    "at_rest",
    "cursed",
    "bitten",
    "suspected",
    "ate_dead",
    "turned_to_stone",
    // #404: justice and breakdowns, for the M5 gate's soak table.
    "broke",
    "fought",
    "judged",
    "exiled",
    "feud",
    "feud_inherited",
    "feud_ended",
    "betrayed",
    "quarrelled",
    "killing",
    "maiming",
    // #319: the cascade families, for its census in the soak table.
    "drenched",
    "rain_spoiled",
    "vermin_drawn",
    "festered",
    "stores_failed",
    "weakened",
    "sickened_weak",
    "wound_festered",
    "healer_lost",
    "roof_fell",
    // Not an event: a feud taken up by a child of the one it passed from (the M5 gate, "a feud
    // across two generations").
    "feud_to_a_child",
];

/// Stockpiles by the landing for provisioned runs (`profile --provision`, `soak --provision`):
/// the tiles, with a stockpile placed on each when `place` is set.
fn provision_piles(world: &mut World, place: bool) -> Vec<hf_sim::ids::TilePos> {
    let fs = world.meta.founding_site;
    let piles: Vec<hf_sim::ids::TilePos> = (0..36)
        .map(|k| hf_sim::ids::TilePos::new(fs.x + (k % 6) - 3, fs.y + 3 + k / 6, fs.z))
        .filter(|at| world.grid.walkable(*at))
        .collect();
    if place {
        for at in &piles {
            world.place_structure(
                hf_sim::structures::StructureKind::Stockpile,
                *at,
                hf_sim::world::Material::WOOD,
                false,
            );
        }
    }
    piles
}

/// A day's meals on the piles, eight a head, more than a party was seen to eat (about seven).
/// A meal keeps five days in the open, so it is laid down a day at a time (C54).
fn provide_meals(world: &mut World, piles: &[hf_sim::ids::TilePos], agents: u32) {
    if piles.is_empty() {
        return;
    }
    let meals = agents as u64 * 8;
    for k in 0..(meals / 500 + 1) {
        let at = piles[k as usize % piles.len()];
        world.create_item(
            hf_sim::items::ItemKind::Meal,
            500,
            hf_sim::items::ItemPlace::Ground(at),
            0.6,
            "provision",
        );
    }
}

/// The world a soak runs: the opening world (the arrival through the portal) or the old test
/// valley with a founded party, paced by `engine`. Guards against every soak running Steady
/// because `Cmd::Soak` had no engine argument (FIXLOG 2026-11-14 #1505).
fn soak_world(
    seed: u64,
    agents: u32,
    opening: bool,
    peoples: Option<u8>,
    provision: bool,
    engine: hf_sim::engine::Engine,
) -> World {
    if opening {
        let mut setup = hf_sim::setup::WorldSetup::new(seed);
        setup.engine = engine;
        if let Some(n) = peoples {
            setup.peoples.count = [n, n];
        }
        World::from_setup(&setup)
    } else {
        let mut w = World::generate(seed, params(false));
        w.meta.engine = engine;
        let party = w.found_party(agents as usize);
        if provision {
            gather_reachable(&mut w, &party);
        }
        w
    }
}

#[allow(clippy::too_many_arguments)]
fn soak(
    seed: u64,
    days: u64,
    agents: u32,
    csv: Option<PathBuf>,
    ring: Option<PathBuf>,
    save_days: u64,
    opening: bool,
    peoples: Option<u8>,
    provision: bool,
    engine: hf_sim::engine::Engine,
) -> Result<()> {
    use hf_sim::time::TICKS_PER_DAY;
    let world = soak_world(seed, agents, opening, peoples, provision, engine);
    let provision = provision && !opening;
    let mut world = world;
    let piles = provision_piles(&mut world, provision);
    let mut s = Session::new(world, Chronicle::working()?);
    // The world's history before the arrival goes to the record before the first day.
    s.write_pending()?;
    let mut out: Box<dyn std::io::Write> = match &csv {
        Some(p) => Box::new(std::fs::File::create(p)?),
        None => Box::new(std::io::stdout()),
    };
    writeln!(
        out,
        "day,tick,ms_p50,ms_p99,ms_max,spikes_over_5ms,humans,away,animals,stalled,starving,plan_failed,leases_expired,drift,mem_short,mem_long,rows,items,structures,caravans,born,died,rss_kb,save_ms"
    )?;
    let ring_paths = ring.as_ref().map(|r| hf_sim::save::ring_paths(r, 3));
    let mut expired_before = s.world.reservations.expired_total;
    let mut worst_stall_pct = 0.0f64;
    let mut stall_person_hours = 0u64;
    let mut starving_person_hours = 0u64;
    let mut worst_starving_pct = 0.0f64;
    let mut worst_starving_day = 0u64;
    let mut person_hours = 0u64;
    let mut drift_days = 0u64;
    let mut rss_by_hour: Vec<(f64, u64)> = Vec::new();
    let mut rss_missing = 0u64;
    let mut most_humans = 0usize;
    let t_start = Instant::now();
    let mut save_ms_total = 0.0;
    let mut saves = 0u32;
    let mut tally: std::collections::BTreeMap<&'static str, u32> = Default::default();
    for day in 1..=days {
        if provision {
            provide_meals(&mut s.world, &piles, agents);
        }
        let mut samples: Vec<f64> = Vec::with_capacity(TICKS_PER_DAY as usize);
        let mut plan_failed = 0u64;
        let mut born = 0u64;
        let mut died = 0u64;
        for h in 0..24 {
            for _ in 0..hf_sim::time::TICKS_PER_HOUR {
                let t0 = Instant::now();
                s.step()?;
                samples.push(t0.elapsed().as_secs_f64() * 1000.0);
                for e in &s.world.last_events {
                    if TALLIED.contains(&e.kind_name()) {
                        *tally.entry(e.kind_name()).or_insert(0u32) += 1;
                    }
                    if let hf_sim::events::EventKind::FeudInherited { agent, from, .. } = &e.kind {
                        let child = s.world.agents.iter().any(|a| {
                            a.id == *agent && (a.mother == Some(*from) || a.father == Some(*from))
                        });
                        if child {
                            *tally.entry("feud_to_a_child").or_insert(0u32) += 1;
                        }
                    }
                    match e.kind_name() {
                        "plan_failed" => plan_failed += 1,
                        "born" => born += 1,
                        "died" => died += 1,
                        _ => {}
                    }
                }
            }
            let humans_now = s
                .world
                .agents
                .iter()
                .filter(|a| a.alive && a.species == hf_sim::agents::Species::Person)
                .count() as u64;
            person_hours += humans_now;
            stall_person_hours += stalled(&s.world) as u64;
            starving_person_hours += starving(&s.world) as u64;
            let _ = h;
        }
        samples.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let p = |q: f64| samples[((samples.len() - 1) as f64 * q) as usize];
        let humans = s
            .world
            .agents
            .iter()
            .filter(|a| a.alive && a.species == hf_sim::agents::Species::Person)
            .count();
        most_humans = most_humans.max(humans);
        let away = s.world.overworld.away.len();
        let spikes = samples.iter().filter(|m| **m > 5.0).count();
        let animals = s
            .world
            .agents
            .iter()
            .filter(|a| a.alive && a.species != hf_sim::agents::Species::Person)
            .count();
        let live = hf_sim::economy::live_counts(&s.world.items);
        let drift = s
            .world
            .ledger
            .expected()
            .iter()
            .filter(|(k, v)| live.get(*k).copied().unwrap_or(0) != **v)
            .count();
        if drift > 0 {
            drift_days += 1;
        }
        let (ms, ml) = s.world.agents.iter().fold((0usize, 0usize), |acc, a| {
            (
                acc.0 + a.memories.short.len(),
                acc.1 + a.memories.long.len(),
            )
        });
        let expired = s.world.reservations.expired_total - expired_before;
        expired_before = s.world.reservations.expired_total;
        let st = stalled(&s.world);
        let pct = if humans > 0 {
            st as f64 * 100.0 / humans as f64
        } else {
            0.0
        };
        worst_stall_pct = worst_stall_pct.max(pct);
        let stg = starving(&s.world);
        let stg_pct = if humans > 0 {
            stg as f64 * 100.0 / humans as f64
        } else {
            0.0
        };
        if stg_pct > worst_starving_pct {
            worst_starving_pct = stg_pct;
            worst_starving_day = day;
        }
        let mut save_ms = 0.0;
        if let Some(rp) = &ring_paths {
            if day % save_days == 0 {
                let slot = hf_sim::save::next_ring_slot(rp);
                let t0 = Instant::now();
                hf_sim::save::write(&s.world, &mut s.chronicle, &slot)?;
                save_ms = t0.elapsed().as_secs_f64() * 1000.0;
                save_ms_total += save_ms;
                saves += 1;
            }
        }
        // An unreadable sample is left blank in the CSV, never written as 0.
        let rss = match rss_kb() {
            Some(kb) => {
                rss_by_hour.push((t_start.elapsed().as_secs_f64() / 3600.0, kb));
                kb.to_string()
            }
            None => {
                rss_missing += 1;
                String::new()
            }
        };
        writeln!(
            out,
            "{day},{},{:.3},{:.3},{:.3},{spikes},{humans},{away},{animals},{st},{stg},{plan_failed},{expired},{drift},{ms},{ml},{},{},{},{},{born},{died},{rss},{save_ms:.1}",
            s.world.tick(),
            p(0.5),
            p(0.99),
            samples[samples.len() - 1],
            s.chronicle.count()?,
            s.world.items.len(),
            s.world.structures.len(),
            s.world.overworld.caravans.len(),
        )?;
        out.flush()?;
    }
    // Report.
    let elapsed_h = t_start.elapsed().as_secs_f64() / 3600.0;
    let stall_pct_overall = if person_hours > 0 {
        stall_person_hours as f64 * 100.0 / person_hours as f64
    } else {
        0.0
    };
    let starving_pct_overall = if person_hours > 0 {
        starving_person_hours as f64 * 100.0 / person_hours as f64
    } else {
        0.0
    };
    let rss_first = rss_by_hour.first().map(|r| r.1).unwrap_or(0);
    let rss_last = rss_by_hour.last().map(|r| r.1).unwrap_or(0);
    // Growth per real hour after the first quarter of the run.
    let q = rss_by_hour.len() / 4;
    let growth_pct_per_h = if rss_by_hour.len() > q + 1 && elapsed_h > 0.0 {
        let (h0, r0) = rss_by_hour[q];
        let (h1, r1) = rss_by_hour[rss_by_hour.len() - 1];
        if h1 - h0 > 0.25 && r0 > 0 {
            (r1 as f64 / r0 as f64 - 1.0) * 100.0 / (h1 - h0)
        } else {
            0.0
        }
    } else {
        0.0
    };
    let humans = s
        .world
        .agents
        .iter()
        .filter(|a| a.alive && a.species == hf_sim::agents::Species::Person)
        .count();
    let ms_per_tick = t_start.elapsed().as_secs_f64() * 1000.0 / (days * TICKS_PER_DAY) as f64;
    // Any unreadable sample makes the trend meaningless, so there is no memory verdict at all.
    let rss_measured = rss_missing == 0 && !rss_by_hour.is_empty();
    // Memory is judged against the world it holds: a growing settlement grows its state, and
    // that is not a leak. Allowed: 1 MB for each person at the peak, plus 24 MB for the
    // chronicle's page cache and the allocator. The old in-memory chronicle broke this
    // (207 MB for 107 people); a leak would too. The %/h trend is still printed.
    let rss_budget_mb = (rss_first / 1024) as f64 + most_humans as f64 + 24.0;
    let rss_text = if rss_measured {
        format!(
            "RSS {} -> {} MB ({:+.1}%/h after warm-up; budget {:.0} MB for {} people at the peak)",
            rss_first / 1024,
            rss_last / 1024,
            growth_pct_per_h,
            rss_budget_mb,
            most_humans
        )
    } else {
        "RSS unavailable on this platform".to_string()
    };
    eprintln!(
        "SOAK seed {seed} days {days} engine {engine}: {:.2} real h, {:.3} ms/tick mean, humans at end {humans}, stalled person-hours {:.2}% (worst day {:.1}%), starving person-hours {:.2}% (worst day {worst_starving_day}), ledger drift days {drift_days}, {rss_text}, saves {saves} mean {:.0} ms, chronicle rows {}",
        elapsed_h,
        ms_per_tick,
        stall_pct_overall,
        worst_stall_pct,
        starving_pct_overall,
        if saves > 0 { save_ms_total / saves as f64 } else { 0.0 },
        s.chronicle.count()?
    );
    // What the peoples milestone added, as it happened in the valley (MP review).
    eprintln!(
        "SOAK events: {}",
        TALLIED
            .iter()
            .map(|k| format!("{k} {}", tally.get(k).copied().unwrap_or(0)))
            .collect::<Vec<_>>()
            .join(", ")
    );
    let ok = stall_pct_overall < 1.0
        && drift_days == 0
        && (!rss_measured || (rss_last / 1024) as f64 <= rss_budget_mb);
    eprintln!(
        "SOAK {} ({})",
        if ok {
            "PASS"
        } else {
            "FAIL (thresholds: stalled <1%, drift 0 days, RSS within 1 MB a person + 24 MB)"
        },
        if rss_measured {
            "memory judged against the peak population"
        } else {
            "no memory verdict: RSS could not be sampled"
        }
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use hf_sim::engine::Engine;

    /// #1505: `ostrog soak` could only ever run Steady, so no soak table showed what
    /// Relentless or Whimsical do to a colony. The soak's world carries the engine it is given,
    /// with or without `--opening`, and Steady when none is asked for.
    #[test]
    fn soak_setup_takes_the_engine() {
        assert_eq!(
            soak_world(6, 12, true, None, false, Engine::Relentless)
                .meta
                .engine,
            Engine::Relentless
        );
        assert_eq!(
            soak_world(7, 4, false, None, false, Engine::Whimsical)
                .meta
                .engine,
            Engine::Whimsical
        );
        // A bare `soak` takes the clap default, which must be Steady (the engine a world had
        // before the flag existed), so the default soak stays what it was.
        let bare = Cli::try_parse_from(["ostrog", "soak"]).expect("a bare soak parses");
        let Cmd::Soak { engine, .. } = bare.cmd else {
            panic!("`soak` parses to Cmd::Soak");
        };
        assert_eq!(engine, Engine::Steady);
        assert_eq!(
            soak_world(6, 12, true, None, false, engine).meta.engine,
            Engine::Steady
        );
    }
}
