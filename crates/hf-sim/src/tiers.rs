//! Stone tiers (#274): the first time someone of a community cuts a stone harder than any it
//! has cut before, it is on record, a new stone to build with; walls dressed from harder stone
//! weather slower (left_sites.rs) and stand longer against a siege (siege.rs `batter`). What a
//! stone is worth to work with is its row of the material table: harder rock takes longer to
//! cut, to dress and to lay (`cut`, `worked`); fine stone (`fine`) makes a house its sleepers
//! are glad of (systems/strain.rs); stone and ore are priced by what they are (trade.rs). The
//! deep is worth reaching: its basalt, gabbro and gneiss are the hardest stone (the slowest
//! to batter down), gabbro and gneiss are fine, its rock takes the best stone edge
//! (`stone_edge`), and gold and gems lie only there (world/gen/ores.rs).

use crate::events::{EventKind, Severity};
use crate::ids::TilePos;
use crate::world::material::MaterialKind;
use crate::world::Material;
use crate::World;

/// A stone worth building fine with: worth twice a plain stone or more (marble, milkstone).
pub const FINE_VALUE: f32 = 2.0;

/// How long cutting the rock at `p` takes, from `base` (a rock of hardness 1.0): stone and
/// ore by their hardness, soil and anything else at the base.
pub fn cut(w: &World, p: TilePos, base: u32) -> u32 {
    if !w.grid.in_bounds(p) {
        return base;
    }
    let m = w.grid.get(p).material;
    (base as f32 * hardness(w, m)).round().max(1.0) as u32
}

/// How hard `m` is to work if it is stone or ore (1.0 otherwise).
pub fn hardness(w: &World, m: Material) -> f32 {
    let d = w.grid.materials.get(m);
    match d.kind {
        MaterialKind::Stone | MaterialKind::Ore => d.hardness.max(0.1),
        _ => 1.0,
    }
}

/// The finest edge a stone tool of `m` takes (#274): softer rock crumbles at the edge, the
/// deep's hard, close-grained rock holds one as fine as copper's. From 0.4 (sandstone, about
/// 0.43) to 0.62 (gabbro); granite about 0.57, basalt 0.61.
pub fn stone_edge(w: &World, m: Material) -> f32 {
    let d = w.grid.materials.get(m);
    if d.kind != MaterialKind::Stone {
        return 0.45;
    }
    (0.25 + 0.2 * d.hardness).clamp(0.4, 0.62)
}

/// Whether `m` is a fine stone (`FINE_VALUE`).
pub fn fine(w: &World, m: Material) -> bool {
    let d = w.grid.materials.get(m);
    d.kind == MaterialKind::Stone && d.value >= FINE_VALUE
}

/// Work on an order whose first stack has just come: dressing, laying or shaping stone takes
/// as long again as the stone is hard. `d` is the order's index.
pub fn worked(w: &mut World, d: usize, material: Material) {
    use crate::items::ItemKind as K;
    let first = w.designations[d].requirements.first().map(|r| r.kind);
    if !matches!(first, Some(K::RoughStone | K::StoneBlock)) {
        return;
    }
    let h = hardness(w, material);
    if w.grid.materials.get(material).kind != MaterialKind::Stone || h == 1.0 {
        return;
    }
    let o = &mut w.designations[d];
    o.work_total = (o.work_total as f32 * h).round().max(1.0) as u32;
    o.work_left = o.work_total;
}

pub fn observe(w: &mut World) {
    let tick = w.meta.tick;
    let cuts: Vec<(
        crate::ids::AgentId,
        crate::world::Material,
        Option<crate::ids::TilePos>,
    )> = w
        .last_events
        .iter()
        .filter_map(|e| match e.kind {
            EventKind::Mined {
                agent,
                rock: Some(r),
                ..
            } => Some((agent, r, e.at)),
            _ => None,
        })
        .collect();
    for (agent, rock, at) in cuts {
        let def = w.grid.materials.get(rock).clone();
        if !matches!(def.kind, crate::world::material::MaterialKind::Stone) {
            continue;
        }
        let Some(cid) = w
            .agents
            .iter()
            .find(|a| a.id == agent)
            .and_then(|a| a.community)
        else {
            continue;
        };
        let hardest = {
            let c = w.communities.iter().find(|c| c.id == cid).unwrap();
            if c.stones.contains(&rock) {
                continue;
            }
            c.stones
                .iter()
                .map(|m| w.grid.materials.get(*m).hardness)
                .fold(0.0, f32::max)
        };
        let first = hardest == 0.0;
        if let Some(c) = w.communities.iter_mut().find(|c| c.id == cid) {
            c.stones.push(rock);
        }
        if !first && def.hardness > hardest {
            w.events.push(
                tick,
                at,
                Severity::Notable,
                EventKind::NewStone {
                    agent,
                    stone: def.name.clone(),
                },
            );
        }
    }
}
