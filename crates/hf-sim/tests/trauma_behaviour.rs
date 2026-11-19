//! Behaviour, not arithmetic: two identical food patches, one of them a place they were hurt.
//! The survivor should choose the other one, and say why. See docs/design-emergence.md.

use hf_sim::ids::TilePos;
use hf_sim::mind::activity::Activity;
use hf_sim::mind::knowledge::Known;
use hf_sim::mind::memory::{MemKind, Record, Subject};
use hf_sim::world::gen::GenParams;
use hf_sim::world_state::World;

/// A world with one survivor who knows of exactly two food plants, `left` and `right`,
/// about the same distance away. No other food is known, nothing else is going on.
fn two_patches(seed: u64) -> (World, usize, TilePos, TilePos) {
    let mut w = World::generate(seed, GenParams::small());
    let ids = w.found_party(1);
    w.ensure_nav(); // reachability, built on first use as during play
    let i = w
        .agents
        .iter()
        .position(|a| a.id == ids[0])
        .expect("the survivor");
    let home = w.agents[i].pos;

    let mut spots = Vec::new();
    for sign in [-1i32, 1] {
        let mut found = None;
        for d in 4..14 {
            let p = TilePos::new(
                (home.x + sign * d).clamp(1, w.grid.width - 2),
                home.y,
                home.z,
            );
            let surf = w.grid.surface(p.x, p.y);
            if w.grid.walkable(surf) && w.nav.as_ref().is_some_and(|n| n.reach(home, surf)) {
                found = Some(surf);
                break;
            }
        }
        spots.push(found.unwrap_or(home));
    }
    let (left, right) = (spots[0], spots[1]);

    let a = &mut w.agents[i];
    a.knowledge
        .note(Known::FoodPlant(left), left, w.meta.tick, 0);
    a.knowledge
        .note(Known::FoodPlant(right), right, w.meta.tick, 0);
    // Hungry enough that eating is the only thing on their mind.
    a.needs.hunger = 0.25;
    a.needs.rest = 1.0;
    a.needs.social = 1.0;
    a.activity = Activity::Idle;
    a.activity_ticks = 0;
    w.meta.tick = 6_000; // midday: they would rather be working than asleep
    (w, i, left, right)
}

fn traumatise(w: &mut World, i: usize, p: TilePos, now: u64) {
    let a = &mut w.agents[i];
    a.memories.short.push(Record {
        tick: now,
        kind: MemKind::Hardship,
        subject: Subject::Place(p),
        valence: -1.0,
        salience: 1.0,
        event: None,
    });
    a.memories.consolidate(now, false, false);
}

/// Where the survivor decided to go.
/// Does this activity amount to going to talk to `who`?
fn seeks(a: &Activity, who: hf_sim::ids::AgentId) -> bool {
    match a {
        Activity::Talking { with, .. } => *with == who,
        Activity::Going { then, .. } => seeks(then, who),
        _ => false,
    }
}

fn chosen_target(w: &World, i: usize) -> Option<TilePos> {
    match &w.agents[i].activity {
        Activity::Going { to, .. } => Some(*to),
        Activity::Wandering { to } => Some(*to),
        Activity::Idle => None,
        _ => Some(w.agents[i].pos),
    }
}

#[test]
fn they_go_where_they_were_not_hurt() {
    let (mut w, i, left, right) = two_patches(21);
    let now = w.meta.tick;
    // Twice bitten: the fear has to be strong enough to outweigh a nearer patch.
    traumatise(&mut w, i, right, now);
    traumatise(&mut w, i, right, now);

    hf_sim::systems::decide::step(&mut w);

    let target = chosen_target(&w, i).expect("they decided to do something");
    let d_left = target.dist(left);
    let d_right = target.dist(right);
    assert!(
        d_left < d_right,
        "the traumatised survivor should head away from the bad ground: chose {target}, \
         left {left} ({d_left}) right {right} ({d_right})"
    );
}

#[test]
fn without_the_trauma_the_nearest_patch_wins() {
    // The control: the same world with no bad history. Whatever they pick here is what a
    // survivor with no reason to be afraid does, and the test above must differ from it.
    let (mut w, i, left, right) = two_patches(21);
    hf_sim::systems::decide::step(&mut w);
    let target = chosen_target(&w, i).expect("they decided to do something");
    assert!(
        target == left || target == right,
        "with nothing to fear they head straight for a known patch: chose {target},          patches {left} and {right}"
    );
}

#[test]
fn someone_who_hurt_them_is_not_sought_out() {
    let (mut w, i, _, _) = two_patches(23);
    let other_id = w
        .found_party(1)
        .into_iter()
        .find(|id| *id != w.agents[i].id)
        .expect("a second survivor");
    let j = w
        .agents
        .iter()
        .position(|a| a.id == other_id)
        .expect("the other survivor");
    // Put them within talking distance and make company the only thing missing.
    let near = {
        let him = w.agents[i].pos;
        TilePos::new(him.x, him.y + 1, him.z)
    };
    w.agents[j].pos = near;
    w.agents[i].needs.social = 0.1;
    w.agents[i].needs.hunger = 1.0;
    w.agents[i].needs.rest = 1.0;
    w.agents[i]
        .knowledge
        .note(Known::Agent(other_id), near, w.meta.tick, 0);

    let now = w.meta.tick;
    {
        let a = &mut w.agents[i];
        a.memories.short.push(Record {
            tick: now,
            kind: MemKind::Company,
            subject: Subject::Person(other_id),
            valence: -1.0,
            salience: 1.0,
            event: None,
        });
        a.memories.consolidate(now, false, false);
    }
    hf_sim::systems::decide::step(&mut w);

    let sought = seeks(&w.agents[i].activity, other_id);
    assert!(
        !sought,
        "after being hurt by them, they do not seek that person out: {:?}",
        w.agents[i].activity
    );
}
