extends Node2D
## Sound: buses, ambience, positional effects. Everything here reads what the world view
## already shows; it plays nothing the simulation did not do (work sounds follow people's
## actual work, fires and water are where they are, animals call only when in view), except
## distant thunder in rain, which is weather texture and never claims a strike.
##
## Buses (built at start): Master <- Music, Ambience (low-pass when the view is underground),
## SFX (cave reverb underground), UI. The settings sliders set bus volumes.

const Rules := preload("res://scripts/rules.gd")
const TILE := 32
const VOICES := 16
# Work-sound codes from SimHost.agents() (field 20): 6 is cooking, 7 is carrying water to a fire.
const WORK := {1: "chop", 2: "mine", 3: "hammer", 4: "hoe", 5: "saw", 6: "cook", 7: "douse"}
const WORK_PERIOD := {1: 0.95, 2: 0.85, 3: 0.7, 4: 1.1, 5: 0.6, 6: 1.3, 7: 1.9}
const CALLS := {1: "deer", 2: "boar", 3: "wolf", 4: "hare", 5: "fox", 6: "crow", 7: "bear", 8: "duck"}
const CALL_RATE := {1: 0.004, 2: 0.006, 3: 0.003, 4: 0.002, 5: 0.004, 6: 0.012, 7: 0.003, 8: 0.01}

var v      # world_view
var music = null  # music_director.gd, which hears the music moments of the sounds rows (#290)
var settings: Dictionary
var manifest := {}
var streams := {}
var ambience := {}     # name -> AudioStreamPlayer
var fire_voices := []  # AudioStreamPlayer2D looping fire
var water_voice: AudioStreamPlayer2D
var pool := []
var pool_i := 0
var next_hit := {}     # agent id -> seconds of anim_t when it next strikes
var last_kind_t := {}  # sound name -> last play time (rate limit)
var acc := 0.0
var t := 0.0
var rng := RandomNumberGenerator.new()
var silent := false
var log_path := ""
var log_lines := []
var open_doors := {}   # Vector2i -> true: doors someone stood in at the last look
var doors_z := -9999   # the level open_doors was taken on

func init(view, s: Dictionary, quiet: bool) -> void:
	v = view
	settings = s
	silent = quiet
	rng.seed = 1
	_buses()
	var f := FileAccess.open("res://assets/audio/sounds.json", FileAccess.READ)
	if f:
		manifest = JSON.parse_string(f.get_as_text())
	for n in ["day_loop", "night_loop", "wind_loop", "rain_light", "rain_mid", "rain_heavy", "cave_loop"]:
		var p := AudioStreamPlayer.new()
		p.stream = _stream(n, 0)
		p.bus = "Ambience"
		p.volume_db = -80.0
		add_child(p)
		ambience[n] = p
	for k in 3:
		var fp := AudioStreamPlayer2D.new()
		fp.stream = _stream("fire_loop", 0)
		fp.bus = "SFX"
		fp.max_distance = 14 * TILE
		fp.attenuation = 1.6
		fp.volume_db = -80.0
		add_child(fp)
		fire_voices.append(fp)
	water_voice = AudioStreamPlayer2D.new()
	water_voice.stream = _stream("water_loop", 0)
	water_voice.bus = "Ambience"
	water_voice.max_distance = 16 * TILE
	water_voice.attenuation = 1.4
	add_child(water_voice)
	for k in VOICES:
		var p2 := AudioStreamPlayer2D.new()
		p2.bus = "SFX"
		p2.max_distance = 22 * TILE
		p2.attenuation = 1.3
		add_child(p2)
		pool.append(p2)
	if not silent:
		for p in ambience.values():
			if p.stream: p.play()
		for p in fire_voices:
			if p.stream: p.play()
		if water_voice.stream: water_voice.play()
	apply_volumes()
	# Fires lit and put out before the first frame (the --ticks run-up) are not news.
	v.sim.drain_sounds()

func _buses() -> void:
	for spec in [["Music", false], ["Ambience", true], ["SFX", true], ["UI", false]]:
		var name: String = spec[0]
		if AudioServer.get_bus_index(name) >= 0:
			continue
		AudioServer.add_bus()
		var i := AudioServer.bus_count - 1
		AudioServer.set_bus_name(i, name)
		AudioServer.set_bus_send(i, "Master")
		if name == "Ambience":
			var lp := AudioEffectLowPassFilter.new()
			lp.cutoff_hz = 900.0
			AudioServer.add_bus_effect(i, lp)
			AudioServer.set_bus_effect_enabled(i, 0, false)
		if name == "SFX":
			var rv := AudioEffectReverb.new()
			rv.room_size = 0.75
			rv.damping = 0.4
			rv.wet = 0.35
			rv.dry = 0.85
			AudioServer.add_bus_effect(i, rv)
			AudioServer.set_bus_effect_enabled(i, 0, false)

