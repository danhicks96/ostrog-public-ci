//! #286: the valley's flora by species. Each kind's data, fruit in its season, look-alikes that
//! sicken whoever does not know them, crops to their ground, and dye that colours clothes.

use hf_sim::agents::Species;
use hf_sim::ids::TilePos;
use hf_sim::items::{Item, ItemKind, ItemPlace};
use hf_sim::plants::{Gives, Plant, PlantKind};
use hf_sim::time::{Season, TICKS_PER_DAY, TICKS_PER_HOUR};
use hf_sim::world::gen::biome::Biome;
use hf_sim::world::gen::GenParams;
use hf_sim::world::{Material, Tile};
use hf_sim::World;

/// Every kind added for #286 has a label, a line of prose, a growth time, and a use in some
/// system: food, a crop, fibre, dye, medicine, wood, grazing, or a look-alike's harm.
#[test]
fn every_new_kind_has_its_data_and_a_use() {
    let first = PlantKind::ALL
        .iter()
        .position(|k| *k == PlantKind::Alder)
        .unwrap();
    let new = &PlantKind::ALL[first..];
    assert!((15..=30).contains(&new.len()), "{} new kinds", new.len());
    for k in new {
        let l = k.lore();
        assert!(
            !l.label.is_empty() && !l.prose.is_empty(),
            "{k:?} has words"
        );
        assert!(l.days > 0.0 && l.days < 100.0, "{k:?} grows");
        assert!(l.climate.0 < l.climate.1, "{k:?} has a warmth range");
        let used = l.gives != Gives::Nothing || l.wood.is_some() || l.grazed || k.is_weed();
        assert!(used, "{k:?} matters to some system");
        if k.bears_fruit() {
            assert!(l.bears != 0, "{k:?} bears in some season");
        }
    }
    // Each look-alike resembles a real food.
    for k in new {
        if let Some((like, harm)) = k.look_alike() {
            assert!(like.yields_food(), "{k:?} looks like a food");
            assert!(harm > 0.0 && harm < 1.0);
            assert!(!k.yields_food());
        }
    }
    // Each new tree its own wood.
    for k in [
        PlantKind::Alder,
        PlantKind::Ash,
        PlantKind::Yew,
        PlantKind::Willow,
        PlantKind::CrabApple,
    ] {
        let wood = hf_sim::world::material::wood_of(k, 0);
        assert!(k.is_tree() && k.blocks_movement());
        assert!(![
            Material::OAK,
            Material::BIRCH,
            Material::PINE,
            Material::WOOD
        ]
        .contains(&wood));
    }
}

/// Fruit comes only in the seasons a kind bears in, and what is left falls after.
#[test]
fn hazelnuts_come_in_autumn() {
    let mut p = Plant::new(TilePos::new(0, 0, 0), PlantKind::Hazel, 1.0, 0);
    p.fruit = 0.0;
    for _ in 0..TICKS_PER_DAY / 60 {
        p.grow(60, 1.0, Season::Summer);
    }
    assert_eq!(p.fruit, 0.0, "no nuts in summer");
    for _ in 0..TICKS_PER_DAY * 3 / 60 {
        p.grow(60, 0.5, Season::Autumn);
    }
    assert!(p.fruit > 0.3, "nuts in autumn: {}", p.fruit);
    for _ in 0..TICKS_PER_DAY * 2 / 60 {
        p.grow(60, 0.0, Season::Winter);
    }
    assert_eq!(p.fruit, 0.0, "fallen by winter");
    // The old kinds bear all year, as before.
    assert!(PlantKind::BerryBush.bears_in(Season::Winter));
}

