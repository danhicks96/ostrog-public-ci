//! docs/design-feelings.md step 1: what is witnessed is felt, by temperament, and fades.

use hf_sim::events::{EventKind, Severity};
use hf_sim::ids::TilePos;
use hf_sim::mind::beliefs::{Source, Topic};
use hf_sim::mind::feelings::FeelingKind as F;
use hf_sim::time::TICKS_PER_HOUR;
use hf_sim::world::gen::GenParams;
use hf_sim::world::{Material, Tile};
use hf_sim::World;

fn party(n: usize) -> (World, Vec<usize>) {
    let mut w = World::generate(5101, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.meta.tick = TICKS_PER_HOUR * 10;
    w.rebuild_caches();
    let ids = w.found_party(n);
    w.meta.autoplan = false;
    let idx: Vec<usize> = ids
        .iter()
        .map(|id| w.agents.iter().position(|a| a.id == *id).unwrap())
        .collect();
    for (k, &i) in idx.iter().enumerate() {
        w.agents[i].pos = w.grid.surface(30 + k as i32, 30);
        w.agents[i].activity = hf_sim::mind::Activity::Idle;
    }
    (w, idx)
}

fn happen(w: &mut World, at: TilePos, kind: EventKind) {
    let tick = w.meta.tick;
    let id = w.events.push(tick, Some(at), Severity::Major, kind);
    let e = w
        .events
        .pending
        .iter()
        .find(|e| e.id == id)
        .unwrap()
        .clone();
    w.last_events = vec![e];
    hf_sim::systems::feel::observe(w);
}

#[test]
fn a_death_is_grieved_by_those_who_cared_and_not_by_others() {
    let (mut w, p) = party(4);
    let dead = w.agents[p[0]].id;
    let t = w.meta.tick;
    w.agents[p[1]].relate(dead, 0.9, 0.5, t);
    let at = w.agents[p[0]].pos;
    happen(
        &mut w,
        at,
        EventKind::Died {
            agent: dead,
            cause: "a test".into(),
        },
    );
    assert!(w.agents[p[1]].feelings.of(F::Grief) > 0.5);
    assert_eq!(w.agents[p[2]].feelings.of(F::Grief), 0.0);
}

#[test]
fn the_same_fight_angers_the_bold_and_frightens_the_rest() {
    let (mut w, p) = party(3);
    let t = w.meta.tick;
    let me = w.agents[p[0]].id;
    for _ in 0..20 {
        w.agents[p[1]]
            .beliefs
            .experience(Topic::Violence, 1.0, 1.0, Source::Person(me), t);
        w.agents[p[2]]
            .beliefs
            .experience(Topic::Violence, -1.0, 1.0, Source::Person(me), t);
    }
    let stranger = w.spawn_agent(hf_sim::agents::Species::Person, w.grid.surface(33, 30));
    let si = w.agents.iter().position(|a| a.id == stranger).unwrap();
    w.agents[si].band = Some(1);
    let at = w.agents[p[0]].pos;
    happen(&mut w, at, EventKind::Fought { a: me, b: stranger });
    assert!(w.agents[p[1]].feelings.of(F::Anger) > 0.0);
    assert!(w.agents[p[2]].feelings.of(F::Fear) > 0.0);
    assert_eq!(w.agents[p[1]].feelings.of(F::Fear), 0.0);
}

#[test]
fn feelings_fade_and_a_strong_one_is_carried_as_a_thought() {
    let (mut w, p) = party(2);
    let me = w.agents[p[0]].id;
    let t = w.meta.tick;
    w.agents[p[0]].feelings.feel(F::Fear, None, 0.9, t, None);
    w.meta.tick = (t / TICKS_PER_HOUR + 1) * TICKS_PER_HOUR + 450;
    hf_sim::systems::feel::step(&mut w);
    assert!(w.agents[p[0]]
        .thoughts
        .list
        .iter()
        .any(|x| x.kind == hf_sim::mind::ThoughtKind::Afraid));
    for _ in 0..80 {
        w.meta.tick += TICKS_PER_HOUR;
        hf_sim::systems::feel::step(&mut w);
    }
    assert_eq!(w.agents[p[0]].feelings.of(F::Fear), 0.0, "faded");
    let _ = me;
}

#[test]
fn fear_passes_to_those_who_care_and_stops_at_those_who_do_not() {
    let (mut w, p) = party(3);
    let (scared, friend, foe) = (p[0], p[1], p[2]);
    let sid = w.agents[scared].id;
    let t = w.meta.tick;
    w.agents[friend].relate(sid, 0.8, 0.5, t);
    w.agents[foe].relate(sid, -0.8, 0.5, t);
    w.agents[scared].feelings.feel(F::Fear, None, 0.9, t, None);
    w.meta.tick = (t / TICKS_PER_HOUR + 1) * TICKS_PER_HOUR + 450;
    hf_sim::systems::feel::step(&mut w);
    assert!(
        w.agents[friend].feelings.of(F::Fear) > 0.0,
        "a friend takes it"
    );
    assert_eq!(
        w.agents[foe].feelings.of(F::Fear),
        0.0,
        "one who dislikes them does not"
    );
}

#[test]
fn fear_of_a_fight_needs_sight_of_it() {
    // #856: a fight 6 tiles away behind a wall must not spread fear or anger — only sight of
    // it should. Two bystanders at the same distance, one with a wall between them and the
    // fight, one with a clear line to it.
    let (mut w, p) = party(4);
    let t = w.meta.tick;
    let me = w.agents[p[0]].id;
    // Make p[1] (behind the wall) bold and p[2] (in sight) bold too, so both would show Anger
    // rather than Fear if they witnessed it — the assertion is about reach, not which feeling.
    for who in [p[1], p[2]] {
        for _ in 0..20 {
            w.agents[who]
                .beliefs
                .experience(Topic::Violence, 1.0, 1.0, Source::Person(me), t);
        }
    }
    let stranger = w.spawn_agent(hf_sim::agents::Species::Person, w.grid.surface(36, 30));
    let si = w.agents.iter().position(|a| a.id == stranger).unwrap();
    w.agents[si].band = Some(1);
    let at = w.agents[p[0]].pos; // (30, 30)
                                 // p[1] at (36, 30): straight line to the fight, 6 tiles away, in sight.
    w.agents[p[1]].pos = w.grid.surface(36, 30);
    // p[2] at (30, 36): 6 tiles away too, but a solid wall at (30, 33) blocks the line.
    w.agents[p[2]].pos = w.grid.surface(30, 36);
    let wall_z = w.agents[p[2]].pos.z;
    for x in 25..=35 {
        w.grid
            .set(TilePos::new(x, 33, wall_z), Tile::solid(Material::GRANITE));
    }
    happen(&mut w, at, EventKind::Fought { a: me, b: stranger });
    assert!(
        w.agents[p[1]].feelings.of(F::Anger) > 0.0,
        "in sight of the fight: should be angered"
    );
    assert_eq!(
        w.agents[p[2]].feelings.of(F::Anger),
        0.0,
        "behind a wall: saw nothing, should feel nothing"
    );
    assert_eq!(
        w.agents[p[2]].feelings.of(F::Fear),
        0.0,
        "behind a wall: saw nothing, should feel nothing"
    );
}

#[test]
fn the_afraid_wander_less() {
    // The same valley twice, for two days; in one, everyone is badly afraid and stays so.
    let wandering = |afraid: bool| -> u32 {
        let mut w = World::opening_world(5);
        let mut n = 0;
        for _ in 0..2 * hf_sim::time::TICKS_PER_DAY {
            if afraid {
                let t = w.meta.tick;
                for a in w
                    .agents
                    .iter_mut()
                    .filter(|a| a.alive && a.community.is_some())
                {
                    a.feelings.feel(F::Fear, None, 1.0, t, None);
                }
            }
            w.step();
            n += w
                .agents
                .iter()
                .filter(|a| {
                    a.alive
                        && a.community.is_some()
                        && matches!(a.activity, hf_sim::mind::Activity::Wandering { .. })
                })
                .count() as u32;
        }
        n
    };
    let (calm, scared) = (wandering(false), wandering(true));
    assert!(scared < calm, "wandering: calm {calm}, afraid {scared}");
}

#[test]
fn a_birth_gladdens_the_family() {
    // OST-543: a grandmother and an older brother are glad of a newborn, a friend of the mother
    // feels some joy about the child, a stranger feels nothing.
    let (mut w, p) = party(6);
    let (mother, gran, brother, friend, stranger) = (p[0], p[1], p[2], p[3], p[4]);
    for a in w.agents.iter_mut() {
        a.community = Some(hf_sim::ids::CommunityId(0));
    }
    let (mid, gid, bid) = (w.agents[mother].id, w.agents[gran].id, w.agents[brother].id);
    w.agents[mother].mother = Some(gid);
    w.agents[gran].children = vec![mid];
    w.agents[brother].mother = Some(mid);
    w.agents[mother].children = vec![bid];
    let t = w.meta.tick;
    w.agents[friend].relate(mid, 0.8, 0.5, t);
    let baby = w.agents[p[5]].id;
    let at = w.agents[mother].pos;
    happen(
        &mut w,
        at,
        EventKind::Born {
            agent: baby,
            mother: mid,
            father: None,
        },
    );
    // Each person feels it their own amount (the jitter of `gain`), so the bounds are wide.
    assert!(w.agents[gran].feelings.of(F::Joy) >= 0.3);
    assert!(w.agents[brother].feelings.of(F::Joy) >= 0.3);
    let f = w.agents[friend].feelings.of(F::Joy);
    assert!(f > 0.1 && f < 0.5, "friend {f}");
    assert_eq!(w.agents[stranger].feelings.of(F::Joy), 0.0);
}
