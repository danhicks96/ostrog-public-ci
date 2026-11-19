//! A world with a past (Phase 17c, issue #50): the valley is simulated before the portal opens,
//! and everything it leaves (ruins, graves, roads, found tools) traces to a recorded row.

#[path = "support/cpu.rs"]
mod cpu;

use hf_sim::events::EventKind;
use hf_sim::structures::StructureKind;
use hf_sim::world::{Material, Shape};
use hf_sim::World;

fn rows(w: &World) -> Vec<&hf_sim::events::Event> {
    w.events
        .pending
        .iter()
        .filter(|e| e.tick < w.meta.epoch)
        .collect()
}

#[test]
fn history_only_from_simulation() {
    for seed in [3u64, 7, 11] {
        let w = World::opening_world(seed);
        let h = &w.history;
        assert!(
            !h.sites.is_empty(),
            "seed {seed}: the valley had people before"
        );
        let r = rows(&w);
        for s in &h.sites {
            assert!(r
                .iter()
                .any(|e| matches!(&e.kind, EventKind::OldFounded { site, .. } if *site == s.name)));
            assert!(r
                .iter()
                .any(|e| matches!(&e.kind, EventKind::OldEnded { site, .. } if *site == s.name)));
        }
        // Every grave is someone the history recorded dying.
        for g in &h.graves {
            let f = h.figures.iter().find(|f| f.id == g.figure).unwrap();
            assert!(f.died.is_some());
            assert!(
                r.iter()
                    .any(|e| matches!(&e.kind, EventKind::OldDied { who, .. } if *who == f.name)),
                "seed {seed}: {} has a grave and a row",
                f.name
            );
            assert!(w
                .structures
                .iter()
                .any(|s| s.pos == g.pos && s.kind == StructureKind::Grave));
        }
        // Every grave marker on the map is one of those.
        for s in w
            .structures
            .iter()
            .filter(|s| s.kind == StructureKind::Grave)
        {
            assert!(h.graves.iter().any(|g| g.pos == s.pos));
        }
        assert!(r.iter().all(|e| e.tick < w.meta.epoch));
    }
}

#[test]
fn found_objects_say_who_made_them_and_why_they_lie_there() {
    let mut found = 0;
    for seed in [3u64, 7, 11] {
        let w = World::opening_world(seed);
        for it in w
            .items
            .iter()
            .filter(|it| it.story.as_ref().is_some_and(|s| !s.origin.is_empty()))
        {
            let s = it.story.as_ref().unwrap();
            let site = w
                .history
                .sites
                .iter()
                .find(|x| s.origin.contains(&x.name))
                .expect("the place it was left is in the history");
            assert!(s.made < w.meta.epoch, "made before the portal");
            assert!(s.maker_name.contains(&site.name) || s.maker_name.starts_with("someone of"));
            let d = hf_sim::legend::describe(&w, it.id).unwrap();
            assert!(
                d.contains("made by") && d.contains("before the portal"),
                "{d}"
            );
            found += 1;
        }
    }
    assert!(found >= 3, "{found}");
}

#[test]
fn stone_outlasts_timber() {
    // Over many valleys: the walls still standing, per wall planned, by material.
    let (mut wood, mut wood_n, mut stone, mut stone_n) = (0, 0, 0, 0);
    for seed in 0..16u64 {
        let w = World::opening_world(seed);
        for s in &w.history.sites {
            let (gone, _) = s.ended.clone().unwrap();
            if gone < 40 {
                continue;
            }
            for (o, m) in &s.houses {
                for dy in 0..4 {
                    for dx in 0..5 {
                        let edge = dx == 0 || dy == 0 || dx == 4 || dy == 3;
                        if !edge || (dy == 3 && dx == 2) {
                            continue;
                        }
                        let stands = w.grid.get(o.offset(dx, dy, 0)).shape == Shape::Wall;
                        if *m == Material::STONE_BLOCK {
                            stone_n += 1;
                            stone += stands as i32;
                        } else {
                            wood_n += 1;
                            wood += stands as i32;
                        }
                    }
                }
            }
        }
    }
    assert!(stone_n > 0 && wood_n > 0);
    let (fs, fw) = (stone as f32 / stone_n as f32, wood as f32 / wood_n as f32);
    // 0.15, not 0.2 (C68, #211): the deity no longer answers the same hard stretch every
    // five years, so different hamlets end at different years and the sample shifted to
    // 0.96 against 0.79; the decay rule is unchanged.
    assert!(
        fs > fw + 0.15,
        "stone {fs:.2} of {stone_n}, timber {fw:.2} of {wood_n}"
    );
}

#[test]
fn the_landing_is_clear_and_the_same_seed_gives_the_same_past() {
    for seed in [3u64, 19] {
        let a = World::opening_world(seed);
        let b = World::opening_world(seed);
        assert_eq!(a.history, b.history);
        let land = a.meta.founding_site;
        for s in &a.history.sites {
            assert!(s.centre.dist(land) >= 20, "a ruin on the landing");
        }
        assert!(a.grid.walkable(land));
    }
}

