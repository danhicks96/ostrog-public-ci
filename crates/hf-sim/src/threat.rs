//! Outside pressure (issue #47): trouble that arrives without being asked for, from causes in
//! the world.
//!
//! The places beyond the valley (expedition.rs `Site`) have people and food stores that rise and
//! fall with the seasons and with good and bad years. When a place goes hungry for days, some of
//! its people set out. They come to the valley as a band: real people (agents) who camp near the
//! settlement, raid its stores at night, and leave when they have enough, when they have lost too
//! many, or when they have waited too long. Settlers respond by what they believe: some go out to
//! confront them, some to tell them they may eat, some keep away. Every step is an event, so the
//! feed and the chronicle tell the arc from the hungry place to the aftermath.
//!
//! Nothing here fires an event without a cause in the world: a band exists because a place ran
//! out of food, and it leaves for reasons the feed can name.
use crate::fmath::Det;

use crate::events::{EventKind, Severity};
use crate::ids::{AgentId, TilePos};
use crate::items::ItemPlace;
use crate::mind::beliefs::Topic;
use crate::mind::Activity;
use crate::rng::{purpose, stream};
use crate::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use crate::World;
use rand::Rng;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum BandState {
    /// Walking in from the edge toward the camp.
    Approaching,
    /// Camped near the settlement since this tick.
    Camped(u64),
    /// Going home.
    Leaving,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Band {
    pub id: u32,
    /// The overworld site they came from.
    pub site: u32,
    pub site_name: String,
    pub members: Vec<AgentId>,
    pub size: u32,
    pub camp: TilePos,
    pub state: BandState,
    pub formed: u64,
    /// Person-days of food they came for, and what they have so far.
    pub want: u32,
    pub taken: u32,
    /// Members hurt badly enough to turn back, or dead.
    pub lost: u32,
    pub sighted: bool,
    /// Someone told them they could eat: they take openly, by day, and leave in peace.
    pub welcomed: bool,
    /// Settlers going after one of them: (settler, raider, since).
    #[serde(default)]
    pub pursuits: Vec<(AgentId, AgentId, u64)>,
    /// Members out raiding: (raider, food item they are going for).
    #[serde(default)]
    pub raiders: Vec<(AgentId, crate::ids::ItemId)>,
    /// Settlers on their way to the camp to say the band may eat.
    #[serde(default)]
    pub envoys: Vec<AgentId>,
    /// Of what they took, how much they have had to eat here.
    #[serde(default)]
    pub eaten: u32,
    /// The night (or, once welcomed, the day) they are taking in, and how much so far: what
    /// a few people can carry off in one night is limited.
    #[serde(default)]
    pub night: u64,
    #[serde(default)]
    pub night_taken: u32,
    /// They have stood at the graves of their own people's hamlet (lore.rs).
    #[serde(default)]
    pub remembered: bool,
    /// Food they have seen, and where (FIXLOG 2026-10-20 #173, C62: they took from every
    /// stack in the valley, seen or not).
    #[serde(default)]
    pub seen: Vec<(crate::ids::ItemId, TilePos)>,
    /// C76: a host come to lay siege over a grudge, not a hungry band.
    #[serde(default)]
    pub host: bool,
    /// C76: people of the valley this host has carried off.
    #[serde(default)]
    pub captives: Vec<AgentId>,
    /// The valley's renowned things they had heard of when they set out, and where the telling
    /// put them (#173, Dan's call: they may come for a treasure whose fame has reached them).
    #[serde(default)]
    pub heard: Vec<(crate::ids::ItemId, TilePos)>,
}

/// The valley's renowned things (a name, or spoken of) lying in the open within thirty tiles
/// of its home, and where they lay: everything word of which could reach a band.
pub fn heard_of(w: &World, home: TilePos) -> Vec<(crate::ids::ItemId, TilePos)> {
    w.items
        .iter()
        .filter(|it| {
            it.story
                .as_ref()
                .is_some_and(|s| s.name.is_some() || s.spoken_of)
        })
        .filter_map(|it| match it.place {
            ItemPlace::Ground(p) if p.dist(home) <= 30 => Some((it.id, p)),
            _ => None,
        })
        .collect()
}

/// How far a thing's fame has travelled, 0 to 1: its name, being spoken of, its deeds and its
/// age in the valley. A new-named figure is a tenth; a year-old one with deeds about a half.
pub fn fame(s: &crate::legend::Story, tick: u64) -> f32 {
    // Not in its first season (#249: a figure stood 28 days before a band carried it off).
    if tick.saturating_sub(s.made) < 30 * crate::time::TICKS_PER_DAY {
        return 0.0;
    }
    let years = tick.saturating_sub(s.made) as f32 / crate::time::TICKS_PER_YEAR as f32;
    let f = 0.1 * u8::from(s.name.is_some()) as f32
        + 0.1 * u8::from(s.spoken_of) as f32
        + 0.08 * s.deeds.len().min(5) as f32
        + 0.25 * years.min(2.0);
    f.min(0.9)
}

/// What word of the valley reached a band before it set out (#232, D34): at most one renowned
/// thing, the one whose fame is widest, and only if its fame carried that far. Before this
/// every band knew every named thing and its tile, and two valleys in four lost all of them
/// by day 300 (docs/history/FIXLOG.md 2026-10-21 C68).
pub fn word_of(
    w: &World,
    home: TilePos,
    site: u32,
    tick: u64,
) -> Vec<(crate::ids::ItemId, TilePos)> {
    let mut r = stream(w.meta.seed, &[purpose::THREATS, 0xFA3E, site as u64, tick]);
    let mut known: Vec<(f32, crate::ids::ItemId, TilePos)> = heard_of(w, home)
        .into_iter()
        .filter_map(|(id, p)| {
            let it = w.items.iter().find(|it| it.id == id)?;
            Some((fame(it.story.as_ref()?, tick), id, p))
        })
        .collect();
    known.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap().then(a.1.cmp(&b.1)));
    // A hamlet's one or two named things are not famous thirty tiles off (#249: with a band
    // every twenty days, "one a visit" was the whole treasury): word travels in proportion
    // to how much there is to tell of, up to four things.
    let few = known.len().min(4) as f32 / 4.0;
    match known.first() {
        Some(&(f, id, p)) if r.gen_bool((f * few) as f64) => vec![(id, p)],
        _ => Vec::new(),
    }
}

