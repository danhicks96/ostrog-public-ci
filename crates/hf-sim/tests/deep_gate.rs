//! #322: a walled way down to the deep is kept by the one who walled it, and can be opened.

use hf_sim::events::EventKind;
use hf_sim::ids::TilePos;
use hf_sim::world::{Material, Tile};
use hf_sim::World;

/// A world with a deep cavern its community broke into through a gallery dug from the east,
/// walled again. Returns the world, the cavern's index, the wall's tile and one stand outside.
pub fn walled_way_down() -> (World, usize, TilePos, TilePos) {
    let mut w = World::from_setup(&hf_sim::setup::WorldSetup::new(52));
    let k = w.grid.caverns.iter().position(|c| c.deep).unwrap();
    let c = w.grid.caverns[k].clone();
    // A gallery out of the cavern's east side, five tiles long.
    let z = c.centre.z;
    let first = (0..)
        .map(|n| c.centre.x + n)
        .find(|&x| !c.contains(TilePos::new(x, c.centre.y, z)))
        .unwrap();
    for n in -1..6 {
        let p = TilePos::new(first + n, c.centre.y, z);
        w.grid.set(p, Tile::AIR);
        w.grid
            .set(p.offset(0, 0, -1), Tile::solid(Material::BASALT));
    }
    let entry = TilePos::new(first, c.centre.y, z);
    let stand = entry.offset(1, 0, 0);
    let i = w
        .agents
        .iter()
        .position(|a| a.alive && a.community.is_some() && !a.is_child(w.meta.tick))
        .unwrap();
    w.agents[i].pos = c.centre;
    w.agents[i].from = entry;
    hf_sim::underground::breaches(&mut w);
    assert_eq!(w.grid.caverns[k].breach, Some(entry));
    // Walled, by someone standing at it.
    w.agents[i].pos = stand;
    w.grid.set(entry, Tile::solid(Material::STONE_BLOCK));
    hf_sim::underground::rise(&mut w);
    assert!(w.grid.caverns[k].sealed);
    w.events.pending.clear();
    (w, k, entry, stand)
}

#[test]
fn the_one_who_walls_the_way_down_keeps_it_and_it_can_be_opened() {
    let (mut w, k, entry, stand) = walled_way_down();
    let keeper = w.grid.caverns[k].keeper.expect("the waller keeps the way");
    let i = w.agents.iter().position(|a| a.id == keeper).unwrap();
    assert_eq!(w.agents[i].pos, stand);
    assert_eq!(
        w.grid.caverns[k].gate_of, w.agents[i].community,
        "the way down is the community's"
    );
    // Someone away from it cannot open it.
    w.agents[i].pos = stand.offset(3, 0, 0);
    assert_eq!(hf_sim::underground::open_gate(&mut w, i), None);
    w.agents[i].pos = stand;
    assert_eq!(hf_sim::underground::open_gate(&mut w, i), Some(k));
    assert!(!w.grid.caverns[k].sealed);
    assert!(!w.grid.get(entry).shape.is_solid(), "the wall is cut out");
    assert!(w
        .events
        .pending
        .iter()
        .any(|e| matches!(e.kind, EventKind::GateOpened { agent, .. } if agent == keeper)));
    // Walled again, it is sealed again.
    w.grid.set(entry, Tile::solid(Material::STONE_BLOCK));
    hf_sim::underground::rise(&mut w);
    assert!(w.grid.caverns[k].sealed);
}

