//! QA round 2026-10-26: guards for the tester's issues against c81, each against a filed issue.

use hf_sim::agents::Species;
use hf_sim::time::TICKS_PER_DAY;
use hf_sim::World;

fn opening(seed: u64) -> World {
    World::from_setup(&hf_sim::setup::WorldSetup::new(seed))
}

#[test]
fn every_wild_animal_takes_up_something_within_days() {
    // #388: on seed 7 the burrower, bats, the crawler and boars read "idle, 0 switches" for a
    // 30-day run. Animals set their activity by hand and never counted a switch, and the
    // burrower, which moves only by digging, never set an activity at all.
    let mut w = opening(7);
    let start: Vec<(hf_sim::ids::AgentId, Species)> = w
        .agents
        .iter()
        .filter(|a| a.alive && a.species != Species::Person && a.kept.is_none())
        .map(|a| (a.id, a.species))
        .collect();
    assert!(start.iter().any(|(_, s)| *s == Species::Burrower));
    assert!(start.iter().any(|(_, s)| *s == Species::Bat));
    assert!(start.iter().any(|(_, s)| *s == Species::Crawler));
    for _ in 0..3 * TICKS_PER_DAY {
        w.step();
    }
    let idle: Vec<String> = start
        .iter()
        .filter_map(|(id, s)| {
            let a = w.agents.iter().find(|a| a.id == *id && a.alive)?;
            (a.switches == 0).then(|| format!("{s:?} {} at {:?}", a.id.0, a.pos))
        })
        .collect();
    assert!(idle.is_empty(), "never took anything up: {idle:?}");
    // Nothing reads as stuck: no living animal has held one activity for the whole three days.
    let stuck: Vec<String> = w
        .agents
        .iter()
        .filter(|a| {
            a.alive
                && a.species != Species::Person
                && a.activity_ticks as u64 >= 3 * TICKS_PER_DAY - 1
        })
        .map(|a| format!("{:?} {}", a.species, a.id.0))
        .collect();
    assert!(stuck.is_empty(), "stuck: {stuck:?}");
}

#[test]
fn the_white_hart_does_not_stand_for_days() {
    // #397: on seed 11 the White Hart stood idle for a day and more. Sampled every hour for
    // ten days, no wild animal (not denned) stands idle more than half a day and an hour.
    let mut w = opening(11);
    assert!(w
        .agents
        .iter()
        .any(|a| a.species == Species::Hart && a.alive));
    let mut worst: Option<(u32, String)> = None;
    for t in 0..10 * TICKS_PER_DAY {
        w.step();
        if t % hf_sim::time::TICKS_PER_HOUR != 0 {
            continue;
        }
        for a in w.agents.iter().filter(|a| {
            a.alive
                && a.species != Species::Person
                && !a.denned
                && matches!(a.activity, hf_sim::mind::Activity::Idle)
        }) {
            if worst.as_ref().is_none_or(|w| a.activity_ticks > w.0) {
                worst = Some((
                    a.activity_ticks,
                    format!("{:?} {} at {:?}", a.species, a.id.0, a.pos),
                ));
            }
        }
    }
    let hart = w
        .agents
        .iter()
        .find(|a| a.species == Species::Hart)
        .unwrap();
    assert!(!hart.alive || hart.switches > 0);
    let (ticks, who) = worst.unwrap_or_default();
    assert!(
        ticks <= 13 * hf_sim::time::TICKS_PER_HOUR as u32,
        "{who} idle {ticks} ticks"
    );
}

fn survivors(w: &World) -> Vec<usize> {
    (0..w.agents.len())
        .filter(|&i| w.agents[i].alive && w.agents[i].species == Species::Person)
        .collect()
}

