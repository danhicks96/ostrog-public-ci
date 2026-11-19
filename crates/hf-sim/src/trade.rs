//! Trade with the world (C36). Each real settlement beyond the valley keeps stores of what its
//! people's way of living and its land give (farmers grain, herders hides and meat, hunters
//! hides and meat and timber, fisherfolk fish and reeds, miners ore, coal and stone), uses
//! them up day by day, and wants what it lacks. A thing's price at a place is its worth (a
//! day's food is about 1, `world::MaterialDef::value` for what is made of something) times how
//! short of it the place is. A caravan sells what it carries for that price and is paid in
//! goods from the settlement's own stores, those the valley is shortest of first, so what comes
//! home was really there and is gone from there. Nothing is made from nothing: the valley's
//! ledger records the goods leaving ("trade") and arriving ("caravan").

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::expedition::{Site, SiteKind};
use crate::ids::{CommunityId, TilePos};
use crate::items::{ItemKind, ItemPlace};
use crate::world::Material;
use crate::world_state::World;

/// One exchange at a settlement, kept on the settlement (the last twelve).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Trade {
    pub tick: u64,
    pub community: CommunityId,
    pub gave: Vec<(ItemKind, u32)>,
    pub got: Vec<(ItemKind, u32)>,
}

/// One exchange between two settlements beyond the valley (P9), kept on both.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Exchange {
    pub tick: u64,
    /// The other settlement.
    pub with: u32,
    pub gave: (ItemKind, u32),
    pub got: (ItemKind, u32),
}

/// What a kind is worth, a day's food being about 1: by its material where it has one, else
/// by kind.
pub fn worth(w: &World, kind: ItemKind, m: Material) -> f32 {
    use ItemKind as K;
    let mat = w.grid.materials.get(m).value;
    let by_kind = match kind {
        K::Berries | K::Root | K::GrainSheaf | K::RawMeat => kind.nutrition_hours() / 6.0,
        K::Meal => 1.2,
        K::Log | K::StoneBlock | K::Firewood | K::Coal => 1.0,
        K::Plank => 0.8,
        K::Fibre | K::Clay | K::Sand => 0.4,
        K::Dye => 1.0,
        K::IronOre | K::CopperOre | K::Ore => 2.0,
        K::Cloth => 2.0,
        K::Clothes => 5.0,
        K::Hide => 3.0,
        K::Leather => 4.0,
        K::Poultice => 2.0,
        K::Tool | K::Saw | K::Hammer | K::SledgeHammer | K::Chisel | K::Needle => 6.0,
        K::Bucket | K::Torch => 1.5,
        K::CopperBar => 5.0,
        K::IronBar => 6.0,
        K::Bar => 6.0,
        K::Gem => 20.0,
        K::Vessel => 2.0,
        K::Ale => 1.5,
        K::Wine => 2.0,
        K::Weapon => 18.0,
        K::Armour => 30.0,
        K::Jewel => 40.0,
        K::Figurine | K::Record => 0.0,
        K::RoughStone => 0.7,
        K::Thatch => 0.6,
        K::Stool => 1.5,
        K::Door | K::Table | K::Chest => 2.5,
        K::BedFrame => 3.5,
    };
    match kind {
        // A bar, spear, mail or ring is worth what its metal is worth, worked.
        K::Bar if mat > 0.0 => mat,
        // #274: stone, ore and gems are worth what they are: a marble block three granite
        // blocks' worth and more, gold ore seven times iron's, a sapphire twice a garnet.
        K::StoneBlock | K::RoughStone if mat > 0.0 => by_kind * mat,
        K::Ore | K::IronOre | K::CopperOre | K::Gem if mat > 0.0 => mat,
        K::Weapon | K::Armour | K::Jewel if mat > 0.0 => by_kind * mat / 6.0,
        _ if mat > 0.0 && by_kind == 0.0 => mat,
        _ => by_kind,
    }
}

