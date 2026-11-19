//! Wildlife: deer, boar, hares and (in stony uplands) goats graze and wander in small ranges, flee people who come
//! close, breed slowly, and die of old age; wolves roam and take hares. Animals use the same
//! movement and need machinery as people but a much simpler decision rule.

use rand::Rng;

use crate::agents::Species;
use crate::events::{EventKind, Severity};
use crate::ids::TilePos;
use crate::mind::Activity;
use crate::rng::{purpose, stream};
use crate::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use crate::world::gen::biome::Biome;
use crate::world_state::World;

pub const STRIPE: u64 = 40;

pub fn step(w: &mut World) {
    let tick = w.meta.tick;
    let seed = w.meta.seed;
    let stripe = (tick % STRIPE) as usize;
    crate::systems::combat::shifted(w);
    let n = w.agents.len();
    for i in 0..n {
        if i % STRIPE as usize != stripe {
            continue;
        }
        let a = &w.agents[i];
        if !a.alive || a.species == Species::Person || a.denned {
            continue;
        }
        if a.is_moving() || !a.path.is_empty() {
            continue;
        }
        let me = a.pos;
        let species = a.species;
        let standing = matches!(a.activity, Activity::Idle) && a.activity_ticks > STANDING;
        let mut r = stream(seed, &[purpose::ANIMALS, a.id.0 as u64, tick]);
        // A hungry grazer eats from what it stands by (#422).
        graze_bite(w, i);
        // A body that can engulf takes a small creature beside it into itself (#423).
        if crate::plan::can(&w.agents[i], crate::plan::Ability::Engulf) && engulf(w, i) {
            continue;
        }
        // The burrower moves only by digging (underground.rs).
        if species == Species::Burrower {
            continue;
        }
        // Whatever its kind's rule chose or failed to choose, an animal left standing half a
        // day takes a short walk (#397, QA 2026-10-26: the White Hart's tree or its step
        // towards the middle can lie off the ground it can walk to, a bat outside its
        // cavern had no rule at all, and either stood for days).
        if standing && stretch(w, i, &mut r) {
            continue;
        }
        // Kept by a community (#285): they graze near home and do not run from people.
        if let Some(cid) = w.agents[i].kept {
            crate::livestock::graze(w, i, cid, &mut r);
            continue;
        }
        if matches!(
            species,
            Species::Wolf | Species::Bear | Species::Crawler | Species::BlindCaveBear
        ) && predator(w, i, &mut r)
        {
            continue;
        }
        if species == Species::Bat {
            bat(w, i, &mut r);
            continue;
        }
        // #423's beasts, each as such a creature would: feed, keep from fire, rest where it
        // lives.
        if species == Species::Slime {
            slime(w, i, &mut r);
            continue;
        }
        if species == Species::Centipede {
            centipede(w, i, &mut r);
            continue;
        }
        if species == Species::Rats && nest(w, i) {
            continue;
        }
        if species == Species::Crawler {
            // Between hunts it keeps to the dark of its cavern, away from any light.
            let home = w.grid.caverns.iter().find(|c| c.centre.z == me.z).cloned();
            let to = match home {
                Some(c) => TilePos::new(
                    c.centre.x + r.gen_range(-c.rx..=c.rx),
                    c.centre.y + r.gen_range(-c.ry..=c.ry),
                    c.centre.z,
                ),
                None => me.offset(r.gen_range(-5..=5), r.gen_range(-5..=5), 0),
            };
            if w.grid.walkable(to)
                && crate::underground::firelight_at(w, to) <= 0.1
                && w.path_agent_to(i, to)
            {
                w.agents[i].set_activity(Activity::Wandering { to });
            }
            continue;
        }
        if species == Species::BlindCaveBear {
            // Between maulings it keeps to the dark of its cavern, away from any light, as
            // the crawler does (#336).
            let home = w.grid.caverns.iter().find(|c| c.centre.z == me.z).cloned();
            let to = match home {
                Some(c) => TilePos::new(
                    c.centre.x + r.gen_range(-c.rx..=c.rx),
                    c.centre.y + r.gen_range(-c.ry..=c.ry),
                    c.centre.z,
                ),
                None => me.offset(r.gen_range(-5..=5), r.gen_range(-5..=5), 0),
            };
            if w.grid.walkable(to)
                && crate::underground::firelight_at(w, to) <= 0.1
                && w.path_agent_to(i, to)
            {
                w.agents[i].set_activity(Activity::Wandering { to });
            }
            continue;
        }
        if species == Species::CaveCroc {
            // An ambusher does not rove or flee: it waits at the water's edge. Only the
            // standing rule (#397) moves it, and the seize above strikes for it (#336).
            croc(w, i, &mut r);
            continue;
        }
        // Flee: a person within 6 tiles sends grazers running the other way; the White Hart
        // goes at fourteen (creatures.rs). They run from the nearest, not whoever is first in
        // the list: one further off on the far side must not send them towards a closer one.
        let wary = if species == Species::Hart { 14 } else { 6 };
        let threat = w
            .agents
            .iter()
            .filter(|o| {
                o.alive && o.species == Species::Person && o.pos.z == me.z && me.dist(o.pos) <= wary
            })
            .min_by_key(|o| me.dist(o.pos))
            .map(|o| o.pos);
        // Crows come down where something has died.
        let carrion = if species == Species::Crow {
            crate::creatures::carrion_near(w, me, 40)
        } else {
            None
        };
        let leader = if species == Species::Wolf {
            crate::creatures::pack_leader(w)
        } else {
            None
        };
        let goal = if let Some(c) = carrion {
            Some(TilePos::new(
                (c.x + r.gen_range(-1..=1)).clamp(1, w.grid.width - 2),
                (c.y + r.gen_range(-1..=1)).clamp(1, w.grid.height - 2),
                c.z,
            ))
        } else if let (Some(t), true) = (threat, species != Species::Wolf) {
            // Away over the ground as it lies there, which on a slope is not the level it
            // stands on now.
            let dx = (me.x - t.x).signum() * r.gen_range(8..14);
            let dy = (me.y - t.y).signum() * r.gen_range(8..14);
            Some(w.grid.surface(
                (me.x + dx).clamp(1, w.grid.width - 2),
                (me.y + dy).clamp(1, w.grid.height - 2),
            ))
        } else if species == Species::Hart {
            // The Hart keeps to the trees, well away from any settlement, and off the edges.
            let homes: Vec<TilePos> = w
                .communities
                .iter()
                .filter(|c| !c.dissolved)
                .map(|c| c.home)
                .collect();
            let edge = me.x < 8 || me.y < 8 || me.x > w.grid.width - 9 || me.y > w.grid.height - 9;
            let tree = w
                .plants
                .iter()
                .filter(|p| {
                    p.alive
                        && p.kind.is_green_tree()
                        && p.pos.z == me.z
                        && p.pos.dist(me) <= 20
                        && p.pos.x > 8
                        && p.pos.y > 8
                        && p.pos.x < w.grid.width - 9
                        && p.pos.y < w.grid.height - 9
                        && homes.iter().all(|h| h.dist(p.pos) > 25)
                })
                .nth(r.gen_range(0..8))
                .map(|p| p.pos);
            tree.or_else(|| {
                let (cx, cy) = (w.grid.width / 2, w.grid.height / 2);
                let step = if edge { 10 } else { 4 };
                // On the ground as it lies there (#397: at the level it stood on, which on a
                // slope down from the edge is air, so it stood at the edge for days).
                Some(
                    w.grid.surface(
                        (me.x + (cx - me.x).signum() * step + r.gen_range(-4..=4))
                            .clamp(1, w.grid.width - 2),
                        (me.y + (cy - me.y).signum() * step + r.gen_range(-4..=4))
                            .clamp(1, w.grid.height - 2),
                    ),
                )
            })
        } else if species == Species::Wolf && leader.is_some_and(|l| l != i) {
            // The pack keeps together behind its leader.
            let lp = w.agents[leader.unwrap()].pos;
            Some(TilePos::new(
                (lp.x + r.gen_range(-2..=2)).clamp(1, w.grid.width - 2),
                (lp.y + r.gen_range(-2..=2)).clamp(1, w.grid.height - 2),
                lp.z,
            ))
        } else if species == Species::Wolf {
            // The leader takes the pack after a deer now and then, or a hare, else roams.
            let deer = if r.gen_bool(0.3) {
                w.agents
                    .iter()
                    .filter(|o| {
                        o.alive
                            && o.species == Species::Deer
                            && o.pos.z == me.z
                            && me.dist(o.pos) <= 24
                    })
                    .min_by_key(|o| me.dist(o.pos))
                    .map(|o| o.pos)
            } else {
                None
            };
            let prey = deer.or_else(|| {
                w.agents
                    .iter()
                    .filter(|o| {
                        o.alive
                            && o.species == Species::Hare
                            && o.pos.z == me.z
                            && me.dist(o.pos) <= 20
                    })
                    .min_by_key(|o| me.dist(o.pos))
                    .map(|o| o.pos)
            });
            prey.or_else(|| {
                Some(TilePos::new(
                    (me.x + r.gen_range(-15..=15)).clamp(1, w.grid.width - 2),
                    (me.y + r.gen_range(-15..=15)).clamp(1, w.grid.height - 2),
                    me.z,
                ))
            })
        } else if let Some(h) = (species == Species::Fox)
            .then(|| fox_quarry(w, i))
            .flatten()
        {
            // A fox a day from its last kill goes after the nearest hare it can see (#486).
            Some(h)
        } else if graze_now(&mut r, species, w.agents[i].needs.hunger) {
            // Graze: head for a nearby bush or shrub with something left on it. A grazer heads
            // for what it eats (`browsed`, what `graze_bite` takes from): sent to the nearest of
            // any grazed kind, a berry bush or reeds it does not bite, it went hungry beside
            // them (#486, FIXLOG 2026-11-02).
            let grazer = crate::size::grazes(species);
            w.plants
                .iter()
                .filter(|p| {
                    p.alive
                        && if grazer {
                            browsed(p.kind)
                        } else {
                            p.kind.grazed()
                        }
                        && p.growth > GRAZED_DOWN + 0.1
                        && p.pos.z == me.z
                        && me.dist(p.pos) <= 12
                })
                .min_by_key(|p| me.dist(p.pos) + (p.variant % 3) as i32)
                .map(|p| p.pos)
                .or_else(|| {
                    // Nothing left near: it goes further afield to look, rather than stand
                    // hungry where there is nothing (#422, FIXLOG 2026-10-27 part 5: hungry
                    // grazers stood still on bare ground and fewer came within the hunters'
                    // reach).
                    // On the ground as it lies there, and a few tries for ground it can stand on.
                    let below = crate::underground::below(w, me);
                    (0..6).find_map(|_| {
                        let (x, y) = (
                            (me.x + r.gen_range(-12..=12)).clamp(1, w.grid.width - 2),
                            (me.y + r.gen_range(-12..=12)).clamp(1, w.grid.height - 2),
                        );
                        let to = if below {
                            TilePos::new(x, y, me.z)
                        } else {
                            w.grid.surface(x, y)
                        };
                        w.grid.walkable(to).then_some(to)
                    })
                })
        } else {
            Some(TilePos::new(
                (me.x + r.gen_range(-6..=6)).clamp(1, w.grid.width - 2),
                (me.y + r.gen_range(-6..=6)).clamp(1, w.grid.height - 2),
                me.z,
            ))
        };
        if let Some(g) = goal {
            if w.grid.walkable(g) && w.path_agent_to(i, g) {
                // A new walk is a new activity (issue 17: the counter never reset for
                // animals, so every animal read as stuck in diagnostics).
                w.agents[i].set_activity(Activity::Wandering { to: g });
            } else if w.agents[i].body.form.is_some() {
                // No way on foot: a body that can climb or squeeze gets past what is in the
                // way, or one heavy enough breaks it (#423).
                get_past(w, i, g);
            }
        }
        // Wolves that reach a deer together bring it down (two or more of the pack at it).
        if species == Species::Wolf {
            if let Some(j) = w
                .agents
                .iter()
                .position(|o| o.alive && o.species == Species::Deer && o.pos.dist(me) <= 1)
            {
                let dp = w.agents[j].pos;
                let pack = brings_down(w, j);
                let fed = w.agents[i]
                    .work_cooldowns
                    .get(&FED_KEY)
                    .is_some_and(|t| *t > tick);
                if pack && !fed && r.gen_bool(0.5) {
                    // The pack eats and rests two days before it hunts deer again.
                    let wolves: Vec<usize> = (0..w.agents.len())
                        .filter(|&k| w.agents[k].alive && w.agents[k].species == Species::Wolf)
                        .collect();
                    for k in wolves {
                        w.agents[k]
                            .work_cooldowns
                            .insert(FED_KEY, tick + 2 * TICKS_PER_DAY);
                        crate::ecology::fed(w, k);
                    }
                    let id = w.agents[j].id;
                    w.agents[j].alive = false;
                    w.agents[j].death = Some("brought down by wolves".into());
                    w.events.push(
                        tick,
                        Some(dp),
                        Severity::Minor,
                        EventKind::Died {
                            agent: id,
                            cause: "brought down by wolves".into(),
                        },
                    );
                }
            }
        }
        // Wolves and foxes that reach a hare take it, and eat (ecology.rs).
        if matches!(species, Species::Wolf | Species::Fox)
            && crate::ecology::hungry_hunter(&w.agents[i], tick)
        {
            if let Some(j) = w.agents.iter().position(|o| {
                o.alive && o.species == Species::Hare && o.kept.is_none() && o.pos.dist(me) <= 1
            }) {
                let id = w.agents[j].id;
                let cause = if species == Species::Wolf {
                    "taken by a wolf"
                } else {
                    "taken by a fox"
                };
                w.agents[j].alive = false;
                w.agents[j].death = Some(cause.into());
                crate::ecology::fed(w, i);
                w.events.push(
                    tick,
                    Some(me),
                    Severity::Trace,
                    EventKind::Died {
                        agent: id,
                        cause: cause.into(),
                    },
                );
            }
        }
    }
    // Daily: breeding and aging, kept to a modest population.
    if tick % TICKS_PER_DAY == TICKS_PER_HOUR * 4 {
        breed_and_age(w);
        name_giants(w);
    }
}

