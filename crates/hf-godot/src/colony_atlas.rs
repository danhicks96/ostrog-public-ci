//! Live map snapshot for the colony watcher. The simulation owns travel progress.
use crate::SimHost;
use godot::prelude::*;
use hf_sim::{
    agents::Species,
    expedition::{CaravanState, Purpose},
    view::known,
};

#[derive(GodotClass)]
#[class(init, base=RefCounted)]
struct ColonyAtlas {}

#[godot_api]
impl ColonyAtlas {
    /// Called on watcher setup or world replacement, never on a per-frame read.
    #[func]
    fn enable_actor_hearing(&self, mut host: Gd<SimHost>) {
        let mut host = host.bind_mut();
        if host
            .world()
            .is_none_or(|w| w.communities.is_empty() || w.watcher_hearing.is_some())
        {
            return;
        }
        host.settle();
        if let Some(s) = &mut host.session {
            s.world.watcher_hearing = Some(Box::default());
        }
        // The old snapshot had no active watcher. Release its world on the worker just as
        // forget_steps does, without cancelling pending ticks or clearing other view caches.
        if let Some(old) = host.snapshot.take() {
            if let Some(worker) = &host.stepper {
                worker.discard(old);
            }
        }
    }

    #[func]
    fn local_actor_sounds(&self, host: Gd<SimHost>, z: i32) -> VarDictionary {
        let host = host.bind();
        let mut out = VarDictionary::new();
        let Some(w) = host.world() else {
            return out;
        };
        if w.communities.is_empty() {
            return out;
        }
        let mut own = VarDictionary::new();
        for a in &w.agents {
            if a.pos.z == z
                && matches!(a.activity, hf_sim::mind::Activity::Working { .. })
                && hf_sim::view::hearing::listener(w, a)
            {
                own.set(a.id.0 as i64, true);
            }
        }
        out.set("own", &own);
        let mut heard = PackedInt64Array::new();
        if let Some(contacts) = &w.watcher_hearing {
            for (token, p) in contacts.cues(w, z, |a| {
                if a.species != Species::Person {
                    return true;
                }
                let row = crate::agent_row(w, a, None);
                row[15] == 2 && (1..=7).contains(&row[20])
            }) {
                heard.push(token as i64);
                heard.push(i64::from(p.x));
                heard.push(i64::from(p.y));
            }
        }
        out.set("heard", &heard);
        out
    }