/// Kinds a caravan will carry out or bring back. Storied things and records are never traded:
/// they are someone's.
pub fn tradeable(kind: ItemKind) -> bool {
    !crate::legend::storied(kind) && !matches!(kind, ItemKind::Figurine | ItemKind::Record)
}

/// The way of living of the people a settlement belongs to, if it is one of the world's.
fn living(w: &World, s: &Site) -> Option<crate::realm::peoples::Living> {
    let rs = w.realm.sites.get(s.realm_site? as usize)?;
    w.realm.peoples.get(rs.people as usize).map(|p| p.living)
}

/// What a place makes, per person a day (or flat for empty country), and of what.
pub fn produces(w: &World, s: &Site) -> Vec<(ItemKind, Material, f32)> {
    use crate::realm::peoples::Living as L;
    use ItemKind as K;
    use Material as M;
    match s.kind {
        SiteKind::Mine => vec![
            (K::IronOre, M::IRON_ORE, 0.12),
            (K::CopperOre, M::COPPER_ORE, 0.08),
            (K::Coal, M::COAL, 0.10),
            (K::StoneBlock, M::GRANITE, 0.15),
            (K::Ore, M::TIN_ORE, 0.03),
        ],
        SiteKind::Forest => vec![(K::Log, M::OAK, 0.0), (K::Hide, M::DEER_HIDE, 0.0)],
        SiteKind::Lakeshore => vec![(K::RawMeat, M::MEAT, 0.0)],
        SiteKind::Ruin => vec![],
        SiteKind::Village => match living(w, s) {
            Some(L::Herding) => vec![
                (K::Hide, M::GOAT_HIDE, 0.05),
                (K::RawMeat, M::MEAT, 0.25),
                (K::Cloth, M::AIR, 0.04),
                (K::Leather, M::GOAT_HIDE, 0.01),
            ],
            Some(L::Hunting) => vec![
                (K::Hide, M::DEER_HIDE, 0.05),
                (K::RawMeat, M::MEAT, 0.25),
                (K::Log, M::PINE, 0.10),
            ],
            Some(L::Fishing) => vec![
                (K::RawMeat, M::MEAT, 0.30),
                (K::Fibre, M::REED, 0.06),
                (K::Vessel, M::CLAY, 0.01),
            ],
            // Farmers, and any village of the old fixed list.
            _ => vec![
                (K::GrainSheaf, M::GRAIN, 0.30),
                (K::Berries, M::BERRY, 0.08),
                (K::Cloth, M::AIR, 0.02),
                (K::Ale, M::AIR, 0.02),
            ],
        },
    }
}

/// What one person's household wants to hold of a kind.
fn base_need(kind: ItemKind) -> f32 {
    use ItemKind as K;
    match kind {
        K::GrainSheaf | K::RawMeat | K::Root | K::Berries | K::Meal => 3.0,
        K::Log | K::StoneBlock => 0.6,
        K::Tool => 0.15,
        K::Cloth => 0.3,
        K::Clothes => 0.2,
        K::Hide | K::Leather => 0.3,
        K::Vessel => 0.2,
        K::IronBar | K::CopperBar | K::Bar => 0.1,
        K::Ale | K::Wine => 0.2,
        K::Poultice => 0.05,
        K::Weapon => 0.05,
        K::Armour => 0.02,
        K::Jewel => 0.02,
        K::Coal => 0.1,
        _ => 0.0,
    }
}

/// How much of a kind a place wants to hold: enough food to see out a season, some of what
/// every household uses, and more of what it cannot make.
pub fn need(w: &World, s: &Site, kind: ItemKind) -> f32 {
    use ItemKind as K;
    let people = s.people as f32;
    if people == 0.0 {
        return 0.0;
    }
    let makes = produces(w, s).iter().any(|(k, _, _)| *k == kind);
    let base = base_need(kind);
    // A people that makes something wants less of it from others; a mine wants food most.
    let mine_food = s.kind == SiteKind::Mine
        && matches!(
            kind,
            K::GrainSheaf | K::RawMeat | K::Meal | K::Root | K::Berries
        );
    people * base * if makes { 0.5 } else { 1.0 } * if mine_food { 2.0 } else { 1.0 }
}

