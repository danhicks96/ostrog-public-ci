//! Execute work plans step by step, and apply the effects of finished designations.

use crate::events::{EventKind, Severity};
use crate::ids::AgentId;
use rand::Rng;

use crate::items::{ItemKind, ItemPlace};
use crate::mind::{Activity, MemKind, Skill, Subject};
use crate::plants::{Plant, PlantKind};
use crate::structures::{Structure, StructureKind};
use crate::work::{DesignationId, DesignationKind, Step};
use crate::world::{Material, Shape, Tile, TileFlags};
use crate::world_state::World;

pub const LEASE_SWEEP: u64 = 60;

/// Within one tile horizontally and one level vertically.
fn near3(a: crate::ids::TilePos, b: crate::ids::TilePos) -> bool {
    (a.x - b.x).abs() <= 1 && (a.y - b.y).abs() <= 1 && (a.z - b.z).abs() <= 1
}

/// Sweep expired leases (every 60 ticks), and the leases of anyone no longer at work.
pub fn sweep(w: &mut World) {
    if w.meta.tick.is_multiple_of(LEASE_SWEEP) {
        let now = w.meta.tick;
        w.reservations.sweep(now);
        // Leases are taken only to start a work plan (decide.rs) and belong to whoever is at
        // that work. Many things take someone off it without the work's own finish or abort:
        // a rite, an observance, a threat, a fey mood. Their leases were kept until they ran
        // out, two game hours for a cook, so in a hungry valley the hearth was always held by
        // someone not cooking and nobody cooked a meal all day (#743, FIXLOG 2026-11-12).
        let working: std::collections::BTreeSet<crate::ids::AgentId> = w
            .agents
            .iter()
            .filter(|a| a.alive && matches!(a.activity, Activity::Working { .. }))
            .map(|a| a.id)
            .collect();
        for who in w.reservations.holders() {
            if !working.contains(&who) {
                w.reservations.revoke_all(who);
            }
        }
    }
}

