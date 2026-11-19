//! #297 ask 5: the one followed in the spirit is named keeper of the observances; a rival with a
//! stronger claim challenges, and the outcome moves who is followed and is on record.

use hf_sim::events::EventKind;
use hf_sim::ids::{AgentId, TilePos};
use hf_sim::justice::Domain;
use hf_sim::mind::beliefs::{Source, Topic};
use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use hf_sim::world::gen::GenParams;
use hf_sim::World;

fn valley(seed: u64, n: usize) -> (World, Vec<AgentId>) {
    let mut w = World::generate(seed, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.rebuild_caches();
    let ids = w.found_party(n);
    w.meta.autoplan = false;
    for (k, id) in ids.iter().enumerate() {
        let i = idx(&w, *id);
        w.agents[i].pos = w.grid.surface(30 + k as i32, 30);
        w.agents[i].defers.clear();
        w.agents[i]
            .beliefs
            .hold(Topic::DeitySigns, 0.5, 0.6, Source::Culture(0), 0);
    }
    w.events.drain();
    (w, ids)
}

fn idx(w: &World, id: AgentId) -> usize {
    w.agents.iter().position(|a| a.id == id).unwrap()
}

/// Everyone but `to` (and those in `not`) defers to `to` in the spirit by `amount`.
fn defer(w: &mut World, ids: &[AgentId], to: AgentId, amount: f32, not: &[AgentId]) {
    for &id in ids {
        if id == to || not.contains(&id) {
            continue;
        }
        let i = idx(w, id);
        match w.agents[i].defers.iter_mut().find(|x| x.0 == to) {
            Some(x) => x.1[Domain::Spirit as usize] = amount,
            None => w.agents[i].defers.push((to, [0.0, 0.0, amount, 0.0])),
        }
    }
}

/// The evening's working out of who is followed, then the keeping.
fn evening(w: &mut World) {
    let day = w.meta.tick / TICKS_PER_DAY + 1;
    w.meta.tick = day * TICKS_PER_DAY + 17 * TICKS_PER_HOUR + 300;
    hf_sim::justice::step(w);
    w.meta.tick = day * TICKS_PER_DAY + 17 * TICKS_PER_HOUR + 600;
    hf_sim::keepers::step(w);
}

fn spirit(w: &World, voter: AgentId, of: AgentId) -> f32 {
    w.agents[idx(w, voter)]
        .defers
        .iter()
        .find(|x| x.0 == of)
        .map(|x| x.1[Domain::Spirit as usize])
        .unwrap_or(0.0)
}

#[test]
fn the_one_followed_in_the_spirit_is_named_keeper_and_keeps_it_when_followed_less() {
    let (mut w, ids) = valley(2981, 6);
    let cid = w.communities[0].id;
    defer(&mut w, &ids, ids[1], 0.5, &[]);
    evening(&mut w);
    assert_eq!(hf_sim::keepers::keeper_of(&w, cid), Some(ids[1]));
    assert!(w
        .events
        .pending
        .iter()
        .any(|e| matches!(e.kind, EventKind::KeeperNamed { agent, .. } if agent == ids[1])));
    // The keeper reads the signs and leads the observance, whoever is followed today.
    assert_eq!(hf_sim::faith::reader_of(&w, cid), Some(ids[1]));
    // Another, who holds signs are only weather, comes to be followed more in the spirit: they
    // are followed, but the keeping is not theirs to claim.
    w.events.drain();
    let k = idx(&w, ids[2]);
    w.agents[k]
        .beliefs
        .hold(Topic::DeitySigns, -0.5, 0.8, Source::Culture(0), 0);
    defer(&mut w, &ids, ids[2], 0.6, &[]);
    for _ in 0..12 {
        evening(&mut w);
    }
    assert_eq!(
        w.communities[0].leaders[Domain::Spirit as usize],
        Some(ids[2])
    );
    assert_eq!(hf_sim::keepers::keeper_of(&w, cid), Some(ids[1]));
    assert!(!w
        .events
        .pending
        .iter()
        .any(|e| matches!(e.kind, EventKind::KeeperChallenged { .. })));
    // The keeper dies: the one followed now is named.
    let k = idx(&w, ids[1]);
    w.agents[k].alive = false;
    evening(&mut w);
    assert_eq!(hf_sim::keepers::keeper_of(&w, cid), Some(ids[2]));
}

#[test]
fn a_rival_with_more_signs_borne_out_challenges_and_takes_the_keeping() {
    let (mut w, ids) = valley(2982, 7);
    let cid = w.communities[0].id;
    let (keeper, rival) = (ids[0], ids[1]);
    defer(&mut w, &ids, keeper, 0.5, &[]);
    evening(&mut w);
    assert_eq!(hf_sim::keepers::keeper_of(&w, cid), Some(keeper));
    // Ten days on: the rival's warnings were borne out three times, the keeper's blessing
    // belied twice, and most now defer to the rival as much as to the keeper.
    w.faith.record.push((rival, 3, 0));
    w.faith.record.push((keeper, 0, 2));
    defer(&mut w, &ids, rival, 0.5, &[ids[6]]);
    w.events.drain();
    w.meta.tick += hf_sim::keepers::CHALLENGE_DAYS * TICKS_PER_DAY;
    let before = spirit(&w, ids[3], keeper);
    evening(&mut w);
    let ch = w.events.pending.iter().find_map(|e| match e.kind {
        EventKind::KeeperChallenged {
            by,
            keeper: k,
            won,
            with_by,
            with_keeper,
            ..
        } => Some((by, k, won, with_by, with_keeper)),
        _ => None,
    });
    let (by, k, won, with_by, with_keeper) = ch.expect("the rival challenged");
    assert_eq!((by, k), (rival, keeper));
    assert!(
        won,
        "{with_by} with the rival, {with_keeper} with the keeper"
    );
    assert!(with_by > with_keeper);
    assert_eq!(hf_sim::keepers::keeper_of(&w, cid), Some(rival));
    assert!(
        spirit(&w, ids[3], keeper) < before,
        "the loser is followed the less"
    );
    assert!(w.agents[idx(&w, keeper)].opinion_of(rival) < 0.0);
    // Not challenged again the next day.
    w.events.drain();
    evening(&mut w);
    assert!(!w
        .events
        .pending
        .iter()
        .any(|e| matches!(e.kind, EventKind::KeeperChallenged { .. })));
}

#[test]
fn a_challenger_followed_closely_by_a_few_is_turned_back_by_the_rest() {
    let (mut w, ids) = valley(2983, 7);
    let cid = w.communities[0].id;
    let (keeper, rival) = (ids[0], ids[1]);
    let tick = w.meta.tick;
    w.faith.keepers.push(hf_sim::keepers::Keeper {
        community: cid,
        agent: keeper,
        since: tick,
        challenged: tick,
    });
    // Two follow the rival closely; the other three follow the keeper a little.
    defer(&mut w, &ids[2..4], rival, 1.5, &[]);
    defer(&mut w, &ids[4..], keeper, 0.4, &[]);
    assert!(hf_sim::keepers::claim(&w, cid, rival) > hf_sim::keepers::claim(&w, cid, keeper));
    let before = spirit(&w, ids[2], rival);
    w.meta.tick += hf_sim::keepers::CHALLENGE_DAYS * TICKS_PER_DAY;
    evening(&mut w);
    let ch = w.events.pending.iter().find_map(|e| match e.kind {
        EventKind::KeeperChallenged {
            by,
            won,
            with_by,
            with_keeper,
            ..
        } if by == rival => Some((won, with_by, with_keeper)),
        _ => None,
    });
    assert_eq!(ch, Some((false, 2, 3)), "turned back");
    assert_eq!(hf_sim::keepers::keeper_of(&w, cid), Some(keeper));
    assert!(
        spirit(&w, ids[2], rival) < before * 0.95 - 0.1,
        "even those who stood with them follow the rival the less"
    );
    assert!(
        spirit(&w, ids[5], keeper) > 0.4 * 0.95,
        "the keeper's side the more"
    );
}
