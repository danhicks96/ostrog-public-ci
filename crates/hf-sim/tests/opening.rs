//! The opening: ten survivors, three begin, seven are scattered. See docs/design-emergence.md.

use hf_sim::opening::{SurvivorSlot, SurvivorState};
use hf_sim::world_state::World;

fn fresh(seed: u64) -> World {
    World::opening_world(seed)
}

/// Same drift check the CLI prints: recorded ledger vs what is actually in the world.
fn ledger_drift(w: &World) -> Vec<String> {
    let live = hf_sim::economy::live_counts(&w.items);
    w.ledger
        .expected()
        .iter()
        .filter(|(k, v)| live.get(*k).copied().unwrap_or(0) != **v)
        .map(|(k, v)| {
            format!(
                "{k}: live {} expected {v}",
                live.get(k).copied().unwrap_or(0)
            )
        })
        .collect()
}

#[test]
fn ten_are_generated_and_three_begin() {
    let w = fresh(1);
    assert_eq!(w.survivor_pool.len(), 10, "ten survivors in the pool");

    let starting = w
        .agents
        .iter()
        .filter(|a| {
            a.alive && a.species == hf_sim::agents::Species::Person && a.community.is_some()
        })
        .count();
    assert_eq!(starting, 3, "three survivors begin at the egress point");

    let missing = w
        .survivor_pool
        .iter()
        .filter(|s| matches!(s.state, SurvivorState::Lost { found: false, .. }))
        .count();
    assert_eq!(missing, 7, "seven are scattered across the world");
}

#[test]
fn the_pool_is_seeded() {
    let a = fresh(7).survivor_pool.clone();
    let b = fresh(7).survivor_pool.clone();
    assert_eq!(format!("{a:?}"), format!("{b:?}"), "same seed, same pool");
}

#[test]
fn the_missing_are_placed_on_the_map_and_far_from_the_start() {
    let w = fresh(3);
    let start = w.meta.founding_site;
    let mut seen = 0;
    for slot in &w.survivor_pool {
        if let SurvivorState::Lost { agent, .. } = slot.state {
            seen += 1;
            let at = w.agents.iter().find(|a| a.id == agent).unwrap().pos;
            assert!(
                w.grid.walkable(at),
                "a missing survivor stands on walkable ground: {at}"
            );
            assert!(
                at.dist(start) >= 40,
                "scattered at least 40 tiles from the egress point, got {}",
                at.dist(start)
            );
        }
    }
    assert_eq!(seen, 7);
}

#[test]
fn rerolling_one_slot_changes_only_that_slot() {
    let mut rerolls = [0u32; 10];
    let base = hf_sim::opening::pool(9, &rerolls);
    rerolls[4] = 1;
    let rolled = hf_sim::opening::pool(9, &rerolls);
    for (i, (a, b)) in base.iter().zip(rolled.iter()).enumerate() {
        if i == 4 {
            continue;
        }
        assert_eq!(
            format!("{a:?}"),
            format!("{b:?}"),
            "slot {i} must not move when slot 4 is rerolled"
        );
    }
    assert_eq!(rolled[4].rerolls, 1);
}

#[test]
fn the_three_who_begin_are_the_chosen_ones() {
    // The wizard's choice is three slot indices; the world must start exactly those.
    let chosen = [0u8, 5, 9];
    let w = World::opening_world_with(2, &chosen, &[0; hf_sim::opening::PARTY]);
    let starting: Vec<&SurvivorSlot> = w
        .survivor_pool
        .iter()
        .filter(|s| matches!(s.state, SurvivorState::Starting))
        .collect();
    assert_eq!(starting.len(), 3);
    let mut got: Vec<u8> = starting.iter().map(|s| s.index).collect();
    got.sort_unstable();
    let mut want: Vec<u8> = chosen.to_vec();
    want.sort_unstable();
    assert_eq!(got, want, "exactly the chosen three begin");
    let humans = w
        .agents
        .iter()
        .filter(|a| {
            a.alive && a.species == hf_sim::agents::Species::Person && a.community.is_some()
        })
        .count();
    assert_eq!(
        humans, 3,
        "three survivors together, not counting the lost or the wildlife"
    );
}

#[test]
fn the_opening_keeps_the_ledger_balanced() {
    let w = fresh(5);
    let drift = ledger_drift(&w);
    assert!(
        drift.is_empty(),
        "provisions are recorded, not conjured: {drift:?}"
    );
}

#[test]
fn the_pool_survives_a_round_trip() {
    let w = fresh(6);
    let before: Vec<u8> = w.survivor_pool.iter().map(|s| s.index).collect();
    let states_before: Vec<String> = w
        .survivor_pool
        .iter()
        .map(|s| format!("{:?}", s.state))
        .collect();
    let back = hf_sim::save::snapshot::Snapshot::capture(&w).restore();
    let after: Vec<u8> = back.survivor_pool.iter().map(|s| s.index).collect();
    let states_after: Vec<String> = back
        .survivor_pool
        .iter()
        .map(|s| format!("{:?}", s.state))
        .collect();
    assert_eq!(before, after, "the pool round-trips");
    assert_eq!(
        states_before, states_after,
        "including who is missing and where"
    );
}

#[test]
fn the_card_is_the_survivor_you_get() {
    // The setup screen shows a card; Beginning must produce exactly that person.
    let mut w = World::generate(41, Default::default());
    let fs = w.meta.founding_site;
    let card = hf_sim::opening::identity(41, 3, 2);
    let slot = SurvivorSlot {
        index: 3,
        rerolls: 2,
        state: SurvivorState::Starting,
        past_events: Vec::new(),
    };
    let id = w.spawn_survivor(&slot, fs);
    let a = w.agents.iter().find(|x| x.id == id).unwrap();
    // The name matters as much as the numbers: the card, the chronicle and whoever goes looking
    // for them all say it, and they must be saying it about the same person.
    assert_eq!(
        a.name, card.name,
        "the survivor is called what the card called them"
    );
    assert_eq!(
        a.name,
        hf_sim::opening::slot_name(41, 3, 2),
        "and what anyone asking about this slot would name them"
    );
    assert_eq!(a.traits.list, card.traits.list, "same traits as the card");
    assert_eq!(a.genome, card.genome, "same body and looks");
    for (got, want) in a.skills.states.iter().zip(card.skills.states.iter()) {
        assert_eq!(got.level, want.level, "same skill levels as the card");
    }
    // And rerolling the card rerolls the survivor.
    let rolled = hf_sim::opening::identity(41, 3, 3);
    assert_ne!(
        format!("{:?}", card.genome),
        format!("{:?}", rolled.genome),
        "a reroll really is someone else"
    );
}
