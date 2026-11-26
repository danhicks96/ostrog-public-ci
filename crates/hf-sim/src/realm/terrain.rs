use super::{RBiome, RealmMap};
use crate::world::gen::Landform;

/// Resolve the local terrain from the surrounding regional relief. Adjacent oceans
/// also locate the shore on the local map; a lake is not an ocean coast.
pub fn at(m: &RealmMap, region: usize) -> (Landform, Vec<(i8, i8)>) {
    let (x, y) = m.xy(region);
    let mut ocean = Vec::new();
    let mut rises = [[0.0_f32; 3]; 3];
    let mut low = 0.0_f32;
    let mut high = 0.0_f32;
    for dy in -1_i8..=1 {
        for dx in -1_i8..=1 {
            let (Some(xx), Some(yy)) = (
                x.checked_add_signed(dx.into()),
                y.checked_add_signed(dy.into()),
            ) else {
                continue;
            };
            if xx >= m.size || yy >= m.size {
                continue;
            }
            let i = m.idx(xx, yy);
            if m.biome[i] == RBiome::Ocean {
                ocean.push((dx, dy));
            }
            let rise = m.elevation[i] - m.elevation[region];
            rises[(dy + 1) as usize][(dx + 1) as usize] = rise;
            low = low.min(rise);
            high = high.max(rise);
        }
    }
    let landform = if !ocean.is_empty() {
        Landform::Coast
    } else if m.biome[region] == RBiome::Swamp {
        Landform::Marsh
    } else {
        // Opposite slopes distinguish a trough or crest from a hillside.
        // Heights are normalized by the realm generator to the interval 0..1.
        let axes = [
            (rises[0][1], rises[2][1]),
            (rises[1][0], rises[1][2]),
            (rises[0][0], rises[2][2]),
            (rises[0][2], rises[2][0]),
        ];
        let trough = axes.iter().map(|&(a, b)| a.min(b)).fold(0.0_f32, f32::max);
        let crest = axes
            .iter()
            .map(|&(a, b)| (-a).min(-b))
            .fold(0.0_f32, f32::max);
        if m.river[region] || trough > 0.012 && trough > crest {
            Landform::Valley
        } else if crest > 0.016 || high - low > 0.09 {
            Landform::Ridge
        } else if m.height(region) > 0.45 {
            Landform::Plateau
        } else {
            Landform::Plain
        }
    };
    (landform, ocean)
}
