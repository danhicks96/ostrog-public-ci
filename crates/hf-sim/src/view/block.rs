//! What a block is and what it holds (#413): ground or rock and its material, water and its
//! kind (fresh, swamp, or none), its quality (#453: fresh, salt, brackish) and depth, the
//! minerals and metals in the tile and rock
//! column, what grows or lies on it, built structures and historical traces, and local
//! temperature and shelter. Respects `view::known` so unfound things are not spoiled.

use crate::floods::WaterKind;
use crate::ids::TilePos;
use crate::items::ItemPlace;
use crate::plants::PlantKind;
use crate::structures::StructureKind;
use crate::world::gen::biome::Biome;
use crate::world::material::{self, MaterialKind};
use crate::world::{Material, Shape, TileFlags};
use crate::world_state::World;

#[derive(Clone, Debug, Default)]
pub struct BlockInfo {
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub surface_z: i32,
    pub title: String,
    pub biome: String,
    pub ground: String,
    pub water: String,
    pub water_kind: String,
    pub water_depth: i32,
    /// What the water is to drink (#453): "fresh", "salt", "brackish", or "none".
    pub water_quality: String,
    pub minerals: Vec<String>,
    pub temperature: String,
    pub plants: Vec<String>,
    pub items: Vec<String>,
    pub structures: Vec<String>,
    pub notes: Vec<String>,
}

pub fn inspect(w: &World, p: TilePos) -> BlockInfo {
    if !w.grid.in_bounds(p) {
        return BlockInfo {
            x: p.x,
            y: p.y,
            z: p.z,
            title: format!("Outside the local map ({}, {}, z{})", p.x, p.y, p.z),
            ..Default::default()
        };
    }

    let surface_z = w.grid.ground_z(p.x, p.y);
    let biome_enum = w.grid.biome_at(p.x, p.y);
    let biome = biome_enum
        .map(|b| b.label().to_string())
        .unwrap_or_default();

    let (title, ground) = describe_ground(w, p, surface_z);
    let (water, water_kind, water_depth, water_quality) = describe_water(w, p, biome_enum);
    let minerals = describe_minerals(w, p, surface_z);
    let temperature = describe_temperature(w, p, surface_z);
    let plants = describe_plants(w, p);
    let items = describe_items(w, p);
    let structures = describe_structures(w, p);
    let notes = describe_notes(w, p);

    BlockInfo {
        x: p.x,
        y: p.y,
        z: p.z,
        surface_z,
        title,
        biome,
        ground,
        water,
        water_kind,
        water_depth,
        water_quality,
        minerals,
        temperature,
        plants,
        items,
        structures,
        notes,
    }
}

fn mat_name(w: &World, p: TilePos, m: Material) -> String {
    if let Some(built) = w.made_of(p) {
        return w.grid.materials.get(built).name.clone();
    }
    w.grid.materials.get(m).name.clone()
}

