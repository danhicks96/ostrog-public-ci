//! #778 (WP-07 7.5 follow-up): witnesses of a manifestation must be able to SEE the spot,
//! not merely stand within radius — a miner in a parallel gallery behind solid rock forms
//! no account of what they could not have seen (resonance.rs witness lists, beliefs share
//! keeps the account's level when blurring the place).

use hf_sim::ids::TilePos;
use hf_sim::mind::techniques::Techniques;
use hf_sim::mind::witnessed::Saw;
use hf_sim::resonance::{self, Potential, Resonance};
use hf_sim::world::gen::GenParams;
use hf_sim::world::{Material, Tile};
use hf_sim::World;

/// A flat meadow; the pair stands in a carved gallery at z=-2, carrier at (42, 30) and
/// witness at (42, 34), the face one column over from the carrier. Everything in the volume
/// x 38..=50, y 26..=38 at z=-2 is open air, so the ONLY thing that can block the witness's
/// view is a wall the test itself builds. Returns (world, carrier, witness).
fn gallery(seed: u64) -> (World, usize, usize) {
    let mut w = World::generate(seed, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.plants.clear();
    w.meta.tick = 7 * hf_sim::time::TICKS_PER_DAY + hf_sim::time::TICKS_PER_HOUR * 8;
    for y in 26..=38 {
        for x in 38..=50 {
            w.grid.set(TilePos::new(x, y, -2), Tile::AIR);
        }
    }
    let ids = w.found_party(2);
    let c = w.agents.iter().position(|a| a.id == ids[0]).unwrap();
    let n = w.agents.iter().position(|a| a.id == ids[1]).unwrap();
    w.agents[c].pos = TilePos::new(42, 30, -2);
    w.agents[n].pos = TilePos::new(42, 34, -2);
    w.agents[c].resonance = Some(Resonance {
        potential: Potential::RockSense,
        sat: resonance::WAKES_AFTER,
        woke: Some(1),
        refused: 0,
        first_manifestation: None,
    });
    w.agents[c].needs.rest = 1.0;
    w.agents[c].techniques = Techniques::default();
    (w, c, n)
}

/// The unheld span one column over from the carrier.
const UNHELD: &[TilePos] = &[TilePos::new(44, 30, -1)];
const FACE: TilePos = TilePos::new(44, 30, -2);

/// A solid rock wall across the gallery at y=32, between the witness and the face.
fn wall_between(w: &mut World) {
    for x in 38..=50 {
        w.grid
            .set(TilePos::new(x, 32, -2), Tile::solid(Material::SOIL));
    }
}

#[test]
fn a_miner_in_a_parallel_gallery_behind_rock_forms_no_account() {
    let (mut w, c, n) = gallery(11);
    wall_between(&mut w);
    assert!(
        !hf_sim::mind::perception::can_see(&w.grid, w.agents[n].pos, FACE),
        "setup: the wall must actually block the view"
    );
    assert!(resonance::balks(&mut w, c, FACE, UNHELD));
    assert!(
        w.agents[n].witnessed.is_empty(),
        "within radius, behind solid rock: they saw nothing"
    );
}

#[test]
fn a_miner_in_the_same_gallery_with_clear_sight_does() {
    let (mut w, c, n) = gallery(12);
    assert!(
        hf_sim::mind::perception::can_see(&w.grid, w.agents[n].pos, FACE),
        "setup: the open gallery must give a clear view"
    );
    assert!(resonance::balks(&mut w, c, FACE, UNHELD));
    let acc = &w.agents[n].witnessed;
    assert_eq!(
        acc.len(),
        1,
        "same gallery, clear sight: they hold the account"
    );
    assert_eq!(acc[0].saw, Saw::RefusedTheCut);
}

#[test]
fn the_quiet_waking_is_not_witnessed_through_rock() {
    let (mut w, c, n) = gallery(13);
    wall_between(&mut w);
    w.agents[c].resonance = Some(Resonance {
        potential: Potential::RockSense,
        sat: 0,
        woke: None,
        refused: 0,
        first_manifestation: None,
    });
    // Both sit by the stone — but the stone is on the carrier's side of the wall.
    for _ in 0..resonance::WAKES_AFTER {
        resonance::sat_by_stone(&mut w, c);
        resonance::sat_by_stone(&mut w, n);
    }
    assert!(
        w.agents[n].witnessed.is_empty(),
        "the waking travels no better through rock than the refusal does"
    );
}

#[test]
fn a_retold_account_keeps_its_level_when_the_place_blurs() {
    let (mut w, c, n) = gallery(14);
    assert!(resonance::balks(&mut w, c, FACE, UNHELD));
    // A stranger far away, out of sight, hears it in talk.
    let ids = w.found_party(1);
    let l = w.agents.iter().position(|a| a.id == ids[0]).unwrap();
    w.agents[l].pos = TilePos::new(10, 10, 1);
    let tick = w.tick();
    hf_sim::systems::beliefs::share(&mut w, n, l, tick);
    let heard = &w.agents[l].witnessed;
    assert_eq!(heard.len(), 1, "the account passed on");
    assert_eq!(
        heard[0].at.z, -2,
        "a refusal born underground is retold underground — the blur moves x/y, never the level"
    );
}
