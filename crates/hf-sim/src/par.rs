//! Work spread across threads, with the result the same as one thread would give (#428, D55).
//!
//! The rule for every parallel part of the sim: the inputs are gathered in a fixed order, each
//! is worked out from a shared read-only view of the world (no RNG draws, no writes to shared
//! state), and the results are applied in that same order on the calling thread. The thread
//! count then changes only how long a tick takes, never what happens in it;
//! `tests/threads.rs` checks that 1 thread and several give the same digest.
//!
//! The pool: the sim's own, built on first use with `threads()` workers, unless the caller is
//! already running inside a rayon pool (a test that sets the count with its own pool), in which
//! case that pool is used.

use std::sync::OnceLock;

use rayon::prelude::*;

use crate::ids::TilePos;
use crate::nav::{AStarScratch, PathResult};
use crate::world_state::World;

/// The most threads the sim takes by default, so a game on a many-core box leaves the other
/// cores to everything else running there.
pub const DEFAULT_MAX_THREADS: usize = 8;

static SETTING: OnceLock<usize> = OnceLock::new();
static POOL: OnceLock<rayon::ThreadPool> = OnceLock::new();

/// Set the thread count before the first tick (`ostrog --threads N`). A later call, or one
/// after the pool exists, changes nothing. Returns the count in force.
pub fn set_threads(n: usize) -> usize {
    let _ = SETTING.set(n.max(1));
    threads()
}

/// The thread count: the setting, else `HF_SIM_THREADS`, else the machine's cores up to
/// `DEFAULT_MAX_THREADS`.
pub fn threads() -> usize {
    *SETTING.get_or_init(|| {
        std::env::var("HF_SIM_THREADS")
            .ok()
            .and_then(|v| v.trim().parse::<usize>().ok())
            .unwrap_or_else(|| {
                std::thread::available_parallelism()
                    .map(|n| n.get())
                    .unwrap_or(1)
                    .min(DEFAULT_MAX_THREADS)
            })
            .max(1)
    })
}

/// Run `f` in the sim's pool (or the rayon pool the caller is already in).
pub fn install<R: Send>(f: impl FnOnce() -> R + Send) -> R {
    if rayon::current_thread_index().is_some() {
        return f();
    }
    POOL.get_or_init(|| {
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads())
            .thread_name(|i| format!("hf-sim-{i}"))
            .build()
            .expect("the sim's thread pool")
    })
    .install(f)
}

/// How many times `map` has spread work of more than one item (for tests: proof that a run
/// went through the parallel parts at all).
pub static SPREAD: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// `f` over `items` in parallel; the results in the items' order.
pub fn map<T: Sync, R: Send>(items: &[T], f: impl Fn(&T) -> R + Sync + Send) -> Vec<R> {
    // One thread: no pool at all (a pool of one only adds the hand-over).
    if items.len() <= 1 || (rayon::current_thread_index().is_none() && threads() == 1) {
        return items.iter().map(f).collect();
    }
    SPREAD.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    install(|| items.par_iter().map(f).collect())
}

thread_local! {
    /// One A* scratch per thread. A search's result does not depend on what the scratch held
    /// before (it is generation-stamped), so which thread's scratch serves which search does
    /// not matter.
    static SCRATCH: std::cell::RefCell<Option<AStarScratch>> = const { std::cell::RefCell::new(None) };
}

/// The path `World::path_agent_to` would give from `start` to `goal` (without the start
/// tile, nearest step last), or None where it would fail.
pub fn path_between(w: &World, start: TilePos, goal: TilePos) -> Option<Vec<TilePos>> {
    let nav = w.nav.as_ref()?;
    if !nav.reach(start, goal) {
        return None;
    }
    SCRATCH.with(|cell| {
        let mut slot = cell.borrow_mut();
        if !slot.as_ref().is_some_and(|s| s.fits(&w.grid)) {
            *slot = Some(AStarScratch::new(&w.grid));
        }
        let scratch = slot.as_mut().expect("scratch");
        match crate::nav::find_path(
            &w.grid,
            scratch,
            start,
            goal,
            crate::nav::astar::DEFAULT_MAX_EXPANSIONS,
        ) {
            // As `World::path_agent_to` (#418): nobody on dry ground sets out through water over
            // their head. Without this here the walks decide.rs searches in a batch (#428) set
            // off on such a path and were stopped at its first deep tile by movement, failing as
            // "path blocked" (#553, FIXLOG 2026-11-01: 17,682 walks to drink in 200 days on seed 2,
            // most to one place).
            PathResult::Found(p)
                if !w.grid.over_head(start) && p.iter().any(|q| w.grid.over_head(*q)) =>
            {
                None
            }
            PathResult::Found(mut p) => {
                p.reverse();
                p.pop();
                Some(p)
            }
            _ => None,
        }
    })
}

/// Many `path_between` searches at once, in parallel; the answers in the requests' order.
/// Short walks are searched on the calling thread: a search of a few tiles takes
/// microseconds, less than handing it to another thread, and spreading every tick's batch
/// raised the 500-person profile's mean `decide` from 0.20 to 0.55 ms. A batch is spread only
/// when its walks add up to `PATHS_SPREAD_FROM_TILES` tiles or more (the searches that made
/// the worst ticks were a few dozen walks of tens of tiles each).
pub fn paths(w: &World, requests: &[(TilePos, TilePos)]) -> Vec<Option<Vec<TilePos>>> {
    let tiles: i32 = requests.iter().map(|(from, to)| from.dist(*to)).sum();
    if requests.len() < 2 || tiles < PATHS_SPREAD_FROM_TILES {
        return requests
            .iter()
            .map(|(from, to)| path_between(w, *from, *to))
            .collect();
    }
    map(requests, |(from, to)| path_between(w, *from, *to))
}

/// The walk length, summed over a batch, from which its searches are spread across threads.
pub const PATHS_SPREAD_FROM_TILES: i32 = 200;
