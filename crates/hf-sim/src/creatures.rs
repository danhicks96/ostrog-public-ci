//! Creatures with depth (issue #51): animals with roles you can read by watching.
//!
//! - The bear dens up when autumn turns cold and is not seen all winter; it wakes in spring.
//! - Ducks fly off in autumn and come back to the water in spring.
//! - Crows come down where something has died (a carcass, a kill, a grave freshly dug).
//! - Wolves keep together behind the pack's leader and bring down deer together.
//! - The White Hart: one pale stag in most valleys, warier than any deer, never hunted for meat;
//!   whoever sees it remembers, and if it is ever brought down it is a legend.
//! - A beast that keeps hurting people is hunted down by those inclined to: its pelt becomes a
//!   trophy with a story.

use crate::agents::Species;
use crate::events::{EventKind, Severity};
use crate::ids::{AgentId, TilePos};
use crate::rng::{purpose, stream};
use crate::time::{Season, TICKS_PER_DAY, TICKS_PER_HOUR};
use crate::World;
use rand::Rng;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CreatureState {
    /// Ducks away south for the winter.
    pub migrants: Vec<crate::agents::Agent>,
    /// Where something died lately and until when the crows will come: (place, until, said).
    pub carrion: Vec<(TilePos, u64, bool)>,
    /// How many people each beast has hurt.
    pub wounds_by: Vec<(AgentId, u32)>,
    /// Who saw the White Hart last, and when.
    pub hart_seen: Vec<(AgentId, u64)>,
    /// The day each cavern's bats were last stirred (underground.rs), by a key per cavern.
    #[serde(default)]
    pub stirred: std::collections::BTreeMap<u64, u64>,
    /// #270: a beast drawn in by the settlement's stores, the stores it is going for, and
    /// until when it keeps at it.
    #[serde(default)]
    pub drawn: Vec<(AgentId, TilePos, u64)>,
    /// C75: beasts of the deep come up through a breach; they hunt the valley by night.
    #[serde(default)]
    pub risen: Vec<AgentId>,
    /// C75: people in a beast's shape under the full moon, until the given tick.
    #[serde(default)]
    pub shifted: Vec<(AgentId, u64)>,
    /// #273: a beast of the deep stirred by a breach, the cavern's index, and when it comes up.
    #[serde(default)]
    pub rising: Vec<(String, usize, u64)>,
    /// #455: beasts of the deep stirred by a breach, coming up by the noise made near the way
    /// (stirring.rs).
    #[serde(default)]
    pub stirring: Vec<crate::stirring::Stir>,
    /// #423: where each of the beasts whose bodies are not skeletons nests.
    #[serde(default)]
    pub nests: Vec<(AgentId, TilePos)>,
}

pub fn note_wound(w: &mut World, beast: AgentId) {
    match w.creatures.wounds_by.iter_mut().find(|x| x.0 == beast) {
        Some(x) => x.1 += 1,
        None => w.creatures.wounds_by.push((beast, 1)),
    }
}

pub fn wounds_by(w: &World, beast: AgentId) -> u32 {
    w.creatures
        .wounds_by
        .iter()
        .find(|x| x.0 == beast)
        .map(|x| x.1)
        .unwrap_or(0)
}

/// The pack's leader: the oldest wolf alive (lowest id).
pub fn pack_leader(w: &World) -> Option<usize> {
    w.agents
        .iter()
        .enumerate()
        .filter(|(_, a)| a.alive && a.species == Species::Wolf && !a.denned)
        .min_by_key(|(_, a)| a.id)
        .map(|(i, _)| i)
}

/// Fresh carrion near `p` the crows would go to.
pub fn carrion_near(w: &World, p: TilePos, reach: i32) -> Option<TilePos> {
    let now = w.meta.tick;
    w.creatures
        .carrion
        .iter()
        .filter(|(c, until, _)| *until > now && c.z == p.z && c.dist(p) <= reach)
        .min_by_key(|(c, _, _)| c.dist(p))
        .map(|(c, _, _)| *c)
}

