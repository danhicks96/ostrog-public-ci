//! The community planner: looks at what the settlement has and lacks, and posts
//! designations. It never does the work and never conjures materials. Runs hourly.
//! Phase 4 scope: one community (the founding party) around the founding site.

use crate::ids::TilePos;
use crate::items::{ItemKind, ItemPlace};
use crate::plants::PlantKind;
use crate::structures::StructureKind;
use crate::time::TICKS_PER_HOUR;
use crate::work::designation::{structure_cost, times};
use crate::work::{Designation, DesignationKind, Requirement};
use crate::world::{Material, Shape};
use crate::world_state::World;

pub fn step(w: &mut World) {
    if !w.meta.autoplan
        || !w.meta.tick.is_multiple_of(TICKS_PER_HOUR)
        || w.agents
            .iter()
            .filter(|a| a.alive && a.species == crate::agents::Species::Person)
            .count()
            == 0
    {
        return;
    }
    // Drop finished orders, hopeless ones, hauls whose stack is no longer there, and hunts
    // whose animal is dead or gone (wolves or a fall took it; with failures fading, such a hunt
    // was never dropped and nobody hunted again).
    // Where a ground item sits, for the stale-haul check below: a set lookup per designation,
    // not a walk of every item per haul (at 500 people that walk, hourly, was the settlement
    // system's spike: hundreds of designations over thousands of items).
    let ground: std::collections::BTreeSet<crate::ids::TilePos> = w
        .items
        .iter()
        .filter_map(|it| match it.place {
            ItemPlace::Ground(p) => Some(p),
            _ => None,
        })
        .collect();
    // Works in a fey mood are the mood's to end (systems/moods.rs), however often they fail.
    let fey_works: std::collections::BTreeSet<crate::work::DesignationId> = w
        .agents
        .iter()
        .filter_map(|a| a.fey.as_ref().map(|f| f.work))
        .collect();
    let living: std::collections::BTreeSet<crate::ids::AgentId> =
        w.agents.iter().filter(|a| a.alive).map(|a| a.id).collect();
    let mut ds = std::mem::take(&mut w.designations);
    let gone = crate::work::split_off(&mut ds, |d| {
        let stale_haul = d.kind == DesignationKind::Haul && !ground.contains(&d.pos);
        let stale_hunt = matches!(d.kind, DesignationKind::Hunt(prey) if !living.contains(&prey));
        !d.done && (d.failures <= 12 || fey_works.contains(&d.id)) && !stale_haul && !stale_hunt
    });
    w.designations = ds;
    w.give_back(gone);
    let tick = w.meta.tick;
    for a in w.agents.iter_mut() {
        a.work_cooldowns.retain(|_, t| *t > tick);
    }
    // Failures are recent trouble, not a mark for life: they fade, one every four hours, so an
    // order that failed while timber was short is taken up again once there is timber (it
    // scores 1 - 0.1 x failures in decide.rs and sat at a tenth of its worth for days).
    if (tick / TICKS_PER_HOUR).is_multiple_of(4) {
        for d in w.designations.iter_mut() {
            d.failures = d.failures.saturating_sub(1);
        }
    }
    let ids: Vec<crate::ids::CommunityId> = w
        .communities
        .iter()
        .filter(|c| !c.dissolved)
        .map(|c| c.id)
        .collect();
    for cid in ids {
        plan_community(w, cid);
    }
}