#[test]
fn a_ruin_and_a_grave_explain_themselves() {
    let w = World::opening_world(11);
    let g = w.history.graves.first().expect("a grave");
    let n = hf_sim::deep::note(&w, g.pos).unwrap();
    assert!(
        n.starts_with("The grave of") && n.contains("before the portal"),
        "{n}"
    );
    // The latest site: an older one may lie under a later one's houses, and then the note
    // names the later one first (seed 11 did so once the flora changed, #286).
    let s = w.history.sites.last().unwrap();
    let n = hf_sim::deep::note(&w, s.houses[0].0).unwrap();
    assert!(n.starts_with("The ruins of") && n.contains(&s.name), "{n}");
}

#[test]
fn history_pass_budget() {
    // One world first, off the clock (#248: the first paid for lazy statics and first-touch
    // pages, and the budget's headroom on the tester's laptop was a third).
    let _ = World::opening_world(9);
    // CPU time, not wall time (#582): with the parallel parts kept on this thread, its CPU
    // clock counts all the work and none of the time a busy machine gave to others.
    let per = cpu::on_this_thread(|| {
        let t = cpu::CpuTimer::start();
        for seed in 0..4u64 {
            let w = World::opening_world(seed);
            assert_eq!(w.history.years, hf_sim::deep::HISTORY_YEARS);
        }
        t.report("four opening worlds").as_secs_f32() / 4.0
    });
    // Measured on a quiet Windows box: 1.18-1.26 s per world, ~60% of the old 2.0 s budget.
    // That budget still failed under load (a soak and the client running: 2.12 s), because a
    // wall-clock assertion in a debug build cannot tell a slowdown from a busy machine (#248).
    // 6.0 s keeps the guard - a real regression is an order of magnitude, not 20% - while
    // leaving room for a machine that is doing something else. Measured as CPU time on the
    // build box (#582): about 1.0 s per world, under load as when quiet. Kept at 6.0 s, not
    // tightened, because on Windows the helper falls back to wall time.
    assert!(per < 6.0, "an opening world with its past takes {per:.2}s");
}

#[test]
fn dry_valleys_have_a_past_too() {
    // A valley with little watered flat ground was still settled: people dug wells, and the
    // well stands in the ruin.
    let mut empty = 0;
    let mut wells = 0;
    for seed in 0..30u64 {
        let w = World::opening_world(seed);
        empty += w.history.sites.is_empty() as u32;
        for s in w.history.sites.iter().filter(|s| s.well.is_some()) {
            wells += 1;
            let p = s.well.unwrap();
            assert!(
                w.structures
                    .iter()
                    .any(|st| st.pos == p && st.kind == hf_sim::structures::StructureKind::Well),
                "{}'s well is in the ruin",
                s.name
            );
        }
    }
    assert!(empty <= 1, "{empty} of 30 valleys had no past");
    assert!(wells >= 5, "{wells}");
}

#[test]
fn a_deeper_past_holds_more_lives() {
    // Ground a hamlet left is settled again after a generation, so choosing a deeper past
    // gives more hamlets, not the same ones further back; and no two share a name.
    let count = |years: u64| -> usize {
        (0..8u64)
            .map(|seed| {
                let mut s = hf_sim::setup::WorldSetup::new(seed);
                s.history_years = years;
                let w = World::from_setup(&s);
                let mut names: Vec<_> = w.history.sites.iter().map(|s| &s.name).collect();
                let n = names.len();
                names.sort();
                names.dedup();
                assert_eq!(names.len(), n, "seed {seed}: two hamlets share a name");
                n
            })
            .sum()
    };
    let (shallow, usual, deep) = (count(50), count(150), count(300));
    assert!(
        shallow < usual && usual < deep,
        "{shallow} < {usual} < {deep}"
    );
}

/// #202 (QA 2026-10-20): a ruin's walls were drawn as crisp as a wall built today, because the
/// client could not tell them apart. `deep::ruins` names every old house wall still standing
/// and every place one fell, and never a wall the colony built.
#[test]
fn ruins_are_told_from_new_walls() {
    let mut w = World::opening_world(7);
    let ruins = hf_sim::deep::ruins(&w);
    let standing: Vec<_> = ruins.iter().filter(|r| r.1).map(|r| r.0).collect();
    assert!(!standing.is_empty(), "seed 7's old hamlet has walls up");
    assert!(ruins.iter().any(|r| !r.1), "and gaps where walls fell");
    let mut seen = std::collections::BTreeSet::new();
    assert!(
        ruins.iter().all(|r| seen.insert(r.0)),
        "a ruin built on a ruin: each tile once"
    );
    for p in &standing {
        assert_eq!(w.grid.get(*p).shape, Shape::Wall, "{p:?}");
    }
    for (p, up) in &ruins {
        if !*up {
            assert!(w.grid.walkable(*p), "{p:?}");
        }
    }
    // Build over one: it is the colony's wall now, not a ruin.
    let p = standing[0];
    w.built_of.insert(p, Material::GRANITE);
    assert!(!hf_sim::deep::ruins(&w).iter().any(|r| r.0 == p));
}