func apply_volumes() -> void:
	var lin := func(x: float) -> float: return linear_to_db(maxf(0.0001, x))
	AudioServer.set_bus_volume_db(0, lin.call(settings.get("master", 0.8)))
	AudioServer.set_bus_volume_db(AudioServer.get_bus_index("Ambience"), lin.call(settings.get("ambient", 0.7)))
	AudioServer.set_bus_volume_db(AudioServer.get_bus_index("SFX"), lin.call(settings.get("fx", 0.8)))
	AudioServer.set_bus_volume_db(AudioServer.get_bus_index("UI"), lin.call(settings.get("fx", 0.8)))
	AudioServer.set_bus_volume_db(AudioServer.get_bus_index("Music"), lin.call(settings.get("music", 0.6)))

func _stream(name: String, idx: int) -> AudioStream:
	var key := "%s|%d" % [name, idx]
	if streams.has(key):
		return streams[key]
	var m: Dictionary = manifest.get(name, {})
	var files: Array = m.get("files", [])
	var st: AudioStream = null
	if not files.is_empty():
		var path: String = "res://assets/audio/" + str(files[idx % files.size()])
		if ResourceLoader.exists(path):
			st = load(path)
		elif FileAccess.file_exists(path):
			st = AudioStreamOggVorbis.load_from_file(path)
		if st is AudioStreamOggVorbis:
			st.loop = m.get("loop", false)
	streams[key] = st
	return st

func variants(name: String) -> int:
	return maxi(1, (manifest.get(name, {}).get("files", []) as Array).size())

## A one-shot at a world position, with pitch and level jitter and a per-sound rate limit.
func play_at(name: String, pos: Vector2, db := 0.0, min_gap := 0.08) -> void:
	if silent:
		return
	if t - float(last_kind_t.get(name, -9.0)) < min_gap:
		return
	last_kind_t[name] = t
	var st := _stream(name, rng.randi() % variants(name))
	if st == null:
		return
	var p: AudioStreamPlayer2D = pool[pool_i]
	pool_i = (pool_i + 1) % VOICES
	p.stream = st
	p.position = pos
	p.pitch_scale = rng.randf_range(0.93, 1.07)
	p.volume_db = db + rng.randf_range(-2.0, 1.0)
	p.play()
	if log_path != "":
		log_lines.append("%.2f %s %d %d" % [t, name, int(pos.x), int(pos.y)])

func ui(name: String) -> void:
	if silent:
		return
	var st := _stream(name, 0)
	if st == null:
		return
	var p := AudioStreamPlayer.new()
	p.stream = st
	p.bus = "UI"
	add_child(p)
	p.play()
	p.finished.connect(p.queue_free)
	if log_path != "":
		log_lines.append("%.2f %s ui ui" % [t, name])

## Run the event pass now (a shot, #221), whatever the accumulator says.
func pump() -> void:
	_events()

func _process(d: float) -> void:
	t += d
	acc += d
	if acc < 0.1:
		return
	acc = 0.0
	_ambience()
	_positional_loops()
	_work_and_calls()
	_events()
	_doors()
	if log_path != "" and int(t * 10) % 50 == 0:
		save_log()

func _fade(p: AudioStreamPlayer, target_lin: float) -> void:
	var cur := db_to_linear(p.volume_db)
	var nxt := lerpf(cur, target_lin, 0.15)
	p.volume_db = linear_to_db(maxf(0.00001, nxt))

func _ambience() -> void:
	var under: bool = not v.under_sky_at_centre()
	AudioServer.set_bus_effect_enabled(AudioServer.get_bus_index("Ambience"), 0, under)
	AudioServer.set_bus_effect_enabled(AudioServer.get_bus_index("SFX"), 0, under)
	var sun: float = v.sun_level()
	var out := 0.0 if under else 1.0
	var winter: bool = v.season == 3
	var sky: int = v.sky
	_fade(ambience["day_loop"], out * sun * (0.2 if winter else 1.0) * (0.4 if (sky == 2 or sky == 5) else 1.0))
	_fade(ambience["night_loop"], out * (1.0 - sun) * (0.0 if winter or v.temperature < 4.0 else 0.9))
	var windy := 0.35 + (0.35 if sky != 0 else 0.0) + (0.3 if winter else 0.0)
	_fade(ambience["wind_loop"], out * windy * 0.8 + (0.15 if under else 0.0))
	_fade(ambience["rain_light"], out * (1.0 if (sky == 2 or sky == 5) else 0.0) * 0.6)
	_fade(ambience["rain_mid"], out * (1.0 if (sky == 2 or sky == 5) else 0.0) * 0.8)
	_fade(ambience["rain_heavy"], 0.0)
	_fade(ambience["cave_loop"], 1.0 if under else 0.0)
	# Gusts in wind, distant thunder now and then in rain (weather texture only: no flash).
	var c: Vector2 = _centre()
	if not under and sky != 0 and rng.randf() < 0.012:
		play_at("gust", c + Vector2(rng.randf_range(-300, 300), rng.randf_range(-200, 200)), -6.0, 3.0)
	if not under and (sky == 2 or sky == 5) and rng.randf() < 0.004:
		play_at("thunder", c + Vector2(rng.randf_range(-500, 500), -400), -4.0, 12.0)

