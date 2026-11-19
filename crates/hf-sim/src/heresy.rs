//! Heresy as an organised split (#297, epic #376, ask 6): a challenge for the keeping
//! (keepers.rs) that is turned back leaves the challenger and those who stood with them holding
//! the signs apart from the keeper. With two or more beside the challenger they become a
//! faction, on record (`FactionFormed`): they keep the season's observance on their own, led by
//! the one who challenged (`keep_apart`, called from observances.rs), and not with the rest.
//!
//! Each evening after the keeping, the faction is reckoned with (`step`). Whoever of it has come
//! to stand with the keeper again (keepers.rs `standing`: whom they follow in the spirit, what
//! they think of the two, the two's records of signs borne out and belied) goes back; anyone of
//! the community who has come to stand with the faction's leader by a clear margin joins it. A
//! faction down to its leader, or whose leader is dead, gone or now the keeper, is brought back,
//! on record (`FactionEnded`). One that holds together two seasons, has kept an observance of its
//! own, is three or more and leaves three or more behind, and does not think well of the keeper,
//! splits off as a community of its own (systems/society.rs `split_off`), its leader the new
//! community's keeper, taking a relic of the old with it if there is one (relics.rs).
//! FIXLOG 2026-10-28 "those who lost a challenge for the keeping went on as before".

use crate::events::{EventKind, Severity};
use crate::ids::{AgentId, CommunityId, TilePos};
use crate::justice::Domain;
use crate::mind::beliefs::{Source, Topic};
use crate::mind::ThoughtKind;
use crate::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use crate::World;
use serde::{Deserialize, Serialize};

/// Who stand with a turned-back challenger, at least, for a faction to form.
pub const FORMS_WITH: usize = 2;
/// How long a faction holds together before it may split off: two seasons (docs/TIME.md).
pub const SPLITS_AFTER: u64 = 10 * TICKS_PER_DAY;
/// By how much more someone must stand with the faction's leader than the keeper to join it.
pub const JOINS_BY: f32 = 0.2;
/// What the faction thinks of the keeper, on average, at most, to split off.
pub const SPLITS_BELOW: f32 = 0.1;

/// Those of a community who hold the signs apart from its keeper.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Faction {
    pub community: CommunityId,
    pub leader: AgentId,
    /// The keeper they stood against.
    pub keeper: AgentId,
    /// Everyone of it, the leader first.
    pub members: Vec<AgentId>,
    pub formed: u64,
    /// Observances kept apart.
    pub kept: u32,
}

/// The faction of `cid` that `who` is of, if any.
pub fn of(w: &World, cid: CommunityId, who: AgentId) -> Option<&Faction> {
    w.faith
        .factions
        .iter()
        .find(|f| f.community == cid && f.members.contains(&who))
}

/// After a challenge for the keeping turned back (keepers.rs `challenge`): `by` and those who
/// stood with them form a faction, if enough did and there is none in the community yet.
pub fn after_challenge(
    w: &mut World,
    cid: CommunityId,
    by: AgentId,
    keeper: AgentId,
    with: &[AgentId],
) {
    if with.len() < FORMS_WITH || w.faith.factions.iter().any(|f| f.community == cid) {
        return;
    }
    let tick = w.meta.tick;
    let mut members = vec![by];
    members.extend(with.iter().copied());
    w.faith.factions.push(Faction {
        community: cid,
        leader: by,
        keeper,
        members: members.clone(),
        formed: tick,
        kept: 0,
    });
    let at = w.agents.iter().find(|a| a.id == by).map(|a| a.pos);
    w.events.push(
        tick,
        at,
        Severity::Notable,
        EventKind::FactionFormed {
            leader: by,
            keeper,
            community: cid,
            members: members.len() as u32,
        },
    );
}

fn idx(w: &World, id: AgentId) -> Option<usize> {
    w.agents.iter().position(|a| a.id == id && a.alive)
}