fn describe_ground(w: &World, p: TilePos, surface_z: i32) -> (String, String) {
    let t = w.grid.get(p);
    let below_p = p.offset(0, 0, -1);
    let below_t = w.grid.get(below_p);

    let rel_z = if p.z == surface_z + 1 {
        "surface".to_string()
    } else if p.z > surface_z + 1 {
        format!("{} above ground", p.z - (surface_z + 1))
    } else {
        let d = (surface_z + 1) - p.z;
        format!("depth {d} below ground")
    };

    if t.is_water() {
        let floor_name = mat_name(w, below_p, below_t.material);
        let title = format!("Water at ({}, {}, z{})", p.x, p.y, p.z);
        let ground = format!("Water over {floor_name} bed ({rel_z})");
        return (title, ground);
    }

    if t.flags.has(TileFlags::DOOR) {
        let mname = mat_name(w, p, t.material);
        let title = format!("Door of {mname} ({}, {}, z{})", p.x, p.y, p.z);
        return (title, format!("Built {mname} doorway ({rel_z})"));
    }

    match t.shape {
        Shape::Wall => {
            let mname = mat_name(w, p, t.material);
            let kind = w.grid.materials.get(t.material).kind;
            let what = match kind {
                MaterialKind::Built => format!("Built {mname} wall"),
                MaterialKind::Ore => format!("Solid {mname} seam"),
                MaterialKind::Soil => format!("Solid {mname} bank"),
                MaterialKind::Wood => format!("Built {mname} wall"),
                _ => format!("Solid {mname} rock"),
            };
            let title = format!("{what} ({}, {}, z{})", p.x, p.y, p.z);
            let mut ground = format!("{what} ({rel_z})");
            if crate::aquifer::wet(w, p) {
                ground.push_str("; water-bearing seam");
            } else if crate::aquifer::damp(w, p) {
                ground.push_str("; damp rock (water behind it)");
            }
            (title, ground)
        }
        Shape::Floor => {
            let mname = mat_name(w, p, t.material);
            let title = format!("Built {mname} floor ({}, {}, z{})", p.x, p.y, p.z);
            let mut ground = format!("Built floor of {mname} ({rel_z})");
            append_surface_marks(w, p, &mut ground);
            (title, ground)
        }
        Shape::RampUp | Shape::RampTop => {
            let mname = mat_name(w, p, t.material);
            let title = format!("Slope of {mname} ({}, {}, z{})", p.x, p.y, p.z);
            (title, format!("Walkable slope of {mname} ({rel_z})"))
        }
        Shape::StairsUp | Shape::StairsDown | Shape::StairsBoth => {
            let mname = mat_name(w, p, t.material);
            let kind_str = match t.shape {
                Shape::StairsUp => "Stairs up",
                Shape::StairsDown => "Stairs down",
                _ => "Stairs up and down",
            };
            let title = format!("{kind_str} in {mname} ({}, {}, z{})", p.x, p.y, p.z);
            (title, format!("{kind_str}, cut in {mname} ({rel_z})"))
        }
        Shape::Ladder => {
            let mname = mat_name(w, p, t.material);
            let title = format!("Ladder of {mname} ({}, {}, z{})", p.x, p.y, p.z);
            (title, format!("Wooden ladder of {mname} ({rel_z})"))
        }
        Shape::Open => {
            if below_t.shape.is_solid() || below_t.shape == Shape::Floor {
                let floor_mat = if let Some(&worn_from) = w.paths.get(&(p.x, p.y)) {
                    if p.z == surface_z + 1 && below_t.material == Material::PACKED_EARTH {
                        let from_name = w.grid.materials.get(worn_from).name.clone();
                        format!("worn path (trodden from {from_name})")
                    } else {
                        mat_name(w, below_p, below_t.material)
                    }
                } else {
                    mat_name(w, below_p, below_t.material)
                };
                let where_kind = if let Some(cav) = w.grid.caverns.iter().find(|c| c.contains(p)) {
                    format!("floor of {}", cav.name)
                } else if w.dug.contains(&p) {
                    "dug gallery floor".to_string()
                } else if p.z <= surface_z {
                    "natural cave floor".to_string()
                } else {
                    "open ground".to_string()
                };
                let cap = uppercase_first(&floor_mat);
                let title = format!("{cap} ({}, {}, z{})", p.x, p.y, p.z);
                let mut ground = format!("{cap} — {where_kind} ({rel_z})");
                append_surface_marks(w, p, &mut ground);
                // The weather's wear on the cover (#289's ground overlay), named
                // only where the client's own rules draw it (world_view.gd) and
                // only on the surface — a dug gallery floor keeps its even air
                // (FIXLOG #346) and takes no rain or frost.
                if p.z == surface_z + 1 {
                    let gp = p.offset(0, 0, -1);
                    if crate::cover::snow_lies(w, gp) {
                        ground.push_str("; snow lying");
                    } else {
                        match crate::cover::cover_at(w, gp) {
                            Material::FROZEN_SOIL => ground.push_str("; frozen hard"),
                            Material::WET_SOIL => ground.push_str("; wet with rain"),
                            _ => {}
                        }
                    }
                }
                (title, ground)
            } else {
                for peel in 1..=6 {
                    let bz = p.z - peel;
                    if bz < w.grid.z_min {
                        break;
                    }
                    let bp = TilePos::new(p.x, p.y, bz);
                    let bt = w.grid.get(bp);
                    if bt.is_water() {
                        let floor_p = bp.offset(0, 0, -1);
                        let floor_name = mat_name(w, floor_p, w.grid.get(floor_p).material);
                        let title = format!(
                            "Water at ({}, {}, z{}, {peel} below z{})",
                            p.x, p.y, bz, p.z
                        );
                        let ground = format!(
                            "Water over {floor_name} bed at z{bz} ({peel} level{} below view z{})",
                            if peel == 1 { "" } else { "s" },
                            p.z
                        );
                        return (title, ground);
                    }
                    if bt.shape.is_solid() || bt.shape == Shape::Floor {
                        let mname = mat_name(w, bp, bt.material);
                        let cap = uppercase_first(&mname);
                        let drop_levels = p.z - bz;
                        let title = format!(
                            "{cap} ({}, {}, z{}, {drop_levels} below z{})",
                            p.x, p.y, bz, p.z
                        );
                        let mut ground = format!(
                            "{cap}: ground at z{} ({drop_levels} level{} below view z{})",
                            bz,
                            if drop_levels == 1 { "" } else { "s" },
                            p.z
                        );
                        append_surface_marks(w, TilePos::new(p.x, p.y, bz), &mut ground);
                        return (title, ground);
                    }
                }
                let title = format!("Open air ({}, {}, z{})", p.x, p.y, p.z);
                (title, format!("Open air ({rel_z})"))
            }
        }
    }
}

