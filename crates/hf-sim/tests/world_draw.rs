//! P5 gates (docs/design-peoples.md section 4): the world draws its peoples by the wizard's
//! choices, recorded in the setup; the same seed and choices make the same world; rarity sets
//! how a people starts (a rare one has no land, a legendary one's land fails), never how it ends.

use hf_sim::setup::WorldSetup;
use hf_sim::species::{self, PeopleSetup};

fn world(seed: u64, p: PeopleSetup) -> hf_sim::World {
    let mut s = WorldSetup::new(seed);
    s.peoples = p;
    hf_sim::World::from_setup(&s)
}

fn kinds(w: &hf_sim::World) -> Vec<String> {
    w.realm
        .peoples
        .iter()
        .map(|p| w.kind(p.species).id.clone())
        .collect()
}

#[test]
fn same_peoples_same_world() {
    let p = PeopleSetup {
        exclude: vec!["tharn".into()],
        rarity: vec![("skarri".into(), "rare".into())],
        ..Default::default()
    };
    let a = world(51, p.clone());
    let b = world(51, p);
    assert_eq!(kinds(&a), kinds(&b));
    assert_eq!(hf_sim::digest::digest(&a), hf_sim::digest::digest(&b));
}

#[test]
fn a_kind_left_out_is_not_in_the_world() {
    for seed in 1..8 {
        let w = world(
            seed,
            PeopleSetup {
                exclude: vec!["human".into(), "grisk".into()],
                exclude_plans: vec!["winged".into()],
                ..Default::default()
            },
        );
        for k in kinds(&w) {
            assert!(
                !["human", "grisk", "skarri"].contains(&k.as_str()),
                "{k} on seed {seed}"
            );
        }
    }
}

#[test]
fn rarity_changes_the_world_and_kinds_spread_before_repeating() {
    let base = world(52, PeopleSetup::default());
    let k = kinds(&base);
    // Fewer peoples than kinds: none repeats.
    let mut uniq = k.clone();
    uniq.sort();
    uniq.dedup();
    assert_eq!(uniq.len(), k.len(), "{k:?}");
    // Making the first people's kind legendary changes who is drawn or how it fares.
    let first = k[0].clone();
    let other = world(
        52,
        PeopleSetup {
            rarity: vec![(first.clone(), "legendary".into())],
            ..Default::default()
        },
    );
    assert_ne!(
        hf_sim::digest::digest(&base),
        hf_sim::digest::digest(&other)
    );
}

#[test]
fn a_rare_people_has_no_land_and_lives_among_others() {
    let mut found = false;
    for seed in 1..10 {
        let w = world(
            seed,
            PeopleSetup {
                rarity: species::library()
                    .iter()
                    .map(|s| (s.id.clone(), "rare".to_string()))
                    .filter(|(k, _)| k != "human")
                    .collect(),
                count: [6, 6],
                ..Default::default()
            },
        );
        // (A rare people of the deep still has its caverns, P10.)
        for p in w
            .realm
            .peoples
            .iter()
            .filter(|p| p.rarity == "rare" && !p.deep)
        {
            assert!(
                !w.realm.sites.iter().any(|s| s.people == p.id),
                "a rare people founds nothing"
            );
            found |= w.realm.sites.iter().any(|s| s.minority == Some(p.id));
        }
    }
    assert!(found, "some rare people lives as households among others");
}

#[test]
fn a_legendary_peoples_land_fails_it() {
    // Over a thousand years most of a legendary people's places are left empty.
    let mut ended = 0;
    let mut all = 0;
    for seed in 1..6 {
        let mut s = WorldSetup::new(seed);
        s.realm_years = 1000;
        s.peoples.rarity = species::library()
            .iter()
            .map(|x| (x.id.clone(), "legendary".to_string()))
            .collect();
        let w = hf_sim::World::from_setup(&s);
        let legendary = |x: &&hf_sim::realm::sites::RSite| {
            w.realm.peoples[x.people as usize].rarity == "legendary"
        };
        all += w.realm.sites.iter().filter(legendary).count();
        ended += w
            .realm
            .sites
            .iter()
            .filter(legendary)
            .filter(|x| !x.alive())
            .count();
    }
    assert!(ended * 10 >= all * 6, "{ended} of {all} left empty");
}

#[test]
fn the_choices_are_checked_and_recorded() {
    assert!(PeopleSetup {
        count: [5, 3],
        ..Default::default()
    }
    .check()
    .is_err());
    assert!(PeopleSetup {
        exclude: species::library().iter().map(|s| s.id.clone()).collect(),
        ..Default::default()
    }
    .check()
    .is_err());
    let p = PeopleSetup {
        exclude: vec!["tharn".into()],
        ..Default::default()
    };
    assert!(p.check().is_ok());
    assert_eq!(p.summary(), "the Tharn left out");
    let mut w = world(53, p);
    hf_sim::systems::flush::step(&mut w);
    let line = w.last_events.iter().find_map(|e| match &e.kind {
        hf_sim::events::EventKind::SetupChosen { peoples, .. } => Some(peoples.clone()),
        _ => None,
    });
    assert_eq!(line.as_deref(), Some("the Tharn left out"));
}
