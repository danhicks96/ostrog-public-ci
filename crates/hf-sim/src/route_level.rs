use super::{Overworld, Site, SiteKind};
use crate::{
    items::ItemKind,
    rng::hash64,
    time::{Calendar, Season},
};

/// A route tile is 32 road units. Coordinates belong to the route in both directions.
pub const TILE: i64 = 32;
pub const RADIUS_X: i32 = 28;
pub const RADIUS_Y: i32 = 18;

pub fn route_seed(world_seed: u64, site: &Site) -> u64 {
    hash64(&[
        world_seed,
        crate::rng::purpose::WORLDGEN_FLORA,
        site.id as u64,
    ])
}

pub fn path_y(seed: u64, x: i32) -> i32 {
    let block = x.div_euclid(8);
    let offset = x.rem_euclid(8);
    let bend = |i: i32| (hash64(&[seed, i as u64]) % 7) as i32 - 3;
    (bend(block) * (8 - offset) + bend(block + 1) * offset) / 8
}

// Ground palette: meadow, track, moss, scree, shallows, mud, wildflowers.
pub fn tile(seed: u64, site: &Site, x: i32, y: i32) -> (i32, i32, u32) {
    let h = hash64(&[seed, x as u64, y as u64]);
    use crate::world::gen::noise::value2;
    let wet = value2(seed ^ 0xa11, x as f32 * 0.12, y as f32 * 0.15);
    let outcrop = value2(seed ^ 0x91, x as f32 * 0.16, y as f32 * 0.11);
    let trees = value2(seed ^ 0xb5, x as f32 * 0.18, y as f32 * 0.18);
    let distance = (y - path_y(seed, x)).abs();
    let ground = if distance == 0 {
        1
    } else if distance > 3
        && wet > 0.77
        && matches!(
            site.kind,
            SiteKind::Village | SiteKind::Lakeshore | SiteKind::Forest
        )
    {
        4
    } else if wet > 0.68 {
        5
    } else if outcrop
        > if matches!(site.kind, SiteKind::Mine | SiteKind::Ruin) {
            0.58
        } else {
            0.81
        }
    {
        3
    } else if wet > 0.56 && trees > 0.4 {
        2
    } else if trees < 0.26 && wet > 0.24 {
        6
    } else {
        0
    };
    // Trees, bracken, berry bushes and roots occur in patches off the walkable track.
    let plant = if distance == 0 || ground == 4 {
        0
    } else if distance == 1 && h % 4 < 2 {
        5 + (h % 2) as i32
    } else if distance > 1 && ground != 3 && (h % 100) < (trees * trees * 55.0) as u64 {
        1 + ((h >> 16) % 4) as i32
    } else {
        0
    };
    (ground, plant, 1 + ((h >> 32) % 4) as u32)
}

pub fn available(
    overworld: &Overworld,
    site: u32,
    x: i32,
    y: i32,
    calendar: Calendar,
    stock: u32,
) -> u32 {
    let used = overworld
        .picked
        .get(&(site, x, y))
        .filter(|(year, _)| *year == calendar.year)
        .map_or(0, |(_, n)| *n);
    stock.saturating_sub(used)
}

/// Gather from the patches alongside this portion of the track, shared by outgoing
/// and returning parties. Only picked plants need to be kept in the save.
pub fn harvest(
    overworld: &mut Overworld,
    seed: u64,
    site: &Site,
    road: i64,
    calendar: Calendar,
    food: ItemKind,
    want: u32,
) -> u32 {
    let plant = match (calendar.season, food) {
        (Season::Summer, ItemKind::Berries) => 5,
        (Season::Autumn, ItemKind::Root) => 6,
        _ => return 0,
    };
    let centre = (road / TILE) as i32;
    let mut gathered = 0;
    for dx in [0, -1, 1, -2, 2, -3, 3] {
        let x = centre + dx;
        for y in [path_y(seed, x) - 1, path_y(seed, x) + 1] {
            let (_, kind, stock) = tile(seed, site, x, y);
            if kind != plant {
                continue;
            }
            let left = available(overworld, site.id, x, y, calendar, stock);
            let take = left.min(want - gathered);
            if take > 0 {
                overworld
                    .picked
                    .insert((site.id, x, y), (calendar.year, stock - left + take));
                gathered += take;
            }
            if gathered == want {
                return gathered;
            }
        }
    }
    gathered
}
