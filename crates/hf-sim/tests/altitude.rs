//! #268: the higher, the colder.

use hf_sim::ids::TilePos;
use hf_sim::world::gen::GenParams;
use hf_sim::World;

#[test]
fn the_same_weather_is_colder_on_high_ground() {
    let mut w = World::generate(2681, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    // A rise three levels up at one corner.
    w.flatten(40, 40, 50, 50, 4);
    w.meta.founding_site = TilePos::new(20, 20, 2);
    w.rebuild_caches();
    let ids = w.found_party(2);
    let (lo, hi) = (
        w.agents.iter().position(|a| a.id == ids[0]).unwrap(),
        w.agents.iter().position(|a| a.id == ids[1]).unwrap(),
    );
    w.agents[lo].pos = w.grid.surface(20, 20);
    w.agents[hi].pos = w.grid.surface(45, 45);
    assert!(w.grid.ground_z(45, 45) > w.grid.ground_z(20, 20));
    let comfort = w.agents[lo].species_def().climate.comfort_c[0];
    let mut wx = hf_sim::systems::weather::current(&w);
    wx.temperature_c = comfort + 2.0; // fine at the landing
    w.weather = wx;
    for a in w.agents.iter_mut() {
        a.needs.warmth = 1.0;
        a.clothes_wear = 0.0;
        a.activity = hf_sim::mind::Activity::Idle;
    }
    for _ in 0..20 {
        hf_sim::systems::hazards::cold_step(&mut w);
    }
    assert_eq!(w.agents[lo].needs.warmth, 1.0, "warm enough below");
    assert!(w.agents[hi].needs.warmth < 1.0, "cold up on the rise");
}