fn append_surface_marks(w: &World, p: TilePos, out: &mut String) {
    if w.scorched.contains_key(&p) || w.scorched.contains_key(&p.offset(0, 0, -1)) {
        out.push_str("; scorched by fire");
    }
    if w.seeping.iter().any(|(sp, _)| *sp == p) {
        out.push_str("; seeping underground water");
    }
    if w.dug.contains(&p) && !crate::underground::held(w, p, None) {
        out.push_str("; unpropped roof above");
    }
}

fn describe_water(w: &World, p: TilePos, biome: Option<Biome>) -> (String, String, i32, String) {
    let mut water_p = None;
    if w.grid.get(p).is_water() {
        water_p = Some(p);
    } else if w.grid.get(p.offset(0, 0, -1)).is_water() {
        water_p = Some(p.offset(0, 0, -1));
    } else if w.grid.get(p).shape == Shape::Open {
        for peel in 2..=6 {
            let bz = p.z - peel;
            if bz < w.grid.z_min {
                break;
            }
            let bp = TilePos::new(p.x, p.y, bz);
            let bt = w.grid.get(bp);
            if bt.is_water() {
                water_p = Some(bp);
                break;
            }
            if bt.shape.is_solid() || bt.shape == Shape::Floor {
                break;
            }
        }
    }

    let Some(wp) = water_p else {
        if w.seeping.iter().any(|(sp, _)| *sp == p) {
            return (
                "Fresh water seeping from the rock wall (ankle-deep)".to_string(),
                "fresh".to_string(),
                0,
                "fresh".to_string(),
            );
        }
        if crate::aquifer::wet(w, p) {
            return (
                "Water-bearing rock seam (cuts here will seep)".to_string(),
                "fresh".to_string(),
                0,
                "fresh".to_string(),
            );
        }
        if crate::aquifer::damp(w, p) {
            return (
                "Damp rock (water stands behind an adjacent face)".to_string(),
                "fresh".to_string(),
                0,
                "fresh".to_string(),
            );
        }
        return (
            "Dry (no standing water)".to_string(),
            "none".to_string(),
            0,
            "none".to_string(),
        );
    };

    let mut z_low = wp.z;
    while z_low > w.grid.z_min && w.grid.get(TilePos::new(wp.x, wp.y, z_low - 1)).is_water() {
        z_low -= 1;
    }
    let mut z_high = wp.z;
    while z_high < w.grid.z_max && w.grid.get(TilePos::new(wp.x, wp.y, z_high + 1)).is_water() {
        z_high += 1;
    }
    let depth = (z_high - z_low + 1).max(1);
    let tile = w.grid.get(wp);
    let deep_flag = tile.flags.has(TileFlags::DEEP) || depth >= 2;
    let frozen = w
        .grid
        .get(TilePos::new(wp.x, wp.y, z_high))
        .flags
        .has(TileFlags::SNOW)
        && w.temp_at(TilePos::new(wp.x, wp.y, z_high)) < -2.0;

    let body = w
        .waters
        .bodies
        .iter()
        .find(|b| b.tiles.iter().any(|tp| tp.x == wp.x && tp.y == wp.y))
        .or_else(|| {
            w.waters
                .flood_of
                .get(&wp)
                .or_else(|| w.waters.left.get(&wp))
                .and_then(|&idx| w.waters.bodies.get(idx as usize))
        });

    let below_mat = w.grid.get(TilePos::new(wp.x, wp.y, z_low - 1)).material;
    let is_swamp = biome == Some(Biome::Fen)
        || below_mat == Material::PEAT
        || (below_mat == Material::MUD
            && matches!(body.map(|b| b.kind), Some(WaterKind::Pond) | None));

    // Fouling (part 2 of #453, FIXLOG 2026-10-30) rides over the kind: a fouled tile reads
    // as foul whatever its ground or body. Salt and brackish water foul and clear the same
    // as fresh, so the quality word below still says what it is to drink.
    let fouled = w.waters.foul.contains_key(&wp);
    let (kind_word, kind_detail) = if fouled {
        ("foul", "foul water")
    } else if is_swamp {
        ("swamp", "peaty swamp water")
    } else if w.waters.left.contains_key(&wp) || w.flooded.contains(&wp) {
        ("fresh", "standing floodwater")
    } else if wp.z <= w.grid.ground_z(wp.x, wp.y) - 2 {
        ("fresh", "fresh underground cavern water")
    } else {
        ("fresh", "clear fresh water")
    };

    let body_note = if let Some(b) = body {
        let bkind = match b.kind {
            WaterKind::Stream => "running stream",
            WaterKind::Lake => "lake",
            WaterKind::Pond => "pond",
        };
        let stage = if b.stage > 0.65 {
            ", in spate"
        } else if b.stage > 0.3 {
            ", running high"
        } else if b.stage < -0.5 {
            ", low in drought"
        } else {
            ""
        };
        format!(" ({bkind}{stage})")
    } else {
        String::new()
    };

    let depth_words = if frozen {
        format!("chilled in frost, {depth} level(s) deep")
    } else if deep_flag {
        format!("deep ({depth} level(s), unfordable)")
    } else {
        format!("shallow ({depth} level, fordable)")
    };

    let fish_stock = w
        .fish
        .get(&(wp.x / 16, wp.y / 16))
        .map(|&(s, _)| s)
        .unwrap_or(0.0);
    let fish_note = if fish_stock >= 3.0 {
        "; holds fish"
    } else if fish_stock > 0.5 {
        "; a few fish"
    } else {
        ""
    };

    let text = format!(
        "{}{body_note} — {depth_words}{fish_note}",
        uppercase_first(kind_detail)
    );
    // The quality the body was surveyed with (#453, FIXLOG 2026-10-29): the sea salt, a
    // lagoon brackish, unbodied water (cavern pools, unmapped floods) fresh. Fouling (part 2,
    // FIXLOG 2026-10-30) rides over `kind_word` above, not over this.
    let quality_word = body.map(|b| b.quality.word()).unwrap_or("fresh");
    (text, kind_word.to_string(), depth, quality_word.to_string())
}

