//! The sites a community left when it moved camp (C72, docs/history/PLAN-NEXT.md M5): the houses
//! stand where they were, nobody's now, and weather into ruins; whoever comes on them later
//! and was not there wonders at them; and a community that moves carries what it can.

use serde::{Deserialize, Serialize};

use crate::events::{EventKind, Severity};
use crate::ids::{CommunityId, TilePos};
use crate::rng::{purpose, stream};
use crate::time::TICKS_PER_DAY;
use crate::world::Shape;
use crate::world_state::World;

/// A camp a community left.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LeftSite {
    /// An id of its own, past the old hamlets' (tales refer to a site by id).
    pub id: u32,
    pub community: CommunityId,
    pub name: String,
    pub home: TilePos,
    /// Where they went.
    pub to: TilePos,
    pub left: u64,
    pub people: u32,
    /// The wall tiles of its houses and the rooms dug into rock, as they stood.
    pub walls: Vec<TilePos>,
    /// The floors under their roofs.
    pub floors: Vec<TilePos>,
}

/// Left sites' ids start here, past any old hamlet's.
pub const FIRST_ID: u32 = 1_000_000;

/// A community moves from `home` to `to`: its old site is recorded as it stands.
pub fn record_move(w: &mut World, cid: CommunityId, home: TilePos, to: TilePos) {
    let Some(c) = w.communities.iter().find(|c| c.id == cid) else {
        return;
    };
    let mut walls = Vec::new();
    let mut floors = Vec::new();
    for o in &c.house_sites {
        let plan = crate::houses::plan(w, *o);
        for (q, _) in crate::houses::walls(&plan.floor) {
            if w.grid.get(q).shape == Shape::Wall {
                walls.push(q);
            }
        }
        floors.extend(plan.floor.iter().copied());
    }
    for h in &c.hollows {
        floors.extend(h.room.iter().copied());
    }
    if walls.is_empty() && floors.is_empty() {
        return;
    }
    let id = FIRST_ID + w.left_sites.len() as u32;
    let people = c.members.len() as u32;
    // The abandoned camp gets its own toponym, not the community's name, so a
    // tale of it reads "the last of <camp> went to <community>" rather than
    // "the last of Eldergate went to Eldergate" (issue #358).
    let name = crate::deep::hamlet_name(w.meta.seed, id);
    // What this camp leaves should teach the way the old hamlets' things do:
    // a made thing left on the floor, whose maker knew its craft, carries that craft on.
    teach_crafts_of_left_things(w, &floors, &name);
    w.left_sites.push(LeftSite {
        id,
        community: cid,
        name,
        home,
        to,
        left: w.meta.tick,
        people,
        walls,
        floors,
    });
}

/// The craft a recipe whose output is `kind` asks of whoever works it, if any (the first
/// recipe for that kind that names a craft at all; a plain stone tool, which names none,
/// is skipped in favour of a later recipe for the same kind that does).
fn craft_for_kind(kind: crate::items::ItemKind) -> Option<crate::mind::Technique> {
    crate::work::recipes::RECIPES
        .iter()
        .filter(|r| r.output.0 == kind)
        .filter_map(|r| crate::work::recipes::craft_of(r.id))
        .next()
}

/// A made thing left behind on the camp's floor teaches its craft on, the way the old
/// hamlets' things do: if its maker knew the craft its recipe calls for, its
/// story's `teaches` is set to that craft, so `Study` and the `decide.rs` designation pick
/// it up unchanged. Food and other unmade things are untouched, since they carry no maker.
fn teach_crafts_of_left_things(w: &mut World, floors: &[TilePos], site_name: &str) {
    let mut taught: Vec<usize> = Vec::new();
    for (idx, it) in w.items.iter().enumerate() {
        let crate::items::ItemPlace::Ground(p) = it.place else {
            continue;
        };
        if !floors.contains(&p) {
            continue;
        }
        let Some(s) = it.story.as_ref() else { continue };
        if s.teaches.is_some() {
            continue;
        }
        let Some(maker_id) = s.maker else { continue };
        let Some(craft) = craft_for_kind(it.kind) else {
            continue;
        };
        let Some(agent) = w.agents.iter().find(|a| a.id == maker_id) else {
            continue;
        };
        if agent.techniques.knows(craft) {
            taught.push(idx);
        }
    }
    for idx in taught {
        let craft = craft_for_kind(w.items[idx].kind);
        if let (Some(craft), Some(s)) = (craft, w.items[idx].story.as_mut()) {
            s.teaches = Some(craft);
            if s.origin.is_empty() {
                s.origin = format!("left at {site_name}");
            }
        }
    }
}

