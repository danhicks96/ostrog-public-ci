//! OST-1304: why most houses get no table. Steps one opening and prints, each 25 days, the cause
//! counts of `hf_sim::table_census`, plus tables built so far. Read-only.
//! Run: cargo run --release -p hf-sim --example table_census -- <seed> <days>
use hf_sim::structures::StructureKind;
use hf_sim::table_census::{census, Cause};
use hf_sim::time::TICKS_PER_DAY;
use hf_sim::World;

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let seed: u64 = a[0].parse().expect("seed");
    let days: u64 = a[1].parse().expect("days");
    let mut w = World::opening_world(seed);
    w.meta.autoplan = true;
    println!("day,houses,hollows,tables,planks,open_build,walls_open,no_spot,has_table,gate,floor_blocks,no_planks,ready");
    for day in 1..=days {
        for _ in 0..TICKS_PER_DAY {
            w.step();
        }
        if day % 25 == 0 || day == days {
            let c = census(&w);
            let tables = w
                .structures
                .iter()
                .filter(|s| s.kind == StructureKind::Table)
                .count();
            println!(
                "{day},{},{},{tables},{},{},{},{},{},{},{},{},{}",
                c.rows.len(),
                c.hollows,
                c.planks,
                c.open_build,
                c.count(Cause::WallsOpen),
                c.count(Cause::NoTableSpot),
                c.count(Cause::HasTable),
                c.count(Cause::OpenBuildGate),
                c.count(Cause::FloorBlocks),
                c.count(Cause::NoPlanks),
                c.count(Cause::ReadyToPost),
            );
        }
    }
}
