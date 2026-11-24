//! P1 gates (docs/design-peoples.md section 2): body plans as data that the body simulation
//! reads, one part vocabulary that old saves still load into, and a body block in the genome
//! whose size changes how fast a person gets hungry and cold.

use hf_sim::body::{Part, Wound, WoundKind};
use hf_sim::genetics::{body, Genome};
use hf_sim::plan::{biped, parse, plans, PartFilter, PartId, Role};
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

#[test]
fn human_plan_matches_old_parts() {
    let b = biped();
    let ids: Vec<PartId> = b.parts.iter().map(|d| d.id).collect();
    assert_eq!(
        ids,
        vec![
            Part::HEAD,
            Part::TORSO,
            Part::ARM_L,
            Part::ARM_R,
            Part::LEG_L,
            Part::LEG_R
        ]
    );
    let words: Vec<&str> = ids.iter().map(|p| p.label()).collect();
    assert_eq!(
        words,
        [
            "head",
            "chest",
            "left arm",
            "right arm",
            "left leg",
            "right leg"
        ]
    );
    assert_eq!(b.hands(), 2.0);
}

#[test]
fn every_built_in_plan_loads() {
    let ps = plans();
    assert_eq!(ps.len(), 12);
    // The skeletons; the beasts' other kinds are checked in tests/body_kinds.rs (#423).
    for p in ps.iter().filter(|p| p.for_people()) {
        assert!(p.has(PartId::HEAD) && p.has(PartId::TORSO), "{}", p.id);
        assert!(p.hands() >= 2.0, "{}", p.id);
    }
    let four = hf_sim::plan::plan("four_armed").unwrap();
    assert_eq!(four.hands(), 3.0, "four arms are three hands' worth");
    assert!(hf_sim::plan::plan("winged")
        .unwrap()
        .parts
        .iter()
        .any(|d| d.roles.contains(&Role::Fly)));
}

#[test]
fn bad_plans_are_refused() {
    let ok = "id = \"x\"\ncell = 48\ntiles = 1\n";
    let part = |id: &str, roles: &str, w: u32| {
        format!("[[part]]\nid = \"{id}\"\nroles = [{roles}]\nwound_weight = {w}\n")
    };
    let full = format!(
        "{ok}{}{}{}{}{}",
        part("head", "\"sense\"", 8),
        part("torso", "\"vital\"", 30),
        part("arm_l", "\"manipulate\"", 10),
        part("arm_r", "\"manipulate\"", 10),
        part("leg_l", "\"locomote\"", 12)
    );
    assert!(parse("x", &full).is_ok());
    let headless = full.replace("id = \"head\"", "id = \"tail\"");
    assert!(parse("x", &headless).is_err(), "no head");
    let one_hand = full.replace(
        "id = \"arm_r\"\nroles = [\"manipulate\"]",
        "id = \"arm_r\"\nroles = [\"balance\"]",
    );
    assert!(parse("x", &one_hand).is_err(), "one hand");
    let unknown = full.replace("id = \"leg_l\"", "id = \"fin\"");
    assert!(parse("x", &unknown).is_err(), "unknown part");
    let unhurtable = full.replace("wound_weight = 30", "wound_weight = 0");
    assert!(
        parse("x", &unhurtable).is_err(),
        "a part that cannot be hurt"
    );
    assert!(parse("y", &full).is_err(), "id must match the file");
}

#[test]
fn old_part_names_still_load() {
    for (old, id) in [
        ("Head", Part::HEAD),
        ("Torso", Part::TORSO),
        ("LeftArm", Part::ARM_L),
        ("RightArm", Part::ARM_R),
        ("LeftLeg", Part::LEG_L),
        ("RightLeg", Part::LEG_R),
    ] {
        let p: PartId = serde_json::from_str(&format!("\"{old}\"")).unwrap();
        assert_eq!(p, id);
    }
    let w = Wound {
        part: Part::LEG_R,
        kind: WoundKind::Break,
        severity: 0.5,
        bleeding: 0.0,
        infected: false,
        since: 3,
        by: None,
        tissue: hf_sim::body::Tissue::Bone,
        treated: Some(hf_sim::body::Treatment::Splinted),
        in_anger: false,
    };
    let bytes = rmp_serde::to_vec_named(&w).unwrap();
    let back: Wound = rmp_serde::from_slice(&bytes).unwrap();
    assert_eq!(back, w);
}

#[test]
fn wounds_land_on_plan_parts() {
    let mut r = ChaCha8Rng::seed_from_u64(7);
    for p in plans() {
        for _ in 0..500 {
            let any = p.random_part(&mut r, PartFilter::Any);
            assert!(p.has(any), "{} {:?}", p.id, any);
            let leg = p.random_part(&mut r, PartFilter::Role(Role::Locomote));
            assert!(
                p.parts
                    .iter()
                    .any(|d| d.id == leg && d.roles.contains(&Role::Locomote)),
                "{} {:?}",
                p.id,
                leg
            );
        }
    }
    // The coils of a serpentine body take most of what lands below.
    let s = hf_sim::plan::plan("serpentine").unwrap();
    assert_eq!(
        s.random_part(&mut r, PartFilter::Role(Role::Locomote)),
        PartId::by_key("coil").unwrap()
    );
}

