//! #205 (QA 2026-10-20): at 500 fed people `perceive` was 86 % of the tick. A look traced a
//! sight line to every thing in range, however many shared a tile, and to every belief in range
//! before asking whether it was even missing. These count the lines one look traces.
//! #238 (FIXLOG 2026-10-28, fix-238-perceive-growth): the shared-store gather — the hearths
//! and stockpile contents within 40 of home every member knows without looking — walked every
//! structure and every item once PER LOOKER, so a look's cost grew with the days' buildings
//! and filled stockpiles. These pin the gather to once a tick and to noting exactly what the
//! per-looker full scan noted.
//! #428: the visible and foul water passes share one grid read per tile, with a reference
//! comparison for the notes and direct observations the original two passes produced.

use hf_sim::agents::Species;
use hf_sim::floods;
use hf_sim::ids::{AgentId, ItemId, StructureId, TilePos};
use hf_sim::items::{ItemKind, ItemPlace};
use hf_sim::mind::knowledge::CAP;
use hf_sim::mind::perception::{line_of_sight, sight_lines};
use hf_sim::mind::{Activity, Evidence, Knowledge, Known, Observation, ObservationKind};
use hf_sim::structures::StructureKind;
use hf_sim::systems::perceive;
use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use hf_sim::world::gen::GenParams;
use hf_sim::world::{Material, Shape, Tile, TileFlags};
use hf_sim::World;