    #[func]
    fn snapshot(&self, host: Gd<SimHost>, community: i64) -> VarDictionary {
        let host = host.bind();
        let mut out = VarDictionary::new();
        let Some(w) = host.world() else { return out };
        let Some(colony) = w
            .communities
            .first()
            .filter(|c| community < 0 || community == i64::from(c.id.0))
        else {
            return out;
        };
        out.set("colony_name", colony.name.as_str());
        out.set("community", colony.id.0 as i64);
        out.set(
            "home",
            w.meta
                .setup
                .as_ref()
                .and_then(|s| s.region)
                .map_or(-1, |r| r as i64),
        );
        let mut members = VarArray::new();
        let away = hf_sim::view::away::list(w);
        for (a, remote) in w
            .agents
            .iter()
            .map(|a| (a, false))
            .chain(w.overworld.away.iter().map(|a| (a, true)))
        {
            if a.community != Some(colony.id)
                || a.species != Species::Person
                || known::is_unfound(w, a.id)
                || !a.alive
            {
                continue;
            }
            let mut row = VarDictionary::new();
            row.set("id", a.id.0 as i64);
            row.set("name", a.name.as_str());
            row.set("away", remote);
            if !remote {
                row.set("tile", Vector3i::new(a.pos.x, a.pos.y, a.pos.z));
            } else if let Some(info) = away.iter().find(|r| r.agent == a.id) {
                row.set("line", info.line.as_str());
                if let Some(region) = info.region {
                    row.set("region", region as i64);
                    row.set("place", info.place.as_str());
                }
                if let Some(c) = w.overworld.caravans.iter().find(|c| {
                    Some(c.id) == a.caravan && c.community == colony.id && c.members.contains(&a.id)
                }) {
                    row.set("caravan", c.id as i64);
                    let (state, day) = match c.state {
                        CaravanState::Outbound { day } => ("Outbound", day),
                        CaravanState::Returning { day } => ("Returning", day),
                        CaravanState::AtSite { day } => ("At destination", day),
                        CaravanState::Stranded { days, .. } => ("Stranded", days),
                        _ => ("Away", 0),
                    };
                    row.set("stage", state);
                    row.set("day", i64::from(day) + 1);
                    let site = match c.purpose {
                        Purpose::Trade { site, .. }
                        | Purpose::Fetch { site, .. }
                        | Purpose::Scout { site } => site,
                    };
                    if let Some(s) = w.overworld.sites.iter().find(|s| s.id == site) {
                        let road = hf_sim::expedition::road_distance(c, w.meta.tick, s.days);
                        let length = i64::from(s.days) * hf_sim::expedition::ROAD_UNITS_PER_DAY;
                        row.set("road", road);
                        let seed = hf_sim::expedition::route_level::route_seed(w.meta.seed, s);
                        let trail_y =
                            hf_sim::expedition::route_level::path_y(seed, (road / 32) as i32);
                        row.set("trail_y", trail_y);
                        row.set(
                            "season",
                            hf_sim::time::Calendar::from_tick(w.meta.tick).season as i32,
                        );
                        let cargo = c
                            .cargo
                            .iter()
                            .map(|(kind, n)| format!("{n} {}", kind.label()))
                            .collect::<Vec<_>>()
                            .join(", ");
                        row.set("cargo", cargo.as_str());
                        row.set("fraction", road as f64 / length.max(1) as f64);
                    }
                    row.set("rations", i64::from(c.rations));
                    row.set(
                        "appearance",
                        &PackedInt32Array::from(crate::agent_row(w, a, None).as_slice()),
                    );
                }
            }
            members.push(&row.to_variant());
        }
        out.set("members", &members);
        out.set(
            "side",
            w.realm_map
                .as_ref()
                .map_or(hf_sim::realm::RealmParams::default().size, |m| m.size) as i64,
        );
        let mut chart = VarDictionary::new();
        if let Some(survey) = w.overworld.surveys.get(&colony.id) {
            for (i, mem) in &survey.regions {
                let mut cell = VarDictionary::new();
                cell.set("terrain", mem.terrain.as_str());
                cell.set("settlement", mem.settlement.as_str());
                cell.set("seen", mem.tick as i64);
                cell.set("live", mem.tick == survey.tick);
                chart.set(*i as i64, &cell);
            }
            out.set("survey_tick", survey.tick as i64);
        }
        out.set("chart", &chart);
        out
    }
    /// Sample the observed surface around a discovered caravan. Reading never advances the world.
    #[func]
    fn level_tiles(&self, host: Gd<SimHost>, community: i64, member: i64) -> PackedInt32Array {
        use hf_sim::expedition::route_level as level;
        let host = host.bind();
        let Some(w) = host.world() else {
            return Default::default();
        };
        if w.communities
            .first()
            .is_none_or(|c| i64::from(c.id.0) != community)
        {
            return Default::default();
        }
        let Some(a) = w.overworld.away.iter().find(|a| {
            i64::from(a.id.0) == member
                && a.alive
                && a.community.is_some_and(|id| i64::from(id.0) == community)
                && !known::is_unfound(w, a.id)
        }) else {
            return Default::default();
        };
        let Some(c) = w.overworld.caravans.iter().find(|c| {
            Some(c.id) == a.caravan && Some(c.community) == a.community && c.members.contains(&a.id)
        }) else {
            return Default::default();
        };
        let site = match c.purpose {
            Purpose::Scout { site } | Purpose::Fetch { site, .. } | Purpose::Trade { site, .. } => {
                site
            }
        };
        let Some(site) = w.overworld.site(site) else {
            return Default::default();
        };
        let centre =
            (hf_sim::expedition::road_distance(c, w.meta.tick, site.days) / level::TILE) as i32;
        let Some(survey) = w.overworld.surveys.get(&c.community) else {
            return Default::default();
        };
        let mut tiles = vec![
            centre - level::RADIUS_X,
            -level::RADIUS_Y,
            2 * level::RADIUS_X + 1,
            2 * level::RADIUS_Y + 1,
        ];
        for y in -level::RADIUS_Y..=level::RADIUS_Y {
            for x in centre - level::RADIUS_X..=centre + level::RADIUS_X {
                let tile = survey
                    .trails
                    .get(&(site.id, x, y))
                    .map_or(-1, |&(seen, code)| {
                        if seen == survey.tick {
                            code
                        } else {
                            code | (1 << 16)
                        }
                    });
                tiles.push(tile);
            }
        }
        PackedInt32Array::from(tiles.as_slice())
    }
    /// Six integers per moving water face visible to the colony at this level.
    #[func]
    fn visible_spill_edges(&self, host: Gd<SimHost>, z: i32) -> PackedInt32Array {
        let host = host.bind();
        let Some(w) = host.world() else {
            return Default::default();
        };
        let Some(memory) = w
            .communities
            .first()
            .and_then(|c| w.overworld.surveys.get(&c.id))
        else {
            return Default::default();
        };
        let mut rows = PackedInt32Array::new();
        // A witnessed face retains its last observation until the next survey. Checking
        // top_water here would reveal changes in a hidden storey between observations.
        for &edge in &memory.bank_spills {
            if !memory.sees_spill(edge, z)
                || hf_sim::water::frozen_at(w, edge.upper)
                || hf_sim::water::frozen_at(w, edge.lower)
            {
                continue;
            }
            let (a, b) = (edge.upper, edge.lower);
            rows.extend([a.x, a.y, a.z, b.x, b.y, b.z]);
        }
        rows
    }

