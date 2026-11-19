//! The Engine (RimWorld would call it a storyteller).
//!
//! It fires nothing. Nothing here pushes an event, spawns a raider, or decides that tonight
//! something interesting happens. An Engine decides how hard the *existing* generators lean:
//! mostly how often strangers turn up at the door and how fussy the welcome is. Every number it
//! produces is computed from recorded state, so it is reproducible from the seed, and a viewer
//! can disagree with it in words while still being shown the same world.

use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::rng::{purpose, stream};
use crate::World;

impl std::fmt::Display for Engine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

impl std::str::FromStr for Engine {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, String> {
        // By the pacing's name or by the deity who paces it (#147: the wizard hook was given
        // "Aun", the deity's name, and failed).
        match s.trim().to_ascii_lowercase().as_str() {
            "steady" | "aun" => Ok(Engine::Steady),
            "relentless" | "hask" => Ok(Engine::Relentless),
            "whimsical" | "mirre" => Ok(Engine::Whimsical),
            other => Err(format!(
                "no deity {other:?}: choose Aun (steady), Hask (relentless) or Mirre (whimsical)"
            )),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum Engine {
    /// Predictable. The valley behaves much the same whether you are thriving or not.
    #[default]
    Steady,
    /// Hard times are hard: the door opens less often when there is least to share.
    Relentless,
    /// Uncorrelated with anything. Swings within written bounds, day to day.
    Whimsical,
}

impl Engine {
    pub const ALL: [Engine; 3] = [Engine::Steady, Engine::Relentless, Engine::Whimsical];

    /// What to call it, in the viewer's words rather than a type name.
    pub fn name(&self) -> &'static str {
        match self {
            Engine::Steady => "Steady",
            Engine::Relentless => "Relentless",
            Engine::Whimsical => "Whimsical",
        }
    }

    pub fn blurb(&self) -> &'static str {
        match self {
            Engine::Steady => "the same handful of chances every day, good times or bad",
            Engine::Relentless => "when you are struggling, fewer strangers come knocking",
            Engine::Whimsical => "no pattern to read; it can be kind or cruel for no reason",
        }
    }
}

/// Multipliers handed to the generators. Each is normally 1.0; bounds are written below.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Rates {
    /// How often newcomers arrive. 0.0 .. 2.0
    pub arrivals: f32,
    /// How choosy the welcome is: above 1 makes them harder to satisfy. 0.5 .. 2.0
    pub welcome: f32,
}

impl Default for Rates {
    fn default() -> Self {
        Rates {
            arrivals: 1.0,
            welcome: 1.0,
        }
    }
}

/// How much strain the community is under, 0..1, read only from what the ledger and the
/// survivors' own state record — never from anything a survivor could not know about.
pub fn tension(w: &World) -> f32 {
    let n = w
        .agents
        .iter()
        .filter(|a| a.alive && a.species == crate::agents::Species::Person)
        .count();
    if n == 0 {
        return 0.0;
    }
    // Work nobody has got round to is its own kind of strain.
    let pending = w.designations.iter().filter(|d| !d.done).count() as f32;
    let backlog = (pending / (n as f32 * 4.0)).clamp(0.0, 1.0);
    strain(w) * 0.8 + backlog * 0.2
}

/// How far the people's needs are from met, 0..1 (food and sleep whole, comfort half), with
/// no account of unfinished work.
pub fn strain(w: &World) -> f32 {
    let mut n = 0usize;
    let mut want = 0.0f32;
    for a in &w.agents {
        if !a.alive || a.species != crate::agents::Species::Person {
            continue;
        }
        n += 1;
        // How far each need is from satisfied; comfort counts half, food and sleep whole.
        want += (1.0 - a.needs.hunger.clamp(0.0, 1.0))
            + (1.0 - a.needs.rest.clamp(0.0, 1.0))
            + 0.5 * (1.0 - a.needs.comfort.clamp(0.0, 1.0));
    }
    if n == 0 {
        return 0.0;
    }
    (want / (2.5 * n as f32)).clamp(0.0, 1.0)
}

/// The rates this Engine gives the generators now. Deterministic in (engine, tension, tick, seed).
pub fn rates(engine: Engine, t: f32, tick: u64, seed: u64) -> Rates {
    let tension = t.clamp(0.0, 1.0);
    match engine {
        Engine::Steady => {
            // Almost nothing moves: the point is that tomorrow looks like today.
            Rates {
                arrivals: (1.0 + tension * 0.15).clamp(0.0, 2.0),
                welcome: 1.0,
            }
        }
        Engine::Relentless => {
            // Strangers can tell a hard winter. The worse it is, the less often they come.
            Rates {
                arrivals: (1.05 - tension * 0.85).clamp(0.05, 2.0),
                welcome: (1.0 + tension * 0.4).clamp(0.5, 2.0),
            }
        }
        Engine::Whimsical => {
            // One roll a day, wide bounds, deliberately uncorrelated with the state.
            let day = tick / crate::time::TICKS_PER_DAY;
            let mut r = stream(seed, &[purpose::ENGINE, day]);
            let swing = r.gen_range(0.35..1.75f32);
            Rates {
                arrivals: swing.clamp(0.0, 2.0),
                welcome: r.gen_range(0.7..1.4f32).clamp(0.5, 2.0),
            }
        }
    }
}

impl World {
    /// What this world's Engine currently asks of the generators.
    pub fn engine_rates(&self) -> Rates {
        rates(
            self.meta.engine,
            tension(self),
            self.meta.tick,
            self.meta.seed,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_empty_world_reads_calm() {
        // Nobody to be under strain, no NaN either.
        let w = World::generate(1, Default::default());
        assert_eq!(tension(&w), 0.0);
    }
}
