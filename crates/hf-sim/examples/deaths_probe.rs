//! Who died, of what, where and doing what, and who left the valley (#418). An opening is
//! stepped as `ostrog soak --opening` steps it; each death is printed with its cause, and a
//! drowning with the last hours of the drowned person's track: where they stood, what they were
//! doing, how far the nearest flood water was, and how high the water stood. Each death of
//! sickness is printed with its kind and the care it had. At the end, the table of the issue:
//! people at day N, deaths by cause, people who left and why.
//!
//! Band members (#1563): the members of a band in `w.bands` are listed the tick the band is set
//! down (a BandSetOut event in `w.last_events`: they stand on their set-down tiles, nobody has
//! walked yet) and again every 200 ticks (10 game-minutes). A member's first-seen tile is the
//! tile they were set down on, and the farthest they have been from it is read at the 200-tick
//! listings and at their death (a walk out and back between two listings is not seen). A band
//! death is the death of anyone ever listed, including one who later left the band's list (a
//! raider sent home hurt, a captive the valley kept); it is counted under band deaths by cause,
//! and again when the death tile is within 3 tiles of the first-seen tile. The band table at the
//! end gives those counts and how many listed members never moved more than 6 tiles. `--engine`
//! picks the pacing (Engine) the world is made with; without it the probe runs Steady, as it
//! always has.
//!
//! Run: cargo run --release -p hf-sim --example deaths_probe -- <seed> <days> [quiet] [--engine steady|relentless|whimsical]
use std::collections::{BTreeMap, VecDeque};

use hf_sim::agents::Species;
use hf_sim::engine::Engine;
use hf_sim::events::EventKind as E;
use hf_sim::ids::{AgentId, TilePos};
use hf_sim::mind::Activity;
use hf_sim::threat::Band;
use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use hf_sim::world::TileFlags;
use hf_sim::World;

/// A band member died "where they came in" when the death tile is at most this far from the tile
/// they were first seen on (`TilePos::dist`: tiles in the plane, 4 more for each level). Three
/// tiles is about a stranger's own spread at the edge (spawn_band stands them in a 3-wide group),
/// so a death inside it is a death of someone who never set off.
const CAME_IN_RADIUS: i32 = 3;
/// A band member "never moved" when the farthest they were ever seen from the tile they were
/// first seen on is at most this many tiles: twice the entry spread, so shuffling about the
/// spot they were set down on does not count as walking anywhere.
const STILL_RADIUS: i32 = 6;
/// How often the members of each band are listed after set-out: 200 ticks is 10 game-minutes
/// (docs/TIME.md, 20 ticks to a game-minute), the probe's usual sampling step.
const BAND_SAMPLE_TICKS: u64 = 200;

/// The probe's view of band members (#1563). Collected from `w.bands`, never from `Agent.band`,
/// so it does not depend on whether a death clears that field. Anyone ever listed stays a band
/// member here for good: a later death counts as a band death even if they have since left the
/// band's list.
#[derive(Default)]
struct BandWatch {
    /// Each band member listed: the tile they were first seen on, and the farthest they have
    /// been from it (`TilePos::dist`) at any listing or at their death.
    seen: BTreeMap<AgentId, (TilePos, i32)>,
    /// Deaths of listed members by cause: (all, of those within CAME_IN_RADIUS of first seen).
    died: BTreeMap<String, (u32, u32)>,
}

impl BandWatch {
    /// Called once after every `w.step()`. A band set out on this tick (BandSetOut in
    /// `w.last_events`) is listed at once, while its members are on their set-down tiles (the
    /// first band step that sends them walking is 46 ticks on, 6063); and every BAND_SAMPLE_TICKS
    /// every band is listed. #1563 review: listing only every 200 ticks put the first-seen tile
    /// up to 200 ticks, some 34 tiles of walking, after they came in, and missed a member who
    /// died before the first listing.
    fn tick(&mut self, w: &World) {
        for e in &w.last_events {
            if let E::BandSetOut { band, .. } = &e.kind {
                if let Some(b) = w.bands.iter().find(|b| b.id == *band) {
                    self.list(w, b);
                }
            }
        }
        if w.meta.tick.is_multiple_of(BAND_SAMPLE_TICKS) {
            self.sample(w);
        }
    }

