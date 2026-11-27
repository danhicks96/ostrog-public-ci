//! Chunk regions: connected walkable components within each 32×32×1 chunk, linked through
//! portal edges (any step whose destination lies in another chunk, which includes every
//! vertical step). A union-find over regions gives `reach(a, b)` as two lookups.
//! Rebuild is incremental: a changed chunk recomputes its own components and the portal
//! edges of itself and its six neighbours, then the union-find is re-run over all edges.

use std::collections::BTreeMap;

use crate::ids::TilePos;
use crate::world::grid::ChunkKey;
use crate::world::{WorldGrid, CHUNK_SIZE};

const CS: i32 = CHUNK_SIZE as i32;
pub const NO_REGION: u32 = u32::MAX;

#[derive(Clone, Debug)]
pub struct Nav {
    w: i32,
    h: i32,
    zmin: i32,
    zc: i32,
    cw: i32,
    ch: i32,
    /// region id per tile (dense), NO_REGION where not walkable.
    region: Vec<u32>,
    /// portal edges by source chunk key.
    edges: BTreeMap<ChunkKey, Vec<(u32, u32)>>,
    /// Component root per region id (ids are `chunk_linear << 10 | local`, so the table
    /// is dense enough to index directly). NO_REGION where no region has that id.
    comp: Vec<u32>,
    /// Union-find scratch, reset to identity for every rebuild.
    parent: Vec<u32>,
    seen_versions: BTreeMap<ChunkKey, u64>,
    pub rebuilds: u64,
    /// The chunks whose tiles changed in the last `update` (systems/nav.rs drops items whose
    /// floor went with them).
    pub last_dirty: Vec<ChunkKey>,
}

impl Nav {
    pub fn build(grid: &WorldGrid) -> Nav {
        let zc = grid.z_max - grid.z_min + 1;
        let cw = (grid.width + CS - 1) / CS;
        let ch = (grid.height + CS - 1) / CS;
        let parent_len = ((cw * ch * zc) as usize) << 10;
        let mut n = Nav {
            w: grid.width,
            h: grid.height,
            zmin: grid.z_min,
            zc,
            cw,
            ch,
            region: vec![NO_REGION; (grid.width * grid.height * zc) as usize],
            edges: BTreeMap::new(),
            comp: Vec::new(),
            parent: (0..parent_len as u32).collect(),
            seen_versions: BTreeMap::new(),
            rebuilds: 0,
            last_dirty: Vec::new(),
        };
        let mut all: Vec<ChunkKey> = Vec::new();
        for z in grid.z_min..=grid.z_max {
            for cy in 0..n.ch {
                for cx in 0..n.cw {
                    all.push((cx, cy, z));
                }
            }
        }
        n.rebuild_components(grid, &all);
        n.rebuild_edges(grid, &all);
        for (k, c) in &grid.chunks {
            n.seen_versions.insert(*k, c.version);
        }
        n.union_all();
        n
    }

    #[inline]
    fn idx(&self, p: TilePos) -> usize {
        (((p.z - self.zmin) * self.h + p.y) * self.w + p.x) as usize
    }

    #[inline]
    fn chunk_linear(&self, k: ChunkKey) -> u32 {
        (((k.2 - self.zmin) * self.ch + k.1) * self.cw + k.0) as u32
    }

    pub fn region_of(&self, p: TilePos) -> u32 {
        if p.x < 0
            || p.y < 0
            || p.x >= self.w
            || p.y >= self.h
            || p.z < self.zmin
            || p.z >= self.zmin + self.zc
        {
            return NO_REGION;
        }
        self.region[self.idx(p)]
    }

    /// Same connected component?
    pub fn reach(&self, a: TilePos, b: TilePos) -> bool {
        let ra = self.region_of(a);
        let rb = self.region_of(b);
        if ra == NO_REGION || rb == NO_REGION {
            return false;
        }
        if ra == rb {
            return true;
        }
        self.comp[ra as usize] == self.comp[rb as usize]
    }

    pub fn component_of(&self, p: TilePos) -> Option<u32> {
        let r = self.region_of(p);
        if r == NO_REGION {
            None
        } else {
            Some(self.comp[r as usize])
        }
    }