pub fn step(w: &mut World) {
    let tick = w.meta.tick;
    // Every tick: deaths leave carrion.
    let deaths: Vec<TilePos> = w
        .last_events
        .iter()
        .filter_map(|e| match e.kind {
            EventKind::Died { .. } | EventKind::Hunted { .. } => e.at,
            _ => None,
        })
        .collect();
    for p in deaths {
        w.creatures.carrion.push((p, tick + TICKS_PER_DAY, false));
    }
    if !tick.is_multiple_of(TICKS_PER_HOUR) {
        return;
    }
    // A carcass picked over and gone greens the ground about it (ecology.rs, #486).
    let spent: Vec<TilePos> = w
        .creatures
        .carrion
        .iter()
        .filter(|c| c.1 <= tick)
        .map(|c| c.0)
        .collect();
    w.creatures.carrion.retain(|c| c.1 > tick);
    for at in spent {
        crate::ecology::carcass_spent(w, at);
    }
    let alive: std::collections::BTreeSet<AgentId> =
        w.agents.iter().filter(|a| a.alive).map(|a| a.id).collect();
    w.creatures.nests.retain(|n| alive.contains(&n.0));
    crows_gathered(w);
    hart_sightings(w);
    incursions(w);
    if tick % TICKS_PER_DAY == 4 * TICKS_PER_HOUR {
        seasons(w);
    }
}

fn crows_gathered(w: &mut World) {
    let tick = w.meta.tick;
    for k in 0..w.creatures.carrion.len() {
        let (c, _, said) = w.creatures.carrion[k];
        // The crows at it eat (ecology.rs, #486): fed crows bear young.
        let at: Vec<usize> = (0..w.agents.len())
            .filter(|&j| {
                let a = &w.agents[j];
                a.alive && a.species == Species::Crow && a.pos.z == c.z && a.pos.dist(c) <= 3
            })
            .collect();
        for &j in &at {
            crate::ecology::fed(w, j);
        }
        let crows = at.len();
        if said {
            continue;
        }
        if crows >= 2 {
            w.creatures.carrion[k].2 = true;
            w.events.push(
                tick,
                Some(c),
                Severity::Minor,
                EventKind::CrowsGathered {
                    at: c,
                    crows: crows as u32,
                },
            );
        }
    }
}

fn hart_sightings(w: &mut World) {
    let tick = w.meta.tick;
    let Some(h) = w
        .agents
        .iter()
        .position(|a| a.alive && a.species == Species::Hart)
    else {
        return;
    };
    let hp = w.agents[h].pos;
    let seers: Vec<usize> = (0..w.agents.len())
        .filter(|&i| {
            let a = &w.agents[i];
            a.alive
                && a.species == Species::Person
                && a.community.is_some()
                && !a.activity.is_asleep()
                && a.pos.z == hp.z
                && a.pos.dist(hp) <= 12
        })
        .collect();
    for i in seers {
        let id = w.agents[i].id;
        let recent = w
            .creatures
            .hart_seen
            .iter()
            .any(|(who, t)| *who == id && tick < t + 10 * TICKS_PER_DAY);
        if recent {
            continue;
        }
        // One record line a sighting (#226): the first witness this minute is Notable, the
        // others beside them Minor.
        let first = !w.creatures.hart_seen.iter().any(|(_, t)| *t == tick);
        w.creatures.hart_seen.retain(|(who, _)| *who != id);
        w.creatures.hart_seen.push((id, tick));
        w.agents[i].remember(
            tick,
            crate::mind::MemKind::Discovery,
            crate::mind::Subject::Place(hp),
            0.95,
            0.7,
            None,
        );
        let at = w.agents[i].pos;
        w.events.push(
            tick,
            Some(at),
            if first {
                Severity::Notable
            } else {
                Severity::Minor
            },
            EventKind::HartSighted { agent: id },
        );
    }
}