fn alone_with(kind: PlantKind, herb_lore: bool) -> (World, usize, TilePos) {
    let mut w = World::generate(21, GenParams::small());
    w.flatten(4, 4, 56, 56, 1);
    w.plants.clear();
    w.items.clear();
    let who = w.spawn_agent(Species::Person, TilePos::new(20, 20, 2));
    let i = w.agents.iter().position(|a| a.id == who).unwrap();
    if herb_lore {
        let now = w.meta.tick;
        w.agents[i].techniques.learn(
            hf_sim::mind::Technique::HerbLore,
            hf_sim::mind::techniques::Learned::Past,
            0.5,
            now,
        );
    }
    w.agents[i].needs.hunger = 0.1;
    w.agents[i].needs.rest = 1.0;
    let spot = TilePos::new(22, 20, 2);
    w.plants.push(Plant::new(spot, kind, 1.0, 1));
    (w, i, spot)
}

/// design rule 3 (knowledge belongs to people): one who does not know a death cap from a
/// chanterelle eats it and sickens, and from then on leaves death caps; one who knows herb lore
/// never takes it.
#[test]
fn a_look_alike_sickens_whoever_does_not_know_it() {
    let (mut w, i, spot) = alone_with(PlantKind::DeathCap, false);
    w.run(TICKS_PER_HOUR * 2);
    let a = &w.agents[i];
    let ill = a
        .body
        .illness
        .as_ref()
        .expect("ate the death cap and sickened");
    assert_eq!(ill.kind, hf_sim::body::IllnessKind::Poisoned);
    assert!(a.shuns.contains(&PlantKind::DeathCap), "learned it");
    // Healed and hungry again beside a fresh one: they leave it.
    w.agents[i].body.illness = None;
    w.agents[i].needs.hunger = 0.1;
    w.plants.retain(|p| p.pos != spot);
    w.plants.push(Plant::new(spot, PlantKind::DeathCap, 1.0, 2));
    w.run(TICKS_PER_HOUR * 2);
    assert!(w.agents[i].body.illness.is_none(), "not taken twice");
    assert!(w.plants.iter().any(|p| p.pos == spot && p.fruit > 0.9));

    let (mut w, i, spot) = alone_with(PlantKind::DeathCap, true);
    w.run(TICKS_PER_HOUR * 2);
    assert!(
        w.agents[i].body.illness.is_none(),
        "herb lore knows a death cap"
    );
    assert!(w.plants.iter().any(|p| p.pos == spot && p.fruit > 0.9));
}

/// A normal person who sees a neighbour sicken from a plant leaves it after (#286): those
/// awake within sight learn it on the spot; the one who ate it and they pass it on by talk,
/// one person at a time.
#[test]
fn a_poisoning_is_learned_by_those_who_see_it_and_told_on() {
    let (mut w, i, _) = alone_with(PlantKind::Hemlock, false);
    let near = w.spawn_agent(Species::Person, TilePos::new(21, 22, 2));
    let far = w.spawn_agent(Species::Person, TilePos::new(50, 50, 2));
    for id in [near, far] {
        let j = w.agents.iter().position(|a| a.id == id).unwrap();
        w.agents[j].needs.hunger = 1.0;
        w.agents[j].needs.rest = 1.0;
    }
    w.run(TICKS_PER_HOUR * 2);
    assert!(w.agents[i].shuns.contains(&PlantKind::Hemlock), "ate it");
    let knows = |w: &World, id| {
        w.agents
            .iter()
            .find(|a| a.id == id)
            .unwrap()
            .shuns
            .contains(&PlantKind::Hemlock)
    };
    assert!(knows(&w, near), "saw it");
    assert!(!knows(&w, far), "not there, not told");
    // Told it in talk, by a stranger, to a listener of middling self-reliance, who believes them
    // (#487: a self-reliant listener may not; tests/social_learning.rs).
    let teller = w.agents[i].id;
    let fj = w.agents.iter().position(|a| a.id == far).unwrap();
    w.agents[fj].dispositions = Some(hf_sim::mind::Dispositions::default());
    w.last_events = vec![hf_sim::events::Event {
        id: hf_sim::ids::EventId(0),
        tick: w.meta.tick,
        at: None,
        severity: hf_sim::events::Severity::Minor,
        kind: hf_sim::events::EventKind::Talked {
            a: teller,
            b: far,
            warmth: 0.5,
        },
    }];
    hf_sim::social::observe(&mut w);
    assert!(knows(&w, far), "told of it");
}

