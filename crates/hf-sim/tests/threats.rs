//! Outside pressure (issue #47): a hungry place beyond the valley sends people; they camp, raid,
//! and leave for reasons the feed can name. Settlers answer by what they believe. Wolves in a
//! hungry time go for someone alone, and a beast that hurts someone gets a name.

use std::collections::BTreeSet;

use hf_sim::agents::Species;
use hf_sim::economy::live_counts;
use hf_sim::events::{Event, EventKind};
use hf_sim::ids::{EventId, TilePos};
use hf_sim::mind::beliefs::{Source, Topic};
use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_HOUR, TICKS_PER_YEAR};
use hf_sim::world::gen::GenParams;
use hf_sim::World;

/// An opened world four days in (past the grace after arrival), with one inhabited place beyond the valley gone hungry.
fn hungry_neighbour(seed: u64) -> World {
    let mut w = World::opening_world(seed);
    w.meta.autoplan = true;
    // Under Hask, who leaves a hungry place hungry. Aun (the default) answers a starving
    // neighbour with a better harvest, and in summer it can recover before anyone sets out
    // (seed 19): that is the deity working, but these tests are about the band's arc.
    w.meta.engine = hf_sim::engine::Engine::Relentless;
    for _ in 0..4 * TICKS_PER_DAY {
        w.step();
    }
    let s = w
        .overworld
        .sites
        .iter_mut()
        .find(|s| s.people > 0)
        .expect("an inhabited place beyond the valley");
    s.food = -(s.people as f32) * 4.0;
    s.hungry_days = 3;
    w
}

fn hold_all(w: &mut World, views: &[(Topic, f32)]) {
    for a in w.agents.iter_mut().filter(|a| a.community.is_some()) {
        for (t, s) in views {
            a.beliefs.hold(*t, *s, 0.9, Source::Event(EventId(0)), 0);
        }
    }
}

fn is_band_event(e: &Event) -> bool {
    matches!(
        e.kind,
        EventKind::BandSetOut { .. }
            | EventKind::StrangersSighted { .. }
            | EventKind::Raided { .. }
            | EventKind::Welcomed { .. }
            | EventKind::BandLeft { .. }
            | EventKind::Fought { .. }
    ) || matches!(&e.kind, EventKind::Chose { what, .. } if what.contains("strangers"))
}

/// Run until the first band has gone home (or `days` pass); every band event with its feed line.
fn until_gone(w: &mut World, days: u64) -> Vec<(Event, String)> {
    until_gone_banded(w, days).0
}

/// `until_gone`, and everyone who was of a band at any step (bands are removed when they go).
fn until_gone_banded(
    w: &mut World,
    days: u64,
) -> (
    Vec<(Event, String)>,
    std::collections::BTreeSet<hf_sim::ids::AgentId>,
) {
    let mut log = Vec::new();
    let mut banded = std::collections::BTreeSet::new();
    for _ in 0..days * TICKS_PER_DAY {
        w.step();
        for b in &w.bands {
            banded.extend(b.members.iter().copied());
        }
        if w.meta.tick.is_multiple_of(TICKS_PER_HOUR) {
            for b in &w.bands {
                if let hf_sim::threat::BandState::Camped(_) = b.state {
                    assert!(
                        !hf_sim::systems::hazards::indoors(w, b.camp)
                            && !w
                                .structures
                                .iter()
                                .any(|s| s.pos.z == b.camp.z && s.pos.dist(b.camp) <= 4),
                        "a camp is out in the open, not among the houses: {:?}",
                        b.camp
                    );
                }
            }
        }
        for e in &w.last_events {
            if is_band_event(e) {
                log.push((e.clone(), hf_sim::view::narrate::line(w, e)));
            }
        }
        if log
            .iter()
            .any(|(e, _)| matches!(e.kind, EventKind::BandLeft { .. }))
        {
            break;
        }
    }
    (log, banded)
}

fn ledger_drift(w: &World) -> Vec<String> {
    let live = live_counts(&w.items);
    w.ledger
        .expected()
        .iter()
        .filter(|(k, v)| live.get(*k).copied().unwrap_or(0) != **v)
        .map(|(k, v)| {
            format!(
                "{k}: live {} expected {v}",
                live.get(k).copied().unwrap_or(0)
            )
        })
        .collect()
}