/// Daily at 04:00: the bear dens or wakes; the ducks leave or come back.
fn seasons(w: &mut World) {
    let tick = w.meta.tick;
    let cal = w.calendar();
    let cold = matches!(cal.season, Season::Winter)
        || (cal.season == Season::Autumn && cal.day_of_season >= 4);
    // Bears.
    for i in 0..w.agents.len() {
        let a = &w.agents[i];
        if !a.alive || a.species != Species::Bear {
            continue;
        }
        if cold && !a.denned {
            w.agents[i].denned = true;
            w.agents[i].path.clear();
            w.agents[i].activity = crate::mind::Activity::Idle;
            let (id, at) = (w.agents[i].id, w.agents[i].pos);
            w.events.push(
                tick,
                Some(at),
                Severity::Minor,
                EventKind::BearDenned { bear: id },
            );
        } else if !cold && a.denned {
            w.agents[i].denned = false;
            let (id, at) = (w.agents[i].id, w.agents[i].pos);
            w.events.push(
                tick,
                Some(at),
                Severity::Notable,
                EventKind::BearWoke { bear: id },
            );
        }
    }
    // Ducks.
    if cal.season == Season::Autumn && cal.day_of_season == 3 {
        let leaving: Vec<usize> = (0..w.agents.len())
            .filter(|&i| w.agents[i].alive && w.agents[i].species == Species::Duck)
            .collect();
        if !leaving.is_empty() {
            let n = leaving.len() as u32;
            let at = w.agents[leaving[0]].pos;
            for i in leaving.into_iter().rev() {
                let a = w.agents.remove(i);
                w.reservations.release_all(a.id);
                w.creatures.migrants.push(a);
            }
            w.events.push(
                tick,
                Some(at),
                Severity::Notable,
                EventKind::DucksLeft { count: n },
            );
        }
    }
    if cal.season == Season::Spring && cal.day_of_season == 2 && !w.creatures.migrants.is_empty() {
        let water = water_tile(w);
        let back = std::mem::take(&mut w.creatures.migrants);
        let n = back.len() as u32;
        let mut r = stream(w.meta.seed, &[purpose::ANIMALS, 0xD0C, tick]);
        for mut a in back {
            if let Some(p) = water {
                let q = w.grid.surface(
                    (p.x + r.gen_range(-3..=3)).clamp(1, w.grid.width - 2),
                    (p.y + r.gen_range(-3..=3)).clamp(1, w.grid.height - 2),
                );
                a.pos = q;
                a.from = q;
            }
            a.path.clear();
            a.activity = crate::mind::Activity::Idle;
            w.agents.push(a);
        }
        w.events.push(
            tick,
            water,
            Severity::Notable,
            EventKind::DucksReturned { count: n },
        );
    }
}

/// Somewhere by water to come back to.
fn water_tile(w: &World) -> Option<TilePos> {
    let land = w.meta.founding_site;
    let mut best: Option<(i32, TilePos)> = None;
    for y in (2..w.grid.height - 2).step_by(3) {
        for x in (2..w.grid.width - 2).step_by(3) {
            let p = w.grid.surface(x, y);
            let wet = w.grid.get(p.offset(0, 0, -1)).is_water() || w.grid.get(p).is_water();
            if wet && w.grid.walkable(p) {
                let d = p.dist(land);
                if best.map(|b| d < b.0).unwrap_or(true) {
                    best = Some((d, p));
                }
            }
        }
    }
    best.map(|b| b.1)
}

/// The White Hart comes to most valleys: one, far from where the party lands, in the trees.
pub fn place_hart(w: &mut World) {
    let mut r = stream(w.meta.seed, &[purpose::ANIMALS, 0x4A27]);
    if !r.gen_bool(0.65) {
        return;
    }
    let land = w.meta.founding_site;
    let trees: Vec<TilePos> = w
        .plants
        .iter()
        .filter(|p| p.alive && p.kind.is_green_tree() && p.pos.dist(land) > 35)
        .map(|p| p.pos)
        .collect();
    if trees.is_empty() {
        return;
    }
    for _ in 0..20 {
        let t = trees[r.gen_range(0..trees.len())];
        let p = w.grid.surface(t.x + 1, t.y);
        if w.grid.walkable(p) {
            let id = w.spawn_agent(Species::Hart, p);
            if let Some(a) = w.agents.iter_mut().find(|a| a.id == id) {
                a.name = "the White Hart".into();
            }
            return;
        }
    }
}

