//! World generation pipeline: heightmap â†’ strata â†’ hydrology â†’ caves â†’ ores â†’ climate and
//! biomes â†’ surface ground â†’ water depth â†’ ramps â†’ cave mouths â†’ flora.
//! Every stage draws from its own purpose-tagged stream so stages can be tuned independently.

pub mod biome;
pub mod caverns;
pub mod caves;
pub mod flora;
pub mod heightmap;
pub mod hydrology;
pub mod mouths;
pub mod noise;
pub mod ores;
pub mod ramps;
pub mod scree;
pub mod strata;
pub mod watershed;

use serde::{Deserialize, Serialize};

use super::WorldGrid;
use crate::plants::Plant;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GenParams {
    pub width: i32,
    pub height: i32,
    pub z_min: i32,
    pub z_max: i32,
    /// Vertical relief of the surface in z-levels (surface spans ground 0..=relief).
    pub relief: i32,
    pub sea_level_fraction: f32,
    pub cave_density: f32,
    pub tree_density: f32,
    /// The world's character (wet/dry, warm/cold, stony/soft). None: drawn from the seed.
    #[serde(default)]
    pub character: Option<biome::Character>,
    /// The lie of the land (#352): a valley unless the setup draws another.
    #[serde(default)]
    pub landform: Landform,
    /// Which place in the wider world the ground is drawn for (#560): 0 is the seed's own
    /// ground, any other value a different ground for that place (setup.rs `place_of`). Left
    /// out of a save at 0, so every world that keeps the seed's ground keeps its digest.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub place: u64,
    /// River crossings inherited from the selected realm region. Older saves keep their grid.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub river: Option<watershed::RiverReach>,
    /// A selected region with no mapped river or upstream inflow. Local lakes can still form.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub drainage_divide: bool,
    /// Directions of ocean neighbors on the regional map, including diagonal corners.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ocean: Vec<(i8, i8)>,
}

/// The lie of the land at the landing (#352).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Landform {
    #[default]
    Valley,
    /// Low and even.
    Plain,
    /// Raised flat ground with falls at its edges.
    Plateau,
    /// A long high back of land across the map.
    Ridge,
    /// The sea along one side.
    Coast,
    /// Low wet ground near the water's level.
    Marsh,
}

impl Landform {
    pub fn name(self) -> &'static str {
        match self {
            Landform::Valley => "a valley",
            Landform::Plain => "a plain",
            Landform::Plateau => "a plateau",
            Landform::Ridge => "a ridge",
            Landform::Coast => "a coast",
            Landform::Marsh => "a marsh",
        }
    }
    /// Drawn from the seed: a valley half the time, each other a tenth.
    pub fn of_seed(seed: u64) -> Landform {
        match crate::rng::hash64(&[seed, 0x352]) % 10 {
            0 => Landform::Plain,
            1 => Landform::Plateau,
            2 => Landform::Ridge,
            3 => Landform::Coast,
            4 => Landform::Marsh,
            _ => Landform::Valley,
        }
    }
}

impl Default for GenParams {
    fn default() -> Self {
        GenParams {
            width: 128,
            height: 128,
            z_min: -16,
            z_max: 8,
            relief: 4,
            sea_level_fraction: 0.22,
            cave_density: 0.5,
            tree_density: 0.35,
            character: None,
            landform: Landform::Valley,
            place: 0,
            river: None,
            drainage_divide: false,
            ocean: Vec::new(),
        }
    }
}

impl GenParams {
    /// Normalized distance from a column to the nearest ocean-facing edge or corner.
    /// An uncharted, explicitly chosen coast retains its original west-facing profile.
    fn shore_distance(&self, x: f32, y: f32) -> f32 {
        let to_edge = |v: f32, side: i8| match side {
            -1 => v,
            1 => 1.0 - v,
            _ => 0.0,
        };
        self.ocean
            .iter()
            .map(|&(dx, dy)| to_edge(x, dx).hypot(to_edge(y, dy)))
            .reduce(f32::min)
            .unwrap_or(x)
    }

    pub fn small() -> Self {
        GenParams {
            width: 64,
            height: 64,
            z_min: -8,
            z_max: 6,
            relief: 3,
            ..Default::default()
        }
    }
}

pub struct Generated {
    pub grid: WorldGrid,
    pub plants: Vec<Plant>,
    pub flora_draw: flora::FloraDraw,
    /// A good founding site: open, near water and trees, on flat ground.
    pub founding_site: crate::ids::TilePos,
    /// The cave mouths cut into hill faces (#482). Not saved: they are in the grid as tiles.
    pub cave_mouths: Vec<mouths::Mouth>,
}

fn is_zero(v: &u64) -> bool {
    *v == 0
}

/// The seed the ground of a place is drawn from: the world's own seed for place 0, so every
/// world made before the place shaped the ground keeps it (#560), else one mixed from both.
pub fn ground_seed(seed: u64, place: u64) -> u64 {
    if place == 0 {
        seed
    } else {
        crate::rng::hash64(&[seed, 0x560, place])
    }
}

pub fn generate(seed: u64, p: &GenParams) -> Generated {
    // The world's own draw of wild kinds (D65) is the world's, wherever in it the valley is.
    let flora_draw = flora::FloraDraw::of_seed(seed);
    // FIXLOG 2026-11-02 (#560): the ground follows the chosen place, not the seed alone.
    let seed = ground_seed(seed, p.place);
    let mut grid = WorldGrid::new(p.width, p.height, p.z_min, p.z_max);
    heightmap::apply(seed, p, &mut grid);
    let river = p
        .river
        .as_ref()
        .map(|reach| reach.carve(seed, p, &mut grid));
    strata::apply(seed, p, &mut grid);
    let ch = biome::Character::resolve(seed, p);
    let water = hydrology::apply_with_channel(seed, p, &mut grid, &ch, river.as_deref());
    caves::apply(seed, p, &mut grid);
    caverns::apply(seed, p, &mut grid);
    ores::apply(seed, p, &mut grid);
    let climate = biome::climate(seed, p, &grid, &water);
    biome::surface(seed, &mut grid, &water, &climate);
    grid.biome = climate.biome.clone();
    // Loose rock on steep bare ground (#289, DEC-8 item 4), after the families are
    // laid so it reads them, before the water deepens and the ramps and plants come.
    scree::apply(seed, p, &mut grid);
    hydrology::deepen(&mut grid, &water, &climate.biome);
    ramps::apply(p, &mut grid, &water);
    let cave_mouths = mouths::apply(seed, p, &mut grid, &water);
    let plants = flora::apply(seed, p, &grid, &water, &climate, &flora_draw);
    let founding_site = flora::founding_site(seed, &grid, &water, &plants);
    Generated {
        grid,
        plants,
        flora_draw,
        founding_site,
        cave_mouths,
    }
}
