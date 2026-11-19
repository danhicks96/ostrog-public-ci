//! #508 (epic #508, WP-07 7.5): witnesses of a manifestation form their own account of what
//! they saw — bounded by observation, never naming the hidden potential — teach it in talk
//! or withhold it, and turn what they saw into feeling, belief and stance toward the carrier
//! (mind/witnessed.rs).

use hf_sim::events::EventKind;
use hf_sim::ids::TilePos;
use hf_sim::mind::beliefs::Topic;
use hf_sim::mind::dispositions::Dispositions;
use hf_sim::mind::feelings::FeelingKind;
use hf_sim::mind::perception::witnesses;
use hf_sim::mind::techniques::{Learned, Technique, Techniques};
use hf_sim::mind::witnessed::{Saw, WitnessRead, WITNESSED_CAP};
use hf_sim::resonance::{self, Potential, Resonance};
use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use hf_sim::world::gen::GenParams;
use hf_sim::world::{Material, Tile};
use hf_sim::World;

/// A flat meadow with two people standing by the face at (42, 30, -2): `carrier` (woken,
/// rested) and `witness`. Returns their indices.
fn pair(seed: u64) -> (World, usize, usize) {
    let mut w = World::generate(seed, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.plants.clear();
    w.meta.tick = 7 * hf_sim::time::TICKS_PER_DAY + hf_sim::time::TICKS_PER_HOUR * 8;
    let ids = w.found_party(2);
    let c = w.agents.iter().position(|a| a.id == ids[0]).unwrap();
    let n = w.agents.iter().position(|a| a.id == ids[1]).unwrap();
    w.agents[c].pos = TilePos::new(42, 30, -2);
    w.agents[n].pos = TilePos::new(43, 30, -2);
    w.agents[c].resonance = Some(Resonance {
        potential: Potential::RockSense,
        sat: resonance::WAKES_AFTER,
        woke: Some(1),
        refused: 0,
        first_manifestation: None,
    });
    w.agents[c].needs.rest = 1.0;
    w.agents[c].techniques = Techniques::default();
    (w, c, n)
}

/// The unheld span one column over from the pair.
const UNHELD: &[TilePos] = &[TilePos::new(44, 30, -1)];

#[test]
fn witnesses_of_a_refusal_hold_their_own_read_not_the_potential() {
    let (mut w, c, n) = pair(1);
    // Pinned low: this test is about reading and telling, not about #1834's caution gate.
    w.agents[n].dispositions = Some(Dispositions {
        caution: 0.5,
        ..Dispositions::default()
    });
    // The witness reads it through thin-place lore.
    let tick = w.tick();
    w.agents[n]
        .techniques
        .learn(Technique::ThinLore, Learned::Past, 1.0, tick);
    assert!(resonance::balks(
        &mut w,
        c,
        TilePos::new(44, 30, -2),
        UNHELD
    ));
    let acc = &w.agents[n].witnessed;
    assert_eq!(acc.len(), 1, "one account, formed where they stood");
    assert_eq!(acc[0].who, w.agents[c].id);
    assert_eq!(acc[0].saw, Saw::RefusedTheCut);
    assert_eq!(
        acc[0].read,
        WitnessRead::StoneRemembers,
        "their lore, not the truth"
    );
    assert_eq!(acc[0].at, TilePos::new(44, 30, -2));
    assert!(acc[0].tells, "a stranger neither shields nor shakes");
    // The record is what they saw: no hidden state is anywhere in it.
    let debug = format!("{:?}", acc[0]);
    assert!(!debug.contains("RockSense"), "{debug}");
    assert!(!debug.contains("resonance"), "{debug}");
}

#[test]
fn a_witness_who_saw_a_failure_fears_the_carrier() {
    let (mut w, c, n) = pair(2);
    assert!(resonance::balks(
        &mut w,
        c,
        TilePos::new(44, 30, -2),
        UNHELD
    ));
    let fear = w.agents[n].feelings.of(FeelingKind::Fear);
    assert!(fear > 0.0, "they saw the dread and hold it: {fear}");
    let about_carrier = w.agents[n]
        .feelings
        .list
        .iter()
        .any(|f| f.kind == FeelingKind::Fear && f.about == Some(w.agents[c].id));
    assert!(about_carrier, "the fear is about the carrier");
    // Fear rationalises the moment as only weather, and cools their opinion.
    assert!(w.agents[n].beliefs.lean(Topic::DeitySigns) < 0.0);
    assert!(w.agents[n].opinion_of(w.agents[c].id) < 0.0);
}

#[test]
fn a_witness_may_teach_what_they_saw_in_talk() {
    let (mut w, c, n) = pair(3);
    assert!(resonance::balks(
        &mut w,
        c,
        TilePos::new(44, 30, -2),
        UNHELD
    ));
    // A third person, out of sight of the face, hears it in talk.
    let ids = w.found_party(1);
    let l = w.agents.iter().position(|a| a.id == ids[0]).unwrap();
    w.agents[l].pos = TilePos::new(10, 10, 1);
    assert!(w.agents[l].witnessed.is_empty());
    let tick = w.tick();
    hf_sim::systems::beliefs::share(&mut w, n, l, tick);
    let heard = &w.agents[l].witnessed;
    assert_eq!(heard.len(), 1, "the account passed on");
    assert_eq!(
        heard[0].who, w.agents[c].id,
        "the carrier's identity travels"
    );
    assert_eq!(heard[0].saw, Saw::RefusedTheCut);
    assert_eq!(
        heard[0].read,
        WitnessRead::UnexplainedDread,
        "a stranger's read is the plain dread"
    );
    assert!(heard[0].tells, "the listener may pass it on");
    // The carrier's own account is untouched by telling it.
    assert_eq!(w.agents[n].witnessed.len(), 1);
}

#[test]
fn a_withheld_account_stays_out_of_the_gossip_chain() {
    let (mut w, c, n) = pair(4);
    // A protective witness: they think well of the carrier and will not speak of it.
    let carrier = w.agents[c].id;
    let tick = w.tick();
    w.agents[n].relate(carrier, 0.8, 0.5, tick);
    assert!(resonance::balks(
        &mut w,
        c,
        TilePos::new(44, 30, -2),
        UNHELD
    ));
    assert_eq!(w.agents[n].witnessed.len(), 1);
    assert!(
        !w.agents[n].witnessed[0].tells,
        "they hold it for the carrier's sake"
    );
    let ids = w.found_party(1);
    let l = w.agents.iter().position(|a| a.id == ids[0]).unwrap();
    w.agents[l].pos = TilePos::new(10, 10, 1);
    let tick = w.tick();
    hf_sim::systems::beliefs::share(&mut w, n, l, tick);
    assert!(
        w.agents[l].witnessed.is_empty(),
        "a withheld account does not travel"
    );
    // A shaken witness (cautious by disposition) also stays silent.
    let (mut w2, c2, n2) = pair(5);
    w2.agents[n2].dispositions = Some(Dispositions {
        caution: 0.9,
        ..Dispositions::default()
    });
    assert!(resonance::balks(
        &mut w2,
        c2,
        TilePos::new(44, 30, -2),
        UNHELD
    ));
    assert!(
        !w2.agents[n2].witnessed[0].tells,
        "too cautious to speak of it"
    );
}

#[test]
fn a_bold_witness_tells_a_cautious_one_stays_silent() {
    // #1834: `form` read caution backwards, so the calmest witnesses (low caution) were the
    // ones silenced. The cautious (high caution) should be the ones who keep it to themselves.
    let (mut w, c, bold) = pair(7);
    w.agents[bold].dispositions = Some(Dispositions {
        caution: 0.1,
        ..Dispositions::default()
    });
    assert!(resonance::balks(
        &mut w,
        c,
        TilePos::new(44, 30, -2),
        UNHELD
    ));
    assert!(w.agents[bold].witnessed[0].tells, "a bold witness tells");

    let (mut w2, c2, cautious) = pair(8);
    w2.agents[cautious].dispositions = Some(Dispositions {
        caution: 0.9,
        ..Dispositions::default()
    });
    assert!(resonance::balks(
        &mut w2,
        c2,
        TilePos::new(44, 30, -2),
        UNHELD
    ));
    assert!(
        !w2.agents[cautious].witnessed[0].tells,
        "a cautious witness stays silent"
    );
}

#[test]
fn non_witnesses_know_nothing() {
    let (mut w, c, n) = pair(6);
    // The witness is out of earshot and sight: across the map on another level.
    w.agents[n].pos = TilePos::new(10, 10, 1);
    assert!(resonance::balks(
        &mut w,
        c,
        TilePos::new(44, 30, -2),
        UNHELD
    ));
    assert!(
        w.agents[n].witnessed.is_empty(),
        "no account without the sighting"
    );
    assert_eq!(w.agents[n].feelings.of(FeelingKind::Fear), 0.0);
    assert_eq!(w.agents[n].feelings.of(FeelingKind::Awe), 0.0);
}

#[test]
fn the_waking_is_seen_by_those_at_the_stone_and_the_record_is_bounded() {
    let (mut w, c, n) = pair(7);
    // Unwoken: the fourth sitting by the stone wakes them.
    w.agents[c].resonance = Some(Resonance {
        potential: Potential::RockSense,
        sat: 0,
        woke: None,
        refused: 0,
        first_manifestation: None,
    });
    for _ in 0..resonance::WAKES_AFTER {
        resonance::sat_by_stone(&mut w, c);
        resonance::sat_by_stone(&mut w, n);
    }
    let acc = &w.agents[n].witnessed;
    assert_eq!(acc.len(), 1);
    assert_eq!(
        acc[0].saw,
        Saw::CameBackUnsettled,
        "the quiet waking is still a sighting"
    );
    assert_eq!(acc[0].read, WitnessRead::UnexplainedDread);
    // The witnessing is weaker than a refusal: felt, but less.
    assert!(w.agents[n].feelings.of(FeelingKind::Fear) > 0.0);
    // The record names no potential.
    let debug = format!("{:?}", acc[0]);
    assert!(!debug.contains("RockSense"), "{debug}");
}

#[test]
fn the_record_is_capped() {
    let (mut w, c, n) = pair(8);
    for _ in 0..WITNESSED_CAP + 3 {
        assert!(resonance::balks(
            &mut w,
            c,
            TilePos::new(44, 30, -2),
            UNHELD
        ));
        w.agents[c].needs.rest = 1.0;
    }
    assert!(w.agents[n].witnessed.len() <= WITNESSED_CAP);
}

/// Every event pushed for a witnessed beat is on the chronicle with its source intact.
#[test]
fn the_beat_stays_a_real_event() {
    let (mut w, c, _n) = pair(9);
    assert!(resonance::balks(
        &mut w,
        c,
        TilePos::new(44, 30, -2),
        UNHELD
    ));
    assert!(w
        .events
        .pending
        .iter()
        .any(|e| matches!(e.kind, EventKind::WouldNotCut { .. })));
}

/// #854: one helper decides who is there to see a thing; a wall hides, sleep hides.
#[test]
fn witnesses_behind_a_wall_are_not_counted() {
    let mut w = World::generate(854, GenParams::default());
    w.flatten(2, 2, 120, 120, 1);
    w.plants.clear();
    w.meta.founding_site = TilePos::new(60, 60, 2);
    w.meta.tick = TICKS_PER_DAY * 30 + TICKS_PER_HOUR * 10;
    w.rebuild_caches();
    w.found_party(3);
    let spot = w.grid.surface(60, 60);
    let place = |w: &mut World, i: usize, x: i32, y: i32| {
        w.agents[i].pos = w.grid.surface(x, y);
        w.agents[i].activity = hf_sim::mind::Activity::Idle;
    };
    place(&mut w, 0, 55, 60);
    place(&mut w, 1, 65, 60);
    place(&mut w, 2, 60, 55);
    for x in 63..=63 {
        for dz in 0..3 {
            w.grid.set(
                TilePos::new(x, 60, spot.z + dz),
                Tile::solid(Material::WOOD),
            );
        }
    }
    let seen = witnesses(&w, spot, 8, true);
    assert_eq!(seen, vec![0, 2], "the one behind the wall is not counted");
    w.agents[2].activity = hf_sim::mind::Activity::Sleeping {
        bed: None,
        until: u64::MAX,
    };
    assert_eq!(witnesses(&w, spot, 8, true), vec![0], "a sleeper never is");
}
