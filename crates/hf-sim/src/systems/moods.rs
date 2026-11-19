//! Fey moods (C35): DF's strange mood, kept to what is recorded. An artisan is taken by a work
//! only for a cause in their own past that the record holds: the craft a master taught them
//! before the portal, a child, a sibling or a parent lost before it, or kin who died here in the
//! valley. Taken, they claim a bench and the finest material in the store and make one piece:
//! a figure of the one they lost, or the best thing their craft can make. The piece is the
//! finest they will ever make; it gets a name, and its story says why it was made. If the work
//! is not done in ten days the mood passes and leaves them low. Runs once a game day.

use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::events::{EventKind, Severity};
use crate::ids::{AgentId, EventId, TilePos};
use crate::items::{ItemKind, ItemPlace};
use crate::rng::{purpose, stream};
use crate::structures::StructureKind;
use crate::time::TICKS_PER_DAY;
use crate::work::{recipes, DesignationId, DesignationKind};
use crate::world::Material;
use crate::world_state::World;

/// A mood in progress.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Fey {
    pub since: u64,
    /// The record it comes from: a past event (Lived) or a death in the valley.
    pub cause: EventId,
    /// "remembering Ada, her daughter, who died of fever", "as Brannoc the smith taught her".
    pub about: String,
    /// Whose figure it is, for a figure ("Ada"), or "".
    pub of: String,
    pub work: DesignationId,
}

/// Days a mood lasts before it passes undone.
pub const FEY_DAYS: u64 = 10;
/// Chance a day that someone with a cause is taken.
const CHANCE: f64 = 1.0 / 240.0;

/// What could take agent `i`: the record's id, the words, whose figure (if a loss), and
/// whether it is the smith's craft (a blade) rather than a figure.
fn causes(w: &World, i: usize) -> Vec<(EventId, String, String, bool)> {
    use crate::past::Formative as F;
    let a = &w.agents[i];
    let mut out = Vec::new();
    for (eid, f) in crate::opening::formatives(w, a.id) {
        match f {
            F::Apprenticed { craft, master } => out.push((
                eid,
                format!("as {master}, {craft}, taught them"),
                String::new(),
                craft.contains("smith"),
            )),
            F::ChildDied { child } => out.push((
                eid,
                format!("remembering {child}, their child"),
                child,
                false,
            )),
            F::SiblingDied { sibling, cause } => out.push((
                eid,
                format!("remembering {sibling}, who died of {cause}"),
                sibling,
                false,
            )),
            F::ParentDied { parent, cause } => out.push((
                eid,
                format!("remembering {parent}, who died of {cause}"),
                parent,
                false,
            )),
            _ => {}
        }
    }
    // Kin who died here, in the last two years.
    let kin: Vec<AgentId> = a
        .children
        .iter()
        .copied()
        .chain(a.partner)
        .chain(a.mother)
        .chain(a.father)
        .collect();
    for (id, tick, kind) in &w.culture_log {
        if let EventKind::Died { agent, cause } = kind {
            if kin.contains(agent) && w.meta.tick.saturating_sub(*tick) < 40 * TICKS_PER_DAY {
                let name = w
                    .agents
                    .iter()
                    .find(|x| x.id == *agent)
                    .map(|x| x.name.clone())
                    .unwrap_or_default();
                out.push((
                    *id,
                    format!("remembering {name}, who died of {cause}"),
                    name,
                    false,
                ));
            }
        }
    }
    out
}

/// The finest material lying within reach of home that agent `i` knows of, for the work:
/// the recipe, the material and the station's tile. (Only what they know: people fetch only
/// what they have seen or been told of, and a work they cannot gather for was dropped at once.)
fn materials(w: &World, i: usize, home: TilePos, blade: bool) -> Option<(u8, Material, TilePos)> {
    let a = &w.agents[i];
    let station = |k: StructureKind| {
        w.structures
            .iter()
            .find(|s| s.kind == k && s.progress >= 1.0 && s.pos.dist(home) <= 45)
            .map(|s| s.pos)
    };
    let have = |k: ItemKind, m: Material| -> bool {
        w.items
            .iter()
            .filter(|it| {
                it.kind == k
                    && it.material == m
                    && matches!(it.place, ItemPlace::Ground(p) if p.dist(home) <= 45)
                    && a.knowledge
                        .get(crate::mind::knowledge::Known::Item(it.id))
                        .is_some()
            })
            .map(|it| it.count)
            .sum::<u32>()
            >= 2
    };
    if blade {
        let at = station(StructureKind::Anvil)?;
        for m in [Material::GOLD, Material::SILVER, Material::BRONZE] {
            if have(ItemKind::Bar, m) {
                return Some((recipes::FEY_BLADE, m, at));
            }
        }
        if have(ItemKind::IronBar, Material::IRON) {
            return Some((recipes::FEY_IRON_BLADE, Material::IRON, at));
        }
        return None;
    }
    let at = station(StructureKind::Workbench)?;
    for m in [
        Material::MARBLE,
        Material::GRANITE,
        Material::BASALT,
        Material::GABBRO,
        Material::GNEISS,
        Material::SLATE,
        Material::LIMESTONE,
        Material::SANDSTONE,
    ] {
        if have(ItemKind::StoneBlock, m) {
            return Some((recipes::FEY_FIGURE, m, at));
        }
    }
    for m in [
        Material::OAK,
        Material::BIRCH,
        Material::PINE,
        Material::FUNGUS_WOOD,
    ] {
        if have(ItemKind::Log, m) {
            return Some((recipes::FEY_CARVING, m, at));
        }
    }
    None
}

