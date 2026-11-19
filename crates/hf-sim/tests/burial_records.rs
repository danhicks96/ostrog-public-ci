//! #566: every death of a person ends in a record of what became of the body. The dead of a
//! lost caravan are not in the valley: nobody walks to them, the death has no place on the
//! map, and they are mourned at home with `in_absence: true`.

use hf_sim::chronicle::Chronicle;
use hf_sim::events::EventKind;
use hf_sim::expedition::{CaravanState, Purpose};
use hf_sim::ids::TilePos;
use hf_sim::items::ItemKind;
use hf_sim::plants::{Plant, PlantKind};
use hf_sim::systems::expedition::form_caravan;
use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use hf_sim::world::gen::GenParams;
use hf_sim::{Session, World};

fn founded(seed: u64) -> (World, hf_sim::ids::CommunityId) {
    let mut w = World::generate(seed, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.plants.clear();
    for i in 0..120 {
        w.plants.push(Plant::new(
            TilePos::new(4 + (i * 7) % 56, 4 + (i * 11) % 56, 2),
            PlantKind::BerryBush,
            1.0,
            i as u32,
        ));
    }
    w.meta.tick = TICKS_PER_HOUR * 8;
    w.rebuild_caches();
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.found_party(10);
    let cid = w.communities[0].id;
    w.drop_item(ItemKind::Log, 40, TilePos::new(32, 30, 2));
    w.ensure_nav();
    (w, cid)
}

#[test]
fn the_dead_of_a_lost_caravan_are_mourned_in_their_absence() {
    let (w, cid) = founded(2001);
    let sid = w.overworld.sites[0].id;
    let mut s = Session::new(w, Chronicle::in_memory().unwrap());
    let id = form_caravan(&mut s.world, cid, Purpose::Scout { site: sid }).expect("a caravan");
    // Once they are on the road, stranded with nothing to eat: each day they may be lost
    // (systems/expedition.rs).
    for _ in 0..TICKS_PER_DAY * 3 {
        if matches!(
            s.world.overworld.caravans[0].state,
            CaravanState::Outbound { .. }
        ) {
            break;
        }
        s.step().unwrap();
    }
    assert!(
        !s.world.overworld.away.is_empty(),
        "the party has left the valley"
    );
    {
        let c = &mut s.world.overworld.caravans[0];
        c.cargo.clear();
        c.rations = 0;
        c.state = CaravanState::Stranded {
            days: 200,
            then_return: false,
            back_day: 0,
        };
    }
    let party = s.world.overworld.caravans[0].members.clone();
    assert!(!party.is_empty());
    let mut lost_at = None;
    for _ in 0..TICKS_PER_DAY * 60 {
        s.step().unwrap();
        if s.world
            .last_events
            .iter()
            .any(|e| matches!(&e.kind, EventKind::CaravanLost { caravan, .. } if *caravan == id))
        {
            lost_at = Some(s.world.meta.tick);
            // The deaths have no place: the bodies are not in the valley.
            for e in s.world.last_events.iter() {
                if let EventKind::Died { agent, .. } = &e.kind {
                    if party.contains(agent) {
                        assert_eq!(e.at, None, "a death on the road has no place in the valley");
                    }
                }
            }
            break;
        }
    }
    let lost_at = lost_at.expect("stranded with no food, the caravan is lost within 60 days");
    assert!(s
        .world
        .rites_due
        .iter()
        .filter(|r| party.contains(&r.dead))
        .all(|r| r.away));

    // Nobody walks to where the body is not; each of the party is mourned at home in a day.
    let mut mourned = Vec::new();
    while s.world.meta.tick < lost_at + TICKS_PER_DAY {
        s.step().unwrap();
        for e in s.world.last_events.iter() {
            if let EventKind::LaidToRest {
                agent, in_absence, ..
            } = &e.kind
            {
                if party.contains(agent) {
                    assert!(
                        *in_absence,
                        "{agent:?} was buried as if the body were there"
                    );
                    mourned.push(*agent);
                }
            }
        }
    }
    mourned.sort();
    let mut want = party.clone();
    want.sort();
    assert_eq!(mourned, want, "every member is mourned, once");
}

#[test]
fn a_person_with_no_people_dies_on_the_record_as_given_no_rite() {
    let (mut w, _) = founded(2002);
    let i = w
        .agents
        .iter()
        .position(|a| a.alive && a.community.is_some())
        .unwrap();
    w.agents[i].community = None;
    let id = w.agents[i].id;
    hf_sim::systems::life::die(&mut w, id, "thirst".into());
    let evs = w.events.drain();
    let row = evs
        .iter()
        .find(|e| matches!(&e.kind, EventKind::LeftUnrested { agent, .. } if *agent == id))
        .expect("a death with no rite is on the record");
    assert!(row.at.is_some(), "the body lies where they fell");
    assert!(w.agents[i]
        .conditions
        .iter()
        .any(|c| c.kind == hf_sim::conditions::ConditionKind::Unrested));
}
