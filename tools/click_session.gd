extends SceneTree

const Rules = preload("res://scripts/rules.gd")

# Scripted click-only session (#413 WP-04 slice 4.3, the #19 method) on a real monitor.
# Drives the live 1280x720 viewer through Viewport.push_input using ONLY mouse events
# (InputEventMouseMotion and InputEventMouseButton — zero InputEventKey events).

var main: Node2D
var out_dir: String = ""
var log_lines: PackedStringArray = PackedStringArray()
var last_mouse: Vector2 = Vector2(640, 360)
var failures: int = 0
var checks_passed: int = 0

func _initialize() -> void:
	out_dir = OS.get_environment("OSTROG_CLICK_SESSION_DIR")
	if out_dir == "":
		out_dir = ProjectSettings.globalize_path("res://../docs/reviews/413-click-only-session")
	DirAccess.make_dir_recursive_absolute(out_dir)
	var scene: PackedScene = load("res://scenes/main.tscn")
	main = scene.instantiate()
	root.add_child(main)
	_run.call_deferred()

func _log(msg: String) -> void:
	print("CLICK_SESSION " + msg)
	log_lines.append(msg)

func _check(cond: bool, desc: String) -> void:
	if cond:
		checks_passed += 1
		_log("PASS: " + desc)
	else:
		failures += 1
		_log("FAIL: " + desc)
		printerr("CLICK_SESSION_FAIL: " + desc)

func _wait_frames(n: int = 3) -> void:
	for i in range(n):
		await process_frame
	if DisplayServer.get_name() != "headless":
		await RenderingServer.frame_post_draw

func _move_mouse(pos: Vector2) -> void:
	# local viewport coords used via push_input(ev, true)
	var mm := InputEventMouseMotion.new()
	mm.position = pos
	mm.global_position = pos
	mm.relative = pos - last_mouse
	last_mouse = pos
	root.push_input(mm, true)
	await process_frame

func _click_at(pos: Vector2, label: String) -> void:
	await _move_mouse(pos)
	var down := InputEventMouseButton.new()
	down.button_index = MOUSE_BUTTON_LEFT
	down.pressed = true
	down.position = pos
	down.global_position = pos
	root.push_input(down, true)
	await process_frame
	var up := InputEventMouseButton.new()
	up.button_index = MOUSE_BUTTON_LEFT
	up.pressed = false
	up.position = pos
	up.global_position = pos
	root.push_input(up, true)
	await _wait_frames(3)
	_log("MOUSE_CLICK (%.1f, %.1f) -> %s" % [pos.x, pos.y, label])

func _click_button(btn: Button, label: String) -> void:
	if btn == null or not btn.is_visible_in_tree():
		_check(false, "button visible for click: " + label)
		return
	var rect := btn.get_global_rect()
	var center := rect.get_center()
	await _click_at(center, "%s [text='%s' rect=(%.0f,%.0f %.0fx%.0f)]" % [
		label, btn.text, rect.position.x, rect.position.y, rect.size.x, rect.size.y
	])

func _find_button(node: Node, substr: String) -> Button:
	if node == null or (node is CanvasItem and not node.visible):
		return null
	if node is Button and str(node.text).contains(substr):
		return node
	for c in node.get_children():
		var found := _find_button(c, substr)
		if found != null:
			return found
	return null

func _drag_mouse(from_pos: Vector2, to_pos: Vector2, steps: int = 8) -> void:
	await _move_mouse(from_pos)
	var down := InputEventMouseButton.new()
	down.button_index = MOUSE_BUTTON_LEFT
	down.pressed = true
	down.position = from_pos
	down.global_position = from_pos
	root.push_input(down, true)
	await process_frame
	for i in range(1, steps + 1):
		var t := float(i) / float(steps)
		var cur := from_pos.lerp(to_pos, t)
		var mm := InputEventMouseMotion.new()
		mm.position = cur
		mm.global_position = cur
		mm.relative = cur - last_mouse
		mm.button_mask = MOUSE_BUTTON_MASK_LEFT
		last_mouse = cur
		root.push_input(mm, true)
		await process_frame
	var up := InputEventMouseButton.new()
	up.button_index = MOUSE_BUTTON_LEFT
	up.pressed = false
	up.position = to_pos
	up.global_position = to_pos
	root.push_input(up, true)
	await _wait_frames(3)
	_log("MOUSE_DRAG (%.1f, %.1f) -> (%.1f, %.1f)" % [from_pos.x, from_pos.y, to_pos.x, to_pos.y])