/// The hare a fox goes after: the nearest within sixteen tiles, once it is hungry again
/// (ecology.rs).
fn fox_quarry(w: &World, i: usize) -> Option<TilePos> {
    let me = w.agents[i].pos;
    if !crate::ecology::hungry_hunter(&w.agents[i], w.meta.tick) {
        return None;
    }
    w.agents
        .iter()
        .filter(|o| {
            o.alive
                && o.species == Species::Hare
                && o.kept.is_none()
                && o.pos.z == me.z
                && me.dist(o.pos) <= 16
        })
        .min_by_key(|o| (me.dist(o.pos), o.id))
        .map(|o| o.pos)
}

/// Whether an animal heads for forage this turn: half the time, and always when a grazer is
/// hungry (#422).
fn graze_now(r: &mut rand_chacha::ChaCha8Rng, species: Species, hunger: f32) -> bool {
    let coin = r.gen_bool(0.5);
    coin || (crate::size::grazes(species) && hunger < HUNGRY)
}

/// A grazer this empty goes looking for forage rather than wandering (#422).
const HUNGRY: f32 = 0.6;
/// The growth of a plant a full belly takes, whatever the size of the belly: a big body's
/// hunger empties faster (systems/needs.rs), so it eats more by eating more often (#422).
pub const BELLY: f32 = 0.3;
/// Grazing leaves a plant at least this grown; it grows back from there.
pub const GRAZED_DOWN: f32 = 0.1;

/// The low green growth a grazer eats down (#422): not the plants people forage or cut (berry
/// bushes, herbs, reeds), so the valley's food and thatch are as they were.
pub fn browsed(k: crate::plants::PlantKind) -> bool {
    k.grazed() && !k.yields_food() && k != crate::plants::PlantKind::Reed
}

