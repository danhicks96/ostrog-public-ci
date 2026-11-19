//! Surface relief: ground z per column from layered noise, quantised to `relief` levels.

use super::noise::fbm2;
use super::GenParams;
use crate::rng::{hash64, purpose};
use crate::world::WorldGrid;

pub fn height_field(seed: u64, p: &GenParams) -> Vec<f32> {
    let s = hash64(&[seed, purpose::WORLDGEN_HEIGHT]);
    let scale = 1.0 / (p.width.max(p.height) as f32 * 0.45);
    let mut h = Vec::with_capacity((p.width * p.height) as usize);
    for y in 0..p.height {
        for x in 0..p.width {
            let nx = x as f32 * scale;
            let ny = y as f32 * scale;
            let base = fbm2(s, nx * 2.0, ny * 2.0, 5, 2.0, 0.5);
            let ridges = (fbm2(s ^ 0xabc, nx * 4.0 + 7.0, ny * 4.0 + 3.0, 3, 2.0, 0.5) - 0.5).abs();
            let v = (base * 0.8 + (1.0 - ridges * 2.0) * 0.2).clamp(0.0, 1.0);
            let (fx, fy) = (x as f32 / p.width as f32, y as f32 / p.height as f32);
            // The lie of the land (#352) bends the noise.
            let v = match p.landform {
                super::Landform::Valley => v,
                super::Landform::Plain => 0.32 + 0.25 * v,
                super::Landform::Plateau => {
                    let d = ((fx - 0.5).powi(2) + (fy - 0.5).powi(2)).sqrt();
                    let rise = (1.0 - ((d - 0.28) / 0.08).clamp(0.0, 1.0)) * 0.45;
                    (0.25 + 0.3 * v + rise).clamp(0.0, 1.0)
                }
                super::Landform::Ridge => {
                    let d = (fx - fy).abs() / std::f32::consts::SQRT_2;
                    let rise = 0.55 * (1.0 - (d / 0.14).min(1.0));
                    (0.3 * v + 0.2 + rise).clamp(0.0, 1.0)
                }
                super::Landform::Coast => {
                    let shore = (fx / 0.3).min(1.0);
                    (v * shore + 0.0 * (1.0 - shore)).clamp(0.0, 1.0)
                }
                super::Landform::Marsh => 0.16 + 0.12 * v,
            };
            h.push(v);
        }
    }
    h
}

pub fn apply(seed: u64, p: &GenParams, grid: &mut WorldGrid) {
    let h = height_field(seed, p);
    for (i, v) in h.iter().enumerate() {
        // Ground z in 0..=relief, with sea_level handled by hydrology (low ground floods).
        let gz = ((v - 0.15).max(0.0) / 0.85 * p.relief as f32).round() as i32;
        grid.ground[i] = gz.clamp(0, p.relief);
    }
}
