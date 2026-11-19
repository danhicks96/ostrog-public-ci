//! C60: moving camp as a proposal people follow or refuse, and the end of a community.

use hf_sim::events::{EventKind, Severity};
use hf_sim::ids::{AgentId, CommunityId, EventId, TilePos};
use hf_sim::project::ProjectKind;
use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use hf_sim::world::gen::GenParams;
use hf_sim::World;

fn valley(seed: u64) -> (World, CommunityId) {
    valley_of(seed, 8)
}

fn valley_of(seed: u64, party: usize) -> (World, CommunityId) {
    let mut w = World::generate(seed, GenParams::default());
    w.flatten(2, 2, 120, 120, 1);
    w.plants.clear();
    w.meta.founding_site = TilePos::new(60, 60, 2);
    w.meta.tick = TICKS_PER_DAY * 30 + TICKS_PER_HOUR * 8;
    w.rebuild_caches();
    let ids = w.found_party(party);
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

fn hard_times(w: &mut World, cid: CommunityId) {
    let tick = w.meta.tick;
    let c = w.communities.iter_mut().find(|c| c.id == cid).unwrap();
    for (k, what) in [
        "Ana died of starvation",
        "strangers took from the store",
        "the rock came down",
        "Bren died of the cold",
    ]
    .iter()
    .enumerate()
    {
        c.hardships.push((
            tick - (4 - k as u64) * TICKS_PER_DAY,
            what.to_string(),
            EventId(k as u32 + 1),
        ));
    }
}

fn regard_all(w: &mut World, cid: CommunityId, opinion: f32) {
    let ids: Vec<_> = w
        .agents
        .iter()
        .filter(|a| a.community == Some(cid))
        .map(|a| a.id)
        .collect();
    for a in w.agents.iter_mut().filter(|a| a.community == Some(cid)) {
        for o in &ids {
            if *o != a.id {
                a.relationships.entry(*o).or_default().opinion = opinion;
            }
        }
    }
}

fn events_of(w: &mut World, f: impl Fn(&EventKind) -> bool) -> Vec<EventKind> {
    w.events
        .drain()
        .into_iter()
        .map(|e| e.kind)
        .filter(|k| f(k))
        .collect()
}

#[test]
fn no_hard_times_no_move() {
    let (mut w, cid) = valley(6001);
    regard_all(&mut w, cid, 0.6);
    hf_sim::systems::moving::consider(&mut w, cid);
    assert!(!w
        .projects
        .iter()
        .any(|p| matches!(p.kind, ProjectKind::Move { .. })));
}

#[test]
fn a_move_has_a_proposer_and_a_cause() {
    let (mut w, cid) = valley(6002);
    hard_times(&mut w, cid);
    regard_all(&mut w, cid, 0.6);
    let home = w.communities.iter().find(|c| c.id == cid).unwrap().home;
    // A house at the old site.
    w.communities
        .iter_mut()
        .find(|c| c.id == cid)
        .unwrap()
        .house_sites
        .push(home.offset(-10, -8, 0));
    hf_sim::systems::moving::consider(&mut w, cid);
    let p = w
        .projects
        .iter()
        .find(|p| matches!(p.kind, ProjectKind::Move { .. }))
        .expect("someone proposes moving camp")
        .clone();
    assert!(p.because.contains("hard things"), "{}", p.because);
    let moved = events_of(&mut w, |k| matches!(k, EventKind::MovedCamp { .. }));
    assert_eq!(
        moved.len(),
        1,
        "everyone thinks well of them: the whole community moves"
    );
    let now = w.communities.iter().find(|c| c.id == cid).unwrap().home;
    assert!(
        now.dist(home) >= 20,
        "a new home {now:?}, away from {home:?}"
    );
    // The old site is left: its house is no longer theirs to plan from.
    assert!(w
        .communities
        .iter()
        .find(|c| c.id == cid)
        .unwrap()
        .house_sites
        .is_empty());
}

#[test]
fn those_who_stay_say_why() {
    let (mut w, cid) = valley(6003);
    hard_times(&mut w, cid);
    regard_all(&mut w, cid, -0.6);
    hf_sim::systems::moving::consider(&mut w, cid);
    let chose = events_of(&mut w, |k| matches!(k, EventKind::Chose { .. }));
    let stayed: Vec<_> = chose
        .iter()
        .filter_map(|k| match k {
            EventKind::Chose { what, because, .. } if what == "stayed" => Some(because.clone()),
            _ => None,
        })
        .collect();
    assert!(!stayed.is_empty(), "some stay");
    assert!(stayed.iter().all(|b| !b.is_empty()));
    assert!(
        !w.communities
            .iter()
            .any(|c| c.id != cid && !c.dissolved && c.members.len() >= 2),
        "too few went to make a new community"
    );
}

#[test]
fn a_community_that_leaves_the_valley_becomes_a_settlement_of_the_world() {
    let mut w = World::opening_world(6004);
    let cid = w.communities[0].id;
    w.meta.tick += TICKS_PER_DAY * 30;
    hard_times(&mut w, cid);
    regard_all(&mut w, cid, 0.6);
    let sites = w.realm.sites.len();
    // No way to any better place in the valley: out of it, to where they know of.
    w.nav = None;
    for st in w.overworld.sites.iter_mut() {
        st.known_by.retain(|c| *c != cid);
    }
    let mut unknowing = w.clone();
    hf_sim::systems::moving::consider(&mut unknowing, cid);
    assert!(
        events_of(&mut unknowing, |k| matches!(
            k,
            EventKind::LeftTheValley { .. }
        ))
        .is_empty(),
        "knowing no place out there, nobody proposes leaving for nowhere"
    );
    w.overworld.sites[0].known_by.push(cid);
    hf_sim::systems::moving::consider(&mut w, cid);
    let left = events_of(&mut w, |k| matches!(k, EventKind::LeftTheValley { .. }));
    assert_eq!(left.len(), 1, "they leave the valley");
    // #1322: the realm holds who arrived, not who set out; nobody has stepped off the map yet.
    assert_eq!(
        w.realm.sites.len(),
        sites,
        "not founded until someone arrives"
    );
    let goers: Vec<usize> = (0..w.agents.len())
        .filter(|&i| w.agents[i].alive && w.agents[i].community.is_none() && w.agents[i].leaving)
        .collect();
    assert!(goers.len() >= 2);
    for &i in &goers {
        hf_sim::systems::society::leave_valley(&mut w, i);
    }
    assert_eq!(
        w.realm.sites.len(),
        sites + 1,
        "a settlement of theirs is founded now that they have arrived"
    );
    let founded = w.realm.sites.last().unwrap();
    assert_eq!(
        founded.population as usize,
        goers.len(),
        "the realm holds everyone who arrived"
    );
    // Everyone thinks well of the one who proposed it, so nobody stayed: the valley's story
    // has ended, and the ending says where they went.
    assert!(
        !w.agents.iter().any(|a| a.alive && a.community == Some(cid)),
        "everyone went"
    );
    w.meta.tick += 2 * TICKS_PER_DAY;
    hf_sim::systems::moving::endings(&mut w);
    let end = hf_sim::systems::moving::ending(&w);
    assert!(end.contains("left the valley for"), "{end}");
}

#[test]
fn dying_out_is_recorded() {
    let (mut w, cid) = valley(6005);
    for a in w.agents.iter_mut().filter(|a| a.community == Some(cid)) {
        a.alive = false;
    }
    w.meta.tick += 2 * TICKS_PER_DAY;
    hf_sim::systems::moving::endings(&mut w);
    let ended = events_of(&mut w, |k| matches!(k, EventKind::CommunityEnded { .. }));
    assert_eq!(ended.len(), 1);
    assert!(matches!(&ended[0], EventKind::CommunityEnded { how, .. } if how == "died out"));
    assert!(
        w.communities
            .iter()
            .find(|c| c.id == cid)
            .unwrap()
            .dissolved
    );
    hf_sim::systems::moving::endings(&mut w);
    assert!(
        events_of(&mut w, |k| matches!(k, EventKind::CommunityEnded { .. })).is_empty(),
        "once"
    );
}

#[test]
fn a_bands_raids_are_one_hardship() {
    // In C60's first soaks one band's nightly raids alone moved a village.
    let (mut w, cid) = valley(6006);
    let home = w.communities.iter().find(|c| c.id == cid).unwrap().home;
    for _ in 0..5 {
        w.meta.tick += TICKS_PER_DAY;
        let tick = w.meta.tick;
        w.events.push(
            tick,
            Some(home.offset(10, 0, 0)),
            hf_sim::events::Severity::Minor,
            EventKind::Raided {
                band: 7,
                what: "meals".into(),
                count: 2,
                open: false,
            },
        );
        w.last_events = w.events.drain();
        hf_sim::systems::moving::observe(&mut w);
    }
    let c = w.communities.iter().find(|c| c.id == cid).unwrap();
    assert_eq!(c.hardships.len(), 1, "{:?}", c.hardships);
}

/// #403 (M5 review 2026-10-26: seed 3's Holtwick kept its first camp's wall through three moves
/// and could never propose one again): a wall is left with the camp it was raised round, its
/// project ends, and at the new camp a wall can be proposed again.
#[test]
fn a_wall_is_left_with_the_old_camp() {
    let (mut w, cid) = valley(6005);
    hard_times(&mut w, cid);
    regard_all(&mut w, cid, 0.6);
    let home = w.communities.iter().find(|c| c.id == cid).unwrap().home;
    let tick = w.meta.tick;
    let ring: Vec<TilePos> = (-6..=6).map(|d| home.offset(d, -6, 0)).collect();
    let gate = ring[6];
    let by = w.communities.iter().find(|c| c.id == cid).unwrap().members[0];
    {
        let c = w.communities.iter_mut().find(|c| c.id == cid).unwrap();
        c.palisade = Some(hf_sim::society::Palisade {
            ring,
            gate,
            begun: tick,
            stood: None,
            closed: false,
        });
    }
    w.projects.push(hf_sim::project::Project {
        id: 9001,
        by,
        community: cid,
        kind: ProjectKind::Wall,
        because: "a test".into(),
        source: None,
        proposed: tick,
        work: None,
        done: false,
        site: Some(gate),
    });
    hf_sim::systems::moving::consider(&mut w, cid);
    let moved = events_of(&mut w, |k| matches!(k, EventKind::MovedCamp { .. }));
    assert_eq!(moved.len(), 1, "the community moves");
    let c = w.communities.iter().find(|c| c.id == cid).unwrap();
    assert!(c.palisade.is_none(), "the old ring stays behind");
    let new_home = c.home;
    // At the new camp: a house, and one who holds strangers a danger.
    let o = new_home.offset(-9, -7, 0);
    w.build_house(
        o.x,
        o.y,
        7,
        6,
        1,
        hf_sim::world::Material::WOOD,
        hf_sim::world::Material::SOIL,
    );
    w.communities
        .iter_mut()
        .find(|c| c.id == cid)
        .unwrap()
        .house_sites
        .push(o);
    let t = w.meta.tick;
    // A raid since (the move answered the hardships before it).
    w.communities
        .iter_mut()
        .find(|c| c.id == cid)
        .unwrap()
        .hardships
        .push((
            t,
            "strangers from Tollmoor took from the store".into(),
            EventId(9),
        ));
    for a in w.agents.iter_mut().filter(|a| a.community == Some(cid)) {
        a.beliefs.hold(
            hf_sim::mind::beliefs::Topic::Strangers,
            0.8,
            0.9,
            hf_sim::mind::beliefs::Source::Culture(0),
            t,
        );
    }
    w.meta.tick = (t / TICKS_PER_DAY + 1) * TICKS_PER_DAY + hf_sim::systems::projects::AT;
    hf_sim::systems::projects::step(&mut w);
    assert!(
        w.projects
            .iter()
            .find(|p| p.id == 9001)
            .is_none_or(|p| p.done),
        "the old camp's wall project ended"
    );
    assert!(
        w.projects
            .iter()
            .any(|p| p.community == cid && p.kind == ProjectKind::Wall && p.open()),
        "a wall proposed at the new camp"
    );
}

/// #403/#496: a wall can have stood in the settlement step immediately before the move while
/// its daily project record is still open. Relocation closes that record with the old site so it
/// cannot suppress the next camp's wall proposal until another planner pass.
#[test]
fn a_completed_wall_project_is_closed_with_the_old_camp() {
    let (mut w, cid) = valley(6007);
    hard_times(&mut w, cid);
    regard_all(&mut w, cid, 0.6);
    let home = w.communities.iter().find(|c| c.id == cid).unwrap().home;
    let tick = w.meta.tick;
    let ring: Vec<TilePos> = (-6..=6).map(|d| home.offset(d, -6, 0)).collect();
    let gate = ring[6];
    let by = w.communities.iter().find(|c| c.id == cid).unwrap().members[0];
    w.communities
        .iter_mut()
        .find(|c| c.id == cid)
        .unwrap()
        .palisade = Some(hf_sim::society::Palisade {
        ring,
        gate,
        begun: tick - TICKS_PER_DAY,
        stood: Some(tick - TICKS_PER_HOUR),
        closed: true,
    });
    w.projects.push(hf_sim::project::Project {
        id: 9002,
        by,
        community: cid,
        kind: ProjectKind::Wall,
        because: "a test wall that has just stood".into(),
        source: None,
        proposed: tick - TICKS_PER_DAY,
        work: None,
        done: false,
        site: Some(gate),
    });

    hf_sim::systems::moving::consider(&mut w, cid);

    let c = w.communities.iter().find(|c| c.id == cid).unwrap();
    assert!(
        c.palisade.is_none(),
        "the completed ring stays with the old camp"
    );
    assert!(
        w.projects
            .iter()
            .find(|p| p.id == 9002)
            .is_some_and(|p| p.done),
        "the completed wall's project closes in the move"
    );
}

/// #433: water first. A community that moves camp goes where there is water a short walk off
/// when any of the places it finds has it.
#[test]
fn a_move_goes_to_water() {
    let (mut w, cid) = valley(6011);
    // A stream two wide down the valley's east side, 35 tiles from home.
    for y in 2..=120 {
        for x in 95..=96 {
            w.grid.set(
                TilePos::new(x, y, 2),
                hf_sim::world::Tile {
                    material: hf_sim::world::Material::WATER,
                    shape: hf_sim::world::Shape::Floor,
                    flags: hf_sim::world::TileFlags(hf_sim::world::TileFlags::WATER),
                },
            );
        }
    }
    w.rebuild_caches();
    hf_sim::floods::survey(&mut w);
    w.ensure_nav();
    hard_times(&mut w, cid);
    regard_all(&mut w, cid, 0.6);
    hf_sim::systems::moving::consider(&mut w, cid);
    let p = w
        .projects
        .iter()
        .find(|p| matches!(p.kind, ProjectKind::Move { .. }))
        .expect("someone proposes moving camp")
        .clone();
    let to = p.site.expect("a place in the valley");
    assert!(
        hf_sim::wells::water_within(&w, to, hf_sim::wells::SHORT_WALK).is_some(),
        "{to:?} by the stream"
    );
}

/// #433: a camp with no water within a long walk, whose well found none, moves to water
/// without waiting for hard times to pile up.
#[test]
fn a_dry_camp_whose_well_failed_moves_to_water() {
    let (mut w, cid) = valley(6012);
    for y in 2..=120 {
        for x in 95..=96 {
            w.grid.set(
                TilePos::new(x, y, 2),
                hf_sim::world::Tile {
                    material: hf_sim::world::Material::WATER,
                    shape: hf_sim::world::Shape::Floor,
                    flags: hf_sim::world::TileFlags(hf_sim::world::TileFlags::WATER),
                },
            );
        }
    }
    w.rebuild_caches();
    hf_sim::floods::survey(&mut w);
    w.ensure_nav();
    regard_all(&mut w, cid, 0.6);
    // Without a failed well, no move.
    hf_sim::systems::moving::consider(&mut w, cid);
    assert!(!w
        .projects
        .iter()
        .any(|p| matches!(p.kind, ProjectKind::Move { .. })));
    let until = w.meta.tick + 10 * TICKS_PER_DAY;
    w.communities
        .iter_mut()
        .find(|c| c.id == cid)
        .unwrap()
        .no_site
        .push((hf_sim::structures::StructureKind::Well, until));
    hf_sim::systems::moving::consider(&mut w, cid);
    let p = w
        .projects
        .iter()
        .find(|p| matches!(p.kind, ProjectKind::Move { .. }))
        .expect("they move for water")
        .clone();
    assert!(p.because.contains("no water"), "{}", p.because);
    let to = p.site.expect("a place in the valley");
    assert!(hf_sim::wells::water_within(&w, to, hf_sim::wells::SHORT_WALK).is_some());
}

// ---------------------------------------------------------------------------------------------
// #1721: Seed 4's decline, staged through the real pipeline (deaths, a raid and a rockfall, to
// a proposal to move camp, answers by trust, and a split or a move).
// ---------------------------------------------------------------------------------------------

/// What the staged decline left, read from the world after day 8's morning.
struct Decline {
    w: World,
    cid: CommunityId,
    /// The one the hardships touched most: the child of the first to die.
    p: AgentId,
    /// The members who thought well of P (+0.6) and those who did not trust P (-0.6).
    trusting: Vec<AgentId>,
    distrusting: Vec<AgentId>,
    /// The two who died, the old home, and the ids of the four rows that became hardships.
    dead: [AgentId; 2],
    home: TilePos,
    rows: [EventId; 4],
}

/// Day `d` of the staged arc at `tick_in_day` (the valley starts on day 30 at 08:00).
fn on_day(d: u64, tick_in_day: u64) -> u64 {
    (30 + d) * TICKS_PER_DAY + tick_in_day
}

/// Bring the clock to one tick before `tick`, so that the next `w.step()` runs the whole
/// pipeline once on `tick` itself.
fn clock_to(w: &mut World, tick: u64) {
    w.meta.tick = tick - 1;
}

/// The id of the one row of the kind `f` the last step flushed.
fn the_row(w: &World, f: impl Fn(&EventKind) -> bool) -> EventId {
    let rows: Vec<EventId> = w
        .last_events
        .iter()
        .filter(|e| f(&e.kind))
        .map(|e| e.id)
        .collect();
    assert_eq!(rows.len(), 1, "one such row this tick, not {rows:?}");
    rows[0]
}

/// Nine grown people in the valley, `distrusting` of them at -0.6 of P and the rest at +0.6.
/// Day 1: the first dies (P's mother) "drowned through the ice"; day 3: the second dies "bled
/// out"; day 5: a band's raid on the store, ten tiles from home; day 7: the rock comes down near
/// home. Every step of it is `w.step()`; between the days the clock is set, not run (a week of
/// walking about would change the moods and opinions this scene sets). Returns the world with
/// the clock one tick short of day 8's 08:17, when people take stock of what they want.
fn declining(seed: u64, distrusting: usize) -> Decline {
    let (mut w, cid) = valley_of(seed, 9);
    let home = w.communities.iter().find(|c| c.id == cid).unwrap().home;
    let people = w
        .communities
        .iter()
        .find(|c| c.id == cid)
        .unwrap()
        .members
        .clone();
    assert_eq!(people.len(), 9);
    // P is the member with the HIGHEST id, and every mood is 0 when `consider` runs, so were the
    // dead's kin not weighed, the tie would fall to the lowest id and not to P: only the weight
    // of kin can make P the proposer.
    let (d1, d2, p) = (people[0], people[1], people[8]);
    // The others: `distrusting` of them at -0.6 of P, the rest at +0.6. None is kin of the dead,
    // none has a partner or a child, so nothing holds any of them but what they think of P.
    let others: Vec<AgentId> = people[2..8].to_vec();
    let distrusting_ids: Vec<AgentId> = others[..distrusting].to_vec();
    let trusting_ids: Vec<AgentId> = others[distrusting..].to_vec();
    for a in w.agents.iter_mut() {
        if distrusting_ids.contains(&a.id) {
            a.relationships.entry(p).or_default().opinion = -0.6;
        } else if trusting_ids.contains(&a.id) {
            a.relationships.entry(p).or_default().opinion = 0.6;
        }
    }
    // P is the first to die's child. (A partner is not kin here: `die` unpairs the survivor, so
    // `moving::consider` finds only the dead's children and parents.)
    for a in w.agents.iter_mut() {
        if a.id == d1 {
            a.children.push(p);
        }
        if a.id == p {
            a.mother = Some(d1);
        }
    }
    let name = |w: &World, id: AgentId| w.agents.iter().find(|a| a.id == id).unwrap().name.clone();
    assert_ne!(name(&w, d1), name(&w, d2), "two names to tell apart");

    // Day 1.
    clock_to(&mut w, on_day(1, 8 * TICKS_PER_HOUR));
    hf_sim::systems::life::die(&mut w, d1, "drowned through the ice".into());
    w.step();
    let r1 = the_row(
        &w,
        |k| matches!(k, EventKind::Died { agent, .. } if *agent == d1),
    );
    // Day 3.
    clock_to(&mut w, on_day(3, 8 * TICKS_PER_HOUR));
    hf_sim::systems::life::die(&mut w, d2, "bled out".into());
    w.step();
    let r2 = the_row(
        &w,
        |k| matches!(k, EventKind::Died { agent, .. } if *agent == d2),
    );
    // Day 5: a band's raid on the store, as `threat.rs` pushes it (not open: they were not told
    // they could take it), ten tiles from home. Band 7 is no band of this world.
    assert!(w.bands.iter().all(|b| b.id != 7));
    clock_to(&mut w, on_day(5, 8 * TICKS_PER_HOUR));
    let tick = w.meta.tick;
    let r3 = w.events.push(
        tick,
        Some(home.offset(10, 0, 0)),
        Severity::Notable,
        EventKind::Raided {
            band: 7,
            what: "meals".into(),
            count: 2,
            open: false,
        },
    );
    w.step();
    // Day 7: the roof of a dug room comes down near home (a `Collapse`, which `underground.rs`
    // pushes; a `CaveIn` is pushed by nothing).
    clock_to(&mut w, on_day(7, 8 * TICKS_PER_HOUR));
    let tick = w.meta.tick;
    let r4 = w.events.push(
        tick,
        Some(home.offset(-6, 4, 0)),
        Severity::Notable,
        EventKind::Collapse {
            by: others[0],
            at: home.offset(-6, 4, 0),
            tiles: 12,
            rock: "granite".into(),
            sank: false,
            hurt: Vec::new(),
        },
    );
    w.step();
    Decline {
        w,
        cid,
        p,
        trusting: trusting_ids,
        distrusting: distrusting_ids,
        dead: [d1, d2],
        home,
        rows: [r1, r2, r3, r4],
    }
}

/// The rows of kind `f` in the last step's flush.
fn rows_of<T>(w: &World, f: impl Fn(&EventKind) -> Option<T>) -> Vec<T> {
    w.last_events.iter().filter_map(|e| f(&e.kind)).collect()
}

/// Run the decline to the morning of day 8 and take stock, and check the links up to the
/// proposal; returns what the proposal said and the days' state for the answers' checks.
fn proposal(d: &mut Decline) -> String {
    let w = &mut d.w;
    let name = |w: &World, id: AgentId| w.agents.iter().find(|a| a.id == id).unwrap().name.clone();
    let (n1, n2) = (name(w, d.dead[0]), name(w, d.dead[1]));
    // Link 1: after day 7 the community holds four hardships, each the row that made it, at the
    // 08:00 of its day (the tick `moving::observe` ran on).
    let held = w
        .communities
        .iter()
        .find(|c| c.id == d.cid)
        .unwrap()
        .hardships
        .clone();
    assert_eq!(
        held,
        vec![
            (
                on_day(1, 8 * TICKS_PER_HOUR),
                format!("{n1} was drowned through the ice"),
                d.rows[0]
            ),
            (
                on_day(3, 8 * TICKS_PER_HOUR),
                format!("{n2} was bled out"),
                d.rows[1]
            ),
            (
                on_day(5, 8 * TICKS_PER_HOUR),
                "strangers from beyond the valley took from the store".to_string(),
                d.rows[2]
            ),
            (
                on_day(7, 8 * TICKS_PER_HOUR),
                "the rock came down".to_string(),
                d.rows[3]
            ),
        ]
    );
    // No proposal yet: the daily stocktaking has not come round since the hardships began.
    assert!(!w
        .projects
        .iter()
        .any(|p| matches!(p.kind, ProjectKind::Move { .. })));
    // Link 2: day 8, 08:17. A calm morning (no one's mood is the cause of anything below), and
    // the opinions this scene set still stand.
    for a in w.agents.iter_mut().filter(|a| a.alive) {
        a.mood = 0.0;
    }
    for id in &d.trusting {
        assert_eq!(
            w.agents
                .iter()
                .find(|a| a.id == *id)
                .unwrap()
                .opinion_of(d.p),
            0.6
        );
    }
    for id in &d.distrusting {
        assert_eq!(
            w.agents
                .iter()
                .find(|a| a.id == *id)
                .unwrap()
                .opinion_of(d.p),
            -0.6
        );
    }
    clock_to(w, on_day(8, hf_sim::systems::projects::AT));
    w.step();
    assert_eq!(w.meta.tick % TICKS_PER_DAY, hf_sim::systems::projects::AT);
    // Four hard things, the three latest named, the earliest (the first death) left out of the
    // sentence: `moving::consider` names at most three, newest first.
    let because = format!(
        "the rock came down; strangers from beyond the valley took from the store; \
         {n2} was bled out: four hard things in two seasons"
    );
    let p_name = name(w, d.p);
    // (The planner's other proposals of the morning, a shelter and a well, are not this one.)
    let proposed = rows_of(w, |k| match k {
        EventKind::Proposed {
            agent,
            what,
            because,
        } if what.starts_with("to move camp") || what.starts_with("to leave the valley") => {
            Some((*agent, what.clone(), because.clone()))
        }
        _ => None,
    });
    assert_eq!(
        proposed,
        vec![(
            d.p,
            "to move camp to a better place in the valley".to_string(),
            because.clone()
        )],
        "{p_name} proposes it"
    );
    let moves: Vec<_> = w
        .projects
        .iter()
        .filter(|p| matches!(p.kind, ProjectKind::Move { .. }))
        .collect();
    assert_eq!(moves.len(), 1);
    let mv = moves[0];
    assert_eq!(mv.by, d.p);
    assert_eq!(mv.community, d.cid);
    assert_eq!(mv.because, because);
    assert_eq!(
        mv.source,
        Some(d.rows[3]),
        "the last hardship is its source"
    );
    assert_eq!(mv.proposed, w.meta.tick);
    let to = mv.site.expect("a place in the valley");
    assert_eq!(mv.kind, ProjectKind::Move { to: Some(to) });
    assert!(to.dist(d.home) >= 20, "{to:?} is far from {:?}", d.home);
    // The proposal answers the hardships it names: they are cleared, and not twice in a season.
    let c = w.communities.iter().find(|c| c.id == d.cid).unwrap();
    assert!(c.hardships.is_empty());
    assert_eq!(c.last_move_proposal, w.meta.tick);
    because
}

/// The answers, by trust: each of those at +0.6 of P went with them "thinks well of", each of
/// those at -0.6 stayed because they "do not trust" P, and one line says so for the feed.
fn answers(d: &Decline) {
    let w = &d.w;
    let name = |id: AgentId| w.agents.iter().find(|a| a.id == id).unwrap().name.clone();
    let p_name = name(d.p);
    // (`Chose` is also the row of other choices, "took 4 grain for the road": these are the
    // answers to the proposal.)
    let mut chose = rows_of(w, |k| match k {
        EventKind::Chose {
            agent,
            what,
            because,
        } if what.starts_with("went with") || what == "stayed" => {
            Some((*agent, what.clone(), because.clone()))
        }
        _ => None,
    });
    let mut want: Vec<(AgentId, String, String)> = Vec::new();
    for id in &d.trusting {
        want.push((
            *id,
            format!("went with {p_name}"),
            format!("thinks well of {p_name}"),
        ));
    }
    for id in &d.distrusting {
        want.push((
            *id,
            "stayed".to_string(),
            format!("does not trust {p_name}"),
        ));
    }
    chose.sort();
    want.sort();
    assert_eq!(chose, want);
    let answered = rows_of(w, |k| match k {
        EventKind::Answered {
            leader,
            went,
            stayed,
            reasons,
        } => Some((*leader, *went, *stayed, reasons.clone())),
        _ => None,
    });
    assert_eq!(answered.len(), 1);
    let (leader, went, stayed, mut reasons) = answered[0].clone();
    assert_eq!(leader, d.p);
    assert_eq!(went as usize, d.trusting.len());
    assert_eq!(stayed as usize, d.distrusting.len());
    let mut want_reasons: Vec<String> = d
        .distrusting
        .iter()
        .map(|id| format!("{} does not trust {p_name}", name(*id)))
        .collect();
    reasons.sort();
    want_reasons.sort();
    assert_eq!(reasons, want_reasons);
}

fn sorted(mut v: Vec<AgentId>) -> Vec<AgentId> {
    v.sort();
    v
}

fn members_of(w: &World, cid: CommunityId) -> Vec<AgentId> {
    sorted(
        w.communities
            .iter()
            .find(|c| c.id == cid)
            .unwrap()
            .members
            .clone(),
    )
}

/// #1721: Seed 4's decline, end to end through `World::step`, from real deaths, a raid and a
/// rockfall to a proposal to move camp, the answers by trust, and what the answers make of the
/// camp. Nothing here is seed 4 itself: the scene is staged on a flat valley, on three seeds.
/// Two of the six who answer distrust the proposer, so the camp splits (`Split`: those who went
/// found a community at the new place and the two stay at the old); with one, the whole
/// community moves (`MovedCamp`, the lone one who stayed staying as one of it still).
#[test]
fn a_run_of_hardships_splits_the_camp_by_trust_and_moves_it() {
    for seed in [6101u64, 6102, 6103] {
        // Two distrust P: a split by trust.
        let mut d = declining(seed, 2);
        let because = proposal(&mut d);
        answers(&d);
        let w = &d.w;
        assert!(
            rows_of(w, |k| matches!(k, EventKind::MovedCamp { .. })
                .then_some(()))
            .is_empty(),
            "seed {seed}: two stayed, so it is a split and not a move of the whole"
        );
        let split = rows_of(w, |k| match k {
            EventKind::Split {
                from,
                to,
                leader,
                members,
                cause,
            } => Some((*from, *to, *leader, *members, cause.clone())),
            _ => None,
        });
        assert_eq!(split.len(), 1, "seed {seed}");
        let (from, new_cid, leader, members, cause) = split[0].clone();
        assert_eq!(from, d.cid);
        assert_eq!(leader, d.p);
        // P and the four who thought well of them.
        assert_eq!(members, 5);
        assert_eq!(cause, because);
        let mut went = d.trusting.clone();
        went.push(d.p);
        assert_eq!(members_of(w, new_cid), sorted(went));
        assert_eq!(members_of(w, d.cid), sorted(d.distrusting.clone()));
        let new_home = w.communities.iter().find(|c| c.id == new_cid).unwrap().home;
        let to = w
            .projects
            .iter()
            .find(|p| matches!(p.kind, ProjectKind::Move { .. }))
            .unwrap()
            .site;
        assert_eq!(Some(new_home), to, "they settle where the proposal said");
        assert!(new_home.dist(d.home) >= 20, "seed {seed}: {new_home:?}");
        assert_eq!(
            w.communities.iter().find(|c| c.id == d.cid).unwrap().home,
            d.home,
            "the two who stayed keep the old camp"
        );

        // One distrusts P: the whole community moves, the one who stayed with it still.
        let mut d = declining(seed, 1);
        let because = proposal(&mut d);
        answers(&d);
        let w = &d.w;
        assert!(rows_of(w, |k| matches!(k, EventKind::Split { .. }).then_some(())).is_empty());
        let moved = rows_of(w, |k| match k {
            EventKind::MovedCamp {
                community,
                leader,
                from,
                to,
                cause,
            } => Some((*community, *leader, *from, *to, cause.clone())),
            _ => None,
        });
        assert_eq!(moved.len(), 1, "seed {seed}");
        let (community, leader, from, to, cause) = moved[0].clone();
        assert_eq!(community, d.cid);
        assert_eq!(leader, d.p);
        assert_eq!(from, d.home);
        assert_eq!(cause, because);
        assert!(to.dist(d.home) >= 20, "seed {seed}: {to:?}");
        let c = w.communities.iter().find(|c| c.id == d.cid).unwrap();
        assert_eq!(c.home, to, "a new home");
        let site = w
            .projects
            .iter()
            .find(|p| matches!(p.kind, ProjectKind::Move { .. }))
            .unwrap()
            .site;
        assert_eq!(Some(to), site, "they moved where the proposal said");
        assert_eq!(c.old_camps, vec![(d.home, w.meta.tick)]);
        let mut all = d.trusting.clone();
        all.push(d.p);
        all.extend(d.distrusting.iter().copied());
        assert_eq!(members_of(w, d.cid), sorted(all));
    }
}

#[test]
fn a_hardship_names_a_drowning_plainly() {
    assert_eq!(
        hf_sim::opening::died_how("drowned through the ice"),
        "drowned through the ice"
    );
    assert_eq!(hf_sim::opening::died_how("bled out"), "bled out");
    assert_eq!(hf_sim::opening::died_how("thirst"), "of thirst");
}
