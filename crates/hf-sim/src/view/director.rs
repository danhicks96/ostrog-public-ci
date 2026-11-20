//! Automatic camera: scores recent events by salience, novelty and recency, avoids places
//! shown recently, and suggests where to look and for how long. It never creates events.

use std::collections::BTreeMap;

use crate::events::{Event, Severity};
use crate::ids::TilePos;
use crate::world_state::World;

/// Where the settlement is: the mean of what its people have built (stockpiles aside) near
/// the home of the largest living community, on that home's level; else that home, else the
/// founding site. Only things near the home count: the valley's past leaves graves and wells
/// all over the map (deep.rs, 28 tiles or more from the landing), and outsiders build camps;
/// averaging those pulled the camera into empty woods (#104).
pub const SETTLEMENT_REACH: i32 = 24;

pub fn settlement_centre(w: &World) -> TilePos {
    let mut members: BTreeMap<crate::ids::CommunityId, usize> = BTreeMap::new();
    for a in w
        .agents
        .iter()
        .filter(|a| a.alive && a.species == crate::agents::Species::Person)
    {
        if let Some(c) = a.community {
            *members.entry(c).or_default() += 1;
        }
    }
    // The largest community (the lowest id on a tie).
    let biggest = members
        .iter()
        .fold(
            None::<(crate::ids::CommunityId, usize)>,
            |best, (&c, &n)| match best {
                Some((_, m)) if m >= n => best,
                _ => Some((c, n)),
            },
        )
        .map(|(c, _)| c);
    let home = biggest
        .and_then(|c| w.communities.iter().find(|x| x.id == c))
        .map(|c| c.home)
        .unwrap_or(w.meta.founding_site);
    let built: Vec<TilePos> = w
        .structures
        .iter()
        .filter(|s| s.kind != crate::structures::StructureKind::Stockpile)
        .map(|s| s.pos)
        .filter(|p| (p.x - home.x).abs().max((p.y - home.y).abs()) <= SETTLEMENT_REACH)
        .collect();
    if built.is_empty() {
        return home;
    }
    let n = built.len() as i32;
    let (sx, sy) = built
        .iter()
        .fold((0, 0), |acc, p| (acc.0 + p.x, acc.1 + p.y));
    TilePos::new(sx / n, sy / n, home.z)
}

/// The people the director weighs: every living human (sleepers included, so the lit
/// house draws the camera at night), flagged busy when working or talking.
/// The unfound lost are not among them: nobody knows where they are (view::known).
pub fn people(w: &World) -> Vec<(TilePos, bool)> {
    let hidden = crate::view::known::unfound(w);
    w.agents
        .iter()
        .filter(|a| a.alive && a.species == crate::agents::Species::Person)
        .filter(|a| !hidden.contains(&a.id))
        .map(|a| {
            let busy = matches!(
                a.activity,
                crate::mind::Activity::Working { .. } | crate::mind::Activity::Talking { .. }
            );
            (a.pos, busy)
        })
        .collect()
}

