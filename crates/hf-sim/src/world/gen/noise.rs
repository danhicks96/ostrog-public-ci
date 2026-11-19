//! Hand-rolled value noise with fBm. Deterministic from a seed; no external crate.

use crate::rng::hash64;

fn lattice(seed: u64, x: i64, y: i64) -> f32 {
    let h = hash64(&[seed, x as u64, y as u64]);
    (h >> 40) as f32 / (1u64 << 24) as f32
}

fn smooth(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

/// Value noise in [0,1] at continuous coordinates.
pub fn value2(seed: u64, x: f32, y: f32) -> f32 {
    let xi = x.floor() as i64;
    let yi = y.floor() as i64;
    let fx = smooth(x - xi as f32);
    let fy = smooth(y - yi as f32);
    let a = lattice(seed, xi, yi);
    let b = lattice(seed, xi + 1, yi);
    let c = lattice(seed, xi, yi + 1);
    let d = lattice(seed, xi + 1, yi + 1);
    let ab = a + (b - a) * fx;
    let cd = c + (d - c) * fx;
    ab + (cd - ab) * fy
}

/// Fractal Brownian motion in [0,1].
pub fn fbm2(seed: u64, x: f32, y: f32, octaves: u32, lacunarity: f32, gain: f32) -> f32 {
    let mut amp = 1.0;
    let mut freq = 1.0;
    let mut sum = 0.0;
    let mut norm = 0.0;
    for o in 0..octaves {
        sum += amp * value2(seed.wrapping_add(o as u64 * 7919), x * freq, y * freq);
        norm += amp;
        amp *= gain;
        freq *= lacunarity;
    }
    sum / norm
}

/// 3D value noise for caves, in [0,1].
pub fn value3(seed: u64, x: f32, y: f32, z: f32) -> f32 {
    let zi = z.floor() as i64;
    let fz = smooth(z - zi as f32);
    let a = value2(seed ^ hash64(&[zi as u64]), x, y);
    let b = value2(seed ^ hash64(&[(zi + 1) as u64]), x, y);
    a + (b - a) * fz
}
