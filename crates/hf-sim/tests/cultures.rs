//! P7 gates (docs/design-peoples.md sections 5.1 and 6): every people has a culture with a
//! stance on each act and a weight on each value; far settlements grow cultures of their own,
//! recorded; a person's stance traces to its source; and reactions differ by culture, each a
//! thought with the row that caused it.

use hf_sim::agents::Species;
use hf_sim::events::EventKind;
use hf_sim::ids::TilePos;
use hf_sim::mind::beliefs::{Source, Topic};
use hf_sim::mind::ThoughtKind;
use hf_sim::plants::{Plant, PlantKind};
use hf_sim::realm::culture::Rite;
use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use hf_sim::world::gen::GenParams;
use hf_sim::World;

fn has_thought(
    w: &World,
    id: hf_sim::ids::AgentId,
    k: ThoughtKind,
) -> Option<Option<hf_sim::ids::EventId>> {
    w.agents.iter().find(|a| a.id == id).and_then(|a| {
        a.thoughts
            .list
            .iter()
            .find(|t| t.kind == k)
            .map(|t| t.source)
    })
}

#[test]
fn every_people_has_a_culture() {
    let w = World::from_setup(&hf_sim::setup::WorldSetup::new(21));
    let r = &w.realm;
    assert!(r.cultures.len() >= r.peoples.len());
    for p in &r.peoples {
        let c = &r.cultures[p.id as usize];
        assert_eq!(c.people, p.id);
        assert_eq!(c.stances.len(), Topic::ACTS.len() + Topic::VALUES.len());
        assert!(c.stances.iter().all(|(_, v)| v.abs() <= 1.0));
    }
    let again = World::from_setup(&hf_sim::setup::WorldSetup::new(21));
    assert_eq!(r.cultures, again.realm.cultures);
}

#[test]
fn far_settlements_grow_cultures_of_their_own() {
    let mut splits = 0;
    for seed in 1..5 {
        let mut s = hf_sim::setup::WorldSetup::new(seed);
        s.realm_years = 1000;
        let w = World::from_setup(&s);
        for c in w.realm.cultures.iter().filter(|c| c.from.is_some()) {
            splits += 1;
            let site = &w.realm.sites[c.site.unwrap() as usize];
            // Its own ways, unless it was taken in war and holds its takers' since (P9).
            assert!(
                site.culture == c.id
                    || w.realm.cultures[site.culture as usize].people == site.people
            );
            assert_eq!(w.realm.cultures[c.from.unwrap() as usize].people, c.people);
        }
    }
    assert!(splits > 0, "some far settlement grew a culture of its own");
}

#[test]
fn a_strangers_stance_traces_to_their_culture() {
    let mut w = World::from_setup(&hf_sim::setup::WorldSetup::new(22));
    let id = w.spawn_agent(Species::Person, TilePos::new(30, 30, 2));
    let c = w.realm.cultures[0].clone();
    hf_sim::species::raise_in(&mut w, id, 0);
    let a = w.agents.iter().find(|a| a.id == id).unwrap();
    let (t, v) = *c
        .stances
        .iter()
        .find(|(_, v)| v.abs() >= 0.2)
        .expect("a firm stance");
    let b = a.beliefs.get(t).expect("held");
    assert!(b.sources.contains(&Source::Culture(0)), "{:?}", b.sources);
    assert!((b.stance - v).abs() < 1e-6);
    assert_eq!(a.rite, c.rite);
}

