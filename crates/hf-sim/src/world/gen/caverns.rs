//! Caverns: great chambers in the middle stratum, two levels high, where the worm caves open
//! out. Some hold a still pool in their lowest part. Each is named, so its discovery can be.
//! Their floors carry fungus forests (flora.rs).

use rand::Rng;
use serde::{Deserialize, Serialize};

use super::GenParams;
use crate::ids::TilePos;
use crate::rng::{purpose, stream};
use crate::world::{Material, Shape, Tile, TileFlags, WorldGrid};

/// How much of the floor's extent the upper level spans.
const UPPER: f32 = 0.82;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Cavern {
    pub name: String,
    /// The middle of its floor level (the lower of its two open levels).
    pub centre: TilePos,
    pub rx: i32,
    pub ry: i32,
    /// Whether a pool lies in it.
    pub pool: bool,
    /// A deep cavern, in the basalt (P10), where the deep's peoples and beasts live.
    #[serde(default)]
    pub deep: bool,
    /// Someone of the valley has stood in it (P10): its breach is recorded once.
    #[serde(default)]
    pub opened: bool,
    /// A deep cavern where the underworld's stone shows in the walls (P11): to stand first
    /// in it is to touch it.
    #[serde(default)]
    pub underworld: bool,
    /// #273: the tile the valley broke in by, once it has.
    #[serde(default)]
    pub breach: Option<TilePos>,
    /// #273: the breach walled up again.
    #[serde(default)]
    pub sealed: bool,
    /// #322: the community whose people broke in, and so whose way down it is.
    #[serde(default)]
    pub gate_of: Option<crate::ids::CommunityId>,
    /// #322: who walled the breach, and so keeps the way: the one who may open it.
    #[serde(default)]
    pub keeper: Option<crate::ids::AgentId>,
}

impl Cavern {
    /// Whether a tile is inside the chamber (either level).
    pub fn contains(&self, p: TilePos) -> bool {
        // The upper level is narrower: the walls are ragged at the foot.
        (p.z == self.centre.z && self.inside(p.x, p.y, 1.0))
            || (p.z == self.centre.z + 1 && self.inside(p.x, p.y, UPPER))
    }
    fn inside(&self, x: i32, y: i32, k: f32) -> bool {
        let dx = (x - self.centre.x) as f32 / (self.rx as f32 * k);
        let dy = (y - self.centre.y) as f32 / (self.ry as f32 * k);
        dx * dx + dy * dy <= 1.0
    }
}

fn name(r: &mut impl Rng) -> String {
    let a = [
        "Glimmering",
        "Pale",
        "Echoing",
        "Drowned",
        "Sunken",
        "Dripping",
        "Whispering",
        "Long",
        "Lantern",
        "Old",
        "Silent",
        "Mothwing",
    ];
    let b = ["Hollow", "Vault", "Deep", "Hall", "Grotto", "Undercroft"];
    format!(
        "the {} {}",
        a[r.gen_range(0..a.len())],
        b[r.gen_range(0..b.len())]
    )
}

pub fn apply(seed: u64, p: &GenParams, grid: &mut WorldGrid) {
    if p.cave_density <= 0.0 {
        return;
    }
    let (top, deep) = super::strata::breaks(p);
    // The middle stratum, leaving a solid floor below and rock above the upper level.
    let (zlo, zhi) = (deep + 1, top - 2);
    if zhi < zlo {
        return;
    }
    let mut r = stream(seed, &[purpose::WORLDGEN_CAVES, 0xCA7E]);
    let want = ((p.width * p.height) / 4000).clamp(2, 5);
    let mut tries = 0;
    while (grid.caverns.len() as i32) < want && tries < 60 {
        tries += 1;
        let rx = r.gen_range(7..=12).min(p.width / 6);
        let ry = r.gen_range(6..=10).min(p.height / 6);
        let cx = r.gen_range(rx + 2..p.width - rx - 2);
        let cy = r.gen_range(ry + 2..p.height - ry - 2);
        let z = r.gen_range(zlo..=zhi);
        let mut cname = name(&mut r);
        while grid.caverns.iter().any(|o| o.name == cname) {
            cname = name(&mut r);
        }
        let c = Cavern {
            name: cname,
            centre: TilePos::new(cx, cy, z),
            rx,
            ry,
            pool: r.gen_bool(0.45),
            deep: false,
            opened: false,
            breach: None,
            sealed: false,
            gate_of: None,
            keeper: None,
            underworld: false,
        };
        // Apart from the others, and with rock enough above everywhere.
        if grid.caverns.iter().any(|o| {
            (o.centre.x - cx).abs() < o.rx + rx + 4 && (o.centre.y - cy).abs() < o.ry + ry + 4
        }) {
            continue;
        }
        let deep_enough = (cy - ry..=cy + ry)
            .all(|y| (cx - rx..=cx + rx).all(|x| grid.ground_z(x, y) - (z + 1) >= 3));
        if !deep_enough {
            continue;
        }
        carve(grid, &c);
        grid.caverns.push(c);
    }
    deep_caverns(seed, p, grid);
}

