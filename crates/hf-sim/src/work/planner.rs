//! Build a concrete plan for one designation from one agent's point of view: what to pick
//! up, where to carry it, where to stand, what to work on. Targets come from the agent's
//! knowledge (items it has seen) and the world's designations (community knowledge, which
//! members share). Returns the plan and the keys it must lease.

use crate::ids::{AgentId, TilePos};
use crate::items::ItemPlace;
use crate::mind::knowledge::Known;
use crate::structures::StructureKind;
use crate::work::{Designation, DesignationKind, Plan, ResKey, Step};
use crate::world_state::World;

pub struct Planned {
    pub plan: Plan,
    pub keys: Vec<ResKey>,
    /// Estimated ticks (for the lease expiry).
    pub estimate: u32,
}

fn stand_tile(w: &World, from: TilePos, target: TilePos) -> Option<TilePos> {
    let g = &w.grid;
    let nav = w.nav.as_ref()?;
    // The target itself only if a way leads there (#553, FIXLOG 2026-11-01): a walkable site
    // with no way to it was taken as the place to stand, so a wall order on a ledge no one could
    // climb to was planned, and failed at the walk, again and again (seed 3: 709 "cannot reach"
    // in 50 days). Unreachable, a tile beside it is looked for as for any solid target.
    if g.walkable(target) && nav.reach(from, target) {
        return Some(target);
    }
    let mut best: Option<(i32, TilePos)> = None;
    for (dx, dy) in [
        (0, 1),
        (1, 0),
        (0, -1),
        (-1, 0),
        (1, 1),
        (-1, 1),
        (1, -1),
        (-1, -1),
    ] {
        let q = target.offset(dx, dy, 0);
        if g.walkable(q) && nav.reach(from, q) {
            let d = from.dist(q);
            if best.map(|b| d < b.0).unwrap_or(true) {
                best = Some((d, q));
            }
        }
    }
    best.map(|b| b.1)
}

/// Nearest known, unreserved ground stack of `kind` reachable from `from`.
fn find_known_item(
    w: &World,
    who: AgentId,
    from: TilePos,
    kind: crate::items::ItemKind,
    want: Option<crate::world::Material>,
    exclude_sites: bool,
) -> Option<(crate::ids::ItemId, TilePos, u32)> {
    let a = w.agents.iter().find(|a| a.id == who)?;
    let nav = w.nav.as_ref()?;
    let mut best: Option<(i32, crate::ids::ItemId, TilePos, u32)> = None;
    for it in &w.items {
        if it.kind != kind || want.is_some_and(|m| it.material != m) {
            continue;
        }
        let ItemPlace::Ground(p) = it.place else {
            continue;
        };
        if a.knowledge.get(Known::Item(it.id)).is_none() {
            continue;
        }
        if !w.reservations.is_free_for(ResKey::Item(it.id), who) {
            continue;
        }
        // Not material lying on another build site; but a roof is built overhead, so what lies
        // on the floor under a roof order is free (felled logs inside a cleared house site were
        // locked by its roof orders, timber for the roof included). What a bench has made lies
        // on the bench's tile, under its next order, and is free too: an order's own inputs
        // are used up as they are brought (C67: the planks sawn for the walls sat at the bench).
        if exclude_sites
            && w.designations.iter().any(|d| {
                d.pos == p
                    && !d.done
                    && !matches!(
                        d.kind,
                        DesignationKind::Haul
                            | DesignationKind::BuildRoof(_)
                            | DesignationKind::ClearRoof
                            | DesignationKind::Craft(_)
                    )
            })
        {
            continue;
        }
        if !nav.reach(from, p) {
            continue;
        }
        let d = from.dist(p);
        if best.map(|b| d < b.0).unwrap_or(true) {
            best = Some((d, it.id, p, it.count));
        }
    }
    best.map(|b| (b.1, b.2, b.3))
}