#[test]
fn hunger_sends_a_band() {
    let mut w = hungry_neighbour(3);
    let mut set_out = None;
    // A hungry place sends people on about one day in three.
    for _ in 0..6 * TICKS_PER_DAY {
        w.step();
        for e in &w.last_events {
            if let EventKind::BandSetOut { band, size, .. } = e.kind {
                set_out = Some((band, size));
            }
        }
        if set_out.is_some() {
            break;
        }
    }
    let (band, size) = set_out.expect("a place long hungry sent people within six days");
    let members = w.agents.iter().filter(|a| a.band == Some(band)).count();
    assert_eq!(members as u32, size, "the band is people, here on the map");
    assert!(
        w.agents
            .iter()
            .filter(|a| a.band == Some(band))
            .all(|a| a.community.is_none() && a.species == Species::Person),
        "strangers, not settlers"
    );
    // A place that is fed sends nobody.
    let mut w = World::opening_world(3);
    w.meta.autoplan = true;
    for s in w.overworld.sites.iter_mut() {
        s.food = s.people as f32 * 30.0;
    }
    for _ in 0..4 * TICKS_PER_DAY {
        w.step();
    }
    assert!(w.bands.is_empty(), "no hunger, no band");
}

/// #1282: Hask's new-game word is "the strong are tested; the broken are let be" (deity.rs),
/// but `threat::sites_daily` kept sending bands at a valley already failing or collapsed
/// (fate.rs stage >= 2). Under Hask, a broken valley gets no bands while it stays broken.
#[test]
fn hask_lets_the_broken_be() {
    let mut w = hungry_neighbour(3);
    let c = w
        .communities
        .iter_mut()
        .find(|c| !c.dissolved)
        .expect("the valley's own community");
    c.fate = 2; // failing, per fate.rs's own stage-2 threshold.
    for _ in 0..10 * TICKS_PER_DAY {
        w.step();
        // fate.rs would otherwise recompute this from food and headcount; hold it broken.
        if let Some(c) = w.communities.iter_mut().find(|c| !c.dissolved) {
            c.fate = c.fate.max(2);
        }
        for e in &w.last_events {
            assert!(
                !matches!(e.kind, EventKind::BandSetOut { .. }),
                "Hask let a band set out at a valley it had already broken"
            );
        }
    }
    assert!(
        w.bands.is_empty(),
        "no band should have formed while the valley stayed failing"
    );
    // Once the valley is no longer broken, a long-hungry place may send a band again.
    if let Some(c) = w.communities.iter_mut().find(|c| !c.dissolved) {
        c.fate = 0;
    }
    let mut set_out = false;
    for _ in 0..6 * TICKS_PER_DAY {
        w.step();
        if w.last_events
            .iter()
            .any(|e| matches!(e.kind, EventKind::BandSetOut { .. }))
        {
            set_out = true;
            break;
        }
    }
    assert!(set_out, "a band may come once the valley has recovered");
}

#[test]
fn a_band_stays_raids_by_night_and_leaves_for_a_reason() {
    let mut lasted = 0;
    for seed in [3u64, 7, 11, 19] {
        let mut w = hungry_neighbour(seed);
        // Settlers who would neither welcome nor chase them: the band's own arc.
        hold_all(&mut w, &[(Topic::Strangers, 0.2), (Topic::Violence, -0.9)]);
        // Sixteen days, not twelve (C71: with the day's temperature curve the right way up the
        // neighbours' hunger runs differently, and seed 7's band sets out on the ninth day).
        let log = until_gone(&mut w, 16);
        let start = log
            .iter()
            .find(|(e, _)| matches!(e.kind, EventKind::BandSetOut { .. }))
            .map(|(e, _)| e.tick)
            .expect("a band set out");
        let (end, line) = log
            .iter()
            .find(|(e, _)| matches!(e.kind, EventKind::BandLeft { .. }))
            .map(|(e, l)| (e.tick, l.clone()))
            .unwrap_or_else(|| panic!("seed {seed}: the band left within sixteen days"));
        assert!(!line.is_empty());
        if end - start >= 12 * TICKS_PER_HOUR {
            lasted += 1;
        }
        for (e, l) in &log {
            // (A lost survivor who rejoins later holds their own views, and may welcome them.)
            if let EventKind::Raided { open: false, .. } = e.kind {
                let hour = (e.tick % TICKS_PER_DAY) / TICKS_PER_HOUR;
                assert!(
                    !(6..21).contains(&hour),
                    "seed {seed}: unwelcomed, they take by night ({hour}h): {l}"
                );
            }
        }
        assert!(
            log.iter()
                .any(|(e, _)| matches!(e.kind, EventKind::StrangersSighted { .. })),
            "seed {seed}: someone saw them before they were gone"
        );
    }
    assert!(
        lasted >= 2,
        "a band is a problem for hours or days: {lasted}/4"
    );
}

#[test]
fn settlers_who_hold_with_force_go_after_them() {
    let mut fights = 0;
    let mut chose = 0;
    for seed in [3u64, 7, 11] {
        let mut w = hungry_neighbour(seed);
        hold_all(&mut w, &[(Topic::Strangers, 0.9), (Topic::Violence, 0.9)]);
        for (e, _) in until_gone(&mut w, 12) {
            match &e.kind {
                EventKind::Fought { .. } => fights += 1,
                EventKind::Chose { what, because, .. } if what.contains("went after") => {
                    chose += 1;
                    assert!(!because.is_empty());
                }
                EventKind::Welcomed { .. } => panic!("nobody here welcomes strangers"),
                _ => {}
            }
        }
    }
    assert!(chose >= 2, "{chose}");
    assert!(fights >= 1, "{fights}");
}

