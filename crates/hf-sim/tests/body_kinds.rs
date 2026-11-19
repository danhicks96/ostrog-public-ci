//! #423: bodies that are not skeletons. The plan kinds (soft, swarm, segmented) and the parts
//! they are made of, appended to the one vocabulary; their files load and are checked; and they
//! are beasts' only.

use hf_sim::plan::{biped, parse, plan, plans, Kind, PartId, Role, SOFT_WHOLE};

/// The vocabulary is append-only: saves store part ids, so the old ones keep their numbers.
#[test]
fn the_vocabulary_only_grew() {
    let old = [
        "head",
        "torso",
        "arm_l",
        "arm_r",
        "leg_l",
        "leg_r",
        "tail",
        "wing_l",
        "wing_r",
        "lower_arm_l",
        "lower_arm_r",
        "foreleg_l",
        "foreleg_r",
        "coil",
    ];
    for (i, k) in old.iter().enumerate() {
        assert_eq!(PartId(i as u8).key(), *k);
    }
    assert_eq!(PartId::MASS, PartId::by_key("mass").unwrap());
    assert_eq!(PartId::MEMBER, PartId::by_key("member").unwrap());
    assert_eq!(PartId::SEGMENT, PartId::by_key("segment").unwrap());
    assert_eq!(PartId::MASS.0, 14);
    assert_eq!(PartId::SEGMENT.label(), "segments");
    // A saved part id reads back as the same part.
    let back: PartId = rmp_serde::from_slice(&rmp_serde::to_vec(&PartId::MEMBER).unwrap()).unwrap();
    assert_eq!(back, PartId::MEMBER);
}

#[test]
fn the_new_kinds_load_with_their_parts() {
    let soft = plan("soft").unwrap();
    assert_eq!(soft.kind, Kind::Soft);
    assert_eq!(soft.parts.len(), 1);
    assert!(soft.has(PartId::MASS));
    assert_eq!(soft.count, [SOFT_WHOLE, SOFT_WHOLE]);
    let swarm = plan("swarm").unwrap();
    assert_eq!(swarm.kind, Kind::Swarm);
    assert!(swarm.has(PartId::MEMBER) && !swarm.has(PartId::HEAD));
    assert!(swarm.count[0] >= 2 && swarm.count[0] <= swarm.count[1]);
    let chain = plan("segmented").unwrap();
    assert_eq!(chain.kind, Kind::Segmented);
    assert!(chain.has(PartId::HEAD) && chain.has(PartId::SEGMENT));
    assert_eq!(Kind::Segmented.counted(), Some(PartId::SEGMENT));
    for p in [soft, swarm, chain] {
        assert!(!p.for_people(), "{}", p.id);
        assert_eq!(p.tiles, 1, "{}: one tile (#421)", p.id);
        assert_eq!(p.hands(), 0.0, "{}", p.id);
    }
    // The old plans are skeletons, unchanged.
    assert_eq!(biped().kind, Kind::Skeletal);
    assert_eq!(biped().count, [0, 0]);
    assert_eq!(plans().iter().filter(|p| p.for_people()).count(), 9);
}

#[test]
fn bad_kinds_are_refused() {
    let head = "[[part]]\nid = \"head\"\nroles = [\"sense\", \"vital\"]\nwound_weight = 10\n";
    let seg = "[[part]]\nid = \"segment\"\nroles = [\"locomote\"]\nwound_weight = 90\n";
    let mass = "[[part]]\nid = \"mass\"\nroles = [\"vital\", \"locomote\"]\nwound_weight = 1\n";
    let top = |kind: &str, count: &str| {
        format!("id = \"x\"\nkind = \"{kind}\"\ncell = 32\ntiles = 1\n{count}")
    };
    let chain = format!("{}{head}{seg}", top("segmented", "count = [6, 9]\n"));
    assert!(parse("x", &chain).is_ok());
    assert!(
        parse("x", &format!("{}{head}{seg}", top("segmented", ""))).is_err(),
        "a chain must say how long"
    );
    assert!(
        parse(
            "x",
            &format!("{}{head}{seg}", top("segmented", "count = [1, 9]\n"))
        )
        .is_err(),
        "a chain of one is no chain"
    );
    assert!(
        parse(
            "x",
            &format!("{}{seg}", top("segmented", "count = [6, 9]\n"))
        )
        .is_err(),
        "a chain needs its head"
    );
    assert!(
        parse("x", &chain.replace("tiles = 1", "tiles = 2")).is_err(),
        "one tile (#421)"
    );
    assert!(parse("x", &format!("{}{mass}", top("soft", ""))).is_ok());
    assert!(
        parse("x", &format!("{}{mass}{head}", top("soft", ""))).is_err(),
        "a soft body has no head"
    );
    assert!(
        parse("x", &format!("{}{mass}", top("soft", "count = [2, 3]\n"))).is_err(),
        "a soft body is not counted"
    );
    assert!(
        parse("x", &format!("{}{mass}", top("blob", ""))).is_err(),
        "unknown kind"
    );
    // A skeleton may not carry the new kinds' parts.
    let skel = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/peoples/plans/biped.toml"
    ))
    .unwrap()
    .replace("id    = \"biped\"", "id = \"x\"");
    assert!(parse("x", &skel).is_ok());
    assert!(parse("x", &format!("{skel}{seg}")).is_err());
    // Every part of every kind moves it or keeps it alive.
    for p in plans().iter().filter(|p| !p.for_people()) {
        assert!(
            p.parts.iter().any(|d| d.roles.contains(&Role::Vital)),
            "{}",
            p.id
        );
    }
}