/// What a kind browses: the low green growth for the grazers and the rats, the mushrooms and
/// the pale fungus of the caverns for the cave slime (#423).
pub fn browses(s: Species, k: crate::plants::PlantKind) -> bool {
    use crate::plants::PlantKind;
    match s {
        Species::Slime => matches!(k, PlantKind::Mushroom | PlantKind::FungusTree),
        _ => browsed(k),
    }
}

/// A hungry grazer by a plant it browses eats from it (#422): as much as fills its belly, down
/// to `GRAZED_DOWN`. How much it eats in a day follows its size through its hunger.
pub fn graze_bite(w: &mut World, i: usize) {
    let a = &w.agents[i];
    if !crate::size::grazes(a.species) || a.needs.hunger >= 0.9 {
        return;
    }
    let me = a.pos;
    let Some(k) = (0..w.plants.len())
        .filter(|&k| {
            let p = &w.plants[k];
            p.alive
                && browses(a.species, p.kind)
                && p.pos.z == me.z
                && p.pos.dist(me) <= 1
                && p.growth > GRAZED_DOWN
        })
        .min_by_key(|&k| (w.plants[k].pos.dist(me), k))
    else {
        return;
    };
    let need = 1.0 - w.agents[i].needs.hunger;
    let take = (need * BELLY).min(w.plants[k].growth - GRAZED_DOWN);
    w.plants[k].growth -= take;
    w.agents[i].needs.hunger = (w.agents[i].needs.hunger + take / BELLY).min(1.0);
    // Its fill, for the food web (ecology.rs): fed grazers bear young, the unfed starve.
    if w.agents[i].needs.hunger >= 0.5 {
        crate::ecology::fed(w, i);
    }
}

/// work_cooldowns key: this beast broke into a house lately, and goes in after whoever is
/// inside (#422).
const BROKE_KEY: u32 = 0x6000_0002;

fn broke_in_lately(w: &World, i: usize) -> bool {
    w.agents[i]
        .work_cooldowns
        .get(&BROKE_KEY)
        .is_some_and(|t| *t > w.meta.tick)
}

/// Whether the beast `i` can break the built door, fence or wall at `q` (#422): wooden, and
/// the beast's force (size.rs `force`) enough against the wood's hardness. Stone it cannot.
pub fn can_break(w: &World, i: usize, q: TilePos) -> bool {
    let t = w.grid.get(q);
    if t.shape != crate::world::Shape::Wall || w.agents[i].species == Species::Person {
        return false;
    }
    let Some(m) = w.made_of(q) else { return false };
    let def = w.grid.materials.get(m);
    let wooden =
        def.kind == crate::world::material::MaterialKind::Wood || m == crate::world::Material::WOOD;
    wooden && crate::size::breaks(w.agents[i].body_kg(w.meta.tick), def.hardness)
}

/// The beast `i` breaks the door, fence or wall at `q` if it can (#422): it comes down as a
/// host's battering brings a wall down (siege.rs `knock_down`), the nearest community mends it,
/// and it is on record. Returns true if it came down.
pub fn try_break(w: &mut World, i: usize, q: TilePos) -> bool {
    if !can_break(w, i, q) {
        return false;
    }
    let door = w.grid.get(q).flags.has(crate::world::TileFlags::DOOR);
    let cid = w
        .communities
        .iter()
        .filter(|c| !c.dissolved)
        .min_by_key(|c| (c.home.dist(q), c.id))
        .map(|c| c.id);
    let of = crate::siege::knock_down(w, q, cid);
    let tick = w.meta.tick;
    w.agents[i]
        .work_cooldowns
        .insert(BROKE_KEY, tick + TICKS_PER_DAY);
    let beast = w.agents[i].id;
    w.events.push(
        tick,
        Some(q),
        Severity::Major,
        EventKind::BrokeIn { beast, of, door },
    );
    true
}

/// A beast that wants someone indoors (#422): the door nearest it among those within six tiles
/// of someone under a roof, if it can break that door. Beside it, it breaks it; else it goes
/// to it. Returns true if it took its turn.
pub fn break_in(w: &mut World, i: usize) -> bool {
    let me = w.agents[i].pos;
    let inside: Vec<TilePos> = w
        .agents
        .iter()
        .filter(|o| {
            o.alive
                && o.species == Species::Person
                && o.pos.z == me.z
                && o.pos.dist(me) <= 60
                && crate::systems::hazards::indoors(w, o.pos)
        })
        .map(|o| o.pos)
        .collect();
    let mut doors: Vec<TilePos> = Vec::new();
    for p in &inside {
        for dy in -6..=6 {
            for dx in -6..=6 {
                let q = p.offset(dx, dy, 0);
                if w.grid.in_bounds(q)
                    && w.grid.get(q).flags.has(crate::world::TileFlags::DOOR)
                    && !doors.contains(&q)
                {
                    doors.push(q);
                }
            }
        }
    }
    let Some(q) = doors
        .into_iter()
        .filter(|q| can_break(w, i, *q))
        .min_by_key(|q| (q.dist(me), *q))
    else {
        return false;
    };
    if q.dist(me) <= 1 {
        return try_break(w, i, q);
    }
    for (dx, dy) in [(0, 1), (1, 0), (0, -1), (-1, 0)] {
        let to = q.offset(dx, dy, 0);
        if w.grid.walkable(to) && w.path_agent_to(i, to) {
            w.agents[i].set_activity(Activity::Wandering { to });
            return true;
        }
    }
    false
}

/// What a body that can engulf takes into itself (#423): a creature of at most this share of
/// its own mass, and never a person.
pub const ENGULF: f32 = 0.3;

/// The beast `i` takes a small creature beside it into itself (#423, the engulf ability): one
/// not a person, not kept, of at most `ENGULF` of its mass. The creature dies of it; the beast is
/// fed, and a soft body grows by the mass it took. Hungry, with none beside it, it goes for the
/// nearest within eight tiles. Returns true if it took its turn.
pub fn engulf(w: &mut World, i: usize) -> bool {
    let tick = w.meta.tick;
    let me = w.agents[i].pos;
    let own = w.agents[i].body_kg(tick);
    let my_id = w.agents[i].id;
    let my_species = w.agents[i].species;
    let small = |o: &crate::agents::Agent| {
        o.alive
            && o.id != my_id
            && o.species != Species::Person
            && o.species != my_species
            && o.kept.is_none()
            && !o.denned
            && o.pos.z == me.z
            && o.body_kg(tick) <= ENGULF * own
    };
    if let Some(j) = (0..w.agents.len())
        .filter(|&j| small(&w.agents[j]) && w.agents[j].pos.dist(me) <= 1)
        .min_by_key(|&j| w.agents[j].id)
    {
        let (prey, kg) = (w.agents[j].id, w.agents[j].body_kg(tick));
        let name = w.agents[i].name.clone();
        crate::systems::life::die(w, prey, format!("engulfed by {name}"));
        let a = &mut w.agents[i];
        a.needs.hunger = 1.0;
        if let Some(f) = a.body.form.as_mut() {
            if f.kind == crate::plan::Kind::Soft {
                let grow = (kg / own.max(0.001) * f.whole as f32).round() as u16;
                f.units = (f.units + grow).min(f.whole);
            }
        }
        a.path.clear();
        a.set_activity(Activity::Idle);
        return true;
    }
    if w.agents[i].needs.hunger >= 0.5 {
        return false;
    }
    let Some(to) = w
        .agents
        .iter()
        .filter(|o| small(o) && o.pos.dist(me) <= 8)
        .min_by_key(|o| (o.pos.dist(me), o.id))
        .map(|o| o.pos)
    else {
        return false;
    };
    if w.path_agent_to(i, to) {
        w.agents[i].set_activity(Activity::Wandering { to });
        return true;
    }
    false
}