/// Run one tick of the plan for agent `i`. Returns true if the activity ended.
pub fn run(w: &mut World, i: usize) -> bool {
    let tick = w.meta.tick;
    let who = w.agents[i].id;
    let Activity::Working { plan } = &w.agents[i].activity else {
        return false;
    };
    let did = plan.designation;
    // Designation vanished or finished by someone else?
    let Some(dix) = w.designations.iter().position(|d| d.id == did && !d.done) else {
        return abort(w, i, "work order gone");
    };
    // Whoever is at a work holds its leases while they are at it, not only from one step to
    // the next (#389, QA 2026-10-26: a carving is three hours of work and the lease renewed
    // at its start lasts two, so a second carver took the order and lost it to the first).
    if tick.is_multiple_of(LEASE_SWEEP) {
        w.reservations.renew(who, tick + 20 * 60 * 2);
    }
    let Some(step) = plan.current().cloned() else {
        return finish(w, i);
    };
    // A minute at a work order counts toward its kind of work (duties.rs, #454).
    if tick.is_multiple_of(crate::time::TICKS_PER_MINUTE) {
        let kind = w.designations[dix].kind;
        crate::duties::worked(w, i, kind);
    }
    match step {
        Step::GoTo { to, adjacent: _ } => {
            // Hunting: the prey moves, so keep walking toward where it is now. Taming too: the
            // order stands where the beast was when it was given, and a tamer who walked there
            // found it gone and failed as "not at the site" (#553, FIXLOG 2026-11-01: 1,162 times
            // in 200 days on seed 1, one goat's order).
            if let DesignationKind::Hunt(prey) | DesignationKind::Tame(prey) =
                w.designations[dix].kind
            {
                let tame = matches!(w.designations[dix].kind, DesignationKind::Tame(_));
                let Some(p) = w
                    .agents
                    .iter()
                    .find(|o| o.id == prey && o.alive)
                    .map(|o| o.pos)
                else {
                    return abort(w, i, "the prey is gone");
                };
                let me = w.agents[i].pos;
                if near3(me, p) {
                    advance(w, i);
                    return false;
                }
                if w.agents[i].activity_ticks > 20 * 40 {
                    let why = if tame {
                        "the beast wandered off"
                    } else {
                        "the prey got away"
                    };
                    return abort(w, i, why);
                }
                if p.dist(to) > 2 && w.agents[i].path.len().is_multiple_of(8) {
                    w.designations[dix].pos = p;
                    if let Activity::Working { plan } = &mut w.agents[i].activity {
                        if let Some(Step::GoTo { to, .. }) = plan.steps.get_mut(plan.cursor) {
                            *to = p;
                        }
                    }
                    if !w.path_agent_to(i, p) {
                        return abort(w, i, "cannot reach the prey");
                    }
                    return false;
                }
            }
            let a = &w.agents[i];
            if a.pos == to {
                advance(w, i);
            } else if a.path_blocked {
                return abort(w, i, "path blocked");
            } else if a.path.is_empty() && !a.is_moving() {
                // Not yet routed, or arrived somewhere else.
                if near3(a.pos, to) && !w.grid.walkable(to) {
                    advance(w, i);
                } else if !w.path_agent_to(i, to) {
                    if std::env::var("HF_TRACE_REACH").is_ok() {
                        let from = w.agents[i].pos;
                        let big = {
                            let mut scratch = crate::nav::AStarScratch::new(&w.grid);
                            match crate::nav::find_path(&w.grid, &mut scratch, from, to, 400_000) {
                                crate::nav::PathResult::Found(p) => {
                                    format!("found len {}", p.len())
                                }
                                crate::nav::PathResult::Unreachable => "unreachable".into(),
                                crate::nav::PathResult::Budget => "budget".into(),
                            }
                        };
                        let back = {
                            let mut scratch = crate::nav::AStarScratch::new(&w.grid);
                            match crate::nav::find_path(&w.grid, &mut scratch, to, from, 400_000) {
                                crate::nav::PathResult::Found(p) => {
                                    let steps: Vec<String> = p
                                        .iter()
                                        .filter(|q| {
                                            q.z != to.z
                                                || p.iter().any(|r| {
                                                    r.z != q.z
                                                        && (r.x - q.x).abs() <= 1
                                                        && (r.y - q.y).abs() <= 1
                                                })
                                        })
                                        .map(|q| {
                                            format!(
                                                "({},{},{}:{:?})",
                                                q.x,
                                                q.y,
                                                q.z,
                                                w.grid.get(*q).shape
                                            )
                                        })
                                        .collect();
                                    format!("found len {} via {}", p.len(), steps.join(" "))
                                }
                                crate::nav::PathResult::Unreachable => "unreachable".into(),
                                crate::nav::PathResult::Budget => "budget".into(),
                            }
                        };
                        let col: Vec<String> = (1..=3)
                            .map(|z| {
                                let t = w.grid.get(crate::ids::TilePos::new(to.x, to.y, z));
                                format!("z{z}:{:?}", t.shape)
                            })
                            .collect();
                        eprintln!(
                            "  big-budget A*: {big}; reverse: {back}; column at goal {col:?}"
                        );
                        let a = &w.agents[i];
                        let reach = w.nav.as_ref().map(|n| n.reach(a.pos, to));
                        eprintln!(
                            "REACH-FAIL {} at {:?} -> {:?} walkable_from {} walkable_to {} reach {:?} plan {}",
                            a.name,
                            a.pos,
                            to,
                            w.grid.walkable(a.pos),
                            w.grid.walkable(to),
                            reach,
                            match &a.activity {
                                Activity::Working { plan } => plan.label.clone(),
                                _ => String::new(),
                            }
                        );
                    }
                    return abort(w, i, "cannot reach");
                }
            }
        }
        Step::PickUp { item, count } => {
            let here = w.agents[i].pos;
            let Some(ix) = w.items.iter().position(|it| it.id == item) else {
                w.agents[i]
                    .knowledge
                    .forget(crate::mind::knowledge::Known::Item(item));
                return abort(w, i, "item gone");
            };
            match w.items[ix].place {
                ItemPlace::Ground(p) if near3(p, here) => {}
                _ => {
                    w.agents[i]
                        .knowledge
                        .doubt(crate::mind::knowledge::Known::Item(item));
                    return abort(w, i, "item moved");
                }
            }
            let take = count.min(w.items[ix].count);
            if take == w.items[ix].count {
                w.items[ix].place = ItemPlace::Carried(who);
            } else {
                w.items[ix].count -= take;
                let kind = w.items[ix].kind;
                let q = w.items[ix].quality;
                let material = w.items[ix].material;
                let rot = w.items[ix].rot;
                let nid = w.alloc_item_id();
                w.items.push(crate::items::Item {
                    id: nid,
                    kind,
                    count: take,
                    place: ItemPlace::Carried(who),
                    quality: q,
                    material,
                    story: None,
                    rot,
                });
                // The lease covered the source stack; the carried split is ours by possession.
            }
            w.agents[i].skills.practise(Skill::Hauling, 0.02);
            advance(w, i);
        }
        Step::DropAt { to, kind, for_site } => {
            let here = w.agents[i].pos;
            if !near3(here, to) {
                return abort(w, i, "not at the drop point");
            }
            let delivery = match w.designations[dix].kind {
                DesignationKind::DeliverProvisions(id) => {
                    if here != to || !w.grid.walkable(to) {
                        return abort(w, i, "the camp drop point is blocked");
                    }
                    if crate::work::planner::provision_stack(w, &w.designations[dix], id).is_none()
                    {
                        w.designations[dix].done = true;
                        return abort(w, i, "provisions no longer bound for this camp");
                    }
                    Some(id)
                }
                _ => None,
            };
            let Some(ix) = w.items.iter().position(|it| {
                it.place == ItemPlace::Carried(who)
                    && it.kind == kind
                    && delivery.is_none_or(|id| it.id == id)
            }) else {
                return abort(w, i, "nothing carried");
            };
            match for_site {
                Some(site) => {
                    let n = w.items[ix].count;
                    let Some(dx) = w.designations.iter().position(|d| d.id == site && !d.done)
                    else {
                        // Site gone: put it down here.
                        w.items[ix].place = ItemPlace::Ground(here);
                        return abort(w, i, "site gone");
                    };
                    let mut delivered = 0;
                    let material = w.items[ix].material;
                    let quality = w.items[ix].quality;
                    // #274: the first stone brought to an order sets how long working it takes.
                    let first_stone = w.designations[dx]
                        .requirements
                        .first()
                        .is_some_and(|r| r.kind == kind && r.delivered == 0);
                    if let Some(r) = w.designations[dx]
                        .requirements
                        .iter_mut()
                        .find(|r| r.kind == kind && r.accepts(material) && r.delivered < r.needed)
                    {
                        delivered = n.min(r.needed - r.delivered);
                        if r.delivered == 0 || r.material == Material::AIR {
                            r.material = material;
                        }
                        // The mean quality of what has come (#233: given back at 0.5).
                        let had = r.delivered as f32;
                        r.quality = (r.quality * had + quality * delivered as f32)
                            / (had + delivered as f32).max(1.0);
                        r.delivered += delivered;
                    }
                    if first_stone && delivered > 0 {
                        crate::tiers::worked(w, dx, material);
                    }
                    // Delivered units are consumed into the site (the build will create the structure).
                    let id = w.items[ix].id;
                    w.consume_item(id, delivered, "delivered");
                    if let Some(ix2) = w.items.iter().position(|it| it.id == id) {
                        w.items[ix2].place = ItemPlace::Ground(here);
                    }
                    let site_pos = w.designations[dx].pos;
                    w.events.push(
                        tick,
                        Some(site_pos),
                        Severity::Trace,
                        EventKind::Hauled {
                            agent: who,
                            item: id,
                            to: site_pos,
                        },
                    );
                }
                None => {
                    // Into a same-kind stack on the tile, else beside it (`put_down`).
                    let id = w.items[ix].id;
                    let survivor = put_down(w, ix, to);
                    w.events.push(
                        tick,
                        Some(to),
                        Severity::Minor,
                        EventKind::Hauled {
                            agent: who,
                            item: id,
                            to,
                        },
                    );
                    w.agents[i].knowledge.note(
                        crate::mind::knowledge::Known::Item(if delivery.is_some() {
                            survivor
                        } else {
                            id
                        }),
                        to,
                        tick,
                        kind as u16,
                    );
                    if delivery.is_some() {
                        w.designations[dix].done = true;
                        return finish(w, i);
                    }
                }
            }
            advance(w, i);
        }
        Step::Work {
            designation,
            station,
        } => {
            let here = w.agents[i].pos;
            if let DesignationKind::Hunt(prey) | DesignationKind::Tame(prey) =
                w.designations[dix].kind
            {
                let Some(p) = w
                    .agents
                    .iter()
                    .find(|o| o.id == prey && o.alive)
                    .map(|o| o.pos)
                else {
                    return abort(w, i, "the prey is gone");
                };
                if !near3(here, p) {
                    // It moved off: back to the chase.
                    if let Activity::Working { plan } = &mut w.agents[i].activity {
                        plan.cursor = 0;
                    }
                    return false;
                }
            }
            let dpos = w.designations[dix].pos;
            if !matches!(
                w.designations[dix].kind,
                DesignationKind::Hunt(_) | DesignationKind::Tame(_)
            ) && !near3(here, dpos)
            {
                return abort(w, i, "not at the site");
            }
            if !w.designations[dix].materials_complete() {
                return abort(w, i, "materials missing");
            }
            let skill = skill_for(w.designations[dix].kind);
            let kind_now = w.designations[dix].kind;
            // Tools: hard work goes faster with a good tool in hand, and wears it down.
            let tool_factor = if uses_tool(kind_now) {
                if w.agents[i].tool_quality <= 0.0 {
                    take_tool(w, i);
                }
                if w.agents[i].tool_quality > 0.0 && w.agents[i].tool.is_some() {
                    // A tool with a story wears on the item itself (legend.rs).
                    if crate::systems::work::wear_held(w, i) {
                        0.7 + 0.6 * w.agents[i].tool_quality
                    } else {
                        0.6
                    }
                } else if w.agents[i].tool_quality > 0.0 {
                    // A tool from before stories (an old save): worn on the person, as it was.
                    let a = &mut w.agents[i];
                    a.tool_wear -= 1.0 / 14_000.0;
                    if a.tool_wear <= 0.0 {
                        let q = a.tool_quality;
                        a.tool_quality = 0.0;
                        a.tool_wear = 0.0;
                        w.events.push(
                            tick,
                            Some(here),
                            Severity::Minor,
                            EventKind::ToolBroke {
                                agent: who,
                                tier: ItemKind::tier(q),
                                what: String::new(),
                                history: String::new(),
                            },
                        );
                        0.6
                    } else {
                        0.7 + 0.6 * a.tool_quality
                    }
                } else {
                    0.6
                }
            } else {
                1.0
            };
            // Accidents: a slip of the axe, a stone on the foot; rarer with a good tool.
            if uses_tool(kind_now) && tick.is_multiple_of(20) {
                let mut r = crate::rng::stream(
                    w.meta.seed,
                    &[crate::rng::purpose::AGENT_BIRTH, 0x4EA5, who.0 as u64, tick],
                );
                let chance = 0.00006 * (1.4 - w.agents[i].tool_quality).max(0.4);
                if r.gen::<f32>() < chance {
                    let (part, wk, cause) = match kind_now {
                        DesignationKind::Chop => (
                            crate::body::Part::LEG_L,
                            crate::body::WoundKind::Cut,
                            "from a slipped axe",
                        ),
                        DesignationKind::Mine | DesignationKind::DigStairs => (
                            crate::body::Part::ARM_R,
                            crate::body::WoundKind::Bruise,
                            "from falling stone",
                        ),
                        _ => (
                            crate::body::Part::ARM_L,
                            crate::body::WoundKind::Cut,
                            "at the building site",
                        ),
                    };
                    crate::systems::health::wound(
                        w,
                        i,
                        part,
                        wk,
                        0.2 + r.gen::<f32>() * 0.3,
                        if wk == crate::body::WoundKind::Cut {
                            0.3
                        } else {
                            0.0
                        },
                        cause,
                        None,
                    );
                }
            }
            // Heat slows everyone, in the heat where they work (#268: the heights run hotter).
            let heat = if w.temp_at(w.agents[i].pos) > 30.0 {
                0.8
            } else {
                1.0
            };
            // Below ground with no light to see by, the work goes slowly; and whoever is
            // working there means to set a lamp, their own undertaking (underground.rs).
            let dark = crate::underground::dark(w, here);
            if dark {
                want_light(w, i, here);
            }
            // #946: a miserable or stressed person's hands are not as sure as a contented
            // one's; a grieving or breaking-down person built exactly as fast as anyone.
            let body_factor = w.agents[i].body.work_factor()
                * heat
                * if dark { 0.6 } else { 1.0 }
                * crate::conditions::work_factor(w, i)
                * spirit(w.agents[i].mood, w.agents[i].stress);
            let speed = w.agents[i].skills.speed(skill) * tool_factor * body_factor;
            let done_now = {
                let d = &mut w.designations[dix];
                let dec = (speed * 1.0).max(0.2);
                d.work_left = d.work_left.saturating_sub(dec.ceil() as u32);
                d.work_left == 0
            };
            w.agents[i].facing = crate::agents::Agent::facing_toward(here, dpos);
            if w.meta.tick.is_multiple_of(20) {
                w.agents[i].skills.practise(skill, 1.0 / 60.0);
                w.agents[i].needs.hunger = (w.agents[i].needs.hunger - 0.004).max(0.0);
            }
            if done_now {
                apply_effect(w, i, designation, station);
                let label = match &w.agents[i].activity {
                    Activity::Working { plan } => plan.label.clone(),
                    _ => String::new(),
                };
                let big = label.starts_with("building") || label.starts_with("digging");
                w.agents[i].remember(
                    tick,
                    MemKind::Work,
                    Subject::Place(dpos),
                    if big { 0.5 } else { 0.25 },
                    if big { 0.5 } else { 0.2 },
                    None,
                );
                // Working alongside someone builds familiarity.
                let near: Vec<AgentId> = w
                    .agents
                    .iter()
                    .filter(|o| {
                        o.id != who
                            && o.alive
                            && o.pos.dist(here) <= 3
                            && matches!(o.activity, Activity::Working { .. })
                    })
                    .map(|o| o.id)
                    .collect();
                for o in near {
                    w.agents[i].relate(o, 0.02, 0.03, tick);
                    w.agents[i].met(o, tick);
                }
                return finish(w, i);
            }
        }
    }
    false
}

/// Work speed factor from mood and stress (#946): a miserable or breaking-down person's
/// hands are not as sure as a contented one's. Mood is -1..1, stress 0..1.
pub fn spirit(mood: f32, stress: f32) -> f32 {
    (0.9 + 0.15 * mood - 0.1 * stress.clamp(0.0, 1.0)).clamp(0.7, 1.05)
}

/// Work that wants a tool: felling, digging, quarrying, building.
fn uses_tool(kind: DesignationKind) -> bool {
    matches!(
        kind,
        DesignationKind::Chop
            | DesignationKind::Mine
            | DesignationKind::DigStairs
            | DesignationKind::DigEarth
            | DesignationKind::DigChannel
            | DesignationKind::Engrave
            | DesignationKind::BuildWall(_)
            | DesignationKind::BuildDoor
            | DesignationKind::BuildStairs(_)
            | DesignationKind::BuildFloor(_)
            | DesignationKind::BuildRoof(_)
            | DesignationKind::BuildStructure(..)
    )
}