    /// Recompute anything whose chunk changed since the last update. Returns chunks rebuilt.
    pub fn update(&mut self, grid: &WorldGrid) -> usize {
        let mut dirty: Vec<ChunkKey> = Vec::new();
        for (k, c) in &grid.chunks {
            if self.seen_versions.get(k).copied() != Some(c.version) {
                dirty.push(*k);
                self.seen_versions.insert(*k, c.version);
            }
        }
        self.last_dirty = dirty.clone();
        if dirty.is_empty() {
            return 0;
        }
        // A tile is walkable by what is under it as well as what it is: a wall that burns
        // or is dug out takes the footing of the tile above with it. So the chunk above each
        // changed chunk is recomputed too (C34 soaks: food lying on a burned wall's top stayed
        // "reachable" for days and people chose it over the hearth until they froze).
        let mut comps: Vec<ChunkKey> = Vec::new();
        for k in &dirty {
            // Revealing or covering natural water can also change an escape route on
            // the level below. Such liquid floors exist only in allocated chunks.
            let below = (k.0, k.1, k.2 - 1);
            if grid.chunks.get(&below).is_some_and(|c| {
                c.tiles
                    .iter()
                    .any(|t| t.liquid_floor() && !t.flags.has(crate::world::TileFlags::DEEP))
            }) && !comps.contains(&below)
            {
                comps.push(below);
            }
            for up in [*k, (k.0, k.1, k.2 + 1)] {
                if up.2 < self.zmin + self.zc && !comps.contains(&up) {
                    comps.push(up);
                }
            }
        }
        self.rebuild_components(grid, &comps);
        let mut touched: Vec<ChunkKey> = Vec::new();
        for k in &comps {
            for d in [
                (0, 0, 0),
                (1, 0, 0),
                (-1, 0, 0),
                (0, 1, 0),
                (0, -1, 0),
                (0, 0, 1),
                (0, 0, -1),
            ] {
                let n = (k.0 + d.0, k.1 + d.1, k.2 + d.2);
                if n.0 >= 0
                    && n.1 >= 0
                    && n.0 < self.cw
                    && n.1 < self.ch
                    && n.2 >= self.zmin
                    && n.2 < self.zmin + self.zc
                    && !touched.contains(&n)
                {
                    touched.push(n);
                }
            }
        }
        self.rebuild_edges(grid, &touched);
        self.union_all();
        self.rebuilds += 1;
        dirty.len()
    }

    /// Recompute the regions of `chunks`, each chunk on its own (#428: in parallel, the
    /// results written in the order given; a chunk's regions depend only on its own tiles).
    fn rebuild_components(&mut self, grid: &WorldGrid, chunks: &[ChunkKey]) {
        let labels = crate::par::map(chunks, |k| self.chunk_components(grid, *k));
        for (k, local) in chunks.iter().zip(labels) {
            let (x0, y0) = (k.0 * CS, k.1 * CS);
            for ly in 0..CS {
                for lx in 0..CS {
                    let p = TilePos::new(x0 + lx, y0 + ly, k.2);
                    if p.x < self.w && p.y < self.h {
                        let i = self.idx(p);
                        self.region[i] = local[(ly * CS + lx) as usize];
                    }
                }
            }
        }
    }

    /// The region id of each tile of chunk `k` (row by row, `NO_REGION` where not walkable),
    /// numbered in scan order.
    ///
    /// The flood fill answers its walkability questions from a bitmap of the chunk built
    /// once per call (WP-01 1.4): the previous version asked `neighbours()` per tile, a
    /// dozen grid reads per pop, and the nightly freeze wave rebuilds some fifty chunks
    /// in one tick. The bitmap is filled with the same `grid.walkable` calls on the same
    /// tiles, and the ortho/diagonal corner-cut rules below are `neighbours`' horizontal
    /// section verbatim, so the component ids are unchanged.
    fn chunk_components(&self, grid: &WorldGrid, k: ChunkKey) -> Vec<u32> {
        const N: usize = (CS * CS) as usize;
        let base = self.chunk_linear(k) << 10;
        let x0 = k.0 * CS;
        let y0 = k.1 * CS;
        let mut walk = [false; N];
        for ly in 0..CS {
            for lx in 0..CS {
                let p = TilePos::new(x0 + lx, y0 + ly, k.2);
                if p.x < self.w && p.y < self.h {
                    walk[(ly * CS + lx) as usize] = grid.walkable(p);
                }
            }
        }
        let mut local_region = vec![NO_REGION; N];
        let mut local: u32 = 0;
        let mut stack: Vec<(i32, i32)> = Vec::new();
        for ly in 0..CS {
            for lx in 0..CS {
                let p = TilePos::new(x0 + lx, y0 + ly, k.2);
                if p.x >= self.w || p.y >= self.h {
                    continue;
                }
                if local_region[(ly * CS + lx) as usize] != NO_REGION
                    || !walk[(ly * CS + lx) as usize]
                {
                    continue;
                }
                let id = base | local;
                local += 1;
                local_region[(ly * CS + lx) as usize] = id;
                stack.push((lx, ly));
                while let Some((cx, cy)) = stack.pop() {
                    // Same-z ortho steps, in `neighbours`' order; out-of-chunk sides
                    // count as unwalkable, as the grid answer was.
                    let mut ortho_ok = [false; 4];
                    for (i, (dx, dy)) in [(0, -1), (1, 0), (0, 1), (-1, 0)].iter().enumerate() {
                        let (nx, ny) = (cx + dx, cy + dy);
                        if nx >= 0 && ny >= 0 && nx < CS && ny < CS {
                            let at = (ny * CS + nx) as usize;
                            ortho_ok[i] = walk[at];
                            if walk[at] && local_region[at] == NO_REGION {
                                local_region[at] = id;
                                stack.push((nx, ny));
                            }
                        }
                    }
                    // Diagonals do not cut corners: only where both flankers walk.
                    for ((dx, dy), (a, b)) in [
                        ((1, -1), (0, 1)),
                        ((1, 1), (1, 2)),
                        ((-1, 1), (2, 3)),
                        ((-1, -1), (3, 0)),
                    ] {
                        if ortho_ok[a] && ortho_ok[b] {
                            let (nx, ny) = (cx + dx, cy + dy);
                            if nx >= 0 && ny >= 0 && nx < CS && ny < CS {
                                let at = (ny * CS + nx) as usize;
                                if walk[at] && local_region[at] == NO_REGION {
                                    local_region[at] = id;
                                    stack.push((nx, ny));
                                }
                            }
                        }
                    }
                }
            }
        }
        local_region
    }

