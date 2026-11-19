//! #1361: the ecology census showed browse, grazers, predators and crows every 30 days, but
//! never the trees — a run could strip the woods around a settlement and the census would not
//! show it. This guards `systems::settlement::tree_census`, the counter the example now prints.

use hf_sim::ids::TilePos;
use hf_sim::plants::{Plant, PlantKind};
use hf_sim::systems::settlement::tree_census;
use hf_sim::world::gen::GenParams;
use hf_sim::World;

fn valley() -> World {
    let mut w = World::generate(1, GenParams::default());
    w.flatten(2, 2, 160, 160, 1);
    w.plants.clear();
    w.meta.founding_site = TilePos::new(80, 80, 2);
    w.found_party(4);
    w.rebuild_caches();
    w
}

#[test]
fn tree_census_counts_mature_young_and_stumps_near_and_far() {
    let mut w = valley();
    let home = w.communities[0].home;

    // Near home (within the planner's 40-tile felling radius): one mature tree, one young
    // tree, one stump.
    w.plants.push(Plant::new(
        home.offset(5, 0, 0),
        PlantKind::Tree,
        1.0, // growth >= 0.95: mature
        0,
    ));
    w.plants.push(Plant::new(
        home.offset(6, 0, 0),
        PlantKind::Tree,
        0.3, // young
        0,
    ));
    w.plants
        .push(Plant::new(home.offset(7, 0, 0), PlantKind::Stump, 1.0, 0));

    // Far from every home (outside 40 tiles): one more mature tree, uncounted in the "near
    // home" columns.
    w.plants
        .push(Plant::new(home.offset(200, 0, 0), PlantKind::Tree, 1.0, 0));

    let c = tree_census(&w);
    assert_eq!(
        c.mature, 2,
        "both mature trees, near and far, count valley-wide"
    );
    assert_eq!(c.young, 1);
    assert_eq!(c.stumps, 1);
    assert_eq!(
        c.mature_near_home, 1,
        "only the near tree is within the felling radius"
    );
    assert_eq!(c.young_near_home, 1);
    assert_eq!(c.stumps_near_home, 1);
}

#[test]
fn tree_census_ignores_dead_plants_and_non_tree_kinds() {
    let mut w = valley();
    let home = w.communities[0].home;

    let mut dead_tree = Plant::new(home.offset(3, 0, 0), PlantKind::Tree, 1.0, 0);
    dead_tree.alive = false;
    w.plants.push(dead_tree);
    // A fallen log is dead wood (`PlantKind::grows` is false for it), not a standing tree.
    w.plants.push(Plant::new(
        home.offset(3, 1, 0),
        PlantKind::FallenLog,
        1.0,
        0,
    ));
    // A shrub is not a tree at all.
    w.plants
        .push(Plant::new(home.offset(3, 2, 0), PlantKind::Shrub, 1.0, 0));

    let c = tree_census(&w);
    assert_eq!(c.mature, 0);
    assert_eq!(c.young, 0);
    assert_eq!(c.stumps, 0);
}
