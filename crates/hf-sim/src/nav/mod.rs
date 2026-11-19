//! Navigation over the 3D grid: a shared neighbour function, bounded A*, and chunk regions
//! joined through portals so `reach(a, b)` is a cheap lookup and unreachable targets are
//! filtered before anyone plans toward them.

pub mod astar;
pub mod regions;
pub mod step;

pub use astar::{find_path, AStarScratch, PathResult};
pub use regions::Nav;
pub use step::{neighbours, step_cost, Step};
