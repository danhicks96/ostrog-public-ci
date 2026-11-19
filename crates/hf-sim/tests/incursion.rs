//! #270: what the settlement keeps draws a beast in, and it does its damage inside.

use hf_sim::events::EventKind;
use hf_sim::ids::TilePos;
use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use hf_sim::world::gen::GenParams;
use hf_sim::World;

#[test]
fn full_stores_draw_a_beast_in_and_it_eats() {
    let mut w = World::generate(2701, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.agents
        .retain(|a| a.species == hf_sim::agents::Species::Person);
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.rebuild_caches();
    w.found_party(2);
    w.meta.autoplan = false;
    w.ensure_nav();
    let home = w.communities[0].home;
    let store = w.grid.surface(home.x + 2, home.y + 2);
    w.drop_item(hf_sim::items::ItemKind::Berries, 80, store);
    let _bear = w.spawn_agent(
        hf_sim::agents::Species::Bear,
        w.grid.surface(home.x + 25, home.y),
    );
    // A summer: bears are about.
    // (By the valley's own calendar, counted from the arrival.)
    let summer = (0..20 * TICKS_PER_DAY)
        .step_by(TICKS_PER_DAY as usize)
        .map(|t| w.meta.epoch + t)
        .find(|&t| {
            hf_sim::time::Calendar::from_tick(t - w.meta.epoch).season
                == hf_sim::time::Season::Summer
        })
        .unwrap();
    let food = |w: &World| -> u32 {
        w.items
            .iter()
            .filter(|it| it.kind.is_food())
            .map(|it| it.count)
            .sum()
    };
    let before = food(&w);
    let (mut drawn, mut raided) = (false, false);
    'days: for d in 0..200u64 {
        for h in 0..24u64 {
            w.meta.tick =
                summer + (d % 4) * TICKS_PER_DAY + d / 4 * 20 * TICKS_PER_DAY + h * TICKS_PER_HOUR;
            w.events.pending.clear();
            hf_sim::creatures::step(&mut w);
            for e in &w.events.pending {
                match e.kind {
                    EventKind::Drawn { animal } => {
                        drawn = true;
                        // It walks in (whichever came: the nearest fox or bear).
                        let i = w.agents.iter().position(|a| a.id == animal).unwrap();
                        w.agents[i].pos = store;
                    }
                    EventKind::StoresRaided { .. } => raided = true,
                    _ => {}
                }
            }
            if raided {
                break 'days;
            }
        }
    }
    assert!(drawn, "the stores drew it");
    assert!(raided, "it got in");
    assert!(food(&w) < before, "and ate");
}
