//! The sim's parallel parts give the same world at any thread count (#428, D55): path
//! searches of a tick's decisions, navigation region rebuilds, and the planner's site searches
//! are spread across threads, and a run on one thread and a run on several must not differ.

use hf_sim::digest::digest;
use hf_sim::world::gen::GenParams;
use hf_sim::World;

/// A party of 500 on the default valley (the profile's size), run for `days` in a pool of
/// `threads` workers; the digest at the end of each day.
fn run(threads: usize, days: u64) -> Vec<String> {
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .build()
        .expect("pool");
    pool.install(|| {
        let mut w = World::generate(1, GenParams::default());
        w.found_party(500);
        let mut out = Vec::new();
        for _ in 0..days {
            for _ in 0..hf_sim::time::TICKS_PER_DAY {
                w.step();
            }
            out.push(digest(&w));
        }
        out
    })
}

#[test]
fn five_hundred_people_on_one_thread_and_on_six_give_the_same_world() {
    let days = 2;
    let spread_before = hf_sim::par::SPREAD.load(std::sync::atomic::Ordering::Relaxed);
    let one = std::thread::spawn(move || run(1, days));
    let six = std::thread::spawn(move || run(6, days));
    let (one, six) = (one.join().expect("one"), six.join().expect("six"));
    assert_eq!(one.len(), days as usize);
    assert_eq!(one, six, "the digests differ between 1 and 6 threads");
    // Both runs went through the parallel parts (many times a day at this size).
    let spread = hf_sim::par::SPREAD.load(std::sync::atomic::Ordering::Relaxed) - spread_before;
    assert!(spread > 100, "only {spread} parallel batches in two runs");
}
