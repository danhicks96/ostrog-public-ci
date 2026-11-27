//! Construction as a visible process (issue #48): a house goes up in stages the feed can
//! tell; a roof is held by walls, laid from the walls inward, bears snow by how well it was
//! made, falls when its wall goes or the snow is too much, hurts whoever is under it, and is
//! mended.

use hf_sim::agents::Species;
use hf_sim::building::Roof;
use hf_sim::events::EventKind;
use hf_sim::ids::TilePos;
use hf_sim::systems::weather::Sky;
use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use hf_sim::world::gen::GenParams;
use hf_sim::world::{Material, Tile};
use hf_sim::World;

fn flat(seed: u64) -> World {
    let mut w = World::generate(seed, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.plants.clear();
    w.agents.retain(|a| a.species == Species::Person);
    w.meta.tick = TICKS_PER_DAY + 8 * TICKS_PER_HOUR;
    w.rebuild_caches();
    w
}

#[test]
fn a_house_goes_up_in_stages() {
    let mut ok = 0;
    for seed in [3u64, 7, 11] {
        let mut w = World::opening_world(seed);
        w.meta.autoplan = true;
        let mut stages: Vec<(u64, String)> = Vec::new();
        let mut open_sky_seen = false;
        let mut notes = std::collections::BTreeSet::new();
        for _ in 0..8 * TICKS_PER_DAY {
            w.step();
            if w.meta.tick.is_multiple_of(TICKS_PER_HOUR) {
                if let Some(o) = w.communities[0].house_sites.first() {
                    if let Some(n) = hf_sim::building::house_note(&w, *o) {
                        let key = if n.contains("going up") {
                            "going up"
                        } else if n.contains("walls up, roof") {
                            "roofing"
                        } else {
                            "roofed"
                        };
                        notes.insert(key);
                    }
                }
            }
            for e in &w.last_events {
                if let EventKind::HouseStage { stage, .. } = &e.kind {
                    stages.push((e.tick, stage.clone()));
                    assert!(!hf_sim::view::narrate::line(&w, e).is_empty());
                }
            }
            // Between "walls up" and "roofed", the inside is open to the sky.
            if stages.last().map(|s| s.1 == "walls up").unwrap_or(false) {
                if let Some(o) = w.communities[0].house_sites.last() {
                    let inner = hf_sim::systems::settlement::house_interior(&w, *o);
                    if inner
                        .iter()
                        .any(|p| !hf_sim::systems::hazards::indoors(&w, *p))
                    {
                        open_sky_seen = true;
                    }
                }
            }
        }
        let first: Vec<&str> = stages.iter().map(|s| s.1.as_str()).collect();
        let m = first.iter().position(|s| *s == "marked out");
        let wu = first.iter().position(|s| *s == "walls up");
        let r = first.iter().position(|s| *s == "roofed");
        if let (Some(m), Some(wu), Some(r)) = (m, wu, r) {
            assert!(m < wu && wu < r, "seed {seed}: {first:?}");
            ok += 1;
        }
        assert!(
            open_sky_seen,
            "seed {seed}: walls stood before the roof did"
        );
        assert!(
            notes.contains("roofing") && notes.contains("roofed"),
            "seed {seed}: the house's note followed it: {notes:?}"
        );
    }
    assert_eq!(
        ok, 3,
        "every seed got a house marked out, walled and roofed"
    );
}

#[test]
fn roofs_are_laid_from_the_walls_and_do_not_fall_in_ordinary_building() {
    for seed in [3u64, 7] {
        let mut w = World::opening_world(seed);
        w.meta.autoplan = true;
        for _ in 0..12 * TICKS_PER_DAY {
            w.step();
            for e in &w.last_events {
                assert!(
                    !matches!(e.kind, EventKind::RoofFell { .. }),
                    "seed {seed}: a roof fell during ordinary building"
                );
            }
        }
        assert!(
            w.roofs.len() >= 20,
            "seed {seed}: roofs went up: {}",
            w.roofs.len()
        );
        for p in w.roofs.keys() {
            assert!(hf_sim::building::supported(&w, *p), "{p:?} hangs unheld");
        }
    }
}

/// A long house (11×7 outside): the middle of its roof is held only by the long walls.
fn long_house(w: &mut World, quality: f32) -> (i32, i32, i32) {
    let (x0, y0, gz) = (20, 20, 1);
    w.build_house(x0, y0, 11, 7, gz, Material::WOOD, Material::WOOD);
    for r in w.roofs.values_mut() {
        r.quality = quality;
    }
    w.ensure_nav();
    (x0, y0, gz + 1)
}

#[test]
fn a_roof_falls_when_its_walls_go_and_hurts_who_is_under_it() {
    let mut w = flat(12);
    let (x0, y0, z) = long_house(&mut w, 0.7);
    let mid = TilePos::new(x0 + 5, y0 + 3, z);
    assert!(hf_sim::building::supported(&w, mid));
    let who = w.spawn_agent(Species::Person, mid);
    // Both long walls taken down (as by fire): the middle is five tiles from the end walls.
    for x in x0 + 1..x0 + 10 {
        w.grid.set(TilePos::new(x, y0, z), Tile::AIR);
        w.grid.set(TilePos::new(x, y0 + 6, z), Tile::AIR);
    }
    let mut fell = None;
    for _ in 0..2 * TICKS_PER_HOUR {
        if let Some(a) = w.agents.iter_mut().find(|a| a.id == who) {
            a.pos = mid;
            a.from = mid;
            a.path.clear();
        }
        w.step();
        for e in &w.last_events {
            if let EventKind::RoofFell { hurt, because, .. } = &e.kind {
                fell = Some((hurt.clone(), because.clone()));
            }
        }
        if fell.is_some() {
            break;
        }
    }
    let (hurt, because) = fell.expect("the unheld roof came down");
    assert!(because.contains("nothing left to hold it"), "{because}");
    assert!(hurt.contains(&who), "it caught the one under it");
    assert!(!w.roofs.contains_key(&mid));
    let a = w.agents.iter().find(|a| a.id == who).unwrap();
    assert!(a.body.wounds.iter().any(|_| true));
    // The end walls still hold the tiles next to them.
    assert!(w.roofs.contains_key(&TilePos::new(x0 + 1, y0 + 3, z)));
}

fn snow_on(w: &mut World, hours: u64) -> Option<String> {
    let mut sky = w.weather;
    sky.sky = Sky::Snow;
    sky.temperature_c = -6.0;
    w.force_weather = Some(sky);
    for _ in 0..hours * TICKS_PER_HOUR {
        w.step();
        for e in &w.last_events {
            if let EventKind::RoofFell { because, .. } = &e.kind {
                return Some(because.clone());
            }
        }
    }
    None
}

#[test]
fn a_poor_roof_gives_way_under_snow_and_a_good_one_does_not() {
    let mut w = flat(13);
    long_house(&mut w, 0.1);
    let fell = snow_on(&mut w, 30).expect("a poorly made roof gave way");
    assert!(fell.contains("snow"), "{fell}");

    let mut w = flat(13);
    long_house(&mut w, 1.0);
    assert_eq!(
        snow_on(&mut w, 30),
        None,
        "a well-made roof bore the same snow"
    );
    assert!(
        w.roofs.values().all(|r: &Roof| r.snow > 20.0),
        "and carries it"
    );
}

#[test]
fn fire_takes_a_thatch_roof() {
    let mut w = flat(14);
    let (x0, y0, z) = long_house(&mut w, 0.7);
    let mut dry = w.weather;
    dry.sky = Sky::Clear;
    dry.temperature_c = 25.0;
    w.force_weather = Some(dry);
    let p = TilePos::new(x0 + 4, y0 + 3, z);
    assert!(w.ignite(p, "in a test"));
    let mut burned = false;
    for _ in 0..TICKS_PER_DAY {
        w.step();
        if w.last_events
            .iter()
            .any(|e| matches!(&e.kind, EventKind::Burned { what, .. } if what == "roof"))
        {
            burned = true;
        }
        if w.fires.is_empty() {
            break;
        }
    }
    assert!(burned, "the roof burned");
    assert!(!w.roofs.contains_key(&p));
}

#[test]
fn a_fallen_roof_is_mended() {
    let mut w = World::opening_world(7);
    w.meta.autoplan = true;
    // Until the first house has its roof.
    let mut roofed_at = None;
    for _ in 0..8 * TICKS_PER_DAY {
        w.step();
        if w.last_events
            .iter()
            .any(|e| matches!(&e.kind, EventKind::HouseStage { stage, .. } if stage == "roofed"))
        {
            roofed_at = Some(w.meta.tick);
            break;
        }
    }
    roofed_at.expect("a house was roofed");
    // Once that roof is whole (within ten days: how fast depends on what else the builders
    // have to do), half of it is lost.
    let o = w.communities[0].house_sites[0];
    let mut inner = hf_sim::systems::settlement::house_interior(&w, o);
    for _ in 0..10 * TICKS_PER_DAY {
        w.step();
        inner = hf_sim::systems::settlement::house_interior(&w, o);
        if inner.iter().all(|p| w.roofs.contains_key(p)) {
            break;
        }
    }
    let had = inner.iter().filter(|p| w.roofs.contains_key(p)).count();
    assert_eq!(had, inner.len());
    for p in inner.iter().take(inner.len() / 2) {
        w.roofs.remove(p);
    }
    // Mended. Only if the household grows the house while its roof is down (seed 7 does once
    // the flora changed, #286: "added to" within the hour, then "walls up", and the whole roof,
    // old room and new, laid again by hour 31) is the stage it reaches "roofed" instead. Either
    // way it is said only when every tile of the house is under its roof.
    let mut added = false;
    let mut mended = false;
    for _ in 0..4 * TICKS_PER_DAY {
        w.step();
        let said: Vec<String> = w
            .last_events
            .iter()
            .filter_map(|e| match &e.kind {
                EventKind::HouseStage { stage, .. } => Some(stage.clone()),
                _ => None,
            })
            .collect();
        added |= said.iter().any(|s| s == "added to");
        if said
            .iter()
            .any(|s| s == "mended" || (added && s == "roofed"))
        {
            let now = hf_sim::systems::settlement::house_interior(&w, o);
            assert!(
                now.iter().all(|p| w.roofs.contains_key(p)),
                "said only with the roof whole"
            );
            mended = true;
            break;
        }
    }
    assert!(mended, "someone mended the roof");
    assert!(inner.iter().all(|p| w.roofs.contains_key(p)));
}

#[test]
fn settlements_build_in_more_than_one_style() {
    let mut roofs = std::collections::BTreeSet::new();
    for seed in 0..12u64 {
        let w = World::opening_world(seed);
        let c = w.communities[0].id;
        roofs.insert(hf_sim::systems::settlement::roof_style(&w, c, Material::WOOD).0);
        assert_eq!(
            hf_sim::systems::settlement::roof_style(&w, c, Material::STONE_BLOCK),
            Material::WOOD,
            "a stone house carries shingles"
        );
    }
    assert!(roofs.contains(&Material::THATCH.0) && roofs.contains(&Material::WOOD.0));
    // With stone to hand, the next house is stone.
    let mut w = World::opening_world(5);
    w.meta.autoplan = true;
    let home = w.communities[0].home;
    w.drop_item(hf_sim::items::ItemKind::StoneBlock, 40, home);
    // A house is someone's wish now (C57): they slept on the ground last night.
    let tick = w.meta.tick;
    for a in w.agents.iter_mut() {
        a.thoughts
            .add(hf_sim::mind::ThoughtKind::SleptRough, tick, None);
    }
    for _ in 0..TICKS_PER_DAY {
        w.step();
        if w.designations
            .iter()
            .any(|d| matches!(d.kind, hf_sim::work::DesignationKind::BuildWall(_)))
        {
            break;
        }
    }
    assert!(w.designations.iter().any(|d| matches!(
        d.kind,
        hf_sim::work::DesignationKind::BuildWall(Material::STONE_BLOCK)
    )));
}

#[test]
fn an_old_save_keeps_its_houses_roofed() {
    let mut w = World::opening_world(9);
    w.meta.autoplan = true;
    for _ in 0..6 * TICKS_PER_DAY {
        w.step();
    }
    let n = w.roofs.len();
    assert!(n > 0);
    // As a save from before roofs: no roof layer at all, and no mark that roofs were saved
    // (since #159 only such a save is roofed on load).
    w.roofs.clear();
    let mut snap = hf_sim::save::snapshot::Snapshot::capture(&w);
    snap.roofs_saved = false;
    let back = snap.restore();
    assert!(
        back.roofs.len() >= n.min(20),
        "standing houses come back roofed: {}",
        back.roofs.len()
    );
}

#[test]
fn an_ordinary_winter_brings_down_few_roofs() {
    // Through the first winter (days 15 to 20) and a little past it. Seed 7's winter has a
    // snow spell of about thirty hours (a ninety-percentile winter), and the four weakest tiles
    // in that village (quality under 0.3, laid by its least skilled hands) give way; seed 3's
    // loses nothing. So: at most one of the two loses roof to snow, and only a few tiles.
    let mut lost_any = 0;
    let mut roofs = 0;
    for seed in [3u64, 7] {
        let mut w = World::opening_world(seed);
        w.meta.autoplan = true;
        let mut tiles = 0;
        for _ in 0..22 * TICKS_PER_DAY {
            w.step();
            for e in &w.last_events {
                if let EventKind::RoofFell {
                    tiles: t, because, ..
                } = &e.kind
                {
                    if because.contains("snow") {
                        tiles += *t;
                    }
                }
            }
        }
        assert!(tiles <= 10, "seed {seed}: snow took {tiles} roof tiles");
        if tiles > 0 {
            lost_any += 1;
        }
        roofs += w.roofs.len();
    }
    assert!(roofs >= 40, "{roofs}");
    assert!(lost_any <= 1, "both winters brought roofs down");
}

#[test]
fn nothing_is_built_over_home() {
    // Home is where reach is measured from (edges for bands, tools in reach, camps); a house
    // built over it walled seed 19's village in from the world.
    for seed in [3u64, 7, 11, 19] {
        let mut w = World::opening_world(seed);
        w.meta.autoplan = true;
        for _ in 0..10 * TICKS_PER_DAY {
            w.step();
        }
        let home = w.communities[0].home;
        assert!(w.grid.walkable(home), "seed {seed}: home is open ground");
    }
}
