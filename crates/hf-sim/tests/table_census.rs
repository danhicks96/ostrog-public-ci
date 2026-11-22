//! OST-1304: the table census names the first thing that stops a house's table, on a house set up
//! by hand in an otherwise empty opening world.
use hf_sim::houses;
use hf_sim::table_census::{census, Cause};
use hf_sim::world::{Material, Tile};
use hf_sim::World;

#[test]
fn census_names_walls_then_floor_then_planks() {
    let mut w = World::opening_world(5);
    w.designations.clear();
    w.items.clear();
    for c in &mut w.communities {
        c.house_sites.clear();
        c.house_plans.clear();
    }
    let origin = w.communities[0].home;
    w.communities[0].house_sites.push(origin);
    let plan = houses::plan(&w, origin);

    // Walls not standing: the first test of step 6b fails.
    for (q, _) in houses::walls(&plan.floor) {
        w.grid.set(q, Tile::AIR);
    }
    let c = census(&w);
    assert_eq!(c.rows.len(), 1);
    assert_eq!(c.count(Cause::WallsOpen), 1);

    // Walls up, ground below the floor not wood: two or more floor tiles want an order, which
    // is what blocks the table (`posted < 2`).
    for (q, _) in houses::walls(&plan.floor) {
        w.grid.set(q, Tile::solid(Material::WOOD));
    }
    for p in &plan.floor {
        w.grid.set(p.offset(0, 0, -1), Tile::solid(Material::SOIL));
    }
    let c = census(&w);
    assert_eq!(c.count(Cause::FloorBlocks), 1);
    assert!(c.rows[0].floor_wanting >= 2);

    // Floor laid, no planks anywhere: only the planks are missing.
    for p in &plan.floor {
        w.grid.set(p.offset(0, 0, -1), Tile::solid(Material::WOOD));
    }
    let c = census(&w);
    assert_eq!(c.planks, 0);
    assert_eq!(c.count(Cause::NoPlanks), 1, "rows: {:?}", c.rows[0].cause);
}

/// The census agrees with the live posting branch (step 6b of `systems::settlement`): where it
/// says floor tiles block the table no table order is posted, and where nothing stops it one is.
#[test]
fn census_agrees_with_what_the_settlement_step_posts() {
    use hf_sim::structures::StructureKind;
    use hf_sim::time::TICKS_PER_HOUR;
    use hf_sim::work::DesignationKind;
    let table_orders = |w: &World, at| {
        w.designations
            .iter()
            .filter(|d| {
                d.pos == at
                    && matches!(
                        d.kind,
                        DesignationKind::BuildStructure(StructureKind::Table, _)
                    )
            })
            .count()
    };
    let setup = |floor_laid: bool| {
        let mut w = World::opening_world(5);
        w.designations.clear();
        w.items.clear();
        for c in &mut w.communities {
            c.house_sites.clear();
            c.house_plans.clear();
        }
        let origin = w.communities[0].home;
        w.communities[0].house_sites.push(origin);
        let plan = houses::plan(&w, origin);
        for (q, _) in houses::walls(&plan.floor) {
            w.grid.set(q, Tile::solid(Material::WOOD));
        }
        for p in &plan.floor {
            let ground = if floor_laid {
                Material::WOOD
            } else {
                Material::SOIL
            };
            w.grid.set(p.offset(0, 0, -1), Tile::solid(ground));
        }
        w.meta.autoplan = true;
        w.meta.tick = (w.meta.tick / TICKS_PER_HOUR + 1) * TICKS_PER_HOUR;
        let gap = houses::bed_gap(&w, w.communities[0].id);
        let spot = houses::layout(&plan, gap).table[0];
        (w, spot)
    };

    let (mut w, spot) = setup(false);
    assert_eq!(census(&w).count(Cause::FloorBlocks), 1);
    hf_sim::systems::settlement::step(&mut w);
    assert_eq!(table_orders(&w, spot), 0, "floor tiles block the table");

    // With the floor laid the planner's own earlier steps (stockpile, hearth, roof) post enough
    // open orders in the same pass to close the gate before 6b is reached, and the census, taken
    // after that pass, agrees that the next pass posts no table.
    let (mut w, spot) = setup(true);
    assert_eq!(
        census(&w).count(Cause::NoPlanks),
        1,
        "before the pass nothing is open"
    );
    hf_sim::systems::settlement::step(&mut w);
    assert_eq!(table_orders(&w, spot), 0, "the gate closed within the pass");
    let c = census(&w);
    assert_eq!(c.rows.len(), 1, "a house is counted once");
    // Here another order already sits on the table spot (6b counts any order there as the slot
    // taken), so the census says HasTable; with the spot free it would say OpenBuildGate. Either
    // way it does not say the table is ready to post.
    assert_eq!(c.count(Cause::ReadyToPost), 0);
    assert_eq!(c.count(Cause::HasTable) + c.count(Cause::OpenBuildGate), 1);
    assert!(c.open_build >= 4);
    w.meta.tick += TICKS_PER_HOUR;
    hf_sim::systems::settlement::step(&mut w);
    assert_eq!(
        table_orders(&w, spot),
        0,
        "open orders still hold the table back"
    );
}
