//! #452: a death becomes accounts that can differ and spread, and a grave a later person reads,
//! while the record of the death stays as it happened (docs/design-stories.md).

use hf_sim::accounts::{How, Said};
use hf_sim::events::EventKind;
use hf_sim::ids::{AgentId, TilePos};
use hf_sim::mind::Trait;
use hf_sim::save::{self, snapshot::Snapshot};
use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use hf_sim::world::gen::GenParams;
use hf_sim::World;

fn valley(seed: u64, n: usize) -> (World, Vec<AgentId>) {
    let mut w = World::generate(seed, GenParams::default());
    w.flatten(2, 2, 120, 120, 1);
    w.plants.clear();
    w.meta.founding_site = TilePos::new(60, 60, 2);
    w.meta.tick = TICKS_PER_DAY * 30 + TICKS_PER_HOUR * 10;
    w.rebuild_caches();
    let ids = w.found_party(n);
    w.ensure_nav();
    for a in w.agents.iter_mut() {
        a.traits.list.clear();
        a.beliefs = Default::default();
    }
    (w, ids)
}

fn idx(w: &World, id: AgentId) -> usize {
    w.agents.iter().position(|a| a.id == id).unwrap()
}

fn put(w: &mut World, id: AgentId, x: i32, y: i32) {
    let i = idx(w, id);
    let p = w.grid.surface(x, y);
    w.agents[i].pos = p;
    w.agents[i].activity = hf_sim::mind::Activity::Idle;
}

fn died_row(w: &World, id: AgentId) -> (hf_sim::ids::EventId, String) {
    w.events
        .pending
        .iter()
        .find_map(|e| match &e.kind {
            EventKind::Died { agent, cause } if *agent == id => Some((e.id, cause.clone())),
            _ => None,
        })
        .expect("the death is on record")
}

#[test]
fn those_who_saw_a_death_hold_it_true_and_nobody_else_holds_it() {
    let (mut w, ids) = valley(4521, 4);
    put(&mut w, ids[0], 60, 60);
    put(&mut w, ids[1], 63, 60);
    put(&mut w, ids[2], 95, 95);
    put(&mut w, ids[3], 62, 61);
    let child = idx(&w, ids[3]);
    w.agents[child].born = (w.meta.tick - TICKS_PER_DAY) as i64;
    hf_sim::systems::life::die(&mut w, ids[0], "fever".into());
    let (row, cause) = died_row(&w, ids[0]);
    assert_eq!(cause, "fever");
    let near = &w.agents[idx(&w, ids[1])].accounts;
    assert_eq!(near.len(), 1, "the one beside them saw it");
    assert_eq!(near[0].death, row);
    assert_eq!(near[0].said, Said::Sickness);
    assert_eq!(near[0].how, How::Saw);
    assert!(
        w.agents[idx(&w, ids[2])].accounts.is_empty(),
        "one across the valley did not see it and holds nothing"
    );
    assert!(
        w.agents[child].accounts.is_empty(),
        "a child keeps no account of it"
    );
}

/// Tell `t`'s account to `l` until it is told (talk passes it half the time), at later ticks.
fn tell_until(w: &mut World, t: AgentId, l: AgentId) {
    for _ in 0..64 {
        let (i, j) = (idx(w, t), idx(w, l));
        hf_sim::accounts::on_talk(w, i, j);
        if !w.agents[j].accounts.is_empty() {
            return;
        }
        w.meta.tick += 1;
    }
    panic!("never told");
}

