extends Node2D
## Client-only accents for facts the bridge already exposes. Effects never create damage,
## projectiles, psychic powers or environmental changes; they only make an observed event or
## visible pose easier to read at a glance.

const TILE := 32
const MAX_ACTIVE := 48
const MAX_HISTORY := 1024
const WORK_PERIOD := {1: 0.95, 2: 0.85, 3: 0.70, 4: 1.10, 5: 0.60, 6: 1.30, 7: 1.90}
const WORK_CODES := [1, 2, 3, 4, 5, 6, 7]
const SHEETS := {
	"weapon_impact": ["weapon_impact.png", 4, 32, 0.28],
	"environmental_hit": ["environmental_hit.png", 4, 32, 0.24],
	"resonance": ["resonance.png", 6, 48, 1.20],
	"fire_flare": ["fire_flare.png", 1, 32, 0.34],
}

var v
var _pool: Array[Sprite2D] = []
var _active: Array[Dictionary] = []
var _last_pose := {}
var _last_work_stamp := {}
var _last_resonance_stamp := -1

static func effect_for_sound(name: String) -> String:
	match name:
		"flare": return "fire_flare"
		"douse": return "environmental_hit"
		# These names are future-safe hooks only. The current bridge does not emit them, so no
		# unsupported projectile or psychic fact is manufactured by this renderer.
		"projectile", "projectile_flash", "weapon_impact": return "weapon_impact"
		"psychic", "resonance", "ancient_resonance": return "resonance"
	return ""

static func material_family(tile: int) -> String:
	# 2026-10-30 #414 review: ids follow hf-sim world/material.rs; packed earth and bone moss
	# are ground, while stone floors and scree are stone.
	match tile & 0xffff:
		13: return "water"
		5, 6, 7, 8, 9, 10, 11, 12, 15, 26, 27, 28, 29, 30, 31, 61, 63, 64: return "stone"
		14, 16: return "wood"
		_: return "earth"

static func material_tint(family: String) -> Color:
	match family:
		"water": return Color(0.55, 0.82, 1.0, 1.0)
		"stone": return Color(0.88, 0.91, 0.98, 1.0)
		"wood": return Color(1.0, 0.78, 0.46, 1.0)
	return Color(1.0, 0.88, 0.62, 1.0)

func init(view) -> void:
	v = view
	z_index = 3
	for _k in MAX_ACTIVE:
		var s := Sprite2D.new()
		s.centered = true
		s.region_enabled = true
		s.visible = false
		add_child(s)
		_pool.append(s)

func active_count() -> int:
	return _active.size()

func reset_world() -> void:
	# 2026-10-30 #414 review: a loaded world must not show old effects or suppress new ids.
	for e in _active:
		var s: Sprite2D = e["sprite"]
		s.visible = false
	_active.clear()
	_last_pose.clear()
	_last_work_stamp.clear()
	_last_resonance_stamp = -1

func on_sound_event(row: Array) -> void:
	if row.size() < 4:
		return
	var kind := effect_for_sound(str(row[0]))
	if kind == "":
		return
	var tile := Vector2i(int(row[1]), int(row[2]))
	if int(row[3]) != v.view_z or not v.cull_rect().grow(4).has_point(tile):
		return
	var pos := Vector2(tile.x * TILE + 16, tile.y * TILE + 16)
	if kind == "resonance":
		_spawn(kind, pos, Color(0.72, 0.9, 1.0, 1.0), Vector2.ONE)
	elif kind == "environmental_hit":
		_spawn(kind, pos, material_tint("water") if str(row[0]) == "douse" else material_tint("earth"), Vector2.ONE)
	else:
		_spawn(kind, pos, Color.WHITE, Vector2.ONE)

func update(delta: float) -> void:
	_advance(delta)
	_scan_agents()
	_scan_thin_place()

