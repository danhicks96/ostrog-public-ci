//! A world with a past (Phase 17c, issue #50): the valley before the portal opened.
//!
//! When an opening world is made, the valley is simulated year by year for the history depth
//! (HISTORY_YEARS) at a coarse grain: hamlets are founded on good ground, their named people are
//! born and die, good and bad years come, famine, fever, fire and raids from the places beyond
//! strike, and in the end each hamlet empties: starved out, burned, or its last people gone to a
//! neighbouring place. What is left is laid on the map, aged by how long ago it was left: timber
//! walls rot in decades and stone stands for centuries, the dead lie under grave markers, roads
//! that carried traffic stay as packed earth until they grow over, and tools lie where they were
//! dropped, each with a story that names its maker and how it came to be there. Every step is a
//! chronicle row before the arrival, so History can always say why a ruin is where it is.
//!
//! Nothing is invented after the fact: every trace comes from a row this pass recorded.
use crate::fmath::Det;

use crate::deity::{Lever, Motive, Trigger};
use crate::events::{EventKind, Severity};
use crate::ids::TilePos;
use crate::rng::{purpose, stream};
use crate::time::TICKS_PER_YEAR;
use crate::world::{Material, Shape, Tile};
use crate::World;
use rand::Rng;
use serde::{Deserialize, Serialize};

