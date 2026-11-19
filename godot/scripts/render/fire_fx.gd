extends Node2D
## Embers and smoke over burning tiles: a pool of emitter pairs moved onto the fires nearest
## the view centre each refresh. In world space, so the grade dims smoke at night while the
## embers (additive, warm) stay bright.

const TILE := 32
const MAX := 24
var v
var pairs := []   # [embers, smoke]

func _soft(w: int, c: Color) -> Texture2D:
	var img := Image.create(w, w, false, Image.FORMAT_RGBA8)
	for y in w:
		for x in w:
			var dx := (x + 0.5) / w - 0.5
			var dy := (y + 0.5) / w - 0.5
			var a := clampf(1.0 - 4.0 * (dx * dx + dy * dy), 0.0, 1.0)
			img.set_pixel(x, y, Color(c.r, c.g, c.b, c.a * a * a))
	return ImageTexture.create_from_image(img)

func init(view) -> void:
	v = view
	var ember_tex: Texture2D = v._tex("effects/ember.png")
	var smoke_tex: Texture2D = v._tex("effects/smoke.png")
	if ember_tex == null:
		ember_tex = _soft(3, Color(1.0, 0.75, 0.35, 1.0))
	if smoke_tex == null:
		smoke_tex = _soft(20, Color(0.32, 0.31, 0.32, 0.85))
	var add := CanvasItemMaterial.new()
	add.blend_mode = CanvasItemMaterial.BLEND_MODE_ADD
	for k in MAX:
		var e := CPUParticles2D.new()
		e.amount = 18
		e.lifetime = 1.1
		e.preprocess = 1.1
		e.texture = ember_tex
		e.material = add
		e.local_coords = false
		e.emission_shape = CPUParticles2D.EMISSION_SHAPE_RECTANGLE
		e.emission_rect_extents = Vector2(10, 4)
		e.direction = Vector2(0, -1)
		e.spread = 25.0
		e.gravity = Vector2(0, -20)
		e.initial_velocity_min = 25.0
		e.initial_velocity_max = 60.0
		e.tangential_accel_min = -30.0
		e.tangential_accel_max = 30.0
		var er := Gradient.new()
		er.set_color(0, Color(1, 0.9, 0.5, 1))
		er.set_color(1, Color(1, 0.3, 0.1, 0))
		e.color_ramp = er
		e.emitting = false
		var s := CPUParticles2D.new()
		s.amount = 14
		s.lifetime = 4.5
		s.preprocess = 4.5
		s.texture = smoke_tex
		s.local_coords = false
		s.emission_shape = CPUParticles2D.EMISSION_SHAPE_RECTANGLE
		s.emission_rect_extents = Vector2(8, 3)
		s.direction = Vector2(0.25, -1)
		s.spread = 15.0
		s.gravity = Vector2(6, -6)
		s.initial_velocity_min = 18.0
		s.initial_velocity_max = 32.0
		s.scale_amount_min = 0.8
		s.scale_amount_max = 1.4
		var grow := Curve.new()
		grow.add_point(Vector2(0, 0.5))
		grow.add_point(Vector2(1, 2.2))
		s.scale_amount_curve = grow
		var sr := Gradient.new()
		sr.set_color(0, Color(1, 1, 1, 0.0))
		sr.add_point(0.15, Color(1, 1, 1, 0.85))
		sr.set_color(1, Color(1, 1, 1, 0.0))
		s.color_ramp = sr
		s.emitting = false
		add_child(s)
		add_child(e)
		pairs.append([e, s])

func update() -> void:
	var vp: Viewport = v.get_viewport()
	var inv: Transform2D = vp.get_canvas_transform().affine_inverse()
	var c: Vector2 = inv * (vp.get_visible_rect().size * 0.5)
	var spots := []
	for f in v.fires:
		spots.append(Vector2(f.x * TILE + 16, f.y * TILE + 22))
	spots.sort_custom(func(a, b): return a.distance_squared_to(c) < b.distance_squared_to(c))
	for k in MAX:
		var on: bool = k < spots.size()
		var e: CPUParticles2D = pairs[k][0]
		var s: CPUParticles2D = pairs[k][1]
		if on:
			if not e.emitting or e.position.distance_to(spots[k]) > 1.0:
				e.position = spots[k]
				s.position = spots[k] + Vector2(0, -8)
		e.emitting = on
		s.emitting = on
