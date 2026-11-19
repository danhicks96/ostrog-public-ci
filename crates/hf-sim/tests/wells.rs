//! #433: a community with no fresh water a short walk from home digs a well. The shaft goes
//! down a level at a time until it strikes water in the rock (#267's strata, #405's
//! struck_water), and the well is built over it; a shaft that reaches the hard rock dry is
//! given up, with why.

use hf_sim::events::EventKind;
use hf_sim::ids::{CommunityId, TilePos};
use hf_sim::project::ProjectKind;
use hf_sim::structures::StructureKind;
use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use hf_sim::work::DesignationKind;
use hf_sim::world::gen::GenParams;
use hf_sim::world::{Material, Shape, Tile, TileFlags};
use hf_sim::World;

/// Flat dry ground at level 1 (people stand at 2), no water anywhere, six people. With
/// `basin`, the rock under the whole valley is sandstone over a middle stratum four levels
/// down, lower than the valley's usual break: water lies three and two levels down.
fn dry_valley(seed: u64, basin: bool) -> (World, CommunityId, TilePos) {
    let mut w = World::generate(seed, GenParams::small());
    w.flatten(0, 0, 64, 64, 1);
    w.plants.clear();
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.meta.tick = TICKS_PER_HOUR * 7;
    let top = if basin { -3 } else { 0 };
    for c in 0..w.grid.strata.len() {
        let (m, _, deep) = w.grid.strata[c];
        w.grid.strata[c] = (m, top as i8, deep);
        w.grid.bedrock[c] = Material::SANDSTONE;
        if let Some(s) = w.grid.soil.get_mut(c) {
            *s = 1;
        }
    }
    // No water in the valley.
    for y in 0..64 {
        for x in 0..64 {
            for z in -8..6 {
                let p = TilePos::new(x, y, z);
                if w.grid.get(p).is_water() {
                    w.grid.set(p, Tile::solid(Material::SOIL));
                }
            }
        }
    }
    w.rebuild_caches();
    hf_sim::floods::survey(&mut w);
    let ids = w.found_party(6);
    let cid = w
        .agents
        .iter()
        .find(|a| a.id == ids[0])
        .unwrap()
        .community
        .unwrap();
    let home = w.communities.iter().find(|c| c.id == cid).unwrap().home;
    w.meta.autoplan = true;
    w.ensure_nav();
    (w, cid, home)
}

fn morning(w: &mut World) {
    let day = w.meta.tick / TICKS_PER_DAY + 1;
    w.meta.tick = day * TICKS_PER_DAY + hf_sim::systems::projects::AT;
    hf_sim::systems::projects::step(w);
}

/// The planner's hour, and whatever level of the shaft it ordered dug at once: the cut on
/// record and the aquifer pass seeing it.
fn dig_an_hour(w: &mut World) {
    w.meta.tick = (w.meta.tick / TICKS_PER_HOUR + 1) * TICKS_PER_HOUR;
    hf_sim::systems::settlement::step(w);
    let cuts: Vec<TilePos> = w
        .designations
        .iter()
        .filter(|d| !d.done && d.kind == DesignationKind::DigStairs)
        .map(|d| d.pos)
        .collect();
    for at in cuts {
        let mut t = Tile::AIR;
        t.shape = Shape::StairsBoth;
        t.material = Material::SANDSTONE;
        w.grid.set(at, t);
        w.dug.insert(at);
        w.designations.retain(|d| d.pos != at);
        let agent = w.agents[0].id;
        let tick = w.meta.tick;
        let id = w.events.push(
            tick,
            Some(at),
            hf_sim::events::Severity::Minor,
            EventKind::Mined {
                agent,
                at,
                found: None,
                rock: None,
            },
        );
        let e = w
            .events
            .pending
            .iter()
            .find(|e| e.id == id)
            .unwrap()
            .clone();
        w.last_events = vec![e];
        hf_sim::aquifer::observe(w);
    }
}

fn well(w: &World) -> Option<&hf_sim::project::Project> {
    w.projects
        .iter()
        .rev()
        .find(|p| p.kind == ProjectKind::Well)
}