/// The real food the look-alike copies is gathered and eaten, by the same person, as before.
#[test]
fn chanterelles_are_eaten() {
    let (mut w, i, spot) = alone_with(PlantKind::Chanterelle, true);
    let before = w.agents[i].needs.hunger;
    w.run(TICKS_PER_HOUR * 2);
    assert!(w.agents[i].needs.hunger > before, "ate the chanterelles");
    assert!(w.agents[i].body.illness.is_none());
    assert!(w.plants.iter().any(|p| p.pos == spot && p.fruit < 0.3));
}

/// Each crop to its own ground (#286): wheat on grassland, barley on heath, oats in the fen,
/// rye on the fell, peas in a watered meadow; the farmers sow what suits.
#[test]
fn crops_follow_their_ground() {
    use hf_sim::plants::best_crop;
    assert_eq!(best_crop(Biome::Grassland, false), PlantKind::Grain);
    assert_eq!(best_crop(Biome::Heath, false), PlantKind::Barley);
    assert_eq!(best_crop(Biome::Stony, false), PlantKind::Barley);
    assert_eq!(best_crop(Biome::Fen, true), PlantKind::Oats);
    assert_eq!(best_crop(Biome::Woodland, false), PlantKind::Oats);
    assert_eq!(best_crop(Biome::Fell, false), PlantKind::Rye);
    assert_eq!(best_crop(Biome::Snowfield, false), PlantKind::Rye);
    assert_eq!(best_crop(Biome::Meadow, true), PlantKind::Peas);
    assert_eq!(best_crop(Biome::Meadow, false), PlantKind::Grain);
    // In the sim: a tile of fell ground is sown with rye.
    let mut w = World::generate(21, GenParams::small());
    w.flatten(4, 4, 56, 56, 1);
    let at = w.grid.surface(30, 30);
    let c = w.grid.col(30, 30);
    w.grid.biome[c] = Biome::Fell;
    if !hf_sim::systems::plants::watered(&w, at) {
        assert_eq!(hf_sim::systems::plants::crop_for(&w, at), PlantKind::Rye);
    }
    w.grid.biome[c] = Biome::Heath;
    assert_eq!(hf_sim::systems::plants::crop_for(&w, at), PlantKind::Barley);
}

/// Crop growth responds to the material immediately below the plant, even within one biome
/// (#493): the biome is the broad ground family, while the surface material carries local soil
/// quality.
#[test]
fn crop_growth_responds_to_surface_material_within_same_biome() {
    fn growth_on(ground: Material) -> f32 {
        let mut w = World::generate(3461, GenParams::small());
        w.flatten(4, 4, 56, 56, 1);
        w.plants.clear();
        let at = w.grid.surface(30, 30);
        let col = w.grid.col(at.x, at.y);
        w.grid.biome[col] = Biome::Grassland;
        w.grid.set(at.offset(0, 0, -1), Tile::solid(ground));
        w.plants.push(Plant::new(at, PlantKind::Grain, 0.2, 0));
        hf_sim::systems::plants::step(&mut w);
        w.plants[0].growth
    }

    let fertile = growth_on(Material::MEADOW);
    let sandy = growth_on(Material::SAND);
    assert!(
        fertile > sandy,
        "same-biome crops should grow better on meadow than sand: {fertile} vs {sandy}"
    );
}