/// How short of something a place is, from 0.3 (glutted) to 3 (has none of what it needs).
fn scarcity(stock: f32, need: f32) -> f32 {
    ((need + 1.0) / (stock + 1.0)).clamp(0.3, 3.0)
}

/// Kinds whose material makes them a different good (#274): one stone or ore or gem is not
/// another, so a place that has granite blocks still lacks marble.
pub fn by_material(kind: ItemKind) -> bool {
    use ItemKind as K;
    matches!(
        kind,
        K::StoneBlock | K::RoughStone | K::Ore | K::IronOre | K::CopperOre | K::Gem
    )
}

/// What a settlement pays for one of a kind (of `m`; `Material::AIR` when it does not matter):
/// its worth times how short of it the place is. Of a stone, ore or gem it holds none of, it
/// is as short as if it had none of the kind (#274).
pub fn price_at(w: &World, s: &Site, kind: ItemKind, m: Material) -> f32 {
    let held = s.store_material.get(&kind).copied();
    let stock = if by_material(kind) && m != Material::AIR && held.is_some_and(|h| h != m) {
        0.0
    } else {
        s.stores.get(&kind).copied().unwrap_or(0.0)
    };
    worth(w, kind, m) * scarcity(stock, need(w, s, kind))
}

/// The material most of a kind near `home` is (what a caravan loading there would carry).
pub fn carried_material(w: &World, home: TilePos, kind: ItemKind) -> Material {
    let mut of: BTreeMap<Material, u32> = BTreeMap::new();
    for it in &w.items {
        if it.kind == kind && it.story.is_none() {
            if let ItemPlace::Ground(p) = it.place {
                if p.dist(home) <= 18 {
                    *of.entry(it.material).or_default() += it.count;
                }
            }
        }
    }
    of.iter()
        .max_by(|a, b| a.1.cmp(b.1).then(b.0.cmp(a.0)))
        .map(|(m, _)| *m)
        .unwrap_or(Material::AIR)
}

/// How much the valley's community at `home` wants one of a kind: its worth times how short
/// the community is of it, by what lies within reach of home and how many there are.
pub fn price_home(w: &World, home: TilePos, people: u32, kind: ItemKind, m: Material) -> f32 {
    let stock: u32 = w
        .items
        .iter()
        .filter(|it| {
            it.kind == kind && matches!(it.place, ItemPlace::Ground(p) if p.dist(home) <= 45)
        })
        .map(|it| it.count)
        .sum();
    worth(w, kind, m) * scarcity(stock as f32, base_need(kind) * people as f32)
}

/// Once a day: every settlement makes what its living and land give and uses up a little of
/// all it holds (food faster). A settlement new to trade starts with ten days of making.
pub fn daily(w: &mut World) {
    for k in 0..w.overworld.sites.len() {
        let s = w.overworld.sites[k].clone();
        let made = produces(w, &s);
        let flat = s.people == 0;
        let fresh = s.stores.is_empty();
        let site = &mut w.overworld.sites[k];
        for (kind, m, rate) in made {
            let per_day = if flat {
                match kind {
                    ItemKind::Log => 0.6,
                    ItemKind::RawMeat => 0.4,
                    _ => 0.05,
                }
            } else {
                rate * s.people as f32
            };
            let add = if fresh { per_day * 10.0 } else { per_day };
            *site.stores.entry(kind).or_insert(0.0) += add;
            site.store_material.insert(kind, m);
        }
        if fresh && s.kind == SiteKind::Ruin && site.stores.is_empty() {
            site.stores.insert(ItemKind::StoneBlock, 10.0);
            site.store_material
                .insert(ItemKind::StoneBlock, Material::GRANITE);
        }
        for (kind, v) in site.stores.iter_mut() {
            let used = if kind.is_food() { 0.04 } else { 0.01 };
            *v *= 1.0 - used;
        }
    }
    if (w.meta.tick / crate::time::TICKS_PER_DAY) % 10 == 3 {
        between_sites(w);
    }
}