/// Members note the food in their sight (a band's own knowledge; at most 48 places).
fn look_about(w: &mut World, k: usize, home: TilePos) {
    let eyes: Vec<TilePos> = w.bands[k]
        .members
        .iter()
        .filter_map(|m| idx(w, *m).map(|i| w.agents[i].pos))
        .collect();
    let r = crate::mind::perception::SIGHT_RADIUS;
    let mut seen = std::mem::take(&mut w.bands[k].seen);
    for it in &w.items {
        let ItemPlace::Ground(p) = it.place else {
            continue;
        };
        if !it.kind.is_food() || p.dist(home) > 30 {
            continue;
        }
        let in_sight = eyes.iter().any(|e| {
            e.z == p.z
                && (e.x - p.x).abs().max((e.y - p.y).abs()) <= r
                && crate::mind::perception::can_see(&w.grid, *e, p)
        });
        if !in_sight {
            continue;
        }
        // Seen before and since carried elsewhere: where it lies now (C71: a band's whole
        // list went stale once the hauls moved its food into the pile, and it camped ten days
        // without a raid, looking at nothing).
        if let Some(s) = seen.iter_mut().find(|s| s.0 == it.id) {
            s.1 = p;
        } else {
            seen.push((it.id, p));
        }
    }
    if seen.len() > 48 {
        seen.drain(..seen.len() - 48);
    }
    w.bands[k].seen = seen;
}

pub const BAND_STEP: u64 = 50;
/// After going after a band, a settler does not choose to again for this long.
pub const CONFRONT_REST: u64 = 10 * TICKS_PER_HOUR;

/// work_cooldowns key for "has gone after this band lately".
pub fn confront_key(band: u32) -> u32 {
    0x5000_0000 | band
}
/// How far from the settlement a band camps.
const CAMP_DIST: i32 = 24;

pub fn step(w: &mut World) {
    let tick = w.meta.tick;
    if tick % TICKS_PER_DAY == 5 * TICKS_PER_HOUR + 17 {
        sites_daily(w);
    }
    if tick % BAND_STEP == 13 {
        bands_step(w);
    }
}

/// What the valley must have done to a place's people before it sends a host (C76, D47).
pub const GRUDGE_HOST: u32 = 4;

fn home(w: &World) -> Option<TilePos> {
    w.communities.iter().find(|c| !c.dissolved).map(|c| c.home)
}

/// The places beyond the valley eat and grow. A hungry place may send people.
fn sites_daily(w: &mut World) {
    let tick = w.meta.tick;
    let day = tick / TICKS_PER_DAY;
    let seed = w.meta.seed;
    let cal = w.calendar();
    let year = cal.year;
    let season = cal.season;
    let Some(home) = home(w) else { return };
    let settled = w.since_party() >= 4 * TICKS_PER_DAY;
    let mut set_out: Vec<(u32, bool)> = Vec::new();
    // What the deity is doing to each place's harvest (deity.rs).
    let harvest: Vec<f32> = w
        .overworld
        .sites
        .iter()
        .map(|s| {
            w.lean(
                crate::deity::Lever::Harvest,
                crate::deity::Scope::Site(s.id),
            )
        })
        .collect();
    for (k, s) in w.overworld.sites.iter_mut().enumerate() {
        if s.people == 0 {
            continue;
        }
        let mut r = stream(seed, &[purpose::THREATS, s.id as u64, day]);
        if s.food == 0.0 && s.hungry_days == 0 && s.last_band == 0 {
            // First look at them: a store that lasts a good while, more or less.
            s.food = s.people as f32 * r.gen_range(4.0..20.0);
        }
        // A good or bad year for this place, the same all year.
        let mut ry = stream(seed, &[purpose::THREATS, 0x1EA2, s.id as u64, year]);
        let luck: f32 = ry.gen_range(0.55..1.35);
        let grow = match season {
            crate::time::Season::Spring => 0.85,
            crate::time::Season::Summer => 1.25,
            crate::time::Season::Autumn => 1.6,
            crate::time::Season::Winter => 0.25,
        };
        s.food += s.people as f32 * (grow * luck * harvest[k] - 1.0);
        // A failing harvest is blight and rot: it takes from what is stored too.
        if harvest[k] < 1.0 && s.food > 0.0 {
            s.food -= s.food * (1.0 - harvest[k]) * 0.12;
        }
        s.food = s.food.max(-(s.people as f32) * 8.0);
        // C77 soaks: a grudge fades, a point every twenty days (sieges came 9 to 21 times in
        // 600 days when it never did).
        if day.is_multiple_of(20) {
            s.grudge = s.grudge.saturating_sub(1);
        }
        if s.food < 0.0 {
            s.hungry_days += 1;
        } else {
            s.hungry_days = 0;
        }
        if s.hungry_days == 1 {
            w.events.push(
                tick,
                None,
                Severity::Trace,
                EventKind::SiteHungry {
                    site: s.name.clone(),
                },
            );
        }
        let busy = w.bands.iter().any(|b| b.site == s.id);
        let rested = tick.saturating_sub(s.last_band) > 8 * TICKS_PER_DAY || s.last_band == 0;
        // (Not in the first days after the portal: nobody out there knows of the valley yet.)
        if s.grudge >= GRUDGE_HOST && !busy && rested && settled && r.gen_bool(0.25) {
            // C76: a grudge sends a host, fed or hungry.
            set_out.push((s.id, true));
        } else if s.hungry_days >= 2 && !busy && rested && settled && r.gen_bool(0.35) {
            set_out.push((s.id, false));
        }
    }
    // #1282: Hask ("the strong are tested; the broken are let be") does not press a valley
    // that is already failing or collapsed (fate.rs stage >= 2) with more bands. A hungry or
    // aggrieved site still keeps its hunger or grudge and tries again once the valley recovers.
    let home_broken = w.meta.engine == crate::engine::Engine::Relentless
        && w.communities.iter().any(|c| !c.dissolved && c.fate >= 2);
    for (id, host) in set_out {
        if home_broken {
            continue;
        }
        // Only a band that actually sets out starts the wait before the next.
        if spawn_band(w, id, home, host) {
            if let Some(s) = w.overworld.sites.iter_mut().find(|s| s.id == id) {
                s.last_band = tick;
                if host {
                    s.grudge = 0;
                }
            }
        }
    }
    crate::siege::daily(w);
}

