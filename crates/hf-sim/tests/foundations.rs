//! Phase 1 gates: determinism, save round trip, manifest tamper rejection.

use hf_sim::chronicle::Chronicle;
use hf_sim::digest::digest;
use hf_sim::world::gen::GenParams;
use hf_sim::{save, Session, World};

fn small_world(seed: u64) -> World {
    World::generate(seed, GenParams::small())
}

#[test]
fn digest_deterministic() {
    let a = {
        let mut w = small_world(42);
        w.run(2_000);
        digest(&w)
    };
    let b = {
        let mut w = small_world(42);
        w.run(2_000);
        digest(&w)
    };
    assert_eq!(a, b, "same seed, same ticks must digest identically");
    let c = {
        let mut w = small_world(43);
        w.run(2_000);
        digest(&w)
    };
    assert_ne!(a, c, "different seeds must differ");
}

#[test]
fn save_roundtrip_identical() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.hfsave");
    // Continuous run to 3000.
    let continuous = {
        let mut s = Session::new(small_world(7), Chronicle::in_memory().unwrap());
        s.run(3_000).unwrap();
        (digest(&s.world), s.chronicle.count().unwrap())
    };
    // Run to 1500, save, load, run 1500 more.
    let mut s = Session::new(small_world(7), Chronicle::in_memory().unwrap());
    s.run(1_500).unwrap();
    save::write(&s.world, &mut s.chronicle, &path).unwrap();
    let m = save::verify(&path).unwrap();
    assert_eq!(m.tick, 1_500);
    let (world, chronicle) = save::read(&path).unwrap();
    let mut s2 = Session::new(world, chronicle);
    assert_eq!(
        digest(&s2.world),
        digest(&s.world),
        "restored world digests like the saved one"
    );
    s2.run(1_500).unwrap();
    assert_eq!(
        digest(&s2.world),
        continuous.0,
        "restored run continues identically"
    );
    assert_eq!(
        s2.chronicle.count().unwrap(),
        continuous.1,
        "chronicle rows match"
    );
}

#[test]
fn save_manifest_mismatch_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.hfsave");
    let mut s = Session::new(small_world(9), Chronicle::in_memory().unwrap());
    s.run(100).unwrap();
    save::write(&s.world, &mut s.chronicle, &path).unwrap();
    // Corrupt world.bin.
    let wb = path.join("world.bin");
    let mut bytes = std::fs::read(&wb).unwrap();
    bytes[10] ^= 0xff;
    std::fs::write(&wb, bytes).unwrap();
    assert!(save::verify(&path).is_err());
    assert!(save::read(&path).is_err());
    // Ring selection skips the corrupt one.
    let good = dir.path().join("g.hfsave");
    save::write(&s.world, &mut s.chronicle, &good).unwrap();
    let pick = save::newest_valid(&[path.clone(), good.clone()]).unwrap();
    assert_eq!(pick.0, good);
}

#[test]
fn save_newer_format_refused() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.hfsave");
    let mut s = Session::new(small_world(3), Chronicle::in_memory().unwrap());
    s.run(10).unwrap();
    save::write(&s.world, &mut s.chronicle, &path).unwrap();
    let mp = path.join("manifest.json");
    let txt = std::fs::read_to_string(&mp)
        .unwrap()
        .replace("\"format_version\": 2", "\"format_version\": 99");
    std::fs::write(&mp, txt).unwrap();
    let err = save::verify(&path).unwrap_err().to_string();
    assert!(err.contains("newer"), "{err}");
    // A format-1 save is refused with a clear message.
    let txt = std::fs::read_to_string(&mp)
        .unwrap()
        .replace("\"format_version\": 99", "\"format_version\": 1");
    std::fs::write(&mp, txt).unwrap();
    let err = match save::read(&path) {
        Ok(_) => panic!("format 1 must be refused"),
        Err(e) => e.to_string(),
    };
    assert!(err.contains("pre-release"), "{err}");
}

