//! A save borrows the world rather than copying it (#431). Each save used to clone the whole
//! world into its snapshot; the clone's small allocations, freed together, were kept by glibc
//! rather than given back, and over a 600-day soak the process held 60 MB of free heap for
//! 25 MB of live state, failing the soak's memory budget. This file counts the bytes a capture
//! allocates through its own global allocator (each test file is its own binary).

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

use hf_sim::save::{decode, encode, snapshot::Snapshot};
use hf_sim::time::TICKS_PER_DAY;

struct Counting;

static ALLOCATED: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCATED.fetch_add(layout.size(), Ordering::Relaxed);
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        ALLOCATED.fetch_add(new_size, Ordering::Relaxed);
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static GLOBAL: Counting = Counting;

/// The count is of every thread, so the tests here take turns.
static ONE_AT_A_TIME: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Bytes allocated (on any thread of this test binary) while `f` runs.
fn allocated_by<T>(f: impl FnOnce() -> T) -> (T, usize) {
    let before = ALLOCATED.load(Ordering::Relaxed);
    let out = f();
    (out, ALLOCATED.load(Ordering::Relaxed) - before)
}

#[test]
fn a_capture_allocates_almost_nothing_however_long_the_game() {
    let _turn = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
    let mut w = hf_sim::World::from_setup(&hf_sim::setup::WorldSetup::new(3));
    w.meta.autoplan = true;
    let mut sizes = Vec::new();
    for day in 1..=12 {
        for _ in 0..TICKS_PER_DAY {
            w.step();
        }
        if day % 4 == 0 {
            // The generated species are the only thing copied: a few kilobytes, the same on
            // day 4 as on day 12. A clone of the world was megabytes and grew with it.
            let (snap, bytes) = allocated_by(|| Snapshot::capture(&w));
            assert!(
                bytes < 64 * 1024,
                "day {day}: a capture allocated {bytes} bytes"
            );
            let (encoded, _) = allocated_by(|| encode(&snap).unwrap());
            sizes.push((day, bytes, encoded.len()));
        }
    }
    // The encoded world is far larger than what the capture allocated: the borrow is real.
    for (day, bytes, encoded) in sizes {
        assert!(
            encoded > 100 * bytes.max(1),
            "day {day}: {bytes} vs {encoded}"
        );
    }
}

#[test]
fn a_borrowed_snapshot_encodes_as_the_owned_one_read_back() {
    let _turn = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
    let mut w = hf_sim::World::from_setup(&hf_sim::setup::WorldSetup::new(5));
    w.meta.autoplan = true;
    for _ in 0..TICKS_PER_DAY {
        w.step();
    }
    // Captured (borrowed) and decoded (owned) snapshots give the same bytes, and the world
    // restored from them is the world saved.
    let bytes = encode(&Snapshot::capture(&w)).unwrap();
    let owned = decode(&bytes).unwrap();
    assert_eq!(encode(&owned).unwrap(), bytes);
    let back = owned.restore();
    assert_eq!(hf_sim::digest::digest(&back), hf_sim::digest::digest(&w));
}

/// #1268: `Knowledge::place_affect` is retired (place feeling lives in `memory::place_multiplier`
/// now), kept only so a save made while it was still written still loads.
#[test]
fn old_saves_with_place_affect_still_load() {
    use hf_sim::ids::TilePos;
    use hf_sim::mind::knowledge::{Belief, Knowledge, Known};
    use std::collections::BTreeMap;

    // A struct in the shape `Knowledge` had before #1268, with `place_affect` still written.
    #[derive(serde::Serialize)]
    struct OldKnowledge {
        map: BTreeMap<Known, Belief>,
        place_affect: BTreeMap<TilePos, f32>,
    }
    let mut place_affect = BTreeMap::new();
    place_affect.insert(TilePos::new(5, 6, 0), 0.5);
    let old = OldKnowledge {
        map: BTreeMap::new(),
        place_affect,
    };
    let mut bytes = Vec::new();
    let mut ser = rmp_serde::Serializer::new(&mut bytes).with_struct_map();
    serde::Serialize::serialize(&old, &mut ser).unwrap();

    // It still loads, the field populated from the old save.
    let loaded: Knowledge = rmp_serde::from_slice(&bytes).unwrap();
    assert_eq!(loaded.place_affect.len(), 1);

    // The re-saved file carries no `place_affect` key.
    let mut re_saved = Vec::new();
    let mut ser = rmp_serde::Serializer::new(&mut re_saved).with_struct_map();
    serde::Serialize::serialize(&loaded, &mut ser).unwrap();
    assert!(
        !re_saved
            .windows(b"place_affect".len())
            .any(|w| w == b"place_affect"),
        "the retired key should not be written back"
    );
}

#[test]
fn checking_a_save_reads_its_files_a_block_at_a_time() {
    let _turn = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
    // The ring checks every slot before each save; it used to read each file whole, the
    // chronicle reaching 40 MB by day 600.
    let mut s = hf_sim::Session::new(
        hf_sim::World::from_setup(&hf_sim::setup::WorldSetup::new(7)),
        hf_sim::chronicle::Chronicle::in_memory().unwrap(),
    );
    for _ in 0..TICKS_PER_DAY {
        s.step().unwrap();
    }
    let dir = tempfile::tempdir().unwrap();
    let slot = dir.path().join("a.hfsave");
    hf_sim::save::write(&s.world, &mut s.chronicle, &slot).unwrap();
    let on_disk: u64 = ["world.bin", "chronicle.sqlite"]
        .iter()
        .map(|f| std::fs::metadata(slot.join(f)).unwrap().len())
        .sum();
    let (m, bytes) = allocated_by(|| hf_sim::save::verify(&slot));
    assert_eq!(m.unwrap().tick, s.world.tick());
    assert!(
        (bytes as u64) < 256 * 1024 && (bytes as u64) * 8 < on_disk,
        "checking {on_disk} bytes of save allocated {bytes}"
    );
    // And the save still loads into the world that was saved.
    let (back, _) = hf_sim::save::read(&slot).unwrap();
    assert_eq!(
        hf_sim::digest::digest(&back),
        hf_sim::digest::digest(&s.world)
    );
}