func _centre() -> Vector2:
	var vp: Viewport = v.get_viewport()
	var inv: Transform2D = vp.get_canvas_transform().affine_inverse()
	return inv * (vp.get_visible_rect().size * 0.5)

func _positional_loops() -> void:
	var c := _centre()
	var spots := []
	for f in v.fires:
		spots.append(Vector2(f.x * TILE + 16, f.y * TILE + 16))
	var S: PackedInt32Array = v.structures
	for i in range(0, S.size(), 8):
		if S[i + 3] == v.view_z and S[i + 4] == 6 and S[i + 6] == 1:
			spots.append(Vector2(S[i + 1] * TILE + 16, S[i + 2] * TILE + 16))
	spots.sort_custom(func(a, b): return a.distance_squared_to(c) < b.distance_squared_to(c))
	for k in fire_voices.size():
		var p: AudioStreamPlayer2D = fire_voices[k]
		if k < spots.size():
			p.position = spots[k]
			p.volume_db = lerpf(p.volume_db, 0.0, 0.3)
		else:
			p.volume_db = lerpf(p.volume_db, -80.0, 0.3)
	# Water: the nearest animated water tile in view.
	var best := Vector2.ZERO
	var bd := 1e18
	for k in v.chunk_nodes:
		var g = v.chunk_nodes[k][0]
		if not g.visible:
			continue
		for dd in g.dyn:
			if dd[2] == "water":
				var p2 := Vector2(dd[0] * TILE + 16, dd[1] * TILE + 16)
				var d2 := p2.distance_squared_to(c)
				if d2 < bd:
					bd = d2
					best = p2
	water_voice.position = best
	water_voice.volume_db = lerpf(water_voice.volume_db, -2.0 if bd < 1e17 else -80.0, 0.3)

func _work_and_calls() -> void:
	var A: PackedInt32Array = v.agents
	var st: int = v.AGENT_STRIDE
	var cull: Rect2i = v.cull_rect()
	var now: float = t
	for i in range(0, A.size(), st):
		if A[i + 3] != v.view_z or not cull.has_point(Vector2i(A[i + 1], A[i + 2])):
			continue
		var pos := Vector2(A[i + 1] * TILE + 16, A[i + 2] * TILE + 24)
		var species: int = A[i + 4]
		if species == 0:
			var snd: int = A[i + 20]
			if not WORK.has(snd) or A[i + 15] != 2:
				next_hit.erase(A[i])
				continue
			var id: int = A[i]
			if not next_hit.has(id):
				next_hit[id] = now + rng.randf_range(0.0, WORK_PERIOD[snd])
			elif now >= next_hit[id]:
				play_at(WORK[snd], pos, -3.0, 0.05)
				next_hit[id] = now + WORK_PERIOD[snd] * rng.randf_range(0.85, 1.2)
		elif CALLS.has(species) and A[i + 15] != 4 and rng.randf() < CALL_RATE[species]:
			play_at(CALLS[species], pos, -5.0, 2.5)

## Fires starting (flare) and being put out (douse), where they happened, if that is in view;
## lightning anywhere.
func _events() -> void:
	var rows: Array = v.sim.drain_sounds()
	var cull: Rect2i = v.cull_rect().grow(4)
	for r in rows:
		# A death or a welcome is the score's to answer, not an effect (#290, WP-08 8.8).
		if Rules.is_music_moment(str(r[0])):
			if music:
				music.moment(str(r[0]))
			continue
		# Effects share the same authoritative client event stream. They never infer a new
		# simulation event; they only decorate a sound the bridge already delivered.
		if v.effects:
			v.effects.on_sound_event(r)
		# A strike of lightning lights the whole sky and is heard across the valley, wherever
		# it fell; the grade shader's flash was never set (#184, QA 2026-10-20, C63).
		if r[0] == "lightning":
			v.light.flash = 1.0
			play_at("thunder", v.get_viewport().get_camera_2d().get_screen_center_position() + Vector2(0, -300), 0.0, 1.0)
			continue
		if int(r[3]) != v.view_z or not cull.has_point(Vector2i(r[1], r[2])):
			continue
		play_at(r[0], Vector2(int(r[1]) * TILE + 16, int(r[2]) * TILE + 16), -2.0, 0.3)

## A door swings open when someone steps into the doorway: the world view draws it open while
## the tile is occupied, so the sound plays when a door in view goes from shut to open.
func _doors() -> void:
	var now := {}
	for k in v.chunk_nodes:
		var g = v.chunk_nodes[k][0]
		if not g.visible:
			continue
		for dd in g.dyn:
			if dd[2] == "door" and v.occupied.has(Vector3i(dd[0], dd[1], v.view_z)):
				now[Vector2i(dd[0], dd[1])] = true
	if doors_z == v.view_z:
		for p in now:
			if not open_doors.has(p):
				play_at("door", Vector2(p.x * TILE + 16, p.y * TILE + 16), -6.0, 0.15)
	open_doors = now
	doors_z = v.view_z

func save_log() -> void:
	if log_path == "":
		return
	var f := FileAccess.open(log_path, FileAccess.WRITE)
	if f:
		f.store_string("\n".join(log_lines))