pub fn step(w: &mut World) {
    let tick = w.meta.tick;
    if tick % TICKS_PER_DAY != 5 * crate::time::TICKS_PER_HOUR + 13 {
        return;
    }
    let day = tick / TICKS_PER_DAY;
    for i in 0..w.agents.len() {
        // A mood in progress. A finished work clears it in the craft (`fulfilled`), so a mood
        // still here whose work is gone, or that has run its days, has passed undone.
        if let Some(f) = w.agents[i].fey.clone() {
            let still = w.designations.iter().any(|d| d.id == f.work && !d.done);
            if !still || tick.saturating_sub(f.since) > FEY_DAYS * TICKS_PER_DAY {
                let gone = crate::work::split_off(&mut w.designations, |d| d.id != f.work);
                w.give_back(gone);
                w.agents[i].fey = None;
                w.agents[i]
                    .thoughts
                    .add(crate::mind::ThoughtKind::FeyLost, tick, None);
            }
            continue;
        }
        let a = &w.agents[i];
        if !a.alive
            || a.species != crate::agents::Species::Person
            || !a.of_age(tick)
            || a.band.is_some()
        {
            continue;
        }
        if a.community.is_none() {
            continue;
        }
        let skill = a
            .skills
            .level(crate::mind::Skill::Crafting)
            .max(a.skills.level(crate::mind::Skill::Construction));
        if skill < 3.0 {
            continue;
        }
        let mut r = stream(
            w.meta.seed,
            &[purpose::AGENT_WORK, a.id.0 as u64, day, 0xFE7],
        );
        // A thing well made is worth the making (Craftsmanship, P7): those who hold it are
        // taken more often, those who scorn it less.
        let craft = w.agents[i]
            .beliefs
            .lean(crate::mind::beliefs::Topic::Craftsmanship);
        if !r.gen_bool((CHANCE * (1.0 + craft as f64)).clamp(0.0, 1.0)) {
            continue;
        }
        let pick = r.gen::<u32>() as usize;
        take(w, i, pick);
    }
}

/// Agent `i` is taken by a work if there is a cause in their record and the material and a
/// bench for it; `pick` chooses among several causes. False (and nothing changed) otherwise.
pub fn take(w: &mut World, i: usize, pick: usize) -> bool {
    let tick = w.meta.tick;
    let Some(cid) = w.agents[i].community else {
        return false;
    };
    let causes = causes(w, i);
    if causes.is_empty() {
        return false;
    }
    let (cause, about, of, blade) = causes[pick % causes.len()].clone();
    let Some(home) = w.communities.iter().find(|c| c.id == cid).map(|c| c.home) else {
        return false;
    };
    let Some((recipe, m, at)) = materials(w, i, home, blade) else {
        return false;
    };
    let who = w.agents[i].id;
    let mut d = crate::systems::settlement::make(w, DesignationKind::Craft(recipe), at, 9);
    d.community = Some(cid);
    d.owner = Some(who);
    if let Some(q) = d.requirements.first_mut() {
        q.want = Some(m);
    }
    let work = d.id;
    w.designations.push(d);
    w.agents[i].fey = Some(Fey {
        since: tick,
        cause,
        about: about.clone(),
        of,
        work,
    });
    let pos = w.agents[i].pos;
    w.events.push(
        tick,
        Some(pos),
        Severity::Notable,
        EventKind::Possessed {
            agent: who,
            because: about,
            cause,
        },
    );
    true
}

/// The piece agent `i` made in a mood: the finest work, and its story says why. Called from
/// the craft when the work done was the mood's.
pub fn fulfilled(w: &mut World, i: usize, item: crate::ids::ItemId) {
    let Some(f) = w.agents[i].fey.take() else {
        return;
    };
    if let Some(it) = w.items.iter_mut().find(|it| it.id == item) {
        it.quality = 1.0;
        if let Some(s) = it.story.as_mut() {
            s.origin = if f.of.is_empty() {
                format!("made in a fey mood, {}", f.about)
            } else {
                format!("a figure of {}, made in a fey mood, {}", f.of, f.about)
            };
        }
    }
    let tick = w.meta.tick;
    w.agents[i]
        .thoughts
        .add(crate::mind::ThoughtKind::Masterwork, tick, None);
}

/// Whether this order is someone's work in a fey mood: the mood, not a failed plan or the
/// planner's count of failures, decides when it ends.
pub fn is_fey_work(w: &World, d: crate::work::DesignationId) -> bool {
    w.agents
        .iter()
        .any(|a| a.fey.as_ref().is_some_and(|f| f.work == d))
}
