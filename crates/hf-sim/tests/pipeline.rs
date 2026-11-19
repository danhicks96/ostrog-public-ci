//! The tick pipeline runs each stage once. Merges of branches that each added a stage beside
//! the same line once left `crafts` in it three times, so its daily pass ran thrice a day.

use std::collections::BTreeSet;

#[test]
fn every_stage_runs_once_a_tick() {
    let mut seen = BTreeSet::new();
    for (name, _) in hf_sim::systems::PIPELINE {
        assert!(seen.insert(*name), "stage {name} is in the pipeline twice");
    }
}

/// #940: a chosen Talk or Tend reads the target's live state without asking whether the
/// chooser could actually see them. A target behind a wall counts as out of sight; one in
/// plain view counts as in sight.
#[test]
fn knowing_census_counts_an_unseen_target() {
    use hf_sim::ids::TilePos;
    use hf_sim::mind::perception::can_see;
    use hf_sim::view::knowing::{Knowing, Watched};
    use hf_sim::world::grid::WorldGrid;
    use hf_sim::world::tile::Tile;
    use hf_sim::world::Material;

    let mut grid = WorldGrid::new(32, 32, -4, 4);
    let chooser = TilePos::new(5, 5, 1);
    let behind_wall = TilePos::new(5, 8, 1);
    let in_plain_view = TilePos::new(9, 5, 1);

    // A wall between the chooser and the Talk target; nothing between the chooser and the
    // Tend patient.
    grid.set(TilePos::new(5, 6, 1), Tile::solid(Material::GRANITE));

    let mut census = Knowing::default();
    census.record(Watched::Talk, can_see(&grid, chooser, behind_wall));
    census.record(Watched::Tend, can_see(&grid, chooser, in_plain_view));

    assert_eq!(
        census.talk.share_unseen(),
        1.0,
        "the walled-off target counts as unseen"
    );
    assert_eq!(
        census.tend.share_unseen(),
        0.0,
        "the patient in plain view counts as seen"
    );
}

/// #926: a loitering hour is attributed to the goal last chosen for that person, not left as
/// a bare "loitering".
#[test]
fn day_shape_attributes_a_hearth_hour() {
    use hf_sim::ids::TilePos;
    use hf_sim::mind::Activity;
    use hf_sim::time::TICKS_PER_HOUR;
    use hf_sim::view::day_shape::DayShape;
    use hf_sim::world::gen::GenParams;
    use hf_sim::World;

    let mut w = World::generate(7301, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.meta.tick = 19 * TICKS_PER_HOUR;
    w.rebuild_caches();
    let ids = w.found_party(1);
    w.meta.autoplan = false;
    let i = w.agents.iter().position(|a| a.id == ids[0]).unwrap();
    w.agents[i].activity = Activity::Loitering { ticks_left: 600 };

    let mut day_shape = DayShape::new();
    day_shape.observe_decision(ids[0], "Gather");
    for _ in 0..600u32 {
        day_shape.observe_tick(&w);
        w.meta.tick += 1;
    }

    let report = day_shape.report();
    let seen = report.get(19, "loitering:Gather");
    assert!((seen - 0.5).abs() < 1e-6, "row 19 loitering:Gather {seen}");
    let total: f64 = (0..24).map(|h| report.total(h)).sum();
    assert!(
        (total - 0.5).abs() < 1e-6,
        "the 24 rows sum to the hours observed: {total}"
    );
}