/// The walled way down, a society of three against a parent, and the parent's child in the
/// gallery beyond the wall. With `keeper_in` the keeper is one of the three; without, the one
/// angriest at the parent stands by the wall in the keeper's place, and the keeper is elsewhere.
/// Returns (world, cavern, wall, target, child, keeper, the one who will open it).
pub fn society_by_the_wall(
    keeper_in: bool,
) -> (World, usize, TilePos, AgentId, AgentId, AgentId, AgentId) {
    use hf_sim::time::TICKS_PER_YEAR;
    let (mut w, k, entry, stand) = walled_way_down();
    // The deep's nearest people lie under the valley.
    let region = w.meta.setup.as_ref().and_then(|s| s.region).unwrap();
    let ds = w
        .realm
        .sites
        .iter()
        .position(|s| s.deep && s.alive())
        .unwrap();
    w.realm.sites[ds].region = region;
    assert!(hf_sim::underground::deep_neighbour(&w).is_some());
    let cid = w.grid.caverns[k].gate_of.unwrap();
    let keeper = w.grid.caverns[k].keeper.unwrap();
    let home = w.communities.iter().find(|c| c.id == cid).unwrap().home;
    let more = w.arrive_party(5, home, Some(cid));
    let t = w.meta.tick as i64;
    for id in &more {
        let i = w.agents.iter().position(|a| a.id == *id).unwrap();
        w.agents[i].born = t - 30 * TICKS_PER_YEAR as i64;
    }
    let target = more[0];
    let child = more[3];
    let ci = w.agents.iter().position(|a| a.id == child).unwrap();
    w.agents[ci].born = t - 6 * TICKS_PER_YEAR as i64;
    w.agents[ci].mother = Some(target);
    w.agents[ci].pos = entry.offset(4, 0, 0);
    w.agents[ci].activity = hf_sim::mind::Activity::Idle;
    let ti = w.agents.iter().position(|a| a.id == target).unwrap();
    w.agents[ti].children.push(child);
    let (members, opener) = if keeper_in {
        (vec![keeper, more[1], more[2]], keeper)
    } else {
        let ki = w.agents.iter().position(|a| a.id == keeper).unwrap();
        w.agents[ki].pos = home;
        let oi = w.agents.iter().position(|a| a.id == more[1]).unwrap();
        w.agents[oi].pos = stand;
        let now = w.meta.tick;
        w.agents[oi].feelings.feel(
            hf_sim::mind::feelings::FeelingKind::Anger,
            Some(target),
            0.9,
            now,
            None,
        );
        (vec![more[1], more[2], more[4]], more[1])
    };
    w.societies.push(hf_sim::societies::Society {
        community: cid,
        against: target,
        members,
        formed: w.meta.tick,
        aim: None,
        struck: false,
    });
    (w, k, entry, target, child, keeper, opener)
}

use hf_sim::ids::AgentId;
use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_HOUR};

fn at(day: u64, hour: u64) -> u64 {
    day * TICKS_PER_DAY + hour * TICKS_PER_HOUR + 77
}

#[test]
fn a_society_lets_the_deep_up_through_its_keepers_wall_and_a_child_is_taken() {
    let (mut w, k, entry, target, child, keeper, _) = society_by_the_wall(true);
    let day0 = w.meta.tick / TICKS_PER_DAY + 1;
    // Some evening they come to terms with the deep's people.
    let mut day = day0;
    while w.societies[0].aim.is_none() {
        assert!(day < day0 + 120, "no bargain in 120 evenings");
        w.meta.tick = at(day, 20);
        hf_sim::societies::step(&mut w);
        day += 1;
    }
    assert!(w
        .events
        .pending
        .iter()
        .any(|e| matches!(e.kind, EventKind::DeepPact { against, .. } if against == target)));
    assert_eq!(w.societies[0].aim.as_ref().unwrap().child, child);
    // At midnight the keeper, one of them, goes to the wall and cuts it out.
    w.events.pending.clear();
    w.meta.tick = at(day, 0);
    hf_sim::societies::step(&mut w);
    assert_eq!(w.societies[0].aim.as_ref().unwrap().opener, Some(keeper));
    let mut taken = false;
    for _ in 0..2 * TICKS_PER_HOUR {
        w.meta.tick += 1;
        hf_sim::systems::nav::step(&mut w);
        hf_sim::societies::step(&mut w);
        hf_sim::systems::movement::step(&mut w);
        if w.events
            .pending
            .iter()
            .any(|e| matches!(e.kind, EventKind::TakenBelow { agent, .. } if agent == child))
        {
            taken = true;
            break;
        }
    }
    assert!(!w.grid.caverns[k].sealed, "the way is open");
    assert!(w.events.pending.iter().any(
        |e| matches!(e.kind, EventKind::LetIn { by, against, .. } if by == keeper && against == target)
    ));
    assert!(taken, "the child is carried down");
    assert!(!w.agents.iter().any(|a| a.id == child && a.alive));
    assert!(w.overworld.away.iter().any(|a| a.id == child));
    assert!(w.societies[0].struck);
    // In the morning the way is found open and the child missed.
    w.meta.tick = at(day, 6);
    hf_sim::societies::step(&mut w);
    assert!(w
        .events
        .pending
        .iter()
        .any(|e| matches!(e.kind, EventKind::GateFoundOpen { child: Some(c), .. } if c == child)));
    assert!(
        w.designations.iter().any(
            |d| d.pos == entry && matches!(d.kind, hf_sim::work::DesignationKind::BuildWall(_))
        ),
        "the valley sets about walling it again"
    );
    let ti = w.agents.iter().position(|a| a.id == target).unwrap();
    assert!(w.agents[ti]
        .feelings
        .list
        .iter()
        .any(|f| f.kind == hf_sim::mind::feelings::FeelingKind::Grief && f.about == Some(child)));
    assert!(
        w.takings[0]
            .takers
            .iter()
            .all(|id| !w.agents.iter().any(|a| a.id == *id && a.alive)),
        "the deep's people are gone down again"
    );
}

