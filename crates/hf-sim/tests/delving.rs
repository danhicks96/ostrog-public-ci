//! #405: the mine sunk below the quarry. A community digs deeper for a reason it has, follows
//! what it finds, and a gallery that opens into a deep cavern is the breach; then the community
//! walls the way or digs on, as it leans.
//!
//! The planner (`delving::plan_now`) is run as the game runs it; the digging of its orders is
//! done here at once (`dig_out`), as the miners would do it, so the test follows the shaft all
//! the way down without the days the work takes.

use hf_sim::delving::{self, Seek};
use hf_sim::events::EventKind;
use hf_sim::ids::{CommunityId, TilePos};
use hf_sim::items::{ItemKind, ItemPlace};
use hf_sim::mind::beliefs::{Source, Topic};
use hf_sim::work::DesignationKind;
use hf_sim::world::{Material, Shape, Tile, TileFlags};
use hf_sim::World;

/// A community of eight or more and a deep cavern in its valley.
fn valley() -> (World, CommunityId, usize) {
    for seed in 1..40 {
        let mut w = World::generate(seed, hf_sim::world::gen::GenParams::default());
        let Some(k) = w.grid.caverns.iter().position(|c| c.deep) else {
            continue;
        };
        let party = w.found_party(10);
        let cid = w
            .agents
            .iter()
            .find(|a| a.id == party[0])
            .and_then(|a| a.community)
            .unwrap();
        return (w, cid, k);
    }
    panic!("no valley with a deep cavern and a community of eight");
}

/// Everyone of the community comes to hold (or deny) that the deep is there to be dug, and
/// has ore enough on hand that the smelter is no reason to dig.
fn lean(w: &mut World, cid: CommunityId, toward: f32) {
    for a in w.agents.iter_mut().filter(|a| a.community == Some(cid)) {
        let id = a.id;
        for _ in 0..30 {
            a.beliefs
                .experience(Topic::DeepDigging, toward, 1.0, Source::Person(id), 0);
        }
    }
    let home = w.communities.iter().find(|c| c.id == cid).unwrap().home;
    for (k, m) in [
        (ItemKind::IronOre, Material::IRON_ORE),
        (ItemKind::CopperOre, Material::COPPER_ORE),
    ] {
        w.create_item_of(k, m, 4, ItemPlace::Ground(home), 1.0, "test");
    }
    // Settled a year.
    w.meta.tick += hf_sim::time::TICKS_PER_YEAR;
}

/// A surface tile with unbroken rock under it to the bottom of the world, `off` tiles east of
/// the cavern's edge: where the shaft is sunk.
fn site_beside(w: &World, k: usize, off: i32) -> TilePos {
    let c = &w.grid.caverns[k];
    for dx in off..off + 12 {
        for dy in [0, 1, -1, 2, -2, 3, -3] {
            let (x, y) = (c.centre.x + c.rx + dx, c.centre.y + dy);
            if x < 2 || y < 2 || x >= w.grid.width - 2 || y >= w.grid.height - 2 {
                continue;
            }
            let s = w.grid.surface(x, y);
            let solid =
                (w.grid.z_min..s.z).all(|z| w.grid.get(TilePos::new(x, y, z)).shape == Shape::Wall);
            if w.grid.walkable(s) && !w.grid.get(s).is_water() && solid {
                return s;
            }
        }
    }
    panic!("no site beside the cavern");
}

fn mine(w: &World, cid: CommunityId) -> delving::Mine {
    w.communities
        .iter()
        .find(|c| c.id == cid)
        .unwrap()
        .mine
        .clone()
        .expect("a mine")
}

/// The rows pushed since the last call, made the last tick's, and the observers run on them.
fn settle(w: &mut World) {
    w.last_events = std::mem::take(&mut w.events.pending);
    delving::observe(w);
    hf_sim::tiers::observe(w);
}

/// The rows the observers wrote about the last tick's, kept with them.
fn answered(w: &mut World) {
    let more = std::mem::take(&mut w.events.pending);
    w.last_events.extend(more);
}