/// Whether the people of two settlements beyond the valley deal with each other: their realms
/// not at war, and the host's culture not holding other kinds in dread (P9).
fn deal(w: &World, a: &Site, b: &Site) -> bool {
    let (Some(ra), Some(rb)) = (
        a.realm_site.and_then(|i| w.realm.sites.get(i as usize)),
        b.realm_site.and_then(|i| w.realm.sites.get(i as usize)),
    ) else {
        return true;
    };
    let polity = |s: u32| {
        w.realm
            .polities
            .iter()
            .find(|p| p.ended.is_none() && p.members.contains(&s))
            .map(|p| p.id)
    };
    if let (Some(x), Some(y)) = (polity(ra.id), polity(rb.id)) {
        if crate::realm::relations::at_war(&w.realm, x, y) {
            return false;
        }
    }
    let dread = |c: u32| {
        w.realm
            .cultures
            .get(c as usize)
            .is_some_and(|c| c.stance(crate::past::Topic::OtherRaces) <= -0.4)
    };
    ra.people == rb.people || (!dread(ra.culture) && !dread(rb.culture))
}

/// Every ten days each pair of settlements beyond the valley that deal with each other swap
/// what one has spare and the other lacks, at the buyer's prices (P9). Goods only move: what
/// one gives the other has, so the world's prices move without the valley.
pub fn between_sites(w: &mut World) {
    let tick = w.meta.tick;
    let n = w.overworld.sites.len();
    for i in 0..n {
        for j in i + 1..n {
            let (a, b) = (w.overworld.sites[i].clone(), w.overworld.sites[j].clone());
            if a.people == 0 || b.people == 0 || !deal(w, &a, &b) {
                continue;
            }
            // The kind `from` can spare that `to` is shortest of, and how many `to` wants.
            let offer = |from: &Site, to: &Site| -> Option<(ItemKind, u32, f32)> {
                from.stores
                    .iter()
                    .filter(|(k, _)| tradeable(**k))
                    .filter_map(|(k, v)| {
                        let spare = (v - need(w, from, *k)).floor();
                        let stock = to.stores.get(k).copied().unwrap_or(0.0);
                        let want = (need(w, to, *k) - stock).floor();
                        let n = spare.min(want);
                        let m = from.store_material.get(k).copied().unwrap_or(Material::AIR);
                        let short = scarcity(stock, need(w, to, *k));
                        (n >= 1.0 && short > 1.2).then(|| (*k, n as u32, price_at(w, to, *k, m)))
                    })
                    .max_by(|x, y| x.2.total_cmp(&y.2).then(y.0.cmp(&x.0)))
            };
            let (Some((ka, na, pa)), Some((kb, nb, _))) = (offer(&a, &b), offer(&b, &a)) else {
                continue;
            };
            // At b's prices: what a gives is worth what b pays back.
            let mb = b.store_material.get(&kb).copied().unwrap_or(Material::AIR);
            let pb = price_at(w, &b, kb, mb).max(0.05);
            let value = na as f32 * pa;
            let back = ((value / pb).floor() as u32).min(nb);
            if back == 0 {
                continue;
            }
            // Only what is paid for goes: a gives as many as b's goods pay for.
            let give = ((back as f32 * pb / pa.max(0.05)).round() as u32).clamp(1, na);
            let ma = a.store_material.get(&ka).copied().unwrap_or(Material::AIR);
            for (k, other, gave, got, m) in [
                (i, b.id, (ka, give), (kb, back), mb),
                (j, a.id, (kb, back), (ka, give), ma),
            ] {
                let s = &mut w.overworld.sites[k];
                if let Some(v) = s.stores.get_mut(&gave.0) {
                    *v -= gave.1 as f32;
                }
                *s.stores.entry(got.0).or_insert(0.0) += got.1 as f32;
                s.store_material.entry(got.0).or_insert(m);
                s.exchanges.push(Exchange {
                    tick,
                    with: other,
                    gave,
                    got,
                });
                if s.exchanges.len() > 12 {
                    s.exchanges.remove(0);
                }
            }
        }
    }
}