#[test]
fn same_genome_same_body_and_children_resemble() {
    let mut r = ChaCha8Rng::seed_from_u64(11);
    let mum = Genome::random(&mut r);
    let dad = Genome::random(&mut r);
    let a = Genome::inherit(&mum, &dad, &mut ChaCha8Rng::seed_from_u64(5));
    let b = Genome::inherit(&mum, &dad, &mut ChaCha8Rng::seed_from_u64(5));
    assert_eq!(a, b, "same parents and stream, same child");
    let mut from_parent = 0;
    for k in 0..hf_sim::genetics::BODY {
        if a.body[k] == mum.body[k] || a.body[k] == dad.body[k] {
            from_parent += 1;
        }
    }
    assert!(
        from_parent >= 6,
        "{from_parent} of 8 body loci from a parent"
    );
    // Old genomes without a body block read as the middle of every range.
    let old: Genome = serde_json::from_str(&format!("{{\"loci\":{:?}}}", mum.loci)).unwrap();
    assert_eq!(old.body, [128; 8]);
}

#[test]
fn size_scales_hunger_and_cold() {
    let mut small = Genome::default();
    small.body[body::SIZE] = 0;
    small.loci[hf_sim::genetics::locus::BUILD] = 0;
    let mut big = small;
    big.body[body::SIZE] = 255;
    big.loci[hf_sim::genetics::locus::BUILD] = 255;
    assert!((small.mass_kg() - 51.0).abs() < 0.01 && (big.mass_kg() - 79.0).abs() < 0.01);
    let ratio = big.hunger_scale() / small.hunger_scale();
    assert!(
        (ratio - hf_sim::fmath::Det::d_powf(79.0f32 / 51.0, 0.75)).abs() < 1e-4,
        "{ratio}"
    );
    assert!(small.heat_loss() > 1.0 && big.heat_loss() < 1.0);
    // Through the needs system: the bigger body is hungrier after a day awake.
    let mut w = hf_sim::World::generate(3901, hf_sim::world::gen::GenParams::small());
    w.meta.founding_site = hf_sim::ids::TilePos::new(30, 30, 2);
    let ids = w.found_party(2);
    let ia = w.agents.iter().position(|a| a.id == ids[0]).unwrap();
    let ib = w.agents.iter().position(|a| a.id == ids[1]).unwrap();
    for (i, g) in [(ia, small), (ib, big)] {
        w.agents[i].genome = g;
        w.agents[i].traits = Default::default();
        w.agents[i].needs.hunger = 1.0;
        w.agents[i].activity = hf_sim::mind::Activity::Idle;
    }
    for _ in 0..hf_sim::time::TICKS_PER_HOUR * 4 {
        w.meta.tick += 1;
        hf_sim::systems::needs::step(&mut w);
    }
    let (ha, hb) = (
        1.0 - w.agents[ia].needs.hunger,
        1.0 - w.agents[ib].needs.hunger,
    );
    assert!((hb / ha - ratio).abs() < 0.02, "{hb} / {ha} vs {ratio}");
}

#[test]
fn capacities_follow_the_parts() {
    use hf_sim::body::{Ailment, Body, PartState, Tissue};
    let near = |a: f32, b: f32| (a - b).abs() < 1e-4;
    let whole = Body::default().capacities();
    assert_eq!(
        (whole.moving, whole.manipulation, whole.sight),
        (1.0, 1.0, 1.0)
    );

    let mut b = Body::default();
    b.parts.insert(PartId::LEG_L, PartState::Lost);
    let c = b.capacities();
    assert!(near(c.moving, 0.6) && near(c.manipulation, 1.0), "{c:?}");

    let mut b = Body::default();
    b.parts.insert(PartId::ARM_R, PartState::Scarred);
    assert!(near(b.capacities().manipulation, 0.85));

    let mut b = Body::default();
    b.ailments.push(Ailment::DimEyes);
    assert!(near(b.capacities().sight, 0.6));

    let mut b = Body::default();
    b.wounds.push(Wound {
        part: PartId::LEG_L,
        kind: WoundKind::Break,
        severity: 0.6,
        bleeding: 0.0,
        infected: false,
        since: 0,
        by: None,
        tissue: Tissue::Bone,
        treated: None,
        in_anger: false,
    });
    assert!(b.capacities().moving < 0.6);
    b.wounds.clear();
    assert_eq!(b.capacities().moving, 1.0);
}

#[test]
fn pain_sums_wounds_by_depth_and_eases_with_treatment() {
    use hf_sim::body::{Body, Tissue};

    fn wound(severity: f32, kind: WoundKind, tissue: Tissue) -> Wound {
        Wound {
            part: Part::TORSO,
            kind,
            severity,
            bleeding: 0.0,
            infected: false,
            since: 0,
            by: None,
            tissue,
            treated: None,
            in_anger: false,
        }
    }

    let whole = Body::default();
    assert_eq!(whole.pain(0, 0), 0.0);

    let mut skin_cut = Body::default();
    skin_cut
        .wounds
        .push(wound(0.3, WoundKind::Cut, Tissue::Skin));

    let mut bone_break = Body::default();
    bone_break
        .wounds
        .push(wound(0.3, WoundKind::Break, Tissue::Bone));
    assert!(skin_cut.pain(0, 0) < bone_break.pain(0, 0));

    let mut three_cuts = Body::default();
    for _ in 0..3 {
        three_cuts
            .wounds
            .push(wound(0.3, WoundKind::Cut, Tissue::Skin));
    }
    assert!(three_cuts.pain(0, 0) > skin_cut.pain(0, 0));

    let unsplinted = bone_break.pain(0, 0);
    bone_break.wounds[0].treated = Some(hf_sim::body::Treatment::Splinted);
    let splinted = bone_break.pain(0, 0);
    assert!(
        (splinted - unsplinted * 0.6).abs() < 1e-5,
        "{splinted} vs {}",
        unsplinted * 0.6
    );

    let untended = bone_break.pain(10, 0);
    let tended = bone_break.pain(10, 20);
    assert!(
        (tended - untended * 0.7).abs() < 1e-5,
        "{tended} vs {}",
        untended * 0.7
    );
}