/// Runs a society's bargain and night through to the morning after; returns the day.
fn through_the_night(w: &mut World) -> u64 {
    let day0 = w.meta.tick / TICKS_PER_DAY + 1;
    let mut day = day0;
    while w.societies[0].aim.is_none() {
        assert!(day < day0 + 120, "no bargain in 120 evenings");
        w.meta.tick = at(day, 20);
        hf_sim::societies::step(w);
        day += 1;
    }
    w.meta.tick = at(day, 0);
    hf_sim::societies::step(w);
    for _ in 0..2 * TICKS_PER_HOUR {
        w.meta.tick += 1;
        hf_sim::systems::nav::step(w);
        hf_sim::societies::step(w);
        hf_sim::systems::movement::step(w);
    }
    w.events.pending.clear();
    w.meta.tick = at(day, 6);
    hf_sim::societies::step(w);
    day
}

fn judged(w: &mut World, day: u64) {
    w.meta.tick = day * TICKS_PER_DAY + 18 * TICKS_PER_HOUR + 300;
    hf_sim::justice::step(w);
}

#[test]
fn the_keeper_who_opened_it_is_named_exiled_and_the_society_comes_out() {
    let (mut w, _k, _entry, target, _child, keeper, _) = society_by_the_wall(true);
    let day = through_the_night(&mut w);
    // Nobody saw: the mother lays it on the keeper of the way, rightly.
    assert!(w.events.pending.iter().any(|e| matches!(
        &e.kind,
        EventKind::Accused { by, of, what } if *by == target && *of == keeper
            && what == hf_sim::justice::DEEP_GATE
    )));
    let members = w.societies[0].members.clone();
    w.events.pending.clear();
    judged(&mut w, day);
    assert!(w.events.pending.iter().any(|e| matches!(
        &e.kind,
        EventKind::Judged { of, verdict, wrongly: false, .. } if *of == keeper && verdict == "exiled"
    )));
    assert!(w
        .events
        .pending
        .iter()
        .any(|e| matches!(e.kind, EventKind::SocietyExposed { against, .. } if against == target)));
    assert!(w.societies.is_empty(), "the society is done");
    let ti = w.agents.iter().position(|a| a.id == target).unwrap();
    for m in members.iter().filter(|m| **m != keeper) {
        assert!(
            w.agents[ti].opinion_of(*m) < 0.0,
            "the valley turns from them"
        );
    }
}