fn spawn_band(w: &mut World, site: u32, home: TilePos, host: bool) -> bool {
    use crate::expedition::Edge;
    let tick = w.meta.tick;
    let Some(s) = w.overworld.sites.iter().find(|s| s.id == site).cloned() else {
        return false;
    };
    w.ensure_nav();
    // In from their own side of the valley if there is a way in there, else the nearest side
    // that has one.
    let edge = [s.edge, Edge::North, Edge::East, Edge::South, Edge::West]
        .into_iter()
        .find_map(|e| crate::systems::expedition::edge_tile(w, e, home));
    let Some(edge) = edge else {
        return false;
    };
    let mut r = stream(w.meta.seed, &[purpose::THREATS, 0xBA2D, site as u64, tick]);
    // C76: a host sent over a grudge is bigger than a hungry band.
    let size = if host {
        (s.people / 5).clamp(4, 10)
    } else {
        (s.people / 10).clamp(2, 5)
    };
    // The camp: on the way in, about CAMP_DIST from home, somewhere they can stand.
    let mut camp = edge;
    for k in 0..20 {
        let t = (CAMP_DIST + k) as f32 / edge.dist(home).max(1) as f32;
        let x = home.x + ((edge.x - home.x) as f32 * t.min(1.0)) as i32 + r.gen_range(-3..=3);
        let y = home.y + ((edge.y - home.y) as f32 * t.min(1.0)) as i32 + r.gen_range(-3..=3);
        let p = w
            .grid
            .surface(x.clamp(2, w.grid.width - 3), y.clamp(2, w.grid.height - 3));
        if camp_ok(w, p) {
            camp = p;
            break;
        }
    }
    let id = w.next_band_id;
    w.next_band_id += 1;
    let mut members = Vec::new();
    for k in 0..size as i32 {
        let p = w.grid.surface(
            (edge.x + k % 3 - 1).clamp(2, w.grid.width - 3),
            (edge.y + k / 3).clamp(2, w.grid.height - 3),
        );
        let a = w.spawn_agent(crate::agents::Species::Person, p);
        // Of their settlement's people and species (P3).
        if let Some((sp, people)) = crate::species::of_site(w, site) {
            crate::species::give(w, a, sp, Some(people));
        }
        if let Some(c) = crate::species::culture_of_site(w, site) {
            crate::species::raise_in(w, a, c);
        }
        if let Some(ag) = w.agents.iter_mut().find(|x| x.id == a) {
            ag.band = Some(id);
            ag.needs.hunger = r.gen_range(0.55..0.8);
            // Desperation makes some of them ready to fight for it, and some not.
            let violent = r.gen_range(-0.6..0.8);
            ag.beliefs.hold(
                Topic::Violence,
                violent,
                0.6,
                crate::mind::beliefs::Source::Event(crate::ids::EventId(0)),
                tick,
            );
        }
        members.push(a);
    }
    // What their people remember of the valley: a hamlet whose last people came to them.
    let kin = w
        .history
        .sites
        .iter()
        .find(|h| {
            h.ended
                .as_ref()
                .is_some_and(|(_, why)| why == &format!("the last of them went to {}", s.name))
        })
        .map(|h| h.id);
    if let Some(k) = kin {
        for m in &members {
            if let Some(ag) = w.agents.iter_mut().find(|x| x.id == *m) {
                ag.tales.push(crate::lore::Tale {
                    site: k,
                    claim: crate::lore::Claim::MovedTo(s.name.clone()),
                    teller: None,
                    how: format!("their family's memory in {}", s.name),
                    heard: tick,
                });
            }
        }
    }
    let eid = w.events.push(
        tick,
        Some(edge),
        Severity::Trace,
        EventKind::BandSetOut {
            band: id,
            site: s.name.clone(),
            size,
        },
    );
    // Fix the placeholder source now the event exists.
    for m in &members {
        if let Some(ag) = w.agents.iter_mut().find(|x| x.id == *m) {
            if let Some(b) = ag
                .beliefs
                .list
                .iter_mut()
                .find(|b| b.topic == Topic::Violence)
            {
                b.sources = vec![crate::mind::beliefs::Source::Event(eid)];
            }
        }
    }
    // A rival of this place comes with them (nemesis.rs).
    crate::nemesis::joins_band(w, site, &members);
    w.bands.push(Band {
        id,
        site,
        site_name: s.name.clone(),
        members,
        size,
        camp,
        state: BandState::Approaching,
        formed: tick,
        want: size * if host { 8 } else { 6 },
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
        heard: word_of(w, home, site, tick),
        host,
        captives: Vec::new(),
    });
    true
}