fn meadow() -> (World, usize, TilePos) {
    let mut w = World::generate(5501, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.plants.clear();
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.meta.tick = TICKS_PER_HOUR * 12;
    w.rebuild_caches();
    let ids = w.found_party(1);
    let li = w.agents.iter().position(|a| a.id == ids[0]).unwrap();
    let z = w.agents[li].pos.z;
    let me = TilePos::new(30, 30, z);
    w.agents[li].pos = me;
    w.agents[li].activity = hf_sim::mind::Activity::Idle;
    (w, li, me)
}

/// Lines traced while the stripe comes round once, so the person looks once.
fn one_look(w: &mut World) -> u64 {
    let before = sight_lines();
    for _ in 0..hf_sim::systems::perceive::STRIPE {
        hf_sim::systems::perceive::step(w);
        w.meta.tick += 1;
    }
    sight_lines() - before
}

#[test]
fn a_heap_on_one_tile_is_one_sight_line() {
    let (mut w, li, me) = meadow();
    w.items.clear();
    let heap = me.offset(7, 2, 0);
    // Full stacks, so none merge: 150 things on the one tile.
    for _ in 0..150 {
        w.drop_item(ItemKind::Log, ItemKind::Log.max_stack(), heap);
    }
    let lines = one_look(&mut w);
    let seen = w.agents[li]
        .knowledge
        .map
        .values()
        .filter(|b| b.pos == heap)
        .count();
    assert!(seen > 100, "the heap is seen: {seen}");
    assert!(
        lines < 100,
        "150 things on one tile took {lines} sight lines"
    );
}

#[test]
fn beliefs_still_in_place_take_no_sight_line() {
    let (mut w, li, me) = meadow();
    w.items.clear();
    // 100 things, each on its own tile, all in sight.
    for k in 0..100 {
        w.drop_item(
            ItemKind::StoneBlock,
            1,
            me.offset(-5 + k % 10, -5 + k / 10, 0),
        );
    }
    one_look(&mut w);
    let known = w.agents[li].knowledge.map.len();
    assert!(known >= 100, "{known}");
    // Nothing moved: seeing them again takes a line each, and checking what is gone none.
    let lines = one_look(&mut w);
    assert!(
        lines < 160,
        "a look with nothing gone took {lines} sight lines"
    );
}

#[test]
fn gone_beliefs_on_one_tile_share_the_live_sight_line() {
    let (mut w, _, me) = meadow();
    w.items.clear();
    let heap = me.offset(7, 2, 0);
    for _ in 0..150 {
        w.drop_item(ItemKind::Log, ItemKind::Log.max_stack(), heap);
    }
    one_look(&mut w);
    w.items.clear();
    let lines = one_look(&mut w);
    assert!(
        lines < 50,
        "150 gone beliefs on one tile took {lines} sight lines"
    );
}

/// The shared-store gather must run once a tick however many people look (#238,
/// fix-238-perceive-growth): on master it ran per looker, walking every structure and every
/// item each time.
#[test]
fn shared_store_gathered_once_a_tick() {
    let mut w = World::generate(5501, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.plants.clear();
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.meta.tick = TICKS_PER_HOUR * 12;
    w.rebuild_caches();
    let ids = w.found_party(45);
    // Only the party's people, awake and idle, so one stripe has several lookers.
    w.agents.retain(|a| ids.contains(&a.id));
    for a in &mut w.agents {
        a.activity = Activity::Idle;
    }
    let stripe = (w.meta.tick % perceive::STRIPE as u64) as usize;
    let lookers = w
        .agents
        .iter()
        .enumerate()
        .filter(|(i, a)| {
            i % perceive::STRIPE as usize == stripe
                && a.alive
                && !a.activity.is_asleep()
                && a.species == Species::Person
        })
        .count();
    assert!(
        lookers >= 3,
        "want several lookers in one tick, got {lookers}"
    );
    let before = perceive::store_walks();
    perceive::step(&mut w);
    let walks = perceive::store_walks() - before;
    assert_eq!(
        walks, 1,
        "{lookers} lookers took {walks} shared-store walks"
    );
    // The next tick's lookers take one more gather, not one each.
    w.meta.tick += 1;
    let before = perceive::store_walks();
    perceive::step(&mut w);
    assert_eq!(perceive::store_walks() - before, 1);
}

/// What the shared-store path notes must be exactly what the per-looker full scan of
/// structures and items noted — noting is digest-visible, so the once-a-tick gather may not
/// drop, add, or reorder notes. The looker stands where nothing is in reach, so the shared
/// store is the only source of notes, and their mind is deliberately full (the C55
/// loaded-mind case): the evictions must match the full scan's too.
#[test]
fn shared_store_notes_match_the_full_scan() {
    let mut w = World::generate(5501, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.plants.clear();
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.meta.tick = TICKS_PER_HOUR * 12;
    w.rebuild_caches();
    let ids = w.found_party(1);
    let li = w.agents.iter().position(|a| a.id == ids[0]).unwrap();
    let z = w.agents[li].pos.z;
    w.agents.retain(|a| a.id == ids[0]);
    // A dry corner beyond every sense's reach of home's things: the only notes this look
    // can take are the shared store's.
    let me = (2..12)
        .flat_map(|y| (2..12).map(move |x| TilePos::new(x, y, z)))
        .find(|p| {
            (-8..=8)
                .flat_map(|dy| (-8..=8).map(move |dx| p.offset(dx, dy, 0)))
                .all(|q| w.grid.in_bounds(q) && !w.grid.get(q).flags.has(TileFlags::WATER))
        })
        .expect("a dry corner for the looker");
    w.agents[li].pos = me;
    w.agents[li].activity = Activity::Idle;

    // Home's hearths and stockpiles: in range and out, finished and not.
    w.place_structure(
        StructureKind::Hearth,
        TilePos::new(44, 44, z),
        Material::STONE_BLOCK,
        true,
    );
    w.place_structure(
        StructureKind::Hearth,
        TilePos::new(46, 44, z),
        Material::STONE_BLOCK,
        true,
    );
    let unfinished = w.structures.len() - 1;
    w.structures[unfinished].progress = 0.5;
    w.place_structure(
        StructureKind::Hearth,
        TilePos::new(35, 35, -8),
        Material::STONE_BLOCK,
        true,
    );
    w.place_structure(
        StructureKind::Stockpile,
        TilePos::new(45, 46, z),
        Material::WOOD,
        false,
    );
    w.place_structure(
        StructureKind::Stockpile,
        TilePos::new(47, 45, z),
        Material::WOOD,
        false,
    );
    w.place_structure(
        StructureKind::Stockpile,
        TilePos::new(36, 36, -7),
        Material::WOOD,
        false,
    );
    let pile = TilePos::new(45, 46, z);
    w.drop_item(ItemKind::Log, 5, pile);
    w.drop_item(ItemKind::StoneBlock, 3, pile);
    w.drop_item(ItemKind::Meal, 2, TilePos::new(47, 45, z));
    // Ground items that are not on a stockpile tile are nobody's shared knowledge, however
    // close to home; an item on an out-of-range pile is out of the store's reach too.
    w.drop_item(ItemKind::Log, 4, TilePos::new(48, 48, z));
    w.drop_item(ItemKind::Log, 4, TilePos::new(36, 36, -7));

    // Preconditions: nothing real within any sense's reach of the looker.
    let reach = 15;
    for it in &w.items {
        if let ItemPlace::Ground(p) = it.place {
            assert!(p.dist(me) > reach, "item in reach of the looker: {p:?}");
        }
    }
    for s in &w.structures {
        assert!(
            s.pos.dist(me) > reach,
            "structure in reach of the looker: {:?}",
            s.pos
        );
    }

    // A deliberately full mind (the C55 loaded-mind case): CAP decoys noted long ago.
    let look_tick = w.meta.tick;
    let mut decoys: Vec<(Known, TilePos)> = Vec::new();
    for i in 0..128u32 {
        decoys.push((
            Known::Item(ItemId(500_000 + i)),
            TilePos::new(47 + i as i32 % 16, 47 + i as i32 / 16, z),
        ));
    }
    for i in 0..64u32 {
        decoys.push((
            Known::Agent(AgentId(500_000 + i)),
            TilePos::new(47 + i as i32 % 16, 55 + i as i32 / 16, z),
        ));
    }
    for i in 0..96u32 {
        let p = TilePos::new(40 + i as i32 % 12, 40 + i as i32 / 12, z);
        decoys.push((Known::FoodPlant(p), p));
    }
    for i in 0..96u32 {
        decoys.push((
            Known::Structure(StructureId(600_000 + i)),
            TilePos::new(40 + i as i32 % 12, 48 + i as i32 / 12, z),
        ));
    }
    assert_eq!(decoys.len(), CAP);
    w.agents[li].knowledge.map.clear();
    for (k, p) in &decoys {
        w.agents[li].knowledge.note(*k, *p, 1, 0);
    }
    assert_eq!(w.agents[li].knowledge.map.len(), CAP);

    perceive::step(&mut w);

    // The expectation is the naive full scan itself (master's per-looker walk), noted into
    // a fresh mind primed with the same decoys.
    let cid = w.agents[li]
        .community
        .expect("the founder is a community member");
    let home = w.communities.iter().find(|c| c.id == cid).unwrap().home;
    let mut expect = Knowledge::default();
    for (k, p) in &decoys {
        expect.note(*k, *p, 1, 0);
    }
    for s in &w.structures {
        if s.kind == StructureKind::Hearth && s.progress >= 1.0 && s.pos.dist(home) <= 40 {
            expect.note(Known::Structure(s.id), s.pos, look_tick, s.kind as u16);
        }
    }
    for s in &w.structures {
        if s.kind == StructureKind::Stockpile && s.pos.dist(home) <= 40 {
            for it in &w.items {
                if it.place == ItemPlace::Ground(s.pos) {
                    expect.note(Known::Item(it.id), s.pos, look_tick, it.kind as u16);
                }
            }
        }
    }
    let got: Vec<_> = w.agents[li]
        .knowledge
        .map
        .iter()
        .map(|(k, b)| (*k, b.pos, b.seen_tick, b.tag))
        .collect();
    let want: Vec<_> = expect
        .map
        .iter()
        .map(|(k, b)| (*k, b.pos, b.seen_tick, b.tag))
        .collect();
    assert_eq!(got, want);
    // The case bit: home's finished hearth and both in-range piles' contents are known;
    // the unfinished hearth, the out-of-range pile and its item, and the loose logs by the
    // pile are not.
    let know = |k: Known| w.agents[li].knowledge.map.contains_key(&k);
    let pile_item = |p: TilePos| {
        w.items
            .iter()
            .find(|it| it.place == ItemPlace::Ground(p))
            .map(|it| it.id)
            .expect("an item on the pile")
    };
    assert!(know(Known::Structure(w.structures[0].id)));
    assert!(!know(Known::Structure(w.structures[1].id)));
    assert!(!know(Known::Structure(w.structures[2].id)));
    assert!(know(Known::Item(pile_item(pile))));
    assert!(know(Known::Item(pile_item(TilePos::new(47, 45, z)))));
    assert!(!know(Known::Item(pile_item(TilePos::new(36, 36, -7)))));
    assert!(!know(Known::Item(pile_item(TilePos::new(48, 48, z)))));
}

#[test]
fn one_look_reads_the_water_scan_square_at_most_once() {
    let (mut w, li, _) = meadow();
    let before = perceive::water_reads();
    for _ in 0..perceive::STRIPE {
        perceive::step(&mut w);
        w.meta.tick += 1;
    }
    let reads = perceive::water_reads() - before;
    let sp = w.agents[li].species_def();
    let sr = sp.smell_radius().max(sp.sight_radius(1.0).min(6));
    let budget = (2 * sr + 1).pow(2) as u64;
    assert!(
        reads <= budget,
        "one human look took {reads} water-grid reads, budget {budget}"
    );
}

fn reference_water_look(w: &mut World, li: usize) {
    let tick = w.meta.tick;
    let me = w.agents[li].pos;
    let sp = w.agents[li].species_def();
    let lit = hf_sim::underground::light_at(w, me);
    let light = if lit >= 1.0 && hf_sim::time::Calendar::is_night(tick) {
        0.4 + 0.3 * hf_sim::time::Calendar::moon(tick)
    } else {
        lit
    };
    let sight = sp.sight_radius(light);
    let smell = sp.smell_radius();
    let d = |p: TilePos| (p.x - me.x).abs().max((p.y - me.y).abs());
    let sees = |p: TilePos| p.z == me.z && d(p) <= sight && line_of_sight(&w.grid, me, p);
    let near = |p: TilePos, r: i32| p.z == me.z && d(p) <= r;

    let mut water = None;
    let wr = sight.min(6);
    'scan: for dy in -wr..=wr {
        for dx in -wr..=wr {
            let p = me.offset(dx, dy, 0);
            if w.grid.in_bounds(p) && w.grid.get(p).flags.has(TileFlags::WATER) && sees(p) {
                water = Some(p);
                break 'scan;
            }
        }
    }
    let water_tag = water.map(|p| {
        let tag = if w.waters.foul.contains_key(&p) {
            floods::TAG_WATER_FOUL
        } else {
            floods::quality(w, p)
                .map(|q| q.tag())
                .unwrap_or(floods::TAG_WATER_FRESH)
        };
        (p, tag)
    });

    let sr = smell.max(wr);
    let mut tainted: Vec<&floods::Body> = Vec::new();
    let mut foul_notes = Vec::new();
    for dy in -sr..=sr {
        for dx in -sr..=sr {
            let p = me.offset(dx, dy, 0);
            if !w.grid.in_bounds(p)
                || !w.grid.get(p).flags.has(TileFlags::WATER)
                || !w.waters.foul.contains_key(&p)
            {
                continue;
            }
            let smelled = smell > 0 && near(p, smell);
            if !smelled && !sees(p) {
                continue;
            }
            if let Some(b) = floods::body_at(w, p) {
                if tainted.iter().any(|t| std::ptr::eq(*t, b)) {
                    continue;
                }
                tainted.push(b);
            }
            foul_notes.push(p);
        }
    }

    let a = &mut w.agents[li];
    if let Some((p, tag)) = water_tag {
        a.knowledge.note(Known::Water(p), p, tick, tag);
        let tile = w.grid.get(p);
        let kind = if tile.flags.has(TileFlags::DEEP) || tile.flags.has(TileFlags::FULL) {
            ObservationKind::WaterDepth
        } else {
            ObservationKind::Wetness
        };
        a.learning.observe(Observation {
            tick,
            pos: p,
            kind,
            strength: if kind == ObservationKind::WaterDepth {
                1.0
            } else {
                0.55
            },
            evidence: Evidence::Direct,
            event: None,
            full: false,
            reading: None,
        });
    }
    for p in foul_notes {
        a.knowledge
            .note(Known::Water(p), p, tick, floods::TAG_WATER_FOUL);
    }
    // The perceive step's tail hands the picked tile to the E16 hazard hook (PR #476), which
    // records only the personal danger reaction; the fused scan stays the single water
    // observation. Model the tail call here so the oracle stays exact.
    if let Some((p, _)) = water_tag {
        hf_sim::systems::hazards::observe_water(w, li, p);
    }
}

fn water_beliefs(w: &World) -> Vec<(Known, TilePos, u64, u16)> {
    w.agents[0]
        .knowledge
        .map
        .iter()
        .filter(|(k, _)| matches!(k, Known::Water(_)))
        .map(|(k, b)| (*k, b.pos, b.seen_tick, b.tag))
        .collect()
}

fn looked_against_reference(mut w: World) -> World {
    let mut reference = w.clone();
    reference_water_look(&mut reference, 0);
    perceive::step(&mut w);
    assert_eq!(water_beliefs(&w), water_beliefs(&reference));
    assert_eq!(
        w.agents[0].learning.observations,
        reference.agents[0].learning.observations
    );
    w
}

fn set_water(w: &mut World, p: TilePos, flags: u8) {
    w.grid.set(
        p,
        Tile {
            material: Material::WATER,
            shape: Shape::Floor,
            flags: TileFlags(flags),
        },
    );
}

fn water_equivalence_fixture(first_flags: u8) -> World {
    let mut w = World::generate(5510, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.plants.clear();
    w.items.clear();
    w.structures.clear();
    w.agents.clear();
    w.communities.clear();
    let me = TilePos::new(30, 30, 2);
    w.meta.founding_site = me;
    w.spawn_agent(Species::Person, me);
    w.agents[0].activity = Activity::Idle;

    set_water(&mut w, me.offset(-2, -2, 0), TileFlags::WATER);
    w.grid.set(
        me.offset(-1, -1, 0),
        Tile {
            material: Material::STONE_BLOCK,
            shape: Shape::Wall,
            flags: TileFlags::NONE,
        },
    );
    set_water(&mut w, me.offset(0, -1, 0), first_flags);
    for p in [me.offset(-2, 2, 0), me.offset(-1, 2, 0), me.offset(0, 2, 0)] {
        set_water(&mut w, p, TileFlags::WATER);
    }
    for p in [me.offset(4, 4, 0), me.offset(5, 4, 0)] {
        set_water(&mut w, p, TileFlags::WATER);
    }
    w.rebuild_caches();
    floods::survey(&mut w);
    for p in [
        me.offset(-2, 2, 0),
        me.offset(-1, 2, 0),
        me.offset(4, 4, 0),
        me.offset(5, 4, 0),
    ] {
        w.waters.foul.insert(p, 12);
    }
    w.meta.tick = 10 * TICKS_PER_DAY + 12 * TICKS_PER_HOUR;
    w
}

#[test]
fn fused_water_scan_matches_the_two_scan_reference() {
    for (flags, kind, strength) in [
        (TileFlags::WATER, ObservationKind::Wetness, 0.55),
        (
            TileFlags::WATER | TileFlags::DEEP,
            ObservationKind::WaterDepth,
            1.0,
        ),
    ] {
        let w = looked_against_reference(water_equivalence_fixture(flags));
        let a = &w.agents[0];
        let me = a.pos;
        let tick = w.meta.tick;
        let blocked = me.offset(-2, -2, 0);
        let first = me.offset(0, -1, 0);
        assert!(!a.knowledge.map.contains_key(&Known::Water(blocked)));
        let first_belief = &a.knowledge.map[&Known::Water(first)];
        assert_eq!(
            (first_belief.pos, first_belief.seen_tick, first_belief.tag),
            (first, tick, floods::TAG_WATER_FRESH)
        );
        // The E16 hazard hook adds no second observation (PR #476, FIXLOG 2026-10-31, c146):
        // the fused scan above stays the single water observation of the picked tile.
        assert_eq!(a.learning.observations.len(), 1);
        let observation = a.learning.observations[0];
        assert_eq!(observation.pos, first);
        assert_eq!(observation.tick, tick);
        assert_eq!(observation.kind, kind);
        assert_eq!(observation.strength, strength);
        assert_eq!(observation.evidence, Evidence::Direct);
        assert!(observation.full);

        let foul: Vec<TilePos> = a
            .knowledge
            .map
            .iter()
            .filter(|(k, b)| matches!(k, Known::Water(_)) && b.tag == floods::TAG_WATER_FOUL)
            .map(|(_, b)| b.pos)
            .collect();
        assert_eq!(foul.len(), 2, "one note per foul body: {foul:?}");
        assert!(foul.contains(&me.offset(-2, 2, 0)));
        assert!(foul.contains(&me.offset(4, 4, 0)));
    }
}

#[test]
fn fused_water_scan_preserves_smelled_foul_behind_a_wall() {
    let mut w = World::generate(5511, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.plants.clear();
    w.items.clear();
    w.structures.clear();
    w.agents.clear();
    w.communities.clear();
    let me = TilePos::new(30, 30, 2);
    w.meta.founding_site = me;
    let id = w.spawn_agent(Species::Person, me);
    let murrow = hf_sim::species::by_key("murrow").expect("the murrow people");
    hf_sim::species::give(&mut w, id, murrow, None);
    w.agents[0].activity = Activity::Idle;
    for y in 24..=36 {
        w.grid.set(
            TilePos::new(28, y, 2),
            Tile {
                material: Material::STONE_BLOCK,
                shape: Shape::Wall,
                flags: TileFlags::NONE,
            },
        );
    }
    for p in [
        TilePos::new(25, 29, 2),
        TilePos::new(26, 29, 2),
        TilePos::new(25, 30, 2),
        TilePos::new(26, 30, 2),
    ] {
        set_water(&mut w, p, TileFlags::WATER);
    }
    w.rebuild_caches();
    floods::survey(&mut w);
    w.waters.foul.insert(TilePos::new(25, 29, 2), 12);
    w.waters.foul.insert(TilePos::new(26, 29, 2), 12);
    w.meta.tick = 10 * TICKS_PER_DAY + 12 * TICKS_PER_HOUR;

    let w = looked_against_reference(w);
    let beliefs = water_beliefs(&w);
    assert_eq!(beliefs.len(), 1);
    assert_eq!(
        beliefs[0],
        (
            Known::Water(TilePos::new(25, 29, 2)),
            TilePos::new(25, 29, 2),
            w.meta.tick,
            floods::TAG_WATER_FOUL,
        )
    );
    assert!(w.agents[0].learning.observations.is_empty());
}
