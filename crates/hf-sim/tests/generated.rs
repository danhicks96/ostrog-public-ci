//! P6 gates (docs/design-peoples.md sections 1.4 and 11): each world generates species of its
//! own from the seed, every one differing from humans on at least four simulated axes, valid
//! like a hand-made one, looking unlike every other species of its world, and kept whole in
//! the save.

use hf_sim::species::{self, axes_from_human, generate, SpeciesId, GENERATED};

#[test]
fn generated_species_differ_four_ways_and_pass_every_check() {
    for seed in 0..100 {
        for s in generate(seed, 30) {
            assert!(
                axes_from_human(&s) >= 4,
                "seed {seed} {}: {}",
                s.id,
                axes_from_human(&s)
            );
            species::validate(&s).unwrap_or_else(|e| panic!("seed {seed}: {e}"));
        }
    }
    assert_eq!(
        generate(7, 30),
        generate(7, 30),
        "a pure function of the seed"
    );
    assert_ne!(generate(7, 30), generate(8, 30));
}

#[test]
fn generated_species_look_unlike_each_other() {
    for seed in 0..40 {
        let gen = generate(seed, 30);
        let mut looks: Vec<(String, String)> = species::library()
            .iter()
            .map(|s| (s.art.body.clone(), s.art.skin[0].clone()))
            .collect();
        for s in &gen {
            let look = (s.art.body.clone(), s.art.skin[0].clone());
            assert!(
                !looks.contains(&look),
                "seed {seed}: {} looks like another",
                s.id
            );
            looks.push(look);
        }
    }
}

#[test]
fn worlds_draw_generated_peoples_and_off_means_none() {
    let mut seen = 0;
    for seed in 1..8 {
        let mut s = hf_sim::setup::WorldSetup::new(seed);
        s.peoples.generated = "many".into();
        let w = hf_sim::World::from_setup(&s);
        assert_eq!(w.generated.len(), 30);
        seen += w
            .realm
            .peoples
            .iter()
            .filter(|p| p.species.0 >= GENERATED)
            .count();
        let mut off = hf_sim::setup::WorldSetup::new(seed);
        off.peoples.generated = "off".into();
        let w = hf_sim::World::from_setup(&off);
        assert!(w.generated.is_empty());
        assert!(w.realm.peoples.iter().all(|p| p.species.0 < GENERATED));
    }
    assert!(seen > 0, "generated kinds are drawn into worlds");
}

#[test]
fn generated_species_survive_a_save() {
    let mut s = hf_sim::setup::WorldSetup::new(4);
    s.peoples.generated = "many".into();
    let mut w = hf_sim::World::from_setup(&s);
    let id = w.spawn_agent(
        hf_sim::agents::Species::Person,
        hf_sim::ids::TilePos::new(30, 30, 2),
    );
    let kind = SpeciesId(GENERATED + 5);
    species::give(&mut w, id, kind, None);
    let name = w.kind(kind).name.clone();
    let mass = w.agents.iter().find(|a| a.id == id).unwrap().mass_kg();
    let bytes = hf_sim::save::encode(&hf_sim::save::snapshot::Snapshot::capture(&w)).unwrap();
    let back = hf_sim::save::decode(&bytes).unwrap().restore();
    assert_eq!(back.generated.len(), 30);
    assert_eq!(back.kind(kind).name, name);
    let a = back.agents.iter().find(|a| a.id == id).unwrap();
    assert_eq!(
        a.species_def().name,
        name,
        "the person's kind comes back with them"
    );
    assert_eq!(a.mass_kg(), mass);
    let c = hf_sim::chronicle::Chronicle::in_memory().unwrap();
    let page = hf_sim::view::legends::page(&back, &c, &format!("species:{}", kind.0));
    assert!(page.contains(&name), "{page}");
}

#[test]
fn a_wound_lands_on_a_part_the_body_has() {
    // A body without legs (a serpentine kind) is never wounded in a leg.
    let mut w = hf_sim::World::from_setup(&hf_sim::setup::WorldSetup::new(14));
    let mut snake = generate(14, 1).remove(0);
    snake.plan = "serpentine".into();
    let snake: &'static species::SpeciesDef = Box::leak(Box::new(snake));
    w.generated = vec![snake];
    let id = w.spawn_agent(
        hf_sim::agents::Species::Person,
        hf_sim::ids::TilePos::new(30, 30, 2),
    );
    species::give(&mut w, id, SpeciesId(GENERATED), None);
    let i = w.agents.iter().position(|a| a.id == id).unwrap();
    for k in 0..20 {
        w.meta.tick += 1;
        hf_sim::systems::health::wound(
            &mut w,
            i,
            hf_sim::body::Part::LEG_L,
            hf_sim::body::WoundKind::Break,
            0.1,
            0.0,
            "test",
            None,
        );
        let part = w.agents[i].body.wounds.last().unwrap().part;
        assert!(
            hf_sim::plan::plan("serpentine").unwrap().has(part),
            "wound {k} on {part:?}"
        );
        assert_eq!(
            part,
            hf_sim::plan::PartId::by_key("coil").unwrap(),
            "a leg's harm lands on the coils"
        );
    }
}

#[test]
fn a_world_with_generated_kinds_resumes_the_same() {
    let mut s = hf_sim::setup::WorldSetup::new(15);
    s.peoples.generated = "many".into();
    let mut a = hf_sim::World::from_setup(&s);
    a.meta.autoplan = true;
    for k in 0..4 {
        let id = a.spawn_agent(
            hf_sim::agents::Species::Person,
            hf_sim::ids::TilePos::new(28 + k, 30, 2),
        );
        species::give(&mut a, id, SpeciesId(GENERATED + k as u16), None);
    }
    let half = hf_sim::time::TICKS_PER_HOUR / 2;
    for _ in 0..(2 * hf_sim::time::TICKS_PER_HOUR + half) {
        a.step();
    }
    let mut b = hf_sim::save::decode(
        &hf_sim::save::encode(&hf_sim::save::snapshot::Snapshot::capture(&a)).unwrap(),
    )
    .unwrap()
    .restore();
    assert_eq!(hf_sim::digest::digest(&a), hf_sim::digest::digest(&b));
    for x in b.agents.iter().filter(|x| x.people.0 >= GENERATED) {
        assert!(
            x.kind.is_some(),
            "a person of a generated kind keeps it across a load"
        );
    }
    for _ in 0..(2 * hf_sim::time::TICKS_PER_HOUR) {
        a.step();
        b.step();
    }
    assert_eq!(hf_sim::digest::digest(&a), hf_sim::digest::digest(&b));
}