func _wheel_at(pos: Vector2, button_idx: MouseButton, label: String) -> void:
	await _move_mouse(pos)
	var down := InputEventMouseButton.new()
	down.button_index = button_idx
	down.pressed = true
	down.factor = 1.0
	down.position = pos
	down.global_position = pos
	root.push_input(down, true)
	var up := InputEventMouseButton.new()
	up.button_index = button_idx
	up.pressed = false
	up.factor = 1.0
	up.position = pos
	up.global_position = pos
	root.push_input(up, true)
	await _wait_frames(3)
	_log("MOUSE_WHEEL %s at (%.1f, %.1f)" % [label, pos.x, pos.y])

func _world_to_screen(world_px: Vector2) -> Vector2:
	return main.get_canvas_transform() * world_px

func _save_shot(name: String) -> void:
	await _wait_frames(3)
	var path := out_dir.path_join(name)
	var img := root.get_texture().get_image()
	var err := img.save_png(path)
	_check(err == OK and img.get_width() == 1280 and img.get_height() == 720, "saved screenshot %s (%dx%d)" % [name, img.get_width(), img.get_height()])

func _run() -> void:
	await _wait_frames(6)
	var display_name := DisplayServer.get_name()
	var gpu_name := RenderingServer.get_video_adapter_name()
	var gpu_vendor := RenderingServer.get_video_adapter_vendor()
	var win_size := root.size
	_log(" Monitor & adapter: display=%s gpu='%s' vendor='%s' window=%dx%d" % [
		display_name, gpu_name, gpu_vendor, win_size.x, win_size.y
	])
	_check(display_name != "headless", "running on a real monitor (not headless)")
	_check(win_size.x == 1280 and win_size.y == 720, "viewport resolution is 1280x720")

	# If first-run help opened automatically, close it with a mouse click on its Close button.
	if main.panels.is_open(&"help"):
		var close_help := _find_button(main.help_panel, "Close")
		await _click_button(close_help, "Close initial Help panel")
		_check(not main.panels.is_open(&"help"), "initial Help panel closed by mouse click")

	# Ensure camera director is off so camera position assertions are deterministic; toggle via bar button.
	if main.director_on:
		await _click_button(main.bar_buttons["director"], "Toggle Director off on bottom bar")
		_check(not main.director_on, "Director turned off via bottom bar click")

	# 1. Hover a settler on the map with mouse motion and capture 01_hover_settler.png.
	var fs: Vector3i = main.sim.settlement_centre()
	var target_id: int = -1
	var target_world_px: Vector2 = Vector2.ZERO
	var agents: PackedInt32Array = main.view.agents
	for i in range(0, agents.size(), main.view.AGENT_STRIDE):
		if agents[i + 3] == main.view.view_z:
			target_id = agents[i]
			target_world_px = Rules.drawn_at(agents, i)
			break
	_check(target_id >= 0, "found a visible settler on settlement z-level (id=%d)" % target_id)
	main.cam.position = target_world_px
	await _wait_frames(3)
	var agent_screen := _world_to_screen( _agent_world_px(target_id) )
	await _move_mouse(agent_screen)
	await _wait_frames(3)
	_check(main.hover_id == target_id, "hovering settler updates hover_id=%d" % main.hover_id)
	_check(main.hud.text.contains("(click to inspect)"), "HUD shows hover line with (click to inspect)")
	await _save_shot("01_hover_settler.png")

	# 2. Speed & pause buttons on the bottom command bar (02_bar_speed_and_pause.png).
	await _click_button(main.bar_buttons["spd2"], "Bottom bar 3x speed")
	_check(main.speed_idx == 2, "clicking 3x sets speed_idx=2")
	await _click_button(main.bar_buttons["spd3"], "Bottom bar 10x speed")
	_check(main.speed_idx == 3, "clicking 10x sets speed_idx=3")
	await _click_button(main.bar_buttons["pause"], "Bottom bar Pause")
	_check(main.speed_idx == 0 and main.bar_buttons["pause"].text.contains("Play"), "clicking Pause pauses simulation (speed_idx=0)")
	await _save_shot("02_bar_speed_and_pause.png")

	# 3. Click settler on map to inspect, then click Why?, Follow, and Favourite on inspect panel (03_inspect_follow_and_fav.png).
	agent_screen = _world_to_screen(_agent_world_px(target_id))
	await _click_at(agent_screen, "Settler id=%d on map" % target_id)
	_check(main.selected_id == target_id and main.panels.is_open(&"inspect"), "clicking settler on map opens inspect panel")
	await _click_button(main.why_btn, "Inspect panel Why? button")
	_check(main.why_open and main.why_btn.text.contains("on"), "clicking Why? enables decision lens on inspect panel")
	await _click_button(main.inspect_follow_btn, "Inspect panel Follow button")
	_check(main.follow_id == target_id and main.inspect_follow_btn.text.contains("Following"), "clicking Follow sets follow_id to settler")
	await _click_button(main.inspect_fav_btn, "Inspect panel Favourite button")
	_check(main.sim.is_favourite(target_id) and main.bar_buttons["favs"].text.contains("(1)"), "clicking Favourite stars settler and updates bottom bar count")
	await _save_shot("03_inspect_follow_and_fav.png")

	# Close inspect panel via its Close button before testing map drag and block inspection.
	var close_insp := _find_button(main.inspect_panel, "Close")
	await _click_button(close_insp, "Inspect panel Close button")
	_check(not main.panels.is_open(&"inspect"), "clicking Close on inspect panel closes it")

	# Re-enable follow via Favourites or inspect so we can verify mouse-drag breaks follow.
	main.follow_id = target_id
	var cam_before_drag: Vector2 = main.cam.position
	await _drag_mouse(Vector2(640, 360), Vector2(480, 260), 8)
	_check(main.follow_id == -1, "dragging the map with left mouse button breaks follow")
	_check(main.cam.position.distance_to(cam_before_drag) > 20.0, "dragging the map pans the camera")

	# 4. Wheel zoom, Z- button to underground, and click underground block to inspect (04_drag_zoom_and_underground_block.png).
	var zoom_before: float = main.cam.zoom.x
	await _wheel_at(Vector2(640, 360), MOUSE_BUTTON_WHEEL_UP, "wheel up (zoom in)")
	_check(main.cam.zoom.x > zoom_before, "mouse wheel up zooms in (%.2f -> %.2f)" % [zoom_before, main.cam.zoom.x])
	await _wheel_at(Vector2(640, 360), MOUSE_BUTTON_WHEEL_DOWN, "wheel down (zoom out)")
	_check(is_equal_approx(main.cam.zoom.x, zoom_before), "mouse wheel down restores zoom (%.2f)" % main.cam.zoom.x)

	for k in range(4):
		await _click_button(main.bar_buttons["z_down"], "Bottom bar Z- (step %d)" % (k + 1))
	_check(main.view.view_z == fs.z - 4, "clicking Z- four times drops view_z to underground z=%d" % main.view.view_z)

	await _click_at(Vector2(520, 340), "Underground rock tile at (520, 340)")
	_check(main.panels.is_open(&"block") and main.block_box.text.contains("underground"), "clicking underground tile opens Block inspection with underground details")
	await _save_shot("04_drag_zoom_and_underground_block.png")

	# 5. Click Home on bar, step Z- to lake water level, click water tile to inspect (05_home_and_water_block_inspect.png).
	var close_blk := _find_button(main.block_panel, "Close")
	await _click_button(close_blk, "Block panel Close button")
	_check(not main.panels.is_open(&"block"), "Block inspection panel closed by mouse click")

	await _click_button(main.bar_buttons["home"], "Bottom bar Home button")
	_check(main.view.view_z == fs.z, "clicking Home restores settlement surface z=%d" % fs.z)

	await _click_button(main.bar_buttons["z_down"], "Bottom bar Z- to lake level z=2")
	var water_world_px := Vector2(84 * 32 + 16, 55 * 32 + 16)
	main.cam.position = water_world_px
	await _wait_frames(3)
	var water_screen := _world_to_screen(water_world_px)
	await _click_at(water_screen, "Water tile (84, 55, z2)")
	_check(main.panels.is_open(&"block") and main.block_box.text.contains("Water"), "clicking water tile opens Block inspection showing water kind and bed")
	await _save_shot("05_home_and_water_block_inspect.png")

	var goto_blk := _find_button(main.block_panel, "Go to")
	await _click_button(goto_blk, "Block panel Go to button")
	await _click_button(close_blk, "Block panel Close button")
	await _click_button(main.bar_buttons["home"], "Bottom bar Home button")

	# 6. Click Favs on bottom bar, click Follow on Favourites panel, and verify Save/Load persistence by mouse (06_favourites_panel_and_save_load.png).
	await _click_button(main.bar_buttons["favs"], "Bottom bar Favs button")
	_check(main.panels.is_open(&"favourites"), "clicking Favs on bottom bar opens Favourites panel")
	var fav_follow_btn := _find_button(main.fav_list_box, "Follow")
	await _click_button(fav_follow_btn, "Favourites row Follow button")
	_check(main.follow_id == target_id, "clicking Follow on Favourites row follows the settler")
	await _save_shot("06_favourites_panel_and_save_load.png")

	var close_fav := _find_button(main.fav_panel, "Close")
	await _click_button(close_fav, "Favourites panel Close button")
	_check(not main.panels.is_open(&"favourites"), "clicking Close on Favourites panel closes it")

	# Click Save on bottom bar, wait for off-thread save to finish, clear favourite, click Load on bottom bar.
	await _click_button(main.bar_buttons["save"], "Bottom bar Save button")
	var wait_save := 0
	while not str(main.save_note).begins_with("saved") and wait_save < 300:
		await process_frame
		wait_save += 1
	_check(str(main.save_note).begins_with("saved"), "clicking Save completes manual save (%s)" % main.save_note)
	main.sim.toggle_favourite(target_id)
	_check(not main.sim.is_favourite(target_id), "cleared favourite in memory before Load")
	await _click_button(main.bar_buttons["load"], "Bottom bar Load button")
	_check(main.sim.is_favourite(target_id), "clicking Load restores favourited settler from manual save")

	# 7. Click History, Legends, Timelapse, Known, Alerts, Pause!, and Marks on bottom bar (07_history_and_legends_panels.png).
	await _click_button(main.bar_buttons["history"], "Bottom bar History button")
	_check(main.panels.is_open(&"summary") and main.summary_box.text.contains("History"), "clicking History opens History panel")
	await _save_shot("07_history_and_legends_panels.png")
	var close_sum := _find_button(main.summary_panel, "Close")
	await _click_button(close_sum, "History panel Close button")
	_check(not main.panels.is_open(&"summary"), "clicking Close on History panel closes it")

	await _click_button(main.bar_buttons["legends"], "Bottom bar Legends button")
	_check(main.panels.is_open(&"legends"), "clicking Legends opens Legends browser")
	var close_leg := _find_button(main.legends_panel, "Close")
	await _click_button(close_leg, "Legends panel Close button")
	_check(not main.panels.is_open(&"legends"), "clicking Close on Legends panel closes it")

	await _click_button(main.bar_buttons["timelapse"], "Bottom bar Timelapse button")
	_check(main.panels.is_open(&"timelapse"), "clicking Timelapse opens Timelapse panel")
	await _click_button(main.bar_buttons["timelapse"], "Bottom bar Timelapse button (close)")
	_check(not main.panels.is_open(&"timelapse"), "clicking Timelapse again closes Timelapse panel")

	await _click_button(main.bar_buttons["known"], "Bottom bar Known toggle")
	_check(main.view.known_only, "clicking Known switches to known-only view")
	await _click_button(main.bar_buttons["known"], "Bottom bar Known toggle (restore all)")
	_check(not main.view.known_only, "clicking Known again restores full view")

	var ann_before: int = main.announce_mode
	await _click_button(main.bar_buttons["announce"], "Bottom bar Alerts mode")
	_check(main.announce_mode != ann_before, "clicking Alerts cycles announcement mode")
	var pm_before: bool = main.pause_on_major
	await _click_button(main.bar_buttons["pause_major"], "Bottom bar Pause! toggle")
	_check(main.pause_on_major != pm_before, "clicking Pause! toggles pause-on-major-events")

	await _click_button(main.bar_buttons["bookmarks"], "Bottom bar Marks button")
	_check(main.bookmarks_popup.visible, "clicking Marks opens camera bookmarks popup")
	var close_bm := _find_button(main.bookmarks_popup, "Close")
	await _click_button(close_bm, "Bookmarks popup Close button")
	_check(not main.bookmarks_popup.visible, "clicking Close on bookmarks popup closes it")

	# 8. Click Settings and Help (?) on bottom bar (08_settings_and_help_panels.png).
	await _click_button(main.bar_buttons["settings"], "Bottom bar Settings button")
	_check(main.settings_panel.visible, "clicking Settings opens Settings panel")
	await _save_shot("08_settings_and_help_panels.png")
	var close_set := _find_button(main.settings_panel, "Close")
	await _click_button(close_set, "Settings panel Close button")
	_check(not main.settings_panel.visible, "clicking Close on Settings panel closes it")

	await _click_button(main.bar_buttons["help"], "Bottom bar Help (?) button")
	_check(main.panels.is_open(&"help"), "clicking ? opens Help panel")
	var close_hlp := _find_button(main.help_panel, "Close")
	await _click_button(close_hlp, "Help panel Close button")
	_check(not main.panels.is_open(&"help"), "clicking Close on Help panel closes it")

	# 9. Click ▼ Bar to hide the bottom command bar, and ▲ Bar to restore it (09_bar_hidden_clean_view.png, 10_bar_restored.png).
	await _click_button(main.bar_toggle_btn, "Bottom-right ▼ Bar hide button")
	_check(not main.bar_panel.visible and main.bar_toggle_btn.text.contains("▲"), "clicking ▼ Bar hides the bottom command bar")
	await _save_shot("09_bar_hidden_clean_view.png")

	await _click_button(main.bar_toggle_btn, "Bottom-right ▲ Bar show button")
	_check(main.bar_panel.visible and main.bar_toggle_btn.text.contains("▼"), "clicking ▲ Bar restores the bottom command bar")
	await _save_shot("10_bar_restored.png")

	_log("SUMMARY: %d passed, %d failed (0 keyboard events used)" % [checks_passed, failures])
	var log_file := FileAccess.open(out_dir.path_join("session.txt"), FileAccess.WRITE)
	if log_file != null:
		for l in log_lines:
			log_file.store_line(l)
		log_file.close()

	quit(1 if failures > 0 else 0)

func _agent_world_px(id: int) -> Vector2:
	var a: PackedInt32Array = main.view.agents
	for i in range(0, a.size(), main.view.AGENT_STRIDE):
		if a[i] == id:
			return Rules.drawn_at(a, i)
	return Vector2(-1, -1)