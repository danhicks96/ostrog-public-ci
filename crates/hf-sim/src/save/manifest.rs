use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const MAGIC: &str = "OSTROG-SAVE";

/// The magic saves carried before the rebrand (issue #2106). Still accepted on load so existing
/// saves keep working; new saves are written with `MAGIC`.
pub const LEGACY_MAGIC: &str = "HEARTHFALL-SAVE";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Manifest {
    pub magic: String,
    pub format_version: u32,
    pub game_version: String,
    pub seed: u64,
    pub tick: u64,
    pub world_sha256: String,
    pub chronicle_sha256: String,
}

pub fn sha256(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    hex::encode(h.finalize())
}

/// The same hash read from a file a block at a time. Saves hash their files this way: the
/// chronicle reaches tens of megabytes in a long game, and reading each slot of the ring whole
/// to check it, every few days, left glibc holding that much freed memory (#431).
pub fn sha256_file(path: &std::path::Path) -> std::io::Result<String> {
    use std::io::Read;
    let mut f = std::fs::File::open(path)?;
    let mut h = Sha256::new();
    let mut buf = vec![0u8; 64 * 1024];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    Ok(hex::encode(h.finalize()))
}

impl Manifest {
    pub fn new(
        format_version: u32,
        game_version: &str,
        seed: u64,
        tick: u64,
        world: &[u8],
        chronicle_sha256: String,
    ) -> Self {
        Manifest {
            magic: MAGIC.to_string(),
            format_version,
            game_version: game_version.to_string(),
            seed,
            tick,
            world_sha256: sha256(world),
            chronicle_sha256,
        }
    }
}