func _advance(delta: float) -> void:
	for k in range(_active.size() - 1, -1, -1):
		var e: Dictionary = _active[k]
		var age: float = float(e["age"]) + delta
		var duration: float = float(e["duration"])
		var s: Sprite2D = e["sprite"]
		if age >= duration:
			s.visible = false
			_active.remove_at(k)
			continue
		e["age"] = age
		var progress := clampf(age / duration, 0.0, 0.999)
		var frame := mini(int(e["frames"]) - 1, int(progress * float(e["frames"])))
		var cell: int = int(e["cell"])
		s.region_rect = Rect2(frame * cell, 0, cell, cell)
		var tint: Color = e["tint"]
		s.modulate = Color(tint.r, tint.g, tint.b, tint.a * (1.0 - progress))

func _take() -> Sprite2D:
	for s in _pool:
		if not s.visible:
			return s
	return null

func _spawn(kind: String, pos: Vector2, tint: Color, scale := Vector2.ONE) -> void:
	if not SHEETS.has(kind) or _active.size() >= MAX_ACTIVE:
		return
	var s := _take()
	if s == null:
		return
	var spec: Array = SHEETS[kind]
	s.texture = v._tex("effects/" + str(spec[0]))
	if s.texture == null:
		return
	s.position = pos
	s.scale = scale
	s.region_rect = Rect2(0, 0, int(spec[2]), int(spec[2]))
	s.modulate = tint
	s.visible = true
	_active.append({"kind": kind, "sprite": s, "age": 0.0, "duration": float(spec[3]), "frames": int(spec[1]), "cell": int(spec[2]), "tint": tint})

func _scan_agents() -> void:
	var A: PackedInt32Array = v.agents
	var cull: Rect2i = v.cull_rect().grow(2)
	var seen := {}
	for i in range(0, A.size(), v.AGENT_STRIDE):
		var id: int = A[i]
		seen[id] = true
		# 2026-10-31 #414: an over-cap id must stay untracked, not look new again every frame.
		if not _last_pose.has(id) and _last_pose.size() >= MAX_HISTORY:
			continue
		var pose: int = A[i + 15]
		var previous: int = int(_last_pose.get(id, -1))
		if A[i + 3] == v.view_z and cull.has_point(Vector2i(A[i + 1], A[i + 2])):
			var pos := Vector2(A[i + 1] * TILE + 16, A[i + 2] * TILE + 16)
			# 2026-10-30 #414 review: pose 7 is any fresh wound and animals use both poses;
			# neither identifies a weapon, so only a generic contact/wound accent is truthful.
			if pose == 7 and previous != 7:
				_spawn("environmental_hit", pos, Color.WHITE, Vector2.ONE)
			elif pose == 6 and previous != 6:
				_spawn("environmental_hit", pos + Vector2(0, -5), Color(1.0, 0.9, 0.72, 1.0), Vector2(0.9, 0.9))
			var code: int = A[i + 20]
			if pose == 2 and code in WORK_CODES:
				var stamp := int(v.anim_t / float(WORK_PERIOD[code]) + float(id) * 0.07)
				if int(_last_work_stamp.get(id, -1)) != stamp:
					_last_work_stamp[id] = stamp
					var family := material_family(v.tile_at(v.view_z, A[i + 1], A[i + 2]))
					_spawn("environmental_hit", pos, material_tint("water") if code == 7 else material_tint(family), Vector2.ONE)
		_last_pose[id] = pose
	_prune_history(seen)

func _prune_history(seen: Dictionary) -> void:
	# 2026-10-30 #414 review: departed ids cannot accumulate or suppress a later world's id.
	for id in _last_pose.keys():
		if not seen.has(id):
			_last_pose.erase(id)
	for id in _last_work_stamp.keys():
		if not seen.has(id):
			_last_work_stamp.erase(id)

func _scan_thin_place() -> void:
	var T: PackedInt32Array = v.thin
	if T.size() < 6 or T[2] != v.view_z or T[4] <= 0:
		return
	var stamp := int(v.anim_t / 2.4)
	if stamp == _last_resonance_stamp:
		return
	_last_resonance_stamp = stamp
	_spawn("resonance", Vector2(T[0] * TILE + 16, T[1] * TILE + 16), Color(0.72, 0.9, 1.0, 0.8), Vector2.ONE)