/// What settlement `s` would do with `gave` of `kind` from the community at `home` (of
/// `people`): it takes as many as it lacks and can pay for from its surplus (what it holds
/// beyond half its need), and pays in the goods the valley is shortest of, no more than `capacity` can
/// carry. How many it takes and what it gives, with materials. Changes nothing.
pub fn quote(
    w: &World,
    s: &Site,
    home: TilePos,
    people: u32,
    kind: ItemKind,
    gave: u32,
    capacity: u32,
) -> (u32, Vec<(ItemKind, Material, u32)>) {
    quote_wanting(
        w,
        s,
        home,
        people,
        kind,
        Material::AIR,
        gave,
        capacity,
        None,
    )
}

/// `quote`, of `material` (#274: a stone, ore or gem is priced by what it is), paid first in
/// `want` where the settlement has it to spare (a fetch, C59).
#[allow(clippy::too_many_arguments)]
pub fn quote_wanting(
    w: &World,
    s: &Site,
    home: TilePos,
    people: u32,
    kind: ItemKind,
    material: Material,
    gave: u32,
    capacity: u32,
    want: Option<ItemKind>,
) -> (u32, Vec<(ItemKind, Material, u32)>) {
    // It wants what it lacks: no more than takes it to half again its need. (The first
    // soaks had farmers buying grain from the valley every week.)
    let held = s.store_material.get(&kind).copied();
    let stock =
        if by_material(kind) && material != Material::AIR && held.is_some_and(|h| h != material) {
            0.0
        } else {
            s.stores.get(&kind).copied().unwrap_or(0.0)
        };
    let gave = gave.min((need(w, s, kind) * 1.5 - stock).floor().max(0.0) as u32);
    let unit = price_at(w, s, kind, material);
    let offered = unit * gave as f32;
    // What it can spare, and what each is worth to the valley against what it costs here.
    let mut goods: Vec<(ItemKind, Material, u32, f32, f32)> = s
        .stores
        .iter()
        .filter(|(k, _)| tradeable(**k) && **k != kind)
        .filter_map(|(k, v)| {
            let spare = (v - need(w, s, *k) * 0.5).floor();
            let m = s.store_material.get(k).copied().unwrap_or(Material::AIR);
            (spare >= 1.0).then(|| {
                let cost = price_at(w, s, *k, m).max(0.05);
                let want = price_home(w, home, people, *k, m);
                (*k, m, spare as u32, cost, want / cost)
            })
        })
        .collect();
    goods.sort_by(|a, b| b.4.total_cmp(&a.4).then(a.0.cmp(&b.0)));
    if let Some(wk) = want {
        goods.sort_by_key(|g| g.0 != wk);
    }
    let can_pay: f32 = goods.iter().map(|g| g.2 as f32 * g.3).sum();
    let took = if offered <= can_pay {
        gave
    } else {
        ((can_pay / unit.max(0.01)).floor() as u32).min(gave)
    };
    let mut budget = unit * took as f32;
    let mut room = capacity;
    let mut got: Vec<(ItemKind, Material, u32)> = Vec::new();
    for (gk, gm, spare, cost, _) in goods {
        if budget < cost * 0.5 || room == 0 {
            continue;
        }
        let n = ((budget / cost).round() as u32).min(spare).min(room);
        if n == 0 {
            continue;
        }
        budget -= n as f32 * cost;
        room -= n;
        got.push((gk, gm, n));
    }
    (took, got)
}

