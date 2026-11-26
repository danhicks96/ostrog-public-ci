//! Opt-in WorldRenderer batch builders (#414, WP-06.2/6.3).
//!
//! The simulation snapshot is read without touching Godot objects. Rayon builds a stable,
//! cullable list of instance transforms; the main thread only copies that list into Godot's
//! `MultiMeshInstance2D` when the experimental project setting is enabled. The established
//! layered GDScript renderer remains the default until the whole view passes the frame gate.

use hf_sim::view::bodies::Bodies;
use hf_sim::{agents::Species, World};
use rayon::prelude::*;

/// Floats returned to the client for each visible agent: id, x/y in tiles, z, species,
/// facing, pose and movement progress.
pub const INSTANCE_STRIDE: usize = 8;
/// Static things: category, id, x, y, z, kind, variant/material, progress/active.
pub const STATIC_STRIDE: usize = 8;
/// Ground tiles: x, y, z, packed tile, column ground z.
pub const GROUND_STRIDE: usize = 5;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Instance {
    pub id: i32,
    pub x: f32,
    pub y: f32,
    pub z: i32,
    pub species: i32,
    pub facing: i32,
    pub pose: i32,
    pub progress: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StaticInstance {
    pub category: i32,
    pub id: i32,
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub kind: i32,
    pub variant_or_material: i32,
    pub progress_or_active: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GroundInstance {
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub packed: i32,
    pub ground_z: i32,
}

/// Build the people/animal render list from one immutable snapshot. `par_iter` is deliberate:
/// the frame path must not make the Godot thread walk the whole agent list serially.
pub fn instances(world: &World, bodies: &Bodies, z: i32) -> Vec<Instance> {
    let mut out: Vec<Instance> = world
        .agents
        .par_iter()
        .filter_map(|a| {
            let body = if a.alive || a.species != Species::Person {
                None
            } else {
                bodies.pose(a.id, world.meta.tick)
            };
            if (!a.alive && body.is_none())
                || a.denned
                || hf_sim::view::known::is_unfound(world, a.id)
                || a.pos.z != z
            {
                return None;
            }
            let row = crate::agent_row(world, a, body);
            let progress = row[14].clamp(0, 1000) as f32 / 1000.0;
            let x = row[12] as f32 + (row[1] - row[12]) as f32 * progress;
            let y = row[13] as f32 + (row[2] - row[13]) as f32 * progress;
            Some(Instance {
                id: row[0],
                x,
                y,
                z: row[3],
                species: row[4],
                facing: row[5],
                pose: row[15],
                progress,
            })
        })
        .collect();
    // Rayon preserves indexed order, but the explicit key keeps the bridge deterministic if the
    // source collection ever changes to a non-indexed iterator.
    out.sort_unstable_by_key(|i| i.id);
    out
}

/// Build static plants, structures and ground items without walking them on the Godot thread.
/// Categories are 0=plant, 1=structure, 2=item; art selection stays in the client so the
/// experimental path can keep the established asset inventory.
pub fn static_instances(world: &World, z: i32) -> Vec<StaticInstance> {
    let mut out: Vec<StaticInstance> = world
        .plants
        .par_iter()
        .filter(|p| p.alive && p.pos.z == z)
        .map(|p| StaticInstance {
            category: 0,
            id: -1,
            x: p.pos.x,
            y: p.pos.y,
            z: p.pos.z,
            kind: p.kind as i32,
            variant_or_material: (p.variant % 1000) as i32,
            progress_or_active: p.growth,
        })
        .chain(
            world
                .structures
                .par_iter()
                .filter(|s| s.pos.z == z)
                .map(|s| StaticInstance {
                    category: 1,
                    id: s.id.0 as i32,
                    x: s.pos.x,
                    y: s.pos.y,
                    z: s.pos.z,
                    kind: s.kind as i32,
                    variant_or_material: s.material.0 as i32,
                    progress_or_active: if s.active { 1.0 } else { s.progress },
                }),
        )
        .chain(world.items.par_iter().filter_map(|it| {
            let hf_sim::items::ItemPlace::Ground(p) = it.place else {
                return None;
            };
            (p.z == z).then_some(StaticInstance {
                category: 2,
                id: it.id.0 as i32,
                x: p.x,
                y: p.y,
                z: p.z,
                kind: it.kind as i32,
                variant_or_material: it.material.0 as i32,
                progress_or_active: it.count as f32,
            })
        }))
        .collect();
    out.sort_unstable_by_key(|i| (i.z, i.y, i.x, i.category, i.id, i.kind));
    out
}

/// Build one deterministic row per tile in a z-level. The packed tile stays integer-valued
/// across the bridge so materials, shape and flags do not lose precision.
pub fn ground_instances(world: &World, z: i32) -> Vec<GroundInstance> {
    let width = world.grid.width;
    let height = world.grid.height;
    (0..height)
        .into_par_iter()
        .flat_map(|y| {
            (0..width)
                .map(|x| {
                    let p = hf_sim::ids::TilePos::new(x, y, z);
                    GroundInstance {
                        x,
                        y,
                        z,
                        packed: world.grid.get(p).pack() as i32,
                        ground_z: world.grid.ground[world.grid.col(x, y)],
                    }
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

#[path = "colony_atlas.rs"]
mod colony_atlas;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instance_stride_is_the_bridge_contract() {
        assert_eq!(INSTANCE_STRIDE, 8);
        let i = Instance {
            id: 7,
            x: 1.25,
            y: 2.5,
            z: 0,
            species: 0,
            facing: 2,
            pose: 1,
            progress: 0.5,
        };
        assert_eq!(i.x + i.y, 3.75);
    }

    #[test]
    fn static_and_ground_strides_are_bridge_contracts() {
        assert_eq!(STATIC_STRIDE, 8);
        assert_eq!(GROUND_STRIDE, 5);
        let s = StaticInstance {
            category: 2,
            id: 7,
            x: 1,
            y: 2,
            z: 3,
            kind: 4,
            variant_or_material: 5,
            progress_or_active: 1.0,
        };
        assert_eq!(s.category + s.id, 9);
    }
}
