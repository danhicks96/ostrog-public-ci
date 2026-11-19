//! #484: the viewer's timelapse keys its retained samples on `digest::world_identity`. The key
//! must survive running and a save round trip (so a resumed or reloaded world finds its own
//! samples) and must differ between worlds (so it never shows another world's), and reading it
//! must not change the world.

use hf_sim::chronicle::Chronicle;
use hf_sim::digest::{digest, world_identity};
use hf_sim::world::gen::GenParams;
use hf_sim::{save, Session, World};

fn small_world(seed: u64) -> World {
    World::generate(seed, GenParams::small())
}

#[test]
fn identity_survives_running_and_a_save_round_trip() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.hfsave");
    let mut s = Session::new(small_world(484), Chronicle::in_memory().unwrap());
    let at_start = world_identity(&s.world);
    s.run(1_200).unwrap();
    assert_eq!(
        world_identity(&s.world),
        at_start,
        "running does not change the identity"
    );
    save::write(&s.world, &mut s.chronicle, &path).unwrap();
    let (world, _) = save::read(&path).unwrap();
    assert_eq!(
        world_identity(&world),
        at_start,
        "a loaded save has its world's identity"
    );
}

#[test]
fn identity_differs_between_worlds_and_reading_it_changes_nothing() {
    let a = small_world(484);
    let b = small_world(485);
    assert_ne!(
        world_identity(&a),
        world_identity(&b),
        "two seeds, two identities"
    );
    let before = digest(&a);
    let _ = world_identity(&a);
    assert_eq!(
        digest(&a),
        before,
        "reading the identity leaves the state as it was"
    );
}