#[test]
fn welcomed_strangers_take_openly_and_leave_in_peace() {
    let mut peaceful = 0;
    for seed in [3u64, 7, 11, 19] {
        let mut w = hungry_neighbour(seed);
        hold_all(
            &mut w,
            &[
                (Topic::Strangers, -0.9),
                (Topic::Hunger, -0.9),
                (Topic::Violence, -0.9),
            ],
        );
        let (log, banded) = until_gone_banded(&mut w, 12);
        // Nobody raises a hand against the strangers of the band. (Two of the community may
        // still come to blows in a bad moment: a newcomer who joined after the village was set
        // does not hold its peace. And two wanderers waiting to be taken in are not the band:
        // in C57's run of seed 11 two of them fought each other, and the test, which then took
        // anyone outside the community for a stranger, read it as a fight with the band.)
        let stranger = |id| banded.contains(&id);
        assert!(
            !log.iter().any(|(e, _)| matches!(e.kind,
                EventKind::Fought { a, b } if stranger(a) || stranger(b))),
            "seed {seed}: nobody here raises a hand"
        );
        if log
            .iter()
            .any(|(e, _)| matches!(e.kind, EventKind::BandLeft { peaceful: true, .. }))
        {
            assert!(log
                .iter()
                .any(|(e, _)| matches!(e.kind, EventKind::Welcomed { .. })));
            peaceful += 1;
        }
    }
    assert!(peaceful >= 2, "{peaceful}/4");
}

#[test]
fn a_hungry_wolf_goes_for_someone_alone_and_gets_a_name() {
    let mut attacked = 0;
    let mut named = Vec::new();
    for seed in 0..4u64 {
        let before = attacked;
        let mut w = World::generate(400 + seed, GenParams::small());
        // Winter, and no hares: a hungry time.
        w.meta.tick = TICKS_PER_YEAR + 16 * TICKS_PER_DAY + 10 * TICKS_PER_HOUR;
        w.agents.retain(|a| a.species == Species::Person);
        let p = w.meta.founding_site;
        let person = w.spawn_agent(Species::Person, p);
        let wp = w.grid.surface(p.x + 8, p.y);
        let wolf = w.spawn_agent(Species::Wolf, wp);
        w.ensure_nav();
        for _ in 0..TICKS_PER_DAY {
            w.step();
            for e in &w.last_events {
                match &e.kind {
                    EventKind::Attacked { animal, agent, .. } => {
                        assert_eq!((*animal, *agent), (wolf, person));
                        attacked += 1;
                        let a = w.agents.iter().find(|a| a.id == person).unwrap();
                        assert!(!a.body.wounds.is_empty(), "it left a mark");
                    }
                    EventKind::Named { agent, name, .. } => {
                        assert_eq!(*agent, wolf);
                        named.push(name.clone());
                    }
                    _ => {}
                }
            }
        }
        let _ = before;
    }
    assert!(attacked >= 2, "{attacked}");
    assert!(!named.is_empty());
    assert!(named
        .iter()
        .all(|n| n.chars().next().unwrap().is_uppercase()));
    // Company drives a wolf off: whoever it reaches with others near is not dragged down.
    let mut w = World::generate(400, GenParams::small());
    w.meta.tick = TICKS_PER_YEAR + 16 * TICKS_PER_DAY + 10 * TICKS_PER_HOUR;
    w.agents.retain(|a| a.species == Species::Person);
    let p = w.meta.founding_site;
    for k in 0..3 {
        w.spawn_agent(Species::Person, TilePos::new(p.x + k, p.y, p.z));
    }
    let wp = w.grid.surface(p.x + 8, p.y);
    w.spawn_agent(Species::Wolf, wp);
    w.ensure_nav();
    for _ in 0..TICKS_PER_DAY {
        w.step();
        for e in &w.last_events {
            if let EventKind::Attacked {
                agent, driven_off, ..
            } = e.kind
            {
                let v = w.agents.iter().find(|a| a.id == agent).unwrap().pos;
                let company = w
                    .agents
                    .iter()
                    .filter(|a| a.alive && a.species == Species::Person && a.id != agent)
                    .any(|a| a.pos.z == v.z && a.pos.dist(v) <= 6);
                assert!(
                    !company || driven_off,
                    "with others near, it was driven off"
                );
            }
        }
    }
}