    /// Recompute the portal edges of `chunks` (#428: in parallel, from the regions as they
    /// stand, stored in the order given).
    fn rebuild_edges(&mut self, grid: &WorldGrid, chunks: &[ChunkKey]) {
        let edges = crate::par::map(chunks, |k| self.chunk_edges(grid, *k));
        for (k, es) in chunks.iter().zip(edges) {
            self.edges.insert(*k, es);
        }
    }

    fn chunk_edges(&self, grid: &WorldGrid, k: ChunkKey) -> Vec<(u32, u32)> {
        const N: usize = (CS * CS) as usize;
        let x0 = k.0 * CS;
        let y0 = k.1 * CS;
        // Same bitmap as `chunk_components` (WP-01 1.4): the horizontal step questions
        // are answered from it; only genuinely cross-chunk or vertical targets touch the
        // grid. The kept step set is `neighbours`' exactly, so the edge list — sorted
        // and deduped below — is unchanged.
        let mut walk = [false; N];
        for ly in 0..CS {
            for lx in 0..CS {
                let p = TilePos::new(x0 + lx, y0 + ly, k.2);
                if p.x < self.w && p.y < self.h {
                    walk[(ly * CS + lx) as usize] = grid.walkable(p);
                }
            }
        }
        let mut edges: Vec<(u32, u32)> = Vec::new();
        for ly in 0..CS {
            for lx in 0..CS {
                let p = TilePos::new(x0 + lx, y0 + ly, k.2);
                if p.x >= self.w || p.y >= self.h {
                    continue;
                }
                let r = self.region[self.idx(p)];
                if r == NO_REGION {
                    continue;
                }
                // Horizontal steps; the in-chunk ones are dropped by the original filter,
                // so only border sides can produce an edge.
                let mut ortho_ok = [false; 4];
                for (i, (dx, dy)) in [(0, -1), (1, 0), (0, 1), (-1, 0)].iter().enumerate() {
                    let (nx, ny) = (lx + dx, ly + dy);
                    let in_chunk = nx >= 0 && ny >= 0 && nx < CS && ny < CS;
                    ortho_ok[i] = if in_chunk {
                        walk[(ny * CS + nx) as usize]
                    } else {
                        grid.walkable(p.offset(*dx, *dy, 0))
                    };
                    if !in_chunk && ortho_ok[i] {
                        let q = p.offset(*dx, *dy, 0);
                        let rq = self.region_of(q);
                        if rq != NO_REGION {
                            edges.push((r, rq));
                        }
                    }
                }
                // Diagonals do not cut corners.
                for ((dx, dy), (a, b)) in [
                    ((1, -1), (0, 1)),
                    ((1, 1), (1, 2)),
                    ((-1, 1), (2, 3)),
                    ((-1, -1), (3, 0)),
                ] {
                    if ortho_ok[a] && ortho_ok[b] {
                        let q = p.offset(dx, dy, 0);
                        let in_chunk = q.x >= x0 && q.y >= y0 && q.x < x0 + CS && q.y < y0 + CS;
                        if !in_chunk && grid.walkable(q) {
                            let rq = self.region_of(q);
                            if rq != NO_REGION {
                                edges.push((r, rq));
                            }
                        }
                    }
                }
                // Vertical steps and ramps: `neighbours`' z-section verbatim (step.rs);
                // these need the neighbouring level's tiles, so the grid calls stay.
                let here = grid.get(p);
                {
                    let q = p.offset(0, 0, 1);
                    if grid.in_bounds(q) {
                        let t = grid.get(q);
                        let linked = here.shape.connects_up() || t.shape.connects_down();
                        if linked && (grid.walkable(q) || t.shape.connects_down()) {
                            let rq = self.region_of(q);
                            if rq != NO_REGION {
                                edges.push((r, rq));
                            }
                        }
                    }
                }
                {
                    let q = p.offset(0, 0, -1);
                    if grid.in_bounds(q) {
                        let t = grid.get(q);
                        let linked = here.shape.connects_down() || t.shape.connects_up();
                        if linked && (grid.walkable(q) || t.shape.connects_up()) {
                            let rq = self.region_of(q);
                            if rq != NO_REGION {
                                edges.push((r, rq));
                            }
                        }
                    }
                }
                if here.shape == crate::world::Shape::RampUp {
                    for (dx, dy) in [(0, -1), (1, 0), (0, 1), (-1, 0)] {
                        let q = p.offset(dx, dy, 1);
                        if grid.walkable(q) {
                            let rq = self.region_of(q);
                            if rq != NO_REGION {
                                edges.push((r, rq));
                            }
                        }
                    }
                }
                for (dx, dy) in [(0, -1), (1, 0), (0, 1), (-1, 0)] {
                    let q = p.offset(dx, dy, -1);
                    if grid.in_bounds(q)
                        && grid.get(q).shape == crate::world::Shape::RampUp
                        && grid.walkable(q)
                    {
                        let rq = self.region_of(q);
                        if rq != NO_REGION {
                            edges.push((r, rq));
                        }
                    }
                }
            }
        }
        edges.sort_unstable();
        edges.dedup();
        edges
    }

