//! C70: crafts as personal knowledge (design rule 3, D39).

use hf_sim::events::EventKind;
use hf_sim::ids::TilePos;
use hf_sim::mind::techniques::{Learned, Technique};
use hf_sim::mind::Trait;
use hf_sim::structures::StructureKind;
use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use hf_sim::work::DesignationKind;
use hf_sim::world::gen::GenParams;
use hf_sim::world::Material;
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
    // Nobody arrives knowing a craft, whatever their skills gave them.
    for a in w.agents.iter_mut() {
        a.techniques
            .list
            .retain(|k| !hf_sim::mind::techniques::CRAFTS.contains(&k.technique));
    }
    w.ensure_nav();
    (w, cid)
}

fn members(w: &World, cid: hf_sim::ids::CommunityId) -> Vec<usize> {
    (0..w.agents.len())
        .filter(|&i| w.agents[i].alive && w.agents[i].community == Some(cid))
        .collect()
}

#[test]
fn every_bench_recipe_asks_a_craft_and_the_rest_ask_none() {
    use hf_sim::work::recipes as rc;
    assert_eq!(rc::craft_of(rc::PLANK), Some(Technique::Carpentry));
    assert_eq!(rc::craft_of(rc::DRESS_STONE), Some(Technique::Masonry));
    assert_eq!(rc::craft_of(rc::IRON_BAR), Some(Technique::Smelting));
    assert_eq!(rc::craft_of(rc::IRON_TOOL), Some(Technique::Smithing));
    assert_eq!(rc::craft_of(rc::CLOTH), Some(Technique::Weaving));
    assert_eq!(rc::craft_of(rc::ALE), Some(Technique::Brewing));
    assert_eq!(
        rc::craft_of(rc::STONE_TOOL),
        None,
        "a stone tool anyone can knap"
    );
    assert_eq!(rc::craft_of(rc::THATCH), None);
    assert_eq!(rc::craft_of(rc::WOOD_SPEAR), None);
}

#[test]
fn only_those_who_know_a_craft_take_its_order_and_the_best_hand_may_try_after_a_day() {
    let (mut w, cid) = meadow(7001, 4);
    let home = w.communities[0].home;
    let bench = home.offset(4, 0, 0);
    w.place_structure(StructureKind::CarpenterBench, bench, Material::WOOD, true);
    let d = hf_sim::systems::settlement::make(
        &mut w,
        DesignationKind::Craft(hf_sim::work::recipes::PLANK),
        bench,
        5,
    );
    w.designations.push(d.clone());
    let m = members(&w, cid);
    // Fresh order: nobody knows carpentry, nobody may try yet.
    for &i in &m {
        assert!(!hf_sim::systems::crafts::may_try(
            &w,
            &w.agents[i],
            Technique::Carpentry,
            &d
        ));
    }
    // A day on: the best hand (or a curious one) may try, the rest may not.
    w.meta.tick += TICKS_PER_DAY + 1;
    let best = *m
        .iter()
        .max_by(|&&x, &&y| {
            w.agents[x]
                .skills
                .level(hf_sim::mind::Skill::Crafting)
                .total_cmp(&w.agents[y].skills.level(hf_sim::mind::Skill::Crafting))
        })
        .unwrap();
    assert!(hf_sim::systems::crafts::may_try(
        &w,
        &w.agents[best],
        Technique::Carpentry,
        &d
    ));
    // Once someone of theirs knows it, nobody tries: they wait for the one who knows.
    let tick = w.meta.tick;
    w.agents[m[0]]
        .techniques
        .learn(Technique::Carpentry, Learned::Past, 0.5, tick);
    for &i in &m {
        if i != m[0] {
            assert!(!hf_sim::systems::crafts::may_try(
                &w,
                &w.agents[i],
                Technique::Carpentry,
                &d
            ));
        }
    }
}