/// Beasts only (the default Dan may overrule): no people's species may have one of the new
/// bodies, from a file or from the generator.
#[test]
fn a_people_has_a_skeleton() {
    let mut s = hf_sim::species::library()[0].clone();
    assert!(hf_sim::species::validate(&s).is_ok());
    for k in ["soft", "swarm", "segmented"] {
        s.plan = k.into();
        assert!(hf_sim::species::validate(&s).is_err(), "{k}");
    }
    for s in hf_sim::species::library() {
        assert!(s.body_plan().for_people(), "{}", s.id);
    }
}

// --- Part 2: wounds, wear and death for each kind -------------------------------------------

use hf_sim::agents::Species;
use hf_sim::body::{Form, Part, WoundKind};
use hf_sim::systems::{combat, health};
use hf_sim::time::{TICKS_PER_HOUR, TICKS_PER_YEAR};
use hf_sim::World;

fn world() -> World {
    let mut w = World::generate(4231, hf_sim::world::gen::GenParams::small());
    w.agents.clear();
    w.plants.clear();
    w
}

/// A grown beast at the surface of (x, y), its body made of `kind` with `units` of it whole.
/// No creature of the valley has these bodies yet (#423 part 4 adds them), so a boar stands in.
fn formed(w: &mut World, x: i32, y: i32, kind: Kind, units: u16) -> usize {
    let p = w.grid.surface(x, y);
    let id = w.spawn_agent(Species::Boar, p);
    let i = w.agents.iter().position(|a| a.id == id).unwrap();
    w.agents[i].born = w.meta.tick as i64 - 3 * TICKS_PER_YEAR as i64;
    w.agents[i].needs.hunger = 1.0;
    w.agents[i].body.form = Some(Form {
        kind,
        units,
        whole: units,
    });
    i
}

fn form(w: &World, i: usize) -> Form {
    w.agents[i].body.form.unwrap()
}

/// Runs the health system to the next hourly pass.
fn an_hour(w: &mut World) {
    for _ in 0..TICKS_PER_HOUR {
        w.meta.tick += 1;
        health::step(w);
    }
}

#[test]
fn a_swarm_thins_and_does_not_bleed() {
    let mut w = world();
    let s = formed(&mut w, 30, 30, Kind::Swarm, 40);
    health::wound(
        &mut w,
        s,
        Part::TORSO,
        WoundKind::Cut,
        0.5,
        0.4,
        "in a test",
        None,
    );
    // A quarter of its whole at full severity: half that, five of forty.
    assert_eq!(form(&w, s).units, 35);
    assert!(w.agents[s].body.wounds.is_empty(), "no wound is kept");
    assert_eq!(w.agents[s].body.blood, 1.0, "a swarm does not bleed");
    assert!(w.agents[s].alive);
    let b = combat::battered(&w, s);
    assert!(b > 0.0 && b < 0.3, "{b}");
    // It weighs, and so hits and pushes, as much as is left of it (#422's size).
    let t = w.meta.tick;
    let before = w.agents[s].body_kg(t);
    health::wound(
        &mut w,
        s,
        Part::TORSO,
        WoundKind::Cut,
        1.0,
        0.4,
        "in a test",
        None,
    );
    assert_eq!(form(&w, s).units, 25);
    assert!((w.agents[s].body_kg(t) / before - 25.0 / 35.0).abs() < 1e-4);
    // On record, in the plan's words.
    assert!(w.events.pending.iter().any(|e| matches!(
        &e.kind,
        hf_sim::events::EventKind::Thinned { part, lost: 10, left: 25, .. } if part == "members"
    )));
    // Down to under a fifth of itself it is felled, and the hourly pass ends it.
    for _ in 0..3 {
        health::wound(
            &mut w,
            s,
            Part::TORSO,
            WoundKind::Cut,
            1.0,
            0.4,
            "in a test",
            None,
        );
    }
    assert!(form(&w, s).spent(), "{:?}", form(&w, s));
    assert!(combat::battered(&w, s) >= 1.0);
    an_hour(&mut w);
    assert!(!w.agents[s].alive);
    assert_eq!(w.agents[s].death.as_deref(), Some("scattered"));
}