#[test]
fn the_feed_tells_the_arc_in_plain_words() {
    let mut w = hungry_neighbour(19);
    let log = until_gone(&mut w, 12);
    let lines: Vec<&str> = log.iter().map(|(_, l)| l.as_str()).collect();
    for l in &lines {
        assert!(!l.contains('#') && !l.contains('{'), "{l}");
    }
    assert!(
        lines.iter().any(|l| l.contains("set out from")),
        "{lines:?}"
    );
    assert!(
        lines
            .last()
            .map(|l| l.contains("strangers") || l.contains("Strangers"))
            .unwrap_or(false),
        "{lines:?}"
    );
}

#[test]
fn raids_keep_the_ledger_and_raiders_leave_the_world() {
    // Issue #90: food a raider carried off left the item list without a ledger entry, and the
    // band walked into overworld.away, where nothing ever takes them out again.
    let mut raided = 0;
    for seed in [3u64, 7] {
        let mut w = hungry_neighbour(seed);
        hold_all(&mut w, &[(Topic::Strangers, 0.2), (Topic::Violence, -0.9)]);
        let mut members = BTreeSet::new();
        let mut left_at = None;
        let mut last_left = None;
        for _ in 0..24 * TICKS_PER_DAY {
            w.step();
            for b in &w.bands {
                members.extend(b.members.iter().copied());
            }
            for e in &w.last_events {
                match e.kind {
                    EventKind::Raided { .. } => raided += 1,
                    EventKind::BandLeft { .. } => {
                        left_at.get_or_insert(e.tick);
                        last_left = Some(e.tick);
                    }
                    _ => {}
                }
            }
            if w.meta.tick.is_multiple_of(TICKS_PER_HOUR) {
                let drift = ledger_drift(&w);
                assert!(drift.is_empty(), "seed {seed}: ledger drift {drift:?}");
            }
            // Every band gone (since M3 more than one neighbour can send one), and a day for
            // any hurt straggler to reach the edge as well.
            if w.bands.is_empty() && last_left.is_some_and(|t| w.meta.tick > t + TICKS_PER_DAY) {
                break;
            }
        }
        assert!(left_at.is_some(), "seed {seed}: the band left");
        assert!(!members.is_empty());
        for m in &members {
            let a = w.agents.iter().find(|a| a.id == *m);
            // A raider brought down and kept by the valley (D47) is held, not left behind: they
            // are ransomed or taken in on that clock. (With #415 seed 7 kept one a day before
            // the end.)
            if a.is_some_and(|a| a.alive && a.held.is_some()) {
                continue;
            }
            assert!(
                a.is_some_and(|a| !a.alive),
                "seed {seed}: {m:?} is gone from the valley"
            );
            assert!(
                !w.overworld.away.iter().any(|a| a.id == *m),
                "seed {seed}: {m:?} is not left waiting in overworld.away"
            );
            assert!(
                !w.items
                    .iter()
                    .any(|it| it.place == hf_sim::items::ItemPlace::Carried(*m)),
                "seed {seed}: nothing is left in the hands of {m:?}"
            );
        }
    }
    assert!(raided >= 1, "the bands took something: {raided}");
}

#[test]
fn a_hurt_raider_leaves_and_is_never_judged_a_newcomer() {
    // Issue #95: a raider hurt in a fight left the band for the edge, but nothing took them
    // off the map there, and the settlement could judge (and take in) the one who robbed it.
    let mut w = hungry_neighbour(7);
    hold_all(&mut w, &[(Topic::Strangers, 0.9), (Topic::Violence, -0.9)]);
    // Wait for a band to set out (when it does depends on the run's path).
    for _ in 0..20 * TICKS_PER_DAY {
        w.step();
        if !w.bands.is_empty() {
            break;
        }
    }
    let raider = *w.bands[0].members.first().expect("a band set out");
    let settler = w
        .agents
        .iter()
        .find(|a| a.alive && a.community.is_some())
        .map(|a| (a.id, a.pos))
        .unwrap();
    // The raider is caught among the houses, already hurt.
    let ri = w.agents.iter().position(|a| a.id == raider).unwrap();
    w.agents[ri].pos = settler.1;
    w.agents[ri].from = settler.1;
    w.agents[ri].path.clear();
    w.agents[ri].activity = hf_sim::mind::Activity::Idle;
    // Fed, so hunger does not end them first: a band from a starving place can take long
    // enough to set out that its people starve on the road (seen since M3 changed the path).
    w.agents[ri].needs.hunger = 1.0;
    hf_sim::systems::health::wound(
        &mut w,
        ri,
        hf_sim::body::Part::TORSO,
        hf_sim::body::WoundKind::Cut,
        0.45,
        0.0,
        "test",
        None,
    );
    // No roof to keep a prisoner under (C76, siege.rs `keep`): this is about one let go.
    for c in w.communities.iter_mut() {
        c.house_sites.clear();
        c.palisade = None;
    }
    hf_sim::threat::pursue(&mut w, settler.0, raider);
    let mut left = false;
    for _ in 0..3 * TICKS_PER_DAY {
        w.step();
        for e in &w.last_events {
            match e.kind {
                EventKind::Joined { agent, .. } | EventKind::TurnedAway { agent, .. } => {
                    assert_ne!(
                        agent, raider,
                        "a leaving raider is not judged as a newcomer"
                    )
                }
                EventKind::Left { agent } if agent == raider => left = true,
                _ => {}
            }
        }
        if left {
            break;
        }
    }
    let a = w.agents.iter().find(|a| a.id == raider).unwrap();
    assert!(a.band.is_none(), "out of the band");
    assert!(left && !a.alive, "the hurt raider left the valley");
    assert!(a.community.is_none());
}

