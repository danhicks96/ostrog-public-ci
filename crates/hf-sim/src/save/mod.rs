//! Save = one consistent unit (DECISIONS D8): `world.bin` + `chronicle.sqlite` +
//! `manifest.json` in a `.hfsave` directory, written atomically via `.tmp` + rename.

pub mod manifest;
pub mod migrate;
pub mod snapshot;

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Context, Result};

use crate::chronicle::Chronicle;
use crate::world_state::World;
use manifest::Manifest;
use snapshot::Snapshot;

/// Format 2: MessagePack with field names (additive fields default); format 1 was a
/// positional pre-release encoding and is refused.
pub const FORMAT_VERSION: u32 = 2;
pub const GAME_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Named-field MessagePack: new fields with `#[serde(default)]` load from older saves.
pub fn encode(snap: &Snapshot) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    let mut ser = rmp_serde::Serializer::new(&mut out).with_struct_map();
    serde::Serialize::serialize(snap, &mut ser)?;
    Ok(out)
}

pub fn decode(bytes: &[u8]) -> Result<Snapshot<'static>> {
    Ok(rmp_serde::from_slice(bytes)?)
}

/// Everything needed to finish a save off the simulation thread. No file is written on
/// the simulation thread: a disk-backed chronicle is pinned at this moment by a read-only
/// connection holding a read transaction (WAL keeps that snapshot while the game appends),
/// an in-memory one is copied memory-to-memory; `commit` writes either.
pub struct Prepared {
    pub dir: PathBuf,
    pub world_bytes: Vec<u8>,
    pub chron: rusqlite::Connection,
    pub seed: u64,
    pub tick: u64,
}

/// Capture the world and copy the chronicle (both must happen on the simulation thread).
pub fn prepare(world: &World, chronicle: &mut Chronicle, dir: &Path) -> Result<Prepared> {
    let snap = Snapshot::capture(world);
    let world_bytes = encode(&snap)?;
    let chron = match chronicle.path() {
        Some(path) => {
            let c = rusqlite::Connection::open_with_flags(
                path,
                rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
            )?;
            // The read transaction fixes the snapshot the backup in `commit` will copy.
            c.execute_batch("BEGIN; SELECT count(*) FROM real_history;")?;
            c
        }
        None => {
            let mut c = rusqlite::Connection::open_in_memory()?;
            let backup = rusqlite::backup::Backup::new(&chronicle.conn, &mut c)?;
            backup.run_to_completion(4096, std::time::Duration::from_millis(0), None)?;
            drop(backup);
            c
        }
    };
    Ok(Prepared {
        dir: dir.to_path_buf(),
        world_bytes,
        chron,
        seed: world.seed(),
        tick: world.tick(),
    })
}

/// Finish a prepared save atomically on any thread: write into `.tmp`, then rename over
/// the slot. The manifest hashes the exact bytes written.
pub fn commit(p: Prepared) -> Result<()> {
    let dir = &p.dir;
    let tmp = PathBuf::from(format!("{}.tmp", dir.display()));
    if tmp.exists() {
        fs::remove_dir_all(&tmp)?;
    }
    fs::create_dir_all(&tmp)?;
    fs::write(tmp.join("world.bin"), &p.world_bytes)?;
    let chron_path = tmp.join("chronicle.sqlite");
    {
        let mut dst = rusqlite::Connection::open(&chron_path)?;
        dst.execute_batch("PRAGMA journal_mode = OFF; PRAGMA synchronous = OFF;")?;
        let backup = rusqlite::backup::Backup::new(&p.chron, &mut dst)?;
        backup.run_to_completion(4096, std::time::Duration::from_millis(0), None)?;
    }
    let manifest = Manifest::new(
        FORMAT_VERSION,
        GAME_VERSION,
        p.seed,
        p.tick,
        &p.world_bytes,
        manifest::sha256_file(&chron_path)?,
    );
    fs::write(
        tmp.join("manifest.json"),
        serde_json::to_vec_pretty(&manifest)?,
    )?;
    if dir.exists() {
        let old = PathBuf::from(format!("{}.old", dir.display()));
        if old.exists() {
            fs::remove_dir_all(&old)?;
        }
        fs::rename(dir, &old)?;
        fs::rename(&tmp, dir)?;
        fs::remove_dir_all(&old)?;
    } else {
        fs::rename(&tmp, dir)?;
    }
    Ok(())
}

