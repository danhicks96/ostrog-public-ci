//! #297 ask 6, heresy as an organised split: a challenge for the keeping turned back leaves a
//! faction holding the signs apart; it keeps its own observance, and in time splits off as a
//! community of its own, taking a relic with it, or is brought back.

use hf_sim::events::EventKind;
use hf_sim::ids::{AgentId, TilePos};
use hf_sim::items::{ItemKind, ItemPlace};
use hf_sim::justice::Domain;
use hf_sim::mind::beliefs::{Source, Topic};
use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use hf_sim::world::gen::GenParams;
use hf_sim::World;

/// Eight by (30, 30), all heeding the signs; ids[0] keeper.
fn valley(seed: u64) -> (World, Vec<AgentId>) {
    let mut w = World::generate(seed, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.rebuild_caches();
    let ids = w.found_party(8);
    w.meta.autoplan = false;
    for (k, id) in ids.iter().enumerate() {
        let i = idx(&w, *id);
        w.agents[i].pos = w.grid.surface(30 + k as i32, 30);
        w.agents[i].activity = hf_sim::mind::activity::Activity::Idle;
        w.agents[i].needs.hunger = 1.0;
        w.agents[i].defers.clear();
        w.agents[i]
            .beliefs
            .hold(Topic::DeitySigns, 0.6, 0.6, Source::Culture(0), 0);
    }
    // Ten days of meals: a community short of food does not gather apart (#415).
    let home = w.communities[0].home;
    w.drop_item(ItemKind::Meal, 240, home.offset(3, 3, 0));
    let cid = w.communities[0].id;
    w.faith.keepers.push(hf_sim::keepers::Keeper {
        community: cid,
        agent: ids[0],
        since: 0,
        challenged: 0,
    });
    w.events.drain();
    (w, ids)
}

fn idx(w: &World, id: AgentId) -> usize {
    w.agents.iter().position(|a| a.id == id).unwrap()
}

fn defer(w: &mut World, who: AgentId, to: AgentId, amount: f32) {
    let i = idx(w, who);
    match w.agents[i].defers.iter_mut().find(|x| x.0 == to) {
        Some(x) => x.1[Domain::Spirit as usize] = amount,
        None => w.agents[i].defers.push((to, [0.0, 0.0, amount, 0.0])),
    }
}

/// ids[1] challenges the keeper with ids[2] and ids[3] standing with them, the rest with the
/// keeper: turned back, and a faction of three forms.
fn challenged(w: &mut World, ids: &[AgentId]) {
    let cid = w.communities[0].id;
    for &id in &ids[2..4] {
        defer(w, id, ids[1], 0.5);
    }
    for &id in &ids[4..] {
        defer(w, id, ids[0], 0.5);
    }
    hf_sim::keepers::challenge(w, cid, ids[1], ids[0]);
}

/// The faction's evening reckoning, `days` days on.
fn evening(w: &mut World, days: u64) {
    let day = w.meta.tick / TICKS_PER_DAY + days;
    w.meta.tick = day * TICKS_PER_DAY + 17 * TICKS_PER_HOUR + 800;
    hf_sim::heresy::step(w);
}

#[test]
fn turned_back_they_hold_apart_keep_their_own_observance_and_split_off_with_the_relic() {
    let (mut w, ids) = valley(2975);
    let cid = w.communities[0].id;
    challenged(&mut w, &ids);
    let formed = w.events.pending.iter().find_map(|e| match &e.kind {
        EventKind::FactionFormed {
            leader,
            keeper,
            members,
            ..
        } => Some((*leader, *keeper, *members)),
        _ => None,
    });
    assert_eq!(formed, Some((ids[1], ids[0], 3)));
    assert!(hf_sim::heresy::apart(&w, cid, ids[3]));
    assert!(!hf_sim::heresy::apart(&w, cid, ids[5]));

    // The season's observance: the faction gathers to its leader, away from the rest.
    w.events.drain();
    let lead = idx(&w, ids[1]);
    w.agents[lead].pos = w.grid.surface(30, 55);
    let day = w.meta.epoch + 5 * TICKS_PER_DAY;
    w.meta.tick = day + 11 * TICKS_PER_HOUR;
    hf_sim::observances::step(&mut w);
    let going_to = |w: &World, id: AgentId| match &w.agents[idx(w, id)].activity {
        hf_sim::mind::activity::Activity::Going { to, .. } => Some(*to),
        _ => None,
    };
    let leader_at = w.agents[lead].pos;
    assert!(going_to(&w, ids[2]).is_some_and(|p| p.dist(leader_at) <= 3));
    // At noon, those gathered round the leader keep it apart; the rest keep it at home.
    for &id in &ids[2..4] {
        let i = idx(&w, id);
        w.agents[i].pos = w.grid.surface(31, 54);
        w.agents[i].activity = hf_sim::mind::activity::Activity::Idle;
    }
    w.meta.tick = day + 12 * TICKS_PER_HOUR;
    hf_sim::observances::step(&mut w);
    let apart = w.events.pending.iter().find_map(|e| match &e.kind {
        EventKind::FactionObservance { by, came, .. } => Some((*by, *came)),
        _ => None,
    });
    assert_eq!(apart, Some((ids[1], 2)));
    let main = w.events.pending.iter().find_map(|e| match &e.kind {
        EventKind::Observance { by, came, .. } => Some((*by, *came)),
        _ => None,
    });
    assert_eq!(main, Some((ids[0], 4)), "the keeper's, without the faction");

    // A relic of the community, kept at home.
    let home = w.communities[0].home;
    let item = w.create_item(
        ItemKind::Hammer,
        1,
        ItemPlace::Ground(home),
        0.9,
        "scenario",
    );
    w.faith.relics.push(hf_sim::relics::Relic {
        item,
        community: cid,
        of: ids[7],
        what: "hammer".into(),
        since: 0,
        held_by: None,
    });

    // Not yet two seasons: they hold together.
    w.events.drain();
    evening(&mut w, 1);
    assert_eq!(w.faith.factions.len(), 1);
    assert!(!w
        .events
        .pending
        .iter()
        .any(|e| matches!(e.kind, EventKind::Split { .. })));
    // Two seasons on, still apart and cold to the keeper: they split off.
    evening(&mut w, 10);
    let split = w.events.pending.iter().find_map(|e| match &e.kind {
        EventKind::Split {
            leader,
            members,
            cause,
            to,
            ..
        } => Some((*leader, *members, cause.clone(), *to)),
        _ => None,
    });
    let (leader, members, cause, to) = split.expect("they split off");
    assert_eq!((leader, members), (ids[1], 3));
    assert!(cause.contains("held the signs apart"), "{cause}");
    for &id in &ids[1..4] {
        assert_eq!(w.agents[idx(&w, id)].community, Some(to));
    }
    assert_eq!(w.agents[idx(&w, ids[5])].community, Some(cid));
    assert_eq!(hf_sim::keepers::keeper_of(&w, to), Some(ids[1]));
    assert!(w.faith.factions.is_empty());
    // The relic went with them: lost to the old, theirs now.
    assert!(w.events.pending.iter().any(|e| matches!(
        &e.kind,
        EventKind::RelicLost { community, by, .. } if *community == cid && *by == Some(ids[1])
    )));
    assert_eq!(
        w.faith
            .relics
            .iter()
            .find(|r| r.item == item)
            .map(|r| r.community),
        Some(to)
    );
}

#[test]
fn a_faction_whose_people_come_round_to_the_keeper_is_brought_back() {
    let (mut w, ids) = valley(2976);
    let cid = w.communities[0].id;
    challenged(&mut w, &ids);
    assert_eq!(w.faith.factions.len(), 1);
    // The keeper's warnings borne out: those who stood with the challenger follow the keeper.
    for &id in &ids[2..4] {
        defer(&mut w, id, ids[1], 0.0);
        defer(&mut w, id, ids[0], 0.8);
    }
    w.events.drain();
    evening(&mut w, 1);
    let ended = w.events.pending.iter().find_map(|e| match &e.kind {
        EventKind::FactionEnded { leader, how, .. } => Some((*leader, how.clone())),
        _ => None,
    });
    assert_eq!(ended, Some((ids[1], "came back to the keeper".to_string())));
    assert!(w.faith.factions.is_empty());
    assert!(!hf_sim::heresy::apart(&w, cid, ids[2]));
    assert!(ids
        .iter()
        .all(|id| w.agents[idx(&w, *id)].community == Some(cid)));
}