/// Every open order of the mine done, as work.rs does it: the rock cut, the stair dug, a
/// `mined` row for each.
fn dig_out(w: &mut World, cid: CommunityId) {
    let m = mine(w, cid);
    let agent = w
        .agents
        .iter()
        .find(|a| a.alive && a.community == Some(cid))
        .unwrap()
        .id;
    let mut done = Vec::new();
    for k in 0..w.designations.len() {
        let d = &w.designations[k];
        let ours = m.cut.contains(&d.pos)
            || (d.pos.x == m.site.x && d.pos.y == m.site.y && d.kind == DesignationKind::DigStairs);
        if d.done || !ours {
            continue;
        }
        let p = d.pos;
        match d.kind {
            DesignationKind::Mine => {
                w.grid.set(p, Tile::AIR);
            }
            DesignationKind::DigStairs => {
                let t = w.grid.get(p);
                w.grid.set(
                    p,
                    Tile {
                        material: t.material,
                        shape: Shape::StairsUp,
                        flags: TileFlags::NONE,
                    },
                );
                let a = w.grid.get(p.offset(0, 0, 1));
                w.grid.set(
                    p.offset(0, 0, 1),
                    Tile {
                        material: a.material,
                        shape: if a.shape == Shape::StairsUp {
                            Shape::StairsBoth
                        } else {
                            Shape::StairsDown
                        },
                        flags: TileFlags::NONE,
                    },
                );
            }
            _ => continue,
        }
        w.dug.insert(p);
        w.designations[k].done = true;
        done.push(p);
    }
    for p in done {
        let tick = w.meta.tick;
        w.events.push(
            tick,
            Some(p),
            hf_sim::events::Severity::Minor,
            EventKind::Mined {
                agent,
                at: p,
                found: None,
                rock: None,
            },
        );
    }
    settle(w);
}

/// Sink the mine hour by hour until `stop` holds or it rests worked out.
fn sink(w: &mut World, cid: CommunityId, stop: impl Fn(&World, &delving::Mine) -> bool) {
    for _ in 0..600 {
        delving::plan_now(w, cid);
        let m = mine(w, cid);
        if stop(w, &m) || m.rest_until > w.meta.tick {
            return;
        }
        dig_out(w, cid);
        w.meta.tick += hf_sim::time::TICKS_PER_HOUR;
    }
}

/// The greedy community's mine down to a deep cavern beside its shaft, and the breach.
fn to_the_breach(toward: f32) -> (World, CommunityId, usize) {
    let (mut w, cid, k) = valley();
    lean(&mut w, cid, toward);
    let site = site_beside(&w, k, 8);
    w.communities
        .iter_mut()
        .find(|c| c.id == cid)
        .unwrap()
        .quarry_site = Some(site);
    sink(&mut w, cid, |_, m| m.look.is_some());
    let m = mine(&w, cid);
    assert_eq!(
        m.seek,
        Some(Seek::Deep),
        "sunk for the deep's gems and gold"
    );
    assert!(m.deep, "the shaft came down into the deep's rock");
    assert_eq!(m.look, Some(k), "a gallery opened into the deep cavern");
    // Someone goes in to look: the planner sends them; here they are put there.
    let i = w
        .agents
        .iter()
        .position(|a| a.alive && a.community == Some(cid) && !a.is_child(0))
        .unwrap();
    let floor = hf_sim::underground::floors(&w, &w.grid.caverns[k].clone())
        .next()
        .unwrap();
    let from = m
        .cut
        .iter()
        .copied()
        .filter(|p| !w.grid.get(*p).shape.is_solid())
        .min_by_key(|p| p.dist(floor))
        .unwrap();
    w.agents[i].from = from;
    w.agents[i].pos = floor;
    hf_sim::underground::breaches(&mut w);
    settle(&mut w);
    answered(&mut w);
    (w, cid, k)
}

#[test]
fn a_people_who_hold_the_deep_is_to_be_dug_dig_down_to_it_and_dig_on() {
    let (w, cid, k) = to_the_breach(1.0);
    let names: Vec<&str> = w.last_events.iter().map(|e| e.kind_name()).collect();
    assert!(names.contains(&"breached"), "{names:?}");
    assert!(names.contains(&"cavern_found"), "{names:?}");
    let beast = names.contains(&"beast_stirred");
    let chose = w
        .last_events
        .iter()
        .find_map(|e| match &e.kind {
            EventKind::DeepChosen {
                community, walled, ..
            } if *community == cid => Some(*walled),
            _ => None,
        })
        .expect("the choice is a row");
    // Greedy, they leave it open, unless a beast stirred: then everyone walls it.
    assert_eq!(chose, beast);
    assert_eq!(mine(&w, cid).chose, Some(chose));
    assert!(w.grid.caverns[k].opened);
}

#[test]
fn a_cautious_community_walls_the_breach() {
    let (w, cid, k) = to_the_breach_cautious();
    let walled = w.last_events.iter().any(|e| {
        matches!(&e.kind, EventKind::DeepChosen { community, walled: true, .. } if *community == cid)
    });
    assert!(walled, "a cautious community walls the way");
    let entry = w.grid.caverns[k].breach.expect("the way in");
    assert!(w.designations.iter().any(|d| d.pos == entry
        && matches!(d.kind, DesignationKind::BuildWall(_))
        && d.community == Some(cid)));
}

