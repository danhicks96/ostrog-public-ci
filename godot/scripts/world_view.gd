extends Node2D

## Draws the world at one view z: ground, peels below, walls with faces, cliffs, water,
## plants, structures, items, inhabitants, and per-tile lighting. Immediate mode (_draw) in
## Phase 1; TileMapLayer batching is a Phase 3 performance task if measurements need it.

signal world_replaced

const TILE := 32
const Rules := preload("res://scripts/rules.gd")
const AGENT_STRIDE := 25
const DESIG_STRIDE := 8
const CAVE_FLOORED := ["soil", "grass", "clay", "pale_stone", "warm_stone", "grey_stone", "dark_stone", "black_stone"]
const SNOWABLE := ["grass", "soil", "peat", "clay", "sand", "gravel", "mud", "wildflower", "frozen_soil", "mossy", "scree", "wet_soil", "packed_earth", "bone_moss"]
const FAMILY := {
	0: "air", 1: "soil", 2: "clay", 3: "sand", 4: "gravel", 5: "pale_stone", 6: "warm_stone",
	7: "grey_stone", 8: "dark_stone", 9: "black_stone", 10: "ore_iron", 11: "ore_copper", 12: "ore_coal",
	13: "water", 14: "wood_floor", 15: "stone_floor", 16: "thatch", 17: "grass", 18: "peat", 19: "snow",
	20: "mud", 21: "wildflower", 22: "mossy", 23: "packed_earth", 24: "frozen_soil", 25: "wet_soil",
	26: "scree", 27: "ore_gem", 28: "marble", 29: "ore_tin", 30: "ore_silver", 31: "ore_gold",
	61: "milkstone", 62: "bone_moss", 63: "black_stone", 64: "dark_stone",
}
const WALL_MAT := {14: "wood", 15: "stone_block", 16: "thatch"}
## Edge blending: where two of these families meet at the same level, the later one in the list
## spills over its neighbour through the pieces in terrain/edges/<family>.png (tools/art/terrain.py,
## same order there). Water is lowest and never spills; built floors, cave floor and ores keep
## their hard edges and take no part.
const BLEND_ORDER := ["water", "black_stone", "dark_stone", "grey_stone", "warm_stone", "pale_stone",
	"gravel", "scree", "mud", "wet_soil", "sand", "clay", "peat", "packed_earth", "soil", "frozen_soil",
	"mossy", "bone_moss", "grass", "wildflower", "ash", "snow"]
# Indices into a 3x3 neighbourhood listed row by row (4 is the tile itself).
const EDGE_IDX := [1, 5, 7, 3]                          # N E S W
const CORNER_IDX := [2, 8, 6, 0]                        # NE SE SW NW
const CORNER_SIDES := [[1, 5], [7, 5], [7, 3], [1, 3]]  # the two sides of each corner, N/S first
const CORNER_BEYOND := [[0, 8], [2, 6], [8, 0], [2, 6]] # the cells beyond those sides
const NB_BITS := [0, 1, 2, 3, 5, 6, 7, 8]               # bit i of a neighbour set is this cell
const FALLBACK := {
	"air": Color(0, 0, 0, 0), "soil": Color(0.42, 0.30, 0.18), "clay": Color(0.55, 0.40, 0.30),
	"sand": Color(0.80, 0.72, 0.50), "gravel": Color(0.55, 0.52, 0.48), "pale_stone": Color(0.75, 0.72, 0.62),
	"warm_stone": Color(0.72, 0.58, 0.42), "grey_stone": Color(0.50, 0.50, 0.52), "dark_stone": Color(0.30, 0.32, 0.36),
	"black_stone": Color(0.18, 0.18, 0.20), "ore_iron": Color(0.55, 0.35, 0.25), "ore_copper": Color(0.60, 0.45, 0.25),
	"ore_coal": Color(0.12, 0.12, 0.12), "water": Color(0.20, 0.40, 0.65), "wood_floor": Color(0.50, 0.35, 0.20),
	"stone_floor": Color(0.62, 0.60, 0.58), "thatch": Color(0.70, 0.60, 0.30), "grass": Color(0.32, 0.52, 0.22),
	"peat": Color(0.25, 0.20, 0.12), "snow": Color(0.92, 0.94, 0.97), "mud": Color(0.35, 0.28, 0.20),
	"cave_floor": Color(0.28, 0.27, 0.26), "ore_gem": Color(0.22, 0.22, 0.32),
	"marble": Color(0.90, 0.89, 0.86), "ore_tin": Color(0.36, 0.38, 0.42),
	"ore_silver": Color(0.66, 0.67, 0.70), "ore_gold": Color(0.70, 0.58, 0.28),
}
const UNDERGROUND_SEEP_VARIANTS := 8
const UNDERGROUND_STRATA_VARIANTS := 8
const STRUCT_NAME := ["bed", "table", "chair", "stool", "chest", "shelf", "hearth", "workbench", "loom", "anvil",
	"cooking_pot", "barrel", "crate", "well", "fence_post", "lamp", "stockpile", "carpenter_bench", "smelter",
	"furnace", "kiln", "grave", "support", "portal_stone", "ward_stone", "tent", "nest", "shrine", "lean_to"]
const ITEM_NAME := ["log", "stone_block", "iron_ore", "copper_ore", "coal", "berries", "grain_sheaf", "root",
	"raw_meat", "meal", "tool", "cloth", "hide", "firewood", "plank", "copper_bar", "iron_bar", "fibre", "clothes",
	"poultice", "saw", "hammer", "sledge_hammer", "chisel", "needle", "bucket", "torch", "record", "gem", "ore",
	"bar", "weapon", "armour", "clay", "sand", "vessel", "leather", "ale", "wine", "jewel", "figurine",
	"rough_stone", "door", "bed_frame", "table", "stool", "chest", "thatch", "dye"]
## Carved walls (sim.engravings): x, y, z, quality tier, 4 ints each.
var engravings: PackedInt32Array = PackedInt32Array()
## Ground items come 8 ints each (id, x, y, z, kind, count, material, quality tier 0..5).
const ITEM_STRIDE := 8
## Material names as icon suffixes by id (sim.material_slugs), fetched once per world.
var mat_slugs: PackedStringArray = PackedStringArray()
## What each built wall at the view level was made of (sim.built_of, C56): Vector2i -> material id.
var built := {}
## The old hamlets' house walls (sim.ruins, #202): Vector3i -> 1 a ruined wall still up, 0 where
## one fell and its stones lie. They change only as a ruin falls or is built over; fetched
## every five seconds.
var ruins := {}
var _ruins_ms := -100000
const PLANT_KIND := ["tree", "shrub", "berry_bush", "reeds", "grain", "root", "mushrooms", "herbs", "fern",
	"vine", "root_crops", "flowering_plant", "wildflowers", "stump", "fallen_log", "fungus_tree",
	"bone_tree", "alder", "ash", "yew", "willow", "crab_apple", "hazel", "bilberry", "wild_strawberry",
	"bramble", "wild_garlic", "chanterelle", "cattail", "death_cap", "hemlock", "nightshade",
	"lily_of_the_valley", "nettle", "woad", "madder", "weld", "yarrow", "comfrey", "barley", "oats",
	"rye", "peas", "flax", "thistle"]
const ANIMAL_NAME := ["person", "deer", "boar", "wolf", "hare", "fox", "crow", "bear", "duck", "goat", "hart", "bat", "crawler", "burrower", "slime", "rats", "centipede", "blind_cave_bear", "cave_croc"]
const OUTFIT := ["tunic", "dress", "apron", "cloak", "coat", "workshirt", "robe", "vest"]
const BUILD := ["slim", "average", "broad"]
const EQUIPMENT := ["pickaxe", "axe", "basket", "hoe", "bundle", "bucket", "sack", "spear"]

var assets
var sim: SimHost
# How each species is drawn (SimHost.species_art, P3), fetched once.
var species_art: Dictionary = {}
var view_z := 0
var w := 0
var h := 0
var zmin := 0
var zmax := 0
var levels := {}   # z -> PackedInt32Array
var levels_above := {}
var _above_z := -99999
var _above_ms := 0
var ground: PackedInt32Array
var plants: PackedInt32Array
var agents: PackedInt32Array
## Bands of strangers (bridge bands(): 9 ints each) and their lines for the HUD.
var bands: PackedInt32Array
## The thin place (sim thin_view()): stone, strength, hollow, echoes, wrong-weather patches.
var thin: PackedInt32Array
var band_lines: PackedStringArray
var structures: PackedInt32Array
var items: PackedInt32Array
var designations: PackedInt32Array
## Roofs at the view level (bridge roofs(): x, y, material, quality%, snow hours).
var roofs: PackedInt32Array
## Who the player is following or inspecting (main.gd sets it): their house's roof is cut away.
var focus_id := -1
var roof_layer: Node2D
var carried := {}
## What each carried load is made of (an id into mat_slugs), for its icon in hand (#186).
var carried_mat := {}
var hour := 12.0
var anim_t := 0.0
var animation_frozen := false
var swap_mat := {}  # key -> ShaderMaterial cache
var show_rect := Rect2i(0, 0, 0, 0) # tile rect to draw (culling)
var shadow_root: Node2D
var entities
var world_marks
## Names over people (#1428).
var name_tags
var rust_renderer
var rust_renderer_enabled := false
var light
var weather
var fire_fx
var effects
var overlay
var fires: Array = []          # burning tiles in view, Vector2i
var _built_rect := Rect2i()
var _built_z := -99999
var _variety := {}             # Vector2i -> family override (client-only surface variety)
var _variety_key := ""
var scorched := {}             # Vector2i -> hours since it burned
## A small newly surveyed patch only redraws its nearby terrain. The bridge still versions 32-tile simulation chunks.
const CH := 8
const SIM_CHUNK := 32
var _ci: CanvasItem = self      # where the ground routines draw (self, a chunk, or the animated layer)
var _chunk_mode := false
var _dyn_rec: Array = []
var ground_root: Node2D
var faces_root: Node2D
var anim_ground: Node2D
var chunk_nodes := {}           # Vector2i -> [ground, faces]
var chunk_seen := {}            # Vector2i -> version key
var _blend_pri := {}            # family -> index in BLEND_ORDER
# What each tile of the chunk being drawn (plus a one-tile border) shows for edge blending:
# rank in BLEND_ORDER (-1: takes no part), level, variant and dim. Filled once per chunk per
# frame and shared by the ground and faces passes.
var _bg_pri := PackedInt32Array()
var _bg_lvl := PackedInt32Array()
var _bg_var := PackedInt32Array()
var _bg_dim := PackedFloat32Array()
var _bg_x0 := 0
var _bg_y0 := 0
var _bg_w := 0
var _bg_key := Vector3i(-1, -1, -1)

