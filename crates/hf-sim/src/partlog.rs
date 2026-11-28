//! A history for each part of a body (Body & Gear, slice 0). Every part has one anatomical
//! identity, a `PartKey`; what happened to it (a wound, its healing, a scar, a loss, a made
//! replacement, an ordinary change such as a callus) is kept as short entries that each point at
//! the chronicle event that recorded it, so a part's story can be followed to the event and the
//! event back to the part. Nothing here is read by the simulation's decisions, and an entry is
//! made only when something is recorded: a part with no entries has no story, and none is made up.
//!
//! The first branch is the left hand and its digits. The vocabulary is a path of small ids, so
//! other branches and other body plans can be appended without renumbering.

use serde::{Deserialize, Serialize};

use crate::ids::EventId;

/// Which side of the body a part is on.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
pub enum Side {
    Left,
    Right,
    Centre,
}

/// A region of the hand and arm. Append only: saved as the variant name.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
pub enum Region {
    Hand,
    Palm,
    HandBack,
    Thumb,
    Index,
    Middle,
    Ring,
    Little,
}

/// A place along a digit or a hand.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
pub enum Spot {
    Whole,
    /// The big knuckle and the segment that bears a ring.
    Base,
    Middle,
    Tip,
    /// The last knuckle, between middle and tip.
    TipKnuckle,
    Pad,
    NailPlate,
    NailBed,
}

/// One part, by its single identity: side, region and spot.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
pub struct PartKey {
    pub side: Side,
    pub region: Region,
    pub spot: Spot,
}

impl PartKey {
    pub const fn new(side: Side, region: Region, spot: Spot) -> PartKey {
        PartKey { side, region, spot }
    }

    /// Words for the part: "left ring finger, base".
    pub fn label(self) -> String {
        let side = match self.side {
            Side::Left => "left ",
            Side::Right => "right ",
            Side::Centre => "",
        };
        let region = match self.region {
            Region::Hand => "hand",
            Region::Palm => "palm",
            Region::HandBack => "back of the hand",
            Region::Thumb => "thumb",
            Region::Index => "index finger",
            Region::Middle => "middle finger",
            Region::Ring => "ring finger",
            Region::Little => "little finger",
        };
        let spot = match self.spot {
            Spot::Whole => "",
            Spot::Base => ", base",
            Spot::Middle => ", middle",
            Spot::Tip => ", tip",
            Spot::TipKnuckle => ", last knuckle",
            Spot::Pad => ", pad",
            Spot::NailPlate => ", nail",
            Spot::NailBed => ", nail bed",
        };
        format!("{side}{region}{spot}")
    }

    /// The part this one belongs to, one step up: a nail to its tip, a tip to its digit, a digit
    /// to its hand. None for the hand itself.
    pub fn parent(self) -> Option<PartKey> {
        let up = |spot| Some(PartKey::new(self.side, self.region, spot));
        match (self.region, self.spot) {
            (Region::Hand, _) => None,
            (_, Spot::NailPlate | Spot::NailBed | Spot::Pad | Spot::TipKnuckle) => up(Spot::Tip),
            (_, Spot::Whole) => Some(PartKey::new(self.side, Region::Hand, Spot::Whole)),
            _ => up(Spot::Whole),
        }
    }
}

/// What happened to a part.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Change {
    Wounded,
    Healed,
    Scarred,
    Lost,
    Replaced,
    /// Ordinary life: a callus, a tattoo, a cut nail.
    Marked,
}

/// One entry: what, when, and the chronicle event that recorded it (None only for a change the
/// chronicle does not tell).
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Entry {
    pub part: PartKey,
    pub tick: u64,
    pub change: Change,
    pub event: Option<EventId>,
    pub note: String,
}

/// Entries kept per person. When full the oldest ordinary ("Marked") entry goes first; wounds,
/// scars, losses and replacements are the significant history and are kept.
pub const KEPT: usize = 64;

/// A person's part histories, oldest first. Empty for everyone until something is recorded, and
/// not written to a save while empty, so old saves load unchanged.
#[derive(Clone, Default, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct PartLog {
    entries: Vec<Entry>,
}

impl PartLog {
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn record(&mut self, e: Entry) {
        if self.entries.len() >= KEPT {
            let at = self
                .entries
                .iter()
                .position(|x| x.change == Change::Marked)
                .unwrap_or(0);
            self.entries.remove(at);
        }
        self.entries.push(e);
    }

    /// Everything recorded for a part, and for the parts under it when `below`; oldest first.
    pub fn history(&self, part: PartKey, below: bool) -> Vec<&Entry> {
        self.entries
            .iter()
            .filter(|e| e.part == part || (below && is_below(e.part, part)))
            .collect()
    }

    /// Entries that point at a chronicle event: the way from an event back to its parts.
    pub fn for_event(&self, ev: EventId) -> Vec<&Entry> {
        self.entries
            .iter()
            .filter(|e| e.event == Some(ev))
            .collect()
    }

    /// Is the part (or one above it) lost, and not since replaced?
    pub fn missing(&self, part: PartKey) -> bool {
        let mut at = Some(part);
        while let Some(p) = at {
            let last = self
                .entries
                .iter()
                .rev()
                .find(|e| e.part == p && matches!(e.change, Change::Lost | Change::Replaced));
            if matches!(last, Some(e) if e.change == Change::Lost) {
                return true;
            }
            at = p.parent();
        }
        false
    }
}

fn is_below(part: PartKey, of: PartKey) -> bool {
    let mut at = part.parent();
    while let Some(p) = at {
        if p == of {
            return true;
        }
        at = p.parent();
    }
    false
}