pub fn plan_for(w: &World, who: AgentId, d: &Designation) -> Option<Planned> {
    // A roof is laid out from the walls: a tile nothing would hold yet waits its turn.
    if matches!(d.kind, DesignationKind::BuildRoof(_)) && !crate::building::supported(w, d.pos) {
        return None;
    }
    if matches!(d.kind, DesignationKind::BuildFloor(_)) {
        let current = w.grid.get(d.pos);
        let under = w.grid.get(d.pos.offset(0, 0, -1));
        let ground_floor = under.shape == crate::world::Shape::Wall
            && !under.flags.has(crate::world::TileFlags::DOOR);
        let stair = current.shape.connects_up() || current.shape.connects_down();
        if !ground_floor && (stair || !crate::building::floor_supported(w, d.pos)) {
            return None;
        }
    }
    if matches!(d.kind, DesignationKind::BuildStairs(_))
        && crate::building::built_stair_shapes(w, d.pos).is_none()
    {
        return None;
    }
    let a = w.agents.iter().find(|a| a.id == who)?;
    let from = a.pos;
    let nav = w.nav.as_ref()?;
    let mut keys = vec![ResKey::Designation(d.id)];
    let mut steps = Vec::new();
    let estimate: u32;
    match d.kind {
        DesignationKind::Haul => {
            // The stack at d.pos goes to the nearest stockpile tile with room.
            let it = w
                .items
                .iter()
                .find(|it| matches!(it.place, ItemPlace::Ground(p) if p == d.pos))?;
            if !w.reservations.is_free_for(ResKey::Item(it.id), who) || !nav.reach(from, d.pos) {
                return None;
            }
            let target = stockpile_slot(w, who, it.kind, it.story.is_some(), d.pos)?;
            keys.push(ResKey::Item(it.id));
            keys.push(ResKey::Tile(target));
            steps.push(Step::GoTo {
                to: d.pos,
                adjacent: false,
            });
            // What they can carry of it (#422); the rest is hauled on another trip.
            steps.push(Step::PickUp {
                item: it.id,
                count: it.count.min(a.load(it.kind)),
            });
            steps.push(Step::GoTo {
                to: target,
                adjacent: false,
            });
            steps.push(Step::DropAt {
                to: target,
                kind: it.kind,
                for_site: None,
            });
            estimate = (from.dist(d.pos) + d.pos.dist(target)) as u32 * 6 + 200;
        }
        DesignationKind::Douse => {
            // To the water, then to the fire's edge, then throw.
            let src = crate::systems::settlement::water_source_near(w, d.pos, 25)?;
            let at_water = stand_tile(w, from, src)?;
            let stand = stand_tile(w, at_water, d.pos)?;
            if !nav.reach(from, at_water) || !nav.reach(at_water, stand) {
                return None;
            }
            steps.push(Step::GoTo {
                to: at_water,
                adjacent: false,
            });
            steps.push(Step::GoTo {
                to: stand,
                adjacent: false,
            });
            steps.push(Step::Work {
                designation: d.id,
                station: None,
            });
            estimate =
                (from.dist(at_water) + at_water.dist(stand)) as u32 * 6 + d.work_left * 2 + 100;
        }
        DesignationKind::FetchWater => {
            // To the nearest water (#433), fill the vessel, then to the store and pour.
            let src = crate::water::nearest(w, d.pos, 40)?;
            let at_water = src.stand;
            let stand = stand_tile(w, at_water, d.pos)?;
            if !nav.reach(from, at_water) || !nav.reach(at_water, stand) {
                return None;
            }
            steps.push(Step::GoTo {
                to: at_water,
                adjacent: false,
            });
            steps.push(Step::GoTo {
                to: stand,
                adjacent: false,
            });
            steps.push(Step::Work {
                designation: d.id,
                station: None,
            });
            estimate =
                (from.dist(at_water) + at_water.dist(stand)) as u32 * 6 + d.work_left * 2 + 100;
        }
        DesignationKind::Tend(patient) => {
            // A poultice in hand, then to the patient's side.
            let trace = std::env::var("HF_TRACE_TEND").is_ok();
            let Some(p) = w.agents.iter().find(|o| o.id == patient && o.alive) else {
                if trace {
                    eprintln!("tend: no patient");
                }
                return None;
            };
            let Some(stand) = stand_tile(w, from, p.pos) else {
                if trace {
                    eprintln!("tend: no stand tile by {:?}", p.pos);
                }
                return None;
            };
            // Poultices may sit on a tile with a haul order; that is still where they are.
            // With none to be had, they go and nurse the patient all the same (#418).
            let poultice =
                find_known_item(w, who, from, crate::items::ItemKind::Poultice, None, false)
                    .filter(|(_, ipos, _)| nav.reach(from, *ipos) && nav.reach(*ipos, stand));
            if poultice.is_none() && trace {
                eprintln!("tend: no known free poultice for {who:?}; nursing");
            }
            if !nav.reach(from, stand) {
                if trace {
                    eprintln!("tend: unreachable {from:?} -> {stand:?}");
                }
                return None;
            }
            let mut via = from;
            if let Some((iid, ipos, _)) = poultice {
                keys.push(ResKey::Item(iid));
                steps.push(Step::GoTo {
                    to: ipos,
                    adjacent: false,
                });
                steps.push(Step::PickUp {
                    item: iid,
                    count: 1,
                });
                via = ipos;
            }
            steps.push(Step::GoTo {
                to: stand,
                adjacent: false,
            });
            steps.push(Step::Work {
                designation: d.id,
                station: None,
            });
            estimate = (from.dist(via) + via.dist(stand)) as u32 * 6 + d.work_left * 2 + 200;
        }
        DesignationKind::Hunt(prey) => {
            let p = w.agents.iter().find(|o| o.id == prey && o.alive)?;
            let stand = stand_tile(w, from, p.pos)?;
            if !nav.reach(from, stand) {
                return None;
            }
            steps.push(Step::GoTo {
                to: stand,
                adjacent: false,
            });
            steps.push(Step::Work {
                designation: d.id,
                station: None,
            });
            estimate = from.dist(stand) as u32 * 6 + 600;
        }
        DesignationKind::Tame(beast) => {
            let p = w.agents.iter().find(|o| o.id == beast && o.alive)?;
            let stand = stand_tile(w, from, p.pos)?;
            if !nav.reach(from, stand) {
                return None;
            }
            steps.push(Step::GoTo {
                to: stand,
                adjacent: false,
            });
            steps.push(Step::Work {
                designation: d.id,
                station: None,
            });
            estimate = from.dist(stand) as u32 * 6 + 600;
        }
        DesignationKind::Snare => {
            let stand = stand_tile(w, from, d.pos)?;
            if !nav.reach(from, stand) {
                return None;
            }
            steps.push(Step::GoTo {
                to: stand,
                adjacent: false,
            });
            steps.push(Step::Work {
                designation: d.id,
                station: None,
            });
            estimate = from.dist(stand) as u32 * 6 + d.work_left * 2;
        }
        DesignationKind::Fish => {
            // From the bank beside the water (#299).
            let stand = stand_tile(w, from, d.pos)?;
            if !nav.reach(from, stand) {
                return None;
            }
            steps.push(Step::GoTo {
                to: stand,
                adjacent: false,
            });
            steps.push(Step::Work {
                designation: d.id,
                station: None,
            });
            estimate = from.dist(stand) as u32 * 6 + d.work_left * 2;
        }
        DesignationKind::DigStairs => {
            // Stand on the tile above the rock and dig down.
            let stand = d.pos.offset(0, 0, 1);
            if !w.grid.walkable(stand) || !nav.reach(from, stand) {
                return None;
            }
            steps.push(Step::GoTo {
                to: stand,
                adjacent: false,
            });
            steps.push(Step::Work {
                designation: d.id,
                station: None,
            });
            estimate = from.dist(stand) as u32 * 6 + d.work_left * 2 + 200;
        }
        DesignationKind::Chop
        | DesignationKind::Mine
        | DesignationKind::Sow
        | DesignationKind::Harvest
        | DesignationKind::DigEarth
        | DesignationKind::DigChannel
        | DesignationKind::Engrave
        | DesignationKind::ClearRoof
        | DesignationKind::Uproot
        | DesignationKind::Study(_)
        | DesignationKind::BeatOut => {
            let stand = stand_tile(w, from, d.pos)?;
            if !nav.reach(from, stand) {
                return None;
            }
            if matches!(
                d.kind,
                DesignationKind::Chop | DesignationKind::Harvest | DesignationKind::Sow
            ) {
                keys.push(ResKey::Plant(d.pos));
            }
            steps.push(Step::GoTo {
                to: stand,
                adjacent: false,
            });
            steps.push(Step::Work {
                designation: d.id,
                station: None,
            });
            estimate = from.dist(stand) as u32 * 6 + d.work_left * 2 + 200;
        }
        DesignationKind::Cook => {
            // Bring raw food to the hearth, then cook: one makes a meal (#172).
            let hearth = w
                .structures
                .iter()
                .find(|s| s.pos == d.pos && s.kind == StructureKind::Hearth)?;
            let stand = stand_tile(w, from, d.pos)?;
            let raw = crate::items::COOKABLE
                .iter()
                .find_map(|k| find_known_item(w, who, from, *k, None, true))?;
            keys.push(ResKey::Item(raw.0));
            keys.push(ResKey::Station(hearth.id));
            steps.push(Step::GoTo {
                to: raw.1,
                adjacent: false,
            });
            steps.push(Step::PickUp {
                item: raw.0,
                count: 1,
            });
            steps.push(Step::GoTo {
                to: stand,
                adjacent: false,
            });
            steps.push(Step::Work {
                designation: d.id,
                station: Some(hearth.id),
            });
            estimate = (from.dist(raw.1) + raw.1.dist(stand)) as u32 * 6 + d.work_left * 2 + 200;
        }
        DesignationKind::BuildWall(_)
        | DesignationKind::BuildDoor
        | DesignationKind::BuildStairs(_)
        | DesignationKind::BuildFloor(_)
        | DesignationKind::BuildRoof(_)
        | DesignationKind::BuildStructure(..)
        | DesignationKind::Craft(_) => {
            if let DesignationKind::Craft(r) = d.kind {
                // The workshop must stand, finished, on the order's tile.
                let st = crate::work::recipes::recipe(r).station;
                if !w
                    .structures
                    .iter()
                    .any(|s| s.pos == d.pos && s.kind == st && s.progress >= 1.0)
                {
                    return None;
                }
            }
            if let Some((kind, need, want)) = d.missing_wants().next() {
                // Deliver materials first: one trip per stack.
                // Of the wood they build of, if it is to hand (their way of building, C58).
                let way = match (d.kind, d.community) {
                    (
                        DesignationKind::BuildWall(_)
                        | DesignationKind::BuildDoor
                        | DesignationKind::BuildStairs(_)
                        | DesignationKind::BuildFloor(_),
                        Some(cid),
                    ) if kind == crate::items::ItemKind::Log && want.is_none() => {
                        crate::systems::settlement::building_way(w, cid).and_then(|(m, _)| m)
                    }
                    _ => None,
                };
                let (iid, ipos, have) = way
                    .and_then(|m| find_known_item(w, who, from, kind, Some(m), true))
                    .map_or_else(|| find_known_item(w, who, from, kind, want, true), Some)?;
                let take = have.min(need).min(a.load(kind));
                keys.push(ResKey::Item(iid));
                let stand = stand_tile(w, from, d.pos)?;
                steps.push(Step::GoTo {
                    to: ipos,
                    adjacent: false,
                });
                steps.push(Step::PickUp {
                    item: iid,
                    count: take,
                });
                steps.push(Step::GoTo {
                    to: stand,
                    adjacent: false,
                });
                steps.push(Step::DropAt {
                    to: d.pos,
                    kind,
                    for_site: Some(d.id),
                });
                estimate = (from.dist(ipos) + ipos.dist(stand)) as u32 * 6 + 200;
            } else {
                let stand = stand_tile(w, from, d.pos)?;
                if !nav.reach(from, stand) {
                    return None;
                }
                let station = if let DesignationKind::Craft(_) = d.kind {
                    w.structures.iter().find(|s| s.pos == d.pos).map(|s| s.id)
                } else {
                    None
                };
                if let Some(sid) = station {
                    keys.push(ResKey::Station(sid));
                }
                steps.push(Step::GoTo {
                    to: stand,
                    adjacent: false,
                });
                steps.push(Step::Work {
                    designation: d.id,
                    station,
                });
                estimate = from.dist(stand) as u32 * 6 + d.work_left * 2 + 200;
            }
        }
    }
    let label = match (
        &d.kind,
        steps.iter().any(|s| matches!(s, Step::PickUp { .. })),
        steps.iter().any(|s| matches!(s, Step::Work { .. })),
    ) {
        (DesignationKind::Haul, _, _) => "hauling to the stockpile".to_string(),
        (DesignationKind::Cook, _, _) => "cooking".to_string(),
        (_, true, false) => format!("fetching materials for the {}", d.label()),
        (k, _, _) => match k {
            DesignationKind::Chop => "felling a tree".into(),
            DesignationKind::ClearRoof => "clearing snow off the roof".into(),
            DesignationKind::Uproot => "pulling a weed".into(),
            DesignationKind::Study(_) => "studying something from the old valley".into(),
            DesignationKind::Mine => "quarrying stone".into(),
            DesignationKind::DigStairs => "digging the shaft".into(),
            DesignationKind::Hunt(_) => "hunting".into(),
            DesignationKind::Fish => "fishing".into(),
            DesignationKind::Snare => "setting a snare".into(),
            DesignationKind::Tame(_) => "taming".into(),
            DesignationKind::Sow => "sowing grain".into(),
            DesignationKind::Harvest => "harvesting".into(),
            DesignationKind::DigEarth => "digging clay".into(),
            DesignationKind::DigChannel => "digging a channel".into(),
            DesignationKind::Engrave => "carving the wall".into(),
            DesignationKind::Craft(_) => format!("making {}", d.label()),
            DesignationKind::Tend(_) => "tending the sick".into(),
            DesignationKind::Douse => "fighting the fire".into(),
            DesignationKind::FetchWater => "fetching water".into(),
            _ => format!("building the {}", d.label()),
        },
    };
    Some(Planned {
        plan: Plan {
            designation: d.id,
            steps,
            cursor: 0,
            label,
        },
        keys,
        estimate,
    })
}