/// The left sites' walls as they lie now, for the client: standing while the wall is still
/// there and nobody's, fallen where its stones lie.
pub fn ruins(w: &World) -> Vec<(TilePos, bool)> {
    let mut out = Vec::new();
    let lived_in: std::collections::BTreeSet<TilePos> = w
        .communities
        .iter()
        .filter(|c| !c.dissolved)
        .flat_map(|c| {
            c.house_sites
                .iter()
                .flat_map(|o| crate::houses::walls(&crate::houses::plan(w, *o).floor))
                .map(|(q, _)| q)
                .collect::<Vec<_>>()
        })
        .collect();
    for s in &w.left_sites {
        for p in &s.walls {
            if lived_in.contains(p) {
                continue;
            }
            let t = w.grid.get(*p);
            if t.shape == Shape::Wall {
                out.push((*p, true));
            } else if t.shape == Shape::Open
                && !t.is_water()
                && w.grid.walkable(*p)
                && !w.structures.iter().any(|st| st.pos == *p)
            {
                out.push((*p, false));
            }
        }
    }
    out
}

/// Whether `p` is a wall of a site somebody left and nobody lives in now.
fn nobodys_wall(w: &World, p: TilePos) -> bool {
    w.left_sites.iter().any(|s| s.walls.contains(&p))
        && !w.communities.iter().filter(|c| !c.dissolved).any(|c| {
            c.house_sites.iter().any(|o| {
                crate::houses::walls(&crate::houses::plan(w, *o).floor)
                    .iter()
                    .any(|(q, _)| *q == p)
            })
        })
}

/// Daily: the walls nobody keeps weather and fall, timber in a few years, stone in many; a
/// roof over them goes when its wall does (the snow takes the rest, building.rs).
pub fn decay(w: &mut World) {
    let tick = w.meta.tick;
    let day = tick / TICKS_PER_DAY;
    let mut fallen: Vec<TilePos> = Vec::new();
    let walls: Vec<TilePos> = w
        .left_sites
        .iter()
        .flat_map(|s| s.walls.iter().copied())
        .collect();
    for p in walls {
        if !nobodys_wall(w, p) {
            continue;
        }
        let t = w.grid.get(p);
        if t.shape != Shape::Wall {
            continue;
        }
        let mut r = stream(
            w.meta.seed,
            &[purpose::HISTORY, 0x1EF7, p.x as u64, p.y as u64, day],
        );
        // Harder stone stands longer (#274): a block of the rock it was dressed from.
        let per_day = match t.material {
            crate::world::Material::STONE_BLOCK => {
                let hard = w
                    .made_of(p)
                    .map(|m| w.grid.materials.get(m).hardness)
                    .unwrap_or(1.0)
                    .max(0.5);
                1.0 / (3000.0 * hard)
            }
            _ => 1.0 / 400.0,
        };
        if rand::Rng::gen::<f32>(&mut r) < per_day {
            fallen.push(p);
        }
    }
    for p in fallen {
        let mut t = w.grid.get(p);
        t.shape = Shape::Open;
        t.material = crate::world::Material::AIR;
        w.grid.set(p, t);
        w.built_of.remove(&p);
        w.roofs.remove(&p);
        for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            let q = p.offset(dx, dy, 0);
            if w.left_sites.iter().any(|s| s.floors.contains(&q)) {
                w.roofs.remove(&q);
            }
        }
        w.events.push(
            tick,
            Some(p),
            Severity::Minor,
            EventKind::Burned {
                at: p,
                what: "a wall of the old camp, fallen with the weather".into(),
            },
        );
    }
}

/// A worn path at or above `paths::WORN` leading away from the site's own ground, read as the
/// compass direction out of it (the strongest-worn tile outside the houses).
fn worn_way_out(w: &World, s: &LeftSite) -> Option<&'static str> {
    let ours = |p: TilePos| s.floors.contains(&p) || s.walls.contains(&p);
    w.wear
        .iter()
        .filter(|(&(x, y), &n)| {
            n >= crate::paths::WORN
                && !ours(TilePos::new(x, y, s.home.z))
                && s.home.dist(TilePos::new(x, y, s.home.z)) <= 15
        })
        .max_by_key(|(&(x, y), _)| s.home.dist(TilePos::new(x, y, s.home.z)))
        .map(|(&(x, y), _)| crate::threat::compass(s.home, TilePos::new(x, y, s.home.z)))
}