func setup(s: SimHost, a) -> void:
	sim = s
	assets = a
	for i in BLEND_ORDER.size():
		_blend_pri[BLEND_ORDER[i]] = i
	var sz: Vector3i = sim.size()
	w = sz.x
	h = sz.y
	var zr: Vector2i = sim.z_range()
	zmin = zr.x
	zmax = zr.y
	RenderingServer.set_default_clear_color(Color(0.09, 0.11, 0.07))
	ground_root = Node2D.new()
	ground_root.z_index = -3
	add_child(ground_root)
	anim_ground = preload("res://scripts/render/anim_ground.gd").new()
	anim_ground.v = self
	anim_ground.z_index = -2
	add_child(anim_ground)
	faces_root = Node2D.new()
	faces_root.z_index = -1
	add_child(faces_root)
	shadow_root = Node2D.new()
	shadow_root.z_index = 1
	add_child(shadow_root)
	entities = preload("res://scripts/render/entity_layer.gd").new()
	entities.z_index = 2
	add_child(entities)
	rust_renderer_enabled = bool(ProjectSettings.get_setting("ostrog/render/world_renderer", false))
	if rust_renderer_enabled:
		ground_root.visible = false
		faces_root.visible = false
		anim_ground.visible = false
		rust_renderer = preload("res://scripts/render/rust_world_renderer.gd").new()
		rust_renderer.z_index = 2
		add_child(rust_renderer)
	roof_layer = preload("res://scripts/render/roof_layer.gd").new()
	roof_layer.v = self
	add_child(roof_layer)
	world_marks = preload("res://scripts/render/world_marks.gd").new()
	world_marks.z_index = 3
	add_child(world_marks)
	world_marks.init(self)
	name_tags = preload("res://scripts/render/name_tags.gd").new()
	# Above weather, effects and the overlay: a name never sits under falling snow.
	name_tags.z_index = 5
	add_child(name_tags)
	name_tags.init(self, sim)
	entities.init(self, shadow_root)
	if rust_renderer_enabled:
		rust_renderer.setup(self)
	fire_fx = preload("res://scripts/render/fire_fx.gd").new()
	fire_fx.z_index = 3
	add_child(fire_fx)
	fire_fx.init(self)
	effects = preload("res://scripts/render/effects.gd").new()
	effects.z_index = 3
	add_child(effects)
	effects.init(self)
	weather = preload("res://scripts/render/weather_layer.gd").new()
	weather.z_index = 3
	add_child(weather)
	weather.init(self)
	overlay = preload("res://scripts/render/overlay_layer.gd").new()
	overlay.z_index = 4
	add_child(overlay)
	overlay.init(self)
	light = preload("res://scripts/render/light_layer.gd").new()
	add_child(light)
	light.init(self)

## Another world replaced the one this view was drawing (a manual load): forget everything
## cached from the old one, as setup() starts with nothing cached. The chunk keys hold no world
## identity and a fresh load's chunk versions all read the same, so without this the old
## world's ground stays on screen.
func reset_world() -> void:
	world_replaced.emit()
	# 2026-10-30 #414 review: transient effects and id history belong to the old world too.
	effects.reset_world()
	if name_tags != null:
		name_tags.reset()
	mat_slugs = PackedStringArray()
	# A new or loaded world has its own generated kinds (P6): fetch them afresh.
	species_art = {}
	var sz: Vector3i = sim.size()
	w = sz.x
	h = sz.y
	var zr: Vector2i = sim.z_range()
	zmin = zr.x
	zmax = zr.y
	for k in chunk_nodes:
		for n in chunk_nodes[k]:
			n.queue_free()
	chunk_nodes.clear()
	floor_stamp = Vector2i(-2,0)
	chunk_seen.clear()
	_variety.clear()
	_variety_key = ""
	_bg_key = Vector3i(-1, -1, -1)
	_above_z = -99999
	_built_z = -99999
	scorched.clear()
	ruins.clear()
	_ruins_ms = -100000
	focus_id = -1

var observer := ColonyAtlas.new()
var remembered := {}
var projected_levels := {} # Terrain reconstructed from surveyed cells and their recorded support.
var projection_reports := {} # Exact survey buffers that produced each projection.
var lower_records := {}
var visible_spills: Array[Vector3i] = [] # upper and lower endpoints, in pairs
var spill_corners := {} # shared crest positions and downstream normals
var surveyed_floor := false
var floor_stamp := Vector2i(-2,0)

func _read_observed_floor() -> void:
	var tick: int = sim.tick()
	var stamp := Vector2i(floori((tick - 17) / 200.0), view_z)
	if stamp == floor_stamp:
		return
	floor_stamp = stamp
	remembered.clear()
	var cells := observer.remembered_floor(sim, view_z)
	surveyed_floor = cells.size() >= 2
	for i in range(2,cells.size(),6):
		remembered[Vector2i(cells[i],cells[i+1])] = Vector4i(cells[i+2],cells[i+3],cells[i+4],cells[i+5])

	if surveyed_floor:
		var mapped := observer.reported_ground(sim, view_z)
		if ground != mapped:
			_invalidate_terrain_columns(ground, mapped)
			ground = mapped
	_read_projected_levels(cells)

func _read_projected_levels(upper_rows: PackedInt32Array) -> void:
	var previous := projected_levels
	var prior_reports := projection_reports
	var prior_records := lower_records
	projected_levels = {}
	projection_reports = {}
	lower_records = {}
	if not surveyed_floor:
		return
	# The upper report includes the material immediately below, but not its depth,
	# fittings, mutable flags or animation. Only a direct lower report supplies these.
	projection_reports[view_z] = upper_rows
	# Read one extra report for the support underneath a scout standing on the summit.
	var highest := mini(zmax, view_z + 5)
	var above := {}
	for z in range(highest, maxi(zmin - 1, view_z - 7), -1):
		if z == view_z:
			above = remembered
			continue
		var rows := observer.remembered_floor(sim, z)
		projection_reports[z] = rows
		if previous.has(z) and prior_reports[z] == rows and prior_reports.get(z + 1) == projection_reports.get(z + 1):
			projected_levels[z] = previous[z]
			above = prior_records[z]
		else:
			above = _project_level(z, above, rows)
		lower_records[z] = above
	if previous != projected_levels:
		_variety.clear()
		_invalidate_projected_chunks(previous)

func _invalidate_projected_chunks(previous: Dictionary) -> void:
	# A changed support affects terrain blending through a one-tile border. Keep
	# every already drawn chunk whose recorded footprint is identical.
	for z in projected_levels:
		var current: PackedInt32Array = projected_levels[z]
		var old: PackedInt32Array = previous.get(z, PackedInt32Array())
		_invalidate_terrain_columns(old, current)

func _invalidate_terrain_columns(old: PackedInt32Array, current: PackedInt32Array) -> void:
	if old.size() != current.size():
		chunk_seen.clear()
		return
	if old == current:
		return
	for k: Vector2i in chunk_seen.keys():
		var left := maxi(0, k.x * CH - 1)
		var right := mini(w, (k.x + 1) * CH + 1)
		for y in range(maxi(0, k.y * CH - 1), mini(h, (k.y + 1) * CH + 1)):
			var start := y * w + left
			var end := y * w + right
			if old.slice(start, end) != current.slice(start, end):
				chunk_seen.erase(k)
				break

func _project_level(z: int, above: Dictionary, rows: PackedInt32Array) -> Dictionary:
	var tiles := PackedInt32Array()
	tiles.resize(w * h)
	var direct := {}
	for i in range(2, rows.size(), 6):
		var at := Vector2i(rows[i], rows[i + 1])
		var record := Vector4i(rows[i + 2], rows[i + 3], rows[i + 4], rows[i + 5])
		direct[at] = record
		tiles[at.y * w + at.x] = record.y
	for at: Vector2i in above:
		var upper: Vector4i = above[at]
		var lower: Vector4i = direct.get(at, Vector4i.ZERO)
		if direct.has(at) and (lower.x == 1 or upper.x != 1 or mat_of(lower.y) == upper.z):
			continue
		tiles[at.y * w + at.x] = _reported_support(upper.z)
	projected_levels[z] = tiles
	return direct

func _lower_in_sight(x: int, y: int, z: int) -> bool:
	var records: Dictionary = lower_records.get(z, {})
	return records.get(Vector2i(x, y), Vector4i.ZERO).x == 1

static func _reported_support(material: int) -> int:
	if material == 0:
		return 0
	if material == 13:
		return material | (1 << 24)
	return material | (1 << 16)