fn describe_minerals(w: &World, p: TilePos, surface_z: i32) -> Vec<String> {
    let mut out = Vec::new();

    for check_p in [p, p.offset(0, 0, -1)] {
        if !w.grid.in_bounds(check_p) {
            continue;
        }
        let t = w.grid.get(check_p);
        if !t.shape.is_solid() {
            continue;
        }
        let def = w.grid.materials.get(t.material);
        if def.kind == MaterialKind::Ore {
            let where_tag = if check_p == p {
                "In this block"
            } else {
                "Underfoot"
            };
            if t.material == Material::GEM_ROCK {
                let host = w.grid.rock_at(w.grid.col(check_p.x, check_p.y), check_p.z);
                let gem_mat = material::gem_in(w.meta.seed, check_p, host);
                let gem_name = &w.grid.materials.get(gem_mat).name;
                out.push(format!(
                    "{where_tag}: gem-bearing rock (holds rough {gem_name}s)"
                ));
            } else if let Some(metal) = def.becomes {
                let metal_name = &w.grid.materials.get(metal).name;
                out.push(format!(
                    "{where_tag}: {} (smelts to {metal_name})",
                    def.name
                ));
            } else if t.material == Material::COAL {
                out.push(format!(
                    "{where_tag}: coal seam (fuel for smelter and forge)"
                ));
            } else {
                out.push(format!("{where_tag}: {}", def.name));
            }
        }
    }

    let mut col_ores: std::collections::BTreeMap<String, (i32, i32)> =
        std::collections::BTreeMap::new();
    for z in w.grid.z_min..=surface_z.min(w.grid.z_max) {
        let cp = TilePos::new(p.x, p.y, z);
        let ct = w.grid.get(cp);
        if !ct.shape.is_solid() {
            continue;
        }
        let def = w.grid.materials.get(ct.material);
        if def.kind == MaterialKind::Ore {
            let label = if ct.material == Material::GEM_ROCK {
                let host = w.grid.rock_at(w.grid.col(cp.x, cp.y), cp.z);
                let gem_mat = material::gem_in(w.meta.seed, cp, host);
                format!("rough {}s", w.grid.materials.get(gem_mat).name)
            } else if let Some(metal) = def.becomes {
                format!("{} ({})", def.name, w.grid.materials.get(metal).name)
            } else {
                def.name.clone()
            };
            col_ores
                .entry(label)
                .and_modify(|(lo, hi)| {
                    *lo = (*lo).min(z);
                    *hi = (*hi).max(z);
                })
                .or_insert((z, z));
        }
    }
    for (ore_label, (z_lo, z_hi)) in col_ores {
        if z_lo == z_hi {
            out.push(format!("In column at z{z_lo}: {ore_label}"));
        } else {
            out.push(format!("In column z{z_lo}..z{z_hi}: {ore_label}"));
        }
    }

    let col = w.grid.col(p.x, p.y);
    if let Some(&upper_rock) = w.grid.bedrock.get(col) {
        let upper_name = &w.grid.materials.get(upper_rock).name;
        if let Some(&(lower_rock, split_z, deep_z)) = w.grid.strata.get(col) {
            let lower_name = &w.grid.materials.get(lower_rock).name;
            let deep_mat = w
                .grid
                .deep
                .get(col)
                .map(|&(m, _)| m)
                .unwrap_or(Material::BASALT);
            let deep_name = &w.grid.materials.get(deep_mat).name;
            let mut parts: Vec<String> = vec![format!("{upper_name} above z{split_z}")];
            match w.grid.bands.get(col) {
                Some(bands) if !bands.is_empty() => {
                    // Shallow to deep: the first middle rock down to the shallowest band's
                    // top, then each band, the deepest naming the bottom of the middle.
                    let shallow_top = bands.last().map(|&(_, t)| t).unwrap_or(deep_z);
                    parts.push(format!("{lower_name} above z{shallow_top}"));
                    for (i, &(rock, _)) in bands.iter().enumerate().rev() {
                        let name = &w.grid.materials.get(rock).name;
                        if i == 0 {
                            parts.push(format!("{name} down to z{deep_z}"));
                        } else {
                            parts.push(format!("{name} above z{}", bands[i - 1].1));
                        }
                    }
                }
                _ => parts.push(format!("{lower_name} down to z{deep_z}")),
            }
            parts.push(format!("{deep_name} in the deep"));
            out.push(format!("Strata here: {}", parts.join(", ")));
        } else {
            out.push(format!("Bedrock here: {upper_name}"));
        }
    }

    out
}