/// Fearful readers see a curse; others read the site itself (#1816, D41): graves among the
/// houses say sickness, scorch says fire, a worn path leading out says which way, and failing
/// all of those, nobody can say where they went. Evidence noted in `how`.
fn read_site(w: &World, s: &LeftSite, fearful: bool) -> (crate::lore::Claim, String) {
    use crate::lore::Claim;
    if fearful {
        return (Claim::Curse, "their own fear".into());
    }
    let near = |p: TilePos| s.floors.contains(&p) || s.walls.iter().any(|t| t.dist(p) <= 1);
    let graves = w
        .graves
        .iter()
        .filter(|g| g.pos.z == s.home.z && near(g.pos))
        .count();
    if graves >= 3 {
        return (Claim::Fever, "the graves there".into());
    }
    let scorched = s
        .walls
        .iter()
        .chain(s.floors.iter())
        .any(|p| w.scorched.contains_key(p));
    if scorched {
        return (Claim::Fire, "the burned timbers".into());
    }
    if let Some(dir) = worn_way_out(w, s) {
        return (
            Claim::MovedTo(format!("the {dir}")),
            "a worn path leading out".into(),
        );
    }
    (Claim::Unknown, "nothing left to read".into())
}

/// Hourly: someone who was not of the community that left a site, standing among its houses,
/// wonders at them and keeps a tale of where the people went.
pub fn wonder(w: &mut World) {
    let tick = w.meta.tick;
    let sites = w.left_sites.clone();
    let mut news = Vec::new();
    for i in 0..w.agents.len() {
        let a = &w.agents[i];
        // A child does not wonder at ruins and keep a tale of them (#358: a two-day-old did).
        if !a.alive
            || a.community.is_none()
            || a.band.is_some()
            || a.activity.is_asleep()
            || a.is_child(tick)
        {
            continue;
        }
        let fearful = a.traits.has(crate::mind::Trait::Cautious)
            || a.beliefs.lean(crate::mind::beliefs::Topic::Death) > 0.3;
        for s in &sites {
            // Those who lived there and left with it know; those who came since do not.
            if a.pos.z != s.home.z
                || a.pos.dist(s.home) > 8
                || (a.community == Some(s.community) && a.came <= s.left)
                || a.tales.iter().any(|t| t.site == s.id)
            {
                continue;
            }
            news.push((i, s.id, s.name.clone(), read_site(w, s, fearful)));
        }
    }
    for (i, site, name, (claim, how)) in news {
        w.agents[i].tales.push(crate::lore::Tale {
            site,
            claim: claim.clone(),
            teller: None,
            how,
            heard: tick,
        });
        let (who, at) = (w.agents[i].id, w.agents[i].pos);
        w.events.push(
            tick,
            Some(at),
            Severity::Notable,
            EventKind::RuinWondered {
                agent: who,
                site: name.clone(),
                guess: claim.telling(&name),
            },
        );
    }
}

/// What a moving community carries (C72): each who goes takes a stack of food from the old
/// stores, up to four, for the road and the new camp; set down there on arrival (act.rs).
pub fn carry(w: &mut World, going: &[crate::ids::AgentId], home: TilePos) {
    let tick = w.meta.tick;
    let mut stacks: Vec<usize> = (0..w.items.len())
        .filter(|&k| {
            let it = &w.items[k];
            it.kind.is_food()
                && matches!(it.place, crate::items::ItemPlace::Ground(p) if p.dist(home) <= 20)
                && w.reservations
                    .holder(crate::work::ResKey::Item(it.id))
                    .is_none()
        })
        .collect();
    stacks.sort_by_key(|&k| w.items[k].id);
    for m in going {
        let Some(k) = stacks.pop() else { break };
        let (id, count) = (w.items[k].id, w.items[k].count);
        let take = count.min(4);
        if take == count {
            w.items[k].place = crate::items::ItemPlace::Carried(*m);
        } else {
            w.items[k].count -= take;
            let mut part = w.items[k].clone();
            part.id = w.alloc_item_id();
            part.count = take;
            part.place = crate::items::ItemPlace::Carried(*m);
            part.story = None;
            w.items.push(part);
            stacks.push(k);
        }
        let _ = id;
        let label = w
            .items
            .iter()
            .find(|it| it.place == crate::items::ItemPlace::Carried(*m))
            .map(|it| it.kind.label())
            .unwrap_or("food");
        if let Some(a) = w.agents.iter().find(|a| a.id == *m) {
            let pos = a.pos;
            w.events.push(
                tick,
                Some(pos),
                Severity::Minor,
                EventKind::Chose {
                    agent: *m,
                    what: format!("took {take} {label} for the road"),
                    because: "the stores were theirs to carry".into(),
                },
            );
        }
    }
}

pub fn step(w: &mut World) {
    if w.left_sites.is_empty() {
        return;
    }
    let tick = w.meta.tick;
    if tick.is_multiple_of(crate::time::TICKS_PER_HOUR) {
        wonder(w);
    }
    if tick % TICKS_PER_DAY == crate::time::TICKS_PER_HOUR * 5 + 3 {
        decay(w);
    }
}
