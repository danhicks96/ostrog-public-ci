//! Phase 13 gate: every state a person can be in resolves to sprite layers that exist.

use std::path::PathBuf;

use hf_sim::ids::TilePos;
use hf_sim::society::Role;
use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_YEAR};
use hf_sim::view::look::{look, EQUIPMENT, OUTFITS};
use hf_sim::world::gen::GenParams;
use hf_sim::World;

fn people_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../godot/assets/people")
}

#[test]
fn sprite_layers_cover_all_states() {
    let dir = people_dir();
    assert!(
        dir.join("swaps.json").exists(),
        "people sheets present at {}",
        dir.display()
    );
    let mut w = World::generate(13, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.meta.founding_site = TilePos::new(30, 30, 2);
    let ids = w.found_party(6);
    let roles = [
        None,
        Some(Role::Forager),
        Some(Role::Builder),
        Some(Role::Farmer),
        Some(Role::Cook),
        Some(Role::Miner),
        Some(Role::Hauler),
    ];
    let mut seen_outfits = std::collections::BTreeSet::new();
    let mut checked = 0;
    for season_day in [0u64, 5, 10, 15] {
        w.meta.tick = season_day * TICKS_PER_DAY + 8 * 1200;
        w.weather = hf_sim::systems::weather::Weather::at(w.meta.seed, w.meta.tick);
        for (k, id) in ids.iter().enumerate() {
            let i = w.agents.iter().position(|a| a.id == *id).unwrap();
            // Ages: child, adult, elder, very old.
            let born_years_ago = [5u64, 30, 56, 66][k % 4];
            w.agents[i].born = w.meta.tick as i64 - (born_years_ago * TICKS_PER_YEAR) as i64;
            w.agents[i].role = roles[k % roles.len()];
            w.agents[i].injured_until = if k % 2 == 0 { w.meta.tick + 100 } else { 0 };
            let lk = look(&w, &w.agents[i]);
            assert!((lk.clothing as usize) < OUTFITS.len());
            assert!(
                lk.hair < 10
                    && lk.hair_colour < 8
                    && lk.clothing_colour < 12
                    && lk.skin < 6
                    && lk.body < 3
            );
            assert!(
                lk.equipment >= -1 && (lk.equipment as usize) < EQUIPMENT.len()
                    || lk.equipment == -1
            );
            let age = if lk.child { "child" } else { "adult" };
            for sheet in [
                format!(
                    "body_{}_{}.png",
                    ["slim", "average", "broad"][lk.body as usize],
                    age
                ),
                format!("clothing_{}.png", OUTFITS[lk.clothing as usize]),
                format!("hair_{:02}.png", lk.hair + 1),
                format!(
                    "overlay_bandage_{}_{}.png",
                    ["slim", "average", "broad"][lk.body as usize],
                    age
                ),
            ] {
                assert!(dir.join(&sheet).exists(), "sheet exists: {sheet}");
            }
            for e in EQUIPMENT {
                assert!(dir.join(format!("equipment_{e}.png")).exists());
            }
            assert_eq!(lk.injured, k % 2 == 0);
            assert_eq!(lk.child, born_years_ago < 15);
            seen_outfits.insert(lk.clothing);
            checked += 1;
        }
    }
    assert!(checked >= 24);
    assert!(
        seen_outfits.len() >= 4,
        "season and role change the outfit: {seen_outfits:?}"
    );
}

/// Issue 37: each job's outfit is the sheet of that name, and the client draws the same name
/// for the same index. A farmer in summer wore the robe because VEST pointed at index 6.
#[test]
fn each_role_wears_its_named_outfit() {
    use hf_sim::time::DAYS_PER_SEASON;
    let mut w = World::generate(13, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.meta.founding_site = TilePos::new(30, 30, 2);
    let ids = w.found_party(1);
    // Midday on the first day of summer, warm, at home: only the role picks the outfit.
    w.meta.tick = DAYS_PER_SEASON * TICKS_PER_DAY + 12 * 1200;
    w.weather = hf_sim::systems::weather::Weather::at(w.meta.seed, w.meta.tick);
    w.weather.temperature_c = 20.0;
    let i = w.agents.iter().position(|a| a.id == ids[0]).unwrap();
    w.agents[i].born = w.meta.tick as i64 - (30 * TICKS_PER_YEAR) as i64;
    w.agents[i].caravan = None;
    for (role, want) in [
        (Role::Farmer, "vest"),
        (Role::Cook, "apron"),
        (Role::Healer, "apron"),
        (Role::Builder, "workshirt"),
        (Role::Miner, "workshirt"),
        (Role::Forager, "tunic"),
        (Role::Hauler, "tunic"),
    ] {
        w.agents[i].role = Some(role);
        let lk = look(&w, &w.agents[i]);
        assert_eq!(
            OUTFITS[lk.clothing as usize], want,
            "{role:?} in summer wears the {want}"
        );
    }
    // The client (world_view.gd OUTFIT) and the art generator (people.py OUTFITS) index the
    // same names in the same order, so the index the bridge sends draws the named sheet.
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    for (file, prefix) in [
        ("godot/scripts/world_view.gd", "const OUTFIT := ["),
        ("tools/art/people.py", "OUTFITS = ["),
    ] {
        let src = std::fs::read_to_string(root.join(file)).unwrap();
        let line = src
            .lines()
            .find(|l| l.starts_with(prefix))
            .unwrap_or_else(|| panic!("{file} has a line starting {prefix}"));
        let names: Vec<&str> = line[prefix.len()..line.rfind(']').unwrap()]
            .split(',')
            .map(|s| s.trim().trim_matches(|c| c == '"' || c == '\''))
            .collect();
        assert_eq!(
            names, OUTFITS,
            "{file} outfit order matches look.rs OUTFITS"
        );
    }
}

/// PNG width and height from the header (bytes 16..24), without an image library.
fn png_size(p: &std::path::Path) -> (u32, u32) {
    let b = std::fs::read(p).unwrap();
    let w = u32::from_be_bytes([b[16], b[17], b[18], b[19]]);
    let h = u32::from_be_bytes([b[20], b[21], b[22], b[23]]);
    (w, h)
}

#[test]
fn people_sheets_carry_the_action_frames_and_every_plan() {
    // #485: the original 34 columns plus talk1-3, eat1-3, sleep2-3, tend1-3 and drink1-3, in every
    // build, age and body plan.
    let dir = people_dir();
    for (plan, prefix, cell) in [
        ("", "", 48),
        ("small_biped", "small_biped_", 48),
        ("digitigrade_tail", "digitigrade_tail_", 48),
        ("winged", "winged_", 48),
        ("four_armed", "four_armed_", 48),
        ("amphibious", "amphibious_", 48),
        ("taur", "taur_", 64),
        ("huge_biped", "huge_biped_", 96),
        ("serpentine", "serpentine_", 96),
    ] {
        for build in ["slim", "average", "broad"] {
            for age in ["adult", "child"] {
                let f = dir.join(format!("body_{prefix}{build}_{age}.png"));
                assert!(f.exists(), "{}", f.display());
                assert_eq!(png_size(&f), (48 * cell, 4 * cell), "{}", f.display());
                if !plan.is_empty() {
                    for layer in ["clothing_tunic", "hair_01", "equipment_spear"] {
                        let g = dir.join(format!("{layer}_{prefix}{build}_{age}.png"));
                        assert!(g.exists(), "{}", g.display());
                    }
                }
            }
        }
    }
    let swaps: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(dir.join("swaps.json")).unwrap()).unwrap();
    let cols = swaps["layout"]["columns"].as_array().unwrap();
    assert_eq!(cols.len(), 48);
    assert_eq!(cols[28], "strike1");
    assert_eq!(cols[33], "lying");
    assert_eq!(cols[34], "talk1");
    assert_eq!(cols[36], "talk3");
    assert_eq!(cols[37], "eat1");
    assert_eq!(cols[39], "eat3");
    assert_eq!(cols[26], "sleep");
    assert_eq!(cols[40], "sleep2");
    assert_eq!(cols[41], "sleep3");
    assert_eq!(cols[42], "tend1");
    assert_eq!(cols[45], "drink1");
    assert_eq!(cols[47], "drink3");
}

#[test]
fn every_species_skin_is_drawn() {
    // P3: species.rs SKIN_NAMES is swaps.json's skin order, and every species' tones are in it.
    let swaps: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(people_dir().join("swaps.json")).unwrap())
            .unwrap();
    let names: Vec<&str> = swaps["skin"]["names"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert_eq!(names, hf_sim::species::SKIN_NAMES.to_vec());
    for sp in hf_sim::species::library() {
        for build in ["slim", "average", "broad"] {
            for age in ["adult", "child"] {
                let body = if sp.art.body == "human" {
                    format!("body_{build}_{age}.png")
                } else {
                    format!("body_{}_{build}_{age}.png", sp.art.body)
                };
                assert!(people_dir().join(&body).exists(), "{body}");
                let cloth = if sp.plan == "biped" {
                    "clothing_tunic.png".to_string()
                } else {
                    format!("clothing_tunic_{}_{build}_{age}.png", sp.plan)
                };
                assert!(people_dir().join(&cloth).exists(), "{cloth}");
            }
        }
    }
}

#[test]
fn an_eater_does_not_hold_a_spear_to_their_face() {
    let mut w = World::opening_world(485);
    w.agents[0].weapon = Some(hf_sim::ids::ItemId(1));
    assert_eq!(look(&w, &w.agents[0]).equipment, 7);
    w.agents[0].activity = hf_sim::mind::Activity::Eating {
        item: hf_sim::ids::ItemId(2),
        ticks_left: 20,
        binge: None,
    };
    assert_eq!(look(&w, &w.agents[0]).equipment, -1);
}
