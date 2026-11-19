//! #569: expertise shapes perception. A farmer, a hunter and a builder stand in one scene with
//! poor ground underfoot, a deer behind a rock wall and a timber wall site short of planks. Each
//! notices what their skill reads, with the reading their skill gives it; a novice notices none
//! of it; grief narrows attention; a constant cue fades from notice and returns when it changes.

use hf_sim::agents::Species;
use hf_sim::ids::{AgentId, TilePos};
use hf_sim::mind::feelings::FeelingKind;
use hf_sim::mind::{Activity, ObservationKind, Reading, Skill};
use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use hf_sim::world::gen::GenParams;
use hf_sim::world::{Material, Tile};
use hf_sim::World;

struct Scene {
    w: World,
    farmer: AgentId,
    hunter: AgentId,
    builder: AgentId,
    novice: AgentId,
}

fn idx(w: &World, id: AgentId) -> usize {
    w.agents.iter().position(|a| a.id == id).unwrap()
}

fn scene(seed: u64) -> Scene {
    let mut w = World::generate(seed, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.plants.clear();
    w.meta.founding_site = TilePos::new(20, 20, 2);
    w.meta.tick = TICKS_PER_DAY * 30 + TICKS_PER_HOUR * 11;
    w.rebuild_caches();
    let ids = w.found_party(4);
    // Everything else that walks is taken away: only the deer below is there to read.
    w.agents
        .retain(|a| a.species == Species::Person && ids.contains(&a.id));
    // A rock wall east of them, and a deer behind it, out of sight.
    for y in 10..31 {
        w.grid
            .set(TilePos::new(25, y, 2), Tile::solid(Material::GRANITE));
    }
    let deer = w.spawn_agent(Species::Deer, TilePos::new(28, 20, 2));
    let _ = deer;
    // A timber wall site, short of its two planks, in plain view.
    let site = TilePos::new(20, 23, 2);
    let d = hf_sim::systems::settlement::make(
        &mut w,
        hf_sim::work::DesignationKind::BuildWall(Material::WOOD),
        site,
        5,
    );
    w.designations.push(d);
    w.rebuild_caches();
    w.ensure_nav();
    let spots = [(20, 20), (21, 20), (20, 21), (21, 21)];
    for (k, id) in ids.iter().enumerate() {
        let i = idx(&w, *id);
        let p = TilePos::new(spots[k].0, spots[k].1, 2);
        let a = &mut w.agents[i];
        a.pos = p;
        a.from = p;
        a.activity = Activity::Loitering { ticks_left: 10_000 };
        a.needs.rest = 1.0;
        a.stress = 0.0;
        a.skills = Default::default();
        a.learning = Default::default();
        // Poor ground under each of them: sand.
        w.grid.set(p.offset(0, 0, -1), Tile::solid(Material::SAND));
    }
    let skilled = [
        (0, Skill::Farming),
        (1, Skill::Animals),
        (2, Skill::Construction),
    ];
    for (k, s) in skilled {
        let i = idx(&w, ids[k]);
        w.agents[i].skills.states[s as usize].level = 15.0;
    }
    w.rebuild_caches();
    Scene {
        w,
        farmer: ids[0],
        hunter: ids[1],
        builder: ids[2],
        novice: ids[3],
    }
}

/// Everyone reads once (perception is striped over twenty ticks, and the reading eye looks on
/// one look in `EXPERT_EVERY`).
fn look(w: &mut World) {
    let n = hf_sim::systems::perceive::STRIPE as u64 * hf_sim::systems::perceive::EXPERT_EVERY;
    for _ in 0..n {
        hf_sim::systems::perceive::step(w);
        w.meta.tick += 1;
    }
}

fn read(w: &World, who: AgentId, kind: ObservationKind) -> Vec<Reading> {
    w.agents[idx(w, who)]
        .learning
        .observations
        .iter()
        .filter(|o| o.kind == kind)
        .filter_map(|o| o.reading)
        .collect()
}

const EXPERT: [ObservationKind; 3] = [
    ObservationKind::Soil,
    ObservationKind::Tracks,
    ObservationKind::Joint,
];

#[test]
fn each_expert_reads_their_own_cue_and_the_novice_none() {
    let mut s = scene(5691);
    look(&mut s.w);
    let w = &s.w;
    assert_eq!(
        read(w, s.farmer, ObservationKind::Soil),
        vec![Reading::Ground { pct: 65 }],
        "the farmer reads the sand as poor ground"
    );
    assert_eq!(
        read(w, s.hunter, ObservationKind::Tracks),
        vec![Reading::Passed(Species::Deer)],
        "the hunter reads the deer behind the wall"
    );
    assert_eq!(
        read(w, s.builder, ObservationKind::Joint),
        vec![Reading::Lacks {
            kind: hf_sim::items::ItemKind::Plank,
            count: 2
        }],
        "the builder sees why the wall is stuck"
    );
    // Each misses the others' cues, and the novice misses all three.
    for (who, own) in [
        (s.farmer, ObservationKind::Soil),
        (s.hunter, ObservationKind::Tracks),
        (s.builder, ObservationKind::Joint),
    ] {
        for k in EXPERT.into_iter().filter(|k| *k != own) {
            assert!(read(w, who, k).is_empty(), "{who:?} {k:?}");
        }
    }
    for k in EXPERT {
        assert!(read(w, s.novice, k).is_empty(), "novice {k:?}");
    }
}

#[test]
fn grief_narrows_attention() {
    let mut s = scene(5692);
    let i = idx(&s.w, s.farmer);
    let t = s.w.meta.tick;
    s.w.agents[i]
        .feelings
        .feel(FeelingKind::Grief, None, 1.0, t, None);
    look(&mut s.w);
    assert!(read(&s.w, s.farmer, ObservationKind::Soil).is_empty());
}

#[test]
fn a_constant_cue_fades_from_notice_and_returns_when_it_changes() {
    let mut s = scene(5693);
    s.w.trace_why(Some(s.farmer));
    look(&mut s.w);
    let i = idx(&s.w, s.farmer);
    let first = s.w.agents[i]
        .learning
        .observations
        .iter()
        .find(|o| o.kind == ObservationKind::Soil)
        .unwrap()
        .tick;
    look(&mut s.w);
    let again = s.w.agents[i]
        .learning
        .observations
        .iter()
        .find(|o| o.kind == ObservationKind::Soil)
        .unwrap()
        .tick;
    assert_eq!(first, again, "habituated: not noticed again");
    let last =
        s.w.why
            .as_ref()
            .unwrap()
            .percepts
            .iter()
            .rev()
            .find(|p| p.domain == "the ground")
            .unwrap()
            .clone();
    assert!(last.habituated && !last.noticed, "{last:?}");
    // The ground changes under them: gravel reads differently and is noticed.
    let p = s.w.agents[i].pos.offset(0, 0, -1);
    s.w.grid.set(p, Tile::solid(Material::GRAVEL));
    look(&mut s.w);
    assert!(read(&s.w, s.farmer, ObservationKind::Soil).contains(&Reading::Ground { pct: 55 }));
}

#[test]
fn the_traced_novice_shows_what_was_there_but_not_noticed() {
    let mut s = scene(5694);
    s.w.trace_why(Some(s.novice));
    look(&mut s.w);
    let t = s.w.why.as_ref().unwrap();
    assert_eq!(t.percepts.len(), 3, "{:?}", t.percepts);
    assert!(t.percepts.iter().all(|p| !p.noticed && p.salience < 0.5));
    assert!(t.percepts.iter().any(|p| p.what.contains("deer")));
}
