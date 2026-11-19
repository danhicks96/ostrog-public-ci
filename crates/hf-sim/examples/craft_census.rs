//! #1385: nobody can tell where the valley's know-how came from. A read-only census of each
//! community's craft knowledge (`CRAFTS`, techniques.rs), by who knows it, how they learned it
//! (`Learned`) and, in words, where it came from (`Know::from`, set by `learn_from`).
//!
//! usage: craft_census <days> <seed> [seed...]
//! Every 100 days, for each living community, lists each craft with its knowers (name, how
//! learned, from). At the end of the run: totals by source across all current knowers, plus
//! how many times a craft was lost and regained (crafts.rs' `CraftLost`/`CraftRegained`).
use hf_sim::events::EventKind;
use hf_sim::mind::techniques::{Know, Learned, CRAFTS};
use hf_sim::time::TICKS_PER_DAY;
use hf_sim::Session;

fn learned_tag(l: Learned) -> &'static str {
    match l {
        Learned::Past => "past",
        Learned::Watched => "watched",
        Learned::Tried => "tried",
        Learned::Studied => "studied",
        Learned::Taught => "taught",
    }
}

/// #1385: tally a set of `Know` rows, restricted to `CRAFTS`, by `Learned` source, plus how
/// many named an outside source in `from`. Pulled out of `main` so it can be guarded by a
/// test: before this issue, nothing tallied craft knowledge by source at all.
fn tally_by_source(knows: &[Know]) -> ([u64; 5], u64) {
    let mut by_source = [0u64; 5];
    let mut from_outside = 0u64;
    for k in knows {
        if !CRAFTS.contains(&k.technique) {
            continue;
        }
        by_source[k.learned as usize] += 1;
        if !k.from.is_empty() {
            from_outside += 1;
        }
    }
    (by_source, from_outside)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Guard for #1385: a craft census must count each `Learned` source separately and must
    /// not count a non-craft technique (here `ThinLore`, which is not in `CRAFTS`) at all.
    #[test]
    fn tallies_crafts_by_learned_source_and_ignores_non_crafts() {
        let mk = |t: Technique, l: Learned, from: &str| Know {
            technique: t,
            skill: 0.5,
            learned: l,
            wins: 0,
            losses: 0,
            since: 0,
            from: from.to_string(),
        };
        let knows = vec![
            mk(Technique::Smithing, Learned::Past, ""),
            mk(
                Technique::Weaving,
                Learned::Studied,
                "a record left at Hobhythe",
            ),
            mk(Technique::Potting, Learned::Taught, ""),
            mk(Technique::HerbLore, Learned::Watched, ""),
            // Not in CRAFTS: must not be tallied.
            mk(Technique::ThinLore, Learned::Studied, "the thin place"),
        ];
        let (by_source, from_outside) = tally_by_source(&knows);
        assert_eq!(by_source[Learned::Past as usize], 1);
        assert_eq!(by_source[Learned::Watched as usize], 1);
        assert_eq!(by_source[Learned::Tried as usize], 0);
        assert_eq!(
            by_source[Learned::Studied as usize],
            1,
            "ThinLore is not a CRAFTS entry and must not be tallied"
        );
        assert_eq!(by_source[Learned::Taught as usize], 1);
        assert_eq!(from_outside, 1, "only the Weaving row named a source");
    }
}

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() < 2 {
        anyhow::bail!("usage: craft_census <days> <seed> [seed...]");
    }
    let days: u64 = args[0].parse()?;
    let seeds: Vec<u64> = args[1..]
        .iter()
        .map(|a| a.parse())
        .collect::<Result<_, _>>()?;

    for seed in seeds {
        println!("== seed {seed} ==");
        let setup = hf_sim::setup::WorldSetup::new(seed);
        let world = hf_sim::world_state::World::from_setup(&setup);
        let mut s = Session::new(world, hf_sim::chronicle::Chronicle::working()?);
        s.write_pending()?;

        let mut lost = 0u64;
        let mut regained = 0u64;
        for day in 1..=days {
            for _ in 0..TICKS_PER_DAY {
                s.step()?;
                for e in &s.world.last_events {
                    match e.kind {
                        EventKind::CraftLost { .. } => lost += 1,
                        EventKind::CraftRegained { .. } => regained += 1,
                        _ => {}
                    }
                }
            }
            if day % 100 != 0 && day != days {
                continue;
            }
            println!("-- day {day} --");
            for c in s.world.communities.iter().filter(|c| !c.dissolved) {
                println!("  {} ({:?}):", c.name, c.id);
                for t in CRAFTS {
                    let knowers: Vec<_> = s
                        .world
                        .agents
                        .iter()
                        .filter(|a| a.alive && a.community == Some(c.id) && a.techniques.knows(t))
                        .collect();
                    if knowers.is_empty() {
                        println!("    {}: nobody", t.name());
                        continue;
                    }
                    let who: Vec<String> = knowers
                        .iter()
                        .map(|a| {
                            let k = a
                                .techniques
                                .list
                                .iter()
                                .find(|k| k.technique == t)
                                .expect("knows() implies present");
                            let from = if k.from.is_empty() {
                                String::new()
                            } else {
                                format!(" ({})", k.from)
                            };
                            format!("{} [{}{}]", a.name, learned_tag(k.learned), from)
                        })
                        .collect();
                    println!("    {}: {}", t.name(), who.join(", "));
                }
            }
        }

        // Totals by source across every current knower of a CRAFTS technique, in any living
        // community, plus the lost/regained tally gathered over the whole run.
        let all_knows: Vec<Know> = s
            .world
            .agents
            .iter()
            .filter(|a| a.alive)
            .flat_map(|a| a.techniques.list.clone())
            .collect();
        let (by_source, from_outside) = tally_by_source(&all_knows);
        println!(
            "totals: past {}, watched {}, tried {}, studied {}, taught {}, named a source {}, crafts lost {lost}, regained {regained}",
            by_source[Learned::Past as usize],
            by_source[Learned::Watched as usize],
            by_source[Learned::Tried as usize],
            by_source[Learned::Studied as usize],
            by_source[Learned::Taught as usize],
            from_outside,
        );
    }
    Ok(())
}
