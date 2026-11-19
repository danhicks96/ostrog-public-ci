extends SceneTree
## What preparing a frame costs on the CPU today (#414 B2, docs/design-renderer.md): a saved
## world is loaded headless (the dummy renderer: nothing is drawn, so this is the preparation,
## not the GPU), then for FRAMES frames it steps the sim one tick, times each read the view's
## refresh makes through the bridge, and times the client's own preparation (WorldView refresh,
## the entity rebuild and its per-frame _process). Prints p50, p95 and max per part, in ms.
## usage, from godot/:
##   ../tools/godot --path . --headless -s ../tools/perf/frame_prep.gd -- SAVE [FRAMES] [--whole-map] [--rust-renderer]

var out := {}

func _t(name: String, f: Callable) -> Variant:
	var t := Time.get_ticks_usec()
	var r = f.call()
	var ms := (Time.get_ticks_usec() - t) / 1000.0
	if not out.has(name):
		out[name] = []
	out[name].append(ms)
	return r

var step := 0

func _process(_d: float) -> bool:
	step += 1
	if step == 2:
		_measure()
	return false

func _measure() -> void:
	var args := OS.get_cmdline_user_args()
	var frames := int(args[1]) if args.size() > 1 else 120
	var sim = SimHost.new()
	get_root().add_child(sim)
	var err: String = sim.load_from(args[0])
	if err != "":
		print("FRAME_PREP load failed: %s" % err)
		quit(1)
		return
	var fs: Vector3i = sim.settlement_centre()
	var zr: Vector2i = sim.z_range()
	var people := 0
	var a: PackedInt32Array = sim.agents()
	print("FRAME_PREP save %s tick %d agent rows %d ints %d centre %s z %s" % [args[0], sim.tick(), a.size() / 24, a.size(), fs, zr])
	var assets = load("res://scripts/assets.gd").new()
	var cam := Camera2D.new()
	get_root().add_child(cam)
	cam.make_current()
	# --rust-renderer: the Rust world renderer on (WP-06 6.4), as the 6.4 gate measures it. The
	# view reads the setting in setup(), so it is set before.
	if args.has("--rust-renderer"):
		ProjectSettings.set_setting("ostrog/render/world_renderer", true)
	var view = load("res://scripts/world_view.gd").new()
	get_root().add_child(view)
	view.setup(sim, assets)
	print("FRAME_PREP rust renderer %s" % ("on" if view.rust_renderer_enabled else "off"))
	view.view_z = fs.z
	# A 1920x1080 window at zoom 1 shows 60 x 34 tiles round the settlement; with
	# --whole-map the view culls nothing (the worst case: every entity of the level).
	var whole := args.has("--whole-map")
	if not whole:
		view.show_rect = Rect2i(fs.x - 30, fs.y - 17, 60, 34)
	print("FRAME_PREP view rect %s" % ("whole map" if whole else str(view.show_rect)))
	for i in frames:
		_t("sim.step(1)", func(): sim.step(1))
		var z: int = fs.z
		_t("bridge level_tiles x3", func():
			for dz in [-1, 0, 1]:
				if z + dz >= zr.x and z + dz <= zr.y:
					sim.level_tiles(z + dz))
		_t("bridge ground", func(): sim.ground())
		_t("bridge plants", func(): sim.plants())
		_t("bridge agents", func(): sim.agents())
		_t("bridge bands+thin+lines", func(): sim.bands(); sim.thin_view(); sim.band_lines())
		_t("bridge structures", func(): sim.structures())
		_t("bridge items", func(): sim.items())
		_t("bridge engravings", func(): sim.engravings())
		_t("bridge designations", func(): sim.designations())
		_t("bridge roofs+built_of", func(): sim.roofs(z); sim.built_of(z))
		_t("bridge carried", func(): sim.carried())
		_t("bridge scorched", func(): sim.scorched(z - 1, 10))
		_t("view.refresh (all reads + script)", func(): view.refresh())
		_t("view entity rebuild (static)", func(): view._rebuild_entities(true))
		_t("view entity rebuild (moving)", func(): view._rebuild_entities(false))
		_t("view._process", func(): view._process(1.0 / 60.0))
	for k in out:
		var v: Array = out[k]
		v.sort()
		var n := v.size()
		print("FRAME_PREP %-36s p50 %7.2f  p95 %7.2f  max %7.2f ms" % [k, v[n / 2], v[mini(n - 1, int(n * 0.95))], v[n - 1]])
	quit(0)