#[test]
fn a_soft_body_loses_some_of_itself() {
    let mut w = world();
    let s = formed(&mut w, 30, 30, Kind::Soft, SOFT_WHOLE);
    health::wound(
        &mut w,
        s,
        Part::HEAD,
        WoundKind::Cut,
        0.5,
        0.4,
        "in a test",
        None,
    );
    assert_eq!(form(&w, s).units, 80, "a fifth of it gone");
    assert!(w.agents[s].body.wounds.is_empty() && w.agents[s].body.blood == 1.0);
    assert_eq!(form(&w, s).label(), "80% of its mass");
    for _ in 0..2 {
        health::wound(
            &mut w,
            s,
            Part::HEAD,
            WoundKind::Cut,
            1.0,
            0.4,
            "in a test",
            None,
        );
    }
    assert_eq!(form(&w, s).units, 0);
    an_hour(&mut w);
    assert_eq!(w.agents[s].death.as_deref(), Some("broken up"));
}

#[test]
fn a_chain_shortens_and_dies_by_its_head() {
    let mut w = world();
    // Enough blows land on the segments (nine in ten of the plan's wound weight) to see each.
    let c = formed(&mut w, 30, 30, Kind::Segmented, 12);
    let mut cut = 0;
    let mut glanced = 0;
    for k in 0..30 {
        w.meta.tick += 1;
        let before = form(&w, c).units;
        let wounds = w.agents[c].body.wounds.len();
        let sev = if k % 2 == 0 { 0.2 } else { 0.5 };
        health::wound(
            &mut w,
            c,
            Part::TORSO,
            WoundKind::Cut,
            sev,
            0.4,
            "in a test",
            None,
        );
        let head = w.agents[c].body.wounds.len() > wounds;
        let lost = before - form(&w, c).units;
        if head {
            assert_eq!(lost, 0, "a blow to the head cuts no segment");
            assert_eq!(w.agents[c].body.wounds.last().unwrap().part, Part::HEAD);
            assert_eq!(w.agents[c].body.wounds.last().unwrap().bleeding, 0.0);
        } else if sev < health::SEVER {
            assert_eq!(lost, 0, "a light blow glances off");
            glanced += 1;
        } else if before > 0 {
            assert_eq!(lost, 1, "a deep one cuts a segment through");
            cut += 1;
        }
        if form(&w, c).units < 4 {
            break;
        }
    }
    assert!(cut >= 3 && glanced >= 3, "{cut} cut, {glanced} glanced");
    assert!(form(&w, c).label().ends_with("of 12 segments"));
    // A chain with its head battered is felled however long it is.
    let mut w = world();
    let c = formed(&mut w, 30, 30, Kind::Segmented, 12);
    w.agents[c].body.wounds.push(hf_sim::body::Wound {
        part: Part::HEAD,
        kind: WoundKind::Cut,
        severity: 1.0,
        bleeding: 0.0,
        infected: false,
        since: 0,
        by: None,
        tissue: hf_sim::body::Tissue::Organ,
        treated: None,
        in_anger: false,
    });
    assert!(combat::battered(&w, c) >= 1.0);
    assert!(!form(&w, c).spent());
    // Two segments and a head still live; one does not.
    let f = Form {
        kind: Kind::Segmented,
        units: 2,
        whole: 12,
    };
    assert!(!f.spent());
    assert!(Form { units: 1, ..f }.spent());
}

