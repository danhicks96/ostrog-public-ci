//! Bounded 3D A*. Scratch arrays are reused between searches (generation-stamped), so a
//! search allocates nothing after warm-up.

use std::cmp::Reverse;
use std::collections::{BTreeSet, BinaryHeap};

use crate::ids::TilePos;
use crate::world::WorldGrid;

use super::step::{neighbours, Step};

pub const DEFAULT_MAX_EXPANSIONS: u32 = 12_000;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PathResult {
    /// Path including start and goal.
    Found(Vec<TilePos>),
    /// Goal provably unreachable (search exhausted).
    Unreachable,
    /// Expansion budget exceeded.
    Budget,
}

#[derive(Clone, Debug)]
pub struct AStarScratch {
    gen: u32,
    stamp: Vec<u32>,
    g: Vec<u32>,
    parent: Vec<u32>,
    closed: Vec<u32>,
    dims: (i32, i32, i32, i32), // w, h, zmin, zcount
    nb: Vec<Step>,
    /// Exact failed searches remain false until the navigation regions change. This is a
    /// transient cost cache: agents are not obstacles, so an unchanged grid cannot make the
    /// same start/goal pair newly reachable.
    failed_paths: BTreeSet<(TilePos, TilePos)>,
}

impl AStarScratch {
    pub fn new(grid: &WorldGrid) -> Self {
        let zc = grid.z_max - grid.z_min + 1;
        let n = (grid.width * grid.height * zc) as usize;
        AStarScratch {
            gen: 0,
            stamp: vec![0; n],
            g: vec![0; n],
            parent: vec![u32::MAX; n],
            closed: vec![0; n],
            dims: (grid.width, grid.height, grid.z_min, zc),
            nb: Vec::with_capacity(16),
            failed_paths: BTreeSet::new(),
        }
    }

    pub fn failed_path(&self, start: TilePos, goal: TilePos) -> bool {
        self.failed_paths.contains(&(start, goal))
    }

    pub fn remember_failed_path(&mut self, start: TilePos, goal: TilePos) {
        self.failed_paths.insert((start, goal));
    }

    pub fn clear_failed_paths(&mut self) {
        self.failed_paths.clear();
    }

    #[doc(hidden)]
    pub fn failed_path_count(&self) -> usize {
        self.failed_paths.len()
    }

    /// Whether this scratch was made for a grid of `grid`'s size.
    pub fn fits(&self, grid: &WorldGrid) -> bool {
        self.dims
            == (
                grid.width,
                grid.height,
                grid.z_min,
                grid.z_max - grid.z_min + 1,
            )
    }

    #[inline]
    pub fn idx(&self, p: TilePos) -> usize {
        let (w, h, zmin, _) = self.dims;
        (((p.z - zmin) * h + p.y) * w + p.x) as usize
    }

    #[inline]
    fn pos(&self, i: usize) -> TilePos {
        let (w, h, zmin, _) = self.dims;
        let i = i as i32;
        TilePos::new(i % w, (i / w) % h, i / (w * h) + zmin)
    }
}

fn heuristic(a: TilePos, b: TilePos) -> u32 {
    let dx = (a.x - b.x).abs();
    let dy = (a.y - b.y).abs();
    let dz = (a.z - b.z).abs();
    let diag = dx.min(dy);
    let straight = dx.max(dy) - diag;
    (straight * 4 + diag * 5 + dz * 8) as u32
}

pub fn find_path(
    grid: &WorldGrid,
    scratch: &mut AStarScratch,
    start: TilePos,
    goal: TilePos,
    max_expansions: u32,
) -> PathResult {
    if !grid.in_bounds(start) || !grid.in_bounds(goal) || !grid.walkable(goal) {
        return PathResult::Unreachable;
    }
    if start == goal {
        return PathResult::Found(vec![start]);
    }
    scratch.gen = scratch.gen.wrapping_add(1);
    if scratch.gen == 0 {
        scratch.stamp.iter_mut().for_each(|s| *s = 0);
        scratch.closed.iter_mut().for_each(|s| *s = 0);
        scratch.gen = 1;
    }
    let gen = scratch.gen;
    let mut open: BinaryHeap<Reverse<(u32, u32, u32)>> = BinaryHeap::new();
    let si = scratch.idx(start);
    scratch.stamp[si] = gen;
    scratch.g[si] = 0;
    scratch.parent[si] = u32::MAX;
    open.push(Reverse((heuristic(start, goal), 0, si as u32)));
    let mut expansions = 0;
    let mut nb = std::mem::take(&mut scratch.nb);
    let result = loop {
        let Some(Reverse((_, g_here, ci))) = open.pop() else {
            break PathResult::Unreachable;
        };
        let ci = ci as usize;
        if scratch.closed[ci] == gen {
            continue;
        }
        if scratch.stamp[ci] == gen && scratch.g[ci] < g_here {
            continue;
        }
        scratch.closed[ci] = gen;
        let cp = scratch.pos(ci);
        if cp == goal {
            let mut path = vec![cp];
            let mut i = ci;
            while scratch.parent[i] != u32::MAX {
                i = scratch.parent[i] as usize;
                path.push(scratch.pos(i));
            }
            path.reverse();
            break PathResult::Found(path);
        }
        expansions += 1;
        if expansions > max_expansions {
            break PathResult::Budget;
        }
        neighbours(grid, cp, &mut nb);
        for s in &nb {
            let ni = scratch.idx(s.to);
            if scratch.closed[ni] == gen {
                continue;
            }
            let ng = g_here + s.cost;
            if scratch.stamp[ni] != gen || ng < scratch.g[ni] {
                scratch.stamp[ni] = gen;
                scratch.g[ni] = ng;
                scratch.parent[ni] = ci as u32;
                open.push(Reverse((ng + heuristic(s.to, goal), ng, ni as u32)));
            }
        }
    };
    scratch.nb = nb;
    result
}