/// The beast `i`, whose path to `goal` failed, gets past what stops a walker by what its body
/// can do (#423): through the gaps of a built wall of wood (squeeze), or up or down a level at a
/// rock face (climb), one step that brings it nearer `goal`. Failing those, a beast heavy enough
/// breaks the wooden wall in its way (#422 `try_break`, siege.rs `knock_down`). Returns true if
/// it moved or broke through.
pub fn get_past(w: &mut World, i: usize, goal: TilePos) -> bool {
    use crate::plan::Ability;
    let me = w.agents[i].pos;
    let squeeze = crate::plan::can(&w.agents[i], Ability::Squeeze);
    let climb = crate::plan::can(&w.agents[i], Ability::Climb);
    let far =
        |p: TilePos| (p.x - goal.x).abs().max((p.y - goal.y).abs()) + 2 * (p.z - goal.z).abs();
    let mut best: Option<(i32, TilePos, &'static str, TilePos)> = None;
    let mut consider = |to: TilePos, how: &'static str, at: TilePos| {
        let d = far(to);
        if d < far(me) && best.is_none_or(|b| (d, to) < (b.0, b.1)) {
            best = Some((d, to, how, at));
        }
    };
    let mut wall = None;
    for (dx, dy) in [(0, -1), (1, 0), (0, 1), (-1, 0)] {
        let side = me.offset(dx, dy, 0);
        if !w.grid.in_bounds(side) {
            continue;
        }
        let t = w.grid.get(side);
        // A built wall, fence or palisade of wood; a natural face is rock nobody built.
        let built_wood =
            t.shape == crate::world::Shape::Wall && t.material == crate::world::Material::WOOD;
        let natural = !matches!(
            t.material,
            crate::world::Material::WOOD | crate::world::Material::STONE_BLOCK
        );
        if built_wood {
            let beyond = me.offset(2 * dx, 2 * dy, 0);
            if squeeze && w.grid.walkable(beyond) {
                consider(beyond, "squeezed through", side);
            }
            if far(side) < far(me) && wall.is_none() {
                wall = Some(side);
            }
        }
        if climb {
            let up = me.offset(dx, dy, 1);
            if t.is_solid() && natural && w.grid.walkable(up) {
                consider(up, "climbed up", side);
            }
            let down = me.offset(dx, dy, -1);
            if !t.is_solid() && !w.grid.walkable(side) && w.grid.walkable(down) {
                consider(down, "climbed down", side);
            }
        }
    }
    if let Some((_, to, how, at)) = best {
        let a = &mut w.agents[i];
        a.from = me;
        a.pos = to;
        a.path.clear();
        a.set_activity(Activity::Wandering { to });
        let (agent, tick) = (a.id, w.meta.tick);
        // Through a wall into the built place is news; up and down the rock is not.
        let severity = if how == "squeezed through" {
            Severity::Notable
        } else {
            Severity::Trace
        };
        w.events.push(
            tick,
            Some(at),
            severity,
            EventKind::Crept {
                agent,
                at,
                how: how.to_string(),
            },
        );
        return true;
    }
    match wall {
        Some(q) => try_break(w, i, q),
        None => false,
    }
}

/// Whether the wolves at the deer `j` are enough to drag it down (#422): their might together
/// at least `DRAG` times the deer's. Two grown wolves bring down a grown deer; a giant stag
/// takes three, and two half-grown wolves cannot.
pub fn brings_down(w: &World, j: usize) -> bool {
    let tick = w.meta.tick;
    let dp = w.agents[j].pos;
    let pack: f32 = w
        .agents
        .iter()
        .filter(|o| o.alive && o.species == Species::Wolf && o.pos.dist(dp) <= 2)
        .map(|o| crate::size::might(o.body_kg(tick)))
        .sum();
    pack >= DRAG * crate::size::might(w.agents[j].body_kg(tick))
}

/// How much more might a pack needs than its prey to drag it down.
pub const DRAG: f32 = 1.2;

/// Half a day standing idle (#397).
const STANDING: u32 = 12 * TICKS_PER_HOUR as u32;

/// A short walk to open ground within six tiles, at its own level below ground and on the
/// surface above it. Returns true if it set off.
fn stretch(w: &mut World, i: usize, r: &mut rand_chacha::ChaCha8Rng) -> bool {
    let me = w.agents[i].pos;
    let below = crate::underground::below(w, me);
    for _ in 0..8 {
        let (x, y) = (
            (me.x + r.gen_range(-6..=6)).clamp(1, w.grid.width - 2),
            (me.y + r.gen_range(-6..=6)).clamp(1, w.grid.height - 2),
        );
        let to = if below {
            TilePos::new(x, y, me.z)
        } else {
            w.grid.surface(x, y)
        };
        if to != me && w.grid.walkable(to) && w.path_agent_to(i, to) {
            w.agents[i].set_activity(Activity::Wandering { to });
            return true;
        }
    }
    false
}

/// work_cooldowns key: this animal has gone for someone lately.
const STRUCK_KEY: u32 = 0x6000_0000;
/// work_cooldowns key: the pack has brought down a deer lately.
const FED_KEY: u32 = 0x6000_0001;

/// Wolves and bears and people (issue #47). A wolf in a hungry winter (no hares within
/// reach) may stalk someone who is alone and go for them; a bear come upon close may
/// charge. Company and a willingness to fight drive them off. A beast that has hurt someone
/// gets a name. Returns true if it decided this animal's turn.
fn predator(w: &mut World, i: usize, r: &mut rand_chacha::ChaCha8Rng) -> bool {
    let tick = w.meta.tick;
    let me = w.agents[i].pos;
    let species = w.agents[i].species;
    if w.agents[i]
        .work_cooldowns
        .get(&STRUCK_KEY)
        .map(|t| *t > tick)
        .unwrap_or(false)
    {
        return false;
    }
    let humans_near = |p: TilePos, d: i32| {
        w.agents
            .iter()
            .filter(|o| {
                o.alive && o.species == Species::Person && o.pos.z == p.z && o.pos.dist(p) <= d
            })
            .count()
    };
    let target = match species {
        Species::Wolf => {
            let winter = w.calendar().season == crate::time::Season::Winter;
            let hares = w.agents.iter().any(|o| {
                o.alive && o.species == Species::Hare && o.pos.z == me.z && o.pos.dist(me) <= 30
            });
            // Only a hungry winter turns a wolf on people, unless the deity has made them bold:
            // then any winter does, or any season with no hares near.
            let bold = w.lean(crate::deity::Lever::WolfHunger, crate::deity::Scope::Valley) > 1.0;
            let hungry = if bold {
                winter || !hares
            } else {
                winter && !hares
            };
            // They hunt from dusk to dawn.
            let hour = (tick % TICKS_PER_DAY) / TICKS_PER_HOUR;
            if !hungry || (6..18).contains(&hour) || !r.gen_bool(0.3) {
                return false;
            }
            let home = w.communities.iter().find(|c| !c.dissolved).map(|c| c.home);
            w.agents
                .iter()
                .enumerate()
                .filter(|(_, o)| {
                    o.alive
                        && o.species == Species::Person
                        && o.pos.z == me.z
                        && o.pos.dist(me) <= 16
                        && humans_near(o.pos, 6) == 1
                        // A wolf does not go in under a roof after anyone, nor in among the
                        // houses unless the deity has made it bold.
                        && !crate::systems::hazards::indoors(w, o.pos)
                        && (bold || home.map(|h| h.dist(o.pos) >= 12).unwrap_or(true))
                })
                .min_by_key(|(_, o)| o.pos.dist(me))
                .map(|(j, _)| j)
        }
        Species::Crawler if w.creatures.risen.contains(&w.agents[i].id) => {
            // C75: the beast come up through the breach hunts the valley from dusk to dawn,
            // going for whoever is out alone; it does not go in under a roof unless it has
            // broken its way in (#422).
            let hour = (tick % TICKS_PER_DAY) / TICKS_PER_HOUR;
            if (6..19).contains(&hour) || !r.gen_bool(0.5) {
                return false;
            }
            let inside = broke_in_lately(w, i);
            let out = w
                .agents
                .iter()
                .enumerate()
                .filter(|(_, o)| {
                    o.alive
                        && o.species == Species::Person
                        && o.pos.dist(me) <= 60
                        && humans_near(o.pos, 4) <= 2
                        && (inside || !crate::systems::hazards::indoors(w, o.pos))
                })
                .min_by_key(|(_, o)| (o.pos.dist(me), o.id))
                .map(|(j, _)| j);
            // Nobody out: a beast heavy enough breaks in where the people are (#422).
            if out.is_none() {
                return break_in(w, i);
            }
            out
        }
        Species::Crawler => {
            // The pale crawler is blind and hunts by sound, at any hour, but it shuns fire: it
            // will not go for anyone a lamp, hearth or flame reaches, and it goes for someone
            // alone.
            if !r.gen_bool(0.4) {
                return false;
            }
            w.agents
                .iter()
                .enumerate()
                .filter(|(_, o)| {
                    o.alive
                        && o.species == Species::Person
                        && o.pos.z == me.z
                        && o.pos.dist(me) <= 12
                        && crate::underground::below(w, o.pos)
                        && crate::underground::firelight_at(w, o.pos) < 0.1
                        && humans_near(o.pos, 4) == 1
                })
                .min_by_key(|(_, o)| o.pos.dist(me))
                .map(|(j, _)| j)
        }
        Species::BlindCaveBear => {
            // The blind bear of the deep is territorial, not a hunter: it goes for whoever
            // comes within reach of where it lies up, and only where no light reaches (#336).
            if !r.gen_bool(0.25) {
                return false;
            }
            w.agents
                .iter()
                .enumerate()
                .filter(|(_, o)| {
                    o.alive
                        && o.species == Species::Person
                        && o.pos.z == me.z
                        && o.pos.dist(me) <= 3
                        && crate::underground::below(w, o.pos)
                        && crate::underground::firelight_at(w, o.pos) < 0.1
                })
                .min_by_key(|(_, o)| o.pos.dist(me))
                .map(|(j, _)| j)
        }
        _ => {
            // Bears: only someone who has come right up on them, and not always.
            let close = w
                .agents
                .iter()
                .enumerate()
                .filter(|(_, o)| {
                    o.alive
                        && o.species == Species::Person
                        && o.pos.z == me.z
                        && o.pos.dist(me) <= 2
                })
                .min_by_key(|(_, o)| o.pos.dist(me))
                .map(|(j, _)| j);
            // (A bear dens up for the winter.)
            let winter = w.calendar().season == crate::time::Season::Winter;
            if close.is_none() || winter || !r.gen_bool(0.02) {
                return false;
            }
            close
        }
    };
    let Some(j) = target else { return false };
    let at = w.agents[j].pos;
    if at.dist(me) > 1 {
        if w.grid.walkable(at) && w.path_agent_to(i, at) {
            w.agents[i].set_activity(Activity::Wandering { to: at });
            return true;
        }
        return false;
    }
    strike(w, i, j, r);
    true
}