#[test]
fn hunger_thins_a_swarm_and_the_dry_shrinks_a_soft_body() {
    let mut w = world();
    let s = formed(&mut w, 30, 30, Kind::Swarm, 20);
    let soft = formed(&mut w, 34, 30, Kind::Soft, SOFT_WHOLE);
    let fed = formed(&mut w, 38, 30, Kind::Swarm, 20);
    w.agents[s].needs.hunger = 0.0;
    for _ in 0..8 {
        w.agents[s].needs.hunger = 0.0;
        an_hour(&mut w);
    }
    assert_eq!(form(&w, s).units, 18, "a member every four hours");
    assert_eq!(form(&w, fed).units, 20, "a fed one keeps its number");
    // The soft body stands above ground; with no water by it, it dries.
    let at = w.agents[soft].pos;
    if health::dry(&w, at) {
        assert_eq!(form(&w, soft).units, SOFT_WHOLE - 8);
    }
    // Below ground it is in the damp and keeps itself.
    let deep = hf_sim::ids::TilePos::new(at.x, at.y, w.grid.ground_z(at.x, at.y) - 4);
    assert!(!health::dry(&w, deep));
}

#[test]
fn a_hunter_does_not_take_a_swarm_at_a_stroke() {
    let mut w = world();
    let s = formed(&mut w, 30, 30, Kind::Swarm, 40);
    let ids = {
        w.meta.founding_site = w.agents[s].pos.offset(1, 0, 0);
        w.found_party(1)
    };
    let h = w.agents.iter().position(|a| a.id == ids[0]).unwrap();
    // Three blows at most, each a quarter of it at full severity: never to under a fifth. If
    // none of a hunt's blows lands, the next hunt's do.
    for _ in 0..6 {
        if form(&w, s).units < 40 {
            break;
        }
        w.meta.tick += 1;
        assert!(!combat::hunted_down(&mut w, h, s));
    }
    assert!(w.agents[s].alive);
    assert!(form(&w, s).units < 40, "the blows thinned it");
    // A skeleton is taken at a stroke, as before.
    let p = w.agents[s].pos;
    let id = w.spawn_agent(Species::Deer, p);
    let d = w.agents.iter().position(|a| a.id == id).unwrap();
    assert!(combat::hunted_down(&mut w, h, d));
}

#[test]
fn a_form_saves_and_old_bodies_have_none() {
    let mut b = hf_sim::body::Body::default();
    assert_eq!(b.form, None);
    b.form = Some(Form {
        kind: Kind::Segmented,
        units: 7,
        whole: 11,
    });
    let back: hf_sim::body::Body =
        rmp_serde::from_slice(&rmp_serde::to_vec_named(&b).unwrap()).unwrap();
    assert_eq!(back, b);
    // A body saved before #423 has no form.
    #[derive(serde::Serialize)]
    struct Old {
        wounds: Vec<hf_sim::body::Wound>,
        blood: f32,
        illness: Option<hf_sim::body::Illness>,
        ailments: Vec<hf_sim::body::Ailment>,
        scars: u8,
    }
    let old = Old {
        wounds: vec![],
        blood: 0.9,
        illness: None,
        ailments: vec![],
        scars: 1,
    };
    let back: hf_sim::body::Body =
        rmp_serde::from_slice(&rmp_serde::to_vec_named(&old).unwrap()).unwrap();
    assert_eq!(back.form, None);
    // No beast of the valley has one of these bodies yet.
    for s in [
        Species::Wolf,
        Species::Bear,
        Species::Crawler,
        Species::Burrower,
    ] {
        assert!(hf_sim::plan::beast_plan(s).is_none());
    }
}

// --- Part 3: abilities as part properties -----------------------------------------------------

use hf_sim::plan::Ability;
use hf_sim::world::{Material, Tile};