/// One of the band, not already busy with a raid, goes to look about the houses, a place at
/// a time (a point within eight tiles of home), unless one is on the way already.
fn scout(w: &mut World, k: usize, home: TilePos) {
    let tick = w.meta.tick;
    let looking = w.bands[k].members.iter().any(|m| {
        idx(w, *m).is_some_and(
            |i| matches!(w.agents[i].activity, Activity::Going { to, .. } if to.dist(home) <= 10),
        )
    });
    if looking {
        return;
    }
    let busy: Vec<AgentId> = w.bands[k].raiders.iter().map(|x| x.0).collect();
    let mut r = stream(
        w.meta.seed,
        &[purpose::THREATS, 0x5C07, w.bands[k].id as u64, tick],
    );
    let p = w.grid.surface(
        (home.x + r.gen_range(-8..=8)).clamp(2, w.grid.width - 3),
        (home.y + r.gen_range(-8..=8)).clamp(2, w.grid.height - 3),
    );
    if let Some(i) = w.bands[k]
        .members
        .iter()
        .filter(|m| !busy.contains(m))
        .find_map(|m| idx(w, *m).filter(|i| !w.agents[*i].body.needs_care()))
    {
        go(w, i, p);
    }
}

/// Open ground for a camp: somewhere to stand, not under a roof, no wall within three tiles
/// and nothing the settlement built within four.
fn camp_ok(w: &World, p: TilePos) -> bool {
    if !w.grid.walkable(p) || crate::systems::hazards::indoors(w, p) {
        return false;
    }
    for dy in -3..=3 {
        for dx in -3..=3 {
            let q = p.offset(dx, dy, 0);
            if q.x < 0 || q.y < 0 || q.x >= w.grid.width || q.y >= w.grid.height {
                continue;
            }
            let t = w.grid.get(q);
            use crate::world::Material as M;
            if t.shape == crate::world::Shape::Wall
                && matches!(t.material, M::WOOD | M::STONE_BLOCK | M::THATCH)
            {
                return false;
            }
        }
    }
    !w.structures
        .iter()
        .any(|st| st.pos.z == p.z && st.pos.dist(p) <= 4)
}

/// The settlement built up around a camp: move it to open ground near by.
fn shift_camp(w: &mut World, k: usize) {
    let c = w.bands[k].camp;
    if camp_ok(w, c) {
        return;
    }
    let mut r = stream(
        w.meta.seed,
        &[purpose::THREATS, 0xCA4F, k as u64, w.meta.tick],
    );
    for _ in 0..30 {
        let x = (c.x + r.gen_range(-10..=10)).clamp(2, w.grid.width - 3);
        let y = (c.y + r.gen_range(-10..=10)).clamp(2, w.grid.height - 3);
        let p = w.grid.surface(x, y);
        if camp_ok(w, p) {
            w.bands[k].camp = p;
            return;
        }
    }
}

fn idx(w: &World, id: AgentId) -> Option<usize> {
    w.agents.iter().position(|a| a.id == id && a.alive)
}

/// Set them walking to `to`; false when there is no way there (#197 case 12: callers that
/// must get somewhere need to know).
fn go(w: &mut World, i: usize, to: TilePos) -> bool {
    if w.agents[i].pos == to {
        return true;
    }
    if w.grid.walkable(to) && w.path_agent_to(i, to) {
        w.agents[i].activity = Activity::Going {
            to,
            then: Box::new(Activity::Loitering { ticks_left: 60 }),
        };
        w.agents[i].activity_ticks = 0;
        return true;
    }
    false
}

