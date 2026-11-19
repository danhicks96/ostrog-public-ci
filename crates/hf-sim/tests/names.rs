//! #862: two living people (or a newcomer and someone who died) must never share a name.

use hf_sim::names::{person_name, unique_person_name};

#[test]
fn a_forced_collision_is_rerolled_to_something_else_and_stays_deterministic() {
    let seed = 4242;
    let id = 7;
    let natural = person_name(seed, id);
    // Everyone already answers to the natural roll: it must reroll to something different.
    let rerolled = unique_person_name(seed, id, |n| n == natural);
    assert_ne!(rerolled, natural);
    // Same seed, same id, same taken rule: the same reroll comes back every time.
    let again = unique_person_name(seed, id, |n| n == natural);
    assert_eq!(rerolled, again);
}

#[test]
fn nothing_taken_keeps_the_plain_roll() {
    let seed = 99;
    let id = 3;
    assert_eq!(
        unique_person_name(seed, id, |_| false),
        person_name(seed, id)
    );
}

#[test]
fn spawned_people_in_one_valley_never_share_a_name() {
    let mut w = hf_sim::World::generate(55, hf_sim::world::gen::GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.meta.founding_site = hf_sim::ids::TilePos::new(30, 30, 2);
    w.rebuild_caches();
    let pos = hf_sim::ids::TilePos::new(30, 30, 2);
    for _ in 0..40 {
        w.spawn_agent(hf_sim::agents::Species::Person, pos);
    }
    let names: Vec<&str> = w
        .agents
        .iter()
        .filter(|a| a.species == hf_sim::agents::Species::Person)
        .map(|a| a.name.as_str())
        .collect();
    let mut sorted = names.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(sorted.len(), names.len(), "{names:?}");
}