pub fn strike(w: &mut World, i: usize, j: usize, r: &mut rand_chacha::ChaCha8Rng) {
    use crate::body::{Part, WoundKind};
    use crate::mind::beliefs::Topic;
    let tick = w.meta.tick;
    let me = w.agents[i].pos;
    let species = w.agents[i].species;
    let (animal, victim) = (w.agents[i].id, w.agents[j].id);
    let company = w
        .agents
        .iter()
        .filter(|o| {
            o.alive
                && o.species == Species::Person
                && o.id != victim
                && o.pos.z == me.z
                && o.pos.dist(me) <= 6
        })
        .count();
    // Someone with a spear in hand stands their ground more often (C33).
    let armed = crate::systems::arms::edge(w, j).is_some();
    let stands = !w.agents[j].activity.is_asleep()
        && w.agents[j].beliefs.lean(Topic::Violence) > -0.3
        && r.gen_bool(if armed { 0.8 } else { 0.55 });
    let driven_off = company > 0 || stands;
    let (cause, sev) = match species {
        Species::Bear => ("mauled by a bear", r.gen_range(0.4..0.8)),
        Species::Crawler if w.creatures.risen.contains(&animal) => {
            ("torn by the beast from below", r.gen_range(0.4..0.75))
        }
        Species::Crawler => ("bitten by the pale crawler", r.gen_range(0.3..0.6)),
        Species::BlindCaveBear => (
            "mauled by the blind bear of the deep",
            r.gen_range(0.4..0.8),
        ),
        Species::CaveCroc => ("seized by the cave crocodile", r.gen_range(0.3..0.6)),
        _ => ("bitten by a wolf", r.gen_range(0.25..0.55)),
    };
    // The kind's bite at its mean size; a big one of its kind bites harder, a young one less
    // (#422, size.rs `might`).
    let sev = sev * crate::size::might(w.agents[i].body_kg(tick))
        / crate::size::might(crate::size::mean_kg(species));
    let parts = [
        Part::LEG_L,
        Part::LEG_R,
        Part::ARM_L,
        Part::ARM_R,
        Part::TORSO,
    ];
    // C75 (judged before the bite: its pain took every lone victim's nerve in the soaks):
    // the struck and those near with the nerve for it fight back; a beast hurt badly
    // enough is felled, and the valley's standing together is on record.
    let (stood, felled) = crate::systems::combat::stand_against(w, i, j, r);
    if !stood.is_empty() {
        w.events.push(
            tick,
            Some(me),
            Severity::Major,
            EventKind::StoodAgainst {
                beast: animal,
                by: stood.clone(),
                felled,
            },
        );
    }
    if felled {
        return;
    }
    let driven_off = driven_off || !stood.is_empty();
    let bites = if driven_off { 1 } else { 2 };
    for _ in 0..bites {
        let part = parts[r.gen_range(0..parts.len())];
        let kind = if matches!(species, Species::Bear | Species::BlindCaveBear) && r.gen_bool(0.3) {
            WoundKind::Break
        } else {
            WoundKind::Cut
        };
        // Armour blunts the teeth and wears for it.
        let guard = crate::systems::arms::guard(w, j)
            .map(|g| g.1)
            .unwrap_or(0.0);
        if guard > 0.0 {
            crate::systems::arms::took_blow(w, j);
        }
        let sev = sev * (1.0 - 0.5 * guard.min(1.0));
        crate::systems::health::wound(w, j, part, kind, sev, 0.15, cause, Some(animal));
        crate::creatures::note_wound(w, animal);
    }
    let rest = match (species, driven_off) {
        (Species::Bear | Species::BlindCaveBear, _) => 8,
        (_, true) => 4,
        _ => 3,
    } * TICKS_PER_DAY;
    // The valley's wolves are one pack: it hunts together and rests together.
    let pack: Vec<usize> = (0..w.agents.len())
        .filter(|&k| {
            k == i
                || (species == Species::Wolf
                    && w.agents[k].alive
                    && w.agents[k].species == Species::Wolf)
        })
        .collect();
    for k in pack {
        w.agents[k].work_cooldowns.insert(STRUCK_KEY, tick + rest);
    }
    w.events.push(
        tick,
        Some(me),
        Severity::Major,
        EventKind::Attacked {
            animal,
            agent: victim,
            driven_off,
        },
    );
    // Off it goes, away from them.
    let v = w.agents[j].pos;
    let to = TilePos::new(
        (me.x + (me.x - v.x).signum() * 14).clamp(1, w.grid.width - 2),
        (me.y + (me.y - v.y).signum() * 14).clamp(1, w.grid.height - 2),
        me.z,
    );
    // A beast below ground stays at its own level.
    let to = if crate::underground::below(w, me) {
        to
    } else {
        w.grid.surface(to.x, to.y)
    };
    if w.grid.walkable(to) && w.path_agent_to(i, to) {
        w.agents[i].set_activity(Activity::Wandering { to });
    }
    // A beast that has hurt someone is known by a name from then on.
    let plain = format!("{species:?}").to_lowercase();
    if w.agents[i].name == plain {
        let giant = w.agents[i].build() == crate::size::Build::Giant;
        let name = beast_name(w.meta.seed, animal.0, species, giant);
        w.agents[i].name = name.clone();
        w.agents[i].named_at = Some(tick);
        let who = w.agents[j].name.clone();
        w.events.push(
            tick,
            Some(me),
            Severity::Notable,
            EventKind::Named {
                agent: animal,
                name,
                because: format!("after it went for {who}"),
            },
        );
    }
}

