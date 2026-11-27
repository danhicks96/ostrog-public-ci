use crate::{
    agents::{Agent, Species},
    expedition::{self, route_level as level, Caravan, CaravanState, Purpose},
    items::{ItemKind, ItemPlace},
    mind::Skill,
    time::Calendar,
    view::known,
    world::gen::watershed::SpillEdge,
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
    /// Moving water faces observed on this survey from the bank or both floors. No occupants or lower-floor
    /// contents are exposed when a moving surface can be seen from the bank.
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub bank_spills: BTreeSet<SpillEdge>,
    pub regions: BTreeMap<usize, RegionMemory>,
    pub trails: BTreeMap<(u32, i32, i32), (u64, i32)>,
}
impl Survey {
    /// A surveyed voxel, or the material seen immediately under a reported voxel.
    /// The newer report wins when a later witness sees the supporting material change.
    pub fn reported_tile(&self, p: crate::ids::TilePos) -> Option<crate::world::Tile> {
        use crate::world::{Material, Tile, TileFlags};
        let direct = self.valley.get(&(p.x, p.y, p.z));
        let above = self.valley.get(&(p.x, p.y, p.z + 1));
        if let Some(&(_, packed, _, _)) = direct
            .filter(|d| above.is_none_or(|u| d.0 >= u.0 || Tile::unpack(d.1).material.0 == u.2))
        {
            return Some(Tile::unpack(packed));
        }
        above.map(|&(_, _, mat, _)| match Material(mat) {
            Material::AIR => Tile::AIR,
            Material::WATER => {
                let mut tile = Tile::floor(Material::WATER);
                tile.flags = TileFlags(TileFlags::WATER);
                tile
            }
            material => Tile::floor(material),
        })
    }

    /// Relief comes from observed faces and their supports. A wall built on a different
    /// floor cannot raise the colony's mapped terrain until someone sees that level.
    pub fn reported_heights(&self, width: usize, height: usize, unknown: i32) -> Vec<i32> {
        let mut heights = vec![unknown; width * height];
        for (&(x, y, z), report) in &self.valley {
            if x < 0 || y < 0 || x as usize >= width || y as usize >= height {
                continue;
            }
            if let Some(level) = self.measured_height(x, y, z, report) {
                let cell = &mut heights[y as usize * width + x as usize];
                *cell = (*cell).max(level);
            }
        }
        heights
    }

    pub fn reported_height(&self, x: i32, y: i32) -> Option<i32> {
        self.valley
            .range((x, y, i32::MIN)..=(x, y, i32::MAX))
            .filter_map(|(&(x, y, z), report)| self.measured_height(x, y, z, report))
            .max()
    }

    fn measured_height(
        &self,
        x: i32,
        y: i32,
        z: i32,
        &(seen, packed, support, _): &ObservedTile,
    ) -> Option<i32> {
        use crate::world::{Material, Shape, Tile};
        let t = Tile::unpack(packed);
        if t.shape == Shape::Wall
            && self
                .valley
                .get(&(x, y, z + 1))
                .is_none_or(|upper| upper.0 <= seen || upper.2 == t.material.0)
        {
            return Some(z);
        }
        if support != Material::AIR.0
            && support != Material::WATER.0
            && self
                .valley
                .get(&(x, y, z - 1))
                .is_none_or(|lower| lower.0 < seen || Tile::unpack(lower.1).shape == Shape::Wall)
        {
            return Some(z - 1);
        }
        None
    }

    pub fn sees_spill(&self, edge: SpillEdge, view_z: i32) -> bool {
        let current = |p: crate::ids::TilePos| {
            self.valley
                .get(&(p.x, p.y, p.z))
                .is_some_and(|row| row.0 == self.tick)
        };
        (view_z == edge.upper.z || view_z == edge.lower.z)
            && current(edge.upper)
            && (current(edge.lower) || (view_z == edge.upper.z && self.bank_spills.contains(&edge)))
    }
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
    let spills: Vec<_> = w.active_river_spills().collect();
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
        let mut bank_spills = BTreeSet::new();
        for a in w.agents.iter().filter(|a| {
            a.alive
                && !a.activity.is_asleep()
                && a.community == Some(cid)
                && a.species == Species::Person
                && !known::is_unfound(w, a.id)
        }) {
            let radius = crate::systems::perceive::sight_radius(w, a, tick);
            for &edge in &spills {
                if a.pos.z == edge.upper.z
                    && a.pos.dist(edge.lower) <= radius
                    && a.pos.dist(edge.upper) <= radius
                    && !bank_spills.contains(&edge)
                    && crate::mind::perception::sees_spill_from_bank(w, a.pos, edge)
                {
                    bank_spills.insert(edge);
                }
            }
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
        bank_spills.extend(
            spills
                .iter()
                .copied()
                .filter(|edge| local.contains(&edge.upper) && local.contains(&edge.lower)),
        );
        survey.bank_spills = bank_spills;
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