#[test]
fn far_from_water_they_dig_a_well_down_to_the_water() {
    let (mut w, cid, home) = dry_valley(4334, true);
    morning(&mut w);
    let p = well(&w).expect("a well is proposed");
    assert!(p.because.contains("no water"), "{}", p.because);
    for _ in 0..12 {
        dig_an_hour(&mut w);
        if well(&w).is_some_and(|p| p.work.is_some()) {
            break;
        }
    }
    let p = well(&w).unwrap();
    assert!(
        w.events
            .pending
            .iter()
            .any(|e| matches!(e.kind, EventKind::StruckWater { .. })),
        "water struck, on record"
    );
    let work = p.work.expect("the well ordered over the shaft");
    let d = w.designations.iter().find(|d| d.id == work).unwrap();
    assert!(matches!(
        d.kind,
        DesignationKind::BuildStructure(StructureKind::Well, _)
    ));
    // Built, it is where they drink, even in a hard frost.
    let site = d.pos;
    w.place_structure(StructureKind::Well, site, Material::STONE_BLOCK, true);
    let mut wx = hf_sim::systems::weather::current(&w);
    wx.temperature_c = -10.0;
    w.weather = wx;
    let s = hf_sim::water::nearest(&w, home, 60).expect("water to drink");
    assert!(s.well && s.source == site);
    let _ = cid;
}

#[test]
fn a_well_dug_to_dry_hard_rock_is_given_up_with_why() {
    let (mut w, _, _) = dry_valley(4335, false);
    morning(&mut w);
    assert!(well(&w).is_some(), "a well is proposed");
    for _ in 0..12 {
        dig_an_hour(&mut w);
        if well(&w).is_some_and(|p| p.done) {
            break;
        }
    }
    assert!(well(&w).unwrap().done, "given up");
    assert!(w.events.pending.iter().any(|e| matches!(
        &e.kind,
        EventKind::Chose { what, because, .. } if what == "gave up on the well" && because.contains("dry")
    )));
    assert!(!w
        .events
        .pending
        .iter()
        .any(|e| matches!(e.kind, EventKind::StruckWater { .. })));
    // Not again at once: with no ground about showing water, not for five years.
    morning(&mut w);
    assert!(well(&w).unwrap().done);
    let tick = w.meta.tick;
    let until = w
        .communities
        .iter()
        .flat_map(|c| c.no_site.iter())
        .find(|(k, _)| *k == StructureKind::Well)
        .map(|(_, u)| *u)
        .unwrap();
    assert!(until > tick + 50 * TICKS_PER_DAY, "{until} {tick}");
}

#[test]
fn water_a_short_walk_off_wants_no_well() {
    let (mut w, _, home) = dry_valley(4336, true);
    let pond = home.offset(5, 0, 0);
    w.grid.set(
        pond,
        Tile {
            material: Material::WATER,
            shape: Shape::Floor,
            flags: TileFlags(TileFlags::WATER),
        },
    );
    hf_sim::floods::survey(&mut w);
    morning(&mut w);
    assert!(well(&w).is_none());
}

/// #433: a well is dug where water is likeliest: over the basin, where the ground shows the
/// water under it, not at a set step from home.
#[test]
fn a_well_is_sited_over_the_basin() {
    let (mut w, _, home) = dry_valley(4342, false);
    // A basin under a patch to the north-west of home only.
    for y in home.y - 10..=home.y - 6 {
        for x in home.x - 10..=home.x - 6 {
            let c = w.grid.col(x, y);
            let (m, _, deep) = w.grid.strata[c];
            w.grid.strata[c] = (m, -3, deep);
        }
    }
    morning(&mut w);
    dig_an_hour(&mut w);
    let site = well(&w).and_then(|p| p.site).expect("a site");
    assert!(
        (home.x - 10..=home.x - 6).contains(&site.x)
            && (home.y - 10..=home.y - 6).contains(&site.y),
        "{site:?} over the basin by {home:?}"
    );
    for _ in 0..12 {
        dig_an_hour(&mut w);
        if well(&w).is_some_and(|p| p.work.is_some() || p.done) {
            break;
        }
    }
    assert!(well(&w).unwrap().work.is_some(), "water struck");
}

/// #433: water under the shaft's own column is struck, though the rock round it is dry.
#[test]
fn water_under_the_shaft_alone_is_struck() {
    let (mut w, _, home) = dry_valley(4343, false);
    let spot = home.offset(-7, -7, 0);
    let c = w.grid.col(spot.x, spot.y);
    let (m, _, deep) = w.grid.strata[c];
    w.grid.strata[c] = (m, -3, deep);
    morning(&mut w);
    dig_an_hour(&mut w);
    let site = well(&w).and_then(|p| p.site).expect("a site");
    assert_eq!((site.x, site.y), (spot.x, spot.y));
    for _ in 0..12 {
        dig_an_hour(&mut w);
        if well(&w).is_some_and(|p| p.work.is_some() || p.done) {
            break;
        }
    }
    assert!(well(&w).unwrap().work.is_some(), "water struck");
}
