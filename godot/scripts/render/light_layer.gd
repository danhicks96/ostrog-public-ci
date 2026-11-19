extends CanvasLayer
## The light and colour grade: a full-screen pass over the world (layer 1, under the UI on
## layer 2). It reads a sky mask (where the sky is overhead at the viewed level) and the point
## lights in view; grade.gdshader does the falloff, the walls that stop it (#180), and the day,
## dusk, night and underground looks.

const Rules := preload("res://scripts/rules.gd")
const TILE := 32
const MAX_LIGHTS := 64
## The glowing fungus of the caverns: a small cool light each (negative intensity).
const FUNGUS_RADIUS := 2.6
const FUNGUS_LIGHTS := 40
# Art bible: hearth 6 tiles, lamp 4.5. (The immediate renderer had these swapped.)
const RADIUS := {6: 6.0, 15: 4.5}
const FIRE_RADIUS := 4.0
## Minimum ambient grade for an underground tile with no point light. This keeps the existing
## sky-mask occlusion and material hue readable without turning an unlit cavern into daylight.
## Guards the black-void regression; FIXLOG 2026-10-30 fix-506-underground-context (#506).
const UNDERGROUND_AMBIENT := 0.18

var v
var rect: ColorRect
var mat: ShaderMaterial
var sky_tex: ImageTexture
var sky_z := -99999
var sky_age := 999.0
var flash := 0.0

func init(view) -> void:
	v = view
	layer = 1
	rect = ColorRect.new()
	rect.set_anchors_preset(Control.PRESET_FULL_RECT)
	rect.mouse_filter = Control.MOUSE_FILTER_IGNORE
	mat = ShaderMaterial.new()
	mat.shader = load("res://shaders/grade.gdshader")
	rect.material = mat
	add_child(rect)

func _build_sky() -> void:
	var w: int = v.w
	var h: int = v.h
	var z: int = v.view_z
	# Two bytes a tile: the sky overhead, and whether the tile stops light (#180).
	var data := PackedByteArray()
	data.resize(w * h * 2)
	var g: PackedInt32Array = v.ground
	for y in h:
		for x in w:
			var t: int = v.tile_at(z, x, y)
			var under: bool = g[y * w + x] >= z and v.shape_of(t) != 2
			data[(y * w + x) * 2] = 0 if under else 255
			data[(y * w + x) * 2 + 1] = 255 if Rules.blocks_light(t) else 0
	var img := Image.create_from_data(w, h, false, Image.FORMAT_RG8, data)
	if sky_tex == null:
		sky_tex = ImageTexture.create_from_image(img)
	else:
		sky_tex.update(img)
	mat.set_shader_parameter("sky_mask", sky_tex)
	mat.set_shader_parameter("map_px", Vector2(w * TILE, h * TILE))
	sky_z = z
	sky_age = 0.0

func update(d: float) -> void:
	var presentation_delta := 0.0 if v.animation_frozen else d
	sky_age += presentation_delta
	if v.view_z != sky_z or sky_age > 4.0:
		_build_sky()
	var vp: Viewport = v.get_viewport()
	var inv: Transform2D = vp.get_canvas_transform().affine_inverse()
	var size: Vector2 = vp.get_visible_rect().size
	var tl: Vector2 = inv * Vector2.ZERO
	var br: Vector2 = inv * size
	mat.set_shader_parameter("world_tl", tl)
	mat.set_shader_parameter("world_br", br)
	mat.set_shader_parameter("sun", v.sun_level())
	mat.set_shader_parameter("under_level", UNDERGROUND_AMBIENT)
	var oc := 0.0
	match v.sky:
		1: oc = 0.5
		2: oc = 0.8
		3: oc = 0.6
	mat.set_shader_parameter("overcast", oc)
	flash = maxf(0.0, flash - presentation_delta * 4.0)
	mat.set_shader_parameter("flash", flash)
	mat.set_shader_parameter("presentation_time", v.anim_t)

## Point lights near the view (hearths, lamps, fires), rebuilt when the world refreshes rather
## than every frame. Nearest the view centre first when there are more than the shader takes.
func rebuild_lights() -> void:
	var c: Rect2i = v.cull_rect()
	var tl := Vector2(c.position * TILE)
	var br := Vector2((c.position + c.size) * TILE)
	var lights := []
	var margin := 7 * TILE
	var S: PackedInt32Array = v.structures
	for i in range(0, S.size(), 8):
		if S[i + 3] != v.view_z or S[i + 6] != 1 or not RADIUS.has(S[i + 4]):
			continue
		var p := Vector2(S[i + 1] * TILE + 16, S[i + 2] * TILE + 16)
		if p.x < tl.x - margin or p.y < tl.y - margin or p.x > br.x + margin or p.y > br.y + margin:
			continue
		lights.append(Vector4(p.x, p.y, RADIUS[S[i + 4]] * TILE, 1.0))
	var B: PackedInt32Array = v.bands
	for i in range(0, B.size(), 9):
		if B[i + 4] != 1 or B[i + 3] != v.view_z:
			continue
		# The camp's fire (entity_layer _camp draws it one tile east and south of the camp).
		var cp := Vector2((B[i + 1] + 1) * TILE + 16, (B[i + 2] + 1) * TILE + 16)
		if cp.x < tl.x - margin or cp.y < tl.y - margin or cp.x > br.x + margin or cp.y > br.y + margin:
			continue
		lights.append(Vector4(cp.x, cp.y, FIRE_RADIUS * 0.7 * TILE, 1.0))
	for f in v.fires:
		var fp := Vector2(f.x * TILE + 16, f.y * TILE + 16)
		if fp.x < tl.x - margin or fp.y < tl.y - margin or fp.x > br.x + margin or fp.y > br.y + margin:
			continue
		lights.append(Vector4(fp.x, fp.y, FIRE_RADIUS * TILE, 1.1))
	# Fires and lamps first; the fungus fills what room is left, nearest the view first.
	var fungus_kind: int = v.PLANT_KIND.find("fungus_tree")
	var glows := []
	var P: PackedInt32Array = v.plants
	for i in range(0, P.size(), 6):
		if P[i + 3] != fungus_kind or P[i + 2] != v.view_z:
			continue
		var gp := Vector2(P[i] * TILE + 16, P[i + 1] * TILE + 4)
		if gp.x < tl.x - margin or gp.y < tl.y - margin or gp.x > br.x + margin or gp.y > br.y + margin:
			continue
		glows.append(Vector4(gp.x, gp.y, FUNGUS_RADIUS * TILE * (0.6 + 0.4 * P[i + 4] / 100.0), -0.75))
	var mid := (tl + br) * 0.5
	if glows.size() > FUNGUS_LIGHTS:
		glows.sort_custom(func(a, b): return Vector2(a.x, a.y).distance_squared_to(mid) < Vector2(b.x, b.y).distance_squared_to(mid))
		glows.resize(FUNGUS_LIGHTS)
	lights.append_array(glows)
	if lights.size() > MAX_LIGHTS:
		lights.sort_custom(func(a, b): return Vector2(a.x, a.y).distance_squared_to(mid) < Vector2(b.x, b.y).distance_squared_to(mid))
		lights.resize(MAX_LIGHTS)
	var n := lights.size()
	while lights.size() < MAX_LIGHTS:
		lights.append(Vector4(0, 0, 1, 0))
	mat.set_shader_parameter("lights", lights)
	mat.set_shader_parameter("light_count", n)
