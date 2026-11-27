//! What one person can see another doing: a small, stable set of acts derived from the
//! activity and the world as they stand. Nothing here is saved; `act_of` is a pure function of
//! existing state, so it changes no behaviour and no digest. Append-only.
//!
//! Nothing calls `act_of` yet. It is a read-only contract for a later consumer; it does not
//! decide whether an onlooker could see the act (see `mind::perception::can_see`) and is not the
//! camera tally of visible activity.

use crate::agents::Agent;
use crate::mind::activity::Activity;
use crate::mind::feelings::FeelingKind;
use crate::structures::StructureKind;
use crate::work::DesignationKind;
use crate::world_state::World;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ActKind {
    /// Asleep with no bed.
    SleepingRough,
    /// Eating with no table or stool within a tile.
    EatingOnGround,
    /// Standing about while the community's needs are not secured.
    IdleInNeed,
    /// Hauling to a stockpile.
    CarryingForStore,
    /// Working on a build order.
    Building,
    /// Looking after someone hurt or sick.
    Tending,
    /// Awake near the grave of someone they held dear.
    Mourning,
    /// Awake and in strong grief.
    Weeping,
}

/// How near a grave counts as mourning at it, in tiles.
const MOURN_RANGE: i32 = 2;
/// How well they must have held the dead.
const HELD: f32 = 0.3;
/// How strong a grief shows as weeping.
const WEEP: f32 = 0.5;

/// What `a` is seen doing, if it is one of the acts; None for walking and most work.
/// `secured`: whether everything `decide::secured` asks is secured for their community.
pub fn act_of(w: &World, a: &Agent, secured: bool) -> Option<ActKind> {
    match &a.activity {
        Activity::Sleeping { bed: None, .. } => return Some(ActKind::SleepingRough),
        Activity::Sleeping { .. } => return None,
        Activity::Eating { .. } => {
            let seated = w.structures.iter().any(|s| {
                matches!(s.kind, StructureKind::Table | StructureKind::Stool)
                    && s.pos.z == a.pos.z
                    && s.pos.dist(a.pos) <= 1
            });
            return (!seated).then_some(ActKind::EatingOnGround);
        }
        Activity::Working { plan } => {
            let kind = w
                .designations
                .iter()
                .find(|d| d.id == plan.designation)
                .map(|d| &d.kind);
            return match kind {
                Some(DesignationKind::Haul) => Some(ActKind::CarryingForStore),
                Some(DesignationKind::Tend(_)) => Some(ActKind::Tending),
                Some(
                    DesignationKind::BuildWall(_)
                    | DesignationKind::BuildDoor
                    | DesignationKind::BuildFloor(_)
                    | DesignationKind::BuildRoof(_)
                    | DesignationKind::BuildStructure(..),
                ) => Some(ActKind::Building),
                _ => None,
            };
        }
        _ => {}
    }
    if a.feelings.of(FeelingKind::Grief) > WEEP {
        return Some(ActKind::Weeping);
    }
    let at_grave = w.graves.iter().any(|g| {
        g.pos.z == a.pos.z
            && g.pos.dist(a.pos) <= MOURN_RANGE
            && a.relationships
                .get(&g.dead)
                .is_some_and(|r| r.opinion > HELD)
    });
    if at_grave {
        return Some(ActKind::Mourning);
    }
    if !secured && matches!(a.activity, Activity::Idle | Activity::Loitering { .. }) {
        return Some(ActKind::IdleInNeed);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::Species;
    use crate::ids::{ItemId, TilePos};
    use crate::world::gen::GenParams;
    use crate::world::Material;

    fn one() -> (World, usize) {
        let mut w = World::generate(7, GenParams::small());
        w.spawn_agent(Species::Person, TilePos::new(30, 30, 2));
        let i = w.agents.len() - 1;
        (w, i)
    }

    #[test]
    fn sleeping_on_the_ground_is_sleeping_rough() {
        let (mut w, i) = one();
        w.agents[i].activity = Activity::Sleeping {
            bed: None,
            until: 9,
        };
        assert_eq!(act_of(&w, &w.agents[i], true), Some(ActKind::SleepingRough));
        w.agents[i].activity = Activity::Sleeping {
            bed: Some(crate::ids::StructureId(1)),
            until: 9,
        };
        assert_eq!(act_of(&w, &w.agents[i], true), None);
    }

    #[test]
    fn eating_beside_a_table_is_not_on_the_ground() {
        let (mut w, i) = one();
        w.agents[i].activity = Activity::Eating {
            item: ItemId(1),
            ticks_left: 5,
            binge: None,
        };
        assert_eq!(
            act_of(&w, &w.agents[i], true),
            Some(ActKind::EatingOnGround)
        );
        w.place_structure(
            StructureKind::Table,
            TilePos::new(31, 30, 2),
            Material::WOOD,
            false,
        );
        assert_eq!(act_of(&w, &w.agents[i], true), None);
    }

    #[test]
    fn standing_about_in_a_hunger_is_idle_in_need() {
        let (mut w, i) = one();
        w.agents[i].activity = Activity::Idle;
        assert_eq!(act_of(&w, &w.agents[i], false), Some(ActKind::IdleInNeed));
        assert_eq!(act_of(&w, &w.agents[i], true), None);
        w.agents[i].activity = Activity::Loitering { ticks_left: 3 };
        assert_eq!(act_of(&w, &w.agents[i], false), Some(ActKind::IdleInNeed));
    }

    #[test]
    fn strong_grief_is_weeping_and_mild_grief_is_not() {
        let (mut w, i) = one();
        w.agents[i].activity = Activity::Idle;
        w.agents[i]
            .feelings
            .feel(FeelingKind::Grief, None, 0.3, 0, None);
        assert_eq!(act_of(&w, &w.agents[i], true), None);
        w.agents[i]
            .feelings
            .feel(FeelingKind::Grief, None, 0.8, 1, None);
        assert_eq!(act_of(&w, &w.agents[i], true), Some(ActKind::Weeping));
        // Asleep, grief shows nothing: the sleeping arms return before it is read.
        w.agents[i].activity = Activity::Sleeping {
            bed: Some(crate::ids::StructureId(1)),
            until: 9,
        };
        assert_eq!(act_of(&w, &w.agents[i], true), None);
    }
}