/// The current community being planned for (set by `plan_community`).
fn plan_community(w: &mut World, cid: crate::ids::CommunityId) {
    let Some(c) = w.communities.iter().find(|c| c.id == cid) else {
        return;
    };
    let home = c.home;
    let people = c
        .members
        .iter()
        .filter(|m| w.agents.iter().any(|a| a.id == **m && a.alive))
        .count() as u32;
    if people == 0 {
        return;
    }
    // What this community holds: what lies within reach of its home and nearer it than any
    // other community's (the M4 review: every count was valley-wide, so after a split a hungry
    // community saw its neighbours' full stores and sowed one plot through a famine).
    let here = Here::of(w, cid, home);
    let house = c.house_sites.first().copied();
    let houses: Vec<TilePos> = c.house_sites.clone();
    w.meta.planning_for = Some(cid);
    let field = c.field_site;
    let members: Vec<crate::ids::AgentId> = c.members.clone();
    let _ = &members;
    // 1. A stockpile zone next to the founding site.
    // A store under water is no store (#405, FIXLOG 2026-10-27): what lies there is hauled out
    // to dry ground, and the store grows elsewhere.
    let stock_tiles: Vec<TilePos> = w
        .structures
        .iter()
        .filter(|s| s.kind == StructureKind::Stockpile && !w.grid.get(s.pos).is_water())
        .map(|s| s.pos)
        .collect();
    if stock_tiles.is_empty()
        && !has_designation(w, |k| {
            matches!(
                k,
                DesignationKind::BuildStructure(StructureKind::Stockpile, _)
            )
        })
    {
        // Grown from need (C71, D35): a tile for every two loose stacks lying about, six at
        // least, spreading from the ground beside home rather than a 4 by 3 block.
        let loose = w
            .items
            .iter()
            .filter(|it| matches!(it.place, ItemPlace::Ground(_)) && here.holds_item(it))
            .count();
        let want = (loose / 2 + 4).clamp(6, 12);
        let taken = occupied(w, true);
        let seed = [
            (3, 3),
            (-3, 3),
            (3, -3),
            (-3, -3),
            (0, 4),
            (4, 0),
            (5, 5),
            (-5, 5),
        ]
        .into_iter()
        .map(|(dx, dy)| w.grid.surface(home.x + dx, home.y + dy))
        .find(|p| crate::grounds::open_ground(w, *p, &taken, false));
        if let Some(seed) = seed {
            for p in crate::grounds::grow(w, &[seed], want, &taken, false, true) {
                post(
                    w,
                    DesignationKind::BuildStructure(StructureKind::Stockpile, Material::SOIL),
                    p,
                    9,
                );
            }
        }
    }

    // 1b. Expand the stockpile when nearly full.
    if !stock_tiles.is_empty()
        && !has_designation(w, |k| {
            matches!(
                k,
                DesignationKind::BuildStructure(StructureKind::Stockpile, _)
            )
        })
    {
        // Once the wall stands whole the store grows inside it, from the tiles that are inside
        // (#377 5.1a, FIXLOG 2026-11-09: half the stores lay outside the ring, because the pile
        // kept growing from the tiles that stood outside it when the wall was laid).
        let wall = w
            .communities
            .iter()
            .find(|c| c.id == cid)
            .and_then(|c| c.palisade.clone())
            .filter(|pal| pal.closed);
        let inside: Vec<TilePos> = match &wall {
            Some(pal) => stock_tiles
                .iter()
                .copied()
                .filter(|p| palisade_holds(pal, *p))
                .collect(),
            None => Vec::new(),
        };
        let seeds: &[TilePos] = if inside.is_empty() {
            &stock_tiles
        } else {
            &inside
        };
        let free = seeds
            .iter()
            .filter(|p| !w.items.iter().any(|it| it.place == ItemPlace::Ground(**p)))
            .count();
        // The store grows with the people: two tiles a head, forty at least.
        let stock_cap = (people as usize * 2).clamp(40, 160);
        if free < 3 && stock_tiles.len() < stock_cap {
            // Four more tiles beside the pile, where the ground allows (C71: not a row).
            let taken = occupied(w, true);
            let mut grown = crate::grounds::grow(w, seeds, 4, &taken, false, false);
            if grown.is_empty() && !inside.is_empty() {
                // The inside is full: grow outward as before rather than leave stores unhauled.
                grown = crate::grounds::grow(w, &stock_tiles, 4, &taken, false, false);
            }
            for p in grown {
                post(
                    w,
                    DesignationKind::BuildStructure(StructureKind::Stockpile, Material::SOIL),
                    p,
                    5,
                );
            }
        }
    }

    // 2. Haul loose items (not in a stockpile) to the stockpile.
    if !stock_tiles.is_empty() {
        // Not what lies where nobody from home can walk (#403, 2026-11-01: planks given back
        // from a wall order on a rise nobody could climb were ordered hauled, and the haul
        // failed until dropped and was posted again). A hauler walks onto the stack's tile
        // (`work/planner.rs`, Haul).
        w.ensure_nav();
        let out_of_reach = |w: &World, p: TilePos| {
            w.nav
                .as_ref()
                .is_some_and(|n| n.component_of(home).is_some() && !n.reach(home, p))
        };
        // A tool on a finished Shelf's tile is kept there, not loose (#1773, 2026-11-15): a
        // shelf only means something if what is put on it stays put.
        let on_finished_shelf = |w: &World, p: TilePos| {
            w.structures
                .iter()
                .any(|s| s.pos == p && s.kind == StructureKind::Shelf && s.progress >= 1.0)
        };
        let loose: Vec<(crate::ids::ItemId, TilePos)> = w
            .items
            .iter()
            .filter_map(|it| match it.place {
                ItemPlace::Ground(p)
                    if !stock_tiles.contains(&p)
                        && !site_tile(w, p)
                        && !out_of_reach(w, p)
                        && (it.kind != ItemKind::Tool || !on_finished_shelf(w, p)) =>
                {
                    Some((it.id, p))
                }
                _ => None,
            })
            .collect();
        let hauls_open = w
            .designations
            .iter()
            .filter(|d| d.kind == DesignationKind::Haul)
            .count();
        // Hauling scales with hands: one open haul per three people, six at least.
        let hauls_cap = (people as usize / 3).max(6);
        for (id, p) in loose.into_iter().take(hauls_cap.saturating_sub(hauls_open)) {
            if !w
                .designations
                .iter()
                .any(|d| d.kind == DesignationKind::Haul && d.pos == p)
            {
                let mut d = make(w, DesignationKind::Haul, p, 4);
                d.requirements = vec![Requirement {
                    kind: item_kind(w, id),
                    needed: 0,
                    delivered: 0,
                    material: crate::world::Material::AIR,
                    want: None,
                    quality: 0.0,
                }];
                w.designations.push(d);
            }
        }
    }

    // 3. Wood: keep a few logs on hand; fell nearby trees when short.
    let logs = count_kind(w, &here, ItemKind::Log);
    let chain = chain_needs(w, cid, &here);
    let logs_wanted = 6 + wanted_logs_for_sites(w) + chain.logs;
    let chops_open = w
        .designations
        .iter()
        .filter(|d| d.kind == DesignationKind::Chop)
        .count() as u32;
    if logs + chops_open * 3 < logs_wanted {
        let needed = ((logs_wanted - logs) / 3 + 1).min(4);
        let trees = chop_candidates(w, home);
        for p in trees.into_iter().take(needed as usize) {
            post(w, DesignationKind::Chop, p, 6);
        }
    }

    // 4. Stone: quarry the nearest exposed rock face when hearth/well need blocks.
    // Rough or dressed (C67): rough stone is dressed at a bench as the work wants it.
    let blocks =
        count_kind(w, &here, ItemKind::StoneBlock) + count_kind(w, &here, ItemKind::RoughStone);
    let blocks_wanted = wanted_blocks_for_sites(w)
        + if w
            .structures
            .iter()
            .any(|s| s.kind == StructureKind::Hearth && here.holds(s.pos))
        {
            0
        } else {
            4
        };
    let mines_open = w
        .designations
        .iter()
        // Only cuts into stone give blocks (#177: an ore face counted too, so a community short
        // of stone could mine a vein for ever and never get a block).
        .filter(|d| {
            d.kind == DesignationKind::Mine
                && w.grid.materials.get(w.grid.get(d.pos).material).kind
                    == crate::world::material::MaterialKind::Stone
        })
        .count() as u32;
    let shafts_open = w
        .designations
        .iter()
        .filter(|d| d.kind == DesignationKind::DigStairs)
        .count() as u32;
    if blocks + mines_open * 2 < blocks_wanted && shafts_open == 0 {
        // #405: harder stone found down the mine is cut there, not at a face.
        let face = if crate::delving::stone_level(w, cid).is_some() {
            None
        } else {
            nearest_rock_face(w, home, 45)
        };
        if let Some(face) = face {
            post(w, DesignationKind::Mine, face, 6);
        } else {
            quarry(w, home, cid);
        }
    }

    // 5. A hearth near the stockpile once there are blocks or a quarry.
    // Their own hearth (the M4 review: a community that split off counted the old one's,
    // never built one, and thirteen of it froze in a winter).
    if !w
        .structures
        .iter()
        .any(|s| s.kind == StructureKind::Hearth && here.holds(s.pos))
        && !has_designation(w, |k| {
            matches!(k, DesignationKind::BuildStructure(StructureKind::Hearth, _))
        })
    {
        if let Some(spot) = find_flat_area(w, home.offset(-3, -3, 0), 1, 1, 1) {
            post(
                w,
                DesignationKind::BuildStructure(StructureKind::Hearth, Material::STONE_BLOCK),
                spot,
                8,
            );
        }
    }

    // 6. Shelter: one house (7×6, wood) with beds for everyone; beds inside once walls stand.
    // Places to sleep: beds, tents, nests (C57).
    let beds = w
        .structures
        .iter()
        .filter(|s| s.kind.is_sleeping_place() && here.holds(s.pos))
        .count() as u32;
    // A home someone has proposed and nobody has yet found a place for (systems/projects.rs).
    let wanted = crate::project::open_of(w, cid, |k| {
        matches!(k, crate::project::ProjectKind::Home(_))
    })
    .filter(|p| p.site.is_none())
    .map(|p| {
        (
            p.id,
            match p.kind {
                crate::project::ProjectKind::Home(f) => f,
                _ => crate::project::HomeForm::House,
            },
        )
    });
    let walls_posted = has_designation(w, |k| matches!(k, DesignationKind::BuildWall(_)));
    let house_done = house.map(|o| house_complete(w, o)).unwrap_or(false);
    // A new house only once every house is roofed (stage 3 or an old save's 0): before, a new
    // one was marked out the moment the last one's walls stood, before its beds could show
    // whether it was needed, and a village of ten was building its seventh house in a week.
    let all_roofed = w
        .communities
        .iter()
        .find(|c| c.id == cid)
        .map(|c| c.house_stage.iter().all(|s| *s == 0 || *s == 3))
        .unwrap_or(true);
    // #483's first production slice: when one person is genuinely short of a bed and the
    // standing house cannot grow a same-level annex, let that household grow upward. The plan
    // is persisted on the existing house; each stage below is material-backed and waits for the
    // previous one, so this never turns into a prefab dropped on the map (FIXLOG 2026-11-06).
    let upper_sleeping = house
        .filter(|_| house_done && all_roofed)
        .is_some_and(|origin| plan_upper_sleeping(w, cid, origin, home, people, beds));
    // A hollow-building kind digs into the rock if there is a face to dig; without one they
    // dig down into the flat ground (C67); where they cannot do that either they build a
    // house, and the reason says so.
    let wanted = match wanted {
        Some((pid, crate::project::HomeForm::Hollow)) => {
            // Room for those without a place to sleep, dug to the size they need (C67).
            let gap = crate::houses::bed_gap(w, cid);
            let sleepers = people.saturating_sub(beds).max(2);
            let enough = |r: &[TilePos], way_in: TilePos| {
                crate::houses::room_sleeps(r, way_in, sleepers, gap)
            };
            // Too many for any room: no search would find one (#428).
            let site = if !crate::houses::a_room_could_sleep(sleepers, gap) {
                None
            } else {
                find_hollow_site(w, home, 30, &enough)
                    .map(|(m, r)| (m, r, false))
                    .or_else(|| find_dugout_site(w, home, 20, &enough).map(|(m, r)| (m, r, true)))
            };
            match site {
                Some((mouth, room, dug_down)) => {
                    if dug_down {
                        post(w, DesignationKind::DigStairs, mouth, 7);
                        post(w, DesignationKind::DigStairs, room[0], 7);
                        for p in &room[1..] {
                            post(w, DesignationKind::Mine, *p, 7);
                        }
                        if let Some(p) = w.projects.iter_mut().find(|p| p.id == pid) {
                            p.because
                                .push_str(", dug down into the ground for want of a rock face");
                        }
                    } else {
                        for p in std::iter::once(mouth).chain(room.iter().copied()) {
                            post(w, DesignationKind::Mine, p, 7);
                        }
                    }
                    if let Some(c) = w.communities.iter_mut().find(|c| c.id == cid) {
                        c.hollows.push(crate::society::Hollow {
                            mouth,
                            room,
                            shelter: false,
                            begun: w.meta.tick,
                            dug_down,
                        });
                    }
                    set_home_site(w, pid, mouth);
                    None
                }
                None => {
                    if let Some(p) = w.projects.iter_mut().find(|p| p.id == pid) {
                        p.kind = crate::project::ProjectKind::Home(crate::project::HomeForm::House);
                        p.because
                            .push_str(", and no rock face near to dig into nor ground to dig down");
                    }
                    Some((pid, crate::project::HomeForm::House))
                }
            }
        }
        Some((pid, f @ (crate::project::HomeForm::Tent | crate::project::HomeForm::Nest))) => {
            let kind = if f == crate::project::HomeForm::Tent {
                StructureKind::Tent
            } else {
                StructureKind::Nest
            };
            let short = people.saturating_sub(beds).clamp(1, 4);
            let mut placed = 0;
            for _ in 0..short {
                if let Some(p) = find_flat_area(w, home.offset(-6, 4, 0), 1, 1, 0) {
                    post(
                        w,
                        DesignationKind::BuildStructure(kind, Material::WOOD),
                        p,
                        6,
                    );
                    placed += 1;
                }
            }
            if placed > 0 {
                set_home_site(w, pid, home);
            }
            None
        }
        other => other,
    };
    // An upper sleeping plan is the household's response to this shortage; do not mark out a
    // second house for the same need while its existing house is growing upward.
    let want_house =
        matches!(wanted, Some((_, crate::project::HomeForm::House))) && !upper_sleeping;
    // A shelter someone proposed after a raid (C58): a hollow for the stores.
    if let Some(pid) =
        crate::project::open_of(w, cid, |k| *k == crate::project::ProjectKind::Shelter)
            .filter(|p| p.site.is_none())
            .map(|p| p.id)
    {
        let size = shelter_room(people);
        // Into a rock face, or, where there is none near (#405: none in four openings' valleys),
        // down into the ground behind a door at the foot of the stair, as a dug-out home is.
        let site = find_hollow_site(w, home, 30, &|r, _| r.len() >= size)
            .map(|(m, r)| (m, r, false))
            .or_else(|| {
                find_barred_dugout_site(w, home, 20, &|r, _| r.len() > size)
                    .map(|(m, r)| (m, r, true))
            });
        if let Some((mouth, room, dug_down)) = site {
            for (k, p) in std::iter::once(mouth)
                .chain(room.iter().copied())
                .enumerate()
            {
                let kind = if dug_down && k < 2 {
                    DesignationKind::DigStairs
                } else {
                    DesignationKind::Mine
                };
                post(w, kind, p, 7);
            }
            if let Some(c) = w.communities.iter_mut().find(|c| c.id == cid) {
                c.hollows.push(crate::society::Hollow {
                    mouth,
                    room,
                    shelter: true,
                    begun: w.meta.tick,
                    dug_down,
                });
            }
            set_home_site(w, pid, mouth);
        }
    }
    palisade(w, cid, home);
    crate::shrines::plan(w, cid, home);
    crate::wells::plan(w, cid, home);
    crate::channels::plan(w, cid, home);
    crate::water::plan_store(w, cid, home);
    if (house.is_none() || house_done) && all_roofed && !walls_posted && want_house {
        // Grown to the household (C67, houses.rs): room for those who lack a bed, at the
        // spacing their bodies need. A household with a house adds to it if the ground beside
        // it allows; else a new house is grown from the nearest open ground.
        let gap = crate::houses::bed_gap(w, cid);
        let posted_beds = w
            .designations
            .iter()
            .filter(|d| matches!(d.kind, DesignationKind::BuildStructure(k, _) if k.is_sleeping_place()))
            .count() as u32;
        let sleepers = people.saturating_sub(beds + posted_beds).max(2);
        let sites: Vec<TilePos> = w
            .communities
            .iter()
            .find(|c| c.id == cid)
            .map(|c| c.house_sites.clone())
            .unwrap_or_default();
        let annex = sites.iter().rev().find_map(|o| {
            crate::houses::annex(w, *o, home, sleepers, gap).map(|(f, d)| (*o, f, d))
        });
        let grown = if annex.is_some() {
            None
        } else {
            crate::houses::new_house(w, home.offset(-7, -5, 0), home, sleepers, gap)
        };
        // Stone if there is stone enough to hand for the new walls, else timber
        // (house_style). To hand means what this community holds, not blocks anywhere in the
        // valley (FIXLOG, C54).
        let blocks = (count_kind(w, &here, ItemKind::StoneBlock)
            + count_kind(w, &here, ItemKind::RoughStone)) as usize;
        let mut marked: Option<(TilePos, Vec<TilePos>, Vec<TilePos>, &str)> = None;
        if let Some((origin, floor, between)) = annex {
            let mut plan = crate::houses::plan(w, origin);
            plan.floor.extend(floor.iter().copied());
            plan.doors.push(between);
            if let Some(c) = w.communities.iter_mut().find(|c| c.id == cid) {
                if let Some(k) = c.house_sites.iter().position(|o| *o == origin) {
                    c.house_plans
                        .resize(c.house_sites.len(), crate::society::HousePlan::default());
                    c.house_plans[k] = plan;
                    c.house_stage.resize(c.house_sites.len(), 0);
                    c.house_stage[k] = 1;
                }
            }
            marked = Some((origin, floor, vec![between], "added to"));
        } else if let Some(plan) = grown {
            let origin = crate::houses::heart(&plan.floor).unwrap_or(plan.floor[0]);
            let (floor, doors) = (plan.floor.clone(), plan.doors.clone());
            if let Some(c) = w.communities.iter_mut().find(|c| c.id == cid) {
                c.house_plans
                    .resize(c.house_sites.len(), crate::society::HousePlan::default());
                c.house_sites.push(origin);
                c.house_plans.push(plan);
                c.house_stage.resize(c.house_sites.len() - 1, 0);
                c.house_stage.push(1);
            }
            marked = Some((origin, floor, doors, "marked out"));
        }
        if let Some((origin, floor, doors, stage)) = marked {
            let trees: Vec<TilePos> = w
                .plants
                .iter()
                .filter(|pl| pl.alive && pl.kind.is_tree() && floor.contains(&pl.pos))
                .map(|pl| pl.pos)
                .collect();
            for t in trees {
                if !has_designation_at(w, t) {
                    post(w, DesignationKind::Chop, t, 9);
                }
            }
            // The walls the new floor needs that do not already stand: the corners first, so
            // the frame shows; the doors last (a door between rooms goes into the old wall).
            let need: Vec<(TilePos, bool)> = crate::houses::walls(&floor)
                .into_iter()
                .filter(|(q, _)| w.grid.get(*q).shape != Shape::Wall)
                .collect();
            let wall = if blocks >= need.len() {
                Material::STONE_BLOCK
            } else {
                Material::WOOD
            };
            for (q, corner) in need {
                if doors.contains(&q) {
                    continue;
                }
                post(
                    w,
                    DesignationKind::BuildWall(wall),
                    q,
                    if corner { 8 } else { 7 },
                );
            }
            for d in &doors {
                post(w, DesignationKind::BuildDoor, *d, 6);
            }
            let by = best_builder(w, cid);
            w.events.push(
                w.meta.tick,
                Some(origin),
                crate::events::Severity::Notable,
                crate::events::EventKind::HouseStage {
                    community: cid,
                    at: origin,
                    stage: stage.into(),
                    by,
                },
            );
            if let Some((pid, _)) = wanted {
                set_home_site(w, pid, origin);
            }
        }
    }
    roofs_and_stages(w, cid);
    let sites: Vec<TilePos> = w
        .communities
        .iter()
        .find(|c| c.id == cid)
        .map(|c| c.house_sites.clone())
        .unwrap_or_default();
    let gap = crate::houses::bed_gap(w, cid);
    for origin in sites {
        let plan = crate::houses::plan(w, origin);
        if crate::houses::complete(w, &plan)
            && beds < people
            && !(upper_sleeping && Some(origin) == house)
        {
            let spots = crate::houses::layout(&plan, gap).beds;
            // Each finished house takes the beds it has room for while anyone lacks one. (This
            // was `people.min(6) - beds` for the whole settlement, so later houses stood empty
            // and people slept rough; found by the test agent's three-year run, 2026-10-14.)
            let inside = |p: TilePos| plan.floor.contains(&p);
            let posted_all = w
                .designations
                .iter()
                .filter(|d| {
                    matches!(
                        d.kind,
                        DesignationKind::BuildStructure(StructureKind::Bed, _)
                    )
                })
                .count() as u32;
            let here_beds = w
                .structures
                .iter()
                .filter(|s| s.kind == StructureKind::Bed && inside(s.pos))
                .count() as u32;
            let posted_here = w
                .designations
                .iter()
                .filter(|d| {
                    matches!(
                        d.kind,
                        DesignationKind::BuildStructure(StructureKind::Bed, _)
                    ) && inside(d.pos)
                })
                .count() as u32;
            let room = (spots.len() as u32).saturating_sub(here_beds + posted_here);
            let short = people.saturating_sub(beds + posted_all);
            let mut to_post = room.min(short);
            for p in spots {
                if to_post == 0 {
                    break;
                }
                if w.structures.iter().any(|s| s.pos == p)
                    || w.designations.iter().any(|d| d.pos == p)
                {
                    continue;
                }
                post(
                    w,
                    DesignationKind::BuildStructure(StructureKind::Bed, Material::WOOD),
                    p,
                    6,
                );
                to_post -= 1;
            }
        }
    }

    // 6a. Hollows: once the room is dug, a door at the mouth and beds inside (C57).
    let hollows: Vec<crate::society::Hollow> = w
        .communities
        .iter()
        .find(|c| c.id == cid)
        .map(|c| c.hollows.clone())
        .unwrap_or_default();
    for h in hollows {
        if !hollow_dug(w, &h) {
            // A tile whose order was dropped (a failed prop, a fall that refilled it) is ordered
            // again; a hollow still not dug after twenty days is given up, and the home or
            // shelter it was for ends with it (#163).
            if w.meta.tick.saturating_sub(h.begun) > 20 * crate::time::TICKS_PER_DAY {
                // A room being dug further that is not finished keeps what was dug (C67); a
                // hollow never opened is given up.
                let open = |p: &TilePos| w.grid.get(*p).shape != Shape::Wall;
                let kept: Vec<TilePos> = h.room.iter().copied().filter(open).collect();
                let opened = (open(&h.mouth)
                    || w.grid.get(h.mouth).flags.has(crate::world::TileFlags::DOOR))
                    && kept.len() > usize::from(h.dug_down);
                if let Some(c) = w.communities.iter_mut().find(|c| c.id == cid) {
                    if opened {
                        if let Some(x) = c.hollows.iter_mut().find(|x| x.mouth == h.mouth) {
                            x.room = kept;
                        }
                    } else {
                        c.hollows.retain(|x| x.mouth != h.mouth);
                    }
                }
                w.designations.retain(|d| {
                    !(matches!(d.kind, DesignationKind::Mine | DesignationKind::DigStairs)
                        && (d.pos == h.mouth || h.room.contains(&d.pos)))
                });
                continue;
            }
            for (k, p) in std::iter::once(h.mouth)
                .chain(h.room.iter().copied())
                .enumerate()
            {
                let t = w.grid.get(p);
                if t.shape == Shape::Wall
                    && !t.flags.has(crate::world::TileFlags::DOOR)
                    && !w.designations.iter().any(|d| !d.done && d.pos == p)
                {
                    // A dugout's shaft, its mouth and its foot, is dug as stairs.
                    let kind = if h.dug_down && k < 2 {
                        DesignationKind::DigStairs
                    } else {
                        DesignationKind::Mine
                    };
                    post(w, kind, p, 7);
                }
            }
            continue;
        }
        // A dugout's way in is its stair; it has no door, but a shelter dug down has one at
        // the room's second tile (#405).
        if let Some(at) = h.door() {
            if !w.grid.get(at).flags.has(crate::world::TileFlags::DOOR)
                && !w.designations.iter().any(|d| d.pos == at)
            {
                post(w, DesignationKind::BuildDoor, at, 6);
            }
        }
        // A shelter keeps the stores: its room becomes stockpile ground, not a bedroom.
        if h.shelter {
            // Not the stair's foot nor the door of one dug down.
            for p in h.room.iter().skip(if h.dug_down { 2 } else { 0 }) {
                if !w.structures.iter().any(|s| s.pos == *p)
                    && !w.designations.iter().any(|d| d.pos == *p)
                {
                    post(
                        w,
                        DesignationKind::BuildStructure(StructureKind::Stockpile, Material::SOIL),
                        *p,
                        6,
                    );
                }
            }
            continue;
        }
        let posted_all = w
            .designations
            .iter()
            .filter(|d| matches!(d.kind, DesignationKind::BuildStructure(k, _) if k.is_sleeping_place()))
            .count() as u32;
        let mut to_post = people.saturating_sub(beds + posted_all);
        // Beds spaced for their bodies, the far tiles first; the tiles by the way in stay
        // clear (and a dugout's foot of the stair, its first, is never slept on).
        let gap = crate::houses::bed_gap(w, cid);
        let (room, way_in) = if h.dug_down {
            (&h.room[1..], h.room[0])
        } else {
            (&h.room[..], h.mouth)
        };
        let spots = crate::houses::room_beds(room, way_in, gap);
        let free = spots
            .iter()
            .filter(|p| {
                !w.structures.iter().any(|s| s.pos == **p)
                    && !w.designations.iter().any(|d| d.pos == **p)
            })
            .count() as u32;
        // Outgrown: they dig further in for the rest (C67).
        if to_post > free && h.room.len() < crate::houses::MAX_ROOM {
            let short = to_post - free;
            let z = room.first().map(|p| p.z).unwrap_or(h.mouth.z);
            let taken: std::collections::BTreeSet<TilePos> =
                w.designations.iter().map(|d| d.pos).collect();
            let g = &w.grid;
            let ok = |p: TilePos| {
                p.z == z
                    && g.in_bounds(p)
                    && g.get(p).shape == Shape::Wall
                    && !g.get(p).flags.has(crate::world::TileFlags::DOOR)
                    && g.get(p.offset(0, 0, 1)).shape == Shape::Wall
                    && (!h.dug_down || g.ground_z(p.x, p.y) == z + 1)
                    && [(1, 0), (-1, 0), (0, 1), (0, -1)].iter().all(|(dx, dy)| {
                        let q = p.offset(*dx, *dy, 0);
                        room.contains(&q) || !g.walkable(q)
                    })
                    && !taken.contains(&p)
                    && !w.communities.iter().any(|c| {
                        c.hollows
                            .iter()
                            .any(|o| o.mouth == p || o.room.contains(&p))
                    })
            };
            let have = spots.len() as u32 + short;
            let grown = crate::houses::dig_room(room, None, way_in, &ok, &|r| {
                crate::houses::room_beds(r, way_in, gap).len() as u32 >= have
            });
            if let Some(grown) = grown {
                let new: Vec<TilePos> = grown[room.len()..].to_vec();
                for p in &new {
                    post(w, DesignationKind::Mine, *p, 6);
                }
                let tick = w.meta.tick;
                if let Some(c) = w.communities.iter_mut().find(|c| c.id == cid) {
                    if let Some(x) = c.hollows.iter_mut().find(|x| x.mouth == h.mouth) {
                        x.room.extend(new);
                        x.begun = tick;
                    }
                }
            }
        }
        for p in spots {
            if to_post == 0 {
                break;
            }
            if w.structures.iter().any(|s| s.pos == p) || w.designations.iter().any(|d| d.pos == p)
            {
                continue;
            }
            post(
                w,
                DesignationKind::BuildStructure(StructureKind::Bed, Material::WOOD),
                p,
                6,
            );
            to_post -= 1;
        }
    }

    // 6b. Once a house stands: wood floor inside, a table with stools, a workbench, lamps.
    // Every house, the first not yet done at a time (#239: only the first house was furnished;
    // every later house and its rooms stood on bare grass with beds).
    for origin in houses {
        let before = w.designations.len();
        let plan = crate::houses::plan(w, origin);
        if crate::houses::complete(w, &plan) {
            let open_build = w
                .designations
                .iter()
                .filter(|d| {
                    matches!(
                        d.kind,
                        DesignationKind::BuildFloor(_) | DesignationKind::BuildStructure(..)
                    )
                })
                .count();
            if open_build < 4 {
                // Floor: interior tiles whose ground top is not yet wood.
                let mut posted = 0;
                for p in plan.floor.iter().copied() {
                    let below = p.offset(0, 0, -1);
                    if w.grid.get(below).material != Material::WOOD
                        && w.grid.get(below).shape == Shape::Wall
                        && !w.designations.iter().any(|d| d.pos == p)
                        && !w.structures.iter().any(|s| s.pos == p)
                    {
                        post(w, DesignationKind::BuildFloor(Material::WOOD), p, 4);
                        posted += 1;
                        if posted >= 3 {
                            break;
                        }
                    }
                }
                // By the door: the table, its stools, the chest.
                let gap = crate::houses::bed_gap(w, cid);
                let near = crate::houses::layout(&plan, gap).table;
                let furnish = [
                    StructureKind::Table,
                    StructureKind::Stool,
                    StructureKind::Stool,
                    StructureKind::Chest,
                ];
                for (kind, p) in furnish.into_iter().zip(near) {
                    let has = w.structures.iter().any(|s| s.pos == p)
                        || w.designations.iter().any(|d| d.pos == p);
                    if !has && posted < 2 {
                        post(
                            w,
                            DesignationKind::BuildStructure(kind, Material::WOOD),
                            p,
                            4,
                        );
                    }
                }
                // Workbench and lamps outside the door: a lamp each side of the step out, the
                // bench a little way along.
                let door = plan.doors[0];
                let (ox, oy) = [(0, 1), (0, -1), (1, 0), (-1, 0)]
                    .into_iter()
                    .find(|(dx, dy)| plan.floor.contains(&door.offset(-dx, -dy, 0)))
                    .unwrap_or((0, 1));
                let step = door.offset(ox, oy, 0);
                let (px, py) = (-oy, ox);
                let spots = [
                    (StructureKind::Lamp, step.offset(-px, -py, 0)),
                    (StructureKind::Lamp, step.offset(px, py, 0)),
                    (
                        StructureKind::Workbench,
                        step.offset(4 * px + ox, 4 * py + oy, 0),
                    ),
                ];
                for (kind, p) in spots {
                    let has = w.structures.iter().any(|s| s.kind == kind && s.pos == p)
                        || w.designations.iter().any(|d| d.pos == p);
                    let free = |q: TilePos| {
                        !w.structures.iter().any(|s| s.pos == q)
                            && !w.designations.iter().any(|d| d.pos == q)
                            && w.grid.walkable(q)
                            && w.grid.get(q).shape == Shape::Open
                    };
                    // #255 part 2 (FIXLOG 2026-10-28): a flank by the door taken by a room
                    // grown against the door side lost its lamp for good; a lamp belongs by
                    // the door, so a free tile by the wall there takes its place. One lamp a
                    // door: once one stands or is wanted within three tiles, that is it —
                    // otherwise the step, run every hour, would post a fresh one each time
                    // (found as a ring of lamps round the hearth, seed 2).
                    let spot = if has {
                        None
                    } else if w.grid.walkable(p) && w.grid.get(p).shape == Shape::Open {
                        Some(p)
                    } else if kind == StructureKind::Lamp {
                        let wanted =
                            w.structures.iter().any(|s| {
                                s.kind == kind && s.pos.z == door.z && s.pos.dist(door) <= 3
                            }) || w.designations.iter().any(|d| {
                                !d.done
                                    && d.pos.z == door.z
                                    && d.pos.dist(door) <= 3
                                    && matches!(
                                        d.kind,
                                        DesignationKind::BuildStructure(StructureKind::Lamp, _)
                                    )
                            });
                        if wanted {
                            None
                        } else {
                            let ring = crate::houses::walls(&plan.floor);
                            let mut cands: Vec<TilePos> = (door.x - 3..=door.x + 3)
                                .flat_map(|x| {
                                    (door.y - 3..=door.y + 3)
                                        .map(move |y| TilePos::new(x, y, door.z))
                                })
                                .filter(|q| {
                                    *q != door
                                        && *q != step
                                        && !spots[..2].iter().any(|(_, s)| s == q)
                                        && !plan.floor.contains(q)
                                        && free(*q)
                                })
                                .collect();
                            // By the wall first, as the flanking spots were; only if the rooms
                            // leave no wall-side tile, the nearest open tile there is.
                            cands.sort_by_key(|q| {
                                (!ring.iter().any(|(r, _)| r.dist(*q) == 1), q.dist(door), *q)
                            });
                            cands.first().copied()
                        }
                    } else {
                        None
                    };
                    if let Some(q) = spot {
                        post(
                            w,
                            DesignationKind::BuildStructure(kind, Material::WOOD),
                            q,
                            3,
                        );
                    }
                }
            }
        }
        // One house's fittings at a time: the next house waits until nothing was posted here.
        if w.designations.len() > before {
            break;
        }
    }

    // 6c. A cellar under the house: stairs from the floor, a room one level down, stockpile
    // tiles and a lamp there, so stores go below ground.
    // Where a stair was begun before, else where the house's layout leaves room.
    let stair = house.and_then(|origin| {
        let plan = crate::houses::plan(w, origin);
        plan.floor
            .iter()
            .copied()
            .find(|p| {
                matches!(
                    w.grid.get(p.offset(0, 0, -1)).shape,
                    Shape::StairsUp | Shape::StairsDown | Shape::StairsBoth
                )
            })
            .or_else(|| crate::houses::layout(&plan, crate::houses::bed_gap(w, cid)).stair)
    });
    if let (Some(origin), Some(stair_top)) = (house, stair) {
        if house_complete(w, origin) && people >= 6 {
            // Two levels down: through the floor layer, into the rock, so the house floor
            // above the room keeps its support.
            let floor_layer = stair_top.offset(0, 0, -1);
            let ground = stair_top.offset(0, 0, -2);
            let cellar_z = ground.z;
            let tf = w.grid.get(floor_layer);
            let tg = w.grid.get(ground);
            let open_cellar = tf.shape != Shape::Wall && tg.shape != Shape::Wall;
            if tf.shape == Shape::Wall
                && !w.designations.iter().any(|d| d.pos == floor_layer)
                && !w.structures.iter().any(|s| s.pos == stair_top)
            {
                post(w, DesignationKind::DigStairs, floor_layer, 5);
            } else if tf.shape != Shape::Wall
                && tg.shape == Shape::Wall
                && !w.designations.iter().any(|d| d.pos == ground)
            {
                post(w, DesignationKind::DigStairs, ground, 5);
            } else if open_cellar {
                // The room grown to need (C71, D35): a tile for every two stacks of food in
                // the stores, four at least, dug outward from the foot of the stair; the lamp
                // on the first open tile by the foot, stockpiles on the rest.
                let foods = w
                    .items
                    .iter()
                    .filter(|it| it.kind.is_food() && here.holds_item(it))
                    .count();
                let want = (foods / 2 + 2).clamp(4, 24);
                let room: Vec<TilePos> = (-6..=6)
                    .flat_map(|dy| (-6..=6).map(move |dx| (dx, dy)))
                    .map(|(dx, dy)| TilePos::new(ground.x + dx, ground.y + dy, cellar_z))
                    .filter(|p| {
                        *p != ground && w.grid.get(*p).shape == Shape::Open && w.dug.contains(p)
                    })
                    .collect();
                if room.len() < want {
                    // Dig on: rock beside the room (or the foot), nearest the foot first.
                    let mut cands: Vec<TilePos> = Vec::new();
                    for q in std::iter::once(ground).chain(room.iter().copied()) {
                        for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                            let p = q.offset(dx, dy, 0);
                            // Not on into damp stone, where the water stands close (#405).
                            if w.grid.get(p).shape == Shape::Wall
                                && !w.designations.iter().any(|d| d.pos == p)
                                && !cands.contains(&p)
                                && !crate::aquifer::damp(w, p)
                            {
                                cands.push(p);
                            }
                        }
                    }
                    cands.sort_by_key(|p| (p.dist(ground), *p));
                    for p in cands.into_iter().take(3) {
                        post(w, DesignationKind::Mine, p, 5);
                    }
                }
                let mut sorted = room.clone();
                sorted.sort_by_key(|p| (p.dist(ground), *p));
                let lamp_here = w.structures.iter().any(|s| {
                    s.kind == StructureKind::Lamp && s.pos.z == cellar_z && s.pos.dist(ground) <= 6
                }) || w.designations.iter().any(|d| {
                    matches!(
                        d.kind,
                        DesignationKind::BuildStructure(StructureKind::Lamp, _)
                    ) && d.pos.z == cellar_z
                        && d.pos.dist(ground) <= 6
                });
                for (k, p) in sorted.into_iter().enumerate() {
                    if !w.grid.walkable(p) || w.grid.get(p).is_water() {
                        continue;
                    }
                    let has = w.structures.iter().any(|s| s.pos == p)
                        || w.designations.iter().any(|d| d.pos == p);
                    if has {
                        continue;
                    }
                    let kind = if k == 0 && !lamp_here {
                        StructureKind::Lamp
                    } else {
                        StructureKind::Stockpile
                    };
                    post(
                        w,
                        DesignationKind::BuildStructure(kind, Material::SOIL),
                        p,
                        4,
                    );
                }
            }
        }
    }

    building_chain(w, cid, &here, home);

    // 7. Food: a field of grain when stored food is low; sow empty field tiles, harvest ripe ones.
    let food = w
        .items
        .iter()
        .filter(|it| it.kind.is_food() && here.holds_item(it))
        .map(|it| it.count)
        .sum::<u32>();
    // The field's tiles (C71, D35): grown from a seed near home to the mouths there are, six
    // tiles a head at first; an old save's 6 by 4 plots are read into it once.
    let mut field_tiles: Vec<TilePos> = w
        .communities
        .iter()
        .find(|c| c.id == cid)
        .map(|c| c.field_tiles.clone())
        .unwrap_or_default();
    if field_tiles.is_empty() {
        let plots: Vec<TilePos> = field
            .into_iter()
            .chain(
                w.communities
                    .iter()
                    .find(|c| c.id == cid)
                    .map(|c| c.extra_fields.clone())
                    .unwrap_or_default(),
            )
            .collect();
        for o in plots {
            for y in 0..4 {
                for x in 0..6 {
                    field_tiles.push(o.offset(x, y, 0));
                }
            }
        }
    }
    if field_tiles.is_empty() && food < people * 6 {
        let taken = occupied(w, false);
        // The nearest open ground to a spot a little off home, on whatever level it lies
        // (the ground falls and rises; a seed held at home's level found nothing near).
        let seed = (0..30i32)
            .flat_map(|r| {
                (-r..=r).flat_map(move |dy: i32| {
                    (-r..=r)
                        .filter(move |dx: &i32| dx.abs() == r || dy.abs() == r)
                        .map(move |dx| (dx, dy))
                })
            })
            .map(|(dx, dy)| w.grid.surface(home.x + 4 + dx, home.y + 6 + dy))
            .find(|p| crate::grounds::open_ground(w, *p, &taken, true));
        if let Some(seed) = seed {
            let want = (people as usize * 6).clamp(12, 48);
            field_tiles = crate::grounds::grow(w, &[seed], want, &taken, true, true);
            if field_tiles.len() >= 8 {
                if let Some(c) = w.communities.iter_mut().find(|c| c.id == cid) {
                    c.field_site = Some(seed);
                }
            } else {
                field_tiles.clear();
            }
        }
    }
    // More plots as the community grows. A plot of 24 grain plants ripens in five days and
    // feeds about two and a half people, so one plot per three people (the 600-day soak
    // starved forty people in a winter with the old one-per-six cap of five).
    let extra: Vec<TilePos> = w
        .communities
        .iter()
        .find(|c| c.id == cid)
        .map(|c| c.extra_fields.clone())
        .unwrap_or_default();
    let stores = crate::systems::culture::stores_food(w, cid);
    let wanted_plots = (people.div_ceil(3) + if stores { 1 } else { 0 }).clamp(1, 16) as usize;
    // More ground as the mouths grow (C71): six tiles a pass, beside the field.
    let wanted_tiles = wanted_plots * 24;
    if !field_tiles.is_empty() && field_tiles.len() < wanted_tiles {
        let taken = occupied(w, false);
        let more = crate::grounds::grow(w, &field_tiles, 6, &taken, true, false);
        field_tiles.extend(more);
    }
    if let Some(c) = w.communities.iter_mut().find(|c| c.id == cid) {
        c.field_tiles = field_tiles.clone();
    }
    let _ = &extra;
    // Sow by the store: with plenty in hand, fewer plots are sown (the rest lie fallow until the
    // store runs down); a harvest nobody needs only lies in the field and rots. The counts are
    // portions, not days: forty portions a mouth is about four days' eating.
    let mouths = people.max(1);
    let sow_plots = if food > mouths * 40 {
        1
    } else if food > mouths * 20 {
        wanted_plots.div_ceil(2)
    } else {
        usize::MAX
    };
    let sow_tiles = sow_plots.saturating_mul(24);
    for (n, p) in field_tiles.iter().copied().enumerate() {
        let sow = n < sow_tiles;
        {
            {
                if w.designations.iter().any(|d| d.pos == p) {
                    continue;
                }
                match w.plants.iter().find(|pl| pl.pos == p && pl.alive) {
                    None if sow => post(w, DesignationKind::Sow, p, 3),
                    None => {}
                    Some(pl) if pl.kind.is_tree() => post(w, DesignationKind::Chop, p, 4),
                    Some(pl) if pl.kind.is_weed() && pl.mature() && weed_presses(w, p) => {
                        // A weed pressing a neighbouring crop is pulled, not sown over
                        // (#286); on a tile being sown the weed is cleared as before.
                        post(w, DesignationKind::Uproot, p, 6)
                    }
                    Some(pl) if !pl.kind.is_crop() && sow => {
                        // Clear shrubs and the like by sowing over them.
                        let _ = pl;
                        w.plants.retain(|q| q.pos != p);
                        post(w, DesignationKind::Sow, p, 3);
                    }
                    Some(pl) if pl.kind.is_crop() && pl.mature() => {
                        post(w, DesignationKind::Harvest, p, 5)
                    }
                    _ => {}
                }
            }
        }
    }

    // 7b. Hunting: when food is short and game is about, send someone after it.
    let hunts_open = w
        .designations
        .iter()
        .filter(|d| matches!(d.kind, DesignationKind::Hunt(_)))
        .count();
    // One hunting party per six people when food runs short, each after its own animal.
    // From autumn the larder must carry five dark days, so hunting starts earlier.
    let season_now = crate::time::Calendar::from_tick(w.meta.tick).season;
    let hunt_below = if matches!(
        season_now,
        crate::time::Season::Autumn | crate::time::Season::Winter
    ) {
        people * 20
    } else {
        people * 8
    };
    let hunts_wanted = if food < hunt_below {
        (people / 6).max(1) as usize
    } else {
        0
    };
    if hunts_open < hunts_wanted {
        let hunted: Vec<crate::ids::AgentId> = w
            .designations
            .iter()
            .filter_map(|d| match d.kind {
                DesignationKind::Hunt(id) => Some(id),
                _ => None,
            })
            .collect();
        let mut prey: Vec<(i32, crate::ids::AgentId, TilePos)> = w
            .agents
            .iter()
            .filter(|a| {
                a.alive
                    && matches!(
                        a.species,
                        crate::agents::Species::Deer
                            | crate::agents::Species::Boar
                            | crate::agents::Species::Hare
                            | crate::agents::Species::Goat
                    )
                    && a.pos.dist(home) <= 60
                    && !hunted.contains(&a.id)
                    && a.kept.is_none()
            })
            .map(|a| (a.pos.dist(home), a.id, a.pos))
            .collect();
        prey.sort();
        // Game that is scarce is left to breed: no hunt when fewer than ten animals are about.
        let scarce = prey.len() + hunted.len() < 10;
        let take = if scarce { 0 } else { hunts_wanted - hunts_open };
        for (_, id, pos) in prey.into_iter().take(take) {
            let mut d = make(w, DesignationKind::Hunt(id), pos, 8);
            d.requirements = vec![];
            w.designations.push(d);
        }
    }

    // 7c. Fishing (#299): when food runs short and there is water near with fish in it, one
    // goes to the bank.
    let fishing_open = w
        .designations
        .iter()
        .any(|d| matches!(d.kind, DesignationKind::Fish));
    if food < hunt_below && !fishing_open {
        // A cavern's pool is only a spot if the way down can be walked (fish::cave_spot).
        w.ensure_nav();
        if let Some(spot) = crate::fish::spot(w, home) {
            let mut d = make(w, DesignationKind::Fish, spot, 7);
            d.requirements = vec![];
            w.designations.push(d);
        }
    }

    // 7d. Snares (#316): when food runs short, up to three set where small game runs; after
    // strangers have raided, one across the way they came.
    let snares_open = w
        .designations
        .iter()
        .filter(|d| matches!(d.kind, DesignationKind::Snare))
        .count();
    if snares_open == 0 && w.traps.len() < 3 {
        if let Some(spot) = crate::traps::spot(w, home, food < hunt_below) {
            let mut d = make(w, DesignationKind::Snare, spot, 6);
            d.requirements = vec![];
            w.designations.push(d);
        }
    }

    // 7e. The herd (#285): tame goats while it is small; kill one for meat when food runs short.
    crate::livestock::plan(w, home, food < hunt_below);

    // 8. Cooking: when a hearth stands and raw food is on hand, cook meals.
    let hearth = w
        .structures
        .iter()
        .find(|s| s.kind == StructureKind::Hearth && s.progress >= 1.0 && here.holds(s.pos))
        .map(|s| s.pos);
    let raw = w
        .items
        .iter()
        .filter(|it| it.kind.is_cookable() && here.holds_item(it))
        .map(|it| it.count)
        .sum::<u32>();
    let meals = count_kind(w, &here, ItemKind::Meal);
    if let Some(hp) = hearth {
        let cooks_open = w
            .designations
            .iter()
            .filter(|d| d.kind == DesignationKind::Cook)
            .count() as u32;
        // Winter grows nothing: from autumn on, cook for the five dark days ahead.
        let season = crate::time::Calendar::from_tick(w.meta.tick).season;
        let winter_ahead = matches!(
            season,
            crate::time::Season::Autumn | crate::time::Season::Winter
        );
        let meal_target = match (winter_ahead, stores) {
            (true, _) => people * 20,
            (false, true) => people * 10,
            (false, false) => people * 6,
        };
        let cooks_wanted = (people / 6).max(2);
        if raw >= 3 && meals < meal_target && cooks_open < cooks_wanted {
            let mut d = make(w, DesignationKind::Cook, hp, 7);
            d.requirements = vec![];
            w.designations.push(d);
        }

        // 8a'. A pot for the fire: a hearth with a Cook order open
        // or freshly posted this pass stands in for "cooked at lately" (no new state needed).
        // If it has no CookingPot, finished or ordered, within a tile, and a Vessel is in
        // store, post a clay pot costing that vessel.
        let cooking = cooks_open > 0
            || w.designations
                .iter()
                .any(|d| d.kind == DesignationKind::Cook && d.pos == hp);
        if cooking {
            let pot_near = w.structures.iter().any(|s| {
                s.kind == StructureKind::CookingPot && here.holds(s.pos) && s.pos.dist(hp) <= 1
            }) || w.designations.iter().any(|d| {
                matches!(
                    d.kind,
                    DesignationKind::BuildStructure(StructureKind::CookingPot, _)
                ) && d.pos.dist(hp) <= 1
            });
            if !pot_near && count_kind(w, &here, ItemKind::Vessel) > 0 {
                // The structure's only cost is the vessel (`structure_cost`), so it is a clay
                // pot whatever the vessel was made of; no other store item is consumed.
                let mat = Material::CLAY;
                let blocked = |p: TilePos| {
                    w.structures.iter().any(|s| s.pos == p)
                        || w.designations.iter().any(|d| d.pos == p)
                };
                let spot = [(1, 0), (-1, 0), (0, 1), (0, -1)]
                    .into_iter()
                    .map(|(dx, dy)| hp.offset(dx, dy, 0))
                    .find(|p| w.grid.walkable(*p) && !blocked(*p));
                if let Some(spot) = spot {
                    let d = make(
                        w,
                        DesignationKind::BuildStructure(StructureKind::CookingPot, mat),
                        spot,
                        7,
                    );
                    w.designations.push(d);
                }
            }
        }
    }

    // 8a. Fires are met by whoever is there and knows what to do (decide.rs, techniques).

    // 8b. The sick and the hurt are tended by whoever knows how and chooses to (decide.rs),
    //     not by an order from the settlement.

    // A carving (C35) is cut when a carver among them has proposed one (systems/projects.rs,
    // which keeps the five days between carvings and three to a house); the planner finds
    // the wall.
    let carve = crate::project::open_of(w, cid, |k| *k == crate::project::ProjectKind::Carve)
        .is_some_and(|p| p.work.is_none());
    if carve && !has_designation(w, |k| *k == DesignationKind::Engrave) {
        if let Some(p) = crate::engrave::wall_to_carve(w, home, 40) {
            let d = make(w, DesignationKind::Engrave, p, 2);
            let id = d.id;
            w.designations.push(d);
            if let Some(p) =
                crate::project::open_of_mut(w, cid, |k| *k == crate::project::ProjectKind::Carve)
            {
                p.work = Some(id);
            }
        }
    }

    // 9. Workshops and recipes: a carpenter and a loom once there are eight people, a smelter
    //    when ore is on hand, an anvil when bars are; orders posted from need, one per recipe.
    if people >= 8 {
        // By the first house, or by home until one stands (#242: a community that had just
        // moved had no house, so its loom could not be sited and was proposed every five days).
        if let Some(origin) = house.or(Some(home)) {
            let spots = [
                (
                    StructureKind::CarpenterBench,
                    Material::WOOD,
                    origin.offset(-3, 0, 0),
                    0u32,
                ),
                (
                    StructureKind::Loom,
                    Material::WOOD,
                    origin.offset(-3, 3, 0),
                    0,
                ),
                (
                    StructureKind::Smelter,
                    Material::STONE_BLOCK,
                    origin.offset(9, 0, 0),
                    1,
                ),
                (
                    StructureKind::Anvil,
                    Material::STONE_BLOCK,
                    origin.offset(9, 3, 0),
                    2,
                ),
                (
                    StructureKind::Kiln,
                    Material::STONE_BLOCK,
                    origin.offset(9, 6, 0),
                    3,
                ),
            ];
            // A workshop is built when someone with its craft wants it (systems/projects.rs);
            // the planner finds the spot.
            for (kind, mat, near, _) in spots {
                let wanted = crate::project::open_of(w, cid, |k| {
                    *k == crate::project::ProjectKind::Workshop(kind)
                })
                .is_some_and(|p| p.work.is_none());
                if !wanted
                    || w.structures
                        .iter()
                        .any(|s| s.kind == kind && here.holds(s.pos))
                {
                    continue;
                }
                // A previous project can leave its community's build order behind when its
                // proposer dies or leaves. Adopt that orphan order instead of letting this
                // project time out and re-propose the same workshop every five days (#468).
                if let Some(id) = w
                    .designations
                    .iter()
                    .find(|d| {
                        !d.done
                            && d.community == Some(cid)
                            && matches!(
                                d.kind,
                                DesignationKind::BuildStructure(x, _) if x == kind
                            )
                    })
                    .map(|d| d.id)
                {
                    if let Some(p) = crate::project::open_of_mut(w, cid, |k| {
                        *k == crate::project::ProjectKind::Workshop(kind)
                    }) {
                        p.work = Some(id);
                    }
                    continue;
                }
                // On their own ground: a workshop nearer another community's home is not
                // theirs, and they would want it again (C60). By the first house first, then
                // round home (#242: after a move the first house stood by the map's edge, on
                // nobody's ground, and a loom was proposed every five days for sixty).
                let ground_spot = std::iter::once(near)
                    .chain(
                        [
                            (-4, 4),
                            (4, 4),
                            (-4, -4),
                            (4, -4),
                            (-7, 0),
                            (7, 0),
                            (0, 7),
                            (0, -7),
                        ]
                        .into_iter()
                        .map(|(dx, dy)| home.offset(dx, dy, 0)),
                    )
                    .find_map(|n| find_flat_area(w, n, 1, 1, 1).filter(|p| here.holds(*p)));
                // An existing upper room is a workspace only after the ordinary same-level
                // search fails. It is already a real, roofed, supported room grown tile by tile
                // by #483; this fallback does not create a tower or bypass the material chain.
                let spot = ground_spot.or_else(|| upper_workspace_spot(w, origin, &here));
                if let Some(spot) = spot {
                    let d = make(w, DesignationKind::BuildStructure(kind, mat), spot, 5);
                    let id = d.id;
                    w.designations.push(d);
                    if let Some(p) = crate::project::open_of_mut(w, cid, |k| {
                        *k == crate::project::ProjectKind::Workshop(kind)
                    }) {
                        p.work = Some(id);
                    }
                } else {
                    // No place for it: said once, and not wanted again for a season.
                    let by = crate::project::open_of(w, cid, |k| {
                        *k == crate::project::ProjectKind::Workshop(kind)
                    })
                    .map(|p| p.by);
                    if let Some(c) = w.communities.iter_mut().find(|c| c.id == cid) {
                        c.no_site.retain(|(k, _)| *k != kind);
                        c.no_site
                            .push((kind, w.meta.tick + 60 * crate::time::TICKS_PER_DAY));
                    }
                    if let Some(p) = w.projects.iter_mut().find(|p| {
                        p.community == cid
                            && !p.done
                            && p.kind == crate::project::ProjectKind::Workshop(kind)
                    }) {
                        p.done = true;
                    }
                    if let Some(by) = by {
                        let tick = w.meta.tick;
                        w.events.push(
                            tick,
                            Some(home),
                            crate::events::Severity::Notable,
                            crate::events::EventKind::Chose {
                                agent: by,
                                what: format!(
                                    "gave up on the {}",
                                    format!("{kind:?}").to_lowercase()
                                ),
                                because: "found no place for it on their own ground".into(),
                            },
                        );
                    }
                }
            }
        }
        use crate::work::recipes as rc;
        let stock = |w: &World, k: ItemKind| count_kind(w, &here, k);
        // Tools in reach of home (as work.rs take_tool looks): a relic in a far ruin is no
        // spare tool for anyone here.
        let tools_here: u32 = w
            .items
            .iter()
            .filter(|it| {
                it.kind == ItemKind::Tool
                    && matches!(it.place, ItemPlace::Ground(p) if p.dist(home) <= 45)
            })
            .map(|it| it.count)
            .sum();
        // Of a material: tin ore, bronze bars (C33).
        let of = |w: &World, k: ItemKind, m: Material| count_of(w, &here, k, m);
        // Arms are wanted only once strangers have threatened these people (systems/arms.rs),
        // a spear for about every third grown person, armour for every sixth.
        let threatened = w
            .communities
            .iter()
            .find(|c| c.id == cid)
            .map(|c| {
                c.threatened > 0
                    && w.meta.tick.saturating_sub(c.threatened) < 40 * crate::time::TICKS_PER_DAY
            })
            .unwrap_or(false);
        let arms = |w: &World, k: ItemKind| -> u32 {
            w.items
                .iter()
                .filter(|it| {
                    it.kind == k
                        && match it.place {
                            ItemPlace::Ground(p) => p.dist(home) <= 45,
                            ItemPlace::Held(a) => w
                                .agents
                                .iter()
                                .any(|x| x.id == a && x.community == Some(cid)),
                            _ => false,
                        }
                })
                .count() as u32
        };
        let want_spears = threatened && arms(w, ItemKind::Weapon) < people / 3 + 1;
        let want_armour = threatened && arms(w, ItemKind::Armour) < people / 6 + 1;
        let iron = stock(w, ItemKind::IronBar);
        let bronze = of(w, ItemKind::Bar, Material::BRONZE);
        let planks = stock(w, ItemKind::Plank);
        let cloth = stock(w, ItemKind::Cloth);
        let coal = stock(w, ItemKind::Coal);
        // Ale and wine are made while someone who holds with drink has proposed brewing from a
        // full store (systems/projects.rs: ten days' eating a mouth, grain or berries to spare).
        let brewing =
            crate::project::open_of(w, cid, |k| *k == crate::project::ProjectKind::Brew).is_some();
        let drink = stock(w, ItemKind::Ale) + stock(w, ItemKind::Wine);
        let vessels = |w: &World, m: Material| count_of(w, &here, ItemKind::Vessel, m);
        let rings = stock(w, ItemKind::Jewel);
        let gems = stock(w, ItemKind::Gem);
        let wants: [(u8, bool); 28] = [
            (
                rc::POULTICE,
                stock(w, ItemKind::Root) >= 2 && stock(w, ItemKind::Poultice) < 3,
            ),
            (
                rc::PLANK,
                stock(w, ItemKind::Log) >= 4 && stock(w, ItemKind::Plank) < 10,
            ),
            (
                rc::IRON_BAR,
                stock(w, ItemKind::IronOre) >= 2
                    && stock(w, ItemKind::Coal) >= 1
                    && stock(w, ItemKind::IronBar) < 6,
            ),
            (
                rc::COPPER_BAR,
                stock(w, ItemKind::CopperOre) >= 2
                    && stock(w, ItemKind::Coal) >= 1
                    && stock(w, ItemKind::CopperBar) < 6,
            ),
            (
                rc::IRON_TOOL,
                stock(w, ItemKind::IronBar) >= 1
                    && stock(w, ItemKind::Plank) >= 1
                    && tools_here < people / 4 + 1,
            ),
            (
                rc::COPPER_TOOL,
                stock(w, ItemKind::IronBar) == 0
                    && of(w, ItemKind::Bar, Material::BRONZE) == 0
                    && stock(w, ItemKind::CopperBar) >= 1
                    && stock(w, ItemKind::Plank) >= 1
                    && tools_here < people / 4 + 1,
            ),
            (
                // Before there is metal: a knapped edge on a haft, made at the workbench.
                rc::STONE_TOOL,
                stock(w, ItemKind::IronBar) == 0
                    && stock(w, ItemKind::CopperBar) == 0
                    && stock(w, ItemKind::RoughStone) >= 1
                    && stock(w, ItemKind::Log) >= 2
                    && tools_here < people / 4 + 1,
            ),
            (
                rc::CLOTH,
                stock(w, ItemKind::Fibre) >= 3 && stock(w, ItemKind::Cloth) < 6,
            ),
            (
                rc::CLOTHES,
                stock(w, ItemKind::Cloth) >= 2
                    && stock(w, ItemKind::Clothes) < people / 5 + 1
                    && stock(w, ItemKind::Dye) == 0,
            ),
            // With dyestuff in, the loom dyes the clothes it makes (#286).
            (
                rc::DYED_CLOTHES,
                stock(w, ItemKind::Cloth) >= 2
                    && stock(w, ItemKind::Clothes) < people / 5 + 1
                    && stock(w, ItemKind::Dye) >= 1,
            ),
            (
                rc::TIN_BAR,
                of(w, ItemKind::Ore, Material::TIN_ORE) >= 2
                    && coal >= 1
                    && of(w, ItemKind::Bar, Material::TIN) < 4,
            ),
            (
                rc::SILVER_BAR,
                of(w, ItemKind::Ore, Material::SILVER_ORE) >= 2
                    && coal >= 1
                    && of(w, ItemKind::Bar, Material::SILVER) < 4,
            ),
            (
                rc::GOLD_BAR,
                of(w, ItemKind::Ore, Material::GOLD_ORE) >= 2
                    && coal >= 1
                    && of(w, ItemKind::Bar, Material::GOLD) < 4,
            ),
            (
                rc::BRONZE,
                stock(w, ItemKind::CopperBar) >= 2
                    && of(w, ItemKind::Bar, Material::TIN) >= 1
                    && coal >= 1
                    && bronze < 6,
            ),
            (rc::IRON_WEAPON, want_spears && iron >= 1 && planks >= 1),
            (
                rc::BRONZE_WEAPON,
                want_spears && iron == 0 && bronze >= 1 && planks >= 1,
            ),
            (
                rc::COPPER_WEAPON,
                want_spears
                    && iron == 0
                    && bronze == 0
                    && stock(w, ItemKind::CopperBar) >= 1
                    && planks >= 1,
            ),
            (rc::IRON_ARMOUR, want_armour && iron >= 2 && cloth >= 1),
            (
                rc::BRONZE_ARMOUR,
                want_armour && iron < 2 && bronze >= 2 && cloth >= 1,
            ),
            (
                rc::BRONZE_TOOL,
                iron == 0 && bronze >= 1 && planks >= 1 && tools_here < people / 4 + 1,
            ),
            (
                rc::POT,
                stock(w, ItemKind::Clay) >= 2
                    && stock(w, ItemKind::Log) >= 3
                    && vessels(w, Material::CLAY) < people / 3 + 2,
            ),
            (
                rc::GLASS,
                stock(w, ItemKind::Sand) >= 3 && coal >= 1 && vessels(w, Material::GLASS) < 3,
            ),
            (
                rc::LEATHER,
                stock(w, ItemKind::Hide) >= 1 && stock(w, ItemKind::Leather) < 8,
            ),
            (
                rc::LEATHER_CLOTHES,
                stock(w, ItemKind::Leather) >= 2 && stock(w, ItemKind::Clothes) < people / 5 + 1,
            ),
            (
                rc::ALE,
                brewing && stock(w, ItemKind::GrainSheaf) >= 8 && drink < 6,
            ),
            (
                rc::WINE,
                brewing && stock(w, ItemKind::Berries) >= 10 && drink < 6,
            ),
            (
                rc::SILVER_RING,
                gems >= 1 && of(w, ItemKind::Bar, Material::SILVER) >= 1 && rings < 2,
            ),
            (
                rc::GOLD_RING,
                gems >= 1 && of(w, ItemKind::Bar, Material::GOLD) >= 1 && rings < 2,
            ),
        ];
        for (id, want) in wants {
            if !want || has_designation(w, |k| *k == DesignationKind::Craft(id)) {
                continue;
            }
            let station = rc::recipe(id).station;
            let at = w
                .structures
                .iter()
                .find(|s| s.kind == station && s.progress >= 1.0 && here.holds(s.pos))
                .map(|s| s.pos);
            if let Some(at) = at {
                let mut d = make(w, DesignationKind::Craft(id), at, 6);
                d.community = Some(cid);
                w.designations.push(d);
            }
        }
        // Wild plants cut for what they give (#286): fibre from reeds and nettles when the loom
        // has nothing to weave, dyestuff when it has cloth and no dye, and healing herbs while
        // someone of these people knows what to do with them and poultices run short.
        let loom = w
            .structures
            .iter()
            .any(|s| s.kind == StructureKind::Loom && here.holds(s.pos));
        let herb_wise = w.agents.iter().any(|a| {
            a.alive
                && a.community == Some(cid)
                && (a.techniques.knows(crate::mind::Technique::HerbLore)
                    || a.techniques.knows(crate::mind::Technique::Poultice))
        });
        type Cut = (fn(crate::plants::Gives) -> bool, bool);
        let wild: [Cut; 3] = [
            (
                |g| matches!(g, crate::plants::Gives::Fibre(_)),
                loom && stock(w, ItemKind::Fibre) < 6,
            ),
            (
                |g| matches!(g, crate::plants::Gives::Dye(_)),
                loom && stock(w, ItemKind::Dye) < 2,
            ),
            (
                |g| matches!(g, crate::plants::Gives::Medicine),
                herb_wise && stock(w, ItemKind::Poultice) < 3,
            ),
        ];
        for (gives, wanted) in wild {
            if !wanted {
                continue;
            }
            let of_it = |p: &crate::plants::Plant| !p.kind.is_crop() && gives(p.kind.lore().gives);
            let open = w
                .designations
                .iter()
                .filter(|d| {
                    d.kind == DesignationKind::Harvest
                        && w.plants.iter().any(|p| p.pos == d.pos && of_it(p))
                })
                .count();
            if open < 2 {
                let cut: Vec<TilePos> = w
                    .plants
                    .iter()
                    .filter(|p| p.alive && of_it(p) && p.pos.dist(home) <= 50)
                    .map(|p| p.pos)
                    .filter(|p| !w.designations.iter().any(|d| d.pos == *p))
                    .take(2 - open)
                    .collect();
                for p in cut {
                    post(w, DesignationKind::Harvest, p, 3);
                }
            }
        }
        // Ore: prospect an exposed vein within reach when the store has little (the smelter
        // is built once ore is on hand, so this comes first).
        if stock(w, ItemKind::IronOre) + stock(w, ItemKind::CopperOre) < 4 {
            let open_ore = w
                .designations
                .iter()
                .filter(|d| {
                    d.kind == DesignationKind::Mine
                        && matches!(
                            w.grid.get(d.pos).material,
                            Material::IRON_ORE | Material::COPPER_ORE | Material::COAL
                        )
                })
                .count();
            // Coal only while coal is short too (2026-10-28 round-16-deep: a coal seam open in
            // the mine's shaft was cut a thousand times in 200 days, the ore never counted).
            // And only with something to burn it in: a smelter or a kiln of theirs standing.
            let burns = w.structures.iter().any(|s| {
                matches!(s.kind, StructureKind::Smelter | StructureKind::Kiln)
                    && s.progress >= 1.0
                    && here.holds(s.pos)
            });
            let mats: &[Material] = if burns && stock(w, ItemKind::Coal) < 4 {
                &[Material::IRON_ORE, Material::COPPER_ORE, Material::COAL]
            } else {
                &[Material::IRON_ORE, Material::COPPER_ORE]
            };
            if open_ore < 2 {
                if let Some(p) = nearest_face_of(w, home, 50, mats) {
                    post(w, DesignationKind::Mine, p, 5);
                }
            }
        }
        // Tin once there is copper to make bronze with, silver or gold once there is a gem to
        // set (C34): an exposed vein of what is wanted, one at a time.
        let tin_wanted = stock(w, ItemKind::CopperBar) >= 2
            && count_of(w, &here, ItemKind::Ore, Material::TIN_ORE)
                + count_of(w, &here, ItemKind::Bar, Material::TIN)
                < 2;
        let precious_wanted = stock(w, ItemKind::Gem) >= 1
            && count_of(w, &here, ItemKind::Bar, Material::SILVER)
                + count_of(w, &here, ItemKind::Bar, Material::GOLD)
                + count_of(w, &here, ItemKind::Ore, Material::SILVER_ORE)
                + count_of(w, &here, ItemKind::Ore, Material::GOLD_ORE)
                == 0;
        for (wanted, mats) in [
            (tin_wanted, &[Material::TIN_ORE][..]),
            (
                precious_wanted,
                &[Material::SILVER_ORE, Material::GOLD_ORE][..],
            ),
        ] {
            let open = w.designations.iter().any(|d| {
                d.kind == DesignationKind::Mine && mats.contains(&w.grid.get(d.pos).material)
            });
            if wanted && !open {
                if let Some(p) = nearest_face_of(w, home, 50, mats) {
                    post(w, DesignationKind::Mine, p, 5);
                }
            }
        }
        // Clay from a wet bank, and sand once the kiln has coal to melt it (C34).
        let kiln = w
            .structures
            .iter()
            .any(|s| s.kind == StructureKind::Kiln && s.progress >= 1.0);
        for (wanted, grounds) in [
            (
                stock(w, ItemKind::Clay) < 6,
                &[Material::WET_SOIL, Material::MUD, Material::CLAY][..],
            ),
            (
                kiln && stock(w, ItemKind::Sand) < 6 && stock(w, ItemKind::Coal) >= 1,
                &[Material::SAND][..],
            ),
        ] {
            let open = w.designations.iter().any(|d| {
                d.kind == DesignationKind::DigEarth
                    && grounds.contains(&w.grid.get(d.pos.offset(0, 0, -1)).material)
            });
            if wanted && !open {
                if let Some(p) = nearest_ground_of(w, home, 40, grounds) {
                    post(w, DesignationKind::DigEarth, p, 3);
                }
            }
        }
    }
    // 10. The mine (#405): the shaft sunk deeper when what is wanted lies below.
    crate::delving::plan(w, cid, home, &here, people);
}