## Block inspection normally describes the live column, including its lower water and
## minerals. Looking through a gap can only describe a surface known to the colony.
func projected_block(p: Vector3i, info: Dictionary) -> Dictionary:
	var at := Vector2i(p.x, p.y)
	if not surveyed_floor or p.z != view_z or not remembered.has(at):
		return info
	var upper: Vector4i = remembered[at]
	if upper.x != 1 or shape_of(upper.y) != 0 or (flags_of(upper.y) & 3):
		return info
	if upper.z != 0 and upper.z != 13 and info.get("height_observed", false) and p.z <= int(info.surface_z) + 1:
		return info # Inspecting the standing surface, not looking down through a gap.
	var surface := "No surveyed surface below"
	var water := "Unknown below this level"
	for z in range(p.z - 1, maxi(zmin - 1, p.z - 7), -1):
		var tile := tile_at(z, p.x, p.y)
		if (flags_of(tile) & 3) == 0 and shape_of(tile) == 0:
			continue
		if _lower_in_sight(p.x, p.y, z):
			return info
		var material: String = FAMILY.get(mat_of(tile), "surface")
		surface = "%s recorded at z%d" % [material.capitalize(), z]
		if flags_of(tile) & 3:
			water = "Previously observed. Current depth and quality unknown."
		break
	# Items, fittings and weather at the inspected level can still be observed.
	var report := {}
	for key in ["biome", "surface_z", "height_observed", "temperature", "items", "structures", "plants"]:
		if info.has(key):
			report[key] = info[key]
	report["title"] = "Looking below (%d, %d, z%d)" % [p.x, p.y, p.z]
	report["ground"] = surface
	report["water"] = water
	return report

func _refresh_spills() -> void:
	visible_spills.clear()
	spill_corners.clear()
	if not surveyed_floor:
		return
	var edges := observer.visible_spill_edges(sim, view_z)
	for i in range(0, edges.size(), 6):
		var upper := Vector3i(edges[i], edges[i + 1], edges[i + 2])
		var lower := Vector3i(edges[i + 3], edges[i + 4], edges[i + 5])
		visible_spills.append(upper)
		visible_spills.append(lower)
		var flow := Vector2(lower.x - upper.x, lower.y - upper.y)
		var middle := Vector2(upper.x + 0.5, upper.y + 0.5) + flow * 0.5
		for side in [-0.5, 0.5]:
			var end := Vector2i((middle + flow.orthogonal() * float(side)).round())
			var key := Vector4i(end.x, end.y, upper.z, lower.z)
			var corner: Vector4 = spill_corners.get(key, Vector4.ZERO)
			spill_corners[key] = corner + Vector4(middle.x - end.x, middle.y - end.y, flow.x, flow.y)


func tile_in_sight(at: Vector2i) -> bool:
	return not surveyed_floor or remembered.get(at, Vector4i.ZERO).x == 1

var known_only := false
var sky := 0
var season := 0
var temperature := 10.0
var water_freeze_level := 2147483647
var occupied := {}
var explored := {}

func refresh() -> void:
	_read_observed_floor()
	_refresh_spills()
	levels.clear()
	explored.clear()
	if known_only:
		var ex: PackedInt32Array = sim.explored(-1, view_z)
		for i in range(0, ex.size(), 2):
			explored[Vector2i(ex[i], ex[i + 1])] = true
	for z in range(view_z - 6, view_z + 1):
		if z >= zmin and z <= zmax:
			levels[z] = projected_levels[z] if surveyed_floor and z < view_z else sim.level_tiles(z)
	# The levels just above the view, so a hill rising above it can show its own top. They
	# change rarely; fetched when the view level changes or every two seconds.
	if not surveyed_floor and (view_z != _above_z or Time.get_ticks_msec() - _above_ms > 2000):
		levels_above.clear()
		for z in range(view_z + 1, view_z + 5):
			if z >= zmin and z <= zmax:
				levels_above[z] = sim.level_tiles(z)
		_above_z = view_z
		_above_ms = Time.get_ticks_msec()
	if surveyed_floor:
		for z in range(view_z + 1, mini(zmax + 1, view_z + 5)):
			levels[z] = projected_levels[z]
	else:
		for z in levels_above:
			levels[z] = levels_above[z]
		ground = sim.ground()
	plants = sim.plants()
	agents = sim.agents()
	bands = sim.bands()
	thin = sim.thin_view()
	band_lines = sim.band_lines()
	structures = sim.structures()
	items = sim.items()
	engravings = sim.engravings()
	if Time.get_ticks_msec() - _ruins_ms > 5000:
		_ruins_ms = Time.get_ticks_msec()
		ruins.clear()
		var ru: PackedInt32Array = sim.ruins()
		for i in range(0, ru.size(), 4):
			ruins[Vector3i(ru[i], ru[i + 1], ru[i + 2])] = ru[i + 3]
	if mat_slugs.is_empty():
		mat_slugs = sim.material_slugs()
	designations = sim.designations()
	roofs = sim.roofs(view_z)
	built.clear()
	var bo: PackedInt32Array = sim.built_of(view_z)
	for i in range(0, bo.size(), 3):
		built[Vector2i(bo[i], bo[i + 1])] = bo[i + 2]
	carried.clear()
	carried_mat.clear()
	var c: PackedInt32Array = sim.carried()
	for i in range(0, c.size(), 3):
		carried[c[i]] = c[i + 1]
		carried_mat[c[i]] = c[i + 2]
	hour = sim.hour()
	sky = sim.sky()
	season = sim.season()
	temperature = sim.temperature()
	water_freeze_level = sim.water_freeze_level()
	occupied.clear()
	for i in range(0, agents.size(), AGENT_STRIDE):
		occupied[Vector3i(agents[i + 1], agents[i + 2], agents[i + 3])] = true
	var cr := cull_rect()
	fires.clear()
	for y in range(maxi(0, cr.position.y - 6), mini(h, cr.position.y + cr.size.y + 6)):
		for x in range(maxi(0, cr.position.x - 6), mini(w, cr.position.x + cr.size.x + 6)):
			if flags_of(tile_at(view_z, x, y)) & 64:
				fires.append(Vector2i(x, y))
	# Ash where fire burned in the last ten days (the ground one level under the view).
	scorched.clear()
	var sc: PackedInt32Array = sim.scorched(view_z - 1, 10)
	for i in range(0, sc.size(), 3):
		scorched[Vector2i(sc[i], sc[i + 1])] = sc[i + 2]
	var vk := "%d|%d|%d|%d|%d" % [view_z, season, sky, int(temperature < -1.0), scorched.size()]
	if vk != _variety_key:
		_variety.clear()
		_variety_key = vk
	_rebuild_entities()
	if rust_renderer_enabled:
		rust_renderer.rebuild()
	roof_layer.queue_redraw()
	light.rebuild_lights()
	fire_fx.update()
	queue_redraw()

var _static_age := 999.0

func _rebuild_entities(force_static := false) -> void:
	var c := cull_rect()
	var with_static := force_static or _static_age > 0.5 or view_z != _built_z or not _built_rect.encloses(show_rect)
	entities.set_wind(2.0 if sky != 0 else 1.0)
	if with_static:
		_static_age = 0.0
		_built_rect = c
		_built_z = view_z
	entities.rebuild(_built_rect if not with_static else c, with_static)
	overlay.queue_redraw()
	if name_tags != null:
		name_tags.queue_redraw()

func _process(d: float) -> void:
	if not animation_frozen:
		anim_t += d
	_static_age += d
	if view_z != _built_z or not _built_rect.encloses(show_rect):
		_rebuild_entities(true)
	entities.animate()
	if rust_renderer_enabled:
		rust_renderer.rebuild(d)
	effects.update(d)
	weather.update()
	light.update(d)
	_sync_chunks()
	queue_redraw()

## Ready the view for a timelapse sample of the tick now (#484 finish). FIXLOG 2026-11-02:
## the windowed same-tick check failed intermittently because three things in a frame came
## from wall-clock time rather than from the tick: the levels above and the ruins were cached
## for up to 2 and 5 real seconds, the static entities were rebuilt on a half-second timer, and
## transient effects (contact flashes, work accents) age in real seconds. A sample reads every
## level and the ruins afresh, rebuilds every entity, and leaves the transient effects out.
func prepare_still() -> void:
	_above_ms = -1000000
	_ruins_ms = -1000000
	refresh()
	_rebuild_entities(true)
	effects.visible = false
	anim_ground.queue_redraw()

func end_still() -> void:
	effects.visible = true
	refresh()

## The culling rect in tiles (what the camera shows, plus a margin), clamped to the map.
func cull_rect() -> Rect2i:
	var r := show_rect
	if r.size == Vector2i.ZERO:
		return Rect2i(0, 0, w, h)
	return r.grow(4)

## Daylight 0..1: full 08:00 to 18:00 with two-hour ramps (art bible).
func sun_level() -> float:
	var s: float = clampf(1.0 - (absf(hour - 13.0) - 5.0) / 2.5, 0.0, 1.0)
	return s * s * (3.0 - 2.0 * s)

func under_sky_at_centre() -> bool:
	var c := cull_rect()
	var cx := clampi(c.position.x + c.size.x / 2, 0, w - 1)
	var cy := clampi(c.position.y + c.size.y / 2, 0, h - 1)
	return view_z >= ground[cy * w + cx]

func tile_at(z: int, x: int, y: int) -> int:
	if not levels.has(z) or x < 0 or y < 0 or x >= w or y >= h:
		return 2 << 16 # solid rock outside
	return levels[z][y * w + x]