#[test]
fn worldgen_has_the_expected_features() {
    let w = small_world(11);
    let g = &w.grid;
    let mut water = 0;
    let mut relief = std::collections::BTreeSet::new();
    for y in 0..g.height {
        for x in 0..g.width {
            let gz = g.ground_z(x, y);
            relief.insert(gz);
            if g.get(hf_sim::ids::TilePos::new(x, y, gz + 1)).is_water() {
                water += 1;
            }
        }
    }
    assert!(relief.len() >= 2, "surface has relief");
    assert!(water > 20, "there is water: {water}");
    assert!(w.plants.len() > 100, "there are plants: {}", w.plants.len());
    let caves: usize = g
        .chunks
        .iter()
        .filter(|(k, _)| k.2 < 0)
        .map(|(_, c)| {
            c.tiles
                .iter()
                .filter(|t| t.shape == hf_sim::world::Shape::Open)
                .count()
        })
        .sum();
    assert!(caves > 0, "there are caves");
    assert!(
        g.walkable(w.meta.founding_site),
        "founding site is walkable"
    );
}

#[test]
fn interrupted_write_recovers() {
    // A crash mid-write leaves a `.tmp` dir and a half-written slot; the ring still yields
    // the previous valid save, and the next save overwrites the broken slot.
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path().join("auto");
    let ring = save::ring_paths(&base, 3);
    let mut s = Session::new(small_world(8), Chronicle::in_memory().unwrap());
    s.run(500).unwrap();
    save::write(&s.world, &mut s.chronicle, &ring[0]).unwrap();
    s.run(500).unwrap();
    save::write(&s.world, &mut s.chronicle, &ring[1]).unwrap();
    // Simulate an interrupted third save: tmp dir with a partial world and no manifest.
    let tmp = std::path::PathBuf::from(format!("{}.tmp", ring[2].display()));
    std::fs::create_dir_all(&tmp).unwrap();
    std::fs::write(tmp.join("world.bin"), b"partial").unwrap();
    std::fs::create_dir_all(&ring[2]).unwrap();
    std::fs::write(ring[2].join("world.bin"), b"garbage").unwrap();
    let (picked, m) = save::newest_valid(&ring).unwrap();
    assert_eq!(picked, ring[1]);
    assert_eq!(m.tick, 1000);
    assert_eq!(
        save::next_ring_slot(&ring),
        ring[2],
        "the broken slot is reused first"
    );
    s.run(100).unwrap();
    save::write(&s.world, &mut s.chronicle, &ring[2]).unwrap();
    assert!(save::verify(&ring[2]).is_ok());
    let (picked, _) = save::newest_valid(&ring).unwrap();
    assert_eq!(picked, ring[2]);
}

#[test]
fn migration_chain_loads_fixture() {
    let fixture =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/saves/v2.hfsave");
    if !fixture.exists() {
        panic!(
            "fixture missing: run `ostrog fixture crates/hf-sim/tests/fixtures/saves/v2.hfsave`"
        );
    }
    let m = save::verify(&fixture).expect("fixture verifies");
    assert_eq!(m.format_version, 2);
    let (world, chronicle) =
        save::read(&fixture).expect("fixture loads through the migration chain");
    assert_eq!(world.tick(), m.tick);
    assert!(chronicle.count().unwrap() > 0);
    assert!(
        chronicle
            .conn
            .query_row(
                "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'built_record'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .is_ok(),
        "loading an older save must restore the current chronicle schema"
    );
    // And it keeps simulating.
    let mut s = Session::new(world, chronicle);
    s.run(200).unwrap();
    assert_eq!(s.world.tick(), m.tick + 200);
}

#[test]
fn a_disk_chronicle_saves_the_moment_it_was_saved() {
    // The game keeps its chronicle in a working file. A save pins it at `prepare`; rows the
    // game writes before `commit` finishes are not in the save, and the working file goes
    // when the chronicle does.
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("d.hfsave");
    let mut s = Session::new(World::opening_world(3), Chronicle::working().unwrap());
    let file = s.chronicle.path().unwrap().to_path_buf();
    assert!(file.exists());
    s.run(3_000).unwrap();
    let at_save = s.chronicle.count().unwrap();
    let tick = s.world.tick();
    let prepared = save::prepare(&s.world, &mut s.chronicle, &path).unwrap();
    s.run(6_000).unwrap();
    assert!(
        s.chronicle.count().unwrap() > at_save,
        "the game went on writing"
    );
    save::commit(prepared).unwrap();
    let (world, chronicle) = save::read(&path).unwrap();
    assert_eq!(world.tick(), tick);
    assert_eq!(
        chronicle.count().unwrap(),
        at_save,
        "the save holds what was there at prepare"
    );
    assert!(
        chronicle.path().is_some(),
        "a loaded chronicle is disk-backed too"
    );
    drop(s);
    assert!(
        !file.exists(),
        "the working file is removed with its chronicle"
    );
}