#[test]
fn abilities_come_from_the_parts() {
    let soft = plan("soft").unwrap();
    for a in [
        Ability::Split,
        Ability::Engulf,
        Ability::Squeeze,
        Ability::Regrow,
    ] {
        assert!(soft.can(a), "{a:?}");
    }
    assert!(!soft.can(Ability::Climb));
    let swarm = plan("swarm").unwrap();
    assert!(swarm.can(Ability::Squeeze) && swarm.can(Ability::Regrow));
    assert!(!swarm.can(Ability::Split) && !swarm.can(Ability::Engulf));
    let chain = plan("segmented").unwrap();
    assert!(chain.can(Ability::Climb) && chain.can(Ability::Regrow));
    // A skeleton has none; and the files may not give what a kind cannot have.
    assert!(plans().iter().filter(|p| p.for_people()).all(|p| [
        Ability::Climb,
        Ability::Split,
        Ability::Engulf,
        Ability::Squeeze,
        Ability::Regrow
    ]
    .iter()
    .all(|a| !p.can(*a))));
    let top = "id = \"x\"\nkind = \"segmented\"\ncell = 32\ntiles = 1\ncount = [6, 9]\n";
    let head = |ab: &str| {
        format!("[[part]]\nid = \"head\"\nroles = [\"sense\", \"vital\"]\nwound_weight = 10\nabilities = [{ab}]\n")
    };
    let seg = |ab: &str| {
        format!("[[part]]\nid = \"segment\"\nroles = [\"locomote\"]\nwound_weight = 90\nabilities = [{ab}]\n")
    };
    assert!(parse(
        "x",
        &format!("{top}{}{}", head(""), seg("\"climb\", \"regrow\""))
    )
    .is_ok());
    assert!(
        parse("x", &format!("{top}{}{}", head(""), seg("\"split\""))).is_err(),
        "only a soft body splits"
    );
    assert!(
        parse("x", &format!("{top}{}{}", head("\"regrow\""), seg(""))).is_err(),
        "a chain grows back segments, not heads"
    );
    assert!(
        parse("x", &format!("{top}{}{}", head(""), seg("\"fly\""))).is_err(),
        "unknown ability"
    );
}

#[test]
fn a_soft_body_cut_deep_splits_in_two() {
    let mut w = world();
    let s = formed(&mut w, 30, 30, Kind::Soft, SOFT_WHOLE);
    let n = w.agents.len();
    // A bruise squashes it; it does not divide.
    health::wound(
        &mut w,
        s,
        Part::TORSO,
        WoundKind::Bruise,
        0.5,
        0.0,
        "in a test",
        None,
    );
    assert_eq!(form(&w, s).units, 80);
    assert_eq!(w.agents.len(), n);
    // A cut that takes a fifth of it or more leaves the piece alive beside it.
    w.meta.tick += 1;
    health::wound(
        &mut w,
        s,
        Part::TORSO,
        WoundKind::Cut,
        0.5,
        0.4,
        "in a test",
        None,
    );
    assert_eq!(form(&w, s).units, 60);
    assert_eq!(w.agents.len(), n + 1);
    let piece = &w.agents[n];
    assert!(piece.alive && piece.species == w.agents[s].species);
    assert_eq!(
        piece.body.form,
        Some(Form {
            kind: Kind::Soft,
            units: 20,
            whole: SOFT_WHOLE
        })
    );
    assert!(piece.pos.dist(w.agents[s].pos) <= 1);
    assert!(w
        .events
        .pending
        .iter()
        .any(|e| matches!(e.kind, hf_sim::events::EventKind::Divided { .. })));
    // A swarm cut the same way only thins.
    let sw = formed(&mut w, 40, 30, Kind::Swarm, 40);
    let n = w.agents.len();
    health::wound(
        &mut w,
        sw,
        Part::TORSO,
        WoundKind::Cut,
        1.0,
        0.4,
        "in a test",
        None,
    );
    assert_eq!(w.agents.len(), n);
}

#[test]
fn a_soft_body_engulfs_small_creatures_and_not_people() {
    let mut w = world();
    let s = formed(&mut w, 30, 30, Kind::Soft, SOFT_WHOLE);
    w.agents[s].body.form.as_mut().unwrap().units = 60;
    w.agents[s].needs.hunger = 0.2;
    let at = w.agents[s].pos;
    let hare = w.spawn_agent(Species::Hare, at.offset(1, 0, 0));
    let deer = w.spawn_agent(Species::Deer, at.offset(0, 1, 0));
    assert!(hf_sim::systems::animals::engulf(&mut w, s));
    let h = w.agents.iter().find(|a| a.id == hare).unwrap();
    assert!(!h.alive);
    assert!(h.death.as_deref().unwrap().starts_with("engulfed by"));
    assert!(
        w.agents.iter().find(|a| a.id == deer).unwrap().alive,
        "too big"
    );
    assert_eq!(w.agents[s].needs.hunger, 1.0, "fed");
    assert!(form(&w, s).units > 60, "it grew by what it took");
    // Fed, with only the deer and a person by it, it takes nothing.
    w.meta.founding_site = at.offset(-1, 0, 0);
    let ids = w.found_party(1);
    let p = w.agents.iter().position(|a| a.id == ids[0]).unwrap();
    w.agents[p].pos = at.offset(-1, 0, 0);
    w.agents[s].needs.hunger = 0.2;
    hf_sim::systems::animals::engulf(&mut w, s);
    assert!(w.agents[p].alive, "never a person");
    // A swarm cannot engulf.
    let sw = formed(&mut w, 40, 30, Kind::Swarm, 30);
    assert!(!hf_sim::plan::can(&w.agents[sw], Ability::Engulf));
}

