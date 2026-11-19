//! P10 gates (docs/design-peoples.md section 9): the deep's peoples live below, in settlements
//! of their own layer; a beast of the deep is its record; and breaking into a deep cavern is a
//! row, through which only what the world's record holds comes up.

use hf_sim::realm::sites::RealmEventKind as K;
use hf_sim::World;

fn old_world(seed: u64) -> World {
    let mut s = hf_sim::setup::WorldSetup::new(seed);
    s.realm_years = 1000;
    World::from_setup(&s)
}

fn rows(w: &mut World) -> Vec<K> {
    hf_sim::systems::flush::step(w);
    w.last_events
        .iter()
        .filter_map(|e| match &e.kind {
            hf_sim::events::EventKind::Realm { what } => Some(what.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn cave_peoples_live_below() {
    let mut deep_peoples = 0;
    for seed in 51..55 {
        let w = old_world(seed);
        let r = &w.realm;
        for p in r.peoples.iter().filter(|p| p.deep) {
            deep_peoples += 1;
            let kind = w.kind(p.species);
            assert!(
                !kind.habitat.layers.iter().any(|l| l == "surface"),
                "{} live below",
                kind.id
            );
        }
        for s in &r.sites {
            assert_eq!(
                s.deep, r.peoples[s.people as usize].deep,
                "seed {seed}: {} is held by a people of the other layer",
                s.name
            );
        }
        // The roads out of the valley lead to the land's settlements, never the deep's.
        for o in &w.overworld.sites {
            if let Some(rs) = o.realm_site {
                assert!(!r.sites[rs as usize].deep);
            }
        }
    }
    assert!(deep_peoples > 0, "some world has peoples of the deep");
}

#[test]
fn a_beast_is_its_record() {
    let mut beasts = 0;
    for seed in 51..55 {
        let mut w = old_world(seed);
        let r = rows(&mut w);
        for b in &w.realm.beasts {
            beasts += 1;
            let woke = r
                .iter()
                .filter(|k| matches!(k, K::BeastWoke { beast, .. } if *beast == b.id))
                .count();
            assert_eq!(woke, 1, "{} woke once, in the record", b.name);
            for site in &b.ruined {
                assert!(r.iter().any(
                    |k| matches!(k, K::BeastAttacked { beast, site: s, .. } if *beast == b.id && s == site)
                ));
                assert!(r.iter().any(|k| matches!(k, K::Ended { site: s, cause }
                    if s == site && cause == &format!("ruined by {}", b.name))));
            }
        }
        // Every attack names a beast that woke and a settlement of the deep.
        for k in &r {
            if let K::BeastAttacked { beast, site, .. } = k {
                assert!((*beast as usize) < w.realm.beasts.len());
                assert!(w.realm.sites[*site as usize].deep);
            }
        }
    }
    assert!(beasts > 0, "something woke in some world");
}

#[test]
fn the_breach_is_a_row() {
    let mut breached = 0;
    for seed in 51..55 {
        let mut w = World::from_setup(&hf_sim::setup::WorldSetup::new(seed));
        let Some(k) = w.grid.caverns.iter().position(|c| c.deep) else {
            continue;
        };
        let c = w.grid.caverns[k].clone();
        assert!(c.centre.z <= w.grid.strata[w.grid.col(c.centre.x, c.centre.y)].2 as i32);
        let i = w
            .agents
            .iter()
            .position(|a| a.alive && a.community.is_some())
            .unwrap();
        w.agents[i].pos = c.centre;
        let before = w.agents.len();
        hf_sim::underground::breaches(&mut w);
        let ev: Vec<_> = w.events.pending.iter().map(|e| e.kind.clone()).collect();
        let at = ev
            .iter()
            .position(|e| matches!(e, hf_sim::events::EventKind::Breached { cavern, .. } if *cavern == c.name))
            .expect("the breach is a row");
        breached += 1;
        assert!(w.grid.caverns[k].opened);
        // Only once.
        let n = w.events.pending.len();
        hf_sim::underground::breaches(&mut w);
        assert_eq!(w.events.pending.len(), n);
        // Whoever came up is of a deep people the world holds.
        for a in &w.agents[before..] {
            let p = w
                .realm
                .peoples
                .iter()
                .find(|p| p.species == a.people)
                .unwrap();
            assert!(p.deep);
        }
        // A beast lairing near: its name is from the record, and the community learned from
        // the breach that the deep is not to be dug.
        for e in &ev[at..] {
            if let hf_sim::events::EventKind::BeastStirred { beast, .. } = e {
                assert!(w.realm.beasts.iter().any(|b| &b.name == beast));
                let a = &w.agents[i];
                assert!(a.beliefs.lean(hf_sim::mind::beliefs::Topic::DeepDigging) < 0.0);
            }
        }
    }
    assert!(breached > 0, "some valley has a deep cavern");
}

#[test]
fn a_beast_near_the_breach_is_known_by_its_record() {
    let mut w = World::from_setup(&hf_sim::setup::WorldSetup::new(52));
    let region = w.meta.setup.as_ref().and_then(|s| s.region).unwrap();
    // The world's first beast, lairing under the valley.
    let b = w
        .realm
        .beasts
        .iter()
        .position(|b| b.layer == "deep_caverns")
        .expect("a beast of the deep caverns");
    w.realm.beasts[b].region = region;
    let name = w.realm.beasts[b].name.clone();
    let k = w.grid.caverns.iter().position(|c| c.deep).unwrap();
    let c = w.grid.caverns[k].clone();
    let i = w
        .agents
        .iter()
        .position(|a| a.alive && a.community.is_some())
        .unwrap();
    w.agents[i].pos = c.centre;
    hf_sim::underground::breaches(&mut w);
    let breach = w
        .events
        .pending
        .iter()
        .find(|e| matches!(e.kind, hf_sim::events::EventKind::Breached { .. }))
        .map(|e| e.id)
        .unwrap();
    assert!(w.events.pending.iter().any(|e| matches!(&e.kind,
        hf_sim::events::EventKind::BeastStirred { beast, .. } if *beast == name)));
    // Each of the community now holds the deep is not to be dug, from the breach.
    let cid = w.agents[i].community;
    for a in w.agents.iter().filter(|a| a.alive && a.community == cid) {
        let held = a
            .beliefs
            .list
            .iter()
            .find(|x| x.topic == hf_sim::mind::beliefs::Topic::DeepDigging)
            .expect("a stance on digging deep");
        assert!(held.stance < 0.0);
        assert!(held
            .sources
            .contains(&hf_sim::mind::beliefs::Source::Event(breach)));
    }
}
