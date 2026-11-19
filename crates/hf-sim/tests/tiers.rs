//! #274: a harder stone than any cut before is a moment on record; walls of harder stone last.

use hf_sim::events::{EventKind, Severity};
use hf_sim::ids::TilePos;
use hf_sim::world::gen::GenParams;
use hf_sim::world::material::MaterialKind;
use hf_sim::World;

fn cut(w: &mut World, agent: hf_sim::ids::AgentId, rock: hf_sim::world::Material) {
    let tick = w.meta.tick;
    let id = w.events.push(
        tick,
        Some(TilePos::new(10, 10, 0)),
        Severity::Minor,
        EventKind::Mined {
            agent,
            at: TilePos::new(10, 10, 0),
            found: None,
            rock: Some(rock),
        },
    );
    let e = w
        .events
        .pending
        .iter()
        .find(|e| e.id == id)
        .unwrap()
        .clone();
    w.last_events = vec![e];
    w.events.pending.clear();
    hf_sim::tiers::observe(w);
}

#[test]
fn a_harder_stone_first_cut_is_on_record_once() {
    let mut w = World::generate(2741, GenParams::small());
    let ids = w.found_party(1);
    let mut stones: Vec<(f32, hf_sim::world::Material)> = (0..w.grid.materials.defs.len())
        .map(|k| hf_sim::world::Material(k as u16))
        .filter(|m| w.grid.materials.get(*m).kind == MaterialKind::Stone)
        .map(|m| (w.grid.materials.get(m).hardness, m))
        .collect();
    stones.sort_by(|a, b| a.0.total_cmp(&b.0));
    let (soft, hard) = (stones[0].1, stones.last().unwrap().1);
    assert!(w.grid.materials.get(hard).hardness > w.grid.materials.get(soft).hardness);
    cut(&mut w, ids[0], soft);
    cut(&mut w, ids[0], hard);
    let first = w
        .events
        .pending
        .iter()
        .filter(|e| matches!(e.kind, EventKind::NewStone { .. }))
        .count();
    assert_eq!(first, 1, "the harder stone, first cut");
    cut(&mut w, ids[0], hard);
    assert_eq!(
        w.events
            .pending
            .iter()
            .filter(|e| matches!(e.kind, EventKind::NewStone { .. }))
            .count(),
        0,
        "not twice"
    );
}

use hf_sim::items::ItemKind;
use hf_sim::world::{Material, Tile};

fn find(w: &World, m: Material) -> TilePos {
    let g = &w.grid;
    for z in (g.z_min..=g.z_max).rev() {
        for y in 0..g.height {
            for x in 0..g.width {
                let p = TilePos::new(x, y, z);
                if g.get(p).material == m {
                    return p;
                }
            }
        }
    }
    panic!("no {m:?}");
}

/// #274: harder rock takes longer to cut, and harder stone longer to dress and to lay, by the
/// hardness in the material table.
#[test]
fn harder_stone_takes_longer_to_cut_dress_and_lay() {
    use hf_sim::work::designation::{times, DesignationKind as D};
    let mut w = World::generate(2742, GenParams::default());
    w.found_party(1);
    let t = w.grid.materials.clone();
    let (soft, hard) = (find(&w, Material::SANDSTONE), find(&w, Material::GRANITE));
    let a = hf_sim::systems::settlement::make(&mut w, D::Mine, soft, 5);
    let b = hf_sim::systems::settlement::make(&mut w, D::Mine, hard, 5);
    let want = |m: Material| (times::MINE as f32 * t.get(m).hardness).round() as u32;
    assert_eq!(a.work_total, want(Material::SANDSTONE));
    assert_eq!(b.work_total, want(Material::GRANITE));
    assert!(b.work_total > a.work_total);
    // The first stone brought to a dressing or a wall sets how long it takes.
    let dress = D::Craft(hf_sim::work::recipes::DRESS_STONE);
    for (kind, m) in [
        (dress, Material::BASALT),
        (D::BuildWall(Material::STONE_BLOCK), Material::GRANITE),
    ] {
        let d = hf_sim::systems::settlement::make(&mut w, kind, soft, 5);
        let base = d.work_total;
        w.designations.push(d);
        let ix = w.designations.len() - 1;
        hf_sim::tiers::worked(&mut w, ix, m);
        let got = w.designations[ix].work_total;
        assert_eq!(
            got,
            (base as f32 * t.get(m).hardness).round() as u32,
            "{kind:?}"
        );
        assert_eq!(w.designations[ix].work_left, got);
    }
    // Timber is not stone: no change.
    let d = hf_sim::systems::settlement::make(&mut w, D::BuildWall(Material::WOOD), soft, 5);
    let base = d.work_total;
    w.designations.push(d);
    let ix = w.designations.len() - 1;
    hf_sim::tiers::worked(&mut w, ix, Material::OAK);
    assert_eq!(w.designations[ix].work_total, base);
}