#[test]
fn a_telling_keeps_its_teller_can_drift_and_never_changes_the_record() {
    let (mut w, ids) = valley(4522, 12);
    put(&mut w, ids[0], 60, 60);
    put(&mut w, ids[1], 62, 60);
    for (k, id) in ids.iter().enumerate().skip(2) {
        put(&mut w, *id, 100, 20 + k as i32 * 3);
    }
    hf_sim::systems::life::die(&mut w, ids[0], "fell".into());
    let (row, _) = died_row(&w, ids[0]);
    // The witness is cautious: they tell it darker, sometimes.
    let wi = idx(&w, ids[1]);
    w.agents[wi].traits.list = vec![Trait::Cautious];
    let mut versions = Vec::new();
    for l in &ids[2..] {
        tell_until(&mut w, ids[1], *l);
        let acc = w.agents[idx(&w, *l)].accounts[0].clone();
        assert_eq!(acc.death, row, "the telling points at the row");
        assert_eq!(acc.teller, Some(ids[1]), "and at who told it");
        assert_eq!(acc.how, How::Told);
        versions.push(acc.said);
    }
    assert!(versions.contains(&Said::Fell), "some were told it true");
    assert!(
        versions.contains(&Said::Killed),
        "and in some tellings the fall became a killing: {versions:?}"
    );
    // The listeners hold different versions; the record still says what happened.
    let (_, cause) = died_row(&w, ids[0]);
    assert_eq!(cause, "fell");
    assert_eq!(
        w.agents[idx(&w, ids[0])].death.as_deref(),
        Some("fell"),
        "the dead's own record is unchanged"
    );
    // The feed tells a changed telling as that teller's, not as what happened.
    let changed = w
        .events
        .pending
        .iter()
        .find(|e| matches!(e.kind, EventKind::AccountTold { changed: true, .. }))
        .expect("a changed telling is in the feed")
        .clone();
    let line = hf_sim::view::narrate::line(&w, &changed);
    assert!(line.starts_with("In "), "{line}");
    assert!(line.contains("'s telling to "), "{line}");
}

#[test]
fn only_one_who_holds_a_dark_account_is_uneasy_at_the_place() {
    let (mut w, ids) = valley(4523, 4);
    put(&mut w, ids[0], 60, 60);
    put(&mut w, ids[1], 62, 60);
    put(&mut w, ids[2], 100, 30);
    put(&mut w, ids[3], 100, 40);
    hf_sim::systems::life::die(&mut w, ids[0], "torn apart in the night".into());
    let at = w.agents[idx(&w, ids[0])].pos;
    // Make the witness's own account a taking, as if a fearful teller had told it them.
    let wi = idx(&w, ids[1]);
    w.agents[wi].accounts[0].said = Said::Taken;
    tell_until(&mut w, ids[1], ids[2]);
    let now = w.meta.tick;
    for id in [ids[2], ids[3]] {
        let i = idx(&w, id);
        w.agents[i].memories.consolidate(now, false, false);
    }
    let told = &w.agents[idx(&w, ids[2])];
    let other = &w.agents[idx(&w, ids[3])];
    assert_eq!(
        told.accounts[0].said,
        Said::Taken,
        "a taking stays a taking"
    );
    assert!(
        told.memories.aversion(at, now, false) > 0.0,
        "told that something took them there, they are uneasy there"
    );
    assert_eq!(
        other.memories.aversion(at, now, false),
        0.0,
        "one who was never told it is not"
    );
}

