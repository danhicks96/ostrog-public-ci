//! Wells (#433): a community with no fresh water within a short walk of home digs a well.
//! Someone proposes it (systems/projects.rs `well`, with why: how far they walk for water);
//! the planner here picks a spot by home and sinks a shaft a level at a time (`plan`) until the
//! digging strikes water in the rock (aquifer.rs, #267's strata: the cut beside wet rock), and
//! then orders the well built over the shaft. Water struck is on record (`StruckWater`). A
//! shaft that reaches the hard rock of the middle stratum, or nine levels, without water is
//! given up, and why is on record; the community does not try again for a season of seasons.
//! A finished well is a place to drink (water.rs), and never freezes.

use crate::events::{EventKind, Severity};
use crate::ids::{CommunityId, TilePos};
use crate::project::{open_of, open_of_mut, ProjectKind};
use crate::structures::StructureKind;
use crate::systems::settlement::{find_flat_area, Here};
use crate::time::TICKS_PER_DAY;
use crate::world::{Material, Shape, TileFlags};
use crate::World;

/// A short walk for water, in tiles from home: farther than this and a well is wanted.
pub const SHORT_WALK: i32 = 12;
/// The deepest a well is dug, in levels below the ground.
pub const MAX_DEPTH: i32 = 9;
/// How long a community that gave up a well waits before trying again (four seasons).
pub const AGAIN_DAYS: u64 = 20;

/// How far from `p` the nearest open water lies (a surveyed body's water, not flood water or
/// the workings', frozen or not) or a well, within `r`; None if there is none.
pub fn water_within(w: &World, p: TilePos, r: i32) -> Option<i32> {
    let wells = w
        .structures
        .iter()
        .filter(|s| s.kind == StructureKind::Well && s.progress >= 1.0)
        .map(|s| s.pos.dist(p));
    let shores = w
        .waters
        .bodies
        .iter()
        .flat_map(|b| b.tiles.iter())
        .filter_map(|q| {
            let t = w.grid.get(*q);
            (t.is_water()
                && !w.flooded.contains(q)
                && !w.waters.left.contains_key(q)
                && !w.dug.contains(q))
            .then(|| q.dist(p))
        });
    wells.chain(shores).filter(|d| *d <= r).min()
}

/// Whether `at` is in the shaft of a well being dug.
pub fn shaft_at(w: &World, at: TilePos) -> bool {
    w.projects.iter().any(|p| {
        p.open()
            && p.kind == ProjectKind::Well
            && p.site
                .is_some_and(|s| s.x == at.x && s.y == at.y && at.z < s.z)
    })
}

/// Whether the rock beside a shaft cut at `at` holds water.
pub fn wet_beside(w: &World, at: TilePos) -> bool {
    // The shaft's own rock, from the ground as it was before the shaft, and the rock around.
    let site = w.projects.iter().find_map(|p| {
        (p.open() && p.kind == ProjectKind::Well)
            .then_some(p.site)
            .flatten()
            .filter(|s| s.x == at.x && s.y == at.y)
    });
    site.is_some_and(|s| crate::aquifer::held(w, at, s.z - 1))
        || [(1, 0), (-1, 0), (0, 1), (0, -1)]
            .iter()
            .any(|(dx, dy)| crate::aquifer::wet(w, at.offset(*dx, *dy, 0)))
}

/// The planner's part (systems/settlement.rs), each planning hour: a spot by home for a
/// wanted well, the next level of its shaft, and once water is struck the well itself.
pub fn plan(w: &mut World, cid: CommunityId, home: TilePos) {
    let Some(p) = open_of(w, cid, |k| *k == ProjectKind::Well).cloned() else {
        return;
    };
    if p.work.is_some() {
        return;
    }
    let tick = w.meta.tick;
    let site = match p.site {
        Some(s) => s,
        None => {
            let here = Here::of(w, cid, home);
            let spot = likeliest(w, home, &here).or_else(|| {
                [(4, 0), (0, 4), (-4, 0), (0, -4)]
                    .into_iter()
                    .map(|(dx, dy)| home.offset(dx, dy, 0))
                    .find_map(|n| find_flat_area(w, n, 1, 1, 1).filter(|q| here.holds(*q)))
            });
            let Some(spot) = spot else {
                give_up(w, cid, home, "found no place by home to dig it".into());
                return;
            };
            if let Some(pp) = open_of_mut(w, cid, |k| *k == ProjectKind::Well) {
                pp.site = Some(spot);
            }
            spot
        }
    };
    let ground = site.z - 1;
    let column = |z: i32| TilePos::new(site.x, site.y, z);
    // Water struck: the well over the shaft.
    if (ground - MAX_DEPTH..=ground).any(|z| w.grid.get(column(z)).flags.has(TileFlags::WATER)) {
        let d = crate::systems::settlement::make(
            w,
            crate::work::DesignationKind::BuildStructure(
                StructureKind::Well,
                Material::STONE_BLOCK,
            ),
            site,
            6,
        );
        let id = d.id;
        w.designations.push(d);
        if let Some(pp) = open_of_mut(w, cid, |k| *k == ProjectKind::Well) {
            pp.work = Some(id);
        }
        return;
    }
    // The next level down: the highest solid tile of the shaft.
    let Some(z) = (ground - MAX_DEPTH..=ground)
        .rev()
        .find(|z| w.grid.get(column(*z)).shape == Shape::Wall)
    else {
        give_up(
            w,
            cid,
            home,
            format!("dug {MAX_DEPTH} levels down and found no water"),
        );
        return;
    };
    let depth = ground - z;
    let c = w.grid.col(site.x, site.y);
    let hard = w
        .grid
        .strata
        .get(c)
        .is_some_and(|&(_, top, _)| z <= top as i32);
    if hard {
        give_up(
            w,
            cid,
            home,
            format!("dug {depth} levels down to the hard rock and found it dry"),
        );
        return;
    }
    let pos = column(z);
    if !w.designations.iter().any(|d| !d.done && d.pos == pos) {
        let d =
            crate::systems::settlement::make(w, crate::work::DesignationKind::DigStairs, pos, 6);
        w.designations.push(d);
    }
    let _ = tick;
}

