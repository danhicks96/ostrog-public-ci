//! #308 (epic #508): a hidden potential drawn once for each of the ten, woken only at the portal
//! stone, and the rock-sense it gives: a woken carrier will not cut a face that would bring the
//! roof down (resonance.rs, docs/design-magic-psy-tech.md).

use hf_sim::events::EventKind;
use hf_sim::ids::TilePos;
use hf_sim::mind::techniques::{Learned, Technique, Techniques};
use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use hf_sim::work::{Designation, DesignationKind};
use hf_sim::world::gen::GenParams;
use hf_sim::world::{Material, Shape, Tile, TileFlags};
use hf_sim::World;

/// A flat meadow at ground z1, with a shaft at (40,30) down to z-2 and, at z-2, a dug room
/// x 42..42+w, y 28..33 opening off the shaft and roofed (z-1) with `roof`. The cut to make
/// is the next column's middle, (42+w, 30), widening the room.
fn mine(seed: u64, width: i32, roof: Material) -> World {
    shallow_mine(seed, width, roof, 1)
}

/// The same, with the ground at `ground` (0 puts the room two levels under the surface).
fn shallow_mine(seed: u64, width: i32, roof: Material, ground: i32) -> World {
    let mut w = World::generate(seed, GenParams::small());
    w.flatten(2, 2, 60, 60, ground);
    w.plants.clear();
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.meta.tick = 7 * TICKS_PER_DAY + TICKS_PER_HOUR * 8;
    let set = |w: &mut World, p: TilePos, shape: Shape, m: Material| {
        w.grid.set(
            p,
            Tile {
                material: m,
                shape,
                flags: TileFlags::NONE,
            },
        )
    };
    set(
        &mut w,
        TilePos::new(40, 30, ground + 1),
        Shape::StairsDown,
        Material::STONE_BLOCK,
    );
    for z in -1..=ground {
        set(
            &mut w,
            TilePos::new(40, 30, z),
            Shape::StairsBoth,
            Material::GRANITE,
        );
    }
    set(
        &mut w,
        TilePos::new(40, 30, -2),
        Shape::StairsUp,
        Material::GRANITE,
    );
    set(&mut w, TilePos::new(41, 30, -2), Shape::Open, Material::AIR);
    w.dug.insert(TilePos::new(41, 30, -2));
    for x in 42..42 + width {
        for y in 28..33 {
            let p = TilePos::new(x, y, -2);
            set(&mut w, p, Shape::Open, Material::AIR);
            w.dug.insert(p);
            set(&mut w, p.offset(0, 0, 1), Shape::Wall, roof);
        }
    }
    for y in 28..33 {
        set(&mut w, TilePos::new(42 + width, y, -1), Shape::Wall, roof);
    }
    w.rebuild_caches();
    w
}

/// Order the cut that widens an 8-wide room (x 42..50) by a column.
fn dig_wall(w: &mut World, cid: hf_sim::ids::CommunityId) {
    let id = w.alloc_designation_id();
    w.designations.push(Designation {
        id,
        kind: DesignationKind::Mine,
        pos: TilePos::new(50, 30, -2),
        requirements: vec![],
        work_left: 20 * 15,
        work_total: 20 * 15,
        max_workers: 1,
        priority: 9,
        created: w.tick(),
        failures: 0,
        done: false,
        community: Some(cid),
        owner: None,
    });
}

use hf_sim::resonance::{self, Potential, Resonance};

fn woken() -> Option<Resonance> {
    Some(Resonance {
        potential: Potential::RockSense,
        sat: resonance::WAKES_AFTER,
        woke: Some(1),
        refused: 0,
        first_manifestation: None,
    })
}

