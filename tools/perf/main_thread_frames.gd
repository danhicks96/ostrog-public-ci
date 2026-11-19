extends SceneTree
## Main-thread time per frame while the game runs (#603): a saved world is loaded headless (the
## dummy renderer: nothing is drawn, so this is the CPU work on the main thread, not the GPU),
## then FRAMES frames are paced at 60 a second; each frame asks for the ticks the speed gives
## (20 a second at 1x, as main.gd), refreshes the view one frame in three (every 50 ms, as
## main.gd) and runs the view's per-frame work, and the time the main thread spent is recorded.
## With --inline the sim steps in place on the main thread, as before #603.
## usage, from godot/:
##   ../tools/godot --path . --headless -s ../tools/perf/main_thread_frames.gd -- SAVE SPEED FRAMES [--inline]

var frame := 0

## The main thread's time on a CPU and time waiting for one, in ns (Linux schedstat), so a slow
## call can be told apart: run time is the call's own work, wait time is the machine's load.
func _sched() -> PackedInt64Array:
	var f := FileAccess.open("/proc/thread-self/schedstat", FileAccess.READ)
	if f == null:
		return PackedInt64Array([0, 0])
	var p := f.get_line().split(" ")
	if p.size() < 2:
		return PackedInt64Array([0, 0])
	return PackedInt64Array([int(p[0]), int(p[1])])
var started := false

func _process(_d: float) -> bool:
	frame += 1
	if frame == 2:
		_run()
	return false

func _run() -> void:
	var args := OS.get_cmdline_user_args()
	var speed := int(args[1]) if args.size() > 1 else 1
	var frames := int(args[2]) if args.size() > 2 else 600
	var inline := args.has("--inline")
	var sim = SimHost.new()
	get_root().add_child(sim)
	if inline:
		sim.set_threaded(false)
	var err: String = sim.load_from(args[0])
	if err != "":
		print("MAIN_FRAMES load failed: %s" % err)
		quit(1)
		return
	var fs: Vector3i = sim.settlement_centre()
	var cam := Camera2D.new()
	get_root().add_child(cam)
	cam.make_current()
	var view = load("res://scripts/world_view.gd").new()
	get_root().add_child(view)
	view.setup(sim, load("res://scripts/assets.gd").new())
	view.view_z = fs.z
	view.show_rect = Rect2i(fs.x - 30, fs.y - 17, 60, 34)
	view.refresh()
	var start_tick: int = sim.tick()
	var times: Array = []
	var step_times: Array = []
	var worst := [0.0, 0.0, 0.0]
	var acc := 0.0
	var refresh_acc := 0.0
	var dt := 1.0 / 60.0
	var t_begin := Time.get_ticks_usec()
	for i in frames:
		var f0 := Time.get_ticks_usec()
		acc += dt * 20.0 * speed
		var n := int(acc)
		acc -= n
		var q0 := _sched()
		var s0 := Time.get_ticks_usec()
		if n > 0:
			sim.step(mini(n, 400))
		var s_ms := (Time.get_ticks_usec() - s0) / 1000.0
		var q1 := _sched()
		if i > 0 and s_ms > worst[0]:
			worst = [s_ms, (q1[0] - q0[0]) / 1000000.0, (q1[1] - q0[1]) / 1000000.0]
		# The first batch of a world copies it for the view, once (sim_thread.rs): left out here.
		if i > 0:
			step_times.append((Time.get_ticks_usec() - s0) / 1000.0)
		sim.drain_feed()
		refresh_acc += dt
		if refresh_acc > 0.05:
			refresh_acc = 0.0
			view.refresh()
		view._process(dt)
		var ms := (Time.get_ticks_usec() - f0) / 1000.0
		times.append(ms)
		# Pace at 60 frames a second, as a real frame would be.
		var left := int(dt * 1000000.0) - (Time.get_ticks_usec() - f0)
		if left > 0:
			OS.delay_usec(left)
	var secs := (Time.get_ticks_usec() - t_begin) / 1000000.0
	sim.sync()
	var ran: int = sim.tick() - start_tick
	times.sort()
	step_times.sort()
	var k := times.size()
	var over16 := times.filter(func(x): return x > 16.7).size()
	var over33 := times.filter(func(x): return x > 33.0).size()
	print("MAIN_FRAMES slowest step call after the first: %.2f ms wall, %.2f ms on a CPU, %.2f ms waiting for one" % worst)
	print("MAIN_FRAMES %s speed %dx frames %d (%.1f s): main thread per frame p50 %.2f p95 %.2f p99 %.2f max %.2f ms; over 16.7 ms %d, over 33 ms %d; step call (after the first) p95 %.2f max %.2f ms; ticks run %d of %d asked" % [
		"inline" if inline else "threaded", speed, k, secs,
		times[k / 2], times[int(k * 0.95)], times[int(k * 0.99)], times[k - 1],
		over16, over33, step_times[int(step_times.size() * 0.95)], step_times[step_times.size() - 1], ran, int(frames * dt * 20.0 * speed)])
	quit(0)