#[test]
fn a_grave_is_read_by_one_who_came_after_and_the_reading_can_be_wrong() {
    let (mut w, ids) = valley(4524, 6);
    put(&mut w, ids[0], 60, 60);
    put(&mut w, ids[1], 62, 60);
    for (k, id) in ids.iter().enumerate().skip(2) {
        put(&mut w, *id, 90, 20 + k as i32 * 3);
    }
    let cid = w.agents[idx(&w, ids[1])].community.unwrap();
    for a in w.agents.iter_mut() {
        a.rite = hf_sim::realm::culture::Rite::Bury;
    }
    hf_sim::systems::life::die(&mut w, ids[0], "fell".into());
    let (row, _) = died_row(&w, ids[0]);
    assert!(hf_sim::systems::rites::lay_to_rest(
        &mut w,
        ids[0],
        cid,
        Some(ids[1]),
        false
    ));
    assert_eq!(
        w.graves.len(),
        1,
        "buried where they lay, they have a grave"
    );
    let g = w.graves[0].clone();
    assert_eq!(g.death, row);
    assert!(w
        .structures
        .iter()
        .any(|s| s.pos == g.pos && s.kind == hf_sim::structures::StructureKind::Grave));
    // Two who came later and never knew them stand by it; one is cautious.
    w.meta.tick += TICKS_PER_DAY;
    for (k, id) in [ids[4], ids[5]].iter().enumerate() {
        let i = idx(&w, *id);
        w.agents[i].came = w.meta.tick - TICKS_PER_HOUR;
        w.agents[i].relationships.clear();
        w.agents[i].pos = g.pos.offset(k as i32 + 1, 0, 0);
        w.agents[i].pos = w.grid.surface(w.agents[i].pos.x, w.agents[i].pos.y);
        if k == 1 {
            w.agents[i].traits.list = vec![Trait::Cautious];
        }
    }
    // One who knew them stands there too, and reads nothing new.
    let knew = idx(&w, ids[2]);
    w.agents[knew].pos = w.grid.surface(g.pos.x, g.pos.y + 1);
    w.meta.tick -= w.meta.tick % TICKS_PER_HOUR;
    hf_sim::accounts::step(&mut w);
    let plain = &w.agents[idx(&w, ids[4])].accounts;
    let fearful = &w.agents[idx(&w, ids[5])].accounts;
    assert_eq!(plain.len(), 1);
    assert_eq!(plain[0].how, How::Grave);
    // #1743: the one who buried them marked the grave, so even a reader who never knew them
    // now learns the name and what the burier wrote them as known for.
    assert_eq!(plain[0].of, Some(ids[0]), "a marked grave names its dead");
    assert_eq!(plain[0].at, g.pos, "the reading names its evidence");
    assert_eq!(plain[0].death, g.death);
    let words = g.words.clone().expect("the burier marked the grave");
    assert_eq!(plain[0].said, Said::Marked(words.clone()));
    assert_eq!(
        fearful[0].said,
        Said::Marked(words),
        "a marked grave is read true, fearful or not"
    );
    assert!(w.agents[knew].accounts.is_empty());
    // #1743: the marked grave already told them who it was, so being told the same death
    // afterwards adds nothing more.
    for _ in 0..16 {
        let (i, j) = (idx(&w, ids[1]), idx(&w, ids[4]));
        hf_sim::accounts::on_talk(&mut w, i, j);
        w.meta.tick += 1;
    }
    let still = &w.agents[idx(&w, ids[4])].accounts;
    assert_eq!(
        still.len(),
        1,
        "already named by the grave, nothing more to be told"
    );
    // The grave's hover gives the record, then what is said and by whom.
    let note = hf_sim::accounts::note(&w, g.pos).unwrap();
    assert!(note.contains("The record: fell."), "{note}");
    assert!(note.contains("Said: "), "{note}");
}

/// #1743: the one who buries a person marks the grave with the dead's name and their
/// strongest skill, and a newcomer standing by it later reads that back; a grave with no
/// words (dug with no one to write it) still only lets them guess "someone".
#[test]
fn a_marked_grave_tells_a_newcomer_who_lies_there() {
    let (mut w, ids) = valley(4524, 6);
    put(&mut w, ids[0], 60, 60);
    put(&mut w, ids[1], 62, 60);
    for (k, id) in ids.iter().enumerate().skip(2) {
        put(&mut w, *id, 90, 20 + k as i32 * 3);
    }
    let cid = w.agents[idx(&w, ids[1])].community.unwrap();
    for a in w.agents.iter_mut() {
        a.rite = hf_sim::realm::culture::Rite::Bury;
    }
    // The dead held a trained skill: this is the "known for" stand-in (#1201 does not exist
    // yet) that the burier writes on the grave.
    let di = idx(&w, ids[0]);
    w.agents[di]
        .skills
        .practise(hf_sim::mind::skills::Skill::Hauling, 500.0);
    hf_sim::systems::life::die(&mut w, ids[0], "fell".into());
    assert!(hf_sim::systems::rites::lay_to_rest(
        &mut w,
        ids[0],
        cid,
        Some(ids[1]),
        false
    ));
    assert_eq!(w.graves.len(), 1);
    let g = w.graves[0].clone();
    let dead_name = w
        .agents
        .iter()
        .find(|a| a.id == ids[0])
        .unwrap()
        .name
        .clone();
    let words = g.words.clone().expect("the burier wrote on the grave");
    assert!(words.starts_with(&dead_name), "{words}");
    assert!(words.contains("carried the load"), "{words}");

    // A newcomer who never knew them stands by the marked grave and reads it.
    w.meta.tick += TICKS_PER_DAY;
    let ni = idx(&w, ids[4]);
    w.agents[ni].came = w.meta.tick - TICKS_PER_HOUR;
    w.agents[ni].relationships.clear();
    w.agents[ni].pos = g.pos.offset(1, 0, 0);
    w.agents[ni].pos = w.grid.surface(w.agents[ni].pos.x, w.agents[ni].pos.y);
    w.meta.tick -= w.meta.tick % TICKS_PER_HOUR;
    hf_sim::accounts::step(&mut w);
    let acc = &w.agents[idx(&w, ids[4])].accounts;
    assert_eq!(acc.len(), 1);
    assert_eq!(acc[0].of, Some(ids[0]), "the marked grave names its dead");
    assert_eq!(acc[0].said, Said::Marked(words));

    // A grave dug with no one to write it (buried in_absence) carries no words, and a reader
    // still can only say someone lies there.
    w.graves[0].words = None;
    w.agents[ni].accounts.clear();
    // OST-165: a grave already read stays read, so forget the reading to read it afresh.
    w.agents[ni].graves_read.clear();
    w.meta.tick += TICKS_PER_HOUR;
    hf_sim::accounts::step(&mut w);
    let acc = &w.agents[idx(&w, ids[4])].accounts;
    assert_eq!(acc.len(), 1, "the unmarked grave is still read");
    assert_eq!(acc[0].of, None, "no name without words on the grave");
    assert_eq!(acc[0].said, Said::Unknown);
}