/// What the camera is looking at, in a viewer's words, for the HUD's director line: the
/// director's own kind names (`identity_formed`) are system terms (#200, QA 2026-10-20, C63;
/// docs/design-glossary.md: reasons in a person's terms).
pub fn watching(kind: &str) -> &'static str {
    match kind {
        "settlement" => "the settlement",
        "people" => "people about the place",
        "people at work" | "built" | "mined" | "crafted" | "cooked" | "harvested" | "hauled"
        | "foraged" | "tree_felled" | "engraved" | "house_stage" | "propped" => "people at work",
        "attacked" | "fought" | "wounded" | "raided" | "beast_slain" | "hunted" | "bitten"
        | "breached" | "took_up_arms" => "a fight",
        "fire_started" | "burned" | "beat_out" | "doused" | "fire_out" => "a fire",
        "caravan_formed" | "caravan_left" | "caravan_returned" | "caravan_lost" | "traded" => {
            "a caravan"
        }
        "born" => "a birth",
        "died" | "laid_to_rest" | "ate_dead" | "old_died" => "a death",
        "roof_fell" | "collapse" | "cave_in" | "flooded" | "furniture_broken" => "a collapse",
        "talked" | "quarrelled" | "drank_together" | "hearth_tale" | "tale_told"
        | "tale_brought" | "shared_dream" | "news_heard" | "proposed" | "gave_ring" | "paired" => {
            "people talking"
        }
        "strangers_sighted" | "band_set_out" | "welcomed" | "band_left" | "arrived"
        | "refugees" | "joined" | "turned_away" | "would_not_stay" | "site_hungry" => "strangers",
        "identity_formed" | "identity_lapsed" | "custom_formed" | "custom_adopted"
        | "custom_lapsed" | "rest_day" | "split" | "moved_camp" | "community_ended" | "chose"
        | "role" | "named" | "item_named" | "cherished" => "the community",
        "fell_ill" | "ailing" | "tended" | "healed" | "recovered" | "starving" | "exhausted" => {
            "someone in need"
        }
        "crows_gathered" | "hart_sighted" | "bear_denned" | "bear_woke" | "ducks_left"
        | "ducks_returned" | "bats_stirred" | "beast_stirred" => "the wild",
        "ghost_seen" | "echo_seen" | "thin_stirred" | "thin_studied" | "thin_quieted"
        | "wrong_weather" | "possessed" | "cursed" | "from_below" | "restless" => {
            "something strange"
        }
        _ => "something happening",
    }
}

