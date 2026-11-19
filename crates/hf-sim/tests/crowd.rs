//! C55: in a crowd, with a full mind, a person keeps the nearest people in mind, not whoever
//! has the highest ids.

use hf_sim::ids::TilePos;
use hf_sim::mind::knowledge::{Known, PEOPLE_CAP};
use hf_sim::time::TICKS_PER_HOUR;
use hf_sim::world::gen::GenParams;
use hf_sim::World;

#[test]
fn in_a_crowd_the_nearest_are_kept_in_mind() {
    let mut w = World::generate(5501, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.plants.clear();
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.meta.tick = TICKS_PER_HOUR * 12;
    w.rebuild_caches();
    let ids = w.found_party(120);
    let z = w.agents[0].pos.z;
    // The looker in the middle; everyone else in rings around them, the low ids nearest (so the
    // highest ids, which a same-tick eviction by key would keep, are the farthest).
    let me = TilePos::new(30, 30, z);
    let n = ids.len();
    let mut placed = 0;
    'rings: for r in 1..20i32 {
        for dy in -r..=r {
            for dx in -r..=r {
                if dx.abs().max(dy.abs()) != r {
                    continue;
                }
                placed += 1;
                if placed >= n {
                    break 'rings;
                }
                // ids[1] nearest, ids[n-1] farthest.
                let id = ids[placed];
                if let Some(a) = w.agents.iter_mut().find(|a| a.id == id) {
                    a.pos = TilePos::new(30 + dx, 30 + dy, z);
                }
            }
        }
    }
    let looker = ids[0];
    let li = w.agents.iter().position(|a| a.id == looker).unwrap();
    w.agents[li].pos = me;
    for a in w.agents.iter_mut() {
        a.activity = hf_sim::mind::Activity::Idle;
    }
    // A full mind: places and things known from before, as someone long settled would have.
    for k in 0..(hf_sim::mind::knowledge::CAP - PEOPLE_CAP) as u32 {
        let known = if k % 2 == 0 {
            Known::Water(TilePos::new(k as i32 % 50, k as i32 / 50, z))
        } else {
            Known::Item(hf_sim::ids::ItemId(100_000 + k))
        };
        w.agents[li]
            .knowledge
            .note(known, TilePos::new(1, 1, z), 0, 0);
    }
    // Step perception through one whole stripe, so the looker looks once.
    for _ in 0..hf_sim::systems::perceive::STRIPE {
        hf_sim::systems::perceive::step(&mut w);
        w.meta.tick += 1;
    }
    let known: Vec<_> = w.agents[li]
        .knowledge
        .map
        .keys()
        .filter_map(|k| match k {
            Known::Agent(id) => Some(*id),
            _ => None,
        })
        .collect();
    assert!(
        known.len() <= PEOPLE_CAP + 1,
        "{} people in mind",
        known.len()
    );
    let far = |id| w.agents.iter().find(|a| a.id == id).unwrap().pos.dist(me);
    let farthest_known = known.iter().map(|id| far(*id)).max().unwrap();
    // Everyone nearer than the farthest one kept is kept too.
    for id in &ids[1..] {
        if far(*id) < farthest_known {
            assert!(
                known.contains(id),
                "{id:?} at {} was nearer than one kept",
                far(*id)
            );
        }
    }
    assert!(
        farthest_known <= 5,
        "kept someone {farthest_known} away while nearer people stood by"
    );
}