fn village(seed: u64, n: usize) -> (World, Vec<hf_sim::ids::AgentId>) {
    let mut w = World::generate(seed, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.plants.clear();
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.meta.tick = TICKS_PER_HOUR * 8;
    w.rebuild_caches();
    let ids = w.found_party(n);
    w.ensure_nav();
    (w, ids)
}

#[test]
fn two_rites_two_thoughts() {
    // A rite can also be marred by chance (#338, `rites::outcome`, a roll keyed on the tick it is
    // given), and then every mourner is troubled whatever rite they keep, which says nothing of
    // the rite kept. That roll is not what this test is about, so the death is moved on a tick
    // at a time until the rite given is not marred (#553, FIXLOG 2026-11-01: the walks changed
    // who reached the body when, and on seed 4501 the rite was marred).
    for shift in 0..12u64 {
        let (mut w, ids) = village(4501, 4);
        w.meta.tick += shift;
        let tick = w.meta.tick;
        // The dead kept the Bury rite, as does one of the living; another keeps a hall for the
        // dead and holds that the dead must have their rite; a fourth keeps a hall but does not
        // care.
        let set = |w: &mut World, id, rite: Rite, burial: f32| {
            let a = w.agents.iter_mut().find(|a| a.id == id).unwrap();
            a.rite = rite;
            a.beliefs
                .hold(Topic::Burial, burial, 0.9, Source::Culture(0), tick);
        };
        set(&mut w, ids[0], Rite::Bury, 0.7);
        set(&mut w, ids[1], Rite::Bury, 0.7);
        set(&mut w, ids[2], Rite::Hall, 0.8);
        set(&mut w, ids[3], Rite::Hall, -0.6);
        for i in 1..4 {
            let dead = ids[0];
            let a = w.agents.iter_mut().find(|a| a.id == ids[i]).unwrap();
            a.relate(dead, 0.5, 0.5, tick);
        }
        hf_sim::systems::life::die(&mut w, ids[0], "fever".into());
        // The rite is given once someone has come to the body (C68, #220): step until then.
        let mut ev = None;
        let mut marred = false;
        for _ in 0..TICKS_PER_DAY {
            w.step();
            if let Some(e) = w
                .last_events
                .iter()
                .find(|e| matches!(&e.kind, EventKind::LaidToRest { agent, rite: Rite::Bury, .. } if *agent == ids[0]))
            {
                ev = Some(e.id);
                marred = w.last_events.iter().any(|e| {
                    matches!(&e.kind, EventKind::RiteOutcome { agent, outcome, .. }
                        if *agent == ids[0] && outcome == "marred")
                });
                break;
            }
        }
        let ev = ev.expect("laid to rest by their own rite");
        if marred {
            continue;
        }
        assert_eq!(
            has_thought(&w, ids[2], ThoughtKind::RiteBroken),
            Some(Some(ev))
        );
        assert_eq!(
            has_thought(&w, ids[1], ThoughtKind::RiteBroken),
            None,
            "their own rite"
        );
        assert_eq!(
            has_thought(&w, ids[3], ThoughtKind::RiteBroken),
            None,
            "it does not matter to them"
        );
        return;
    }
    panic!("every rite given in twelve tries was marred");
}

#[test]
fn a_felled_tree_troubles_those_who_hold_it_wrong() {
    let (mut w, ids) = village(4502, 2);
    let tick = w.meta.tick;
    let tree = TilePos::new(36, 30, 2);
    w.plants.push(Plant::new(tree, PlantKind::Tree, 1.0, 1));
    w.rebuild_caches();
    {
        let a = w.agents.iter_mut().find(|a| a.id == ids[1]).unwrap();
        a.beliefs
            .hold(Topic::CutLivingTree, -0.9, 0.9, Source::Culture(0), tick);
        a.pos = TilePos::new(33, 30, 2);
    }
    hf_sim::systems::rites::tree_felled(&mut w, ids[0], tree);
    hf_sim::systems::flush::step(&mut w);
    let ev = w
        .last_events
        .iter()
        .find(|e| matches!(e.kind, EventKind::TreeFelled { .. }))
        .expect("the felling is a row")
        .id;
    assert_eq!(
        has_thought(&w, ids[1], ThoughtKind::TreeFelled),
        Some(Some(ev))
    );
    assert_eq!(has_thought(&w, ids[0], ThoughtKind::TreeFelled), None);
}

#[test]
fn a_new_kind_joining_is_welcome_to_some_and_not_others() {
    let (mut w, ids) = village(4503, 3);
    let tick = w.meta.tick;
    let cid = w.communities[0].id;
    let grisk = hf_sim::species::by_key("grisk").unwrap();
    let newcomer = w.spawn_agent(Species::Person, TilePos::new(32, 32, 2));
    hf_sim::species::give(&mut w, newcomer, grisk, None);
    w.join_community(newcomer, cid);
    for (id, lean) in [(ids[0], 0.8), (ids[1], -0.8), (ids[2], 0.0)] {
        let a = w.agents.iter_mut().find(|a| a.id == id).unwrap();
        a.beliefs
            .hold(Topic::OtherRaces, lean, 0.9, Source::Culture(0), tick);
    }
    let joined = w.events.push(
        tick,
        None,
        hf_sim::events::Severity::Major,
        EventKind::Joined {
            agent: newcomer,
            community: cid,
        },
    );
    hf_sim::systems::rites::joined(&mut w, newcomer, cid, joined);
    assert_eq!(
        has_thought(&w, ids[0], ThoughtKind::NewKindWelcome),
        Some(Some(joined))
    );
    assert_eq!(
        has_thought(&w, ids[1], ThoughtKind::NewKindUneasy),
        Some(Some(joined))
    );
    assert_eq!(has_thought(&w, ids[2], ThoughtKind::NewKindWelcome), None);
    assert_eq!(has_thought(&w, ids[2], ThoughtKind::NewKindUneasy), None);
}

/// Whether the one person of a village fells the tree they are asked to within `hours`.
fn felled_within(cut_living: Option<f32>, hours: u64) -> bool {
    let (mut w, ids) = village(4504, 1);
    let tick = w.meta.tick;
    let tree = TilePos::new(34, 30, 2);
    w.plants.push(Plant::new(tree, PlantKind::Tree, 1.0, 1));
    w.rebuild_caches();
    if let Some(v) = cut_living {
        let a = w.agents.iter_mut().find(|a| a.id == ids[0]).unwrap();
        a.beliefs
            .hold(Topic::CutLivingTree, v, 0.9, Source::Culture(0), tick);
    }
    let cid = w.communities[0].id;
    let mut d =
        hf_sim::systems::settlement::make(&mut w, hf_sim::work::DesignationKind::Chop, tree, 9);
    d.community = Some(cid);
    w.designations.push(d);
    for _ in 0..hours * TICKS_PER_HOUR {
        w.step();
        if !w
            .plants
            .iter()
            .any(|p| p.pos == tree && p.alive && p.kind == PlantKind::Tree)
        {
            return true;
        }
    }
    false
}

#[test]
fn one_who_holds_it_wrong_will_not_fell_a_living_tree() {
    assert!(
        felled_within(None, 12),
        "the tree is felled by someone who does not mind"
    );
    assert!(
        !felled_within(Some(-0.9), 12),
        "nobody who holds it wrong fells it"
    );
}
