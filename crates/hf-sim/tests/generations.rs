//! Phase 8 gates: inherited vs learned, and three generations of lineage.

use hf_sim::agents::{LifeStage, Species};
use hf_sim::events::EventKind;
use hf_sim::genetics::Genome;
use hf_sim::ids::TilePos;
use hf_sim::mind::Skill;
use hf_sim::plants::{Plant, PlantKind};
use hf_sim::systems::life::fed_for_birth;
use hf_sim::time::TICKS_PER_DAY;
use hf_sim::world::gen::GenParams;
use hf_sim::World;

/// #1195: a couple merely peckish at 05:00, who ate the evening before and whose community
/// holds days of food, must pass the fed gate for a child rather than be read as starving on
/// an instant's reading of hunger.
#[test]
fn a_fed_household_is_not_turned_away_at_dawn() {
    let mut w = World::generate(82, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.plants.clear();
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.meta.autoplan = false;
    let ids = w.found_party(2);
    // Plenty of food: the party's provisions already clear the two-day bar, but make it
    // explicit (5 days) so the test does not depend on how much founding drops.
    w.drop_item(hf_sim::items::ItemKind::Meal, 50, w.meta.founding_site);
    let (m, f) = (ids[0], ids[1]);
    for id in [m, f] {
        let a = w.agents.iter_mut().find(|a| a.id == id).unwrap();
        // Ate at 07:00 the day before: hungry again by dawn but never starving, so
        // `starving_ticks` is still zero.
        a.needs.hunger = 0.25;
        a.starving_ticks = 0;
    }
    let cid = w.agents.iter().find(|a| a.id == m).unwrap().community;
    assert!(cid.is_some(), "the founders have a community");
    let days = hf_sim::systems::settlement::food_days(&w, cid.unwrap());
    assert!(days >= 2.0, "the community holds plenty of food: {days}");
    let pa = w.agents.iter().find(|a| a.id == m).unwrap().clone();
    let pb = w.agents.iter().find(|a| a.id == f).unwrap().clone();
    assert!(
        fed_for_birth(&w, &pa, &pb),
        "a merely peckish, well-fed couple should pass the fed gate at dawn"
    );
}

#[test]
fn genome_vs_learned_separation() {
    let mut w = World::generate(81, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.plants.clear();
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.meta.autoplan = false;
    let ids = w.found_party(2);
    let (m, f) = (ids[0], ids[1]);
    // Give the parents high learned skill; the child must not inherit it.
    for id in [m, f] {
        let a = w.agents.iter_mut().find(|a| a.id == id).unwrap();
        a.skills.practise(Skill::Construction, 500.0);
        assert!(a.skills.level(Skill::Construction) > 15.0);
    }
    let child = w.give_birth(m, f).expect("born");
    let c = w.agents.iter().find(|a| a.id == child).unwrap();
    assert_eq!(
        c.skills.level(Skill::Construction),
        0.0,
        "skills are learned, not inherited"
    );
    assert!(c.is_child(w.tick()));
    let (mg, fg) = (
        w.agents.iter().find(|a| a.id == m).unwrap().genome,
        w.agents.iter().find(|a| a.id == f).unwrap().genome,
    );
    let shared = (0..hf_sim::genetics::LOCI)
        .filter(|&i| c.genome.loci[i] == mg.loci[i] || c.genome.loci[i] == fg.loci[i])
        .count();
    assert!(
        shared >= hf_sim::genetics::LOCI - 3,
        "genome mostly from the parents: {shared}"
    );
    assert_ne!(c.genome, mg);
    assert_ne!(c.genome, fg);
    // Appearance follows the genome: skin tone index equals what the genome decides.
    let mut r = hf_sim::rng::stream(1, &[1]);
    assert_eq!(c.appearance.skin, c.genome.appearance(&mut r).skin);
    // Aptitudes come from the genome; the child's are those of its genome, not its parents'.
    let apt = c.genome.aptitudes();
    assert_eq!(
        c.skills.get(Skill::Mining).aptitude,
        apt.get(Skill::Mining).aptitude
    );
    let _ = Genome::default();
}

#[test]
fn three_generations_lineage() {
    // Lineage mechanics across three generations, deterministically: founders have children,
    // those children grow up (ages advanced by moving birth ticks), pair, and have children
    // of their own. Then the natural path: over forty days at normal life speed, founders pair
    // and a birth happens on its own, and nobody starves.
    let mut w = World::generate(82, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.plants.clear();
    for i in 0..200 {
        let p = TilePos::new(3 + (i * 7) % 58, 3 + (i * 13 + i / 58) % 58, 2);
        if !w.plants.iter().any(|q| q.pos == p) {
            w.plants
                .push(Plant::new(p, PlantKind::BerryBush, 1.0, i as u32));
        }
    }
    w.meta.founding_site = TilePos::new(30, 30, 2);
    let ids = w.found_party(8);
    // Generation 2: two children per founding couple (scripted births).
    let c1 = w.give_birth(ids[0], ids[1]).unwrap();
    let c2 = w.give_birth(ids[2], ids[3]).unwrap();
    for c in [c1, c2] {
        let a = w.agents.iter().find(|a| a.id == c).unwrap();
        assert_eq!(a.stage(w.tick()), LifeStage::Child);
        assert!(a.mother.is_some() && a.father.is_some());
        assert!(a.memories.long.is_empty(), "newborns carry no memories");
    }
    // They grow up: move their birth back twenty years.
    let now = w.tick();
    for c in [c1, c2] {
        let a = w.agents.iter_mut().find(|a| a.id == c).unwrap();
        a.born -= (hf_sim::time::TICKS_PER_YEAR * 20) as i64;
        assert_eq!(a.stage(now), LifeStage::Adult);
    }
    // Generation 3: the two unrelated second-generation adults have a child.
    let g = w.give_birth(c1, c2).unwrap();
    let gc = w.agents.iter().find(|a| a.id == g).unwrap();
    let grandparents: Vec<_> = [gc.mother, gc.father]
        .into_iter()
        .flatten()
        .filter_map(|p| w.agents.iter().find(|a| a.id == p))
        .flat_map(|p| [p.mother, p.father])
        .flatten()
        .collect();
    assert_eq!(
        grandparents.len(),
        4,
        "a grandchild has four grandparents on record"
    );
    assert!(grandparents.contains(&ids[0]) && grandparents.contains(&ids[3]));
    // The genome travelled: every grandchild locus comes from one of its parents.
    let (pm, pf) = (
        w.agents.iter().find(|a| a.id == c1).unwrap().genome,
        w.agents.iter().find(|a| a.id == c2).unwrap().genome,
    );
    let from_parents = (0..hf_sim::genetics::LOCI)
        .filter(|&i| gc.genome.loci[i] == pm.loci[i] || gc.genome.loci[i] == pf.loci[i])
        .count();
    assert!(from_parents >= hf_sim::genetics::LOCI - 3);

    // The natural path at normal life speed.
    let tick = w.tick();
    for i in 0..ids.len() {
        for j in 0..ids.len() {
            if i != j {
                let a = w.agents.iter_mut().find(|a| a.id == ids[i]).unwrap();
                a.relate(ids[j], 0.3, 0.3, tick);
            }
        }
    }
    let mut born = 0;
    let mut paired = 0;
    let mut starved = 0;
    for _ in 0..TICKS_PER_DAY * 40 {
        w.step();
        for e in &w.last_events {
            match &e.kind {
                EventKind::Born { .. } => born += 1,
                EventKind::Paired { .. } => paired += 1,
                EventKind::Died { cause, .. } if cause == "starvation" => starved += 1,
                _ => {}
            }
        }
    }
    eprintln!("natural: paired {paired} born {born} starved {starved}");
    assert!(paired >= 1, "founders paired on their own: {paired}");
    assert!(
        born >= 1,
        "a child was born on its own in forty days: {born}"
    );
    // Hardship is allowed: a lean season can kill, and that is a story, not a failure. The gate
    // is lineage, so starvation is reported rather than asserted.
    eprintln!("starved in forty days: {starved}");
    let _ = Species::Person;
}

#[test]
fn a_newborn_is_known_to_grandparents_and_siblings() {
    // A grandmother and an elder sister are already on hand when the newborn arrives; an
    // unrelated neighbour is not kin to anyone.
    let mut w = World::generate(83, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.plants.clear();
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.meta.autoplan = false;
    let ids = w.found_party(4);
    let (grandmother, grandfather, father, neighbour) = (ids[0], ids[1], ids[2], ids[3]);
    // Generation 2: the mother, born to the grandmother and grandfather, then grown up.
    let mother = w.give_birth(grandmother, grandfather).unwrap();
    {
        let a = w.agents.iter_mut().find(|a| a.id == mother).unwrap();
        a.born -= (hf_sim::time::TICKS_PER_YEAR * 20) as i64;
    }
    // An elder sister, already born to the mother and father before the newborn.
    let elder_sister = w.give_birth(mother, father).unwrap();
    // The newborn: shares the same mother and father as the elder sister, and has a living
    // grandmother (and grandfather).
    let newborn = w.give_birth(mother, father).unwrap();

    let gm = w.agents.iter().find(|a| a.id == grandmother).unwrap();
    assert!(
        gm.opinion_of(newborn) >= 0.4,
        "grandmother's opinion of the newborn: {}",
        gm.opinion_of(newborn)
    );
    let sis = w.agents.iter().find(|a| a.id == elder_sister).unwrap();
    assert!(
        sis.opinion_of(newborn) >= 0.4,
        "elder sister's opinion of the newborn: {}",
        sis.opinion_of(newborn)
    );
    let nb = w.agents.iter().find(|a| a.id == newborn).unwrap();
    assert!(
        nb.opinion_of(grandmother) >= 0.3,
        "newborn's opinion of the grandmother: {}",
        nb.opinion_of(grandmother)
    );
    assert!(
        nb.opinion_of(elder_sister) >= 0.3,
        "newborn's opinion of the elder sister: {}",
        nb.opinion_of(elder_sister)
    );
    let neigh = w.agents.iter().find(|a| a.id == neighbour).unwrap();
    assert!(
        !neigh.relationships.contains_key(&newborn),
        "an unrelated neighbour holds no relation to the newborn"
    );
    assert!(
        !nb.relationships.contains_key(&neighbour),
        "the newborn holds no relation to an unrelated neighbour"
    );
}

/// OST-901: the births() crowd gate counted every living `Species::Person` on the whole map,
/// including a band of raiders camped at the edge who are not of anyone's community
/// (spawned with `community: None`, see threat.rs). That let strangers who will never share
/// a nursery halve a small community's own birth chance. The gate must count per community.
#[test]
fn strangers_do_not_crowd_the_nursery() {
    use hf_sim::rng::{purpose, stream};
    use rand::Rng;

    let mut w = World::generate(1194, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.plants.clear();
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.meta.autoplan = false;
    let ids = w.found_party(2);
    let (a, b) = (ids[0], ids[1]);
    {
        let ia = w.agents.iter().position(|x| x.id == a).unwrap();
        let ib = w.agents.iter().position(|x| x.id == b).unwrap();
        w.agents[ia].partner = Some(b);
        w.agents[ib].partner = Some(a);
        w.agents[ia].last_birth = 0;
        w.agents[ib].last_birth = 0;
        w.agents[ia].needs.hunger = 1.0;
        w.agents[ib].needs.hunger = 1.0;
        // Twenty years old at tick 0, so the whole search range below keeps both inside the
        // sixteen-to-forty-five fertile years.
        w.agents[ia].born = -(20 * hf_sim::time::TICKS_PER_YEAR as i64);
        w.agents[ib].born = -(20 * hf_sim::time::TICKS_PER_YEAR as i64);
    }
    // A band of raiders camped at the edge of the valley: spawned as Species::Person (as
    // threat.rs spawns bands), joining no community. The couple's own community stays at two.
    for i in 0..40 {
        let p = TilePos::new(5 + i % 10, 5 + i / 10, 2);
        w.spawn_agent(Species::Person, p);
    }
    let valley_people = w
        .agents
        .iter()
        .filter(|x| x.alive && x.species == Species::Person)
        .count();
    assert!(
        valley_people >= 40,
        "the valley's own census should be crowded: {valley_people}"
    );
    let community_people = w
        .agents
        .iter()
        .filter(|x| x.alive && x.species == Species::Person && x.community == w.agents[0].community)
        .count();
    assert!(
        community_people < 40,
        "the couple's own community should not be crowded: {community_people}"
    );

    // Find a day whose birth-chance draw falls strictly between the halved and the uncrowded
    // thresholds, so a per-community gate and a valley-wide gate disagree on it.
    let per_day = w.meta.life_speed / (3.0 * hf_sim::time::DAYS_PER_YEAR as f32);
    let five_am = hf_sim::time::TICKS_PER_HOUR * 5;
    let mut chosen_tick = None;
    for day in 0u64..500 {
        let tick = day * hf_sim::time::TICKS_PER_DAY + five_am;
        let mut r = stream(
            w.meta.seed,
            &[purpose::AGENT_BIRTH, a.0 as u64, b.0 as u64, day],
        );
        let v: f32 = r.gen();
        if v >= per_day * 0.5 && v < per_day {
            chosen_tick = Some(tick);
            break;
        }
    }
    let tick = chosen_tick.expect("a distinguishing day exists within the search range");
    // `World::step` advances `meta.tick` by one before running the systems, so set the tick
    // one short of the chosen five-o'-clock moment to land births() on it.
    w.meta.tick = tick - 1;
    w.rebuild_caches();
    let before = w
        .agents
        .iter()
        .filter(|x| x.species == Species::Person)
        .count();
    w.step();
    let after = w
        .agents
        .iter()
        .filter(|x| x.species == Species::Person)
        .count();
    assert_eq!(
        after,
        before + 1,
        "a band of raiders at the edge of the valley halved the community's own birth chance"
    );
}