/// A beast killed by someone: a named one (or the Hart) leaves a trophy with its story.
pub fn slain(w: &mut World, hunter: usize, beast: usize) {
    let tick = w.meta.tick;
    let b = &w.agents[beast];
    let species = b.species;
    let plain = format!("{species:?}").to_lowercase();
    let named = b.name != plain;
    if !named {
        return;
    }
    let name = b.name.clone();
    let hurt = wounds_by(w, b.id);
    let at = w.agents[hunter].pos;
    let who = w.agents[hunter].id;
    let hname = w.agents[hunter].name.clone();
    let id = w.create_lone_item(
        crate::items::ItemKind::Hide,
        1,
        crate::items::ItemPlace::Ground(at),
        0.9,
        "trophy",
    );
    if let Some(it) = w.items.iter_mut().find(|it| it.id == id) {
        it.material = crate::world::material::hide_of(species).unwrap_or_default();
    }
    let what = if species == Species::Hart {
        "the White Hart's hide".to_string()
    } else {
        format!("{name}'s pelt")
    };
    let origin = if species == Species::Hart {
        format!("taken from the White Hart, brought down by {hname}")
    } else if hurt > 0 {
        format!(
            "taken from {name}, who had hurt {hurt} {}, by {hname}",
            if hurt == 1 { "person" } else { "people" }
        )
    } else {
        format!("taken from {name} by {hname}")
    };
    if let Some(it) = w.items.iter_mut().find(|it| it.id == id) {
        it.story = Some(Box::new(crate::legend::Story {
            maker_name: hname.clone(),
            made: tick,
            wear: 1.0,
            name: Some(what.clone()),
            origin,
            spoken_of: true,
            ..Default::default()
        }));
    }
    w.events.push(
        tick,
        Some(at),
        Severity::Major,
        EventKind::BeastSlain {
            agent: who,
            beast: name,
            hurt,
        },
    );
}

