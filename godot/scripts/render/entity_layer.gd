extends Node2D
## Plants, structures, items and creatures as pooled, y-sorted sprites, so each can carry its
## own material (wind sway on canopies; rim light on people later) and cast a shadow on the
## ground beneath. world_view.gd still draws ground, walls and designation ghosts underneath.
##
## Sort key: every sprite's position is its foot (bottom-left of the tile it stands on), with
## a small per-kind lift so that on one row plants < structures < items < creatures, as the
## immediate renderer did.

const TILE := 32
const ADULT_HAIR := ["crop", "bowl", "long", "ponytail", "bun", "balding", "braids", "crest", "curly", "fringe"]
const ADULT_BODY := {"slim": "people/expressive_adult_slim_body.png", "average": "people/expressive_bowl_body.png", "broad": "people/expressive_long_body.png"}
const PERSON_LAYERS := ["body", "bruise_left", "bruise_right", "cloth", "hair", "band", "equip"]
const Rules := preload("res://scripts/rules.gd")
const TREE_SPECIES := ["oak", "birch", "pine"]
## The valley's other trees (#286): each its own kind, drawn like the three species by stage,
## with autumn and winter sheets; the yew is evergreen and only snowed on.
const TREE_KINDS := ["alder", "ash", "yew", "willow", "crab_apple"]
const EVERGREEN := ["pine", "yew"]
## Field crops by four growth stages (`<name>_0..3.png`), like the grain.
const CROPS := ["grain", "root", "barley", "oats", "rye", "peas", "flax"]
## Wild plants by three growth stages with optional season sheets (tools/art/plants.py).
const STAGED := ["herbs", "fern", "vine", "root_crops", "flowering_plant", "hazel", "bilberry",
	"wild_strawberry", "bramble", "wild_garlic", "chanterelle", "cattail", "death_cap", "hemlock",
	"nightshade", "lily_of_the_valley", "nettle", "woad", "madder", "weld", "yarrow", "comfrey"]
const FLAT_STRUCT := ["stockpile", "bed", "fence_post"]
# Sheets whose frames are states, not animation (tools/art/structures.py): frame 0 cold or
# unlit, frame 1 lit. They show the lit frame while active and never cycle; cycling them made a
# lit lamp blink between dark and lit (issue 38). The hearth's three frames and the cooking
# pot's two are all flame, so those still animate.
const STATE_STRUCT := ["lamp", "smelter", "furnace", "kiln", "portal_stone"]
const STATE_LIT := 1

# People sheet columns (tools/art/people.py FRAMES; also swaps.json "layout").
const P_IDLE := 0      # idle, idle2, idle3
const P_GLANCE := 3
const P_WALK := 4      # walk1-6: contact, down, passing x2
const P_SWING := 10    # swing1-3: wind-up, swing, strike (chop, mine)
const P_STOOP := 13    # stoop1-3: bend, reach, draw back (hoe, field, forage, beat out, douse)
const P_TAP := 16      # tap1-3: raise, strike, recoil (build, craft, cook)
const P_HOLD := 19     # standing with a load
const P_CARRY := 20    # carry1-6: the walk with a load
const P_SLEEP := 26
const P_SIT := 27
const P_STRIKE := 28   # strike1-3: draw back, thrust, recover (a fight, a hunt's blow)
const P_HIT := 31      # struck
const P_FALL := 32     # falling
const P_LYING := 33    # down
const P_TALK := 34     # talk1-3: turn, gesture, settle (#485, Activity::Talking)
const P_EAT := 37      # eat1-3: lift, hand to mouth, lower (#485, Activity::Eating)
const WALK_FPS := 10.0
## Body sets that have layer sheets of their own (a biped kind on its own rig, tools/art/people.py
## RIG): body set -> bool, looked up once per set.
var own_rig := {}
const WALK_BOB := [1, 1, 0, 1, 1, 0]   # body bob per walk frame, so a carried load rides along
const SWING_TIMES := [0.38, 0.1, 0.32]
const STOOP_TIMES := [0.3, 0.4, 0.3]
const TAP_TIMES := [0.2, 0.12, 0.18]
# Animal sheet columns (tools/art/animals.py FRAMES): idle, idle2, walk1-6, sleep. The cell size
# comes from the sheet (width / A_FRAMES, height / 4): the deer is 32x48, the bear 48x48.
const A_FRAMES := 9
const A_WALK := 2
const A_SLEEP := 8
# Where a carried item's 16 px icon sits, by facing (S, W, E, N), relative to the foot: in both
# arms at the chest, below the face; out in front when side-on; behind the body (only the top
# showing over the shoulders) when walking away.
const CARRY_AT := [Vector2(8, -TILE + 14), Vector2(4, -TILE + 14), Vector2(12, -TILE + 14), Vector2(8, -TILE + 8)]
## People are drawn in 48 px cells since style bible v2 (tools/art/people.py writes the cell size
## into swaps.json); the cell is centred on its tile with the feet on the tile's bottom edge.
const CARRY_AT_48 := [Vector2(8, -24), Vector2(2, -25), Vector2(14, -25), Vector2(8, -36)]

func _pcell(plan := "") -> int:
	var cell := int(v.assets.swaps.get("layout", {}).get("cell", TILE))
	return int(v.assets.swaps.get("plan_cells", {}).get(plan, cell))

