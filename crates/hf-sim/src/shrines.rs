//! Shrines (#297, epic #376, ask 1): a community that keeps its observances comes to want a
//! place to keep them. Someone who heeds the signs proposes a shrine (systems/projects.rs
//! `shrine`, with why); the planner finds it a spot on the community's own ground and orders it
//! built, of stone where the houses are stone and of timber where they are not (`plan`). Once it
//! stands it is the community's (`step`), and the season's observance is kept there
//! (observances.rs): those free to go walk to it an hour before noon.
//!
//! A shrine is kept up by being used: it wears a twentieth a day and is made good whenever an
//! observance is kept at it, so a year of lapsed observances lets it fall. A timber one burns.
//! A community that moves camp leaves its shrine behind. Each loss is on record (`ShrineLost`),
//! those who heed the signs take it hard, and whoever proposes the next one remembers it.
//! FIXLOG 2026-10-27 "nowhere to keep the observances".

use crate::events::{EventKind, Severity};
use crate::ids::{CommunityId, TilePos};
use crate::mind::beliefs::Topic;
use crate::mind::ThoughtKind;
use crate::project::{open_of, open_of_mut, ProjectKind};
use crate::structures::StructureKind;
use crate::systems::settlement::{find_flat_area, Here};
use crate::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use crate::world::{Material, Shape};
use crate::World;
use serde::{Deserialize, Serialize};

/// What a shrine loses a day untended: a year (four seasons) without an observance kept at it
/// and it falls (docs/TIME.md: a season is five days).
pub const WEAR_A_DAY: f32 = 0.05;

/// A community's shrine.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ShrineSite {
    pub community: CommunityId,
    pub pos: TilePos,
    pub since: u64,
}

/// The standing shrine of `cid`, if it has one.
pub fn of(w: &World, cid: CommunityId) -> Option<TilePos> {
    w.faith
        .shrines
        .iter()
        .find(|s| s.community == cid)
        .map(|s| s.pos)
        .filter(|p| standing(w, *p))
}

fn standing(w: &World, p: TilePos) -> bool {
    w.structures
        .iter()
        .any(|s| s.pos == p && s.kind == StructureKind::Shrine && s.progress >= 1.0)
}

/// The planner's part (systems/settlement.rs): a spot for a wanted shrine on the community's
/// own ground, and the order to build it.
pub fn plan(w: &mut World, cid: CommunityId, home: TilePos) {
    let Some(pid) = open_of(w, cid, |k| *k == ProjectKind::Shrine)
        .filter(|p| p.work.is_none())
        .map(|p| p.id)
    else {
        return;
    };
    let here = Here::of(w, cid, home);
    // Of what the first house is walled with, as the wall round the houses is.
    let mat = w
        .communities
        .iter()
        .find(|c| c.id == cid)
        .and_then(|c| c.house_sites.first().copied())
        .map(|o| w.grid.get(o))
        .filter(|t| t.shape == Shape::Wall && t.material == Material::STONE_BLOCK)
        .map(|_| Material::STONE_BLOCK)
        .unwrap_or(Material::WOOD);
    let spot = [
        (0, -6),
        (6, -6),
        (-6, -6),
        (0, 7),
        (7, 7),
        (-7, 7),
        (9, 0),
        (-9, 0),
    ]
    .into_iter()
    .map(|(dx, dy)| home.offset(dx, dy, 0))
    .find_map(|n| find_flat_area(w, n, 1, 1, 1).filter(|p| here.holds(*p)));
    let tick = w.meta.tick;
    let Some(spot) = spot else {
        // No place for it: given up, and not wanted again for a season of seasons.
        let by = open_of(w, cid, |k| *k == ProjectKind::Shrine).map(|p| p.by);
        if let Some(p) = open_of_mut(w, cid, |k| *k == ProjectKind::Shrine) {
            p.done = true;
        }
        if let Some(c) = w.communities.iter_mut().find(|c| c.id == cid) {
            c.no_site.retain(|(k, _)| *k != StructureKind::Shrine);
            c.no_site
                .push((StructureKind::Shrine, tick + 60 * TICKS_PER_DAY));
        }
        if let Some(agent) = by {
            w.events.push(
                tick,
                Some(home),
                Severity::Notable,
                EventKind::Chose {
                    agent,
                    what: "gave up on the shrine".into(),
                    because: "found no place for it on their own ground".into(),
                },
            );
        }
        return;
    };
    let d = crate::systems::settlement::make(
        w,
        crate::work::DesignationKind::BuildStructure(StructureKind::Shrine, mat),
        spot,
        5,
    );
    let id = d.id;
    w.designations.push(d);
    if let Some(p) = w.projects.iter_mut().find(|p| p.id == pid) {
        p.work = Some(id);
        p.site = Some(spot);
    }
}