#[test]
fn accounts_and_graves_survive_a_save() {
    let (mut w, ids) = valley(4525, 4);
    put(&mut w, ids[0], 60, 60);
    put(&mut w, ids[1], 62, 60);
    let cid = w.agents[idx(&w, ids[1])].community.unwrap();
    for a in w.agents.iter_mut() {
        a.rite = hf_sim::realm::culture::Rite::Bury;
    }
    hf_sim::systems::life::die(&mut w, ids[0], "froze".into());
    hf_sim::systems::rites::lay_to_rest(&mut w, ids[0], cid, Some(ids[1]), false);
    let bytes = save::encode(&Snapshot::capture(&w)).unwrap();
    let back = save::decode(&bytes).unwrap().restore();
    assert_eq!(back.graves, w.graves);
    assert_eq!(
        back.agents[idx(&back, ids[1])].accounts,
        w.agents[idx(&w, ids[1])].accounts
    );
    assert!(!back.graves.is_empty());
}

/// OST-165: the reading of a grave outlives the account of it, so forgetting what they
/// believe of the death does not send a person back to read the same stone every few weeks.
#[test]
fn a_grave_is_read_once_even_when_its_account_is_forgotten() {
    let (mut w, ids) = valley(4524, 5);
    put(&mut w, ids[0], 60, 60);
    put(&mut w, ids[1], 62, 60);
    for (k, id) in ids.iter().enumerate().skip(2) {
        put(&mut w, *id, 90, 20 + k as i32 * 3);
    }
    let cid = w.agents[idx(&w, ids[1])].community.unwrap();
    for a in w.agents.iter_mut() {
        a.rite = hf_sim::realm::culture::Rite::Bury;
    }
    hf_sim::systems::life::die(&mut w, ids[0], "fell".into());
    assert!(hf_sim::systems::rites::lay_to_rest(
        &mut w,
        ids[0],
        cid,
        Some(ids[1]),
        false
    ));
    let g = w.graves[0].clone();
    w.meta.tick += TICKS_PER_DAY;
    let r = idx(&w, ids[4]);
    w.agents[r].came = w.meta.tick - TICKS_PER_HOUR;
    w.agents[r].relationships.clear();
    let p = g.pos.offset(1, 0, 0);
    w.agents[r].pos = w.grid.surface(p.x, p.y);
    w.meta.tick -= w.meta.tick % TICKS_PER_HOUR;
    let reads = |w: &World| {
        w.events
            .pending
            .iter()
            .filter(|e| matches!(&e.kind, EventKind::GraveRead { agent, .. } if *agent == ids[4]))
            .count()
    };
    hf_sim::accounts::step(&mut w);
    assert_eq!(reads(&w), 1, "the newcomer reads the grave once");
    // Thirty told accounts of other deaths push the reading out of what they keep.
    w.agents[r].accounts.clear();
    for _ in 0..30 {
        w.agents[r].accounts.push(hf_sim::accounts::Account {
            death: hf_sim::ids::EventId(u32::MAX),
            of: None,
            at: g.pos,
            said: Said::Unknown,
            teller: None,
            how: How::Told,
            heard: w.meta.tick,
        });
    }
    w.agents[r].accounts.retain(|x| x.death != g.death);
    for _ in 0..10 {
        w.meta.tick += TICKS_PER_HOUR;
        hf_sim::accounts::step(&mut w);
    }
    assert_eq!(
        reads(&w),
        1,
        "standing there longer gives no second reading"
    );
}