#[test]
fn a_try_is_recorded_and_a_win_teaches_the_craft() {
    // Over seeds, a try wins sometimes and loses sometimes; each is a record line, a win
    // leaves the craft known "worked out by trying", a loss a two-day cooldown.
    let (mut wins, mut losses) = (0, 0);
    for seed in 7010..7030u64 {
        let (mut w, cid) = meadow(seed, 3);
        let i = members(&w, cid)[0];
        let won = hf_sim::systems::crafts::tried(&mut w, i, Technique::Carpentry, "1 logs");
        let evs = w.events.drain();
        let line = evs
            .iter()
            .find(|e| matches!(e.kind, EventKind::CraftTried { .. }))
            .map(|e| hf_sim::view::narrate::line(&w, e))
            .expect("the try is recorded");
        if won {
            wins += 1;
            assert!(w.agents[i].techniques.knows(Technique::Carpentry));
            let k = w.agents[i]
                .techniques
                .list
                .iter()
                .find(|k| k.technique == Technique::Carpentry)
                .unwrap();
            assert_eq!(k.learned, Learned::Tried);
            assert!(k.from.contains("by trying"), "{}", k.from);
            assert!(line.contains("worked out carpentry"), "{line}");
        } else {
            losses += 1;
            assert!(!w.agents[i].techniques.knows(Technique::Carpentry));
            assert!(line.contains("spoiled 1 logs"), "{line}");
        }
        let key = 0x7300_0000 | Technique::Carpentry as u32;
        assert!(w.agents[i]
            .work_cooldowns
            .get(&key)
            .is_some_and(|t| *t > w.meta.tick));
    }
    assert!(wins >= 2 && losses >= 2, "{wins} wins, {losses} losses");
}

fn at_lesson_time(w: &mut World) {
    let tick = w.meta.tick;
    let next = (tick / TICKS_PER_DAY + 1) * TICKS_PER_DAY + TICKS_PER_HOUR * 7 + 13;
    w.meta.tick = next;
}

#[test]
fn the_craft_goes_with_the_last_who_knew_it() {
    let (mut w, cid) = meadow(7002, 4);
    let m = members(&w, cid);
    let tick = w.meta.tick;
    w.agents[m[1]]
        .techniques
        .learn(Technique::Smelting, Learned::Past, 0.5, tick);
    at_lesson_time(&mut w);
    hf_sim::systems::crafts::step(&mut w);
    let c = w.communities.iter().find(|c| c.id == cid).unwrap();
    assert!(
        c.crafts
            .iter()
            .any(|(t, who)| *t == Technique::Smelting && *who == w.agents[m[1]].name),
        "{:?}",
        c.crafts
    );
    w.events.drain();
    let dead = w.agents[m[1]].id;
    hf_sim::systems::life::die(&mut w, dead, "a fever".into());
    at_lesson_time(&mut w);
    hf_sim::systems::crafts::step(&mut w);
    let evs = w.events.drain();
    let lost = evs
        .iter()
        .find(|e| matches!(&e.kind, EventKind::CraftLost { craft, .. } if craft == "smelting"))
        .expect("the loss is recorded");
    let line = hf_sim::view::narrate::line(&w, lost);
    assert!(
        line.contains("dead") && line.contains("nobody in"),
        "{line}"
    );
    let c = w.communities.iter().find(|c| c.id == cid).unwrap();
    assert!(!c.crafts.iter().any(|(t, _)| *t == Technique::Smelting));
}

