//! Plant growth. Striped: each plant advances once per 60 ticks (a game-hour has 20 stripes),
//! so growth is cheap and still smooth at observation scale.

use crate::world_state::World;
use rand::Rng;

pub const STRIPE: u64 = 60;

pub fn step(w: &mut World) {
    let t = w.meta.tick;
    let gf = w.weather.growth_factor;
    let stripe = (t % STRIPE) as usize;
    // Nothing grows in the hollow by the portal stone (thin.rs `hollow`). The stone and the
    // hollow's reach are found once, and only this stripe's plants are asked (#553, FIXLOG
    // 2026-11-01): asking every plant every tick, each finding the stone among all the
    // structures, was 0.072 ms of a 0.239 ms tick on seed 3 over 200 days, the place open.
    let hollow: Vec<bool> = match crate::thin::stone(w) {
        Some(s) if w.thin.strength > 0.3 => {
            let reach = crate::thin::radius(w) * 0.5;
            w.plants
                .iter()
                .enumerate()
                .map(|(i, p)| {
                    i % STRIPE as usize == stripe && p.pos.z == s.z && p.pos.dist(s) as f32 <= reach
                })
                .collect()
        }
        _ => Vec::new(),
    };
    // Crops beside water drink (#325): they grow a third faster, and a drought does not touch
    // them.
    let drought = crate::spells::now(w) == Some(crate::spells::SpellKind::Drought);
    let season = w.calendar().season;
    let crop = |k: crate::plants::PlantKind| k == crate::plants::PlantKind::Root || k.is_crop();
    let watered: Vec<bool> = w
        .plants
        .iter()
        .enumerate()
        .map(|(i, p)| {
            i % STRIPE as usize == stripe
                && p.alive
                && crop(p.kind)
                && crate::systems::plants::watered(w, p.pos)
        })
        .collect();
    // The weeds alive right now: a crop within a step of one is crowded and grows at half
    // rate until it is pulled. Gathered once (weeds are few) so the crop scan below stays
    // cheap (#286).
    let weeds: Vec<crate::ids::TilePos> = w
        .plants
        .iter()
        .filter(|p| p.alive && p.kind.is_weed())
        .map(|p| p.pos)
        .collect();
    // Each crop to its own ground and season (#286): how well it suits the biome it is sown
    // in, rye on into autumn, the thirsty ones slow away from water. A thistle within a step
    // crowds the crop: it grows at half the rate until the weed is pulled.
    let suits: Vec<f32> = w
        .plants
        .iter()
        .enumerate()
        .map(|(i, p)| {
            if i % STRIPE as usize != stripe || !p.alive || !p.kind.is_crop() {
                return 1.0;
            }
            let crowded = weeds.iter().any(|t| t.z == p.pos.z && t.dist(p.pos) <= 1);
            let ground = w
                .grid
                .biome_at(p.pos.x, p.pos.y)
                .map(|b| crate::plants::crop_suits(p.kind, b))
                .unwrap_or(1.0);
            let ground_material = w.grid.get(p.pos.offset(0, 0, -1)).material;
            let material_factor = crate::plants::crop_ground_factor(p.kind, ground_material);
            let thirst = if crate::plants::thirsty(p.kind) && !watered[i] {
                0.5
            } else {
                1.0
            };
            let weed = if crowded { 0.5 } else { 1.0 };
            ground * material_factor * thirst * weed * crate::plants::crop_season(p.kind, season)
        })
        .collect();
    for (i, p) in w.plants.iter_mut().enumerate() {
        if p.alive && i % STRIPE as usize == stripe && !hollow.get(i).copied().unwrap_or(false) {
            let g = if watered[i] {
                (if drought { gf / 0.4 } else { gf }) * 1.3
            } else {
                gf
            };
            p.grow(STRIPE, g * suits[i], season);
        }
    }
    // #286: thistle, the weed of worked ground. A worked field picks it up: each empty field
    // tile of a living community rolls a small chance in every eighth stripe, i.e. about sixty
    // times a day (~0.5 thistles per empty tile per day at this rate), from a stream of its
    // own so the rest of the world is where it was. Left alone it crowds the crops beside it;
    // the planner orders it pulled where it presses them (settlement.rs). The block runs once
    // per stripe, not once per tick: the filter and the roll key both used t / STRIPE, so the
    // sixty ticks of a stripe re-tested the same tiles with the same stream — sixty identical
    // answers and sixty scans of w.plants per tile (coordinator review, #676).
    if t.is_multiple_of(STRIPE) {
        let mut seed_at: Vec<crate::ids::TilePos> = Vec::new();
        for c in &w.communities {
            if c.dissolved {
                continue;
            }
            for (n, f) in c.field_tiles.iter().enumerate() {
                if !(n as u64 + t / STRIPE).is_multiple_of(8) {
                    continue;
                }
                let mut r = crate::rng::stream(
                    w.meta.seed,
                    &[
                        crate::rng::purpose::PLANTS,
                        0x7715,
                        c.id.0 as u64,
                        f.x as u64,
                        f.y as u64,
                        f.z as u64,
                        t / STRIPE,
                    ],
                );
                if !r.gen_bool(0.008) {
                    continue;
                }
                if w.plants.iter().any(|p| p.alive && p.pos == *f) {
                    continue;
                }
                seed_at.push(*f);
            }
        }
        for f in seed_at {
            let variant = (f.x * 31 + f.y * 17) as u32;
            w.plants.push(crate::plants::Plant::new(
                f,
                crate::plants::PlantKind::Thistle,
                0.15,
                variant,
            ));
        }
    }
}

/// Water within three tiles at the same level, or the level below (#325), or reached there by
/// a dug channel (channels.rs).
pub fn watered(w: &World, p: crate::ids::TilePos) -> bool {
    if !w.channels.is_empty() && crate::channels::waters(w, p) {
        return true;
    }
    for dz in [0, -1] {
        for dy in -3..=3 {
            for dx in -3..=3 {
                let q = p.offset(dx, dy, dz);
                if w.grid.in_bounds(q) && w.grid.get(q).is_water() {
                    return true;
                }
            }
        }
    }
    false
}

/// What farmers sow at `pos` (#286): the food crop that does best on its ground (plants.rs
/// `best_crop`), or flax on one tile in six of a field whose people have a loom and no reeds
/// or nettles within reach to cut for it.
pub fn crop_for(w: &World, pos: crate::ids::TilePos) -> crate::plants::PlantKind {
    use crate::plants::{Gives, PlantKind};
    let biome = w
        .grid
        .biome_at(pos.x, pos.y)
        .unwrap_or(crate::world::gen::biome::Biome::Grassland);
    let wet = watered(w, pos);
    if (pos.x * 7 + pos.y * 3).rem_euclid(6) == 0
        && w.structures
            .iter()
            .any(|s| s.kind == crate::structures::StructureKind::Loom && s.pos.dist(pos) <= 45)
        && !w.plants.iter().any(|p| {
            p.alive
                && !p.kind.is_crop()
                && matches!(p.kind.lore().gives, Gives::Fibre(_))
                && p.pos.dist(pos) <= 50
        })
    {
        return PlantKind::Flax;
    }
    crate::plants::best_crop(biome, wet)
}
