//! The census behind #940: `spoken_for` (systems/decide.rs), the Talk checks and the Tend scan
//! read other people's live state without asking whether the chooser could actually see them.
//! `Knowing` tallies, for every chosen Talk, Tend, Forage or EatItem, whether the target person
//! (or, for a pick claim, the claimer) was in line of sight of the chooser at the moment chosen
//! (`mind::perception::can_see`), plus the day's "nothing to gather" and "they left" failures.
//!
//! Read-only: nothing here is read by a system, so keeping the tally cannot change a run's
//! digest. `systems::decide::choose` calls `observe_decision` right after a goal is picked;
//! callers (an example, a test) call `observe_failures` once per tick to add the day's counts
//! from `World::last_events`.

use crate::ids::TilePos;
use crate::mind::perception::can_see;
use crate::world_state::World;

/// Which of the four watched goals a decision was.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Watched {
    Talk,
    Tend,
    Forage,
    EatItem,
}

impl Watched {
    pub fn label(self) -> &'static str {
        match self {
            Watched::Talk => "talk",
            Watched::Tend => "tend",
            Watched::Forage => "forage",
            Watched::EatItem => "eat_item",
        }
    }
}

/// Seen/unseen counts for one watched goal.
#[derive(Default, Clone, Copy, Debug)]
pub struct Tally {
    pub seen: u64,
    pub unseen: u64,
}

impl Tally {
    /// The share of counted choices whose target was out of sight (`0.0` with nothing counted).
    pub fn share_unseen(&self) -> f32 {
        let total = self.seen + self.unseen;
        if total == 0 {
            0.0
        } else {
            self.unseen as f32 / total as f32
        }
    }
}

/// The running tally. Build one with `Knowing::default()` and feed it from a stepped `World`.
#[derive(Default, Clone, Debug)]
pub struct Knowing {
    pub talk: Tally,
    pub tend: Tally,
    pub forage: Tally,
    pub eat_item: Tally,
    /// `PlanFailed` reason "nothing to gather", by day.
    pub nothing_to_gather: u64,
    /// `PlanFailed` reason "they left", by day.
    pub they_left: u64,
}

impl Knowing {
    fn tally_mut(&mut self, watched: Watched) -> &mut Tally {
        match watched {
            Watched::Talk => &mut self.talk,
            Watched::Tend => &mut self.tend,
            Watched::Forage => &mut self.forage,
            Watched::EatItem => &mut self.eat_item,
        }
    }

    pub fn tally(&self, watched: Watched) -> Tally {
        match watched {
            Watched::Talk => self.talk,
            Watched::Tend => self.tend,
            Watched::Forage => self.forage,
            Watched::EatItem => self.eat_item,
        }
    }

    /// Record one chosen decision: `chooser` is the chooser's position and `target` is the
    /// target person's (or claimer's) position, both at the moment chosen. `w.grid` is used for
    /// the line-of-sight test, so the result matches what the chooser's own perception would
    /// have found that tick.
    pub fn observe_decision(
        &mut self,
        w: &World,
        watched: Watched,
        chooser: TilePos,
        target: TilePos,
    ) {
        self.record(watched, can_see(&w.grid, chooser, target));
    }

    /// Record one chosen decision whose sight result (`seen`) the caller already worked out.
    /// Used where the caller already holds a mutable borrow of the `World` this tallies, so
    /// `observe_decision` cannot also borrow it.
    pub fn record(&mut self, watched: Watched, seen: bool) {
        let t = self.tally_mut(watched);
        if seen {
            t.seen += 1;
        } else {
            t.unseen += 1;
        }
    }

    /// Add this tick's "nothing to gather" and "they left" plan failures from `w.last_events`.
    /// Call once per tick (an example or test owns the cadence).
    pub fn observe_failures(&mut self, w: &World) {
        for e in &w.last_events {
            if let crate::events::EventKind::PlanFailed { reason, .. } = &e.kind {
                match reason.as_str() {
                    "nothing to gather" => self.nothing_to_gather += 1,
                    "they left" => self.they_left += 1,
                    _ => {}
                }
            }
        }
    }

    /// A one-line-per-goal report of the out-of-sight share, plus the day's failure counts.
    pub fn report(&self) -> String {
        format!(
            "talk {:.2} ({}/{}), tend {:.2} ({}/{}), forage {:.2} ({}/{}), eat_item {:.2} ({}/{}); nothing to gather {}, they left {}",
            self.talk.share_unseen(),
            self.talk.unseen,
            self.talk.seen + self.talk.unseen,
            self.tend.share_unseen(),
            self.tend.unseen,
            self.tend.seen + self.tend.unseen,
            self.forage.share_unseen(),
            self.forage.unseen,
            self.forage.seen + self.forage.unseen,
            self.eat_item.share_unseen(),
            self.eat_item.unseen,
            self.eat_item.seen + self.eat_item.unseen,
            self.nothing_to_gather,
            self.they_left,
        )
    }
}
