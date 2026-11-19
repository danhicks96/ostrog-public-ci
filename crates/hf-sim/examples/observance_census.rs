//! Why observances are kept or not (#297 follow-up): openings of seeds 1 to 4, run DAYS game-days
//! (default 120). At each season's observance it prints, for each community, what the gate saw
//! (people, the one followed in the spirit and whether awake, how many awake heed the signs
//! enough to lead, the strongest lean among them) and what came of it; then a tally per seed of
//! kept, lapsed and skipped (nothing kept, nothing to lapse) by reason.
//!
//! Run: cargo run --release -p hf-sim --example observance_census [days]

use hf_sim::events::EventKind;
use hf_sim::mind::beliefs::Topic;
use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_HOUR};

fn main() {
    let days: u64 = std::env::args()
        .nth(1)
        .and_then(|a| a.parse().ok())
        .unwrap_or(120);
    let verbose = std::env::var("HF_VERBOSE").is_ok();
    for seed in 1..5u64 {
        let mut w = hf_sim::World::opening_world(seed);
        let mut tally: std::collections::BTreeMap<String, u32> = Default::default();
        for _ in 0..days * TICKS_PER_DAY {
            let due = [w.since_arrival() + 1].iter().any(|&t| {
                t >= TICKS_PER_DAY
                    && t % TICKS_PER_DAY == 12 * TICKS_PER_HOUR
                    && hf_sim::time::Calendar::from_tick(t).day_of_season == 1
            });
            let mut seen: Vec<String> = Vec::new();
            if due {
                for c in w.communities.iter().filter(|c| !c.dissolved) {
                    let members: Vec<&hf_sim::agents::Agent> = w
                        .agents
                        .iter()
                        .filter(|a| {
                            a.alive
                                && a.species == hf_sim::agents::Species::Person
                                && a.community == Some(c.id)
                        })
                        .collect();
                    let leader = c.leaders[hf_sim::justice::Domain::Spirit as usize];
                    let keeper = hf_sim::faith::reader_of(&w, c.id);
                    let awake: Vec<_> =
                        members.iter().filter(|a| !a.activity.is_asleep()).collect();
                    let heed = awake
                        .iter()
                        .filter(|a| a.beliefs.lean(Topic::DeitySigns) >= 0.2)
                        .count();
                    let held = members
                        .iter()
                        .filter(|a| a.beliefs.get(Topic::DeitySigns).is_some())
                        .count();
                    let best = members
                        .iter()
                        .map(|a| a.beliefs.lean(Topic::DeitySigns))
                        .fold(f32::MIN, f32::max);
                    seen.push(format!(
                        "day {} c{} people {} awake {} spirit-led {:?} reader {:?} hold-a-view {} awake-heed>=0.2 {} best-lean {:.2} kept-last {}",
                        w.since_arrival() / TICKS_PER_DAY,
                        c.id.0,
                        members.len(),
                        awake.len(),
                        leader.map(|x| x.0),
                        keeper.map(|x| x.0),
                        held,
                        heed,
                        best,
                        w.faith.kept.iter().any(|(k, _)| *k == c.id)
                    ));
                }
            }
            w.step();
            if due {
                let mut any = false;
                for e in &w.last_events {
                    match &e.kind {
                        EventKind::Observance { came, .. } => {
                            any = true;
                            *tally.entry("kept".into()).or_default() += 1;
                            if verbose {
                                println!("  kept, {came} came");
                            }
                        }
                        EventKind::ObservanceLapsed { why, .. } => {
                            any = true;
                            *tally.entry(format!("lapsed: {why}")).or_default() += 1;
                        }
                        _ => {}
                    }
                }
                if !any {
                    for s in &seen {
                        let why = if s.contains("awake-heed>=0.2 0") && s.contains("reader None") {
                            "skipped: nobody to lead (no reader, nobody awake heeds 0.2)"
                        } else {
                            "skipped: other (nobody came, not kept last)"
                        };
                        *tally.entry(why.into()).or_default() += 1;
                    }
                }
                if verbose {
                    for s in &seen {
                        println!("seed {seed} {s}");
                    }
                }
            }
        }
        println!("seed {seed}: {tally:?}");
    }
}