#[test]
fn when_another_opened_it_the_keeper_is_blamed_and_exiled_wrongly() {
    let (mut w, _k, _entry, target, _child, keeper, opener) = society_by_the_wall(false);
    assert_ne!(keeper, opener);
    let day = through_the_night(&mut w);
    assert!(
        w.events
            .pending
            .iter()
            .any(|e| matches!(e.kind, EventKind::LetIn { by, .. } if by == opener))
            || w.takings.iter().any(|t| t.opener == opener)
    );
    assert!(w.events.pending.iter().any(|e| matches!(
        &e.kind,
        EventKind::Accused { by, of, .. } if *by == target && *of == keeper
    )));
    w.events.pending.clear();
    judged(&mut w, day);
    assert!(w.events.pending.iter().any(|e| matches!(
        &e.kind,
        EventKind::Judged { of, verdict, wrongly: true, .. } if *of == keeper && verdict == "exiled"
    )));
    assert!(
        !w.events
            .pending
            .iter()
            .any(|e| matches!(e.kind, EventKind::SocietyExposed { .. })),
        "the society stays hidden"
    );
    let oi = w.agents.iter().position(|a| a.id == opener).unwrap();
    assert!(w.agents[oi].alive && w.agents[oi].community.is_some());
}

// ---- Going after the child (#322, 2026-10-26 fix-322-search) ----

use hf_sim::taken::{Outcome, Stage};

/// The society's night through to the morning, with a friend of the mother's who is not of the
/// society: the party sets out. Returns (world, mother, child, friend).
fn party_set_out() -> (World, AgentId, AgentId, AgentId) {
    let (mut w, k, _entry, target, child, _keeper, _) = society_by_the_wall(true);
    let cid = w.grid.caverns[k].gate_of.unwrap();
    let members = w.societies[0].members.clone();
    let now = w.meta.tick;
    let friend = w
        .agents
        .iter()
        .find(|a| {
            a.alive
                && a.community == Some(cid)
                && !a.is_child(now)
                && a.id != target
                && !members.contains(&a.id)
        })
        .map(|a| a.id)
        .unwrap();
    let fi = w.agents.iter().position(|a| a.id == friend).unwrap();
    w.agents[fi].relate(target, 0.8, 0.5, now);
    through_the_night(&mut w);
    (w, target, child, friend)
}

/// Everyone in the party at the gathering place, and the party goes down.
fn go_down(w: &mut World) {
    let stand = w.gone_below[0].stand;
    for id in w.gone_below[0].party.clone() {
        let i = w.agents.iter().position(|a| a.id == id).unwrap();
        w.agents[i].pos = stand;
    }
    w.meta.tick = (w.meta.tick / 200 + 1) * 200;
    hf_sim::taken::step(w);
}

#[test]
fn the_mother_and_a_friend_go_down_after_the_taken_child() {
    let (mut w, target, child, friend) = party_set_out();
    assert_eq!(w.gone_below.len(), 1);
    let rec = w.gone_below[0].clone();
    assert_eq!(rec.child, child);
    assert_eq!(rec.stage, Stage::Gathering);
    assert!(rec.party.contains(&target), "the mother goes");
    assert!(rec.party.contains(&friend), "and one close to her");
    for m in &w.societies[0].members {
        assert!(!rec.party.contains(m), "none of the society go");
    }
    go_down(&mut w);
    assert_eq!(w.gone_below[0].stage, Stage::Down);
    assert!(w.events.pending.iter().any(
        |e| matches!(&e.kind, EventKind::WentDownAfter { agents, child: c, .. } if *c == child && agents.contains(&target))
    ));
    assert!(w.overworld.away.iter().any(|a| a.id == target));
    // On the day they are due, they come up, as the roll says.
    let expect = hf_sim::taken::roll(&w, 0);
    let due = w.gone_below[0].back_at;
    w.meta.tick = due.div_ceil(200) * 200;
    hf_sim::taken::step(&mut w);
    let word = expect.word();
    assert!(w
        .events
        .pending
        .iter()
        .any(|e| matches!(&e.kind, EventKind::CameUpFromBelow { outcome, .. } if outcome == word)));
    assert!(w.agents.iter().any(|a| a.id == target));
}

