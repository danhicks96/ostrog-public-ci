//! Roofs, support and collapse (issue #48).
//!
//! A roof is a layer over walkable tiles, kept apart from the grid so that it never changes the
//! ground height of a column. A roof tile stands only if a built wall holds it: one within
//! SPAN tiles along its row or column, reached through roofed tiles. A roof that loses its
//! wall (burned, dug out) falls the next hour. A roof also carries the snow on it, and gives
//! way when the load passes what it was built to bear: a well-made roof bears a long snow, a
//! poor one does not. What falls hurts whoever is under it, and leaves the house open until
//! someone mends it (settlement.rs posts the missing roof tiles again).

use crate::events::{EventKind, Severity};
use crate::ids::{AgentId, TilePos};
use crate::time::TICKS_PER_HOUR;
use crate::world::{Material, Shape};
use crate::World;
use serde::{Deserialize, Serialize};

/// How far a roof reaches from the wall that holds it.
pub const SPAN: i32 = 3;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Roof {
    /// THATCH or WOOD (shingles).
    pub material: Material,
    /// 0..1, from the builder's hand.
    pub quality: f32,
    pub built: u64,
    /// Hours of snow lying on it.
    pub snow: f32,
}

impl Roof {
    /// Hours of lying snow it can bear.
    /// Over 80 winters (seeds 1..40, two years each) the most snow that lay at once had median
    /// 15 hours, 90th percentile 30, worst 40 (with the settling below): a roof of quality 0.3
    /// gives way about one winter in ten, one of 0.6 never did.
    pub fn bears(&self) -> f32 {
        18.0 + 40.0 * self.quality
    }
}

pub fn is_built_wall(w: &World, p: TilePos) -> bool {
    let t = w.grid.get(p);
    t.shape == Shape::Wall
        && matches!(
            t.material,
            Material::WOOD | Material::STONE_BLOCK | Material::THATCH
        )
}

/// Would a roof at `p` be held, counting `extra` as roofed too (for planning)?
pub fn held(w: &World, p: TilePos, extra: &dyn Fn(TilePos) -> bool) -> bool {
    for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
        for k in 1..=SPAN {
            let q = p.offset(dx * k, dy * k, 0);
            if q.x < 0 || q.y < 0 || q.x >= w.grid.width || q.y >= w.grid.height {
                break;
            }
            if is_built_wall(w, q) {
                return true;
            }
            if !(w.roofs.contains_key(&q) || extra(q)) {
                break;
            }
        }
    }
    false
}

pub fn supported(w: &World, p: TilePos) -> bool {
    held(w, p, &|_| false)
}

/// Whether an upper floor at `p` is carried by a built wall one level below. Existing upper
/// floor tiles carry the span onward, up to the same limit as a roof. Ground-floor orders use
/// the solid tile immediately below instead. FIXLOG 2026-10-31, #483 (pending checkpoint):
/// this guard prevents work on a floor that would float over an unsupported room.
pub fn floor_supported(w: &World, p: TilePos) -> bool {
    for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
        for k in 1..=SPAN {
            let q = p.offset(dx * k, dy * k, 0);
            if q.x < 0 || q.y < 0 || q.x >= w.grid.width || q.y >= w.grid.height {
                break;
            }
            if is_built_wall(w, q.offset(0, 0, -1)) {
                return true;
            }
            if w.grid.get(q).shape != Shape::Floor {
                break;
            }
        }
    }
    false
}

/// The two shapes after building a flight from `p` to the tile above. Existing opposite
/// connections become `StairsBoth` rather than being erased (FIXLOG 2026-10-31, #483 review).
pub fn built_stair_shapes(w: &World, p: TilePos) -> Option<(Shape, Shape)> {
    let above = p.offset(0, 0, 1);
    if !w.grid.walkable(p) || w.structures.iter().any(|s| s.pos == p || s.pos == above) {
        return None;
    }
    let lower = match w.grid.get(p).shape {
        Shape::Open => Shape::StairsUp,
        Shape::StairsDown => Shape::StairsBoth,
        _ => return None,
    };
    let upper = match w.grid.get(above).shape {
        Shape::Open => Shape::StairsDown,
        Shape::StairsUp => Shape::StairsBoth,
        _ => return None,
    };
    Some((lower, upper))
}