/// Hourly: a shrine built for a community is taken as theirs, and one lost is told; once a
/// day, at dawn, each wears.
pub fn step(w: &mut World) {
    let tick = w.meta.tick;
    if tick % TICKS_PER_HOUR != 41 {
        return;
    }
    claim(w);
    losses(w);
    if tick % TICKS_PER_DAY == 6 * TICKS_PER_HOUR + 41 {
        wear(w);
    }
}

/// A shrine finished at a community's wanted site is theirs.
fn claim(w: &mut World) {
    let tick = w.meta.tick;
    let built: Vec<(CommunityId, TilePos)> = w
        .projects
        .iter()
        .filter(|p| p.kind == ProjectKind::Shrine)
        .filter_map(|p| p.site.map(|s| (p.community, s)))
        .filter(|(_, s)| standing(w, *s))
        // Not one left behind at an old camp: it would be claimed and lost again each hour.
        .filter(|(c, s)| {
            w.communities
                .iter()
                .find(|x| x.id == *c && !x.dissolved)
                .is_some_and(|x| Here::of(w, *c, x.home).holds(*s))
        })
        .collect();
    for (cid, pos) in built {
        if w.faith.shrines.iter().any(|s| s.pos == pos) || of(w, cid).is_some() {
            continue;
        }
        w.faith.shrines.retain(|s| s.community != cid);
        w.faith.shrines.push(ShrineSite {
            community: cid,
            pos,
            since: tick,
        });
    }
}

/// A shrine gone (burned, broken) or left behind by a move is lost.
fn losses(w: &mut World) {
    let all = w.faith.shrines.clone();
    for s in all {
        let Some(home) = w
            .communities
            .iter()
            .find(|c| c.id == s.community && !c.dissolved)
            .map(|c| c.home)
        else {
            w.faith.shrines.retain(|x| x.pos != s.pos);
            continue;
        };
        let why = if !standing(w, s.pos) {
            if w.scorched.contains_key(&s.pos.offset(0, 0, -1)) {
                "it burned"
            } else {
                "it was broken down"
            }
        } else if !Here::of(w, s.community, home).holds(s.pos) {
            "it was left behind at the old camp"
        } else {
            continue;
        };
        lose(w, s.community, s.pos, why);
    }
}

fn lose(w: &mut World, cid: CommunityId, pos: TilePos, why: &str) {
    let tick = w.meta.tick;
    w.faith.shrines.retain(|x| x.pos != pos);
    let ev = w.events.push(
        tick,
        Some(pos),
        Severity::Major,
        EventKind::ShrineLost {
            community: cid,
            why: why.to_string(),
        },
    );
    w.faith.lost.retain(|(c, _, _)| *c != cid);
    w.faith.lost.push((cid, ev, why.to_string()));
    // Those who heed the signs take it hard.
    for k in 0..w.agents.len() {
        let a = &w.agents[k];
        if a.alive
            && a.community == Some(cid)
            && a.beliefs.lean(Topic::DeitySigns) >= crate::observances::LEADS_AT
        {
            w.agents[k]
                .thoughts
                .add(ThoughtKind::ShrineLost, tick, Some(ev));
        }
    }
}

/// Each shrine wears; one worn through falls.
fn wear(w: &mut World) {
    let all = w.faith.shrines.clone();
    for s in all {
        let Some(st) = w
            .structures
            .iter_mut()
            .find(|x| x.pos == s.pos && x.kind == StructureKind::Shrine)
        else {
            continue;
        };
        st.condition = (st.condition - WEAR_A_DAY).max(0.0);
        if st.condition <= 0.0 {
            let mat = st.material;
            w.structures
                .retain(|x| !(x.pos == s.pos && x.kind == StructureKind::Shrine));
            // OST-1302: a shrine that falls leaves its stone or timber lying on
            // its tile (a burned one leaves nothing, see `burn`).
            let (kind, n) = if mat == Material::STONE_BLOCK {
                (crate::items::ItemKind::RoughStone, 2)
            } else {
                (crate::items::ItemKind::Log, 1)
            };
            let from = w
                .communities
                .iter()
                .find(|c| c.id == s.community)
                .map(|c| c.name.clone())
                .unwrap_or_default();
            let id = w.create_lone_item(
                kind,
                n,
                crate::items::ItemPlace::Ground(s.pos),
                0.5,
                "shrine rubble",
            );
            if let Some(it) = w.items.iter_mut().find(|it| it.id == id) {
                it.material = mat;
                it.story = Some(Box::new(crate::legend::Story {
                    maker_name: from.clone(),
                    made: w.meta.tick,
                    wear: 1.0,
                    origin: format!("from the shrine of {from}"),
                    ..Default::default()
                }));
            }
            lose(w, s.community, s.pos, "it fell to ruin, untended");
        }
    }
}

/// An observance kept at the shrine makes it good.
pub fn tended(w: &mut World, pos: TilePos) {
    if let Some(st) = w
        .structures
        .iter_mut()
        .find(|x| x.pos == pos && x.kind == StructureKind::Shrine)
    {
        st.condition = 1.0;
    }
}