func _carry_at(facing: int, plan := "") -> Vector2:
	var at: Vector2 = CARRY_AT_48[facing] if _pcell() == 48 else CARRY_AT[facing]
	if _pcell(plan) != _pcell():
		# a plan with its own cell (taur, huge, serpentine): the load rides at its chest
		at.y += _plan_dy(plan) - float(v.assets.swaps.get("plan_carry_lift", {}).get(plan, 0))
	return at

## A plan two tiles across (huge, serpentine) stands on a 2 x 2 block centred on its tile, so
## the bottom of its cell is half a tile below the tile's.
func _plan_dy(plan: String) -> float:
	var tiles := int(v.assets.swaps.get("plan_tiles", {}).get(plan, 1))
	return (tiles - 1) * TILE / 2.0

func _person_offset(plan := "") -> Vector2:
	var pc := _pcell(plan)
	return Vector2(-(pc - TILE) / 2.0, -pc + _plan_dy(plan))

var v   # world_view
var shadows: Node2D
var sway_tree: ShaderMaterial
var sway_low: ShaderMaterial
var blob: Texture2D
## A soft white disc, tinted for ground effects (the hollow, patches of wrong weather).
var disc: Texture2D
var _pool := {}
var _used := {}
var animated := []   # [kind, node, data...] creatures, refreshed every frame
var animated_static := []  # lit hearths and other animated structures
# Relationship lines (#192, WP-08 8.4): the viewer asks who is bound to whom, and the
# valley answers with what it knows — one line per pair, coloured by the from-side's
# opinion band (the same thresholds as the inspect panel's "people:" line; when the two
# sides disagree the lower row's colour stands, per the bridge's doc). Toggle: R.
const REL_BAND_COLOUR := {
	1: Color(0.847, 0.569, 0.247), # friend
	2: Color(0.788, 0.706, 0.345), # likes
	3: Color(0.561, 0.604, 0.525), # knows
	4: Color(0.639, 0.302, 0.302), # dislikes
}
var relationships_visible := false
var rel_triples := PackedInt32Array()
var rel_lines: Node2D
const STATIC := ["sprite", "shadow", "iblob"]
const DYNAMIC := ["person", "asprite", "ashadow", "blob"]
var sun := 1.0
var hour := 12.0

func init(view, shadow_parent: Node2D) -> void:
	v = view
	shadows = shadow_parent
	y_sort_enabled = true
	rel_lines = Node2D.new()
	rel_lines.name = "relationship_lines"
	add_child(rel_lines)
	var sh: Shader = load("res://shaders/sway.gdshader")
	sway_tree = ShaderMaterial.new(); sway_tree.shader = sh
	sway_tree.set_shader_parameter("amp", 1.6)
	sway_low = ShaderMaterial.new(); sway_low.shader = sh
	sway_low.set_shader_parameter("amp", 0.8)
	blob = _make_blob()
	disc = _make_disc()

func set_wind(wind: float) -> void:
	sway_tree.set_shader_parameter("wind", wind)
	sway_low.set_shader_parameter("wind", wind)

func _make_blob() -> Texture2D:
	var img := Image.create(22, 8, false, Image.FORMAT_RGBA8)
	for y in 8:
		for x in 22:
			var dx := (x - 10.5) / 11.0
			var dy := (y - 3.5) / 4.0
			var a := clampf(1.0 - (dx * dx + dy * dy), 0.0, 1.0)
			img.set_pixel(x, y, Color(0, 0, 0, a * a))
	return ImageTexture.create_from_image(img)

func _make_disc() -> Texture2D:
	var img := Image.create(32, 32, false, Image.FORMAT_RGBA8)
	for y in 32:
		for x in 32:
			var dx := (x - 15.5) / 16.0
			var dy := (y - 15.5) / 16.0
			var a := clampf(1.0 - (dx * dx + dy * dy), 0.0, 1.0)
			img.set_pixel(x, y, Color(1, 1, 1, sqrt(a)))
	return ImageTexture.create_from_image(img)

## A soft tinted disc on the ground, under everything standing on it.
func _disc(center: Vector2, radius: float, col: Color) -> void:
	var s: Sprite2D = _take("blob")
	s.texture = disc
	s.region_rect = Rect2(0, 0, 32, 32)
	s.offset = Vector2.ZERO
	s.scale = Vector2(radius / 16.0, radius / 16.0 * 0.85)
	s.position = center - Vector2(radius, radius * 0.85)
	s.modulate = col

