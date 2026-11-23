//! E16 phase 1: personal evidence is bounded, local, and persistent.

use hf_sim::agents::Species;
use hf_sim::events::{Event, EventKind, Severity};
use hf_sim::ids::{AgentId, EventId, TilePos};
use hf_sim::mind::{Evidence, Observation, ObservationKind, OutcomeKind};
use hf_sim::save::{self, snapshot::Snapshot};
use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use hf_sim::world::gen::GenParams;
use hf_sim::world::{Material, Shape, Tile, TileFlags};
use hf_sim::World;

fn event(id: u32, tick: u64, at: Option<TilePos>, kind: EventKind) -> Event {
    Event {
        id: EventId(id),
        tick,
        at,
        severity: Severity::Notable,
        kind,
    }
}

fn confirmed_lesson(
    w: &World,
    agent: AgentId,
    cue: ObservationKind,
    outcome: OutcomeKind,
    at: TilePos,
    tick: u64,
) {
    let learning = &w.agents.iter().find(|a| a.id == agent).unwrap().learning;
    let lesson = learning.lesson(cue, outcome, Some(at)).unwrap();
    assert_eq!(lesson.confirmations, 1);
    assert_eq!(lesson.contradictions, 0);
    assert_eq!(lesson.formed, tick);
    assert_eq!(lesson.last_tested, tick);
}

#[test]
fn observations_and_lessons_belong_to_one_person_and_survive_save_encoding() {
    let mut w = World::generate(116, GenParams::small());
    let p = w.meta.founding_site;
    let swimmer = w.spawn_agent(Species::Person, p);
    let bystander = w.spawn_agent(Species::Person, p.offset(2, 0, 0));
    let swimmer_i = w.agents.iter().position(|a| a.id == swimmer).unwrap();
    let bystander_i = w.agents.iter().position(|a| a.id == bystander).unwrap();

    w.agents[swimmer_i].learning.observe(Observation {
        tick: 12,
        pos: TilePos::new(8, 8, 1),
        kind: ObservationKind::WaterCurrent,
        strength: 0.8,
        evidence: Evidence::MisledBy(AgentId(91)),
        event: Some(EventId(44)),
        full: false,
        reading: None,
    });
    w.agents[swimmer_i].learning.learn(
        ObservationKind::WaterCurrent,
        OutcomeKind::Displacement,
        Some(TilePos::new(8, 8, 1)),
        false,
        13,
        Some(EventId(45)),
    );

    assert_eq!(w.agents[bystander_i].learning.observations.len(), 0);
    assert_eq!(w.agents[bystander_i].learning.lessons.len(), 0);

    let bytes = save::encode(&Snapshot::capture(&w)).unwrap();
    let back = save::decode(&bytes).unwrap().restore();
    let saved = back.agents.iter().find(|a| a.id == swimmer).unwrap();
    assert_eq!(saved.learning.observations.len(), 1);
    assert_eq!(saved.learning.lessons.len(), 1);
    assert_eq!(saved.learning.observations[0].tick, 12);
    assert_eq!(saved.learning.observations[0].pos, TilePos::new(8, 8, 1));
    assert_eq!(saved.learning.observations[0].event, Some(EventId(44)));
    assert_eq!(saved.learning.lessons[0].formed, 13);
    assert_eq!(saved.learning.lessons[0].contradictions, 1);
    assert_eq!(
        saved.learning.observations[0].evidence,
        Evidence::MisledBy(AgentId(91))
    );
    assert_eq!(saved.learning.lessons[0].origins, vec![EventId(45)]);
    let restored_bystander = back.agents.iter().find(|a| a.id == bystander).unwrap();
    assert!(restored_bystander.learning.observations.is_empty());
    assert!(restored_bystander.learning.lessons.is_empty());
}

#[test]
fn saves_from_before_learning_default_to_no_personal_evidence() {
    let fixture =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/saves/v2.hfsave");
    let (old, _chronicle) = save::read(&fixture).unwrap();
    assert!(!old.agents.is_empty());
    assert!(old
        .agents
        .iter()
        .all(|a| { a.learning.observations.is_empty() && a.learning.lessons.is_empty() }));
}