/// #270: what the settlement keeps draws things in. Once a day a fox, a swarm of rats (#423), or
/// out of winter a bear,
/// within sixty tiles may be drawn by the smell of food lying in store (more of it, more
/// likely); it walks in to the stores, eats there, a bear breaks what is beside it, and anyone
/// near may stand against it (systems/combat.rs). Each hour the drawn go on toward it.
fn incursions(w: &mut World) {
    use rand::Rng;
    let tick = w.meta.tick;
    let Some(home) = w.communities.iter().find(|c| !c.dissolved).map(|c| c.home) else {
        return;
    };
    // The drawn, an hour on.
    let drawn = std::mem::take(&mut w.creatures.drawn);
    let mut keep = Vec::new();
    for (id, at, until) in drawn {
        let Some(i) = w.agents.iter().position(|a| a.id == id && a.alive) else {
            continue;
        };
        if tick >= until {
            continue;
        }
        let me = w.agents[i].pos;
        if me.dist(at) > 1 {
            if w.path_agent_to(i, at) {
                w.agents[i].set_activity(crate::mind::Activity::Wandering { to: at });
            } else if w.agents[i].body.form.is_some() {
                // Rats get in through the gaps of a wooden wall (#423).
                crate::systems::animals::get_past(w, i, at);
            }
            keep.push((id, at, until));
            continue;
        }
        let bear = w.agents[i].species == Species::Bear;
        // It eats what lies there.
        let mut left = if bear { 6 } else { 2 };
        let mut took = 0;
        let near: Vec<(crate::ids::ItemId, u32)> = w
            .items
            .iter()
            .filter(|it| {
                it.kind.is_food()
                    && matches!(it.place, crate::items::ItemPlace::Ground(p) if p.dist(at) <= 2)
            })
            .map(|it| (it.id, it.count))
            .collect();
        for (iid, n) in near {
            if left == 0 {
                break;
            }
            let k = n.min(left);
            w.consume_item(iid, k, "eaten by a beast in the stores");
            left -= k;
            took += k;
        }
        // A bear breaks what is beside it.
        let mut broke = String::new();
        if bear {
            if let Some(s) = w
                .structures
                .iter_mut()
                .filter(|s| s.pos.z == at.z && s.pos.dist(at) <= 3 && s.condition > 0.3)
                .min_by_key(|s| s.pos.dist(at))
            {
                s.condition = (s.condition - 0.4).max(0.0);
                broke = format!("{:?}", s.kind).to_lowercase();
            }
        }
        let animal = w.agents[i].id;
        w.events.push(
            tick,
            Some(at),
            Severity::Major,
            EventKind::StoresRaided {
                animal,
                took,
                broke,
            },
        );
        // Anyone near may stand against it.
        if let Some(j) = (0..w.agents.len()).find(|&k| {
            let o = &w.agents[k];
            o.alive
                && o.species == Species::Person
                && o.community.is_some()
                && o.pos.z == at.z
                && o.pos.dist(at) <= 5
        }) {
            let mut r = crate::rng::stream(
                w.meta.seed,
                &[crate::rng::purpose::ANIMALS, 0x2700, animal.0 as u64, tick],
            );
            let (stood, felled) = crate::systems::combat::stand_against(w, i, j, &mut r);
            if !stood.is_empty() {
                w.events.push(
                    tick,
                    Some(at),
                    Severity::Major,
                    EventKind::StoodAgainst {
                        beast: animal,
                        by: stood,
                        felled,
                    },
                );
            }
        }
        // Off it goes.
        if let Some(i) = w.agents.iter().position(|a| a.id == animal && a.alive) {
            let away = TilePos::new(
                (at.x + (at.x - home.x).signum() * 20).clamp(1, w.grid.width - 2),
                (at.y + (at.y - home.y).signum() * 20).clamp(1, w.grid.height - 2),
                at.z,
            );
            let away = w.grid.surface(away.x, away.y);
            if w.grid.walkable(away) && w.path_agent_to(i, away) {
                w.agents[i].set_activity(crate::mind::Activity::Wandering { to: away });
            }
        }
    }
    w.creatures.drawn = keep;
    if tick % TICKS_PER_DAY != 3 * TICKS_PER_HOUR || !w.creatures.drawn.is_empty() {
        return;
    }
    // What lies in store, and where most of it is.
    let stores: Vec<(TilePos, u32)> = w
        .items
        .iter()
        .filter(|it| it.kind.is_food())
        .filter_map(|it| match it.place {
            crate::items::ItemPlace::Ground(p) if p.dist(home) <= 25 => Some((p, it.count)),
            _ => None,
        })
        .collect();
    let food: u32 = stores.iter().map(|s| s.1).sum();
    if food < 20 {
        return;
    }
    let mut r = crate::rng::stream(
        w.meta.seed,
        &[crate::rng::purpose::ANIMALS, 0x2701, tick / TICKS_PER_DAY],
    );
    // One day in forty at twenty in store, more often as it grows.
    if !r.gen_bool((food as f64 / 800.0).min(0.15)) {
        return;
    }
    let winter = w.calendar().season == crate::time::Season::Winter;
    let Some(beast) = w
        .agents
        .iter()
        .filter(|a| {
            a.alive
                && (a.species == Species::Fox
                    || a.species == Species::Rats
                    || (a.species == Species::Bear && !winter))
                && a.pos.dist(home) <= 60
        })
        .min_by_key(|a| (a.pos.dist(home), a.id))
        .map(|a| a.id)
    else {
        return;
    };
    let target = stores
        .iter()
        .max_by_key(|s| (s.1, s.0))
        .map(|s| s.0)
        .unwrap();
    w.creatures
        .drawn
        .push((beast, target, tick + 2 * TICKS_PER_DAY));
    w.events.push(
        tick,
        Some(target),
        Severity::Notable,
        EventKind::Drawn { animal: beast },
    );
}