/// #274: a bed in a house walled with fine stone (marble) is a better night than one in a
/// granite house.
#[test]
fn a_house_of_fine_stone_is_a_better_night() {
    for (stone, glad) in [(Material::MARBLE, true), (Material::GRANITE, false)] {
        let mut w = World::generate(2743, GenParams::small());
        w.flatten(2, 2, 60, 60, 1);
        w.rebuild_caches();
        let ids = w.found_party(1);
        let o = TilePos::new(20, 20, 2);
        w.communities[0].house_sites.push(o);
        let plan = hf_sim::houses::plan(&w, o);
        for (q, _) in hf_sim::houses::walls(&plan.floor) {
            w.grid.set(q, Tile::solid(Material::STONE_BLOCK));
            w.built_of.insert(q, stone);
        }
        let i = w.agents.iter().position(|a| a.id == ids[0]).unwrap();
        w.agents[i].pos = plan.floor[0];
        assert_eq!(
            hf_sim::houses::fine_stone(&w, plan.floor[0]).is_some(),
            glad
        );
        let tick = w.meta.tick;
        let id = w.events.push(
            tick,
            Some(plan.floor[0]),
            Severity::Minor,
            EventKind::Slept {
                agent: ids[0],
                hours: 8.0,
                in_bed: true,
            },
        );
        let e = w
            .events
            .pending
            .iter()
            .find(|e| e.id == id)
            .unwrap()
            .clone();
        w.last_events = vec![e];
        hf_sim::systems::strain::observe(&mut w);
        assert_eq!(
            w.agents[i]
                .thoughts
                .has(hf_sim::mind::ThoughtKind::FineHouse),
            glad,
            "{stone:?}"
        );
    }
}

/// #274: stone, ore and gems are priced by what they are, and a place that has one stone still
/// lacks another: a settlement glutted with granite blocks pays little for granite and well
/// for marble.
#[test]
fn a_neighbour_pays_for_the_stone_it_lacks() {
    let w0 = World::generate(2744, GenParams::small());
    let mut w = w0;
    w.found_party(1);
    let k = w
        .overworld
        .sites
        .iter()
        .position(|s| s.people > 0)
        .expect("a peopled place");
    {
        let s = &mut w.overworld.sites[k];
        s.stores.insert(ItemKind::StoneBlock, 200.0);
        s.store_material
            .insert(ItemKind::StoneBlock, Material::GRANITE);
        s.stores.insert(ItemKind::Gem, 20.0);
        s.store_material.insert(ItemKind::Gem, Material::GARNET);
    }
    let s = w.overworld.sites[k].clone();
    let price = |kind, m| hf_sim::trade::price_at(&w, &s, kind, m);
    let granite = price(ItemKind::StoneBlock, Material::GRANITE);
    let marble = price(ItemKind::StoneBlock, Material::MARBLE);
    assert!(marble > granite * 5.0, "marble {marble}, granite {granite}");
    // Gems: a stone it has none of is worth more to it than one it holds.
    let garnet = price(ItemKind::Gem, Material::GARNET);
    let amethyst = price(ItemKind::Gem, Material::AMETHYST);
    assert!(
        amethyst > garnet * 2.0,
        "amethyst {amethyst}, garnet {garnet}"
    );
    // Ore by its metal: gold ore is worth more than iron ore.
    let gold = price(ItemKind::Ore, Material::GOLD_ORE);
    let iron = price(ItemKind::Ore, Material::IRON_ORE);
    assert!(gold > iron * 3.0, "gold ore {gold}, iron ore {iron}");
    // A caravan selling marble there is paid more than one selling granite.
    let home = w.communities[0].home;
    let paid = |m| {
        let (took, got) =
            hf_sim::trade::quote_wanting(&w, &s, home, 10, ItemKind::StoneBlock, m, 6, 400, None);
        (took, got.iter().map(|g| g.2).sum::<u32>())
    };
    let (gt, gg) = paid(Material::GRANITE);
    let (mt, mg) = paid(Material::MARBLE);
    assert_eq!(gt, 0, "glutted with granite, it takes none");
    assert!(
        mt > 0 && mg > gg,
        "marble took {mt} for {mg}, granite {gt} for {gg}"
    );
}

/// #274: the deep is worth reaching: its rock is the hardest stone, so its walls stand a siege
/// longest; gabbro and gneiss are fine stone; and a stone tool of the deep's rock takes a
/// better edge than any of the upper or middle strata's.
#[test]
fn the_deep_gives_what_the_surface_cannot() {
    let w = World::generate(2745, GenParams::small());
    let t = &w.grid.materials;
    let upper = [Material::LIMESTONE, Material::SANDSTONE];
    let middle = [Material::GRANITE, Material::SLATE, Material::MARBLE];
    let deep = [Material::BASALT, Material::GABBRO, Material::GNEISS];
    let best_above = upper
        .iter()
        .chain(&middle)
        .map(|m| hf_sim::tiers::stone_edge(&w, *m))
        .fold(0.0, f32::max);
    for d in deep {
        for a in upper.iter().chain(&middle) {
            assert!(t.get(d).hardness > t.get(*a).hardness, "{d:?} over {a:?}");
        }
        assert!(hf_sim::tiers::stone_edge(&w, d) > best_above, "{d:?}");
    }
    assert!(hf_sim::tiers::fine(&w, Material::GABBRO) && hf_sim::tiers::fine(&w, Material::GNEISS));
    assert!(!hf_sim::tiers::fine(&w, Material::GRANITE));
    // A copper tool's edge (0.6) is the bar the deep's stone reaches.
    assert!(hf_sim::tiers::stone_edge(&w, Material::BASALT) >= 0.6);
    assert!(hf_sim::tiers::stone_edge(&w, Material::SANDSTONE) < 0.45);
}