#[test]
fn collapses_distinguish_harm_from_no_change() {
    let mut w = World::generate(117, GenParams::small());
    let p = w.meta.founding_site;
    let miner = w.spawn_agent(Species::Person, p);
    let other = w.spawn_agent(Species::Person, p.offset(1, 0, 0));
    w.meta.tick = 90;
    let cases = [
        (
            event(
                81,
                4,
                Some(p),
                EventKind::Collapse {
                    by: miner,
                    at: p,
                    tiles: 1,
                    rock: "stone".into(),
                    sank: false,
                    hurt: vec![miner],
                },
            ),
            OutcomeKind::Harm,
        ),
        (
            event(
                82,
                5,
                Some(p.offset(1, 0, 0)),
                EventKind::Collapse {
                    by: miner,
                    at: p.offset(1, 0, 0),
                    tiles: 1,
                    rock: "stone".into(),
                    sank: false,
                    hurt: vec![],
                },
            ),
            OutcomeKind::NoChange,
        ),
        (
            event(
                83,
                6,
                Some(p.offset(2, 0, 0)),
                EventKind::Collapse {
                    by: miner,
                    at: p.offset(2, 0, 0),
                    tiles: 2,
                    rock: "stone".into(),
                    sank: false,
                    hurt: vec![],
                },
            ),
            OutcomeKind::NoChange,
        ),
        (
            event(
                84,
                7,
                Some(p.offset(3, 0, 0)),
                EventKind::Collapse {
                    by: miner,
                    at: p.offset(3, 0, 0),
                    tiles: 2,
                    rock: "stone".into(),
                    sank: false,
                    hurt: vec![other],
                },
            ),
            OutcomeKind::Harm,
        ),
    ];

    for (e, outcome) in cases {
        let at = e.at.unwrap();
        w.last_events.clear();
        w.last_events.push(e.clone());
        hf_sim::systems::learn::observe(&mut w);
        confirmed_lesson(
            &w,
            miner,
            ObservationKind::StructuralWeakness,
            outcome,
            at,
            e.tick,
        );
        let opposite = if outcome == OutcomeKind::Harm {
            OutcomeKind::NoChange
        } else {
            OutcomeKind::Harm
        };
        let actor = w.agents.iter().find(|a| a.id == miner).unwrap();
        assert!(actor
            .learning
            .lesson(ObservationKind::StructuralWeakness, opposite, Some(at))
            .is_none());
    }
    let bystander = w.agents.iter().find(|a| a.id == other).unwrap();
    assert!(bystander.learning.observations.is_empty());
    assert!(bystander.learning.lessons.is_empty());
}

#[test]
fn water_lessons_keep_the_event_worksite_tick_and_confirmation() {
    let mut w = World::generate(118, GenParams::small());
    let p = w.meta.founding_site;
    let miner = w.spawn_agent(Species::Person, p);
    let witness = w.spawn_agent(Species::Person, p.offset(1, 0, 0));
    let miner_i = w.agents.iter().position(|a| a.id == miner).unwrap();
    let struck_at = p.offset(3, 4, -1);
    let breach_at = p.offset(5, 6, -1);

    w.last_events.push(event(
        91,
        42,
        Some(struck_at),
        EventKind::StruckWater {
            agent: miner,
            tiles: 3,
        },
    ));
    w.agents[miner_i].pos = p.offset(9, 9, 0);
    w.meta.tick = 90;
    hf_sim::systems::learn::observe(&mut w);
    confirmed_lesson(
        &w,
        miner,
        ObservationKind::Saturation,
        OutcomeKind::Discovery,
        struck_at,
        42,
    );

    w.last_events.clear();
    w.last_events.push(event(
        92,
        43,
        Some(breach_at),
        EventKind::WaterBrokeIn {
            agent: miner,
            tiles: 4,
        },
    ));
    w.agents[miner_i].pos = p.offset(10, 10, 0);
    w.meta.tick = 91;
    hf_sim::systems::learn::observe(&mut w);
    confirmed_lesson(
        &w,
        miner,
        ObservationKind::Saturation,
        OutcomeKind::Discovery,
        breach_at,
        43,
    );

    w.last_events.clear();
    w.last_events.push(event(
        93,
        44,
        None,
        EventKind::WaterBrokeIn {
            agent: miner,
            tiles: 1,
        },
    ));
    hf_sim::systems::learn::observe(&mut w);
    let actor = w.agents.iter().find(|a| a.id == miner).unwrap();
    assert_eq!(actor.learning.observations.len(), 2);
    assert_eq!(actor.learning.lessons.len(), 2);
    assert_eq!(actor.learning.observations[0].tick, 42);
    assert_eq!(actor.learning.observations[0].pos, struck_at);
    assert_eq!(actor.learning.observations[1].tick, 43);
    assert_eq!(actor.learning.observations[1].pos, breach_at);
    assert!(actor
        .learning
        .lessons
        .iter()
        .all(|l| l.outcome != OutcomeKind::Displacement));
    assert!(actor
        .learning
        .observations
        .iter()
        .all(|o| o.pos != TilePos::new(0, 0, 0)));
    let bystander = w.agents.iter().find(|a| a.id == witness).unwrap();
    assert!(bystander.learning.observations.is_empty());
    assert!(bystander.learning.lessons.is_empty());
}