#[test]
fn about_two_in_ten_carry_one_drawn_once_and_the_same_every_time() {
    let mut carry = 0;
    for seed in 0..100u64 {
        for slot in 0..10u8 {
            let a = resonance::draw(seed, slot, 0);
            assert_eq!(a, resonance::draw(seed, slot, 0), "never drawn differently");
            carry += usize::from(a.is_some());
        }
    }
    assert!((150..250).contains(&carry), "{carry} of 1000");
    // The ten of an opening carry what their slots drew, and none of it is woken.
    let w = World::opening_world(3);
    let held: Vec<_> = w
        .agents
        .iter()
        .filter_map(|a| a.resonance.clone())
        .collect();
    assert!(held.iter().all(|r| r.woke.is_none() && r.sat == 0));
    assert!(!w.events.pending.iter().any(|e| matches!(
        e.kind,
        EventKind::CameBackUnsettled { .. } | EventKind::WouldNotCut { .. }
    )));
}

#[test]
fn it_wakes_only_in_a_carrier_after_sitting_by_the_stone() {
    let mut w = mine(1, 8, Material::LIMESTONE);
    let ids = w.found_party(2);
    let (c, n) = (
        w.agents.iter().position(|a| a.id == ids[0]).unwrap(),
        w.agents.iter().position(|a| a.id == ids[1]).unwrap(),
    );
    w.agents[c].resonance = Some(Resonance {
        potential: Potential::RockSense,
        sat: 0,
        woke: None,
        refused: 0,
        first_manifestation: None,
    });
    w.agents[n].resonance = None;
    w.agents[c].needs.rest = 1.0;
    for k in 0..resonance::WAKES_AFTER {
        assert!(
            w.agents[c].resonance.as_ref().unwrap().woke.is_none(),
            "not after {k}"
        );
        resonance::sat_by_stone(&mut w, c);
        resonance::sat_by_stone(&mut w, n);
    }
    assert!(w.agents[c].resonance.as_ref().unwrap().woke.is_some());
    assert!(w.agents[c].needs.rest < 1.0, "it tires them");
    assert!(
        w.agents[n].resonance.is_none(),
        "one who carries nothing gains nothing"
    );
    let lines: Vec<_> = w
        .events
        .pending
        .iter()
        .filter(|e| matches!(e.kind, EventKind::CameBackUnsettled { .. }))
        .collect();
    assert_eq!(lines.len(), 1);
    let line = hf_sim::view::narrate::line(&w, lines[0]);
    assert!(line.contains("could not say why"), "{line}");
}

#[test]
fn a_woken_carrier_will_not_cut_a_face_that_would_bring_the_roof_down() {
    let mut refused = 0;
    for seed in 0..8u64 {
        let mut w = mine(seed, 8, Material::LIMESTONE);
        w.meta.autoplan = false;
        let ids = w.found_party(1);
        let i = w.agents.iter().position(|a| a.id == ids[0]).unwrap();
        let cid = w.agents[i].community.unwrap();
        w.agents[i].techniques = Techniques::default();
        w.agents[i].resonance = woken();
        dig_wall(&mut w, cid);
        w.ensure_nav();
        for _ in 0..TICKS_PER_HOUR * 8 {
            // Kept rested: a worn-out carrier feels nothing (7.4; the test after the next).
            w.agents[i].needs.rest = 1.0;
            w.step();
            assert!(
                !w.last_events
                    .iter()
                    .any(|e| matches!(e.kind, EventKind::Collapse { .. })),
                "seed {seed}: the roof came down"
            );
            refused += w
                .last_events
                .iter()
                .filter(|e| matches!(e.kind, EventKind::WouldNotCut { .. }))
                .count();
        }
        assert!(
            w.grid.get(TilePos::new(50, 30, -2)).shape.is_solid(),
            "seed {seed}: the face is left uncut"
        );
    }
    assert!(refused >= 8, "each of them balked at it: {refused}");
}