#[test]
fn regrowing_takes_food() {
    let mut w = world();
    let s = formed(&mut w, 30, 30, Kind::Swarm, 30);
    let c = formed(&mut w, 34, 30, Kind::Segmented, 10);
    let hungry = formed(&mut w, 38, 30, Kind::Swarm, 30);
    for i in [s, c, hungry] {
        w.agents[i].body.form.as_mut().unwrap().units -= 5;
    }
    w.agents[hungry].needs.hunger = 0.3;
    for _ in 0..48 {
        w.agents[s].needs.hunger = 1.0;
        w.agents[c].needs.hunger = 1.0;
        w.agents[hungry].needs.hunger = 0.3;
        an_hour(&mut w);
    }
    assert_eq!(
        form(&w, s).units,
        25 + 5,
        "a member every six hours, to whole"
    );
    assert_eq!(form(&w, c).units, 5 + 2, "a segment a day");
    assert_eq!(form(&w, hungry).units, 25, "not while hungry");
    // It cost them food.
    let before = 1.0;
    w.agents[s].body.form.as_mut().unwrap().units = 20;
    w.agents[s].needs.hunger = before;
    for _ in 0..6 {
        an_hour(&mut w);
    }
    assert!(w.agents[s].needs.hunger < before);
}

#[test]
fn a_swarm_squeezes_through_a_wooden_wall_and_not_stone() {
    let mut w = world();
    let s = formed(&mut w, 30, 30, Kind::Swarm, 30);
    let at = w.agents[s].pos;
    w.grid.set(at.offset(1, 0, 0), Tile::solid(Material::WOOD));
    let beyond = at.offset(2, 0, 0);
    assert!(w.grid.walkable(beyond));
    assert!(hf_sim::systems::animals::get_past(
        &mut w,
        s,
        at.offset(6, 0, 0)
    ));
    assert_eq!(w.agents[s].pos, beyond);
    assert!(w.events.pending.iter().any(|e| matches!(
        &e.kind,
        hf_sim::events::EventKind::Crept { how, .. } if how == "squeezed through"
    )));
    // Stone it cannot get through, and it is too light to break anything.
    let s2 = formed(&mut w, 40, 30, Kind::Swarm, 30);
    let at2 = w.agents[s2].pos;
    w.grid
        .set(at2.offset(1, 0, 0), Tile::solid(Material::STONE_BLOCK));
    assert!(!hf_sim::systems::animals::get_past(
        &mut w,
        s2,
        at2.offset(6, 0, 0)
    ));
    assert_eq!(w.agents[s2].pos, at2);
    // A chain cannot squeeze.
    let c = formed(&mut w, 30, 40, Kind::Segmented, 10);
    let atc = w.agents[c].pos;
    w.grid.set(atc.offset(1, 0, 0), Tile::solid(Material::WOOD));
    hf_sim::systems::animals::get_past(&mut w, c, atc.offset(6, 0, 0));
    assert_ne!(w.agents[c].pos, atc.offset(2, 0, 0));
}

#[test]
fn a_chain_climbs_a_rock_face() {
    let mut w = world();
    let c = formed(&mut w, 30, 30, Kind::Segmented, 10);
    let at = w.agents[c].pos;
    w.grid
        .set(at.offset(1, 0, 0), Tile::solid(Material::GRANITE));
    w.grid.set(at.offset(1, 0, 1), Tile::AIR);
    w.grid.set(at.offset(1, 0, 2), Tile::AIR);
    let up = at.offset(1, 0, 1);
    assert!(w.grid.walkable(up));
    assert!(hf_sim::systems::animals::get_past(
        &mut w,
        c,
        at.offset(5, 0, 1)
    ));
    assert_eq!(w.agents[c].pos, up);
    // And back down where a walker would need a stair.
    assert!(hf_sim::systems::animals::get_past(
        &mut w,
        c,
        at.offset(-3, 0, 0)
    ));
    assert_eq!(w.agents[c].pos.z, at.z);
    // A swarm does not climb.
    let s = formed(&mut w, 40, 30, Kind::Swarm, 30);
    let a2 = w.agents[s].pos;
    w.grid
        .set(a2.offset(1, 0, 0), Tile::solid(Material::GRANITE));
    w.grid.set(a2.offset(1, 0, 1), Tile::AIR);
    assert!(!hf_sim::systems::animals::get_past(
        &mut w,
        s,
        a2.offset(5, 0, 1)
    ));
}