#[test]
fn brought_back_the_child_is_home_and_afraid_of_the_dark() {
    let (mut w, target, child, _) = party_set_out();
    go_down(&mut w);
    hf_sim::taken::settle(&mut w, 0, Outcome::BroughtBack);
    let ci = w
        .agents
        .iter()
        .position(|a| a.id == child)
        .expect("the child is back");
    assert!(!w.overworld.away.iter().any(|a| a.id == child));
    let c = &w.agents[ci];
    assert!(c.traits.has(hf_sim::mind::Trait::FearsTheDark));
    assert!(c.thoughts.has(hf_sim::mind::ThoughtKind::BroughtBack));
    let ti = w.agents.iter().position(|a| a.id == target).unwrap();
    let f = &w.agents[ti].feelings.list;
    use hf_sim::mind::feelings::FeelingKind;
    assert!(f
        .iter()
        .any(|f| f.kind == FeelingKind::Joy && f.about == Some(child)));
    assert!(!f
        .iter()
        .any(|f| f.kind == FeelingKind::Grief && f.about == Some(child)));
    assert_eq!(w.gone_below[0].stage, Stage::Done);
}

#[test]
fn a_party_can_lose_one_below_or_come_up_hurt_or_with_nothing() {
    let (mut base, target, child, friend) = party_set_out();
    go_down(&mut base);
    // Lost: the last to join does not come back.
    let mut w = base.clone();
    hf_sim::taken::settle(&mut w, 0, Outcome::Lost);
    let fa = w.agents.iter().find(|a| a.id == friend).unwrap();
    assert!(!fa.alive, "the friend is lost in the deep");
    assert!(w.agents.iter().any(|a| a.id == target && a.alive));
    assert!(w.events.pending.iter().any(
        |e| matches!(&e.kind, EventKind::CameUpFromBelow { lost: Some(l), outcome, .. } if *l == friend && outcome == "lost")
    ));
    assert!(
        w.overworld.away.iter().any(|a| a.id == child),
        "still below"
    );
    assert_eq!(w.gone_below[0].stage, Stage::Below);
    // Hurt.
    let mut w = base.clone();
    hf_sim::taken::settle(&mut w, 0, Outcome::Hurt);
    let now = w.meta.tick;
    assert!(w
        .agents
        .iter()
        .any(|a| a.id == target && a.alive && a.injured(now)));
    assert_eq!(w.gone_below[0].stage, Stage::Below);
    // Nothing found.
    let mut w = base.clone();
    hf_sim::taken::settle(&mut w, 0, Outcome::NothingFound);
    assert!(w.agents.iter().any(|a| a.id == target && a.alive));
    assert!(w.agents.iter().any(|a| a.id == friend && a.alive));
    assert!(w.overworld.away.iter().any(|a| a.id == child));
    assert_eq!(w.gone_below[0].stage, Stage::Below);
}

#[test]
fn a_child_not_gone_after_turns_up_grown_among_the_deep_people() {
    let (mut w, _k, _entry, target, child, _keeper, _) = society_by_the_wall(true);
    // The mother is hurt the night it happens, and nobody else is her own: nobody goes.
    let ti = w.agents.iter().position(|a| a.id == target).unwrap();
    w.agents[ti].injured_until = u64::MAX;
    through_the_night(&mut w);
    assert_eq!(w.gone_below.len(), 1);
    assert_eq!(w.gone_below[0].stage, Stage::Below);
    assert!(w.gone_below[0].party.is_empty());
    let site = w.gone_below[0].site.clone();
    // Still a child: they are not seen.
    let at = w.gone_below[0].stand;
    hf_sim::taken::among_them(&mut w, &site, at);
    assert_eq!(w.gone_below[0].stage, Stage::Below);
    // Years on, the deep's people come up, and the child is among them, grown.
    w.meta.tick += 15 * hf_sim::time::TICKS_PER_YEAR;
    w.events.pending.clear();
    hf_sim::taken::among_them(&mut w, &site, at);
    assert!(w.events.pending.iter().any(
        |e| matches!(&e.kind, EventKind::GrownBelow { agent, years, .. } if *agent == child && *years >= 15)
    ));
    assert_eq!(w.gone_below[0].stage, Stage::Done);
    let ti = w.agents.iter().position(|a| a.id == target).unwrap();
    assert!(w.agents[ti].feelings.list.iter().any(|f| {
        f.kind == hf_sim::mind::feelings::FeelingKind::Grief && f.about == Some(child)
    }));
}