/// The nearest open, reachable surface tile within `radius` whose ground is one of `mats`, with
/// nothing ordered on it (a clay bank, a sand bar).
fn nearest_ground_of(w: &World, near: TilePos, radius: i32, mats: &[Material]) -> Option<TilePos> {
    let g = &w.grid;
    let nav = w.nav.as_ref()?;
    let mut best: Option<(i32, TilePos)> = None;
    for y in (near.y - radius).max(1)..(near.y + radius).min(g.height - 1) {
        for x in (near.x - radius).max(1)..(near.x + radius).min(g.width - 1) {
            let p = g.surface(x, y);
            if !g.walkable(p) || !mats.contains(&g.get(p.offset(0, 0, -1)).material) {
                continue;
            }
            let d = near.dist(p);
            if best.is_some_and(|b| d >= b.0) || w.designations.iter().any(|q| q.pos == p) {
                continue;
            }
            if nav.reach(near, p) {
                best = Some((d, p));
            }
        }
    }
    best.map(|b| b.1)
}

/// A well or a water tile within `radius` of `p` that can be stood next to.
pub fn water_source_near(w: &World, p: TilePos, radius: i32) -> Option<TilePos> {
    let g = &w.grid;
    let mut best: Option<(i32, TilePos)> = None;
    for s in w
        .structures
        .iter()
        .filter(|s| s.kind == StructureKind::Well && s.progress >= 1.0)
    {
        let d = s.pos.dist(p);
        if d <= radius && best.map(|b| d < b.0).unwrap_or(true) {
            best = Some((d, s.pos));
        }
    }
    let r = radius.min(30);
    for y in (p.y - r).max(0)..(p.y + r).min(g.height) {
        for x in (p.x - r).max(0)..(p.x + r).min(g.width) {
            // Water lies at the surface of its column, often a level below the grass around
            // it; looking only on the fire's own level found no lake beside a lakeside fire.
            let q = g.surface(x, y);
            if (q.z - p.z).abs() > 2 {
                continue;
            }
            let t = g.get(q);
            if t.is_water() {
                let d = q.dist(p);
                if best.map(|b| d < b.0).unwrap_or(true) {
                    best = Some((d, q));
                }
            }
        }
    }
    best.map(|b| b.1)
}