    /// List every living member of every band in `w.bands`.
    fn sample(&mut self, w: &World) {
        for b in &w.bands {
            self.list(w, b);
        }
    }

    /// List every living member of one band.
    fn list(&mut self, w: &World, b: &Band) {
        for m in &b.members {
            if let Some(a) = w.agents.iter().find(|a| a.id == *m && a.alive) {
                self.note(a.id, a.pos);
            }
        }
    }

    /// Record where a band member is: the first time is where they were first seen.
    fn note(&mut self, id: AgentId, pos: TilePos) {
        let e = self.seen.entry(id).or_insert((pos, 0));
        e.1 = e.1.max(pos.dist(e.0));
    }

    /// A person died at `pos` of `cause`. If they were ever listed as a band member (even if no
    /// band lists them now), count it and give back where they were first seen and the farthest
    /// they had been; else `None`.
    fn died(&mut self, id: AgentId, cause: &str, pos: TilePos) -> Option<(TilePos, i32)> {
        let e = self.seen.get_mut(&id)?;
        e.1 = e.1.max(pos.dist(e.0));
        let (first, farthest) = *e;
        let c = self.died.entry(cause.to_string()).or_default();
        c.0 += 1;
        c.1 += (pos.dist(first) <= CAME_IN_RADIUS) as u32;
        Some((first, farthest))
    }

    /// Listed band members never seen more than STILL_RADIUS from where they were first seen.
    fn never_moved(&self) -> usize {
        self.seen
            .values()
            .filter(|(_, farthest)| *farthest <= STILL_RADIUS)
            .count()
    }

    /// The band table: members listed, how many never moved, deaths by cause.
    fn report(&self) -> String {
        let all: u32 = self.died.values().map(|c| c.0).sum();
        let came: u32 = self.died.values().map(|c| c.1).sum();
        let mut out = format!(
            "  band members listed {}; never moved more than {STILL_RADIUS} tiles {}; died {all}, of those within {CAME_IN_RADIUS} tiles of where they were first seen {came}\n",
            self.seen.len(),
            self.never_moved(),
        );
        for (cause, (n, k)) in &self.died {
            out.push_str(&format!(
                "  band deaths, {cause}: {n} (where they came in: {k})\n"
            ));
        }
        out
    }
}

/// What the command line asked for.
struct Args {
    seed: u64,
    days: u64,
    quiet: bool,
    engine: Engine,
}

/// `<seed> <days> [quiet] [--engine NAME]`, the flag anywhere after the program name. The flag and
/// its value are taken out first so that they are not mistaken for the optional `quiet` word.
fn parse_args(args: &[String]) -> Result<Args, String> {
    let mut engine = Engine::default();
    let mut rest: Vec<&String> = Vec::new();
    let mut it = args.iter().skip(1);
    while let Some(a) = it.next() {
        if a == "--engine" {
            let v = it
                .next()
                .ok_or("--engine needs a name: steady, relentless or whimsical")?;
            engine = v.parse()?;
        } else {
            rest.push(a);
        }
    }
    Ok(Args {
        seed: rest.first().and_then(|a| a.parse().ok()).unwrap_or(2),
        days: rest.get(1).and_then(|a| a.parse().ok()).unwrap_or(200),
        quiet: rest.get(2).is_some(),
        engine,
    })
}

#[derive(Default)]
struct Sick {
    kind: String,
    hours: u32,
    in_bed: u32,
    tended: u32,
    hungry: u32,
    treated: bool,
    care: u32,
    near: u32,
    near_any_z: u32,
    designated: u32,
}

