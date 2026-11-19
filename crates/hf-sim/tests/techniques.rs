//! Techniques (design-organic.md section 3): nobody is sent to a fire. Whoever is there and
//! knows what to do decides to, with what they know; others run, or stand. Watching it work
//! teaches it, and so does trying it and seeing it work.

use hf_sim::events::EventKind;
use hf_sim::ids::{AgentId, TilePos};
use hf_sim::mind::techniques::{Learned, Technique, Techniques};
use hf_sim::mind::Trait;
use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use hf_sim::work::DesignationKind;
use hf_sim::world::gen::GenParams;
use hf_sim::world::{Material, Shape, Tile};
use hf_sim::World;

fn grass_meadow(seed: u64) -> World {
    let mut w = World::generate(seed, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.plants.clear();
    for y in 10..50 {
        for x in 10..50 {
            w.grid
                .set(TilePos::new(x, y, 1), Tile::solid(Material::GRASS));
            let above = TilePos::new(x, y, 2);
            if w.grid.get(above).shape != Shape::Open {
                w.grid.set(above, Tile::AIR);
            }
        }
    }
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.meta.tick = 7 * TICKS_PER_DAY + TICKS_PER_HOUR * 11;
    w.rebuild_caches();
    let mut dry = w.weather;
    dry.sky = hf_sim::systems::weather::Sky::Clear;
    dry.temperature_c = 24.0;
    w.force_weather = Some(dry);
    // Grass carries a fire only in dry heat (C71, #258): the weather of the moment too.
    w.weather = dry;
    w
}

/// Run until the fire is out or a day passes; return (fought events, fire-fighting
/// designations that had no owner, fire still burning).
fn run_fire(w: &mut World, ids: &[AgentId]) -> (usize, usize, bool) {
    w.ensure_nav();
    assert!(w.ignite(TilePos::new(36, 33, 2), "in a test"));
    let mut fought = 0;
    let mut unowned = 0;
    for _ in 0..TICKS_PER_DAY {
        w.step();
        unowned += w
            .designations
            .iter()
            .filter(|d| matches!(d.kind, DesignationKind::Douse | DesignationKind::BeatOut))
            .filter(|d| d.owner.is_none())
            .count();
        for e in &w.last_events {
            if let EventKind::BeatOut { agent, .. } | EventKind::Doused { agent, .. } = &e.kind {
                assert!(ids.contains(agent));
                fought += 1;
            }
        }
        if w.fires.is_empty() {
            break;
        }
    }
    (fought, unowned, !w.fires.is_empty())
}

#[test]
fn fire_fought_without_orders() {
    // Ordinary people: some know how to fight a grass fire, from their lives.
    let mut w = grass_meadow(2101);
    let ids = w.found_party(6);
    w.meta.autoplan = true;
    let knows = w
        .agents
        .iter()
        .filter(|a| {
            a.techniques.knows(Technique::BeatFlames) || a.techniques.knows(Technique::CarryWater)
        })
        .count();
    assert!(knows > 0, "someone here has fought a fire before");
    let (fought, unowned, _) = run_fire(&mut w, &ids);
    assert_eq!(
        unowned, 0,
        "no fire-fighting order was posted by the settlement"
    );
    assert!(fought >= 1, "someone chose to fight it");

    // The same meadow and people, but nobody knows how and nobody is the kind to try.
    let mut w = grass_meadow(2101);
    let ids = w.found_party(6);
    w.meta.autoplan = true;
    for a in w.agents.iter_mut() {
        a.techniques = Techniques::default();
        a.traits
            .list
            .retain(|t| *t != Trait::Curious && *t != Trait::Industrious);
    }
    let (fought, unowned, _) = run_fire(&mut w, &ids);
    assert_eq!(unowned, 0);
    assert_eq!(fought, 0, "nobody who did not know how went at it");
}

#[test]
fn technique_spreads_by_watching() {
    let mut learned = 0;
    for seed in 0..6u64 {
        let mut w = grass_meadow(2200 + seed);
        let ids = w.found_party(6);
        w.meta.autoplan = true;
        // One knows how; the rest do not.
        for (k, a) in w.agents.iter_mut().enumerate() {
            a.techniques = Techniques::default();
            if k == 0 {
                a.techniques
                    .learn(Technique::BeatFlames, Learned::Past, 0.9, 0);
            }
            a.traits
                .list
                .retain(|t| *t != Trait::Curious && *t != Trait::Industrious);
        }
        let (fought, _, _) = run_fire(&mut w, &ids);
        if fought == 0 {
            continue;
        }
        learned +=
            w.agents
                .iter()
                .filter(|a| {
                    a.techniques.list.iter().any(|k| {
                        k.technique == Technique::BeatFlames && k.learned == Learned::Watched
                    })
                })
                .count();
    }
    assert!(learned >= 1, "someone who watched it work learned it");
}

#[test]
fn trying_it_can_teach_it() {
    let mut t = Techniques::default();
    t.outcome(Technique::BeatFlames, false, 1);
    assert!(!t.knows(Technique::BeatFlames), "a failure teaches nothing");
    t.outcome(Technique::BeatFlames, true, 2);
    let k = &t.list[0];
    assert_eq!(k.learned, Learned::Tried);
    assert_eq!(k.wins, 1);
    let before = k.skill;
    t.outcome(Technique::BeatFlames, true, 3);
    assert!(
        t.skill(Technique::BeatFlames) > before,
        "and use makes it better"
    );
}

#[test]
fn survivors_bring_what_their_pasts_taught() {
    // Over many villages, those who stood in the line at a village fire know how to fight one.
    let mut fought_fire = 0;
    let mut know = 0;
    for seed in 0..30u64 {
        let w = World::opening_world(seed);
        for a in w.agents.iter().filter(|a| !a.past.is_empty()) {
            let life = hf_sim::past::life(
                seed,
                w.survivor_pool
                    .iter()
                    .find(|s| hf_sim::opening::slot_name(seed, s.index, s.rerolls) == a.name)
                    .map(|s| s.index)
                    .unwrap(),
                0,
                &a.traits,
            );
            if life
                .events
                .iter()
                .any(|e| e.what == hf_sim::past::Formative::FoughtFire)
            {
                fought_fire += 1;
                if a.techniques.knows(Technique::BeatFlames) {
                    know += 1;
                }
            }
        }
    }
    assert!(fought_fire >= 3, "{fought_fire}");
    assert_eq!(
        know, fought_fire,
        "everyone who fought a fire at home knows how"
    );
}