## The thin place at the portal stone: the hollow (grey, where nothing grows), patches of the
## wrong season (snow or thaw), and an echo standing by the stone at dusk, pale and see-through.
func _thin(z: int) -> void:
	var T: PackedInt32Array = v.thin
	if T.size() < 6 or T[2] != z:
		return
	var at := Vector2(T[0] * TILE + 16, T[1] * TILE + 16)
	var hollow := T[4] / 10.0
	if hollow > 0.0:
		_disc(at, hollow * TILE, Color(0.36, 0.33, 0.42, 0.38))
	var n: int = T[5]
	var k := 6
	var body: Texture2D = v._tex("people/body_average_adult.png")
	for e in n:
		if body != null and T[k + 2] == z:
			var foot := Vector2(T[k] * TILE, (T[k + 1] + 1) * TILE - 1)
			var s := _sprite(body, Rect2(0, 0, _pcell(), _pcell()), foot, null, Color(0.78, 0.9, 1.0, 0.42), "asprite")
			s.offset = _person_offset()
		k += 3
	if k >= T.size():
		return
	var m: int = T[k]
	k += 1
	for p in m:
		if T[k + 2] == z:
			var c := Vector2(T[k] * TILE + 16, T[k + 1] * TILE + 16)
			_disc(c, 3.5 * TILE, Color(0.95, 0.97, 1.0, 0.45) if T[k + 3] == 1 else Color(0.62, 0.78, 0.36, 0.32))
		k += 4

# ---------------------------------------------------------------- pooling
func _new_sprite(parent: Node) -> Sprite2D:
	var s := Sprite2D.new()
	s.centered = false
	s.region_enabled = true
	parent.add_child(s)
	return s

func _new_person(parent: Node) -> Node2D:
	var p := Node2D.new()
	for n in PERSON_LAYERS:
		var s := _new_sprite(p)
		s.name = n
		if n in ["bruise_left", "bruise_right"]:
			# Discolor the person's inherited skin instead of covering it with an opaque patch.
			s.self_modulate.a = 0.74
		s.offset = _person_offset()
	var c := Sprite2D.new()
	c.name = "carry"
	c.centered = false
	c.position = _carry_at(0)
	p.add_child(c)
	# The activity emote (#192, WP-08 8.4): the work's own tool icon bobbing over the head,
	# only when the watcher is close (the roof cut-away's zoom). Hidden until animate() sets it.
	var em := Sprite2D.new()
	em.name = "emote"
	em.centered = true
	em.visible = false
	em.set_meta("kind", 0)
	p.add_child(em)
	parent.add_child(p)
	return p

func _take(kind: String) -> Node:
	var arr: Array = _pool.get(kind, [])
	var n: int = _used.get(kind, 0)
	var node: Node
	if n < arr.size():
		node = arr[n]
	else:
		match kind:
			"person": node = _new_person(self)
			"shadow", "blob", "iblob", "ashadow": node = _new_sprite(shadows)
			_: node = _new_sprite(self)
		arr.append(node)
		_pool[kind] = arr
	_used[kind] = n + 1
	node.visible = true
	return node

func _sprite(tex: Texture2D, region: Rect2, foot: Vector2, mat: Material = null, mod := Color(1, 1, 1), pool := "sprite") -> Sprite2D:
	var s: Sprite2D = _take(pool)
	s.texture = tex
	s.region_rect = region
	s.position = foot
	s.offset = Vector2(0, -region.size.y)
	s.material = mat
	s.modulate = mod
	return s

## A sun shadow: the sprite flipped onto the ground, flattened and leaned away from the sun.
func _shadow(tex: Texture2D, region: Rect2, foot: Vector2, strength: float, pool := "shadow") -> void:
	if sun < 0.06 or strength <= 0.0:
		return
	var s: Sprite2D = _take(pool)
	s.texture = tex
	s.region_rect = region
	s.flip_v = true
	s.offset = Vector2.ZERO
	var t := clampf((hour - 13.0) / 6.0, -1.0, 1.0)
	s.position = foot + Vector2(0, -2)
	s.scale = Vector2(1.0, 0.26 + 0.3 * absf(t))
	s.skew = -t * 0.85
	s.modulate = Color(0, 0, 0, 0.24 * strength * sun)

func _blob(center: Vector2, a: float, pool := "blob", size := 1.0) -> void:
	var s: Sprite2D = _take(pool)
	s.texture = blob
	s.region_rect = Rect2(0, 0, 22, 8)
	s.offset = Vector2.ZERO
	s.scale = Vector2(size, size)
	s.position = center - Vector2(11, 5) * size
	s.modulate = Color(1, 1, 1, a)

# ---------------------------------------------------------------- rebuild
## Rebuild what is in view. Creatures move every tick, so they are rebuilt on every refresh;
## plants, structures and items only when `with_static` (the view moved, or half a second
## passed), which keeps a wide view of a forest cheap.
func rebuild(cull: Rect2i, with_static: bool) -> void:
	var groups: Array = DYNAMIC + (STATIC if with_static else [])
	for k in groups:
		_used[k] = 0
	animated.clear()
	sun = v.sun_level()
	hour = v.hour
	var z: int = v.view_z
	var x0 := cull.position.x - 2
	var y0 := cull.position.y - 2
	var x1 := cull.position.x + cull.size.x + 2
	var y1 := cull.position.y + cull.size.y + 4
	if with_static and not v.rust_renderer_enabled:
		animated_static.clear()
		var P: PackedInt32Array = v.plants
		for i in range(0, P.size(), 6):
			if P[i + 2] == z and P[i] >= x0 and P[i] < x1 and P[i + 1] >= y0 and P[i + 1] < y1:
				_plant(i)
		var S: PackedInt32Array = v.structures
		for i in range(0, S.size(), 8):
			if S[i + 3] == z and S[i + 1] >= x0 and S[i + 1] < x1 and S[i + 2] >= y0 and S[i + 2] < y1:
				_structure(i)
		var B: PackedInt32Array = v.bands
		for i in range(0, B.size(), 9):
			if B[i + 4] == 1 and B[i + 3] == z and B[i + 1] >= x0 and B[i + 1] < x1 and B[i + 2] >= y0 and B[i + 2] < y1:
				_camp(i)
		var I: PackedInt32Array = v.items
		for i in range(0, I.size(), v.ITEM_STRIDE):
			if I[i + 3] == z and I[i + 1] >= x0 and I[i + 1] < x1 and I[i + 2] >= y0 and I[i + 2] < y1:
				_item(i)
	_thin(z)
	# People use layered sprites in both renderers for faces, clothing and held items.
	var A: PackedInt32Array = v.agents
	for i in range(0, A.size(), v.AGENT_STRIDE):
		if v.rust_renderer_enabled and A[i + 4] != 0:
			continue
		if A[i + 3] == z and A[i + 1] >= x0 and A[i + 1] < x1 and A[i + 2] >= y0 and A[i + 2] < y1:
			_agent(i)
	for k in groups:
		if not _pool.has(k):
			continue
		var arr: Array = _pool[k]
		for j in range(_used.get(k, 0), arr.size()):
			arr[j].visible = false
	_cache_relationship_lines()
	animate()