/// Nearest solid tile of one of `mats` with a walkable, reachable neighbour (an exposed
/// face), within `radius` of `near`, on any level.
pub fn nearest_face_of(
    w: &World,
    near: TilePos,
    radius: i32,
    mats: &[Material],
) -> Option<TilePos> {
    let g = &w.grid;
    let nav = w.nav.as_ref()?;
    let mut best: Option<(i32, TilePos)> = None;
    for z in g.z_min..=g.z_max {
        for y in (near.y - radius).max(1)..(near.y + radius).min(g.height - 1) {
            for x in (near.x - radius).max(1)..(near.x + radius).min(g.width - 1) {
                let p = TilePos::new(x, y, z);
                let t = g.get(p);
                if t.shape != Shape::Wall || !mats.contains(&t.material) {
                    continue;
                }
                if w.designations.iter().any(|d| d.pos == p) {
                    continue;
                }
                let open = [(1, 0), (-1, 0), (0, 1), (0, -1)]
                    .iter()
                    .map(|(dx, dy)| p.offset(*dx, *dy, 0))
                    .any(|q| g.walkable(q) && nav.reach(near, q));
                if !open {
                    continue;
                }
                let d = near.dist(p) + (near.z - z).abs() * 6;
                if best.map(|b| d < b.0).unwrap_or(true) {
                    best = Some((d, p));
                }
            }
        }
    }
    best.map(|b| b.1)
}

fn has_designation_at(w: &World, p: TilePos) -> bool {
    w.designations.iter().any(|d| !d.done && d.pos == p)
}

/// The member with the surest hand at building, to name in the feed.
fn best_builder(w: &World, cid: crate::ids::CommunityId) -> Option<crate::ids::AgentId> {
    w.agents
        .iter()
        .filter(|a| a.alive && a.community == Some(cid))
        .max_by(|a, b| {
            a.skills
                .level(crate::mind::Skill::Construction)
                .partial_cmp(&b.skills.level(crate::mind::Skill::Construction))
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(b.id.cmp(&a.id))
        })
        .map(|a| a.id)
}

/// The roof a house gets: shingles on a stone house; on a timber house thatch or shingles, by
/// what this community's builders favour (the same for every house it builds, so a settlement
/// has a look of its own).
pub fn roof_style(w: &World, cid: crate::ids::CommunityId, wall: Material) -> Material {
    if wall == Material::STONE_BLOCK {
        return Material::WOOD;
    }
    // Thatch is bundled from reeds (C67): none where there are none to cut.
    let reeds = w.communities.iter().find(|c| c.id == cid).is_some_and(|c| {
        w.plants
            .iter()
            .any(|p| p.alive && p.kind == PlantKind::Reed && p.pos.dist(c.home) <= 50)
            || w.items.iter().any(|it| {
                matches!(it.kind, ItemKind::Thatch | ItemKind::Fibre)
                    && matches!(it.place, ItemPlace::Ground(p) if p.dist(c.home) <= 45)
            })
    });
    // Their way of building, once their first house has set it (C58).
    if let Some((_, roof)) = building_way(w, cid) {
        return if roof == Material::THATCH && !reeds {
            Material::WOOD
        } else {
            roof
        };
    }
    if reeds && crate::rng::hash64(&[w.meta.seed, 0x400F, cid.0 as u64]).is_multiple_of(2) {
        Material::THATCH
    } else {
        Material::WOOD
    }
}

/// What a community's open work wants that is not to hand, worked back down the building
/// chain (C67, docs/design-rules.md section 2): the fittings to make, the planks, dressed
/// stone and thatch to refine, and the logs, rough stone and fibre that takes.
#[derive(Default, Debug)]
pub struct ChainNeeds {
    /// Fittings to make: (recipe, how many).
    pub make: Vec<(u8, u32)>,
    pub planks: u32,
    pub blocks: u32,
    pub thatch: u32,
    pub logs: u32,
    pub rough: u32,
    pub fibre: u32,
}

