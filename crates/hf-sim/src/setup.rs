//! The new-game choices (Phase 17d): who watches, how deep the valley's past runs, what kind of
//! valley it is, which three begin, and what they hold in common.
//!
//! Everything here is chosen before the world exists, because each choice shapes what is
//! simulated: the deity watches the history pass as it watches the living valley, the depth
//! sets how many years of it run, the character steers generation. The same setup always
//! makes the same world. The choice itself is recorded as an event at the arrival, so the
//! founders' shared convictions have a source like any other belief.

use crate::engine::Engine;
use crate::events::{EventKind, Severity};
use crate::opening::{PARTY, STARTING};
use crate::past::Topic;
use crate::world::gen::biome::Character;
use crate::world::gen::GenParams;
use crate::World;
use serde::{Deserialize, Serialize};

/// The history depths offered, in years before the portal.
pub const HISTORY_CHOICES: [u64; 4] = [0, 50, 150, 300];
/// The default depth.
pub const HISTORY_DEFAULT: u64 = 150;
/// Years of the wider world's history (realm/, M3) before the portal, by default.
pub const REALM_DEFAULT_YEARS: u64 = 500;
/// The world ages the wizard offers.
pub const REALM_YEAR_CHOICES: [u64; 3] = [200, 500, 1000];

/// The region a setup's valley is, and the climate it gives (from_setup and the wizard's
/// preview share this, so what the player sees is what they get).
pub fn resolve_region(seed: u64, m: &crate::realm::RealmMap, region: Option<usize>) -> usize {
    region
        .filter(|&r| r < m.size * m.size && crate::realm::embark::habitable(m, r))
        .unwrap_or_else(|| crate::realm::embark::default_region(seed, m))
}

/// The place a region's valley is drawn for (#560, GenParams::place): 0 for the region the
/// seed itself would choose, so a world nobody chose a region for keeps the seed's own ground,
/// and the region's index plus one anywhere else, so each region is its own valley.
pub fn place_of(seed: u64, m: &crate::realm::RealmMap, region: usize) -> u64 {
    if region == crate::realm::embark::default_region(seed, m) {
        0
    } else {
        region as u64 + 1
    }
}

/// The generation parameters of the valley at a resolved region (from_setup and the wizard's
/// preview share this, so what the player sees is what they get): the region's climate unless
/// a character was chosen, its own ground, and the lie of that ground's land unless chosen.
fn valley_params(
    seed: u64,
    m: &crate::realm::RealmMap,
    region: usize,
    character: Option<Character>,
    landform: Option<crate::world::gen::Landform>,
) -> GenParams {
    let place = place_of(seed, m, region);
    GenParams {
        character: character.or(Some(crate::realm::embark::character_of(m, region))),
        landform: landform.unwrap_or_else(|| {
            crate::world::gen::Landform::of_seed(crate::world::gen::ground_seed(seed, place))
        }),
        place,
        ..GenParams::default()
    }
}

/// A few lines about a region, for the wizard: its country, climate and water, and whether
/// people could live there.
pub fn region_lines(m: &crate::realm::RealmMap, i: usize) -> Vec<String> {
    let mut out = vec![format!(
        "{}, about {:.0} C through the year, wetter than {:.0}% of the land",
        m.biome[i].label(),
        m.temperature[i],
        m.rain[i] * 100.0
    )];
    let (x, y) = m.xy(i);
    let near_river = (x.saturating_sub(1)..=(x + 1).min(m.size - 1))
        .any(|xx| (y.saturating_sub(1)..=(y + 1).min(m.size - 1)).any(|yy| m.river[m.idx(xx, yy)]));
    if near_river {
        out.push("A river runs by.".into());
    }
    if !crate::realm::embark::habitable(m, i) {
        out.push("Nobody could live here.".into());
    }
    out
}

