//! Wars between realms (M3, C29).

use hf_sim::realm::sites::{self, RealmEventKind};
use hf_sim::realm::{self, RealmParams};

#[test]
fn wars_have_causes_and_battles_belong_to_their_wars() {
    let m = realm::generate(1, &RealmParams::default());
    let (rl, events) = sites::run(1, &m, 800);
    assert!(!rl.wars.is_empty(), "no war in 800 years");
    for w in &rl.wars {
        assert!(!w.cause.is_empty());
        assert_ne!(
            rl.polities[w.a as usize].people, rl.polities[w.b as usize].people,
            "a war within one people"
        );
    }
    for e in &events {
        if let RealmEventKind::Battle { war, won_by, .. } = e.kind {
            let w = &rl.wars[war as usize];
            assert!(w.began <= e.year && w.ended.as_ref().is_none_or(|(y, _)| *y >= e.year));
            assert!(won_by == w.a || won_by == w.b);
        }
        if let RealmEventKind::Died { figure, cause } = &e.kind {
            if let Some(place) = cause.strip_prefix("fell at ") {
                assert!(
                    rl.sites.iter().any(|s| s.name == place),
                    "{place} is no place"
                );
                assert!(rl.figures[*figure as usize].born + 16 <= e.year);
            }
        }
    }
}

#[test]
fn a_realm_holds_only_its_own_people_s_settlements() {
    for seed in [1u64, 4] {
        let m = realm::generate(seed, &RealmParams::default());
        let (rl, events) = sites::run(seed, &m, 700);
        for p in rl.polities.iter().filter(|p| p.ended.is_none()) {
            assert!(!p.members.is_empty(), "{} holds nothing", p.name);
            for &s in &p.members {
                assert_eq!(
                    rl.sites[s as usize].people, p.people,
                    "{} holds a settlement of another people",
                    p.name
                );
            }
        }
        // Every realm that ended has the event that ended it.
        for p in rl.polities.iter().filter(|p| p.ended.is_some()) {
            let (year, cause) = p.ended.clone().unwrap();
            assert!(events.iter().any(|e| e.year == year
                && matches!(&e.kind, RealmEventKind::PolityEnded { polity, cause: c } if *polity == p.id && *c == cause)));
        }
    }
}