// --- Part 4: the creatures ----------------------------------------------------------------------

use hf_sim::ids::TilePos;
use hf_sim::world::gen::caverns::Cavern;

fn cavern(name: &str, centre: TilePos, pool: bool, deep: bool) -> Cavern {
    Cavern {
        name: name.into(),
        centre,
        rx: 4,
        ry: 4,
        pool,
        deep,
        opened: false,
        underworld: false,
        breach: None,
        sealed: false,
        gate_of: None,
        keeper: None,
    }
}

/// A small world whose caverns are these, dug out with a floor.
fn cave_world(caverns: Vec<Cavern>) -> World {
    let mut w = World::generate(19, hf_sim::world::gen::GenParams::small());
    w.agents.clear();
    w.grid.caverns = caverns;
    for c in w.grid.caverns.clone() {
        for y in c.centre.y - c.ry..=c.centre.y + c.ry {
            for x in c.centre.x - c.rx..=c.centre.x + c.rx {
                let p = TilePos::new(x, y, c.centre.z);
                if c.contains(p) {
                    w.grid
                        .set(p.offset(0, 0, -1), Tile::solid(Material::GRANITE));
                    w.grid.set(p, Tile::AIR);
                    w.grid.set(p.offset(0, 0, 1), Tile::AIR);
                }
            }
        }
    }
    w
}

fn one(w: &World, s: Species) -> usize {
    let found: Vec<usize> = (0..w.agents.len())
        .filter(|&i| w.agents[i].alive && w.agents[i].species == s)
        .collect();
    assert_eq!(found.len(), 1, "{s:?}");
    found[0]
}

#[test]
fn each_new_beast_is_born_whole_of_its_kind() {
    let mut w = world();
    for (s, kind) in [
        (Species::Slime, Kind::Soft),
        (Species::Rats, Kind::Swarm),
        (Species::Centipede, Kind::Segmented),
    ] {
        let p = hf_sim::plan::beast_plan(s).unwrap();
        assert_eq!(p.kind, kind);
        for k in 0..6 {
            let at = w.grid.surface(20 + 3 * k, 20);
            let id = w.spawn_agent(s, at);
            let a = w.agents.iter().find(|a| a.id == id).unwrap();
            let f = a.body.form.expect("born with its form");
            assert_eq!(f.kind, kind);
            assert_eq!(f.units, f.whole);
            assert!((p.count[0]..=p.count[1]).contains(&f.whole), "{s:?} {f:?}");
            assert!(hf_sim::size::animal(s).is_some(), "{s:?} has a size");
        }
    }
    // The valley's other beasts are skeletons, as before.
    let id = w.spawn_agent(Species::Wolf, w.grid.surface(10, 10));
    assert!(w
        .agents
        .iter()
        .find(|a| a.id == id)
        .unwrap()
        .body
        .form
        .is_none());
}

#[test]
fn they_are_placed_where_they_live() {
    let shallow = cavern("shallow", TilePos::new(12, 12, -2), false, false);
    let wet_deep = cavern("wet deep", TilePos::new(40, 40, -6), true, true);
    let dry_deep = cavern("dry deep", TilePos::new(12, 40, -7), false, true);
    let mut w = cave_world(vec![shallow.clone(), wet_deep.clone(), dry_deep.clone()]);
    hf_sim::systems::animals::populate(&mut w);
    let slime = one(&w, Species::Slime);
    assert!(
        wet_deep.contains(w.agents[slime].pos),
        "the slime in the wet deep"
    );
    let c = one(&w, Species::Centipede);
    assert!(
        shallow.contains(w.agents[c].pos),
        "the centipede in the shallow cavern"
    );
    let rats = one(&w, Species::Rats);
    let at = w.agents[rats].pos;
    assert!(
        !hf_sim::underground::below(&w, at),
        "the rats on open ground"
    );
    assert!(at.dist(w.meta.founding_site) >= 16);
    for i in [slime, c, rats] {
        let id = w.agents[i].id;
        assert!(w.creatures.nests.iter().any(|n| n.0 == id), "a nest");
    }
    // No pool underground: no slime, for it would dry.
    let mut w = cave_world(vec![shallow, dry_deep]);
    hf_sim::systems::animals::populate(&mut w);
    assert!(!w.agents.iter().any(|a| a.species == Species::Slime));
}