/// One or two deep caverns in the basalt (P10), below the others, on a stream of their own so
/// the middle stratum's caverns are where they were.
fn deep_caverns(seed: u64, p: &GenParams, grid: &mut WorldGrid) {
    let (_, deep) = super::strata::breaks(p);
    let (zlo, zhi) = (p.z_min + 2, deep - 2);
    if zhi < zlo {
        return;
    }
    let mut r = stream(seed, &[purpose::WORLDGEN_CAVES, 0xDEE9]);
    let want = r.gen_range(1..=2);
    let mut made = 0;
    let mut tries = 0;
    while made < want && tries < 40 {
        tries += 1;
        let rx = r.gen_range(6..=10).min(p.width / 6);
        let ry = r.gen_range(5..=8).min(p.height / 6);
        if rx < 4 || ry < 4 {
            return;
        }
        let cx = r.gen_range(rx + 2..p.width - rx - 2);
        let cy = r.gen_range(ry + 2..p.height - ry - 2);
        let z = r.gen_range(zlo..=zhi);
        let mut cname = name(&mut r);
        while grid.caverns.iter().any(|o| o.name == cname) {
            cname = name(&mut r);
        }
        let c = Cavern {
            name: cname,
            centre: TilePos::new(cx, cy, z),
            rx,
            ry,
            pool: r.gen_bool(0.3),
            deep: true,
            opened: false,
            breach: None,
            sealed: false,
            gate_of: None,
            keeper: None,
            underworld: false,
        };
        if grid.caverns.iter().any(|o| {
            o.centre.z - z <= 3
                && (o.centre.x - cx).abs() < o.rx + rx + 4
                && (o.centre.y - cy).abs() < o.ry + ry + 4
        }) {
            continue;
        }
        carve(grid, &c);
        grid.caverns.push(c);
        made += 1;
    }
    // The lowest deep cavern shows the underworld's stone, one world in two.
    if made > 0 && r.gen_bool(0.5) {
        if let Some(c) = grid
            .caverns
            .iter_mut()
            .filter(|c| c.deep)
            .min_by_key(|c| (c.centre.z, c.centre.x, c.centre.y))
        {
            c.underworld = true;
        }
    }
}

fn carve(grid: &mut WorldGrid, c: &Cavern) {
    let (cx, cy, z, rx, ry) = (c.centre.x, c.centre.y, c.centre.z, c.rx, c.ry);
    {
        for y in cy - ry..=cy + ry {
            for x in cx - rx..=cx + rx {
                if !c.inside(x, y, 1.0) {
                    continue;
                }
                // Ragged walls: the outer ring only on the lower level.
                let upper = c.inside(x, y, UPPER);
                for zz in [z, z + 1] {
                    if zz == z + 1 && !upper {
                        continue;
                    }
                    grid.set(
                        TilePos::new(x, y, zz),
                        Tile {
                            material: Material::AIR,
                            shape: Shape::Open,
                            flags: TileFlags::NONE,
                        },
                    );
                }
                // The pool fills a still basin in the middle of the floor.
                if c.pool && c.inside(x, y, 0.38) {
                    grid.set(
                        TilePos::new(x, y, z),
                        Tile {
                            material: Material::WATER,
                            shape: Shape::Open,
                            flags: TileFlags(TileFlags::WATER),
                        },
                    );
                }
            }
        }
    }
}