// ---- #1564: a band set down far from its camp, behind water over the head ------------------
//
// Why this scene: a band is set down on the map edge (threat.rs `spawn_band`) and each band step
// sends every member not near the camp with `go`, which calls `World::path_agent_to` and does
// nothing when that returns false (threat.rs `go`). The staged valley below has the geometry of
// seed 4 after day 553 (home (106, 17), south edge (106, 125), camp about 84 tiles from the
// edge) with a river across the whole valley between them, which seed 4's own map did not have
// in the runs checked. The tests read what `path_agent_to` does, check by check, and what the
// band does about it. FIXLOG 2026-11-15 #1564 has the finding and says what it does not show.

/// A floor tile of water. `WorldGrid::walkable` lets a person stand on it (no DEEP flag); it is
/// over a person's head (`WorldGrid::over_head`) only when the tile above is water as well.
fn water_floor() -> hf_sim::world::Tile {
    use hf_sim::world::{Material, Shape, Tile, TileFlags};
    Tile {
        material: Material::WATER,
        shape: Shape::Floor,
        flags: TileFlags(TileFlags::WATER),
    }
}

/// The river across the staged valley.
#[derive(Clone, Copy)]
enum River {
    /// Water over the head (levels 2 and 3), `width` rows of it from y 70, in every column except
    /// the column `ford` if given (dry ground there).
    Deep { ford: Option<i32>, width: i32 },
    /// A river two rows wide (y 70 and 71) one level deep (level 2 only): wading, never over the head.
    Wading,
}

/// The first row of the river; it runs to y 70 + width - 1. Two rows is the width of the river
/// most of the tests use: a straight walk across it spends two tiles in it.
const RIVER_TOP: i32 = 70;
const RIVER_WIDTH: usize = 2;

/// A valley 128 tiles square (so its south edge walk row is y = 128 - 3 = 125), flat at level 1
/// (people stand at 2), the settlers' home at (106, 17) and the river across all of it from y 70.
/// Calm clear weather, so the river stays as it is made.
fn far_valley(seed: u64, river: River) -> World {
    let mut w = World::generate(seed, GenParams::default());
    w.flatten(0, 0, 128, 128, 1);
    w.plants.clear();
    w.meta.founding_site = TilePos::new(106, 17, 2);
    // Day 30, 08:00 (docs/TIME.md: 28,800 ticks a day, 1,200 an hour).
    w.meta.tick = TICKS_PER_DAY * 30 + TICKS_PER_HOUR * 8;
    let width = match river {
        River::Deep { width, .. } => width,
        River::Wading => 2,
    };
    for y in RIVER_TOP..RIVER_TOP + width {
        for x in 0..=127 {
            match river {
                River::Deep { ford, .. } if ford == Some(x) => {}
                River::Deep { .. } => {
                    w.grid.set(TilePos::new(x, y, 2), water_floor());
                    w.grid.set(TilePos::new(x, y, 3), water_floor());
                }
                River::Wading => {
                    w.grid.set(TilePos::new(x, y, 2), water_floor());
                }
            }
        }
    }
    w.rebuild_caches();
    hf_sim::floods::survey(&mut w);
    let mut wx = hf_sim::systems::weather::current(&w);
    wx.sky = hf_sim::systems::weather::Sky::Clear;
    w.force_weather = Some(wx);
    w.weather = wx;
    w.found_party(8);
    w.meta.autoplan = true;
    w.meta.engine = hf_sim::engine::Engine::Relentless;
    w.ensure_nav();
    w
}

/// `far_valley` with a hungry place to its south and the band that place sends, the tick it is
/// set down (the members stand on their set-down tiles; the first band step is 46 ticks on).
/// Returns the band's id.
fn far_band(seed: u64, river: River) -> (World, u32) {
    far_band_from(seed, river, hf_sim::expedition::Edge::South)
}

