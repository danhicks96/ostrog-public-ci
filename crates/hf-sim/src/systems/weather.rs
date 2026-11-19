//! Weather: a deterministic seasonal + daily temperature curve and a sky state.
//! Phase 1 keeps it simple and recomputed each tick from (seed, tick).
use crate::fmath::Det;

use serde::{Deserialize, Serialize};

use crate::rng::hash64;
use crate::time::{Calendar, Season, TICKS_PER_DAY, TICKS_PER_HOUR};
use crate::world_state::World;

#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize, Default)]
pub enum Sky {
    #[default]
    Clear,
    Overcast,
    Rain,
    Snow,
    /// Appended (#279): fog lying in the valley: little light, little seen.
    Fog,
    /// Appended (#279): a storm: heavy rain, wind, and lightning.
    Storm,
}

impl Sky {
    /// Rain falling (rain or a storm).
    pub fn wet(self) -> bool {
        matches!(self, Sky::Rain | Sky::Storm)
    }
}

#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize, Default)]
pub struct Weather {
    pub temperature_c: f32,
    pub sky: Sky,
    /// 0 = dormant, 1 = full growth.
    pub growth_factor: f32,
    /// 0..1 ambient light from the sun.
    pub daylight: f32,
    /// Appended (#393): the rain is a drizzle, not a steady rain.
    #[serde(default)]
    pub drizzle: bool,
}

impl Weather {
    /// How hard it rains, in units of a steady rain (#393): a drizzle a third of one, a storm
    /// three.
    pub fn rainfall(&self) -> f32 {
        match self.sky {
            Sky::Rain if self.drizzle => 0.3,
            Sky::Rain => 1.0,
            Sky::Storm => 3.0,
            _ => 0.0,
        }
    }

    pub fn at(seed: u64, tick: u64) -> Weather {
        let cal = Calendar::from_tick(tick);
        let (base, growth) = match cal.season {
            Season::Spring => (12.0, 0.8),
            Season::Summer => (22.0, 1.0),
            Season::Autumn => (10.0, 0.5),
            Season::Winter => (-4.0, 0.0),
        };
        // A late frost or an early one (#405, FIXLOG 2026-10-27): the first night of spring
        // and the last of autumn keep winter's cold until the morning, one time in six by
        // the seed. Without it the shoulder seasons' coldest hour was 5 C and the killing
        // frost (thermal.rs, below -5 C) could not come.
        let base = if late_or_early_frost(seed, &cal) && cal.hour < 9 {
            -4.0
        } else {
            base
        };
        let frac = Calendar::day_fraction(tick);
        // Warmest mid-afternoon, coldest before dawn (#256: the sign was the other way, so a
        // winter noon was -9 C and its midnight +1 C, and frostbite came at work).
        let diurnal = ((frac - 0.6) * std::f32::consts::TAU).d_cos() * 5.0;
        // Sky changes on a 6-hour schedule, hashed from seed and period.
        let period = tick / (6 * TICKS_PER_HOUR);
        let h = hash64(&[seed, 0x5157, period]) % 100;
        let sky = match (cal.season, h) {
            (Season::Winter, 0..=24) => Sky::Snow,
            // A storm of a summer or autumn afternoon's heat (#279).
            (Season::Summer | Season::Autumn, 0..=3) => Sky::Storm,
            (_, 0..=14) => Sky::Rain,
            (_, 15..=39) => Sky::Overcast,
            // Fog in the cool turn of the year (#279).
            (Season::Spring | Season::Autumn, 40..=48) => Sky::Fog,
            _ => Sky::Clear,
        };
        let daylight = {
            let hour = (tick % TICKS_PER_DAY) as f32 / TICKS_PER_HOUR as f32;
            let d = ((hour - 13.0) / 7.5).abs();
            (1.0 - d * d).clamp(0.0, 1.0)
                * match sky {
                    Sky::Clear => 1.0,
                    Sky::Fog | Sky::Storm => 0.45,
                    _ => 0.7,
                }
        };
        // One rain in three is a drizzle (#393), drawn apart from the sky's own draw.
        let drizzle = sky == Sky::Rain && hash64(&[seed, 0x393, period]).is_multiple_of(3);
        Weather {
            temperature_c: base + diurnal,
            sky,
            growth_factor: growth,
            daylight,
            drizzle,
        }
    }
}

/// Whether the night before the day of `cal` is a frost at the turn of the year (#405): the
/// first day of spring or the last of autumn, one time in six, hashed from the seed and day.
pub fn late_or_early_frost(seed: u64, cal: &Calendar) -> bool {
    let edge = (cal.season == Season::Spring && cal.day_of_season == 1)
        || (cal.season == Season::Autumn && cal.day_of_season == crate::time::DAYS_PER_SEASON);
    let day = cal.year * crate::time::DAYS_PER_YEAR + cal.day_of_year;
    edge && hash64(&[seed, 0xF205, day]).is_multiple_of(6)
}

/// The weather as the world has it now: forced, or the curve with the deity's lean. The step
/// and a load both use this, so a loaded game has the sky a continuous one has (FIXLOG
/// 2026-10-20 #160, C62: a load once rebuilt the unleaned sky and a spurious snow line followed).
pub fn current(w: &World) -> Weather {
    match w.force_weather {
        Some(f) => f,
        None => {
            let mut wx = Weather::at(w.meta.seed, w.meta.tick);
            // A deity leaning the cold (deity.rs): degrees, and snow instead of rain below zero.
            let lean = w.lean(crate::deity::Lever::Cold, crate::deity::Scope::Valley);
            wx.temperature_c += lean;
            // A spell over the valley (#278): heat, cold, drought, a dark sky, a good year.
            crate::spells::lean(w, &mut wx);
            if wx.sky.wet() && wx.temperature_c < 0.0 {
                wx.sky = Sky::Snow;
            }
            wx
        }
    }
}

pub fn step(w: &mut World) {
    // Seasons are the same on either clock (the epoch is whole years); the year people count
    // is from the arrival.
    let prev = Calendar::from_tick(w.since_arrival().saturating_sub(1));
    let now = w.calendar();
    let before = w.weather.sky;
    w.weather = current(w);
    // Rain or snow starting, or stopping, is news (issue 25); clear and overcast trading
    // places all day is not.
    let after = w.weather.sky;
    let wet = |s: Sky| matches!(s, Sky::Rain | Sky::Snow | Sky::Storm | Sky::Fog);
    if before != after && (wet(before) || wet(after)) {
        let kind = match after {
            Sky::Rain => "rain",
            Sky::Snow => "snow",
            Sky::Storm => "a storm",
            Sky::Fog => "fog came down",
            _ if before == Sky::Fog => "the fog lifted",
            _ if before == Sky::Storm => "the storm passed",
            _ if before == Sky::Snow => "the snow stopped",
            _ => "the rain stopped",
        };
        w.events.push(
            w.meta.tick,
            None,
            crate::events::Severity::Minor,
            crate::events::EventKind::Weather { kind: kind.into() },
        );
    }
    if prev.season != now.season {
        w.events.push(
            w.meta.tick,
            None,
            crate::events::Severity::Notable,
            crate::events::EventKind::SeasonChanged {
                season: format!("{:?}", now.season),
                year: now.year,
            },
        );
    }
}
