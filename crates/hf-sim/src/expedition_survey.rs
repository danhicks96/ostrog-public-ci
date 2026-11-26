use crate::{
    agents::{Agent, Species},
    expedition::{self, route_level as level, Caravan, CaravanState, Purpose},
    items::{ItemKind, ItemPlace},
    mind::Skill,
    time::Calendar,
    view::known,
    World,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Last observation tick, packed tile, material below, and fitting (or -1).
pub type ObservedTile = (u64, u32, u16, i32);

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct Survey {
    pub tick: u64,
    #[serde(default)]
    pub valley: BTreeMap<(i32, i32, i32), ObservedTile>,
    pub regions: BTreeMap<usize, RegionMemory>,
    pub trails: BTreeMap<(u32, i32, i32), (u64, i32)>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RegionMemory {
    pub tick: u64,
    pub terrain: String,
    pub settlement: String,
}

/// Odd-row offset coordinates keep existing realm region IDs while measuring hex distance.
pub fn footprint(x: i32, y: i32, r: i32) -> impl Iterator<Item = (i32, i32)> {
    let q = x - y.div_euclid(2);
    (-r..=r).flat_map(move |dy| {
        let ny = y + dy;
        ((-r).max(-dy - r)..=r.min(-dy + r)).map(move |dq| (q + dq + ny.div_euclid(2), ny))
    })
}
fn eyes(a: &Agent, light: f32) -> i32 {
    let sight = a.species_def().sight_radius(light);
    if sight == 0 {
        0
    } else {
        sight + (a.skills.level(Skill::Foraging) / 4.0) as i32
    }
}
fn daylight(tick: u64) -> f32 {
    if Calendar::is_night(tick) {
        0.4 + 0.3 * Calendar::moon(tick)
    } else {
        1.0
    }
}

pub fn observe(w: &mut World) {
    let home = w.meta.setup.as_ref().and_then(|s| s.region);
    // Old saves rebuild the stable geography from the world seed once.
    if home.is_some() && w.realm_map.is_none() {
        w.realm_map = Some(crate::realm::generate(w.meta.seed, &Default::default()));
    }
    let n = w.realm_map.as_ref().map_or(0, |m| m.size as i32);
    let tick = w.meta.tick;
    let cal = Calendar::from_tick(tick);
    let light = daylight(tick);
    let torches: BTreeSet<_> = w
        .items
        .iter()
        .filter_map(|i| match i.place {
            ItemPlace::Carried(who) if i.kind == ItemKind::Torch && i.count > 0 => Some(who),
            _ => None,
        })
        .collect();
    for colony in &w.communities {
        let cid = colony.id;
        let mut visible = BTreeSet::new();
        let mut add = |region: usize, eyes: i32| {
            let radius = if eyes > 0 { (eyes / 4).max(1) } else { 0 };
            visible.extend(
                footprint((region as i32) % n, (region as i32) / n, radius)
                    .filter(|&(x, y)| x >= 0 && y >= 0 && x < n && y < n)
                    .map(|(x, y)| (y * n + x) as usize),
            );
        };
        if let (Some(home), Some(range)) = (
            home,
            w.agents
                .iter()
                .filter(|a| {
                    a.alive
                        && !a.activity.is_asleep()
                        && a.community == Some(cid)
                        && a.species == Species::Person
                        && !known::is_unfound(w, a.id)
                        && a.pos.z >= w.grid.surface(a.pos.x, a.pos.y).z
                })
                .map(|a| eyes(a, if torches.contains(&a.id) { 1.0 } else { light }))
                .max(),
        ) {
            add(home, range);
        }
        let mut local = BTreeSet::new();
        for a in w.agents.iter().filter(|a| {
            a.alive
                && !a.activity.is_asleep()
                && a.community == Some(cid)
                && a.species == Species::Person
                && !known::is_unfound(w, a.id)
        }) {
            let radius = crate::systems::perceive::sight_radius(w, a, tick);
            for dy in -radius..=radius {
                for dx in -radius..=radius {
                    let p = a.pos.offset(dx, dy, 0);
                    if w.grid.in_bounds(p)
                        && p.dist(a.pos) <= radius
                        && !local.contains(&p)
                        && crate::mind::perception::line_of_sight(&w.grid, a.pos, p)
                    {
                        local.insert(p);
                    }
                }
            }
        }
        let fittings: BTreeMap<_, _> = w
            .structures
            .iter()
            .filter(|s| local.contains(&s.pos))
            .map(|s| (s.pos, s.kind as i32))
            .collect();
        let mut trail = Vec::new();
        for c in w.overworld.caravans.iter().filter(|c| {
            c.community == cid
                && matches!(
                    c.state,
                    CaravanState::Outbound { .. }
                        | CaravanState::Returning { .. }
                        | CaravanState::Stranded { .. }
                        | CaravanState::AtSite { .. }
                )
        }) {
            let lit = c.cargo.iter().any(|&(k, n)| k == ItemKind::Torch && n > 0);
            let Some(range) = w
                .overworld
                .away
                .iter()
                .filter(|a| {
                    a.alive
                        && !a.activity.is_asleep()
                        && a.community == Some(cid)
                        && c.members.contains(&a.id)
                        && !known::is_unfound(w, a.id)
                })
                .map(|a| eyes(a, if lit { 1.0 } else { light }))
                .max()
            else {
                continue;
            };
            let sid = destination(c);
            let Some(site) = w.overworld.site(sid) else {
                continue;
            };
            let road = expedition::road_distance(c, tick, site.days);
            let seed = level::route_seed(w.meta.seed, site);
            let x = (road / level::TILE) as i32;
            for (x, y) in footprint(x, level::path_y(seed, x), range) {
                let (ground, plant, stock) = level::tile(seed, site, x, y);
                let available = level::available(&w.overworld, sid, x, y, cal, stock);
                trail.push((
                    (sid, x, y),
                    ground | (plant << 4) | ((available as i32) << 8) | ((cal.season as i32) << 12),
                ));
            }
            if let (Some(home), Some(target)) = (
                home,
                site.realm_site
                    .and_then(|id| w.realm.sites.get(id as usize))
                    .map(|s| s.region),
            ) {
                let f = road as f64
                    / (i64::from(site.days).max(1) * expedition::ROAD_UNITS_PER_DAY) as f64;
                let interp =
                    |a: usize, b: usize| (a as f64 + (b as f64 - a as f64) * f).round() as usize;
                let n = n as usize;
                add(
                    interp(home / n, target / n) * n + interp(home % n, target % n),
                    range,
                );
            }
        }
        let survey = w.overworld.surveys.entry(cid).or_default();
        survey.tick = tick;
        for p in local {
            survey.valley.insert(
                (p.x, p.y, p.z),
                (
                    tick,
                    w.grid.get(p).pack(),
                    w.grid.get(p.offset(0, 0, -1)).material.0,
                    *fittings.get(&p).unwrap_or(&-1),
                ),
            );
        }
        for (key, value) in trail {
            survey.trails.insert(key, (tick, value));
        }
        let Some(map) = &w.realm_map else { continue };
        let mut names = BTreeMap::new();
        for site in w
            .realm
            .sites
            .iter()
            .filter(|s| !s.deep && visible.contains(&s.region))
        {
            names.insert(
                site.region,
                if site.ended.is_some() {
                    format!("Ruins of {}", site.name)
                } else {
                    site.name.clone()
                },
            );
        }
        for i in visible {
            survey.regions.insert(
                i,
                RegionMemory {
                    tick,
                    terrain: map.biome[i].label().into(),
                    settlement: names.remove(&i).unwrap_or_default(),
                },
            );
        }
    }
}
pub fn destination(c: &Caravan) -> u32 {
    match c.purpose {
        Purpose::Trade { site, .. } | Purpose::Fetch { site, .. } | Purpose::Scout { site } => site,
    }
}