static func mat_of(t: int) -> int: return t & 0xffff
static func shape_of(t: int) -> int: return (t >> 16) & 0xff
static func flags_of(t: int) -> int: return (t >> 24) & 0xff

func _tex(p: String) -> Texture2D:
	return assets.tex(p)

func fam_tex(fam: String) -> Texture2D:
	return _tex("terrain/%s.png" % fam)

func season_tint(fam: String) -> Color:
	# Living ground follows the season; stone, water and floors do not.
	if fam in ["grass", "peat", "soil", "mud", "clay", "wildflower", "mossy", "wet_soil"]:
		match season:
			0: return Color(0.97, 1.0, 0.93)
			1: return Color(0.93, 1.0, 0.88)
			2: return Color(1.0, 0.9, 0.72)
			3: return Color(0.86, 0.9, 1.0)
	return Color(1, 1, 1)

## In autumn, a family's own autumn sheet (`<path>_autumn.png`, tools/art/terrain.py) when it
## has one; it is drawn untinted. Null otherwise.
func autumn_tex(path: String) -> Texture2D:
	if season != 2:
		return null
	return assets.optional(path + "_autumn.png")

func snowing() -> bool:
	return season == 3 and (temperature < 1.0 or sky == 3)

func draw_family(fam: String, x: int, y: int, dim: float = 0.0, frame_anim: bool = false, alpha: float = 1.0) -> void:
	var r := Rect2(x * TILE, y * TILE, TILE, TILE)
	if fam == "unseen_slope":
		# The terrain height is available, but no observed surface material reaches
		# this far down. Keep its relief without supplying live water or construction.
		_ci.draw_rect(r, Color("#555b59").darkened(dim))
		return
	var t := fam_tex(fam)
	var tint := season_tint(fam)
	var at := autumn_tex("terrain/%s" % fam)
	if at:
		t = at
		tint = Color(1, 1, 1)
	var mod := Color(1, 1, 1).darkened(dim) * tint
	mod.a = alpha
	if t == null:
		_ci.draw_rect(r, FALLBACK.get(fam, Color.MAGENTA).darkened(dim))
		return
	var variants := int(t.get_width() / TILE)
	var v := _variant(x, y, variants)
	var row := 0
	if frame_anim:
		var frames: int = mini(4, variants)
		v = int(anim_t * 3.0 + (x + y) % 4) % frames
		row = (x * 3 + y) % maxi(1, int(t.get_height() / TILE))
	_ci.draw_texture_rect_region(t, r, Rect2(v * TILE, row * TILE, TILE, TILE), mod)
	if dim < 0.3 and fam in SNOWABLE and snowing():
		var s := fam_tex("snow")
		if s:
			var sv := (x * 5 + y * 11) % maxi(1, int(s.get_width() / TILE))
			_ci.draw_texture_rect_region(s, r, Rect2(sv * TILE, 0, TILE, TILE), Color(1, 1, 1, 0.82).darkened(dim))

## Which column of a terrain sheet a tile uses: a mixed 32-bit hash of the coordinates, so every
## column is reached about equally with no short repeat. (The old linear form was always even,
## so half the columns and two of the four blend mask rows were never drawn: issue 83.)
static func _variant(x: int, y: int, variants: int) -> int:
	var hv := (x * 374761393 + y * 668265263) & 0xFFFFFFFF
	hv = ((hv ^ (hv >> 13)) * 1274126177) & 0xFFFFFFFF
	hv ^= hv >> 16
	return hv % maxi(1, variants)

func wall_bits(z: int, x: int, y: int, mat: int) -> int:
	var b := 0
	for d in [[0, -1, 1], [1, 0, 2], [0, 1, 4], [-1, 0, 8]]:
		var t := tile_at(z, x + d[0], y + d[1])
		if shape_of(t) == 2 and (mat_of(t) == mat or (flags_of(t) & 8)):
			b |= d[2]
	return b

## The sheet name of a wall: its own wood or stone when the sim says what it was built of and
## there is art for it (C56), else timber, stone block or thatch.
func wall_name(z: int, x: int, y: int, mat: int) -> String:
	if z == view_z and built.has(Vector2i(x, y)):
		var m: int = built[Vector2i(x, y)]
		# A probe, not a need: a material with no wall art of its own is not a missing asset
		# (#184, QA 2026-10-20, C63: every one was reported in missing_assets).
		if m >= 0 and m < mat_slugs.size() and assets.optional("structures/wall_%s.png" % mat_slugs[m]) != null:
			return mat_slugs[m]
	return WALL_MAT.get(mat, "stone_block")

func draw_wall(z: int, x: int, y: int, mat: int) -> void:
	var r := Rect2(x * TILE, y * TILE, TILE, TILE)
	var name: String = wall_name(z, x, y, mat)
	var sheet := _tex("structures/wall_%s.png" % name)
	var bits := wall_bits(z, x, y, mat)
	if sheet == null:
		var c: Color = FALLBACK.get(FAMILY.get(mat, "grey_stone"), Color.GRAY)
		_ci.draw_rect(r, c.lightened(0.15))
		_ci.draw_rect(Rect2(r.position + Vector2(0, TILE - 7), Vector2(TILE, 7)), c.darkened(0.45))
		return
	# A wall from the valley's past is weathered: darker, with moss, chips and cracks over it
	# (#202, QA 2026-10-20: it was drawn as crisp as a wall built today).
	var ruin: bool = ruins.get(Vector3i(x, y, z), -1) == 1
	_ci.draw_texture_rect_region(sheet, r, Rect2(bits * TILE, 0, TILE, TILE), Rules.RUIN_TINT if ruin else Color(0.82, 0.8, 0.78))
	if ruin:
		var ov := _tex("overlays/ruin.png")
		if ov:
			_ci.draw_texture_rect_region(ov, r, Rect2(Rules.ruin_variant(x, y) * TILE, 0, TILE, TILE))
	# Thickness cue: lit bevel on open north/west edges, dark bevel on open south/east edges.
	var lit := Color(1, 1, 0.9, 0.35)
	var dark := Color(0, 0, 0, 0.45)
	if bits & 1 == 0: _ci.draw_rect(Rect2(r.position, Vector2(TILE, 2)), lit)
	if bits & 8 == 0: _ci.draw_rect(Rect2(r.position, Vector2(2, TILE)), lit)
	if bits & 4 == 0: _ci.draw_rect(Rect2(r.position + Vector2(0, TILE - 3), Vector2(TILE, 3)), dark)
	if bits & 2 == 0: _ci.draw_rect(Rect2(r.position + Vector2(TILE - 2, 0), Vector2(2, TILE)), dark)
	_ci.draw_rect(r, Color(0.05, 0.04, 0.03, 1.0), false, 1.0)

## A built wall's contact shadows on the ground east and south of it (#505): depth from the
## walls that stand there, nothing drawn where no wall is.
func _draw_wall_shadows(x: int, y: int, mat: int) -> void:
	var east := tile_at(view_z, x + 1, y)
	var south := tile_at(view_z, x, y + 1)
	var face_h := 0
	if not (shape_of(south) == 2 and WALL_MAT.has(mat_of(south))):
		var face := _tex("overlays/wall_face_%s.png" % wall_name(view_z, x, y, mat))
		face_h = face.get_height() if face else 10
	var origin := Vector2(x * TILE, y * TILE)
	for e in Rules.wall_contact_shadows(shape_of(east) != 2 and x + 1 < w, shape_of(south) != 2 and y + 1 < h, face_h):
		var r: Rect2 = e[0]
		_ci.draw_rect(Rect2(origin + r.position, r.size), Color(0, 0, 0, float(e[1])))

func draw_wall_face(z: int, x: int, y: int, mat: int) -> void:
	var name: String = wall_name(z, x, y, mat)
	var south := tile_at(z, x, y + 1)
	if shape_of(south) == 2 and WALL_MAT.has(mat_of(south)):
		return
	var face := _tex("overlays/wall_face_%s.png" % name)
	if face:
		var fh := face.get_height()
		_ci.draw_texture_rect_region(face, Rect2(x * TILE, (y + 1) * TILE, TILE, fh), Rect2(0, 0, TILE, fh))
	else:
		_ci.draw_rect(Rect2(x * TILE, (y + 1) * TILE, TILE, 10), Color(0.1, 0.08, 0.06, 0.85))

func _draw() -> void:
	if not levels.has(view_z):
		return
	var cull := show_rect
	if cull.size == Vector2i.ZERO:
		cull = Rect2i(0, 0, w, h)
	var x0: int = maxi(0, cull.position.x)
	var y0: int = maxi(0, cull.position.y)
	var x1: int = mini(w, cull.position.x + cull.size.x)
	var y1: int = mini(h, cull.position.y + cull.size.y)
	# --- designations (ghosts) ---
	for i in range(0, designations.size(), DESIG_STRIDE):
		if designations[i + 3] == view_z:
			_draw_designation(i)
	# Plants, structures, items and creatures are pooled sprites in render/entity_layer.gd;
	# light and colour are render/light_layer.gd; weather is render/weather_layer.gd.