/// A caravan from the community at `home` (of `people`) sells `gave` of `kind` at a
/// settlement, on the terms of `quote`. The settlement's stores and its record change.
#[allow(clippy::too_many_arguments)]
pub fn barter(
    w: &mut World,
    site_id: u32,
    community: CommunityId,
    home: TilePos,
    people: u32,
    kind: ItemKind,
    gave: u32,
    capacity: u32,
) -> (u32, Vec<(ItemKind, Material, u32)>) {
    barter_wanting(
        w,
        site_id,
        community,
        home,
        people,
        kind,
        Material::AIR,
        gave,
        capacity,
        None,
    )
}

/// `barter` of `material`, paid first in `want` (a fetch, C59).
#[allow(clippy::too_many_arguments)]
pub fn barter_wanting(
    w: &mut World,
    site_id: u32,
    community: CommunityId,
    home: TilePos,
    people: u32,
    kind: ItemKind,
    material: Material,
    gave: u32,
    capacity: u32,
    want: Option<ItemKind>,
) -> (u32, Vec<(ItemKind, Material, u32)>) {
    let Some(k) = w.overworld.sites.iter().position(|s| s.id == site_id) else {
        return (0, Vec::new());
    };
    let (took, got) = quote_wanting(
        w,
        &w.overworld.sites[k],
        home,
        people,
        kind,
        material,
        gave,
        capacity,
        want,
    );
    let site = &mut w.overworld.sites[k];
    *site.stores.entry(kind).or_insert(0.0) += took as f32;
    if took > 0 && material != Material::AIR {
        site.store_material.entry(kind).or_insert(material);
    }
    for (gk, _, n) in &got {
        if let Some(v) = site.stores.get_mut(gk) {
            *v = (*v - *n as f32).max(0.0);
        }
    }
    site.trades.push(Trade {
        tick: w.meta.tick,
        community,
        gave: vec![(kind, took)],
        got: got.iter().map(|(k, _, n)| (*k, *n)).collect(),
    });
    if site.trades.len() > 12 {
        site.trades.remove(0);
    }
    (took, got)
}

/// How many of the community's people its caravans to settlement `site` have lost on the road
/// in the last 120 days: those named lost in a caravan's log, and every member of a caravan
/// never heard from again. A road that has cost lives is worth less.
/// #1292: a site the community has seen come armed within the last 20 days is not an errand's
/// destination — what a normal person would do here is not walk berries to the town whose men
/// camped at the wall last week. Reads `Community.known_others`, what the valley itself saw
/// (not `Site.grudge`, the town's own count, which nobody in the valley has perceived).
fn recently_hostile(w: &World, community: CommunityId, site: u32) -> bool {
    let Some(c) = w.communities.iter().find(|c| c.id == community) else {
        return false;
    };
    c.known_others.iter().any(|k| {
        k.site == site
            && k.armed
            && w.meta.tick.saturating_sub(k.last_seen) < 20 * crate::time::TICKS_PER_DAY
    })
}

pub fn lost_on(w: &World, community: CommunityId, site: u32) -> usize {
    use crate::expedition::{CaravanState, Purpose};
    let since = w.meta.tick.saturating_sub(120 * crate::time::TICKS_PER_DAY);
    w.overworld
        .caravans
        .iter()
        .filter(|c| {
            c.community == community
                && c.formed >= since
                && matches!(c.purpose, Purpose::Trade { site: x, .. } | Purpose::Scout { site: x } | Purpose::Fetch { site: x, .. } if x == site)
        })
        .map(|c| {
            // One by one on the road, and all who were left when it was lost whole (FIXLOG
            // 2026-10-20 #167, C62: the emptied member list counted nobody).
            let whole = if c.state == CaravanState::Lost {
                (c.lost as usize).max(c.members.len())
            } else {
                0
            };
            whole + c.log.iter().filter(|l| l.contains("was lost on the road")).count()
        })
        .sum()
}