/// Once a day at 17:40, after the keeping (keepers.rs, 17:30): factions grow, shrink, are
/// brought back or split off.
pub fn step(w: &mut World) {
    if w.meta.tick % TICKS_PER_DAY != 17 * TICKS_PER_HOUR + 800 || w.faith.factions.is_empty() {
        return;
    }
    for f in w.faith.factions.clone() {
        reckon(w, f);
    }
}

fn reckon(w: &mut World, mut f: Faction) {
    let tick = w.meta.tick;
    let cid = f.community;
    let here =
        |w: &World, id: AgentId| idx(w, id).is_some_and(|i| w.agents[i].community == Some(cid));
    let keeper = crate::keepers::keeper_of(w, cid);
    let ended = if !here(w, f.leader) {
        Some("lost the one who led them")
    } else if keeper == Some(f.leader) {
        Some("took the keeping")
    } else {
        None
    };
    if let Some(how) = ended {
        end(w, &f, how);
        return;
    }
    let keeper = keeper.unwrap_or(f.keeper);
    f.keeper = keeper;
    // Back to the keeper, or over to the faction.
    let all: Vec<usize> = (0..w.agents.len())
        .filter(|&k| {
            let a = &w.agents[k];
            a.alive
                && a.species == crate::agents::Species::Person
                && a.community == Some(cid)
                && !a.is_child(tick)
                && a.id != keeper
                && a.id != f.leader
        })
        .collect();
    for k in all {
        let id = w.agents[k].id;
        let to_leader = crate::keepers::standing(w, k, f.leader);
        let to_keeper = crate::keepers::standing(w, k, keeper);
        if f.members.contains(&id) {
            if to_keeper >= to_leader {
                f.members.retain(|m| *m != id);
            }
        } else if to_leader > to_keeper + JOINS_BY
            && w.agents[k].beliefs.lean(Topic::DeitySigns) >= crate::observances::LEADS_AT
        {
            f.members.push(id);
        }
    }
    f.members.retain(|m| here(w, *m));
    if f.members.len() < 2 {
        end(w, &f, "came back to the keeper");
        return;
    }
    let rest = (0..w.agents.len())
        .filter(|&k| {
            let a = &w.agents[k];
            a.alive && a.community == Some(cid) && !f.members.contains(&a.id)
        })
        .count();
    let cold = f
        .members
        .iter()
        .filter_map(|m| idx(w, *m))
        .map(|i| w.agents[i].opinion_of(keeper))
        .sum::<f32>()
        / f.members.len() as f32;
    if tick.saturating_sub(f.formed) >= SPLITS_AFTER
        && f.kept >= 1
        && f.members.len() >= 3
        && rest >= 3
        && cold <= SPLITS_BELOW
    {
        w.faith
            .factions
            .retain(|x| !(x.community == cid && x.leader == f.leader));
        let cause = "they held the signs apart from the keeper".to_string();
        if let Some(to) = crate::systems::society::split_off(w, cid, f.leader, &f.members, cause) {
            crate::keepers::name(w, to, f.leader);
            crate::relics::carried_off_by_split(w, cid, to, f.leader);
            return;
        }
        // No place for them to go: they hold together.
        w.faith.factions.push(f);
        return;
    }
    if let Some(x) = w
        .faith
        .factions
        .iter_mut()
        .find(|x| x.community == cid && x.leader == f.leader)
    {
        *x = f;
    }
}

fn end(w: &mut World, f: &Faction, how: &str) {
    let tick = w.meta.tick;
    w.faith
        .factions
        .retain(|x| !(x.community == f.community && x.leader == f.leader));
    let at = w.agents.iter().find(|a| a.id == f.leader).map(|a| a.pos);
    w.events.push(
        tick,
        at,
        Severity::Notable,
        EventKind::FactionEnded {
            leader: f.leader,
            community: f.community,
            how: how.to_string(),
        },
    );
}

/// Whether `who` of `cid` keeps the observance apart, with a faction.
pub fn apart(w: &World, cid: CommunityId, who: AgentId) -> bool {
    of(w, cid, who).is_some()
}

