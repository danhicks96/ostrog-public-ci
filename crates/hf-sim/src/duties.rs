//! Duties (#454, docs/design-institutions.md): a role is the work someone has been doing, not a
//! place handed out by skill.
//!
//! Every working minute on a work order counts toward the kind of work it is (building and
//! felling, sowing and harvest, cooking, mining and digging, hauling, tending the hurt). Each
//! morning the tally keeps 85 parts in a hundred of itself, so it reflects the last several
//! days. A kind with two hours or more in the tally, and at least a third of the person's most
//! done kind, is one of their duties; their role is the most done. Several people can share a
//! role, one person can hold two duties, and a duty nobody keeps doing fades. Duties weigh the
//! choice of work (systems/decide.rs), so habits hold; a role taken or let go is on record.
//! Before this (C-era `assign_roles`) each community handed one of each role to its best hand by
//! skill every day.
//!
//! Care and gathering (#602): a completed forage counts `FORAGE_CREDIT` minutes toward gathering
//! and a completed tend `TEND_CREDIT` toward tending (the order is ten minutes; the nursing it
//! begins lasts a day). Those two are a duty at two hours whatever else the person does, without
//! the third-of-the-top rule, because a community relies on whoever gathers and tends however
//! much else they do; they fade as every kind does. They make no one less keen on other work.

use serde::{Deserialize, Serialize};

use crate::events::{EventKind, Severity};
use crate::society::Role;
use crate::work::DesignationKind;
use crate::world_state::World;

/// The kinds of work tallied, in the tally's order.
pub const KINDS: [Role; 7] = [
    Role::Builder,
    Role::Farmer,
    Role::Cook,
    Role::Miner,
    Role::Hauler,
    Role::Healer,
    Role::Forager,
];
/// Minutes a completed forage counts toward gathering: eight at the plant and the walk to it.
pub const FORAGE_CREDIT: u32 = 15;
/// Minutes a completed tend counts toward tending (#602): the order is ten minutes, but the
/// nursing it begins lasts the day; counted as three hours, so one tend makes a healer for two
/// days and a second within them keeps it.
pub const TEND_CREDIT: u32 = 180;
/// Minutes of a kind in the tally for it to be a duty: two hours.
pub const DUTY_MINUTES: u32 = 120;
/// What the tally keeps of itself each day, in hundredths.
pub const KEEPS: u32 = 85;
/// How far ahead a challenger's minutes must stand of the current role's, in hundredths, to
/// start counting toward a switch: 125 is 25% ahead.
pub const SWITCH_MARGIN: u32 = 125;
/// Mornings a challenger must hold the margin before it takes the role.
pub const SWITCH_MORNINGS: u8 = 3;

/// Someone's recent work, by kind (`KINDS`), in minutes, and the duties it makes.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Duties {
    /// By `KINDS`; a save from before #602 has six (no gathering) and is filled out on use.
    pub minutes: Vec<u32>,
    pub held: Vec<Role>,
    /// A role ahead of the current one by `SWITCH_MARGIN`, and how many consecutive mornings
    /// running: a role is identity, so one day's lead does not retitle someone.
    #[serde(default)]
    pub challenger: Option<(Role, u8)>,
}

/// The kind of work an order is, for the tally.
pub fn kind_of(k: DesignationKind) -> Option<Role> {
    use DesignationKind as K;
    Some(match k {
        K::BuildWall(_)
        | K::BuildDoor
        | K::BuildFloor(_)
        | K::BuildRoof(_)
        | K::BuildStructure(..)
        | K::BuildStairs(_)
        | K::ClearRoof
        | K::Chop => Role::Builder,
        K::Sow | K::Harvest => Role::Farmer,
        K::Cook => Role::Cook,
        K::Mine | K::DigStairs | K::DigEarth | K::DigChannel => Role::Miner,
        K::Haul | K::FetchWater => Role::Hauler,
        K::Tend(_) => Role::Healer,
        _ => return None,
    })
}

/// How much keener someone with these duties is on an order of this kind: as the role was
/// (C-era), keener on their own work, a little less on the rest, unchanged with none.
pub fn fit(held: &[Role], k: DesignationKind) -> f32 {
    use DesignationKind as K;
    // Gathering and tending are not a trade that takes someone off other work (#602): one who
    // holds only those is as keen on any order as one who holds nothing.
    let specialised = held.iter().any(|r| !relied_on(*r));
    let own = |r: Role| held.contains(&r);
    match k {
        K::BuildWall(_)
        | K::BuildDoor
        | K::BuildStairs(_)
        | K::BuildFloor(_)
        | K::BuildRoof(_)
        | K::BuildStructure(..)
        | K::Chop
            if own(Role::Builder) =>
        {
            1.5
        }
        K::Sow | K::Harvest if own(Role::Farmer) => 1.5,
        K::Cook if own(Role::Cook) => 1.6,
        K::Mine | K::DigStairs if own(Role::Miner) => 1.5,
        K::Haul if own(Role::Hauler) => 1.5,
        K::Tend(_) if own(Role::Healer) => 1.5,
        _ if specialised => 0.8,
        _ => 1.0,
    }
}