    #[func]
    fn reported_ground(&self, host: Gd<SimHost>, view_z: i32) -> PackedInt32Array {
        let host = host.bind();
        let Some(w) = host.world() else {
            return Default::default();
        };
        let (width, height) = (w.grid.width as usize, w.grid.height as usize);
        let unknown = w.grid.z_min - 1;
        let mut heights = if let Some(chart) = w
            .communities
            .first()
            .and_then(|c| w.overworld.surveys.get(&c.id))
        {
            chart.reported_heights(width, height, unknown)
        } else {
            vec![unknown; width * height]
        };
        // Unknown neighbours contribute a level edge, rather than an invented deep cliff.
        for h in &mut heights {
            if *h == unknown {
                *h = view_z - 1;
            }
        }
        PackedInt32Array::from(heights.as_slice())
    }

    #[func]
    fn remembered_floor(&self, host: Gd<SimHost>, z: i32) -> PackedInt32Array {
        let host = host.bind();
        let Some(w) = host.world() else {
            return Default::default();
        };
        let Some(colony) = w.communities.first() else {
            return Default::default();
        };
        let mut rows = vec![w.grid.width, w.grid.height];
        let Some(memory) = w.overworld.surveys.get(&colony.id) else {
            // A colony with no report knows no cells, even when another colony has a chart.
            return PackedInt32Array::from(rows.as_slice());
        };
        for (&(x, y, level), &(seen, tile, below, fitting)) in &memory.valley {
            if level == z {
                rows.extend([
                    x,
                    y,
                    i32::from(seen == memory.tick),
                    tile as i32,
                    i32::from(below),
                    fitting,
                ]);
            }
        }
        PackedInt32Array::from(rows.as_slice())
    }
}