# The pairs, in agents() row space, from the bridge; cached per rebuild and drawn per
# frame in animate() so the lines ride along as people walk.
func _cache_relationship_lines() -> void:
	rel_triples = PackedInt32Array()
	if not relationships_visible or v.sim == null:
		return
	var R: PackedInt32Array = v.sim.relationship_lines()
	if R.size() % 3 == 0:
		rel_triples = R

func _draw_relationship_lines() -> void:
	for c in rel_lines.get_children():
		rel_lines.remove_child(c)
		c.queue_free()
	if rel_triples.is_empty() \
			or get_viewport().get_canvas_transform().get_scale().x < 1.75:
		return
	# Row -> the person's head anchor, from the live entity nodes (person entries only;
	# animals take rows but hold no relationships).
	var head := {}
	for e in animated:
		if e[0] == "person":
			# e[2] is the agents() array offset; the bridge numbers rows (0, 1, 2...).
			head[int(e[2] / v.AGENT_STRIDE)] = e[1].position + Vector2(16, _person_offset(e[3]).y - 12)
	for k in range(0, rel_triples.size(), 3):
		var a = head.get(rel_triples[k], null)
		var b = head.get(rel_triples[k + 1], null)
		if a == null or b == null:
			continue
		var ln := Line2D.new()
		ln.points = PackedVector2Array([a, b])
		ln.default_color = REL_BAND_COLOUR.get(rel_triples[k + 2], Color(0.6, 0.6, 0.6))
		ln.width = 2.0
		rel_lines.add_child(ln)

func _plant(i: int) -> void:
	var P: PackedInt32Array = v.plants
	var px: int = P[i]
	var py: int = P[i + 1]
	var kind: int = P[i + 3]
	var growth: float = P[i + 4] / 100.0
	var variant: int = P[i + 5]
	var foot := Vector2(px * TILE, (py + 1) * TILE - 4)
	var name: String = v.PLANT_KIND[kind] if kind < v.PLANT_KIND.size() else "shrub"
	var tex: Texture2D = null
	if name == "tree" or name in TREE_KINDS:
		var species: String = TREE_SPECIES[variant % 3] if name == "tree" else name
		var stage := 0 if growth < 0.35 else (1 if growth < 0.8 else 2)
		# Season sheets: russet and gold canopies in autumn, bare snowy crowns in winter,
		# snow-laden pines while snow lies; the plain sheet otherwise or if one is missing.
		var suffix := ""
		if not species in EVERGREEN:
			if v.season == 2: suffix = "_autumn"
			elif v.season == 3: suffix = "_winter"
		elif v.snowing():
			suffix = "_winter"
		tex = v._tex("plants/%s_%d%s.png" % [species, stage, suffix])
		if tex == null:
			tex = v._tex("plants/%s_%d.png" % [species, stage])
		if tex == null:
			return
		var tint := Color(1, 1, 1)
		var tw := tex.get_width()
		var th := tex.get_height()
		var at := foot + Vector2((TILE - tw) / 2.0, 0)
		var reg := Rect2(0, 0, tw, th)
		_sprite(tex, reg, at, sway_tree, tint)
		_shadow(tex, reg, at, 1.0)
		return
	var stage := 0 if growth < 0.35 else (1 if growth < 0.8 else 2)
	if name in CROPS:
		tex = v._tex("plants/%s_%d.png" % [name, clampi(int(growth * 4), 0, 3)])
	elif name in STAGED:
		# Each stage has its own season sheets: autumn colour (not herbs) and the dormant
		# winter plant, dry stems and seed heads under snow (tools/art/plants.py).
		tex = _season_tex("plants/%s_%d" % [name, stage])
	else:
		match name:
			"berry_bush":
				tex = _season_tex("plants/berry_bush_%s" % ("fruit" if growth > 0.6 else "bare"))
			"reeds":
				# Young stands grow through reed_0 and reed_1; a grown stand is reed_2 or one of
				# the three full clumps in reeds.png, by variant, so a reed bed is not one stamp.
				if stage < 2 or variant % 4 == 0:
					tex = _season_tex("plants/reed_%d" % stage)
				else:
					tex = _season_tex("plants/reeds")
					variant = variant / 4
			"wildflowers":
				if v.season == 3:
					return # gone under the winter; they come back in spring
				tex = v._tex("plants/wildflowers.png")
			"stump":
				tex = v._tex("plants/stump.png")
			"fallen_log":
				tex = v._tex("plants/log.png")
			_:
				tex = _season_tex("plants/%s" % name)
	if tex == null:
		return
	var variants := int(tex.get_width() / TILE)
	var vv := variant % maxi(1, variants)
	var reg2 := Rect2(vv * TILE, 0, TILE, tex.get_height())
	var still: bool = name == "stump" or name == "fallen_log"
	_sprite(tex, reg2, foot, null if still else sway_low)
	if tex.get_height() > 20:
		_shadow(tex, reg2, foot, 0.6)