fn bands_step(w: &mut World) {
    let tick = w.meta.tick;
    let hour = (tick % TICKS_PER_DAY) / TICKS_PER_HOUR;
    // Fog covers them as the dark does (#279).
    let night = !(5..22).contains(&hour) || w.weather.sky == crate::systems::weather::Sky::Fog;
    let Some(home) = home(w) else { return };
    w.ensure_nav();
    let n = w.bands.len();
    // From the last: a band that leaves is removed without shifting the ones still to step
    // (#177: removing it shifted the next into its place, and a `return` skipped the rest).
    for k in (0..n).rev() {
        if k >= w.bands.len() {
            break;
        }
        let band = w.bands[k].clone();
        let alive: Vec<AgentId> = band
            .members
            .iter()
            .copied()
            .filter(|m| idx(w, *m).is_some())
            .collect();
        let lost_now = band.size - alive.len() as u32;
        // (While leaving, those who have walked off the map are not lost.)
        if band.state != BandState::Leaving && lost_now > w.bands[k].lost {
            w.bands[k].lost = lost_now;
        }
        if alive.is_empty() {
            w.bands[k].state = BandState::Leaving;
        }
        // Pursuits: a settler who reaches a raider has it out with them.
        resolve_pursuits(w, k);
        let band = w.bands[k].clone();
        match band.state {
            BandState::Approaching => {
                let mut there = 0;
                for m in &alive {
                    let Some(i) = idx(w, *m) else { continue };
                    if w.agents[i].pos.dist(band.camp) <= 4 {
                        there += 1;
                    } else if !matches!(w.agents[i].activity, Activity::Going { to, .. } if to.dist(band.camp) <= 3)
                    {
                        let c = band.camp;
                        go(w, i, c);
                    }
                }
                if there * 2 >= alive.len().max(1) {
                    w.bands[k].state = BandState::Camped(tick);
                    if w.bands[k].host {
                        let (site, size, camp) = (
                            w.bands[k].site_name.clone(),
                            w.bands[k].size,
                            w.bands[k].camp,
                        );
                        w.events.push(
                            tick,
                            Some(camp),
                            Severity::Major,
                            EventKind::Besieged { site, size },
                        );
                    }
                }
            }
            BandState::Camped(since) => {
                let (mem, site_id) = (w.bands[k].members.clone(), w.bands[k].site);
                crate::nemesis::seek(w, &mem, site_id);
                shift_camp(w, k);
                sightings(w, k);
                envoys(w, k);
                share_out(w, k);
                raids(w, k, night, home);
                if w.bands[k].host {
                    crate::siege::besiege(w, k, since);
                    crate::siege::batter(w, k, since);
                }
                let band = &w.bands[k];
                let fed = band.taken >= band.want;
                let broken = band.lost * 2 >= band.size;
                let tired =
                    tick.saturating_sub(since) > if band.host { 10 } else { 6 } * TICKS_PER_DAY;
                // C76: a host lifts its siege when the wall holds and the stores stay out of
                // reach: three days camped with less than half of what it came for.
                let lifted = band.host && crate::siege::lifts(w, k, since);
                if fed || broken || tired || lifted {
                    if w.bands[k].host {
                        // How it ended, on record (#402): sated before driven off before the
                        // wall before tired.
                        use crate::siege::Ending;
                        let how = if fed {
                            Ending::Sated
                        } else if broken {
                            Ending::DrivenOff
                        } else if lifted {
                            Ending::WallHeld
                        } else {
                            Ending::Tired
                        };
                        crate::siege::ended(w, k, since, how);
                    }
                    w.bands[k].state = BandState::Leaving;
                    // Whatever is in their hands goes with them; only what a raider was
                    // carrying off counts as taken from the stores (the rest they foraged).
                    let mine: Vec<AgentId> = w.bands[k].members.clone();
                    let raiders: Vec<AgentId> = w.bands[k].raiders.iter().map(|x| x.0).collect();
                    let in_hand: Vec<(crate::ids::ItemId, u32, bool)> = w
                        .items
                        .iter()
                        .filter(|it| it.kind.is_food() || it.story.is_some())
                        .filter_map(|it| match it.place {
                            ItemPlace::Carried(c) if mine.contains(&c) => {
                                Some((it.id, it.count, raiders.contains(&c) && it.kind.is_food()))
                            }
                            _ => None,
                        })
                        .collect();
                    for (item, n, raided) in in_hand {
                        if raided {
                            w.bands[k].taken += n;
                        }
                        w.consume_item(item, n, if raided { "raided" } else { "carried off" });
                    }
                }
            }
            BandState::Leaving => {
                let site = w
                    .overworld
                    .sites
                    .iter()
                    .find(|s| s.id == band.site)
                    .cloned();
                let edge = site
                    .as_ref()
                    .and_then(|s| crate::systems::expedition::edge_tile(w, s.edge, band.camp));
                let mut gone = Vec::new();
                for m in &alive {
                    let Some(i) = idx(w, *m) else { continue };
                    match edge {
                        Some(e) if w.agents[i].pos.dist(e) > 2 => {
                            // One with no way to the edge (walled in, the way dug out) goes as
                            // one with no edge does, off the map from where they stand; else
                            // they stood forever, the band was never removed, and `busy` kept its
                            // site from sending another (#197 case 12, FIXLOG 2026-11-12).
                            if !matches!(w.agents[i].activity, Activity::Going { to, .. } if to == e)
                                && !go(w, i, e)
                            {
                                gone.push(*m);
                            }
                        }
                        _ => gone.push(*m),
                    }
                }
                // Off the map and home: out of the world, with whatever is in their hands.
                for m in gone {
                    if let Some(i) = idx(w, m) {
                        crate::systems::society::leave_valley(w, i);
                    }
                }
                let still: usize = band
                    .members
                    .iter()
                    .filter(|m| idx(w, **m).is_some())
                    .count();
                if still == 0 {
                    let b = w.bands.remove(k);
                    if let Some(s) = w.overworld.sites.iter_mut().find(|s| s.id == b.site) {
                        s.food += b.taken as f32;
                    }
                    w.events.push(
                        tick,
                        Some(b.camp),
                        Severity::Notable,
                        EventKind::BandLeft {
                            band: b.id,
                            site: b.site_name.clone(),
                            took: b.taken,
                            lost: b.lost,
                            peaceful: b.welcomed && b.lost == 0,
                        },
                    );
                    continue;
                }
            }
        }
    }
}

/// A member at the camp who is going hungry eats from what the band has taken.
fn share_out(w: &mut World, k: usize) {
    let camp = w.bands[k].camp;
    let members = w.bands[k].members.clone();
    let unit = 2.0 / crate::mind::needs::HUNGER_HOURS;
    for m in members {
        let Some(i) = idx(w, m) else { continue };
        if w.agents[i].pos.dist(camp) > 5 {
            continue;
        }
        while w.agents[i].needs.hunger < 0.35 && w.bands[k].taken > w.bands[k].eaten {
            w.bands[k].eaten += 1;
            w.agents[i].needs.hunger = (w.agents[i].needs.hunger + unit).min(1.0);
            w.agents[i].last_meal = Some(w.meta.tick);
        }
    }
}

/// Anyone of the settlement who is awake and within twelve tiles of a camped band sees them.
fn sightings(w: &mut World, k: usize) {
    if w.bands[k].sighted {
        return;
    }
    let tick = w.meta.tick;
    let members = w.bands[k].members.clone();
    for m in &members {
        let Some(i) = idx(w, *m) else { continue };
        let p = w.agents[i].pos;
        let seer = w
            .agents
            .iter()
            .find(|o| {
                o.alive
                    && o.community.is_some()
                    && o.band.is_none()
                    && o.pos.z == p.z
                    && o.pos.dist(p) <= 12
                    && !o.activity.is_asleep()
            })
            .map(|o| o.id);
        if let Some(s) = seer {
            w.bands[k].sighted = true;
            crate::systems::arms::threatened(w, p);
            let (id, size) = (w.bands[k].id, w.bands[k].size);
            w.events.push(
                tick,
                Some(p),
                Severity::Major,
                EventKind::StrangersSighted {
                    agent: s,
                    band: id,
                    size,
                    at: p,
                },
            );
            return;
        }
    }
}