/// A name of the kind people give a beast they fear: what it looks like, what it did. A giant
/// is named for its size (#422).
pub fn beast_name(seed: u64, id: u32, species: Species, giant: bool) -> String {
    let mut r = stream(seed, &[purpose::ANIMALS, 0x4E4D, id as u64]);
    let first: &[&str] = if giant {
        &["Great", "Big", "Broad", "Tall", "Huge", "Old"]
    } else {
        &[
            "Grey", "Black", "Old", "Torn", "Red", "Lame", "Scar", "Long", "White", "Hollow",
        ]
    };
    let last: &[&str] = match species {
        Species::Bear => &["paw", "back", "hide", "claw", "maw"],
        Species::BlindCaveBear => &["paw", "back", "maw", "claw"],
        Species::CaveCroc => &["maw", "tail", "scale", "jaw"],
        _ => &["jaw", "fang", "tooth", "coat", "ear", "flank"],
    };
    format!(
        "{}{}",
        first[r.gen_range(0..first.len())],
        last[r.gen_range(0..last.len())]
    )
}

/// A giant of its kind seen by people is known by a name from then on (#422), as a beast that
/// hurt someone is: legend material. Daily, for any unnamed giant with a person within ten
/// tiles.
pub fn name_giants(w: &mut World) {
    let tick = w.meta.tick;
    for i in 0..w.agents.len() {
        let a = &w.agents[i];
        let plain = format!("{:?}", a.species).to_lowercase();
        if !a.alive
            || a.species == Species::Person
            || a.name != plain
            || a.build() != crate::size::Build::Giant
        {
            continue;
        }
        let at = a.pos;
        let Some(seen_by) = w
            .agents
            .iter()
            .filter(|o| {
                o.alive && o.species == Species::Person && o.pos.z == at.z && o.pos.dist(at) <= 10
            })
            .min_by_key(|o| (o.pos.dist(at), o.id))
            .map(|o| o.name.clone())
        else {
            continue;
        };
        let (id, species) = (a.id, a.species);
        let name = beast_name(w.meta.seed, id.0, species, true);
        w.agents[i].name = name.clone();
        w.agents[i].named_at = Some(tick);
        let kind = plain;
        w.events.push(
            tick,
            Some(at),
            Severity::Notable,
            EventKind::Named {
                agent: id,
                name,
                because: format!("a {kind} bigger than any {seen_by} had seen"),
            },
        );
    }
}

/// Species, breeding cap and life years: the table `breed_and_age` reads.
const WILD_CAPS: [(Species, usize, f32); 11] = [
    (Species::Deer, 14usize, 12.0f32),
    (Species::Boar, 8, 10.0),
    (Species::Hare, 24, 4.0),
    (Species::Wolf, 4, 8.0),
    (Species::Fox, 5, 6.0),
    (Species::Crow, 8, 5.0),
    (Species::Bear, 2, 15.0),
    (Species::Duck, 12, 4.0),
    (Species::Goat, 8, 10.0),
    // #423: a swarm of rats breeds a new one now and then; centipedes are few.
    (Species::Rats, 2, 3.0),
    (Species::Centipede, 2, 6.0),
];

/// The cap on the wild kind `s` before herd leaning (`breed_and_age`); None for any other.
/// Read by the ecology census (#1665, FIXLOG 2026-11-16).
pub fn wild_cap(s: Species) -> Option<usize> {
    WILD_CAPS.iter().find(|c| c.0 == s).map(|c| c.1)
}

fn breed_and_age(w: &mut World) {
    let tick = w.meta.tick;
    let seed = w.meta.seed;
    let counts = |w: &World, s: Species| {
        w.agents
            .iter()
            .filter(|a| a.alive && a.species == s)
            .count()
    };
    let caps = WILD_CAPS;
    let mut births = Vec::new();
    let mut deaths: Vec<(crate::ids::AgentId, &str)> = Vec::new();
    let season = w.calendar().season;
    // The deity may lean the herds (deity.rs): grazers breed faster and the valley holds more.
    let herds = w.lean(crate::deity::Lever::Herds, crate::deity::Scope::Valley);
    for (species, cap, life_years) in caps {
        let grazer = matches!(
            species,
            Species::Deer | Species::Boar | Species::Hare | Species::Duck
        );
        let (cap, chance) = if grazer {
            ((cap as f32 * herds.clamp(0.5, 1.6)) as usize, 0.06 * herds)
        } else {
            (cap, 0.06)
        };
        // Hares breed fast, several litters a year: since wolves and foxes eat them (#486), at
        // the deer's rate the foxes kept them at one or two in every valley (FIXLOG 2026-11-02).
        let chance = if species == Species::Hare {
            chance * 2.5
        } else {
            chance
        };
        let n = counts(w, species);
        for a in w.agents.iter().filter(|a| a.alive && a.species == species) {
            let mut r = stream(
                seed,
                &[purpose::ANIMALS, 0xB1, a.id.0 as u64, tick / TICKS_PER_DAY],
            );
            let years = a.age_years(tick) as f32;
            if years > life_years && r.gen::<f32>() < 0.15 {
                deaths.push((a.id, "old age"));
            } else if let Some(cause) = crate::ecology::starving(a, tick, season)
                .filter(|_| crate::ecology::starves_today(seed, a, tick))
            {
                // The food web (#486): too long without its food, it dies of the want.
                deaths.push((a.id, cause));
            } else if n < cap
                && years >= 1.0
                && r.gen::<f32>() < chance
                && crate::ecology::fit_to_bear(a, tick)
            {
                births.push((species, a.pos));
            }
        }
    }
    for (id, cause) in deaths {
        if let Some(a) = w.agents.iter_mut().find(|a| a.id == id) {
            a.alive = false;
            a.death = Some(cause.into());
            // A death of want is recorded, and leaves carrion (creatures.rs); old age was not
            // recorded before and is not now.
            if cause != "old age" {
                let at = a.pos;
                w.events.push(
                    tick,
                    Some(at),
                    Severity::Trace,
                    EventKind::Died {
                        agent: id,
                        cause: cause.into(),
                    },
                );
            }
        }
    }
    for (species, pos) in births {
        let id = w.spawn_agent(species, pos);
        if let Some(a) = w.agents.iter_mut().find(|a| a.id == id) {
            a.born = tick as i64;
        }
        // A young swarm or centipede nests where it was born (#423).
        if crate::plan::beast_plan(species).is_some() {
            w.creatures.nests.push((id, pos));
        }
    }
}

/// Seed the map with wildlife at generation (called from `found_party` worlds and tests).
pub fn populate(w: &mut World) {
    let seed = w.meta.seed;
    let mut r = stream(seed, &[purpose::ANIMALS, 0x5EED]);
    let (width, height, home) = (w.grid.width, w.grid.height, w.meta.founding_site);
    let mut placed = 0;
    let wanted = [
        (Species::Deer, 8),
        (Species::Boar, 4),
        (Species::Hare, 14),
        (Species::Wolf, 2),
        (Species::Fox, 3),
        (Species::Crow, 4),
        (Species::Bear, 2),
        (Species::Duck, 6),
        (Species::Goat, 5),
    ];
    for (species, n) in wanted {
        let mut tries = 0;
        let mut k = 0;
        while k < n && tries < 400 {
            tries += 1;
            let x = r.gen_range(2..width - 2);
            let y = r.gen_range(2..height - 2);
            let p = w.grid.surface(x, y);
            if !w.grid.walkable(p) || p.dist(home) < 16 {
                continue;
            }
            // Goats live only where there is stony high ground; a soft valley has none.
            if species == Species::Goat
                && !matches!(w.grid.biome_at(x, y), Some(Biome::Stony | Biome::Fell))
            {
                continue;
            }
            let id = w.spawn_agent(species, p);
            let years = r.gen_range(1..5);
            if let Some(a) = w.agents.iter_mut().find(|a| a.id == id) {
                a.born = w.meta.tick as i64 - (crate::time::TICKS_PER_YEAR * years) as i64;
            }
            k += 1;
            placed += 1;
        }
    }
    let _ = placed;
    place_deep(w, &mut r);
    // From a stream of their own, so the valley's other animals are where they were (#423).
    let mut r = stream(seed, &[purpose::ANIMALS, 0x4230]);
    place_bodies(w, &mut r);
}

