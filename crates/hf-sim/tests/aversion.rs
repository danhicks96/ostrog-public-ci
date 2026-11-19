//! Trauma and temperament: what happened to a survivor changes where they will go, how far the
//! shadow of a bad place reaches, and how quickly it fades. See docs/design-emergence.md.

use hf_sim::ids::TilePos;
use hf_sim::mind::memory::{MemKind, Record, Subject};
use hf_sim::mind::traits::Trait;
use hf_sim::world::gen::GenParams;
use hf_sim::world_state::World;

const AVERSION_RADIUS: i32 = 6;

/// A world with one survivor at the founding site; returns (world, index).
fn lone_survivor(seed: u64) -> (World, usize) {
    let mut w = World::generate(seed, GenParams::small());
    let ids = w.found_party(1);
    let i = w
        .agents
        .iter()
        .position(|a| a.id == ids[0])
        .expect("the survivor");
    (w, i)
}

/// Scare the survivor badly at `p` and let them sleep on it, so it becomes a real memory.
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

fn walkable_near(w: &World, from: TilePos, dx: i32, dy: i32) -> TilePos {
    // Ring search so the helper never silently returns `from` just because one axis was blocked.
    for r in 0..40i32 {
        for cand in [
            TilePos::new(from.x + dx + r, from.y + dy, from.z),
            TilePos::new(from.x + dx - r, from.y + dy, from.z),
            TilePos::new(from.x + dx, from.y + dy + r, from.z),
            TilePos::new(from.x + dx, from.y + dy - r, from.z),
        ] {
            let x = cand.x.clamp(1, w.grid.width - 2);
            let y = cand.y.clamp(1, w.grid.height - 2);
            let surf = w.grid.surface(x, y);
            if w.grid.walkable(surf) && surf != from {
                return surf;
            }
        }
    }
    from
}

/// A tile exactly `d` tiles east of `p`. Distances in these tests are arithmetic, not walked:
/// what is being tested is the shadow a memory casts, not the shape of one particular map.
fn east(p: TilePos, d: i32) -> TilePos {
    TilePos::new(p.x + d, p.y, p.z)
}

#[test]
fn a_bad_place_is_avoided_and_the_avoidance_is_local() {
    let (mut w, i) = lone_survivor(11);
    let home = w.agents[i].pos;
    let p = walkable_near(&w, home, 8, 0);
    assert!(p != home, "the test needs a real second place");
    traumatise(&mut w, i, p, 1000);
    let a = &w.agents[i];

    // One serious incident with a whole night to sleep on it leaves them about half pulled away.
    let at = a.memories.aversion(p, 1000, false);
    assert!(at > 0.4, "one bad incident leaves real wariness: {at}");

    let near_v = a
        .memories
        .aversion(east(p, AVERSION_RADIUS / 2), 1000, false);
    assert!(
        near_v > 0.0 && near_v < at,
        "the shadow reaches nearby ground but is weaker: near {near_v} at {at}"
    );

    let far_v = a.memories.aversion(east(p, 40), 1000, false);
    assert!(far_v < 0.05, "far-off ground is unaffected: {far_v}");
}

#[test]
fn being_hurt_there_twice_bites_deeper() {
    let (mut w, i) = lone_survivor(18);
    let p = walkable_near(&w, w.agents[i].pos, 8, 0);
    traumatise(&mut w, i, p, 1000);
    let once = w.agents[i].memories.aversion(p, 1000, false);
    traumatise(&mut w, i, p, 30_000);
    let twice = w.agents[i].memories.aversion(p, 30_000, false);
    assert!(
        twice > once,
        "the second incident compounds: {once} -> {twice}"
    );
    assert!(
        twice > 0.7,
        "twice-hurt ground is close to off limits: {twice}"
    );
}

#[test]
fn trauma_fades_with_time() {
    let (mut w, i) = lone_survivor(12);
    let p = walkable_near(&w, w.agents[i].pos, 9, 0);
    traumatise(&mut w, i, p, 1000);
    let a = &w.agents[i];
    let fresh = a.memories.aversion(p, 1_000, false);
    let year = 288_000; // 10 days ≈ 100 ticks/min × 1440 min × 10
    let old = a.memories.aversion(p, 1_000 + year, false);
    assert!(
        old < fresh * 0.6,
        "a bad memory loses its grip with time: fresh {fresh} old {old}"
    );
}

