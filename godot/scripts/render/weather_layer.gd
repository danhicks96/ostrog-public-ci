extends Node2D
## Weather in world space, so the grade dims it at night: rain with splashes, snow, and leaves
## blowing in autumn. Emitters follow the camera and only run where the sky is overhead.

var v
var rain: CPUParticles2D
var splash: CPUParticles2D
var snow: CPUParticles2D
var leaves: CPUParticles2D

func _tex(w: int, h: int, c: Color, soft := false) -> Texture2D:
	var img := Image.create(w, h, false, Image.FORMAT_RGBA8)
	for y in h:
		for x in w:
			var a := c.a
			if soft:
				var dx := (x + 0.5) / w - 0.5
				var dy := (y + 0.5) / h - 0.5
				a *= clampf(1.0 - 3.2 * (dx * dx + dy * dy), 0.0, 1.0)
			img.set_pixel(x, y, Color(c.r, c.g, c.b, a))
	return ImageTexture.create_from_image(img)

func _emitter(amount: int, life: float, tex: Texture2D) -> CPUParticles2D:
	var p := CPUParticles2D.new()
	p.amount = amount
	p.lifetime = life
	p.preprocess = life
	p.texture = tex
	p.local_coords = false
	p.emission_shape = CPUParticles2D.EMISSION_SHAPE_RECTANGLE
	p.emitting = false
	add_child(p)
	return p

func init(view) -> void:
	v = view
	rain = _emitter(700, 0.42, _tex(1, 7, Color(0.80, 0.86, 0.98, 0.55)))
	rain.direction = Vector2(-0.2, 1.0)
	rain.spread = 2.0
	rain.gravity = Vector2.ZERO
	rain.initial_velocity_min = 640.0
	rain.initial_velocity_max = 780.0
	splash = _emitter(160, 0.22, _tex(3, 1, Color(0.85, 0.9, 1.0, 0.6)))
	splash.gravity = Vector2.ZERO
	splash.initial_velocity_min = 0.0
	splash.initial_velocity_max = 0.0
	var ramp := Gradient.new()
	ramp.set_color(0, Color(1, 1, 1, 0.8))
	ramp.set_color(1, Color(1, 1, 1, 0.0))
	splash.color_ramp = ramp
	splash.scale_amount_min = 1.0
	splash.scale_amount_max = 2.0
	snow = _emitter(380, 7.0, _tex(3, 3, Color(1, 1, 1, 0.95), true))
	snow.direction = Vector2(0.25, 1.0)
	snow.spread = 25.0
	snow.gravity = Vector2(0, 3)
	snow.initial_velocity_min = 22.0
	snow.initial_velocity_max = 42.0
	snow.tangential_accel_min = -8.0
	snow.tangential_accel_max = 8.0
	snow.scale_amount_min = 0.7
	snow.scale_amount_max = 1.4
	leaves = _emitter(36, 9.0, _tex(3, 2, Color(1, 1, 1, 1)))
	leaves.direction = Vector2(1.0, 0.45)
	leaves.spread = 20.0
	leaves.gravity = Vector2(0, 5)
	leaves.initial_velocity_min = 25.0
	leaves.initial_velocity_max = 55.0
	leaves.angular_velocity_min = -120.0
	leaves.angular_velocity_max = 120.0
	var lc := Gradient.new()
	lc.set_color(0, Color(0.78, 0.36, 0.14))
	lc.set_color(1, Color(0.9, 0.7, 0.22))
	lc.add_point(0.5, Color(0.62, 0.24, 0.12))
	leaves.color_initial_ramp = lc

func update() -> void:
	var vp: Viewport = v.get_viewport()
	var inv: Transform2D = vp.get_canvas_transform().affine_inverse()
	var size: Vector2 = vp.get_visible_rect().size
	var tl: Vector2 = inv * Vector2.ZERO
	var br: Vector2 = inv * size
	var c := (tl + br) * 0.5
	var ext := (br - tl) * 0.5 + Vector2(40, 40)
	for p in [rain, splash, snow, leaves]:
		p.position = c
		p.emission_rect_extents = ext
	var under_sky: bool = v.under_sky_at_centre()
	rain.emitting = under_sky and (v.sky == 2 or v.sky == 5)
	splash.emitting = rain.emitting
	snow.emitting = under_sky and v.sky == 3
	leaves.emitting = under_sky and v.season == 2 and v.sky != 2
