//! P9 gates (docs/design-peoples.md section 7): settlements beyond the valley trade with each
//! other at fair prices and the world's routes are recorded per decade; allies fight beside each
//! other and never against; refugees flee with a recorded cause and carry their settlement into
//! the valley; and a child of two kinds is one parent's kind whole.

use hf_sim::realm::sites::RealmEventKind as K;
use hf_sim::species::SpeciesId;
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
fn trade_between_sites_balances() {
    let mut exchanges = 0;
    for seed in 1..5u64 {
        let mut w = World::opening_world(seed);
        for _ in 0..30 {
            hf_sim::trade::daily(&mut w);
        }
        let before = w.overworld.sites.clone();
        let total = |sites: &[hf_sim::expedition::Site]| {
            let mut t: std::collections::BTreeMap<hf_sim::items::ItemKind, f64> =
                Default::default();
            for s in sites {
                for (k, v) in &s.stores {
                    *t.entry(*k).or_default() += *v as f64;
                }
            }
            t
        };
        hf_sim::trade::between_sites(&mut w);
        // Nothing made from nothing: every kind's total across the settlements is unchanged.
        let (t0, t1) = (total(&before), total(&w.overworld.sites));
        for (k, v) in &t1 {
            let was = t0.get(k).copied().unwrap_or(0.0);
            assert!((v - was).abs() < 0.01, "seed {seed}: {k:?} {was} -> {v}");
        }
        // Fair at the buyer's prices: what one gives is worth what the other pays back, to
        // within one of the dearer thing.
        for (i, s) in w.overworld.sites.iter().enumerate() {
            for x in s.exchanges.iter().filter(|x| x.tick == w.meta.tick) {
                let j = w
                    .overworld
                    .sites
                    .iter()
                    .position(|o| o.id == x.with)
                    .unwrap();
                if j < i {
                    continue;
                }
                exchanges += 1;
                let buyer = &before[j];
                let price = |k| {
                    let m = buyer
                        .store_material
                        .get(&k)
                        .or(before[i].store_material.get(&k))
                        .copied()
                        .unwrap_or(hf_sim::world::Material::AIR);
                    hf_sim::trade::price_at(&w, buyer, k, m)
                };
                let (gave, got) = (
                    x.gave.1 as f32 * price(x.gave.0),
                    x.got.1 as f32 * price(x.got.0),
                );
                let unit = price(x.gave.0).max(price(x.got.0));
                assert!(
                    (gave - got).abs() <= unit + 0.01,
                    "seed {seed}: gave {gave} for {got}"
                );
            }
        }
    }
    assert!(exchanges > 0, "some settlements traded with each other");
}

#[test]
fn routes_are_recorded_between_cultures() {
    let mut w = old_world(41);
    let r = rows(&mut w);
    let mut traded = 0;
    for k in &r {
        if let K::Traded {
            a,
            b,
            caravans,
            cultures,
            ..
        } = k
        {
            traded += 1;
            assert!(*caravans >= 10);
            assert_ne!(a, b);
            assert_ne!(cultures.0, cultures.1, "a row is between two cultures");
        }
    }
    assert!(
        traded > 0,
        "a decade's caravans between two cultures is a row"
    );
    assert!(
        w.realm.relations.iter().any(|x| x.trade > 0 && x.a != x.b),
        "cultures that traded are related"
    );
}

#[test]
fn allies_join_battles_and_never_fight_each_other() {
    let mut joined = 0;
    for seed in 41..44 {
        let mut w = old_world(seed);
        let r = rows(&mut w);
        let realm = &w.realm;
        for k in &r {
            let K::Battle { war, with, .. } = k else {
                continue;
            };
            let wr = &realm.wars[*war as usize];
            for q in with {
                joined += 1;
                assert!(*q != wr.a && *q != wr.b, "an ally is not a side");
                assert!(
                    realm
                        .alliances
                        .iter()
                        .any(|x| (x.a == *q || x.b == *q)
                            && (x.against == wr.a || x.against == wr.b)),
                    "seed {seed}: realm {q} fought in war {war} without an alliance"
                );
            }
        }
        for wr in &realm.wars {
            for x in &realm.alliances {
                let pair = (x.a == wr.a && x.b == wr.b) || (x.a == wr.b && x.b == wr.a);
                let active = x.since <= wr.began && x.ended.as_ref().is_none_or(|e| e.0 > wr.began);
                assert!(!(pair && active), "seed {seed}: allies went to war");
            }
        }
    }
    assert!(joined > 0, "allies fought beside each other");
}

