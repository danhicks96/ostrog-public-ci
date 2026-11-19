//! C73: a wall round the houses, proposed after a raid and grown round what stands, with one
//! gate; the night's raiders do not come for what is behind it (D42).

use hf_sim::events::EventKind;
use hf_sim::ids::TilePos;
use hf_sim::items::{ItemKind, ItemPlace};
use hf_sim::mind::beliefs::{Source, Topic};
use hf_sim::project::ProjectKind;
use hf_sim::structures::{Structure, StructureKind};
use hf_sim::threat::{Band, BandState};
use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use hf_sim::work::DesignationKind;
use hf_sim::world::gen::GenParams;
use hf_sim::world::{Material, Tile, TileFlags};
use hf_sim::World;

fn meadow(seed: u64, people: usize) -> (World, hf_sim::ids::CommunityId) {
    let mut w = World::generate(seed, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.plants.clear();
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.meta.tick = TICKS_PER_HOUR * 8;
    w.rebuild_caches();
    let ids = w.found_party(people);
    let cid = w
        .agents
        .iter()
        .find(|a| a.id == ids[0])
        .unwrap()
        .community
        .unwrap();
    w.meta.autoplan = true;
    w.ensure_nav();
    (w, cid)
}

/// A house by home, a band camped that took from the store, and one who holds strangers a
/// danger.
fn raided(w: &mut World, cid: hf_sim::ids::CommunityId) {
    let home = w.communities.iter().find(|c| c.id == cid).unwrap().home;
    let o = home.offset(-9, -7, 0);
    w.build_house(o.x, o.y, 7, 6, 1, Material::WOOD, Material::SOIL);
    {
        let c = w.communities.iter_mut().find(|c| c.id == cid).unwrap();
        c.house_sites.push(o);
        c.house_stage.push(3);
    }
    let tick = w.meta.tick;
    let camp = TilePos::new(8, 8, 2);
    let id = 1;
    w.bands.push(Band {
        id,
        site: w.overworld.sites[0].id,
        site_name: "Tollmoor".into(),
        members: Vec::new(),
        size: 3,
        camp,
        state: BandState::Camped(tick),
        formed: tick,
        want: 18,
        taken: 7,
        lost: 0,
        sighted: true,
        welcomed: false,
        pursuits: Vec::new(),
        raiders: Vec::new(),
        envoys: Vec::new(),
        eaten: 0,
        night: 0,
        night_taken: 0,
        remembered: false,
        seen: Vec::new(),
        heard: Vec::new(),
        host: false,
        captives: Vec::new(),
    });
    // The night's raid recorded as the community's hardship (moving.rs observe).
    {
        let c = w.communities.iter_mut().find(|c| c.id == cid).unwrap();
        c.hardships.push((
            tick,
            "strangers from Tollmoor took from the store".into(),
            hf_sim::ids::EventId(1),
        ));
    }
    let first = w.communities.iter().find(|c| c.id == cid).unwrap().members[0];
    let a = w.agents.iter_mut().find(|a| a.id == first).unwrap();
    a.beliefs
        .hold(Topic::Strangers, 0.8, 0.9, Source::Culture(0), tick);
    w.rebuild_caches();
    w.ensure_nav();
}

#[test]
fn after_a_raid_someone_proposes_a_wall_and_the_ring_is_grown_round_what_stands() {
    let (mut w, cid) = meadow(7301, 9);
    raided(&mut w, cid);
    {
        let t = w.meta.tick;
        w.meta.tick = (t / TICKS_PER_DAY + 1) * TICKS_PER_DAY + hf_sim::systems::projects::AT;
        hf_sim::systems::projects::step(&mut w);
        // The planner runs on the hour.
        w.meta.tick = (w.meta.tick / TICKS_PER_HOUR + 1) * TICKS_PER_HOUR;
    }
    let p = w
        .projects
        .iter()
        .find(|p| p.community == cid && p.kind == ProjectKind::Wall)
        .expect("a wall is proposed");
    assert!(p.because.contains("strangers"), "{}", p.because);
    hf_sim::systems::settlement::step(&mut w);
    let c = w.communities.iter().find(|c| c.id == cid).unwrap();
    let pal = c.palisade.clone().expect("the ring is laid out");
    assert!(pal.ring.len() >= 20, "{}", pal.ring.len());
    assert!(pal.ring.contains(&pal.gate));
    // Round the house and the hearth: both strictly inside the ring's box.
    let (x0, x1) = (
        pal.ring.iter().map(|q| q.x).min().unwrap(),
        pal.ring.iter().map(|q| q.x).max().unwrap(),
    );
    let (y0, y1) = (
        pal.ring.iter().map(|q| q.y).min().unwrap(),
        pal.ring.iter().map(|q| q.y).max().unwrap(),
    );
    let home = c.home;
    let o = c.house_sites[0];
    for q in [home, o, o.offset(6, 5, 0)] {
        assert!(q.x > x0 && q.x < x1 && q.y > y0 && q.y < y1, "{q:?} inside");
    }
    // Orders: a wall on every ring tile but the gate, a door there.
    let doors = w
        .designations
        .iter()
        .filter(|d| matches!(d.kind, DesignationKind::BuildDoor) && pal.ring.contains(&d.pos))
        .count();
    let walls = w
        .designations
        .iter()
        .filter(|d| matches!(d.kind, DesignationKind::BuildWall(_)) && pal.ring.contains(&d.pos))
        .count();
    assert_eq!(doors, 1, "one gate");
    assert_eq!(walls, pal.ring.len() - 1);
    assert!(
        !hf_sim::systems::settlement::in_walls(&w, home),
        "not a wall yet"
    );
}

#[test]
fn a_wall_that_stands_keeps_the_nights_raiders_from_the_stores() {
    let (mut w, cid) = meadow(7302, 9);
    raided(&mut w, cid);
    {
        let t = w.meta.tick;
        w.meta.tick = (t / TICKS_PER_DAY + 1) * TICKS_PER_DAY + hf_sim::systems::projects::AT;
        hf_sim::systems::projects::step(&mut w);
        // The planner runs on the hour.
        w.meta.tick = (w.meta.tick / TICKS_PER_HOUR + 1) * TICKS_PER_HOUR;
    }
    hf_sim::systems::settlement::step(&mut w);
    let pal = w
        .communities
        .iter()
        .find(|c| c.id == cid)
        .unwrap()
        .palisade
        .clone()
        .unwrap();
    // The builders have done their work: every ring tile a wall, the gate a door.
    for q in &pal.ring {
        let mut t = Tile::solid(Material::WOOD);
        if *q == pal.gate {
            t.flags = TileFlags(TileFlags::DOOR);
        }
        w.grid.set(*q, t);
    }
    w.designations.retain(|d| !pal.ring.contains(&d.pos));
    w.rebuild_caches();
    w.ensure_nav();
    w.events.drain();
    hf_sim::systems::settlement::step(&mut w);
    let evs = w.events.drain();
    let walled = evs
        .iter()
        .find(|e| matches!(e.kind, EventKind::Walled { .. }))
        .expect("the wall standing is said");
    let line = hf_sim::view::narrate::line(&w, walled);
    assert!(line.contains("gate to the"), "{line}");
    let home = w.communities.iter().find(|c| c.id == cid).unwrap().home;
    assert!(hf_sim::systems::settlement::in_walls(&w, home));
    assert!(!hf_sim::systems::settlement::in_walls(
        &w,
        TilePos::new(8, 8, 2)
    ));
    // Food inside, seen by the band: not a target at night.
    let food = w.create_item(
        ItemKind::GrainSheaf,
        6,
        ItemPlace::Ground(home.offset(1, 1, 0)),
        0.6,
        "test",
    );
    w.bands[0].seen = vec![(food, home.offset(1, 1, 0))];
    let camp = w.bands[0].camp;
    for k in 0..3 {
        let m = w.spawn_agent(hf_sim::agents::Species::Person, camp.offset(k, 0, 0));
        w.agents.iter_mut().find(|a| a.id == m).unwrap().band = Some(1);
        w.bands[0].members.push(m);
    }
    for a in w.agents.iter_mut() {
        a.needs.hunger = 1.0;
        a.needs.rest = 1.0;
    }
    let tick = w.meta.tick;
    w.meta.tick = (tick / TICKS_PER_DAY + 1) * TICKS_PER_DAY + 23 * TICKS_PER_HOUR;
    let mut went = false;
    for _ in 0..TICKS_PER_HOUR * 2 {
        w.step();
        went |= w
            .bands
            .iter()
            .any(|b| b.raiders.iter().any(|r| r.1 == food));
    }
    assert!(!went, "the barred gate keeps them from the stores");
}

#[test]
fn a_careful_one_proposes_a_wall_without_holding_strangers_a_danger() {
    // C73 soaks: 238 night raids and no wall, because few held the belief strongly.
    let (mut w, cid) = meadow(7303, 9);
    raided(&mut w, cid);
    let tick = w.meta.tick;
    for a in w.agents.iter_mut() {
        a.beliefs
            .hold(Topic::Strangers, 0.0, 0.9, Source::Culture(0), tick);
        a.beliefs
            .hold(Topic::Violence, 0.0, 0.9, Source::Culture(0), tick);
        a.traits
            .list
            .retain(|t| *t != hf_sim::mind::Trait::Cautious);
    }
    let careful = w.communities.iter().find(|c| c.id == cid).unwrap().members[1];
    w.agents
        .iter_mut()
        .find(|a| a.id == careful)
        .unwrap()
        .traits
        .list
        .push(hf_sim::mind::Trait::Cautious);
    let t = w.meta.tick;
    w.meta.tick = (t / TICKS_PER_DAY + 1) * TICKS_PER_DAY + hf_sim::systems::projects::AT;
    hf_sim::systems::projects::step(&mut w);
    let p = w
        .projects
        .iter()
        .find(|p| p.kind == ProjectKind::Wall)
        .expect("the careful one wants a wall");
    assert_eq!(p.by, careful);
    assert!(p.because.contains("careful"), "{}", p.because);
}

#[test]
fn a_wall_with_a_breach_keeps_nobody_out_and_is_mended() {
    // C73 soaks: a wall that had stood counted for ever, though half its tiles had fallen.
    let (mut w, cid) = meadow(7304, 9);
    raided(&mut w, cid);
    {
        let tick = w.meta.tick;
        let c = w.communities.iter_mut().find(|c| c.id == cid).unwrap();
        c.hardships.push((
            tick,
            "strangers from Tollmoor took from the store".into(),
            hf_sim::ids::EventId(1),
        ));
    }
    let t = w.meta.tick;
    w.meta.tick = (t / TICKS_PER_DAY + 1) * TICKS_PER_DAY + hf_sim::systems::projects::AT;
    hf_sim::systems::projects::step(&mut w);
    w.meta.tick = (w.meta.tick / TICKS_PER_HOUR + 1) * TICKS_PER_HOUR;
    hf_sim::systems::settlement::step(&mut w);
    let pal = w
        .communities
        .iter()
        .find(|c| c.id == cid)
        .unwrap()
        .palisade
        .clone()
        .unwrap();
    for q in &pal.ring {
        let mut t = Tile::solid(Material::WOOD);
        if *q == pal.gate {
            t.flags = TileFlags(TileFlags::DOOR);
        }
        w.grid.set(*q, t);
    }
    w.designations.retain(|d| !pal.ring.contains(&d.pos));
    w.rebuild_caches();
    hf_sim::systems::settlement::step(&mut w);
    let home = w.communities.iter().find(|c| c.id == cid).unwrap().home;
    assert!(hf_sim::systems::settlement::in_walls(&w, home), "closed");
    // A breach.
    let gap = *pal.ring.iter().find(|q| **q != pal.gate).unwrap();
    w.grid.set(gap, Tile::AIR);
    w.rebuild_caches();
    w.meta.tick += TICKS_PER_HOUR;
    hf_sim::systems::settlement::step(&mut w);
    assert!(
        !hf_sim::systems::settlement::in_walls(&w, home),
        "a breach lets them in"
    );
    assert!(
        w.designations
            .iter()
            .any(|d| d.pos == gap && matches!(d.kind, DesignationKind::BuildWall(_))),
        "the breach is mended"
    );
}

#[test]
fn a_wall_half_built_is_finished_first() {
    // C74: a nearly done wall's last orders waited behind everything else for 400 days.
    let (mut w, cid) = meadow(7401, 9);
    raided(&mut w, cid);
    {
        let tick = w.meta.tick;
        let c = w.communities.iter_mut().find(|c| c.id == cid).unwrap();
        c.hardships.push((
            tick,
            "strangers from Tollmoor took from the store".into(),
            hf_sim::ids::EventId(1),
        ));
    }
    let t = w.meta.tick;
    w.meta.tick = (t / TICKS_PER_DAY + 1) * TICKS_PER_DAY + hf_sim::systems::projects::AT;
    hf_sim::systems::projects::step(&mut w);
    w.meta.tick = (w.meta.tick / TICKS_PER_HOUR + 1) * TICKS_PER_HOUR;
    hf_sim::systems::settlement::step(&mut w);
    let pal = w
        .communities
        .iter()
        .find(|c| c.id == cid)
        .unwrap()
        .palisade
        .clone()
        .unwrap();
    // Two thirds built.
    let n = pal.ring.len() * 2 / 3;
    for q in pal.ring.iter().filter(|q| **q != pal.gate).take(n) {
        w.grid.set(*q, Tile::solid(Material::WOOD));
        w.designations.retain(|d| d.pos != *q);
    }
    w.rebuild_caches();
    w.meta.tick += TICKS_PER_HOUR;
    hf_sim::systems::settlement::step(&mut w);
    let left: Vec<u8> = w
        .designations
        .iter()
        .filter(|d| !d.done && pal.ring.contains(&d.pos))
        .map(|d| d.priority)
        .collect();
    assert!(!left.is_empty());
    assert!(
        left.iter().all(|p| *p >= 8),
        "the rest of the wall comes first: {left:?}"
    );
}

#[test]
fn a_people_known_to_hold_strangers_a_danger_walls_itself_in_unraided() {
    let (mut w, cid) = meadow(7311, 9);
    // A house to wall.
    let home = w.communities.iter().find(|c| c.id == cid).unwrap().home;
    let o = home.offset(-9, -7, 0);
    w.build_house(o.x, o.y, 7, 6, 1, Material::WOOD, Material::SOIL);
    {
        let c = w.communities.iter_mut().find(|c| c.id == cid).unwrap();
        c.house_sites.push(o);
        c.house_stage.push(3);
    }
    // Known by it (#54), and one of them holds it.
    let line = "hold that strangers are a danger".to_string();
    w.communities
        .iter_mut()
        .find(|c| c.id == cid)
        .unwrap()
        .identity
        .push(line);
    let t = w.meta.tick;
    let i = w
        .agents
        .iter()
        .position(|a| a.community == Some(cid))
        .unwrap();
    w.agents[i].beliefs.hold(
        hf_sim::mind::beliefs::Topic::Strangers,
        0.9,
        0.9,
        hf_sim::mind::beliefs::Source::Event(hf_sim::ids::EventId(0)),
        t,
    );
    w.meta.tick = (t / TICKS_PER_DAY + 1) * TICKS_PER_DAY + hf_sim::systems::projects::AT;
    hf_sim::systems::projects::step(&mut w);
    assert!(w
        .projects
        .iter()
        .any(|p| p.community == cid && p.kind == ProjectKind::Wall));
}

/// A wall proposed after a raid and its ring laid out by the planner. Returns the palisade.
fn ring_laid(w: &mut World, cid: hf_sim::ids::CommunityId) -> hf_sim::society::Palisade {
    raided(w, cid);
    let t = w.meta.tick;
    w.meta.tick = (t / TICKS_PER_DAY + 1) * TICKS_PER_DAY + hf_sim::systems::projects::AT;
    hf_sim::systems::projects::step(w);
    w.meta.tick = (w.meta.tick / TICKS_PER_HOUR + 1) * TICKS_PER_HOUR;
    hf_sim::systems::settlement::step(w);
    w.communities
        .iter()
        .find(|c| c.id == cid)
        .unwrap()
        .palisade
        .clone()
        .expect("the ring is laid out")
}

/// #403 (M5 review 2026-10-26: seed 1's ring of 134 stood whole from day 160 and was never
/// closed, for four tiles of a stream on its outline): water on the outline holds the line, as
/// it did when the ring was laid.
#[test]
fn a_ring_built_up_to_the_water_is_closed() {
    let (mut w, cid) = meadow(7311, 9);
    let pal = ring_laid(&mut w, cid);
    // A stream crosses the outline: one tile of the ring (not the gate) is shallow water, and
    // the ring is laid without it, as the planner lays a ring by water.
    let wet = *pal.ring.iter().find(|q| **q != pal.gate).unwrap();
    let mut t = w.grid.get(wet);
    t.flags = TileFlags(t.flags.0 | TileFlags::WATER);
    w.grid.set(wet, t);
    let ring: Vec<TilePos> = pal.ring.iter().copied().filter(|q| *q != wet).collect();
    w.communities
        .iter_mut()
        .find(|c| c.id == cid)
        .unwrap()
        .palisade
        .as_mut()
        .unwrap()
        .ring = ring.clone();
    assert!(w.grid.walkable(wet), "shallow water, walkable");
    for q in &ring {
        let mut t = Tile::solid(Material::WOOD);
        if *q == pal.gate {
            t.flags = TileFlags(TileFlags::DOOR);
        }
        w.grid.set(*q, t);
    }
    w.designations.retain(|d| !pal.ring.contains(&d.pos));
    w.rebuild_caches();
    w.ensure_nav();
    w.meta.tick += TICKS_PER_HOUR;
    hf_sim::systems::settlement::step(&mut w);
    let c = w.communities.iter().find(|c| c.id == cid).unwrap();
    let pal = c.palisade.as_ref().unwrap();
    assert!(
        !pal.ring.contains(&wet),
        "the water is not taken into the ring"
    );
    assert!(pal.closed, "closed up to the water");
}

/// #403 (seed 2's Stoneside: eighteen timber wall orders waited 380 days on planks nobody of it
/// could saw): a community without carpentry that can dress stone walls itself in stone, and
/// timber orders already posted are posted again in stone.
#[test]
fn a_community_that_cannot_saw_planks_walls_itself_in_stone() {
    let (mut w, cid) = meadow(7312, 9);
    let pal = ring_laid(&mut w, cid);
    let timber = w
        .designations
        .iter()
        .filter(|d| {
            pal.ring.contains(&d.pos) && d.kind == DesignationKind::BuildWall(Material::WOOD)
        })
        .count();
    assert!(timber > 0, "timber first, by those who can saw");
    // Carpentry is lost; one of them dresses stone.
    let tick = w.meta.tick;
    let mut first = true;
    for a in w.agents.iter_mut().filter(|a| a.community == Some(cid)) {
        a.techniques
            .list
            .retain(|k| k.technique != hf_sim::mind::Technique::Carpentry);
        if first {
            a.techniques.learn(
                hf_sim::mind::Technique::Masonry,
                hf_sim::mind::techniques::Learned::Taught,
                0.5,
                tick,
            );
            first = false;
        }
    }
    w.meta.tick += TICKS_PER_HOUR;
    hf_sim::systems::settlement::step(&mut w);
    let walls: Vec<DesignationKind> = w
        .designations
        .iter()
        .filter(|d| !d.done && pal.ring.contains(&d.pos) && d.pos != pal.gate)
        .map(|d| d.kind)
        .collect();
    assert!(!walls.is_empty());
    assert!(
        walls
            .iter()
            .all(|k| *k == DesignationKind::BuildWall(Material::STONE_BLOCK)),
        "{walls:?}"
    );
}

/// #403 (M5 review 2026-10-26: seed 4's Merehollow, 208 of its 211 ring tiles stood at day
/// 600, but three tiles of its outline held a table, a bed and a lamp of a building no house
/// site names; the laying had left them out of the ring, nothing could be built where they
/// stood, and the wall never closed): the line goes round what stands on it, at the laying.
#[test]
fn a_ring_goes_round_what_stands_on_the_line() {
    let (mut w, cid) = meadow(7313, 9);
    // A community camped far from its first house, so the line drawn round the house runs
    // beyond the planner's pull-in of what the wall goes round (25 tiles from home).
    let home = TilePos::new(55, 55, 2);
    w.communities.iter_mut().find(|c| c.id == cid).unwrap().home = home;
    w.meta.founding_site = home;
    raided(&mut w, cid);
    // A named house far out by the fields, so the line is drawn past a shelter nobody names.
    let o = TilePos::new(10, 8, 2);
    w.build_house(o.x, o.y, 7, 6, 1, Material::WOOD, Material::SOIL);
    {
        let c = w.communities.iter_mut().find(|c| c.id == cid).unwrap();
        c.house_sites.push(o);
        c.house_stage.push(3);
    }
    // The shelter the line would cut through, with a table, a bed and a lamp on its floor
    // (as at Merehollow).
    let s = TilePos::new(6, 18, 2);
    w.build_house(s.x, s.y, 7, 6, 1, Material::WOOD, Material::SOIL);
    let furniture: Vec<TilePos> = (0..3).map(|i| TilePos::new(8, 20 + i, 2)).collect();
    for (i, pos) in furniture.iter().copied().enumerate() {
        let kind = [
            StructureKind::Table,
            StructureKind::Bed,
            StructureKind::Lamp,
        ][i];
        let id = w.alloc_structure_id();
        w.structures.push(Structure {
            id,
            kind,
            pos,
            material: Material::WOOD,
            progress: 1.0,
            active: false,
            condition: 1.0,
            owner: None,
            quality: 0.5,
        });
    }
    let pal = ring_laid(&mut w, cid);
    // Nothing stands on the line: the shelter and its furniture lie strictly inside the
    // ring's outline.
    let (x0, x1) = (
        pal.ring.iter().map(|q| q.x).min().unwrap(),
        pal.ring.iter().map(|q| q.x).max().unwrap(),
    );
    let (y0, y1) = (
        pal.ring.iter().map(|q| q.y).min().unwrap(),
        pal.ring.iter().map(|q| q.y).max().unwrap(),
    );
    for p in furniture {
        assert!(
            !(p.x == x0 || p.x == x1 || p.y == y0 || p.y == y1),
            "what stands at {p:?} is not on the outline ({x0}..={x1}, {y0}..={y1})"
        );
    }
    // Built, the ring closes.
    for q in &pal.ring {
        let mut t = Tile::solid(Material::WOOD);
        if *q == pal.gate {
            t.flags = TileFlags(TileFlags::DOOR);
        }
        w.grid.set(*q, t);
    }
    w.designations.retain(|d| !pal.ring.contains(&d.pos));
    w.rebuild_caches();
    w.ensure_nav();
    w.meta.tick += TICKS_PER_HOUR;
    hf_sim::systems::settlement::step(&mut w);
    let pal = w
        .communities
        .iter()
        .find(|c| c.id == cid)
        .unwrap()
        .palisade
        .as_ref()
        .unwrap();
    assert!(pal.closed, "the ring round the shelter closes");
}

/// #403, the same fault come to the ring after the laying: a building gone up on the line
/// since is gone round, and the ring still closes.
#[test]
fn a_ring_goes_round_what_goes_up_on_the_line_after_the_laying() {
    let (mut w, cid) = meadow(7314, 9);
    let pal = ring_laid(&mut w, cid);
    let (x0, y0, y1) = (
        pal.ring.iter().map(|q| q.x).min().unwrap(),
        pal.ring.iter().map(|q| q.y).min().unwrap(),
        pal.ring.iter().map(|q| q.y).max().unwrap(),
    );
    // A tile of the laid line, left out of the ring as the laying would leave a building's
    // floor, with a bench since set on it.
    let q = *pal
        .ring
        .iter()
        .find(|q| **q != pal.gate && q.x == x0 && q.y > y0 && q.y < y1)
        .expect("a line tile on the west side");
    let id = w.alloc_structure_id();
    w.structures.push(Structure {
        id,
        kind: StructureKind::Table,
        pos: q,
        material: Material::WOOD,
        progress: 1.0,
        active: false,
        condition: 1.0,
        owner: None,
        quality: 0.5,
    });
    w.communities
        .iter_mut()
        .find(|c| c.id == cid)
        .unwrap()
        .palisade
        .as_mut()
        .unwrap()
        .ring
        .retain(|r| *r != q);
    w.designations.retain(|d| d.pos != q);
    w.rebuild_caches();
    // The line moves out a course round the bench.
    w.meta.tick += TICKS_PER_HOUR;
    hf_sim::systems::settlement::step(&mut w);
    let pal = w
        .communities
        .iter()
        .find(|c| c.id == cid)
        .unwrap()
        .palisade
        .as_ref()
        .unwrap();
    let nx0 = pal.ring.iter().map(|q| q.x).min().unwrap();
    assert_eq!(
        nx0,
        x0 - 1,
        "the line moved out a tile round what stands on it"
    );
    assert!(!pal.ring.contains(&q), "the bench tile is not walled");
    // Built, the ring closes.
    for r in &pal.ring {
        let mut t = Tile::solid(Material::WOOD);
        if *r == pal.gate {
            t.flags = TileFlags(TileFlags::DOOR);
        }
        w.grid.set(*r, t);
    }
    w.designations.retain(|d| !pal.ring.contains(&d.pos));
    w.rebuild_caches();
    w.ensure_nav();
    w.meta.tick += TICKS_PER_HOUR;
    hf_sim::systems::settlement::step(&mut w);
    let pal = w
        .communities
        .iter()
        .find(|c| c.id == cid)
        .unwrap()
        .palisade
        .as_ref()
        .unwrap();
    assert!(pal.closed, "the ring closes round the bench");
}

/// A ring laid out for a community whose home is near the map's west edge, so the outline's
/// west side runs at the edge clamp (x = 5) and cannot move out.
fn ring_at_the_edge(seed: u64) -> (World, hf_sim::ids::CommunityId, hf_sim::society::Palisade) {
    let (mut w, cid) = meadow(seed, 9);
    let home = TilePos::new(16, 30, 2);
    w.communities.iter_mut().find(|c| c.id == cid).unwrap().home = home;
    w.meta.founding_site = home;
    let pal = ring_laid(&mut w, cid);
    (w, cid, pal)
}

fn build_ring(w: &mut World, ring: &[TilePos], gate: TilePos) {
    for q in ring {
        let mut t = Tile::solid(Material::WOOD);
        if *q == gate {
            t.flags = TileFlags(TileFlags::DOOR);
        }
        w.grid.set(*q, t);
    }
    w.designations.retain(|d| !ring.contains(&d.pos));
}

/// #403 (2026-11-01, seed 3's Holtwick at day 600: 78 of 79 ring tiles stood; the last, on a
/// rise inside the ring whose ways up the ring itself had walled, failed its order at
/// priority 8 for 580 days; and two bedroom tiles on the line, shut off by the wall, kept the
/// ring from ever closing): a ring tile on a ledge nobody can reach is not ordered for ever,
/// and ground on the outline nobody can reach is no way in.
#[test]
fn a_ring_closes_round_a_ledge_and_a_shut_room_nobody_can_reach() {
    let (mut w, cid, pal) = ring_at_the_edge(7315);
    let x0 = pal.ring.iter().map(|q| q.x).min().unwrap();
    assert_eq!(x0, 5, "the west side runs at the map's edge clamp");
    let (y0, y1) = (
        pal.ring.iter().map(|q| q.y).min().unwrap(),
        pal.ring.iter().map(|q| q.y).max().unwrap(),
    );
    let west: Vec<TilePos> = pal
        .ring
        .iter()
        .copied()
        .filter(|q| q.x == x0 && q.y > y0 + 1 && q.y < y1 - 1 && *q != pal.gate)
        .collect();
    assert!(west.len() >= 6);
    // The ledge: one ring tile's ground raised a level, the ring tile on top of it, with no
    // way up. Its order, as the planner posted it, waits there.
    let low = west[1];
    let ledge = low.offset(0, 0, 1);
    // The shut room: a bed on the line, its sides walled up, left out of the ring as the
    // laying leaves what stands on the line.
    let room = west[4];
    let mut ring: Vec<TilePos> = pal
        .ring
        .iter()
        .copied()
        .filter(|q| *q != low && *q != room)
        .collect();
    build_ring(&mut w, &ring, pal.gate);
    w.designations.retain(|d| d.pos != low && d.pos != room);
    w.grid.set(low, Tile::solid(Material::SOIL));
    for dy in -1..=1 {
        for dx in [-1, 1] {
            w.grid
                .set(room.offset(dx, dy, 0), Tile::solid(Material::SOIL));
        }
    }
    let id = w.alloc_structure_id();
    w.structures.push(Structure {
        id,
        kind: StructureKind::Bed,
        pos: room,
        material: Material::WOOD,
        progress: 1.0,
        active: false,
        condition: 1.0,
        owner: None,
        quality: 0.5,
    });
    // A store to haul to.
    let home = w.communities.iter().find(|c| c.id == cid).unwrap().home;
    let id = w.alloc_structure_id();
    w.structures.push(Structure {
        id,
        kind: StructureKind::Stockpile,
        pos: home.offset(2, 2, 0),
        material: Material::SOIL,
        progress: 1.0,
        active: false,
        condition: 1.0,
        owner: None,
        quality: 0.5,
    });
    ring.push(ledge);
    w.communities
        .iter_mut()
        .find(|c| c.id == cid)
        .unwrap()
        .palisade
        .as_mut()
        .unwrap()
        .ring = ring;
    let mut order = hf_sim::systems::settlement::make(
        &mut w,
        DesignationKind::BuildWall(Material::WOOD),
        ledge,
        8,
    );
    order.community = Some(cid);
    order.failures = 9;
    // The planks for it were brought before the way up was walled.
    order.requirements[0].delivered = 2;
    order.requirements[0].material = Material::WOOD;
    w.designations.push(order);
    w.rebuild_caches();
    w.ensure_nav();
    let nav = w.nav.as_ref().unwrap();
    assert!(w.grid.walkable(ledge) && !nav.reach(home, ledge));
    assert!(w.grid.walkable(room) && !nav.reach(home, room));
    for _ in 0..2 {
        w.meta.tick += TICKS_PER_HOUR;
        hf_sim::systems::settlement::step(&mut w);
    }
    let c = w.communities.iter().find(|c| c.id == cid).unwrap();
    let pal = c.palisade.as_ref().unwrap();
    assert!(
        pal.closed,
        "the ledge and the shut room are no way in: the ring is closed"
    );
    assert!(
        !w.designations
            .iter()
            .any(|d| !d.done && d.pos == ledge && matches!(d.kind, DesignationKind::BuildWall(_))),
        "nobody is sent to wall a ledge nobody can climb to"
    );
    // What was brought is left by the site, and nobody is sent to haul it from where nobody
    // can walk.
    let planks: u32 = w
        .items
        .iter()
        .filter(|it| it.kind == ItemKind::Plank && it.place == ItemPlace::Ground(ledge))
        .map(|it| it.count)
        .sum();
    assert_eq!(planks, 2, "the planks are given back at the ledge");
    // Everything else stored, the hauls open now are the planks' own.
    let store = home.offset(2, 2, 0);
    for it in w.items.iter_mut() {
        if matches!(it.place, ItemPlace::Ground(p) if p != ledge) {
            it.place = ItemPlace::Ground(store);
        }
    }
    w.designations.retain(|d| d.kind != DesignationKind::Haul);
    w.meta.tick += TICKS_PER_HOUR;
    hf_sim::systems::settlement::step(&mut w);
    w.ensure_nav();
    let nav = w.nav.as_ref().unwrap();
    assert!(
        !w.designations
            .iter()
            .any(|d| !d.done && d.kind == DesignationKind::Haul && !nav.reach(home, d.pos)),
        "no haul from where nobody can walk"
    );
}

/// #403 (2026-11-01, seed 3's Holtwick: a house at the map's edge stood on the line, which
/// could not go round it there; the laying walled the roofed passage between two bedrooms and
/// shut both beds off): the line is not laid across a building's roofed floor, and the
/// building's own walls hold it there.
#[test]
fn a_ring_at_the_edge_does_not_wall_a_house_through() {
    let (mut w, cid) = meadow(7316, 9);
    let home = TilePos::new(16, 30, 2);
    w.communities.iter_mut().find(|c| c.id == cid).unwrap().home = home;
    w.meta.founding_site = home;
    // A house from x 2 to 11 (south of the one `raided` builds), its door on the south side
    // at x 7, a bed in its south-west corner.
    let o = TilePos::new(2, 33, 1);
    w.build_house(o.x, o.y, 10, 6, 1, Material::WOOD, Material::SOIL);
    let bed = TilePos::new(3, 37, 2);
    let id = w.alloc_structure_id();
    w.structures.push(Structure {
        id,
        kind: StructureKind::Bed,
        pos: bed,
        material: Material::WOOD,
        progress: 1.0,
        active: false,
        condition: 1.0,
        owner: None,
        quality: 0.5,
    });
    // A bench out by the door, so the line runs past the doorway, not across it.
    let id = w.alloc_structure_id();
    w.structures.push(Structure {
        id,
        kind: StructureKind::Table,
        pos: TilePos::new(9, 40, 2),
        material: Material::WOOD,
        progress: 1.0,
        active: false,
        condition: 1.0,
        owner: None,
        quality: 0.5,
    });
    w.rebuild_caches();
    w.ensure_nav();
    let pal = ring_laid(&mut w, cid);
    let x0 = pal.ring.iter().map(|q| q.x).min().unwrap();
    assert_eq!(x0, 5, "the line runs through the house at the edge clamp");
    let roofed: Vec<TilePos> = pal
        .ring
        .iter()
        .copied()
        .filter(|q| w.roofs.contains_key(q))
        .collect();
    assert!(
        roofed.is_empty(),
        "no wall laid on the house's floor: {roofed:?}"
    );
    let ring = pal.ring.clone();
    build_ring(&mut w, &ring, pal.gate);
    w.rebuild_caches();
    w.ensure_nav();
    w.meta.tick += TICKS_PER_HOUR;
    hf_sim::systems::settlement::step(&mut w);
    w.ensure_nav();
    let nav = w.nav.as_ref().unwrap();
    assert!(nav.reach(home, bed), "the bed can still be walked to");
    let pal = w
        .communities
        .iter()
        .find(|c| c.id == cid)
        .unwrap()
        .palisade
        .as_ref()
        .unwrap();
    assert!(pal.closed, "the house's walls hold the line: closed");
}

/// #377 5.1a (FIXLOG 2026-11-09): once the wall stands whole the store grows inside it. A pile
/// that has tiles outside the ring (laid before the wall) and tiles inside, every one holding
/// something, used to grow from all of them, nearest the first, so the new tiles went outside.
#[test]
fn a_store_grows_inside_the_wall_once_it_stands() {
    let (mut w, cid) = meadow(7311, 9);
    let pal = ring_laid(&mut w, cid);
    // The builders have done their work: every ring tile a wall, the gate a door.
    for q in &pal.ring {
        let mut t = Tile::solid(Material::WOOD);
        if *q == pal.gate {
            t.flags = TileFlags(TileFlags::DOOR);
        }
        w.grid.set(*q, t);
    }
    w.designations.retain(|d| !pal.ring.contains(&d.pos));
    w.rebuild_caches();
    w.ensure_nav();
    hf_sim::systems::settlement::step(&mut w);
    let home = w.communities.iter().find(|c| c.id == cid).unwrap().home;
    assert!(
        hf_sim::systems::settlement::in_walls(&w, home),
        "the wall stands whole"
    );
    // A pile laid before the wall: two tiles outside the ring (first in the list, so the pile's
    // origin is outside) and four inside, each holding a sheaf, so the pile has no free tile.
    let x1 = pal.ring.iter().map(|q| q.x).max().unwrap();
    let outside = [(x1 + 3, home.y), (x1 + 3, home.y + 1)];
    // Four open tiles of the ring's interior, found rather than assumed (the ring is as small as
    // what it was grown round).
    let (x0, y0) = (
        pal.ring.iter().map(|q| q.x).min().unwrap(),
        pal.ring.iter().map(|q| q.y).min().unwrap(),
    );
    let y1 = pal.ring.iter().map(|q| q.y).max().unwrap();
    let none = std::collections::BTreeSet::new();
    let mut interior: Vec<TilePos> = Vec::new();
    for y in y0 + 1..y1 {
        for x in x0 + 1..x1 {
            let p = w.grid.surface(x, y);
            if hf_sim::grounds::open_ground(&w, p, &none, false)
                && hf_sim::systems::settlement::in_walls(&w, p)
            {
                interior.push(p);
            }
        }
    }
    assert!(
        interior.len() >= 8,
        "the ring holds room for the test ({} open tiles in {x0}..{x1} by {y0}..{y1})",
        interior.len()
    );
    let mut tiles: Vec<TilePos> = Vec::new();
    for (x, y) in outside {
        tiles.push(w.grid.surface(x, y));
    }
    tiles.extend(interior.iter().take(4).copied());
    assert!(!hf_sim::systems::settlement::in_walls(&w, tiles[0]));
    assert!(hf_sim::systems::settlement::in_walls(&w, tiles[2]));
    for p in &tiles {
        let id = w.alloc_structure_id();
        w.structures.push(Structure {
            id,
            kind: StructureKind::Stockpile,
            pos: *p,
            material: Material::SOIL,
            progress: 1.0,
            active: false,
            condition: 1.0,
            owner: None,
            quality: 0.5,
        });
        w.create_item(ItemKind::GrainSheaf, 1, ItemPlace::Ground(*p), 0.6, "test");
    }
    w.designations.retain(|d| {
        !matches!(
            d.kind,
            DesignationKind::BuildStructure(StructureKind::Stockpile, _)
        )
    });
    w.rebuild_caches();
    w.ensure_nav();
    w.meta.tick = (w.meta.tick / TICKS_PER_HOUR + 1) * TICKS_PER_HOUR;
    hf_sim::systems::settlement::step(&mut w);
    let posted: Vec<TilePos> = w
        .designations
        .iter()
        .filter(|d| {
            matches!(
                d.kind,
                DesignationKind::BuildStructure(StructureKind::Stockpile, _)
            )
        })
        .map(|d| d.pos)
        .collect();
    assert!(!posted.is_empty(), "a full pile is grown");
    for p in &posted {
        assert!(
            hf_sim::systems::settlement::in_walls(&w, *p),
            "the new store tile {p:?} is inside the wall"
        );
    }
}