fn describe_temperature(w: &World, p: TilePos, surface_z: i32) -> String {
    let below = crate::underground::below(w, p);
    let roofed = crate::building::roofed(w, p);
    // The simulation's own reading at this tile (#268): the valley temperature lapsed by
    // height above (or warmth by depth below) the founding site's ground, so a dug cellar
    // reads warmer than a winter surface and a height colder; the thin place's wrong
    // weather overlays it as it always did.
    let mut base_temp = w.temp_at(p);
    for &(tp, _, cold) in &w.thin.patches {
        if tp.dist(p) <= 4 {
            base_temp += if cold { -10.0 } else { 10.0 };
        }
    }

    let near_hearth = w.structures.iter().any(|s| {
        s.active
            && matches!(
                s.kind,
                StructureKind::Hearth
                    | StructureKind::Smelter
                    | StructureKind::Furnace
                    | StructureKind::Kiln
            )
            && s.pos.z == p.z
            && s.pos.dist(p) <= 5
    });
    let temp_c = if near_hearth {
        base_temp.max(18.0)
    } else {
        base_temp
    };

    let shelter = if below && p.z < surface_z {
        let light = crate::underground::light_at(w, p);
        let light_str = if light >= 0.5 {
            "lit"
        } else if light >= 0.15 {
            "dimly lit"
        } else {
            "dark"
        };
        format!("underground ({light_str})")
    } else if roofed {
        if let Some(r) = w.roofs.get(&p).filter(|r| r.snow > 0.0) {
            format!("roofed indoors (snow {:.0}h)", r.snow)
        } else {
            "roofed indoors".to_string()
        }
    } else {
        "open to the sky".to_string()
    };

    let warmth = if near_hearth {
        ", warmed by a fire"
    } else {
        ""
    };
    format!("{temp_c:.0} °C — {shelter}{warmth}")
}

