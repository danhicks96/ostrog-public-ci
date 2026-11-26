//! Beyond the map edge: an overworld of named sites, and caravans that go there for a
//! reason and come back changed (or not at all). Travel is resolved in day-steps.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::ids::{AgentId, CommunityId};
use crate::items::ItemKind;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum SiteKind {
    Village,
    Ruin,
    Mine,
    Forest,
    Lakeshore,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Edge {
    North,
    East,
    South,
    West,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Site {
    pub id: u32,
    pub name: String,
    pub kind: SiteKind,
    pub edge: Edge,
    pub days: u32,
    /// What the place has to give, and what it wants.
    pub offers: BTreeMap<ItemKind, u32>,
    pub wants: Vec<ItemKind>,
    /// 0..1 chance per travel day of trouble.
    pub danger: f32,
    pub known_by: Vec<CommunityId>,
    /// People living there (0 for a ruin or empty country), and their food stores in
    /// person-days. Stores rise and fall with the seasons; a long hunger sends people out.
    #[serde(default)]
    pub people: u32,
    #[serde(default)]
    pub food: f32,
    #[serde(default)]
    pub hungry_days: u32,
    #[serde(default)]
    pub last_band: u64,
    /// C76: what the valley has done to their people (hurt, routed, kept); at two they send a
    /// host to lay siege, not a hungry band.
    #[serde(default)]
    pub grudge: u32,
    /// C76: people of the valley they carried off, and when.
    #[serde(default)]
    pub captives: Vec<(crate::ids::AgentId, u64)>,
    /// The settlement of the world (World::realm) this place is, when it is one.
    #[serde(default)]
    pub realm_site: Option<u32>,
    /// What it holds to trade (trade.rs, C36): made by its living and land, used up day by
    /// day, sold to caravans and bought from them. Fractions accumulate.
    #[serde(default)]
    pub stores: BTreeMap<ItemKind, f32>,
    /// What each held kind is made of (grain, goat hide, granite).
    #[serde(default)]
    pub store_material: BTreeMap<ItemKind, crate::world::Material>,
    /// The last dozen exchanges with the valley.
    #[serde(default)]
    pub trades: Vec<crate::trade::Trade>,
    /// The last dozen exchanges with other settlements beyond the valley (P9).
    #[serde(default)]
    pub exchanges: Vec<crate::trade::Exchange>,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum Purpose {
    Trade {
        site: u32,
        carry: ItemKind,
    },
    Scout {
        site: u32,
    },
    /// Appended (C59): carry `carry` to `site` and bring back `want`, a building material the
    /// community lacks and has no way to get at home.
    Fetch {
        site: u32,
        carry: ItemKind,
        want: ItemKind,
    },
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum CaravanState {
    /// Members walk to the departure tile.
    Forming,
    Outbound {
        day: u32,
    },
    AtSite {
        day: u32,
    },
    Returning {
        day: u32,
    },
    Stranded {
        days: u32,
        then_return: bool,
        back_day: u32,
    },
    Lost,
    Done,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Caravan {
    pub id: u32,
    pub community: CommunityId,
    pub members: Vec<AgentId>,
    pub purpose: Purpose,
    pub cargo: Vec<(ItemKind, u32)>,
    /// What the goods brought back are made of (trade.rs): grain, goat hide, iron ore.
    #[serde(default)]
    pub materials: BTreeMap<ItemKind, crate::world::Material>,
    /// The goods' quality (trade.rs pays by kind; what comes home unsold keeps it).
    #[serde(default)]
    pub qualities: BTreeMap<ItemKind, f32>,
    /// Meals for the road, apart from the goods (FIXLOG 2026-10-20 #165, C62: kept in the cargo,
    /// a party carrying meals to trade sold its own rations with them).
    #[serde(default)]
    pub rations: u32,
    /// How many were with it when it was lost whole (#167: the list is emptied then, and the
    /// road's toll counted nobody).
    #[serde(default)]
    pub lost: u32,
    pub state: CaravanState,
    pub formed: u64,
    pub departed: u64,
    /// Tick the journey ended (returned, lost, or given up); 0 while under way.
    #[serde(default)]
    pub finished: u64,
    pub depart_tile: crate::ids::TilePos,
    /// What happened on the road, in order.
    pub log: Vec<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Overworld {
    pub sites: Vec<Site>,
    pub caravans: Vec<Caravan>,
    pub next_caravan_id: u32,
    /// People off the map, kept whole (identity, memories, skills) until they return.
    pub away: Vec<crate::agents::Agent>,
    #[serde(default)]
    pub picked: BTreeMap<(u32, i32, i32), (u64, u32)>,
    #[serde(default)]
    pub surveys: BTreeMap<CommunityId, survey::Survey>,
}

const SITE_NAMES: &[&str] = &[
    "Greywater",
    "Tollmoor",
    "Hollin's Rest",
    "Brackenford",
    "Saltmere",
    "Oldstone",
    "Fennwick",
    "Thornhallow",
    "Redcroft",
    "Mirewell",
];

impl Overworld {
    /// Those of community `cid` off the map on the road and expected back, alive: on a caravan
    /// still under way (not lost, not done) or on a journey of their own. Not the captive, nor
    /// the child gone below for good, who are also kept in `away` (#197 cases 7 and 14).
    pub fn on_the_road(&self, cid: CommunityId) -> impl Iterator<Item = &crate::agents::Agent> {
        self.away.iter().filter(move |a| {
            a.alive
                && a.community == Some(cid)
                && (a.journey.is_some()
                    || self.caravans.iter().any(|c| {
                        !matches!(c.state, CaravanState::Lost | CaravanState::Done)
                            && c.members.contains(&a.id)
                    }))
        })
    }

    pub fn generate(seed: u64) -> Overworld {
        use rand::seq::SliceRandom;
        use rand::Rng;
        let mut r = crate::rng::stream(seed, &[crate::rng::purpose::NAMES, 0x0F_F000]);
        let mut names = SITE_NAMES.to_vec();
        names.shuffle(&mut r);
        let kinds = [
            SiteKind::Village,
            SiteKind::Village,
            SiteKind::Mine,
            SiteKind::Ruin,
            SiteKind::Forest,
            SiteKind::Lakeshore,
        ];
        let edges = [Edge::North, Edge::East, Edge::South, Edge::West];
        let mut sites = Vec::new();
        for (i, kind) in kinds.iter().enumerate() {
            let people = match kind {
                SiteKind::Village => r.gen_range(30..60),
                SiteKind::Mine => r.gen_range(12..25),
                _ => 0,
            };
            let days = r.gen_range(2..6);
            sites.push(Site::new(
                i as u32,
                names[i].to_string(),
                *kind,
                edges[i % 4],
                days,
                people,
            ));
        }
        Overworld {
            sites,
            caravans: Vec::new(),
            next_caravan_id: 0,
            away: Vec::new(),
            picked: BTreeMap::new(),
            surveys: BTreeMap::new(),
        }
    }

    pub fn site(&self, id: u32) -> Option<&Site> {
        self.sites.iter().find(|s| s.id == id)
    }

    /// The valley's neighbours from its world (M3): the nearest living settlements, a working
    /// mine, a ruin, and the nearest forest and lake country, each in the direction it lies
    /// and as many days away as the map puts it. Names are the settlements' own; the wild
    /// places take the nearest people's words for them.
    pub fn from_realm(
        realm: &crate::realm::sites::Realm,
        m: &crate::realm::RealmMap,
        region: usize,
    ) -> Overworld {
        use crate::realm::sites::SiteKind as RK;
        use crate::realm::RBiome;
        let (vx, vy) = m.xy(region);
        let d2 = |r: usize| {
            let (x, y) = m.xy(r);
            (x as i64 - vx as i64).pow(2) + (y as i64 - vy as i64).pow(2)
        };
        let edge_of = |r: usize| {
            let (x, y) = m.xy(r);
            let (dx, dy) = (x as i64 - vx as i64, y as i64 - vy as i64);
            if dx.abs() >= dy.abs() {
                if dx >= 0 {
                    Edge::East
                } else {
                    Edge::West
                }
            } else if dy >= 0 {
                Edge::South
            } else {
                Edge::North
            }
        };
        // About two and a half regions a day on foot, at least a day.
        let days_of = |r: usize| (((d2(r) as f32).sqrt() / 2.5).round() as u32).clamp(1, 12);
        // The deep's settlements (P10) are under the land, not on the roads out of the valley.
        let mut by_distance: Vec<&crate::realm::sites::RSite> =
            realm.sites.iter().filter(|s| !s.deep).collect();
        by_distance.sort_by_key(|s| (d2(s.region), s.id));
        let mut sites: Vec<Site> = Vec::new();
        let mut add = |name: String, kind: SiteKind, r: usize, people: u32, from: Option<u32>| {
            let id = sites.len() as u32;
            let mut s = Site::new(id, name, kind, edge_of(r), days_of(r), people);
            s.realm_site = from;
            sites.push(s);
        };
        for s in by_distance
            .iter()
            .filter(|s| s.alive() && s.kind != RK::Mine)
            .take(3)
        {
            add(
                s.name.clone(),
                SiteKind::Village,
                s.region,
                (s.population as u32).clamp(8, 200),
                Some(s.id),
            );
        }
        if let Some(s) = by_distance.iter().find(|s| s.alive() && s.kind == RK::Mine) {
            add(
                s.name.clone(),
                SiteKind::Mine,
                s.region,
                (s.population as u32).clamp(6, 40),
                Some(s.id),
            );
        }
        if let Some(s) = by_distance.iter().find(|s| !s.alive()) {
            add(s.name.clone(), SiteKind::Ruin, s.region, 0, Some(s.id));
        }
        // The nearest wild country of each kind, named in the nearest people's tongue.
        let speaker = by_distance
            .first()
            .and_then(|s| realm.peoples.get(s.people as usize));
        let wild = |want: &dyn Fn(RBiome) -> bool| {
            (0..m.size * m.size)
                .filter(|&r| r != region && want(m.biome[r]))
                .min_by_key(|&r| (d2(r), r))
        };
        if let Some(r) = wild(&|b| {
            matches!(
                b,
                RBiome::TemperateForest | RBiome::Taiga | RBiome::Rainforest
            )
        }) {
            let name = speaker
                .map(|p| p.language.word(realm.seed, "forest"))
                .unwrap_or_else(|| "the forest".into());
            add(name, SiteKind::Forest, r, 0, None);
        }
        if let Some(r) = wild(&|b| b == RBiome::Lake) {
            let name = speaker
                .map(|p| p.language.word(realm.seed, "lake"))
                .unwrap_or_else(|| "the lake".into());
            add(name, SiteKind::Lakeshore, r, 0, None);
        }
        Overworld {
            sites,
            caravans: Vec::new(),
            next_caravan_id: 0,
            away: Vec::new(),
            picked: BTreeMap::new(),
            surveys: BTreeMap::new(),
        }
    }
}

impl Site {
    /// A place beyond the valley: what it offers and wants and how dangerous the road is come
    /// from its kind.
    pub fn new(id: u32, name: String, kind: SiteKind, edge: Edge, days: u32, people: u32) -> Site {
        let mut offers = BTreeMap::new();
        let wants = match kind {
            SiteKind::Village => {
                offers.insert(ItemKind::Cloth, 6);
                offers.insert(ItemKind::Meal, 10);
                offers.insert(ItemKind::Tool, 1);
                vec![ItemKind::Log, ItemKind::StoneBlock, ItemKind::Hide]
            }
            SiteKind::Mine => {
                offers.insert(ItemKind::IronOre, 8);
                offers.insert(ItemKind::Coal, 6);
                vec![ItemKind::Meal, ItemKind::GrainSheaf]
            }
            SiteKind::Ruin => {
                offers.insert(ItemKind::Tool, 1);
                offers.insert(ItemKind::StoneBlock, 10);
                vec![]
            }
            SiteKind::Forest => {
                offers.insert(ItemKind::Log, 14);
                offers.insert(ItemKind::Hide, 3);
                vec![]
            }
            SiteKind::Lakeshore => {
                offers.insert(ItemKind::RawMeat, 8);
                vec![ItemKind::Log]
            }
        };
        Site {
            id,
            name,
            kind,
            edge,
            days,
            offers,
            wants,
            danger: match kind {
                SiteKind::Ruin => 0.25,
                SiteKind::Mine => 0.12,
                _ => 0.06,
            },
            known_by: Vec::new(),
            people,
            // Spring stores are low after the winter: the hungry gap.
            food: 0.0,
            hungry_days: 0,
            last_band: 0,
            grudge: 0,
            captives: Vec::new(),
            realm_site: None,
            stores: BTreeMap::new(),
            store_material: BTreeMap::new(),
            trades: Vec::new(),
            exchanges: Vec::new(),
        }
    }
}

/// What a line of `Caravan.log` says happened on the road (#1584). Read-only: the census example
/// counts these; nothing in the sim uses it. Lines about the site (arrival, trade) are not road
/// incidents and classify as `None`.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum RoadIncident {
    Robbed,
    Waylaid,
    Fall,
    Fog,
    MemberLost,
    Berries,
    FoundRoad,
    NothingHeard,
}

pub fn road_incident(line: &str) -> Option<RoadIncident> {
    use RoadIncident::*;
    let table = [
        ("robbed on the road", Robbed),
        ("waylaid", Waylaid),
        ("a fall on bad ground", Fall),
        ("lost the way in fog", Fog),
        ("was lost on the road", MemberLost),
        ("found wild berries", Berries),
        ("gathered", Berries),
        ("found the road again", FoundRoad),
        ("nothing more was heard", NothingHeard),
    ];
    table
        .iter()
        .find(|(s, _)| line.contains(s))
        .map(|(_, k)| *k)
}

#[path = "route_level.rs"]
pub mod route_level;
#[path = "expedition_survey.rs"]
pub mod survey;

pub const ROAD_UNITS_PER_DAY: i64 = 1200;

/// Local road coordinates for observing an expedition between daily travel decisions.
/// One segment is a travel day. The same clock and caravan state locate the party
/// when the watcher opens the map, after reload, or while watching a different party.
pub fn road_distance(c: &Caravan, tick: u64, days: u32) -> i64 {
    use crate::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
    let dawn = 6 * TICKS_PER_HOUR;
    let elapsed = (tick % TICKS_PER_DAY + TICKS_PER_DAY - dawn) % TICKS_PER_DAY;
    let length = i64::from(days) * ROAD_UNITS_PER_DAY;
    let along =
        |day: u32, fraction: i64| (i64::from(day) * ROAD_UNITS_PER_DAY + fraction).min(length);
    let (travelled, total) = if matches!(c.state, CaravanState::Outbound { day: 0 }) {
        let since = tick.saturating_sub(c.departed);
        let until_update =
            TICKS_PER_DAY - (c.departed % TICKS_PER_DAY + TICKS_PER_DAY - dawn) % TICKS_PER_DAY;
        (since.min(until_update), until_update)
    } else {
        (elapsed, TICKS_PER_DAY)
    };
    let fraction = (travelled * ROAD_UNITS_PER_DAY as u64 / total) as i64;
    match c.state {
        CaravanState::Outbound { day } => along(day, fraction),
        CaravanState::Returning { day } => length - along(day, fraction),
        CaravanState::Stranded {
            back_day,
            then_return,
            ..
        } => {
            let at = along(back_day, 0);
            if then_return {
                length - at
            } else {
                at
            }
        }
        CaravanState::AtSite { .. } => length,
        _ => 0,
    }
}