/// Write a save. The chronicle must belong to this world (same tick).
pub fn write(world: &World, chronicle: &mut Chronicle, dir: &Path) -> Result<()> {
    commit(prepare(world, chronicle, dir)?)
}

/// Verify a save directory without loading it. Returns the manifest.
pub fn verify(dir: &Path) -> Result<Manifest> {
    let m: Manifest =
        serde_json::from_slice(&fs::read(dir.join("manifest.json")).context("manifest missing")?)?;
    if m.magic != manifest::MAGIC && m.magic != manifest::LEGACY_MAGIC {
        bail!("not an Ostrog save");
    }
    if m.format_version > FORMAT_VERSION {
        bail!(
            "save format {} is newer than this game understands ({}); update the game",
            m.format_version,
            FORMAT_VERSION
        );
    }
    // Hashed a block at a time, not read whole: the ring checks every slot before each save.
    let world_sha = manifest::sha256_file(&dir.join("world.bin")).context("world.bin missing")?;
    let chron_sha =
        manifest::sha256_file(&dir.join("chronicle.sqlite")).context("chronicle missing")?;
    if world_sha != m.world_sha256 {
        bail!("world.bin does not match manifest (corrupt or interrupted write)");
    }
    if chron_sha != m.chronicle_sha256 {
        bail!("chronicle.sqlite does not match manifest (corrupt or interrupted write)");
    }
    Ok(m)
}

/// Load a save. Runs the migration chain if the format is older. The chronicle is opened
/// from a copy inside the save directory.
pub fn read(dir: &Path) -> Result<(World, Chronicle)> {
    let m = verify(dir)?;
    let world_bytes = fs::read(dir.join("world.bin"))?;
    let world_bytes = migrate::migrate(m.format_version, FORMAT_VERSION, world_bytes)?;
    let snap: Snapshot = decode(&world_bytes)?;
    let world = snap.restore();
    if world.tick() != m.tick {
        return Err(anyhow!(
            "manifest tick {} disagrees with world tick {}",
            m.tick,
            world.tick()
        ));
    }
    // The save's chronicle file is never opened for writing: it is copied into a fresh
    // working chronicle so the save on disk stays byte-identical to its manifest.
    let mut chronicle = Chronicle::working()?;
    {
        let src = rusqlite::Connection::open_with_flags(
            dir.join("chronicle.sqlite"),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )?;
        let backup = rusqlite::backup::Backup::new(&src, &mut chronicle.conn)?;
        backup.run_to_completion(1024, std::time::Duration::from_millis(0), None)?;
    }
    // The backup replaces the freshly initialized schema with the save's older one. Re-apply
    // additive tables and indexes before simulation can record events introduced since the save.
    chronicle.ensure_schema()?;
    if let Some(mt) = chronicle.max_tick()? {
        if mt as u64 > world.tick() {
            bail!(
                "chronicle is ahead of the world ({} > {})",
                mt,
                world.tick()
            );
        }
    }
    Ok((world, chronicle))
}

/// Ring slot paths `base.0.hfsave` .. `base.{n-1}.hfsave`.
pub fn ring_paths(base: &Path, n: usize) -> Vec<PathBuf> {
    (0..n)
        .map(|i| PathBuf::from(format!("{}.{i}.hfsave", base.display())))
        .collect()
}

/// The slot to overwrite next: the oldest (lowest tick) or the first invalid one.
pub fn next_ring_slot(ring: &[PathBuf]) -> PathBuf {
    let mut best: Option<(u64, PathBuf)> = None;
    for p in ring {
        match verify(p) {
            Ok(m) => {
                if best.as_ref().map(|b| m.tick < b.0).unwrap_or(true) {
                    best = Some((m.tick, p.clone()));
                }
            }
            Err(_) => return p.clone(),
        }
    }
    best.map(|b| b.1).unwrap_or_else(|| ring[0].clone())
}

/// Choose the newest valid save among ring slots `name.0.hfsave`, `name.1.hfsave`, ...
pub fn newest_valid(ring: &[PathBuf]) -> Option<(PathBuf, Manifest)> {
    let mut best: Option<(PathBuf, Manifest)> = None;
    for p in ring {
        if let Ok(m) = verify(p) {
            if best.as_ref().map(|(_, b)| m.tick > b.tick).unwrap_or(true) {
                best = Some((p.clone(), m));
            }
        }
    }
    best
}
