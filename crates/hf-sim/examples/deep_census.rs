//! How deep the valley digs and what it meets there (#405, #274): an opening of one seed run DAYS
//! game-days, with the deepest cut below the ground, what was mined by rock and the rows of the
//! way down counted (mines sunk and deepened, the deep reached, caverns found, breaches, walls).
//!
//! Run: cargo run --release -p hf-sim --example deep_census <seed> [days [from to]] [off]
//! (`off`: no mine is sunk, `World::meta.no_delving`, to compare what the mine costs: deaths by
//! cause, people's hours by what they were doing, and where the coal cut went.)
//! (Four seeds side by side: one process each.)

fn main() {
    let args: Vec<u64> = std::env::args()
        .skip(1)
        .filter_map(|a| a.parse().ok())
        .collect();
    let seed = args.first().copied().unwrap_or(1);
    let days = args.get(1).copied().unwrap_or(200);
    // Optional: the notable rows of days `from` to `to`, as lines.
    let (from, to) = (
        args.get(2).copied().unwrap_or(u64::MAX),
        args.get(3).copied().unwrap_or(0),
    );
    let off = std::env::args().any(|a| a == "off");
    let t = std::time::Instant::now();
    let mut w = hf_sim::World::opening_world(seed);
    w.meta.no_delving = off;
    let mut deaths: std::collections::BTreeMap<String, u32> = Default::default();
    let mut hours: std::collections::BTreeMap<String, u32> = Default::default();
    let mut coal_cut: std::collections::BTreeMap<&str, u32> = Default::default();
    let mut coal_days: Vec<String> = Vec::new();
    let start = w.meta.tick;
    let mut rows: std::collections::BTreeMap<String, u32> = Default::default();
    let mut rocks: std::collections::BTreeMap<String, u32> = Default::default();
    let mut firsts: Vec<String> = Vec::new();
    const KINDS: [&str; 22] = [
        "born",
        "died",
        "joined",
        "split",
        "moved_camp",
        "left_the_valley",
        "left",
        "went_away",
        "collapse",
        "propped",
        "breached",
        "cavern_found",
        "sealed",
        "barred",
        "from_below",
        "beast_stirred",
        "rose",
        "struck_water",
        "water_broke_in",
        "cave_in",
        "mine_sunk",
        "deep_chosen",
    ];
    for _ in 0..days * hf_sim::time::TICKS_PER_DAY {
        w.step();
        let day = (w.meta.tick - start) / hf_sim::time::TICKS_PER_DAY;
        if w.meta.tick.is_multiple_of(hf_sim::time::TICKS_PER_HOUR) {
            for a in w.agents.iter().filter(|a| {
                a.alive && a.community.is_some() && a.species == hf_sim::agents::Species::Person
            }) {
                let what = match &a.activity {
                    hf_sim::mind::Activity::Working { plan } => {
                        format!("work: {}", plan.label.split(" (").next().unwrap_or(""))
                    }
                    other => other.name().to_string(),
                };
                *hours.entry(what).or_default() += 1;
            }
        }
        if w.meta.tick.is_multiple_of(50 * hf_sim::time::TICKS_PER_DAY) {
            let coal: u32 = w
                .items
                .iter()
                .filter(|it| it.kind == hf_sim::items::ItemKind::Coal)
                .map(|it| it.count)
                .sum();
            coal_days.push(format!("d{day}:{coal}"));
        }
        for e in &w.last_events {
            if (from..=to).contains(&day) && e.severity >= hf_sim::events::Severity::Notable {
                println!("  day {day}: {}", hf_sim::view::narrate::line(&w, e));
            }
            let k = e.kind_name();
            if k == "left_the_valley" {
                firsts.push(format!("day {day}: {}", hf_sim::view::narrate::line(&w, e)));
            }
            // Where a shaft reaching the deep lies against the deep caverns: the tiles from
            // the shaft to each one's nearest floor, on the level.
            if let hf_sim::events::EventKind::ReachedDeep { community, .. } = &e.kind {
                if let Some(m) = w
                    .communities
                    .iter()
                    .find(|c| c.id == *community)
                    .and_then(|c| c.mine.as_ref())
                {
                    let far: Vec<i32> = w
                        .grid
                        .caverns
                        .iter()
                        .filter(|c| c.deep)
                        .map(|c| {
                            let dx = ((m.site.x - c.centre.x).abs() - c.rx).max(0);
                            let dy = ((m.site.y - c.centre.y).abs() - c.ry).max(0);
                            dx.max(dy)
                        })
                        .collect();
                    firsts.push(format!("shaft at {} to the deep caverns {far:?}", m.site));
                }
            }
            if KINDS.contains(&k) {
                if !rows.contains_key(k) {
                    firsts.push(format!("{k} day {day}"));
                }
                *rows.entry(k.to_string()).or_default() += 1;
            }
            if let hf_sim::events::EventKind::Died { cause, .. } = &e.kind {
                *deaths.entry(cause.clone()).or_default() += 1;
            }
            if let hf_sim::events::EventKind::Mined {
                rock: Some(r), at, ..
            } = e.kind
            {
                if r == hf_sim::world::Material::COAL {
                    let by =
                        if w.communities
                            .iter()
                            .filter_map(|c| c.mine.as_ref())
                            .any(|m| m.cut.contains(&at))
                        {
                            "mine"
                        } else if w.communities.iter().filter_map(|c| c.quarry_site).any(|q| {
                            (q.x - at.x).abs().max((q.y - at.y).abs()) <= 10 && at.z < q.z - 1
                        }) {
                            "quarry"
                        } else {
                            "other"
                        };
                    *coal_cut.entry(by).or_default() += 1;
                }
            }
            if let hf_sim::events::EventKind::Mined { rock: Some(r), .. } = e.kind {
                *rocks
                    .entry(w.grid.materials.get(r).name.clone())
                    .or_default() += 1;
            }
        }
    }
    let deepest = w
        .dug
        .iter()
        .map(|p| w.grid.ground_z(p.x, p.y) - p.z)
        .max()
        .unwrap_or(0);
    let lowest = w.dug.iter().map(|p| p.z).min().unwrap_or(0);
    let people = w
        .agents
        .iter()
        .filter(|a| a.alive && a.species == hf_sim::agents::Species::Person)
        .count();
    println!(
        "seed {seed} days {days}{}: {people} people, {:.0} s; deepest cut {deepest} below the ground (z {lowest}); deep caverns at z {:?}",
        if off { " (no mine)" } else { "" },
        t.elapsed().as_secs_f32(),
        w.grid
            .caverns
            .iter()
            .filter(|c| c.deep)
            .map(|c| c.centre.z)
            .collect::<Vec<_>>()
    );
    for c in w.communities.iter().filter(|c| !c.dissolved) {
        let m = c.mine.as_ref();
        println!(
            "  {}: {} members, stance {:.2}, mine {:?}",
            c.name,
            c.members.len(),
            hf_sim::delving::stance(&w, c.id),
            m.map(|m| format!(
                "seek {:?} foot z {} cut {} deep {} stone_level {:?} chose {:?}",
                m.seek,
                m.foot.z,
                m.cut.len(),
                m.deep,
                m.stone_level,
                m.chose
            ))
        );
    }
    println!("  rows {rows:?}");
    println!("  first {firsts:?}");
    println!("  mined {rocks:?}");
    println!("  deaths {deaths:?}");
    let digging: u32 = hours
        .iter()
        .filter(|(k, _)| {
            ["quarrying", "shaft", "prop", "the support"]
                .iter()
                .any(|x| k.contains(x))
        })
        .map(|(_, n)| *n)
        .sum();
    let all: u32 = hours.values().sum();
    println!("  digging person-hours {digging} of {all}");
    let mut h: Vec<(String, u32)> = hours.into_iter().collect();
    h.sort_by_key(|b| std::cmp::Reverse(b.1));
    println!("  person-hours {:?}", &h[..h.len().min(30)]);
    println!("  coal cut by {coal_cut:?}; coal held {coal_days:?}");
    // Where the coal lies at the end: in a stockpile or not, how far from the nearest home
    // (by `TilePos::dist`, which counts a level as four tiles), and at what depth.
    let mut lies: std::collections::BTreeMap<String, u32> = Default::default();
    for it in w
        .items
        .iter()
        .filter(|it| it.kind == hf_sim::items::ItemKind::Coal)
    {
        let key = match it.place {
            hf_sim::items::ItemPlace::Ground(p) => {
                let near = w
                    .communities
                    .iter()
                    .filter(|c| !c.dissolved)
                    .map(|c| c.home.dist(p))
                    .min()
                    .unwrap_or(-1);
                let pile = w
                    .structures
                    .iter()
                    .any(|s| s.pos == p && s.kind == hf_sim::structures::StructureKind::Stockpile);
                format!(
                    "{} {} below {}",
                    if pile { "pile" } else { "ground" },
                    if near <= 45 { "near" } else { "far" },
                    (w.grid.ground_z(p.x, p.y) - p.z).max(0)
                )
            }
            _ => "carried or in a bench".to_string(),
        };
        *lies.entry(key).or_default() += it.count;
    }
    println!("  coal lies {lies:?}");
}