/// Rye stands a killing frost that takes the wheat beside it.
#[test]
fn rye_stands_the_frost() {
    let mut w = World::generate(3461, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.rebuild_caches();
    w.found_party(2);
    w.meta.autoplan = false;
    let mut wx = hf_sim::systems::weather::current(&w);
    wx.temperature_c = -12.0;
    w.weather = wx;
    let (a, b) = (w.grid.surface(35, 35), w.grid.surface(36, 35));
    w.communities[0].field_tiles.extend([a, b]);
    for (p, k) in [(a, PlantKind::Grain), (b, PlantKind::Rye)] {
        w.plants.push(Plant::new(p, k, 0.5, 0));
    }
    let day = (0..40u64)
        .find(|&d| {
            w.meta.tick = w.meta.epoch + d * TICKS_PER_DAY;
            w.calendar().season == Season::Autumn
        })
        .unwrap();
    w.meta.tick = w.meta.epoch + day * TICKS_PER_DAY + 4 * TICKS_PER_HOUR + 555;
    hf_sim::thermal::step(&mut w);
    let alive = |p: TilePos| w.plants.iter().any(|q| q.pos == p && q.alive);
    assert!(!alive(a), "the wheat froze");
    assert!(alive(b), "the rye stood");
}

/// Clothes dyed with woad wear blue (#286): the dye is the clothes' material and sets the
/// colour they are drawn in.
#[test]
fn woad_dyed_clothes_wear_blue() {
    let r = hf_sim::work::recipes::recipe(hf_sim::work::recipes::DYED_CLOTHES);
    assert_eq!(
        r.inputs[0].0,
        ItemKind::Dye,
        "the dye first, so it is the material"
    );
    assert_eq!(
        hf_sim::work::recipes::craft_of(hf_sim::work::recipes::DYED_CLOTHES),
        Some(hf_sim::mind::Technique::Weaving)
    );
    let mut w = World::generate(3461, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.rebuild_caches();
    let ids = w.found_party(1);
    let i = w.agents.iter().position(|a| a.id == ids[0]).unwrap();
    w.agents[i].clothes_wear = 0.1;
    w.items.retain(|it| it.kind != ItemKind::Clothes);
    let id = w.alloc_item_id();
    let home = w.communities[0].home;
    w.items.push(Item {
        id,
        kind: ItemKind::Clothes,
        count: 1,
        place: ItemPlace::Ground(home),
        quality: 0.5,
        material: Material::WOAD,
        story: None,
        rot: 0.0,
    });
    let day = w.meta.tick / TICKS_PER_DAY + 1;
    w.meta.tick = day * TICKS_PER_DAY + 5 * TICKS_PER_HOUR;
    hf_sim::systems::life::step(&mut w);
    assert!(w.agents[i].clothes_wear > 0.9, "put the new set on");
    assert_eq!(w.agents[i].appearance.clothing_colour, 9, "woad blue");
}

/// #486: a world draws a limited community from the existing wild catalogue, never an
/// undrawn kind; the original flora and survivor-sown crops remain available everywhere.
#[test]
fn each_world_draws_a_limited_flora_community() {
    use hf_sim::world::gen::flora::{FLORA_CANDIDATES, FLORA_DRAW_COUNT};
    let mut communities = std::collections::BTreeSet::new();
    for seed in 1..=8u64 {
        let a = World::generate(seed, GenParams::small());
        let b = World::generate(seed, GenParams::small());
        let draw = a
            .meta
            .flora_draw
            .as_ref()
            .expect("the world records its draw");
        assert_eq!(
            Some(draw),
            b.meta.flora_draw.as_ref(),
            "seed {seed} repeats"
        );
        assert_eq!(draw.drawn.len(), FLORA_DRAW_COUNT, "seed {seed} is limited");
        let unique: std::collections::BTreeSet<_> = draw.drawn.iter().copied().collect();
        assert_eq!(unique.len(), FLORA_DRAW_COUNT, "seed {seed} has no repeats");
        for k in &draw.drawn {
            assert!(FLORA_CANDIDATES.contains(k), "seed {seed} draws wild {k:?}");
        }
        for k in FLORA_CANDIDATES {
            assert_eq!(draw.allows(k), draw.drawn.contains(&k), "seed {seed} {k:?}");
        }
        for k in [
            PlantKind::Tree,
            PlantKind::BerryBush,
            PlantKind::Mushroom,
            PlantKind::Grain,
            PlantKind::Flax,
        ] {
            assert!(draw.allows(k), "seed {seed} keeps core/crop {k:?}");
        }
        for p in &a.plants {
            assert!(
                draw.allows(p.kind),
                "seed {seed} placed undrawn {:?} at {:?}",
                p.kind,
                p.pos
            );
        }
        communities.insert(draw.drawn.clone());
    }
    assert!(
        communities.len() >= 4,
        "eight seeds produce meaningfully different communities: {communities:?}"
    );
}

fn flora_report(w: &World) -> Vec<String> {
    hf_sim::world::gen::biome::describe(
        w.meta.seed,
        &w.meta.params,
        &w.grid,
        &w.plants,
        w.meta.flora_draw.as_ref(),
    )
}

fn flora_summary(lines: Vec<String>) -> Vec<String> {
    lines
        .into_iter()
        .filter(|l| l.starts_with("flora draw:") || l.starts_with("flora not drawn:"))
        .collect()
}

/// The report names both sides of the draw, and a zero census for an undrawn kind says why
/// rather than blaming its habitat.
#[test]
fn flora_report_distinguishes_not_drawn_from_no_habitat() {
    let w = World::generate(11, GenParams::small());
    let draw = w.meta.flora_draw.as_ref().unwrap();
    let lines = flora_report(&w);
    let drawn = lines.iter().find(|l| l.starts_with("flora draw:")).unwrap();
    let not_drawn = lines
        .iter()
        .find(|l| l.starts_with("flora not drawn:"))
        .unwrap();
    for k in hf_sim::world::gen::flora::FLORA_CANDIDATES {
        let summary = if draw.allows(k) { drawn } else { not_drawn };
        assert!(summary.contains(k.label()), "{k:?} is named once");
        if !draw.allows(k) && !w.plants.iter().any(|p| p.kind == k) {
            assert!(
                lines
                    .iter()
                    .any(|l| l.contains(k.label()) && l.contains("not drawn for this world")),
                "{k:?} has an explicit absence reason"
            );
        }
    }
}

/// A changed record, not a fresh seed draw, drives the report; saving and loading preserves it.
#[test]
fn flora_report_uses_the_saved_record() {
    let mut w = World::generate(13, GenParams::small());
    let removed = w.meta.flora_draw.as_mut().unwrap().drawn.remove(0);
    let lines = flora_report(&w);
    let drawn = lines.iter().find(|l| l.starts_with("flora draw:")).unwrap();
    let not_drawn = lines
        .iter()
        .find(|l| l.starts_with("flora not drawn:"))
        .unwrap();
    assert!(!drawn.contains(removed.label()));
    assert!(not_drawn.contains(removed.label()));
    let restored = hf_sim::save::snapshot::Snapshot::capture(&w).restore();
    assert_eq!(restored.meta.flora_draw, w.meta.flora_draw);
    assert_eq!(flora_report(&restored), lines);
}

/// A legacy world has no draw to report. Do not invent one from its seed or mark kinds undrawn.
#[test]
fn legacy_flora_draw_is_reported_as_unknown() {
    let mut w = World::generate(17, GenParams::small());
    w.meta.flora_draw = None;
    let restored = hf_sim::save::snapshot::Snapshot::capture(&w).restore();
    assert_eq!(restored.meta.flora_draw, None);
    let lines = flora_report(&restored);
    assert!(lines
        .iter()
        .any(|l| l == "flora draw: not recorded in this world (legacy save)"));
    assert!(!lines.iter().any(|l| l.starts_with("flora not drawn:")));
    assert!(!lines.iter().any(|l| l.contains("not drawn for this world")));
}

/// What the setup preview shows is exactly the draw summary from the generated world's record.
#[test]
fn setup_preview_shows_the_worlds_flora_draw() {
    let seed = 3;
    let preview = hf_sim::setup::preview(seed, None, None);
    let w = World::from_setup(&hf_sim::setup::WorldSetup::new(seed));
    assert_eq!(
        w.meta.flora_draw,
        Some(hf_sim::world::gen::flora::FloraDraw::of_seed(seed))
    );
    assert_eq!(
        flora_summary(preview.lines),
        flora_summary(flora_report(&w))
    );
}

/// A seed can admit a species that a selected site's ground does not support.
#[test]
fn landing_plant_examples_match_the_selected_sites_surface() {
    use hf_sim::realm::{self, RealmParams};
    use hf_sim::setup::{preview_in, WorldSetup};

    let seed = 7;
    let map = realm::generate(seed, &RealmParams::default());
    for region in [16423, 7846, 9368, 28317] {
        let mut preview = preview_in(seed, &map, None, Some(region));
        let examples = preview.surface_plant_examples();
        let mut setup = WorldSetup::new(seed);
        setup.region = Some(region);
        setup.history_years = 0;
        let world = World::from_setup(&setup);
        assert_eq!(examples.len(), 5);
        for (kind, count) in examples {
            let live = world
                .plants
                .iter()
                .filter(|p| {
                    p.kind == kind && p.alive && p.pos.z == world.grid.surface(p.pos.x, p.pos.y).z
                })
                .count();
            assert!(count > 0);
            assert_eq!(count, live, "region {region}: {kind:?}");
        }

        // Keep the world draw and replace only the surface. Dead wood, empty ground
        // and absent world species must not appear in the examples.
        use PlantKind::{FallenLog, Shrub, Willow, Yarrow};
        preview.plants = vec![0, FallenLog as u8 + 1];
        assert!(preview.surface_plant_examples().is_empty());
        for (kind, n) in [(Yarrow, 1), (Shrub, 4), (Willow, 9)] {
            preview
                .plants
                .extend(std::iter::repeat_n(kind as u8 + 1, n));
        }
        assert_eq!(
            preview.surface_plant_examples(),
            [(Willow, 9), (Shrub, 4), (Yarrow, 1)]
        );
    }
}

/// Multi-seed realized-density control: removing kinds from the bag changes which plant occupies
/// a suitable low-flora site, never how many suitable sites are occupied. The full-catalogue
/// control uses the same generated grid, climate, water and purpose-tagged placement stream.
#[test]
fn limited_flora_preserves_realized_density_against_full_catalogue() {
    use hf_sim::world::gen::flora::{apply, FloraDraw, FLORA_CANDIDATES};
    use hf_sim::world::gen::{biome, caverns, caves, heightmap, hydrology, ores, ramps, strata};
    for seed in 1..=8u64 {
        let params = GenParams::small();
        let mut grid =
            hf_sim::world::WorldGrid::new(params.width, params.height, params.z_min, params.z_max);
        heightmap::apply(seed, &params, &mut grid);
        strata::apply(seed, &params, &mut grid);
        let character = biome::Character::resolve(seed, &params);
        let water = hydrology::apply(seed, &params, &mut grid, &character);
        caves::apply(seed, &params, &mut grid);
        caverns::apply(seed, &params, &mut grid);
        ores::apply(seed, &params, &mut grid);
        let climate = biome::climate(seed, &params, &grid, &water);
        biome::surface(seed, &mut grid, &water, &climate);
        grid.biome = climate.biome.clone();
        hydrology::deepen(&mut grid, &water, &climate.biome);
        ramps::apply(&params, &mut grid, &water);
        let limited = apply(
            seed,
            &params,
            &grid,
            &water,
            &climate,
            &FloraDraw::of_seed(seed),
        );
        let all = FloraDraw {
            drawn: FLORA_CANDIDATES.to_vec(),
        };
        let control = apply(seed, &params, &grid, &water, &climate, &all);
        let sites = |plants: &[Plant]| plants.iter().map(|p| p.pos).collect::<Vec<_>>();
        assert_eq!(
            sites(&limited),
            sites(&control),
            "seed {seed}: limited draw changed realized plant-site density"
        );
    }
}

/// Wild plants read the site as crops do: a berry bush grows fastest
/// by water on meadow, slower in a crowd, slower still under a mature oak, and slowest dry on
/// scree.
#[test]
fn wild_growth_reads_the_ground() {
    use hf_sim::world::{Shape, TileFlags};
    #[derive(Clone, Copy, PartialEq)]
    enum Site {
        WetMeadow,
        Scree,
        UnderOak,
        Crowd,
        Logs,
    }
    fn growth_of(kind: PlantKind, site: Site) -> f32 {
        let mut w = World::generate(3461, GenParams::small());
        w.flatten(4, 4, 56, 56, 1);
        w.plants.clear();
        w.weather.growth_factor = 1.0;
        let at = w.grid.surface(30, 30);
        let col = w.grid.col(at.x, at.y);
        w.grid.biome[col] = Biome::Grassland;
        let ground = if site == Site::Scree {
            Material::SCREE
        } else {
            Material::MEADOW
        };
        w.grid.set(at.offset(0, 0, -1), Tile::solid(ground));
        w.plants.push(Plant::new(at, kind, 0.1, 0));
        match site {
            Site::WetMeadow => {
                w.grid.set(
                    at.offset(2, 0, -1),
                    Tile {
                        material: Material::WATER,
                        shape: Shape::Open,
                        flags: TileFlags(TileFlags::WATER),
                    },
                );
            }
            Site::UnderOak => {
                w.plants
                    .push(Plant::new(at.offset(1, 0, 0), PlantKind::Tree, 1.0, 0))
            }
            Site::Crowd => {
                for (dx, dy) in [(0, 1), (0, -1), (-1, 0), (-1, 1)] {
                    w.plants
                        .push(Plant::new(at.offset(dx, dy, 0), PlantKind::Reed, 0.1, 0));
                }
            }
            Site::Logs => {
                for (dx, dy) in [(0, 1), (0, -1), (-1, 0), (-1, 1)] {
                    w.plants.push(Plant::new(
                        at.offset(dx, dy, 0),
                        PlantKind::FallenLog,
                        1.0,
                        0,
                    ));
                }
            }
            Site::Scree => {}
        }
        let before = w.plants[0].growth;
        for stripe in 0..(5 * TICKS_PER_DAY / 60) {
            w.meta.tick = stripe * 60;
            hf_sim::systems::plants::step(&mut w);
        }
        w.plants[0].growth - before
    }
    let growth_at = |site| growth_of(PlantKind::BerryBush, site);
    // Dead wood is no crowd: logs around a bush leave it as the bare meadow does.
    let bare = growth_of(PlantKind::BerryBush, Site::Logs);
    // A wild carrot reads the ground too: dry scree is slower than dry meadow.
    let root_scree = growth_of(PlantKind::Root, Site::Scree);
    let root_logs = growth_of(PlantKind::Root, Site::Logs);
    assert!(
        root_logs > root_scree * 1.5,
        "root {root_logs} vs {root_scree}"
    );
    let wet = growth_at(Site::WetMeadow);
    let crowd = growth_at(Site::Crowd);
    let oak = growth_at(Site::UnderOak);
    let scree = growth_at(Site::Scree);
    assert!(
        wet > crowd && crowd > oak && oak > scree,
        "wet {wet}, crowd {crowd}, oak {oak}, scree {scree}"
    );
    assert!(bare > crowd * 1.2, "logs {bare} vs crowd {crowd}");
    assert!(wet >= scree * 1.5, "wet {wet} vs scree {scree}");
}

/// The chunk-wise water search answers exactly what a tile-by-tile scan of the same window does:
/// water within three tiles on the plant's level or the one below.
#[test]
fn water_near_matches_a_tile_by_tile_scan() {
    let w = World::generate(1671, GenParams::small());
    let mut wet = 0;
    for y in (0..w.grid.height).step_by(3) {
        for x in (0..w.grid.width).step_by(3) {
            let p = TilePos::new(x, y, w.grid.ground_z(x, y) + 1);
            let mut brute = false;
            for dz in [0, -1] {
                for dy in -3..=3 {
                    for dx in -3..=3 {
                        let q = p.offset(dx, dy, dz);
                        brute |= w.grid.in_bounds(q) && w.grid.get(q).is_water();
                    }
                }
            }
            assert_eq!(w.grid.water_near(p, 3), brute, "at {p:?}");
            wet += usize::from(brute);
        }
    }
    assert!(wet > 0, "the test world has some water to find");
}