/// At night (or openly by day once welcomed), up to two of them go for food in the stores and
/// carry it back to the camp.
fn raids(w: &mut World, k: usize, night: bool, home: TilePos) {
    let tick = w.meta.tick;
    // By day and by night they keep their eyes open.
    look_about(w, k, home);
    let band = w.bands[k].clone();
    let open = band.welcomed;
    // Raiders who have reached their food take it; those back at camp with food hand it over.
    let mut still_out = Vec::new();
    for (r, item) in band.raiders.iter().copied() {
        let Some(i) = idx(w, r) else { continue };
        let carrying = w.items.iter().position(|it| {
            it.place == ItemPlace::Carried(r) && (it.kind.is_food() || it.story.is_some())
        });
        if let Some(ci) = carrying {
            if w.agents[i].pos.dist(band.camp) <= 3 {
                let it = w.items[ci].clone();
                // Into the band's stores and out of the valley's: through the ledger.
                w.consume_item(it.id, it.count, "raided");
                if !it.kind.is_food() {
                    // A treasure carried off is a loss the valley names (#173).
                    let what = it
                        .story
                        .as_ref()
                        .and_then(|s| s.name.clone())
                        .unwrap_or_else(|| crate::legend::noun(it.kind, it.material));
                    w.events.push(
                        tick,
                        Some(band.camp),
                        Severity::Major,
                        EventKind::Raided {
                            band: band.id,
                            what,
                            count: it.count,
                            open: band.welcomed,
                        },
                    );
                    continue;
                }
                w.bands[k].taken += it.count;
                crate::systems::arms::threatened(w, band.camp);
                let first = w.bands[k].night_taken == 0;
                w.bands[k].night_taken += it.count;
                // Recorded where it was taken (#210): the hardship falls on the community
                // whose store it was, not the one nearest the strangers' camp.
                let taken_at = w.agents[i].pos;
                w.events.push(
                    tick,
                    Some(taken_at),
                    if first {
                        Severity::Notable
                    } else {
                        Severity::Minor
                    },
                    EventKind::Raided {
                        band: band.id,
                        what: it.kind.label().to_string(),
                        count: it.count,
                        open: band.welcomed,
                    },
                );
                continue;
            }
            still_out.push((r, item));
            if !matches!(w.agents[i].activity, Activity::Going { to, .. } if to == band.camp) {
                go(w, i, band.camp);
            }
            continue;
        }
        // Not carrying yet: at the food, take up to four. A treasure they came for is looked
        // for where the telling put it; if it is not there, they do not know where it is.
        if let Some(&(_, told)) = band.heard.iter().find(|h| h.0 == item) {
            let there = w
                .items
                .iter()
                .any(|it| it.id == item && it.place == ItemPlace::Ground(told));
            if !there {
                if w.agents[i].pos.dist(told) <= 1 {
                    w.bands[k].heard.retain(|h| h.0 != item);
                    continue;
                }
                if !matches!(w.agents[i].activity, Activity::Going { to, .. } if to == told) {
                    go(w, i, told);
                }
                still_out.push((r, item));
                continue;
            }
        }
        let target = w.items.iter().position(|it| it.id == item);
        if let Some(ti) = target {
            {
                let ItemPlace::Ground(p) = w.items[ti].place else {
                    continue;
                };
                if w.agents[i].pos.dist(p) <= 1 {
                    let take = w.items[ti].count.min(4);
                    if take == w.items[ti].count {
                        w.items[ti].place = ItemPlace::Carried(r);
                        w.bands[k].heard.retain(|h| h.0 != item);
                    } else {
                        // What they split off is what the stack was (#173: it became plain
                        // food of middling quality).
                        w.items[ti].count -= take;
                        let mut part = w.items[ti].clone();
                        part.id = w.alloc_item_id();
                        part.count = take;
                        part.place = ItemPlace::Carried(r);
                        part.story = None;
                        w.items.push(part);
                    }
                    let c = band.camp;
                    go(w, i, c);
                    still_out.push((r, item));
                } else {
                    if !matches!(w.agents[i].activity, Activity::Going { to, .. } if to == p) {
                        go(w, i, p);
                    }
                    still_out.push((r, item));
                }
            }
        }
    }
    w.bands[k].raiders = still_out;
    // A raider moving among the houses at night may wake someone.
    let mut rw = stream(
        w.meta.seed,
        &[purpose::THREATS, 0x3A4E, band.id as u64, tick],
    );
    let out: Vec<TilePos> = w.bands[k]
        .raiders
        .iter()
        .filter_map(|(r, _)| idx(w, *r).map(|i| w.agents[i].pos))
        .collect();
    for p in out {
        let woken = w.agents.iter().position(|o| {
            o.alive
                && o.community.is_some()
                && o.activity.is_asleep()
                && o.pos.z == p.z
                && o.pos.dist(p) <= 4
        });
        if let Some(j) = woken {
            if rw.gen_bool(0.25) {
                w.agents[j].activity = Activity::Idle;
                w.agents[j].activity_ticks = 0;
                w.agents[j].idle_ticks = 0;
            }
        }
    }
    // Send more out when it is the time for it, until they have what they can carry off.
    let key = (tick + 2 * TICKS_PER_HOUR) / TICKS_PER_DAY;
    if w.bands[k].night != key {
        w.bands[k].night = key;
        w.bands[k].night_taken = 0;
    }
    let in_hand = 4 * w.bands[k].raiders.len() as u32;
    // By day, a band whose seen places have all gone empty sends one of its own to look about
    // the houses (C71: the hauls move the food into the pile beside home within a day, and a
    // band that looked only by night, in the dark, camped ten days without a raid).
    let known_valid = w.bands[k].seen.iter().any(|(id, p)| {
        w.items
            .iter()
            .any(|it| it.id == *id && it.place == ItemPlace::Ground(*p))
    });
    if !night && !open && !known_valid {
        scout(w, k, home);
    }
    if !(night || open)
        || w.bands[k].raiders.len() >= 2
        || w.bands[k].night_taken + in_hand >= 3 * w.bands[k].size
    {
        return;
    }
    let busy: Vec<AgentId> = w.bands[k].raiders.iter().map(|x| x.0).collect();
    let going_for: Vec<crate::ids::ItemId> = w.bands[k].raiders.iter().map(|x| x.1).collect();
    // What they know of (#173): food they have seen still where they saw it, not leased, not
    // behind a shelter's barred door (C58); and first, a treasure they heard of.
    let free = |w: &World, id: crate::ids::ItemId| {
        w.reservations
            .holder(crate::work::ResKey::Item(id))
            .is_none()
    };
    // A treasure they heard of (#232): never by guests who were told they may eat, and not
    // behind a shelter's barred door, the same rule as the food.
    let mut targets: Vec<(crate::ids::ItemId, TilePos)> = w.bands[k]
        .heard
        .iter()
        .copied()
        .filter(|(id, p)| {
            !open
                && !going_for.contains(id)
                && free(w, *id)
                && !crate::systems::settlement::in_shelter(w, *p)
                && (!night || !crate::systems::settlement::in_walls(w, *p))
        })
        .collect();
    let food: Vec<(crate::ids::ItemId, TilePos)> = w.bands[k]
        .seen
        .iter()
        .copied()
        .filter(|(id, p)| {
            !going_for.contains(id)
                && free(w, *id)
                && !crate::systems::settlement::in_shelter(w, *p)
                // Behind a wall that stands whole, with its gate barred at night (C73).
                && (!night || !crate::systems::settlement::in_walls(w, *p))
                && w.items
                    .iter()
                    .any(|it| it.id == *id && it.place == ItemPlace::Ground(*p))
        })
        .collect();
    targets.truncate(1);
    if targets.is_empty() && food.is_empty() {
        // Nothing known: one of them goes to look about the houses, a place at a time.
        scout(w, k, home);
        return;
    }
    let members = w.bands[k].members.clone();
    for m in members {
        if w.bands[k].raiders.len() >= 2 {
            break;
        }
        if busy.contains(&m) {
            continue;
        }
        let Some(i) = idx(w, m) else { continue };
        if w.agents[i].body.needs_care() {
            continue;
        }
        let me = w.agents[i].pos;
        let taken: Vec<crate::ids::ItemId> = w.bands[k].raiders.iter().map(|x| x.1).collect();
        let pick = targets
            .iter()
            .find(|(id, _)| !taken.contains(id))
            .or_else(|| {
                food.iter()
                    .filter(|(id, _)| !taken.contains(id))
                    .min_by_key(|(_, p)| p.dist(me))
            });
        let Some(&(item, p)) = pick else {
            continue;
        };
        go(w, i, p);
        w.bands[k].raiders.push((m, item));
    }
}

