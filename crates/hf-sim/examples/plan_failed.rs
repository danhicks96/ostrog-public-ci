//! #553: what the failed plans of an opening are. Steps an opening and tallies every
//! `plan_failed` event by goal and reason, by person, and by person, what they set out to do
//! (the activity they started, with its target) and reason, so a plan chosen and failed over and
//! over shows as one row. Also prints alive, the digest and the mean tick time.
//! Run: cargo run --release -p hf-sim --example plan_failed -- <seed> <days> [threads]
use std::collections::BTreeMap;
use std::time::Instant;

use hf_sim::agents::Species;
use hf_sim::events::EventKind;
use hf_sim::mind::knowledge::Known;
use hf_sim::mind::Activity;
use hf_sim::time::TICKS_PER_DAY;

/// What someone set out on, with its target, as a short line (no counters, so it stays the
/// same while they do it).
fn what(a: &Activity) -> String {
    match a {
        Activity::Going { to, then } => format!("going to {to} then {}", what(then)),
        Activity::Working { plan } => format!("work {:?} '{}'", plan.designation, plan.label),
        Activity::Foraging { plant, .. } => format!("foraging {plant}"),
        Activity::Eating { item, .. } => format!("eating {item:?}"),
        Activity::Talking { with, .. } => format!("talking {with:?}"),
        Activity::Sleeping { bed, .. } => format!("sleeping {bed:?}"),
        Activity::Wandering { to } => format!("wandering {to}"),
        other => other.name().to_string(),
    }
}

/// The thing a plan is for: a plant, an item or a person (through a walk to it).
#[derive(Clone, Copy, PartialEq, Debug)]
enum Target {
    None,
    Plant(hf_sim::ids::TilePos),
    Item(hf_sim::ids::ItemId),
    Person(hf_sim::ids::AgentId),
}

fn target(a: &Activity) -> Target {
    match a {
        Activity::Going { then, .. } => target(then),
        Activity::Foraging { plant, .. } => Target::Plant(*plant),
        Activity::Eating { item, .. } => Target::Item(*item),
        Activity::Talking { with, .. } => Target::Person(*with),
        _ => Target::None,
    }
}

fn age(t: u64) -> &'static str {
    match t / hf_sim::time::TICKS_PER_HOUR {
        0 => "<1h",
        1..=5 => "1-6h",
        6..=23 => "6-24h",
        24..=167 => "1-7d",
        _ => ">7d",
    }
}