#[test]
fn the_sense_adds_nothing_where_the_roof_is_held_or_they_know_shoring() {
    let mut w = mine(2, 8, Material::LIMESTONE);
    let ids = w.found_party(1);
    let i = w.agents.iter().position(|a| a.id == ids[0]).unwrap();
    w.agents[i].resonance = woken();
    let at = TilePos::new(50, 30, -2);
    assert!(!resonance::balks(&mut w, i, at, &[]), "nothing unheld");
    w.agents[i]
        .techniques
        .learn(Technique::Shoring, Learned::Past, 0.6, 0);
    assert!(
        !resonance::balks(&mut w, i, at, &[TilePos::new(46, 30, -2)]),
        "one who knows shoring props instead"
    );
    w.agents[i].techniques = Techniques::default();
    w.agents[i].resonance.as_mut().unwrap().woke = None;
    assert!(
        !resonance::balks(&mut w, i, at, &[TilePos::new(46, 30, -2)]),
        "not woken: nothing"
    );
}

#[test]
fn the_hidden_state_survives_a_save() {
    let mut w = World::opening_world(5);
    let i = w.agents.iter().position(|a| a.community.is_some()).unwrap();
    w.agents[i].resonance = woken();
    let bytes = hf_sim::save::encode(&hf_sim::save::snapshot::Snapshot::capture(&w)).unwrap();
    let back = hf_sim::save::decode(&bytes).unwrap().restore();
    for (a, b) in w.agents.iter().zip(back.agents.iter()) {
        assert_eq!(a.resonance, b.resonance);
    }
}

/// WP-07 slice 7.1: the hidden state is shown nowhere. A carrier's inspect readout, look,
/// feeling line and because-line are exactly what they are without the potential, dormant or
/// woken, so a later view change cannot leak it without failing here.
#[test]
fn nothing_a_viewer_reads_about_a_person_shows_the_potential() {
    let mut w = World::opening_world(3);
    let i = w
        .agents
        .iter()
        .position(|a| a.alive && a.species == hf_sim::agents::Species::Person)
        .expect("a person");
    let read = |w: &World, i: usize| {
        let a = &w.agents[i];
        (
            format!("{:?}", hf_sim::view::experience::inspect(a)),
            format!("{:?}", hf_sim::view::look::look(w, a)),
            hf_sim::view::why::feeling(w, a),
            hf_sim::view::why::because(w, a),
        )
    };
    w.agents[i].resonance = None;
    let plain = read(&w, i);
    for state in [
        Some(Resonance {
            potential: Potential::RockSense,
            sat: 0,
            woke: None,
            refused: 0,
            first_manifestation: None,
        }),
        woken(),
    ] {
        w.agents[i].resonance = state;
        assert_eq!(
            read(&w, i),
            plain,
            "a viewer can tell the potential is there"
        );
    }
    // Nor does any readout name it.
    let all = format!("{:?}", read(&w, i)).to_lowercase();
    for word in ["resonance", "rocksense", "rock-sense", "potential"] {
        assert!(!all.contains(word), "{word} shows in what a viewer reads");
    }
}

#[test]
fn first_manifestation_is_ambiguous_personal_and_saved() {
    let mut w = mine(9, 8, Material::LIMESTONE);
    let ids = w.found_party(2);
    let lore = w.agents.iter().position(|a| a.id == ids[0]).unwrap();
    let stranger = w.agents.iter().position(|a| a.id == ids[1]).unwrap();
    for i in [lore, stranger] {
        w.agents[i].resonance = woken();
        w.agents[i].techniques = Techniques::default();
    }
    w.agents[lore]
        .techniques
        .learn(Technique::ThinLore, Learned::Studied, 0.6, 0);
    let at = TilePos::new(50, 30, -2);
    let unheld = [TilePos::new(46, 30, -2)];
    assert!(resonance::balks(&mut w, lore, at, &unheld));
    assert!(resonance::balks(&mut w, stranger, at, &unheld));
    assert_eq!(
        w.agents[lore]
            .resonance
            .as_ref()
            .unwrap()
            .first_manifestation,
        Some(resonance::Manifestation {
            at,
            interpretation: resonance::Interpretation::StoneRemembers,
        })
    );
    assert_eq!(
        w.agents[stranger]
            .resonance
            .as_ref()
            .unwrap()
            .first_manifestation
            .unwrap()
            .interpretation,
        resonance::Interpretation::UnexplainedDread
    );
    let first = w.agents[lore]
        .resonance
        .as_ref()
        .unwrap()
        .first_manifestation;
    assert!(resonance::balks(
        &mut w,
        lore,
        TilePos::new(49, 30, -2),
        &unheld
    ));
    assert_eq!(
        w.agents[lore]
            .resonance
            .as_ref()
            .unwrap()
            .first_manifestation,
        first
    );

    let bytes = hf_sim::save::encode(&hf_sim::save::snapshot::Snapshot::capture(&w)).unwrap();
    let back = hf_sim::save::decode(&bytes).unwrap().restore();
    assert_eq!(
        back.agents[lore]
            .resonance
            .as_ref()
            .unwrap()
            .first_manifestation,
        first,
        "the person's interpretation survives save/load"
    );
}

