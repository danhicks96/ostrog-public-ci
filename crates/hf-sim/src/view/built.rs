//! Who built what stands here (#292 item 2, WP-08 slice 8.6): "Built by Aelsin, Y1 day 3
//! 14:05", for the block panel, from the chronicle's `built_record`. Said only while something
//! built stands on the tile (a wall, door or floor of `built_of`, or a structure), so a wall
//! since mined out or burned does not keep its builder's name. Read-only.

use crate::chronicle::Chronicle;
use crate::ids::TilePos;
use crate::world_state::World;

pub fn line(w: &World, c: &Chronicle, p: TilePos) -> Option<String> {
    let stands = w.built_of.contains_key(&p) || w.structures.iter().any(|s| s.pos == p);
    if !stands {
        return None;
    }
    let (tick, who) = c.built_at(p).ok()??;
    let name = w
        .agents
        .iter()
        .chain(w.overworld.away.iter())
        .find(|a| a.id == who)
        .map(|a| a.name.clone())
        .unwrap_or_else(|| "someone no longer remembered".into());
    Some(format!(
        "Built by {name}, {}",
        crate::view::history::when(w, tick)
    ))
}
