//! #297 ask 7: the revered dead's things become relics, kept at the shrine; a relic carried off
//! is felt as a loss by believers; in hard times believers whose food is secured go to the
//! shrine to pray.

use hf_sim::events::EventKind;
use hf_sim::ids::{AgentId, ItemId, TilePos};
use hf_sim::items::{ItemKind, ItemPlace};
use hf_sim::mind::beliefs::{Source, Topic};
use hf_sim::mind::feelings::FeelingKind;
use hf_sim::mind::ThoughtKind;
use hf_sim::structures::StructureKind;
use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use hf_sim::world::gen::GenParams;
use hf_sim::world::Material;
use hf_sim::World;

/// A party of `n` by (30, 30), all heeding the signs, with a shrine twenty tiles off.
fn valley(seed: u64, n: usize) -> (World, Vec<AgentId>, TilePos) {
    let mut w = World::generate(seed, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.rebuild_caches();
    let ids = w.found_party(n);
    w.meta.autoplan = false;
    for (k, id) in ids.iter().enumerate() {
        let i = idx(&w, *id);
        w.agents[i].pos = w.grid.surface(30 + k as i32, 30);
        w.agents[i].activity = hf_sim::mind::activity::Activity::Idle;
        w.agents[i].needs.hunger = 1.0;
        w.agents[i]
            .beliefs
            .hold(Topic::DeitySigns, 0.6, 0.6, Source::Culture(0), 0);
    }
    // Ten days of meals: one short of food does not leave for the shrine (#415).
    let home = w.communities[0].home;
    w.drop_item(
        hf_sim::items::ItemKind::Meal,
        (n * 30) as u32,
        home.offset(3, 3, 0),
    );
    let cid = w.communities[0].id;
    let at = w.grid.surface(30, 50);
    w.place_structure(StructureKind::Shrine, at, Material::WOOD, false);
    w.faith.shrines.push(hf_sim::shrines::ShrineSite {
        community: cid,
        pos: at,
        since: 0,
    });
    assert_eq!(hf_sim::shrines::of(&w, cid), Some(at));
    w.meta.tick = w.meta.epoch + 3 * TICKS_PER_DAY;
    w.events.drain();
    (w, ids, at)
}

fn idx(w: &World, id: AgentId) -> usize {
    w.agents.iter().position(|a| a.id == id).unwrap()
}

/// A named tool in `who`'s hand.
fn give_tool(w: &mut World, who: AgentId) -> ItemId {
    let id = w.create_item(ItemKind::Hammer, 1, ItemPlace::Held(who), 0.9, "scenario");
    let i = idx(w, who);
    hf_sim::legend::take_up(w, i, id);
    id
}

/// `who` dies; the tick's events flushed, then the dead's things go to kin and relics are made.
fn dies(w: &mut World, who: AgentId) {
    hf_sim::systems::life::die(w, who, "old age".into());
    w.last_events = w.events.drain();
    hf_sim::legend::step(w);
    hf_sim::relics::step(w);
}

#[test]
fn the_keepers_tool_becomes_a_relic_kept_at_the_shrine_and_its_theft_is_felt() {
    let (mut w, ids, shrine) = valley(2971, 6);
    let cid = w.communities[0].id;
    w.faith.keepers.push(hf_sim::keepers::Keeper {
        community: cid,
        agent: ids[0],
        since: 0,
        challenged: 0,
    });
    let tool = give_tool(&mut w, ids[0]);
    dies(&mut w, ids[0]);
    let made = w.events.pending.iter().find_map(|e| match &e.kind {
        EventKind::RelicMade { of, why, .. } => Some((*of, why.clone())),
        _ => None,
    });
    assert_eq!(
        made,
        Some((ids[0], "keeper of the observances".to_string()))
    );
    assert!(hf_sim::relics::is_relic(&w, tool));
    // Out of use, set down by the shrine: nobody holds it.
    let it = w.items.iter().find(|it| it.id == tool).unwrap();
    assert!(matches!(it.place, ItemPlace::Ground(p) if p.dist(shrine) <= 1));
    assert!(w.agents.iter().all(|a| a.tool != Some(tool)));
    assert_eq!(
        hf_sim::relics::place_of(&w, cid).map(|p| p.0.dist(shrine) <= 1),
        Some(true)
    );

    // One of them picks it up and leaves the valley with it.
    w.last_events.clear();
    w.events.drain();
    w.meta.tick += 1;
    w.items.iter_mut().find(|it| it.id == tool).unwrap().place = ItemPlace::Carried(ids[1]);
    hf_sim::relics::step(&mut w);
    assert!(hf_sim::relics::is_relic(&w, tool), "carried is not lost");
    let before = w.agents[idx(&w, ids[2])].opinion_of(ids[1]);
    let thief = idx(&w, ids[1]);
    hf_sim::systems::society::leave_valley(&mut w, thief);
    w.meta.tick += 1;
    hf_sim::relics::step(&mut w);
    let lost = w.events.pending.iter().find_map(|e| match &e.kind {
        EventKind::RelicLost { why, by, .. } => Some((why.clone(), *by)),
        _ => None,
    });
    assert_eq!(
        lost,
        Some(("it was carried out of the valley".to_string(), Some(ids[1])))
    );
    assert!(!hf_sim::relics::is_relic(&w, tool));
    let k = idx(&w, ids[2]);
    assert!(w.agents[k].thoughts.has(ThoughtKind::RelicLost));
    assert!(w.agents[k].opinion_of(ids[1]) < before);
}

#[test]
fn only_the_revered_leave_relics_and_a_move_takes_them_along() {
    let (mut w, ids, _) = valley(2972, 6);
    // Two warnings of theirs borne out and none belied: revered.
    w.faith.record.push((ids[1], 2, 0));
    // As many belied as borne out: not.
    w.faith.record.push((ids[2], 2, 2));
    let t1 = give_tool(&mut w, ids[1]);
    let t2 = give_tool(&mut w, ids[2]);
    let t3 = give_tool(&mut w, ids[3]);
    dies(&mut w, ids[1]);
    assert!(hf_sim::relics::is_relic(&w, t1));
    w.last_events.clear();
    w.events.drain();
    dies(&mut w, ids[2]);
    w.last_events.clear();
    dies(&mut w, ids[3]);
    assert!(!hf_sim::relics::is_relic(&w, t2));
    assert!(!hf_sim::relics::is_relic(&w, t3));
    assert_eq!(w.faith.relics.len(), 1);
    // The community moves camp: they take the relic with them, as anyone would.
    w.last_events.clear();
    w.events.drain();
    let new_home = w.grid.surface(30, 3);
    w.communities[0].home = new_home;
    w.meta.tick = (w.meta.tick / TICKS_PER_HOUR + 1) * TICKS_PER_HOUR + 43;
    hf_sim::relics::step(&mut w);
    assert!(hf_sim::relics::is_relic(&w, t1), "not lost");
    let it = w.items.iter().find(|it| it.id == t1).unwrap();
    assert!(matches!(it.place, ItemPlace::Ground(p) if p.dist(new_home) <= 1));
    assert!(!w
        .events
        .pending
        .iter()
        .any(|e| matches!(e.kind, EventKind::RelicLost { .. })));
}

/// Each daylight hour of `days` days, at the pilgrims' minute, until `f` says to stop.
fn hours(w: &mut World, days: u64, mut f: impl FnMut(&mut World) -> bool) {
    let start = w.meta.tick / TICKS_PER_DAY + 1;
    for d in start..start + days {
        for h in 8..=16 {
            w.meta.tick = d * TICKS_PER_DAY + h * TICKS_PER_HOUR + 29;
            hf_sim::pilgrimage::step(w);
            if f(w) {
                return;
            }
        }
    }
}

#[test]
fn in_grief_a_believer_goes_to_the_shrine_and_is_eased_but_not_while_hungry() {
    let (mut w, ids, shrine) = valley(2973, 6);
    let tick = w.meta.tick;
    for &id in &ids[1..3] {
        let i = idx(&w, id);
        w.agents[i]
            .feelings
            .feel(FeelingKind::Grief, Some(ids[5]), 0.6, tick, None);
    }
    // ids[2] goes hungry the whole time: their food is not secured (#415).
    let hungry = idx(&w, ids[2]);
    let mut set_out = Vec::new();
    let watch = |w: &mut World, set_out: &mut Vec<AgentId>| {
        w.agents[hungry].needs.hunger = 0.2;
        for e in w.events.drain() {
            if let EventKind::Pilgrimage { agent, to, why } = e.kind {
                assert_eq!(to, "the shrine");
                assert_eq!(why, "in grief");
                set_out.push(agent);
            }
        }
        // Nobody else wanders off meanwhile.
        for a in w.agents.iter_mut() {
            if !matches!(a.activity, hf_sim::mind::activity::Activity::Going { .. }) {
                a.activity = hf_sim::mind::activity::Activity::Idle;
            }
        }
    };
    hours(&mut w, 4, |w| {
        watch(w, &mut set_out);
        !set_out.is_empty()
    });
    assert_eq!(set_out, vec![ids[1]], "the fed one in grief went");
    let k = idx(&w, ids[1]);
    assert!(matches!(
        &w.agents[k].activity,
        hf_sim::mind::activity::Activity::Going { to, .. } if to.dist(shrine) <= 2
    ));
    // Come to it, they pray there and are eased.
    w.agents[k].pos = w.grid.surface(shrine.x + 1, shrine.y + 1);
    let grief = |w: &World| {
        w.agents[k]
            .feelings
            .list
            .iter()
            .find(|f| f.kind == FeelingKind::Grief)
            .map(|f| f.strength)
            .unwrap()
    };
    let g0 = grief(&w);
    w.meta.tick += TICKS_PER_HOUR;
    hf_sim::pilgrimage::step(&mut w);
    assert!(w.agents[k].thoughts.has(ThoughtKind::Prayed));
    assert!(grief(&w) < g0);
    assert!(w.faith.pilgrims.is_empty());
    // Four days more: the hungry one, as much in grief, never goes.
    hours(&mut w, 4, |w| {
        watch(w, &mut set_out);
        false
    });
    assert!(!set_out.contains(&ids[2]), "{set_out:?}");
    // Those not in hard times do not go.
    assert!(!set_out.contains(&ids[3]));
}

/// OST-505: one warning sends each believer to pray over it once, not every few hours for as
/// long as it stays open; a second warning sends them again.
#[test]
fn one_warning_sends_each_person_to_pray_once() {
    let (mut w, ids, _shrine) = valley(2973, 4);
    let cid = w.communities[0].id;
    let warn = |w: &mut World, ev: u32| {
        let reader = ids[0];
        let until = w.meta.tick + 5 * TICKS_PER_DAY;
        w.faith.signs.push(hf_sim::faith::SignRead {
            reader,
            community: cid,
            good: false,
            of: "a bad omen".into(),
            event: hf_sim::ids::EventId(ev),
            until,
        });
    };
    let goings = |w: &mut World, going: &mut Vec<AgentId>| {
        // Nobody walks in this test: whoever set out is back, idle, by the next hour.
        w.faith.pilgrims.clear();
        for a in w.agents.iter_mut() {
            a.activity = hf_sim::mind::activity::Activity::Idle;
        }
        for e in w.events.drain() {
            if let EventKind::Pilgrimage { agent, why, .. } = e.kind {
                if why.starts_with("afraid") {
                    going.push(agent);
                }
            }
        }
    };
    let mut going = Vec::new();
    warn(&mut w, 900_001);
    hours(&mut w, 5, |w| {
        goings(w, &mut going);
        false
    });
    for id in &ids {
        let n = going.iter().filter(|a| *a == id).count();
        assert!(n <= 1, "{id:?} went {n} times over one warning");
    }
    assert!(!going.is_empty(), "somebody still goes to pray");
    let first = going.len();
    warn(&mut w, 900_002);
    hours(&mut w, 5, |w| {
        goings(w, &mut going);
        false
    });
    for id in &ids {
        let n = going.iter().filter(|a| *a == id).count();
        assert!(n <= 2, "{id:?} went {n} times over two warnings");
    }
    assert!(going.len() > first, "a second warning sends them again");
}