/// Is `p` under a roof (or under rock, underground)?
pub fn roofed(w: &World, p: TilePos) -> bool {
    w.roofs.contains_key(&p) || w.grid.get(p.offset(0, 0, 1)).shape == Shape::Wall
}

/// Hourly: snow builds up and melts; roofs that have lost their wall or bear too much fall.
pub fn step(w: &mut World) {
    let tick = w.meta.tick;
    if tick % TICKS_PER_HOUR != 41 {
        return;
    }
    if !w.roofs.is_empty() {
        step_roofs(w);
    }
    collapse_unsupported_upper(w);
}

fn step_roofs(w: &mut World) {
    let tick = w.meta.tick;
    let snowing = w.weather.sky == crate::systems::weather::Sky::Snow;
    // The thaw is where the roof stands (#268): a roof on the heights keeps its snow longer.
    let valley = w.weather.temperature_c;
    let base = w
        .grid
        .ground_z(w.meta.founding_site.x, w.meta.founding_site.y);
    for (q, r) in w.roofs.iter_mut() {
        let thaw = crate::thermal::lapse_c(valley, base, q.z) > 1.0;
        if snowing {
            // Thatch is steep and sheds a little of it.
            r.snow += if r.material == Material::THATCH {
                0.85
            } else {
                1.0
            };
        } else if thaw {
            r.snow = (r.snow - 2.0).max(0.0);
        } else {
            // Below freezing it still settles, blows off and slides.
            r.snow = (r.snow - 0.25).max(0.0);
        }
    }
    let mut falls: Vec<(TilePos, &'static str)> = Vec::new();
    for (p, r) in &w.roofs {
        if !supported(w, *p) {
            falls.push((*p, "with nothing left to hold it"));
        } else if r.snow > r.bears() {
            falls.push((*p, "under the weight of the snow"));
        }
    }
    if falls.is_empty() {
        return;
    }
    // One event per stretch of roof that came down together.
    let because = falls[0].1;
    let mut hurt: Vec<AgentId> = Vec::new();
    for (p, _) in &falls {
        let m = w
            .roofs
            .remove(p)
            .map(|r| r.material)
            .unwrap_or(Material::WOOD);
        let m = &m;
        let under: Vec<usize> = (0..w.agents.len())
            .filter(|&i| w.agents[i].alive && w.agents[i].pos == *p)
            .collect();
        for i in under {
            let mut r = crate::rng::stream(
                w.meta.seed,
                &[crate::rng::purpose::BUILDING, w.agents[i].id.0 as u64, tick],
            );
            use rand::Rng;
            let part = [crate::body::Part::HEAD, crate::body::Part::TORSO][r.gen_range(0..2)];
            let kind = if r.gen_bool(0.3) {
                crate::body::WoundKind::Break
            } else {
                crate::body::WoundKind::Bruise
            };
            crate::systems::health::wound(
                w,
                i,
                part,
                kind,
                0.3 + r.gen::<f32>() * 0.4,
                0.05,
                "when the roof came down",
                None,
            );
            hurt.push(w.agents[i].id);
        }
        // What it was made of lies where it fell, its wood and its cause recorded (#177).
        w.create_item_of(
            crate::items::ItemKind::Log,
            if *m == Material::THATCH {
                Material::WOOD
            } else {
                *m
            },
            1,
            crate::items::ItemPlace::Ground(*p),
            0.4,
            "roof fell",
        );
    }
    let at = falls[0].0;
    let row = w.events.push(
        tick,
        Some(at),
        Severity::Major,
        EventKind::RoofFell {
            at,
            tiles: falls.len() as u32,
            hurt,
            because: because.to_string(),
        },
    );
    // The rain comes in where it fell (cascade.rs, #319).
    for (p, _) in &falls {
        w.cascades.unroofed.insert(*p, (row, tick));
    }
}

/// Remove a built upper-room component when its last lower support or stair connection is gone.
/// The component is deliberately found from real built tiles, never from a house template, so a
/// fire, battering or another physical removal can leave only the supported pieces behind.
fn collapse_unsupported_upper(w: &mut World) {
    let seeds: Vec<TilePos> = w
        .built_of
        .keys()
        .copied()
        .filter(|p| p.z > w.grid.ground_z(p.x, p.y))
        .filter(|p| match w.grid.get(*p).shape {
            Shape::Floor => !floor_supported(w, *p),
            Shape::StairsDown | Shape::StairsBoth => {
                !w.grid.get(p.offset(0, 0, -1)).shape.connects_up()
            }
            Shape::StairsUp => !w.grid.get(p.offset(0, 0, 1)).shape.connects_down(),
            _ => false,
        })
        .collect();
    let mut visited = std::collections::BTreeSet::new();
    for seed in seeds {
        if visited.contains(&seed) {
            continue;
        }
        let mut component = std::collections::BTreeSet::new();
        let mut pending = vec![seed];
        while let Some(p) = pending.pop() {
            if !is_upper_built_tile(w, p, seed.z) || !component.insert(p) {
                continue;
            }
            for dy in -1..=1 {
                for dx in -1..=1 {
                    if dx != 0 || dy != 0 {
                        pending.push(p.offset(dx, dy, 0));
                    }
                }
            }
        }
        visited.extend(component.iter().copied());
        if !component.is_empty() {
            upper_component_fell(w, &component);
        }
    }
}

fn is_upper_built_tile(w: &World, p: TilePos, level: i32) -> bool {
    p.z == level
        && w.built_of.contains_key(&p)
        && matches!(
            w.grid.get(p).shape,
            Shape::Floor | Shape::Wall | Shape::StairsUp | Shape::StairsDown | Shape::StairsBoth
        )
}

fn upper_component_fell(w: &mut World, component: &std::collections::BTreeSet<TilePos>) {
    let tick = w.meta.tick;
    let at = *component
        .iter()
        .next()
        .expect("upper component is non-empty");
    let caught: Vec<usize> = (0..w.agents.len())
        .filter(|&i| w.agents[i].alive && component.contains(&w.agents[i].pos))
        .collect();
    for p in component {
        w.grid.set(*p, crate::world::Tile::AIR);
        w.built_of.remove(p);
        w.roofs.remove(p);
        for d in w.designations.iter_mut().filter(|d| d.pos == *p) {
            d.done = true;
        }
    }
    let structures: std::collections::BTreeSet<_> = w
        .structures
        .iter()
        .filter(|s| component.contains(&s.pos))
        .map(|s| s.id)
        .collect();
    w.structures.retain(|s| !structures.contains(&s.id));

    let clear: std::collections::BTreeSet<TilePos> = component
        .iter()
        .flat_map(|p| {
            (-3..=3).flat_map(move |dy| {
                (-3..=3).flat_map(move |dx| (-2..=0).map(move |dz| p.offset(dx, dy, dz)))
            })
        })
        .filter(|p| !component.contains(p) && w.grid.walkable(*p))
        .collect();
    let mut hurt = Vec::new();
    for i in caught {
        if !w.grid.walkable(w.agents[i].pos) {
            if let Some(q) = clear
                .iter()
                .copied()
                .min_by_key(|q| (q.dist(w.agents[i].pos), *q))
            {
                w.agents[i].pos = q;
                w.agents[i].from = q;
                w.agents[i].path.clear();
                w.agents[i].move_total = 0;
            }
        }
        hurt.push(w.agents[i].id);
        crate::systems::health::wound(
            w,
            i,
            crate::body::Part::LEG_L,
            crate::body::WoundKind::Bruise,
            0.35,
            0.1,
            "when the upper room came down",
            None,
        );
    }
    w.events.push(
        tick,
        Some(at),
        Severity::Major,
        EventKind::UpperFloorFell {
            at,
            tiles: component.len() as u32,
            hurt,
            because: "when its lower support gave way".into(),
        },
    );
}

/// A save from before roofs: every house whose walls stand gets the roof it would have had.
pub fn roof_old_houses(w: &mut World) {
    let tick = w.meta.tick;
    let mut add = Vec::new();
    for c in &w.communities {
        for origin in &c.house_sites {
            // The enclosed part of each house (C69: a house in the middle of a room being
            // added to has a roofed old part and an unwalled new one, and was given no roof
            // at all): a floor tile every neighbour of which is floor or a standing wall.
            let floor = crate::systems::settlement::house_interior(w, *origin);
            let set: std::collections::BTreeSet<TilePos> = floor.iter().copied().collect();
            let wall = w.grid.get(*origin).material;
            let m = crate::systems::settlement::roof_style(w, c.id, wall);
            for p in floor {
                let enclosed = (-1..=1).all(|dy| {
                    (-1..=1).all(|dx| {
                        let q = p.offset(dx, dy, 0);
                        set.contains(&q) || w.grid.get(q).shape == crate::world::Shape::Wall
                    })
                });
                if enclosed {
                    add.push((p, m));
                }
            }
        }
    }
    for (p, m) in add {
        w.roofs.insert(
            p,
            Roof {
                material: m,
                quality: 0.6,
                built: tick,
                snow: 0.0,
            },
        );
    }
}

/// What a house at `p` has come to, for the screen: "A timber house with thatch: walls 18 of
/// 22, roof 4 of 20." Once roofed, how much snow lies on it and how much it will bear.
pub fn house_note(w: &World, p: TilePos) -> Option<String> {
    for c in &w.communities {
        for o in &c.house_sites {
            let plan = crate::houses::plan(w, *o);
            let ring = crate::houses::walls(&plan.floor);
            if !plan.floor.contains(&p) && !ring.iter().any(|(q, _)| *q == p) {
                continue;
            }
            let mut walls = 0;
            let mut edges = 0;
            let mut stone = false;
            // What most of its walls were built of (C56): "an oak house", "a granite house".
            let mut of: std::collections::BTreeMap<Material, u32> = Default::default();
            for (q, _) in ring {
                edges += 1;
                let t = w.grid.get(q);
                if t.shape == Shape::Wall {
                    walls += 1;
                    stone |= t.material == Material::STONE_BLOCK;
                    if let Some(m) = w.made_of(q) {
                        *of.entry(m).or_default() += 1;
                    }
                }
            }
            let most = of
                .iter()
                .max_by(|a, b| a.1.cmp(b.1).then(b.0.cmp(a.0)))
                .map(|(m, _)| w.grid.materials.get(*m).name.clone());
            let inner = plan.floor;
            let roofs: Vec<&Roof> = inner.iter().filter_map(|q| w.roofs.get(q)).collect();
            let thatch = roofs.iter().any(|r| r.material == Material::THATCH);
            let built = most.unwrap_or_else(|| if stone { "stone" } else { "timber" }.to_string());
            let a = if built.starts_with(['a', 'e', 'i', 'o', 'u']) {
                "An"
            } else {
                "A"
            };
            let kind = match (stone, roofs.is_empty(), thatch) {
                (true, _, _) | (false, true, _) => format!("{a} {built} house"),
                (false, false, true) => format!("{a} {built} house with thatch"),
                (false, false, false) => format!("{a} {built} house with shingles"),
            };
            if walls < edges {
                return Some(format!("{kind} going up: walls {walls} of {edges}."));
            }
            if roofs.len() < inner.len() {
                return Some(format!(
                    "{kind}: walls up, roof {} of {}.",
                    roofs.len(),
                    inner.len()
                ));
            }
            let snow = roofs.iter().map(|r| r.snow).fold(0.0f32, f32::max);
            let bears = roofs.iter().map(|r| r.bears()).fold(f32::MAX, f32::min);
            return Some(if snow >= 1.0 {
                format!(
                    "{kind}, roofed. Snow on it {:.0} hours; its weakest part bears about {:.0}.",
                    snow, bears
                )
            } else {
                format!("{kind}, roofed.")
            });
        }
    }
    None
}