fn realm_default_years() -> u64 {
    REALM_DEFAULT_YEARS
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WorldSetup {
    pub seed: u64,
    /// The lie of the land (#352): drawn from the seed unless chosen.
    #[serde(default)]
    pub landform: Option<crate::world::gen::Landform>,
    /// Who watches (the three deities are the three Engine modes).
    pub engine: Engine,
    /// Years of the valley's past to simulate before the portal opens.
    pub history_years: u64,
    /// The valley's character; None draws it from the seed.
    pub character: Option<Character>,
    /// The three of the ten who begin together.
    pub chosen: [u8; STARTING],
    /// Per-slot reroll counters from the arrival cards.
    pub rerolls: [u32; PARTY],
    /// Convictions the founders share, each a stance from -1 to 1 (past.rs Topic).
    pub ideology: Vec<(Topic, f32)>,
    /// Years of the wider world's history before the portal (0: no world beyond the valley).
    #[serde(default = "realm_default_years")]
    pub realm_years: u64,
    /// The region of the world map the valley is; None lets the seed choose a habitable one.
    #[serde(default)]
    pub region: Option<usize>,
    /// Which peoples the world draws, how many, how rare (P5).
    #[serde(default)]
    pub peoples: crate::species::PeopleSetup,
}

impl WorldSetup {
    pub fn new(seed: u64) -> WorldSetup {
        WorldSetup {
            seed,
            engine: Engine::default(),
            history_years: HISTORY_DEFAULT,
            character: None,
            chosen: [0, 1, 2],
            rerolls: [0; PARTY],
            ideology: Vec::new(),
            realm_years: REALM_DEFAULT_YEARS,
            region: None,
            peoples: Default::default(),
            landform: None,
        }
    }

    /// The lie of the land: as chosen, else the seed's (#352).
    pub fn landform(&self) -> crate::world::gen::Landform {
        self.landform
            .unwrap_or_else(|| crate::world::gen::Landform::of_seed(self.seed))
    }

    /// The generation parameters this setup asks for.
    pub fn params(&self) -> GenParams {
        GenParams {
            character: self.character,
            landform: self.landform(),
            ..GenParams::default()
        }
    }

    /// One line for the record and the screen.
    fn summary(&self, w: &World) -> (String, String) {
        let ch = self
            .character
            .unwrap_or_else(|| Character::of_seed(self.seed))
            .label();
        // Read after "held in common that": "A", "A and that B", "A, that B and that C".
        let words: Vec<String> = self
            .ideology
            .iter()
            .map(|(t, s)| held_words(*t, *s))
            .collect();
        let held = match words.split_last() {
            None => String::new(),
            Some((last, [])) => last.clone(),
            Some((last, rest)) => format!("{} and that {last}", rest.join(", that ")),
        };
        // Record the landform separately from the landing site climate.
        let lf = w.meta.params.landform;
        let ch = format!("{ch} ({})", lf.name());
        (ch, held)
    }
}

/// What a stance on a topic sounds like as a shared conviction. Every one is a statement, so
/// it reads whole on its own (the wizard's buttons) and after "that" (the arrival's "held in
/// common that ...", a people "who hold that ...").
pub fn held_words(t: Topic, s: f32) -> String {
    let side = s >= 0.0;
    let words = match (t, side) {
        (Topic::Drink, false) => "drink ruins people",
        (Topic::Drink, true) => "a drink eases a hard life",
        (Topic::Violence, false) => "no hand is raised in anger",
        (Topic::Violence, true) => "force answers force",
        (Topic::Fire, false) => "a fire is fought",
        (Topic::Fire, true) => "you run from a fire",
        (Topic::Hunger, false) => "in hunger, food is shared",
        (Topic::Hunger, true) => "in hunger, each keeps their own",
        (Topic::Strangers, false) => "strangers are welcome",
        (Topic::Strangers, true) => "strangers are a danger",
        (Topic::Death, false) => "the dead are let go",
        (Topic::Death, true) => "the living are held close",
        (Topic::Trust, false) => "wrongs are forgiven",
        (Topic::Trust, true) => "trust is earned slowly",
        // Acts and values (P7) in the words a belief is said in.
        (t, side) => crate::mind::beliefs::phrase(t, if side { 1.0 } else { -1.0 }),
    };
    words.to_string()
}

impl World {
    /// Make the world a setup describes: generate the valley, let the chosen deity watch its
    /// past, bring the three through at seven on the first morning of spring, and give them
    /// what they hold in common.
    pub fn from_setup(setup: &WorldSetup) -> World {
        let mut s = setup.clone();
        let years = s.history_years.min(*HISTORY_CHOICES.last().unwrap());
        // The world first (M3): its map, the region the valley is, and that region's climate
        // for the valley unless the setup chose one.
        let map = (s.realm_years > 0)
            .then(|| crate::realm::generate(s.seed, &crate::realm::RealmParams::default()));
        let mut params = s.params();
        if let Some(m) = &map {
            let region = resolve_region(s.seed, m, s.region);
            s.region = Some(region);
            params = valley_params(s.seed, m, region, s.character, s.landform);
            s.character = params.character;
        }
        // The clock starts at the arrival, late enough that everyone's whole life fits before
        // it (D18), the valley's own past (deep.rs) and the world's (realm/).
        let epoch = crate::past::epoch(s.seed, &s.rerolls)
            .max((years + 2) * crate::time::TICKS_PER_YEAR)
            .max((s.realm_years + 2) * crate::time::TICKS_PER_YEAR);
        let mut w = World::generate_at(s.seed, params, epoch);
        // The species generated for this world (P6), before its peoples are drawn.
        w.generated = crate::species::generate(
            s.seed,
            crate::species::generated_count(&s.peoples.generated),
        )
        .into_iter()
        .map(|d| &*Box::leak(Box::new(d)))
        .collect();
        w.meta.engine = s.engine;
        w.meta.setup = Some(s.clone());
        if let (Some(m), Some(region)) = (&map, s.region) {
            let (realm, events) = crate::realm::sites::run_with(
                s.seed,
                m,
                s.realm_years as i64,
                &[region],
                &s.peoples,
                &w.generated,
            );
            let mut ids = Vec::with_capacity(events.len());
            for (tick, sev, kind) in crate::realm::chronicle::to_events(&events, realm.year, epoch)
            {
                ids.push(w.events.push(tick, None, sev, kind));
            }
            let mut realm = realm;
            crate::realm::stories::remap(&mut realm, &ids);
            // The valley's neighbours are the world's nearest places.
            w.overworld = crate::expedition::Overworld::from_realm(&realm, m, region);
            w.realm = realm;
            w.realm.arrived = w.realm.year;
            w.realm_map = Some(m.clone());
        }
        crate::deep::run(&mut w, years);
        w.meta.tick = epoch + 7 * crate::time::TICKS_PER_HOUR;
        w.rebuild_caches();
        // The choices were made before anyone stepped through: recorded first, at the arrival.
        let (valley, held) = s.summary(&w);
        let d = crate::deity::of(s.engine);
        let choice = w.events.push(
            w.meta.tick,
            Some(w.meta.founding_site),
            Severity::Notable,
            EventKind::SetupChosen {
                deity: format!("{}, {}", d.name, d.title),
                history_years: years,
                valley,
                held,
                peoples: s.peoples.summary(),
            },
        );
        w.begin_opening(&s.chosen, &s.rerolls);
        // After the party, so the three who begin are the first people the world has (ids 0..3).
        crate::creatures::place_hart(&mut w);
        // The stone they came through by.
        crate::thin::place_stone(&mut w);
        if !s.ideology.is_empty() {
            w.seed_ideology(&s.ideology, choice);
        }
        // What the three are, as a people, from the first morning.
        crate::systems::culture::identity(&mut w);
        w
    }
}

/// A look at the valley a setup would make, before anything is simulated: the character and
/// biome lines `ostrog describe` prints, and per column what the main view shows from
/// above. Generation alone takes milliseconds.
pub struct Preview {
    pub width: i32,
    pub height: i32,
    pub lines: Vec<String>,
    /// Per column, the biome index (biome::Biome as u8), or 255 for open water.
    pub columns: Vec<u8>,
    /// Per column, the packed tile (Tile::pack) the main view draws from above: the water at
    /// the surface where there is water, else the top of the ground (#560).
    pub tiles: Vec<u32>,
    /// Per column, the ground's level (WorldGrid::ground_z).
    pub heights: Vec<i32>,
    /// Per column, the kind of the plant standing there (PlantKind as u8) plus one, or 0.
    pub plants: Vec<u8>,
    pub founding: crate::ids::TilePos,
    /// The character used: the one given, or the region's.
    pub character: Character,
    /// The region of the world the valley is.
    pub region: usize,
}

/// The valley a setup would make, before anything is simulated: from the given character, or
/// from its region's climate (the region given, if people could live there, else the seed's).
pub fn preview(seed: u64, character: Option<Character>, region: Option<usize>) -> Preview {
    let m = crate::realm::generate(seed, &crate::realm::RealmParams::default());
    preview_in(seed, &m, character, region)
}

/// The character a region's valley has unless one is chosen, without generating it.
pub fn region_character(seed: u64, m: &crate::realm::RealmMap, region: Option<usize>) -> Character {
    crate::realm::embark::character_of(m, resolve_region(seed, m, region))
}

/// `preview` on a world map already made (the wizard keeps one per seed).
pub fn preview_in(
    seed: u64,
    m: &crate::realm::RealmMap,
    character: Option<Character>,
    region: Option<usize>,
) -> Preview {
    let region = resolve_region(seed, m, region);
    let p = valley_params(seed, m, region, character, None);
    let g = crate::world::gen::generate(seed, &p);
    let lines =
        crate::world::gen::biome::describe(seed, &p, &g.grid, &g.plants, Some(&g.flora_draw))
            .into_iter()
            .filter(|l| {
                l.starts_with("character")
                    || l.starts_with("biome mix")
                    || l.starts_with("flora draw:")
                    || l.starts_with("flora not drawn:")
            })
            .collect();
    let n = (g.grid.width * g.grid.height) as usize;
    let mut columns = Vec::with_capacity(n);
    let mut tiles = Vec::with_capacity(n);
    let mut heights = Vec::with_capacity(n);
    for y in 0..g.grid.height {
        for x in 0..g.grid.width {
            let top = g.grid.surface(x, y);
            let ground = top.offset(0, 0, -1);
            let wet = g.grid.get(top).is_water() || g.grid.get(ground).is_water();
            columns.push(if wet {
                255
            } else {
                g.grid.biome_at(x, y).map(|b| b as u8).unwrap_or(0)
            });
            let shown = if g.grid.get(top).is_water() {
                top
            } else {
                ground
            };
            tiles.push(g.grid.get(shown).pack());
            heights.push(g.grid.ground_z(x, y));
        }
    }
    let mut plants = vec![0u8; n];
    for pl in &g.plants {
        let (x, y) = (pl.pos.x, pl.pos.y);
        if pl.alive
            && x >= 0
            && y >= 0
            && x < g.grid.width
            && y < g.grid.height
            && pl.pos.z == g.grid.ground_z(x, y) + 1
        {
            plants[(y * g.grid.width + x) as usize] = pl.kind as u8 + 1;
        }
    }
    Preview {
        width: g.grid.width,
        height: g.grid.height,
        lines,
        columns,
        tiles,
        heights,
        plants,
        founding: g.founding_site,
        character: Character::resolve(seed, &p),
        region,
    }
}

impl Preview {
    /// Examples for the landing note: two abundant kinds and up to three other wild
    /// species from the world's flora draw found on this surface.
    pub fn surface_plant_examples(&self) -> Vec<(crate::plants::PlantKind, usize)> {
        use crate::plants::PlantKind;
        use crate::world::gen::flora::FLORA_CANDIDATES;
        use std::cmp::Reverse;

        let mut counts = [0usize; 256];
        for &column in &self.plants {
            counts[usize::from(column)] += 1;
        }
        let mut present: Vec<_> = PlantKind::ALL
            .into_iter()
            .map(|kind| (kind, counts[kind as usize + 1]))
            .filter(|(kind, n)| *n > 0 && kind.grows())
            .collect();
        present.sort_by_key(|&(kind, n)| (Reverse(n), kind));
        present
            .iter()
            .take(2)
            .chain(
                present
                    .iter()
                    .skip(2)
                    .filter(|(kind, _)| FLORA_CANDIDATES.contains(kind))
                    .take(3),
            )
            .copied()
            .collect()
    }
}