/// Agent `i` has spent hours over a record or tool from the old valley (C13). Whether it gives
/// up its craft depends on what it is (a record says more than a tool), on their curiosity and
/// their hands; it can be studied again by someone else, or by them after a while.
fn study(w: &mut World, i: usize, item: crate::ids::ItemId) {
    use crate::mind::techniques::{Learned, Technique};
    let tick = w.meta.tick;
    let Some(it) = w.items.iter().find(|it| it.id == item).cloned() else {
        return;
    };
    let Some(s) = it.story.as_ref() else { return };
    let Some(t): Option<Technique> = s.teaches else {
        return;
    };
    let site = w
        .history
        .sites
        .iter()
        .find(|x| s.origin.contains(&x.name))
        .map(|x| x.name.clone())
        .unwrap_or_else(|| "the old valley".to_string());
    let what = crate::legend::noun(it.kind, it.material);
    if w.agents[i].techniques.knows(t) {
        return;
    }
    let mut r = crate::rng::stream(
        w.meta.seed,
        &[
            crate::rng::purpose::LEGEND,
            0x57D7,
            w.agents[i].id.0 as u64,
            item.0 as u64,
            tick,
        ],
    );
    let curious = w.agents[i].traits.has(crate::mind::Trait::Curious);
    let chance =
        0.3 + if it.kind == ItemKind::Record {
            0.35
        } else {
            0.1
        } + if curious { 0.15 } else { 0.0 }
            + w.agents[i].skills.level(Skill::Crafting) / 60.0;
    let success = r.gen::<f32>() < chance;
    let who = w.agents[i].id;
    if success {
        let from = format!("studying the {what} left at {site}");
        w.agents[i]
            .techniques
            .learn_from(t, Learned::Studied, 0.4, tick, &from);
    } else {
        // Not now: they will not sit over the same thing again for a few days.
        w.agents[i].work_cooldowns.insert(
            0x5700_0000 | (item.0 & 0x00FF_FFFF),
            tick + 3 * crate::time::TICKS_PER_DAY,
        );
    }
    let at = w.agents[i].pos;
    w.events.push(
        tick,
        Some(at),
        if success {
            Severity::Notable
        } else {
            Severity::Minor
        },
        EventKind::Studied {
            agent: who,
            what,
            site,
            learned: t.name().to_string(),
            success,
        },
    );
}

/// Wear the held tool by one tick of hard work; false if it broke.
pub fn wear_held(w: &mut World, i: usize) -> bool {
    // A good tool lasts: a fine iron one many times longer than a crude stone edge.
    let q = w.agents[i].tool_quality.clamp(0.05, 1.0);
    crate::legend::wear(w, i, 1.0 / (14_000.0 * (1.0 + 6.0 * q)))
}

/// A tool carried, in a chest, or kept on a finished shelf, within `radius` tiles of `pos`:
/// what a quick idle craft (whittling, #980) or a mend (#999) can reach for without a haul.
/// A tool on a shelf's tile is kept there (#1773, 2026-11-15), so it counts as in reach the
/// same as one in a chest.
pub fn tool_in_reach(
    w: &World,
    pos: crate::ids::TilePos,
    radius: i32,
) -> Option<crate::ids::ItemId> {
    w.items
        .iter()
        .find(|it| {
            it.kind == ItemKind::Tool
                && it.count > 0
                && match it.place {
                    ItemPlace::Ground(p) if p.dist(pos) <= radius => w.structures.iter().any(|s| {
                        s.pos == p
                            && s.progress >= 1.0
                            && matches!(s.kind, StructureKind::Chest | StructureKind::Shelf)
                    }),
                    ItemPlace::Carried(a) | ItemPlace::Held(a) => w
                        .agents
                        .iter()
                        .any(|x| x.id == a && x.pos.dist(pos) <= radius),
                    _ => false,
                }
        })
        .map(|it| it.id)
}