/// A cautious community does not sink its shaft into the deep for its own sake: here it came
/// there for ore, and broke into the cavern following a vein.
fn to_the_breach_cautious() -> (World, CommunityId, usize) {
    let (mut w, cid, k) = to_the_breach(1.0);
    // They came to fear it after: the choice is made on how they lean when they break in. Run
    // the choice again for a community leaning against.
    for a in w.agents.iter_mut().filter(|a| a.community == Some(cid)) {
        let id = a.id;
        for _ in 0..60 {
            a.beliefs
                .experience(Topic::DeepDigging, -1.0, 1.0, Source::Person(id), 0);
        }
    }
    assert!(delving::stance(&w, cid) < 0.0);
    let agent = w
        .agents
        .iter()
        .find(|a| a.community == Some(cid))
        .unwrap()
        .id;
    let cavern = w.grid.caverns[k].name.clone();
    w.designations
        .retain(|d| !matches!(d.kind, DesignationKind::BuildWall(_)));
    let tick = w.meta.tick;
    w.events.push(
        tick,
        None,
        hf_sim::events::Severity::Major,
        EventKind::Breached { agent, cavern },
    );
    settle(&mut w);
    answered(&mut w);
    (w, cid, k)
}

#[test]
fn a_shaft_sunk_for_ore_stops_where_the_vein_shows_and_follows_it() {
    let (mut w, cid, k) = valley();
    // Ore wanted and a smelter's craft known; none showing.
    let i = w
        .agents
        .iter()
        .position(|a| a.alive && a.community == Some(cid))
        .unwrap();
    w.agents[i].techniques.learn(
        hf_sim::mind::Technique::Smelting,
        hf_sim::mind::techniques::Learned::Past,
        0.5,
        0,
    );
    let site = site_beside(&w, k, 8);
    w.communities
        .iter_mut()
        .find(|c| c.id == cid)
        .unwrap()
        .quarry_site = Some(site);
    // A smelter standing at home.
    let home = w.communities.iter().find(|c| c.id == cid).unwrap().home;
    let id = w.alloc_structure_id();
    w.structures.push(hf_sim::structures::Structure {
        id,
        kind: hf_sim::structures::StructureKind::Smelter,
        pos: home.offset(2, 2, 0),
        material: Material::STONE_BLOCK,
        progress: 1.0,
        active: false,
        condition: 1.0,
        owner: None,
        quality: 0.5,
    });
    // A vein of iron four levels under the ground, four tiles off the shaft.
    let z = site.z - 1 - 5;
    let vein: Vec<TilePos> = (4..8)
        .map(|dx| TilePos::new(site.x + dx, site.y, z))
        .collect();
    for p in &vein {
        let t = w.grid.get(*p);
        w.grid.set(
            *p,
            Tile {
                material: Material::IRON_ORE,
                ..t
            },
        );
    }
    sink(&mut w, cid, |w, _| {
        vein.iter().all(|p| !w.grid.get(*p).shape.is_solid())
    });
    let m = mine(&w, cid);
    assert_eq!(m.seek, Some(Seek::Ore));
    assert!(
        vein.iter().all(|p| !w.grid.get(*p).shape.is_solid()),
        "the vein was followed to its end"
    );
    assert_eq!(m.foot.z, z, "the shaft stopped at the vein's level");
    assert!(!m.deep);
}

