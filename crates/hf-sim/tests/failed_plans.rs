//! #553: plans chosen only to fail, again and again. On seed 3 (c147) 61,654 plans failed in
//! 200 days against 10,227 choices. Each test builds the case the run showed and checks what a
//! normal person does: not walk to the bush a neighbour is picking, not chase someone walking
//! off, not plan a walk to a ledge nobody can climb to, follow the goat they mean to tame, and
//! not set out again at once on a walk that found no way.

use hf_sim::agents::Species;
use hf_sim::ids::TilePos;
use hf_sim::items::ItemKind;
use hf_sim::mind::knowledge::Known;
use hf_sim::mind::Activity;
use hf_sim::plants::{Plant, PlantKind};
use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use hf_sim::work::{DesignationKind, ResKey};
use hf_sim::world::gen::GenParams;
use hf_sim::world::{Material, Shape, Tile, TileFlags};
use hf_sim::World;

/// A flat meadow, a party of `n` founded at (20, 20) on day 30 mid-morning, no food lying
/// about, everyone content but for what the test sets.
fn meadow(seed: u64, n: usize) -> (World, Vec<usize>) {
    let mut w = World::generate(seed, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.plants.clear();
    w.meta.founding_site = TilePos::new(20, 20, 2);
    w.meta.tick = TICKS_PER_DAY * 30 + 10 * TICKS_PER_HOUR;
    w.meta.autoplan = false;
    w.rebuild_caches();
    let ids = w.found_party(n);
    w.items.retain(|it| !it.kind.is_food());
    w.designations.clear();
    w.ensure_nav();
    let ix: Vec<usize> = ids
        .iter()
        .map(|id| w.agents.iter().position(|a| a.id == *id).unwrap())
        .collect();
    let at = w.grid.surface(20, 20);
    for &i in &ix {
        let a = &mut w.agents[i];
        a.pos = at;
        a.from = at;
        a.path.clear();
        a.activity = Activity::Idle;
        a.idle_ticks = 0;
        a.needs.hunger = 1.0;
        a.needs.thirst = 1.0;
        a.needs.rest = 1.0;
        a.needs.social = 1.0;
        a.needs.warmth = 1.0;
    }
    (w, ix)
}

fn bush(w: &mut World, who: &[usize], x: i32, y: i32) -> TilePos {
    let p = w.grid.surface(x, y);
    w.plants.push(Plant::new(p, PlantKind::BerryBush, 1.0, 1));
    let now = w.meta.tick;
    for &i in who {
        w.agents[i]
            .knowledge
            .note(Known::FoodPlant(p), p, now, PlantKind::BerryBush as u16);
    }
    p
}

/// The plant or item someone has set out for, if any.
fn food_target(a: &hf_sim::agents::Agent) -> Option<String> {
    let act = match &a.activity {
        Activity::Going { then, .. } => &**then,
        other => other,
    };
    match act {
        Activity::Foraging { plant, .. } => Some(format!("plant {plant}")),
        Activity::Eating { item, .. } => Some(format!("item {item:?}")),
        _ => None,
    }
}

#[test]
fn a_neighbour_does_not_walk_to_the_bush_another_is_already_going_to_pick() {
    // Seed 3, 50 days: 1,315 of 4,124 "nothing to gather" were a second picker arriving at a
    // bush another had reached first. Two hungry people of one camp, a bush five tiles off and
    // another fifteen: the second goes to the other bush, not after the first.
    let (mut w, ix) = meadow(5531, 2);
    for &i in &ix {
        w.agents[i].needs.hunger = 0.3;
    }
    let near = bush(&mut w, &ix, 25, 20);
    let far = bush(&mut w, &ix, 35, 20);
    hf_sim::systems::decide::step(&mut w);
    let a = food_target(&w.agents[ix[0]]);
    let b = food_target(&w.agents[ix[1]]);
    assert!(
        a.is_some() && b.is_some(),
        "both set out for food: {a:?} {b:?}"
    );
    assert_ne!(a, b, "two set out for the same bush ({near}, {far})");
}

#[test]
fn the_last_of_a_stack_is_not_walked_to_by_two_and_a_haulers_stack_by_none() {
    // Seed 3, 200 days: 10,855 "food gone" (another ate it first) and 1,809 "food moved" (a
    // hauler carried it off) among the meals set out for.
    let (mut w, ix) = meadow(5532, 2);
    for &i in &ix {
        w.agents[i].needs.hunger = 0.3;
    }
    let at = w.grid.surface(24, 20);
    w.drop_item(ItemKind::Berries, 1, at);
    let id = w.items.last().unwrap().id;
    let now = w.meta.tick;
    for &i in &ix {
        w.agents[i]
            .knowledge
            .note(Known::Item(id), at, now, ItemKind::Berries as u16);
    }
    bush(&mut w, &ix, 35, 20);
    hf_sim::systems::decide::step(&mut w);
    let want = Some(format!("item {id:?}"));
    let going: Vec<_> = ix.iter().map(|&i| food_target(&w.agents[i])).collect();
    assert_eq!(
        going.iter().filter(|t| **t == want).count(),
        1,
        "one of them goes for the single berry: {going:?}"
    );

    // The same stack, claimed by a hauler's work: nobody hungry sets out to eat it.
    let (mut w, ix) = meadow(5533, 2);
    w.agents[ix[0]].needs.hunger = 0.3;
    let at = w.grid.surface(24, 20);
    w.drop_item(ItemKind::Berries, 5, at);
    let id = w.items.last().unwrap().id;
    let now = w.meta.tick;
    w.agents[ix[0]]
        .knowledge
        .note(Known::Item(id), at, now, ItemKind::Berries as u16);
    bush(&mut w, &ix[..1], 35, 20);
    let hauler = w.agents[ix[1]].id;
    assert!(w
        .reservations
        .acquire_all(hauler, &[ResKey::Item(id)], now + TICKS_PER_HOUR));
    w.agents[ix[1]].activity = Activity::Loitering { ticks_left: 500 };
    hf_sim::systems::decide::step(&mut w);
    let t = food_target(&w.agents[ix[0]]);
    assert!(t.is_some(), "they set out for food");
    assert_ne!(t, Some(format!("item {id:?}")), "not the hauler's stack");
}

#[test]
fn nobody_sets_out_to_talk_to_someone_walking_off() {
    // Seed 3, 200 days: 8,029 talks failed as "they left", most with the other walking away.
    for walking in [false, true] {
        let (mut w, ix) = meadow(5534, 2);
        let (a, b) = (ix[0], ix[1]);
        w.agents[a].needs.social = 0.0;
        let there = w.grid.surface(28, 20);
        w.agents[b].pos = there;
        w.agents[b].from = there;
        w.agents[b].activity = Activity::Loitering { ticks_left: 5000 };
        if walking {
            let to = w.grid.surface(45, 20);
            w.agents[b].activity = Activity::Going {
                to,
                then: Box::new(Activity::Idle),
            };
            assert!(w.path_agent_to(b, to));
        }
        let (bid, now) = (w.agents[b].id, w.meta.tick);
        w.agents[a]
            .knowledge
            .note(Known::Agent(bid), there, now, Species::Person as u16);
        hf_sim::systems::decide::step(&mut w);
        let talks = match &w.agents[a].activity {
            Activity::Going { then, .. } => {
                matches!(**then, Activity::Talking { with, .. } if with == bid)
            }
            _ => false,
        };
        assert_eq!(
            talks,
            !walking,
            "lonely, they go to talk to one standing about, not one walking off (walking {walking}): {:?}",
            w.agents[a].activity.name()
        );
    }
}

#[test]
fn an_order_on_a_ledge_nobody_can_climb_to_is_not_planned() {
    // Seed 3, 200 days: 4,224 walks to fetch for walls, roofs and beds failed "cannot reach":
    // the planner took a walkable site as the place to stand without asking whether a way
    // led there, so the same order was planned and failed at the walk again and again.
    let (mut w, ix) = meadow(5535, 1);
    let i = ix[0];
    // A pillar of rock three levels high; its top is ground to stand on that nobody can reach.
    for z in 2..=4 {
        w.grid
            .set(TilePos::new(40, 40, z), Tile::solid(Material::STONE_BLOCK));
    }
    let top = TilePos::new(40, 40, 5);
    w.rebuild_caches();
    w.nav = None;
    w.ensure_nav();
    assert!(w.grid.walkable(top), "the pillar's top is ground");
    assert!(
        !w.nav.as_ref().unwrap().reach(w.agents[i].pos, top),
        "and no way leads up"
    );
    let here = w.agents[i].pos;
    w.drop_item(ItemKind::StoneBlock, 2, here);
    let id = w.items.last().unwrap().id;
    let now = w.meta.tick;
    w.agents[i]
        .knowledge
        .note(Known::Item(id), here, now, ItemKind::StoneBlock as u16);
    let who = w.agents[i].id;
    let wall = |w: &mut World, at: TilePos| {
        hf_sim::systems::settlement::make(
            w,
            DesignationKind::BuildWall(Material::STONE_BLOCK),
            at,
            5,
        )
    };
    let ledge = wall(&mut w, top);
    assert!(
        hf_sim::work::planner::plan_for(&w, who, &ledge).is_none(),
        "a wall on the ledge is not planned"
    );
    let open = w.grid.surface(30, 30);
    let ground = wall(&mut w, open);
    assert!(
        hf_sim::work::planner::plan_for(&w, who, &ground).is_some(),
        "a wall on open ground is"
    );
}

#[test]
fn a_tamer_follows_the_goat_from_where_the_order_was_given() {
    // Seed 1, 200 days: 1,162 tamings failed "not at the site": the order stands where the
    // goat was when it was given, the tamer walked to where it was when they set out, and the
    // work asked that they stand at the order's tile.
    let (mut w, ix) = meadow(5536, 1);
    let i = ix[0];
    let order_at = w.grid.surface(30, 20);
    let goat = w.spawn_agent(Species::Goat, order_at);
    let mut d = hf_sim::systems::settlement::make(&mut w, DesignationKind::Tame(goat), order_at, 5);
    d.requirements = vec![];
    let did = d.id;
    w.designations.push(d);
    // The goat has wandered ten tiles off since.
    let g = w.agents.iter().position(|a| a.id == goat).unwrap();
    let off = w.grid.surface(30, 30);
    w.agents[g].pos = off;
    w.agents[g].from = off;
    let who = w.agents[i].id;
    let planned = {
        let d = w.designations.iter().find(|d| d.id == did).unwrap();
        hf_sim::work::planner::plan_for(&w, who, d).expect("taming is planned")
    };
    w.agents[i].activity = Activity::Working { plan: planned.plan };
    let mut tamed = false;
    let mut gave_up = Vec::new();
    for _ in 0..TICKS_PER_HOUR {
        w.step();
        for e in &w.last_events {
            if let hf_sim::events::EventKind::PlanFailed { agent, reason, .. } = &e.kind {
                if *agent == who {
                    gave_up.push(reason.clone());
                }
            }
        }
        if w.agents[g].kept.is_some() {
            tamed = true;
            break;
        }
    }
    assert!(tamed, "the goat was tamed; gave up: {gave_up:?}");
    assert!(
        !gave_up.iter().any(|r| r == "not at the site"),
        "{gave_up:?}"
    );
}

fn spate_tile() -> Tile {
    Tile {
        material: Material::WATER,
        shape: Shape::Floor,
        flags: TileFlags(TileFlags::WATER | TileFlags::FULL),
    }
}

#[test]
fn a_walk_through_water_over_the_head_is_not_set_out_on_or_chosen_again_at_once() {
    // Seed 2, 200 days: 17,682 walks to drink failed "path blocked" at their first step, into
    // water over the head. The walks searched together for the tick's decisions (#428) did
    // not refuse such a way as a single walk does (#418), and the same walk was chosen again
    // half a minute after each failure.
    let (mut w, ix) = meadow(5537, 2);
    let (a, b) = (ix[0], ix[1]);
    // The stream in spate from one side of the map to the other at y 30 and 31.
    w.flatten(0, 0, 64, 64, 1);
    for y in 30..=31 {
        for x in 0..=63 {
            w.grid.set(TilePos::new(x, y, 2), spate_tile());
        }
    }
    w.rebuild_caches();
    w.nav = None;
    w.ensure_nav();
    let near = w.grid.surface(30, 27);
    let far = w.grid.surface(30, 34);
    assert!(
        hf_sim::par::path_between(&w, near, far).is_none(),
        "no way through water over the head"
    );
    // Lonely, with the one they would talk to across the spate.
    w.agents[a].pos = near;
    w.agents[a].from = near;
    w.agents[a].needs.social = 0.0;
    w.agents[b].pos = far;
    w.agents[b].from = far;
    w.agents[b].activity = Activity::Loitering { ticks_left: 50_000 };
    let (bid, now) = (w.agents[b].id, w.meta.tick);
    w.agents[a]
        .knowledge
        .note(Known::Agent(bid), far, now, Species::Person as u16);
    let target = TilePos::new(far.x, far.y + 1, far.z);
    hf_sim::systems::decide::step(&mut w);
    assert!(
        w.agents[a].path.is_empty() && !matches!(w.agents[a].activity, Activity::Going { .. }),
        "they do not set out: {:?}",
        w.agents[a].activity.name()
    );
    assert!(
        w.agents[a]
            .work_cooldowns
            .contains_key(&hf_sim::systems::decide::no_way_key(target)),
        "and remember there is no way there now"
    );
    // At their next thoughts within the hour they do not try the same walk again.
    for _ in 0..(TICKS_PER_HOUR / 2) {
        w.meta.tick += 1;
        hf_sim::systems::decide::step(&mut w);
        assert!(
            !matches!(&w.agents[a].activity, Activity::Going { to, .. } if *to == target),
            "set out again on the walk with no way"
        );
    }
}