pub(crate) fn chain_needs(w: &World, cid: crate::ids::CommunityId, here: &Here) -> ChainNeeds {
    use crate::work::recipes as rc;
    let missing = |k: ItemKind| -> u32 {
        w.designations
            .iter()
            .filter(|d| d.community == Some(cid) && !d.done)
            .flat_map(|d| d.missing())
            .filter(|(x, _)| *x == k)
            .map(|(_, n)| n)
            .sum()
    };
    // What crafts already ordered will bring.
    let coming = |id: u8| -> u32 {
        let r = rc::recipe(id);
        w.designations
            .iter()
            .filter(|d| d.community == Some(cid) && !d.done && d.kind == DesignationKind::Craft(id))
            .count() as u32
            * r.output.1
    };
    let stock = |k: ItemKind| count_kind(w, here, k);
    let mut n = ChainNeeds::default();
    let mut planks_for = 0;
    for (id, kind) in [
        (rc::DOOR, ItemKind::Door),
        (rc::BED_FRAME, ItemKind::BedFrame),
        (rc::TABLE, ItemKind::Table),
        (rc::STOOL, ItemKind::Stool),
        (rc::CHEST, ItemKind::Chest),
    ] {
        let gap = missing(kind).saturating_sub(stock(kind) + coming(id));
        if gap > 0 {
            n.make.push((id, gap));
            planks_for += gap * rc::recipe(id).inputs[0].1;
        }
    }
    n.planks = (missing(ItemKind::Plank) + planks_for)
        .saturating_sub(stock(ItemKind::Plank) + coming(rc::PLANK));
    n.blocks = missing(ItemKind::StoneBlock)
        .saturating_sub(stock(ItemKind::StoneBlock) + coming(rc::DRESS_STONE));
    n.thatch =
        missing(ItemKind::Thatch).saturating_sub(stock(ItemKind::Thatch) + coming(rc::THATCH));
    n.logs = n.planks.div_ceil(2);
    n.rough =
        (n.blocks + missing(ItemKind::RoughStone)).saturating_sub(stock(ItemKind::RoughStone));
    n.fibre = (n.thatch * 2).saturating_sub(stock(ItemKind::Fibre));
    n
}

/// Continue the persisted upper sleeping room for one existing house, or propose it from an
/// individual colonist's unmet rest need. This is intentionally one level and one room: the
/// autonomous site/cultural choice, upper workshops and collapse/removal remain later #483
/// slices.
fn plan_upper_sleeping(
    w: &mut World,
    cid: crate::ids::CommunityId,
    origin: TilePos,
    home: TilePos,
    people: u32,
    beds: u32,
) -> bool {
    let gap = crate::houses::bed_gap(w, cid);
    let mut plan = crate::houses::plan(w, origin);
    if plan.upper_floor.is_empty() {
        if beds >= people {
            return false;
        }
        if !upper_storey_building_is_secured(w, cid) {
            return false;
        }
        let cultural = w.customs.iter().any(|c| {
            c.community == cid
                && !c.lapsed
                && c.intensity >= 0.1
                && matches!(c.kind, crate::culture::CustomKind::UpperStorey)
        });
        let unmet = w.communities.iter().find(|c| c.id == cid).is_some_and(|c| {
            c.members.iter().any(|id| {
                w.agents.iter().any(|a| {
                    a.id == *id
                        && a.alive
                        && a.species == crate::agents::Species::Person
                        && a.needs.rest <= 0.35
                })
            })
        });
        if !unmet && !cultural {
            return false;
        }
        let posted_beds = w
            .designations
            .iter()
            .filter(|d| {
                !d.done
                    && matches!(
                        d.kind,
                        DesignationKind::BuildStructure(k, _) if k.is_sleeping_place()
                    )
            })
            .count() as u32;
        let short = people.saturating_sub(beds + posted_beds).max(1);
        // A valid same-level annex is the control path: it is cheaper, keeps the household on
        // the ground it already knows, and must win whenever the ground can carry it.
        if crate::houses::annex(w, origin, home, short, gap).is_some() {
            return false;
        }
        let cramped =
            crate::houses::new_house(w, home.offset(-7, -5, 0), home, short, gap).is_none();
        let Some(reason) = upper_storey_reason(cultural, cramped, people > beds) else {
            return false;
        };
        let Some((upper_floor, lower_stair)) =
            crate::houses::upper_sleeping_candidate(w, &plan, gap)
        else {
            return false;
        };
        plan.upper_floor = upper_floor;
        plan.upper_stair = Some(lower_stair);
        plan.upper_reason = Some(reason);
        if let Some(c) = w.communities.iter_mut().find(|c| c.id == cid) {
            if let Some(k) = c.house_sites.iter().position(|o| *o == origin) {
                c.house_plans
                    .resize(c.house_sites.len(), crate::society::HousePlan::default());
                c.house_plans[k] = plan.clone();
            } else {
                return false;
            }
        } else {
            return false;
        }
        w.events.push(
            w.meta.tick,
            Some(origin),
            crate::events::Severity::Notable,
            crate::events::EventKind::HouseStage {
                community: cid,
                at: origin,
                stage: format!("upper room for {}", reason.label()),
                by: best_builder(w, cid),
            },
        );
    }

    let Some(lower_stair) = plan.upper_stair else {
        return false;
    };
    let upper_stair = lower_stair.offset(0, 0, 1);
    let wood = Material::WOOD;
    let wall = crate::houses::walls(&plan.floor)
        .into_iter()
        .find_map(|(p, _)| match w.grid.get(p).material {
            Material::WOOD | Material::STONE_BLOCK => Some(w.grid.get(p).material),
            _ => None,
        })
        .unwrap_or(wood);
    let stairs_built = matches!(
        (w.grid.get(lower_stair).shape, w.grid.get(upper_stair).shape),
        (
            Shape::StairsUp | Shape::StairsBoth,
            Shape::StairsDown | Shape::StairsBoth
        )
    );
    if !stairs_built {
        if !w.designations.iter().any(|d| {
            !d.done && d.pos == lower_stair && matches!(d.kind, DesignationKind::BuildStairs(_))
        }) {
            post(w, DesignationKind::BuildStairs(wood), lower_stair, 9);
        }
        return true;
    }

    // The upper stair is an opening. Every other upper tile is laid one by one only after the
    // lower support check has passed; the worker repeats the support guard at execution time.
    let floors_done = plan
        .upper_floor
        .iter()
        .filter(|p| **p != upper_stair)
        .all(|p| w.grid.get(*p).shape == Shape::Floor);
    if !floors_done {
        for p in plan
            .upper_floor
            .iter()
            .copied()
            .filter(|p| *p != upper_stair)
        {
            if w.grid.get(p).shape != Shape::Floor
                && crate::building::floor_supported(w, p)
                && !w.designations.iter().any(|d| !d.done && d.pos == p)
            {
                post(w, DesignationKind::BuildFloor(wood), p, 8);
            }
        }
        return true;
    }

    let ring = crate::houses::walls(&plan.upper_floor);
    let walls_done = ring
        .iter()
        .all(|(p, _)| w.grid.get(*p).shape == Shape::Wall);
    if !walls_done {
        for (p, _) in ring {
            if w.grid.get(p).shape != Shape::Wall
                && !w.designations.iter().any(|d| {
                    !d.done && d.pos == p && matches!(d.kind, DesignationKind::BuildWall(_))
                })
            {
                post(w, DesignationKind::BuildWall(wall), p, 7);
            }
        }
        return true;
    }

    let roofed = plan
        .upper_floor
        .iter()
        .filter(|p| **p != upper_stair)
        .all(|p| w.roofs.contains_key(p));
    if !roofed {
        let planned = |p: TilePos| {
            w.designations
                .iter()
                .any(|d| !d.done && d.pos == p && matches!(d.kind, DesignationKind::BuildRoof(_)))
        };
        let roof = roof_style(w, cid, wall);
        let to_roof: Vec<TilePos> = plan
            .upper_floor
            .iter()
            .copied()
            .filter(|p| {
                *p != upper_stair
                    && !w.roofs.contains_key(p)
                    && !planned(*p)
                    && crate::building::held(w, *p, &planned)
            })
            .collect();
        for p in to_roof {
            post(w, DesignationKind::BuildRoof(roof), p, 6);
        }
        return true;
    }

    let upper_plan = crate::society::HousePlan {
        floor: plan
            .upper_floor
            .iter()
            .copied()
            .filter(|p| *p != upper_stair)
            .collect(),
        doors: vec![upper_stair],
        ..crate::society::HousePlan::default()
    };
    let spots = crate::houses::layout(&upper_plan, gap)
        .beds
        .into_iter()
        .filter(|p| *p != upper_stair);
    let posted_all = w
        .designations
        .iter()
        .filter(|d| {
            !d.done
                && matches!(d.kind, DesignationKind::BuildStructure(k, _) if k.is_sleeping_place())
        })
        .count() as u32;
    let mut to_post = people.saturating_sub(beds + posted_all);
    for p in spots {
        if to_post == 0 {
            break;
        }
        if w.structures.iter().any(|s| s.pos == p)
            || w.designations.iter().any(|d| !d.done && d.pos == p)
        {
            continue;
        }
        post(
            w,
            DesignationKind::BuildStructure(StructureKind::Bed, wood),
            p,
            6,
        );
        to_post -= 1;
    }
    true
}

/// Rule 7 for an upper-storey choice (#483): food, water and the existing shelter must be safe
/// before a household spends time on vertical growth. This is deliberately checked when the
/// plan is proposed, before any new material-backed designation is posted.
fn upper_storey_building_is_secured(w: &World, cid: crate::ids::CommunityId) -> bool {
    let days = crate::systems::settlement::food_days(w, cid);
    if days < crate::systems::decide::SECURE_DAYS {
        return false;
    }
    let Some(c) = w.communities.iter().find(|c| c.id == cid) else {
        return false;
    };
    c.members.iter().all(|id| {
        w.agents
            .iter()
            .find(|a| a.id == *id && a.alive && a.species == crate::agents::Species::Person)
            .is_some_and(|a| crate::systems::decide::secured(w, a, Some(days)).all())
    })
}

/// Select and record the first reason that makes the upper room a normal household choice.
/// The pure decision is kept small so the control case and precedence stay guarded without
/// needing a lucky generated world in a test.
pub(crate) fn upper_storey_reason(
    cultural: bool,
    cramped: bool,
    grown_household: bool,
) -> Option<crate::society::UpperStoreyReason> {
    if !grown_household {
        return None;
    }
    if cultural {
        Some(crate::society::UpperStoreyReason::CulturalPreference)
    } else if cramped {
        Some(crate::society::UpperStoreyReason::CrampedSite)
    } else {
        Some(crate::society::UpperStoreyReason::GrownHousehold)
    }
}

/// A finished upper room can host one workshop when no same-level site exists. Requiring every
/// floor, wall and roof to be complete keeps a workshop from becoming a shortcut around the
/// staged construction chain, while the navigation check keeps the project reachable through
/// the material-backed stair (#483).
fn upper_workspace_spot(w: &mut World, origin: TilePos, here: &Here) -> Option<TilePos> {
    let plan = crate::houses::plan(w, origin);
    let upper_stair = plan.upper_stair.map(|p| p.offset(0, 0, 1));
    let upper_floor: Vec<TilePos> = plan
        .upper_floor
        .iter()
        .copied()
        .filter(|p| Some(*p) != upper_stair)
        .collect();
    if upper_floor.is_empty()
        || !upper_floor
            .iter()
            .all(|p| w.grid.get(*p).shape == Shape::Floor && w.roofs.contains_key(p))
        || !crate::houses::walls(&plan.upper_floor)
            .iter()
            .all(|(p, _)| w.grid.get(*p).shape == Shape::Wall)
    {
        return None;
    }
    w.ensure_nav();
    let reachable = |p: TilePos| w.nav.as_ref().is_some_and(|nav| nav.reach(here.home, p));
    upper_floor.into_iter().find(|p| {
        here.holds(*p)
            && reachable(*p)
            && !w.structures.iter().any(|s| s.pos == *p)
            && !w.designations.iter().any(|d| !d.done && d.pos == *p)
    })
}

/// 6d. The building chain (C67): benches where there are none and the work wants them, then
/// the orders that refine and make what the open work is waiting on.
fn building_chain(w: &mut World, cid: crate::ids::CommunityId, here: &Here, home: TilePos) {
    use crate::work::recipes as rc;
    let n = chain_needs(w, cid, here);
    let stock = |w: &World, k: ItemKind| count_kind(w, here, k);
    // A bench is built of raw stuff (as a carpenter's workshop is of a log in Dwarf Fortress):
    // the first thing the chain needs.
    let benches = [
        (
            StructureKind::CarpenterBench,
            n.planks > 0 || !n.make.is_empty(),
            home.offset(-4, 2, 0),
        ),
        (
            StructureKind::Workbench,
            n.blocks > 0 || n.thatch > 0,
            home.offset(4, -2, 0),
        ),
    ];
    for (kind, wanted, near) in benches {
        if !wanted
            || w.structures
                .iter()
                .any(|s| s.kind == kind && here.holds(s.pos))
            || w.designations.iter().any(|d| {
                d.community == Some(cid)
                    && matches!(d.kind, DesignationKind::BuildStructure(x, _) if x == kind)
            })
        {
            continue;
        }
        if let Some(spot) = find_flat_area(w, near, 1, 1, 1).filter(|p| here.holds(*p)) {
            let d = make(
                w,
                DesignationKind::BuildStructure(kind, Material::WOOD),
                spot,
                7,
            );
            w.designations.push(d);
        }
    }
    // Orders: as many at once as the gap asks, up to three of a kind.
    let mut orders: Vec<(u8, u32)> = Vec::new();
    if n.planks > 0 && stock(w, ItemKind::Log) >= 1 {
        orders.push((rc::PLANK, n.planks.div_ceil(2)));
    }
    if n.blocks > 0 && stock(w, ItemKind::RoughStone) >= 1 {
        orders.push((rc::DRESS_STONE, n.blocks));
    }
    if n.thatch > 0 && stock(w, ItemKind::Fibre) >= 2 {
        orders.push((rc::THATCH, n.thatch));
    }
    let planks = stock(w, ItemKind::Plank);
    for (id, gap) in &n.make {
        if planks >= rc::recipe(*id).inputs[0].1 {
            orders.push((*id, *gap));
        }
    }
    for (id, gap) in orders {
        let station = rc::recipe(id).station;
        let Some(at) = w
            .structures
            .iter()
            .find(|s| s.kind == station && s.progress >= 1.0 && here.holds(s.pos))
            .map(|s| s.pos)
        else {
            continue;
        };
        let open = w
            .designations
            .iter()
            .filter(|d| d.community == Some(cid) && !d.done && d.kind == DesignationKind::Craft(id))
            .count() as u32;
        for _ in open..gap.min(3) {
            let mut d = make(w, DesignationKind::Craft(id), at, 6);
            d.community = Some(cid);
            w.designations.push(d);
        }
    }
    // Reeds cut for the thatch.
    if n.fibre > 0 {
        let open_reed = w
            .designations
            .iter()
            .filter(|d| {
                d.kind == DesignationKind::Harvest
                    && w.plants
                        .iter()
                        .any(|p| p.pos == d.pos && p.kind == PlantKind::Reed)
            })
            .count() as u32;
        let want = n.fibre.min(3).saturating_sub(open_reed);
        let reeds: Vec<TilePos> = w
            .plants
            .iter()
            .filter(|p| p.alive && p.kind == PlantKind::Reed && p.pos.dist(home) <= 50)
            .map(|p| p.pos)
            .filter(|p| !w.designations.iter().any(|d| d.pos == *p))
            .take(want as usize)
            .collect();
        for p in reeds {
            post(w, DesignationKind::Harvest, p, 5);
        }
    }
}

/// The floor of the house at `origin`, under its roof.
pub fn house_interior(w: &World, origin: TilePos) -> Vec<TilePos> {
    crate::houses::plan(w, origin).floor
}

/// Once a house's walls stand, roof it; roof again what has fallen or burned. And say in the
/// feed when a house reaches a stage.
fn roofs_and_stages(w: &mut World, cid: crate::ids::CommunityId) {
    let Some(c) = w.communities.iter().find(|c| c.id == cid) else {
        return;
    };
    let sites = c.house_sites.clone();
    let mut stages = c.house_stage.clone();
    stages.resize(sites.len(), 0);
    for (k, origin) in sites.iter().copied().enumerate() {
        let plan = crate::houses::plan(w, origin);
        let walls = crate::houses::complete(w, &plan);
        let mid = crate::houses::heart(&plan.floor).unwrap_or(origin);
        let inner = plan.floor.clone();
        let roofed = inner.iter().filter(|p| w.roofs.contains_key(p)).count();
        let before = stages[k];
        // 1 marked out, 2 walls up and never roofed, 3 roofed, 4 was roofed and has lost some.
        let stage = if !walls {
            1
        } else if roofed == inner.len() {
            3
        } else if before >= 3 {
            4
        } else {
            2
        };
        if stage != before {
            let said = match (before, stage) {
                (0, _) => None, // an old save: say nothing about houses already standing
                (_, 2) if before < 2 => Some("walls up"),
                (4, 3) => Some("mended"),
                (_, 3) => Some("roofed"),
                _ => None,
            };
            if let Some(s) = said {
                let eid = w.events.push(
                    w.meta.tick,
                    Some(mid),
                    crate::events::Severity::Notable,
                    crate::events::EventKind::HouseStage {
                        community: cid,
                        at: mid,
                        stage: s.into(),
                        by: None,
                    },
                );
                if s == "roofed" {
                    note_building_way(w, cid, origin, eid);
                }
            }
            stages[k] = stage;
        }
        // The first completed upper room becomes a recorded building custom. Future households
        // can therefore choose upward growth for a cultural reason whose origin is this actual
        // room, rather than a hidden global preference or a random tower rule.
        if !plan.upper_floor.is_empty()
            && plan
                .upper_floor
                .iter()
                .all(|p| *p == plan.upper_stair.unwrap_or(*p) || w.roofs.contains_key(p))
            && upper_storey_way(w, cid).is_none()
        {
            let eid = w.events.push(
                w.meta.tick,
                Some(mid),
                crate::events::Severity::Notable,
                crate::events::EventKind::HouseStage {
                    community: cid,
                    at: mid,
                    stage: "upper room roofed; a way of building upward was learned".into(),
                    by: None,
                },
            );
            note_upper_storey_way(w, cid, origin, eid, plan.upper_reason);
        }
        if !walls {
            // Walls that are gone (burned, fallen) or whose orders were dropped are built
            // again. Nothing reposted them, and a house left with gaps blocked every later
            // house: in the C57 soaks seed 3's first house lost four walls to a fire on day 2
            // and 67 people had 18 places to sleep at day 300 (FIXLOG, C57).
            let mut stone = 0;
            let mut timber = 0;
            let mut gaps = Vec::new();
            for (q, _) in crate::houses::walls(&plan.floor) {
                let door = plan.doors.contains(&q);
                let t = w.grid.get(q);
                if t.shape == Shape::Wall {
                    if t.material == Material::STONE_BLOCK {
                        stone += 1;
                    } else {
                        timber += 1;
                    }
                } else if !w.designations.iter().any(|d| !d.done && d.pos == q) {
                    gaps.push((q, door));
                }
            }
            let wall = if stone > timber {
                Material::STONE_BLOCK
            } else {
                Material::WOOD
            };
            for (q, door) in gaps {
                if door {
                    post(w, DesignationKind::BuildDoor, q, 6);
                } else {
                    post(w, DesignationKind::BuildWall(wall), q, 7);
                }
            }
            continue;
        }
        // Snow lying heavy on the roof: someone goes up and clears it, a stretch at a time.
        let heavy: Vec<TilePos> = inner
            .iter()
            .filter(|p| w.roofs.get(p).is_some_and(|r| r.snow > 0.55 * r.bears()))
            .copied()
            .collect();
        for p in heavy {
            let covered = w.designations.iter().any(|d| {
                !d.done && matches!(d.kind, DesignationKind::ClearRoof) && d.pos.dist(p) <= 3
            });
            if !covered {
                post(w, DesignationKind::ClearRoof, p, 9);
            }
        }
        // A roof half done is finished before new work: what is left of it comes first.
        if roofed * 2 >= inner.len() {
            for d in w.designations.iter_mut() {
                if !d.done
                    && matches!(d.kind, DesignationKind::BuildRoof(_))
                    && inner.contains(&d.pos)
                {
                    d.priority = d.priority.max(8);
                }
            }
        }
        let wall = w.grid.get(origin).material;
        // The roof's material: what the house is roofed with already, unless that is thatch
        // and there is none to be had, in which case what the community can lay now (#257:
        // an annex waited sixty-eight days for thatch that no reeds within reach would give).
        let roof = match inner
            .iter()
            .find_map(|p| w.roofs.get(p).map(|r| r.material))
        {
            Some(Material::THATCH) => roof_style(w, cid, wall),
            Some(m) => m,
            None => roof_style(w, cid, wall),
        };
        for p in inner {
            if w.roofs.contains_key(&p)
                || w.designations.iter().any(|d| {
                    !d.done && d.pos == p && matches!(d.kind, DesignationKind::BuildRoof(_))
                })
            {
                continue;
            }
            // Only what a wall will hold, counting what is planned.
            let planned = |q: TilePos| {
                w.designations.iter().any(|d| {
                    !d.done && d.pos == q && matches!(d.kind, DesignationKind::BuildRoof(_))
                })
            };
            if crate::building::held(w, p, &planned) {
                post(w, DesignationKind::BuildRoof(roof), p, 6);
            }
        }
    }
    if let Some(c) = w.communities.iter_mut().find(|c| c.id == cid) {
        c.house_stage = stages;
    }
}

