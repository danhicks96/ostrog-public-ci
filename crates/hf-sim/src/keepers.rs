//! Keepers (#297, epic #376, ask 5): a community names the one it follows in the spirit
//! (justice.rs) keeper of its observances and its shrine. The keeping is a role held, not the
//! day's deference: once named, the keeper leads the observances and reads the signs
//! (faith.rs `reader_of`) until they die, leave, or lose it to a challenge.
//!
//! One of the community who heeds the signs and has a stronger claim than the keeper (more
//! followed in the spirit, more signs borne out and fewer belied, more who hold the signs as
//! they taught them) challenges, at most once in ten days. Each grown member stands with one
//! or the other, by whom they follow in the spirit, what they think of them and a little the
//! two's records; the side with more wins, the keeper on a tie. A claim one person can
//! challenge on is a following the whole community gives, so a devoted few can push a
//! challenge that the rest turn back. Those who stood with the winner follow them the more, and
//! everyone follows the loser the less; the two think the less of each other. It is on record.
//! FIXLOG 2026-10-27 "nobody kept the observances".

use crate::events::{EventKind, Severity};
use crate::ids::{AgentId, CommunityId};
use crate::justice::Domain;
use crate::mind::beliefs::{Source, Topic};
use crate::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use crate::World;
use serde::{Deserialize, Serialize};

/// Days between one challenge in a community and the next.
pub const CHALLENGE_DAYS: u64 = 10;
/// The least claim that can challenge a keeper.
pub const CLAIM_AT: f32 = 0.5;

/// A community's keeper.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Keeper {
    pub community: CommunityId,
    pub agent: AgentId,
    pub since: u64,
    /// When the keeping was last challenged.
    pub challenged: u64,
}

fn idx(w: &World, id: AgentId) -> Option<usize> {
    w.agents.iter().position(|a| a.id == id && a.alive)
}

/// The keeper of `cid`, if they are living and still of it.
pub fn keeper_of(w: &World, cid: CommunityId) -> Option<AgentId> {
    let k = w.faith.keepers.iter().find(|k| k.community == cid)?;
    idx(w, k.agent)
        .filter(|&i| w.agents[i].community == Some(cid))
        .map(|_| k.agent)
}

fn members(w: &World, cid: CommunityId) -> Vec<usize> {
    let tick = w.meta.tick;
    (0..w.agents.len())
        .filter(|&k| {
            let a = &w.agents[k];
            a.alive
                && a.species == crate::agents::Species::Person
                && a.community == Some(cid)
                && !a.is_child(tick)
        })
        .collect()
}

/// `who`'s claim to the keeping of `cid`: how much its people follow them in the spirit, half
/// a point for each sign of theirs borne out less each belied, and a fifth for each who holds
/// the signs as they taught them.
pub fn claim(w: &World, cid: CommunityId, who: AgentId) -> f32 {
    let all = members(w, cid);
    let deference: f32 = all
        .iter()
        .flat_map(|&k| w.agents[k].defers.iter())
        .filter(|x| x.0 == who)
        .map(|x| x.1[Domain::Spirit as usize])
        .sum();
    let (borne, belied) = w.faith.record_of(who);
    let heeded = all
        .iter()
        .filter(|&&k| {
            w.agents[k].id != who
                && w.agents[k]
                    .beliefs
                    .get(Topic::DeitySigns)
                    .is_some_and(|b| b.stance > 0.0 && b.sources.contains(&Source::Person(who)))
        })
        .count();
    deference + 0.5 * (borne as f32 - belied as f32) + 0.2 * heeded as f32
}

/// Once a day at 17:30, after the followed are worked out (justice.rs, 17:15): keepers named and
/// challenged.
pub fn step(w: &mut World) {
    if w.meta.tick % TICKS_PER_DAY != 17 * TICKS_PER_HOUR + 600 {
        return;
    }
    let cids: Vec<CommunityId> = w
        .communities
        .iter()
        .filter(|c| !c.dissolved)
        .map(|c| c.id)
        .collect();
    for cid in cids {
        keep(w, cid);
    }
}