/// Runs the animals and needs systems for `ticks`.
fn run_animals(w: &mut World, ticks: u64) {
    for _ in 0..ticks {
        w.meta.tick += 1;
        hf_sim::systems::needs::step(w);
        hf_sim::systems::animals::step(w);
        hf_sim::systems::movement::step(w);
    }
}

#[test]
fn a_hungry_slime_feeds_on_the_cave_fungus_and_shrinks_from_fire() {
    let c = cavern("wet", TilePos::new(20, 20, -4), true, true);
    let mut w = cave_world(vec![c.clone()]);
    let id = w.spawn_agent(Species::Slime, c.centre);
    let s = w.agents.iter().position(|a| a.id == id).unwrap();
    w.agents[s].needs.hunger = 0.1;
    let patch = c.centre.offset(3, 0, 0);
    w.plants.push(hf_sim::plants::Plant::new(
        patch,
        hf_sim::plants::PlantKind::Mushroom,
        1.0,
        0,
    ));
    run_animals(&mut w, 4 * TICKS_PER_HOUR);
    assert!(
        w.plants.last().unwrap().growth < 1.0,
        "it fed on the mushrooms"
    );
    assert!(w.agents[s].needs.hunger > 0.1);
    // A fire by it: it creeps away.
    let at = w.agents[s].pos;
    let fire = at.offset(1, 0, 0);
    w.lights.push((fire, 4.0, true));
    assert!(hf_sim::underground::firelight_at(&w, at) > 0.1);
    run_animals(&mut w, TICKS_PER_HOUR);
    assert!(w.agents[s].pos.dist(fire) > 1, "it shrank from the fire");
}

#[test]
fn a_centipede_hunts_bats_and_keeps_from_people() {
    let c = cavern("shallow", TilePos::new(20, 20, -2), false, false);
    let mut w = cave_world(vec![c.clone()]);
    let id = w.spawn_agent(Species::Centipede, c.centre);
    let ci = w.agents.iter().position(|a| a.id == id).unwrap();
    w.agents[ci].needs.hunger = 0.2;
    let bat = w.spawn_agent(Species::Bat, c.centre.offset(1, 0, 0));
    run_animals(&mut w, TICKS_PER_HOUR);
    let b = w.agents.iter().find(|a| a.id == bat).unwrap();
    assert!(!b.alive);
    assert_eq!(b.death.as_deref(), Some("taken by a centipede"));
    assert!(w.agents[ci].needs.hunger > 0.9);
    // Someone comes close: it goes off.
    let me = w.agents[ci].pos;
    let pid = w.spawn_agent(Species::Person, me.offset(2, 0, 0));
    let p = w.agents.iter().position(|a| a.id == pid).unwrap();
    let start = w.agents[ci].pos.dist(w.agents[p].pos);
    for _ in 0..TICKS_PER_HOUR {
        w.meta.tick += 1;
        hf_sim::systems::animals::step(&mut w);
        hf_sim::systems::movement::step(&mut w);
        w.agents[p].pos = me.offset(2, 0, 0);
    }
    assert!(
        w.agents[ci].pos.dist(w.agents[p].pos) > start,
        "it kept away"
    );
}

#[test]
fn rats_stood_against_thin_and_scatter() {
    let mut w = world();
    let at = w.grid.surface(30, 30);
    w.meta.founding_site = at;
    let ids = w.found_party(4);
    let id = w.spawn_agent(Species::Rats, at.offset(1, 0, 0));
    let r = w.agents.iter().position(|a| a.id == id).unwrap();
    for pid in &ids {
        let p = w.agents.iter().position(|a| a.id == *pid).unwrap();
        w.agents[p].pos = at;
        w.agents[p].beliefs = Default::default();
    }
    let whole = form(&w, r).whole;
    let victim = w.agents.iter().position(|a| a.id == ids[0]).unwrap();
    let mut rng = <rand_chacha::ChaCha8Rng as rand::SeedableRng>::seed_from_u64(423);
    let mut thinned = false;
    for _ in 0..20 {
        if !w.agents[r].alive {
            break;
        }
        w.meta.tick += 1;
        combat::stand_against(&mut w, r, victim, &mut rng);
        thinned |= form(&w, r).units < whole;
    }
    assert!(thinned, "the blows took some of them");
    // Enough blows scatter them; it is said so.
    if !w.agents[r].alive {
        assert!(w.agents[r]
            .death
            .as_deref()
            .unwrap()
            .starts_with("scattered by"));
    }
}