fn top<K: std::fmt::Debug + Clone>(m: &BTreeMap<K, u64>, n: usize, title: &str) {
    let mut v: Vec<(K, u64)> = m.iter().map(|(k, c)| (k.clone(), *c)).collect();
    v.sort_by_key(|b| std::cmp::Reverse(b.1));
    println!("-- {title} ({} keys)", v.len());
    for (k, c) in v.into_iter().take(n) {
        println!("{c:>8}  {k:?}");
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let seed: u64 = args.get(1).and_then(|a| a.parse().ok()).unwrap_or(3);
    let days: u64 = args.get(2).and_then(|a| a.parse().ok()).unwrap_or(200);
    if let Some(t) = args.get(3).and_then(|a| a.parse().ok()) {
        hf_sim::par::set_threads(t);
    }
    let mut w = hf_sim::World::opening_world(seed);
    // HF553_THIN: as `tests/thin.rs` they_study_it_and_the_wards_quiet_it stages it.
    if std::env::var("HF553_THIN").is_ok() {
        for a in &mut w.agents {
            a.traits.list.retain(|t| *t != hf_sim::mind::Trait::Curious);
        }
        w.ensure_nav();
    }
    // Who came and went, and what the people died of.
    let mut born = 0u64;
    let mut left = 0u64;
    let mut arrived = 0u64;
    let mut died: BTreeMap<String, u64> = BTreeMap::new();
    let mut quieted_day: Option<u64> = None;
    let mut studied = 0u64;
    let mut thin_open_ticks = 0u64;
    let mut started: Vec<String> = Vec::new();
    let mut last: Vec<String> = Vec::new();
    let t0 = w.meta.tick;
    let mut by_reason: BTreeMap<(String, String), u64> = BTreeMap::new();
    let mut by_person: BTreeMap<String, u64> = BTreeMap::new();
    let mut by_set: BTreeMap<(String, String), u64> = BTreeMap::new();
    let mut by_kind: BTreeMap<(String, String), u64> = BTreeMap::new();
    let mut by_day: BTreeMap<u64, u64> = BTreeMap::new();
    let mut why: BTreeMap<(String, String), u64> = BTreeMap::new();
    // Per person: the target they set out for, when, and how old their belief in it was then.
    let mut aim: Vec<(Target, u64, u64)> = Vec::new();
    // Who reached each plant or item, and when.
    let mut reached: BTreeMap<String, Vec<(usize, u64)>> = BTreeMap::new();
    // Forage failures in a row per person (reset by a forage that did not fail), and how
    // long the runs were when they ended.
    let mut streak: Vec<u32> = Vec::new();
    let mut runs: BTreeMap<u32, u64> = BTreeMap::new();
    let mut forage_arrivals = 0u64;
    // Who last picked each bush, and when (a forage that ended without failing).
    let mut picks: BTreeMap<String, (usize, u64)> = BTreeMap::new();
    let trace = std::env::var("HF553_TRACE").ok();
    let block = std::env::var("HF553_BLOCK").is_ok();
    // Per-system time over the run (HF553_PROF), to say where a change in tick time comes from.
    let prof = std::env::var("HF553_PROF").is_ok();
    let n_stages = hf_sim::systems::PIPELINE.len();
    let mut stage = vec![0f64; n_stages];
    let mut stage_ms = vec![0f64; n_stages];
    let mut blocks_shown = 0;
    let mut fails = 0u64;
    let mut chose = 0u64;
    let mut run = 0f64;
    for _ in 0..days * TICKS_PER_DAY {
        // Where each walker stands and the next tile of their path, before the step, to say
        // why a walk was blocked (HF553_BLOCK).
        let before: Vec<(hf_sim::ids::TilePos, Option<hf_sim::ids::TilePos>)> = if block {
            w.agents
                .iter()
                .map(|a| (a.pos, a.path.last().copied()))
                .collect()
        } else {
            Vec::new()
        };
        let t = Instant::now();
        if prof {
            w.step_timed(&mut stage);
            for (k, v) in stage.iter().enumerate() {
                stage_ms[k] += v;
            }
        } else {
            w.step();
        }
        run += t.elapsed().as_secs_f64();
        if started.len() < w.agents.len() {
            started.resize(w.agents.len(), String::new());
            last.resize(w.agents.len(), String::new());
            aim.resize(w.agents.len(), (Target::None, 0, 0));
            streak.resize(w.agents.len(), 0);
        }
        let tick = w.meta.tick;
        if w.thin.strength > 0.3 {
            thin_open_ticks += 1;
        }
        for e in &w.last_events {
            match &e.kind {
                EventKind::Born { .. } => born += 1,
                EventKind::Left { .. } => left += 1,
                EventKind::Arrived { .. } => arrived += 1,
                EventKind::ThinStudied { .. } => studied += 1,
                EventKind::ThinQuieted { .. } => {
                    quieted_day.get_or_insert((w.meta.tick - t0) / TICKS_PER_DAY);
                }
                EventKind::Died { agent, cause }
                    if w.agents
                        .iter()
                        .any(|a| a.id == *agent && a.species == Species::Person) =>
                {
                    let c: String = cause
                        .split_whitespace()
                        .take(3)
                        .collect::<Vec<_>>()
                        .join(" ");
                    *died.entry(c).or_default() += 1;
                }
                EventKind::Chose { .. } => chose += 1,
                EventKind::PlanFailed {
                    agent,
                    goal,
                    reason,
                } => {
                    fails += 1;
                    *by_day
                        .entry((w.meta.tick - t0) / TICKS_PER_DAY / 20 * 20)
                        .or_default() += 1;
                    let ix = w.agents.iter().position(|a| a.id == *agent).unwrap();
                    let name = w.agents[ix].name.clone();
                    *by_reason.entry((goal.clone(), reason.clone())).or_default() += 1;
                    *by_person.entry(name.clone()).or_default() += 1;
                    let set = started[ix].clone();
                    let kind: String = if set.starts_with("work") {
                        set.split('\'').nth(1).unwrap_or("").to_string()
                    } else if let Some(t) = set.split(" then ").nth(1) {
                        format!("going then {}", t.split(' ').next().unwrap_or(""))
                    } else {
                        set.split(' ').next().unwrap_or("").to_string()
                    };
                    *by_kind.entry((kind.clone(), reason.clone())).or_default() += 1;
                    let (tg, since, seen) = aim[ix];
                    let others = |key: String| {
                        reached
                            .get(&key)
                            .is_some_and(|v| v.iter().any(|(j, t)| *j != ix && *t >= since))
                    };
                    let class = match tg {
                        Target::Plant(p) => {
                            let pl = w.plants.iter().find(|pl| pl.pos == p);
                            let state = match pl {
                                None => "plant gone".to_string(),
                                Some(pl) if !pl.alive => "plant dead".to_string(),
                                Some(pl) if pl.fruit == 0.0 => "fruit 0".to_string(),
                                Some(_) => "fruit under 0.3".to_string(),
                            };
                            format!(
                                "{state}; another reached it first {}; belief age {}; walk {}",
                                others(format!("{p}")),
                                age(since.saturating_sub(seen)),
                                age(tick - since)
                            )
                        }
                        Target::Item(id) => {
                            let it = w.items.iter().find(|it| it.id == id);
                            format!(
                                "item {}; another reached it first {}; belief age {}",
                                match it {
                                    None => "gone".to_string(),
                                    Some(it) => format!("{:?}", it.place)
                                        .split('(')
                                        .next()
                                        .unwrap()
                                        .to_string(),
                                },
                                others(format!("{id:?}")),
                                age(since.saturating_sub(seen))
                            )
                        }
                        Target::Person(o) => {
                            let me = w.agents[ix].pos;
                            match w.agents.iter().find(|x| x.id == o) {
                                None => "partner missing".into(),
                                Some(x) if !x.alive => "partner dead".into(),
                                Some(x) => format!(
                                    "partner {} dist {} z-diff {}; walk {}",
                                    x.activity.name(),
                                    match x.pos.dist(me) {
                                        0..=2 => "<=2",
                                        3..=6 => "3-6",
                                        7..=14 => "7-14",
                                        _ => ">14",
                                    },
                                    x.pos.z != me.z,
                                    age(tick - since)
                                ),
                            }
                        }
                        Target::None => String::new(),
                    };
                    *why.entry((format!("{kind}/{reason}"), class)).or_default() += 1;
                    if block && reason == "path blocked" && blocks_shown < 40 {
                        blocks_shown += 1;
                        let (pos, next) =
                            before.get(ix).copied().unwrap_or((w.agents[ix].pos, None));
                        let g = &w.grid;
                        let to = match &w.agents[ix].activity {
                            _ if set.starts_with("going to (") => set
                                .trim_start_matches("going to (")
                                .split(')')
                                .next()
                                .map(|c| {
                                    let v: Vec<i32> =
                                        c.split(',').filter_map(|x| x.parse().ok()).collect();
                                    hf_sim::ids::TilePos::new(v[0], v[1], v[2])
                                }),
                            _ => None,
                        };
                        if let Some(to) = to {
                            let me = w.agents[ix].pos;
                            let path = hf_sim::par::path_between(&w, me, to);
                            let first_flood = path
                                .as_ref()
                                .and_then(|p| p.iter().rev().position(|q| g.over_head(*q)));
                            let first_bad = path.as_ref().and_then(|p| {
                                let mut prev = me;
                                p.iter().rev().position(|q| {
                                    let bad = !g.walkable(*q)
                                        || (q.x - prev.x).abs() > 1
                                        || (q.y - prev.y).abs() > 1
                                        || (q.z - prev.z).abs() > 1;
                                    prev = *q;
                                    bad
                                })
                            });
                            println!(
                                "PATH {name} {me} -> {to}: len {:?} first over-head step {:?} first unwalkable/non-adjacent step {:?}",
                                path.as_ref().map(|p| p.len()),
                                first_flood,
                                first_bad
                            );
                        }
                        println!(
                            "BLOCK {name} d{} {set} was at {pos} next {:?}: walkable {:?} over_head {:?} here over_head {} now at {} walkable here {}",
                            (tick - t0) / TICKS_PER_DAY,
                            next,
                            next.map(|n| g.walkable(n)),
                            next.map(|n| g.over_head(n)),
                            g.over_head(pos),
                            w.agents[ix].pos,
                            g.walkable(w.agents[ix].pos),
                        );
                    }
                    if block && reason == "not at the site" && blocks_shown < 80 {
                        blocks_shown += 1;
                        if let Activity::Working { .. } = &w.agents[ix].activity {
                        } else if let Some(did) = set
                            .strip_prefix("work DesignationId(")
                            .and_then(|r| r.split(')').next())
                            .and_then(|n| n.parse::<u32>().ok())
                        {
                            if let Some(d) = w.designations.iter().find(|d| d.id.0 == did) {
                                let beast = match d.kind {
                                    hf_sim::work::DesignationKind::Tame(b) => {
                                        w.agents.iter().find(|o| o.id == b).map(|o| o.pos)
                                    }
                                    _ => None,
                                };
                                println!(
                                    "SITE {name} d{} {:?} at {} order at {} beast at {:?}",
                                    (tick - t0) / TICKS_PER_DAY,
                                    d.kind,
                                    w.agents[ix].pos,
                                    d.pos,
                                    beast
                                );
                            }
                        }
                    }
                    if reason == "nothing to gather" {
                        streak[ix] += 1;
                        if trace.as_deref().is_some_and(|t| t == "*" || t == name) {
                            if let Target::Plant(p) = tg {
                                let pl = w.plants.iter().find(|pl| pl.pos == p);
                                println!(
                                    "TRACE {name} d{} h{} at {} {p} fruit {:?} in season {:?} seen {}h before start; last picked {:?}",
                                    (tick - t0) / TICKS_PER_DAY,
                                    (tick % TICKS_PER_DAY) / hf_sim::time::TICKS_PER_HOUR,
                                    w.agents[ix].pos,
                                    pl.map(|pl| pl.fruit),
                                    pl.map(|pl| pl.kind.bears_in(w.calendar().season)),
                                    since.saturating_sub(seen) / hf_sim::time::TICKS_PER_HOUR,
                                    picks.get(&format!("{p}")).map(|(j, t)| (
                                        w.agents[*j].name.clone(),
                                        (tick - t) / hf_sim::time::TICKS_PER_HOUR
                                    ))
                                );
                            }
                        }
                    }
                    *by_set
                        .entry((format!("{name}: {set}"), reason.clone()))
                        .or_default() += 1;
                }
                _ => {}
            }
        }
        // What each person set out on, written down when it changes (a failure is reported the
        // tick after it, so this is what they were doing when it failed).
        for (ix, a) in w.agents.iter().enumerate() {
            if a.species != Species::Person || !a.alive {
                continue;
            }
            if a.activity_ticks <= 1 {
                let tg = target(&a.activity);
                if tg != Target::None && tg != aim[ix].0 {
                    let seen = match tg {
                        Target::Plant(p) => a.knowledge.get(Known::FoodPlant(p)),
                        Target::Item(id) => a.knowledge.get(Known::Item(id)),
                        Target::Person(o) => a.knowledge.get(Known::Agent(o)),
                        Target::None => None,
                    }
                    .map_or(tick, |b| b.seen_tick);
                    aim[ix] = (tg, tick, seen);
                }
                match &a.activity {
                    Activity::Foraging { plant, .. } => {
                        let v = reached.entry(format!("{plant}")).or_default();
                        if v.last() != Some(&(ix, tick - 1)) && v.last() != Some(&(ix, tick)) {
                            forage_arrivals += 1;
                        }
                        v.push((ix, tick));
                    }
                    Activity::Eating { item, .. } => reached
                        .entry(format!("{item:?}"))
                        .or_default()
                        .push((ix, tick)),
                    _ => {}
                }
                let now = what(&a.activity);
                if now != last[ix] {
                    // A forage that ran on past its first ticks was a picking, not a failure.
                    if last[ix].starts_with("foraging") && a.activity_ticks <= 1 {
                        let failed_now = w.last_events.iter().any(|e| {
                            matches!(&e.kind, EventKind::PlanFailed { agent, .. } if *agent == a.id)
                        });
                        if !failed_now {
                            picks.insert(
                                last[ix].trim_start_matches("foraging ").to_string(),
                                (ix, tick),
                            );
                        }
                        if !failed_now && streak[ix] > 0 {
                            *runs.entry(streak[ix]).or_default() += 1;
                            streak[ix] = 0;
                        }
                    }
                    if !matches!(a.activity, Activity::Idle) {
                        started[ix] = now.clone();
                    }
                    last[ix] = now;
                }
            }
        }
    }
    let alive = w
        .agents
        .iter()
        .filter(|a| a.alive && a.species == Species::Person)
        .count();
    println!(
        "seed {seed} days {days} threads {} plan_failed {fails} chose {chose} alive_people {alive} mean_ms {:.3} digest {}",
        hf_sim::par::threads(),
        run * 1000.0 / (days * TICKS_PER_DAY) as f64,
        hf_sim::digest::digest(&w)
    );
    println!("failures per 20 days {by_day:?}");
    println!(
        "people: born {born} arrived {arrived} left {left} died {} {died:?}",
        died.values().sum::<u64>()
    );
    {
        let ward_orders: Vec<String> = w
            .designations
            .iter()
            .filter(|d| {
                matches!(
                    d.kind,
                    hf_sim::work::DesignationKind::BuildStructure(
                        hf_sim::structures::StructureKind::WardStone,
                        _
                    )
                )
            })
            .map(|d| {
                format!(
                    "{} done {} failures {} materials {}",
                    d.pos,
                    d.done,
                    d.failures,
                    d.materials_complete()
                )
            })
            .collect();
        let built = w
            .structures
            .iter()
            .filter(|x| x.kind == hf_sim::structures::StructureKind::WardStone)
            .map(|x| format!("{:.2}", x.progress))
            .collect::<Vec<_>>();
        let best = w
            .agents
            .iter()
            .filter(|a| a.alive)
            .map(|a| a.techniques.skill(hf_sim::mind::Technique::ThinLore))
            .fold(0.0f32, f32::max);
        println!(
            "thin detail: chosen {:?} best lore {best:.2} ward stones {built:?} ward orders {ward_orders:?}",
            w.thin.chosen
        );
    }
    println!(
        "thin: studied {studied} quieted day {quieted_day:?} strength {:.2} days over 0.3 {:.1}; plants {}",
        w.thin.strength,
        thin_open_ticks as f64 / TICKS_PER_DAY as f64,
        w.plants.len()
    );
    if prof {
        let mut v: Vec<(f64, &str)> = stage_ms
            .iter()
            .zip(hf_sim::systems::PIPELINE.iter())
            .map(|(ms, (name, _))| (*ms / (days * TICKS_PER_DAY) as f64, *name))
            .collect();
        v.sort_by(|a, b| b.0.total_cmp(&a.0));
        for (ms, name) in v.iter().take(12) {
            println!("stage {name:<20} {ms:.4} ms/tick");
        }
    }
    println!("forage arrivals {forage_arrivals}; failed-forage runs ended by a picking (run length: count) {runs:?}");
    top(&by_reason, 30, "by goal / reason");
    top(&by_kind, 30, "by what was set out on / reason");
    top(&why, 40, "why, by what / reason and the state found");
    top(&by_person, 15, "by person");
    top(&by_set, 40, "by person, what and target, reason");
}