# ---------------------------------------------------------------- cached ground chunks
func _sync_chunks() -> void:
	if rust_renderer_enabled:
		return
	var cw := (w + CH - 1) / CH
	var chh := (h + CH - 1) / CH
	var vers: PackedInt64Array = sim.chunk_versions(view_z - 6, view_z + 4)
	var versions_w := (w + SIM_CHUNK - 1) / SIM_CHUNK
	var c := cull_rect()
	var key := "%d|%s|%d|%d" % [view_z, _variety_key, int(snowing()), int(under_sky_at_centre())]
	for cy in chh:
		for cx in cw:
			var k := Vector2i(cx, cy)
			var nodes: Array = chunk_nodes.get(k, [])
			if nodes.is_empty():
				for f in [false, true]:
					var n = preload("res://scripts/render/ground_chunk.gd").new()
					n.v = self
					n.cx = cx
					n.cy = cy
					n.faces = f
					(faces_root if f else ground_root).add_child(n)
					nodes.append(n)
				chunk_nodes[k] = nodes
			var vis := c.intersects(Rect2i(cx * CH, cy * CH, CH, CH))
			nodes[0].visible = vis
			nodes[1].visible = vis
			if not vis:
				continue
			var index := (cy * CH / SIM_CHUNK) * versions_w + cx * CH / SIM_CHUNK
			var ver := "%s|%d" % [key, vers[index] if index < vers.size() else 0]
			if chunk_seen.get(k, "") != ver:
				chunk_seen[k] = ver
				nodes[0].queue_redraw()
				nodes[1].queue_redraw()

func _draw_chunk(ci: CanvasItem, cx: int, cy: int, faces: bool) -> void:
	_ci = ci
	_chunk_mode = true
	var x0 := cx * CH
	var y0 := cy * CH
	var x1 := mini(w, x0 + CH)
	var y1 := mini(h, y0 + CH)
	_prep_blend(x0, y0, x1, y1)
	if faces:
		# Banks spill over the water's edge here, above the animated water layer and under the
		# cliff faces.
		for y in range(y0, y1):
			for x in range(x0, x1):
				if _bg_pri[(y - _bg_y0) * _bg_w + x - _bg_x0] == 0:
					_draw_blend(x, y)
		for y in range(y0, y1):
			for x in range(x0, x1):
				_draw_cliffs(x, y)
	else:
		ci.dyn.clear()
		_dyn_rec = ci.dyn
		for y in range(y0, y1):
			for x in range(x0, x1):
				_draw_ground(x, y)
	_chunk_mode = false
	_ci = self

func _draw_dynamic(ci: CanvasItem) -> void:
	_ci = ci
	for k in chunk_nodes:
		var g = chunk_nodes[k][0]
		if not g.visible:
			continue
		for d in g.dyn:
			var x: int = d[0]
			var y: int = d[1]
			match d[2]:
				"water":
					_draw_water(d[3], x, y, d[4], d[5])
				"fire":
					var fire := _tex("structures/fire.png")
					var r := Rect2(x * TILE, y * TILE, TILE, TILE)
					if fire:
						var f := (int(anim_t * 8.0) + x * 3 + y) % maxi(1, int(fire.get_width() / TILE))
						_ci.draw_texture_rect_region(fire, r, Rect2(f * TILE, 0, TILE, TILE))
					else:
						_ci.draw_rect(Rect2(x * TILE + 6, y * TILE + 6, 20, 20), Color(1.0, 0.5, 0.1))
				"door":
					var open_door: bool = occupied.has(Vector3i(x, y, view_z))
					var door := _tex("structures/door_open.png" if open_door else "structures/door_closed.png")
					var r2 := Rect2(x * TILE, y * TILE, TILE, TILE)
					if door:
						_ci.draw_texture_rect(door, r2, false)
					else:
						_ci.draw_rect(r2.grow(-6), Color(0.45, 0.3, 0.15))
	var bounds := cull_rect()
	for i in range(0, visible_spills.size(), 2):
		var upper := visible_spills[i]
		if bounds.has_point(Vector2i(upper.x, upper.y)):
			_draw_spill(ci, upper, visible_spills[i + 1])
	_ci = self

# A bright crest gives way to a shaded face and moving streaks at its foot.
func _draw_spill(ci: CanvasItem, upper: Vector3i, lower: Vector3i) -> void:
	var flow := Vector2(lower.x - upper.x, lower.y - upper.y)
	var mid := Vector2(upper.x + 0.5, upper.y + 0.5) + flow * 0.5
	var lip := PackedVector2Array()
	var foot := PackedVector2Array()
	var depth := minf(TILE * 0.68, 8.0 + 5.0 * (upper.z - lower.z))
	for sign_value in [-0.5, 0.5]:
		var end := Vector2i((mid + flow.orthogonal() * float(sign_value)).round())
		var corner: Vector4 = spill_corners[Vector4i(end.x, end.y, upper.z, lower.z)]
		var normal := Vector2(corner.z, corner.w)
		var top := Vector2(end) * TILE
		var offset := Vector2(corner.x, corner.y)
		var local_depth := depth
		if normal == Vector2.ZERO:
			normal = flow
		elif normal.length_squared() > 1.0 and offset != Vector2.ZERO:
			# The foot must fit between consecutive inside bends in the crest.
			top += offset * TILE * 0.42
			local_depth = minf(depth, TILE * 0.28)
		lip.append(top)
		foot.append(top + normal.normalized() * local_depth)
	var points := PackedVector2Array([lip[0], lip[1], foot[1], foot[0]])
	var crest := Color("#a4d3cc")
	var base := Color("#326878")
	ci.draw_polygon(points, PackedColorArray([crest, crest, base, base]))
	ci.draw_line(lip[0], lip[1], Color("#c0dedb"), 1.5, true)
	for n in 5:
		var lane := (n + 0.5) / 5.0
		var head := lip[0].lerp(lip[1], lane)
		var tail := foot[0].lerp(foot[1], lane)
		var phase := fposmod(anim_t * 1.35 + n * 0.37 + upper.x * 0.13 + upper.y * 0.21, 1.0)
		ci.draw_line(head.lerp(tail, phase), head.lerp(tail, minf(phase + 0.44, 1.0)), Color(0.83, 0.94, 0.93, 0.72), 1.5, true)
		var froth := tail + (tail - head).normalized() * (1.0 + sin(phase * TAU))
		var along := (foot[1] - foot[0]).normalized()
		var spread := 1.3 + sin(phase * PI)
		ci.draw_line(froth - along * spread, froth + along * spread, Color(0.78, 0.90, 0.89, 0.35 + 0.25 * sin(phase * PI)), 1.6, true)

func _ground_family(x: int, y: int) -> Array:
	# returns [family, dim, animated, level]; level is the height of the visible surface (a
	# water cell at z and a solid top at z - 1 share level z), so edge blending only joins
	# ground that lies flush.
	var t := tile_at(view_z, x, y)
	var sh := shape_of(t)
	var fl := flags_of(t)
	if fl & 3:
		return ["deep_water" if fl & 2 else "water", 0.0, true, view_z]
	if sh == 2:
		return [FAMILY.get(mat_of(t), "grey_stone"), 0.0, false, view_z + 1]
	if sh == 1:
		return [FAMILY.get(mat_of(t), "stone_floor"), 0.0, false, view_z + 1]
	# open: look below
	for peel in range(1, 7):
		var z := view_z - peel
		if z < zmin:
			break
		var b := tile_at(z, x, y)
		var bs := shape_of(b)
		var bf := flags_of(b)
		var dim: float = [0.0, 0.0, 0.25, 0.42, 0.56, 0.66, 0.74][mini(peel, 6)] if view_z > ground[y * w + x] + 1 else [0.0, 0.0, 0.1, 0.3, 0.48, 0.62, 0.7][mini(peel, 6)]
		if bf & 3:
			return ["deep_water" if bf & 2 else "water", dim, true, z]
		if bs == 2 or bs == 1 or bs >= 3:
			var fam: String = FAMILY.get(mat_of(b), "soil")
			if fam == "wood_floor" and bs == 2: fam = "wood_floor"
			# Below ground, natural floors of soil or rock are cave floor (built floors and ore
			# keep their own look).
			if ground[y * w + x] >= view_z and fam in CAVE_FLOORED:
				fam = "cave_floor"
			return [fam, dim, false, z + 1]
	if surveyed_floor:
		var terrain_z: int = ground[y * w + x]
		var shade := clampf(0.07 * (view_z - terrain_z), 0.1, 0.5)
		return ["unseen_slope", shade, false, terrain_z + 1]
	return ["black_stone", 0.9, false, -99999]

## The water sheets carry depth and local texture. Frost glazes them only on a
## witnessed, exposed water surface. A remembered floor has no ice observation.
func _draw_water(family: String, x: int, y: int, dim: float, z: int) -> void:
	var live := z == view_z or not surveyed_floor or _lower_in_sight(x, y, z)
	var flowing := live and z < water_freeze_level
	draw_family(family, x, y, dim, flowing)
	if family == "deep_water" and _deep_rim(x, y, z):
		draw_family("water", x, y, dim, flowing, 0.5)
	if flowing or not _observed_water_surface(x, y, z):
		return
	var origin := Vector2(x * TILE, y * TILE)
	var glaze := Color("#c5e7ef") if family == "water" else Color("#abcdda")
	glaze.a = 0.83
	_ci.draw_rect(Rect2(origin, Vector2(TILE, TILE)), glaze)
	# A few short, still highlights break up the surface without implying cracks or strength.
	var mark := _variant(x, y, 11)
	if mark < 4:
		var offset := Vector2(4 + _variant(y, x, 12), 6 + mark * 5)
		_ci.draw_line(origin + offset, origin + offset + Vector2(8 + mark, -2), Color(0.85, 0.94, 0.96, 0.47), 1.2, true)

func _observed_water_surface(x: int, y: int, z: int) -> bool:
	if z == view_z:
		var record: Vector4i = remembered.get(Vector2i(x, y), Vector4i.ZERO)
		if record.x != 1 or not (flags_of(record.y) & 3) or flags_of(record.y) & 128:
			return false
	elif not _lower_in_sight(x, y, z) or not (flags_of(tile_at(z, x, y)) & 3) or flags_of(tile_at(z,x,y)) & 128:
		return false
	return _water_has_air_above(x, y, z)

