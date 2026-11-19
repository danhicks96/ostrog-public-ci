extends SceneTree
## Windowed #484 integration evidence. Run through tools/shots/timelapse.sh, never --headless.

const MainScene := preload("res://scenes/main.tscn")
const Timelapse := preload("res://scripts/timelapse.gd")

var main
var failed := 0
var checks := 0
var output_dir := "user://timelapse-evidence"
var series_dir := ""

func _init() -> void:
	call_deferred("_run")

func check(ok: bool, what: String) -> void:
	checks += 1
	if not ok:
		failed += 1
		printerr("TIMELAPSE_VISUAL_FAIL %s" % what)

func _capture() -> void:
	main.timelapse_next_tick = main.sim.tick()
	main._capture_timelapse()
	while main.timelapse_capture_active:
		await process_frame

func _remove_tree(path: String) -> void:
	var absolute := ProjectSettings.globalize_path(path)
	if not DirAccess.dir_exists_absolute(absolute):
		return
	var dir := DirAccess.open(absolute)
	if dir == null:
		return
	for name in dir.get_files():
		DirAccess.remove_absolute(absolute.path_join(name))
	for name in dir.get_directories():
		_remove_tree(absolute.path_join(name))
	DirAccess.remove_absolute(absolute)

func _copy_sample(frame: Dictionary, destination: String) -> void:
	var png: String = frame["path"]
	var json := png.replace(".png", ".json")
	check(DirAccess.copy_absolute(ProjectSettings.globalize_path(png), ProjectSettings.globalize_path(destination + ".png")) == OK, "copy evidence PNG")
	check(DirAccess.copy_absolute(ProjectSettings.globalize_path(json), ProjectSettings.globalize_path(destination + ".json")) == OK, "copy evidence JSON")