/// A settler who has caught up with a raider: they have it out. Whether it comes to blows
/// depends on what each of them holds about force; a raider who will not fight runs, and a
/// raider who is hurt turns back for home.
fn resolve_pursuits(w: &mut World, k: usize) {
    let tick = w.meta.tick;
    let pursuits = std::mem::take(&mut w.bands[k].pursuits);
    let mut keep = Vec::new();
    for (s, r, since) in pursuits {
        let (Some(si), Some(ri)) = (idx(w, s), idx(w, r)) else {
            continue;
        };
        if tick.saturating_sub(since) > 2 * TICKS_PER_HOUR {
            continue; // lost them
        }
        let (sp, rp) = (w.agents[si].pos, w.agents[ri].pos);
        let key = confront_key(w.bands[k].id);
        w.agents[si]
            .work_cooldowns
            .insert(key, tick + CONFRONT_REST);
        if sp.dist(rp) > 1 {
            go(w, si, rp);
            keep.push((s, r, since));
            continue;
        }
        let mut rng = stream(
            w.meta.seed,
            &[purpose::THREATS, 0xF167, s.0 as u64, r.0 as u64, tick],
        );
        let raider_fights = w.agents[ri].beliefs.lean(Topic::Violence) > -0.2 && rng.gen_bool(0.7);
        // Food in a raider's hands is dropped where they stand.
        for it in w.items.iter_mut() {
            if it.place == ItemPlace::Carried(r) {
                it.place = ItemPlace::Ground(rp);
            }
        }
        w.bands[k].raiders.retain(|x| x.0 != r);
        if raider_fights {
            crate::systems::arms::threatened(w, sp);
            w.agents[si].fought_at = tick;
            w.agents[ri].fought_at = tick;
            // `y` strikes at `x` (systems/combat.rs: a spear lands more often and cuts deeper,
            // armour turns some blows and blunts the rest).
            for (x, y) in [(si, ri), (ri, si)] {
                let other = w.agents[y].id;
                w.agents[x].relate(other, -0.5, 0.2, tick);
                crate::systems::combat::blow(w, y, x, &mut rng, "in a fight with a stranger");
            }
            // C75: neighbours with the nerve for it come to the fight and strike too.
            let helpers: Vec<usize> = (0..w.agents.len())
                .filter(|&k| {
                    k != si
                        && w.agents[k].community.is_some()
                        && w.agents[k].community == w.agents[si].community
                        && w.agents[k].pos.z == rp.z
                        && w.agents[k].pos.dist(rp) <= 8
                        && crate::systems::combat::can_stand(w, k)
                        && crate::systems::combat::nerve(w, k) >= 0.5
                })
                .collect();
            for k in helpers {
                if !w.agents[ri].alive || w.agents[ri].body.needs_care() {
                    break;
                }
                w.agents[k].fought_at = tick;
                w.agents[k].relate(r, -0.5, 0.2, tick);
                crate::systems::combat::blow(w, k, ri, &mut rng, "in a fight with a stranger");
            }
            w.events.push(
                tick,
                Some(rp),
                Severity::Major,
                EventKind::Fought { a: s, b: r },
            );
            // A stranger who fought and lives may come back for them (nemesis.rs).
            let site_id = w.bands[k].site;
            crate::nemesis::fought(w, si, ri, site_id);
        }
        // Either way the raider goes back to camp; a hurt one heads home.
        let hurt = w.agents[ri].body.needs_care();
        if hurt {
            // C76: their people will remember it.
            let site_id = w.bands[k].site;
            if let Some(st) = w.overworld.sites.iter_mut().find(|x| x.id == site_id) {
                st.grudge += 1;
            }
            if crate::siege::keep(w, k, si, ri, &mut rng) {
                continue;
            }
        }
        if hurt {
            w.bands[k].lost += 1;
            w.bands[k].members.retain(|m| *m != r);
            w.agents[ri].band = None;
            // Leaving on their own now: society::leavers takes them off the map at the edge,
            // and nobody judges a leaver as a newcomer on the way.
            w.agents[ri].leaving = true;
            // Gone back the way they came.
            let site_edge = w
                .overworld
                .sites
                .iter()
                .find(|x| x.id == w.bands[k].site)
                .map(|x| x.edge);
            if let Some(e) = site_edge.and_then(|e| crate::systems::expedition::edge_tile(w, e, rp))
            {
                go(w, ri, e);
            }
        } else {
            let c = w.bands[k].camp;
            go(w, ri, c);
        }
    }
    w.bands[k].pursuits.extend(keep);
    rout(w, k);
}

