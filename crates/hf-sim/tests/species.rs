//! P3 gates (docs/design-peoples.md sections 1, 2.2 and 12): the species library loads and
//! refuses bad files, old saves keep their people human, the world's peoples are of species,
//! people who come from a settlement are of its species, and the species is named and explained.

use hf_sim::agents::Species;
use hf_sim::species::{self, parse, SpeciesId, SKIN_NAMES};

const GOOD: &str = include_str!("../../../data/peoples/species/grisk.toml");

#[test]
fn the_library_loads() {
    let lib = species::library();
    // Six of the surface, three of the deep (P10), three more of the surface (C50): the
    // design's twelve anchors. Ids never move.
    assert_eq!(lib.len(), 12);
    assert_eq!(lib[0].id, "human");
    assert_eq!(species::by_key("grisk"), Some(SpeciesId(2)));
    assert_eq!(species::by_key("below"), Some(SpeciesId(8)));
    for s in lib {
        assert!(s.sentient, "{}", s.id);
        assert!(hf_sim::plan::plan(&s.plan).is_some(), "{}", s.id);
    }
    assert_eq!(species::library_hash().len(), 16);
}

#[test]
fn the_library_refuses_bad_files() {
    assert!(parse("grisk", GOOD).is_ok());
    for (what, from, to) in [
        (
            "no such plan",
            "plan     = \"small_biped\"",
            "plan = \"octopod\"",
        ),
        ("unknown skin", "\"lichen\"", "\"plaid\""),
        (
            "grown before born",
            "mature_years = 10",
            "mature_years = 30",
        ),
        ("a sense above 3", "dark_sight = 3", "dark_sight = 5"),
        (
            "comfort out of order",
            "comfort_c = [-4, 20]",
            "comfort_c = [20, -4]",
        ),
        ("unknown topic", "strangers = 0.6", "luck = 0.6"),
    ] {
        let bad = GOOD.replace(from, to);
        assert_ne!(bad, GOOD, "{what}: the test edit applied");
        assert!(parse("grisk", &bad).is_err(), "{what}");
    }
    assert!(parse("hollin", GOOD).is_err(), "the id must match the file");
}

#[test]
fn old_saves_keep_their_people_human() {
    let s: Species = serde_json::from_str("\"Human\"").unwrap();
    assert_eq!(s, Species::Person);
    assert_eq!(SpeciesId::default(), SpeciesId::HUMAN);
    assert_eq!(species::get(SpeciesId(999)).id, "human");
}

#[test]
fn the_worlds_peoples_are_of_species() {
    let mut kinds = std::collections::BTreeSet::new();
    for seed in 1..12 {
        let a = hf_sim::World::from_setup(&hf_sim::setup::WorldSetup::new(seed));
        let b = hf_sim::World::from_setup(&hf_sim::setup::WorldSetup::new(seed));
        let sa: Vec<_> = a.realm.peoples.iter().map(|p| p.species).collect();
        let sb: Vec<_> = b.realm.peoples.iter().map(|p| p.species).collect();
        assert_eq!(sa, sb, "same seed, same peoples");
        kinds.extend(sa);
    }
    assert!(kinds.len() >= 4, "{kinds:?}");
}

#[test]
fn one_from_a_settlement_is_of_its_people() {
    let mut w = hf_sim::World::from_setup(&hf_sim::setup::WorldSetup::new(5));
    let site = w
        .overworld
        .sites
        .iter()
        .find(|s| s.realm_site.is_some())
        .expect("a settlement of the world")
        .id;
    let rs = w
        .overworld
        .sites
        .iter()
        .find(|s| s.id == site)
        .unwrap()
        .realm_site
        .unwrap();
    let pi = w.realm.sites[rs as usize].people as usize;
    let grisk = species::by_key("grisk").unwrap();
    w.realm.peoples[pi].species = grisk;
    assert_eq!(species::of_site(&w, site), Some((grisk, pi)));
    let id = w.spawn_agent(Species::Person, hf_sim::ids::TilePos::new(30, 30, 2));
    species::give(&mut w, id, grisk, Some(pi));
    let a = w.agents.iter().find(|a| a.id == id).unwrap();
    assert_eq!(a.people, grisk);
    let tone = SKIN_NAMES[a.appearance.skin as usize];
    assert!(
        species::get(grisk).art.skin.iter().any(|s| s == tone),
        "{tone}"
    );
    let m = a.mass_kg();
    assert!((30.0..=46.0).contains(&m), "{m}");
    assert!(a.hunger_scale() < 1.0 && a.heat_loss() > 1.0);
    assert!(a.lifespan_years() <= 40.0);
    // Named in their people's language, not the valley's.
    let lang_name = w.realm.peoples[pi]
        .language
        .name(w.meta.seed, "person", id.0 as u64);
    assert_eq!(a.name.to_lowercase(), lang_name.to_lowercase());
}