/// The arrival fixture's camp, pinned. The day-1 mood gate in
/// `the_arrival_is_a_shock_that_fades_and_the_calm_shake_it_faster` reads the community
/// members' needs outcomes within reach of the founding site, and that setting — the
/// ground, the forage, stray predators — shifts with world gen (seen with the cave
/// mouths of PR #501, which failed this test deterministically at seed 7): the same
/// fixture disease `size.rs`'s `beast` pins for the runt/giant roll (FIXLOG 2026-10-31,
/// #495). What the gate measures is the shock thought's cost, so the camp is the
/// fixture's: flat ground, a fixed orchard, pool, edge trees and beds, the provisions
/// at fixed spots, no wolves or bears at the door, the starters at the same fireside
/// (FIXLOG 2026-11-01, the PR #501 review).
fn pin_camp(w: &mut World) {
    use hf_sim::ids::TilePos;
    use hf_sim::items::{ItemKind, ItemPlace};
    use hf_sim::plants::{Plant, PlantKind};
    use hf_sim::world::{Material, Shape, Tile, TileFlags};

    let fs = w.meta.founding_site;
    let r = 12;
    let in_camp = |p: TilePos| (p.x - fs.x).abs() <= r && (p.y - fs.y).abs() <= r;
    w.flatten(fs.x - r, fs.y - r, 2 * r + 1, 2 * r + 1, fs.z);
    // What there is to eat and drink, and the ground it grows on, is the fixture's now.
    w.plants.retain(|p| !in_camp(p.pos));
    w.items
        .retain(|it| !matches!(it.place, ItemPlace::Ground(p) if in_camp(p)));
    // The orchard, some trees at the camp's edge, and the pool: food, wood and water
    // within reach, whatever the valley's draw gave this seed.
    for dy in (-9..=9).step_by(3) {
        for dx in (-9..=9).step_by(3) {
            if dx == 0 && dy == 0 {
                continue;
            }
            let kind = if (dx + dy) % 6 == 0 {
                PlantKind::BerryBush
            } else {
                PlantKind::Shrub
            };
            let p = w.grid.surface(fs.x + dx, fs.y + dy);
            w.plants.push(Plant::new(p, kind, 1.0, 0));
        }
    }
    for (dx, dy) in [
        (-10, -10),
        (-10, 0),
        (-10, 10),
        (0, -10),
        (0, 10),
        (10, -10),
        (10, 0),
        (10, 10),
    ] {
        let p = w.grid.surface(fs.x + dx, fs.y + dy);
        w.plants.push(Plant::new(p, PlantKind::Tree, 1.0, 0));
    }
    let water = Tile {
        material: Material::WATER,
        shape: Shape::Floor,
        flags: TileFlags(TileFlags::WATER),
    };
    for (dx, dy) in [(-8, 6), (-7, 6), (-8, 7), (-7, 7)] {
        w.grid
            .set(TilePos::new(fs.x + dx, fs.y + dy, fs.z + 1), water);
    }
    // The provisions the party came through with, at fixed spots.
    let s0 = w.grid.surface(fs.x + 3, fs.y + 2);
    let s1 = w.grid.surface(fs.x + 4, fs.y + 2);
    let s2 = w.grid.surface(fs.x + 2, fs.y + 3);
    let s3 = w.grid.surface(fs.x + 3, fs.y + 3);
    let s4 = w.grid.surface(fs.x + 4, fs.y + 3);
    let s5 = w.grid.surface(fs.x + 5, fs.y + 2);
    w.drop_item(ItemKind::Meal, 12, s0);
    w.drop_item(ItemKind::Berries, 25, s1);
    w.drop_item(ItemKind::Root, 20, s2);
    w.drop_item(ItemKind::GrainSheaf, 15, s3);
    w.drop_item(ItemKind::Log, 12, s4);
    w.drop_item(ItemKind::Tool, 1, s5);
    // Beds for the three who begin here: what the first week is for is recovering from
    // the arrival, not proving the camp can be built from nothing.
    for (dx, dy) in [(-2, -2), (-1, -2), (1, -2)] {
        w.place_structure(
            hf_sim::structures::StructureKind::Bed,
            TilePos::new(fs.x + dx, fs.y + dy, fs.z),
            Material::WOOD,
            false,
        );
    }
    // No wolves or bears at the door: a stray predator on day 1 is world gen's roll.
    w.agents.retain(|a| {
        a.species == Species::Person
            || !in_camp(a.pos)
            || !matches!(a.species, Species::Wolf | Species::Bear)
    });
    // The three who begin here sit by their beds at the same fireside whatever the ground
    // was.
    let seats = [(-2, -1), (-1, -1), (1, -1)];
    let members: Vec<usize> = (0..w.agents.len())
        .filter(|&i| w.agents[i].community.is_some())
        .collect();
    for (k, &i) in members.iter().enumerate() {
        let (dx, dy) = seats[k % seats.len()];
        let p = w.grid.surface(fs.x + dx, fs.y + dy);
        w.agents[i].pos = p;
        w.agents[i].from = p;
        w.agents[i].path.clear();
    }
    w.rebuild_caches();
}