/// Whether `p` lies inside a community's wall that stands whole (C73): the night's raiders
/// do not come for what is behind a barred gate.
pub fn in_walls(w: &World, p: TilePos) -> bool {
    w.communities.iter().any(|c| {
        c.palisade
            .as_ref()
            .is_some_and(|pal| palisade_holds(pal, p))
    })
}

/// Whether `p` lies inside this wall, and the wall stands whole (C73). `in_walls` asks it of
/// every community; the settlement planner asks it of its own (#377 5.1a, FIXLOG 2026-11-09).
pub fn palisade_holds(pal: &crate::society::Palisade, p: TilePos) -> bool {
    pal.closed && {
        let (x0, x1) = (
            pal.ring.iter().map(|q| q.x).min().unwrap_or(0),
            pal.ring.iter().map(|q| q.x).max().unwrap_or(0),
        );
        let (y0, y1) = (
            pal.ring.iter().map(|q| q.y).min().unwrap_or(0),
            pal.ring.iter().map(|q| q.y).max().unwrap_or(0),
        );
        // Inside the outline on the map, within a few levels of the ring.
        pal.ring.first().is_some_and(|q| (q.z - p.z).abs() <= 3)
            && p.x > x0
            && p.x < x1
            && p.y > y0
            && p.y < y1
    }
}

/// What stands on the wall's line: a structure other than a store's marker (a bench, a bed,
/// a hearth), or a roofed tile of a standing building. The ring goes round these, not
/// through them (#403, M5 review 2026-10-26: seed 4's Merehollow).
fn stands_on_the_line(w: &World, q: TilePos) -> bool {
    w.structures
        .iter()
        .any(|s| s.pos == q && s.kind != StructureKind::Stockpile)
        || w.roofs.contains_key(&q)
}

/// Whether the outline tile `q` is the roofed floor of a building whose every way out (a door,
/// an open side) lies strictly inside the outline `(x0, x1, y0, y1)`: its walls hold the line
/// there (#403, 2026-11-01: a house at the map's edge, which the line could not go round).
fn held_by_building(w: &World, q: TilePos, (x0, x1, y0, y1): (i32, i32, i32, i32)) -> bool {
    if !w.roofs.contains_key(&q) {
        return false;
    }
    let mut floor = vec![q];
    let mut i = 0;
    while i < floor.len() {
        let p = floor[i];
        i += 1;
        for (dx, dy) in [(0, 1), (1, 0), (0, -1), (-1, 0)] {
            let n = p.offset(dx, dy, 0);
            if floor.contains(&n) {
                continue;
            }
            if w.roofs.contains_key(&n) {
                floor.push(n);
                // A roofed stretch this large is no single building.
                if floor.len() > 400 {
                    return false;
                }
            } else if w.grid.walkable(n) && !(n.x > x0 && n.x < x1 && n.y > y0 && n.y < y1) {
                return false;
            }
        }
    }
    true
}

/// A wall proposed after a raid (C73, D42): the ring round what stands, the houses and the
/// stores and the hearth, two tiles out, grown a tile at a time by the builders as a house
/// is, with one gate on the side the fields lie. Posted once; said once it stands.
fn palisade(w: &mut World, cid: crate::ids::CommunityId, home: TilePos) {
    let Some(c) = w.communities.iter().find(|c| c.id == cid) else {
        return;
    };
    let wanted = crate::project::open_of(w, cid, |k| *k == crate::project::ProjectKind::Wall)
        .filter(|p| p.site.is_none())
        .map(|p| p.id);
    let first_house = c.house_sites.first().copied();
    // A ring at a camp since left (a save from before #403, when a move kept it) is let go: it is
    // not built or mended from the new home, and a wall may be proposed here.
    if c.palisade.as_ref().is_some_and(|p| p.gate.dist(home) > 45) {
        if let Some(c) = w.communities.iter_mut().find(|c| c.id == cid) {
            c.palisade = None;
        }
        return;
    }
    if let Some(pal) = c.palisade.clone() {
        let standing = |w: &World, q: &TilePos| {
            let t = w.grid.get(*q);
            (*q == pal.gate && t.flags.has(crate::world::TileFlags::DOOR))
                || (*q != pal.gate && t.shape == Shape::Wall)
        };
        // A ring tile nobody from home can reach is no way in (C74: five tiles on a ledge by
        // the map's edge failed every builder, and the wall never closed).
        // Only while home itself is on open ground: a home tile nobody can stand on reaches
        // nothing, and every tile would count.
        w.ensure_nav();
        let cut_off = |w: &World, q: &TilePos| {
            w.nav
                .as_ref()
                .is_some_and(|n| n.component_of(home).is_some() && !n.reach(home, *q))
        };
        // A ring tile is out of reach when neither it nor a tile beside it on its level, where
        // a builder would stand (`work/planner.rs` `stand_tile`), can be walked to from home.
        let unreachable = |w: &World, q: &TilePos| {
            cut_off(w, q)
                && (-1..=1)
                    .flat_map(|dx| (-1..=1).map(move |dy| (dx, dy)))
                    .filter(|d| *d != (0, 0))
                    .all(|(dx, dy)| {
                        let n = q.offset(dx, dy, 0);
                        !w.grid.walkable(n) || cut_off(w, &n)
                    })
        };
        let whole = pal.ring.iter().all(|q| standing(w, q) || unreachable(w, q));
        // Orders at ring tiles nobody can reach are withdrawn, what was brought for them left
        // by the site, and posted again if a way opens (#403, 2026-11-01: seed 3's Holtwick
        // tile (5,8,4) on a rise inside the ring, whose ways up the ring itself had walled,
        // failed its order at priority 8 for 580 days).
        let cut: Vec<TilePos> = pal
            .ring
            .iter()
            .copied()
            .filter(|q| !standing(w, q) && unreachable(w, q))
            .collect();
        if !cut.is_empty() {
            let gone = crate::work::designation::split_off(&mut w.designations, |d| {
                !(!d.done
                    && cut.contains(&d.pos)
                    && matches!(
                        d.kind,
                        DesignationKind::BuildWall(_) | DesignationKind::BuildDoor
                    ))
            });
            w.give_back(gone);
        }
        // The rest of the outline (slopes, the edges of other levels) must not let anyone
        // walk round the wall (C73: in a hilly valley the ring was 40 tiles of a 130-tile
        // outline, the rest open, and the night's raiders walked in).
        let (mut x0, mut x1) = (
            pal.ring.iter().map(|q| q.x).min().unwrap_or(0),
            pal.ring.iter().map(|q| q.x).max().unwrap_or(0),
        );
        let (mut y0, mut y1) = (
            pal.ring.iter().map(|q| q.y).min().unwrap_or(0),
            pal.ring.iter().map(|q| q.y).max().unwrap_or(0),
        );
        let sealed = (x0..=x1)
            .flat_map(|x| (y0..=y1).map(move |y| (x, y)))
            .filter(|(x, y)| *x == x0 || *x == x1 || *y == y0 || *y == y1)
            .all(|(x, y)| {
                let s = w.grid.surface(x, y);
                // Water holds the line, as when the ring was laid (#403, M5 review 2026-10-26:
                // seed 1's ring of 134 stood whole for 440 days, never closed for four tiles of
                // a stream on its outline that the laying had left to the water).
                // Ground nobody from home can reach is no way in, as a ring tile is (#403,
                // 2026-11-01: seed 3's Holtwick, whole from day 10062, never closed for two
                // bedroom tiles of a house on the line that its wall had shut off). The floor
                // of a building whose every way out lies inside the line is held by the
                // building's own walls.
                pal.ring.iter().any(|q| q.x == x && q.y == y)
                    || !w.grid.walkable(s)
                    || w.grid.get(s).is_water()
                    || cut_off(w, &s)
                    || held_by_building(w, s, (x0, x1, y0, y1))
            });
        let closed = whole && sealed;
        let tick = w.meta.tick;
        let first = closed && pal.stood.is_none();
        if let Some(c) = w.communities.iter_mut().find(|c| c.id == cid) {
            if let Some(p) = c.palisade.as_mut() {
                p.closed = closed;
                if first {
                    p.stood = Some(tick);
                }
            }
        }
        if first {
            let side = match (pal.gate.x - home.x, pal.gate.y - home.y) {
                (dx, dy) if dx.abs() >= dy.abs() && dx > 0 => "east",
                (dx, dy) if dx.abs() >= dy.abs() => "west",
                (_, dy) if dy > 0 => "south",
                _ => "north",
            };
            w.events.push(
                tick,
                Some(pal.gate),
                crate::events::Severity::Major,
                crate::events::EventKind::Walled {
                    community: cid,
                    gate: pal.gate,
                    side: side.into(),
                },
            );
        }
        // A wall half built is finished before new work, as a roof is (C74: seed 3's last
        // orders waited behind everything else at priority 5 for four hundred days).
        let up = pal.ring.iter().filter(|q| standing(w, q)).count();
        if up * 2 >= pal.ring.len() {
            for d in w.designations.iter_mut() {
                if !d.done
                    && matches!(
                        d.kind,
                        DesignationKind::BuildWall(_) | DesignationKind::BuildDoor
                    )
                    && pal.ring.contains(&d.pos)
                {
                    d.priority = d.priority.max(8);
                }
            }
        }
        // Ground on the outline that has come open since (a tree felled, a bush gone): taken
        // into the ring (C73: fifteen such tiles left a wall of 85 open for good). What has
        // gone up on the line since (a building, a bench) moves the line out a tile on that
        // side, round it, as at the laying (#403).
        let mut ring = pal.ring.clone();
        for _ in 0..128 {
            let mut grown = false;
            let (mut ox0, mut ox1, mut oy0, mut oy1) = (false, false, false, false);
            for (x, y) in (x0..=x1)
                .flat_map(|x| (y0..=y1).map(move |y| (x, y)))
                .filter(|(x, y)| *x == x0 || *x == x1 || *y == y0 || *y == y1)
            {
                let q = w.grid.surface(x, y);
                if ring.iter().any(|r| r.x == x && r.y == y)
                    || !w.grid.walkable(q)
                    || w.grid.get(q).is_water()
                {
                    continue;
                }
                if stands_on_the_line(w, q) {
                    ox0 |= x == x0;
                    ox1 |= x == x1;
                    oy0 |= y == y0;
                    oy1 |= y == y1;
                } else {
                    ring.push(q);
                    grown = true;
                }
            }
            if ox0 && x0 > 5 {
                x0 -= 1;
                grown = true;
            }
            if ox1 && x1 < w.grid.width - 6 {
                x1 += 1;
                grown = true;
            }
            if oy0 && y0 > 5 {
                y0 -= 1;
                grown = true;
            }
            if oy1 && y1 < w.grid.height - 6 {
                y1 += 1;
                grown = true;
            }
            if !grown {
                break;
            }
        }
        if ring.len() != pal.ring.len() {
            if let Some(c) = w.communities.iter_mut().find(|c| c.id == cid) {
                if let Some(p) = c.palisade.as_mut() {
                    p.ring = ring.clone();
                }
            }
        }
        // Built and mended: every ring tile not standing and with no order gets one (C73:
        // seed 3's wall stood at 64 of 85 with five orders open, the rest never reposted).
        if !whole || ring.len() != pal.ring.len() {
            let pal = crate::society::Palisade {
                ring,
                ..pal.clone()
            };
            let wall = wall_material(w, cid, first_house);
            // Orders of the other material are posted again of this one, what was brought for
            // them left at the site (#403: seed 2's Stoneside, which no longer knew how to saw
            // a plank, left eighteen timber orders waiting on planks for 380 days).
            let stale = crate::work::designation::split_off(&mut w.designations, |d| {
                !(!d.done
                    && pal.ring.contains(&d.pos)
                    && matches!(d.kind, DesignationKind::BuildWall(m) if m != wall))
            });
            w.give_back(stale);
            // A store's marker on the line is moved off it: the goods are hauled elsewhere.
            w.structures
                .retain(|st| !(st.kind == StructureKind::Stockpile && pal.ring.contains(&st.pos)));
            for q in pal.ring.clone() {
                if !standing(w, &q)
                    && !unreachable(w, &q)
                    && !w.designations.iter().any(|d| !d.done && d.pos == q)
                {
                    if q == pal.gate {
                        post(w, DesignationKind::BuildDoor, q, 6);
                    } else {
                        post(w, DesignationKind::BuildWall(wall), q, 5);
                    }
                }
            }
        }
        return;
    }
    let Some(pid) = wanted else { return };
    // What the wall goes round: the houses' walls, the hearth, the stores, the benches.
    let mut inside: Vec<TilePos> = vec![home];
    let (gw, gh) = (w.grid.width, w.grid.height);
    for o in &c.house_sites {
        let plan = crate::houses::plan(w, *o);
        // A house too near the map's edge for a ring two tiles out stays outside the wall
        // (C74: the edge clamp cut the ring through such a house, its roof and chest on the
        // line, and the wall never closed).
        let near_edge = crate::houses::walls(&plan.floor)
            .iter()
            .any(|(q, _)| q.x < 7 || q.y < 7 || q.x > gw - 8 || q.y > gh - 8);
        if near_edge {
            continue;
        }
        inside.extend(
            crate::houses::walls(&plan.floor)
                .into_iter()
                .map(|(q, _)| q),
        );
    }
    let here = Here::of(w, cid, home);
    inside.extend(
        w.structures
            .iter()
            .filter(|s| {
                s.pos.z == home.z
                    && s.pos.dist(home) <= 25
                    && here.holds(s.pos)
                    && !matches!(s.kind, StructureKind::Grave | StructureKind::Well)
            })
            .map(|s| s.pos),
    );
    let inside: Vec<TilePos> = inside
        .into_iter()
        .filter(|q| (q.z - home.z).abs() <= 3)
        .collect();
    let g = &w.grid;
    let mut x0 = (inside.iter().map(|q| q.x).min().unwrap_or(home.x) - 2).max(5);
    let mut x1 = (inside.iter().map(|q| q.x).max().unwrap_or(home.x) + 2).min(g.width - 6);
    let mut y0 = (inside.iter().map(|q| q.y).min().unwrap_or(home.y) - 2).max(5);
    let mut y1 = (inside.iter().map(|q| q.y).max().unwrap_or(home.y) + 2).min(g.height - 6);
    // The line goes round what stands on it (#403: seed 4's Merehollow laid its ring across
    // a standing building; the furniture on the outline was left out of the ring, nothing
    // could be built where it stood, and the wall never closed). A structure or a roofed
    // tile on the outline moves the line out a tile on that side, until the building stands
    // inside the ring.
    for _ in 0..128 {
        let blocked = (x0..=x1)
            .flat_map(|x| (y0..=y1).map(move |y| (x, y)))
            .filter(|(x, y)| *x == x0 || *x == x1 || *y == y0 || *y == y1)
            .map(|(x, y)| g.surface(x, y))
            .find(|q| stands_on_the_line(w, *q));
        let Some(q) = blocked else { break };
        let mut moved = false;
        if q.x == x0 && x0 > 5 {
            x0 -= 1;
            moved = true;
        }
        if q.x == x1 && x1 < g.width - 6 {
            x1 += 1;
            moved = true;
        }
        if q.y == y0 && y0 > 5 {
            y0 -= 1;
            moved = true;
        }
        if q.y == y1 && y1 < g.height - 6 {
            y1 += 1;
            moved = true;
        }
        if !moved {
            break;
        }
    }
    let mut ring: Vec<TilePos> = Vec::new();
    for x in x0..=x1 {
        for y in y0..=y1 {
            if x == x0 || x == x1 || y == y0 || y == y1 {
                // On the ground there is at every column of the outline, whatever its level
                // (C73: laid on home's level only, a hilly ring was 40 tiles of a 130-tile
                // outline and never closed); water and rock hold the line themselves.
                let q = g.surface(x, y);
                let t = g.get(q);
                // Not on what stands (a bench, a bed) nor on the roofed floor of a building
                // the line could not go round (#403, 2026-11-01: at the map's edge seed 3's
                // Holtwick walled the passage between two bedrooms and shut the beds off).
                if t.shape == Shape::Open
                    && !t.is_water()
                    && g.walkable(q)
                    && !stands_on_the_line(w, q)
                {
                    ring.push(q);
                }
            }
        }
    }
    if ring.len() < 8 {
        return;
    }
    // The gate: the ring tile nearest the fields (else the landing), on the ring's own side.
    let toward = c
        .field_tiles
        .first()
        .copied()
        .or(c.field_site)
        .unwrap_or(w.meta.founding_site);
    let gate = *ring.iter().min_by_key(|q| (q.dist(toward), **q)).unwrap();
    let wall = wall_material(w, cid, first_house);
    w.structures
        .retain(|st| !(st.kind == StructureKind::Stockpile && ring.contains(&st.pos)));
    for q in &ring {
        if *q == gate {
            post(w, DesignationKind::BuildDoor, *q, 6);
        } else {
            post(w, DesignationKind::BuildWall(wall), *q, 5);
        }
    }
    let tick = w.meta.tick;
    if let Some(c) = w.communities.iter_mut().find(|c| c.id == cid) {
        c.palisade = Some(crate::society::Palisade {
            ring,
            gate,
            begun: tick,
            stood: None,
            closed: false,
        });
    }
    set_home_site(w, pid, gate);
}

/// What a wall is built of (C73): what the first house is walled with, else timber; but stone
/// when nobody of the community can saw a plank and one of it can dress stone (#403: a timber
/// wall ordered of a community that had lost carpentry never closed). By what is known, not by
/// what is in the stores, so the choice does not turn back and forth as planks come and go.
fn wall_material(
    w: &World,
    cid: crate::ids::CommunityId,
    first_house: Option<TilePos>,
) -> Material {
    let stone_house = first_house
        .map(|o| w.grid.get(o))
        .is_some_and(|t| t.shape == Shape::Wall && t.material == Material::STONE_BLOCK);
    if stone_house {
        return Material::STONE_BLOCK;
    }
    let knows = |t: crate::mind::Technique| {
        w.agents
            .iter()
            .any(|a| a.alive && a.community == Some(cid) && a.techniques.knows(t))
    };
    if !knows(crate::mind::Technique::Carpentry) && knows(crate::mind::Technique::Masonry) {
        Material::STONE_BLOCK
    } else {
        Material::WOOD
    }
}

/// All edge tiles of the house at `origin` are walls (or the door).
pub fn house_complete(w: &World, origin: TilePos) -> bool {
    crate::houses::complete(w, &crate::houses::plan(w, origin))
}

/// Dig a stair shaft two levels down near home, then mine the rock around its foot.
/// #267: the deepest soil within two tiles of `p`'s column, so a gallery cut under it meets
/// rock on every side, not the soil of a level neighbour.
fn soil_round(w: &World, p: TilePos) -> i32 {
    (-2..=2)
        .flat_map(|dy| (-2..=2).map(move |dx| (dx, dy)))
        .map(|(dx, dy)| {
            let (x, y) = (
                (p.x + dx).clamp(0, w.grid.width - 1),
                (p.y + dy).clamp(0, w.grid.height - 1),
            );
            w.grid.soil_depth(w.grid.col(x, y))
        })
        .max()
        .unwrap_or(1)
}

fn quarry(w: &mut World, home: TilePos, cid: crate::ids::CommunityId) {
    let Some(site) = quarry_site(w, home, cid) else {
        return;
    };
    quarry_at(w, home, cid, site);
}

