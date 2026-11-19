extends SceneTree
## Real-client frame-time gate for WP-06 slice 6.1 (D53).
##
## Run from the godot directory on a real windowed GPU client:
##   ../tools/godot --path . --script ../tools/perf/frame_time.gd -- SAVE CSV FRAMES WARMUP
##
## SAVE is a client-readable .hfsave directory containing the mixed-kind 500-person profile.
## CSV is written exactly as supplied (absolute paths are recommended).  Stage 1 has one sim
## worker and no renderer-preparation pool yet, so worker_busy_ms is deliberately zero and the
## verdict says so; later renderer stages must replace that column with their per-worker timers.

const FRAME_BUDGET_MS := 16.7
const FRAME_HARD_LIMIT_MS := 33.0

var startup_frames := 0
var measuring := false
var finished := false
var warmup := 30
var target_frames := 600
var measured := 0
var total_frames := 0
var refresh_accum := 0.0
var save_path := ""
var csv_path := ""
var sim
var view
var output: FileAccess
var wall_samples: Array[float] = []
var process_samples: Array[float] = []
var sim_samples: Array[float] = []
var render_wait_samples: Array[float] = []
var worker_samples: Array[float] = []

func _process(_delta: float) -> bool:
	if finished:
		return false
	if startup_frames == 0:
		startup_frames = 1
		_start()
		return false
	if not measuring:
		_measure_one_frame(_delta)
	return false

func _start() -> void:
	var args := OS.get_cmdline_user_args()
	if args.size() < 2:
		print("FRAME_TIME usage: SAVE CSV [FRAMES] [WARMUP]")
		quit(2)
		return
	save_path = args[0]
	csv_path = args[1]
	if args.size() >= 3:
		target_frames = maxi(1, int(args[2]))
	if args.size() >= 4:
		warmup = maxi(0, int(args[3]))

	# The gate is intentionally a real-client measurement: no headless renderer and no vsync
	# pacing.  Keep the requested window stable so results from different machines compare.
	DisplayServer.window_set_mode(DisplayServer.WINDOW_MODE_WINDOWED)
	DisplayServer.window_set_size(Vector2i(1920, 1080))
	DisplayServer.window_set_vsync_mode(DisplayServer.VSYNC_DISABLED)

	output = FileAccess.open(csv_path, FileAccess.WRITE)
	if output == null:
		print("FRAME_TIME cannot open CSV: %s" % csv_path)
		quit(1)
		return
	output.store_line("frame,wall_ms,process_ms,render_wait_ms,sim_thread_ms,worker_busy_ms,worker_count,within_16_7,over_33")

	sim = SimHost.new()
	get_root().add_child(sim)
	var err: String = sim.load_from(save_path)
	if err != "":
		print("FRAME_TIME load failed: %s" % err)
		output.close()
		quit(1)
		return
	sim.sync()

	var assets = load("res://scripts/assets.gd").new()
	view = load("res://scripts/world_view.gd").new()
	get_root().add_child(view)
	view.setup(sim, assets)
	var centre: Vector3i = sim.settlement_centre()
	view.view_z = centre.z
	view.show_rect = Rect2i(centre.x - 30, centre.y - 17, 60, 34)
	view.refresh()
	var camera := Camera2D.new()
	get_root().add_child(camera)
	camera.position = Vector2(centre.x, centre.y) * 32.0 + Vector2(16, 16)
	camera.make_current()

	print("FRAME_TIME start save=%s frames=%d warmup=%d window=1920x1080 vsync=off renderer=%s" % [
		save_path, target_frames, warmup, ProjectSettings.get_setting("rendering/renderer/rendering_method", "unknown")])
	# The next process callback owns the first frame; setting this true here would leave the
	# harness rendering forever without ever entering _measure_one_frame.
	measuring = false

func _measure_one_frame(delta: float) -> void:
	measuring = true
	var frame_start := Time.get_ticks_usec()
	# Ask the sim for one client frame's worth of work, then publish the most recent snapshot to
	# the existing real view.  last_step_ms is the Rust worker timer, not a guessed wall slice.
	sim.step(1)
	sim.drain_feed()
	# Match the shipped client: bridge refreshes happen every 50 ms, while the scene's existing
	# render nodes still draw every frame.  Refreshing every frame would measure an artificial
	# worst case rather than the client loop in godot/scripts/main.gd.
	refresh_accum += delta
	if refresh_accum > 0.05:
		refresh_accum = 0.0
		view.refresh()
	var sim_ms: float = sim.last_step_ms()
	await RenderingServer.frame_post_draw
	var wall_ms := (Time.get_ticks_usec() - frame_start) / 1000.0
	var process_s := float(Performance.get_monitor(Performance.TIME_PROCESS))
	var process_ms := process_s * 1000.0
	var render_wait_ms := maxf(0.0, wall_ms - process_ms)
	var worker_ms := 0.0  # Stage 1 has no render-preparation worker pool yet.
	total_frames += 1
	if total_frames <= warmup:
		measuring = false
		return
	measured += 1
	wall_samples.append(wall_ms)
	process_samples.append(process_ms)
	sim_samples.append(sim_ms)
	render_wait_samples.append(render_wait_ms)
	worker_samples.append(worker_ms)
	output.store_line(",".join(PackedStringArray([
		str(measured), _f(wall_ms), _f(process_ms), _f(render_wait_ms), _f(sim_ms), _f(worker_ms),
		"0", "1" if wall_ms <= FRAME_BUDGET_MS else "0", "1" if wall_ms > FRAME_HARD_LIMIT_MS else "0"])))
	measuring = false
	if measured >= target_frames:
		_finish()

func _finish() -> void:
	if finished:
		return
	finished = true
	measuring = false
	output.close()
	var p50 := _percentile(wall_samples, 0.50)
	var p95 := _percentile(wall_samples, 0.95)
	var p99 := _percentile(wall_samples, 0.99)
	var max_ms := _percentile(wall_samples, 1.0)
	var over16 := wall_samples.filter(func(v: float) -> bool: return v > FRAME_BUDGET_MS).size()
	var over33 := wall_samples.filter(func(v: float) -> bool: return v > FRAME_HARD_LIMIT_MS).size()
	var pass_gate := p95 <= FRAME_BUDGET_MS and over33 == 0
	print("FRAME_TIME %s p50 %.2f p95 %.2f p99 %.2f max %.2f ms; within_16_7 %d/%d; over_33 %d; sim_thread_p95 %.2f ms; worker_count 0 (stage 1)" % [
		"PASS" if pass_gate else "FAIL", p50, p95, p99, max_ms, wall_samples.size() - over16,
		wall_samples.size(), over33, _percentile(sim_samples, 0.95)])
	print("FRAME_TIME CSV %s" % csv_path)
	quit(0 if pass_gate else 1)

func _percentile(values: Array[float], fraction: float) -> float:
	if values.is_empty():
		return 0.0
	var sorted: Array[float] = values.duplicate()
	sorted.sort()
	return sorted[mini(sorted.size() - 1, maxi(0, int(ceil(float(sorted.size()) * fraction)) - 1))]

func _f(value: float) -> String:
	return "%.3f" % value