#[test]
fn the_arrival_is_a_shock_that_fades_and_the_calm_shake_it_faster() {
    // #392: survivors came through the portal at mood 0 and stress 0, as if nothing had
    // happened.
    use hf_sim::mind::thoughts::ThoughtKind;
    let mut w = opening(7);
    pin_camp(&mut w);
    let now = w.meta.tick;
    let people = survivors(&w);
    assert!(people.len() >= 10);
    for &i in &people {
        assert!(
            w.agents[i].thoughts.has(ThoughtKind::Arrival),
            "{} came through with no shock",
            w.agents[i].name
        );
        assert!(
            w.agents[i].stress >= 0.05,
            "{} at stress 0",
            w.agents[i].name
        );
    }
    // The calm shake it faster: among those it lasts for by temperament alone (not the
    // easygoing), the calmer the shorter.
    let mut by_calm: Vec<(f32, u64)> = people
        .iter()
        .map(|&i| &w.agents[i])
        .filter(|a| !a.traits.has(hf_sim::mind::Trait::Easygoing))
        .map(|a| {
            let t = a
                .thoughts
                .list
                .iter()
                .find(|t| t.kind == ThoughtKind::Arrival)
                .unwrap();
            (a.genome.calm(), t.until - now)
        })
        .collect();
    by_calm.sort_by(|a, b| a.0.total_cmp(&b.0));
    for pair in by_calm.windows(2) {
        assert!(pair[1].1 <= pair[0].1, "calmer but longer: {by_calm:?}");
    }
    let (least, most) = (by_calm[0], by_calm[by_calm.len() - 1]);
    assert!(most.0 > least.0 + 0.2 && most.1 < least.1, "{by_calm:?}");
    // Mood of those in the community: lower on the first day than the same people's in the
    // same world without the shock, and back up once it has passed. The calm arm is the
    // same pinned world with the thought lifted, not a second world gen's roll of the dice.
    let members: Vec<hf_sim::ids::AgentId> = people
        .iter()
        .filter(|&&i| w.agents[i].community.is_some())
        .map(|&i| w.agents[i].id)
        .collect();
    let mean = |w: &World| {
        let m: Vec<f32> = w
            .agents
            .iter()
            .filter(|a| a.alive && members.contains(&a.id))
            .map(|a| a.mood)
            .collect();
        m.iter().sum::<f32>() / m.len().max(1) as f32
    };
    let mut calm = w.clone();
    for a in calm.agents.iter_mut() {
        a.thoughts.list.retain(|t| t.kind != ThoughtKind::Arrival);
    }
    // OST-205: needs now bite in mood, so the camp's hunger can keep the mood down for days;
    // the shock fading is read off the thought, which is what it is.
    let thought = |w: &World, arrival_only: bool| {
        let now = w.meta.tick;
        let m: Vec<f32> = w
            .agents
            .iter()
            .filter(|a| a.alive && members.contains(&a.id))
            .map(|a| {
                if arrival_only {
                    a.thoughts.sum_of(ThoughtKind::Arrival, now)
                } else {
                    a.thoughts.sum(now)
                }
            })
            .collect();
        m.iter().sum::<f32>() / m.len().max(1) as f32
    };
    let mut daily = Vec::new();
    for day in 0..7 {
        // Sampled at the hour they came through, each day.
        for _ in 0..TICKS_PER_DAY {
            w.step();
            if day == 0 {
                calm.step();
            }
        }
        if day == 0 {
            let (shocked, not) = (mean(&w), mean(&calm));
            assert!(shocked < not, "day 1: {shocked} against {not}");
            // The shock is the thought; since #415 the shocked hurry to secure food and
            // shelter and their needs fare better for it, which eats into the mood the
            // thought costs — in an unpinned camp that edge can even outweigh the thought
            // (the PR #501 failure at this gate), so the camp is pinned and this gate
            // compares the pure thought cost.
            let (shocked, not) = (thought(&w, false), thought(&calm, false));
            assert!(
                shocked < not - 0.1,
                "day 1 thoughts: {shocked} against {not}"
            );
        }
        // Other thoughts about the new camp can worsen while the arrival shock fades.
        daily.push(thought(&w, true));
    }
    assert!(daily[0] < daily[5] - 0.1, "{daily:?}");
    assert!(
        w.agents
            .iter()
            .all(|a| !a.thoughts.has(ThoughtKind::Arrival)),
        "the shock outlasted a week"
    );
}