/// An hour before noon on an observance day (observances.rs `gather`): those of a faction free
/// to go walk to its leader instead of the shrine.
pub fn gather_apart(w: &mut World, cid: CommunityId) {
    use crate::mind::Activity;
    let Some(f) = w
        .faith
        .factions
        .iter()
        .find(|f| f.community == cid)
        .cloned()
    else {
        return;
    };
    let Some(li) = idx(w, f.leader) else { return };
    let at: TilePos = w.agents[li].pos;
    // The leader waits where they are.
    if matches!(
        w.agents[li].activity,
        Activity::Idle | Activity::Loitering { .. } | Activity::Talking { .. }
    ) {
        w.agents[li].path.clear();
        w.agents[li].activity = Activity::Loitering {
            ticks_left: 2 * TICKS_PER_HOUR as u32,
        };
    }
    for (n, m) in f.members.iter().skip(1).enumerate() {
        let Some(k) = idx(w, *m) else { continue };
        let a = &w.agents[k];
        if a.band.is_some()
            || a.caravan.is_some()
            || a.leaving
            || a.pos.dist(at) > 45
            || a.pos.dist(at) <= 2
            || !crate::pilgrimage::free_to_go(w, k)
            || !matches!(
                a.activity,
                Activity::Idle | Activity::Loitering { .. } | Activity::Talking { .. }
            )
        {
            continue;
        }
        let (dx, dy) = [(0, 2), (2, 0), (0, -2), (-2, 0), (2, 2), (-2, 2)][n % 6];
        let spot = w.grid.surface(at.x + dx, at.y + dy);
        let spot = if w.grid.walkable(spot) { spot } else { at };
        if w.path_agent_to(k, spot) {
            w.agents[k].activity = Activity::Going {
                to: spot,
                then: Box::new(Activity::Loitering {
                    ticks_left: 2 * TICKS_PER_HOUR as u32,
                }),
            };
            w.agents[k].activity_ticks = 0;
        }
    }
}

/// At noon on an observance day (observances.rs `keep`): a faction keeps its own, led by its
/// leader, with those of it gathered near. On record (`FactionObservance`); those who came are
/// glad of it, hold the signs the surer, and follow the leader the more in the spirit.
pub fn keep_apart(w: &mut World, cid: CommunityId) {
    let tick = w.meta.tick;
    let Some(f) = w
        .faith
        .factions
        .iter()
        .find(|f| f.community == cid)
        .cloned()
    else {
        return;
    };
    let Some(li) = idx(w, f.leader) else { return };
    if w.agents[li].activity.is_asleep() {
        return;
    }
    let at = w.agents[li].pos;
    let came: Vec<usize> = f
        .members
        .iter()
        .skip(1)
        .filter_map(|m| idx(w, *m))
        .filter(|&k| {
            let a = &w.agents[k];
            a.pos.z == at.z
                && a.pos.dist(at) <= crate::observances::GATHERED_WITHIN
                && !a.activity.is_asleep()
        })
        .collect();
    if came.is_empty() {
        return;
    }
    let ev = w.events.push(
        tick,
        Some(at),
        Severity::Notable,
        EventKind::FactionObservance {
            by: f.leader,
            community: cid,
            came: came.len() as u32,
        },
    );
    for &k in came.iter().chain(std::iter::once(&li)) {
        w.agents[k]
            .thoughts
            .add(ThoughtKind::KeptObservance, tick, Some(ev));
        w.agents[k]
            .beliefs
            .experience(Topic::DeitySigns, 1.0, 0.08, Source::Event(ev), tick);
    }
    for &k in &came {
        crate::justice::credit(w, k, f.leader, Domain::Spirit, 0.1);
    }
    if let Some(x) = w
        .faith
        .factions
        .iter_mut()
        .find(|x| x.community == cid && x.leader == f.leader)
    {
        x.kept += 1;
    }
}