/// Take the best tool from the community's store (abstracted: the store is a walk away and
/// tools are shared). Returns true when one was taken.
fn take_tool(w: &mut World, i: usize) -> bool {
    let home = w.agents[i]
        .community
        .and_then(|c| w.communities.iter().find(|x| x.id == c).map(|x| x.home))
        .unwrap_or(w.meta.founding_site);
    let best = w
        .items
        .iter()
        .filter(|it| {
            it.kind == ItemKind::Tool
                && it.count > 0
                && matches!(it.place, ItemPlace::Ground(p) if p.dist(home) <= 45)
                // Not one someone holds a lease on, about to haul it (#177).
                && w.reservations.is_free_for(crate::work::ResKey::Item(it.id), w.agents[i].id)
                // Nor a relic, kept and not used (relics.rs, FIXLOG 2026-10-28).
                && !crate::relics::is_relic(w, it.id)
        })
        .max_by(|a, b| {
            a.quality
                .partial_cmp(&b.quality)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|it| (it.id, it.quality));
    if let Some((id, q)) = best {
        // The tool is taken into the hand, not used up: it is the same tool until it breaks.
        let id = match w.items.iter().position(|it| it.id == id) {
            Some(ix) if w.items[ix].count > 1 => {
                w.items[ix].count -= 1;
                let one = crate::items::Item {
                    id: w.alloc_item_id(),
                    count: 1,
                    story: None,
                    ..w.items[ix].clone()
                };
                let nid = one.id;
                w.items.push(one);
                nid
            }
            _ => id,
        };
        crate::legend::take_up(w, i, id);
        w.agents[i].tool_quality = q.max(0.05);
        w.agents[i].tool_wear = w
            .items
            .iter()
            .find(|it| it.id == id)
            .and_then(|it| it.story.as_ref())
            .map(|s| s.wear)
            .unwrap_or(1.0);
        true
    } else {
        false
    }
}

pub fn skill_for(kind: DesignationKind) -> Skill {
    match kind {
        DesignationKind::Chop => Skill::Construction,
        DesignationKind::Mine
        | DesignationKind::DigStairs
        | DesignationKind::DigEarth
        | DesignationKind::DigChannel => Skill::Mining,
        DesignationKind::Hunt(_) | DesignationKind::Fish | DesignationKind::Snare => Skill::Animals,
        DesignationKind::Sow | DesignationKind::Harvest => Skill::Farming,
        DesignationKind::Cook => Skill::Cooking,
        DesignationKind::Craft(_) => Skill::Crafting,
        DesignationKind::Tend(_) => Skill::Cooking,
        DesignationKind::Douse | DesignationKind::BeatOut | DesignationKind::FetchWater => {
            Skill::Hauling
        }
        DesignationKind::Haul | DesignationKind::DeliverProvisions(_) => Skill::Hauling,
        _ => Skill::Construction,
    }
}

fn advance(w: &mut World, i: usize) {
    if let Activity::Working { plan } = &mut w.agents[i].activity {
        plan.advance();
        if plan.finished() {
            // Leave the leases to `finish` on the next tick.
        }
    }
    let now = w.meta.tick;
    let who = w.agents[i].id;
    w.reservations.renew(who, now + 20 * 60 * 2);
}

fn finish(w: &mut World, i: usize) -> bool {
    let who = w.agents[i].id;
    w.reservations.release_all(who);
    w.agents[i].activity = Activity::Idle;
    w.agents[i].activity_ticks = 0;
    w.agents[i].idle_ticks = 10;
    w.agents[i].path.clear();
    true
}

fn abort(w: &mut World, i: usize, reason: &str) -> bool {
    let who = w.agents[i].id;
    let (label, did) = match &w.agents[i].activity {
        Activity::Working { plan } => (plan.label.clone(), Some(plan.designation)),
        _ => (String::new(), None),
    };
    // Put down anything carried so nothing vanishes.
    let here = w.agents[i].pos;
    for it in w.items.iter_mut() {
        if it.place == ItemPlace::Carried(who) && !it.kind.is_food() {
            it.place = ItemPlace::Ground(here);
        }
    }
    if let Some(did) = did {
        if let Some(d) = w.designations.iter_mut().find(|d| d.id == did) {
            d.failures += 1;
            if matches!(d.kind, DesignationKind::DeliverProvisions(_)) {
                w.agents[i]
                    .work_cooldowns
                    .insert(did.0, w.meta.tick + crate::time::TICKS_PER_HOUR / 4);
            }
        }
    }
    w.reservations.release_all(who);
    let tick = w.meta.tick;
    let a = &mut w.agents[i];
    a.activity = Activity::Idle;
    a.activity_ticks = 0;
    a.idle_ticks = 40;
    a.path.clear();
    a.path_blocked = false;
    a.remember(tick, MemKind::Work, Subject::Place(here), -0.2, 0.15, None);
    w.events.push(
        w.meta.tick,
        Some(here),
        Severity::Trace,
        EventKind::PlanFailed {
            agent: who,
            goal: label,
            reason: reason.into(),
        },
    );
    true
}

fn apply_effect(
    w: &mut World,
    i: usize,
    did: DesignationId,
    station: Option<crate::ids::StructureId>,
) {
    let tick = w.meta.tick;
    let who: AgentId = w.agents[i].id;
    let Some(dix) = w.designations.iter().position(|d| d.id == did) else {
        return;
    };
    let d = w.designations[dix].clone();
    let pos = d.pos;
    // Anyone standing on a tile about to become solid steps aside first.
    if matches!(
        d.kind,
        DesignationKind::BuildWall(_) | DesignationKind::BuildDoor
    ) {
        displace_from(w, pos);
    }
    // Anything built on a tile clears the plants there.
    if matches!(
        d.kind,
        DesignationKind::BuildWall(_)
            | DesignationKind::BuildDoor
            | DesignationKind::BuildStairs(_)
            | DesignationKind::BuildFloor(_)
            | DesignationKind::BuildRoof(_)
            | DesignationKind::BuildStructure(..)
    ) {
        w.plants.retain(|p| p.pos != pos);
    }
    match d.kind {
        DesignationKind::BuildWall(m) => {
            // The tile stays timber or stone to everything that asks (fire, mining, houses);
            // what it was built of, an oak wall or a granite one, is kept beside it (C56).
            w.grid.set(pos, Tile::solid(m));
            w.built_of.insert(pos, built_of(&d, m));
            let sid = w.alloc_structure_id();
            w.events.push(
                tick,
                Some(pos),
                // One wall tile is not news: the house's stages are (settlement.rs).
                Severity::Trace,
                EventKind::Built {
                    agent: who,
                    structure: sid,
                    at: pos,
                },
            );
        }
        DesignationKind::BuildDoor => {
            let mut t = Tile::solid(Material::WOOD);
            w.built_of.insert(pos, built_of(&d, Material::WOOD));
            t.flags = TileFlags(TileFlags::DOOR);
            w.grid.set(pos, t);
            let sid = w.alloc_structure_id();
            w.events.push(
                tick,
                Some(pos),
                // One wall tile is not news: the house's stages are (settlement.rs).
                Severity::Trace,
                EventKind::Built {
                    agent: who,
                    structure: sid,
                    at: pos,
                },
            );
        }
        DesignationKind::BuildStairs(m) => {
            let above = pos.offset(0, 0, 1);
            let Some((lower_shape, upper_shape)) = crate::building::built_stair_shapes(w, pos)
            else {
                return;
            };
            let material = built_of(&d, m);
            w.grid.set(
                pos,
                Tile {
                    material: m,
                    shape: lower_shape,
                    flags: TileFlags::NONE,
                },
            );
            w.grid.set(
                above,
                Tile {
                    material: m,
                    shape: upper_shape,
                    flags: TileFlags::NONE,
                },
            );
            w.built_of.insert(pos, material);
            w.built_of.insert(above, material);
            // The stair is a real opening through the ceiling below (E10.2, #483).
            w.roofs.remove(&pos);
            let sid = w.alloc_structure_id();
            w.events.push(
                tick,
                Some(pos),
                Severity::Trace,
                EventKind::Built {
                    agent: who,
                    structure: sid,
                    at: pos,
                },
            );
        }
        DesignationKind::BuildFloor(m) => {
            // Ground floors keep their original representation: the built tile is immediately
            // below the order. An upper floor instead occupies the order's own tile and must be
            // carried by walls below (FIXLOG 2026-10-31, #483, pending checkpoint). Neither case
            // fills an open room below with rock (#174).
            let current = w.grid.get(pos);
            let below = pos.offset(0, 0, -1);
            let under = w.grid.get(below);
            let stair = current.shape.connects_up() || current.shape.connects_down();
            if under.shape == Shape::Wall && !under.flags.has(TileFlags::DOOR) {
                w.grid.set(below, Tile::solid(m));
                w.built_of.insert(below, built_of(&d, m));
            } else if stair {
                return;
            } else if crate::building::floor_supported(w, pos) {
                w.grid.set(pos, Tile::floor(m));
                w.built_of.insert(pos, built_of(&d, m));
            } else {
                return;
            }
        }
        DesignationKind::BuildRoof(m) => {
            // How well it is made decides how much snow it will bear (building.rs).
            let skill = w
                .agents
                .iter()
                .find(|a| a.id == who)
                .map(|a| a.skills.level(crate::mind::Skill::Construction))
                .unwrap_or(0.0);
            let mut r = crate::rng::stream(
                w.meta.seed,
                &[
                    crate::rng::purpose::BUILDING,
                    0x2F,
                    pos.x as u64,
                    pos.y as u64,
                    tick,
                ],
            );
            // Someone who knows the old way of laying a roof lays one that bears more (C13).
            let roofing = w
                .agents
                .iter()
                .find(|a| a.id == who)
                .map(|a| a.techniques.bonus(crate::mind::Technique::Roofing))
                .unwrap_or(0.0);
            let quality = (0.25 + skill / 25.0 + 0.25 * roofing + r.gen_range(-0.15..0.15f32))
                .clamp(0.1, 1.0);
            if roofing > 0.0 {
                crate::systems::learn::witnessed(w, i, crate::mind::Technique::Roofing);
            }
            w.roofs.insert(
                pos,
                crate::building::Roof {
                    // Thatch is cut, not brought; a timber roof is of the poles' wood (C56).
                    material: if m == Material::THATCH {
                        m
                    } else {
                        built_of(&d, m)
                    },
                    quality,
                    built: tick,
                    snow: 0.0,
                },
            );
        }
        DesignationKind::BuildStructure(kind, material) => {
            // Made of what was brought for it (C32): an oak table, a granite well.
            let material = built_of(&d, material);
            // Setting a prop where others can see it shows them how (underground.rs).
            if kind == StructureKind::Support
                && w.agents[i]
                    .techniques
                    .knows(crate::mind::Technique::Shoring)
            {
                crate::systems::learn::witnessed(w, i, crate::mind::Technique::Shoring);
            }
            let sid = w.alloc_structure_id();
            w.structures.push(Structure {
                id: sid,
                kind,
                pos,
                material,
                progress: 1.0,
                active: kind.emits_light(),
                condition: 1.0,
                owner: None,
                quality: d
                    .requirements
                    .first()
                    .filter(|r| {
                        r.quality > 0.0 && crate::items::FITTINGS.iter().any(|(_, k)| *k == r.kind)
                    })
                    .map_or(0.5, |r| r.quality),
            });
            // Furniture is not news on its own; a hearth or a workshop is.
            use StructureKind as K;
            let sev = match kind {
                K::Stockpile
                | K::Bed
                | K::Table
                | K::Chair
                | K::Stool
                | K::Chest
                | K::Shelf
                | K::Lamp
                | K::Barrel
                | K::Crate
                | K::FencePost => Severity::Trace,
                _ => Severity::Notable,
            };
            w.events.push(
                tick,
                Some(pos),
                sev,
                EventKind::Built {
                    agent: who,
                    structure: sid,
                    at: pos,
                },
            );
            for a in w.agents.iter_mut() {
                if a.pos.dist(pos) <= 14 {
                    a.knowledge.note(
                        crate::mind::knowledge::Known::Structure(sid),
                        pos,
                        tick,
                        kind as u16,
                    );
                }
            }
        }
        DesignationKind::Study(item) => {
            study(w, i, item);
        }
        DesignationKind::ClearRoof => {
            // Up on the roof with a board: the snow comes off this stretch of roof (the tiles
            // within reach of the ladder).
            for (p, r) in w.roofs.iter_mut() {
                if p.z == pos.z && p.dist(pos) <= 3 {
                    r.snow = 0.0;
                }
            }
        }
        DesignationKind::Uproot => {
            // Pulled out by the root: the weed is gone and nothing of it is kept (#286).
            w.plants
                .retain(|p| !(p.pos == pos && p.alive && p.kind.is_weed()));
        }
        DesignationKind::Chop => {
            if let Some(px) = w
                .plants
                .iter()
                .position(|p| p.pos == pos && p.alive && p.kind.yields_wood())
            {
                let growth = w.plants[px].growth;
                let variant = w.plants[px].variant;
                let wood = crate::world::material::wood_of(w.plants[px].kind, variant);
                // A felled tree leaves its stump, and one in three leaves its crown lying as a
                // fallen log beside it (art bible: stumps and logs where trees were felled).
                w.plants[px] = Plant::new(pos, PlantKind::Stump, 1.0, variant);
                crate::systems::rites::tree_felled(w, who, pos);
                if (variant / 3).is_multiple_of(3) {
                    let spot = [(1, 0), (0, 1), (-1, 0), (0, -1)]
                        .iter()
                        .map(|(dx, dy)| pos.offset(*dx, *dy, 0))
                        .find(|q| {
                            w.grid.walkable(*q)
                                && !w.plants.iter().any(|p| p.pos == *q && p.alive)
                                && !w.structures.iter().any(|s| s.pos == *q)
                                && !w.designations.iter().any(|d| d.pos == *q)
                        });
                    if let Some(q) = spot {
                        w.plants
                            .push(Plant::new(q, PlantKind::FallenLog, 1.0, variant / 9));
                    }
                }
                let logs = 2 + (growth * 3.0) as u32;
                let drop = w.agents[i].pos;
                let id = w.create_item_of(
                    ItemKind::Log,
                    wood,
                    logs,
                    ItemPlace::Ground(drop),
                    0.6,
                    "chop",
                );
                w.agents[i].knowledge.note(
                    crate::mind::knowledge::Known::Item(id),
                    drop,
                    tick,
                    ItemKind::Log as u16,
                );
                w.events.push(
                    tick,
                    Some(pos),
                    Severity::Minor,
                    EventKind::Harvested {
                        agent: who,
                        at: pos,
                    },
                );
            }
        }
        DesignationKind::Mine => {
            let t = w.grid.get(pos);
            // Damp stone, water close behind it: a cautious digger leaves it (#390).
            if t.shape == Shape::Wall && crate::aquifer::left_damp(w, i, dix, pos) {
                return;
            }
            // The roof: would this cut leave part of the excavation unheld?
            if t.shape == Shape::Wall {
                let unheld = crate::underground::unheld_if_opened(w, pos);
                // #308: one whose rock-sense has woken will not cut a face that would bring the
                // roof down, and cannot say why (resonance.rs).
                if crate::resonance::balks(w, i, pos, &unheld) {
                    let d = &mut w.designations[dix];
                    d.work_left = d.work_total;
                    d.failures += 3;
                    return;
                }
                if !unheld.is_empty() {
                    if w.agents[i]
                        .techniques
                        .knows(crate::mind::Technique::Shoring)
                    {
                        // Prop first and come back to this cut when the roof is held; with
                        // nowhere to set a prop, leave the cut alone.
                        let can = crate::underground::prop_first(w, i, &unheld);
                        let d = &mut w.designations[dix];
                        d.work_left = d.work_total;
                        d.failures += if can { 1 } else { 3 };
                        return;
                    }
                    let who = w.agents[i].id;
                    let mut r = crate::rng::stream(
                        w.meta.seed,
                        &[crate::rng::purpose::AGENT_WORK, who.0 as u64, tick, 0xCA7E],
                    );
                    if r.gen::<f32>() < 0.35 {
                        let fall: Vec<_> = unheld.into_iter().filter(|q| *q != pos).collect();
                        if !fall.is_empty() {
                            crate::underground::collapse(w, i, pos, &fall);
                            return;
                        }
                    }
                }
            }
            if t.shape == Shape::Wall {
                let yields = w.grid.materials.get(t.material).yields;
                let rock = t.material;
                let open = Tile {
                    material: Material::AIR,
                    shape: Shape::Open,
                    flags: TileFlags::NONE,
                };
                w.grid.set(pos, open);
                w.dug.insert(pos);
                let drop = w.agents[i].pos;
                let found = match yields {
                    Some(Material::IRON_ORE) => {
                        w.create_item_of(
                            ItemKind::IronOre,
                            Material::IRON_ORE,
                            2,
                            ItemPlace::Ground(drop),
                            0.6,
                            "mine",
                        );
                        Some("iron ore".to_string())
                    }
                    Some(Material::COPPER_ORE) => {
                        w.create_item_of(
                            ItemKind::CopperOre,
                            Material::COPPER_ORE,
                            2,
                            ItemPlace::Ground(drop),
                            0.6,
                            "mine",
                        );
                        Some("copper ore".to_string())
                    }
                    Some(m @ (Material::TIN_ORE | Material::SILVER_ORE | Material::GOLD_ORE)) => {
                        w.create_item_of(ItemKind::Ore, m, 2, ItemPlace::Ground(drop), 0.6, "mine");
                        Some(w.grid.materials.get(m).name.clone())
                    }
                    Some(Material::COAL) => {
                        w.create_item_of(
                            ItemKind::Coal,
                            Material::COAL,
                            2,
                            ItemPlace::Ground(drop),
                            0.6,
                            "mine",
                        );
                        Some("coal".to_string())
                    }
                    Some(Material::GEM_ROCK) => {
                        // The gem and the stone round it follow the rock the pocket lies in
                        // (#267): the deep is basalt, gabbro or gneiss by region.
                        let host = w.grid.rock_at(w.grid.col(pos.x, pos.y), pos.z);
                        let host = if w.grid.materials.get(host).kind
                            == crate::world::material::MaterialKind::Stone
                        {
                            host
                        } else {
                            Material::BASALT
                        };
                        let gem = crate::world::material::gem_in(w.meta.seed, pos, host);
                        w.create_item_of(
                            ItemKind::Gem,
                            gem,
                            1,
                            ItemPlace::Ground(drop),
                            0.7,
                            "mine",
                        );
                        w.create_item_of(
                            ItemKind::RoughStone,
                            host,
                            1,
                            ItemPlace::Ground(drop),
                            0.6,
                            "mine",
                        );
                        Some("rough gems".to_string())
                    }
                    Some(Material::STONE_BLOCK) => {
                        // Stone of the rock it was cut from (C32), rough until it is dressed
                        // at a bench (C67).
                        let id = w.create_item_of(
                            ItemKind::RoughStone,
                            rock,
                            2,
                            ItemPlace::Ground(drop),
                            0.6,
                            "mine",
                        );
                        w.agents[i].knowledge.note(
                            crate::mind::knowledge::Known::Item(id),
                            drop,
                            tick,
                            ItemKind::RoughStone as u16,
                        );
                        None
                    }
                    _ => None,
                };
                let sev = if found.is_some() {
                    Severity::Notable
                } else {
                    Severity::Minor
                };
                let eid = w.events.push(
                    tick,
                    Some(pos),
                    sev,
                    EventKind::Mined {
                        agent: who,
                        at: pos,
                        found: found.clone(),
                        rock: Some(rock),
                    },
                );
                if let Some(f) = found {
                    discover(w, who, pos, format!("{f} here"), eid);
                }
                // Did this open into a natural (undug) cave?
                let into_cave = [(1, 0), (-1, 0), (0, 1), (0, -1)].iter().any(|(dx, dy)| {
                    let q = pos.offset(*dx, *dy, 0);
                    q.z <= w.grid.ground_z(q.x, q.y)
                        && w.grid.get(q).shape == Shape::Open
                        && !w.dug.contains(&q)
                });
                if into_cave {
                    // One of the great chambers, by its name, or a cave.
                    let what = [(1, 0), (-1, 0), (0, 1), (0, -1)]
                        .iter()
                        .find_map(|(dx, dy)| {
                            let q = pos.offset(*dx, *dy, 0);
                            w.grid.caverns.iter().find(|c| c.contains(q))
                        })
                        .map(|c| {
                            format!(
                                "{}, a cavern grown with pale fungus{}",
                                c.name,
                                if c.pool { " around a still pool" } else { "" }
                            )
                        })
                        .unwrap_or_else(|| "a cave".to_string());
                    // Already known to their people: not a discovery again.
                    let known = w.agents[i]
                        .community
                        .and_then(|c| w.communities.iter().find(|x| x.id == c))
                        .is_some_and(|c| {
                            c.discoveries
                                .iter()
                                .any(|(k, p, _)| *k == what && p.dist(pos) < 30)
                        });
                    if !known {
                        let eid = w.events.push(
                            tick,
                            Some(pos),
                            Severity::Major,
                            EventKind::Discovered {
                                agent: who,
                                at: pos,
                                what: what.clone(),
                            },
                        );
                        discover(w, who, pos, what, eid);
                        w.agents[i].remember(
                            tick,
                            MemKind::Discovery,
                            Subject::Place(pos),
                            0.6,
                            0.8,
                            Some(eid),
                        );
                    }
                }
            }
        }
        DesignationKind::DigStairs => {
            let t = w.grid.get(pos);
            if t.shape == Shape::Wall && crate::aquifer::left_damp(w, i, dix, pos) {
                return;
            }
            if t.shape == Shape::Wall {
                let yields = w.grid.materials.get(t.material).yields;
                w.grid.set(
                    pos,
                    Tile {
                        material: t.material,
                        shape: Shape::StairsUp,
                        flags: TileFlags::NONE,
                    },
                );
                let above = pos.offset(0, 0, 1);
                let ta = w.grid.get(above);
                let new_above = match ta.shape {
                    Shape::StairsUp => Shape::StairsBoth,
                    _ => Shape::StairsDown,
                };
                w.grid.set(
                    above,
                    Tile {
                        material: if ta.shape == Shape::Open {
                            Material::STONE_BLOCK
                        } else {
                            ta.material
                        },
                        shape: new_above,
                        flags: TileFlags::NONE,
                    },
                );
                let drop = w.agents[i].pos;
                if let Some(Material::STONE_BLOCK) = yields {
                    let id = w.create_item_of(
                        ItemKind::RoughStone,
                        t.material,
                        2,
                        ItemPlace::Ground(drop),
                        0.6,
                        "dig",
                    );
                    w.agents[i].knowledge.note(
                        crate::mind::knowledge::Known::Item(id),
                        drop,
                        tick,
                        ItemKind::RoughStone as u16,
                    );
                }
                w.events.push(
                    tick,
                    Some(pos),
                    Severity::Notable,
                    EventKind::Mined {
                        agent: who,
                        at: pos,
                        found: Some("a way down".into()),
                        rock: None,
                    },
                );
            }
        }
        DesignationKind::Sow => {
            if !w.plants.iter().any(|p| p.pos == pos && p.alive) {
                let below = pos.offset(0, 0, -1);
                let mut t = w.grid.get(below);
                if t.shape == Shape::Wall {
                    t.material = Material::SOIL;
                    w.grid.set(below, t);
                }
                let variant = (pos.x * 31 + pos.y * 17) as u32;
                // The crop that suits this ground (#286).
                let crop = crate::systems::plants::crop_for(w, pos);
                w.plants.push(Plant::new(pos, crop, 0.02, variant));
            }
        }
        DesignationKind::Engrave => {
            crate::engrave::carve(w, i, pos);
            w.agents[i].skills.practise(Skill::Construction, 0.5);
        }
        DesignationKind::DigEarth => {
            // Clay from a wet bank, sand from sand (C34); the ground is left as it was.
            let ground = w.grid.get(pos.offset(0, 0, -1)).material;
            let got = match ground {
                Material::WET_SOIL | Material::MUD | Material::CLAY => {
                    Some((ItemKind::Clay, Material::CLAY))
                }
                Material::SAND => Some((ItemKind::Sand, Material::SAND)),
                _ => None,
            };
            if let Some((kind, m)) = got {
                let drop = w.agents[i].pos;
                let id = w.create_item_of(kind, m, 3, ItemPlace::Ground(drop), 0.6, "dig");
                w.agents[i].knowledge.note(
                    crate::mind::knowledge::Known::Item(id),
                    drop,
                    tick,
                    kind as u16,
                );
                w.agents[i].skills.practise(Skill::Mining, 0.2);
            }
        }
        DesignationKind::DigChannel => {
            // One tile of a ditch (channels.rs): the ground is left as it was, the cut recorded.
            crate::channels::dug(w, pos);
            w.agents[i].skills.practise(Skill::Mining, 0.2);
        }
        DesignationKind::Harvest => {
            // What a plant gives when cut, from its row of data (plants.rs `lore`, #286): reeds,
            // nettles and flax fibre for the loom, dye plants dyestuff, healing herbs a
            // poultice in the hands of one who knows herb lore (roots-and-herbs otherwise),
            // and a field crop its harvest.
            if let Some(px) = w.plants.iter().position(|p| {
                p.pos == pos
                    && p.alive
                    && (p.kind.is_crop()
                        || matches!(
                            p.kind.lore().gives,
                            crate::plants::Gives::Fibre(_)
                                | crate::plants::Gives::Dye(_)
                                | crate::plants::Gives::Medicine
                        ))
            }) {
                let kind = w.plants[px].kind;
                let lore = kind.lore();
                // A crop is harvested grown (settlement.rs posts it then); a wild plant is cut.
                w.plants.remove(px);
                let skill = w.agents[i].skills.level(Skill::Farming);
                let knows_herbs = {
                    let t = &w.agents[i].techniques;
                    t.knows(crate::mind::Technique::HerbLore)
                        || t.knows(crate::mind::Technique::Poultice)
                };
                let (item, material, n, quality) = match lore.gives {
                    crate::plants::Gives::Crop(k, m) => {
                        (k, m, lore.yields + (skill / 2.0) as u32, 0.5 + skill / 40.0)
                    }
                    crate::plants::Gives::Fibre(m) => (ItemKind::Fibre, m, lore.yields, 0.5),
                    crate::plants::Gives::Dye(m) => (ItemKind::Dye, m, lore.yields, 0.5),
                    _ if knows_herbs => (ItemKind::Poultice, Material::AIR, 1, 0.5),
                    _ => (ItemKind::Root, Material::AIR, lore.yields, 0.4),
                };
                let drop = w.agents[i].pos;
                let id = w.create_item_of(
                    item,
                    material,
                    n,
                    ItemPlace::Ground(drop),
                    quality,
                    "harvest",
                );
                w.agents[i].knowledge.note(
                    crate::mind::knowledge::Known::Item(id),
                    drop,
                    tick,
                    item as u16,
                );
                if !kind.is_crop() {
                    w.agents[i].skills.practise(Skill::Farming, 0.2);
                }
                // Every cut is recorded, as a felling is (the plant census reads it, #286).
                w.events.push(
                    tick,
                    Some(pos),
                    Severity::Minor,
                    EventKind::Harvested {
                        agent: who,
                        at: pos,
                    },
                );
            }
        }
        DesignationKind::Douse => {
            // A bucket of water on the fire here and its neighbours.
            let mut put_out = 0;
            for (dx, dy) in [(0, 0), (1, 0), (-1, 0), (0, 1), (0, -1)] {
                if w.extinguish(pos.offset(dx, dy, 0)) {
                    put_out += 1;
                }
            }
            w.agents[i]
                .techniques
                .outcome(crate::mind::Technique::CarryWater, put_out > 0, tick);
            if put_out > 0 {
                w.agents[i].skills.practise(Skill::Hauling, 0.2);
                w.events.push(
                    tick,
                    Some(pos),
                    Severity::Notable,
                    EventKind::Doused {
                        agent: who,
                        at: pos,
                    },
                );
            }
            let _ = station;
        }
        DesignationKind::FetchWater => {
            // A vessel of water poured into the community's store (#433).
            let cid = w.designations[dix].community.or(w.agents[i].community);
            if let Some(c) = cid.and_then(|cid| w.communities.iter_mut().find(|c| c.id == cid)) {
                c.water = (c.water + crate::water::VESSEL).min(crate::water::STORE_MAX);
            }
            w.agents[i].skills.practise(Skill::Hauling, 0.1);
            let _ = station;
        }
        DesignationKind::BeatOut => {
            // Beating at flames: it works more often for someone who knows how. It can scorch.
            let skill = w.agents[i]
                .techniques
                .skill(crate::mind::Technique::BeatFlames);
            let mut r = crate::rng::stream(
                w.meta.seed,
                &[crate::rng::purpose::AGENT_WORK, 0xBEA7, who.0 as u64, tick],
            );
            let won = rand::Rng::gen::<f32>(&mut r) < 0.3 + 0.55 * skill && w.extinguish(pos);
            w.agents[i]
                .techniques
                .outcome(crate::mind::Technique::BeatFlames, won, tick);
            if won {
                // One Notable line a fire (#221: "beat out the flames" three times in twelve
                // minutes): the first tile this hour is Notable, the rest of the fire Minor.
                let key = 0x7200_0000;
                let again = w.agents[i]
                    .work_cooldowns
                    .get(&key)
                    .is_some_and(|t| *t > tick);
                w.agents[i]
                    .work_cooldowns
                    .insert(key, tick + crate::time::TICKS_PER_HOUR);
                w.events.push(
                    tick,
                    Some(pos),
                    if again {
                        Severity::Minor
                    } else {
                        Severity::Notable
                    },
                    EventKind::BeatOut {
                        agent: who,
                        at: pos,
                    },
                );
            } else if rand::Rng::gen::<f32>(&mut r) < 0.2 {
                crate::systems::health::wound(
                    w,
                    i,
                    crate::body::Part::ARM_L,
                    crate::body::WoundKind::Burn,
                    0.1 + rand::Rng::gen::<f32>(&mut r) * 0.15,
                    0.0,
                    "beating at a fire",
                    None,
                );
            }
        }
        DesignationKind::Tend(patient) => {
            // The poultice in hand goes on the patient's wounds; a healer's care lingers a day.
            let poultice = w.items.iter().position(|it| {
                it.place == ItemPlace::Carried(who) && it.kind == ItemKind::Poultice
            });
            if let Some(ix) = poultice {
                let id = w.items[ix].id;
                w.consume_item(id, 1, "poultice");
            }
            // Without a poultice (#418), whoever came nurses them: fed, watered and kept warm,
            // tended for a day, and nothing more.
            let nursing = poultice.is_none();
            if nursing {
                if let Some(j) = w.agents.iter().position(|o| o.id == patient && o.alive) {
                    w.agents[j].tended_until = tick + crate::time::TICKS_PER_DAY;
                    let ppos = w.agents[j].pos;
                    w.events.push(
                        tick,
                        Some(ppos),
                        Severity::Notable,
                        EventKind::Tended {
                            healer: who,
                            patient,
                        },
                    );
                    // A tend done counts toward tending as a duty (#602, duties.rs).
                    crate::duties::credit(
                        w,
                        i,
                        crate::society::Role::Healer,
                        crate::duties::TEND_CREDIT,
                    );
                }
            } else if let Some(j) = w.agents.iter().position(|o| o.id == patient && o.alive) {
                let skill = w.agents[i].skills.level(Skill::Cooking);
                // The old herb lore (C13): the fever breaks sooner, wounds close faster.
                let lore = w.agents[i]
                    .techniques
                    .bonus(crate::mind::Technique::HerbLore);
                let seed = w.meta.seed;
                let p = &mut w.agents[j];
                // Each wound its own treatment (C74): stitched, splinted, salved or dressed.
                for wd in p.body.wounds.iter_mut() {
                    wd.infected = false;
                    wd.severity = (wd.severity - 0.15 - skill / 100.0 - 0.15 * lore).max(0.02);
                    wd.bleeding = 0.0;
                    wd.treated = Some(crate::body::Treatment::for_wound(wd.kind, wd.tissue));
                }
                if let Some(ill) = p.body.illness.as_mut() {
                    ill.severity = (ill.severity - 0.2 - 0.25 * lore).max(0.02);
                    // The right answer to it, or not (#328): herb lore and practice know it.
                    let mut r = crate::rng::stream(
                        seed,
                        &[
                            crate::rng::purpose::AGENT_WORK,
                            patient.0 as u64,
                            tick,
                            0x7E4D,
                        ],
                    );
                    use rand::Rng;
                    let right = r.gen::<f32>() < (0.45 + skill / 40.0 + 0.4 * lore).min(0.95);
                    ill.treated = Some(right);
                }

                p.tended_until = tick + crate::time::TICKS_PER_DAY;
                let ppos = p.pos;
                p.remember(
                    tick,
                    crate::mind::MemKind::Company,
                    crate::mind::Subject::Person(who),
                    0.6,
                    0.7,
                    None,
                );
                w.agents[i].skills.practise(Skill::Cooking, 0.3);
                if lore > 0.0 {
                    crate::systems::learn::witnessed(w, i, crate::mind::Technique::HerbLore);
                }
                w.events.push(
                    tick,
                    Some(ppos),
                    Severity::Notable,
                    EventKind::Tended {
                        healer: who,
                        patient,
                    },
                );
                // A tend done counts toward tending as a duty (#602, duties.rs).
                crate::duties::credit(
                    w,
                    i,
                    crate::society::Role::Healer,
                    crate::duties::TEND_CREDIT,
                );
            }
            let _ = station;
        }
        DesignationKind::Craft(r) => {
            // Inputs were consumed as they were delivered; make the output by the station.
            let rc = crate::work::recipes::recipe(r);
            let skill = w.agents[i].skills.level(Skill::Crafting);
            // One who does not know the craft is trying it (C70, design rule 3): the work
            // comes out only if the try works; else the materials are spoiled.
            let trying =
                crate::work::recipes::craft_of(r).filter(|t| !w.agents[i].techniques.knows(*t));
            let good = match trying {
                Some(t) => {
                    let spoiled = rc
                        .inputs
                        .iter()
                        .map(|(k, n)| format!("{n} {}", k.label()))
                        .collect::<Vec<_>>()
                        .join(" and ");
                    crate::systems::crafts::tried(w, i, t, &spoiled)
                }
                None => true,
            };
            if good {
                let base = 0.25 + skill / 24.0;
                // Soft copper takes no fine edge, bronze a better one, stone a crude one.
                use crate::work::recipes as rcs;
                let cap = match r {
                    rcs::COPPER_TOOL | rcs::COPPER_WEAPON => 0.6,
                    rcs::BRONZE_TOOL | rcs::BRONZE_WEAPON | rcs::BRONZE_ARMOUR => 0.85,
                    // #274: by the stone it is knapped from; the deep's takes the best edge.
                    rcs::STONE_TOOL => crate::tiers::stone_edge(
                        w,
                        d.requirements
                            .first()
                            .map(|q| q.material)
                            .unwrap_or(Material::AIR),
                    ),
                    _ => 1.0,
                };
                // The old crafts (C13): an edge set truly, cloth woven close, a stronger poultice.
                let craft_t = match rc.output.0 {
                    ItemKind::Tool
                    | ItemKind::Saw
                    | ItemKind::Hammer
                    | ItemKind::SledgeHammer
                    | ItemKind::Chisel
                    | ItemKind::Needle
                    | ItemKind::Weapon
                    | ItemKind::Armour => Some(crate::mind::Technique::Smithing),
                    ItemKind::Cloth | ItemKind::Clothes => Some(crate::mind::Technique::Weaving),
                    ItemKind::Poultice => Some(crate::mind::Technique::HerbLore),
                    _ => None,
                };
                let craft_bonus = craft_t
                    .map(|t| w.agents[i].techniques.bonus(t))
                    .unwrap_or(0.0);
                let quality = (base + w.agents[i].tool_quality * 0.1 + 0.2 * craft_bonus)
                    .clamp(0.05, (cap + 0.15 * craft_bonus).min(1.0));
                // Work done in a fey mood (systems/moods.rs) is the finest of a life.
                let fey = w.agents[i].fey.as_ref().is_some_and(|f| f.work == did);
                let quality = if fey { 1.0 } else { quality };
                if let Some(t) = craft_t {
                    if craft_bonus > 0.0 {
                        crate::systems::learn::witnessed(w, i, t);
                    }
                }
                // What it is made of follows what went in (C32): the first input's material,
                // worked (an ore becomes its metal); the recipe's own material if nothing carried one.
                let fed = d
                    .requirements
                    .first()
                    .map(|q| q.material)
                    .unwrap_or(Material::AIR);
                let fed = w.grid.materials.get(fed).becomes.unwrap_or(fed);
                let material = match rc.makes {
                    crate::work::recipes::Makes::Fixed(m) => m,
                    crate::work::recipes::Makes::FromInput(_) if fed != Material::AIR => fed,
                    crate::work::recipes::Makes::FromInput(m) => m,
                };
                let drop = w.agents[i].pos;
                // A storied piece is its own thing from the start, not one of a stack (legend.rs).
                let id = if crate::legend::storied(rc.output.0) {
                    w.create_lone_item(
                        rc.output.0,
                        rc.output.1,
                        ItemPlace::Ground(drop),
                        quality,
                        "craft",
                    )
                } else {
                    w.create_item_of(
                        rc.output.0,
                        material,
                        rc.output.1,
                        ItemPlace::Ground(drop),
                        quality,
                        "craft",
                    )
                };
                if let Some(it) = w.items.iter_mut().find(|it| it.id == id) {
                    it.material = material;
                }
                if crate::legend::storied(rc.output.0) {
                    crate::legend::made(w, id, i);
                }
                if fey {
                    crate::systems::moods::fulfilled(w, i, id);
                }
                // A ring keeps the gem it was set with (C34).
                if rc.output.0 == ItemKind::Jewel {
                    let gem = d
                        .requirements
                        .get(1)
                        .map(|q| q.material)
                        .unwrap_or(Material::AIR);
                    let gem_name = w.grid.materials.get(gem).name.clone();
                    if let Some(s) = w
                        .items
                        .iter_mut()
                        .find(|it| it.id == id)
                        .and_then(|it| it.story.as_mut())
                    {
                        if gem != Material::AIR {
                            s.origin = format!("set with a {gem_name}");
                        }
                    }
                }
                w.agents[i].knowledge.note(
                    crate::mind::knowledge::Known::Item(id),
                    drop,
                    tick,
                    rc.output.0 as u16,
                );
                w.agents[i].skills.practise(Skill::Crafting, 0.5);
                w.events.push(
                    tick,
                    Some(pos),
                    Severity::Minor,
                    EventKind::Crafted {
                        agent: who,
                        kind: rc.output.0,
                        tier: ItemKind::tier(quality),
                    },
                );
            }
            let _ = station;
        }
        DesignationKind::Cook => {
            // One carried raw food → one meal (quality from skill), dropped by the hearth: a
            // rabbit on a stick is a meal (FIXLOG 2026-10-20 #172, C62, the maintainer's call; it took
            // "2", which was 1 when one was carried). The food is what the plan picked up, not
            // the first cookable thing in hand.
            let planned = match &w.agents[i].activity {
                Activity::Working { plan } => plan.steps.iter().find_map(|s| match s {
                    crate::work::Step::PickUp { item, .. } => w
                        .items
                        .iter()
                        .find(|it| it.id == *item)
                        .map(|it| (it.kind, it.material)),
                    _ => None,
                }),
                _ => None,
            };
            let carried = |it: &crate::items::Item| {
                it.place == ItemPlace::Carried(who) && it.kind.is_cookable()
            };
            if let Some(ix) = w
                .items
                .iter()
                .position(|it| {
                    carried(it) && planned.is_none_or(|(k, m)| it.kind == k && it.material == m)
                })
                .or_else(|| w.items.iter().position(carried))
            {
                let id = w.items[ix].id;
                // The meal remembers what went into it (C34).
                let of = crate::world::material::food_of(w.items[ix].kind);
                w.consume_item(id, 1, "cook");
                let skill = w.agents[i].skills.level(Skill::Cooking);
                let drop = w.agents[i].pos;
                // Old hearth cooking (C13): better meals.
                let hearth = w.agents[i]
                    .techniques
                    .bonus(crate::mind::Technique::Hearthcraft);
                let mid = w.create_item_of(
                    ItemKind::Meal,
                    of,
                    1,
                    ItemPlace::Ground(drop),
                    (0.5 + skill / 30.0 + 0.2 * hearth).min(1.0),
                    "cook",
                );
                if hearth > 0.0 {
                    crate::systems::learn::witnessed(w, i, crate::mind::Technique::Hearthcraft);
                }
                w.agents[i].knowledge.note(
                    crate::mind::knowledge::Known::Item(mid),
                    drop,
                    tick,
                    ItemKind::Meal as u16,
                );
                w.events.push(
                    tick,
                    Some(pos),
                    Severity::Minor,
                    EventKind::Cooked {
                        agent: who,
                        item: mid,
                    },
                );
            }
            let _ = station;
        }
        DesignationKind::Fish => crate::fish::caught(w, i, d.pos),
        DesignationKind::Snare => crate::traps::set(w, i, d.pos),
        DesignationKind::Tame(beast) => crate::livestock::tamed(w, i, beast),
        DesignationKind::Hunt(prey) => {
            // A body that is not a skeleton is not taken at a stroke (#423): only when the
            // hunter's blows leave too little of it to live; else it gets away.
            let caught = w
                .agents
                .iter()
                .position(|o| o.id == prey && o.alive)
                .filter(|&j| crate::systems::combat::hunted_down(w, i, j));
            if let Some(j) = caught {
                let species = w.agents[j].species;
                let ppos = w.agents[j].pos;
                // Boar and wolves fight back.
                let fight = match species {
                    crate::agents::Species::Boar => 0.25,
                    crate::agents::Species::Wolf => 0.35,
                    crate::agents::Species::Bear => 0.5,
                    _ => 0.0,
                };
                if fight > 0.0 {
                    let mut r = crate::rng::stream(
                        w.meta.seed,
                        &[crate::rng::purpose::AGENT_BIRTH, 0x4EA4, who.0 as u64, tick],
                    );
                    // A spear keeps a cornered beast at its length (C33).
                    let reach = crate::systems::arms::edge(w, i).map(|e| e.1).unwrap_or(0.0);
                    if r.gen::<f32>()
                        < fight
                            * (1.2 - w.agents[i].skills.level(Skill::Animals) / 25.0)
                            * (1.0 - 0.4 * reach.min(1.0))
                    {
                        let part = [
                            crate::body::Part::LEG_L,
                            crate::body::Part::LEG_R,
                            crate::body::Part::ARM_L,
                            crate::body::Part::ARM_R,
                        ][r.gen_range(0..4)];
                        let cause = if species == crate::agents::Species::Boar {
                            "from a boar's tusk"
                        } else {
                            "from a wolf's bite"
                        };
                        let beast = w.agents[j].id;
                        crate::systems::health::wound(
                            w,
                            i,
                            part,
                            crate::body::WoundKind::Cut,
                            0.3 + r.gen::<f32>() * 0.3,
                            0.4,
                            cause,
                            Some(beast),
                        );
                    }
                }
                w.agents[j].alive = false;
                w.agents[j].death = Some("hunted".into());
                w.agents[i].fought_at = w.meta.tick;
                crate::systems::arms::struck_with(w, i, true);
                crate::creatures::slain(w, i, j);
                // A deer is a few days of food for a family, not a single sitting.
                let meat = match species {
                    crate::agents::Species::Deer => 10,
                    crate::agents::Species::Boar => 7,
                    crate::agents::Species::Goat => 6,
                    crate::agents::Species::BlindCaveBear => 8,
                    // Nothing on a slime or a centipede anyone would eat (#423).
                    crate::agents::Species::Slime | crate::agents::Species::Centipede => 0,
                    _ => 2,
                };
                // As much as this one weighs against its kind (#422): a yearling gives less, a
                // big one more.
                let meat = ((meat as f32 * w.agents[j].body_kg(tick)
                    / crate::size::mean_kg(species))
                .round() as u32)
                    .max(u32::from(meat > 0));
                let skill = w.agents[i].skills.level(Skill::Animals);
                let drop = w.agents[i].pos;
                // And its hide, for the tanner (C34).
                if let Some(h) = crate::world::material::hide_of(species) {
                    w.create_item_of(ItemKind::Hide, h, 1, ItemPlace::Ground(drop), 0.6, "hunt");
                }
                if meat > 0 {
                    let id = w.create_item(
                        ItemKind::RawMeat,
                        meat + (skill / 8.0) as u32,
                        ItemPlace::Ground(drop),
                        0.5 + skill / 40.0,
                        "hunt",
                    );
                    w.agents[i].knowledge.note(
                        crate::mind::knowledge::Known::Item(id),
                        drop,
                        tick,
                        ItemKind::RawMeat as u16,
                    );
                }
                w.agents[i].skills.practise(Skill::Animals, 0.5);
                w.events.push(
                    tick,
                    Some(ppos),
                    Severity::Notable,
                    EventKind::Hunted {
                        agent: who,
                        prey: format!("{species:?}").to_lowercase(),
                    },
                );
                // A beast's blood on the hands: now and then, beast fever (#328).
                let mut r = crate::rng::stream(
                    w.meta.seed,
                    &[crate::rng::purpose::AGENT_WORK, who.0 as u64, tick, 0xBEF3],
                );
                if rand::Rng::gen_bool(&mut r, 0.04) {
                    crate::systems::health::catch(w, i, crate::body::IllnessKind::BeastFever);
                }
            }
        }
        DesignationKind::Haul | DesignationKind::DeliverProvisions(_) => {}
    }
    if uses_tool(d.kind) && w.agents[i].tool.is_some() {
        crate::legend::used(w, i, d.kind);
    }
    // Its materials went into the work: nothing to give back when it is swept (#162).
    for r in w.designations[dix].requirements.iter_mut() {
        r.delivered = 0;
    }
    w.designations[dix].done = true;
    let _ = tick;
}

/// Set the stack at `ix` down at `to`: into a stack there of the same kind and material while
/// both fit the kind's stack, averaging its quality and rot, else beside it. A thing with a
/// story is put down beside a stack, never into one, nor a stack into it: either way one story
/// would be lost or stand for many things (legend.rs).
pub fn put_down(w: &mut World, ix: usize, to: crate::ids::TilePos) -> crate::ids::ItemId {
    let id = w.items[ix].id;
    let n = w.items[ix].count;
    let kind = w.items[ix].kind;
    let storied = w.items[ix].story.is_some();
    let material = w.items[ix].material;
    if let Some(tx) = w.items.iter().position(|it| {
        !storied
            && it.story.is_none()
            && it.id != id
            && it.place == ItemPlace::Ground(to)
            && it.kind == kind
            && it.material == material
            // Up to the kind's stack, as `create_item_of` merges (FIXLOG
            // 2026-10-20 #171, C62: hauls made stacks of twenty meals).
            && it.count + n <= kind.max_stack()
    }) {
        let (a, b) = (w.items[tx].count as f32, n as f32);
        w.items[tx].rot = (w.items[tx].rot * a + w.items[ix].rot * b) / (a + b);
        // As good as what went into it, on average (#171: it kept the
        // target's quality, so ten fine meals on ten crude made twenty crude).
        w.items[tx].quality = (w.items[tx].quality * a + w.items[ix].quality * b) / (a + b);
        w.items[tx].count += n;
        let target = w.items[tx].id;
        w.items.remove(ix);
        target
    } else {
        w.items[ix].place = ItemPlace::Ground(to);
        id
    }
}

/// work_cooldowns key: this person last meant to set a lamp below ground.
const LIGHT_KEY: u32 = 0x5000_0001;

/// Working in the dark below ground, someone sets about putting a lamp there: their own order,
/// unless a lamp stands or is wanted within five tiles, at most once in four hours.
fn want_light(w: &mut World, i: usize, here: crate::ids::TilePos) {
    let tick = w.meta.tick;
    if w.agents[i]
        .work_cooldowns
        .get(&LIGHT_KEY)
        .is_some_and(|t| *t > tick)
    {
        return;
    }
    w.agents[i]
        .work_cooldowns
        .insert(LIGHT_KEY, tick + 4 * crate::time::TICKS_PER_HOUR);
    let lamp_near = w
        .structures
        .iter()
        .any(|s| s.kind == StructureKind::Lamp && s.pos.z == here.z && s.pos.dist(here) <= 5)
        || w.designations.iter().any(|d| {
            !d.done
                && d.pos.z == here.z
                && d.pos.dist(here) <= 5
                && matches!(
                    d.kind,
                    DesignationKind::BuildStructure(StructureKind::Lamp, _)
                )
        });
    if lamp_near {
        return;
    }
    let spot = [(0, 1), (1, 0), (-1, 0), (0, -1), (1, 1), (-1, 1)]
        .iter()
        .map(|(dx, dy)| here.offset(*dx, *dy, 0))
        .find(|p| {
            w.grid.walkable(*p)
                && !w.structures.iter().any(|s| s.pos == *p)
                && !w.designations.iter().any(|d| !d.done && d.pos == *p)
        });
    let Some(at) = spot else { return };
    let mut d = crate::systems::settlement::make(
        w,
        DesignationKind::BuildStructure(StructureKind::Lamp, Material::WOOD),
        at,
        7,
    );
    d.owner = Some(w.agents[i].id);
    d.community = w.agents[i].community;
    w.designations.push(d);
    let who = w.agents[i].id;
    w.events.push(
        tick,
        Some(at),
        Severity::Minor,
        EventKind::Chose {
            agent: who,
            what: "set about putting a lamp where they work below".into(),
            because: "because it is too dark down there to see the rock".into(),
        },
    );
}

/// Record a find in the finder's community.
fn discover(
    w: &mut World,
    who: AgentId,
    at: crate::ids::TilePos,
    what: String,
    eid: crate::ids::EventId,
) {
    let Some(cid) = w
        .agents
        .iter()
        .find(|a| a.id == who)
        .and_then(|a| a.community)
    else {
        return;
    };
    if let Some(c) = w.communities.iter_mut().find(|c| c.id == cid) {
        // One record per thing found at a place: a tile can hold a cave and later a vein
        // (issue 28), but the same find is not recorded twice.
        if !c.discoveries.iter().any(|(k, p, _)| *p == at && *k == what) {
            c.discoveries.push((what, at, eid));
            if c.discoveries.len() > 64 {
                c.discoveries.remove(0);
            }
        }
    }
}

/// Move any agent standing on `pos` to the nearest walkable neighbour (not into walls): one
/// from which they can still walk home with `pos` closed, where there is one (#447, FIXLOG
/// 2026-10-28: a builder finishing a wall from its own tile was put off on the far side, on a
/// ledge the wall had been the only way off, and starved there). A builder steps off on the
/// home side.
pub fn displace_from(w: &mut World, pos: crate::ids::TilePos) {
    let open: Vec<crate::ids::TilePos> = [
        (0, 1),
        (1, 0),
        (0, -1),
        (-1, 0),
        (1, 1),
        (-1, 1),
        (1, -1),
        (-1, -1),
    ]
    .iter()
    .map(|(dx, dy)| pos.offset(*dx, *dy, 0))
    .filter(|q| {
        w.grid.walkable(*q)
            && !w
                .designations
                .iter()
                .any(|d| d.pos == *q && !d.done && matches!(d.kind, DesignationKind::BuildWall(_)))
    })
    .collect();
    // No open tile touches it (a pocket walled in on every side, #197 case 8): the nearest open
    // tile within three, so nobody and nothing is left inside the wall.
    let open = if open.is_empty() {
        nearest_open(w, pos).into_iter().collect()
    } else {
        open
    };
    let Some(&first) = open.first() else {
        return;
    };
    let here: Vec<usize> = (0..w.agents.len())
        .filter(|&i| w.agents[i].pos == pos)
        .collect();
    for i in here {
        let home = w.agents[i]
            .community
            .and_then(|c| w.communities.iter().find(|x| x.id == c))
            .map(|c| c.home)
            .unwrap_or(w.meta.founding_site);
        let q = open
            .iter()
            .copied()
            .find(|q| joined_home(w, *q, pos, home))
            .unwrap_or(first);
        let a = &mut w.agents[i];
        a.pos = q;
        a.from = q;
        a.move_total = 0;
        a.path.clear();
    }
    // Whatever lay there (logs from the trees felled to clear the site) is moved aside, so
    // nothing ends up inside a wall where nobody can reach it.
    for it in w.items.iter_mut() {
        if it.place == ItemPlace::Ground(pos) {
            it.place = ItemPlace::Ground(first);
        }
    }
}

/// The nearest walkable tile within three of `pos` (same level, then one level either way), not
/// one a wall is about to be built on: ties go to the lowest (z, y, x). None if all is rock.
fn nearest_open(w: &World, pos: crate::ids::TilePos) -> Option<crate::ids::TilePos> {
    let mut best: Option<(i32, crate::ids::TilePos)> = None;
    for dz in [0, 1, -1] {
        for dy in -3..=3 {
            for dx in -3..=3 {
                let q = pos.offset(dx, dy, dz);
                if q == pos || !w.grid.in_bounds(q) || !w.grid.walkable(q) {
                    continue;
                }
                if w.designations.iter().any(|d| {
                    d.pos == q && !d.done && matches!(d.kind, DesignationKind::BuildWall(_))
                }) {
                    continue;
                }
                let d = dx.abs().max(dy.abs()) + 2 * dz.abs();
                let key = (d, q.z, q.y, q.x);
                if best.is_none_or(|(bd, bq)| key < (bd, bq.z, bq.y, bq.x)) {
                    best = Some((d, q));
                }
            }
        }
    }
    best.map(|(_, q)| q)
}

/// Tiles walked out from a displaced person's landing before it is taken to be open country.
const JOINED_SEARCH: usize = 3000;

/// Whether someone at `from` can walk to `home` with `closed` shut (#447): a search over the
/// ground as it will be, which finds home, or walks out over more than `JOINED_SEARCH` tiles
/// (open country, not a ledge or a pocket).
fn joined_home(
    w: &World,
    from: crate::ids::TilePos,
    closed: crate::ids::TilePos,
    home: crate::ids::TilePos,
) -> bool {
    let mut seen = std::collections::BTreeSet::new();
    let mut queue = std::collections::VecDeque::new();
    seen.insert(from);
    queue.push_back(from);
    let mut nb = Vec::new();
    while let Some(p) = queue.pop_front() {
        if p == home || seen.len() > JOINED_SEARCH {
            return true;
        }
        crate::nav::step::neighbours(&w.grid, p, &mut nb);
        for s in &nb {
            if s.to != closed && seen.insert(s.to) {
                queue.push_back(s.to);
            }
        }
    }
    false
}

/// Items an agent carries that are not food (used by the renderer).
pub fn carried_kind(w: &World, who: AgentId) -> Option<ItemKind> {
    w.items
        .iter()
        .find(|it| it.place == ItemPlace::Carried(who))
        .map(|it| it.kind)
}

/// What a thing was built of: the material of the first stack delivered for it (C32, C56), or
/// what the order named when nothing was recorded (an old save, a built-in-place test).
fn built_of(d: &crate::work::Designation, named: Material) -> Material {
    d.requirements
        .first()
        .map(|q| q.material)
        .filter(|m| *m != Material::AIR)
        .unwrap_or(named)
}