fn describe_plants(w: &World, p: TilePos) -> Vec<String> {
    let mut out = Vec::new();
    for check_p in [p, p.offset(0, 0, 1)] {
        for pl in w.plants.iter().filter(|pl| pl.alive && pl.pos == check_p) {
            let name = match pl.kind {
                PlantKind::Tree | PlantKind::FungusTree | PlantKind::BoneTree => {
                    let wood = material::wood_of(pl.kind, pl.variant);
                    format!("{} tree", w.grid.materials.get(wood).name)
                }
                PlantKind::Stump => {
                    let wood = material::wood_of(PlantKind::Tree, pl.variant);
                    format!("{} stump", w.grid.materials.get(wood).name)
                }
                PlantKind::FallenLog => {
                    let wood = material::wood_of(PlantKind::Tree, pl.variant);
                    format!("fallen {} log", w.grid.materials.get(wood).name)
                }
                other => other.label().to_string(),
            };
            let pct = (pl.growth * 100.0).round() as i32;
            let status = if !pl.kind.grows() {
                "deadwood".to_string()
            } else if pl.kind.yields_food() && pl.fruit >= 0.8 {
                format!("{pct}% grown, ready to harvest")
            } else {
                format!("{pct}% grown")
            };
            out.push(format!("{} ({status})", uppercase_first(&name)));
        }
    }
    out
}