/// A coal face beside the camp, open to be cut, with and without coal in the store and a
/// smelter to burn it in.
fn coal_face(coal: u32, smelter: bool) -> bool {
    let mut w = World::generate(41, hf_sim::world::gen::GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.plants.clear();
    w.meta.founding_site = TilePos::new(14, 14, 2);
    w.meta.tick = hf_sim::time::TICKS_PER_HOUR * 7;
    w.rebuild_caches();
    let ids = w.found_party(10);
    let cid = w
        .agents
        .iter()
        .find(|a| a.id == ids[0])
        .and_then(|a| a.community)
        .unwrap();
    w.meta.autoplan = true;
    let home = w.communities.iter().find(|c| c.id == cid).unwrap().home;
    let face = TilePos::new(home.x + 4, home.y + 4, home.z);
    w.grid.set(
        face,
        Tile {
            material: Material::COAL,
            shape: Shape::Wall,
            flags: TileFlags::NONE,
        },
    );
    if smelter {
        let id = w.alloc_structure_id();
        w.structures.push(hf_sim::structures::Structure {
            id,
            kind: hf_sim::structures::StructureKind::Smelter,
            pos: home.offset(-4, 2, 0),
            material: Material::STONE_BLOCK,
            progress: 1.0,
            active: false,
            condition: 1.0,
            owner: None,
            quality: 0.5,
        });
    }
    if coal > 0 {
        w.create_item_of(
            ItemKind::Coal,
            Material::COAL,
            coal,
            ItemPlace::Ground(home),
            1.0,
            "test",
        );
    }
    w.rebuild_caches();
    w.nav = None;
    w.ensure_nav();
    w.meta.tick = (w.meta.tick / hf_sim::time::TICKS_PER_HOUR + 1) * hf_sim::time::TICKS_PER_HOUR;
    hf_sim::systems::settlement::step(&mut w);
    w.designations
        .iter()
        .any(|d| d.pos == face && d.kind == DesignationKind::Mine)
}

#[test]
fn coal_is_cut_for_the_smelter_only_while_coal_is_short() {
    assert!(
        coal_face(0, true),
        "short of coal and ore: the coal face is cut"
    );
    assert!(
        !coal_face(6, true),
        "with coal in the store, ore is wanted, not more coal"
    );
    assert!(
        !coal_face(0, false),
        "with nothing to burn it in, no coal is cut"
    );
}

#[test]
fn the_quarry_gallery_cuts_stone_not_a_coal_seam() {
    let mut w = World::generate(41, hf_sim::world::gen::GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.plants.clear();
    w.meta.founding_site = TilePos::new(14, 14, 2);
    w.meta.tick = hf_sim::time::TICKS_PER_HOUR * 7;
    w.rebuild_caches();
    let ids = w.found_party(10);
    let cid = w
        .agents
        .iter()
        .find(|a| a.id == ids[0])
        .and_then(|a| a.community)
        .unwrap();
    w.meta.autoplan = true;
    let home = w.communities.iter().find(|c| c.id == cid).unwrap().home;
    // The quarry's shaft dug from the ground down into the rock, a coal seam beside its foot.
    let site = w.grid.surface(home.x + 8, home.y);
    let g = site.z - 1;
    let foot = (0..6)
        .map(|k| TilePos::new(site.x, site.y, g - k))
        .find(|p| {
            w.grid.materials.get(w.grid.get(*p).material).kind
                == hf_sim::world::material::MaterialKind::Stone
        })
        .expect("rock under the site");
    for z in foot.z..=g {
        let p = TilePos::new(site.x, site.y, z);
        let t = w.grid.get(p);
        w.grid.set(
            p,
            Tile {
                shape: Shape::StairsBoth,
                ..t
            },
        );
    }
    let seam = foot.offset(1, 0, 0);
    let t = w.grid.get(seam);
    w.grid.set(
        seam,
        Tile {
            material: Material::COAL,
            shape: Shape::Wall,
            ..t
        },
    );
    w.communities
        .iter_mut()
        .find(|c| c.id == cid)
        .unwrap()
        .quarry_site = Some(site);
    w.rebuild_caches();
    w.nav = None;
    w.ensure_nav();
    w.meta.tick = (w.meta.tick / hf_sim::time::TICKS_PER_HOUR + 1) * hf_sim::time::TICKS_PER_HOUR;
    hf_sim::systems::settlement::step(&mut w);
    let cuts: Vec<TilePos> = w
        .designations
        .iter()
        .filter(|d| d.kind == DesignationKind::Mine && d.pos.z == foot.z)
        .map(|d| d.pos)
        .collect();
    assert!(!cuts.is_empty(), "the gallery is cut for stone");
    assert!(!cuts.contains(&seam), "not into the coal seam: {cuts:?}");
}

#[test]
fn the_mine_waits_while_food_is_short() {
    let (mut w, cid, k) = valley();
    lean(&mut w, cid, 1.0);
    let site = site_beside(&w, k, 8);
    let c = w.communities.iter_mut().find(|c| c.id == cid).unwrap();
    c.quarry_site = Some(site);
    c.fate = 1;
    delving::plan_now(&mut w, cid);
    assert!(
        !w.designations
            .iter()
            .any(|d| d.pos.x == site.x && d.pos.y == site.y),
        "no cut ordered while struggling"
    );
    w.communities.iter_mut().find(|c| c.id == cid).unwrap().fate = 0;
    delving::plan_now(&mut w, cid);
    assert!(w
        .designations
        .iter()
        .any(|d| d.pos.x == site.x && d.pos.y == site.y));
}