/// The best thing for the community at `home` to carry to a known settlement: the kind it has
/// What a party could load at `home`, by kind: tradeable, storyless stacks within 18 tiles
/// (as society::take_items loads).
pub fn stock_to_carry(w: &World, home: TilePos) -> BTreeMap<ItemKind, u32> {
    let mut stock: BTreeMap<ItemKind, u32> = BTreeMap::new();
    for it in &w.items {
        if it.story.is_none() && tradeable(it.kind) {
            if let ItemPlace::Ground(p) = it.place {
                if p.dist(home) <= 18 {
                    *stock.entry(it.kind).or_default() += it.count;
                }
            }
        }
    }
    stock
}

/// Below this many of a kind a community has none to spare for a journey.
pub fn spare_line(people: u32) -> u32 {
    16 + people * 2
}

/// Where to fetch `want` from (C59, FIXLOG 2026-10-20 #164, C62): a peopled settlement the
/// community knows that will pay at least four `want` for something it has to spare, on the
/// same terms a barter there pays (`quote_wanting`: only from what it holds above half its
/// need), less what the road there has cost them. (Site id, the kind to carry.)
pub fn best_fetch(
    w: &World,
    community: CommunityId,
    home: TilePos,
    people: u32,
    want: ItemKind,
) -> Option<(u32, ItemKind)> {
    let stock = stock_to_carry(w, home);
    let line = spare_line(people);
    let mut best: Option<(u32, ItemKind, f32)> = None;
    for s in w.overworld.sites.iter().filter(|s| {
        s.people > 0 && s.known_by.contains(&community) && !recently_hostile(w, community, s.id)
    }) {
        for (kind, n) in &stock {
            if *n < line || *kind == want {
                continue;
            }
            let carry = (*n - line / 2).min(12);
            let m = carried_material(w, home, *kind);
            let (_, got) = quote_wanting(w, s, home, people, *kind, m, carry, 30, Some(want));
            let fetched: u32 = got.iter().filter(|g| g.0 == want).map(|g| g.2).sum();
            if fetched < 4 {
                continue;
            }
            let v = fetched as f32 / (1.0 + s.days as f32 * 0.1)
                - 20.0 * lost_on(w, community, s.id) as f32;
            if v > 0.0 && best.as_ref().is_none_or(|b| v > b.2) {
                best = Some((s.id, *kind, v));
            }
        }
    }
    best.map(|(s, k, _)| (s, k))
}

/// a surplus of that a settlement it knows will pay most for, less what the road there has
/// cost them, and where. (Site id, kind, value.)
pub fn best_errand(
    w: &World,
    community: CommunityId,
    home: TilePos,
    people: u32,
) -> Option<(u32, ItemKind, f32)> {
    let stock = stock_to_carry(w, home);
    let spare_line = spare_line(people);
    let mut best: Option<(u32, ItemKind, f32)> = None;
    // Only where a caravan of theirs has been: what a settlement pays is learned by going.
    for s in w.overworld.sites.iter().filter(|s| {
        s.people > 0 && s.known_by.contains(&community) && !recently_hostile(w, community, s.id)
    }) {
        for (kind, n) in &stock {
            if *n < spare_line {
                continue;
            }
            // What the trade would bring, as the valley values it, less what it gives up;
            // a long road is worth less.
            let carry = (*n - spare_line / 2).min(12);
            let m = carried_material(w, home, *kind);
            let (took, got) = quote_wanting(w, s, home, people, *kind, m, carry, 30, None);
            let gain: f32 = got
                .iter()
                .map(|(k, m, g)| price_home(w, home, people, *k, *m) * *g as f32)
                .sum::<f32>()
                - price_home(w, home, people, *kind, m) * took as f32;
            let v = gain / (1.0 + s.days as f32 * 0.1) - 20.0 * lost_on(w, community, s.id) as f32;
            if v > 0.0 && best.as_ref().is_none_or(|b| v > b.2) {
                best = Some((s.id, *kind, v));
            }
        }
    }
    best
}