/// `far_band` with the hungry place on the given side of the valley (its own edge, which
/// `spawn_band` tries first). The river runs west to east at y 70-71, so the South edge is on
/// the far side of it from home (106, 17) and the North, East and West edges are on home's side.
fn far_band_from(seed: u64, river: River, edge: hf_sim::expedition::Edge) -> (World, u32) {
    let mut w = far_valley(seed, river);
    let s = w
        .overworld
        .sites
        .first_mut()
        .expect("a place beyond the valley");
    s.edge = edge;
    // 47 people: a hungry band is (people / 10) clamped to 2..=5 = 47 / 10 = 4 of them.
    s.people = 47;
    s.food = -(s.people as f32) * 4.0;
    s.hungry_days = 3;
    s.last_band = 0;
    s.grudge = 0;
    for _ in 0..12 * TICKS_PER_DAY {
        w.step();
        for e in &w.last_events {
            if let EventKind::BandSetOut { band, .. } = e.kind {
                return (w, band);
            }
        }
    }
    panic!("no band set out within 12 days");
}

/// What `path_agent_to` (world_state.rs) looks at for a walk `from` to `to`, in the order it looks,
/// read on a scratch search so that nothing in the world changes.
#[derive(Debug, PartialEq)]
struct WalkCheck {
    /// `go` asks `grid.walkable(to)` first.
    walkable: bool,
    /// `nav.reach(from, to)`: false is cached as a failed walk and refused.
    reach: bool,
    /// The failed-walk cache already holds this pair.
    cached_failure: bool,
    /// The A* search within `DEFAULT_MAX_EXPANSIONS` (12,000): "found", "unreachable" or "budget".
    search: &'static str,
    /// Tiles on the path found, start and goal included.
    path_len: usize,
    /// Of those, the tiles over a person's head: the #418 rule refuses a path with any of them
    /// when the walker's own tile is not over their head.
    over_head: usize,
    start_over_head: bool,
}

fn check_walk(w: &World, from: TilePos, to: TilePos) -> WalkCheck {
    use hf_sim::nav::{find_path, AStarScratch, PathResult};
    let mut scratch = AStarScratch::new(&w.grid);
    let mut c = WalkCheck {
        walkable: w.grid.walkable(to),
        reach: w.nav.as_ref().expect("nav").reach(from, to),
        cached_failure: w.astar.as_ref().expect("astar").failed_path(from, to),
        search: "",
        path_len: 0,
        over_head: 0,
        start_over_head: w.grid.over_head(from),
    };
    match find_path(
        &w.grid,
        &mut scratch,
        from,
        to,
        hf_sim::nav::astar::DEFAULT_MAX_EXPANSIONS,
    ) {
        PathResult::Found(p) => {
            c.search = "found";
            c.path_len = p.len();
            c.over_head = p.iter().filter(|q| w.grid.over_head(**q)).count();
        }
        PathResult::Unreachable => c.search = "unreachable",
        PathResult::Budget => c.search = "budget",
    }
    c
}

fn idx_of(w: &World, id: hf_sim::ids::AgentId) -> usize {
    w.agents.iter().position(|a| a.id == id).expect("agent")
}

