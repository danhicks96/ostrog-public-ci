//! P0 gates (docs/design-peoples.md section 10): carvings, brewing, workshops and journeys are
//! done because someone wants them, and the record says who and why. Without such a person
//! they are not done, however full the store.

use hf_sim::events::{EventKind, Severity};
use hf_sim::expedition::Purpose;
use hf_sim::ids::{EventId, TilePos};
use hf_sim::items::{ItemKind as K, ItemPlace};
use hf_sim::mind::beliefs::{Source, Topic};
use hf_sim::mind::Skill;
use hf_sim::plants::{Plant, PlantKind};
use hf_sim::project::ProjectKind;
use hf_sim::structures::StructureKind;
use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use hf_sim::work::{recipes, DesignationKind};
use hf_sim::world::gen::GenParams;
use hf_sim::world::Material as M;
use hf_sim::world::Tile;
use hf_sim::World;

fn village(seed: u64, people: usize) -> World {
    let mut w = World::generate(seed, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.plants.clear();
    for i in 0..100 {
        w.plants.push(Plant::new(
            TilePos::new(4 + (i * 7) % 56, 4 + (i * 11) % 56, 2),
            PlantKind::BerryBush,
            1.0,
            i as u32,
        ));
    }
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.meta.tick = TICKS_PER_HOUR * 8;
    w.build_house(36, 24, 7, 6, 1, M::STONE_BLOCK, M::STONE_BLOCK);
    // Water by home (#433): a well, so nobody here wants one.
    w.place_structure(
        StructureKind::Well,
        TilePos::new(32, 33, 2),
        M::STONE_BLOCK,
        true,
    );
    w.rebuild_caches();
    w.found_party(people);
    w.meta.autoplan = true;
    w.communities[0].house_sites.push(TilePos::new(36, 24, 2));
    w.ensure_nav();
    w
}

/// The day's proposals, at their hour.
fn propose(w: &mut World) {
    let day = w.meta.tick / TICKS_PER_DAY + 1;
    w.meta.tick = day * TICKS_PER_DAY + hf_sim::systems::projects::AT;
    hf_sim::systems::projects::step(w);
}

/// A few hours of the planner.
fn plan(w: &mut World) {
    for _ in 0..3 {
        w.meta.tick = (w.meta.tick / TICKS_PER_HOUR + 1) * TICKS_PER_HOUR;
        hf_sim::systems::settlement::step(w);
    }
}

fn members(w: &World) -> Vec<usize> {
    (0..w.agents.len())
        .filter(|i| w.agents[*i].community.is_some())
        .collect()
}

fn train(w: &mut World, i: usize, skill: Skill) {
    for _ in 0..400 {
        w.agents[i].skills.practise(skill, 1.0);
    }
}

fn lean_all(w: &mut World, t: Topic, lean: f32) {
    for a in w.agents.iter_mut() {
        a.beliefs.hold(t, lean, 0.9, Source::Event(EventId(0)), 0);
    }
}

#[test]
fn a_carving_is_proposed_by_one_who_remembers() {
    // Nobody who builds well: no carving, however bare the walls.
    let mut w = village(3801, 6);
    let ids: Vec<_> = members(&w).iter().map(|i| w.agents[*i].id).collect();
    let tick = w.meta.tick;
    let paired = w.events.push(
        tick,
        Some(TilePos::new(30, 30, 2)),
        Severity::Major,
        EventKind::Paired {
            a: ids[0],
            b: ids[1],
        },
    );
    hf_sim::systems::flush::step(&mut w);
    hf_sim::systems::culture::observe(&mut w);
    for i in members(&w) {
        w.agents[i].skills = Default::default();
    }
    let mut none = w.clone();
    propose(&mut none);
    plan(&mut none);
    assert!(none.projects.is_empty(), "{:?}", none.projects);
    assert!(!none
        .designations
        .iter()
        .any(|d| d.kind == DesignationKind::Engrave));
    // A carver: they propose it, for the pairing, and the planner finds the wall.
    let ci = members(&w)[2];
    train(&mut w, ci, Skill::Construction);
    propose(&mut w);
    let p = w
        .projects
        .iter()
        .find(|p| p.kind == ProjectKind::Carve)
        .expect("a carving proposed")
        .clone();
    assert_eq!(p.by, w.agents[ci].id);
    assert_eq!(p.source, Some(paired), "{}", p.because);
    assert!(p.because.starts_with("remembers it: "), "{}", p.because);
    plan(&mut w);
    let d = w
        .designations
        .iter()
        .find(|d| d.kind == DesignationKind::Engrave)
        .expect("the wall is marked for carving");
    let note = hf_sim::project::note(&w, d.pos).expect("the order says whose wish it is");
    assert!(note.contains("wish: remembers it"), "{note}");
    assert_eq!(
        w.projects.iter().find(|x| x.id == p.id).unwrap().work,
        Some(d.id)
    );
    hf_sim::systems::flush::step(&mut w);
    assert!(w
        .last_events
        .iter()
        .any(|e| matches!(&e.kind, EventKind::Proposed { agent, .. } if agent == &p.by)));
}

fn full_store(w: &mut World) {
    let home = w.communities[0].home;
    w.create_item(
        K::GrainSheaf,
        12,
        ItemPlace::Ground(home.offset(1, 1, 0)),
        0.6,
        "test",
    );
    for k in 0..32 {
        w.create_item(
            K::Root,
            25,
            ItemPlace::Ground(home.offset(k % 8 - 4, 3 + k / 8, 0)),
            0.6,
            "test",
        );
    }
}

#[test]
fn nobody_who_cares_for_drink_nobody_brews() {
    // Neutral on drink, a full store: nobody proposes it (the old rule brewed for anyone not
    // set against drink).
    let mut w = village(3802, 6);
    lean_all(&mut w, Topic::Drink, 0.0);
    full_store(&mut w);
    let mut neutral = w.clone();
    propose(&mut neutral);
    assert!(!neutral.projects.iter().any(|p| p.kind == ProjectKind::Brew));
    // One who holds that drink eases things proposes it, with the belief's source.
    let bi = members(&w)[3];
    w.agents[bi].beliefs = Default::default();
    w.agents[bi]
        .beliefs
        .hold(Topic::Drink, 0.8, 0.9, Source::Event(EventId(7)), 0);
    propose(&mut w);
    let p = w
        .projects
        .iter()
        .find(|p| p.kind == ProjectKind::Brew)
        .expect("brewing proposed");
    assert_eq!(p.by, w.agents[bi].id);
    assert_eq!(p.source, Some(EventId(7)));
    assert!(p.because.contains("drink"), "{}", p.because);
}

#[test]
fn a_workshop_is_wanted_by_its_maker() {
    let mut w = village(3803, 10);
    let home = w.communities[0].home;
    w.create_item(
        K::IronOre,
        6,
        ItemPlace::Ground(home.offset(2, 2, 0)),
        0.6,
        "test",
    );
    let mut nobody = w.clone();
    for i in members(&nobody) {
        nobody.agents[i].skills = Default::default();
    }
    propose(&mut nobody);
    assert!(
        !nobody
            .projects
            .iter()
            .any(|p| p.kind == ProjectKind::Workshop(StructureKind::Smelter)),
        "ore at hand and nobody to smelt it: no smelter"
    );
    let si = members(&w)[4];
    train(&mut w, si, Skill::Crafting);
    propose(&mut w);
    let p = w
        .projects
        .iter()
        .find(|p| p.kind == ProjectKind::Workshop(StructureKind::Smelter))
        .expect("a smelter proposed")
        .clone();
    plan(&mut w);
    let work = w.projects.iter().find(|x| x.id == p.id).unwrap().work;
    assert!(w.designations.iter().any(|d| Some(d.id) == work
        && matches!(
            d.kind,
            DesignationKind::BuildStructure(StructureKind::Smelter, _)
        )));
    let _ = recipes::ALE;
}

#[test]
fn a_journey_has_a_proposer() {
    // Everyone afraid of strangers: a surplus and a known settlement, and nobody goes.
    let mut w = village(3804, 14);
    let home = w.communities[0].home;
    let cid = w.communities[0].id;
    w.drop_item(K::Log, 60, home.offset(3, 0, 0));
    w.drop_item(K::Meal, 60, home.offset(3, 1, 0));
    for _ in 0..2 {
        hf_sim::trade::daily(&mut w);
    }
    let k = w.overworld.sites.iter().position(|s| s.people > 0).unwrap();
    w.overworld.sites[k].known_by.push(cid);
    let mut afraid = w.clone();
    lean_all(&mut afraid, Topic::Strangers, 0.8);
    for _ in 0..6 {
        propose(&mut afraid);
    }
    assert!(afraid.overworld.caravans.is_empty(), "nobody would go");
    lean_all(&mut w, Topic::Strangers, -0.3);
    for _ in 0..6 {
        propose(&mut w);
        if !w.overworld.caravans.is_empty() {
            break;
        }
    }
    let c = w.overworld.caravans.first().expect("a party forms").clone();
    let p = w
        .projects
        .iter()
        .find(|p| matches!(p.kind, ProjectKind::Journey(_)))
        .expect("a journey proposed");
    assert!(c.members.contains(&p.by), "the proposer leads the party");
    assert!(matches!(c.purpose, Purpose::Trade { .. }));
    assert!(p.because.contains("will pay"), "{}", p.because);
    // Every journey proposed went, led by whoever proposed it (the review of C38-C44: a
    // proposal used to be recorded even when no party could form).
    for _ in 0..12 {
        propose(&mut w);
    }
    for p in w
        .projects
        .iter()
        .filter(|p| matches!(p.kind, ProjectKind::Journey(_)))
    {
        assert!(
            w.overworld
                .caravans
                .iter()
                .any(|c| c.formed == p.proposed && c.members.contains(&p.by)),
            "a journey proposed on {} with no party led by its proposer",
            p.proposed
        );
    }
}

fn raiders_camped(w: &mut World, taken: u32) {
    w.bands.push(hf_sim::threat::Band {
        id: 0,
        site: 0,
        site_name: "Ashby".into(),
        members: Vec::new(),
        size: 3,
        camp: TilePos::new(50, 30, 2),
        state: hf_sim::threat::BandState::Camped(w.meta.tick),
        formed: w.meta.tick,
        want: 18,
        taken,
        lost: 0,
        sighted: true,
        welcomed: false,
        pursuits: Vec::new(),
        raiders: Vec::new(),
        envoys: Vec::new(),
        eaten: 0,
        night: 0,
        night_taken: 0,
        remembered: false,
        seen: Vec::new(),
        heard: Vec::new(),
        host: false,
        captives: Vec::new(),
    });
}

#[test]
fn a_raid_makes_someone_who_holds_with_force_want_spears() {
    // C52: with strangers camped and taking from the store, someone who holds that force
    // answers force proposes spears, and the bench work is posted; nobody of the kind, nothing.
    for (lean, wants) in [(0.6f32, true), (-0.8, false)] {
        let mut w = village(5201, 6);
        w.place_structure(
            StructureKind::Workbench,
            TilePos::new(38, 26, 2),
            M::WOOD,
            true,
        );
        w.drop_item(K::Log, 6, TilePos::new(33, 30, 2));
        lean_all(&mut w, Topic::Violence, lean);
        lean_all(&mut w, Topic::Strangers, lean);
        assert!(
            !w.items.iter().any(|it| it.kind == K::Weapon),
            "no spear before"
        );
        // No raid yet: nothing.
        propose(&mut w);
        assert!(!w.projects.iter().any(|p| p.kind == ProjectKind::Arms));
        raiders_camped(&mut w, 0);
        propose(&mut w);
        assert!(
            !w.projects.iter().any(|p| p.kind == ProjectKind::Arms),
            "strangers camped who have taken nothing are not a raid"
        );
        w.bands[0].taken = 5;
        propose(&mut w);
        let p = w.projects.iter().find(|p| p.kind == ProjectKind::Arms);
        assert_eq!(p.is_some(), wants, "lean {lean}");
        if let Some(p) = p {
            assert!(p.because.contains("Ashby"), "{}", p.because);
            let spears = w
                .designations
                .iter()
                .filter(|d| d.kind == DesignationKind::Craft(recipes::WOOD_SPEAR) && !d.done)
                .count();
            assert_eq!(spears, 2);
            // Made at the bench within a few days, of the wood it was cut from.
            for _ in 0..TICKS_PER_DAY * 3 {
                w.step();
                if w.items.iter().any(|it| it.kind == K::Weapon) {
                    break;
                }
            }
            let spear = w.items.iter().find(|it| it.kind == K::Weapon);
            assert!(spear.is_some(), "a spear was made");
            assert_eq!(spear.unwrap().material, M::OAK);
        }
    }
}

#[test]
fn a_fetch_names_what_was_lacking() {
    // C59: no rock face within reach, the hearth still wants stone, and a known settlement has
    // blocks to spare: someone goes to fetch them, carrying the logs the valley has too many of.
    let mut w = village(5901, 14);
    let home = w.communities[0].home;
    let cid = w.communities[0].id;
    let blocks: Vec<_> = w
        .items
        .iter()
        .filter(|i| i.kind == K::StoneBlock)
        .map(|i| (i.id, i.count))
        .collect();
    for (id, n) in blocks {
        w.consume_item(id, n, "test");
    }
    w.structures.retain(|s| s.kind != StructureKind::Hearth);
    w.drop_item(K::Log, 60, home.offset(3, 0, 0));
    w.drop_item(K::Meal, 60, home.offset(3, 1, 0));
    for _ in 0..2 {
        hf_sim::trade::daily(&mut w);
    }
    let k = w.overworld.sites.iter().position(|s| s.people > 0).unwrap();
    w.overworld.sites[k].known_by.push(cid);
    w.overworld.sites[k].stores.insert(K::StoneBlock, 40.0);
    lean_all(&mut w, Topic::Strangers, -0.3);
    for _ in 0..6 {
        propose(&mut w);
        if !w.overworld.caravans.is_empty() {
            break;
        }
    }
    let c = w.overworld.caravans.first().expect("a party forms").clone();
    assert!(
        matches!(
            c.purpose,
            Purpose::Fetch {
                want: K::StoneBlock,
                ..
            }
        ),
        "{:?}",
        c.purpose
    );
    let p = w
        .projects
        .iter()
        .find(|p| matches!(p.kind, ProjectKind::Journey(Purpose::Fetch { .. })))
        .expect("a fetch proposed");
    assert!(p.because.contains("no rock face"), "{}", p.because);
    assert!(c.members.contains(&p.by));
}

/// As `a_fetch_names_what_was_lacking`, up to the settlement: a valley short of stone with no
/// face in reach, logs and meals to spare, and the index of a peopled settlement it knows.
fn short_of_stone(seed: u64) -> (World, hf_sim::ids::CommunityId, usize) {
    let mut w = village(seed, 14);
    let home = w.communities[0].home;
    let cid = w.communities[0].id;
    let blocks: Vec<_> = w
        .items
        .iter()
        .filter(|i| i.kind == K::StoneBlock)
        .map(|i| (i.id, i.count))
        .collect();
    for (id, n) in blocks {
        w.consume_item(id, n, "test");
    }
    w.structures.retain(|s| s.kind != StructureKind::Hearth);
    w.drop_item(K::Log, 60, home.offset(3, 0, 0));
    w.drop_item(K::Meal, 60, home.offset(3, 1, 0));
    for _ in 0..2 {
        hf_sim::trade::daily(&mut w);
    }
    let k = w.overworld.sites.iter().position(|s| s.people > 0).unwrap();
    w.overworld.sites[k].known_by.push(cid);
    (w, cid, k)
}

#[test]
fn a_fetch_is_never_to_an_empty_site() {
    // #164: the fetch site was any known settlement holding eight, even one with nobody left
    // to trade, where the barter pays nothing.
    let (mut w, cid, k) = short_of_stone(16401);
    w.overworld.sites[k].stores.insert(K::StoneBlock, 40.0);
    w.overworld.sites[k].people = 0;
    // The others it knows have people, and no stone.
    let mut others = 0;
    for (j, s) in w.overworld.sites.iter_mut().enumerate() {
        if j != k && s.people > 0 {
            s.stores.insert(K::StoneBlock, 0.0);
            if !s.known_by.contains(&cid) {
                s.known_by.push(cid);
            }
            others += 1;
        }
    }
    assert!(others > 0);
    lean_all(&mut w, Topic::Strangers, -0.3);
    for _ in 0..6 {
        propose(&mut w);
    }
    assert!(
        !w.projects
            .iter()
            .any(|p| matches!(p.kind, ProjectKind::Journey(Purpose::Fetch { .. }))),
        "no journey to a place with nobody in it"
    );
}

#[test]
fn spare_means_what_barter_will_pay() {
    // #164: "to spare" was eight in store, but barter pays only from above half the need: a
    // settlement with twenty blocks and a need of forty was chosen and paid none.
    let (mut w, cid, k) = short_of_stone(16402);
    let need = hf_sim::trade::need(&w, &w.overworld.sites[k], K::StoneBlock);
    w.overworld.sites[k]
        .stores
        .insert(K::StoneBlock, (need * 0.5 + 2.0).max(8.0));
    assert!(
        need * 0.5 + 2.0 >= 8.0,
        "the site's need is high enough to test: {need}"
    );
    let home = w.communities[0].home;
    assert_eq!(
        hf_sim::trade::best_fetch(&w, cid, home, 14, K::StoneBlock),
        None,
        "two to spare is not a fetch"
    );
}

#[test]
fn fetch_cargo_is_what_the_site_will_take() {
    // #164: the cargo was the best errand for another settlement; the fetch site, full of it,
    // took none and paid nothing.
    let (mut w, cid, k) = short_of_stone(16403);
    w.overworld.sites[k].stores.insert(K::StoneBlock, 60.0);
    let home = w.communities[0].home;
    let (site, carry) =
        hf_sim::trade::best_fetch(&w, cid, home, 14, K::StoneBlock).expect("a fetch");
    let s = w.overworld.site(site).unwrap().clone();
    let (took, got) = hf_sim::trade::quote_wanting(
        &w,
        &s,
        home,
        14,
        carry,
        hf_sim::trade::carried_material(&w, home, carry),
        12,
        30,
        Some(K::StoneBlock),
    );
    assert!(took > 0, "the settlement takes the {carry:?}");
    assert!(
        got.iter().any(|g| g.0 == K::StoneBlock && g.2 >= 4),
        "and pays in blocks: {got:?}"
    );
    // Glut the settlement with that cargo: another must be chosen, or none.
    let need = hf_sim::trade::need(&w, &s, carry);
    w.overworld.sites[k].stores.insert(carry, need * 2.0 + 10.0);
    if let Some((site2, carry2)) = hf_sim::trade::best_fetch(&w, cid, home, 14, K::StoneBlock) {
        assert!(
            site2 != site || carry2 != carry,
            "not the cargo it has too much of"
        );
    }
}

#[test]
fn trees_out_of_chop_reach_are_no_wood_in_reach() {
    // #164: a tree anywhere within 60 counted as wood in reach, while the planner fells only
    // trees within 40 that someone can reach: with the only tree walled in, no felling and no
    // fetch either.
    let mut w = village(16404, 14);
    let home = w.communities[0].home;
    let cid = w.communities[0].id;
    let logs: Vec<_> = w
        .items
        .iter()
        .filter(|i| i.kind == K::Log)
        .map(|i| (i.id, i.count))
        .collect();
    for (id, n) in logs {
        w.consume_item(id, n, "test");
    }
    w.plants.retain(|p| !p.kind.yields_wood());
    // The one tree is walled in where no one can stand beside it.
    let far = TilePos::new(home.x + 20, home.y + 20, home.z);
    for (dx, dy) in [(0, 1), (1, 0), (0, -1), (-1, 0)] {
        w.grid.set(far.offset(dx, dy, 0), Tile::solid(M::GRANITE));
    }
    w.rebuild_caches();
    w.ensure_nav();
    w.plants.push(Plant::new(far, PlantKind::Tree, 1.0, 7));
    assert!(
        hf_sim::systems::settlement::chop_candidates(&w, home).is_empty(),
        "the planner would fell nothing"
    );
    w.drop_item(K::StoneBlock, 60, home.offset(3, 0, 0));
    w.drop_item(K::Meal, 60, home.offset(3, 1, 0));
    for _ in 0..2 {
        hf_sim::trade::daily(&mut w);
    }
    let k = w.overworld.sites.iter().position(|s| s.people > 0).unwrap();
    w.overworld.sites[k].known_by.push(cid);
    w.overworld.sites[k].stores.insert(K::Log, 80.0);
    lean_all(&mut w, Topic::Strangers, -0.3);
    for _ in 0..6 {
        propose(&mut w);
        if !w.overworld.caravans.is_empty() {
            break;
        }
    }
    assert!(
        w.projects.iter().any(|p| matches!(
            p.kind,
            ProjectKind::Journey(Purpose::Fetch { want: K::Log, .. })
        )),
        "a fetch for timber: {:?}",
        w.projects.iter().map(|p| &p.kind).collect::<Vec<_>>()
    );
}