// ---- The grown child comes home (#322, 2026-10-28 round-17) ----

/// A child taken with nobody going after it (the mother hurt), the keeper wrongly accused at
/// dawn, fifteen years on: (world, mother, child, keeper, opener).
fn grown_below() -> (World, AgentId, AgentId, AgentId, AgentId) {
    let (mut w, _k, _entry, target, child, keeper, opener) = society_by_the_wall(false);
    let ti = w.agents.iter().position(|a| a.id == target).unwrap();
    w.agents[ti].injured_until = u64::MAX;
    through_the_night(&mut w);
    assert_eq!(w.gone_below[0].stage, Stage::Below);
    assert_eq!(w.gone_below[0].opener, Some(opener));
    assert!(
        w.gone_below[0].accused.contains(&keeper),
        "the keeper blamed"
    );
    w.meta.tick += 15 * hf_sim::time::TICKS_PER_YEAR;
    w.agents[ti].injured_until = 0;
    w.events.pending.clear();
    (w, target, child, keeper, opener)
}

#[test]
fn grown_below_they_come_up_to_seek_their_family_and_are_taken_back_naming_the_opener() {
    use hf_sim::mind::feelings::FeelingKind;
    let (mut w, mother, child, keeper, opener) = grown_below();
    let cid = w.gone_below[0].community;
    // In time, of their own accord, they come up.
    let day0 = w.meta.tick / TICKS_PER_DAY + 1;
    let mut day = day0;
    while w.gone_below[0].stage == Stage::Below {
        assert!(day < day0 + 600, "nobody came up in 600 days");
        w.meta.tick = day * TICKS_PER_DAY + 9 * TICKS_PER_HOUR;
        hf_sim::taken::step(&mut w);
        day += 1;
    }
    assert_eq!(w.gone_below[0].stage, Stage::Seeking);
    assert!(w.events.pending.iter().any(
        |e| matches!(&e.kind, EventKind::SoughtKin { agent, years, .. } if *agent == child && *years >= 15)
    ));
    let ci = w.agents.iter().position(|a| a.id == child).unwrap();
    assert_eq!(w.agents[ci].community, None, "a stranger until known");
    // This test valley's way down has no path up to the houses, so the seeker stands at it;
    // the meeting is set up by hand below.
    let mi = w.agents.iter().position(|a| a.id == mother).unwrap();
    let grief = |w: &World| {
        w.agents[mi]
            .feelings
            .list
            .iter()
            .find(|f| f.kind == FeelingKind::Grief && f.about == Some(child))
            .map(|f| f.strength)
    };
    assert!(grief(&w).is_some(), "the mother's grief stirred");
    // Met, and known, and taken back: they name the one who opened the way.
    let mut base = w.clone();
    let before = w.agents[mi].opinion_of(keeper);
    hf_sim::taken::meet(&mut w, 0, Some(mother), hf_sim::taken::Meeting::TakenBack);
    let ci = w.agents.iter().position(|a| a.id == child).unwrap();
    assert_eq!(w.agents[ci].community, Some(cid));
    assert!(!w.agents[ci].leaving);
    assert_eq!(w.gone_below[0].stage, Stage::Done);
    assert!(grief(&w).is_none());
    assert!(w.events.pending.iter().any(|e| matches!(
        &e.kind,
        EventKind::Accused { by, of, .. } if *by == mother && *of == opener
    )));
    assert!(w.agents[mi]
        .suspects
        .iter()
        .any(|x| x.0 == opener && x.1 >= 0.9));
    assert!(
        w.agents[mi].opinion_of(keeper) > before,
        "the keeper was wrongly blamed"
    );
    // Known but turning back: the grief deepens, and they walk back down.
    hf_sim::taken::meet(
        &mut base,
        0,
        Some(mother),
        hf_sim::taken::Meeting::KnownTurnedBack,
    );
    assert!(base.gone_below[0].going_back);
    let stand = base.gone_below[0].stand;
    let ci = base.agents.iter().position(|a| a.id == child).unwrap();
    base.agents[ci].pos = stand;
    base.meta.tick += 1;
    hf_sim::taken::step(&mut base);
    assert_eq!(base.gone_below[0].stage, Stage::Done);
    assert!(base.overworld.away.iter().any(|a| a.id == child));
    assert!(!base.agents.iter().any(|a| a.id == child));
}