fn describe_items(w: &World, p: TilePos) -> Vec<String> {
    let mut out = Vec::new();
    for it in w
        .items
        .iter()
        .filter(|it| it.place == ItemPlace::Ground(p) && crate::view::known::item_told(w, it))
    {
        if let Some(story_line) = crate::legend::describe(w, it.id) {
            out.push(uppercase_first(&story_line));
        } else {
            let name = crate::items::stack_name(&w.grid.materials, it.kind, it.material);
            let tier = crate::items::ItemKind::tier(it.quality);
            let qual = if tier >= 3 {
                format!(" ({})", crate::items::ItemKind::tier_name(tier))
            } else {
                String::new()
            };
            let spoil = if it.rot >= 0.6 {
                ", going bad"
            } else if it.rot >= 0.25 {
                ", ageing"
            } else {
                ""
            };
            out.push(format!("{} {name}{qual}{spoil}", it.count));
        }
    }
    out
}

fn describe_structures(w: &World, p: TilePos) -> Vec<String> {
    let mut out = Vec::new();
    for s in w.structures.iter().filter(|s| s.pos == p) {
        let mat = &w.grid.materials.get(s.material).name;
        let kind_words = split_camel(&format!("{:?}", s.kind));
        let state = if s.progress < 1.0 {
            format!("building ({:.0}%)", s.progress * 100.0)
        } else if s.active {
            "active".to_string()
        } else {
            "complete".to_string()
        };
        out.push(format!(
            "{} of {mat} ({state})",
            uppercase_first(&kind_words)
        ));
    }
    if w.traps.iter().any(|tr| tr.pos == p) {
        out.push("Snare set here".to_string());
    }
    for d in w.designations.iter().filter(|d| !d.done && d.pos == p) {
        // Why nobody has taken it yet, not just that it exists (#292 item 6, WP-08 8.6).
        out.push(super::waiting::line(w, d));
    }
    out
}

fn describe_notes(w: &World, p: TilePos) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(n) = crate::thin::note(w, p) {
        out.push(n);
    }
    if let Some(n) = crate::project::note(w, p) {
        out.push(n);
    }
    if let Some(n) = crate::engrave::note(w, p) {
        out.push(n);
    }
    if let Some(n) = crate::building::house_note(w, p) {
        out.push(n);
    }
    if let Some(n) = crate::deep::note(w, p) {
        out.push(n);
    }
    if let Some(n) = crate::accounts::note(w, p) {
        out.push(n);
    }
    for a in w
        .agents
        .iter()
        .filter(|a| !a.alive && a.pos == p && !crate::view::known::is_unfound(w, a.id))
    {
        let how = a.death.as_deref().unwrap_or("died here");
        out.push(format!("Body of {} ({how})", a.name));
    }
    out
}

fn split_camel(s: &str) -> String {
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() && i > 0 {
            out.push(' ');
        }
        out.push(c.to_ascii_lowercase());
    }
    out
}

