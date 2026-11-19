//! The time model. Source of truth is docs/TIME.md; these constants must match that table.
//!
//! The tick is the only simulation unit. 1 tick = 3 game-seconds. 20 ticks per real second at 1×.

use serde::{Deserialize, Serialize};

pub const TICKS_PER_REAL_SECOND_1X: u32 = 20;
pub const GAME_SECONDS_PER_TICK: u32 = 3;
pub const TICKS_PER_MINUTE: u64 = 20;
pub const TICKS_PER_HOUR: u64 = 1_200;
pub const HOURS_PER_DAY: u64 = 24;
pub const TICKS_PER_DAY: u64 = TICKS_PER_HOUR * HOURS_PER_DAY; // 28_800
pub const DAYS_PER_SEASON: u64 = 5;
pub const SEASONS_PER_YEAR: u64 = 4;
pub const DAYS_PER_YEAR: u64 = DAYS_PER_SEASON * SEASONS_PER_YEAR; // 20
pub const TICKS_PER_YEAR: u64 = TICKS_PER_DAY * DAYS_PER_YEAR; // 576_000

/// Movement costs in ticks per tile (docs/TIME.md "Movement").
pub const WALK_TICKS_PER_TILE: u32 = 4;
pub const HEAVY_TICKS_PER_TILE: u32 = 6;
pub const RUN_TICKS_PER_TILE: u32 = 3;
pub const LEVEL_CHANGE_TICKS: u32 = 8;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Season {
    Spring,
    Summer,
    Autumn,
    Winter,
}

/// Calendar view of a tick count.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Calendar {
    pub year: u64,
    pub season: Season,
    pub day_of_season: u64, // 1-based
    pub day_of_year: u64,   // 0-based
    pub hour: u64,
    pub minute: u64,
}

impl Calendar {
    pub fn from_tick(tick: u64) -> Self {
        let day = tick / TICKS_PER_DAY;
        let in_day = tick % TICKS_PER_DAY;
        let year = day / DAYS_PER_YEAR;
        let day_of_year = day % DAYS_PER_YEAR;
        let season_idx = day_of_year / DAYS_PER_SEASON;
        let season = match season_idx {
            0 => Season::Spring,
            1 => Season::Summer,
            2 => Season::Autumn,
            _ => Season::Winter,
        };
        Self {
            year,
            season,
            day_of_season: day_of_year % DAYS_PER_SEASON + 1,
            day_of_year,
            hour: in_day / TICKS_PER_HOUR,
            minute: (in_day % TICKS_PER_HOUR) / TICKS_PER_MINUTE,
        }
    }

    /// 0.0 at midnight, 0.5 at noon.
    pub fn day_fraction(tick: u64) -> f32 {
        (tick % TICKS_PER_DAY) as f32 / TICKS_PER_DAY as f32
    }

    pub fn is_night(tick: u64) -> bool {
        let h = (tick % TICKS_PER_DAY) / TICKS_PER_HOUR;
        !(6..22).contains(&h)
    }

    /// How full the moon is (P11): 0 new, 1 full, on a ten-day cycle from the world's first
    /// tick, full on the fifth night of each, so two full moons a twenty-day year.
    pub fn moon(tick: u64) -> f32 {
        let cycle = MOON_DAYS * TICKS_PER_DAY;
        // The night belongs to the day it began on: the hours after midnight are last night's.
        let t = (tick + cycle - 6 * TICKS_PER_HOUR) % cycle;
        let phase = t as f32 / cycle as f32;
        (1.0 - (phase - 0.45).abs() * 2.0).clamp(0.0, 1.0)
    }

    /// The night of the full moon, from dusk to dawn.
    pub fn full_moon(tick: u64) -> bool {
        Self::is_night(tick) && Self::moon(tick) >= 0.9
    }

    /// The hours a people sleeps through (P4): the night for most, the middle of the day
    /// (08:00 to 18:00) for a nocturnal people.
    pub fn rest_time(tick: u64, nocturnal: bool) -> bool {
        if nocturnal {
            let h = (tick % TICKS_PER_DAY) / TICKS_PER_HOUR;
            (8..18).contains(&h)
        } else {
            Self::is_night(tick)
        }
    }
}

/// Days in the moon's cycle (P11).
pub const MOON_DAYS: u64 = 10;

/// Age in whole game-years given birth tick and current tick.
pub fn age_years(born: i64, now: u64) -> u64 {
    ((now as i64 - born).max(0) as u64) / TICKS_PER_YEAR
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_matches_docs() {
        assert_eq!(TICKS_PER_DAY, 28_800);
        assert_eq!(TICKS_PER_YEAR, 576_000);
        // A day at 1x is 24 real minutes.
        assert_eq!(TICKS_PER_DAY / TICKS_PER_REAL_SECOND_1X as u64, 24 * 60);
        let c = Calendar::from_tick(TICKS_PER_DAY * 7 + TICKS_PER_HOUR * 13 + TICKS_PER_MINUTE * 5);
        assert_eq!(c.year, 0);
        assert_eq!(c.season, Season::Summer);
        assert_eq!(c.day_of_season, 3);
        assert_eq!(c.hour, 13);
        assert_eq!(c.minute, 5);
    }
}