#[test]
fn refugees_flee_for_a_recorded_cause() {
    let mut w = old_world(42);
    let r = rows(&mut w);
    let mut fled = 0;
    for k in &r {
        if let K::Fled {
            from,
            to,
            count,
            cause,
            ..
        } = k
        {
            fled += 1;
            assert!(*count >= 3 && !cause.is_empty());
            assert_ne!(Some(*from), *to);
        }
    }
    assert!(fled > 0);
}

#[test]
fn refugees_carry_their_site() {
    let mut w = World::opening_world(3);
    let (rs, name) = w
        .overworld
        .sites
        .iter()
        .find_map(|s| s.realm_site.map(|r| (r, s.name.clone())))
        .expect("a neighbour of the world's");
    let site = w.realm.sites[rs as usize].clone();
    let before = w.agents.len();
    let year = w.realm.year;
    hf_sim::realm::live::refugees(
        &mut w,
        &[hf_sim::realm::sites::RealmEvent {
            year,
            kind: K::Fled {
                from: rs,
                to: None,
                people: site.people,
                count: 30,
                cause: "famine".into(),
            },
        }],
    );
    let came: Vec<_> = w.agents[before..].to_vec();
    assert!(came.len() >= 2, "a few of those who fled come");
    let species = w.realm.peoples[site.people as usize].species;
    for a in &came {
        assert_eq!(a.people, species, "of their settlement's kind");
        assert_eq!(a.culture, Some(site.culture), "raised in its ways");
        assert!(a.community.is_none());
    }
    let told = w.events.pending.iter().any(|e| {
        matches!(&e.kind, hf_sim::events::EventKind::Refugees { from, cause, agents }
            if *from == name && cause == "famine" && agents.len() == came.len())
    });
    assert!(told, "the feed says who came, from where and why");
}

#[test]
fn hybrid_takes_one_plan() {
    let human = SpeciesId::HUMAN;
    let hollin = hf_sim::species::by_key("hollin").unwrap();
    let grisk = hf_sim::species::by_key("grisk").unwrap();
    assert!(hf_sim::species::fertile(human, hollin));
    assert!(
        !hf_sim::species::fertile(human, grisk),
        "only kinds that list each other"
    );
    // In the valley: a Hollin mother and a human father.
    let mut w = World::opening_world(5);
    let p = hf_sim::ids::TilePos::new(30, 30, 2);
    let m = w.spawn_agent(hf_sim::agents::Species::Person, p);
    let f = w.spawn_agent(hf_sim::agents::Species::Person, p);
    hf_sim::species::give(&mut w, m, hollin, None);
    let mut kinds = std::collections::BTreeSet::new();
    for k in 0..12u64 {
        w.meta.tick += 997 * (k + 1);
        let c = w.give_birth(m, f).unwrap();
        let get = |id| w.agents.iter().find(|a| a.id == id).unwrap();
        let (child, mother, father) = (get(c), get(m), get(f));
        let parent = if child.people == mother.people {
            mother
        } else {
            assert_eq!(
                child.people, father.people,
                "one parent's kind, never another"
            );
            father
        };
        assert_eq!(
            child.genome.body, parent.genome.body,
            "that parent's body whole"
        );
        assert_eq!(child.species_def().plan, parent.species_def().plan);
        kinds.insert(child.people);
        let line = hf_sim::view::narrate::line(&w, w.events.pending.last().unwrap());
        assert!(line.contains("of a Hollin mother"), "{line}");
    }
    assert_eq!(kinds.len(), 2, "children of both kinds");
    // In the world: a figure born of two peoples is one of them, of kinds that have children.
    let mut w = old_world(41);
    let r = rows(&mut w);
    let realm = &w.realm;
    for k in &r {
        if let K::Born {
            figure,
            parent: Some(p),
            other: Some(o),
            ..
        } = k
        {
            let people = |f: u32| realm.figures[f as usize].people;
            let sp = |f: u32| realm.peoples[people(f) as usize].species;
            assert!(hf_sim::species::fertile(sp(*p), sp(*o)));
            assert!(people(*figure) == people(*p) || people(*figure) == people(*o));
        }
    }
}
