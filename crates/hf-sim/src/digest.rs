//! World digest: a stable hash of all simulation state, for determinism and save tests.
//! Uses the named MessagePack encoding of the snapshot so that any new state field is covered
//! automatically once it is part of the snapshot.

use sha2::{Digest, Sha256};

use crate::world_state::World;

pub fn digest(world: &World) -> String {
    let snap = crate::save::snapshot::Snapshot::capture(world);
    let bytes = crate::save::encode(&snap).expect("snapshot serializes");
    let mut h = Sha256::new();
    h.update(&bytes);
    hex::encode(h.finalize())
}

/// Which world this is, independent of how far it has run (#484): the same for every tick and
/// every save of one world, different between worlds made from different seeds or new-game
/// choices. The viewer's timelapse keys its retained samples on it, so a reloaded or resumed
/// world finds its own samples and never another world's. Reads the world; changes nothing.
pub fn world_identity(world: &World) -> String {
    let m = &world.meta;
    let key = serde_json::json!({
        "seed": m.seed,
        "name": m.name,
        "founding_site": m.founding_site,
        "epoch": m.epoch,
        "params": m.params,
        "setup": m.setup,
    });
    let mut h = Sha256::new();
    h.update(key.to_string().as_bytes());
    hex::encode(h.finalize())
}
