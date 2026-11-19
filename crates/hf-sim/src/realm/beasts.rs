//! The deep's beasts (P10 of milestone MP, docs/design-peoples.md section 9): Dwarf Fortress's
//! forgotten beasts, kept to what is recorded. Once a century something may wake in the deep
//! caverns or the underworld under the mountains: a huge thing of one body plan, made of basalt,
//! fungus, magma, bone or crystal, with a name in the words of the deep people nearest it. It
//! is born with a row, `BeastWoke`, and from then on its attacks on the deep's settlements are
//! rows too (`BeastAttacked`), so its legend is its record. Nothing is said of it that the
//! record does not hold.

use rand::Rng;
use serde::{Deserialize, Serialize};

use super::map::{RBiome, RealmMap};
use super::sites::{Realm, RealmEvent, RealmEventKind};
use crate::rng::{purpose, stream};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Beast {
    pub id: u32,
    /// "Ulkarth, the Basalt Wyrm".
    pub name: String,
    pub plan: String,
    /// What it is made of: basalt, fungus, magma, bone, crystal.
    pub made_of: String,
    /// deep_caverns or underworld.
    pub layer: String,
    /// The region it lairs under.
    pub region: usize,
    pub woke: i64,
    /// Settlements it left empty.
    pub ruined: Vec<u32>,
}

/// How far, in regions, a beast goes to feed.
pub const BEAST_REACH: i64 = 8;

const PLANS: [(&str, &str); 5] = [
    ("serpentine", "Wyrm"),
    ("huge_biped", "Colossus"),
    ("four_armed", "Render"),
    ("taur", "Strider"),
    ("digitigrade_tail", "Stalker"),
];
const MADE: [&str; 5] = ["basalt", "fungus", "magma", "bone", "crystal"];

fn dist(m: usize, a: usize, b: usize) -> i64 {
    ((a % m) as i64 - (b % m) as i64)
        .abs()
        .max(((a / m) as i64 - (b / m) as i64).abs())
}

fn push(out: &mut Vec<RealmEvent>, year: i64, kind: RealmEventKind) {
    out.push(RealmEvent { year, kind });
}

/// One year of the deep's beasts: one may wake (in the 37th year of a century from the third
/// on, with chance one in two, at most three awake), and each awake may fall on the nearest
/// settlement of the deep within its reach, one year in ten.
pub fn step(realm: &mut Realm, m: &RealmMap, out: &mut Vec<RealmEvent>) {
    let year = realm.year;
    let mut r = stream(
        realm.seed,
        &[purpose::REALM_HISTORY, year as u64, 0x6265_6173],
    ); // "beas"
       // Nothing wakes in the first two centuries: the eldest of the deep are old.
    if year >= 200 && year % 100 == 37 && realm.beasts.len() < 3 && r.gen_bool(0.5) {
        wake(realm, m, &mut r, out);
    }
    let n = m.size.max(1);
    for b in 0..realm.beasts.len() {
        // A beast feeds seldom: one year in ten, and it sleeps on a full belly.
        if r.gen::<f32>() >= 0.1 {
            continue;
        }
        let region = realm.beasts[b].region;
        let Some(site) = realm
            .sites
            .iter()
            .filter(|s| s.alive() && s.deep && dist(n, s.region, region) <= BEAST_REACH)
            .min_by_key(|s| (dist(n, s.region, region), s.id))
            .map(|s| s.id)
        else {
            continue;
        };
        let s = &mut realm.sites[site as usize];
        let dead = (s.population * r.gen_range(0.1..0.3)).ceil() as u32;
        s.population = (s.population - dead as f32).max(0.0);
        let left = s.population;
        let beast = realm.beasts[b].id;
        push(
            out,
            year,
            RealmEventKind::BeastAttacked { beast, site, dead },
        );
        let name = realm.beasts[b].name.clone();
        // Figures of the place may be among the dead.
        let there: Vec<u32> = realm
            .figures
            .iter()
            .filter(|f| f.alive() && f.site == site)
            .map(|f| f.id)
            .collect();
        for f in there {
            if r.gen::<f32>() < 0.15 {
                let cause = format!("slain by {name}");
                realm.figures[f as usize].died = Some((year, cause.clone()));
                push(out, year, RealmEventKind::Died { figure: f, cause });
            }
        }
        if left < 4.0 {
            realm.end_site(site, &format!("ruined by {name}"), out);
            realm.beasts[b].ruined.push(site);
        }
    }
}

fn wake(realm: &mut Realm, m: &RealmMap, r: &mut impl Rng, out: &mut Vec<RealmEvent>) {
    let n = m.size.max(1);
    // Under a mountain near the deep's settlements if there are any, else under any mountain.
    let deep: Vec<usize> = realm
        .sites
        .iter()
        .filter(|s| s.alive() && s.deep)
        .map(|s| s.region)
        .collect();
    let mountains: Vec<usize> = (0..n * n)
        .filter(|&i| m.biome[i] == RBiome::Mountain)
        .collect();
    let near: Vec<usize> = mountains
        .iter()
        .copied()
        .filter(|&i| deep.iter().any(|&d| dist(n, d, i) <= BEAST_REACH))
        .collect();
    let pool = if near.is_empty() { &mountains } else { &near };
    if pool.is_empty() {
        return;
    }
    let region = pool[r.gen_range(0..pool.len())];
    let id = realm.beasts.len() as u32;
    let (plan, form) = PLANS[r.gen_range(0..PLANS.len())];
    let made = MADE[r.gen_range(0..MADE.len())];
    let layer = if made == "magma" || r.gen_bool(0.3) {
        "underworld"
    } else {
        "deep_caverns"
    };
    // Its name in the words of the deep people nearest it, or the world's first people.
    let teller = realm
        .sites
        .iter()
        .filter(|s| s.deep)
        .min_by_key(|s| (dist(n, s.region, region), s.id))
        .map(|s| s.people)
        .unwrap_or(0);
    let word = realm
        .peoples
        .get(teller as usize)
        .map(|p| p.language.name(realm.seed, "beast", id as u64))
        .unwrap_or_else(|| "Unnamed".into());
    let mut epithet = made.to_string();
    if let Some(c) = epithet.get_mut(0..1) {
        c.make_ascii_uppercase();
    }
    realm.beasts.push(Beast {
        id,
        name: format!("{word}, the {epithet} {form}"),
        plan: plan.to_string(),
        made_of: made.to_string(),
        layer: layer.to_string(),
        region,
        woke: realm.year,
        ruined: Vec::new(),
    });
    push(
        out,
        realm.year,
        RealmEventKind::BeastWoke { beast: id, region },
    );
}

/// The beast lairing nearest a region in a layer, within its reach, if any.
pub fn near<'a>(realm: &'a Realm, region: usize, layer: &str) -> Option<&'a Beast> {
    let n = realm.map_size.max(1);
    realm
        .beasts
        .iter()
        .filter(|b| b.layer == layer && dist(n, b.region, region) <= BEAST_REACH)
        .min_by_key(|b| (dist(n, b.region, region), b.id))
}