fn act(a: &Activity) -> String {
    match a {
        Activity::Going { to, then } => {
            format!("going to {:?} then {}", (to.x, to.y, to.z), act(then))
        }
        Activity::Working { plan } => {
            let s = format!("{:?}", plan);
            format!("working {}", &s[..s.len().min(90)])
        }
        Activity::Sleeping { bed, .. } => format!("sleeping (bed {})", bed.is_some()),
        Activity::Wandering { to } => format!("wandering to {:?}", (to.x, to.y, to.z)),
        o => o.name().to_string(),
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let Args {
        seed,
        days,
        quiet,
        engine,
    } = parse_args(&args).unwrap_or_else(|e| {
        eprintln!("{e}");
        std::process::exit(2)
    });
    let mut setup = hf_sim::setup::WorldSetup::new(seed);
    setup.engine = engine;
    let mut w = World::from_setup(&setup);
    let mut band_watch = BandWatch::default();
    let mut track: BTreeMap<AgentId, VecDeque<String>> = BTreeMap::new();
    let mut stages: VecDeque<Vec<f32>> = VecDeque::new();
    let mut sick: BTreeMap<AgentId, Sick> = BTreeMap::new();
    let mut deaths: BTreeMap<String, u32> = BTreeMap::new();
    let mut was_leaving: BTreeMap<AgentId, bool> = BTreeMap::new();
    let mut last_comm: BTreeMap<AgentId, Option<u32>> = BTreeMap::new();
    let mut left: BTreeMap<String, u32> = BTreeMap::new();
    let mut valley: std::collections::BTreeSet<AgentId> = Default::default();
    for _ in 0..days * TICKS_PER_DAY {
        w.step();
        let tick = w.meta.tick;
        if tick % TICKS_PER_HOUR == 301 {
            stages.push_back(w.waters.bodies.iter().map(|b| b.stage).collect());
            if stages.len() > 12 {
                stages.pop_front();
            }
            for a in w
                .agents
                .iter()
                .filter(|a| a.alive && a.species == Species::Person)
            {
                if let Some(ill) = a.body.illness {
                    let s = sick.entry(a.id).or_default();
                    s.kind = ill.kind.name().into();
                    s.hours += 1;
                    s.in_bed +=
                        matches!(a.activity, Activity::Sleeping { bed: Some(_), .. }) as u32;
                    s.tended += (a.tended_until > tick) as u32;
                    s.hungry += (a.needs.hunger < 0.2) as u32;
                    s.treated |= ill.treated == Some(true);
                    s.care += a.body.needs_care() as u32;
                    let healer = |h: &&hf_sim::agents::Agent| {
                        h.alive
                            && h.id != a.id
                            && h.community.is_some()
                            && h.community == a.community
                            && h.techniques.knows(hf_sim::mind::Technique::Poultice)
                    };
                    s.near += w
                        .agents
                        .iter()
                        .filter(healer)
                        .any(|h| h.pos.z == a.pos.z && h.pos.dist(a.pos) <= 25)
                        as u32;
                    s.near_any_z +=
                        w.agents
                            .iter()
                            .filter(healer)
                            .any(|h| h.pos.dist(a.pos) <= 25) as u32;
                    s.designated += w
                        .designations
                        .iter()
                        .any(|d| !d.done && d.kind == hf_sim::work::DesignationKind::Tend(a.id))
                        as u32;
                } else {
                    sick.remove(&a.id);
                }
            }
        }
        band_watch.tick(&w);
        let floods = !w.flooded.is_empty() || !w.waters.full.is_empty();
        if tick.is_multiple_of(if floods { 20 } else { 200 }) {
            for a in w
                .agents
                .iter()
                .filter(|a| a.alive && a.species == Species::Person)
            {
                if a.community.is_some() {
                    valley.insert(a.id);
                }
                let near = w.flooded.iter().map(|p| p.dist(a.pos)).min().unwrap_or(99);
                let t = w.grid.get(a.pos);
                let line = format!(
                    "  d{} h{:02}:{:02} at {:?} {}{}{} flood {}t away; {}",
                    tick / TICKS_PER_DAY,
                    (tick % TICKS_PER_DAY) / TICKS_PER_HOUR,
                    (tick % TICKS_PER_HOUR) / 20,
                    (a.pos.x, a.pos.y, a.pos.z),
                    if t.is_water() { "WATER " } else { "" },
                    if t.flags.has(TileFlags::FULL) {
                        "FULL "
                    } else {
                        ""
                    },
                    if w.flooded.contains(&a.pos) {
                        "FLOODED "
                    } else {
                        ""
                    },
                    near,
                    act(&a.activity),
                );
                let tr = track.entry(a.id).or_default();
                tr.push_back(line);
                if tr.len() > 40 {
                    tr.pop_front();
                }
            }
        }
        for a in w.agents.iter().filter(|a| a.species == Species::Person) {
            let prev = was_leaving.insert(a.id, a.leaving).unwrap_or(false);
            let comm = a.community.map(|c| c.0);
            let before = last_comm.insert(a.id, comm).flatten();
            if a.leaving && !prev && !quiet {
                println!(
                    "d{} LEAVING {} community {:?} (was {:?}) band {:?} held {} age {}",
                    tick / TICKS_PER_DAY,
                    a.name,
                    comm,
                    before,
                    a.band,
                    a.held.is_some(),
                    a.age_years(tick)
                );
            }
        }
        for e in w.last_events.clone() {
            match &e.kind {
                E::Died { agent, .. } => {
                    let Some(a) = w.agents.iter().find(|a| a.id == *agent) else {
                        continue;
                    };
                    if a.species != Species::Person {
                        continue;
                    }
                    let cause = a.death.clone().unwrap_or_default();
                    let ill = a.body.illness.map(|i| i.kind.name()).unwrap_or("-");
                    let key = if cause == "fever" {
                        format!("ill: {ill}")
                    } else {
                        cause.clone()
                    };
                    let band = band_watch.died(a.id, &key, a.pos);
                    *deaths.entry(key).or_default() += 1;
                    if quiet {
                        continue;
                    }
                    println!(
                        "d{} h{:02} DIED {} ({}y, {}) {} at {:?} night {} community {:?}{}",
                        tick / TICKS_PER_DAY,
                        (tick % TICKS_PER_DAY) / TICKS_PER_HOUR,
                        a.name,
                        a.age_years(tick),
                        if a.is_child(tick) { "child" } else { "adult" },
                        cause,
                        (a.pos.x, a.pos.y, a.pos.z),
                        hf_sim::time::Calendar::is_night(tick),
                        a.community.map(|c| c.0),
                        match band {
                            Some((first, far)) => format!(
                                " ever listed as a band member, first seen {:?} farthest {far}",
                                (first.x, first.y, first.z)
                            ),
                            None => String::new(),
                        },
                    );
                    if cause.contains("drowned") {
                        let bi = w.waters.flood_of.get(&a.pos).copied().or_else(|| {
                            w.waters
                                .bodies
                                .iter()
                                .position(|b| b.tiles.contains(&a.pos))
                                .map(|i| i as u16)
                        });
                        if let Some(bi) = bi {
                            let b = &w.waters.bodies[bi as usize];
                            let hist: Vec<String> = stages
                                .iter()
                                .map(|s| format!("{:.2}", s[bi as usize]))
                                .collect();
                            println!(
                                "  body {bi} {:?} surface z{} area {} stage last 12h: {}",
                                b.kind,
                                b.surface,
                                b.area(),
                                hist.join(" ")
                            );
                        }
                        if let Some(tr) = track.get(agent) {
                            for l in tr {
                                println!("{l}");
                            }
                        }
                    }
                    if cause == "fever" {
                        if let Some(tr) = track.get(agent) {
                            for l in tr.iter().rev().take(6).rev() {
                                println!("{l}");
                            }
                        }
                        if let Some(s) = sick.get(agent) {
                            let poultices: u32 = w
                                .items
                                .iter()
                                .filter(|it| it.kind == hf_sim::items::ItemKind::Poultice)
                                .map(|it| it.count)
                                .sum();
                            let healers = w
                                .agents
                                .iter()
                                .filter(|h| {
                                    h.alive
                                        && h.community.is_some()
                                        && h.community == a.community
                                        && (h.techniques.knows(hf_sim::mind::Technique::Poultice)
                                            || h.role == Some(hf_sim::society::Role::Healer))
                                })
                                .count();
                            let beds = w
                                .structures
                                .iter()
                                .filter(|b| b.kind == hf_sim::structures::StructureKind::Bed)
                                .count();
                            println!(
                                "  {} for {}h: in bed {}h, tended {}h, hungry {}h, right care {}; poultices in the world {}, healers in the community {}, beds {}; needing care {}h, a healer within 25 on their level {}h (any level {}h), a tending set {}h",
                                s.kind, s.hours, s.in_bed, s.tended, s.hungry, s.treated, poultices, healers, beds, s.care, s.near, s.near_any_z, s.designated
                            );
                        }
                    }
                }
                E::FellIll { what, .. } => {
                    *deaths.entry(format!("(fell ill: {what})")).or_default() += 1;
                }
                E::Tended { .. } => {
                    *deaths.entry("(tended)".into()).or_default() += 1;
                }
                E::Left { agent } if !quiet => {
                    let n = w
                        .agents
                        .iter()
                        .find(|a| a.id == *agent)
                        .map(|a| a.name.clone());
                    println!("d{} LEFT {:?}", tick / TICKS_PER_DAY, n);
                }
                _ => {}
            }
        }
        if tick.is_multiple_of(TICKS_PER_DAY) && !quiet {
            let alive = w
                .agents
                .iter()
                .filter(|a| a.alive && a.species == Species::Person && a.community.is_some())
                .count();
            if (tick / TICKS_PER_DAY).is_multiple_of(10) {
                println!("d{} people {alive}", tick / TICKS_PER_DAY);
            }
        }
    }
    let alive = w
        .agents
        .iter()
        .filter(|a| a.alive && a.species == Species::Person)
        .count();
    for a in w
        .agents
        .iter()
        .filter(|a| a.species == Species::Person && !a.alive && a.death.is_none())
    {
        let k = if valley.contains(&a.id) {
            "of the valley"
        } else if a.band.is_some() {
            "a visiting band going home"
        } else {
            "a stranger who did not stay"
        };
        *left.entry(k.into()).or_default() += 1;
    }
    println!("SEED {seed} day {days}: people {alive}; deaths {deaths:?}; left {left:?}");
    println!("BAND TABLE seed {seed} engine {engine} day {days}");
    print!("{}", band_watch.report());
}

#[cfg(test)]
mod tests {
    use super::*;
    use hf_sim::events::{Event, Severity};
    use hf_sim::ids::EventId;
    use hf_sim::threat::BandState;
    use hf_sim::world::gen::GenParams;

    fn argv(v: &[&str]) -> Vec<String> {
        std::iter::once("deaths_probe")
            .chain(v.iter().copied())
            .map(String::from)
            .collect()
    }

    fn band(id: u32, members: Vec<AgentId>) -> Band {
        Band {
            id,
            site: 0,
            site_name: "test".into(),
            size: members.len() as u32,
            members,
            camp: TilePos::new(0, 0, 0),
            state: BandState::Approaching,
            formed: 0,
            want: 0,
            taken: 0,
            lost: 0,
            sighted: false,
            welcomed: false,
            pursuits: Vec::new(),
            raiders: Vec::new(),
            envoys: Vec::new(),
            eaten: 0,
            night: 0,
            night_taken: 0,
            remembered: false,
            seen: Vec::new(),
            host: false,
            captives: Vec::new(),
            heard: Vec::new(),
        }
    }

    fn person(w: &mut World, x: i32, y: i32, band: Option<u32>) -> AgentId {
        let id = w.spawn_agent(Species::Person, TilePos::new(x, y, 0));
        w.agents.iter_mut().find(|a| a.id == id).unwrap().band = band;
        id
    }

    fn move_to(w: &mut World, id: AgentId, x: i32, y: i32) {
        w.agents.iter_mut().find(|a| a.id == id).unwrap().pos = TilePos::new(x, y, 0);
    }

    /// Two bands in `w.bands`, one stranger with a band number who is in no band, one settler,
    /// one band member already dead, and one band member whose `Agent.band` was cleared.
    /// Every figure below is worked out by hand from the positions set here.
    #[test]
    fn band_members_are_listed_from_w_bands_and_counted_when_they_die() {
        let mut w = World::generate(1, GenParams::small());
        // First seen: A (20,20), B (22,20), C (24,20) of band 1; D (40,40), F (30,30) of band 2.
        let a = person(&mut w, 20, 20, Some(1));
        let b = person(&mut w, 22, 20, Some(1));
        let c = person(&mut w, 24, 20, Some(1));
        let d = person(&mut w, 40, 40, Some(2));
        // F is in band 2's list but `Agent.band` is None: the probe must not depend on that field.
        let f = person(&mut w, 30, 30, None);
        // G is in band 2's list but is already dead when the first listing is made.
        let g = person(&mut w, 60, 60, Some(2));
        w.agents.iter_mut().find(|x| x.id == g).unwrap().alive = false;
        // S is a settler; H carries a band number but is in no band's list.
        let s = person(&mut w, 50, 50, None);
        let h = person(&mut w, 70, 70, Some(3));
        w.bands.push(band(1, vec![a, b, c]));
        w.bands.push(band(2, vec![d, f, g]));

        let mut bw = BandWatch::default();
        bw.sample(&w);
        // Listed: A, B, C, D, F = 5. Not G (dead), not S (settler), not H (in no band's list).
        assert_eq!(bw.seen.len(), 5, "A, B, C, D and F only");
        assert_eq!(bw.seen.get(&a), Some(&(TilePos::new(20, 20, 0), 0)));
        assert_eq!(bw.seen.get(&f), Some(&(TilePos::new(30, 30, 0), 0)));
        for (who, name) in [(g, "dead G"), (s, "settler S"), (h, "H, in no band")] {
            assert!(!bw.seen.contains_key(&who), "{name} must not be listed");
        }

        // They move. B to (22,27): farthest max(|0|,|7|) = 7. C to (27,22): max(3,2) = 3.
        // D to (40,46): max(0,6) = 6. A and F stay: 0.
        move_to(&mut w, b, 22, 27);
        move_to(&mut w, c, 27, 22);
        move_to(&mut w, d, 40, 46);
        bw.sample(&w);
        let far = |bw: &BandWatch, id| bw.seen[&id].1;
        assert_eq!(
            [a, b, c, d, f].map(|i| far(&bw, i)),
            [0, 7, 3, 6, 0],
            "farthest from first seen, A B C D F"
        );
        // B comes back to (22,21), 1 from where they began: the farthest stays 7, not 1.
        move_to(&mut w, b, 22, 21);
        bw.sample(&w);
        assert_eq!(
            far(&bw, b),
            7,
            "farthest is a maximum, not the last distance"
        );

        // Deaths. All at z 0, so a tile's distance is the larger of the x and y gaps.
        // A at (20,20): 0 from (20,20), came in.
        let r = bw.died(a, "thirst", TilePos::new(20, 20, 0));
        assert_eq!(r, Some((TilePos::new(20, 20, 0), 0)));
        // C at (27,22): max(|27-24|, |22-20|) = 3, the edge of 3, came in.
        bw.died(c, "thirst", TilePos::new(27, 22, 0));
        // B at (22,28): |28-20| = 8 from (22,20), not where they came in; farthest becomes 8.
        let r = bw.died(b, "thirst", TilePos::new(22, 28, 0));
        assert_eq!(r, Some((TilePos::new(22, 20, 0), 8)));
        // D at (40,44): 4 from (40,40), one past the edge: not where they came in, and the
        // farthest stays 6 (a death nearer than a past listing does not lower it).
        let r = bw.died(d, "drowned", TilePos::new(40, 44, 0));
        assert_eq!(r, Some((TilePos::new(40, 40, 0), 6)));
        // Not band members: the settler, H, and G who died before they were ever listed.
        for who in [s, h, g] {
            assert_eq!(bw.died(who, "thirst", TilePos::new(50, 50, 0)), None);
        }
        // Band deaths: thirst 3 (A, C, B) of which 2 came in (A, C); drowned 1 of which 0.
        assert_eq!(bw.died.get("thirst"), Some(&(3, 2)));
        assert_eq!(bw.died.get("drowned"), Some(&(1, 0)));
        assert_eq!(bw.died.len(), 2, "only the causes of band members");
        // Never moved more than 6: A 0, C 3, D 6 (the edge), F 0 = 4. Not B (8).
        assert_eq!(bw.never_moved(), 4);
        assert_eq!(
            bw.report(),
            "  band members listed 5; never moved more than 6 tiles 4; died 4, of those within 3 \
             tiles of where they were first seen 2\n  band deaths, drowned: 1 (where they came in: 0)\n  \
             band deaths, thirst: 3 (where they came in: 2)\n"
        );
    }

    /// A level counts 4 tiles (`TilePos::dist`): dying straight above the entry tile is not
    /// "where they came in".
    #[test]
    fn a_death_one_level_up_is_not_where_they_came_in() {
        let mut bw = BandWatch::default();
        let id = AgentId(7);
        bw.note(id, TilePos::new(20, 20, 0));
        // |dz| 1 * 4 = 4 from the entry tile, past the 3.
        bw.died(id, "fell", TilePos::new(20, 20, 1));
        assert_eq!(bw.died.get("fell"), Some(&(1, 0)));
    }

    /// The empty case: people and no band, and a table with nothing in it.
    #[test]
    fn with_no_bands_the_table_is_empty() {
        let mut w = World::generate(1, GenParams::small());
        let s = person(&mut w, 20, 20, None);
        let stranger = person(&mut w, 25, 25, Some(1));
        assert!(w.bands.is_empty());
        let mut bw = BandWatch::default();
        bw.sample(&w);
        assert!(bw.seen.is_empty());
        assert_eq!(bw.never_moved(), 0);
        assert_eq!(bw.died(s, "thirst", TilePos::new(20, 20, 0)), None);
        // A band number on the agent alone does not make a band member.
        assert_eq!(bw.died(stranger, "thirst", TilePos::new(25, 25, 0)), None);
        assert_eq!(
            bw.report(),
            "  band members listed 0; never moved more than 6 tiles 0; died 0, of those within 3 \
             tiles of where they were first seen 0\n"
        );
    }

    fn set_out(band: u32, size: u32, tick: u64) -> Event {
        Event {
            id: EventId(1),
            tick,
            at: None,
            severity: Severity::Trace,
            kind: E::BandSetOut {
                band,
                site: "test".into(),
                size,
            },
        }
    }

    /// A band is set down at in-day tick 6017 and its first walking step is 46 ticks on, so a
    /// member can be about 34 tiles away at the first 200-tick listing (#1563 review). The
    /// first-seen tile must be the set-down tile, and a member who dies before that first
    /// listing must still be counted. Every figure is worked out by hand from the positions set.
    #[test]
    fn a_band_is_listed_on_the_tick_it_is_set_out_not_at_the_next_200_tick_listing() {
        let mut w = World::generate(1, GenParams::small());
        // Set down at A (20,20), B (21,20), C (22,21), D (21,21), the band's own spread.
        let a = person(&mut w, 20, 20, Some(1));
        let b = person(&mut w, 21, 20, Some(1));
        let c = person(&mut w, 22, 21, Some(1));
        let d = person(&mut w, 21, 21, Some(1));
        w.bands.push(band(1, vec![a, b, c, d]));
        let mut bw = BandWatch::default();

        // Tick 6016: the band is in `w.bands` but nothing has been set out and 6016 is not a
        // multiple of 200: nothing is listed.
        w.meta.tick = 6016;
        bw.tick(&w);
        assert!(
            bw.seen.is_empty(),
            "a tick with no set-out and no listing lists no one"
        );

        // Tick 6017: BandSetOut for band 1. Listed on this tick, on the set-down tiles.
        w.meta.tick = 6017;
        w.last_events = vec![set_out(1, 4, 6017)];
        bw.tick(&w);
        assert_eq!(
            bw.seen.len(),
            4,
            "A, B, C and D are listed the tick they are set out"
        );
        assert_eq!(bw.seen.get(&a), Some(&(TilePos::new(20, 20, 0), 0)));
        assert_eq!(bw.seen.get(&c), Some(&(TilePos::new(22, 21, 0), 0)));

        // Tick 6100: D dies at (24,21) of thirst, 3 from (21,21), before the first 200-tick
        // listing. It is a band death, where they came in.
        w.meta.tick = 6100;
        w.last_events.clear();
        move_to(&mut w, d, 24, 21);
        w.agents.iter_mut().find(|x| x.id == d).unwrap().alive = false;
        bw.tick(&w);
        assert_eq!(
            bw.died(d, "thirst", TilePos::new(24, 21, 0)),
            Some((TilePos::new(21, 21, 0), 3)),
            "D died before the first 200-tick listing and is still a band member"
        );

        // Tick 6200: A has walked 30 tiles east to (50,20), B 30 south to (21,50); C stays at
        // (22,21), 0 from where C was set down. The 200-tick listing finds them there.
        w.meta.tick = 6200;
        move_to(&mut w, a, 50, 20);
        move_to(&mut w, b, 21, 50);
        bw.tick(&w);
        assert_eq!(
            [a, b, c].map(|i| bw.seen[&i]),
            [
                (TilePos::new(20, 20, 0), 30),
                (TilePos::new(21, 20, 0), 30),
                (TilePos::new(22, 21, 0), 0),
            ],
            "first seen is the set-down tile, not the tile at the 6200 listing; farthest is 30"
        );
        // Never moved more than 6: C (0) and D (3, a death inside the radius) = 2. Not A or B.
        assert_eq!(bw.never_moved(), 2);
        assert_eq!(bw.died.get("thirst"), Some(&(1, 1)));
    }

    /// Anyone ever listed stays a band member: a raider sent home hurt, or a captive kept by the
    /// valley (siege.rs clears `Agent.band` and takes them out of the band's list), who dies
    /// later is still a band death. The probe counts it so; this pins that.
    #[test]
    fn a_member_listed_and_later_out_of_every_band_list_who_dies_is_still_a_band_death() {
        let mut w = World::generate(1, GenParams::small());
        let a = person(&mut w, 20, 20, Some(1));
        let b = person(&mut w, 22, 20, Some(1));
        w.bands.push(band(1, vec![a, b]));
        let mut bw = BandWatch::default();
        w.meta.tick = 6017;
        w.last_events = vec![set_out(1, 2, 6017)];
        bw.tick(&w);
        assert_eq!(bw.seen.len(), 2);

        // A is taken by the valley (out of the band's list, `Agent.band` cleared); the band
        // itself goes on without them. A 200-tick listing follows.
        w.bands[0].members.retain(|m| *m != a);
        w.agents.iter_mut().find(|x| x.id == a).unwrap().band = None;
        w.last_events.clear();
        w.meta.tick = 6200;
        bw.tick(&w);
        assert_eq!(bw.seen.len(), 2, "listing someone else does not forget A");

        // The band is gone altogether, and A dies of thirst in the valley, 25 tiles from where
        // they were first seen: a band death, not where they came in.
        w.bands.clear();
        w.meta.tick = 6400;
        bw.tick(&w);
        assert_eq!(
            bw.died(a, "thirst", TilePos::new(45, 20, 0)),
            Some((TilePos::new(20, 20, 0), 25))
        );
        assert_eq!(bw.died.get("thirst"), Some(&(1, 0)));
        assert_eq!(bw.never_moved(), 1, "B, who never moved; A went 25");
    }

    #[test]
    fn the_engine_flag_is_read_wherever_it_stands_and_is_not_taken_for_quiet() {
        let a = parse_args(&argv(&["4", "700", "--engine", "relentless"])).unwrap();
        assert_eq!(
            (a.seed, a.days, a.quiet, a.engine),
            (4, 700, false, Engine::Relentless)
        );
        // Before the numbers, and by the deity's name.
        let a = parse_args(&argv(&["--engine", "hask", "4", "700"])).unwrap();
        assert_eq!(
            (a.seed, a.days, a.quiet, a.engine),
            (4, 700, false, Engine::Relentless)
        );
        // No flag: Steady, as before; the third bare word is still `quiet`.
        let a = parse_args(&argv(&["3", "10"])).unwrap();
        assert_eq!(
            (a.seed, a.days, a.quiet, a.engine),
            (3, 10, false, Engine::Steady)
        );
        let a = parse_args(&argv(&["3", "10", "quiet", "--engine", "whimsical"])).unwrap();
        assert_eq!(
            (a.seed, a.days, a.quiet, a.engine),
            (3, 10, true, Engine::Whimsical)
        );
        // No arguments at all: the old defaults.
        let a = parse_args(&argv(&[])).unwrap();
        assert_eq!(
            (a.seed, a.days, a.quiet, a.engine),
            (2, 200, false, Engine::Steady)
        );
    }

    #[test]
    fn a_bad_or_missing_engine_name_is_an_error() {
        assert!(parse_args(&argv(&["4", "700", "--engine", "loud"])).is_err());
        assert!(parse_args(&argv(&["4", "700", "--engine"])).is_err());
    }
}
