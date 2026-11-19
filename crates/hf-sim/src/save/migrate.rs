//! Explicit versioned migrations of `world.bin`. Each step is a function from the bytes of
//! version N to the bytes of version N+1. Fixture saves per version live in
//! `crates/hf-sim/tests/fixtures/saves/` and the migration test loads every one of them.

use anyhow::{bail, Result};

type Step = fn(Vec<u8>) -> Result<Vec<u8>>;

/// Index i migrates version (i+1) -> (i+2). Format 1 (positional, pre-release) cannot be
/// read by anything newer, so its step refuses with a clear message.
const STEPS: &[Step] = &[v1_to_v2];

fn v1_to_v2(_bytes: Vec<u8>) -> Result<Vec<u8>> {
    bail!("format 1 saves come from the pre-release build and cannot be loaded")
}

pub fn migrate(from: u32, to: u32, mut bytes: Vec<u8>) -> Result<Vec<u8>> {
    if from > to {
        bail!("cannot migrate downward from {from} to {to}");
    }
    let mut v = from;
    while v < to {
        let idx = (v - 1) as usize;
        let Some(step) = STEPS.get(idx) else {
            bail!("no migration step from format {v} to {}", v + 1);
        };
        bytes = step(bytes)?;
        v += 1;
    }
    Ok(bytes)
}