#[test]
fn the_sense_tires_fails_when_worn_out_and_comes_back_with_sleep() {
    // WP-07 7.4: cost, failure and recovery.
    let mut w = mine(4, 8, Material::LIMESTONE);
    let ids = w.found_party(1);
    let i = w.agents.iter().position(|a| a.id == ids[0]).unwrap();
    w.agents[i].techniques = Techniques::default();
    w.agents[i].resonance = woken();
    let at = TilePos::new(50, 30, -2);
    let unheld = [TilePos::new(46, 30, -2)];
    // Rested, it stops them, and that costs them.
    w.agents[i].needs.rest = 0.62;
    assert!(resonance::balks(&mut w, i, at, &unheld));
    let after = w.agents[i].needs.rest;
    assert!((0.62 - after - resonance::COST).abs() < 1e-5, "{after}");
    // A second firing tires them past the point where it comes at all.
    assert!(resonance::balks(&mut w, i, at, &unheld));
    assert!(w.agents[i].needs.rest < resonance::FAILS_BELOW);
    // Worn out, the same deadly face: nothing stops them, nothing is recorded, nothing is spent.
    let refused = w.agents[i].resonance.as_ref().unwrap().refused;
    let rest = w.agents[i].needs.rest;
    assert!(
        !resonance::balks(&mut w, i, at, &unheld),
        "a worn-out carrier feels nothing"
    );
    assert_eq!(w.agents[i].resonance.as_ref().unwrap().refused, refused);
    assert_eq!(w.agents[i].needs.rest, rest);
    // Slept: it is back.
    w.agents[i].needs.rest = 0.95;
    assert!(
        resonance::balks(&mut w, i, at, &unheld),
        "rested, it comes back"
    );
}

#[test]
fn a_worn_out_carrier_cuts_the_deadly_face_like_anyone() {
    // The run-level failure: with the sense gone, the ordinary chance of bringing the roof down
    // applies; across seeds some tired carrier cuts the face.
    let mut cut = 0;
    for seed in 0..8u64 {
        let mut w = mine(seed, 8, Material::LIMESTONE);
        w.meta.autoplan = false;
        let ids = w.found_party(1);
        let i = w.agents.iter().position(|a| a.id == ids[0]).unwrap();
        let cid = w.agents[i].community.unwrap();
        w.agents[i].techniques = Techniques::default();
        w.agents[i].resonance = woken();
        w.agents[i].needs.rest = 0.35;
        dig_wall(&mut w, cid);
        w.ensure_nav();
        for _ in 0..TICKS_PER_HOUR * 4 {
            w.step();
            assert!(
                !w.last_events
                    .iter()
                    .any(|e| matches!(e.kind, EventKind::WouldNotCut { .. })),
                "seed {seed}: the sense came to a worn-out carrier"
            );
            if !w.grid.get(TilePos::new(50, 30, -2)).shape.is_solid() {
                cut += 1;
                break;
            }
        }
    }
    assert!(cut >= 1, "no worn-out carrier cut the face");
}
