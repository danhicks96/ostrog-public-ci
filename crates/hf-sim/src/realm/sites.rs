//! Sites and the yearly step that grows them: settlements that rise toward what their land can
//! carry, send out daughter settlements when crowded, open mines in the hills, and suffer
//! famine and plague; a site that empties ends, with the year and the cause, and stays on the
//! map as a ruin. Everything that happens is a RealmEvent (written to the chronicle from C26).

use std::collections::BTreeMap;

use rand::Rng;
use serde::{Deserialize, Serialize};

use super::embark::habitable;
use super::map::{RBiome, RealmMap};
use super::peoples::People;
use crate::rng::{purpose, stream};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SiteKind {
    Hamlet,
    Village,
    Town,
    Mine,
}

impl SiteKind {
    pub fn label(self) -> &'static str {
        match self {
            SiteKind::Hamlet => "hamlet",
            SiteKind::Village => "village",
            SiteKind::Town => "town",
            SiteKind::Mine => "mine",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RSite {
    pub id: u32,
    pub people: u32,
    pub region: usize,
    pub name: String,
    pub kind: SiteKind,
    pub founded: i64,
    pub parent: Option<u32>,
    pub population: f32,
    pub peak: f32,
    /// (year, cause) once nobody is left.
    pub ended: Option<(i64, String)>,
    /// Households of a rare people living here among its own (P5): the people's index.
    #[serde(default)]
    pub minority: Option<u32>,
    /// The culture its people here hold (P7).
    #[serde(default)]
    pub culture: u32,
    /// In the caverns under its region (P10), not on the land.
    #[serde(default)]
    pub deep: bool,
}

impl RSite {
    pub fn alive(&self) -> bool {
        self.ended.is_none()
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum RealmEventKind {
    Founded {
        site: u32,
        parent: Option<u32>,
    },
    Became {
        site: u32,
        kind: SiteKind,
    },
    Famine {
        site: u32,
        lost: u32,
    },
    Plague {
        site: u32,
        lost: u32,
    },
    Ended {
        site: u32,
        cause: String,
    },
    Born {
        figure: u32,
        site: u32,
        parent: Option<u32>,
        /// The parent's partner, the other parent (P9).
        #[serde(default)]
        other: Option<u32>,
    },
    Died {
        figure: u32,
        cause: String,
    },
    /// A figure led the settlers who founded a site.
    Led {
        figure: u32,
        site: u32,
    },
    Moved {
        figure: u32,
        to: u32,
    },
    PolityFounded {
        polity: u32,
        seat: u32,
        ruler: Option<u32>,
    },
    Joined {
        polity: u32,
        site: u32,
    },
    Succeeded {
        polity: u32,
        figure: u32,
    },
    PolityEnded {
        polity: u32,
        cause: String,
    },
    WarBegan {
        war: u32,
        a: u32,
        b: u32,
        cause: String,
    },
    Battle {
        war: u32,
        site: u32,
        won_by: u32,
        /// Allied realms that fought beside either side (P9).
        #[serde(default)]
        with: Vec<u32>,
    },
    /// A settlement taken in war: its people are the takers' now.
    Taken {
        site: u32,
        by: u32,
    },
    /// A far settlement's people grew a culture of their own (P7).
    CultureSplit {
        culture: u32,
        from: u32,
        site: u32,
    },
    WarEnded {
        war: u32,
        cause: String,
    },
    /// Caravans went between two settlements over a decade (P9): one row per pair per decade.
    Traded {
        a: u32,
        b: u32,
        caravans: u32,
        /// "grain for timber".
        goods: String,
        /// The two cultures, as they were that decade.
        #[serde(default)]
        cultures: (u32, u32),
    },
    /// Two figures of different cultures married, their settlements trading (P9); `b` went to
    /// live at `a`'s settlement.
    Married {
        a: u32,
        b: u32,
        at: u32,
    },
    /// Two realms at war with the same third made common cause (P9).
    Allied {
        alliance: u32,
        a: u32,
        b: u32,
        against: u32,
    },
    /// Something woke in the deep under a region (P10).
    BeastWoke {
        beast: u32,
        region: usize,
    },
    /// A beast of the deep fell on a settlement of the deep (P10).
    BeastAttacked {
        beast: u32,
        site: u32,
        dead: u32,
    },
    /// People left a settlement after a famine, a plague or a war, for another (P9).
    Fled {
        from: u32,
        to: Option<u32>,
        /// The people who fled (a taken settlement's old people).
        people: u32,
        count: u32,
        cause: String,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RealmEvent {
    pub year: i64,
    pub kind: RealmEventKind,
}

/// A valley departure's destination, named and spotted at the choice (#1322) but not founded
/// until its first leaver actually steps off the map: the realm should hold who arrived, not
/// who set out.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PendingSettle {
    pub id: u32,
    pub folk: u32,
    pub region: usize,
    pub kind: SiteKind,
    pub name: String,
    pub parent: Option<u32>,
    /// Set once the first arrival founds the site.
    pub site: Option<u32>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Realm {
    /// Salt for per-region preferences (from the seed).
    #[serde(default)]
    pub seed_salt: u64,
    /// The world's seed (names are words of the founders' language from it).
    #[serde(default)]
    pub seed: u64,
    /// The region map's side, so region indices can be measured (0 in a realm-less world).
    #[serde(default)]
    pub map_size: usize,
    pub year: i64,
    pub peoples: Vec<People>,
    pub sites: Vec<RSite>,
    /// Which site holds each region (living sites only).
    pub claimed: BTreeMap<usize, u32>,
    /// Which deep site holds the caverns under each region (P10).
    #[serde(default)]
    pub claimed_deep: BTreeMap<usize, u32>,
    /// Regions nobody of the realm settles: the valley the party comes into has its own past
    /// (deep.rs).
    #[serde(default)]
    pub reserved: std::collections::BTreeSet<usize>,
    #[serde(default)]
    pub figures: Vec<super::figures::Figure>,
    /// The world's cultures (P7): each people's first, then those grown at far settlements.
    #[serde(default)]
    pub cultures: Vec<super::culture::Culture>,
    /// Each culture's tellings of the world's history (P8).
    #[serde(default)]
    pub stories: Vec<super::stories::Story>,
    #[serde(default)]
    pub next_story: u32,
    /// Every name given so far (settlements and figures), so no two share one: a record that
    /// says "Ivouth joined" twice must mean the same place. Rebuilt from the lists after a load.
    #[serde(skip)]
    pub names: std::collections::BTreeSet<String>,
    #[serde(default)]
    pub polities: Vec<super::figures::Polity>,
    #[serde(default)]
    pub wars: Vec<super::war::War>,
    /// Between each pair of cultures that have met (P9).
    #[serde(default)]
    pub relations: Vec<super::relations::Relation>,
    /// Trade routes between settlements, with this decade's caravans (P9).
    #[serde(default)]
    pub routes: Vec<super::relations::Route>,
    #[serde(default)]
    pub alliances: Vec<super::relations::Alliance>,
    /// The deep's beasts (P10).
    #[serde(default)]
    pub beasts: Vec<super::beasts::Beast>,
    /// Flights from settlements taken this year, settled by `relations::step`, which knows
    /// what each host's land carries: (from, people, count, cause).
    #[serde(default)]
    pub fleeing: Vec<(u32, u32, u32, String)>,
    /// The realm's year when the party came through (its years go on after).
    #[serde(default)]
    pub arrived: i64,
    /// Notable things that happened in the world after the arrival, with where (a region),
    /// for news a traveller can bring back (the latest 200).
    #[serde(default)]
    pub news: Vec<(i64, usize, RealmEventKind)>,
    /// The latest realm year the valley has heard news of, by settlement.
    #[serde(default)]
    pub heard: BTreeMap<u32, i64>,
    /// Valley departures named and spotted but not yet founded (#1322).
    #[serde(default)]
    pub pending: Vec<PendingSettle>,
    #[serde(default)]
    pub next_pending: u32,
}

/// How many people a region can feed: by its country, more with rain, a river or a shore.
pub fn capacity(m: &RealmMap, i: usize) -> f32 {
    let base = match m.biome[i] {
        RBiome::Grassland => 140.0,
        RBiome::TemperateForest => 100.0,
        RBiome::Savanna => 90.0,
        RBiome::Rainforest => 60.0,
        RBiome::Steppe => 55.0,
        RBiome::Taiga => 40.0,
        RBiome::Swamp => 30.0,
        RBiome::Tundra => 20.0,
        RBiome::Desert => 8.0,
        RBiome::Mountain => 25.0,
        RBiome::Ice | RBiome::Ocean | RBiome::Lake => 0.0,
    };
    let (x, y) = m.xy(i);
    let mut water = 1.0;
    for dy in -1i32..=1 {
        for dx in -1i32..=1 {
            let (nx, ny) = (x as i32 + dx, y as i32 + dy);
            if nx < 0 || ny < 0 || nx as usize >= m.size || ny as usize >= m.size {
                continue;
            }
            let j = m.idx(nx as usize, ny as usize);
            if m.river[j] {
                water = 1.6f32.max(water);
            } else if m.biome[j].is_water() {
                water = 1.3f32.max(water);
            }
        }
    }
    base * (0.7 + 0.6 * m.rain[i]) * water
}

/// How many people the caverns under a region can feed (P10): fungus and blind fish, most
/// under mountains, where the caves run deepest; none under the sea.
pub fn capacity_deep(m: &RealmMap, i: usize) -> f32 {
    if !m.land(i) {
        return 0.0;
    }
    // Rich enough under mountains for a daughter settlement to be worth the move; elsewhere
    // the deep's settlements hold on but do not spread (the first worlds filled the whole
    // underground with two thousand of them).
    match m.biome[i] {
        RBiome::Mountain => 60.0,
        RBiome::Ice => 5.0,
        RBiome::Desert | RBiome::Tundra => 20.0,
        _ => 25.0,
    }
}

/// What a region feeds in the layer a site lives in.
pub fn capacity_of(m: &RealmMap, i: usize, deep: bool) -> f32 {
    if deep {
        capacity_deep(m, i)
    } else {
        capacity(m, i)
    }
}

fn kind_for(pop: f32) -> SiteKind {
    if pop < 60.0 {
        SiteKind::Hamlet
    } else if pop < 250.0 {
        SiteKind::Village
    } else {
        SiteKind::Town
    }
}

impl Realm {
    pub(crate) fn found(
        &mut self,
        people: u32,
        region: usize,
        kind: SiteKind,
        pop: f32,
        parent: Option<u32>,
        out: &mut Vec<RealmEvent>,
    ) {
        let id = self.sites.len() as u32;
        let name = self.unique_name(people, "site", id as u64);
        self.found_named(people, region, kind, pop, parent, name, out);
    }

    /// As `found`, with a name already drawn (#1322: a valley departure's site is named at the
    /// choice, before anyone has arrived, so the departure line and the eventual founding agree).
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn found_named(
        &mut self,
        people: u32,
        region: usize,
        kind: SiteKind,
        pop: f32,
        parent: Option<u32>,
        name: String,
        out: &mut Vec<RealmEvent>,
    ) {
        let id = self.sites.len() as u32;
        let deep = self.peoples[people as usize].deep;
        self.sites.push(RSite {
            id,
            people,
            region,
            name,
            kind,
            founded: self.year,
            parent,
            population: pop,
            peak: pop,
            ended: None,
            minority: None,
            culture: people,
            deep,
        });
        // Its culture: the parent settlement's, unless it lies far from the people's homeland
        // and its parent still holds the homeland's culture: then it grows its own (P7).
        if let Some(pid) = parent {
            let parent_culture = self.sites[pid as usize].culture;
            let home = self.peoples[people as usize].homeland;
            let n = self.map_size.max(1);
            let d = ((region % n) as i64 - (home % n) as i64)
                .abs()
                .max((region / n) as i64 - (home / n) as i64)
                .unsigned_abs() as usize;
            let homeland_culture = parent_culture == people;
            let mut culture = parent_culture;
            if d >= super::culture::SPLIT_DISTANCE
                && homeland_culture
                && (parent_culture as usize) < self.cultures.len()
            {
                let cid = self.cultures.len() as u32;
                let site_name = self.sites[id as usize].name.clone();
                let people_name = self.peoples[people as usize].name.clone();
                let c = super::culture::split(
                    self.seed,
                    &self.cultures[parent_culture as usize],
                    cid,
                    id,
                    &site_name,
                    self.year,
                    &people_name,
                );
                self.cultures.push(c);
                culture = cid;
                out.push(RealmEvent {
                    year: self.year,
                    kind: RealmEventKind::CultureSplit {
                        culture: cid,
                        from: parent_culture,
                        site: id,
                    },
                });
            }
            self.sites[id as usize].culture = culture;
        }
        // A rare people (P5) has no homeland: some of its households live in others' places.
        let rare: Vec<u32> = self
            .peoples
            .iter()
            .filter(|p| p.rarity == "rare" && p.id != people && !p.deep)
            .map(|p| p.id)
            .collect();
        if !rare.is_empty() && !deep {
            let h = crate::rng::hash64(&[self.seed, 0x4d494e, id as u64]);
            if h.is_multiple_of(5) {
                self.sites[id as usize].minority = Some(rare[(h / 5) as usize % rare.len()]);
            }
        }
        if deep {
            self.claimed_deep.insert(region, id);
        } else {
            self.claimed.insert(region, id);
        }
        out.push(RealmEvent {
            year: self.year,
            kind: RealmEventKind::Founded { site: id, parent },
        });
    }

    /// A settlement is left empty for a cause (also from war.rs).
    pub fn end_site(&mut self, site: u32, cause: &str, out: &mut Vec<RealmEvent>) {
        self.end(site, cause, out);
    }

    fn end(&mut self, site: u32, cause: &str, out: &mut Vec<RealmEvent>) {
        let s = &mut self.sites[site as usize];
        s.ended = Some((self.year, cause.to_string()));
        s.population = 0.0;
        if s.deep {
            self.claimed_deep.remove(&s.region);
        } else {
            self.claimed.remove(&s.region);
        }
        out.push(RealmEvent {
            year: self.year,
            kind: RealmEventKind::Ended {
                site,
                cause: cause.to_string(),
            },
        });
    }

    /// The nearest unclaimed region within `reach` of `from` that `ok` accepts, preferring
    /// richer land (ties by index, so the choice is deterministic).
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn spot(
        &self,
        m: &RealmMap,
        from: usize,
        reach: i32,
        min_worth: f32,
        deep: bool,
        ok: impl Fn(usize) -> bool,
    ) -> Option<usize> {
        let claimed = if deep {
            &self.claimed_deep
        } else {
            &self.claimed
        };
        let (x, y) = m.xy(from);
        let mut best: Option<(f32, usize)> = None;
        for dy in -reach..=reach {
            for dx in -reach..=reach {
                let (nx, ny) = (x as i32 + dx, y as i32 + dy);
                if nx < 0 || ny < 0 || nx as usize >= m.size || ny as usize >= m.size {
                    continue;
                }
                let j = m.idx(nx as usize, ny as usize);
                if claimed.contains_key(&j) || self.reserved.contains(&j) || !ok(j) {
                    continue;
                }
                // Not cheek by jowl with another settlement.
                let crowded = (-1i32..=1).any(|ey| {
                    (-1i32..=1).any(|ex| {
                        let (qx, qy) = (nx + ex, ny + ey);
                        qx >= 0
                            && qy >= 0
                            && (qx as usize) < m.size
                            && (qy as usize) < m.size
                            && claimed.contains_key(&m.idx(qx as usize, qy as usize))
                    })
                });
                if crowded {
                    continue;
                }
                let d = ((dx * dx + dy * dy) as f32).sqrt();
                // Each region has its own pull (a spring, a ford, good soil nobody can see on
                // the map), so settlements spread unevenly instead of on a lattice.
                let pull = 0.1
                    + 1.8 * (crate::rng::hash64(&[self.seed_salt, j as u64]) >> 40) as f32
                        / (1u64 << 24) as f32;
                let worth = capacity_of(m, j, deep) * pull;
                // Only a spot worth the move is settled: gaps stay between settlements, and
                // they gather along rivers and on good land.
                if worth < min_worth {
                    continue;
                }
                let score = worth / (1.0 + d);
                if best.is_none_or(|(b, bi)| score > b || (score == b && j < bi)) {
                    best = Some((score, j));
                }
            }
        }
        best.map(|b| b.1)
    }

    /// One year for every living site.
    pub fn step(&mut self, seed: u64, m: &RealmMap, out: &mut Vec<RealmEvent>) {
        self.year += 1;
        let year = self.year;
        let n = self.sites.len();
        for k in 0..n {
            if !self.sites[k].alive() {
                continue;
            }
            let mut r = stream(seed, &[purpose::REALM_HISTORY, year as u64, k as u64]);
            let (region, kind, deep) =
                (self.sites[k].region, self.sites[k].kind, self.sites[k].deep);
            let cap = if kind == SiteKind::Mine {
                40.0
            } else {
                capacity_of(m, region, deep) * 3.0
            };
            // A legendary people's land fails it, a little more each century (P5): it may
            // hold on, but most such peoples are ruins and tales by the portal's time.
            let people = self.sites[k].people as usize;
            let cap = if self
                .peoples
                .get(people)
                .is_some_and(|p| p.rarity == "legendary")
            {
                cap * (1.0 - 0.15 * (year as f32 / 100.0)).max(0.1)
            } else {
                cap
            };
            let s = &mut self.sites[k];
            // A good or a bad year, then growth toward what the land carries.
            let luck: f32 = r.gen_range(-0.04..0.04);
            let growth = (0.035 + luck) * s.population * (1.0 - s.population / cap.max(1.0));
            s.population = (s.population + growth).max(0.0);
            s.peak = s.peak.max(s.population);
            let id = s.id;
            // Famine when the year is bad and the land is full, or the land is poor and the
            // place small (no stores to carry it through).
            let poor = cap < 120.0 && s.population < 40.0;
            if luck < -0.03 && (s.population > cap * 0.85 || poor) && r.gen::<f32>() < 0.5 {
                let lost = s.population * r.gen_range(0.15..if poor { 0.7 } else { 0.4 });
                s.population -= lost;
                out.push(RealmEvent {
                    year,
                    kind: RealmEventKind::Famine {
                        site: id,
                        lost: lost as u32,
                    },
                });
            }
            // Plague finds crowded places.
            if s.population > 150.0 && r.gen::<f32>() < 0.006 {
                let lost = s.population * r.gen_range(0.2..0.5);
                s.population -= lost;
                out.push(RealmEvent {
                    year,
                    kind: RealmEventKind::Plague {
                        site: id,
                        lost: lost as u32,
                    },
                });
            }
            if s.population < 4.0 {
                let cause = if s.kind == SiteKind::Mine {
                    "the seam gave out"
                } else {
                    "hunger emptied it"
                };
                self.end(id, cause, out);
                continue;
            }
            // A small place on poor land, after a hard spell, may give up and go back to the
            // settlement it came from.
            if s.kind == SiteKind::Hamlet
                && cap < 120.0
                && s.population < 25.0
                && r.gen::<f32>() < 0.04
            {
                let (pop, parent) = (s.population, s.parent);
                if let Some(pa) = parent.filter(|&pa| self.sites[pa as usize].alive()) {
                    self.sites[pa as usize].population += pop;
                    self.end(id, "they went back to their kin", out);
                    continue;
                }
            }
            let s = &mut self.sites[k];
            if s.kind != SiteKind::Mine {
                let now = kind_for(s.population);
                if now != s.kind {
                    s.kind = now;
                    out.push(RealmEvent {
                        year,
                        kind: RealmEventKind::Became {
                            site: id,
                            kind: now,
                        },
                    });
                }
            }
            let (pop, people) = (self.sites[k].population, self.sites[k].people);
            // Crowded: some go to found a daughter settlement nearby.
            if kind != SiteKind::Mine && pop > cap * 0.7 && r.gen::<f32>() < 0.12 {
                // The deep's peoples spread under any land; the others over land they can farm.
                let ok = |j: usize| if deep { m.land(j) } else { habitable(m, j) };
                if let Some(spot) = self.spot(m, region, 6, 70.0, deep, ok) {
                    let go = (pop * 0.2).min(40.0);
                    self.sites[k].population -= go;
                    self.found(people, spot, kind_for(go), go, Some(id), out);
                }
            }
            // Hills nearby: a mine (the deep's peoples live in the rock already).
            if kind != SiteKind::Mine && !deep && pop > 80.0 && r.gen::<f32>() < 0.02 {
                if let Some(spot) =
                    self.spot(m, region, 4, 0.0, false, |j| m.biome[j] == RBiome::Mountain)
                {
                    self.sites[k].population -= 12.0;
                    self.found(people, spot, SiteKind::Mine, 12.0, Some(id), out);
                }
            }
        }
    }
}

/// The world's peoples and their settlements, from their first homes through `years` years.
pub fn run(seed: u64, m: &RealmMap, years: i64) -> (Realm, Vec<RealmEvent>) {
    run_reserving(seed, m, years, &[])
}

/// As `run`, leaving `reserved` regions to nobody (a homeland there moves to the nearest
/// habitable region that is free).
pub fn run_reserving(
    seed: u64,
    m: &RealmMap,
    years: i64,
    reserved: &[usize],
) -> (Realm, Vec<RealmEvent>) {
    run_with(
        seed,
        m,
        years,
        reserved,
        &crate::species::PeopleSetup::default(),
        &[],
    )
}

/// As `run_reserving`, with the wizard's choices about the world's peoples (P5).
pub fn run_with(
    seed: u64,
    m: &RealmMap,
    years: i64,
    reserved: &[usize],
    choice: &crate::species::PeopleSetup,
    generated: &[&crate::species::SpeciesDef],
) -> (Realm, Vec<RealmEvent>) {
    let mut realm = Realm {
        reserved: reserved.iter().copied().collect(),
        map_size: m.size,
        seed_salt: crate::rng::hash64(&[seed, purpose::REALM_HISTORY, 0x70756c6c]),
        seed,
        year: 0,
        peoples: super::peoples::peoples_with(seed, m, choice, generated),
        ..Default::default()
    };
    // Each people's first culture (P7), with the same index as the people.
    let kind_of = |id: crate::species::SpeciesId| {
        if id.0 >= crate::species::GENERATED {
            generated
                .get((id.0 - crate::species::GENERATED) as usize)
                .copied()
                .unwrap_or_else(|| crate::species::get(id))
        } else {
            crate::species::get(id)
        }
    };
    realm.cultures = realm
        .peoples
        .iter()
        .map(|p| super::culture::first(seed, p, kind_of(p.species)))
        .collect();
    for p in 0..realm.peoples.len() {
        let home = realm.peoples[p].homeland;
        if realm.reserved.contains(&home) {
            let (hx, hy) = m.xy(home);
            let free = (0..m.size * m.size)
                .filter(|&j| habitable(m, j) && !realm.reserved.contains(&j))
                .min_by_key(|&j| {
                    let (x, y) = m.xy(j);
                    (
                        (x as i64 - hx as i64).pow(2) + (y as i64 - hy as i64).pow(2),
                        j,
                    )
                });
            if let Some(j) = free {
                realm.peoples[p].homeland = j;
            }
        }
    }
    let mut out = Vec::new();
    for p in 0..realm.peoples.len() {
        // A rare people of the land founds no homeland: it lives among others (found() places
        // it). A people of the deep always has its own caverns.
        if realm.peoples[p].rarity == "rare" && !realm.peoples[p].deep {
            continue;
        }
        let home = realm.peoples[p].homeland;
        realm.found(p as u32, home, SiteKind::Hamlet, 30.0, None, &mut out);
    }
    for _ in 0..years {
        let first = out.len();
        realm.step(seed, m, &mut out);
        super::figures::step(&mut realm, m.size, first, &mut out);
        super::war::step(&mut realm, m.size, &mut out);
        super::relations::step(&mut realm, m, first, &mut out);
        super::beasts::step(&mut realm, m, &mut out);
        // Each culture's stories of the year (P8), pointing at the rows by their place in the
        // history until the chronicle gives them ids (stories::remap).
        let rows: Vec<(crate::ids::EventId, RealmEvent)> = out[first..]
            .iter()
            .enumerate()
            .map(|(k, e)| (crate::ids::EventId((first + k) as u32), e.clone()))
            .collect();
        super::stories::record(&mut realm, &rows);
        let year = realm.year;
        super::stories::retell(&mut realm, year);
    }
    (realm, out)
}

impl Realm {
    /// The people whose settlement lies nearest a region (the valley's neighbours), if any.
    pub fn nearest_people(&self, region: usize) -> Option<&super::peoples::People> {
        if self.map_size == 0 {
            return None;
        }
        let n = self.map_size;
        let (vx, vy) = ((region % n) as i64, (region / n) as i64);
        self.sites
            .iter()
            .filter(|s| !s.deep)
            .min_by_key(|s| {
                let (x, y) = ((s.region % n) as i64, (s.region / n) as i64);
                ((x - vx).pow(2) + (y - vy).pow(2), s.id)
            })
            .and_then(|s| self.peoples.get(s.people as usize))
    }
}

impl Realm {
    /// A name in a people's tongue nobody in the world bears yet (the n-th word of a kind, or
    /// the next ones after it if taken).
    pub fn unique_name(&mut self, people: u32, kind: &str, n: u64) -> String {
        if self.names.is_empty() && (!self.sites.is_empty() || !self.figures.is_empty()) {
            self.names = self
                .sites
                .iter()
                .map(|s| s.name.clone())
                .chain(self.figures.iter().map(|f| f.name.clone()))
                .collect();
        }
        let lang = &self.peoples[people as usize].language;
        let mut k = 0u64;
        loop {
            let mut name = lang.name(self.seed, kind, n + k * 1_000_003);
            // A language with few sounds runs out of short names: after forty tries, two words
            // run together, as such peoples do ("Ybre" and "tal", Ybretal).
            if k > 40 {
                name += &lang
                    .name(self.seed, "suffix", n.wrapping_mul(31) + k)
                    .to_lowercase();
            }
            if !self.names.contains(&name) || k > 400 {
                self.names.insert(name.clone());
                return name;
            }
            k += 1;
        }
    }
}