/// #423's beasts: a swarm of rats on open ground away from the landing, a cave slime in a wet
/// cavern of the deep (cave_ecology.rs), and a centipede in the shallowest cavern. Each nests
/// where it is put.
fn place_bodies(w: &mut World, r: &mut rand_chacha::ChaCha8Rng) {
    let home = w.meta.founding_site;
    for _ in 0..400 {
        let x = r.gen_range(2..w.grid.width - 2);
        let y = r.gen_range(2..w.grid.height - 2);
        let p = w.grid.surface(x, y);
        if w.grid.walkable(p) && p.dist(home) >= 16 && !w.grid.get(p).is_water() {
            let id = w.spawn_agent(Species::Rats, p);
            w.creatures.nests.push((id, p));
            break;
        }
    }
    let caverns = w.grid.caverns.clone();
    for (species, c) in [
        (Species::Slime, crate::cave_ecology::slime_cavern(&caverns)),
        (
            Species::Centipede,
            crate::cave_ecology::centipede_cavern(&caverns),
        ),
    ] {
        let Some(c) = c else { continue };
        let floors: Vec<TilePos> = (c.centre.y - c.ry..=c.centre.y + c.ry)
            .flat_map(|y| {
                (c.centre.x - c.rx..=c.centre.x + c.rx).map(move |x| TilePos::new(x, y, c.centre.z))
            })
            .filter(|p| c.contains(*p) && w.grid.walkable(*p) && !w.grid.get(*p).is_water())
            .collect();
        if floors.is_empty() {
            continue;
        }
        let p = floors[r.gen_range(0..floors.len())];
        let id = w.spawn_agent(species, p);
        w.creatures.nests.push((id, p));
    }
}

/// Where the beast `i` nests (#423): where it was put or born, else where it stands.
fn nest_of(w: &World, i: usize) -> TilePos {
    let id = w.agents[i].id;
    w.creatures
        .nests
        .iter()
        .find(|n| n.0 == id)
        .map_or(w.agents[i].pos, |n| n.1)
}

/// Walks the beast `i` to `to`, or, with no way on foot, gets past what is in the way by what
/// its body can do (#423). Returns true if it moved.
fn go(w: &mut World, i: usize, to: TilePos) -> bool {
    if to == w.agents[i].pos || !w.grid.walkable(to) {
        return false;
    }
    if w.path_agent_to(i, to) {
        w.agents[i].set_activity(Activity::Wandering { to });
        return true;
    }
    get_past(w, i, to)
}

/// A beast goes off away from `from`, as far as `n` tiles and nearer if that is rock: straight
/// away if it can, else aslant, else to the side, as long as it ends further off than it is
/// (cornered against a cavern wall it slips along it). At its own level below ground, and over
/// the ground as it lies above it. Returns true if it moved.
fn away(w: &mut World, i: usize, from: TilePos, n: i32) -> bool {
    let me = w.agents[i].pos;
    // Standing on it: any way off.
    let (sx, sy) = match ((me.x - from.x).signum(), (me.y - from.y).signum()) {
        (0, 0) => (1, 0),
        d => d,
    };
    // The eight ways round, straightest away first.
    const RING: [(i32, i32); 8] = [
        (1, 0),
        (1, 1),
        (0, 1),
        (-1, 1),
        (-1, 0),
        (-1, -1),
        (0, -1),
        (1, -1),
    ];
    let at = RING.iter().position(|d| *d == (sx, sy)).unwrap_or(0);
    let below = crate::underground::below(w, me);
    let start = me.dist(from);
    for turn in [0i32, 1, -1, 2, -2] {
        let (dx, dy) = RING[(at as i32 + turn).rem_euclid(8) as usize];
        for k in (1..=n).rev() {
            let (x, y) = (
                (me.x + dx * k).clamp(1, w.grid.width - 2),
                (me.y + dy * k).clamp(1, w.grid.height - 2),
            );
            let to = if below {
                TilePos::new(x, y, me.z)
            } else {
                w.grid.surface(x, y)
            };
            if to.dist(from) > start && w.grid.walkable(to) && go(w, i, to) {
                return true;
            }
        }
    }
    false
}

/// The fire whose light reaches `p` below ground, if any, nearest first: a lamp, a hearth, a
/// smelter or a fire (underground.rs `firelight_at`).
fn fire_near(w: &World, p: TilePos) -> Option<TilePos> {
    if crate::underground::firelight_at(w, p) <= 0.1 {
        return None;
    }
    w.lights
        .iter()
        .filter(|(q, _, fire)| *fire && q.z == p.z)
        .min_by_key(|(q, _, _)| (q.dist(p), *q))
        .map(|(q, _, _)| *q)
}

/// The nearest plant within `reach` the beast `i` browses that has growth left on it.
fn forage_near(w: &World, i: usize, reach: i32) -> Option<TilePos> {
    let (me, s) = (w.agents[i].pos, w.agents[i].species);
    w.plants
        .iter()
        .filter(|p| {
            p.alive
                && browses(s, p.kind)
                && p.growth > GRAZED_DOWN + 0.1
                && p.pos.z == me.z
                && p.pos.dist(me) <= reach
        })
        .min_by_key(|p| (me.dist(p.pos), p.pos))
        .map(|p| p.pos)
}

/// The cave slime (#423). It shrinks from firelight, creeping away from any lamp or flame that
/// reaches it; hungry, it creeps to the mushrooms and pale fungus of its cavern and feeds on
/// them (`graze_bite`), and takes any small creature it comes on (`engulf`); fed, it rests in the
/// damp by the cavern's pool, or where it was put. It is slow: it moves one turn in two.
fn slime(w: &mut World, i: usize, r: &mut rand_chacha::ChaCha8Rng) {
    let me = w.agents[i].pos;
    if let Some(fire) = fire_near(w, me) {
        away(w, i, fire, 4);
        return;
    }
    if r.gen_bool(0.5) {
        return;
    }
    if w.agents[i].needs.hunger < HUNGRY {
        if let Some(to) = forage_near(w, i, 14) {
            go(w, i, to);
        }
        return;
    }
    let water = (-6..=6)
        .flat_map(|dy| (-6..=6).map(move |dx| (dx, dy)))
        .map(|(dx, dy)| me.offset(dx, dy, 0))
        .filter(|q| w.grid.in_bounds(*q) && w.grid.get(*q).is_water())
        .min_by_key(|q| (q.dist(me), *q));
    match water {
        Some(q) if q.dist(me) > 1 => {
            let by = [(0, 1), (1, 0), (0, -1), (-1, 0)]
                .iter()
                .map(|(dx, dy)| q.offset(*dx, *dy, 0))
                .find(|p| w.grid.walkable(*p) && !w.grid.get(*p).is_water());
            if let Some(p) = by {
                go(w, i, p);
            }
        }
        Some(_) => {}
        None => {
            let home = nest_of(w, i);
            if home.dist(me) > 3 {
                go(w, i, home);
            }
        }
    }
}