#[test]
fn the_seeker_meets_their_mother_when_they_reach_her() {
    let (mut w, mother, child, _keeper, _opener) = grown_below();
    hf_sim::taken::come_up(&mut w, 0);
    w.events.pending.clear();
    let mi = w.agents.iter().position(|a| a.id == mother).unwrap();
    let ci = w.agents.iter().position(|a| a.id == child).unwrap();
    let mp = w.agents[mi].pos;
    w.agents[ci].pos = mp;
    w.meta.tick += 1;
    hf_sim::taken::step(&mut w);
    let met = w.events.pending.iter().find_map(|e| match &e.kind {
        EventKind::KinMet { agent, by, outcome } if *agent == child => Some((*by, outcome.clone())),
        _ => None,
    });
    let (by, outcome) = met.expect("they met");
    assert_eq!(by, Some(mother));
    // However it went, it went as the meeting said, and nobody waits on.
    if outcome == "taken back" {
        let ci = w.agents.iter().position(|a| a.id == child).unwrap();
        assert!(w.agents[ci].community.is_some());
        assert_eq!(w.gone_below[0].stage, Stage::Done);
    } else {
        assert!(outcome.contains("turned back below"), "{outcome}");
        assert!(w.gone_below[0].going_back);
    }
}

// ---- Talk of who did it (#322, 2026-10-26 fix-322-rumour) ----

#[test]
fn talk_of_who_opened_the_way_goes_round_and_another_accuses_them() {
    let (mut w, k, entry, target, _child, keeper, opener) = society_by_the_wall(false);
    let cid = w.grid.caverns[k].gate_of.unwrap();
    let members = w.societies[0].members.clone();
    let now = w.meta.tick;
    let free: Vec<AgentId> = w
        .agents
        .iter()
        .filter(|a| {
            a.alive
                && a.community == Some(cid)
                && !a.is_child(now)
                && ![target, keeper, opener].contains(&a.id)
                && !members.contains(&a.id)
        })
        .map(|a| a.id)
        .collect();
    let (g, h) = (free[0], free[1]);
    // The gallery runs on past the wall; G is awake at its far end that night, twelve tiles
    // off: near enough to glimpse someone out by the way, not to be sure who.
    for n in 6..14 {
        let p = entry.offset(n, 0, 0);
        w.grid.set(p, Tile::AIR);
        w.grid
            .set(p.offset(0, 0, -1), Tile::solid(Material::BASALT));
    }
    let gi = w.agents.iter().position(|a| a.id == g).unwrap();
    w.agents[gi].pos = entry.offset(12, 0, 0);
    w.agents[gi].activity = hf_sim::mind::Activity::Idle;
    let day = through_the_night(&mut w);
    let gi = w.agents.iter().position(|a| a.id == g).unwrap();
    assert!(
        w.agents[gi]
            .suspects
            .iter()
            .any(|(of, s)| *of == opener && *s >= 0.5),
        "G glimpsed the opener out by the way"
    );
    // The mother lays it on the keeper, wrongly, and the keeper is exiled that evening.
    judged(&mut w, day);
    assert!(w.events.pending.iter().any(|e| matches!(
        &e.kind,
        EventKind::Judged { of, wrongly: true, .. } if *of == keeper
    )));
    // H thinks well of G; they talk, three times.
    let hi = w.agents.iter().position(|a| a.id == h).unwrap();
    let t = w.meta.tick;
    w.agents[hi].relate(g, 0.8, 0.5, t);
    let before = w.agents[hi].opinion_of(opener);
    for _ in 0..3 {
        w.last_events = vec![hf_sim::events::Event {
            id: hf_sim::ids::EventId(0),
            tick: w.meta.tick,
            at: None,
            severity: hf_sim::events::Severity::Minor,
            kind: EventKind::Talked {
                a: g,
                b: h,
                warmth: 0.5,
            },
        }];
        hf_sim::social::observe(&mut w);
    }
    let hi = w.agents.iter().position(|a| a.id == h).unwrap();
    assert!(
        w.agents[hi].opinion_of(opener) < before,
        "what H heard colours what H thinks of the opener"
    );
    // The next evening H, not a parent, accuses the opener, and it is judged rightly.
    w.events.pending.clear();
    w.meta.tick = at(day + 1, 17);
    hf_sim::societies::step(&mut w);
    assert!(w.events.pending.iter().any(|e| matches!(
        &e.kind,
        EventKind::Accused { by, of, .. } if *of == opener && *by != target
    )));
    judged(&mut w, day + 1);
    assert!(w.events.pending.iter().any(|e| matches!(
        &e.kind,
        EventKind::Judged { of, wrongly: false, verdict, .. } if *of == opener && verdict == "exiled"
    )));
    assert!(w
        .events
        .pending
        .iter()
        .any(|e| matches!(e.kind, EventKind::SocietyExposed { against, .. } if against == target)));
}