## The season's sheet for a plant when one exists (`<base>_autumn.png`, `<base>_winter.png`,
## tools/art/plants.py), else the plain sheet.
func _season_tex(base: String) -> Texture2D:
	var t: Texture2D = null
	if v.season == 2:
		t = v.assets.optional(base + "_autumn.png")
	elif v.season == 3:
		t = v.assets.optional(base + "_winter.png")
	if t == null:
		t = v._tex(base + ".png")
	return t

func _structure(i: int) -> void:
	var S: PackedInt32Array = v.structures
	var x: int = S[i + 1]
	var y: int = S[i + 2]
	var kind: int = S[i + 4]
	var active: bool = S[i + 6] == 1
	var name: String = v.STRUCT_NAME[kind] if kind < v.STRUCT_NAME.size() else "crate"
	var tex: Texture2D = v._tex("structures/%s.png" % name)
	if tex == null:
		return
	var foot := Vector2(x * TILE, (y + 1) * TILE - 3)
	var th := tex.get_height()
	var frames := int(tex.get_width() / TILE)
	var states := STATE_STRUCT.has(name)
	var f := STATE_LIT if states and active and frames > STATE_LIT else 0
	var reg := Rect2(f * TILE, 0, TILE, th)
	var s := _sprite(tex, reg, foot)
	if frames > 1 and active and not states:
		animated_static.append(["struct", s, frames, th, x + y])
	if not FLAT_STRUCT.has(name):
		# The unlit frame's outline: a lamp's lit frame carries a glow halo that is not solid.
		_shadow(tex, Rect2(0, 0, TILE, th), foot, 0.8)

func _item(i: int) -> void:
	var I: PackedInt32Array = v.items
	var x: int = I[i + 1]
	var y: int = I[i + 2]
	var kind: int = I[i + 4]
	var name: String = v.ITEM_NAME[kind] if kind < v.ITEM_NAME.size() else "log"
	# On the ground, the 32 px icon derived from a reference sheet (style bible v2); the 16 px
	# one is what people carry, and the fallback for a kind with no 32 px icon yet.
	# A stack of a particular material (C32) has its own icon where one is drawn: birch logs,
	# marble blocks, garnets. The kind's own icon otherwise.
	var tex: Texture2D = null
	var mat: int = I[i + 6]
	if mat > 0 and mat < v.mat_slugs.size():
		tex = v.assets.optional("items32/%s_%s.png" % [name, v.mat_slugs[mat]])
	if tex == null:
		tex = v.assets.optional("items32/%s.png" % name)
	if tex == null:
		tex = v._tex("items/%s.png" % name)
	if tex == null:
		return
	var tw := tex.get_width()
	var th := tex.get_height()
	var foot := Vector2(x * TILE + (TILE - tw) / 2.0, y * TILE + (TILE + th) / 2.0)
	_sprite(tex, Rect2(0, 0, tw, th), foot)
	_blob(Vector2(x * TILE + 16, foot.y - 1), 0.25, "iblob")

## A band's camp (threat.rs): a lean-to, a small fire to its south-east, and a bedroll for
## each of them still here, up to three.
func _camp(i: int) -> void:
	var B: PackedInt32Array = v.bands
	var cx: int = B[i + 1]
	var cy: int = B[i + 2]
	var lean: Texture2D = v._tex("structures/lean_to.png")
	if lean != null:
		var lw := lean.get_width()
		var lh := lean.get_height()
		var foot := Vector2(cx * TILE + (TILE - lw) / 2.0, (cy + 1) * TILE - 2)
		_sprite(lean, Rect2(0, 0, lw, lh), foot)
		_shadow(lean, Rect2(0, 0, lw, lh), foot, 0.9)
	var fire: Texture2D = v._tex("structures/campfire.png")
	if fire != null:
		var fh := fire.get_height()
		var s := _sprite(fire, Rect2(0, 0, TILE, fh), Vector2((cx + 1) * TILE, (cy + 2) * TILE - 3))
		animated_static.append(["struct", s, int(fire.get_width() / TILE), fh, cx + cy])
	var roll: Texture2D = v._tex("structures/bedroll.png")
	if roll != null:
		var spots := [Vector2i(-1, 1), Vector2i(0, 2), Vector2i(-2, 2)]
		for k in range(mini(B[i + 7], spots.size())):
			var at: Vector2i = spots[k]
			_sprite(roll, Rect2(0, 0, TILE, roll.get_height()), Vector2((cx + at.x) * TILE, (cy + at.y + 1) * TILE - 2))