func _water_has_air_above(x: int, y: int, z: int) -> bool:
	if z == zmax:
		return true
	var above := tile_at(z + 1, x, y)
	var shape := shape_of(above)
	return shape != 1 and shape != 2 and (flags_of(above) & 3) == 0

func _deep_rim(x: int, y: int, z: int) -> bool:
	for dy in range(-1, 2):
		for dx in range(-1, 2):
			if not (flags_of(tile_at(z, x + dx, y + dy)) & 2):
				return true
	return false

## Seasonal looks of persistent ground. Which family a tile is comes from the generator
## (world/gen/biome.rs: meadow, moss, scree, frozen soil, wet soil and the rest are materials);
## the client only changes how some of them look with the season and the weather: a meadow is
## plain grass out of spring and summer, bare and wet ground freezes in a hard frost, bare soil
## goes wet in rain, and burned ground shows ash. Cached per tile until the season, sky,
## freeze or level changes. docs/ART_SPEC.md records which family is which.
func _surface_variety(fam: String, x: int, y: int) -> String:
	var k := Vector2i(x, y)
	if _variety.has(k):
		return _variety[k]
	if scorched.has(k) and fam in ["grass", "soil", "wildflower", "mossy", "peat", "wet_soil", "frozen_soil"]:
		_variety[k] = "ash"
		return "ash"
	var out := fam
	match fam:
		"wildflower":
			if season >= 2:
				out = "grass"
		"soil", "clay", "wet_soil", "mud":
			if temperature < -1.0:
				out = "frozen_soil"
			elif (sky == 2 or sky == 5) and fam != "mud":
				out = "wet_soil"
	_variety[k] = out
	return out

func _ground_family_below(x: int, y: int) -> String:
	var b := tile_at(view_z - 1, x, y)
	return FAMILY.get(mat_of(b), "soil")

## Seepage is a presentation cue only when the existing world data contains water next to the
## visible cave floor. It never classifies the water or invents pollution, salinity or quality.
## FIXLOG 2026-10-30 (#519): a deep peel checks its revealed floor, not the camera's level.
func _cave_has_adjacent_water(x: int, y: int, visible_z: int) -> bool:
	for z in [visible_z, visible_z - 1]:
		for d in [[0, -1], [1, 0], [0, 1], [-1, 0]]:
			if flags_of(tile_at(z, x + d[0], y + d[1])) & 3:
				return true
	return false

func _draw_underground_details(fam: String, x: int, y: int, visible_z: int) -> void:
	if fam != "cave_floor" or not _cave_has_adjacent_water(x, y, visible_z):
		return
	var seep := _tex("overlays/cave_seepage.png")
	if seep:
		var v := _variant(x, y, UNDERGROUND_SEEP_VARIANTS)
		_ci.draw_texture_rect_region(seep, Rect2(x * TILE, y * TILE, TILE, TILE),
			Rect2(v * TILE, 0, TILE, TILE), Color(1, 1, 1, 0.78))

func _draw_ground(x: int, y: int) -> void:
	var t := tile_at(view_z, x, y)
	var sh := shape_of(t)
	var m := mat_of(t)
	if sh == 2 and (flags_of(t) & 8):
		# a door: draw the ground beneath, then the door leaf
		var gd := _ground_family_below(x, y)
		draw_family(gd, x, y, 0.0, false)
		if _chunk_mode:
			_dyn_rec.append([x, y, "door"])
			return
		var open_door: bool = occupied.has(Vector3i(x, y, view_z))
		var door := _tex("structures/door_open.png" if open_door else "structures/door_closed.png")
		var r := Rect2(x * TILE, y * TILE, TILE, TILE)
		if door:
			_ci.draw_texture_rect(door, r, false)
		else:
			_ci.draw_rect(r.grow(-6), Color(0.45, 0.3, 0.15))
		return
	if sh == 2 and WALL_MAT.has(m):
		# built wall: show the floor family first then the wall sprite
		draw_wall(view_z, x, y, m)
		return
	var g := _ground_family(x, y)
	# Solid at view z with more solid above it: on the surface that is a hill rising above the
	# view, drawn as its own top surface with a lit north/west lip and shaded south/east edge;
	# underground (or deeper than four levels) it is unexcavated rock, drawn dark.
	if sh == 2 and ground[y * w + x] > view_z:
		var top: int = ground[y * w + x]
		if top - view_z <= 4 and levels.has(top) and under_sky_at_centre():
			var tt := tile_at(top, x, y)
			if (not surveyed_floor or _lower_in_sight(x, y, top)) and ruins.get(Vector3i(x, y, top), -1) == 1:
				# An old house on ground a level up: its walls drawn as walls, weathered (#202),
				# not as a rise of block-paved ground.
				draw_wall(top, x, y, mat_of(tt))
				return
			var tf: String = FAMILY.get(mat_of(tt), g[0])
			tf = _surface_variety(tf, x, y)
			draw_family(tf, x, y, 0.0, false)
			_draw_blend(x, y)
			var r := Rect2(x * TILE, y * TILE, TILE, TILE)
			var rise := top - view_z
			if rise >= 2:
				_ci.draw_rect(r, Color(1, 1, 0.92, 0.035 * (rise - 1)))
			var lit := Color(1, 1, 0.88, 0.32)
			var shade := Color(0.02, 0.02, 0.05, 0.30)
			if y > 0 and ground[(y - 1) * w + x] < top:
				_ci.draw_rect(Rect2(r.position, Vector2(TILE, 2)), lit)
			if x > 0 and ground[y * w + x - 1] < top:
				_ci.draw_rect(Rect2(r.position, Vector2(2, TILE)), lit)
			if y + 1 < h and ground[(y + 1) * w + x] < top:
				_ci.draw_rect(Rect2(r.position + Vector2(0, TILE - 3), Vector2(TILE, 3)), shade)
			if x + 1 < w and ground[y * w + x + 1] < top:
				_ci.draw_rect(Rect2(r.position + Vector2(TILE - 2, 0), Vector2(2, TILE)), shade)
			return
		draw_family(g[0], x, y, 0.7, false)
		return
	var fam: String = g[0]
	if g[1] < 0.15 and not g[2]:
		fam = _surface_variety(fam, x, y)
	if g[2] and _chunk_mode:
		_dyn_rec.append([x, y, "water", fam, g[1], g[3]])
	else:
		if g[2]:
			_draw_water(fam, x, y, g[1], g[3])
		else:
			draw_family(fam, x, y, g[1])
		if sh < 5:
			_draw_blend(x, y)
		_draw_underground_details(fam, x, y, g[3])
	if sh != 3 and g[1] < 0.5:
		_draw_relief(x, y)
	# Keyed by the level people walk on here, which is the view's or, on a rise, one above it.
	var rubble_z: int = ground[y * w + x] + 1
	if (not surveyed_floor or rubble_z == view_z or _lower_in_sight(x, y, rubble_z)) and ruins.get(Vector3i(x, y, rubble_z), -1) == 0:
		# Where an old wall fell, its stones lie (#202).
		var rb := _tex("overlays/rubble.png")
		if rb:
			_ci.draw_texture_rect_region(rb, Rect2(x * TILE, y * TILE, TILE, TILE), Rect2(Rules.ruin_variant(x, y) * TILE, 0, TILE, TILE))
	if sh >= 5: # stairs / ladder
		# A ladder (shape 8) has its own sheet; it was drawn as stairs (#184, QA 2026-10-20, C63).
		var stairs := _tex("structures/%s.png" % Rules.stair_sheet(sh))
		var r := Rect2(x * TILE, y * TILE, TILE, TILE)
		if stairs:
			_ci.draw_texture_rect(stairs, r, false)
		else:
			_ci.draw_rect(r.grow(-4), Color(0.15, 0.12, 0.10))
			for i in range(4):
				_ci.draw_line(Vector2(r.position.x + 6, r.position.y + 8 + i * 5), Vector2(r.end.x - 6, r.position.y + 8 + i * 5), Color(0.5, 0.45, 0.4), 1)
	if sh == 3:
		_draw_ramp(x, y)
	if flags_of(t) & 64 and _chunk_mode:
		_dyn_rec.append([x, y, "fire"])
	elif flags_of(t) & 64:
		var fire := _tex("structures/fire.png")
		if fire:
			# Every frame of the sheet, not the first two (#184, QA 2026-10-20, C63).
			var f: int = Rules.fire_frame(anim_t * 6.0, x + y, fire.get_width(), TILE)
			_ci.draw_texture_rect_region(fire, Rect2(x * TILE, y * TILE, TILE, TILE), Rect2(f * TILE, 0, TILE, TILE))
		else:
			_ci.draw_rect(Rect2(x * TILE + 6, y * TILE + 6, 20, 20), Color(1.0, 0.5, 0.1))