#[test]
fn a_species_is_named_and_explained() {
    let w = hf_sim::World::from_setup(&hf_sim::setup::WorldSetup::new(3));
    let c = hf_sim::chronicle::Chronicle::in_memory().unwrap();
    let grisk = species::by_key("grisk").unwrap();
    let page = hf_sim::view::legends::page(&w, &c, &format!("species:{}", grisk.0));
    assert!(page.contains("[b]Grisk[/b]"), "{page}");
    assert!(page.contains("see in the dark"), "{page}");
    assert!(page.contains("Peoples of this kind"), "{page}");
    let people = hf_sim::view::legends::page(&w, &c, "people:0");
    assert!(people.contains("[url=species:"), "{people}");
}

#[test]
fn a_stranger_of_another_kind_is_of_an_age_its_kind_is_grown_at() {
    let mut w = hf_sim::World::from_setup(&hf_sim::setup::WorldSetup::new(6));
    for key in ["screeling", "hollin", "grisk"] {
        let sp = species::by_key(key).unwrap();
        for k in 0..12 {
            let id = w.spawn_agent(Species::Person, hf_sim::ids::TilePos::new(20 + k, 30, 2));
            species::give(&mut w, id, sp, None);
            let a = w.agents.iter().find(|a| a.id == id).unwrap();
            let age = a.age_years(w.meta.tick);
            let (adult, elder) = a.stage_years();
            assert!(
                age >= adult && (age as f32) <= elder as f32 * 0.8 + 1.0,
                "{key} aged {age}"
            );
            assert!(!a.is_child(w.meta.tick), "{key} aged {age} is grown");
        }
    }
    // A Hollin of thirty is still a child; a human of thirty is not.
    let h = species::get(species::by_key("hollin").unwrap());
    assert!(h.life.mature_years > 30);
}

#[test]
fn humans_keep_their_ages_and_other_kinds_have_their_own() {
    let mut w = hf_sim::World::from_setup(&hf_sim::setup::WorldSetup::new(9));
    let now = w.meta.tick;
    let year = hf_sim::time::TICKS_PER_YEAR as i64;
    let h = w.spawn_agent(Species::Person, hf_sim::ids::TilePos::new(30, 30, 2));
    let hollin = species::by_key("hollin").unwrap();
    let o = w.spawn_agent(Species::Person, hf_sim::ids::TilePos::new(31, 30, 2));
    species::give(&mut w, o, hollin, None);
    let set_age = |w: &mut hf_sim::World, id, y: i64| {
        let a = w.agents.iter_mut().find(|a| a.id == id).unwrap();
        a.born = now as i64 - y * year - year / 2;
    };
    // A human of fifteen is not yet of age (as before P5); sixteen is.
    set_age(&mut w, h, 15);
    assert!(!w.agents.iter().find(|a| a.id == h).unwrap().of_age(now));
    set_age(&mut w, h, 16);
    let a = w.agents.iter().find(|a| a.id == h).unwrap();
    assert!(a.of_age(now));
    assert_eq!(a.fertile_years(), 16..46);
    assert_eq!(a.ailing_years(), (50, 55));
    // A Hollin of a hundred can still have children; old age comes at 250.
    set_age(&mut w, o, 100);
    let b = w.agents.iter().find(|a| a.id == o).unwrap();
    assert!(b.of_age(now) && b.fertile_years().contains(&100));
    assert_eq!(b.ailing_years().0, 250);
}