/// A stockpile tile that is empty or already holds the same kind with room, nearest to `near`.
/// A thing with a story never stacks (legend.rs): it wants an empty tile, and a tile holding
/// one takes nothing else.
fn stockpile_slot(
    w: &World,
    who: AgentId,
    kind: crate::items::ItemKind,
    storied: bool,
    near: TilePos,
) -> Option<TilePos> {
    let nav = w.nav.as_ref()?;
    let mut best: Option<(i32, TilePos)> = None;
    for s in w
        .structures
        .iter()
        .filter(|s| s.kind == StructureKind::Stockpile && s.progress >= 1.0)
    {
        // Nothing is put down in standing water (#405).
        if !w.reservations.is_free_for(ResKey::Tile(s.pos), who)
            || !nav.reach(near, s.pos)
            || w.grid.get(s.pos).is_water()
        {
            continue;
        }
        let here: Vec<&crate::items::Item> = w
            .items
            .iter()
            .filter(|it| it.place == ItemPlace::Ground(s.pos))
            .collect();
        let ok = here.is_empty()
            || (!storied
                && here.len() == 1
                && here[0].story.is_none()
                && here[0].kind == kind
                && here[0].count < kind.max_stack());
        if !ok {
            continue;
        }
        // Food belongs in the cool store below ground when the community has one.
        let cellar_bonus = if kind.is_food() && s.pos.z < near.z {
            30
        } else {
            0
        };
        let d = near.dist(s.pos) + if here.is_empty() { 2 } else { 0 } - cellar_bonus;
        if best.map(|b| d < b.0).unwrap_or(true) {
            best = Some((d, s.pos));
        }
    }
    best.map(|b| b.1)
}