/// Where the community's quarry shaft is, chosen now if it has none (the mine of delving.rs
/// is sunk from the same shaft, #405).
pub(crate) fn quarry_site(
    w: &mut World,
    home: TilePos,
    cid: crate::ids::CommunityId,
) -> Option<TilePos> {
    let existing = w
        .communities
        .iter()
        .find(|c| c.id == cid)
        .and_then(|c| c.quarry_site);
    Some(match existing {
        Some(s) => s,
        None => {
            // Where the stone is (C71, D35): the nearest open tile to home, six or more
            // tiles off, with stone below the ground under it, not a set offset.
            let taken = occupied(w, true);
            // #267: under the soil, which lies two or three deep on level low ground.
            let stone_below = |p: TilePos| {
                let g = w.grid.ground_z(p.x, p.y);
                let t = w.grid.get(TilePos::new(p.x, p.y, g - soil_round(w, p)));
                t.shape == Shape::Wall
                    && matches!(
                        w.grid.materials.get(t.material).kind,
                        crate::world::material::MaterialKind::Stone
                            | crate::world::material::MaterialKind::Ore
                    )
            };
            // Rows across threads (#428: 29 ms at 500 people); the least (distance, tile) wins,
            // whichever row it is found in.
            let rows: Vec<i32> = (-25..=25).collect();
            let row_best = |dy: &i32| {
                let mut best: Option<(i32, TilePos)> = None;
                for dx in -25..=25 {
                    let p = w.grid.surface(home.x + dx, home.y + dy);
                    let d = p.dist(home);
                    // Not within two tiles of standing water (#390): a shaft dug beside a
                    // lake lets the lake in.
                    if !(6..=25).contains(&d)
                        // `taken` holds every living tree (`occupied(w, true)`).
                        || !crate::grounds::open_ground_trees_taken(w, p, &taken)
                        || !stone_below(p)
                        || crate::aquifer::near_water(w, p, 2)
                    {
                        continue;
                    }
                    if best.is_none_or(|(bd, bp)| (d, p) < (bd, bp)) {
                        best = Some((d, p));
                    }
                }
                best
            };
            let best = crate::par::map(&rows, row_best).into_iter().flatten().min();
            let (_, s) = best?;
            if let Some(c) = w.communities.iter_mut().find(|c| c.id == cid) {
                c.quarry_site = Some(s);
            }
            s
        }
    })
}

fn quarry_at(w: &mut World, home: TilePos, cid: crate::ids::CommunityId, site: TilePos) {
    // `site` is the surface tile (z = ground + 1) as it was when the site was chosen. Shaft
    // levels: that ground, down through the soil to the first level of rock (#264: `ground_z`
    // follows the dig, so the shaft went a level deeper every time stone was wanted, to z -11;
    // #267: the soil is one to three deep, and the gallery is cut in the rock under it).
    let g = site.z - 1;
    let soil = soil_round(w, site);
    let bottom = TilePos::new(site.x, site.y, g - soil);
    for z in (g - soil..=g).rev() {
        let level = TilePos::new(site.x, site.y, z);
        if w.grid.get(level).shape == Shape::Wall {
            post(w, DesignationKind::DigStairs, level, 7);
            return;
        }
    }
    // A lamp at the foot of the shaft once it is open.
    let foot = TilePos::new(site.x + 1, site.y, g - soil);
    if w.grid.walkable(foot)
        && !w.structures.iter().any(|s| s.pos == foot)
        && !w.designations.iter().any(|d| d.pos == foot)
    {
        post(
            w,
            DesignationKind::BuildStructure(StructureKind::Lamp, Material::WOOD),
            foot,
            6,
        );
    }
    // Gallery: mine stone neighbours of any open tile at the foot level, nearest first, as far
    // as the stone goes (C71: no set radius), while the stores want stone.
    let stone_stock: u32 = w
        .items
        .iter()
        .filter(|it| {
            matches!(it.kind, ItemKind::RoughStone | ItemKind::StoneBlock)
                && here_holds(w, home, it)
        })
        .map(|it| it.count)
        .sum();
    if stone_stock >= 24 {
        return;
    }
    // #405: a community that sank its shaft for harder stone cuts its stone there.
    let z = crate::delving::stone_level(w, cid).unwrap_or(g - soil);
    let mut cands: Vec<TilePos> = Vec::new();
    for dy in -10..=10 {
        for dx in -10..=10 {
            let p = TilePos::new(site.x + dx, site.y + dy, z);
            let t = w.grid.get(p);
            // Stone only: the gallery is cut for blocks (2026-10-28 round-16-deep: it took ore
            // too, and a coal seam at the gallery's level was cut 239 times in a 600-day census,
            // the coal piling up unburned and the blocks never coming).
            if t.shape != Shape::Wall
                || w.grid.materials.get(t.material).kind
                    != crate::world::material::MaterialKind::Stone
            {
                continue;
            }
            let adjacent_open = [(0, 1), (1, 0), (0, -1), (-1, 0)]
                .iter()
                .any(|(ax, ay)| w.grid.walkable(p.offset(*ax, *ay, 0)));
            if adjacent_open && !w.designations.iter().any(|d| d.pos == p) {
                cands.push(p);
            }
        }
    }
    cands.sort_by_key(|p| bottom.dist(*p));
    for p in cands.into_iter().take(3) {
        post(w, DesignationKind::Mine, p, 6);
    }
}

/// Whether an item lies within reach of `home` (the quarry's stone count, C71).
fn here_holds(_w: &World, home: TilePos, it: &crate::items::Item) -> bool {
    matches!(it.place, ItemPlace::Ground(p) if p.dist(home) <= 30)
}

fn has_designation(w: &World, f: impl Fn(&DesignationKind) -> bool) -> bool {
    w.designations
        .iter()
        .any(|d| d.community == w.meta.planning_for && f(&d.kind))
}

/// A construction site holds its own materials; field work does not (harvest dropped on
/// a field tile must still be hauled, or it piles up there: 600 stacks by day 120 in soak v5).
fn site_tile(w: &World, p: TilePos) -> bool {
    w.designations.iter().any(|d| {
        d.pos == p
            && !matches!(
                d.kind,
                DesignationKind::Haul
                    | DesignationKind::Chop
                    | DesignationKind::Mine
                    | DesignationKind::Sow
                    | DesignationKind::Harvest
                    | DesignationKind::DigEarth
                    | DesignationKind::DigChannel
                    | DesignationKind::Hunt(_)
                    | DesignationKind::Fish
                    | DesignationKind::Snare
                    | DesignationKind::Tame(_)
                    | DesignationKind::Cook
                    // What a bench made lies at the bench, free to carry (C67).
                    | DesignationKind::Craft(_)
            )
    })
}

fn item_kind(w: &World, id: crate::ids::ItemId) -> ItemKind {
    w.items
        .iter()
        .find(|it| it.id == id)
        .map(|it| it.kind)
        .unwrap_or(ItemKind::Log)
}

/// Where a community's things are: on the ground within reach of its home (45 tiles, as far as
/// work looks for tools and stock), and no nearer any other standing community's home.
pub(crate) struct Here {
    home: TilePos,
    others: Vec<TilePos>,
}

impl Here {
    pub(crate) fn of(w: &World, cid: crate::ids::CommunityId, home: TilePos) -> Here {
        Here {
            home,
            others: w
                .communities
                .iter()
                .filter(|c| c.id != cid && !c.dissolved)
                .map(|c| c.home)
                .collect(),
        }
    }

    pub(crate) fn holds(&self, p: TilePos) -> bool {
        let d = p.dist(self.home);
        d <= 45 && self.others.iter().all(|o| p.dist(*o) >= d)
    }

    pub fn holds_item(&self, it: &crate::items::Item) -> bool {
        matches!(it.place, ItemPlace::Ground(p) if self.holds(p))
    }
}

pub(crate) fn count_kind(w: &World, here: &Here, kind: ItemKind) -> u32 {
    w.items
        .iter()
        .filter(|it| it.kind == kind && here.holds_item(it))
        .map(|it| it.count)
        .sum()
}

/// Ground stacks of `kind` made of `m` (C33): tin ore, bronze bars.
pub(crate) fn count_of(w: &World, here: &Here, kind: ItemKind, m: Material) -> u32 {
    w.items
        .iter()
        .filter(|it| it.kind == kind && it.material == m && here.holds_item(it))
        .map(|it| it.count)
        .sum()
}

fn wanted_logs_for_sites(w: &World) -> u32 {
    w.designations
        .iter()
        .flat_map(|d| d.missing())
        .filter(|(k, _)| *k == ItemKind::Log)
        .map(|(_, n)| n)
        .sum()
}

pub(crate) fn wanted_blocks_for_sites(w: &World) -> u32 {
    w.designations
        .iter()
        .flat_map(|d| d.missing())
        .filter(|(k, _)| *k == ItemKind::StoneBlock)
        .map(|(_, n)| n)
        .sum()
}

/// Hours of hunger a person's food must satisfy in a day: hunger empties in six hours awake
/// (sixteen of them) and a quarter as fast asleep (mind/needs.rs).
pub const EATING_HOURS_A_DAY: f32 = 18.0;

/// Days of eating a community holds (#415): the food lying in its ground (`Here`) and what its
/// members carry, against `EATING_HOURS_A_DAY` a living member. Read by `decide` for whether
/// tomorrow's food is secured.
pub fn food_days(w: &World, cid: crate::ids::CommunityId) -> f32 {
    let Some(c) = w.communities.iter().find(|c| c.id == cid) else {
        return 0.0;
    };
    let here = Here::of(w, cid, c.home);
    let people = c
        .members
        .iter()
        .filter(|m| w.agents.iter().any(|a| a.id == **m && a.alive))
        .count()
        .max(1) as f32;
    let hours: f32 = w
        .items
        .iter()
        .filter(|it| it.kind.is_food())
        .filter(|it| match it.place {
            ItemPlace::Carried(id) => c.members.contains(&id),
            _ => here.holds_item(it),
        })
        .map(|it| it.count as f32 * it.kind.nutrition_hours())
        .sum();
    hours / (people * EATING_HOURS_A_DAY)
}

pub fn make(w: &mut World, kind: DesignationKind, pos: TilePos, priority: u8) -> Designation {
    let id = w.alloc_designation_id();
    let (work, reqs) = match kind {
        // Built of what was refined at a bench (C67, docs/design-rules.md section 2): a timber
        // wall of two planks (one log sawn), a stone wall of a dressed block.
        DesignationKind::BuildWall(m) => (
            times::WALL,
            if m == Material::WOOD {
                vec![(ItemKind::Plank, 2)]
            } else {
                vec![(ItemKind::StoneBlock, 1)]
            },
        ),
        DesignationKind::BuildDoor => (times::DOOR, vec![(ItemKind::Door, 1)]),
        DesignationKind::BuildStairs(m) => (
            times::BUILD_STAIRS,
            vec![(
                if m == Material::WOOD {
                    ItemKind::Plank
                } else {
                    ItemKind::StoneBlock
                },
                2,
            )],
        ),
        DesignationKind::BuildFloor(m) => (
            times::FLOOR,
            vec![(
                if m == Material::WOOD {
                    ItemKind::Plank
                } else {
                    ItemKind::StoneBlock
                },
                1,
            )],
        ),
        // A batten for either; thatch bundled from reeds, shingles split from planks.
        DesignationKind::BuildRoof(m) => (
            times::ROOF,
            if m == Material::THATCH {
                vec![(ItemKind::Plank, 1), (ItemKind::Thatch, 1)]
            } else {
                vec![(ItemKind::Plank, 1)]
            },
        ),
        // A shrine of the timber the place builds with, where it does not build in stone (#297).
        DesignationKind::BuildStructure(StructureKind::Shrine, Material::WOOD) => (
            times::structure(StructureKind::Shrine),
            vec![(ItemKind::Log, 4)],
        ),
        DesignationKind::BuildStructure(k, _) => (times::structure(k), structure_cost(k)),
        DesignationKind::Craft(r) => {
            let rc = crate::work::recipes::recipe(r);
            (
                rc.minutes * crate::time::TICKS_PER_MINUTE as u32,
                rc.inputs.to_vec(),
            )
        }
        DesignationKind::Chop => (times::CHOP, vec![]),
        DesignationKind::ClearRoof => (times::CLEAR_ROOF, vec![]),
        DesignationKind::Uproot => (times::UPROOT, vec![]),
        DesignationKind::Study(_) => (times::STUDY, vec![]),
        // #274: harder rock takes longer to cut (`MaterialDef::hardness`, 1.0 the base).
        DesignationKind::Mine => (crate::tiers::cut(w, pos, times::MINE), vec![]),
        DesignationKind::DigStairs => (crate::tiers::cut(w, pos, times::DIG_STAIRS), vec![]),
        DesignationKind::Hunt(_) => (times::HUNT, vec![]),
        DesignationKind::Fish => (times::FISH, vec![]),
        DesignationKind::Snare => (times::SNARE, vec![]),
        DesignationKind::Tame(_) => (times::HUNT, vec![]),
        DesignationKind::Tend(_) => (times::TEND, vec![]),
        DesignationKind::Douse => (times::DOUSE, vec![]),
        DesignationKind::FetchWater => (times::POUR, vec![]),
        DesignationKind::BeatOut => (times::BEAT, vec![]),
        DesignationKind::Sow => (times::SOW, vec![]),
        DesignationKind::Harvest => (times::HARVEST, vec![]),
        DesignationKind::DigEarth => (times::DIG_EARTH, vec![]),
        DesignationKind::DigChannel => (times::DIG_CHANNEL, vec![]),
        DesignationKind::Engrave => (times::ENGRAVE, vec![]),
        DesignationKind::Haul => (0, vec![]),
        DesignationKind::Cook => (times::COOK, vec![]),
    };
    Designation {
        id,
        kind,
        pos,
        community: w.meta.planning_for,
        requirements: reqs
            .into_iter()
            .enumerate()
            .map(|(ix, (item, needed))| Requirement {
                kind: item,
                needed,
                delivered: 0,
                material: crate::world::Material::AIR,
                want: match kind {
                    DesignationKind::Craft(r) => crate::work::recipes::recipe(r).want(ix),
                    _ => None,
                },
                quality: 0.0,
            })
            .collect(),
        work_left: work,
        work_total: work,
        max_workers: 1,
        priority,
        created: w.meta.tick,
        failures: 0,
        done: false,
        owner: None,
    }
}

/// Whether a live crop stands within a step of `p`: the weed there presses it and is worth
/// an Uproot order on a tile that is not being sown (#286).
pub(crate) fn weed_presses(w: &World, p: TilePos) -> bool {
    w.plants
        .iter()
        .any(|q| q.alive && q.kind.is_crop() && q.pos.z == p.z && q.pos.dist(p) <= 1)
}

pub(crate) fn post(w: &mut World, kind: DesignationKind, pos: TilePos, priority: u8) {
    let d = make(w, kind, pos, priority);
    w.designations.push(d);
}

/// A rectangle of flat, walkable, unoccupied ground near `near` (search outward).
/// Tiles a site may not use: anything built or ordered, plus (optionally) living trees.
/// Built once per search so the per-tile test is a set lookup, not a scan of every
/// structure, designation and plant (that scan was 20 ms an hour in the soak).
pub(crate) fn occupied(w: &World, trees: bool) -> std::collections::BTreeSet<TilePos> {
    let mut set: std::collections::BTreeSet<TilePos> = w.structures.iter().map(|s| s.pos).collect();
    set.extend(w.designations.iter().map(|d| d.pos));
    // Every community's home and the founding site, with a little ground round them: the place
    // everything else measures its way from (reach from home finds edges, tools, camps). A
    // house built over it once walled a village in from the world (seed 19, C12).
    let mut homes: Vec<TilePos> = w.communities.iter().map(|c| c.home).collect();
    homes.push(w.meta.founding_site);
    for h in homes {
        for dy in -2..=2 {
            for dx in -2..=2 {
                set.insert(h.offset(dx, dy, 0));
            }
        }
    }
    if trees {
        set.extend(
            w.plants
                .iter()
                .filter(|pl| pl.alive && pl.kind.is_tree())
                .map(|pl| pl.pos),
        );
    }
    set
}

pub fn find_flat_area(
    w: &World,
    near: TilePos,
    wdt: i32,
    hgt: i32,
    margin: i32,
) -> Option<TilePos> {
    find_flat_area_within(w, near, wdt, hgt, margin, 30)
}

/// `find_flat_area`, looking no further than `radius` rings from `near` (C60: a move's search
/// tries several far points; thirty rings at each cost a 500-person valley 168 ms).
pub fn find_flat_area_within(
    w: &World,
    near: TilePos,
    wdt: i32,
    hgt: i32,
    margin: i32,
    radius: i32,
) -> Option<TilePos> {
    let g = &w.grid;
    let taken = occupied(w, true);
    let mut best: Option<(i32, TilePos)> = None;
    for r in 0..radius {
        for dy in -r..=r {
            for dx in -r..=r {
                if dx.abs() != r && dy.abs() != r {
                    continue;
                }
                let x0 = near.x + dx;
                let y0 = near.y + dy;
                if x0 - margin < 1
                    || y0 - margin < 1
                    || x0 + wdt + margin >= g.width - 1
                    || y0 + hgt + margin >= g.height - 1
                {
                    continue;
                }
                let gz = g.ground_z(x0, y0);
                let mut ok = true;
                'scan: for y in (y0 - margin)..(y0 + hgt + margin) {
                    for x in (x0 - margin)..(x0 + wdt + margin) {
                        if g.ground_z(x, y) != gz {
                            ok = false;
                            break 'scan;
                        }
                        let p = TilePos::new(x, y, gz + 1);
                        let t = g.get(p);
                        if t.shape != Shape::Open || t.is_water() || !g.walkable(p) {
                            ok = false;
                            break 'scan;
                        }
                        if taken.contains(&p) {
                            ok = false;
                            break 'scan;
                        }
                    }
                }
                if ok
                    && !w
                        .nav
                        .as_ref()
                        .map(|n| n.reach(w.meta.founding_site, TilePos::new(x0, y0, gz + 1)))
                        .unwrap_or(true)
                {
                    ok = false;
                }
                if ok {
                    let d = near.dist(TilePos::new(x0, y0, gz + 1));
                    if best.map(|b| d < b.0).unwrap_or(true) {
                        best = Some((d, TilePos::new(x0, y0, gz + 1)));
                    }
                }
            }
        }
        if best.is_some() && r > 6 {
            break;
        }
    }
    best.map(|b| b.1)
}

/// Like `find_flat_area` but trees are allowed (they will be felled); water and buildings not.
pub fn find_flat_area_with_trees(w: &World, near: TilePos, wdt: i32, hgt: i32) -> Option<TilePos> {
    let g = &w.grid;
    let taken = occupied(w, false);
    let mut best: Option<(i32, TilePos)> = None;
    for r in 0..30i32 {
        for dy in -r..=r {
            for dx in -r..=r {
                if dx.abs() != r && dy.abs() != r {
                    continue;
                }
                let x0 = near.x + dx;
                let y0 = near.y + dy;
                if x0 < 2 || y0 < 2 || x0 + wdt >= g.width - 2 || y0 + hgt >= g.height - 2 {
                    continue;
                }
                let gz = g.ground_z(x0, y0);
                let mut ok = true;
                // The footprint and a ring of one tile round it (C57): without the ring a house
                // was set wall to wall against another's door, and the beds of both were out
                // of reach (seed 3's soak: 55 bed orders failing, 58 people, 3 places to sleep).
                'scan: for y in (y0 - 1)..(y0 + hgt + 1) {
                    for x in (x0 - 1)..(x0 + wdt + 1) {
                        let ring = x < x0 || y < y0 || x >= x0 + wdt || y >= y0 + hgt;
                        let p = TilePos::new(x, y, gz + 1);
                        let t = g.get(p);
                        if ring {
                            if t.shape == Shape::Wall
                                || taken.contains(&p) && !w.plants.iter().any(|pl| pl.pos == p)
                            {
                                ok = false;
                                break 'scan;
                            }
                            continue;
                        }
                        if g.ground_z(x, y) != gz {
                            ok = false;
                            break 'scan;
                        }
                        if t.shape != Shape::Open
                            || t.is_water()
                            || !g.walkable(p)
                            || taken.contains(&p)
                        {
                            ok = false;
                            break 'scan;
                        }
                    }
                }
                if ok
                    && !w
                        .nav
                        .as_ref()
                        .map(|n| n.reach(w.meta.founding_site, TilePos::new(x0, y0, gz + 1)))
                        .unwrap_or(true)
                {
                    ok = false;
                }
                if ok {
                    let d = near.dist(TilePos::new(x0, y0, gz + 1));
                    if best.map(|b| d < b.0).unwrap_or(true) {
                        best = Some((d, TilePos::new(x0, y0, gz + 1)));
                    }
                }
            }
        }
        if best.is_some() && r > 6 {
            break;
        }
    }
    best.map(|b| b.1)
}