#[test]
fn cautious_survivors_give_a_bad_place_a_wider_berth() {
    let (mut w, i) = lone_survivor(13);
    let home = w.agents[i].pos;
    let p = walkable_near(&w, home, 8, 0);
    traumatise(&mut w, i, p, 1000);
    // One tile short of the ordinary shadow; a cautious survivor still flinches there.
    let edge = east(p, AVERSION_RADIUS);

    let ordinary = w.agents[i].memories.aversion(edge, 1000, false);
    let cautious = w.agents[i].memories.aversion(edge, 1000, true);
    assert!(
        cautious > ordinary,
        "a cautious survivor keeps further away: cautious {cautious} ordinary {ordinary}"
    );

    // And the trait is not decoration: it changes what the scorer sees.
    let mut w2 = World::generate(13, GenParams::small());
    let ids = w2.found_party(1);
    let j = w2.agents.iter().position(|a| a.id == ids[0]).unwrap();
    traumatise(&mut w2, j, p, 1000);
    // Traits are set outright: the survivor may have been born cautious, which would make the
    // comparison meaningless.
    w2.agents[j].traits.list = vec![Trait::Sociable];
    let before = w2.agents[j].place_multiplier(edge, 1000);
    w2.agents[j].traits.list = vec![Trait::Cautious];
    let after = w2.agents[j].place_multiplier(edge, 1000);
    assert!(
        after < before,
        "the place is less attractive once they are cautious: {before} -> {after}"
    );
}

#[test]
fn a_curious_survivor_is_more_willing_to_go_back() {
    let (mut w, i) = lone_survivor(14);
    let p = walkable_near(&w, w.agents[i].pos, 8, 0);
    traumatise(&mut w, i, p, 1000);
    w.agents[i].traits.list = vec![Trait::Sociable];
    let ordinary = w.agents[i].place_multiplier(p, 1000);
    w.agents[i].traits.list = vec![Trait::Curious];
    let curious = w.agents[i].place_multiplier(p, 1000);
    assert!(
        curious > ordinary,
        "curiosity pulls them back despite themselves: {ordinary} -> {curious}"
    );
}

#[test]
fn good_times_erase_the_bad() {
    let (mut w, i) = lone_survivor(15);
    let p = walkable_near(&w, w.agents[i].pos, 8, 0);
    traumatise(&mut w, i, p, 1000);
    let scared = w.agents[i].memories.aversion(p, 1000, false);

    // Repeated good meals in the same place, slept on each time.
    let mut now = 2000;
    for k in 0..6 {
        let a = &mut w.agents[i];
        a.memories.short.push(Record {
            tick: now,
            kind: MemKind::Meal,
            subject: Subject::Place(p),
            valence: 1.0,
            salience: 0.9,
            event: None,
        });
        a.memories.consolidate(now, false, false);
        now += 28_800;
        let _ = k;
    }
    let settled = w.agents[i].memories.aversion(p, now, false);
    assert!(
        settled < scared * 0.7,
        "good experiences wear down the fear: {scared} -> {settled}"
    );
}

#[test]
fn someone_who_hurt_you_is_avoided() {
    let (mut w, i) = lone_survivor(16);
    let ids = w.found_party(2);
    let other = ids[1];
    let now = 5_000;
    {
        let a = &mut w.agents[i];
        a.memories.short.push(Record {
            tick: now,
            kind: MemKind::Company,
            subject: Subject::Person(other),
            valence: -1.0,
            salience: 1.0,
            event: None,
        });
        a.memories.consolidate(now, false, false);
    }
    let a = &w.agents[i];
    let wary = a.memories.aversion_of(other);
    assert!(wary > 0.4, "the person who hurt you is avoided: {wary}");
}

#[test]
fn a_safe_place_has_no_shadow() {
    let (w, i) = lone_survivor(17);
    let p = walkable_near(&w, w.agents[i].pos, 8, 0);
    assert_eq!(w.agents[i].memories.aversion(p, 1000, false), 0.0);
    assert_eq!(w.agents[i].place_multiplier(p, 1000), 1.0);
}