/// A working minute on an order of `kind` (systems/work.rs `run`).
pub fn worked(w: &mut World, i: usize, kind: DesignationKind) {
    if let Some(r) = kind_of(kind) {
        credit(w, i, r, 1);
    }
}

/// `minutes` toward `role` for agent `i`: a working minute, a completed forage (systems/act.rs)
/// or a completed tend (systems/work.rs).
pub fn credit(w: &mut World, i: usize, role: Role, minutes: u32) {
    let Some(k) = KINDS.iter().position(|x| *x == role) else {
        return;
    };
    let d = &mut w.agents[i].duties;
    if d.minutes.len() < KINDS.len() {
        d.minutes.resize(KINDS.len(), 0);
    }
    d.minutes[k] = d.minutes[k].saturating_add(minutes);
}

/// Care and gathering: a duty at `DUTY_MINUTES` without the third-of-the-top rule (#602).
fn relied_on(r: Role) -> bool {
    matches!(r, Role::Healer | Role::Forager)
}

/// The duties a tally makes, most done first.
pub fn of(minutes: &[u32]) -> Vec<Role> {
    let top = minutes.iter().copied().max().unwrap_or(0);
    let mut held: Vec<(u32, usize)> = minutes
        .iter()
        .enumerate()
        .filter(|&(k, &m)| m >= DUTY_MINUTES && (m * 3 >= top || relied_on(KINDS[k])))
        .map(|(k, &m)| (m, k))
        .collect();
    held.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    held.into_iter().map(|(_, k)| KINDS[k]).collect()
}

/// Each morning (systems/society.rs): the tally fades, and each grown member of a community
/// holds the duties it makes; a role taken or let go is on record.
pub fn daily(w: &mut World) {
    let tick = w.meta.tick;
    for i in 0..w.agents.len() {
        let a = &mut w.agents[i];
        if !a.alive || a.community.is_none() {
            continue;
        }
        for m in a.duties.minutes.iter_mut() {
            *m = *m * KEEPS / 100;
        }
        // A duty is held only by a grown member: a child's minutes go on counting and fading,
        // and they take the role on their first morning of age.
        let held = if a.of_age(tick) {
            of(&a.duties.minutes)
        } else {
            Vec::new()
        };
        let top = held.first().copied();
        a.duties.held = held.clone();

        // A first role, or leaving one whose minutes fell below DUTY_MINUTES, is immediate;
        // only a challenger replacing a role still held needs the margin and the
        // consecutive mornings.
        let role = match a.role {
            None => top,
            Some(cur) if !held.contains(&cur) => {
                a.duties.challenger = None;
                top
            }
            Some(cur) => match top {
                Some(t) if t == cur => {
                    a.duties.challenger = None;
                    Some(cur)
                }
                Some(t) => {
                    let cur_min = a.duties.minutes[KINDS.iter().position(|x| *x == cur).unwrap()];
                    let t_min = a.duties.minutes[KINDS.iter().position(|x| *x == t).unwrap()];
                    if t_min * 100 >= cur_min * SWITCH_MARGIN {
                        let streak = match a.duties.challenger {
                            Some((r, n)) if r == t => n + 1,
                            _ => 1,
                        };
                        if streak >= SWITCH_MORNINGS {
                            a.duties.challenger = None;
                            Some(t)
                        } else {
                            a.duties.challenger = Some((t, streak));
                            Some(cur)
                        }
                    } else {
                        a.duties.challenger = None;
                        Some(cur)
                    }
                }
                None => Some(cur),
            },
        };
        if a.role == role {
            continue;
        }
        let (id, pos, was) = (a.id, a.pos, a.role);
        a.role = role;
        let kind = match (role, was) {
            (Some(r), _) => EventKind::RoleTaken {
                agent: id,
                role: format!("{r:?}").to_lowercase(),
            },
            (None, Some(r)) => EventKind::RoleLeft {
                agent: id,
                role: format!("{r:?}").to_lowercase(),
            },
            (None, None) => continue,
        };
        w.events.push(tick, Some(pos), Severity::Minor, kind);
    }
}

/// For inspect: "cooking (14 hours lately), hauling (5 hours lately)".
pub fn describe(a: &crate::agents::Agent) -> String {
    a.duties
        .held
        .iter()
        .map(|r| {
            let k = KINDS.iter().position(|x| x == r).unwrap_or(0);
            format!(
                "{} ({} hours lately)",
                word(*r),
                a.duties.minutes.get(k).copied().unwrap_or(0) / 60
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}

pub fn word(r: Role) -> &'static str {
    match r {
        Role::Builder => "building",
        Role::Farmer => "farming",
        Role::Cook => "cooking",
        Role::Miner => "mining",
        Role::Hauler => "hauling",
        Role::Healer => "tending the hurt",
        Role::Forager => "foraging",
    }
}
