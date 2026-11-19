//! The weather ground overlay (#289, DEC-8 items 2+3): what the surface of a tile
//! wears from the weather right now. The client has always drawn these looks
//! (world_view.gd: frozen soil in hard frost, wet soil in rain, snow lying on
//! exposed covers in a cold winter); this module is the simulation's own statement
//! of the same rules, so behaviour can key on what the player sees. Derived from
//! the saved weather alone — nothing new is stored per tile — and keyed on the
//! valley reading exactly as the client's rules are, so the two never disagree.

use crate::ids::TilePos;
use crate::time::Season;
use crate::world::Material;
use crate::World;

/// The hard-frost line, below which the bare earth families freeze. Mirrors the
/// client's `temperature < -1.0` frozen-soil rule.
pub const FROST_C: f32 = -1.0;

/// Covers the snow lies on (the client's SNOWABLE list in material names): the
/// ground families, bare or vegetated, and not rock or wood.
fn snowable(m: Material) -> bool {
    matches!(
        m,
        Material::GRASS
            | Material::SOIL
            | Material::PEAT
            | Material::CLAY
            | Material::SAND
            | Material::GRAVEL
            | Material::MUD
            | Material::MEADOW
            | Material::FROZEN_SOIL
            | Material::MOSS
            | Material::SCREE
            | Material::WET_SOIL
            | Material::PACKED_EARTH
            | Material::BONE_MOSS
    )
}

/// The surface's effective family under the weather: the bare-earth families
/// (soil, clay, wet soil, mud) freeze in a hard frost and turn wet in rain, as the
/// client draws them. Other families, and every cover under snow, keep their
/// stored material — snow is [`snow_lies`], laid over whatever this returns.
pub fn cover_at(w: &World, p: TilePos) -> Material {
    let m = w.grid.get(p).material;
    match m {
        Material::SOIL | Material::CLAY | Material::WET_SOIL | Material::MUD => {
            if w.weather.temperature_c < FROST_C {
                Material::FROZEN_SOIL
            } else if w.weather.sky.wet() && m != Material::MUD {
                Material::WET_SOIL
            } else {
                m
            }
        }
        _ => m,
    }
}

/// Whether snow lies on the tile's cover: a cold winter (below the melt line, or
/// snow falling), an exposed tile, and a cover the snow can hold. Melt order and
/// depth are the client's presentation; here the answer is what behaviour keys on
/// — a snow-covered tile does not carry fire, and the block facts name the snow.
pub fn snow_lies(w: &World, p: TilePos) -> bool {
    if w.calendar().season != Season::Winter {
        return false;
    }
    if w.weather.temperature_c >= 1.0 && w.weather.sky != crate::systems::weather::Sky::Snow {
        return false;
    }
    // Exposure: a roof over the standing tile keeps the cover beneath it clear,
    // as the client's dim test does for the piece above the ground.
    if crate::building::roofed(w, p.offset(0, 0, 1)) {
        return false;
    }
    let m = w.grid.get(p).material;
    snowable(m)
}
