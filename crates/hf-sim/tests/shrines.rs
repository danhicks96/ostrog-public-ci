//! #297 ask 1: a community that keeps its observances raises a shrine, keeps them there, and
//! loses it to neglect, fire or a move.

use hf_sim::events::EventKind;
use hf_sim::ids::{AgentId, TilePos};
use hf_sim::justice::Domain;
use hf_sim::mind::beliefs::{Source, Topic};
use hf_sim::mind::ThoughtKind;
use hf_sim::structures::StructureKind;
use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use hf_sim::work::DesignationKind;
use hf_sim::world::gen::GenParams;
use hf_sim::world::Material;
use hf_sim::World;

/// A party of `n` standing together by (30, 30), the first followed in the spirit and sure of
/// the signs, the community having kept the last observance.
fn valley(seed: u64, n: usize) -> (World, Vec<AgentId>) {
    let mut w = World::generate(seed, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.rebuild_caches();
    let ids = w.found_party(n);
    w.meta.autoplan = false;
    w.communities[0].leaders[Domain::Spirit as usize] = Some(ids[0]);
    for (k, id) in ids.iter().enumerate() {
        let i = idx(&w, *id);
        w.agents[i].pos = w.grid.surface(30 + k as i32, 30);
        w.agents[i].activity = hf_sim::mind::activity::Activity::Idle;
        w.agents[i]
            .beliefs
            .hold(Topic::DeitySigns, 0.6, 0.6, Source::Culture(0), 0);
    }
    // Ten days of meals: a community short of food does not gather (#415).
    let home = w.communities[0].home;
    w.drop_item(
        hf_sim::items::ItemKind::Meal,
        (n * 30) as u32,
        home.offset(3, 3, 0),
    );
    let cid = w.communities[0].id;
    let tick = w.meta.tick;
    w.faith.kept.push((cid, tick));
    w.events.drain();
    (w, ids)
}

fn idx(w: &World, id: AgentId) -> usize {
    w.agents.iter().position(|a| a.id == id).unwrap()
}

/// The morning's planning of projects, at its hour on the next day.
fn plan_projects(w: &mut World) {
    w.meta.tick = (w.meta.tick / TICKS_PER_DAY + 1) * TICKS_PER_DAY + hf_sim::systems::projects::AT;
    w.meta.autoplan = true;
    hf_sim::systems::projects::step(w);
    w.meta.autoplan = false;
}

/// Proposed, sited, built and taken as the community's: the shrine's tile.
fn raise_shrine(w: &mut World) -> TilePos {
    plan_projects(w);
    let cid = w.communities[0].id;
    let home = w.communities[0].home;
    w.meta.planning_for = Some(cid);
    hf_sim::shrines::plan(w, cid, home);
    let d = w
        .designations
        .iter()
        .find(|d| {
            matches!(
                d.kind,
                DesignationKind::BuildStructure(StructureKind::Shrine, _)
            )
        })
        .expect("the planner ordered the shrine built")
        .clone();
    let DesignationKind::BuildStructure(_, mat) = d.kind else {
        unreachable!()
    };
    w.designations.retain(|x| x.id != d.id);
    w.place_structure(StructureKind::Shrine, d.pos, mat, false);
    w.meta.tick = (w.meta.tick / TICKS_PER_HOUR + 1) * TICKS_PER_HOUR + 41;
    hf_sim::shrines::step(w);
    assert_eq!(hf_sim::shrines::of(w, cid), Some(d.pos), "taken as theirs");
    d.pos
}

#[test]
fn a_community_that_keeps_observances_raises_a_shrine_and_keeps_them_there() {
    let (mut w, ids) = valley(2977, 6);
    plan_projects(&mut w);
    let proposed = w.events.pending.iter().find_map(|e| match &e.kind {
        EventKind::Proposed {
            agent,
            what,
            because,
        } if what == "to raise a shrine" => Some((*agent, because.clone())),
        _ => None,
    });
    let (by, because) = proposed.expect("someone proposed a shrine");
    assert_eq!(by, ids[0], "the one followed in the spirit");
    assert!(because.contains("the signs must be heeded"), "{because}");
    // Raised where the planner put it, of the timber the place builds with (no stone house).
    w.projects.clear();
    let at = raise_shrine(&mut w);
    let st = w.structures.iter().find(|s| s.pos == at).unwrap();
    assert!(
        st.material == Material::WOOD
            || w.grid.materials.get(st.material).kind
                == hf_sim::world::material::MaterialKind::Wood
    );
    assert!(w.communities[0].home.dist(at) <= 12);
    // The season's observance: an hour before noon those free walk to the shrine.
    w.events.drain();
    let day = w.meta.epoch + 5 * TICKS_PER_DAY;
    w.meta.tick = day + 11 * TICKS_PER_HOUR;
    hf_sim::observances::step(&mut w);
    let going = w
        .agents
        .iter()
        .filter(|a| {
            matches!(&a.activity, hf_sim::mind::activity::Activity::Going { to, .. } if to.dist(at) <= 3)
        })
        .count();
    assert!(going >= 4, "{going} went to the shrine");
    // They arrive (all but one, who stays away at the far edge).
    let away = idx(&w, ids[5]);
    for &id in &ids {
        let i = idx(&w, id);
        w.agents[i].activity = hf_sim::mind::activity::Activity::Idle;
        w.agents[i].pos = if i == away {
            w.grid.surface(58, 58)
        } else {
            w.grid.surface(at.x + 1, at.y + 1)
        };
    }
    // Worn a little since it was raised.
    w.structures
        .iter_mut()
        .find(|s| s.pos == at)
        .unwrap()
        .condition = 0.5;
    w.meta.tick = day + 12 * TICKS_PER_HOUR;
    hf_sim::observances::step(&mut w);
    let kept = w.events.pending.iter().find_map(|e| match &e.kind {
        EventKind::Observance {
            by,
            came,
            at_shrine,
            ..
        } => Some((*by, *came, *at_shrine)),
        _ => None,
    });
    assert_eq!(kept, Some((ids[0], 4, true)), "kept at the shrine");
    assert_eq!(
        w.structures.iter().find(|s| s.pos == at).unwrap().condition,
        1.0,
        "made good by being used"
    );
}

#[test]
fn a_shrine_left_untended_falls_and_is_remembered() {
    let (mut w, ids) = valley(2978, 6);
    let at = raise_shrine(&mut w);
    let cid = w.communities[0].id;
    // A year without an observance kept at it: it wears through at dawn.
    w.events.drain();
    w.structures
        .iter_mut()
        .find(|s| s.pos == at)
        .unwrap()
        .condition = 0.04;
    w.meta.tick = (w.meta.tick / TICKS_PER_DAY + 1) * TICKS_PER_DAY + 6 * TICKS_PER_HOUR + 41;
    hf_sim::shrines::step(&mut w);
    assert!(w.events.pending.iter().any(|e| matches!(
        &e.kind,
        EventKind::ShrineLost { community, why } if *community == cid && why.contains("untended")
    )));
    assert!(hf_sim::shrines::of(&w, cid).is_none());
    assert!(!w.structures.iter().any(|s| s.kind == StructureKind::Shrine));
    let k = idx(&w, ids[2]);
    assert!(w.agents[k].thoughts.has(ThoughtKind::ShrineLost));
    // The next one proposed remembers it.
    w.projects.clear();
    w.events.drain();
    plan_projects(&mut w);
    assert!(w.events.pending.iter().any(|e| matches!(
        &e.kind,
        EventKind::Proposed { what, because, .. }
            if what == "to raise a shrine" && because.contains("remembers the shrine")
    )));
}

#[test]
fn a_timber_shrine_burns_and_one_left_behind_by_a_move_is_lost() {
    let (mut w, _) = valley(2979, 6);
    let at = raise_shrine(&mut w);
    let cid = w.communities[0].id;
    // Timber carries a fire; stone does not.
    assert!(hf_sim::systems::hazards::fuel_at(&w, at) > 0);
    let stone = w.grid.surface(20, 40);
    w.place_structure(StructureKind::Shrine, stone, Material::GRANITE, false);
    assert_eq!(hf_sim::systems::hazards::fuel_at(&w, stone), 0);
    w.structures.retain(|s| s.pos != stone);
    // Burned: the structure gone and ash where it stood.
    w.events.drain();
    w.structures.retain(|s| s.pos != at);
    let tick = w.meta.tick;
    w.scorched.insert(at.offset(0, 0, -1), tick);
    w.meta.tick += TICKS_PER_HOUR;
    hf_sim::shrines::step(&mut w);
    assert!(w.events.pending.iter().any(|e| matches!(
        &e.kind,
        EventKind::ShrineLost { why, .. } if why == "it burned"
    )));
    // Raised again, then the community moves camp far off and leaves it standing.
    w.projects.clear();
    w.faith.lost.clear();
    let at = raise_shrine(&mut w);
    w.events.drain();
    w.communities[0].home = TilePos::new(at.x + 60, at.y, at.z);
    w.meta.tick += TICKS_PER_HOUR;
    hf_sim::shrines::step(&mut w);
    assert!(w.events.pending.iter().any(|e| matches!(
        &e.kind,
        EventKind::ShrineLost { why, .. } if why.contains("left behind")
    )));
    assert!(w.structures.iter().any(|s| s.pos == at), "still standing");
    // Not taken up and lost again each hour.
    w.events.drain();
    w.meta.tick += TICKS_PER_HOUR;
    hf_sim::shrines::step(&mut w);
    assert!(hf_sim::shrines::of(&w, cid).is_none());
    assert!(!w
        .events
        .pending
        .iter()
        .any(|e| matches!(e.kind, EventKind::ShrineLost { .. })));
}

#[test]
fn in_play_the_shrine_is_proposed_built_and_taken_as_theirs() {
    use hf_sim::items::ItemKind;
    let mut w = World::generate(2980, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.plants.clear();
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.meta.tick = TICKS_PER_HOUR * 7;
    w.rebuild_caches();
    let ids = w.found_party(6);
    let cid = w.communities[0].id;
    let home = w.communities[0].home;
    for id in &ids {
        let i = idx(&w, *id);
        w.agents[i]
            .beliefs
            .hold(Topic::DeitySigns, 0.6, 0.6, Source::Culture(0), 0);
    }
    let tick = w.meta.tick;
    w.faith.kept.push((cid, tick));
    w.drop_item(ItemKind::Log, 30, home.offset(3, 0, 0));
    w.drop_item(ItemKind::Meal, 60, home.offset(3, 1, 0));
    w.meta.autoplan = true;
    w.ensure_nav();
    for _ in 0..3 * TICKS_PER_DAY {
        w.step();
        if hf_sim::shrines::of(&w, cid).is_some() {
            break;
        }
    }
    let at = hf_sim::shrines::of(&w, cid).expect("the shrine stands and is theirs");
    assert!(home.dist(at) <= 12);
}
