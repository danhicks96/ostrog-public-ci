//! #453 (epic #477, part 3): foul water sickens — a drink from a fouled tile can bring the
//! flux. The roll is in the actual drinking action (`water::drink`): the source tile's foul
//! map entry is checked first (a salt or brackish tile can be fouled too; salt's own effects
//! are part 4). Wells and barrel stores stay fresh by definition: they are no tile of open
//! water, so `drink` finds no `source` and never rolls.

use hf_sim::body::IllnessKind;
use hf_sim::floods;
use hf_sim::ids::TilePos;
use hf_sim::items::{Item, ItemKind, ItemPlace};
use hf_sim::systems::water_quality::{self, FOUL_HOURS};
use hf_sim::time::TICKS_PER_HOUR;
use hf_sim::world::gen::GenParams;
use hf_sim::world::{Material, Shape, Tile, TileFlags};
use hf_sim::World;

fn water_tile() -> Tile {
    Tile {
        material: Material::WATER,
        shape: Shape::Floor,
        flags: TileFlags(TileFlags::WATER),
    }
}

/// The part 2 fouled-pond fixture: flat ground, a pond, meat well rotten one tile east of its
/// shore, and one hour of the fouling system. Returns the world and a fouled tile at the
/// pond's edge.
fn fouled_pond(seed: u64) -> (World, TilePos) {
    let mut w = World::generate(seed, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.plants.clear();
    w.meta.founding_site = TilePos::new(40, 40, 2);
    for y in 20..=24 {
        for x in 20..=24 {
            w.grid.set(TilePos::new(x, y, 2), water_tile());
        }
    }
    w.rebuild_caches();
    floods::survey(&mut w);
    let meat_p = TilePos::new(25, 21, 2);
    let meat_id = w.alloc_item_id();
    w.items.push(Item {
        id: meat_id,
        kind: ItemKind::RawMeat,
        count: 1,
        place: ItemPlace::Ground(meat_p),
        quality: 0.5,
        material: Material::MEAT,
        story: None,
        rot: 0.9,
    });
    w.meta.tick = 9 * TICKS_PER_HOUR + 300;
    water_quality::step(&mut w);
    // The pond tile beside the meat, on the shore where a drinker stands.
    let fouled = TilePos::new(24, 22, 2);
    assert_eq!(
        w.waters.foul.get(&fouled),
        Some(&FOUL_HOURS),
        "the fixture fouls the pond tile beside the meat"
    );
    (w, fouled)
}

/// A founder standing on the shore tile whose first open-water neighbour east is `fouled`.
fn drinker_beside(w: &mut World, fouled: TilePos) -> usize {
    w.weather.temperature_c = 10.0;
    let ids = w.found_party(1);
    let i = w
        .agents
        .iter()
        .position(|a| a.id == ids[0])
        .expect("the founder");
    w.agents[i].pos = fouled.offset(1, 0, 0);
    i
}

#[test]
fn a_drink_from_fouled_water_brings_the_flux() {
    // The seed is pinned so the drink roll lands under the chance (0.2, Sickness lever
    // aside); with the part 3 change the drink brings the flux. If an unrelated rng change
    // shifts this particular stream, pick another seed that pins a catching roll.
    let (mut w, fouled) = fouled_pond(4543);
    let i = drinker_beside(&mut w, fouled);
    assert!(
        hf_sim::water::drink(&mut w, i),
        "they drank from the fouled pond"
    );
    assert_eq!(
        w.agents[i].body.illness.map(|x| x.kind),
        Some(IllnessKind::Flux),
        "a foul drink brings the flux"
    );
}

#[test]
fn a_well_sharing_a_stand_with_a_foul_pond_is_kept() {
    use hf_sim::mind::{Known, ObservationKind, OutcomeKind};
    use hf_sim::structures::StructureKind;
    let (mut w, fouled) = fouled_pond(4543);
    let i = drinker_beside(&mut w, fouled);
    let from = w.agents[i].pos;
    let tick = w.meta.tick;
    w.agents[i].needs.thirst = 0.45;
    w.agents[i]
        .knowledge
        .note(Known::Water(fouled), fouled, tick, floods::TAG_WATER_FOUL);
    w.agents[i].learning.learn(
        ObservationKind::FoulWater,
        OutcomeKind::Illness,
        Some(fouled),
        true,
        tick,
        None,
    );
    w.agents[i].learning.lessons[0].confidence = 0.6;
    w.place_structure(
        StructureKind::Well,
        from.offset(0, -1, 0),
        Material::STONE_BLOCK,
        true,
    );
    w.ensure_nav();
    let chosen = hf_sim::water::nearest_knowing(&w, i, from, 30, &w.agents[i].knowledge, false)
        .expect("a shared stand must keep its well when the pond is refused");
    assert!(chosen.well, "the reachable well is still a choice");
    assert_eq!(chosen.stand, from);
}

#[test]
fn a_well_does_not_test_the_foul_pond_beside_it() {
    use hf_sim::mind::{Known, ObservationKind, OutcomeKind};
    use hf_sim::structures::StructureKind;
    let run = |threads| {
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .unwrap()
            .install(|| {
                let mut digests = Vec::new();
                for finished in [false, true] {
                    let (mut w, original_foul) = fouled_pond(4543);
                    let i = drinker_beside(&mut w, original_foul);
                    let fouled = original_foul.offset(0, 2, 0);
                    w.waters.foul.insert(fouled, FOUL_HOURS);
                    let at = w.agents[i].pos;
                    let tick = w.meta.tick;
                    w.agents[i].needs.thirst = 0.45;
                    w.agents[i].knowledge.note(
                        Known::Water(fouled),
                        fouled,
                        tick,
                        floods::TAG_WATER_FOUL,
                    );
                    w.agents[i].learning.learn(
                        ObservationKind::FoulWater,
                        OutcomeKind::Illness,
                        Some(fouled),
                        true,
                        tick,
                        None,
                    );
                    w.agents[i].learning.lessons[0].confidence = 0.6;
                    w.place_structure(
                        StructureKind::Well,
                        at.offset(0, 1, 0),
                        Material::STONE_BLOCK,
                        true,
                    );
                    w.structures.last_mut().unwrap().progress = if finished { 1.0 } else { 0.5 };
                    w.ensure_nav();
                    if finished {
                        let chosen = hf_sim::water::nearest_knowing(
                            &w,
                            i,
                            at,
                            30,
                            &w.agents[i].knowledge,
                            false,
                        )
                        .expect("the well is within reach");
                        assert!(chosen.well, "they chose the well over the known foul pond");
                        w.agents[i].pos = chosen.stand;
                    }
                    assert!(hf_sim::water::drink(&mut w, i));
                    if finished {
                        assert!(
                            w.agents[i].body.illness.is_none(),
                            "the chosen well must supply its own water"
                        );
                        assert_eq!(w.agents[i].learning.lessons[0].confidence, 0.6);
                        assert_eq!(w.agents[i].learning.lessons[0].contradictions, 0);
                    } else {
                        assert_eq!(w.agents[i].body.illness.unwrap().kind, IllnessKind::Flux);
                    }
                    digests.push(hf_sim::digest::digest(&w));
                }
                digests
            })
    };
    assert_eq!(run(1), run(4));
}

#[test]
fn the_flux_teaches_the_drinker() {
    use hf_sim::mind::{Evidence, OutcomeKind};
    let (mut w, fouled) = fouled_pond(4543);
    let i = drinker_beside(&mut w, fouled);
    let bystander = w.found_party(1)[0];
    let j = w.agents.iter().position(|a| a.id == bystander).unwrap();
    w.agents[j].pos = w.agents[i].pos.offset(0, 1, 0);
    assert!(hf_sim::water::drink(&mut w, i));
    assert_eq!(w.agents[i].body.illness.unwrap().kind, IllnessKind::Flux);
    let lesson = w.agents[i]
        .learning
        .lessons
        .iter()
        .find(|lesson| lesson.outcome == OutcomeKind::Illness && lesson.pos == Some(fouled))
        .expect("the illness taught the drinker about the water they drank");
    assert_eq!(lesson.confidence, 0.35);
    assert_eq!(lesson.cue, hf_sim::mind::ObservationKind::FoulWater);
    let event = w
        .events
        .pending
        .iter()
        .find(|e| matches!(e.kind, hf_sim::events::EventKind::FellIll { agent, .. } if agent == w.agents[i].id))
        .expect("the actual illness event");
    assert_eq!(lesson.origins, vec![event.id]);
    let observation = w.agents[i]
        .learning
        .observations
        .iter()
        .find(|o| o.pos == fouled && o.event == Some(event.id))
        .expect("a direct observation at the water");
    assert_eq!(observation.evidence, Evidence::Direct);
    assert!(
        w.agents[j].learning.lessons.is_empty(),
        "knowledge stays personal"
    );
}

#[test]
fn a_safe_drink_weakens_an_existing_water_lesson() {
    let (mut w, fouled) = fouled_pond(4543);
    let i = drinker_beside(&mut w, fouled);
    assert!(hf_sim::water::drink(&mut w, i));
    assert_eq!(w.agents[i].learning.lessons.len(), 1);
    w.agents[i].body.illness = None;
    w.waters.foul.remove(&fouled);
    w.meta.tick += 1;
    assert!(hf_sim::water::drink(&mut w, i));
    let lesson = &w.agents[i].learning.lessons[0];
    assert!(lesson.confidence < 0.35);
    assert_eq!(lesson.contradictions, 1);
    assert!(w.agents[i].body.illness.is_none());
}

#[test]
fn no_illness_event_means_no_new_illness_lesson() {
    let (mut w, fouled) = fouled_pond(4543);
    let i = drinker_beside(&mut w, fouled);
    w.agents[i].immune.push(IllnessKind::Flux);
    assert!(hf_sim::water::drink(&mut w, i));
    assert!(w.agents[i].body.illness.is_none());
    assert!(w.agents[i].learning.lessons.is_empty());
    assert!(!w.events.pending.iter().any(|e| matches!(
        e.kind,
        hf_sim::events::EventKind::FellIll { agent, .. } if agent == w.agents[i].id
    )));
}

#[test]
fn the_water_lesson_and_its_evidence_survive_a_save() {
    use hf_sim::save::{decode, encode, snapshot::Snapshot};
    let run = |threads| {
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .unwrap()
            .install(|| {
                let (mut w, fouled) = fouled_pond(4543);
                let i = drinker_beside(&mut w, fouled);
                assert!(hf_sim::water::drink(&mut w, i));
                // Finish a normal tick before saving, including the real illness event.
                w.step();
                let mut back = decode(&encode(&Snapshot::capture(&w)).unwrap())
                    .unwrap()
                    .restore();
                assert_eq!(
                    w.agents[i].learning.lessons,
                    back.agents[i].learning.lessons
                );
                assert_eq!(
                    w.agents[i].learning.observations,
                    back.agents[i].learning.observations
                );
                assert_eq!(hf_sim::digest::digest(&w), hf_sim::digest::digest(&back));
                w.run(TICKS_PER_HOUR);
                back.run(TICKS_PER_HOUR);
                assert_eq!(hf_sim::digest::digest(&w), hf_sim::digest::digest(&back));
                hf_sim::digest::digest(&w)
            })
    };
    assert_eq!(run(1), run(4));
}

#[test]
fn a_drink_from_fresh_water_brings_nothing() {
    let (mut w, _fouled) = fouled_pond(4536);
    // Across the pond from the rot: the first open-water neighbour west of this shore is a
    // fresh tile (nothing foul reaches it).
    w.weather.temperature_c = 10.0;
    let ids = w.found_party(1);
    let i = w
        .agents
        .iter()
        .position(|a| a.id == ids[0])
        .expect("the founder");
    w.agents[i].pos = TilePos::new(19, 24, 2);
    let fresh_tile = TilePos::new(20, 24, 2);
    assert!(!w.waters.foul.contains_key(&fresh_tile));
    assert!(hf_sim::water::drink(&mut w, i), "they drank from the pond");
    assert_eq!(w.agents[i].needs.thirst, 1.0, "the drink served");
    assert!(
        w.agents[i].body.illness.is_none(),
        "a fresh drink brings nothing"
    );
}

#[test]
fn an_immune_drinker_stays_healthy() {
    let (mut w, fouled) = fouled_pond(4543);
    let i = drinker_beside(&mut w, fouled);
    w.agents[i].immune.push(IllnessKind::Flux);
    assert!(
        hf_sim::water::drink(&mut w, i),
        "they drank from the fouled pond"
    );
    assert!(
        w.agents[i].body.illness.is_none(),
        "one who has had the flux and is guarded against it stays healthy"
    );
}

fn two_ponds_at_one_stand() -> (World, usize, TilePos, TilePos) {
    use hf_sim::mind::{Known, ObservationKind, OutcomeKind};
    let mut w = World::generate(4543, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.plants.clear();
    w.meta.founding_site = TilePos::new(40, 40, 2);
    w.weather.temperature_c = 10.0;
    w.meta.tick = 9 * TICKS_PER_HOUR + 300;
    let stand = TilePos::new(30, 30, 2);
    let bad = stand.offset(0, 1, 0);
    let good = stand.offset(1, 0, 0);
    // Each pool has one bank. They meet the same dry stand but are separate bodies.
    for x in 28..=33 {
        for y in 28..=33 {
            if [(stand.x, stand.y), (bad.x, bad.y), (good.x, good.y)].contains(&(x, y)) {
                continue;
            }
            for z in [2, 3] {
                w.grid.set(
                    TilePos::new(x, y, z),
                    Tile {
                        material: Material::STONE_BLOCK,
                        shape: Shape::Wall,
                        flags: TileFlags::NONE,
                    },
                );
            }
        }
    }
    for p in [bad, good] {
        w.grid.set(p, water_tile());
    }
    w.rebuild_caches();
    floods::survey(&mut w);
    assert!(!std::ptr::eq(
        floods::body_at(&w, bad).unwrap(),
        floods::body_at(&w, good).unwrap()
    ));
    w.waters.foul.insert(bad, FOUL_HOURS);
    let who = w.found_party(1)[0];
    let i = w.agents.iter().position(|a| a.id == who).unwrap();
    w.agents[i].pos = stand;
    w.agents[i].needs.thirst = 0.45;
    w.agents[i]
        .knowledge
        .map
        .retain(|k, _| !matches!(k, Known::Water(_)));
    let tick = w.meta.tick;
    w.agents[i]
        .knowledge
        .note(Known::Water(good), good, tick, floods::TAG_WATER_FRESH);
    w.agents[i].learning.learn(
        ObservationKind::FoulWater,
        OutcomeKind::Illness,
        Some(bad),
        true,
        tick,
        None,
    );
    w.agents[i].learning.lessons[0].confidence = 0.6;
    assert!(w.agents[i].knowledge.get(Known::Water(bad)).is_none());
    w.ensure_nav();
    let spots = hf_sim::water::spots(&w, stand, 3);
    for source in [bad, good] {
        assert!(
            spots.iter().any(|s| s.source == source && s.stand == stand),
            "both sources must offer the same actual standing tile"
        );
    }
    (w, i, bad, good)
}

#[test]
fn two_ponds_sharing_a_stand_keep_the_acceptable_source() {
    let (w, i, _, good) = two_ponds_at_one_stand();
    let stand = w.agents[i].pos;
    let chosen = hf_sim::water::nearest_knowing(&w, i, stand, 3, &w.agents[i].knowledge, false)
        .expect("the fresh pool beside the refused pool remains a choice");
    assert_eq!(chosen.stand, stand);
    assert_eq!(chosen.source, good);
}

#[test]
fn two_ponds_sharing_a_stand_supply_the_accepted_water() {
    let run = |threads| {
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .unwrap()
            .install(|| {
                let (mut w, i, bad, _) = two_ponds_at_one_stand();
                let before = w.agents[i].learning.lessons.clone();
                assert!(hf_sim::water::drink(&mut w, i));
                assert_eq!(
                    w.agents[i].learning.lessons, before,
                    "drinking the fresh pool must not test a lesson about the other pool"
                );
                assert!(w.agents[i].body.illness.is_none());
                assert!(w.agents[i]
                    .knowledge
                    .get(hf_sim::mind::Known::Water(bad))
                    .is_none());
                assert_eq!(w.agents[i].needs.thirst, 1.0);
                hf_sim::digest::digest(&w)
            })
    };
    assert_eq!(run(1), run(4));
}

#[test]
fn shared_bank_keeps_the_water_allowed_by_its_own_confidence() {
    use hf_sim::mind::{ObservationKind, OutcomeKind};
    let run = |threads| {
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .unwrap()
            .install(|| {
                let (mut w, i, bad, good) = two_ponds_at_one_stand();
                w.agents[i].learning.lessons[0].confidence = 0.9;
                w.agents[i].learning.learn(
                    ObservationKind::FoulWater,
                    OutcomeKind::Illness,
                    Some(good),
                    true,
                    w.meta.tick,
                    None,
                );
                w.agents[i]
                    .learning
                    .lessons
                    .iter_mut()
                    .find(|l| l.pos == Some(good))
                    .unwrap()
                    .confidence = 0.3;
                w.agents[i].needs.thirst = 0.15;
                assert_eq!(w.agents[i].learning.lessons.len(), 2);
                let bad_lesson = w.agents[i]
                    .learning
                    .lessons
                    .iter()
                    .find(|l| l.pos == Some(bad))
                    .unwrap()
                    .clone();
                assert_eq!(bad_lesson.confidence, 0.9);
                let at = w.agents[i].pos;
                let chosen =
                    hf_sim::water::nearest_knowing(&w, i, at, 3, &w.agents[i].knowledge, true)
                        .expect("the other pond's weaker lesson permits drinking at this thirst");
                assert_eq!(chosen.source, good);
                assert!(hf_sim::water::drink(&mut w, i));
                assert_eq!(
                    w.agents[i]
                        .learning
                        .lessons
                        .iter()
                        .find(|l| l.pos == Some(bad)),
                    Some(&bad_lesson),
                    "the refused pond did not supply the drink"
                );
                assert!(w.agents[i].body.illness.is_none());
                assert_eq!(w.agents[i].needs.thirst, 1.0);
                hf_sim::digest::digest(&w)
            })
    };
    assert_eq!(run(1), run(4));
}