fn perception_world(seed: u64, flags: u8, blocked: bool) -> (World, AgentId, AgentId, TilePos) {
    let mut w = World::generate(seed, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.plants.clear();
    w.items.clear();
    w.agents.clear();
    w.meta.founding_site = TilePos::new(30, 30, 2);
    let observer = w.spawn_agent(Species::Person, TilePos::new(20, 30, 2));
    let bystander = w.spawn_agent(Species::Person, TilePos::new(40, 30, 2));
    let water = TilePos::new(22, 30, 2);
    w.grid.set(
        water,
        Tile {
            material: Material::WATER,
            shape: Shape::Floor,
            flags: TileFlags(flags),
        },
    );
    if blocked {
        w.grid.set(
            TilePos::new(21, 30, 2),
            Tile {
                material: Material::STONE_BLOCK,
                shape: Shape::Wall,
                flags: TileFlags::NONE,
            },
        );
    }
    w.rebuild_caches();
    (w, observer, bystander, water)
}

fn look(w: &mut World) {
    let noon = 10 * TICKS_PER_DAY + 12 * TICKS_PER_HOUR;
    for tick in noon..noon + hf_sim::systems::perceive::STRIPE as u64 {
        w.meta.tick = tick;
        hf_sim::systems::perceive::step(w);
    }
}

fn assert_only_observer_saw_water(flags: u8, expected: ObservationKind, strength: f32) {
    let (mut w, observer, bystander, water) = perception_world(119, flags, false);
    look(&mut w);
    let seeing = &w.agents.iter().find(|a| a.id == observer).unwrap().learning;
    assert_eq!(seeing.observations.len(), 1);
    assert_eq!(seeing.observations[0].pos, water);
    assert_eq!(seeing.observations[0].kind, expected);
    assert_eq!(seeing.observations[0].strength, strength);
    assert_eq!(seeing.observations[0].evidence, Evidence::Direct);
    assert!(seeing.observations[0].full);
    let other = &w
        .agents
        .iter()
        .find(|a| a.id == bystander)
        .unwrap()
        .learning;
    assert!(other.observations.is_empty());
    assert!(other.lessons.is_empty());
}

#[test]
fn direct_perception_distinguishes_shallow_and_deep_water_for_the_observer_only() {
    assert_only_observer_saw_water(TileFlags::WATER, ObservationKind::Wetness, 0.55);
    assert_only_observer_saw_water(
        TileFlags::WATER | TileFlags::DEEP,
        ObservationKind::WaterDepth,
        1.0,
    );
}

#[test]
fn direct_water_observation_requires_line_of_sight() {
    let (mut w, observer, bystander, _) = perception_world(120, TileFlags::WATER, true);
    look(&mut w);
    for id in [observer, bystander] {
        assert!(w
            .agents
            .iter()
            .find(|a| a.id == id)
            .unwrap()
            .learning
            .observations
            .is_empty());
    }
}