fn keep(w: &mut World, cid: CommunityId) {
    let tick = w.meta.tick;
    let Some(keeper) = keeper_of(w, cid) else {
        // None, or the last is dead or gone: the one followed in the spirit is named.
        w.faith.keepers.retain(|k| k.community != cid);
        let followed = w
            .communities
            .iter()
            .find(|c| c.id == cid)
            .and_then(|c| c.leaders[Domain::Spirit as usize]);
        if let Some(agent) = followed.filter(|a| idx(w, *a).is_some()) {
            name(w, cid, agent);
        }
        return;
    };
    let last = w
        .faith
        .keepers
        .iter()
        .find(|k| k.community == cid)
        .map(|k| k.challenged)
        .unwrap_or(0);
    if tick.saturating_sub(last) < CHALLENGE_DAYS * TICKS_PER_DAY {
        return;
    }
    let held = claim(w, cid, keeper);
    let rival = members(w, cid)
        .into_iter()
        .filter(|&k| {
            w.agents[k].id != keeper
                && w.agents[k].beliefs.lean(Topic::DeitySigns) >= crate::observances::LEADS_AT
        })
        .map(|k| (claim(w, cid, w.agents[k].id), w.agents[k].id))
        .filter(|(c, _)| *c >= CLAIM_AT && *c > held)
        .max_by(|a, b| a.0.total_cmp(&b.0).then(b.1.cmp(&a.1)));
    if let Some((_, by)) = rival {
        challenge(w, cid, by, keeper);
    }
}

pub(crate) fn name(w: &mut World, cid: CommunityId, agent: AgentId) {
    let tick = w.meta.tick;
    w.faith.keepers.retain(|k| k.community != cid);
    w.faith.keepers.push(Keeper {
        community: cid,
        agent,
        since: tick,
        challenged: tick,
    });
    let at = idx(w, agent).map(|i| w.agents[i].pos);
    w.events.push(
        tick,
        at,
        Severity::Notable,
        EventKind::KeeperNamed {
            agent,
            community: cid,
        },
    );
}

/// How much `voter` stands with `who`: whom they follow in the spirit, what they think of
/// them, and a little the record of their signs.
pub(crate) fn standing(w: &World, voter: usize, who: AgentId) -> f32 {
    let a = &w.agents[voter];
    let defer = a
        .defers
        .iter()
        .find(|x| x.0 == who)
        .map(|x| x.1[Domain::Spirit as usize])
        .unwrap_or(0.0);
    let (borne, belied) = w.faith.record_of(who);
    defer + 0.5 * a.opinion_of(who) + 0.1 * (borne as f32 - belied as f32)
}

/// `by` challenges `keeper` for the keeping of `cid`.
pub fn challenge(w: &mut World, cid: CommunityId, by: AgentId, keeper: AgentId) {
    let tick = w.meta.tick;
    let voters: Vec<usize> = members(w, cid)
        .into_iter()
        .filter(|&k| w.agents[k].id != by && w.agents[k].id != keeper)
        .collect();
    // Even, they stay with the keeper.
    let (with_by, with_keeper): (Vec<usize>, Vec<usize>) = voters
        .iter()
        .partition(|&&k| standing(w, k, by) > standing(w, k, keeper));
    let won = with_by.len() > with_keeper.len();
    let (winner, loser, side) = if won {
        (by, keeper, &with_by)
    } else {
        (keeper, by, &with_keeper)
    };
    for &k in side {
        crate::justice::credit(w, k, winner, Domain::Spirit, 0.15);
    }
    for &k in &voters {
        crate::justice::discredit(w, k, loser, Domain::Spirit, 0.15);
    }
    if let (Some(i), Some(j)) = (idx(w, winner), idx(w, loser)) {
        w.agents[i].relate(loser, -0.1, 0.05, tick);
        w.agents[j].relate(winner, -0.2, 0.05, tick);
    }
    if let Some(k) = w.faith.keepers.iter_mut().find(|k| k.community == cid) {
        k.challenged = tick;
        if won {
            k.agent = by;
            k.since = tick;
        }
    }
    let at = idx(w, by).map(|i| w.agents[i].pos);
    w.events.push(
        tick,
        at,
        Severity::Notable,
        EventKind::KeeperChallenged {
            by,
            keeper,
            community: cid,
            won,
            with_by: with_by.len() as u32,
            with_keeper: with_keeper.len() as u32,
        },
    );
    // Turned back, the challenger and those who stood with them may hold apart (heresy.rs).
    if !won {
        let with: Vec<AgentId> = with_by.iter().map(|&k| w.agents[k].id).collect();
        crate::heresy::after_challenge(w, cid, by, keeper, &with);
    }
}