/// C75: a band that has lost half its number to hurts loses its nerve and goes home, all of
/// it, whatever it came for.
fn rout(w: &mut World, k: usize) {
    let b = &w.bands[k];
    if matches!(b.state, BandState::Leaving) || b.lost == 0 || b.lost * 2 < b.size.max(1) {
        return;
    }
    let (site, lost, tick) = (b.site_name.clone(), b.lost, w.meta.tick);
    let at = b.camp;
    w.bands[k].state = BandState::Leaving;
    w.bands[k].pursuits.clear();
    w.bands[k].raiders.clear();
    let site_id = w.bands[k].site;
    if let Some(st) = w.overworld.sites.iter_mut().find(|x| x.id == site_id) {
        st.grudge += 1;
    }
    w.events.push(
        tick,
        Some(at),
        Severity::Major,
        EventKind::Routed { site, lost },
    );
}

/// A settler sets out after one of a band (decide.rs Confront).
pub fn pursue(w: &mut World, settler: AgentId, raider: AgentId) {
    let tick = w.meta.tick;
    let Some(b) = w
        .agents
        .iter()
        .find(|a| a.id == raider)
        .and_then(|a| a.band)
    else {
        return;
    };
    if let Some(band) = w.bands.iter_mut().find(|x| x.id == b) {
        if !band.pursuits.iter().any(|p| p.0 == settler) {
            band.pursuits.push((settler, raider, tick));
        }
    }
}

/// A settler sets out for the camp to tell them they may eat (decide.rs Welcome).
pub fn send_envoy(w: &mut World, settler: AgentId, band: u32) {
    if let Some(b) = w.bands.iter_mut().find(|x| x.id == band) {
        if !b.envoys.contains(&settler) {
            b.envoys.push(settler);
        }
    }
}

/// An envoy who reaches the camp says it.
fn envoys(w: &mut World, k: usize) {
    let camp = w.bands[k].camp;
    let id = w.bands[k].id;
    let list = w.bands[k].envoys.clone();
    let mut keep = Vec::new();
    for e in list {
        let Some(i) = idx(w, e) else { continue };
        if w.agents[i].pos.dist(camp) <= 4 {
            welcome(w, e, id);
            // Those who are welcomed think better of the one who came.
            let members = w.bands[k].members.clone();
            let tick = w.meta.tick;
            for m in members {
                if let Some(j) = idx(w, m) {
                    w.agents[j].relate(e, 0.4, 0.3, tick);
                    w.agents[i].relate(m, 0.2, 0.2, tick);
                    // Talk at the fire: what each side knows of the old valley may pass.
                    crate::lore::on_talk(w, i, j);
                }
            }
        } else if matches!(w.agents[i].activity, crate::mind::Activity::Going { to, .. } if to.dist(camp) <= 4)
        {
            keep.push(e);
        }
    }
    w.bands[k].envoys = keep;
}

/// A settler tells the band they may eat.
pub fn welcome(w: &mut World, settler: AgentId, band: u32) {
    let tick = w.meta.tick;
    if let Some(b) = w.bands.iter_mut().find(|x| x.id == band) {
        if b.welcomed {
            return;
        }
        b.welcomed = true;
        let at = b.camp;
        w.events.push(
            tick,
            Some(at),
            Severity::Major,
            EventKind::Welcomed {
                agent: settler,
                band,
            },
        );
    }
}

/// Is this agent one of a band that is still here?
pub fn band_of(w: &World, id: AgentId) -> Option<&Band> {
    let b = w.agents.iter().find(|a| a.id == id)?.band?;
    w.bands.iter().find(|x| x.id == b)
}

/// Which way `to` lies from `from`, as people say it ("north-east").
pub fn compass(from: TilePos, to: TilePos) -> &'static str {
    let (dx, dy) = ((to.x - from.x) as f32, (from.y - to.y) as f32);
    if dx.abs() < 1.0 && dy.abs() < 1.0 {
        return "here";
    }
    let a = dy.d_atan2(dx).to_degrees().rem_euclid(360.0);
    const NAMES: [&str; 8] = [
        "east",
        "north-east",
        "north",
        "north-west",
        "west",
        "south-west",
        "south",
        "south-east",
    ];
    NAMES[(((a + 22.5) / 45.0) as usize) % 8]
}

/// One line for the screen while a band is a known problem (sighted), else None.
pub fn status_line(w: &World, b: &Band) -> Option<String> {
    if !b.sighted {
        return None;
    }
    let here = b.members.iter().filter(|m| idx(w, **m).is_some()).count();
    let dir = home(w).map(|h| compass(h, b.camp)).unwrap_or("near");
    Some(match b.state {
        BandState::Leaving => format!("The strangers from {} are leaving.", b.site_name),
        _ => {
            let mut s = format!(
                "Strangers from {} are camped to the {dir}, {here} of them",
                b.site_name
            );
            if b.welcomed {
                s.push_str(", told they may eat");
            } else if b.taken > 0 {
                s.push_str(&format!(", and have taken {} from the stores", b.taken));
            }
            s.push('.');
            s
        }
    })
}