func _agent(i: int) -> void:
	var A: PackedInt32Array = v.agents
	var x: int = A[i + 1]
	var y: int = A[i + 2]
	var species: int = A[i + 4]
	var fx: int = A[i + 12]
	var fy: int = A[i + 13]
	var prog: float = A[i + 14] / 1000.0
	var base := Vector2(lerpf(fx, x, prog) * TILE, lerpf(fy, y, prog) * TILE)
	# A change of level (#324): lift the sprite through the step in an arc, so a ramp or a
	# ledge reads as a climb up or a drop down, not a slide across flat ground.
	var dz: int = A[i + 3] - A[i + 23]
	if dz != 0 and prog < 1.0:
		base.y -= sin(prog * PI) * 10.0 * absi(dz)
	var foot := base + Vector2(0, TILE - 1)
	if species != 0:
		var name: String = v.ANIMAL_NAME[species] if species < v.ANIMAL_NAME.size() else "deer"
		var tex: Texture2D = v._tex("animals/%s.png" % name)
		if tex == null:
			return
		var ch := int(tex.get_height() / 4)
		var cw := int(tex.get_width() / A_FRAMES)
		# a wide cell (the bear) is centred on its tile
		var s := _sprite(tex, Rect2(0, 0, cw, ch), foot - Vector2((cw - TILE) / 2.0, 0), null, Color(1, 1, 1), "asprite")
		_blob(base + Vector2(16, TILE - 3), 0.35, "blob", float(cw) / TILE)
		animated.append(["animal", s, i, ch, cw])
		return
	var p: Node2D = _take("person")
	p.position = foot
	var carry_kind: int = v.carried.get(A[i], -1)
	var plan := dress_person(p, A, i, carry_kind)
	var carry: Sprite2D = p.get_node("carry")
	carry.texture = null
	var icon_scale := 1.0
	var iname := ""
	if carry_kind >= 0 and carry_kind < v.ITEM_NAME.size():
		# The same icon as on the ground, its material's where one is drawn, at half size: the
		# old 16 px set was drawn apart from the 32 px one (#186, QA 2026-10-20, C63).
		iname = v.ITEM_NAME[carry_kind]
		var mat: int = v.carried_mat.get(A[i], 0)
		if mat > 0 and mat < v.mat_slugs.size():
			carry.texture = v.assets.optional("items32/%s_%s.png" % [iname, v.mat_slugs[mat]])
		if carry.texture == null:
			carry.texture = v.assets.optional("items32/%s.png" % iname)
		if carry.texture != null:
			var big: int = maxi(carry.texture.get_width(), carry.texture.get_height())
			icon_scale = 16.0 / float(maxi(big, 1))
		else:
			carry.texture = v._tex("items/%s.png" % iname)
	_set_carry_stack(carry, iname, v.carried_count.get(A[i], 0), icon_scale)
	carry.visible = carry.texture != null
	_blob(base + Vector2(16, TILE - 3), 0.4)
	animated.append(["person", p, i, plan])

## A handful of building materials uses the same icon and hand anchor for every unit.
## Keep the far pieces inside the silhouette of the carrier, clear of the lead hand.
func _set_carry_stack(carry: Sprite2D, kind: String, count: int, icon_scale: float) -> void:
	# The 32 px ground plank icon in each wood shows two boards. Use one face per
	# held unit so a real pair still reads as two against the loom or the carrier.
	var plank := kind == "plank" and carry.texture != null and carry.texture.get_size() == Vector2(32, 32)
	carry.scale = Vector2.ONE * icon_scale
	carry.region_enabled = plank
	carry.offset = Vector2.ZERO
	carry.self_modulate = Color.WHITE
	if plank:
		carry.region_rect = Rect2(0, 6, 32, 10)
		carry.scale = Vector2(0.76, 0.72)
		carry.offset = Vector2(-4, 6) / carry.scale
		# Keep the wood texture's own colour, with a lighter cut face in the hands.
		carry.self_modulate = Color(1.65, 1.57, 1.45)
	var extras := 0
	if carry.texture != null and kind in ["log", "stone_block", "plank"]:
		extras = clampi(count - 1, 0, 2)
	while carry.get_child_count() < extras:
		var piece := Sprite2D.new()
		piece.centered = false
		piece.show_behind_parent = true
		carry.add_child(piece)
	for i in carry.get_child_count():
		var piece: Sprite2D = carry.get_child(i)
		piece.visible = i < extras
		if piece.visible:
			piece.texture = carry.texture
			piece.region_enabled = plank
			piece.region_rect = carry.region_rect
			piece.offset = carry.offset
			piece.self_modulate = carry.self_modulate
			# Back row first, then the top unit. Work in world pixels for either icon resolution.
			var delta := Vector2(4, -1) if i == 0 else Vector2(2, -5)
			if plank:
				delta = Vector2(4, -4) if i == 0 else Vector2(1, -8)
			piece.position = delta / carry.scale

