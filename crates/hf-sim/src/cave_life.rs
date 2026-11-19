//! Cave life (#271): mushrooms spread over dark floors below ground, faster where water stands
//! near, so a cavern found is worth keeping: a crop that wants no sun. Once a day each living
//! patch may seed an open floor tile beside it (a third of the time, two thirds by water),
//! unless eight already grow within six tiles.

use crate::ids::TilePos;
use crate::plants::{Plant, PlantKind};
use crate::rng::{purpose, stream};
use crate::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use crate::World;
use rand::Rng;

pub fn step(w: &mut World) {
    let tick = w.meta.tick;
    if tick % TICKS_PER_DAY != 3 * TICKS_PER_HOUR + 444 {
        return;
    }
    let day = tick / TICKS_PER_DAY;
    let mut r = stream(w.meta.seed, &[purpose::WEATHER, 0x2710, day]);
    let patches: Vec<TilePos> = w
        .plants
        .iter()
        .filter(|p| p.alive && p.kind == PlantKind::Mushroom)
        .map(|p| p.pos)
        .collect();
    if patches.is_empty() {
        return;
    }
    let mut new: Vec<TilePos> = Vec::new();
    for p in &patches {
        // Crowded already: eight or more within six tiles.
        let near = patches
            .iter()
            .chain(new.iter())
            .filter(|q| q.z == p.z && q.dist(*p) <= 6)
            .count();
        if near >= 8 {
            continue;
        }
        let damp = w.grid.caverns.iter().any(|c| c.pool && c.contains(*p))
            || [(1, 0), (-1, 0), (0, 1), (0, -1)]
                .iter()
                .any(|(dx, dy)| w.grid.get(p.offset(*dx, *dy, 0)).is_water());
        if !r.gen_bool(if damp { 0.66 } else { 0.33 }) {
            continue;
        }
        let (dx, dy) = [(1, 0), (-1, 0), (0, 1), (0, -1)][r.gen_range(0..4)];
        let q = p.offset(dx, dy, 0);
        if w.grid.in_bounds(q)
            && w.grid.walkable(q)
            && crate::underground::below(w, q)
            && !w.grid.get(q).is_water()
            && !w.plants.iter().any(|pl| pl.pos == q && pl.alive)
            && !new.contains(&q)
        {
            new.push(q);
        }
    }
    for q in new {
        let v = r.gen::<u32>();
        w.plants.push(Plant::new(q, PlantKind::Mushroom, 0.2, v));
    }
}
