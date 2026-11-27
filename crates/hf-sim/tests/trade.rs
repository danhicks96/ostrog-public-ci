//! C36 gates: the world's settlements hold real stores that their living makes and time uses
//! up; what they pay follows how short they are; a caravan's trade moves goods out of one
//! ledger and into the other without making or losing any; the valley trades only where it
//! has been.

use hf_sim::chronicle::Chronicle;
use hf_sim::events::EventKind;
use hf_sim::expedition::{CaravanState, Purpose};
use hf_sim::ids::TilePos;
use hf_sim::items::{ItemKind, ItemPlace};
use hf_sim::systems::expedition::form_caravan;
use hf_sim::time::TICKS_PER_DAY;
use hf_sim::trade;
use hf_sim::world::gen::GenParams;
use hf_sim::world::Material;
use hf_sim::{Session, World};

fn founded(seed: u64) -> World {
    let mut w = World::generate(seed, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.meta.tick = hf_sim::time::TICKS_PER_HOUR * 8;
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.rebuild_caches();
    w.found_party(10);
    w.ensure_nav();
    w
}

/// The index of a peopled settlement.
fn peopled(w: &World) -> usize {
    w.overworld
        .sites
        .iter()
        .position(|s| s.people > 0)
        .expect("a peopled settlement")
}

/// The indices of every peopled settlement.
fn peopled_sites(w: &World) -> Vec<usize> {
    w.overworld
        .sites
        .iter()
        .enumerate()
        .filter(|(_, s)| s.people > 0)
        .map(|(i, _)| i)
        .collect()
}

#[test]
fn prices_follow_scarcity() {
    let mut w = founded(3601);
    let k = peopled(&w);
    let kind = ItemKind::Log;
    w.overworld.sites[k].stores.insert(kind, 0.0);
    let short = trade::price_at(&w, &w.overworld.sites[k], kind, Material::OAK);
    w.overworld.sites[k].stores.insert(kind, 500.0);
    let glut = trade::price_at(&w, &w.overworld.sites[k], kind, Material::OAK);
    assert!(short > glut * 3.0, "short {short} vs glutted {glut}");
    // A better material is worth more at the same stock.
    let oak = trade::price_at(&w, &w.overworld.sites[k], ItemKind::Bar, Material::BRONZE);
    let gold = trade::price_at(&w, &w.overworld.sites[k], ItemKind::Bar, Material::GOLD);
    assert!(gold > oak, "gold {gold} vs bronze {oak}");
    // At home: a store full of something makes the valley want it less.
    let home = w.communities[0].home;
    let before = trade::price_home(&w, home, 10, ItemKind::Cloth, Material::AIR);
    w.drop_item(ItemKind::Cloth, 60, home.offset(2, 0, 0));
    let after = trade::price_home(&w, home, 10, ItemKind::Cloth, Material::AIR);
    assert!(before > after, "home price {before} then {after}");
}

#[test]
fn stores_are_made_and_rot() {
    let mut w = founded(3602);
    let k = peopled(&w);
    w.overworld.sites[k].stores.clear();
    trade::daily(&mut w);
    let s = w.overworld.sites[k].clone();
    assert!(!s.stores.is_empty(), "a peopled place makes something");
    for (kind, _, _) in trade::produces(&w, &s) {
        assert!(s.stores.contains_key(&kind), "{kind:?} made");
        assert!(
            s.store_material.contains_key(&kind),
            "{kind:?} has a material"
        );
    }
    // With nothing made, food goes faster than anything else.
    w.overworld.sites[k].people = 0;
    w.overworld.sites[k].stores.clear();
    w.overworld.sites[k]
        .stores
        .insert(ItemKind::GrainSheaf, 100.0);
    w.overworld.sites[k]
        .stores
        .insert(ItemKind::StoneBlock, 100.0);
    let made: Vec<ItemKind> = trade::produces(&w, &w.overworld.sites[k].clone())
        .iter()
        .map(|p| p.0)
        .collect();
    trade::daily(&mut w);
    let st = &w.overworld.sites[k].stores;
    if !made.contains(&ItemKind::GrainSheaf) {
        assert!((st[&ItemKind::GrainSheaf] - 96.0).abs() < 0.01);
    }
    if !made.contains(&ItemKind::StoneBlock) {
        assert!((st[&ItemKind::StoneBlock] - 99.0).abs() < 0.01);
    }
}

#[test]
fn barter_moves_goods_and_makes_none() {
    let mut w = founded(3603);
    let k = peopled(&w);
    for _ in 0..3 {
        trade::daily(&mut w);
    }
    let cid = w.communities[0].id;
    let home = w.communities[0].home;
    let id = w.overworld.sites[k].id;
    let total = |w: &World| -> f32 { w.overworld.sites[k].stores.values().sum() };
    let before = w.overworld.sites[k].clone();
    let sum_before = total(&w);
    let (took, got) = trade::barter(&mut w, id, cid, home, 10, ItemKind::Log, 6, 40);
    assert!(took > 0, "a settlement with stores buys logs");
    assert!(!got.is_empty(), "and pays for them");
    let paid: u32 = got.iter().map(|g| g.2).sum();
    assert!(paid <= 40, "no more than the party carries");
    let sum_after = total(&w);
    assert!(
        (sum_before + took as f32 - paid as f32 - sum_after).abs() < 0.01,
        "ledger: {sum_before} + {took} - {paid} != {sum_after}"
    );
    // Paid only from what it could spare, in the materials it holds.
    let after = &w.overworld.sites[k];
    for (kind, m, n) in &got {
        let need = trade::need(&w, &before, *kind);
        assert!(
            after.stores[kind] + 1.0 >= need * 0.5,
            "{kind:?}: {} left of a need {need} after paying {n}",
            after.stores[kind]
        );
        assert_eq!(before.store_material.get(kind), Some(m));
    }
    let t = after.trades.last().expect("the trade is recorded");
    assert_eq!(t.gave, vec![(ItemKind::Log, took)]);
    assert_eq!(t.community, cid);
    // It buys no more than it can pay for.
    let (took, _) = trade::barter(&mut w, id, cid, home, 10, ItemKind::Weapon, 200, 400);
    assert!(took < 200, "took {took} of 200 spears");
    // It does not buy what it has plenty of.
    let k_has = w.overworld.sites[k].stores.keys().next().copied();
    if let Some(plenty) = k_has {
        w.overworld.sites[k].stores.insert(plenty, 1000.0);
        let (took, _) = trade::barter(&mut w, id, cid, home, 10, plenty, 12, 40);
        assert_eq!(took, 0, "{plenty:?} bought by a place that has a thousand");
    }
    // A place with nothing to pay with takes nothing.
    w.overworld.sites[k].stores.clear();
    let (took, got) = trade::barter(&mut w, id, cid, home, 10, ItemKind::Weapon, 6, 40);
    assert_eq!((took, got.len()), (0, 0));
}

#[test]
fn errands_go_only_where_they_have_been() {
    let mut w = founded(3604);
    let cid = w.communities[0].id;
    let home = w.communities[0].home;
    w.drop_item(ItemKind::Log, 60, home.offset(3, 0, 0));
    for _ in 0..2 {
        trade::daily(&mut w);
    }
    assert!(
        trade::best_errand(&w, cid, home, 10).is_none(),
        "no settlement is known yet"
    );
    let k = peopled(&w);
    w.overworld.sites[k].known_by.push(cid);
    let (site, kind, value) = trade::best_errand(&w, cid, home, 10).expect("an errand");
    assert_eq!(site, w.overworld.sites[k].id);
    assert_eq!(kind, ItemKind::Log);
    assert!(value > 0.0);
    // A road that has cost lives is worth less: a caravan there never heard from again.
    let lost = form_caravan(
        &mut w,
        cid,
        Purpose::Trade {
            site,
            carry: ItemKind::Log,
        },
    )
    .expect("a caravan forms");
    let c = w
        .overworld
        .caravans
        .iter_mut()
        .find(|c| c.id == lost)
        .unwrap();
    c.state = CaravanState::Lost;
    let n = c.members.len() as f32;
    let after = trade::best_errand(&w, cid, home, 10);
    assert!(
        after.is_none_or(|e| e.0 != site || e.2 <= value - 20.0 * n + 0.01),
        "{after:?} after losing {n} on the road (was {value})"
    );
    w.overworld.caravans.clear();
    // Nobody walks out to a settlement with nothing to pay with.
    w.overworld.sites[k].stores.clear();
    assert!(trade::best_errand(&w, cid, home, 10).is_none());
}

/// #1292: a site the valley has seen come armed within the last 20 days is skipped for an
/// errand, however well it pays. What the valley never saw come armed, or saw too long ago,
/// stays a destination even if the site itself (unseen by the valley) holds a grudge.
#[test]
fn no_trade_with_besiegers() {
    let mut w = founded(3607);
    w.meta.tick += 30 * TICKS_PER_DAY;
    let cid = w.communities[0].id;
    let home = w.communities[0].home;
    w.drop_item(ItemKind::Log, 60, home.offset(3, 0, 0));
    for _ in 0..2 {
        trade::daily(&mut w);
    }
    let sites = peopled_sites(&w);
    assert!(sites.len() >= 2, "at least two peopled settlements");
    for &k in &sites {
        w.overworld.sites[k].known_by.push(cid);
    }
    let (best_site, _, best_value) =
        trade::best_errand(&w, cid, home, 10).expect("an errand without hostility");
    let tick = w.meta.tick;
    let known_other = hf_sim::known::KnownOther {
        site: best_site,
        name: w.overworld.site(best_site).unwrap().name.clone(),
        first_seen: tick,
        last_seen: tick.saturating_sub(5 * TICKS_PER_DAY),
        most: 10,
        armed: true,
        from: hf_sim::expedition::Edge::North,
    };
    w.communities[0].known_others.push(known_other);
    let (site, _, value) = trade::best_errand(&w, cid, home, 10).expect("a safer errand elsewhere");
    assert_ne!(site, best_site, "the besieging site is skipped");
    assert!(value <= best_value, "not the best-paying site");
    // Seen too long ago: back to the best-paying site.
    w.communities[0].known_others[0].last_seen = tick.saturating_sub(25 * TICKS_PER_DAY);
    let (site, _, _) = trade::best_errand(&w, cid, home, 10).expect("an errand");
    assert_eq!(site, best_site, "an old sighting does not hold");
    // Unarmed: also back to the best-paying site.
    w.communities[0].known_others[0].last_seen = tick.saturating_sub(5 * TICKS_PER_DAY);
    w.communities[0].known_others[0].armed = false;
    let (site, _, _) = trade::best_errand(&w, cid, home, 10).expect("an errand");
    assert_eq!(
        site, best_site,
        "unarmed visitors do not hold a caravan back"
    );
}

#[test]
fn a_caravan_brings_back_what_the_settlement_held() {
    let mut w = founded(3605);
    let home = w.communities[0].home;
    let cid = w.communities[0].id;
    w.drop_item(ItemKind::Log, 40, home.offset(2, 0, 0));
    let k = peopled(&w);
    let site = w.overworld.sites[k].id;
    let mut s = Session::new(w, Chronicle::in_memory().unwrap());
    let id = form_caravan(
        &mut s.world,
        cid,
        Purpose::Trade {
            site,
            carry: ItemKind::Log,
        },
    )
    .expect("caravan forms");
    let mut done = false;
    for _ in 0..TICKS_PER_DAY * 30 {
        s.step().unwrap();
        if s.world.last_events.iter().any(
            |e| matches!(&e.kind, EventKind::CaravanReturned { caravan, .. } if *caravan == id),
        ) {
            done = true;
            break;
        }
    }
    assert!(done, "the caravan came back");
    let c = s
        .world
        .overworld
        .caravans
        .iter()
        .find(|c| c.id == id)
        .unwrap();
    assert_eq!(c.state, CaravanState::Done);
    let st = s
        .world
        .overworld
        .sites
        .iter()
        .find(|x| x.id == site)
        .unwrap();
    let t = st.trades.last().expect("a trade at the settlement");
    assert!(st.known_by.contains(&cid));
    // Each kind brought home came in the material the settlement held it in.
    for (kind, _) in &t.got {
        let m = st.store_material[kind];
        assert!(
            s.world.items.iter().any(|it| it.kind == *kind
                && it.material == m
                && matches!(it.place, ItemPlace::Ground(_))),
            "{kind:?} of {m:?} is on the map"
        );
    }
}

#[test]
fn stores_survive_a_save() {
    let mut w = founded(3606);
    trade::daily(&mut w);
    let k = peopled(&w);
    let before = w.overworld.sites[k].stores.clone();
    let bytes = hf_sim::save::encode(&hf_sim::save::snapshot::Snapshot::capture(&w)).unwrap();
    let back = hf_sim::save::decode(&bytes).unwrap().restore();
    assert_eq!(back.overworld.sites[k].stores, before);
}

#[test]
fn a_trade_shows_on_the_map_and_the_settlements_page() {
    let mut w = World::from_setup(&hf_sim::setup::WorldSetup::new(3));
    let m = hf_sim::realm::generate(3, &hf_sim::realm::RealmParams::default());
    let gold = |px: &[u8]| px.chunks(3).filter(|c| *c == [232, 196, 120]).count();
    assert_eq!(
        gold(&hf_sim::view::legends::map_rgb(&w, &m, w.realm.year)),
        0
    );
    trade::daily(&mut w);
    // The farthest settlement of the world beside the valley, so the line has length.
    let n = m.size;
    let valley = w.meta.setup.as_ref().and_then(|s| s.region).unwrap();
    let far = |s: &hf_sim::expedition::Site| {
        let to = w.realm.sites[s.realm_site.unwrap() as usize].region;
        let dx = (to % n) as i64 - (valley % n) as i64;
        let dy = (to / n) as i64 - (valley / n) as i64;
        dx.abs().max(dy.abs())
    };
    let k = (0..w.overworld.sites.len())
        .filter(|i| w.overworld.sites[*i].realm_site.is_some() && w.overworld.sites[*i].people > 0)
        .max_by_key(|i| far(&w.overworld.sites[*i]))
        .expect("a settlement of the world beside the valley");
    assert!(
        far(&w.overworld.sites[k]) >= 2,
        "a settlement two regions off"
    );
    let cid = w.communities[0].id;
    let home = w.communities[0].home;
    w.overworld.sites[k].known_by.push(cid);
    let id = w.overworld.sites[k].id;
    let (took, _) = trade::barter(&mut w, id, cid, home, 10, ItemKind::Log, 6, 30);
    assert!(took > 0);
    // A line on the map of the present, none on the map of long ago.
    assert!(gold(&hf_sim::view::legends::map_rgb(&w, &m, w.realm.year)) > 0);
    assert_eq!(gold(&hf_sim::view::legends::map_rgb(&w, &m, 10)), 0);
    let rs = w.overworld.sites[k].realm_site.unwrap();
    let c = Chronicle::in_memory().unwrap();
    let page = hf_sim::view::legends::page(&w, &c, &format!("site:{rs}"));
    assert!(page.contains("They hold for trade:"), "{page}");
    assert!(
        page.contains(&format!("A caravan from the colony traded {took} logs for")),
        "{page}"
    );
}