/// How far from home a well is sited.
pub const SITE_RADIUS: i32 = 15;

/// Where water is likeliest within reach of home (#433), as anyone digging a well looks for
/// it: ground that shows water under it (the damp and the green over a basin's wet rock: the
/// rock of the column holds water within the depth a well is dug, read from #267's strata),
/// then the lowest ground, then where plants grow thick, then the nearest. Open ground the
/// community holds and can walk to; None if there is none.
pub(crate) fn likeliest(w: &World, home: TilePos, here: &Here) -> Option<TilePos> {
    likeliest_with(w, home, Some(here)).map(|(p, _)| p)
}

/// Whether any ground within reach of home shows water under it.
fn likeliest_shows(w: &World, home: TilePos) -> bool {
    likeliest_with(w, home, None).is_some_and(|(_, shows)| shows)
}

fn likeliest_with(w: &World, home: TilePos, here: Option<&Here>) -> Option<(TilePos, bool)> {
    let nav = w.nav.as_ref()?;
    let g = &w.grid;
    let home_ground = g.ground_z(home.x, home.y);
    let mut best: Option<(i64, TilePos, bool)> = None;
    for y in home.y - SITE_RADIUS..=home.y + SITE_RADIUS {
        for x in home.x - SITE_RADIUS..=home.x + SITE_RADIUS {
            if x < 1 || y < 1 || x >= g.width - 1 || y >= g.height - 1 {
                continue;
            }
            let s = g.surface(x, y);
            let d = s.dist(home);
            if d < 3
                || !g.walkable(s)
                || g.get(s).is_water()
                || g.get(s.offset(0, 0, -1)).shape != Shape::Wall
                || w.structures.iter().any(|t| t.pos == s)
                || w.designations.iter().any(|t| !t.done && t.pos == s)
                || here.is_some_and(|h| !h.holds(s))
                || !nav.reach(home, s)
            {
                continue;
            }
            let ground = s.z - 1;
            // Not where the hard rock lies at the surface: nobody digs a well into bare rock.
            let c = g.col(x, y);
            if g.strata
                .get(c)
                .is_some_and(|&(_, top, _)| top as i32 >= ground - 1)
            {
                continue;
            }
            let shows =
                (ground - MAX_DEPTH..ground).any(|z| crate::aquifer::wet(w, TilePos::new(x, y, z)));
            let green = w
                .plants
                .iter()
                .filter(|p| p.alive && p.pos.dist(s) <= 2)
                .count()
                .min(5) as i64;
            let score = if shows { 1000 } else { 0 } - 30 * (ground - home_ground) as i64
                + 3 * green
                - d as i64;
            if best
                .is_none_or(|(b, bp, _)| (score, std::cmp::Reverse(s)) > (b, std::cmp::Reverse(bp)))
            {
                best = Some((score, s, shows));
            }
        }
    }
    best.map(|(_, p, shows)| (p, shows))
}

fn give_up(w: &mut World, cid: CommunityId, home: TilePos, why: String) {
    let tick = w.meta.tick;
    let by = open_of(w, cid, |k| *k == ProjectKind::Well).map(|p| p.by);
    // Dry ground all round (nowhere within reach shows water): not tried again for a long
    // while, five years; else after a year, for another spot.
    let again = if likeliest_shows(w, home) {
        AGAIN_DAYS
    } else {
        AGAIN_DAYS * 5
    };
    if let Some(p) = open_of_mut(w, cid, |k| *k == ProjectKind::Well) {
        p.done = true;
    }
    if let Some(c) = w.communities.iter_mut().find(|c| c.id == cid) {
        c.no_site.retain(|(k, _)| *k != StructureKind::Well);
        c.no_site
            .push((StructureKind::Well, tick + again * TICKS_PER_DAY));
    }
    if let Some(agent) = by {
        w.events.push(
            tick,
            Some(home),
            Severity::Notable,
            EventKind::Chose {
                agent,
                what: "gave up on the well".into(),
                because: why,
            },
        );
    }
}