fn uppercase_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::gen::GenParams;
    use crate::world::Tile;

    #[test]
    fn block_inspect_reports_water_minerals_and_ground() {
        let mut w = World::generate(1, GenParams::small());
        let p = TilePos::new(20, 20, w.grid.ground_z(20, 20));
        // Place iron ore in the block and inspect it.
        w.grid.set(p, Tile::solid(Material::IRON_ORE));
        let info = inspect(&w, p);
        assert!(
            info.ground.contains("iron ore"),
            "ground should name iron ore: {}",
            info.ground
        );
        assert!(
            info.minerals.iter().any(|m| m.contains("smelts to iron")),
            "minerals should report what the ore smelts to: {:?}",
            info.minerals
        );

        // Place fresh water in a meadow/grassland column and check classification & depth.
        let wp = p.offset(0, 0, 1);
        let col = w.grid.col(p.x, p.y);
        if let Some(b) = w.grid.biome.get_mut(col) {
            *b = Biome::Meadow;
        }
        let mut wt = Tile::AIR;
        wt.material = Material::WATER;
        wt.flags = TileFlags(TileFlags::WATER);
        w.grid.set(wp, wt);
        let winfo = inspect(&w, wp);
        assert_eq!(winfo.water_kind, "fresh");
        assert!(winfo.water_depth >= 1);
        assert!(
            winfo.water.to_lowercase().contains("fresh"),
            "water text: {}",
            winfo.water
        );

        // In a Fen biome, standing water classifies as swamp.
        if let Some(b) = w.grid.biome.get_mut(col) {
            *b = Biome::Fen;
        }
        assert_eq!(inspect(&w, wp).water_kind, "swamp");

        // Rot beside water fouls it (#453 part 2): the meat lies in the water's own tile, the
        // fouling the hourly system would set from it stands for the test (inspect does not
        // run systems).
        let id = w.alloc_item_id();
        w.items.push(crate::items::Item {
            id,
            kind: crate::items::ItemKind::RawMeat,
            count: 1,
            place: ItemPlace::Ground(wp),
            quality: 0.5,
            material: Material::MEAT,
            story: None,
            rot: 0.9,
        });
        w.waters
            .foul
            .insert(wp, crate::systems::water_quality::FOUL_HOURS);
        let rot_info = inspect(&w, wp);
        assert_eq!(rot_info.water_kind, "foul");
        assert!(rot_info.water.to_lowercase().contains("foul"));

        // A looked-down-to ground tile is reported at its actual z, not one above it.
        let ground_p = TilePos::new(21, 21, w.grid.ground_z(21, 21));
        let middle_p = ground_p.offset(0, 0, 1);
        let view_p = ground_p.offset(0, 0, 2);
        w.grid.set(ground_p, Tile::solid(Material::PEAT));
        w.grid.set(middle_p, Tile::AIR);
        w.grid.set(view_p, Tile::AIR);
        let looked = inspect(&w, view_p);
        assert!(
            looked
                .ground
                .contains(&format!("ground at z{}", ground_p.z)),
            "looked-down ground should name its actual z: {}",
            looked.ground
        );
        assert!(!looked
            .ground
            .contains(&format!("ground at z{}", ground_p.z + 1)));

        // Gem-bearing rock is visible in the mineral read model.
        let gem_p = TilePos::new(22, 22, w.grid.ground_z(22, 22));
        w.grid.set(gem_p, Tile::solid(Material::GEM_ROCK));
        let gem_info = inspect(&w, gem_p);
        assert!(
            gem_info
                .minerals
                .iter()
                .any(|m| m.contains("gem-bearing rock")),
            "minerals should report gem-bearing rock: {:?}",
            gem_info.minerals
        );
    }

    #[test]
    fn favourites_persist_in_chronicle_without_changing_world_digest() {
        let w = World::generate(1, GenParams::small());
        let d_before = crate::digest::digest(&w);
        let mut chron = crate::chronicle::Chronicle::in_memory().unwrap();
        assert!(chron.favourites().is_empty());
        let mut favs = std::collections::BTreeSet::new();
        favs.insert(crate::ids::AgentId(0));
        favs.insert(crate::ids::AgentId(2));
        chron.set_favourites(&favs).unwrap();
        assert_eq!(chron.favourites(), favs);
        assert_eq!(crate::digest::digest(&w), d_before);

        let tmp = tempfile::tempdir().unwrap();
        let save_dir = tmp.path().join("fav_test.hfsave");
        crate::save::write(&w, &mut chron, &save_dir).unwrap();
        let (w2, chron2) = crate::save::read(&save_dir).unwrap();
        assert_eq!(crate::digest::digest(&w2), d_before);
        assert_eq!(chron2.favourites(), favs);
    }
}