#[test]
fn a_member_does_not_take_up_talk_against_another() {
    let (mut w, _k, _entry, _target, _child, _keeper, opener) = society_by_the_wall(false);
    through_the_night(&mut w);
    let other = *w.takings[0].members.iter().find(|m| **m != opener).unwrap();
    let teller = w
        .agents
        .iter()
        .find(|a| a.alive && a.id != opener && !w.takings[0].members.contains(&a.id))
        .map(|a| a.id)
        .unwrap();
    hf_sim::societies::suspect(&mut w, teller, opener, 1.0);
    w.last_events = vec![hf_sim::events::Event {
        id: hf_sim::ids::EventId(0),
        tick: w.meta.tick,
        at: None,
        severity: hf_sim::events::Severity::Minor,
        kind: EventKind::Talked {
            a: teller,
            b: other,
            warmth: 0.5,
        },
    }];
    hf_sim::social::observe(&mut w);
    let oi = w.agents.iter().position(|a| a.id == other).unwrap();
    assert!(!w.agents[oi].suspects.iter().any(|(of, _)| *of == opener));
}

// ---- The whole story in the chronicle (#322, 2026-10-26 fix-322-story) ----

#[test]
fn the_chronicle_reads_the_taken_childs_story_in_order() {
    // Run as through_the_night does, but keep every line for the chronicle.
    let (mut w, _k, _entry, _target, child, _keeper, _) = society_by_the_wall(true);
    let mut c = hf_sim::chronicle::Chronicle::in_memory().unwrap();
    let day0 = w.meta.tick / TICKS_PER_DAY + 1;
    let mut day = day0;
    while w.societies[0].aim.is_none() {
        w.meta.tick = at(day, 20);
        hf_sim::societies::step(&mut w);
        day += 1;
    }
    w.meta.tick = at(day, 0);
    hf_sim::societies::step(&mut w);
    for _ in 0..2 * TICKS_PER_HOUR {
        w.meta.tick += 1;
        hf_sim::systems::nav::step(&mut w);
        hf_sim::societies::step(&mut w);
        hf_sim::systems::movement::step(&mut w);
    }
    w.meta.tick = at(day, 6);
    hf_sim::societies::step(&mut w);
    go_down(&mut w);
    w.meta.tick += TICKS_PER_DAY;
    hf_sim::taken::settle(&mut w, 0, Outcome::BroughtBack);
    let evs = w.events.drain();
    c.record(&evs).unwrap();
    let story = hf_sim::view::history::story_of(&w, &c, child).unwrap();
    let find = |needle: &str| {
        story
            .iter()
            .position(|l| l.contains(needle))
            .unwrap_or_else(|| panic!("no line with {needle:?} in {story:#?}"))
    };
    let pact = find("have had word with the people of");
    let night = find("up out of the deep");
    let taken = find("was taken in the night");
    let found = find("stood open");
    let down = find("went down into the deep after");
    let back = find("brought back from");
    assert!(pact < night && night < taken && taken < found && found < down && down < back);
}