/// The nearest solid rock tile that a person can stand next to (an exposed face).
/// The trees the planner would fell for the community at `home`: mature, standing within 40,
/// with a reachable side and no order on them; failing those, the caverns' fungus trees within
/// 60 (C58). Nearest first. The fetch planner asks the same (#164).
pub fn chop_candidates(w: &World, home: TilePos) -> Vec<TilePos> {
    let mut trees: Vec<TilePos> = w
        .plants
        .iter()
        .filter(|p| p.alive && p.kind.is_tree() && p.mature() && home.dist(p.pos) <= 40)
        .map(|p| p.pos)
        .filter(|p| {
            let g = &w.grid;
            let nav = w.nav.as_ref();
            [(0, 1), (1, 0), (0, -1), (-1, 0)].iter().any(|(dx, dy)| {
                let q = p.offset(*dx, *dy, 0);
                g.walkable(q) && nav.map(|n| n.reach(home, q)).unwrap_or(false)
            })
        })
        .filter(|p| !w.designations.iter().any(|d| d.pos == *p))
        .collect();
    // No timber left above: the fungus trees of the caverns are wood too (C58, the C19
    // "cavern settling" gap), if a way down reaches them.
    if trees.is_empty() {
        let nav = w.nav.as_ref();
        trees = w
            .plants
            .iter()
            .filter(|p| {
                p.alive
                    && p.kind == PlantKind::FungusTree
                    && p.mature()
                    && (p.pos.x - home.x).abs().max((p.pos.y - home.y).abs()) <= 60
            })
            .map(|p| p.pos)
            .filter(|p| {
                [(0, 1), (1, 0), (0, -1), (-1, 0)].iter().any(|(dx, dy)| {
                    let q = p.offset(*dx, *dy, 0);
                    w.grid.walkable(q) && nav.map(|n| n.reach(home, q)).unwrap_or(false)
                })
            })
            .filter(|p| !w.designations.iter().any(|d| d.pos == *p))
            .collect();
    }
    trees.sort_by_key(|p| home.dist(*p));
    trees
}

/// Mature green trees (`growth >= 0.95`, `Plant::mature`), young green trees and stumps,
/// valley-wide and within 40 tiles of any living community's home — the planner's felling
/// radius (`chop_candidates` above). For the ecology census (#1361): a run that strips the
/// woods around a settlement should show it here even while the valley-wide count still looks
/// fine.
pub fn tree_census(w: &World) -> TreeCensus {
    let homes: Vec<TilePos> = w
        .communities
        .iter()
        .filter(|c| !c.dissolved)
        .map(|c| c.home)
        .collect();
    let near_home = |p: TilePos| homes.iter().any(|h| h.dist(p) <= 40);
    let mut c = TreeCensus::default();
    for p in w.plants.iter().filter(|p| p.alive) {
        if p.kind.is_green_tree() {
            let (all, near) = if p.mature() {
                (&mut c.mature, &mut c.mature_near_home)
            } else {
                (&mut c.young, &mut c.young_near_home)
            };
            *all += 1;
            if near_home(p.pos) {
                *near += 1;
            }
        } else if p.kind == PlantKind::Stump {
            c.stumps += 1;
            if near_home(p.pos) {
                c.stumps_near_home += 1;
            }
        }
    }
    c
}

#[derive(Default, Clone, Copy, Debug)]
pub struct TreeCensus {
    pub mature: usize,
    pub young: usize,
    pub stumps: usize,
    pub mature_near_home: usize,
    pub young_near_home: usize,
    pub stumps_near_home: usize,
}

pub fn nearest_rock_face(w: &World, near: TilePos, radius: i32) -> Option<TilePos> {
    let g = &w.grid;
    let mut best: Option<(i32, TilePos)> = None;
    let nav = w.nav.as_ref()?;
    for y in (near.y - radius).max(1)..(near.y + radius).min(g.height - 1) {
        for x in (near.x - radius).max(1)..(near.x + radius).min(g.width - 1) {
            for z in (near.z - 1)..=(near.z + 2) {
                let p = TilePos::new(x, y, z);
                let t = g.get(p);
                // Stone only: it is for blocks (ore has its own rule, `nearest_face_of`, #177).
                if t.shape != Shape::Wall
                    || g.materials.get(t.material).kind
                        != crate::world::material::MaterialKind::Stone
                {
                    continue;
                }
                if w.designations.iter().any(|d| d.pos == p) {
                    continue;
                }
                let stand = [(0, -1), (1, 0), (0, 1), (-1, 0)]
                    .iter()
                    .map(|(dx, dy)| p.offset(*dx, *dy, 0))
                    .find(|q| g.walkable(*q) && nav.reach(near, *q));
                if stand.is_some() {
                    let d = near.dist(p);
                    if best.map(|b| d < b.0).unwrap_or(true) {
                        best = Some((d, p));
                    }
                }
            }
        }
    }
    best.map(|b| b.1)
}

/// The home a proposal asked for has its place (C57).
fn set_home_site(w: &mut World, pid: u32, site: TilePos) {
    if let Some(p) = w.projects.iter_mut().find(|p| p.id == pid) {
        p.site = Some(site);
    }
}

/// Every tile of a hollow's mouth and room is dug out.
fn hollow_dug(w: &World, h: &crate::society::Hollow) -> bool {
    std::iter::once(h.mouth)
        .chain(h.room.iter().copied())
        .all(|p| {
            w.grid.get(p).shape != Shape::Wall
                || w.grid.get(p).flags.has(crate::world::TileFlags::DOOR)
        })
}

/// Whether the home a proposal asked for stands (C57): a house roofed, a hollow dug with its
/// door, the tents or nests raised.
pub fn home_built(
    w: &World,
    cid: crate::ids::CommunityId,
    form: crate::project::HomeForm,
    site: TilePos,
) -> bool {
    use crate::project::HomeForm as F;
    let Some(c) = w.communities.iter().find(|c| c.id == cid) else {
        return true;
    };
    match form {
        F::House => c
            .house_sites
            .iter()
            .position(|o| *o == site)
            .map(|k| {
                let stage = c.house_stage.get(k).copied().unwrap_or(0);
                stage >= 3 || (stage == 0 && house_complete(w, site))
            })
            .unwrap_or(true),
        F::Hollow => c
            .hollows
            .iter()
            .find(|h| h.mouth == site)
            .map(|h| {
                hollow_dug(w, h)
                    && h.door()
                        .is_none_or(|d| w.grid.get(d).flags.has(crate::world::TileFlags::DOOR))
            })
            .unwrap_or(true),
        F::Tent | F::Nest => !w.designations.iter().any(|d| {
            !d.done
                && d.community == Some(cid)
                && matches!(
                    d.kind,
                    DesignationKind::BuildStructure(StructureKind::Tent | StructureKind::Nest, _)
                )
        }),
    }
}

/// A rock face near home with room behind it for a hollow (C57): a mouth tile with walkable,
/// reachable ground before it, and behind it solid rock under rock enough for a room that
/// `enough` accepts, dug a tile at a time straight in first (C67: the room is the size its
/// household needs, not three rows of three). The nearest to home.
pub fn find_hollow_site(
    w: &World,
    near: TilePos,
    radius: i32,
    enough: &(dyn Fn(&[TilePos], TilePos) -> bool + Sync),
) -> Option<(TilePos, Vec<TilePos>)> {
    let g = &w.grid;
    let nav = w.nav.as_ref()?;
    let rock = |p: TilePos| {
        g.in_bounds(p) && {
            let t = g.get(p);
            t.shape == Shape::Wall
                && g.materials.get(t.material).kind == crate::world::material::MaterialKind::Stone
        }
    };
    let taken: std::collections::BTreeSet<TilePos> = w.designations.iter().map(|d| d.pos).collect();
    // Each row searched on its own, across threads, and the rows' bests taken in row order
    // (#428): the nearest, and of the nearest the first in the scan, as one walk would find.
    let rows: Vec<i32> = ((near.y - radius).max(2)..(near.y + radius).min(g.height - 2)).collect();
    let row_best = |y: &i32| {
        let y = *y;
        let mut best: Option<(i32, TilePos, Vec<TilePos>)> = None;
        for x in (near.x - radius).max(2)..(near.x + radius).min(g.width - 2) {
            for z in (near.z - 1)..=(near.z + 1) {
                let mouth = TilePos::new(x, y, z);
                if !rock(mouth) || taken.contains(&mouth) || !rock(mouth.offset(0, 0, 1)) {
                    continue;
                }
                let d = near.dist(mouth);
                if best.as_ref().is_some_and(|b| d >= b.0) {
                    continue;
                }
                for (dx, dy) in [(0, -1), (1, 0), (0, 1), (-1, 0)] {
                    let stand = mouth.offset(-dx, -dy, 0);
                    if !g.in_bounds(stand) || !g.walkable(stand) || !nav.reach(near, stand) {
                        continue;
                    }
                    let ok = |p: TilePos| {
                        p != mouth
                            && p.z == z
                            && rock(p)
                            && rock(p.offset(0, 0, 1))
                            && !taken.contains(&p)
                            // Not out through the face beside the mouth.
                            && (p.x - mouth.x) * dx + (p.y - mouth.y) * dy >= 1
                            && [(1, 0), (-1, 0), (0, 1), (0, -1)]
                                .iter()
                                .all(|(ex, ey)| !g.walkable(p.offset(*ex, *ey, 0)))
                    };
                    let room = crate::houses::dig_room(
                        &[],
                        Some(mouth.offset(dx, dy, 0)),
                        mouth.offset(dx * 3, dy * 3, 0),
                        &ok,
                        &|r| enough(r, mouth),
                    );
                    if let Some(room) = room {
                        best = Some((d, mouth, room));
                        break;
                    }
                }
            }
        }
        best
    };
    nearest_of_rows(crate::par::map(&rows, row_best))
}

/// Flat ground near home to dig a home down into where there is no rock face (C67): a shaft
/// two levels down through the ground a person stands on, and at its foot a room that
/// `enough` accepts, dug through solid ground with solid ground above it. Returns the top of
/// the shaft and the room, the foot of the shaft first. The nearest to home, and not on the
/// home tile's own doorstep.
pub fn find_dugout_site(
    w: &World,
    near: TilePos,
    radius: i32,
    enough: &(dyn Fn(&[TilePos], TilePos) -> bool + Sync),
) -> Option<(TilePos, Vec<TilePos>)> {
    dugout_site(w, near, radius, enough, false)
}

/// A dugout whose room is reached from the foot of its stair through one tile only (#405): the
/// room's second tile, where a door can bar the way, as a shelter's mouth does. The room is
/// returned as `find_dugout_site` returns it, the foot first and the door's tile second.
pub fn find_barred_dugout_site(
    w: &World,
    near: TilePos,
    radius: i32,
    enough: &(dyn Fn(&[TilePos], TilePos) -> bool + Sync),
) -> Option<(TilePos, Vec<TilePos>)> {
    dugout_site(w, near, radius, enough, true)
}

fn dugout_site(
    w: &World,
    near: TilePos,
    radius: i32,
    enough: &(dyn Fn(&[TilePos], TilePos) -> bool + Sync),
    door: bool,
) -> Option<(TilePos, Vec<TilePos>)> {
    let g = &w.grid;
    let nav = w.nav.as_ref()?;
    let solid = |p: TilePos| {
        g.in_bounds(p) && {
            let t = g.get(p);
            t.shape == Shape::Wall && !t.flags.has(crate::world::TileFlags::DOOR)
        }
    };
    let taken: std::collections::BTreeSet<TilePos> = w.designations.iter().map(|d| d.pos).collect();
    // By rows across threads, as `find_hollow_site` (#428: 34 ms at 500 people).
    let rows: Vec<i32> = ((near.y - radius).max(3)..(near.y + radius).min(g.height - 3)).collect();
    let row_best = |y: &i32| {
        let y = *y;
        let mut best: Option<(i32, TilePos, Vec<TilePos>)> = None;
        for x in (near.x - radius).max(3)..(near.x + radius).min(g.width - 3) {
            let gz = g.ground_z(x, y);
            let stand = TilePos::new(x, y, gz + 1);
            let d = near.dist(stand);
            if d < 3 || best.as_ref().is_some_and(|b| d >= b.0) {
                continue;
            }
            let top = TilePos::new(x, y, gz);
            let foot = top.offset(0, 0, -1);
            if foot.z <= g.z_min
                || !g.walkable(stand)
                || taken.contains(&stand)
                || w.structures.iter().any(|s| s.pos == stand)
                || !solid(top)
                || !solid(foot)
                || taken.contains(&top)
                || taken.contains(&foot)
                || crate::aquifer::near_water(w, stand, 2)
                || !nav.reach(near, stand)
            {
                continue;
            }
            // Barred: the door's tile is the only one beside the foot.
            let first = foot.offset(0, 1, 0);
            let ok = |p: TilePos| {
                p.z == foot.z
                    && p != foot
                    && g.in_bounds(p)
                    && solid(p)
                    && solid(p.offset(0, 0, 1))
                    && g.ground_z(p.x, p.y) == gz
                    && !taken.contains(&p)
                    && [(1, 0), (-1, 0), (0, 1), (0, -1)]
                        .iter()
                        .all(|(ex, ey)| !g.walkable(p.offset(*ex, *ey, 0)))
                    && (!door || p == first || (p.x - foot.x).abs() + (p.y - foot.y).abs() > 1)
            };
            let room = crate::houses::dig_room(
                &[foot],
                door.then_some(first),
                foot.offset(0, 2, 0),
                &ok,
                &|r| enough(&r[1..], foot),
            );
            if let Some(room) = room {
                best = Some((d, top, room));
            }
        }
        best
    };
    nearest_of_rows(crate::par::map(&rows, row_best))
}

/// The best of each row's best, rows in scan order: the nearest, the earliest row on a tie
/// (a row's best is already its earliest at its distance).
fn nearest_of_rows(
    rows: Vec<Option<(i32, TilePos, Vec<TilePos>)>>,
) -> Option<(TilePos, Vec<TilePos>)> {
    let mut best: Option<(i32, TilePos, Vec<TilePos>)> = None;
    for r in rows.into_iter().flatten() {
        if best.as_ref().is_none_or(|b| r.0 < b.0) {
            best = Some(r);
        }
    }
    best.map(|(_, m, r)| (m, r))
}

/// How many tiles a shelter for the stores is dug to (C67): half a tile a person, four at
/// least and sixteen at most.
pub fn shelter_room(people: u32) -> usize {
    (people as usize / 2).clamp(4, 16)
}

/// Whether `p` lies in a shelter's room behind its door (C58): what is kept there raiders do
/// not reach.
pub fn in_shelter(w: &World, p: TilePos) -> bool {
    w.communities.iter().any(|c| {
        c.hollows.iter().any(|h| {
            // Behind the door: for one dug down, past the stair's foot and the door's tile.
            h.shelter
                && h.room
                    .iter()
                    .skip(if h.dug_down { 2 } else { 0 })
                    .any(|q| *q == p)
                && h.door()
                    .is_some_and(|d| w.grid.get(d).flags.has(crate::world::TileFlags::DOOR))
        })
    })
}

/// A community's way of building (C58), if its first house has set one: the wood it builds of
/// and the roof it lays.
pub fn building_way(
    w: &World,
    cid: crate::ids::CommunityId,
) -> Option<(Option<Material>, Material)> {
    w.customs.iter().find_map(|c| match c.kind {
        crate::culture::CustomKind::Building { wood, roof } if c.community == cid && !c.lapsed => {
            Some((wood, roof))
        }
        _ => None,
    })
}

/// Whether this community has learned to favour a needed upper room, and the custom that records
/// where it learned that preference.
fn upper_storey_way(w: &World, cid: crate::ids::CommunityId) -> Option<&crate::culture::Custom> {
    w.customs.iter().find(|c| {
        c.community == cid && !c.lapsed && matches!(c.kind, crate::culture::CustomKind::UpperStorey)
    })
}

fn note_upper_storey_way(
    w: &mut World,
    cid: crate::ids::CommunityId,
    origin: TilePos,
    event: crate::ids::EventId,
    reason: Option<crate::society::UpperStoreyReason>,
) {
    let why = reason.map_or("a need for another room", |r| r.label());
    let narrative = format!(
        "We build upward when another room is needed, as the upper room at {origin} proved for {why}."
    );
    let tick = w.meta.tick;
    crate::systems::culture::add_custom(
        w,
        cid,
        crate::culture::CustomKind::UpperStorey,
        event,
        tick,
        narrative,
    );
}

/// When a house is first roofed (C58): the first sets the community's way of building, the
/// wood most of its walls are of and the roof it was given, recorded from that event; each
/// later one keeps the way alive.
fn note_building_way(
    w: &mut World,
    cid: crate::ids::CommunityId,
    origin: TilePos,
    event: crate::ids::EventId,
) {
    if building_way(w, cid).is_some() {
        for c in w.customs.iter_mut().filter(|c| {
            c.community == cid
                && !c.lapsed
                && matches!(c.kind, crate::culture::CustomKind::Building { .. })
        }) {
            c.intensity = c.intensity.max(0.8);
        }
        return;
    }
    let mut woods: std::collections::BTreeMap<Material, u32> = Default::default();
    for (q, _) in crate::houses::walls(&crate::houses::plan(w, origin).floor) {
        if let Some(m) = w.made_of(q) {
            if w.grid.materials.get(m).kind == crate::world::material::MaterialKind::Wood {
                *woods.entry(m).or_default() += 1;
            }
        }
    }
    let wood = woods
        .iter()
        .max_by(|a, b| a.1.cmp(b.1).then(b.0.cmp(a.0)))
        .map(|(m, _)| *m);
    let Some(roof) = house_interior(w, origin)
        .iter()
        .find_map(|p| w.roofs.get(p).map(|r| r.material))
    else {
        return;
    };
    let way = crate::culture::CustomKind::Building { wood, roof };
    let label = crate::culture::Custom {
        id: crate::culture::CustomId(0),
        community: cid,
        kind: way.clone(),
        origin: event,
        origin_tick: w.meta.tick,
        narrative: String::new(),
        intensity: 0.8,
        adopted_by: Vec::new(),
        formed: 0,
        lapsed: false,
        version: 1,
    }
    .label();
    let narrative = format!("We {label}, as our first house was built.");
    let tick = w.meta.tick;
    crate::systems::culture::add_custom(w, cid, way, event, tick, narrative);
}

#[cfg(test)]
mod upper_storey_reason_tests {
    use super::upper_storey_reason;
    use crate::society::UpperStoreyReason;

    #[test]
    fn upper_storey_reason_has_a_recorded_cause_and_a_control_case() {
        assert_eq!(
            upper_storey_reason(false, false, false),
            None,
            "a household that fits on the ground must not grow an unnecessary upper floor"
        );
        assert_eq!(
            upper_storey_reason(false, true, true),
            Some(UpperStoreyReason::CrampedSite)
        );
        assert_eq!(
            upper_storey_reason(false, false, true),
            Some(UpperStoreyReason::GrownHousehold)
        );
        assert_eq!(
            upper_storey_reason(true, true, true),
            Some(UpperStoreyReason::CulturalPreference)
        );
    }
}