#[test]
fn a_species_skin_reaches_the_client_and_children_take_their_kinds_tones() {
    let mut w = hf_sim::World::from_setup(&hf_sim::setup::WorldSetup::new(8));
    let grisk = species::by_key("grisk").unwrap();
    let mum = w.spawn_agent(Species::Person, hf_sim::ids::TilePos::new(30, 30, 2));
    let dad = w.spawn_agent(Species::Person, hf_sim::ids::TilePos::new(31, 30, 2));
    species::give(&mut w, mum, grisk, None);
    species::give(&mut w, dad, grisk, None);
    let a = w.agents.iter().find(|a| a.id == mum).unwrap().clone();
    let look = hf_sim::view::look::look(&w, &a);
    assert_eq!(
        look.skin, a.appearance.skin,
        "the species tone is sent as it is"
    );
    assert!(look.skin as usize >= 6, "a Grisk tone, not a human one");
    let child = w.give_birth(mum, dad).unwrap();
    let c = w.agents.iter().find(|a| a.id == child).unwrap();
    let tone = SKIN_NAMES[c.appearance.skin as usize];
    assert!(
        species::get(grisk).art.skin.iter().any(|s| s == tone),
        "{tone}"
    );
}

#[test]
fn a_band_of_meat_eaters_eats_what_it_carries_before_foraging_again() {
    // The C43 soak: a Skarri band member, foraging berries worth half to a carnivore whenever
    // somewhat hungry, carried 1,300 stacks. Hungry with food in hand, they now eat it.
    let mut w = hf_sim::World::opening_world(3);
    w.meta.autoplan = true;
    w.meta.engine = hf_sim::engine::Engine::Relentless;
    let skarri = species::by_key("skarri").unwrap();
    for p in w.realm.peoples.iter_mut() {
        p.species = skarri;
    }
    let day = hf_sim::time::TICKS_PER_DAY;
    for _ in 0..4 * day {
        w.step();
    }
    if let Some(s) = w.overworld.sites.iter_mut().find(|s| s.people > 0) {
        s.food = -(s.people as f32) * 4.0;
        s.hungry_days = 3;
    }
    let mut worst = 0;
    let mut seen = false;
    for _ in 0..12 * day {
        w.step();
        for a in w.agents.iter().filter(|a| a.alive && a.band.is_some()) {
            seen |= a.people == skarri;
            let carried = w
                .items
                .iter()
                .filter(|it| {
                    it.kind.is_food() && it.place == hf_sim::items::ItemPlace::Carried(a.id)
                })
                .count();
            worst = worst.max(carried);
        }
    }
    assert!(seen, "a Skarri band came");
    assert!(worst <= 4, "a band member carried {worst} stacks of food");
}

#[test]
fn the_wizard_takes_kinds_and_deities_by_name() {
    // #147: the wizard's hook was given display names ("Tharn", "Skarri:rare", "Aun") and the
    // world fell back to a different one.
    let mut p = hf_sim::species::PeopleSetup {
        exclude: vec!["Tharn".into(), "the Below".into()],
        rarity: vec![("Skarri".into(), "Rare".into())],
        ..Default::default()
    };
    p.normalise();
    assert_eq!(p.exclude, vec!["tharn".to_string(), "below".to_string()]);
    assert_eq!(p.rarity, vec![("skarri".to_string(), "rare".to_string())]);
    assert!(p.check().is_ok());
    // What no kind answers to is named in the refusal.
    let mut bad = hf_sim::species::PeopleSetup {
        exclude: vec!["Dragons".into()],
        ..Default::default()
    };
    bad.normalise();
    assert!(bad.check().unwrap_err().contains("\"Dragons\""));
    // Leaving everyone out is refused, with what to do.
    let none = hf_sim::species::PeopleSetup {
        exclude: species::library().iter().map(|s| s.id.clone()).collect(),
        ..Default::default()
    };
    assert!(none
        .check()
        .unwrap_err()
        .contains("leave in at least one kind"));
    // Deities by their names as well as the pacing's.
    use hf_sim::engine::Engine;
    for (name, e) in [
        ("Aun", Engine::Steady),
        ("hask", Engine::Relentless),
        ("Mirre", Engine::Whimsical),
        ("steady", Engine::Steady),
    ] {
        assert_eq!(name.parse::<Engine>().unwrap(), e);
    }
    assert!("Zeus".parse::<Engine>().unwrap_err().contains("Aun"));
}