/// The default depth of the valley's past (setup.rs offers others).
pub const HISTORY_YEARS: u64 = crate::setup::HISTORY_DEFAULT;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OldSite {
    pub id: u32,
    pub name: String,
    pub centre: TilePos,
    /// Years before the arrival.
    pub founded: u64,
    pub ended: Option<(u64, String)>,
    pub people: u32,
    pub peak: u32,
    /// House origins (5×4 outside) and their wall material.
    pub houses: Vec<(TilePos, Material)>,
    pub burned: bool,
    /// Where they dug a well, for a hamlet with no water in reach.
    #[serde(default)]
    pub well: Option<TilePos>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Figure {
    pub id: u32,
    pub name: String,
    pub site: u32,
    pub born: u64,
    /// (years before arrival, cause)
    pub died: Option<(u64, String)>,
    pub craft: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Grave {
    pub pos: TilePos,
    pub figure: u32,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct History {
    pub years: u64,
    pub sites: Vec<OldSite>,
    pub figures: Vec<Figure>,
    pub graves: Vec<Grave>,
    /// Each time the deity leaned on an old hamlet's year.
    #[serde(default)]
    pub answers: Vec<OldAnswer>,
}

/// The deity leaning on one old hamlet's year (deity.rs, in the past).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OldAnswer {
    /// Years before the arrival.
    pub year: u64,
    pub site: u32,
    pub trigger: Trigger,
    pub lever: Lever,
    /// Multiplier on the hamlet's harvest, or on its chance of fire or fever.
    pub strength: f32,
    /// Whether what it leaned toward came to pass that year, so people could see it.
    pub followed: bool,
}

/// What the deity notices about one hamlet, year to year.
#[derive(Clone, Copy, Default)]
struct Watch {
    /// Consecutive years with more than enough people.
    fat: u32,
    /// Consecutive hard years (trouble, or too few).
    hard: u32,
    /// Years since any trouble.
    quiet: u32,
    /// The year it last answered here.
    last: Option<u64>,
    /// What it last answered here (#211): the same stretch is not answered again for a
    /// generation.
    last_trigger: Option<Trigger>,
}

/// Whether the deity answers what it sees at a hamlet this year: (trigger, lever, strength).
/// The same motives as in the living valley: Aun lowers the high and lifts the low, Hask
/// presses whoever is strong, Mirre stirs a place that has been quiet too long.
fn old_answer(
    engine: crate::engine::Engine,
    at: &Watch,
    y: u64,
    r: &mut impl Rng,
) -> Option<(Trigger, Lever, f32)> {
    if at.last.is_some_and(|l| l.saturating_sub(y) < 5) {
        return None;
    }
    let answer = old_motive(engine, at, r)?;
    // Not the same kind year for the same hard stretch every five years for thirty (#211): a
    // hardship answered once is left to run its course for twenty years. (Hask's pressing of
    // the strong and Mirre's stirring are their natures, and repeat.)
    // Aun's answers to plenty as well (#246: "held back the harvest" at one hamlet four
    // times five years apart); Hask's pressing of the strong and Mirre's stirring repeat by
    // their natures.
    let paced = answer.0 == Trigger::Hardship
        || (answer.0 == Trigger::Plenty && crate::deity::of(engine).motive == Motive::Balance);
    if paced
        && at.last_trigger == Some(answer.0)
        && at.last.is_some_and(|l| l.saturating_sub(y) < 20)
    {
        return None;
    }
    Some(answer)
}

fn old_motive(
    engine: crate::engine::Engine,
    at: &Watch,
    r: &mut impl Rng,
) -> Option<(Trigger, Lever, f32)> {
    match crate::deity::of(engine).motive {
        Motive::Balance => {
            if at.fat >= 3 {
                Some((Trigger::Plenty, Lever::Harvest, 0.7))
            } else if at.hard >= 3 {
                Some((Trigger::Hardship, Lever::Harvest, 1.35))
            } else {
                None
            }
        }
        // Strong: well peopled for two years, or untroubled for seven.
        Motive::Test => (at.fat >= 2 || at.quiet >= 7).then(|| {
            if r.gen_bool(0.5) {
                (Trigger::Plenty, Lever::DryStorms, 12.0)
            } else {
                (Trigger::Plenty, Lever::Sickness, 7.0)
            }
        }),
        Motive::Amusement => (at.quiet >= 10 && r.gen_bool(0.5)).then(|| {
            let (l, s) = [
                (Lever::Harvest, 0.7),
                (Lever::DryStorms, 12.0),
                (Lever::Sickness, 7.0),
                (Lever::Harvest, 1.35),
            ][r.gen_range(0..4)];
            (Trigger::LongQuiet, l, s)
        }),
    }
}

/// "41 years before the portal".
pub fn ago(years: u64) -> String {
    match years {
        0 => "the year the portal opened".to_string(),
        1 => "the year before the portal".to_string(),
        n => format!("{n} years before the portal"),
    }
}

pub(crate) fn hamlet_name(seed: u64, id: u32) -> String {
    let mut r = stream(seed, &[purpose::HISTORY, 0x4A3E, id as u64]);
    let a = [
        "Ash", "Old", "Low", "High", "Wether", "Barrow", "Mill", "Cold", "Whin", "Hob", "Bram",
        "Sedge",
    ];
    let b = [
        "ton", "ley", "by", "thorpe", "stow", "worth", "hythe", "combe", "wick", "field",
    ];
    format!(
        "{}{}",
        a[r.gen_range(0..a.len())],
        b[r.gen_range(0..b.len())]
    )
}

/// The technique a craft of the old valley can teach (C13).
pub fn craft_teaches(craft: &str) -> Option<crate::mind::techniques::Technique> {
    use crate::mind::techniques::Technique as T;
    Some(match craft {
        "smith" => T::Smithing,
        "carpenter" => T::Roofing,
        "weaver" => T::Weaving,
        "herb-wife" => T::HerbLore,
        "potter" => T::Hearthcraft,
        _ => return None,
    })
}

/// Run the history pass on a freshly generated opening world (before the party arrives).
pub fn run(w: &mut World, years: u64) {
    let years = (w.meta.epoch / TICKS_PER_YEAR).saturating_sub(1).min(years);
    if years < 10 {
        return;
    }
    let seed = w.meta.seed;
    let mut r = stream(seed, &[purpose::HISTORY]);
    let mut h = History {
        years,
        ..Default::default()
    };
    let neighbours: Vec<String> = w
        .overworld
        .sites
        .iter()
        .filter(|s| s.people > 0)
        .map(|s| s.name.clone())
        .collect();
    let mut fig_seq = 0u32;
    // Good ground for hamlets, found once.
    let mut spots = hamlet_spots(w, &mut r);
    let tick_of = |w: &World, y: u64, season: u64| {
        w.meta
            .epoch
            .saturating_sub(y * TICKS_PER_YEAR)
            .saturating_add(season * crate::time::DAYS_PER_SEASON * crate::time::TICKS_PER_DAY)
    };
    // Who lived here: the people of the world's nearest settlement (M3), who name their
    // hamlets and children in their own tongue; in a world without a realm, the old tables.
    let valley_people = w
        .meta
        .setup
        .as_ref()
        .and_then(|s| s.region)
        .and_then(|r| w.realm.nearest_people(r))
        .cloned();
    let person_name = |id: u32| match &valley_people {
        Some(p) => p.language.name(seed, "valley person", id as u64),
        None => crate::names::person_name(seed, 2_000_000 + id),
    };
    let place_name = |id: u32| match &valley_people {
        Some(p) => p.language.name(seed, "valley hamlet", id as u64),
        None => hamlet_name(seed, id),
    };
    let new_figure =
        |h: &mut History, seq: &mut u32, site: u32, born: u64, craft: Option<String>| {
            let id = *seq;
            *seq += 1;
            h.figures.push(Figure {
                id,
                name: person_name(id),
                site,
                born,
                died: None,
                craft,
            });
            id
        };
    let mut watch: Vec<Watch> = Vec::new();
    // Ground a hamlet left, with the year it emptied: after a generation it is settled again,
    // on the old foundations.
    let mut fallow: Vec<(u64, Spot)> = Vec::new();
    // A deeper past holds more lives.
    let most = (3 + years / 30) as usize;
    for y in (1..=years).rev() {
        let mut k = 0;
        while k < fallow.len() {
            if fallow[k].0.saturating_sub(y) >= 30 {
                spots.push(fallow.remove(k).1);
            } else {
                k += 1;
            }
        }
        watch.resize(h.sites.len().max(watch.len()), Watch::default());
        let alive = h.sites.iter().filter(|s| s.ended.is_none()).count();
        // A new hamlet, now and then, while the valley has room.
        // Not in the last years: the valley empties before the portal opens.
        if y > 15
            && alive < 3
            && h.sites.len() < most
            && r.gen_bool(if alive == 0 { 0.3 } else { 0.07 })
        {
            let pick = spots.iter().position(|(c, _, _)| {
                !h.sites
                    .iter()
                    .any(|s| s.centre.dist(*c) < 20 && s.ended.is_none())
            });
            if let Some(site) = pick.map(|k| spots.remove(k)) {
                let id = h.sites.len() as u32;
                // A new name, even on old ground: tales must tell one place from another.
                let mut name = place_name(id);
                let mut alt = 1;
                while h.sites.iter().any(|s| s.name == name) && alt < 50 {
                    name = place_name(id + 1000 * alt);
                    alt += 1;
                }
                let a = new_figure(&mut h, &mut fig_seq, id, y + r.gen_range(18..35), None);
                let b = new_figure(&mut h, &mut fig_seq, id, y + r.gen_range(18..35), None);
                let crafts = ["smith", "carpenter", "potter", "weaver"];
                let c = new_figure(
                    &mut h,
                    &mut fig_seq,
                    id,
                    y + r.gen_range(20..40),
                    Some(crafts[r.gen_range(0..crafts.len())].to_string()),
                );
                let _ = c;
                let (an, bn) = (
                    h.figures[a as usize].name.clone(),
                    h.figures[b as usize].name.clone(),
                );
                let houses = site.1;
                let well = site.2;
                h.sites.push(OldSite {
                    id,
                    name: name.clone(),
                    centre: site.0,
                    founded: y,
                    ended: None,
                    people: r.gen_range(8..16),
                    peak: 0,
                    houses,
                    well,
                    burned: false,
                });
                let t = tick_of(w, y, 0);
                w.events.push(
                    t,
                    Some(site.0),
                    Severity::Notable,
                    EventKind::OldFounded {
                        site: name,
                        by: format!("{an} and {bn}"),
                        people: valley_people
                            .as_ref()
                            .map(|p| p.name.clone())
                            .unwrap_or_default(),
                    },
                );
            }
        }
        for k in 0..h.sites.len() {
            if h.sites[k].ended.is_some() {
                continue;
            }
            let centre = h.sites[k].centre;
            let name = h.sites[k].name.clone();
            let id = h.sites[k].id;
            // The deity watches the old valley as it watches the living one: it leans the
            // hamlet's year and sends nothing itself.
            watch.resize(h.sites.len(), Watch::default());
            let answer = old_answer(
                w.meta.engine,
                &watch[k],
                y,
                &mut stream(seed, &[purpose::HISTORY, 0xDE17, y, id as u64]),
            );
            let (harvest, storms, fever) = match answer {
                Some((_, Lever::Harvest, v)) => (v, 1.0, 1.0),
                Some((_, Lever::DryStorms, v)) => (1.0, v as f64, 1.0),
                Some((_, Lever::Sickness, v)) => (1.0, 1.0, v as f64),
                _ => (1.0, 1.0, 1.0),
            };
            let luck: f32 = r.gen_range(0.5..1.4) * harvest;
            let mut trouble: Option<(&'static str, String)> = None;
            if luck < 0.62 && r.gen_bool(0.5) {
                trouble = Some(("famine", format!("A famine winter at {name}.")));
            } else if r.gen_bool((0.04 * fever).min(0.9)) {
                trouble = Some(("fever", format!("Fever went through {name}.")));
            } else if r.gen_bool((0.02 * storms).min(0.9)) {
                trouble = Some(("fire", format!("{name} burned.")));
            } else if !neighbours.is_empty() && r.gen_bool(0.03) {
                let n = &neighbours[r.gen_range(0..neighbours.len())];
                trouble = Some(("raid", format!("Hungry people from {n} raided {name}.")));
            }
            // People: the year's luck, less what trouble took, toward what the land here can
            // feed (about forty).
            let now = h.sites[k].people as f32;
            let mut p = now * (1.0 + (0.14 * luck - 0.08) * (1.0 - now / 40.0).max(-1.0));
            if let Some((what, _)) = &trouble {
                p *= match *what {
                    "famine" => 0.7,
                    "fever" => 0.8,
                    "fire" => 0.75,
                    _ => 0.85,
                };
            }
            h.sites[k].people = p.round().max(0.0) as u32;
            h.sites[k].peak = h.sites[k].peak.max(h.sites[k].people);
            if let Some((trigger, lever, strength)) = answer {
                let what = trouble.as_ref().map(|t| t.0);
                let followed = match lever {
                    Lever::Harvest if strength < 1.0 => what == Some("famine"),
                    Lever::Harvest => what.is_none(),
                    Lever::DryStorms => what == Some("fire"),
                    Lever::Sickness => what == Some("fever"),
                    _ => false,
                };
                h.answers.push(OldAnswer {
                    year: y,
                    site: id,
                    trigger,
                    lever,
                    strength,
                    followed,
                });
                let d = crate::deity::of(w.meta.engine);
                let t = tick_of(w, y, 1);
                w.events.push(
                    t,
                    Some(centre),
                    Severity::Trace,
                    EventKind::OldAnswered {
                        site: name.clone(),
                        deity: d.name.to_string(),
                        trigger: trigger.label().to_string(),
                        lever: lever.label().to_string(),
                    },
                );
                watch[k].last = Some(y);
                watch[k].last_trigger = Some(trigger);
                // The stretch it answered is answered (#211: the same hard stretch at one
                // hamlet was answered every five years for thirty, "and it came to pass").
                if trigger == Trigger::Hardship {
                    watch[k].hard = 0;
                }
            }
            let people = h.sites[k].people;
            let at = &mut watch[k];
            at.fat = if people >= 24 { at.fat + 1 } else { 0 };
            at.hard = if trouble.is_some() || people < 8 {
                at.hard + 1
            } else {
                0
            };
            at.quiet = if trouble.is_some() { 0 } else { at.quiet + 1 };
            if let Some((_, line)) = &trouble {
                let t = tick_of(w, y, 3);
                w.events.push(
                    t,
                    Some(centre),
                    Severity::Notable,
                    EventKind::OldTrouble {
                        site: name.clone(),
                        what: line.clone(),
                    },
                );
            }
            // The named people of the hamlet: the old die, trouble takes some, children come.
            let here: Vec<usize> = (0..h.figures.len())
                .filter(|&f| h.figures[f].site == id && h.figures[f].died.is_none())
                .collect();
            for f in here {
                let age = h.figures[f].born.saturating_sub(y);
                let cause = if age > 50 && r.gen_bool(((age - 50) as f64 / 40.0).min(0.6)) {
                    Some("old age")
                } else if let Some((what, _)) = &trouble {
                    let p = match *what {
                        "famine" => 0.12,
                        "fever" => 0.15,
                        "fire" => 0.08,
                        _ => 0.1,
                    };
                    if r.gen_bool(p) {
                        Some(match *what {
                            "famine" => "hunger",
                            "fever" => "fever",
                            "fire" => "the fire",
                            _ => "the raid",
                        })
                    } else {
                        None
                    }
                } else {
                    None
                };
                if let Some(c) = cause {
                    h.figures[f].died = Some((y, c.to_string()));
                    let t = tick_of(w, y, 2);
                    w.events.push(
                        t,
                        Some(centre),
                        Severity::Notable,
                        EventKind::OldDied {
                            who: h.figures[f].name.clone(),
                            site: name.clone(),
                            cause: c.to_string(),
                        },
                    );
                }
            }
            if h.sites[k].people > 8 && r.gen_bool(0.25) {
                let crafts = ["smith", "carpenter", "potter", "weaver", "herb-wife"];
                let craft = if r.gen_bool(0.3) {
                    Some(crafts[r.gen_range(0..crafts.len())].to_string())
                } else {
                    None
                };
                new_figure(&mut h, &mut fig_seq, id, y, craft);
            }
            // The end of a hamlet.
            let few = h.sites[k].people < 4;
            let leave = !neighbours.is_empty() && h.sites[k].people < 12 && r.gen_bool(0.02);
            // A fire ends a hamlet only that year, if it leaves too few to rebuild.
            let burned_out = matches!(trouble, Some(("fire", _))) && h.sites[k].people < 5;
            if burned_out {
                h.sites[k].burned = true;
            }
            if few || leave || burned_out {
                let why = if leave || (few && !neighbours.is_empty() && r.gen_bool(0.5)) {
                    let n = &neighbours[r.gen_range(0..neighbours.len())];
                    format!("the last of them went to {n}")
                } else if burned_out {
                    "it was not rebuilt after the fire".to_string()
                } else {
                    match &trouble {
                        Some(("famine", _)) => "it emptied in the famine".to_string(),
                        Some(("fever", _)) => "the fever left too few to keep it".to_string(),
                        _ => "the young left and the old died".to_string(),
                    }
                };
                h.sites[k].ended = Some((y, why.clone()));
                fallow.push((y, (centre, h.sites[k].houses.clone(), h.sites[k].well)));
                let t = tick_of(w, y, 3);
                w.events.push(
                    t,
                    Some(centre),
                    Severity::Notable,
                    EventKind::OldEnded { site: name, why },
                );
            }
        }
    }
    // Anything still standing in the last years empties before the portal (the valley is the
    // party's to find empty; its people are the neighbours now).
    for k in 0..h.sites.len() {
        if h.sites[k].ended.is_none() {
            let why = match neighbours.first() {
                Some(n) => format!("the last of them went to {n}"),
                None => "the young left and the old died".to_string(),
            };
            h.sites[k].ended = Some((1, why.clone()));
            let t = tick_of(w, 1, 3);
            let (c, name) = (h.sites[k].centre, h.sites[k].name.clone());
            w.events.push(
                t,
                Some(c),
                Severity::Notable,
                EventKind::OldEnded { site: name, why },
            );
        }
    }
    lay_traces(w, &mut h, &mut r);
    w.history = h;
    w.rebuild_caches();
    w.nav = None;
}

/// Good spots for hamlets: flat, by water, away from the egress point and from each other.
/// Each is a centre and its house origins (5×4 each).
type Spot = (TilePos, Vec<(TilePos, Material)>, Option<TilePos>);

fn hamlet_spots(w: &World, r: &mut rand_chacha::ChaCha8Rng) -> Vec<Spot> {
    let land = w.meta.founding_site;
    let mut out: Vec<Spot> = Vec::new();
    // Good flat ground with no water in reach: a hamlet can live there if it digs a well.
    let mut dry: Vec<(TilePos, TilePos)> = Vec::new();
    for _ in 0..140 {
        if out.len() >= 6 {
            break;
        }
        let x = r.gen_range(12..w.grid.width - 18);
        let y = r.gen_range(12..w.grid.height - 18);
        // Flat ground at the height of the ground there, not of the egress point.
        let near = w.grid.surface(x, y);
        if TilePos::new(x, y, land.z).dist(land) < 28 {
            continue;
        }
        let Some(o) = crate::systems::settlement::find_flat_area(w, near, 12, 9, 1) else {
            continue;
        };
        let centre = o.offset(5, 4, 0);
        if o.dist(land) < 24
            || out.iter().any(|s| s.0.dist(centre) < 20)
            || dry.iter().any(|d| d.1.dist(centre) < 20)
            || o.x < 8
            || o.y < 8
            || o.x + 20 >= w.grid.width
            || o.y + 20 >= w.grid.height
        {
            continue;
        }
        // Water within reach.
        let wet = (-14..26).step_by(2).any(|dy| {
            (-14..26).step_by(2).any(|dx| {
                let p = w.grid.surface(
                    (o.x + dx).clamp(0, w.grid.width - 1),
                    (o.y + dy).clamp(0, w.grid.height - 1),
                );
                w.grid.get(p.offset(0, 0, -1)).is_water() || w.grid.get(p).is_water()
            })
        });
        if !wet {
            dry.push((o, centre));
            continue;
        }
        out.push((centre, houses(o, r), None));
    }
    // A valley with too little watered ground is settled where people can dig a well.
    for (o, centre) in dry {
        if out.len() >= 2 {
            break;
        }
        out.push((centre, houses(o, r), Some(o.offset(10, 6, 0))));
    }
    out
}

/// Two or three houses on a 12×9 flat area at `o`, all timber or all stone.
fn houses(o: TilePos, r: &mut rand_chacha::ChaCha8Rng) -> Vec<(TilePos, Material)> {
    let m = if r.gen_bool(0.3) {
        Material::STONE_BLOCK
    } else {
        Material::WOOD
    };
    let mut hs = vec![(o, m), (o.offset(6, 0, 0), m)];
    if r.gen_bool(0.5) {
        hs.push((o.offset(3, 5, 0), m));
    }
    hs
}

fn lay_traces(w: &mut World, h: &mut History, r: &mut rand_chacha::ChaCha8Rng) {
    let tick = |w: &World, y: u64| w.meta.epoch.saturating_sub(y * TICKS_PER_YEAR);
    for s in h.sites.clone() {
        let (gone, why) = s.ended.clone().unwrap_or((1, String::new()));
        // The well they dug outlasts the houses.
        if let Some(p) = s.well {
            w.plants.retain(|pl| pl.pos != p);
            if w.grid.walkable(p) && w.grid.get(p).shape == Shape::Open {
                w.place_structure(
                    crate::structures::StructureKind::Well,
                    p,
                    Material::GRANITE,
                    false,
                );
            }
        }
        // Houses: walls that have not yet fallen. Timber rots in decades (and burns); stone
        // stands for centuries.
        for (o, m) in &s.houses {
            for dy in 0..4 {
                for dx in 0..5 {
                    let p = o.offset(dx, dy, 0);
                    w.plants.retain(|pl| pl.pos != p);
                    let edge = dx == 0 || dy == 0 || dx == 4 || dy == 3;
                    let door = dy == 3 && dx == 2;
                    if !edge || door {
                        continue;
                    }
                    let keep = match *m {
                        Material::STONE_BLOCK => (-(gone as f32) / 220.0).d_exp(),
                        _ if s.burned => 0.0,
                        _ => (-(gone as f32) / 28.0).d_exp(),
                    };
                    if r.gen::<f32>() < keep {
                        w.grid.set(p, Tile::solid(*m));
                    }
                }
            }
        }
        // Graves of those who died here, in a row beside the hamlet.
        let dead: Vec<Figure> = h
            .figures
            .iter()
            .filter(|f| {
                f.site == s.id
                    && f.died
                        .as_ref()
                        .is_some_and(|d| d.1 != "the raid" || r.gen_bool(0.5))
            })
            .cloned()
            .collect();
        let row = s.centre.offset(-5, 6, 0);
        for (k, f) in dead.iter().enumerate().take(12) {
            let p = w.grid.surface(
                (row.x + (k as i32 % 6) * 2).clamp(1, w.grid.width - 2),
                (row.y + (k as i32 / 6) * 2).clamp(1, w.grid.height - 2),
            );
            if !w.grid.walkable(p) || w.grid.get(p).shape != Shape::Open {
                continue;
            }
            w.plants.retain(|pl| pl.pos != p);
            w.place_structure(
                crate::structures::StructureKind::Grave,
                p,
                Material::GRANITE,
                false,
            );
            h.graves.push(Grave {
                pos: p,
                figure: f.id,
            });
        }
        // What was left: a tool or two where a house stood, made by someone who lived here.
        let makers: Vec<Figure> = h
            .figures
            .iter()
            .filter(|f| f.site == s.id && f.craft.is_some())
            .cloned()
            .collect();
        let kinds = [
            crate::items::ItemKind::Tool,
            crate::items::ItemKind::Hammer,
            crate::items::ItemKind::Chisel,
            crate::items::ItemKind::Bucket,
            crate::items::ItemKind::Needle,
        ];
        let n = if s.burned { 1 } else { r.gen_range(1..4) };
        for _ in 0..n {
            let Some((o, _)) = s.houses.get(r.gen_range(0..s.houses.len())) else {
                break;
            };
            let p = o.offset(r.gen_range(1..4), r.gen_range(1..3), 0);
            let maker_fig = makers.get(r.gen_range(0..makers.len().max(1))).cloned();
            // What a craftsman leaves is the work of their craft; anyone else's, anything.
            use crate::items::ItemKind as K;
            let kind = match maker_fig.as_ref().and_then(|f| f.craft.as_deref()) {
                Some("smith") => [K::Tool, K::Hammer, K::Chisel][r.gen_range(0..3)],
                Some("carpenter") => [K::Saw, K::Chisel][r.gen_range(0..2)],
                Some("weaver") => K::Needle,
                Some(_) => K::Bucket,
                None => kinds[r.gen_range(0..kinds.len())],
            };
            let q = r.gen_range(0.3..0.75f32) * (-(gone as f32) / 160.0).d_exp();
            let id = w.create_lone_item(
                kind,
                1,
                crate::items::ItemPlace::Ground(p),
                q.max(0.1),
                "history",
            );
            // A tool shaped by a craft can show someone how it was done.
            let teaches = maker_fig
                .as_ref()
                .and_then(|f| f.craft.as_deref())
                .and_then(craft_teaches);
            let (maker, made) = match &maker_fig {
                Some(f) => (
                    format!(
                        "{}, the {} of {}",
                        f.name,
                        f.craft.clone().unwrap_or_default(),
                        s.name
                    ),
                    f.born.saturating_sub(25),
                ),
                None => (format!("someone of {}", s.name), s.founded),
            };
            // Made while the hamlet stood.
            let made_tick = tick(w, made.clamp(gone, s.founded));
            if let Some(it) = w.items.iter_mut().find(|it| it.id == id) {
                it.story = Some(Box::new(crate::legend::Story {
                    maker: None,
                    maker_name: maker,
                    made: made_tick,
                    wear: (-(gone as f32) / 90.0).d_exp().max(0.15),
                    origin: format!("left at {} when {} ({})", s.name, why, ago(gone)),
                    teaches,
                    ..Default::default()
                }));
            }
        }
        // A record: what a craftsman of the hamlet cut into slate, if it had one who could.
        if let Some(f) = makers
            .iter()
            .find(|f| f.craft.as_deref().and_then(craft_teaches).is_some())
        {
            if let Some((o, _)) = s.houses.first() {
                let p = o.offset(2, 1, 0);
                let id = w.create_lone_item(
                    crate::items::ItemKind::Record,
                    1,
                    crate::items::ItemPlace::Ground(p),
                    0.5,
                    "history",
                );
                let made_tick = tick(w, f.born.saturating_sub(30).clamp(gone, s.founded));
                if let Some(it) = w.items.iter_mut().find(|it| it.id == id) {
                    it.story = Some(Box::new(crate::legend::Story {
                        maker_name: format!(
                            "{}, the {} of {}",
                            f.name,
                            f.craft.clone().unwrap_or_default(),
                            s.name
                        ),
                        made: made_tick,
                        wear: 1.0,
                        origin: format!("left at {} when {} ({})", s.name, why, ago(gone)),
                        teaches: f.craft.as_deref().and_then(craft_teaches),
                        ..Default::default()
                    }));
                }
            }
        }
        // Roads out of the hamlet, toward the edge it traded across, worn by its years.
        let worn = (s.peak as f32) * ((s.founded - gone) as f32) * (-(gone as f32) / 45.0).d_exp();
        if worn > 60.0 {
            let edge = match r.gen_range(0..4) {
                0 => TilePos::new(s.centre.x, 2, 0),
                1 => TilePos::new(s.centre.x, w.grid.height - 3, 0),
                2 => TilePos::new(2, s.centre.y, 0),
                _ => TilePos::new(w.grid.width - 3, s.centre.y, 0),
            };
            road(w, s.centre, edge, r);
        }
    }
}

/// Lay packed earth along a wandering line from `a` toward `b`.
fn road(w: &mut World, a: TilePos, b: TilePos, r: &mut rand_chacha::ChaCha8Rng) {
    let (mut x, mut y) = (a.x as f32, a.y as f32);
    let steps = a.dist(b).max(1) * 2;
    let mut wob = 0.0f32;
    for _ in 0..steps {
        let dx = b.x as f32 - x;
        let dy = b.y as f32 - y;
        let len = (dx * dx + dy * dy).sqrt();
        if len < 1.0 {
            break;
        }
        wob = (wob + r.gen_range(-0.35..0.35f32)).clamp(-0.8, 0.8);
        let (ux, uy) = (dx / len, dy / len);
        x += ux * 0.5 - uy * wob * 0.5;
        y += uy * 0.5 + ux * wob * 0.5;
        let (ix, iy) = (x.round() as i32, y.round() as i32);
        if ix < 1 || iy < 1 || ix >= w.grid.width - 1 || iy >= w.grid.height - 1 {
            break;
        }
        let top = w.grid.surface(ix, iy).offset(0, 0, -1);
        let t = w.grid.get(top);
        if t.shape == Shape::Wall && t.material.is_vegetated()
            || matches!(
                t.material,
                Material::SOIL | Material::MEADOW | Material::MOSS
            )
        {
            w.grid.set(top, Tile::solid(Material::PACKED_EARTH));
        }
    }
}

/// What the history says about the tile at `p` (a grave or a ruin), for hover.
pub fn note(w: &World, p: TilePos) -> Option<String> {
    let h = &w.history;
    if let Some(g) = h.graves.iter().find(|g| g.pos == p) {
        let f = h.figures.iter().find(|f| f.id == g.figure)?;
        let site = h.sites.iter().find(|s| s.id == f.site)?;
        let (y, cause) = f.died.clone()?;
        let who = match &f.craft {
            Some(c) => format!("{}, the {c} of {}", f.name, site.name),
            None => format!("{} of {}", f.name, site.name),
        };
        let how = match cause.as_str() {
            "old age" => "who died old".to_string(),
            c => format!("who died of {c}"),
        };
        return Some(format!("The grave of {who}, {how}, {}.", ago(y)));
    }
    if let Some(s) = h.sites.iter().rev().find(|s| s.well == Some(p)) {
        return Some(format!(
            "The well the people of {} dug, {}.",
            s.name,
            ago(s.founded)
        ));
    }
    let here = |s: &&OldSite| {
        s.houses.iter().any(|(o, _)| {
            p.z == o.z && (o.x - 1..o.x + 6).contains(&p.x) && (o.y - 1..o.y + 5).contains(&p.y)
        })
    };
    // The latest to live here, and whoever lived here before them.
    let mut lived = h.sites.iter().filter(here).collect::<Vec<_>>();
    let s = lived.pop()?;
    let (y, why) = s.ended.clone()?;
    let before = lived
        .last()
        .map(|o| format!(" They built on the older ruins of {}.", o.name))
        .unwrap_or_default();
    Some(format!(
        "The ruins of {}: founded {}; {} {}.{before}",
        s.name,
        ago(s.founded),
        why,
        ago(y)
    ))
}

/// The old hamlets' houses as they lie now, for the client (#202, QA 2026-10-20: a ruin's
/// walls were drawn with the same crisp blocks as a wall built today): (tile, standing) for
/// each place an old house wall stood: standing while its wall is still there (and no one has
/// built on it since), else fallen, where its stones lie, unless something else is there now.
pub fn ruins(w: &World) -> Vec<(TilePos, bool)> {
    // The camps communities left since the arrival (C72) lie among the old hamlets' ruins.
    let mut out = crate::left_sites::ruins(w);
    for s in &w.history.sites {
        if s.ended.is_none() {
            continue;
        }
        for (o, m) in &s.houses {
            for dy in 0..4 {
                for dx in 0..5 {
                    let edge = dx == 0 || dy == 0 || dx == 4 || dy == 3;
                    if !edge || (dy == 3 && dx == 2) {
                        continue;
                    }
                    let p = o.offset(dx, dy, 0);
                    // A hamlet built on an older one's ruins shares its houses: once each.
                    if !w.grid.in_bounds(p)
                        || w.built_of.contains_key(&p)
                        || out.iter().any(|(q, _)| *q == p)
                    {
                        continue;
                    }
                    let t = w.grid.get(p);
                    if t.shape == Shape::Wall && t.material == *m {
                        out.push((p, true));
                    } else if t.shape == Shape::Open
                        && !t.is_water()
                        && w.grid.walkable(p)
                        && !w.structures.iter().any(|st| st.pos == p)
                    {
                        out.push((p, false));
                    }
                }
            }
        }
    }
    out
}