## Dress each pooled sprite with the current person's inherited appearance and equipment.
func dress_person(p: Node2D, A: PackedInt32Array, i: int, carry_kind: int) -> String:
	# Strangers from beyond the valley: road dust and hunger on them, so they read apart from
	# the settlers at a glance.
	p.modulate = Color(0.86, 0.8, 0.72) if A[i + 21] != 0 else Color(1, 1, 1)
	var body: int = A[i + 6]
	var age := "child" if A[i + 16] == 1 else "adult"
	var build: String = v.BUILD[body % 3]
	# Every layer is drawn per build and age (tools/art/people.py), so each sits on the same
	# skeleton as the body under it in every frame. The average adult's hair, clothing and
	# equipment sheets have no suffix.
	var ba := "%s_%s" % [build, age]
	var sfx := "" if ba == "average_adult" else "_" + ba
	# Their species (P3): its own body sheet (head, markings), and the clothing, hair and
	# equipment of its body plan; a people without hair draws none.
	var sp: int = A[i + 22]
	if not v.species_art.has(sp):
		v.species_art = v.sim.species_art()
	var art: Dictionary = v.species_art.get(sp, {})
	var body_set: String = art.get("body", "human")
	var plan: String = art.get("plan", "biped")
	var body_name := ba if body_set == "human" else "%s_%s" % [body_set, ba]
	var plan_ba := ba
	if plan != "biped":
		plan_ba = "%s_%s" % [plan, ba]
		sfx = "_" + plan_ba
	elif body_set != "human":
		# A biped kind drawn on a rig of its own (taller, stooped: tools/art/people.py RIG) wears
		# hair, clothing, equipment and bandage drawn on that rig: <layer>_<body>_<build>_<age>.
		if not own_rig.has(body_set):
			var probe := "res://assets/people/overlay_bandage_%s_average_adult.png" % body_set
			own_rig[body_set] = ResourceLoader.exists(probe) or FileAccess.file_exists(probe)
		if own_rig[body_set]:
			plan_ba = "%s_%s" % [body_set, ba]
			sfx = "_" + plan_ba
	# Each plan's sheets use its own cell (swaps.json "plan_cells"): cut and anchor by it.
	for n in PERSON_LAYERS:
		p.get_node(n).offset = _person_offset(plan)
	p.get_node("emote").position = Vector2(16, _person_offset(plan).y - 12)
	var layers := [
		["body", "people/body_%s.png" % body_name, "skin", A[i + 7]],
		["cloth", "people/clothing_%s%s.png" % [v.OUTFIT[A[i + 10] % 8], sfx], "cloth", A[i + 11]],
		["hair", "people/hair_%02d%s.png" % [A[i + 8] % 10 + 1, sfx], "hair", A[i + 9]],
	]
	# Inherited hair selects its fitted sheet. All densities share the same logical rig.
	if body_set == "human" and age == "adult":
		var style: String = ADULT_HAIR[A[i + 8] % ADULT_HAIR.size()]
		layers[0][1] = ADULT_BODY[build]
		layers[2][1] = "people/expressive_adult_%s_%s.png" % [build, style]
		if build == "average" and style == "bowl":
			layers[2][1] = "people/expressive_bowl_hair.png"
			if v.OUTFIT[A[i + 10] % 8] == "vest":
				layers[1][1] = "people/expressive_bowl_clothing.png"
		elif build == "broad" and style == "long":
			layers[2][1] = "people/expressive_long_hair.png"
	for l in layers:
		var s: Sprite2D = p.get_node(l[0])
		s.texture = v._swapped(l[1], l[2], l[3])
		if l[0] == "hair" and not art.get("hair", true):
			# #485: a kind without hair wears its head marking in this slot instead, one of
			# three chosen by the inherited hair-style gene, in the person's own skin colours.
			s.texture = v._swapped(Rules.marking_sheet(body_set, A[i + 8], ba), "skin", A[i + 7])
		s.visible = s.texture != null
	var marks: int = v.arm_bruises.get(A[i], 0)
	for side in ["left", "right"]:
		var bruise: Sprite2D = p.get_node("bruise_" + side)
		var bit := 1 if side == "left" else 2
		bruise.texture = v._tex("people/overlay_bruise_%s_%s.png" % [side, ba]) if body_set == "human" and (marks & bit) != 0 else null
		bruise.visible = bruise.texture != null
	var band: Sprite2D = p.get_node("band")
	band.texture = v._tex("people/overlay_bandage_%s.png" % plan_ba) if A[i + 17] == 1 and (marks == 0 or (marks & 4) != 0) else null
	band.visible = band.texture != null
	var equipment: int = A[i + 19]
	var eq: Sprite2D = p.get_node("equip")
	eq.texture = null
	if equipment >= 0 and equipment < v.EQUIPMENT.size() and carry_kind < 0:
		eq.texture = v._tex("people/equipment_%s%s.png" % [v.EQUIPMENT[equipment], sfx])
	eq.visible = eq.texture != null
	return plan