## What a tile shows for edge blending: [family, dim, level, variant], or [] when it takes no
## part (walls, doors, stairs, buried rock, built floors). A ramp takes part as the ground it is
## drawn with: ramps lie on level steps, where biome families now change, and left out they
## showed square edges there. Mirrors _draw_ground's choice of
## family.
func _blend_look(x: int, y: int) -> Array:
	if x < 0 or y < 0 or x >= w or y >= h:
		return []
	var out: Array = []
	var t := tile_at(view_z, x, y)
	var sh := shape_of(t)
	if sh == 2 and ((flags_of(t) & 8) or WALL_MAT.has(mat_of(t))):
		pass
	elif sh >= 5:
		pass
	elif sh == 2 and ground[y * w + x] > view_z:
		var top: int = ground[y * w + x]
		if top - view_z <= 4 and levels.has(top) and under_sky_at_centre():
			var tf: String = FAMILY.get(mat_of(tile_at(top, x, y)), FAMILY.get(mat_of(t), "grey_stone"))
			tf = _surface_variety(tf, x, y)
			out = [tf, 0.0, top + 1, _variant(x, y, 8)]
	else:
		var g := _ground_family(x, y)
		var fam: String = g[0]
		if g[1] < 0.15 and not g[2]:
			fam = _surface_variety(fam, x, y)
		out = [fam, g[1], g[3], _variant(x, y, 8)]
	if not out.is_empty() and not _blend_pri.has(out[0]):
		out = []
	return out

func _prep_blend(x0: int, y0: int, x1: int, y1: int) -> void:
	var key := Vector3i(x0, y0, Engine.get_process_frames())
	if key == _bg_key:
		return
	_bg_key = key
	_bg_x0 = x0 - 1
	_bg_y0 = y0 - 1
	_bg_w = x1 - x0 + 2
	var n := _bg_w * (y1 - y0 + 2)
	_bg_pri.resize(n)
	_bg_lvl.resize(n)
	_bg_var.resize(n)
	_bg_dim.resize(n)
	var i := 0
	for y in range(_bg_y0, y1 + 1):
		for x in range(_bg_x0, x1 + 1):
			var b := _blend_look(x, y)
			if b.is_empty():
				_bg_pri[i] = -1
			else:
				_bg_pri[i] = _blend_pri[b[0]]
				_bg_dim[i] = b[1]
				_bg_lvl[i] = b[2]
				_bg_var[i] = b[3]
			i += 1

## Whether neighbour j's family spills over a tile of rank pr at level lvl. Flush ground: the
## higher rank spills. One level up: the upper ground's lip overhangs the lower tile whatever
## the ranks, so a family change that falls on a level step (biomes follow height, so most do)
## still gets an irregular edge instead of a square one; water takes its banks the same way.
## Water itself never spills, and the same family across a step is left to the relief shading.
func _spills(j: int, pr: int, lvl: int) -> bool:
	var np: int = _bg_pri[j]
	if np <= 0 or np == pr:
		return false
	if _bg_lvl[j] == lvl:
		return np > pr
	return _bg_lvl[j] == lvl + 1

## Draws, over tile (x, y), the edges of every neighbour family that spills over it (_spills):
## an edge piece per side that neighbour holds, a rounded fillet where it holds both sides of a
## corner, a small corner piece where it holds only the diagonal. Lower families first, so the
## highest ends on top. On water the wet line goes down first, under the banks.
func _draw_blend(x: int, y: int) -> void:
	var c := (y - _bg_y0) * _bg_w + x - _bg_x0
	var pr: int = _bg_pri[c]
	if pr < 0:
		return
	var lvl: int = _bg_lvl[c]
	var water := pr == 0
	# Most tiles have no higher neighbour: settle that on the packed arrays before building any.
	var any := false
	for j in [c - _bg_w - 1, c - _bg_w, c - _bg_w + 1, c - 1, c + 1, c + _bg_w - 1, c + _bg_w, c + _bg_w + 1]:
		if _spills(j, pr, lvl):
			any = true
			break
	if not any:
		return
	var nb := []
	var fams := []
	for dy in range(-1, 2):
		for dx in range(-1, 2):
			var j := c + dy * _bg_w + dx
			var np: int = _bg_pri[j]
			if _spills(j, pr, lvl):
				var f: String = BLEND_ORDER[np]
				nb.append([f, _bg_dim[j], _bg_lvl[j], _bg_var[j]])
				if not (f in fams):
					fams.append(f)
			else:
				nb.append([])
	if fams.is_empty():
		return
	var me := [BLEND_ORDER[pr], _bg_dim[c], lvl, _bg_var[c]]
	fams.sort_custom(func(a, b): return _blend_pri[a] < _blend_pri[b])
	var r := Rect2(x * TILE, y * TILE, TILE, TILE)
	var row: int = me[3] & 3    # one mask row for every piece on this tile
	if water:
		var shore := _tex("terrain/edges/shore.png")
		if shore:
			for f in fams:
				_draw_pieces(shore, f, nb, r, row, Color(1, 1, 1).darkened(me[1]))
	# Snow over the snowable pieces is laid once, as their union, after the last of them: piece
	# by piece at 82 % it would build up where pieces overlap and show as pale squares.
	var snow_bits := 0
	var snow_dim := 1.0
	var last_snowable := -1
	if snowing():
		for i in 8:
			var n: Array = nb[NB_BITS[i]]
			if not n.is_empty() and n[0] in SNOWABLE and n[1] < 0.3:
				snow_bits |= 1 << i
				snow_dim = minf(snow_dim, n[1])
				last_snowable = maxi(last_snowable, fams.find(n[0]))
	for fi in fams.size():
		var f: String = fams[fi]
		var t := _tex("terrain/edges/%s.png" % f)
		var tint := season_tint(f)
		var at := autumn_tex("terrain/edges/%s" % f)
		if at:
			t = at
			tint = Color(1, 1, 1)
		if t:
			var dim: float = me[1]
			for n in nb:
				if not n.is_empty() and n[0] == f:
					dim = n[1]
					break
			_draw_pieces(t, f, nb, r, row, Color(1, 1, 1).darkened(dim) * tint)
		if fi == last_snowable:
			var cover := _tex("terrain/edges/snow_cover.png")
			if cover:
				var src := Rect2((snow_bits % 32) * TILE, (row * 8 + snow_bits / 32) * TILE, TILE, TILE)
				_ci.draw_texture_rect_region(cover, r, src, Color(1, 1, 1, 0.82).darkened(snow_dim))

## nb is the 3x3 neighbourhood, row by row (index 4 is the tile itself). Sheet columns, four
## each in the order NE SE SW NW except the edges (N E S W): edges, corners (only the diagonal
## is the family), round fillets (both sides and both cells beyond them are: the inside corner
## of a straight-edged mass), mid fillets (one cell beyond), chamfers (neither: a diagonal
## staircase, cut straight across).
func _draw_pieces(t: Texture2D, f: String, nb: Array, r: Rect2, row: int, mod: Color) -> void:
	for i in 4:
		var n: Array = nb[EDGE_IDX[i]]
		if not n.is_empty() and n[0] == f:
			_ci.draw_texture_rect_region(t, r, Rect2(i * TILE, row * TILE, TILE, TILE), mod)
	for i in 4:
		var a: Array = nb[CORNER_SIDES[i][0]]
		var b: Array = nb[CORNER_SIDES[i][1]]
		var ha: bool = not a.is_empty() and a[0] == f
		var hb: bool = not b.is_empty() and b[0] == f
		if ha and hb:
			var o := 0
			for j in CORNER_BEYOND[i]:
				if not nb[j].is_empty() and nb[j][0] == f:
					o += 1
			var col: int = [16, 12, 8][o] + i
			_ci.draw_texture_rect_region(t, r, Rect2(col * TILE, row * TILE, TILE, TILE), mod)
		elif not ha and not hb:
			var d: Array = nb[CORNER_IDX[i]]
			if not d.is_empty() and d[0] == f:
				_ci.draw_texture_rect_region(t, r, Rect2((4 + i) * TILE, row * TILE, TILE, TILE), mod)

## Soft relief where the ground steps by a level: a shaded bank along a side that rises to a
## higher neighbour, a lit lip on a north or west edge that drops away, a dark drop on a south
## or east edge. Light from the upper-left, as everywhere.
func _draw_relief(x: int, y: int) -> void:
	var g0: int = ground[y * w + x]
	var r := Rect2(x * TILE, y * TILE, TILE, TILE)
	for d in [[0, -1], [-1, 0], [0, 1], [1, 0]]:
		var nx: int = x + d[0]
		var ny: int = y + d[1]
		if nx < 0 or ny < 0 or nx >= w or ny >= h:
			continue
		var gn: int = ground[ny * w + nx]
		if gn == g0:
			continue
		var k := clampf(absf(gn - g0), 1.0, 2.0)
		if gn > g0:
			# the bank of higher ground on this side: two-step shade into the tile
			var a1 := 0.16 * k
			var a2 := 0.08 * k
			match [d[0], d[1]]:
				[0, -1]:
					_ci.draw_rect(Rect2(r.position, Vector2(TILE, 3)), Color(0.02, 0.03, 0.06, a1))
					_ci.draw_rect(Rect2(r.position + Vector2(0, 3), Vector2(TILE, 3)), Color(0.02, 0.03, 0.06, a2))
				[-1, 0]:
					_ci.draw_rect(Rect2(r.position, Vector2(3, TILE)), Color(0.02, 0.03, 0.06, a1))
					_ci.draw_rect(Rect2(r.position + Vector2(3, 0), Vector2(3, TILE)), Color(0.02, 0.03, 0.06, a2))
				[0, 1]:
					_ci.draw_rect(Rect2(r.position + Vector2(0, TILE - 2), Vector2(TILE, 2)), Color(0.02, 0.03, 0.06, a2))
				[1, 0]:
					_ci.draw_rect(Rect2(r.position + Vector2(TILE - 2, 0), Vector2(2, TILE)), Color(0.02, 0.03, 0.06, a2))
		else:
			match [d[0], d[1]]:
				[0, -1]:
					_ci.draw_rect(Rect2(r.position, Vector2(TILE, 2)), Color(1, 1, 0.86, 0.22))
				[-1, 0]:
					_ci.draw_rect(Rect2(r.position, Vector2(2, TILE)), Color(1, 1, 0.86, 0.18))
				[0, 1]:
					_ci.draw_rect(Rect2(r.position + Vector2(0, TILE - 3), Vector2(TILE, 3)), Color(0.02, 0.02, 0.05, 0.26))
				[1, 0]:
					_ci.draw_rect(Rect2(r.position + Vector2(TILE - 2, 0), Vector2(2, TILE)), Color(0.02, 0.02, 0.05, 0.2))