#[test]
fn a_lone_knower_teaches_at_the_bench_and_some_refuse_for_their_reasons() {
    let (mut w, cid) = meadow(7003, 4);
    let home = w.communities[0].home;
    w.place_structure(
        StructureKind::CarpenterBench,
        home.offset(4, 0, 0),
        Material::WOOD,
        true,
    );
    let m = members(&w, cid);
    let tick = w.meta.tick;
    w.agents[m[0]]
        .techniques
        .learn(Technique::Carpentry, Learned::Past, 0.6, tick);
    let teacher = w.agents[m[0]].id;
    // Everyone awake; the best other hand is easygoing and would rather not.
    for &i in &m {
        w.agents[i].needs.rest = 1.0;
        w.agents[i]
            .traits
            .list
            .retain(|t| !matches!(t, Trait::Easygoing | Trait::Solitary));
    }
    let best = *m[1..]
        .iter()
        .max_by(|&&x, &&y| {
            w.agents[x]
                .skills
                .level(hf_sim::mind::Skill::Crafting)
                .total_cmp(&w.agents[y].skills.level(hf_sim::mind::Skill::Crafting))
                .then(w.agents[y].id.cmp(&w.agents[x].id))
        })
        .unwrap();
    w.agents[best].traits.list.push(Trait::Easygoing);
    at_lesson_time(&mut w);
    hf_sim::systems::crafts::step(&mut w);
    let evs = w.events.drain();
    let refused = evs
        .iter()
        .find(|e| matches!(&e.kind, EventKind::Chose { what, .. } if what.starts_with("refused to learn carpentry")))
        .expect("the easygoing one refuses");
    let line = hf_sim::view::narrate::line(&w, refused);
    assert!(line.contains("rather not be tied to a bench"), "{line}");
    assert!(!w.agents[best].techniques.knows(Technique::Carpentry));
    // Asked, they are not asked again for a month; the next day the next hand is taught.
    at_lesson_time(&mut w);
    hf_sim::systems::crafts::step(&mut w);
    let evs = w.events.drain();
    let taught = evs
        .iter()
        .find(|e| matches!(&e.kind, EventKind::Taught { .. }))
        .expect("a lesson the next day");
    let EventKind::Taught {
        pupil, teacher: t2, ..
    } = &taught.kind
    else {
        unreachable!()
    };
    assert_eq!(*t2, teacher);
    let p = w.agents.iter().find(|a| a.id == *pupil).unwrap();
    assert!(p.techniques.knows(Technique::Carpentry));
    let k = p
        .techniques
        .list
        .iter()
        .find(|k| k.technique == Technique::Carpentry)
        .unwrap();
    assert_eq!(k.learned, Learned::Taught);
    assert!(k.from.starts_with("taught by"), "{}", k.from);
    let line = hf_sim::view::narrate::line(&w, taught);
    assert!(
        line.contains("taught") && line.contains("carpentry"),
        "{line}"
    );
}

#[test]
fn arrivals_bring_the_crafts_of_their_trades() {
    use hf_sim::past::Formative;
    let mut found = false;
    for seed in 1..40u64 {
        let w = World::opening_world(seed);
        for a in w.agents.iter().filter(|a| a.community.is_some()) {
            for (_, f) in hf_sim::opening::formatives(&w, a.id) {
                if let Formative::Apprenticed { craft, .. } = f {
                    if craft == "the carpenter" {
                        found = true;
                        assert!(a.techniques.knows(Technique::Carpentry), "{}", a.name);
                        let k = a
                            .techniques
                            .list
                            .iter()
                            .find(|k| k.technique == Technique::Carpentry)
                            .unwrap();
                        assert!(k.from.starts_with("apprenticed to"), "{}", k.from);
                    }
                }
            }
        }
        if found {
            break;
        }
    }
    assert!(found, "some arrival was apprenticed to the carpenter");
}