#[test]
fn a_carving_is_not_taken_from_its_carver_halfway() {
    // #389: on seed 7 "carving the wall / work order gone" failed four times in 30 days. Not a
    // missing rock face (a carving is cut into a house wall, engrave.rs): a carving is three
    // hours of work in one step, the carver's lease on it was renewed for two only when the
    // plan moved on a step, and a second carver took the order up and lost it when the first
    // finished. Here one person takes up a carving by hand, on open ground, and holds its
    // lease every tick until it is cut.
    use hf_sim::work::{DesignationKind, ResKey};
    let mut w = World::generate(389, hf_sim::world::gen::GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.plants.clear();
    w.meta.founding_site = hf_sim::ids::TilePos::new(30, 30, 2);
    w.rebuild_caches();
    let ids = w.found_party(1);
    w.ensure_nav();
    let i = w.agents.iter().position(|a| a.id == ids[0]).unwrap();
    let at = w.agents[i].pos.offset(2, 0, 0);
    let d = hf_sim::systems::settlement::make(&mut w, DesignationKind::Engrave, at, 2);
    let did = d.id;
    w.designations.push(d);
    let planned =
        hf_sim::work::planner::plan_for(&w, ids[0], w.designations.last().unwrap()).unwrap();
    let tick = w.meta.tick;
    assert!(w.reservations.acquire_all(
        ids[0],
        &planned.keys,
        tick + planned.estimate as u64 + 600
    ));
    w.agents[i].activity = hf_sim::mind::Activity::Working { plan: planned.plan };
    let mut worked = 0u64;
    for _ in 0..TICKS_PER_DAY / 2 {
        let a = &mut w.agents[i];
        a.needs.hunger = 1.0;
        a.needs.rest = 1.0;
        let on_it = matches!(&a.activity, hf_sim::mind::Activity::Working { plan }
            if plan.designation == did);
        if !on_it {
            break;
        }
        assert_eq!(
            w.reservations.holder(ResKey::Designation(did)),
            Some(ids[0]),
            "lease lost after {worked} ticks at it"
        );
        w.step();
        worked += 1;
    }
    assert!(
        w.designations.iter().all(|d| d.id != did || d.done),
        "not cut in half a day"
    );
    assert!(worked > 2 * hf_sim::time::TICKS_PER_HOUR, "{worked}");
}