func _draw_ramp(x: int, y: int) -> void:
	# A slope rising toward the neighbour whose ground is one higher: a lit line on the
	# uphill edge, a soft shadow on the downhill edge, and two faint diagonal strokes.
	var g: int = ground[y * w + x]
	var r := Rect2(x * TILE, y * TILE, TILE, TILE)
	var lit := Color(1, 1, 0.85, 0.16)
	var shade := Color(0, 0, 0, 0.14)
	for d in [[0, -1], [1, 0], [0, 1], [-1, 0]]:
		var nx: int = x + d[0]
		var ny: int = y + d[1]
		if nx < 0 or ny < 0 or nx >= w or ny >= h: continue
		if ground[ny * w + nx] == g + 1:
			match [d[0], d[1]]:
				[0, -1]:
					_ci.draw_rect(Rect2(r.position, Vector2(TILE, 2)), lit)
					_ci.draw_rect(Rect2(r.position + Vector2(0, TILE - 4), Vector2(TILE, 4)), shade)
				[0, 1]:
					_ci.draw_rect(Rect2(r.position + Vector2(0, TILE - 2), Vector2(TILE, 2)), lit)
					_ci.draw_rect(Rect2(r.position, Vector2(TILE, 4)), shade)
				[1, 0]:
					_ci.draw_rect(Rect2(r.position + Vector2(TILE - 2, 0), Vector2(2, TILE)), lit)
					_ci.draw_rect(Rect2(r.position, Vector2(4, TILE)), shade)
				[-1, 0]:
					_ci.draw_rect(Rect2(r.position, Vector2(2, TILE)), lit)
					_ci.draw_rect(Rect2(r.position + Vector2(TILE - 4, 0), Vector2(4, TILE)), shade)
			return

## Select the face treatment only where the natural solid has an exposed southern edge.
## FIXLOG 2026-10-30 (#519): underground rock can be exposed even though its surface is higher.
func _natural_face_overlay(x: int, y: int) -> String:
	var t := tile_at(view_z, x, y)
	if shape_of(t) != 2 or WALL_MAT.has(mat_of(t)):
		return ""
	if shape_of(tile_at(view_z, x, y + 1)) == 2:
		return ""
	return "strata" if view_z < ground[y * w + x] else "cliff"

func _draw_cliffs(x: int, y: int) -> void:
	var t := tile_at(view_z, x, y)
	if shape_of(t) != 2:
		return
	if WALL_MAT.has(mat_of(t)):
		if not (flags_of(t) & 8):
			draw_wall_face(view_z, x, y, mat_of(t))
			_draw_wall_shadows(x, y, mat_of(t))
		return
	var overlay := _natural_face_overlay(x, y)
	if view_z < ground[y * w + x] and overlay.is_empty():
		return # fully buried rock has neither a face nor its shadow
	if not overlay.is_empty():
		# Natural solid at view z: a raised mass with a face on its exposed southern edge.
		var fam: String = FAMILY.get(mat_of(t), "soil")
		var face := _tex("overlays/cliff_face.png")
		var r := Rect2(x * TILE, (y + 1) * TILE, TILE, TILE)
		if face:
			var fh := face.get_height()
			var variants := int(face.get_width() / TILE)
			var v := 0
			if fam.ends_with("stone"): v = 1
			elif fam == "gravel" or fam == "scree" or fam.begins_with("ore"): v = 2
			v = v % maxi(1, variants)
			_ci.draw_texture_rect_region(face, Rect2(r.position, Vector2(TILE, fh)), Rect2(v * TILE, 0, TILE, fh))
		else:
			_ci.draw_rect(Rect2(r.position, Vector2(TILE, 10)), Color(0.22, 0.18, 0.14, 0.9))
			_ci.draw_rect(Rect2(r.position + Vector2(0, 10), Vector2(TILE, 6)), Color(0, 0, 0, 0.25))
		# Underground faces retain the existing material face and add restrained broken strata.
		# The overlay is a presentation detail; the family selected above remains authoritative.
		if overlay == "strata":
			var strata := _tex("overlays/strata_face.png")
			if strata:
				var sv := _variant(x, y, UNDERGROUND_STRATA_VARIANTS)
				_ci.draw_texture_rect_region(strata, Rect2(r.position, Vector2(TILE, 16)),
					Rect2(sv * TILE, 0, TILE, 16), Color(1, 1, 1, 0.62))
	# Shadow to the east of a raised mass (light from upper-left).
	var east := tile_at(view_z, x + 1, y)
	if shape_of(east) != 2:
		_ci.draw_rect(Rect2((x + 1) * TILE, y * TILE, 6, TILE), Color(0, 0, 0, 0.18))

func _draw_designation(i: int) -> void:
	var x: int = designations[i + 1]
	var y: int = designations[i + 2]
	var code: int = designations[i + 4]
	var progress: int = designations[i + 5]
	var ready: bool = designations[i + 6] == 1
	var mat: int = designations[i + 7]
	var r := Rect2(x * TILE, y * TILE, TILE, TILE)
	var ghost := Color(0.85, 0.95, 1.0, 0.35 if ready else 0.2)
	var tex: Texture2D = null
	match code % 100:
		0:
			# A wall going up: once its timber or stone is at the site it is drawn as built so
			# far (footing, half, nearly closed), turned to run the way the wall runs.
			var stone := mat == 15
			if ready:
				var sheet := _tex("structures/wall_build_%s.png" % ("stone_block" if stone else "wood"))
				if sheet:
					var stage := 0 if progress < 34 else (1 if progress < 75 else 2)
					var vertical := _wall_run_vertical(x, y)
					if vertical:
						_ci.draw_set_transform(r.position + Vector2(TILE, 0), PI / 2.0, Vector2.ONE)
						_ci.draw_texture_rect_region(sheet, Rect2(0, 0, TILE, TILE), Rect2(stage * TILE, 0, TILE, TILE))
						_ci.draw_set_transform(Vector2.ZERO, 0.0, Vector2.ONE)
					else:
						_ci.draw_texture_rect_region(sheet, r, Rect2(stage * TILE, 0, TILE, TILE))
					return
			tex = _tex("structures/wall_%s.png" % ("stone_block" if stone else "wood"))
		16:
			return  # roofs are drawn by render/roof_layer.gd
		1: tex = _tex("structures/door_closed.png")
		3:
			var kind := code / 100
			var name: String = STRUCT_NAME[kind] if kind < STRUCT_NAME.size() else "crate"
			tex = _tex("structures/%s.png" % name)
		4, 5, 7, 10, 25:
			_ci.draw_rect(r.grow(-3), Color(1.0, 0.8, 0.3, 0.5), false, 2.0)
			return
		11:
			return
		6:
			_ci.draw_rect(r.grow(-6), Color(0.6, 0.45, 0.2, 0.35))
			return
		8, 9, 24:
			return
	if tex:
		var fw: int = mini(TILE, tex.get_width())
		_ci.draw_texture_rect_region(tex, Rect2(r.position + Vector2(0, TILE - tex.get_height()), Vector2(fw, tex.get_height())), Rect2(0, 0, fw, tex.get_height()), ghost)
	else:
		_ci.draw_rect(r.grow(-2), ghost, false, 2.0)
	if progress > 0:
		_ci.draw_rect(Rect2(r.position + Vector2(4, TILE - 5), Vector2((TILE - 8) * progress / 100.0, 3)), Color(1, 0.9, 0.4, 0.9))

## Does the wall at (x, y) run north-south? (A built or planned wall above or below it.)
func _wall_run_vertical(x: int, y: int) -> bool:
	for dy in [-1, 1]:
		var t := tile_at(view_z, x, y + dy)
		if shape_of(t) == 2:
			return true
		for j in range(0, designations.size(), DESIG_STRIDE):
			if designations[j + 4] in [0, 1] and designations[j + 1] == x and designations[j + 2] == y + dy and designations[j + 3] == view_z:
				return true
	return false

var swapped_cache := {}
func _swapped(path: String, group: String, idx: int) -> Texture2D:
	var key := "%s|%s|%d" % [path, group, idx]
	if swapped_cache.has(key):
		return swapped_cache[key]
	var tex := _tex(path)
	if tex == null or assets.swaps.is_empty() or not assets.swaps.has(group):
		swapped_cache[key] = tex
		return tex
	var g = assets.swaps[group]
	var src: Array = g.get("source", [])
	var targets: Array = g.get("targets", [])
	if targets.is_empty():
		swapped_cache[key] = tex
		return tex
	var dst: Array = targets[idx % targets.size()]
	var img := tex.get_image()
	img.convert(Image.FORMAT_RGBA8)
	var map := {}
	for k in range(mini(src.size(), dst.size())):
		map[Color.html(src[k]).to_rgba32()] = Color.html(dst[k])
	for py in range(img.get_height()):
		for px in range(img.get_width()):
			var c := img.get_pixel(px, py)
			if c.a == 0.0: continue
			var k := c.to_rgba32()
			if map.has(k):
				img.set_pixel(px, py, map[k])
	var out := ImageTexture.create_from_image(img)
	swapped_cache[key] = out
	return out