#[test]
fn a_craft_is_not_lost_while_its_knower_is_away_on_the_communitys_journey() {
    // #259: the only potter led the community's own caravan, and on the second day the record
    // said nobody knew potting any more.
    let (mut w, cid) = meadow(7004, 5);
    let m = members(&w, cid);
    let tick = w.meta.tick;
    w.agents[m[0]]
        .techniques
        .learn(Technique::Potting, Learned::Past, 0.5, tick);
    at_lesson_time(&mut w);
    hf_sim::systems::crafts::step(&mut w);
    // Off on a journey: out of the valley's agents, into the ones away.
    let potter = w.agents.remove(m[0]);
    w.overworld.away.push(potter);
    at_lesson_time(&mut w);
    hf_sim::systems::crafts::step(&mut w);
    let evs = w.events.drain();
    assert!(
        !evs.iter()
            .any(|e| matches!(e.kind, EventKind::CraftLost { .. })),
        "no loss while the potter is away"
    );
    let c = w.communities.iter().find(|c| c.id == cid).unwrap();
    assert!(c.crafts.iter().any(|(t, _)| *t == Technique::Potting));
    // Nobody tries potting in the meantime either.
    let bench = w.communities[0].home.offset(4, 0, 0);
    let d = hf_sim::systems::settlement::make(
        &mut w,
        DesignationKind::Craft(hf_sim::work::recipes::POT),
        bench,
        5,
    );
    w.meta.tick += TICKS_PER_DAY + 1;
    for &i in &members(&w, cid) {
        assert!(!hf_sim::systems::crafts::may_try(
            &w,
            &w.agents[i],
            Technique::Potting,
            &d
        ));
    }
}

#[test]
fn a_craft_lost_and_known_again_is_said() {
    // #259: nothing was recorded when a craft came back.
    let (mut w, cid) = meadow(7005, 4);
    let m = members(&w, cid);
    let tick = w.meta.tick;
    w.agents[m[1]]
        .techniques
        .learn(Technique::Weaving, Learned::Past, 0.5, tick);
    at_lesson_time(&mut w);
    hf_sim::systems::crafts::step(&mut w);
    let weaver = w.agents[m[1]].id;
    hf_sim::systems::life::die(&mut w, weaver, "a fever".into());
    at_lesson_time(&mut w);
    hf_sim::systems::crafts::step(&mut w);
    w.events.drain();
    let tick = w.meta.tick;
    w.agents[m[2]]
        .techniques
        .learn(Technique::Weaving, Learned::Studied, 0.4, tick);
    at_lesson_time(&mut w);
    hf_sim::systems::crafts::step(&mut w);
    let evs = w.events.drain();
    let again = evs
        .iter()
        .find(|e| matches!(&e.kind, EventKind::CraftRegained { craft, .. } if craft == "weaving close"))
        .expect("the craft known again is said");
    let line = hf_sim::view::narrate::line(&w, again);
    assert!(line.contains("again"), "{line}");
    let c = w.communities.iter().find(|c| c.id == cid).unwrap();
    assert!(c.crafts_lost.is_empty());
}

#[test]
fn a_craft_gone_with_a_split_is_said_once_and_rightly() {
    // C72 soak: "gone from the area" of people who had split off to the valley's second
    // community, said four times for one person.
    let (mut w, cid) = meadow(7006, 5);
    let m = members(&w, cid);
    let tick = w.meta.tick;
    w.agents[m[1]]
        .techniques
        .learn(Technique::Masonry, Learned::Past, 0.5, tick);
    at_lesson_time(&mut w);
    hf_sim::systems::crafts::step(&mut w);
    // The mason goes with a split to a new community across the meadow.
    let other = w.found_community(TilePos::new(50, 50, 2));
    let mason = w.agents[m[1]].id;
    w.join_community(mason, other);
    w.events.drain();
    let mut said = 0;
    for _ in 0..5 {
        at_lesson_time(&mut w);
        hf_sim::systems::crafts::step(&mut w);
        let evs = w.events.drain();
        for e in evs.iter().filter(
            |e| matches!(&e.kind, EventKind::CraftLost { community, .. } if *community == cid),
        ) {
            said += 1;
            let line = hf_sim::view::narrate::line(&w, e);
            assert!(line.contains("gone to"), "{line}");
            assert!(!line.contains("from the valley"), "{line}");
        }
    }
    assert_eq!(said, 1, "said once");
}
