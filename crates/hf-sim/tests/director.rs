//! The camera's sense of where the settlement is (#104): the valley's past leaves graves and
//! wells far across the map, and those are not the settlement.

use hf_sim::events::{Event, EventKind, Severity};
use hf_sim::ids::{AgentId, EventId, ItemId, TilePos};
use hf_sim::structures::StructureKind;
use hf_sim::view::director::{self, Director};
use hf_sim::World;

#[test]
fn the_settlement_is_where_its_people_live_not_where_the_old_graves_lie() {
    for seed in [3u64, 7] {
        let w = World::opening_world(seed);
        let site = w.meta.founding_site;
        let far_past = w
            .structures
            .iter()
            .filter(|s| matches!(s.kind, StructureKind::Grave | StructureKind::Well))
            .filter(|s| s.pos.dist(site) > 28)
            .count();
        assert!(far_past > 0, "seed {seed}: the valley has a past to ignore");
        let c = director::settlement_centre(&w);
        let reach = (c.x - site.x).abs().max((c.y - site.y).abs());
        assert!(
            reach <= director::SETTLEMENT_REACH,
            "seed {seed}: centre {c:?} is {reach} from the landing {site:?}"
        );
        assert_eq!(c.z, site.z, "seed {seed}: on the landing's level");
    }
}

fn mk(id: u32, tick: u64, at: TilePos, severity: Severity, kind: EventKind) -> Event {
    Event {
        id: EventId(id),
        tick,
        at: Some(at),
        severity,
        kind,
    }
}

/// #1606: a cell stacked with routine Minor events (hauling to a stockpile) no longer
/// outscores a cell with a single Major death, because repeats of one kind in one cell
/// are discounted (director.rs `suggest`, harmonic in the n-th occurrence).
#[test]
fn a_death_outweighs_a_stockpile_of_hauling() {
    let a = TilePos::new(0, 0, 0);
    let b = TilePos::new(200, 200, 0);
    let mut events = Vec::new();
    for i in 0..30u32 {
        events.push(mk(
            i,
            100,
            a,
            Severity::Minor,
            EventKind::Hauled {
                agent: AgentId(0),
                item: ItemId(0),
                to: a,
                load: None,
            },
        ));
    }
    events.push(mk(
        30,
        100,
        b,
        Severity::Major,
        EventKind::Died {
            agent: AgentId(1),
            cause: "drowned".into(),
        },
    ));
    let mut director = Director::default();
    director.observe(100, &events);
    // Same presence (nobody awake in either cell) and recency (all observed at tick 100).
    let suggestion = director.suggest(100, TilePos::new(1000, 1000, 0), &[]);
    assert_eq!(
        suggestion.at, b,
        "a single death should outweigh 30 stacked `hauled` events in another cell, got {suggestion:?}"
    );
}

/// #1606 acceptance: with no events at all, a cell where people are working still beats
/// an empty fallback cell — the repeat discount must not touch the people term.
#[test]
fn busy_people_still_draw_the_camera() {
    let work_cell = TilePos::new(0, 0, 0);
    let fallback = TilePos::new(500, 500, 0);
    let people = vec![(work_cell, true), (work_cell, true), (work_cell, true)];
    let mut director = Director::default();
    let suggestion = director.suggest(100, fallback, &people);
    assert_eq!(
        suggestion.at, work_cell,
        "three busy people should draw the camera over an empty fallback cell, got {suggestion:?}"
    );
}

/// A death with nobody beside it still draws the camera over one busy person
/// elsewhere (Major events about a person take full presence).
#[test]
fn a_death_alone_in_the_woods_is_shown() {
    let death = TilePos::new(0, 0, 0);
    let worker = TilePos::new(30, 0, 0);
    let mut director = Director::default();
    director.observe(
        100,
        &[mk(
            1,
            100,
            death,
            Severity::Major,
            EventKind::Died {
                agent: AgentId(1),
                cause: "thirst".into(),
            },
        )],
    );
    let suggestion = director.suggest(100, TilePos::new(1000, 1000, 0), &[(worker, true)]);
    assert_eq!(
        suggestion.at, death,
        "a lonely death should outscore one busy person, got {suggestion:?}"
    );
}
