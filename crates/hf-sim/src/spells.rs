//! Spells over the valley (#278): a heat wave or a cold snap, a drought, a darkened sky, an ill
//! wind, a good year. Each grips the whole valley for days, begins and lifts on record, and
//! leans the weather and what follows from it (systems/weather.rs `current`, health.rs). One at
//! a time; one begins on about one day in thirty, as the season allows.

use crate::events::{EventKind, Severity};
use crate::rng::{purpose, stream};
use crate::time::{Season, TICKS_PER_DAY, TICKS_PER_HOUR};
use crate::World;
use rand::Rng;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpellKind {
    HeatWave,
    ColdSnap,
    Drought,
    DarkSky,
    IllWind,
    GoodYear,
}

impl SpellKind {
    pub fn name(self) -> &'static str {
        match self {
            SpellKind::HeatWave => "a heat wave",
            SpellKind::ColdSnap => "a cold snap",
            SpellKind::Drought => "a drought",
            SpellKind::DarkSky => "a darkened sky",
            SpellKind::IllWind => "an ill wind",
            SpellKind::GoodYear => "a kind season",
        }
    }
    fn onset(self) -> &'static str {
        match self {
            SpellKind::HeatWave => "The air turned hot and stayed so: a heat wave.",
            SpellKind::ColdSnap => "A hard cold came down over the colony.",
            SpellKind::Drought => "The rain stopped coming; the ground began to crack.",
            SpellKind::DarkSky => "A haze closed over the sun and the days went dim.",
            SpellKind::IllWind => "A sour wind came down the colony, and people began to cough.",
            SpellKind::GoodYear => "Everything that grew seemed to grow faster.",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Spell {
    pub kind: SpellKind,
    pub began: u64,
    pub until: u64,
}

/// The spell gripping the valley now, if any.
pub fn now(w: &World) -> Option<SpellKind> {
    let t = w.meta.tick;
    w.spells
        .iter()
        .find(|s| s.began <= t && t < s.until)
        .map(|s| s.kind)
}

/// Once a day at six: a spell that has run its days lifts; else one may begin.
pub fn step(w: &mut World) {
    let tick = w.meta.tick;
    if tick % TICKS_PER_DAY != 6 * TICKS_PER_HOUR + 11 {
        return;
    }
    let day = tick / TICKS_PER_DAY;
    for s in w
        .spells
        .iter()
        .filter(|s| s.until <= tick && s.until + TICKS_PER_DAY > tick)
    {
        let days = ((s.until - s.began) / TICKS_PER_DAY) as u32;
        let what = s.kind.name().to_string();
        w.events.push(
            tick,
            None,
            Severity::Notable,
            EventKind::SpellLifted { what, days },
        );
    }
    w.spells.retain(|s| s.until > tick);
    if !w.spells.is_empty() || w.since_arrival() < 10 * TICKS_PER_DAY {
        return;
    }
    let mut r = stream(w.meta.seed, &[purpose::WEATHER, 0x5BE1, day]);
    if !r.gen_bool(1.0 / 30.0) {
        return;
    }
    let season = w.calendar().season;
    let choices: &[SpellKind] = match season {
        Season::Spring => &[SpellKind::IllWind, SpellKind::GoodYear, SpellKind::DarkSky],
        Season::Summer => &[SpellKind::HeatWave, SpellKind::Drought, SpellKind::GoodYear],
        Season::Autumn => &[SpellKind::Drought, SpellKind::IllWind, SpellKind::DarkSky],
        Season::Winter => &[SpellKind::ColdSnap, SpellKind::DarkSky],
    };
    let kind = choices[r.gen_range(0..choices.len())];
    let days = r.gen_range(3..=7);
    w.spells.push(Spell {
        kind,
        began: tick,
        until: tick + days * TICKS_PER_DAY,
    });
    w.events.push(
        tick,
        None,
        Severity::Major,
        EventKind::SpellBegan {
            what: kind.name().to_string(),
            how: kind.onset().to_string(),
        },
    );
}

/// How a spell leans the day's weather (called from weather::current).
pub fn lean(w: &World, wx: &mut crate::systems::weather::Weather) {
    use crate::systems::weather::Sky;
    match now(w) {
        Some(SpellKind::HeatWave) => wx.temperature_c += 9.0,
        Some(SpellKind::ColdSnap) => wx.temperature_c -= 10.0,
        Some(SpellKind::Drought) => {
            if wx.sky == Sky::Rain || wx.sky == Sky::Storm {
                wx.sky = Sky::Clear;
            }
            wx.growth_factor *= 0.4;
            wx.temperature_c += 3.0;
        }
        Some(SpellKind::DarkSky) => {
            if wx.sky == Sky::Clear {
                wx.sky = Sky::Overcast;
            }
            wx.daylight *= 0.5;
            wx.growth_factor *= 0.6;
        }
        Some(SpellKind::GoodYear) => wx.growth_factor = (wx.growth_factor * 1.4).min(1.3),
        Some(SpellKind::IllWind) | None => {}
    }
}