# ---------------------------------------------------------------- per frame
func animate() -> void:
	var t: float = v.anim_t
	sway_tree.set_shader_parameter("presentation_time", t)
	sway_low.set_shader_parameter("presentation_time", t)
	_draw_relationship_lines()
	var A: PackedInt32Array = v.agents
	for e in animated_static:
		var f := int(t * 6.0) % int(e[2])
		e[1].region_rect = Rect2(f * TILE, 0, TILE, e[3])
	for e in animated:
		match e[0]:
			"struct":
				var f := int(t * 6.0) % int(e[2])
				e[1].region_rect = Rect2(f * TILE, 0, TILE, e[3])
			"animal":
				var i: int = e[2]
				# Every pose has a frame of the sheet's own (#187, QA 2026-10-20, C63).
				var af: int = Rules.animal_frame_for_row(A, i, t)
				e[1].region_rect = Rect2(af * e[4], A[i + 5] * e[3], e[4], e[3])
			"person":
				var i: int = e[2]
				var pose: int = A[i + 15]
				# Idle/walking agents can already have a load before their activity changes.
				if pose in [0, 1] and v.carried.has(A[i]):
					pose = 3
				var ph: float = t + float(A[i] % 7) * 0.137
				var frame := P_IDLE
				var bob := 0
				match pose:
					0:
						frame = P_IDLE + int(t * 1.6 + A[i]) % 3
						if int(t * 0.8 + A[i] * 0.37) % 7 == 0:
							frame = P_GLANCE
					1:
						var k := int(ph * WALK_FPS) % 6
						frame = P_WALK + k
					2:
						var code: int = A[i + 20]
						if code == Rules.WORK_TEND:
							frame = Rules.held_frame(Rules.P_TEND, ph, Rules.TEND_TIMES)
						elif code == 1 or code == 2:
							frame = P_SWING + _cycle(ph, SWING_TIMES)
						elif code == 3 or code == 5 or code == 6:
							frame = P_TAP + _cycle(ph, TAP_TIMES)
						else:
							# hoe and field work, beating out fire, dousing, foraging (no code)
							frame = P_STOOP + _cycle(ph, STOOP_TIMES)
					3:
						if A[i + 12] != A[i + 1] or A[i + 13] != A[i + 2]:
							var k := int(ph * WALK_FPS) % 6
							frame = P_CARRY + k
							bob = WALK_BOB[k]
						else:
							frame = P_HOLD
					4: frame = Rules.sleep_frame(t, A[i])
					5: frame = P_SIT
					6: frame = P_STRIKE + int(ph * 5.0) % 3
					7: frame = P_HIT
					# The dead, where they fell (#185, view::bodies): falling, then lying.
					8, 9: frame = Rules.dead_frame(pose)
					10: frame = P_TALK + Rules.everyday_frame(pose, ph)
					11: frame = P_EAT + Rules.everyday_frame(pose, ph)
					12: frame = Rules.held_frame(Rules.P_DRINK, ph, Rules.DRINK_TIMES)
				set_person_frame(e[1], e[3], frame, A[i + 5])
				var c: Sprite2D = e[1].get_node("carry")
				if c.visible:
					var facing: int = A[i + 5] % 4
					c.position = _carry_at(facing, e[3]) + Vector2(0, bob)
					e[1].move_child(c, 0 if facing == 3 else e[1].get_child_count() - 1)
				# The activity emote (#192): the work's own tool over the head, and only when
				# the watcher is close enough to read it (the roof cut-away's zoom).
				var em: Sprite2D = e[1].get_node("emote")
				var emote_kind: int = A[i + 24] if A.size() > i + 24 else 0
				if emote_kind <= 0 or emote_kind > v.ITEM_NAME.size() \
						or get_viewport().get_canvas_transform().get_scale().x < 1.75:
					em.visible = false
					em.set_meta("kind", 0)
				elif int(em.get_meta("kind", 0)) != emote_kind:
					var ename: String = v.ITEM_NAME[emote_kind - 1]
					var tex: Texture2D = v.assets.optional("items32/%s.png" % ename)
					if tex == null:
						tex = v.assets.optional("items/%s.png" % ename)
					if tex == null:
						em.set_meta("kind", 0)
						em.visible = false
					else:
						em.texture = tex
						em.scale = Vector2.ONE * (14.0 / float(maxi(tex.get_width(), 1)))
						em.set_meta("kind", emote_kind)
						em.visible = true
				else:
					em.visible = true


## Each sheet may have a different pixel density, but body layers share a logical
## cell, foot origin and frame index in both local and travelling scenes.
func set_person_frame(person: Node2D, plan: String, frame: int, facing: int) -> void:
	var logical_cell := _pcell(plan)
	for layer_name in PERSON_LAYERS:
		var layer: Sprite2D = person.get_node(layer_name)
		if layer.texture == null or not layer.visible:
			continue
		var source_cell := layer.texture.get_height() / 4
		var density := float(source_cell) / logical_cell
		layer.region_rect = Rect2(frame * source_cell, facing * source_cell, source_cell, source_cell)
		layer.offset = _person_offset(plan) * density
		layer.scale = Vector2.ONE / density
		layer.texture_filter = CanvasItem.TEXTURE_FILTER_LINEAR if density > 1.0 else CanvasItem.TEXTURE_FILTER_PARENT_NODE

## Index of the frame within a work cycle at time t; times are each frame's hold in seconds.
func _cycle(t: float, times: Array) -> int:
	var total := 0.0
	for d in times:
		total += d
	var u := fmod(t, total)
	for k in times.size():
		u -= times[k]
		if u < 0.0:
			return k
	return times.size() - 1