/// (score, representative position, kind name, event count)
type CellScore = (f32, TilePos, &'static str, u32);

#[derive(Clone, Debug)]
pub struct Suggestion {
    pub at: TilePos,
    pub score: f32,
    pub reason: String,
    /// The kind of event (or "people at work", "settlement") the camera went to look at; the
    /// viewer is shown `watching(kind)`, never this name (#200).
    pub kind: &'static str,
    /// Suggested dwell in ticks.
    pub dwell: u32,
}

#[derive(Clone, Debug, Default)]
pub struct Director {
    /// Recent notable events kept for scoring (tick, pos, severity, kind name).
    recent: Vec<(u64, TilePos, Severity, &'static str)>,
    /// Where the camera has been shown recently (coarse cell -> tick).
    shown: BTreeMap<(i32, i32, i32), u64>,
    pub current: Option<Suggestion>,
    pub since: u64,
}

const CELL: i32 = 12;
const KEEP_TICKS: u64 = 20 * 60 * 5; // five game-minutes... no: 5 real minutes at 1x = 6000 ticks

impl Director {
    pub fn observe(&mut self, tick: u64, events: &[Event]) {
        for e in events {
            if e.severity >= Severity::Minor {
                if let Some(p) = e.at {
                    self.recent.push((e.tick, p, e.severity, e.kind_name()));
                }
            }
        }
        self.recent
            .retain(|(t, ..)| tick.saturating_sub(*t) < KEEP_TICKS);
    }

    fn cell(p: TilePos) -> (i32, i32, i32) {
        (p.x.div_euclid(CELL), p.y.div_euclid(CELL), p.z)
    }

    /// Pick the best place to look. `min_hold` keeps the current target until its dwell
    /// is over unless something clearly bigger happens.
    /// `people` = (position, busy) of inhabitants (see `people`); busy ones pull the camera
    /// harder than idle or sleeping ones. Where people are now outranks where events were.
    pub fn suggest(
        &mut self,
        tick: u64,
        fallback: TilePos,
        people: &[(TilePos, bool)],
    ) -> Suggestion {
        if let Some(cur) = &self.current {
            if tick.saturating_sub(self.since) < cur.dwell as u64 {
                return cur.clone();
            }
        }
        let mut by_cell: BTreeMap<(i32, i32, i32), CellScore> = BTreeMap::new();
        // Who is awake where, now: an event cell with nobody in it is a place where
        // something happened, not where something is happening.
        let mut awake: BTreeMap<(i32, i32, i32), u32> = BTreeMap::new();
        for (p, _) in people {
            *awake.entry(Self::cell(*p)).or_default() += 1;
        }
        // Repeats of one kind in one cell count for less: the n-th Minor event of a kind
        // in a cell weighs 1/n of the first, so a stockpile's thirtieth `hauled` row does
        // not simply outscore a single death elsewhere (#1606). Major and Notable events
        // are never discounted this way, only Minor ones; `recent` stays in its existing
        // chronological order, so this is the n-th-seen-so-far count, not a final tally.
        let mut kind_count: BTreeMap<((i32, i32, i32), &'static str), u32> = BTreeMap::new();
        for (t, p, sev, kind) in &self.recent {
            let age = tick.saturating_sub(*t) as f32;
            let recency = (1.0 - age / KEEP_TICKS as f32).max(0.0);
            let sal = match sev {
                Severity::Major => 3.0,
                Severity::Notable => 1.6,
                Severity::Minor => 0.7,
                Severity::Trace => 0.0,
            } * match *kind {
                "slept" => 0.3,
                "talked" | "built" | "mined" | "cooked" | "crafted" => 1.3,
                "foraged" | "hauled" => 0.9,
                "caravan_left" | "caravan_returned" | "caravan_lost" | "born" | "died"
                | "arrived" | "split" | "cave_in" | "collapse" | "fire_started" | "burned" => 1.8,
                _ => 1.0,
            };
            let c = Self::cell(*p);
            let shown_penalty = match self.shown.get(&c) {
                Some(s) if tick.saturating_sub(*s) < KEEP_TICKS * 2 => 0.35,
                _ => 1.0,
            };
            let present = awake.get(&c).copied().unwrap_or(0);
            // A fire is something happening whether or not anyone stands in it (#219: a
            // strike, the spread and the beating-out all went by off camera). So is a Major
            // event about a person: dying alone is the saddest death and the common one, and
            // it must not score less than one person at work.
            let about_person = matches!(sev, Severity::Major)
                && matches!(*kind, "died" | "captured" | "broke" | "exiled");
            let presence =
                if about_person || matches!(*kind, "caravan_left" | "fire_started" | "burned") {
                    1.0
                } else {
                    0.25 + 0.75 * (present.min(3) as f32 / 3.0)
                };
            let repeat_discount = if matches!(sev, Severity::Minor) {
                let n = kind_count.entry((c, *kind)).or_insert(0);
                *n += 1;
                1.0 / *n as f32
            } else {
                1.0
            };
            let e = by_cell.entry(c).or_insert((0.0, *p, kind, 0));
            e.0 += sal * recency * shown_penalty * presence * repeat_discount;
            e.3 += 1;
            if sal >= 1.6 {
                e.1 = *p;
                e.2 = kind;
            }
        }
        for (p, busy) in people {
            let c = Self::cell(*p);
            let shown_penalty = match self.shown.get(&c) {
                Some(s) if tick.saturating_sub(*s) < KEEP_TICKS => 0.5,
                _ => 1.0,
            };
            let e = by_cell.entry(c).or_insert((0.0, *p, "people", 0));
            e.0 += (if *busy { 1.4 } else { 0.6 }) * shown_penalty;
            if *busy {
                e.1 = *p;
                e.2 = "people at work";
            }
        }
        // The settlement core (where things are built) is always worth a look.
        {
            let c = Self::cell(fallback);
            let e = by_cell.entry(c).or_insert((0.0, fallback, "settlement", 0));
            e.0 += 0.8;
        }
        let best = by_cell.iter().max_by(|a, b| {
            a.1 .0
                .partial_cmp(&b.1 .0)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let s = match best {
            Some((c, (score, p, kind, n))) if *score > 0.5 => {
                self.shown.insert(*c, tick);
                Suggestion {
                    at: *p,
                    score: *score,
                    reason: format!("{} ({} events)", kind, n),
                    kind,
                    dwell: match *kind {
                        "caravan_left" | "caravan_returned" => 20 * 45,
                        _ => 20 * 25 + (*n).min(20) * 20,
                    },
                }
            }
            _ => Suggestion {
                at: fallback,
                score: 0.0,
                reason: "settlement".into(),
                kind: "settlement",
                dwell: 20 * 40,
            },
        };
        self.current = Some(s.clone());
        self.since = tick;
        s
    }
}