/// #1564: a band set down on the south edge, its camp 80-odd tiles north across a river over the
/// head that has no ford, never takes a step. Of the candidates in the issue, only the refusal of
/// a path through water over the head holds here; each other one is ruled out by a line below.
/// This test documents today's behaviour. The river runs across every column, so no way round
/// exists and a slice that finds one (or walks in legs) cannot turn it over; only a band that is
/// set out by another edge can, which is #1566 ("else the next edge").
#[test]
fn a_band_set_down_behind_deep_water_does_not_take_a_step() {
    let (mut w, band) = far_band(
        4,
        River::Deep {
            ford: None,
            width: 2,
        },
    );
    let b = w.bands.iter().find(|b| b.id == band).unwrap().clone();
    let home = w.communities[0].home;
    assert_eq!(home, TilePos::new(106, 17, 2), "the settlers' home");
    assert_eq!(b.size, 4, "47 people / 10");
    assert_eq!(b.members.len(), 4);
    // Set down on the south edge row: edge_tile is (home.x 106, 128 - 3 = 125); member k stands at
    // x = 106 + k % 3 - 1, y = 125 + k / 3 clamped to 125 (threat.rs spawn_band).
    let set_down: Vec<TilePos> = b
        .members
        .iter()
        .map(|m| w.agents[idx_of(&w, *m)].pos)
        .collect();
    assert_eq!(
        set_down,
        vec![
            TilePos::new(105, 125, 2),
            TilePos::new(106, 125, 2),
            TilePos::new(107, 125, 2),
            TilePos::new(105, 125, 2),
        ]
    );
    // The camp: CAMP_DIST 24 south of home, jittered by up to 3: y = 17 + 24 +- 3 (one less
    // where the float product truncates) = 37..=44, x = 106 +- 3 = 103..=109.
    assert!(
        (37..=44).contains(&b.camp.y) && (103..=109).contains(&b.camp.x),
        "{:?}",
        b.camp
    );
    // From any set-down tile the walk is 125 - camp.y steps (diagonals allowed, |dx| <= 4 < dy),
    // so 125 - camp.y + 1 tiles on the path, start and goal included.
    let want_len = (125 - b.camp.y + 1) as usize;

    for from in &set_down {
        let c = check_walk(&w, *from, b.camp);
        assert_eq!(
            c,
            WalkCheck {
                // not "(d) the camp is not a place to stand"
                walkable: true,
                // not "(c)/(d) the tiles are not joined": the nav regions ignore water depth
                reach: true,
                // not "(a) a failed walk is cached" (nothing cached yet)
                cached_failure: false,
                // not "(a) the 12,000-expansion limit": the search finds the way
                search: "found",
                path_len: want_len,
                // the way found goes through the river, two tiles of it over the head
                over_head: RIVER_WIDTH,
                start_over_head: false,
            },
            "from {from:?}"
        );
    }

    // What `go` does: `path_agent_to` refuses, leaves the member as they are, and caches nothing,
    // so the same search is made again at every band step (every 50 ticks).
    let i = idx_of(&w, b.members[0]);
    assert!(!w.path_agent_to(i, b.camp));
    assert!(w.agents[i].path.is_empty());
    assert!(
        !w.astar.as_ref().unwrap().failed_path(set_down[0], b.camp),
        "the refusal is not cached"
    );

    // Twelve game-hours on (14,400 ticks): nobody has moved more than 3 tiles from where they
    // were set down and nobody is within 4 tiles of the camp. (Nobody in this valley can drink, the
    // settlers included, so thirst says nothing about the band's walk and is not asserted.)
    for _ in 0..12 * TICKS_PER_HOUR {
        w.step();
    }
    for (m, from) in b.members.iter().zip(&set_down) {
        let a = &w.agents[idx_of(&w, *m)];
        assert!(a.alive, "{}", a.name);
        assert!(a.pos.dist(*from) <= 3, "{} at {:?}", a.name, a.pos);
        assert!(a.pos.dist(b.camp) > 4, "{} at {:?}", a.name, a.pos);
    }
}

/// #1564: the controls for the refusal, on the same valley with a settler stood on the south edge
/// and the camp tile (104, 40): the walk is refused when the way found goes through water over the
/// head and the walker is not already in it, and only then.
#[test]
fn the_walk_is_refused_only_for_a_path_through_water_over_the_head() {
    let camp = TilePos::new(104, 40, 2);
    let from = TilePos::new(106, 125, 2);
    // (name, river, expected: path_agent_to, A*'s over-head tiles on the path it found, and the
    // path's length in tiles). Straight from (106, 125) to (104, 40) is 125 - 40 = 85 steps, so
    // 86 tiles, start and goal included.
    let cases: [(&str, River, bool, usize, usize); 5] = [
        // A river one level deep: wading, nothing over the head. Walked.
        ("wading", River::Wading, true, 0, 86),
        // Water over the head one row wide, no ford: one tile over the head is enough to refuse.
        // (Every other refused case here has two such tiles, so without this one a rule of "two or
        // more" would pass.)
        (
            "one row, no ford",
            River::Deep {
                ford: None,
                width: 1,
            },
            false,
            1,
            86,
        ),
        // Water over the head, two rows, one dry column at x 100, 6 tiles from the straight line
        // (the camp is at x 104, the walker at x 106): A* takes the ford. Walked.
        (
            "near ford",
            River::Deep {
                ford: Some(100),
                width: 2,
            },
            true,
            0,
            86,
        ),
        // The same river with its only ford 86 columns west (x 20): A* still goes straight across
        // (nav/step.rs OVER_HEAD makes each deep tile cost 200 more, and the way round costs less
        // than that only in the next case), so `path_agent_to` refuses the path it is given and
        // makes no search for a way round.
        (
            "far ford, two rows",
            River::Deep {
                ford: Some(20),
                width: 2,
            },
            false,
            RIVER_WIDTH,
            86,
        ),
        // A river three rows wide with the same far ford: here A* goes round by the ford (a path
        // of 173 tiles against 86 straight), so the walk is accepted. Whether it goes round or
        // across turns on the river's width against the length of the detour.
        (
            "far ford, three rows",
            River::Deep {
                ford: Some(20),
                width: 3,
            },
            true,
            0,
            173,
        ),
    ];
    for (name, river, accepted, over, len) in cases {
        let mut w = far_valley(4, river);
        let i = w.agents.iter().position(|a| a.alive).unwrap();
        w.agents[i].pos = from;
        w.agents[i].path.clear();
        let c = check_walk(&w, from, camp);
        assert_eq!(
            (c.search, c.over_head, c.path_len),
            ("found", over, len),
            "{name}: {c:?}"
        );
        assert_eq!(w.path_agent_to(i, camp), accepted, "{name}");
        assert_eq!(!w.agents[i].path.is_empty(), accepted, "{name}");
    }
    // Someone already in the water over their head may go through it to get out (world_state.rs):
    // a person standing in the river at (105, 71, 2) is given the path across, over-head tiles and all.
    let mut w = far_valley(
        4,
        River::Deep {
            ford: None,
            width: 2,
        },
    );
    let i = w.agents.iter().position(|a| a.alive).unwrap();
    let in_river = TilePos::new(105, 71, 2);
    w.agents[i].pos = in_river;
    w.agents[i].path.clear();
    assert!(w.grid.over_head(in_river));
    let c = check_walk(&w, in_river, camp);
    assert_eq!(c.search, "found");
    // The path is over the head at its start (row 71) and at row 70, and nowhere else: 2 tiles.
    assert_eq!((c.over_head, c.start_over_head), (2, true), "{c:?}");
    assert!(w.path_agent_to(i, camp));
    assert!(!w.agents[i].path.is_empty());
}

