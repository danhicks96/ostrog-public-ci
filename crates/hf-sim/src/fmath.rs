//! Platform-independent transcendental functions for the simulation.
//!
//! `f32::exp`, `ln`, `powf`, `sin`, `cos` and `atan2` call the platform's C maths library,
//! and glibc (Linux) and the MSVC runtime (Windows) round their last bits differently. One
//! last-bit difference in a skill level, a decay factor or a direction changes a later
//! comparison, and the world digest for the same commit and seed then differs between the
//! two machines (#234, FIXLOG 2026-10-22, fix/234). `libm` is a pure-Rust port of musl's
//! maths library: the same bits on every platform. Use these in anything that feeds the
//! world state; the plain methods are fine for the viewer.

pub trait Det: Copy {
    fn d_exp(self) -> Self;
    fn d_ln(self) -> Self;
    fn d_powf(self, e: Self) -> Self;
    fn d_sin(self) -> Self;
    fn d_cos(self) -> Self;
    fn d_atan2(self, x: Self) -> Self;
}

impl Det for f32 {
    fn d_exp(self) -> f32 {
        libm::expf(self)
    }
    fn d_ln(self) -> f32 {
        libm::logf(self)
    }
    fn d_powf(self, e: f32) -> f32 {
        libm::powf(self, e)
    }
    fn d_sin(self) -> f32 {
        libm::sinf(self)
    }
    fn d_cos(self) -> f32 {
        libm::cosf(self)
    }
    fn d_atan2(self, x: f32) -> f32 {
        libm::atan2f(self, x)
    }
}

impl Det for f64 {
    fn d_exp(self) -> f64 {
        libm::exp(self)
    }
    fn d_ln(self) -> f64 {
        libm::log(self)
    }
    fn d_powf(self, e: f64) -> f64 {
        libm::pow(self, e)
    }
    fn d_sin(self) -> f64 {
        libm::sin(self)
    }
    fn d_cos(self) -> f64 {
        libm::cos(self)
    }
    fn d_atan2(self, x: f64) -> f64 {
        libm::atan2(self, x)
    }
}

#[cfg(test)]
mod tests {
    use super::Det;

    #[test]
    fn known_bits() {
        // musl's results, the same on every platform; a change here changes every digest.
        assert_eq!(1.0f32.d_exp().to_bits(), 0x402d_f854);
        assert_eq!(0.5f64.d_cos().to_bits(), 0x3fec_1528_065b_7d50);
    }
}
