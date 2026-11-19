//! #336 (WP-05 slice 5.4): the blind cave fish of the dark pools. A pool cavern the valley has
//! opened is a fishing spot when there is no surface water worth the going; one never stood in,
//! or walled up again, is not.

use hf_sim::fish;
use hf_sim::ids::TilePos;
use hf_sim::World;

/// A generated world with a pool cavern, and that cavern's index.
fn world_with_pool_cavern() -> (World, usize) {
    for seed in 1..40u64 {
        let w = World::opening_world(seed);
        if let Some(k) = w.grid.caverns.iter().position(|c| c.pool) {
            return (w, k);
        }
    }
    panic!("no pool cavern in seeds 1..40");
}

/// A dry, walkable tile of the cavern's floor beside its pool: somewhere to stand.
fn bank(w: &World, k: usize) -> TilePos {
    let c = &w.grid.caverns[k];
    for z in [c.centre.z - 1, c.centre.z] {
        for y in c.centre.y - c.ry..=c.centre.y + c.ry {
            for x in c.centre.x - c.rx..=c.centre.x + c.rx {
                let p = TilePos::new(x, y, z);
                if !w.grid.in_bounds(p) || !w.grid.get(p).is_water() {
                    continue;
                }
                for (a, b) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                    let q = p.offset(a, b, 0);
                    if w.grid.walkable(q) && !w.grid.get(q).is_water() {
                        return q;
                    }
                }
            }
        }
    }
    panic!("pool cavern with no bank");
}

#[test]
fn an_opened_pool_cavern_is_fished_and_a_closed_one_is_not() {
    let (mut w, k) = world_with_pool_cavern();
    let here = bank(&w, k);
    w.ensure_nav();
    // Never stood in: not a spot.
    w.grid.caverns[k].opened = false;
    assert_eq!(
        fish::cave_spot(&w, here),
        None,
        "an unopened cavern was fished"
    );
    // Opened: its pool is, a water tile of this cavern beside a bank.
    w.grid.caverns[k].opened = true;
    let spot = fish::cave_spot(&w, here).expect("an opened pool cavern is a fishing spot");
    assert!(w.grid.get(spot).is_water(), "the spot is water: {spot:?}");
    let c = w.grid.caverns[k].clone();
    assert!(
        c.contains(TilePos::new(spot.x, spot.y, c.centre.z)),
        "the spot is in the cavern's pool: {spot:?}"
    );
    // Deterministic.
    assert_eq!(fish::cave_spot(&w, here), Some(spot));
    // Walled up again: not a spot.
    w.grid.caverns[k].sealed = true;
    assert_eq!(
        fish::cave_spot(&w, here),
        None,
        "a sealed cavern was fished"
    );
}

#[test]
fn a_pool_cavern_beyond_the_reach_of_home_is_not_a_spot() {
    let (mut w, k) = world_with_pool_cavern();
    w.grid.caverns[k].opened = true;
    w.ensure_nav();
    let c = w.grid.caverns[k].clone();
    // The surface corner farthest from the cavern: more than the cave reach away.
    let far = w.grid.surface(
        if c.centre.x < w.grid.width / 2 {
            w.grid.width - 3
        } else {
            2
        },
        if c.centre.y < w.grid.height / 2 {
            w.grid.height - 3
        } else {
            2
        },
    );
    assert!(
        c.centre.dist(far) > 60,
        "fixture: the corner is not far enough"
    );
    assert_eq!(
        fish::cave_spot(&w, far),
        None,
        "a cavern beyond the reach was fished"
    );
    // And from its own bank it is a spot, so the None above is the reach, not the cavern.
    assert!(fish::cave_spot(&w, bank(&w, k)).is_some());
}