    /// Rebuild the component table from the portal edges with an array union-find.
    /// (A BTreeMap union-find here cost tens of milliseconds per rebuild in the soak.)
    fn union_all(&mut self) {
        let n = ((self.cw * self.ch * self.zc) as usize) << 10;
        debug_assert_eq!(self.parent.len(), n);
        // FIXLOG 2026-10-31 #428: reused scratch must not carry links into a dirty update.
        for (i, p) in self.parent.iter_mut().enumerate() {
            *p = i as u32;
        }
        fn find(parent: &mut [u32], mut x: u32) -> u32 {
            while parent[x as usize] != x {
                let p = parent[x as usize];
                parent[x as usize] = parent[p as usize];
                x = p;
            }
            x
        }
        for es in self.edges.values() {
            for (a, b) in es {
                let ra = find(&mut self.parent, *a);
                let rb = find(&mut self.parent, *b);
                if ra != rb {
                    let (lo, hi) = if ra < rb { (ra, rb) } else { (rb, ra) };
                    self.parent[hi as usize] = lo;
                }
            }
        }
        let mut comp = vec![NO_REGION; n];
        for r in self.region.iter().copied() {
            if r != NO_REGION && comp[r as usize] == NO_REGION {
                comp[r as usize] = find(&mut self.parent, r);
            }
        }
        self.comp = comp;
    }

    pub fn region_count(&self) -> usize {
        self.comp.iter().filter(|c| **c != NO_REGION).count()
    }
    pub fn component_count(&self) -> usize {
        self.comp
            .iter()
            .filter(|c| **c != NO_REGION)
            .collect::<std::collections::BTreeSet<_>>()
            .len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::grid::{reset_walkable_reads, walkable_reads};

    #[test]
    fn union_all_refills_poisoned_parent_scratch() {
        let grid = WorldGrid::new(40, 40, -1, 1);
        let expected = Nav::build(&grid);
        let mut actual = Nav::build(&grid);
        actual.parent.fill(0);
        actual.union_all();
        assert_eq!(actual.parent, expected.parent);
        assert_eq!(actual.comp, expected.comp);
    }

    #[test]
    fn chunk_components_reads_each_tile_once_for_walkability() {
        let grid = WorldGrid::new(32, 32, 0, 1);
        let nav = Nav::build(&grid);

        reset_walkable_reads();
        nav.chunk_components(&grid, (0, 0, 1));

        assert_eq!(walkable_reads(), 1024);
    }
}