/// What the issue's Acceptance asks of a band set down far from its camp: within two game-days
/// every member comes within 4 tiles of the camp (the nearest each gets over all the ticks, since
/// they forage once they are there), and then nobody of the band has died of thirst. The distance
/// is asserted first, so that a band that never walks fails with the walk's message. The thirst half
/// is not about the walk: in this valley a band camped at (104, 40) dies of thirst even when it
/// walks in (a reviewer tried a wading river and a ford 6 columns off by hand: both fail the
/// thirst half with the distance half met), so a correct walk alone would not pass it.
fn assert_band_comes_in(w: &mut World, band: u32) {
    let b = w.bands.iter().find(|b| b.id == band).unwrap().clone();
    let mut nearest: Vec<i32> = b
        .members
        .iter()
        .map(|m| w.agents[idx_of(w, *m)].pos.dist(b.camp))
        .collect();
    let mut thirst_deaths = Vec::new();
    // OST-205: a band walked in parched now reads as miserable and may break down on the way;
    // someone in a breakdown who dies of thirst is not what this gate is about (the walk and
    // the water are), so those are left out of the count.
    let mut broke = Vec::new();
    for _ in 0..2 * TICKS_PER_DAY {
        w.step();
        for e in &w.last_events {
            if let EventKind::Broke { agent, .. } = &e.kind {
                broke.push(*agent);
            }
            if let EventKind::Died { agent, cause } = &e.kind {
                if b.members.contains(agent) && cause.contains("thirst") && !broke.contains(agent) {
                    thirst_deaths.push(*agent);
                }
            }
        }
        for (k, m) in b.members.iter().enumerate() {
            let a = &w.agents[idx_of(w, *m)];
            if a.alive {
                nearest[k] = nearest[k].min(a.pos.dist(b.camp));
            }
        }
    }
    for (k, m) in b.members.iter().enumerate() {
        let a = &w.agents[idx_of(w, *m)];
        assert!(
            nearest[k] <= 4,
            "{} came no nearer than {} tiles to the camp {:?}",
            a.name,
            nearest[k],
            b.camp
        );
    }
    assert!(
        thirst_deaths.is_empty(),
        "died of thirst: {thirst_deaths:?}"
    );
}

/// #1564 (the issue's Acceptance): a band whose own edge is on the far side of a river it cannot
/// cross reaches its camp. Fails today at the distance half ("came no nearer than 85 tiles"): the
/// band stands on the south edge (see the first test above). The valley has no ford, so no search
/// for a way round and no walking in legs can turn it. Nor would a walk alone: the thirst half
/// fails for any band camped at (104, 40) in this valley, walked in or not. A band set out by an
/// edge it can walk in from, whose camp is elsewhere, can pass both, which is #1566.
#[test]
#[ignore = "#1566: a band whose way in from its own edge is refused must be set out by an edge it can walk in from"]
fn a_band_from_the_far_side_reaches_its_camp() {
    let (mut w, band) = far_band(
        4,
        River::Deep {
            ford: None,
            width: 2,
        },
    );
    assert_band_comes_in(&mut w, band);
}

/// #1564: the control for the test above: the same check passes today for the same valley and
/// the same hungry place when the band comes in by the East edge, which is on home's side of the
/// river. (Its members are set down within 2 tiles of their camp there (the camp is moved by the
/// edge), so the distance half is met from the start and the check there is the thirst half; the
/// camp at (104, 40) of the test above is where thirst kills even a band that walks in.)
#[test]
fn the_acceptance_check_passes_for_a_band_that_comes_in_by_the_east_edge() {
    let (mut w, band) = far_band_from(
        4,
        River::Deep {
            ford: None,
            width: 2,
        },
        hf_sim::expedition::Edge::East,
    );
    assert_band_comes_in(&mut w, band);
}
