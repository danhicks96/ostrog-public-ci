//! Perception into knowledge. Striped so each agent looks around once a game-minute. Agents
//! only ever learn what is within sight on their own level.

use crate::ids::TilePos;
use crate::items::ItemPlace;
use crate::mind::knowledge::Known;
use crate::mind::perception::line_of_sight;
use crate::mind::{Evidence, Observation, ObservationKind};
use crate::structures::StructureKind;
use crate::world::TileFlags;
use crate::world_state::World;

pub const STRIPE: u32 = 20;

thread_local! {
    static STORE_WALKS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
    static WATER_READS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

/// The quality tag a looker notes for a water tile: foul if the tile is fouled (the nose or
/// the eye makes that of it), else the surveyed quality, else fresh.
fn perceived_tag(w: &World, p: TilePos) -> u16 {
    if w.waters.foul.contains_key(&p) {
        return crate::floods::TAG_WATER_FOUL;
    }
    crate::floods::quality(w, p)
        .map(|q| q.tag())
        .unwrap_or(crate::floods::TAG_WATER_FRESH)
}

/// How many times the shared-store gather has walked the world's structures and items, for
/// the cost tests (#238): at most once a tick, however many people look.
pub fn store_walks() -> u64 {
    STORE_WALKS.with(|c| c.get())
}

/// Grid reads in the fused visible/foul water scan, for the cost tests (#428). Counted per
/// look and committed once from whichever thread ran the look, so on the parallel path it
/// holds one thread's reads; the cost tests use a single-looker world, which runs serially
/// and sees the exact count.
pub fn water_reads() -> u64 {
    WATER_READS.with(|c| c.get())
}

/// Side of the square cells the perception index buckets things into.
const CELL: i32 = 16;

fn cell(p: TilePos) -> (i32, i32, i32) {
    (p.x.div_euclid(CELL), p.y.div_euclid(CELL), p.z)
}

/// What there is to see, bucketed by cell and built once a tick for everyone looking that
/// tick: each person then looks only at the cells within sight instead of the whole valley,
/// and checks what they remember against lookups instead of whole lists (at 300 people the
/// full scans made `perceive` most of the ticks over 5 ms).
type Cell = (i32, i32, i32);
type Bucket = std::collections::BTreeMap<Cell, Vec<(Known, TilePos, u16)>>;

#[derive(Default)]
struct Index {
    items: Bucket,
    plants: Bucket,
    structures: Bucket,
    agents: std::collections::BTreeMap<Cell, Vec<(usize, Known, TilePos, u16)>>,
    ground: std::collections::BTreeMap<crate::ids::ItemId, TilePos>,
    fruit: std::collections::BTreeSet<TilePos>,
    standing: std::collections::BTreeSet<crate::ids::StructureId>,
    /// Food on the ground, which a nose finds without sight (P4).
    food: std::collections::BTreeSet<crate::ids::ItemId>,
    /// People and beasts awake: moving and working make the noise hearing picks up (P4).
    awake: std::collections::BTreeSet<usize>,
}

impl Index {
    /// Only the cells someone looks into this tick: everything else is skipped, not bucketed
    /// (the M4 review's profile: building the whole index every tick was most of `perceive`).
    fn build(w: &World, needed: &std::collections::BTreeSet<Cell>) -> Index {
        let mut ix = Index::default();
        let want = |p: TilePos| needed.contains(&cell(p));
        for it in &w.items {
            if let ItemPlace::Ground(p) = it.place {
                if !want(p) {
                    continue;
                }
                ix.items
                    .entry(cell(p))
                    .or_default()
                    .push((Known::Item(it.id), p, it.kind as u16));
                ix.ground.insert(it.id, p);
                if it.kind.is_food() {
                    ix.food.insert(it.id);
                }
            }
        }
        for pl in w
            .plants
            .iter()
            .filter(|pl| pl.alive && pl.kind.bears_fruit() && pl.fruit > 0.3 && want(pl.pos))
        {
            ix.plants.entry(cell(pl.pos)).or_default().push((
                Known::FoodPlant(pl.pos),
                pl.pos,
                pl.kind as u16,
            ));
            ix.fruit.insert(pl.pos);
        }
        for s in w.structures.iter().filter(|s| want(s.pos)) {
            ix.structures.entry(cell(s.pos)).or_default().push((
                Known::Structure(s.id),
                s.pos,
                s.kind as u16,
            ));
            ix.standing.insert(s.id);
        }
        for (j, o) in w
            .agents
            .iter()
            .enumerate()
            .filter(|(_, o)| o.alive && want(o.pos))
        {
            if !o.activity.is_asleep() {
                ix.awake.insert(j);
            }
            ix.agents.entry(cell(o.pos)).or_default().push((
                j,
                Known::Agent(o.id),
                o.pos,
                o.species as u16,
            ));
        }
        ix
    }

    /// The cells within `r` tiles of `me`, in a fixed order.
    fn near(me: TilePos, r: i32) -> impl Iterator<Item = (i32, i32, i32)> {
        let (x0, x1) = ((me.x - r).div_euclid(CELL), (me.x + r).div_euclid(CELL));
        let (y0, y1) = ((me.y - r).div_euclid(CELL), (me.y + r).div_euclid(CELL));
        (y0..=y1).flat_map(move |y| (x0..=x1).map(move |x| (x, y, me.z)))
    }
}

/// What members of a community know without looking: its hearths and the contents of its
/// stockpiles within 40 of home. Gathered once a tick for everyone looking that tick,
/// indexed by community, instead of re-walked per looker (#238: the per-looker walk of
/// every structure and every item grew with the days' buildings and filled stockpiles).
#[derive(Default)]
struct SharedStore {
    /// Hearths near home, in the world's structure order.
    hearths: Vec<(Known, TilePos, u16)>,
    /// Stockpiles near home, in the world's structure order, each with what sits on it.
    piles: Vec<(TilePos, Vec<(crate::ids::ItemId, u16)>)>,
}

impl SharedStore {
    /// One walk of the world for every community (fix-238-perceive-growth): ground items
    /// bucketed by tile, then the structures walked once, filed by home distance. The note
    /// order within a community is the same as the per-looker walk this replaced, so
    /// noting — digest-visible — is byte-identical.
    fn gather(w: &World) -> std::collections::BTreeMap<crate::ids::CommunityId, SharedStore> {
        STORE_WALKS.with(|c| c.set(c.get() + 1));
        let mut on_ground: std::collections::BTreeMap<TilePos, Vec<(crate::ids::ItemId, u16)>> =
            Default::default();
        for it in &w.items {
            if let ItemPlace::Ground(p) = it.place {
                on_ground
                    .entry(p)
                    .or_default()
                    .push((it.id, it.kind as u16));
            }
        }
        let mut stores: std::collections::BTreeMap<crate::ids::CommunityId, SharedStore> =
            Default::default();
        for c in &w.communities {
            let mut store = SharedStore::default();
            for s in &w.structures {
                if s.pos.dist(c.home) > 40 {
                    continue;
                }
                if s.kind == StructureKind::Hearth && s.progress >= 1.0 {
                    store
                        .hearths
                        .push((Known::Structure(s.id), s.pos, s.kind as u16));
                }
                if s.kind == StructureKind::Stockpile {
                    store
                        .piles
                        .push((s.pos, on_ground.get(&s.pos).cloned().unwrap_or_default()));
                }
            }
            stores.insert(c.id, store);
        }
        stores
    }
}

/// The strongest an expert cue can be: an attention below `NOTICE / EXPERT_MAX` cannot notice
/// any, so its scan is skipped (#569).
const EXPERT_MAX: f32 = 0.9;
/// How far the reading eye reaches for animal sign and stuck sites (#569).
const SIGN_RANGE: i32 = 10;
const SITE_RANGE: i32 = 8;
/// A reading like one made within this range in the last day is habituated (#569).
const HABIT_RANGE: i32 = 6;
/// The reading eye looks on one look in this many (#569): attention is limited, and it keeps
/// the cost of expertise to a fraction of perception's (measured at 500 people, FIXLOG).
pub const EXPERT_EVERY: u64 = 4;

/// A building site stuck for want of something (#569), gathered once a tick for every expert
/// looking that tick instead of each walking the designations.
struct StuckSite {
    pos: TilePos,
    id: u32,
    kind: crate::items::ItemKind,
    count: u32,
}

fn stuck_sites(w: &World) -> Vec<StuckSite> {
    use crate::work::DesignationKind as K;
    w.designations
        .iter()
        .filter(|d| {
            !d.done
                && matches!(
                    d.kind,
                    K::BuildWall(_)
                        | K::BuildDoor
                        | K::BuildStairs(_)
                        | K::BuildFloor(_)
                        | K::BuildRoof(_)
                        | K::BuildStructure(..)
                )
        })
        .filter_map(|d| {
            d.missing_wants().next().map(|(kind, count, _)| StuckSite {
                pos: d.pos,
                id: d.id.0,
                kind,
                count,
            })
        })
        .collect()
}

/// Expertise shapes what is noticed (#569, mind/attention.rs): for one looker, the ground under
/// them, the nearest animal that passed unseen, and the nearest building site they can see that
/// is stuck for want of something. Each is weighed against their attention for it; what reaches
/// `NOTICE` comes back as an observation with its reading, and for the traced person every cue
/// also comes back as a percept, noticed or not. A domain their attention cannot reach is not
/// scanned at all unless they are traced, which keeps the look's cost to the experts.
fn expert_look(
    w: &World,
    i: usize,
    ix: &Index,
    sees: &dyn Fn(TilePos) -> bool,
    traced: bool,
    stuck: &[StuckSite],
) -> (Vec<Observation>, Vec<crate::why::Percept>) {
    use crate::mind::attention::{attention, salience, Domain, NOTICE};
    use crate::mind::Reading;
    use crate::work::DesignationKind as K;
    let a = &w.agents[i];
    let me = a.pos;
    let tick = w.meta.tick;
    let task = match &a.activity {
        crate::mind::Activity::Working { plan } => w
            .designations
            .iter()
            .find(|d| d.id == plan.designation)
            .and_then(|d| match d.kind {
                K::Sow | K::Harvest => Some(Domain::Ground),
                K::Hunt(_) | K::Snare | K::Tame(_) => Some(Domain::Sign),
                K::BuildWall(_)
                | K::BuildDoor
                | K::BuildStairs(_)
                | K::BuildFloor(_)
                | K::BuildRoof(_)
                | K::BuildStructure(..) => Some(Domain::Building),
                _ => None,
            }),
        _ => None,
    };
    let mut noticed = Vec::new();
    let mut percepts = Vec::new();
    for domain in [Domain::Ground, Domain::Sign, Domain::Building] {
        let att = attention(a, domain, task == Some(domain));
        if att * EXPERT_MAX < NOTICE && !traced {
            continue;
        }
        let cue: Option<(ObservationKind, TilePos, f32, Reading)> = match domain {
            Domain::Ground => {
                use crate::world::Material as M;
                let below = me.offset(0, 0, -1);
                let ground = w.grid.in_bounds(below).then(|| w.grid.get(below).material);
                ground
                    .filter(|m| {
                        matches!(
                            *m,
                            M::MEADOW
                                | M::SOIL
                                | M::GRASS
                                | M::PEAT
                                | M::WET_SOIL
                                | M::MOSS
                                | M::MUD
                                | M::CLAY
                                | M::PACKED_EARTH
                                | M::SAND
                                | M::GRAVEL
                                | M::SCREE
                                | M::FROZEN_SOIL
                                | M::SNOW
                                | M::BONE_MOSS
                        )
                    })
                    .map(|m| {
                        let f =
                            crate::plants::crop_ground_factor(crate::plants::PlantKind::Oats, m);
                        let pct = (f * 100.0).round().clamp(0.0, 255.0) as u8;
                        (ObservationKind::Soil, me, 0.8, Reading::Ground { pct })
                    })
            }
            Domain::Sign => Index::near(me, SIGN_RANGE)
                .flat_map(|c| ix.agents.get(&c).into_iter().flatten())
                .filter(|&&(j, _, p, _)| {
                    j != i
                        && p.z == me.z
                        && p.dist(me) <= SIGN_RANGE
                        && w.agents[j].species != crate::agents::Species::Person
                        && !w.agents[j].denned
                        && !sees(p)
                })
                .min_by_key(|&&(j, _, p, _)| (p.dist(me), j))
                .map(|&(j, _, p, _)| {
                    let near = 1.0 - p.dist(me) as f32 / (2.0 * SIGN_RANGE as f32);
                    (
                        ObservationKind::Tracks,
                        p,
                        EXPERT_MAX * near,
                        Reading::Passed(w.agents[j].species),
                    )
                }),
            Domain::Building => stuck
                .iter()
                .filter(|s| s.pos.z == me.z && s.pos.dist(me) <= SITE_RANGE && sees(s.pos))
                .min_by_key(|s| (s.pos.dist(me), s.id))
                .map(|s| {
                    (
                        ObservationKind::Joint,
                        s.pos,
                        0.85,
                        Reading::Lacks {
                            kind: s.kind,
                            count: s.count,
                        },
                    )
                }),
        };
        let Some((kind, pos, strength, reading)) = cue else {
            continue;
        };
        // Habituation: the same reading made near here within the day has faded from notice;
        // a changed reading is noticed again.
        let habituated = a.learning.observations.iter().any(|o| {
            o.kind == kind
                && o.pos.z == pos.z
                && o.pos.dist(pos) <= HABIT_RANGE
                && o.reading == Some(reading)
                && tick.saturating_sub(o.tick) < crate::time::TICKS_PER_DAY
        });
        let s = salience(att, strength, habituated);
        if s >= NOTICE {
            noticed.push(Observation {
                tick,
                pos,
                kind,
                strength,
                evidence: Evidence::Direct,
                event: None,
                full: false,
                reading: Some(reading),
            });
        }
        if traced {
            percepts.push(crate::why::Percept {
                tick,
                domain: domain.name(),
                what: crate::view::experience::reading_label(reading),
                salience: s,
                noticed: s >= NOTICE,
                habituated,
            });
        }
    }
    (noticed, percepts)
}

/// This person's current sight radius under the same daylight, moonlight and lamp rules as a
/// striped perception pass.
pub fn sight_radius(w: &World, a: &crate::agents::Agent, tick: u64) -> i32 {
    let lit = crate::underground::light_at(w, a.pos);
    let light = if lit >= 1.0 && crate::time::Calendar::is_night(tick) {
        0.4 + 0.3 * crate::time::Calendar::moon(tick)
    } else {
        lit
    };
    a.species_def().sight_radius(light)
}

/// What one looker takes away from a look. Computed from a shared read-only view of the
/// world (parallel over the lookers), then applied on the calling thread in looker order,
/// so the thread count changes only how long a look takes, never what it notes (#428, D55;
/// WP-01 slice 1.2: the per-looker work is ~75 % of the 500-person mean tick).
struct LookOut {
    heard: Vec<crate::view::hearing::Contact>,
    /// The community whose map-cells this looker explores.
    community: Option<crate::ids::CommunityId>,
    /// Expert observations, in the order they were noticed.
    expert: Vec<Observation>,
    /// Things noted from the senses, in the order sensed.
    notes: Vec<(Known, TilePos, u16)>,
    /// Visible water, with its perceived tag.
    water_note: Option<(TilePos, u16)>,
    /// The learning observation water noting makes.
    water_obs: Option<Observation>,
    /// Fouled-water tiles noted foul.
    foul: Vec<TilePos>,
    /// Beliefs in sight that are gone.
    forgets: Vec<Known>,
    /// Standing water the hazard pass reacts to.
    wet: Option<TilePos>,
    /// Expert cues for the traced person only.
    percepts: Vec<crate::why::Percept>,
}

pub fn step(w: &mut World) {
    let tick = w.meta.tick;
    let stripe = (tick % STRIPE as u64) as usize;
    let n = w.agents.len();
    // Who looks this tick, and the cells they look into.
    let lookers: Vec<usize> = (0..n)
        .filter(|&i| {
            i % STRIPE as usize == stripe
                && w.agents[i].alive
                && !w.agents[i].activity.is_asleep()
                && w.agents[i].species == crate::agents::Species::Person
        })
        .collect();
    if let Some(cache) = w.watcher_hearing.as_mut() {
        cache.begin_tick(tick);
    }
    if lookers.is_empty() {
        return;
    }
    let needed: std::collections::BTreeSet<Cell> = lookers
        .iter()
        .flat_map(|&i| {
            // As far as any of their senses could reach (P4).
            let sp = w.agents[i].species_def();
            let reach = sp
                .sight_radius(1.0)
                .max(sp.hearing_radius())
                .max(sp.smell_radius());
            Index::near(w.agents[i].pos, reach)
        })
        .collect();
    // Built once a tick for everyone looking that tick, as before; the lookers then share
    // them read-only across the pool.
    let index = Index::build(w, &needed);
    let stores = SharedStore::gather(w);
    // Gathered when some looker is due an expert look (the only reader), as the lazy
    // per-tick gather did; the walk is pure, so when it runs changes nothing but its cost.
    let stuck: Vec<StuckSite> = if lookers
        .iter()
        .any(|&i| (tick / STRIPE as u64 + i as u64).is_multiple_of(EXPERT_EVERY))
    {
        stuck_sites(w)
    } else {
        Vec::new()
    };
    let outs = crate::par::map(&lookers, |&i| look(w, i, tick, &index, &stores, &stuck));
    for (&i, out) in lookers.iter().zip(outs) {
        apply(w, i, tick, out);
    }
}

/// One looker's pass over the world: everything the senses take, worked out from the shared
/// read-only view; no writes (the par idiom, see par.rs).
fn look(
    w: &World,
    i: usize,
    tick: u64,
    ix: &Index,
    st: &std::collections::BTreeMap<crate::ids::CommunityId, SharedStore>,
    stuck: &[StuckSite],
) -> LookOut {
    let mut out = LookOut {
        heard: Vec::new(),
        community: None,
        expert: Vec::new(),
        notes: Vec::new(),
        water_note: None,
        water_obs: None,
        foul: Vec::new(),
        forgets: Vec::new(),
        wet: None,
        percepts: Vec::new(),
    };
    let me = w.agents[i].pos;
    out.community = w.agents[i].community;
    let mut seen: Vec<(Known, TilePos, u16)> = Vec::new();
    // Their kind's senses (P4): sight shortened by the dark unless their eyes are made
    // for it, hearing of people moving and working through walls, a nose for food and
    // living things.
    let sp = w.agents[i].species_def();
    let sight = sight_radius(w, &w.agents[i], tick);
    let hear = sp.hearing_radius();
    let smell = sp.smell_radius();
    let reach = sight.max(hear).max(smell);
    let grid = &w.grid;
    let d = |p: TilePos| (p.x - me.x).abs().max((p.y - me.y).abs());
    // Line of sight is most of a look's cost, and many things share a tile (a stockpile's
    // items, a crowd): each tile's answer is worked out once a look (#205).
    let side = (2 * sight + 1).max(1) as usize;
    let los_memo = std::cell::RefCell::new(vec![0u8; side * side]);
    let sees = |p: TilePos| {
        if p.z != me.z || d(p) > sight {
            return false;
        }
        let at = (p.y - me.y + sight) as usize * side + (p.x - me.x + sight) as usize;
        let mut memo = los_memo.borrow_mut();
        if memo[at] == 0 {
            memo[at] = if line_of_sight(grid, me, p) { 1 } else { 2 };
        }
        memo[at] == 1
    };
    let near = |p: TilePos, r: i32| p.z == me.z && d(p) <= r;
    // Items on the ground: seen, or food smelled.
    for c in Index::near(me, reach) {
        for &(k, p, tag) in ix.items.get(&c).into_iter().flatten() {
            let food = matches!(k, Known::Item(id) if ix.food.contains(&id));
            if (food && near(p, smell)) || sees(p) {
                seen.push((k, p, tag));
            }
        }
    }
    // The community's store is shared knowledge: members know what sits on their own
    // stockpile tiles, on any level, without having to look. Gathered once a tick for
    // all lookers (#238, FIXLOG 2026-10-28 fix-238-perceive-growth), not walked per
    // looker.
    if let Some(cid) = w.agents[i].community {
        if let Some(store) = st.get(&cid) {
            // Nobody forgets where home's fire is: the community's hearths are known to
            // all of it, wherever they are (a memory full of berry bushes once pushed the
            // only hearth out of mind, and people froze looking for it).
            for &(k, p, tag) in &store.hearths {
                seen.push((k, p, tag));
            }
            for &(p, ref items) in &store.piles {
                for &(id, tag) in items {
                    seen.push((Known::Item(id), p, tag));
                }
            }
        }
    }
    // Food plants with fruit, structures (beds, hearths, stockpiles), and other people.
    for c in Index::near(me, reach) {
        for &(k, p, tag) in ix.plants.get(&c).into_iter().flatten() {
            if near(p, smell) || sees(p) {
                seen.push((k, p, tag));
            }
        }
    }
    for c in Index::near(me, sight) {
        for &(k, p, tag) in ix.structures.get(&c).into_iter().flatten() {
            if sees(p) {
                seen.push((k, p, tag));
            }
        }
    }
    let mut others = Vec::new();
    let hears = |j: usize, p| near(p, hear) && ix.awake.contains(&j);
    for c in Index::near(me, reach) {
        for &(j, k, p, _) in ix.agents.get(&c).into_iter().flatten() {
            if j != i && (near(p, smell) || hears(j, p) || sees(p)) {
                others.push((k, p, j));
            }
        }
    }
    // In a crowd, the nearest `PEOPLE_CAP` of them. Noting everyone sensed, a full mind
    // forgot people as fast as it noted them and kept whoever had the highest ids (all
    // were seen on the same tick, and ties go by key), and the churn was most of a fed
    // 500-person tick (C55). Fewer than that, all of them, in the order sensed.
    if others.len() > crate::mind::knowledge::PEOPLE_CAP {
        // The same nearest set as a full sort by (distance, key), found by selection (#205).
        let mut by_near: Vec<(i32, Known, usize)> = others
            .iter()
            .enumerate()
            .map(|(n, o)| (d(o.1), o.0, n))
            .collect();
        by_near.select_nth_unstable(crate::mind::knowledge::PEOPLE_CAP - 1);
        let mut keep = vec![false; others.len()];
        for &(_, _, n) in &by_near[..crate::mind::knowledge::PEOPLE_CAP] {
            keep[n] = true;
        }
        let mut n = 0;
        others.retain(|_| {
            n += 1;
            keep[n - 1]
        });
    }
    let listener = &w.agents[i];
    let collect = w.watcher_hearing.is_some() && crate::view::hearing::listener(w, listener);
    for (k, p, j) in others {
        seen.push((k, p, w.agents[j].species as u16));
        if collect && hears(j, p) {
            if let Some(row) = crate::view::hearing::Contact::sample(w, i, j) {
                out.heard.push(row);
            }
        }
    }
    // A look-alike plant is food to whoever does not know it for one (#286): filtered
    // here, reading only this looker's own mind.
    let a = &w.agents[i];
    out.notes = seen
        .into_iter()
        .filter(|&(k, _, tag)| {
            if let Known::FoodPlant(_) = k {
                !crate::plants::PlantKind::from_tag(tag).is_some_and(|pk| {
                    pk.look_alike().is_some() && crate::plants::knows_look_alike(a, pk)
                })
            } else {
                true
            }
        })
        .collect();
    // Water within a smaller radius (cheap scan). The belief is tagged with the quality
    // as perceived (#453 part 4, FIXLOG 2026-10-30): what the eye takes of it by sight
    // (salt, brackish, fresh — floods.rs `TAG_WATER_*`), and a fouled tile seen or smelled
    // is noted foul even where a sight line fails (humans have no nose to speak of; the
    // smell pass is for those who do). The tile underfoot is considered before the first
    // other scan hit, so another visible water tile cannot hide current danger
    // (FIXLOG 2026-10-30, PR #476).
    let mut water: Option<TilePos> = None;
    let wr = sight.min(6);
    // Fouled water seen or smelled, wherever it stands in reach: one foul shore noted
    // is the pond remembered foul (water.rs `believed_tag` taints the whole body), so
    // one tile per body is noted, not every fouled tile — a noted field of them churned
    // the mind's place budget. The nose reaches where the sight window does not.
    let sr = smell.max(wr);
    let mut tainted: Vec<&crate::floods::Body> = Vec::new();
    // Counted per look and committed once: the scan runs on pool threads, where a
    // thread-local would only ever see the calling thread's reads (the cost tests read it
    // from the serial single-looker path, which sees the exact count).
    let mut reads: u64 = 0;
    // One sr-radius grid read per tile; the wr subsequence keeps first-visible order
    // (#428, FIXLOG 2026-10-31). The underfoot tile overrides an earlier scan hit
    // (PR #476): standing in visible water, the current tile is the danger to note.
    for dy in -sr..=sr {
        for dx in -sr..=sr {
            let p = me.offset(dx, dy, 0);
            if !w.grid.in_bounds(p) {
                continue;
            }
            reads += 1;
            let tile = w.grid.get(p);
            if !tile.flags.has(TileFlags::WATER) {
                continue;
            }
            if (water.is_none() || p == me) && dx.abs() <= wr && dy.abs() <= wr && sees(p) {
                water = Some(p);
            }
            if !w.waters.foul.contains_key(&p) {
                continue;
            }
            let smelled = smell > 0 && near(p, smell);
            if !smelled && !sees(p) {
                continue;
            }
            if let Some(b) = crate::floods::body_at(w, p) {
                if tainted.iter().any(|t| std::ptr::eq(*t, b)) {
                    continue;
                }
                tainted.push(b);
            }
            out.foul.push(p);
        }
    }
    WATER_READS.with(|c| c.set(c.get() + reads));
    out.water_note = water.map(|p| (p, perceived_tag(w, p)));
    if let Some(p) = water {
        out.water_obs = Some(Observation {
            tick,
            pos: p,
            kind: if w.grid.get(p).flags.has(TileFlags::DEEP)
                || w.grid.get(p).flags.has(TileFlags::FULL)
            {
                ObservationKind::WaterDepth
            } else {
                ObservationKind::Wetness
            },
            strength: if w.grid.get(p).flags.has(TileFlags::DEEP)
                || w.grid.get(p).flags.has(TileFlags::FULL)
            {
                1.0
            } else {
                0.55
            },
            evidence: Evidence::Direct,
            event: None,
            full: false,
            reading: None,
        });
        out.wet = Some(p);
    }
    let traced = w.why_target() == Some(w.agents[i].id);
    let due = (tick / STRIPE as u64 + i as u64).is_multiple_of(EXPERT_EVERY);
    if due {
        let (expert, percepts) = expert_look(w, i, ix, &sees, traced, stuck);
        out.expert = expert;
        if traced {
            out.percepts = percepts;
        }
    }
    // Beliefs about things in sight range that are no longer there get corrected: what is
    // no longer where it was believed to be, checked against the index first and by line
    // of sight only then: a sight line for every belief in range was a third of a look
    // (#205). The same per-look tile memo answers a missing belief on a tile already
    // checked above without tracing that tile again.
    //
    // A belief this look re-notes is not a candidate: the serial pass noted it before the
    // sweep, so the sweep saw the fresh position (which matches the index wherever it is
    // visible) and kept it. Skipping noted keys here is the same outcome without applying
    // the notes twice.
    let noted: std::collections::BTreeSet<Known> = out.notes.iter().map(|&(k, _, _)| k).collect();
    let a = &w.agents[i];
    out.forgets = a
        .knowledge
        .map
        .iter()
        .filter(|(k, b)| {
            if noted.contains(*k) {
                return false;
            }
            if b.pos.z != me.z || (b.pos.x - me.x).abs().max((b.pos.y - me.y).abs()) > sight {
                return false;
            }
            let missing = match k {
                Known::Item(id) => ix.ground.get(id) != Some(&b.pos),
                Known::FoodPlant(p) => !ix.fruit.contains(p),
                Known::Structure(id) => !ix.standing.contains(id),
                _ => false,
            };
            missing && sees(b.pos)
        })
        .map(|(k, _)| *k)
        .collect();
    out
}

/// The calling-thread half of a look: `out`, computed in parallel by `look`, applied in
/// looker order, so every write lands exactly where the serial pass put it.
fn apply(w: &mut World, i: usize, tick: u64, out: LookOut) {
    if let Some(cache) = w.watcher_hearing.as_mut() {
        cache.record(tick, out.heard);
    }
    let me = w.agents[i].pos;
    // What the community has seen of the map: the cell stood in and its neighbours.
    if let Some(cid) = out.community {
        if let Some(c) = w.communities.iter_mut().find(|c| c.id == cid) {
            let (cx, cy) = (me.x.div_euclid(8), me.y.div_euclid(8));
            for dx in -1..=1 {
                for dy in -1..=1 {
                    c.explored.insert((cx + dx, cy + dy, me.z));
                }
            }
        }
    }
    let traced = w.why_target() == Some(w.agents[i].id);
    if traced {
        if let Some(t) = w.why.as_mut() {
            for p in out.percepts {
                t.perceive(p);
            }
        }
    }
    let a = &mut w.agents[i];
    for o in out.expert {
        a.learning.observe(o);
    }
    for (k, p, tag) in out.notes {
        a.knowledge.note(k, p, tick, tag);
    }
    if let Some((p, tag)) = out.water_note {
        a.knowledge.note(Known::Water(p), p, tick, tag);
    }
    if let Some(obs) = out.water_obs {
        a.learning.observe(obs);
    }
    for p in out.foul {
        a.knowledge
            .note(Known::Water(p), p, tick, crate::floods::TAG_WATER_FOUL);
    }
    for k in out.forgets {
        a.knowledge.forget(k);
    }
    if let Some(p) = out.wet {
        crate::systems::hazards::observe_water(w, i, p);
    }
}