/// The cave centipede (#423). It keeps from fire and from anyone who comes within three tiles;
/// hungry, it hunts the bats of its cavern and takes one it reaches; by day it lies up in a
/// crevice at the cavern's edge, by night it roams. It climbs where it cannot walk (`go`).
fn centipede(w: &mut World, i: usize, r: &mut rand_chacha::ChaCha8Rng) {
    let tick = w.meta.tick;
    let me = w.agents[i].pos;
    let person = w
        .agents
        .iter()
        .filter(|o| {
            o.alive && o.species == Species::Person && o.pos.z == me.z && o.pos.dist(me) <= 3
        })
        .min_by_key(|o| (o.pos.dist(me), o.id))
        .map(|o| o.pos);
    if let Some(p) = person {
        away(w, i, p, 6);
        return;
    }
    if let Some(fire) = fire_near(w, me) {
        away(w, i, fire, 5);
        return;
    }
    if w.agents[i].needs.hunger < 0.5 {
        let bat = (0..w.agents.len())
            .filter(|&j| {
                let o = &w.agents[j];
                o.alive && o.species == Species::Bat && o.pos.z == me.z && o.pos.dist(me) <= 12
            })
            .min_by_key(|&j| (w.agents[j].pos.dist(me), w.agents[j].id));
        if let Some(j) = bat {
            if w.agents[j].pos.dist(me) <= 1 {
                let prey = w.agents[j].id;
                crate::systems::life::die(w, prey, "taken by a centipede".into());
                w.agents[i].needs.hunger = 1.0;
                w.agents[i].set_activity(Activity::Idle);
            } else {
                let to = w.agents[j].pos;
                go(w, i, to);
            }
            return;
        }
    }
    let hour = (tick % TICKS_PER_DAY) / TICKS_PER_HOUR;
    let home = nest_of(w, i);
    if (6..18).contains(&hour) {
        if home.dist(me) > 1 {
            go(w, i, home);
        }
        return;
    }
    let to = TilePos::new(
        (home.x + r.gen_range(-8..=8)).clamp(1, w.grid.width - 2),
        (home.y + r.gen_range(-8..=8)).clamp(1, w.grid.height - 2),
        home.z,
    );
    go(w, i, to);
}

/// The rats lie up in their nest by day, as rats do, unless they are hungry (#423); by night,
/// or hungry, they feed, run from people and raid the stores as the valley's other beasts do.
/// Returns true if this was its turn.
fn nest(w: &mut World, i: usize) -> bool {
    let tick = w.meta.tick;
    let hour = (tick % TICKS_PER_DAY) / TICKS_PER_HOUR;
    let day = (6..19).contains(&hour);
    let hungry = w.agents[i].needs.hunger < HUNGRY;
    if !day || hungry || w.creatures.drawn.iter().any(|d| d.0 == w.agents[i].id) {
        return false;
    }
    let home = nest_of(w, i);
    if home.dist(w.agents[i].pos) > 1 {
        go(w, i, home);
    }
    true
}

/// The cave crocodile of the pools (#336): an ambusher that seizes whoever steps within
/// reach of the water's edge. It does not hunt; it waits, and the seize is its whole turn.
/// Returns true if it struck.
fn croc(w: &mut World, i: usize, r: &mut rand_chacha::ChaCha8Rng) -> bool {
    let me = w.agents[i].pos;
    let beside_water = |p: TilePos| {
        [(1, 0), (-1, 0), (0, 1), (0, -1)]
            .iter()
            .any(|&(dx, dy)| w.grid.get(p.offset(dx, dy, 0)).is_water())
    };
    let Some((j, _)) = w
        .agents
        .iter()
        .enumerate()
        .filter(|(_, o)| {
            o.alive
                && o.species == Species::Person
                && o.pos.z == me.z
                && o.pos.dist(me) <= 2
                && beside_water(o.pos)
        })
        .min_by_key(|(_, o)| (o.pos.dist(me), o.id))
    else {
        return false;
    };
    strike(w, i, j, r);
    true
}

/// The creatures of the deep: bats in every cavern, the pale crawler in the deepest one, and a
/// burrower somewhere in the rock of the middle stratum.
fn place_deep(w: &mut World, r: &mut rand_chacha::ChaCha8Rng) {
    let caverns = w.grid.caverns.clone();
    let floor = |w: &World,
                 c: &crate::world::gen::caverns::Cavern,
                 r: &mut rand_chacha::ChaCha8Rng| {
        // Enumerating the dry floor guarantees habitat populations when placement is possible;
        // bounded random sampling silently lost them (FIXLOG 2026-10-30, PR #495 review).
        let floors: Vec<TilePos> = (c.centre.y - c.ry..=c.centre.y + c.ry)
            .flat_map(|y| {
                (c.centre.x - c.rx..=c.centre.x + c.rx).map(move |x| TilePos::new(x, y, c.centre.z))
            })
            .filter(|p| c.contains(*p) && w.grid.walkable(*p) && !w.grid.get(*p).is_water())
            .collect();
        (!floors.is_empty()).then(|| floors[r.gen_range(0..floors.len())])
    };
    for c in &caverns {
        let profile = crate::cave_ecology::profile(c);
        for _ in 0..profile.bats {
            if let Some(p) = floor(w, c, r) {
                w.spawn_agent(Species::Bat, p);
            }
        }
    }
    if let Some(c) = crate::cave_ecology::crawler_cavern(&caverns) {
        if let Some(p) = floor(w, c, r) {
            w.spawn_agent(Species::Crawler, p);
        }
    }
    // #336: a blind bear in a deep cavern that is not the crawler's, and a cave crocodile
    // lying up at the edge of the shallowest pool cavern.
    if let Some(c) = crate::cave_ecology::blind_bear_cavern(&caverns) {
        if let Some(p) = floor(w, c, r) {
            w.spawn_agent(Species::BlindCaveBear, p);
        }
    }
    if let Some(c) = crate::cave_ecology::croc_cavern(&caverns) {
        let floors: Vec<TilePos> = (c.centre.y - c.ry..=c.centre.y + c.ry)
            .flat_map(|y| {
                (c.centre.x - c.rx..=c.centre.x + c.rx).map(move |x| TilePos::new(x, y, c.centre.z))
            })
            .filter(|p| c.contains(*p) && w.grid.walkable(*p) && !w.grid.get(*p).is_water())
            .collect();
        // It lies up where something coming to drink steps within reach: a dry tile beside
        // the pool, else any dry floor of the cavern.
        let edge: Vec<TilePos> = floors
            .iter()
            .copied()
            .filter(|p| {
                [(1, 0), (-1, 0), (0, 1), (0, -1)]
                    .iter()
                    .any(|&(dx, dy)| w.grid.get(p.offset(dx, dy, 0)).is_water())
            })
            .collect();
        let pick = if edge.is_empty() { &floors } else { &edge };
        if !pick.is_empty() {
            w.spawn_agent(Species::CaveCroc, pick[r.gen_range(0..pick.len())]);
        }
    }
    if let Some(c) = crate::cave_ecology::burrower_cavern(&caverns) {
        if let Some(at) = floor(w, c, r) {
            w.spawn_agent(Species::Burrower, at);
            return;
        }
    }
    // Small worlds can have a deep cavern whose floor has no valid opening. Keep the old rock
    // fallback rather than silently removing the species from the world.
    let p = w.meta.params.clone();
    let (top, deep) = crate::world::gen::strata::breaks(&p);
    for _ in 0..40 {
        let (x, y) = (
            r.gen_range(8..w.grid.width - 8),
            r.gen_range(8..w.grid.height - 8),
        );
        let z = r.gen_range(deep + 1..top);
        let at = TilePos::new(x, y, z);
        if w.grid.get(at).shape.is_solid() && w.grid.ground_z(x, y) - z >= 3 {
            w.grid.set(at, crate::world::Tile::AIR);
            w.spawn_agent(Species::Burrower, at);
            break;
        }
    }
}

/// Bats flit about their cavern, and scatter from anyone who comes near.
fn bat(w: &mut World, i: usize, r: &mut rand_chacha::ChaCha8Rng) {
    let me = w.agents[i].pos;
    let Some(c) = w.grid.caverns.iter().find(|c| c.contains(me)).cloned() else {
        return;
    };
    let person = w
        .agents
        .iter()
        .find(|o| o.alive && o.species == Species::Person && o.pos.z == me.z && o.pos.dist(me) <= 4)
        .map(|o| o.pos);
    let to = match person {
        Some(p) => me.offset((me.x - p.x).signum() * 5, (me.y - p.y).signum() * 5, 0),
        None => TilePos::new(
            c.centre.x + r.gen_range(-c.rx..=c.rx),
            c.centre.y + r.gen_range(-c.ry..=c.ry),
            c.centre.z,
        ),
    };
    if c.contains(to) && w.grid.walkable(to) && w.path_agent_to(i, to) {
        w.agents[i].set_activity(Activity::Wandering { to });
    }
}