func _run() -> void:
	var configured_output := OS.get_environment("HF_TIMELAPSE_OUTPUT")
	if configured_output != "":
		output_dir = configured_output
	DirAccess.make_dir_recursive_absolute(ProjectSettings.globalize_path(output_dir))
	var test_root := "user://test_484_visual_%d" % OS.get_process_id()
	_remove_tree(test_root)
	main = MainScene.instantiate()
	get_root().add_child(main)
	while main.view == null:
		await process_frame
	main.speed_idx = 0
	main.director_on = false
	main.timelapse_recording = false
	while main.timelapse_capture_active:
		await process_frame
	main.timelapse_root = test_root
	main._open_timelapse_series()
	series_dir = main.timelapse_series_dir
	check(series_dir.begins_with(test_root + "/world-"), "the series lives under the world's identity")
	check(main.timelapse.frames.is_empty(), "a fresh world starts an empty series")
	main.timelapse_recording = true
	main._use_current_timelapse_view()
	while main.timelapse_capture_active:
		await process_frame
	check(main.timelapse.frames.size() == 1, "first real viewport sample enters the ring")
	if main.timelapse.frames.is_empty():
		_finish({})
		return
	var first: Dictionary = main.timelapse.frames[-1].duplicate(true)
	var metadata = JSON.parse_string(FileAccess.get_file_as_string(str(first["path"]).replace(".png", ".json")))
	check(FileAccess.file_exists(str(first["path"])), "viewport PNG exists")
	check(metadata is Dictionary and int(metadata.get("tick", -1)) == main.sim.tick(), "JSON names the captured simulation tick")
	check(metadata is Dictionary and str(metadata.get("state_digest", "")) == main.sim.state_digest(), "JSON state digest matches the source world")
	_copy_sample(first, output_dir.path_join("same-tick-a"))
	var source_tick: int = main.sim.tick()
	var source_digest: String = main.sim.state_digest()
	var watched: Dictionary = main.timelapse_watch.duplicate(true)
	var live_position: Vector2 = main.cam.position + Vector2(384, 192)
	var live_zoom := Vector2.ONE * 2.0
	var live_known: bool = not main.view.known_only
	main.cam.position = live_position
	main.cam.zoom = live_zoom
	main.view.known_only = live_known
	await _capture()
	var second: Dictionary = main.timelapse.frames[-1]
	_copy_sample(second, output_dir.path_join("same-tick-b"))
	check(main.timelapse.frames.size() == 1 and int(second["tick"]) == int(first["tick"]), "same-tick recapture replaces rather than duplicates")
	check(str(second["png_sha256"]) == str(first["png_sha256"]), "identical seed/tick/filter produce equivalent viewport PNGs")
	check(main.timelapse_watch == watched, "live camera movement does not change the watched view")
	check(main.cam.position.is_equal_approx(live_position) and main.cam.zoom.is_equal_approx(live_zoom) and main.view.known_only == live_known, "capture restores the independent live camera and filter")
	check(main.sim.tick() == source_tick and main.sim.state_digest() == source_digest, "capture leaves source state and time unchanged")
	main.sim.step(1)
	main.sim.sync()
	await _capture()
	main.panels.open(&"timelapse")
	var panel = main.timelapse_panel
	panel.select_frame(0)
	panel.pin_button.pressed.emit()
	panel.select_frame(1)
	source_tick = main.sim.tick()
	source_digest = main.sim.state_digest()
	panel.play_button.pressed.emit()
	panel._process(1.1)
	check(panel.selected_tick == source_tick, "panel playback reaches the second real sample")
	panel.select_frame(0)
	check(not panel.playing and panel.reference_tick == int(first["tick"]), "scrub pauses playback and retains the pinned tick")
	check(main.sim.tick() == source_tick and main.sim.state_digest() == source_digest, "playback, scrub and pin do not mutate the source")
	panel.select_frame(main.timelapse.frames.size() - 1)
	var start_tick := int(first["tick"])
	var target_tick := start_tick + 120
	while main.sim.tick() < target_tick:
		main.sim.step(1)
		main.sim.sync()
		await _capture()
	check(main.timelapse.frames.size() == 120, "121 real viewport samples retain exactly 120")
	check(int(main.timelapse.frames[0]["tick"]) == start_tick + 1 and int(main.timelapse.frames[-1]["tick"]) == target_tick, "real retention preserves chronological tick identity")
	check(panel.selected_tick == target_tick, "follow-latest survives append plus eviction")
	check(panel.reference_tick == start_tick + 1, "pinned sample eviction falls back by tick, not array index")
	check(not FileAccess.file_exists("%s/%020d.png" % [series_dir, start_tick]), "sample 121 deletes the oldest viewport PNG")
	check(not FileAccess.file_exists("%s/%020d.json" % [series_dir, start_tick]), "sample 121 deletes the oldest JSON")
	var retained_files := 0
	for frame in main.timelapse.frames:
		check(FileAccess.file_exists(str(frame["path"])), "retained PNG exists at tick %d" % int(frame["tick"]))
		check(FileAccess.file_exists(str(frame["path"]).replace(".png", ".json")), "retained JSON exists at tick %d" % int(frame["tick"]))
		retained_files += 2
	var actual_files := DirAccess.open(ProjectSettings.globalize_path(series_dir)).get_files().size()
	check(actual_files == retained_files, "series directory contains exactly the 120 retained PNG/JSON pairs")
	# Exact days from here on (#484 finish): each sample lands on a noon tick, a save taken
	# before them reloads into the same series, and re-running the same days reproduces the
	# same state digests and the same PNG bytes without the review touching the source.
	var save_base := ProjectSettings.globalize_path(test_root).path_join("replay")
	check(main.sim.save_to(save_base + ".0.hfsave"), "a save of the source starts")
	var waited := 0
	while main.sim.newest_valid(save_base, 1) == "" and waited < 1200:
		waited += 1
		await process_frame
	check(main.sim.newest_valid(save_base, 1) != "", "the save is written and verifies")
	var growth: Array = []
	for day in range(4):
		var noon: int = Timelapse.next_sample_tick(main.sim.tick() + 1, main.timelapse_epoch)
		main.sim.step(noon - main.sim.tick())
		main.sim.sync()
		await _capture()
		var frame: Dictionary = main.timelapse.frames[-1].duplicate(true)
		check(int(frame["tick"]) == noon and bool(frame.get("scheduled", false)), "day %d sample is at exactly noon (tick %d)" % [day, noon])
		check(frame.get("events", {}).has("lines"), "day %d sample carries its event markers" % day)
		var destination := output_dir.path_join("day-%d" % day)
		_copy_sample(frame, destination)
		growth.append({
			"day": day,
			"tick": frame["tick"],
			"calendar": frame["calendar"],
			"state_digest": frame["state_digest"],
			"png_sha256": frame["png_sha256"],
			"bytes": frame["bytes"],
			"events": (frame["events"]["lines"] as Array).size(),
			"structure_rows": main.sim.structures().size() / 8,
			"png": destination + ".png",
			"json": destination + ".json",
		})
	check(str(growth[0]["state_digest"]) != str(growth[-1]["state_digest"]), "multi-day captures contain advanced simulation state")
	var retained_before_reload: int = main.timelapse.frames.size()
	check(main.sim.load_from(main.sim.newest_valid(save_base, 1)) == "", "the earlier save loads")
	main._world_replaced()
	while main.timelapse_capture_active:
		await process_frame
	check(main.timelapse.frames.size() == retained_before_reload, "the reloaded world reopens its own series from disk (%d samples)" % main.timelapse.frames.size())
	check(main.timelapse_watch.get("position", []) == main.timelapse.frames[-1]["camera"]["position"], "the reloaded series keeps watching the same view")
	var replayed: Array = []
	for g in growth:
		main.sim.step(int(g["tick"]) - main.sim.tick())
		main.sim.sync()
		await _capture()
		var again: Dictionary = _frame_at(int(g["tick"]))
		_copy_sample(again, output_dir.path_join("rerun-day-%d" % int(g["day"])))
		check(str(again.get("replay", "")) == "confirmed", "re-running day %d from the save confirms the stored sample" % int(g["day"]))
		check(str(again.get("state_digest", "")) == str(g["state_digest"]), "re-run day %d reaches the same state digest" % int(g["day"]))
		check(bool(again.get("replay_png_match", false)) and str(again.get("png_sha256", "")) == str(g["png_sha256"]), "re-run day %d renders the same PNG bytes" % int(g["day"]))
		replayed.append({"tick": g["tick"], "replay": again.get("replay", ""), "png_match": again.get("replay_png_match", false)})
	# A history the world no longer has: tamper the stored digest of day 2, re-capture day 2,
	# and that sample and every later one must be removed, files included.
	var day3_png := str(_frame_at(int(growth[3]["tick"])).get("path", ""))
	main.sim.load_from(main.sim.newest_valid(save_base, 1))
	main._world_replaced()
	_frame_at(int(growth[2]["tick"]))["state_digest"] = "not-this-world"
	main.sim.step(int(growth[2]["tick"]) - main.sim.tick())
	main.sim.sync()
	await _capture()
	check(str(_frame_at(int(growth[2]["tick"])).get("replay", "")) == "diverged", "a different state at a stored tick is reported as diverged")
	check(_frame_at(int(growth[3]["tick"])).is_empty() and not FileAccess.file_exists(day3_png), "samples after a divergence are removed with their files")
	# The whole valley, fitted to the window, at the settlement's level.
	main.view.known_only = false
	var t0 := Time.get_ticks_msec()
	main._use_valley_timelapse_view()
	while main.timelapse_capture_active:
		await process_frame
	var valley_ms := Time.get_ticks_msec() - t0
	var valley: Dictionary = main.timelapse.frames[-1]
	check(str(valley["camera"].get("mode", "")) == "valley", "the valley sample records its watch mode")
	check(is_equal_approx(float(valley["camera"]["zoom"]), Timelapse.valley_zoom(main.get_viewport_rect().size, Vector2(main.view.w, main.view.h) * 32.0)), "the valley sample fits the whole map")
	check(int(valley["image_pixels"][0]) <= Timelapse.CAPTURE_MAX.x and int(valley["image_pixels"][1]) <= Timelapse.CAPTURE_MAX.y, "the stored image is within the capture bound")
	_copy_sample(valley, output_dir.path_join("valley"))
	# The review panel over the series, for the evidence screenshot.
	main.panels.open(&"timelapse")
	main._sync_timelapse_panel()
	panel.select_frame(main.timelapse.frames.size() - 2)
	panel.reference_tick = int(growth[0]["tick"])
	panel.sync(main.timelapse.frames, true, main.timelapse.total_bytes)
	panel.select_frame(main.timelapse.frames.size() - 2)
	for i in range(4):
		await RenderingServer.frame_post_draw
	check(main.get_viewport().get_texture().get_image().save_png(output_dir.path_join("panel.png")) == OK, "panel evidence screenshot writes")
	var on_disk := 0
	for f in DirAccess.open(ProjectSettings.globalize_path(series_dir)).get_files():
		if f.ends_with(".png"):
			on_disk += FileAccess.get_file_as_bytes(series_dir.path_join(f)).size()
	check(on_disk == main.timelapse.total_bytes, "retained bytes reported equal the PNG bytes on disk (%d)" % on_disk)
	_finish({"equivalent_png_sha256": first["png_sha256"], "retained_files": retained_files, "growth": growth, "replayed": replayed, "valley_capture_ms": valley_ms, "valley_bytes": valley["bytes"], "retained_png_bytes": on_disk})

func _frame_at(tick: int) -> Dictionary:
	for f in main.timelapse.frames:
		if int(f["tick"]) == tick:
			return f
	return {}

func _finish(evidence: Dictionary) -> void:
	if main != null:
		main.timelapse_recording = false
		main.speed_idx = 0
	var report := {
		"issue": 484,
		"checks": checks,
		"failed": failed,
		"evidence": evidence,
	}
	DirAccess.make_dir_recursive_absolute(ProjectSettings.globalize_path(output_dir))
	var file := FileAccess.open(output_dir.path_join("report.json"), FileAccess.WRITE)
	if file != null:
		file.store_string(JSON.stringify(report, "\t") + "\n")
		file.close()
	print("TIMELAPSE_VISUAL %d checks, %d failed; report %s" % [checks, failed, output_dir.path_join("report.json")])
	if main != null:
		main.queue_free()
		await process_frame
	_remove_tree(series_dir.get_base_dir())
	quit(1 if failed > 0 else 0)
