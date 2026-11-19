extends Node2D
## Ostrog client: live simulation at 20 ticks/s (1x), speed control, feed, inspect,
## camera director, away summary, --shot and --footage headless modes.

const TILE := 32
const Rules := preload("res://scripts/rules.gd")
const Timelapse := preload("res://scripts/timelapse.gd")
const TimelapsePanel := preload("res://scripts/timelapse_panel.gd")
const HistoryReel := preload("res://scripts/history_reel.gd")
const AwayMap := preload("res://scripts/away_map.gd")
## The key list, shown on [?] (and once, on a first launch) instead of on every HUD line (#181).
const HELP_TEXT := "[b]Mouse & Keys[/b]\n\n[b]Mouse[/b]\nClick any person or animal to inspect them (Follow, ★ Favourite, Go to)\nClick any ground, water, or rock tile to inspect what it holds\nDrag with any mouse button to pan (dragging stops following)\nMouse wheel zooms toward the pointer\nThe bottom bar has every command (B to hide/show)\nClick any feed line or top announcement to look where it happened\n\n[b]Keys[/b]\n[Space] pause   [1 2 3] speed   [Home] settlement\n[, .] or PgUp/PgDn   level up and down\n[C] camera director   [F] follow inspected   [J] ★ favourites\n[P] people   [H] history   [T] timelapse   [L] legends\n[V] what the community has seen   [G] feed rate   [K] waiting work\n[N] alerts   [Shift+N] pause on major   [B] toggle bar\n[F5] save   [F9] load   [O] settings   [?] help\n[Ctrl+F1–8] set bookmark, [F1–8] go to it"
const TICKS_PER_SEC := 20.0
const SPEEDS := [0, 1, 3, 10]

var sim: SimHost
var view
var assets
var cam: Camera2D
var shot_path := ""
## Verification only: step through the wizard, shoot each page, then begin.
var wizard_shot_path := ""
## Verification only: choices the wizard shot makes first, "key=value" pairs separated by commas
## (deity=relentless, years=300, held=hunger:-1).
var wizard_choices := ""
## Start from the arrival: three of ten, paced by this Engine ("--opening=relentless").
var opening_arg := ""
var footage_dir := ""
var footage_every := 0
var footage_frames := 0
var footage_n := 0
var steps := 0
var seed_v := 1
## --fixed-seed: the New game wizard opens on --seed (default 1) instead of a fresh random
## valley (#559). Wizard shots always use --seed, so they are repeatable.
var fixed_seed_arg := false
var stage := ""
var zoom_arg := 1.0
var z_arg := 9999
var focus_arg := Vector2i(-1, -1)
var audio_log := ""
## --debug-hud: the HUD also shows the step time and the engine's strain (#1740); off by default.
var debug_hud := false
## --debug-log[=PATH] or env OSTROG_DEBUG_LOG: the decision log (#565); "" is off.
var debug_log := ""
var ignite_arg := Vector2i(-1, -1)  # --ignite=x,y: set the ground alight (review shots)
# --hungry-neighbour (review shots): empty the stores of one place beyond the valley before the
# run; whether and when its people come is still the simulation's (threat.rs).
var hungry_arg := false
var focus_band := false  # --at=band: centre on the first band's camp
var ignite_after := 160             # --after-ignite=N: ticks to run once it is alight  # --audio-log=PATH: footage/shot runs play sound and log what played
var history_arg := false
## Open the Legends browser at this page ("world:0", "site:3" ...) for a shot.
var legends_arg := ""
## Headless runs skip the autosave ring unless --ring is given (recovery demonstrations).
var ring_arg := false
var known_arg := false
var rel_lines_arg := false
var director_arg := true
var inspect_arg := -1
# --why (shots): open the inspect panel's Why? lens (#567).
var why_arg := false
var inspect_block_arg := Vector3i(-1, -1, -1)
var fav_arg := PackedInt32Array()
var fav_panel_arg := false
## --waiting (shots): open the Waiting work panel (#292, WP-08 8.6).
var waiting_panel_arg := false
var bar_hidden_arg := false
var feed_rate_arg := -1
var first_run_hints_arg := false
var feed_rate := 1
var feed_rate_btn: Button
## Name tags over people (#1428): Rules.NAME_TAG_NAMES, kept in settings.
var name_tag_mode := 1
var name_tag_btn: Button
var hints_active := false
var hint_inspected_person := false
var hint_inspected_block := false
var hint_used_bar := false
## The arrival setup screen, when this launch is a hands-on one.
var setup_panel: Node = null
## Whoever the pointer is over, shown in the HUD before you commit to clicking them.
var hover_id := -1
## The hovered person's words for the tag beside them (#1429), cached with the HUD line.
var hover_tag_text := ""
var hover_cache_id := -2
var hover_cache_text := ""
var hover_tile := Vector2i(-1, -1)  # the tile under the mouse, for a building's progress

var speed_idx := 1
var tick_accum := 0.0
var hud: Label
var feed_box: RichTextLabel
var feed_lines: Array = []
## Fold key -> the ticks it was toasted at in the last ten game-days (#1487, Rules.toast_repeat).
var toast_seen: Dictionary = {}
## Every panel on screen by id: which are open, and which is on top (#351).
var panels := PanelManager.new()
## Transient lines for what should not need the feed to be read.
var toaster: Toaster
## What earns a toast (#329): Rules.ANNOUNCE_NAMES, cycled with N and kept between runs;
## Shift+N pauses the world on a major line.
var announce_mode := 0
var pause_on_major := false
const ANNOUNCE_CFG := "user://announce.cfg"
var inspect_box: RichTextLabel
var inspect_panel: PanelContainer
var portrait
var inspect_follow_btn: Button
var inspect_fav_btn: Button
## The inspect panel's text as last shown: it is set again only when it changes, so a scroll
## holds (#178), and rebuilt a few times a second, not every frame.
var inspect_text := ""
# The Why? lens (#567): shown on the inspect panel when open; whoever is selected is traced.
var why_open := false
var why_traced_id := -1
var why_name := ""
var why_btn: Button
var inspect_accum := 0.0
var block_panel: PanelContainer
var block_box: RichTextLabel
var selected_block := Vector3i(-1, -1, -1)
var fav_panel: PanelContainer
## Work nobody has taken, and why (#292, WP-08 8.6).
var waiting_panel: PanelContainer
var waiting_list_box: VBoxContainer
var fav_list_box: VBoxContainer
var fav_empty_label: Label
var people_panel: PanelContainer
var people_list_box: VBoxContainer
var people_empty_label: Label
var people_title: Label
var people_panel_arg := false
var fav_accum := 0.0
var bar_panel: PanelContainer
var bar_toggle_btn: Button
var bar_buttons: Dictionary = {}
var bookmarks_popup: PanelContainer
var lmb_down := false
var lmb_start := Vector2.ZERO
var lmb_dragging := false
var help_panel: PanelContainer
var help_hdr: HBoxContainer
var help_text: RichTextLabel
var summary_box: RichTextLabel
var summary_panel: PanelContainer
## The valley's ending (C60): shown once, when nobody of any community is left in it.
var ending_shown := false
var ending_accum := 0.0
## The Legends browser (L): the world map at a year, and pages about the world's peoples,
## settlements, figures and realms with links between them (view::legends).
var legends_panel: PanelContainer
var legends_text: RichTextLabel
var legends_map: TextureRect
var legends_year: HSlider
var legends_year_label: Label
## Those of the valley in another place (#562), under the map of the present.
var legends_away: RichTextLabel
var legends_back: Array = []
var selected_id := -1
var follow_id := -1
var director_on := true
var director_target := Vector2.ZERO
var director_reason := ""
var director_next_check := 0.0
var last_input_time := -1000.0
var focus_lost_tick := -1
## The away summary opens when focus returns after more than a game-hour away (a real minute at
## 1x, twenty seconds at 3x): shorter than that, a glance at the feed covers it (issue 101).
const AWAY_TICKS := 1200
var window_focused := true
var refresh_accum := 0.0
var step_ms_avg := 0.0
var persist
var last_autosave_tick := 0
var save_note := ""
var save_note_until := 0.0
var settings_panel: PanelContainer
## WP-09 9.12: "Your files and data", opened from Settings.
var files_page: PanelContainer
## D94: the bar's Load asks before replacing the running valley.
var load_popup: PanelContainer
var load_question_label: Label
var settings_box: VBoxContainer
var director_cb: CheckBox
var debug_log_hint: Label
var sound  # scripts/audio/sound.gd
var music  # scripts/audio/music_director.gd
var started_from_save := false
var fresh_arg := false
## Step the sim in place on the main thread, not on its own thread (#603), for debugging.
var sim_inline_arg := false
var save_at_end := ""
## Optional machine-readable result for deterministic client smoke runs (#294 / #378).
var automation_json_path := ""
var load_arg := ""
var timelapse := Timelapse.new()
var timelapse_panel
var timelapse_recording := false
var timelapse_capture_active := false
var timelapse_next_tick := -1
var timelapse_watch: Dictionary = {}
var timelapse_series_dir := ""
var timelapse_identity := ""
var timelapse_root := Timelapse.ROOT  # tests point this elsewhere so they never prune real series
var timelapse_epoch := 0

func _parse_args() -> void:
	for a in OS.get_cmdline_user_args():
		if a.begins_with("--shot="): shot_path = a.substr(7)
		elif a.begins_with("--wizard-shot="): wizard_shot_path = a.substr(14)
		elif a.begins_with("--wizard-choices="): wizard_choices = a.substr(17)
		elif a.begins_with("--opening="): opening_arg = a.substr(10)
		elif a.begins_with("--footage="): footage_dir = a.substr(10)
		elif a.begins_with("--every="): footage_every = int(a.substr(8))
		elif a.begins_with("--frames="): footage_frames = int(a.substr(9))
		elif a.begins_with("--steps="): steps = int(a.substr(8))
		elif a.begins_with("--ticks="): steps = int(a.substr(8))  # same thing, the CLI's word
		elif a.begins_with("--seed="): seed_v = int(a.substr(7))
		elif a.begins_with("--z="): z_arg = int(a.substr(4))
		elif a.begins_with("--stage="): stage = a.substr(8)
		elif a.begins_with("--zoom="): zoom_arg = float(a.substr(7))
		elif a.begins_with("--speed="): speed_idx = clampi(int(a.substr(8)), 0, SPEEDS.size() - 1)
		elif a == "--no-director": director_arg = false
		elif a == "--fresh": fresh_arg = true
		elif a == "--sim-inline": sim_inline_arg = true
		elif a == "--fixed-seed": fixed_seed_arg = true
		elif a == "--history": history_arg = true
		elif a.begins_with("--legends="): legends_arg = a.substr(10)
		elif a == "--ring": ring_arg = true
		elif a == "--known": known_arg = true
		elif a == "--rel-lines": rel_lines_arg = true
		elif a == "--music-fast": pass  # read by audio/music_director.gd
		elif a == "--verbose-audio": pass  # read by audio/music_director.gd
		elif a == "--debug-hud": debug_hud = true
		elif a == "--debug-log": debug_log = "default"
		elif a.begins_with("--debug-log="): debug_log = a.substr(12)
		elif a.begins_with("--save-at-end="): save_at_end = a.substr(14)
		elif a.begins_with("--automation-json="): automation_json_path = a.substr(18)
		elif a.begins_with("--load="): load_arg = a.substr(7)
		elif a.begins_with("--inspect="): inspect_arg = int(a.substr(10))
		elif a == "--why": why_arg = true
		elif a.begins_with("--inspect-block="):
			var bp := a.substr(16).split(",")
			if bp.size() >= 3:
				inspect_block_arg = Vector3i(int(bp[0]), int(bp[1]), int(bp[2]))
			elif bp.size() == 2:
				inspect_block_arg = Vector3i(int(bp[0]), int(bp[1]), 9999)
		elif a.begins_with("--favourite="):
			for fid in a.substr(12).split(",", false):
				fav_arg.append(int(fid))
		elif a == "--favourites": fav_panel_arg = true
		elif a == "--people": people_panel_arg = true
		elif a == "--waiting": waiting_panel_arg = true
		elif a == "--hide-bar": bar_hidden_arg = true
		elif a == "--first-run-hints": first_run_hints_arg = true
		elif a.begins_with("--feed-rate="):
			var fr := a.substr(12).to_lower()
			if fr == "calm" or fr == "0": feed_rate_arg = 0
			elif fr == "normal" or fr == "1": feed_rate_arg = 1
			elif fr == "full" or fr == "2": feed_rate_arg = 2
		elif a.begins_with("--audio-log="): audio_log = a.substr(12)
		elif a.begins_with("--after-ignite="): ignite_after = int(a.substr(15))
		elif a.begins_with("--ignite="):
			var q := a.substr(9).split(",")
			ignite_arg = Vector2i(int(q[0]), int(q[1]))
		elif a == "--hungry-neighbour": hungry_arg = true
		elif a == "--at=band": focus_band = true
		elif a.begins_with("--at="):
			var p := a.substr(5).split(",")
			focus_arg = Vector2i(int(p[0]), int(p[1]))
		else:
			# A mistyped flag used to do nothing silently (issue 21: --ticks looked like it worked).
			print("UNKNOWN_FLAG %s (ignored)" % a)

## The decision log (#565): off unless --debug-log or OSTROG_DEBUG_LOG. Its header (seed,
## commit, settings) is written by the sim at the first decision.
func _start_debug_log() -> void:
	if debug_log == "":
		debug_log = OS.get_environment("OSTROG_DEBUG_LOG")
	if debug_log == "" and persist.settings["debug_log"]:
		debug_log = "default"
	if debug_log == "":
		return
	_debug_log_on()

## Turns the log on at `debug_log` ("default" and "1" mean the user data folder) and says where.
func _debug_log_on() -> void:
	if debug_log == "default" or debug_log == "1" or debug_log == "":
		debug_log = ProjectSettings.globalize_path("user://ostrog-debug.log")
	var settings := "command=client seed=%d ticks=%d stage=%s opening=%s load=%s" % [seed_v, steps, stage if stage != "" else "-", opening_arg if opening_arg != "" else "-", load_arg if load_arg != "" else "-"]
	var cmd := " ".join(PackedStringArray([OS.get_executable_path()]) + OS.get_cmdline_args() + OS.get_cmdline_user_args())
	sim.debug_log_start(debug_log, settings, cmd)
	print("DEBUG_LOG on: %s" % debug_log)

func _on_debug_log_toggled(on: bool) -> void:
	persist.settings["debug_log"] = on
	persist.save_settings()
	if on:
		debug_log = ""
		_debug_log_on()
	else:
		sim.debug_log_stop()
		debug_log = ""
	if debug_log_hint != null:
		debug_log_hint.text = "Decision log: " + (debug_log if on else "off")

func _ready() -> void:
	# Closing the window goes through _save_and_quit (FIXLOG 2026-11-13).
	get_tree().set_auto_accept_quit(false)
	_parse_args()
	assets = preload("res://scripts/assets.gd").new()
	persist = preload("res://scripts/persist.gd").new()
	persist.load_settings()
	feed_rate = clampi(int(persist.settings.get("feed_rate", 1)), 0, Rules.FEED_RATE_NAMES.size() - 1)
	if feed_rate_arg >= 0:
		feed_rate = feed_rate_arg
	name_tag_mode = clampi(int(persist.settings.get("name_tags", 1)), 0, Rules.NAME_TAG_NAMES.size() - 1)
	director_arg = director_arg and persist.settings["director"]
	sim = SimHost.new()
	add_child(sim)
	if sim_inline_arg:
		sim.set_threaded(false)
	print(sim.hello())
	_start_debug_log()
	var headless := shot_path != "" or footage_dir != ""
	var explicit_seed := false
	for a in OS.get_cmdline_user_args():
		if a.begins_with("--seed="): explicit_seed = true
	# The wizard-shot and opening flags exist to see the arrival, so they start fresh on purpose.
	var newest: String = "" if ((headless and not ring_arg) or stage != "" or fresh_arg or explicit_seed or wizard_shot_path != "" or opening_arg != "") else sim.newest_valid(persist.ring_base(), persist.RING)
	if stage == "demo":
		sim.stage_demo(seed_v)
	elif stage == "peoples":
		sim.stage_peoples(seed_v)
	elif stage == "walls":
		sim.stage_walls(seed_v)
	elif stage == "homes":
		sim.stage_homes(seed_v)
	elif opening_arg != "":
		var err: String = sim.opening(seed_v, [0, 1, 2], PackedInt64Array([]), opening_arg)
		print("opening %s -> %s" % [opening_arg, "ok" if err == "" else err])
		if err != "": _fresh_world(seed_v)
	elif load_arg != "":
		var err: String = sim.load_from(load_arg)
		print("LOAD %s -> %s" % [load_arg, "ok" if err == "" else err])
		if err != "": _fresh_world(seed_v)
	elif newest != "":
		var err: String = sim.load_from(newest)
		if err == "":
			started_from_save = true
			print("loaded %s" % newest)
		else:
			print("load failed (%s), starting fresh" % err)
			_fresh_world(seed_v)
			_note("autosave could not be read (%s); started a fresh valley" % err)
	else:
		# A fresh, hands-on start asks first: who begins, and who paces them, for whatever seed
		# was asked for (issue 36: any --seed used to skip the screen, so only seed 1 had one).
		# Every automated path (shots, footage, a windowless run, a run that only saves, staged
		# demo, --fresh, resume) goes straight to a world: nobody is there to press Begin.
		var automated := headless or DisplayServer.get_name() == "headless" or save_at_end != ""
		var hands_on := (wizard_shot_path != "" or not automated) and stage == "" and load_arg == "" and not fresh_arg
		if hands_on:
			# FIXLOG 2026-11-02 (#559): a New game is a fresh random valley. --seed alone (the
			# watch launcher passes one to skip the autosaves) no longer pins the wizard to it;
			# --fixed-seed does, and a wizard shot always uses it.
			var fixed := fixed_seed_arg or wizard_shot_path != ""
			_show_setup(seed_v if fixed else -1)
			return
		_fresh_world(seed_v)
		var had_saves := false
		for i in range(persist.RING):
			if FileAccess.file_exists(persist.ring_base() + ".%d.hfsave/manifest.json" % i):
				had_saves = true
		if had_saves and not fresh_arg and not explicit_seed and stage == "":
			_note("No autosave could be read; started a new valley. The old files are still in the saves folder.", 20.0)
			print("autosaves present but none valid; started fresh")
		elif not headless and stage == "":
			_note("A new valley. Watch, or press H for history and O for settings.", 20.0)
	if hungry_arg:
		print("HUNGRY %s" % sim.starve_neighbour())
	# A shot of someone inspected traces them from the start, so the Why? lens has cycles (#567).
	if inspect_arg >= 0:
		sim.set_why_target(inspect_arg)
	sim.step(steps)
	sim.sync()  # the step thread (#603): the world at that tick before going on
	_world_ready()

## A fresh world without the wizard (shots, footage, --fresh, a save that could not be read):
## the game's own start, the arrival through the portal paced steadily, not the old twelve-person
## test valley, which has no portal stone, no lost and no past (#104).
func _fresh_world(s: int) -> void:
	var err: String = sim.opening(s, [0, 1, 2], PackedInt64Array([]), "steady")
	if err != "":
		print("opening failed (%s), falling back to a generated world" % err)
		sim.generate(s, false)

# The setup screen, shown before the first world exists on a hands-on launch.
func _show_setup(seed: int) -> void:
	var script = preload("res://scripts/setup.gd")
	setup_panel = script.new()
	add_child(setup_panel)
	setup_panel.start(sim, seed)
	setup_panel.confirmed.connect(_on_setup_chosen)
	if wizard_shot_path != "":
		_shoot_and_begin()

# Flash the arrival screen to a PNG, then press Begin the way a player would.
func _shoot_and_begin() -> void:
	await get_tree().process_frame
	await get_tree().process_frame
	for pair in wizard_choices.split(",", false):
		var kv := pair.split("=")
		if kv.size() != 2:
			continue
		match kv[0]:
			"deity": setup_panel.deity = kv[1]
			"years": setup_panel.history_years = int(kv[1])
			"held":
				var tv := kv[1].split(":")
				setup_panel.held[tv[0]] = int(tv[1])
			"exclude": setup_panel.excluded[kv[1]] = true
			"rarity":
				var rv := kv[1].split(":")
				setup_panel.rarities[rv[0]] = rv[1]
	# Every page of the wizard, the way a player would step through it.
	for k in setup_panel.PAGES.size():
		setup_panel.goto(k)
		await get_tree().process_frame
		await get_tree().process_frame
		# The valley's picture is made off the main thread (#560): shoot it once it is there.
		var waited := 0
		while setup_panel.preview_pending() and waited < 600:
			await get_tree().process_frame
			waited += 1
		_save_viewport(wizard_shot_path.replace(".png", "_%d.png" % (k + 1)))
	if is_instance_valid(setup_panel):
		setup_panel._begin()
	await get_tree().process_frame
	await get_tree().process_frame
	_save_viewport(wizard_shot_path.replace(".png", "_started.png"))
	get_tree().quit()

func _save_viewport(path: String) -> void:
	var img := get_viewport().get_texture().get_image()
	if img == null:
		print("WIZARD_SHOT %s FAILED (no viewport image)" % path)
		return
	var err := img.save_png(path)
	print("WIZARD_SHOT %s err %d size %s" % [path, err, img.get_size()])

func _on_setup_chosen(setup: Dictionary) -> void:
	# Everything the wizard chose travels together: the deity, the depth of the past, the
	# valley, the three whose cards were on the screen (rerolls and all), and what they hold.
	seed_v = int(setup["seed"])
	var err: String = sim.begin(setup)
	if err != "":
		# Never start a different world than the one chosen (#147): back to the wizard, at the
		# page the trouble is on, with what went wrong.
		print("BEGIN REFUSED (%s)" % err)
		if is_instance_valid(setup_panel):
			setup_panel.refused(err)
		return
	if is_instance_valid(setup_panel):
		setup_panel.queue_free()
		setup_panel = null
	# The world's years before the portal play out first, then a click comes in (#561). The
	# wizard shot goes straight in: it photographs the arrival.
	if wizard_shot_path == "" and _show_history_reel():
		return
	_after_begin()

## The history reel over a new world (#561); false when the world has no history beyond the
## valley to show.
func _show_history_reel() -> bool:
	var reel = HistoryReel.new()
	add_child(reel)
	if not reel.start(sim):
		reel.queue_free()
		return false
	reel.finished.connect(func():
		reel.queue_free()
		_after_begin())
	return true

func _after_begin() -> void:
	if hungry_arg:
		print("HUNGRY %s" % sim.starve_neighbour())
	# A shot of someone inspected traces them from the start, so the Why? lens has cycles (#567).
	if inspect_arg >= 0:
		sim.set_why_target(inspect_arg)
	sim.step(steps)
	sim.sync()  # the step thread (#603): the world at that tick before going on
	_world_ready()

# Everything that needs a world to exist. Called once the world exists, however it was made.
func _world_ready() -> void:
	# --ignite=x,y (review shots): set the ground alight after the requested steps, then give
	# it a few in-game minutes so there is flame, smoke and someone reacting to photograph.
	if ignite_arg.x >= 0:
		print("IGNITE %s -> %s" % [ignite_arg, sim.ignite_at(ignite_arg.x, ignite_arg.y)])
		sim.step(ignite_after)
		sim.sync()  # the step thread (#603): the world at that tick before going on
	last_autosave_tick = sim.tick()
	view = preload("res://scripts/world_view.gd").new()
	add_child(view)
	view.setup(sim, assets)
	var fs: Vector3i = sim.settlement_centre()
	view.view_z = fs.z if z_arg == 9999 else z_arg
	view.refresh()
	cam = Camera2D.new()
	add_child(cam)
	var focus := Vector2(fs.x, fs.y)
	if focus_band:
		var B: PackedInt32Array = sim.bands()
		print("BANDS %s" % [B])
		if B.size() >= 9:
			focus_arg = Vector2i(B[1], B[2])
			# The camp's level too: on seed 11 it is a level above the landing (#204, QA
			# 2026-10-20, C63).
			if z_arg == 9999:
				view.view_z = clampi(B[3], view.zmin, view.zmax)
				view.refresh()
	if focus_arg.x >= 0:
		focus = Vector2(focus_arg.x, focus_arg.y)
	cam.position = focus * TILE + Vector2(16, 16)
	cam.zoom = Vector2(zoom_arg, zoom_arg)
	cam.make_current()
	# A shot honours --director too (#221: every --shot HUD read "director: off", so the
	# "watching ..." line could only be checked with --footage); --no-director still turns it off.
	director_on = director_arg
	why_open = why_arg
	if inspect_arg >= 0:
		selected_id = inspect_arg
		var p := _agent_pos(inspect_arg)
		if p.x >= 0: cam.position = p
	for fid in fav_arg:
		if not sim.is_favourite(fid):
			sim.toggle_favourite(fid)
	director_target = cam.position
	var headless_run := shot_path != "" or footage_dir != "" or DisplayServer.get_name() == "headless"
	hints_active = first_run_hints_arg or (not bool(persist.settings.get("hints_seen", false)) and not headless_run)
	if inspect_arg >= 0:
		hint_inspected_person = true
	if inspect_block_arg.x >= 0:
		hint_inspected_block = true
	_build_ui()
	_build_settings()
	_build_audio()
	get_viewport().gui_embed_subwindows = true
	# `centre` is where the people live now; `founding` the landing (#221: the centre was
	# printed as "founding", and an --at= placed from it landed wrong).
	print("world %s view_z %d centre %s founding %s missing_assets %s" % [sim.world_name(), view.view_z, fs, sim.founding_site(), assets.report_missing()])
	if known_arg:
		view.known_only = true; view.refresh()
	if rel_lines_arg:
		view.entities.relationships_visible = true; view.refresh()
	if history_arg:
		_show_history()
	if legends_arg != "":
		legends_text.set_meta("page", legends_arg)
		_show_legends(legends_arg)
	if inspect_block_arg.x >= 0:
		var bz: int = int(view.view_z) if inspect_block_arg.z == 9999 else inspect_block_arg.z
		_inspect_block(Vector3i(inspect_block_arg.x, inspect_block_arg.y, bz))
	if fav_panel_arg:
		_show_favourites()
	if people_panel_arg:
		_show_people()
	if waiting_panel_arg:
		_show_waiting()
	if bar_hidden_arg:
		_set_bar_visible(false)
	if persist.settings["timelapse"] and shot_path == "" and footage_dir == "" and stage == "" and DisplayServer.get_name() != "headless":
		_set_timelapse_recording(true)
	if shot_path != "":
		_do_shot()
	elif footage_dir != "":
		_do_footage()
	elif save_at_end != "":
		_do_save_and_quit()
	elif automation_json_path != "":
		get_tree().quit(0 if _write_automation_json() else 1)

# A loaded save replaced the world mid-session (F9). Reset what _world_ready set up for the
# first world and everything the client remembered about the old one (issue 97).
func _world_replaced() -> void:
	last_autosave_tick = sim.tick()
	focus_lost_tick = -1
	feed_lines.clear()
	feed_box.text = ""
	selected_id = -1
	follow_id = -1
	why_traced_id = -1
	why_name = ""
	hover_id = -1
	hover_cache_id = -2
	selected_block = Vector3i(-1, -1, -1)
	panels.close(&"summary")
	panels.close(&"block")
	panels.close(&"favourites")
	ending_shown = false
	view.reset_world()
	var fs: Vector3i = sim.settlement_centre()
	view.view_z = clampi(fs.z, view.zmin, view.zmax)
	cam.position = Vector2(fs.x, fs.y) * TILE + Vector2(16, 16)
	director_target = cam.position
	director_next_check = 0.0
	timelapse.clear()
	timelapse_series_dir = ""
	timelapse_identity = ""
	timelapse_watch = {}
	timelapse_next_tick = -1
	view.refresh()
	if timelapse_recording:
		_set_timelapse_recording(true)
	_sync_timelapse_panel()

func _notification(what: int) -> void:
	if what == NOTIFICATION_WM_CLOSE_REQUEST:
		_save_and_quit()
	elif what == NOTIFICATION_APPLICATION_FOCUS_OUT or what == NOTIFICATION_WM_WINDOW_FOCUS_OUT:
		if window_focused:
			window_focused = false
			focus_lost_tick = sim.tick()
	elif what == NOTIFICATION_APPLICATION_FOCUS_IN or what == NOTIFICATION_WM_WINDOW_FOCUS_IN:
		if not window_focused:
			window_focused = true
			if focus_lost_tick >= 0 and sim.tick() - focus_lost_tick > AWAY_TICKS:
				_show_summary(focus_lost_tick)

var _quitting := false

## Closing the window saves the valley first (FIXLOG 2026-11-13): wait for a save already running,
## write one more into the autosave ring if the world moved since the last one (Rules.save_on_close),
## wait for its write and save lock to finish, then quit. Bounded, so a stuck write cannot hold the
## window open forever.
func _save_and_quit() -> void:
	if _quitting:
		return
	_quitting = true
	var review_run := shot_path != "" or footage_dir != "" or stage != "" or wizard_shot_path != ""
	var headless := DisplayServer.get_name() == "headless"
	if sim != null and Rules.save_on_close(is_instance_valid(view), review_run, headless, ring_arg,
			float(persist.settings["autosave_days"]), sim.tick(), last_autosave_tick):
		var deadline := Time.get_ticks_msec() + 30000
		while sim.save_status() == "saving" and Time.get_ticks_msec() < deadline:
			OS.delay_msec(20)
		if sim.save_to(sim.next_ring_slot(persist.ring_base(), persist.RING)):
			last_autosave_tick = sim.tick()
			while sim.save_status() == "saving" and Time.get_ticks_msec() < deadline:
				OS.delay_msec(20)
			print("SAVED ON CLOSE tick %d" % last_autosave_tick)
	get_tree().quit()

# ---------------------------------------------------------------- UI
## One place for how overlay text looks, so the HUD, feed, inspector and panels agree.
func _style_text(c: Control, size: int) -> void:
	var key := "normal_font_size" if c is RichTextLabel else "font_size"
	c.add_theme_font_size_override(key, size)
	c.add_theme_color_override("font_shadow_color", Color(0, 0, 0, 0.9))
	c.add_theme_constant_override("shadow_offset_x", 1)
	c.add_theme_constant_override("shadow_offset_y", 1)

## Puts Mouse & Keys at the left edge under the HUD, as tall as its text and never over the
## bottom bar (Rules.help_rect); the HUD grows a line for strangers or troubles, so every frame
## while it is open (FIXLOG 2026-11-13, #797 finding 1).
func _place_help() -> void:
	var bg := help_panel.get_theme_stylebox("panel")
	var sep := (help_hdr.get_parent() as VBoxContainer).get_theme_constant("separation")
	var chrome := help_hdr.get_combined_minimum_size().y + sep + bg.get_minimum_size().y
	var r := Rules.help_rect(get_viewport().get_visible_rect().size, hud.position.y + hud.size.y,
		chrome + help_text.get_content_height())
	help_text.custom_minimum_size.y = r.size.y - chrome
	help_panel.position = r.position
	help_panel.size = r.size

## Every panel uses the same dimmed backing, so a popup reads as part of the same interface.
func _panel_bg() -> StyleBoxFlat:
	var bg := StyleBoxFlat.new()
	bg.bg_color = Color(0.05, 0.05, 0.07, 0.82)
	bg.content_margin_left = 10; bg.content_margin_right = 10
	bg.content_margin_top = 8; bg.content_margin_bottom = 8
	return bg

func _make_btn(label: String, tip: String, cb: Callable) -> Button:
	var b := Button.new()
	b.text = label
	b.tooltip_text = tip
	b.focus_mode = Control.FOCUS_NONE
	_style_text(b, 12)
	b.pressed.connect(cb)
	return b

func _build_ui() -> void:
	var layer := CanvasLayer.new()
	layer.layer = 2  # above the light and colour grade (layer 1)
	add_child(layer)
	var font_size := 14
	hud = Label.new()
	hud.position = Vector2(10, 6)
	# Wrapped to a width that keeps clear of the inspect panel (#181, QA 2026-10-20, C63).
	hud.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	_style_text(hud, font_size)
	hud.add_theme_stylebox_override("normal", _panel_bg())
	layer.add_child(hud)
	feed_box = RichTextLabel.new()
	feed_box.bbcode_enabled = true
	feed_box.scroll_active = false
	feed_box.fit_content = true
	_style_text(feed_box, 13)
	feed_box.add_theme_stylebox_override("normal", _panel_bg())
	# Sized to its lines, growing up from the bottom: a fixed 220 px block covered the stores
	# with one line in it (#182, QA 2026-10-20, C63). A click on a line looks where it happened.
	feed_box.set_anchors_preset(Control.PRESET_BOTTOM_LEFT)
	feed_box.anchor_top = 1.0
	feed_box.offset_left = 10
	feed_box.offset_top = -54
	feed_box.offset_right = 10 + Rules.FEED_W
	feed_box.offset_bottom = -54
	feed_box.grow_vertical = Control.GROW_DIRECTION_BEGIN
	feed_box.mouse_filter = Control.MOUSE_FILTER_STOP
	feed_box.gui_input.connect(_on_feed_input)
	layer.add_child(feed_box)

	# Inspect panel (#413): person or animal details with clickable Follow, Favourite, Go to, Close.
	inspect_panel = PanelContainer.new()
	inspect_panel.set_anchors_preset(Control.PRESET_TOP_RIGHT)
	inspect_panel.anchor_left = 1.0
	inspect_panel.anchor_right = 1.0
	inspect_panel.offset_left = -340
	inspect_panel.offset_top = 36
	inspect_panel.offset_right = -10
	inspect_panel.anchor_bottom = 1.0
	inspect_panel.offset_bottom = -54.0
	inspect_panel.visible = false
	inspect_panel.add_theme_stylebox_override("panel", _panel_bg())
	var insp_vbox := VBoxContainer.new()
	insp_vbox.add_theme_constant_override("separation", 6)
	inspect_panel.add_child(insp_vbox)
	var insp_btns := HFlowContainer.new()
	insp_btns.add_theme_constant_override("h_separation", 6); insp_btns.add_theme_constant_override("v_separation", 4)
	insp_vbox.add_child(insp_btns)
	inspect_follow_btn = _make_btn("Follow [F]", "Keep the camera on this person [F]", func():
		if selected_id >= 0:
			follow_id = -1 if follow_id == selected_id else selected_id
			_update_inspect_buttons()
			_fx("ui_click")
	)
	insp_btns.add_child(inspect_follow_btn)
	inspect_fav_btn = _make_btn("☆ Favourite", "Mark or unmark this person as a favourite to watch", func():
		if selected_id >= 0:
			sim.toggle_favourite(selected_id)
			_update_inspect_buttons()
			if panels.is_open(&"favourites"):
				_refresh_favourites()
			_fx("ui_click")
	)
	insp_btns.add_child(inspect_fav_btn)
	why_btn = _make_btn("Why?", "Show why this person is doing what they do: the options they weighed and what they put off", func():
		why_open = not why_open
		inspect_text = ""
		_update_inspect()
		_fx("ui_click")
	)
	insp_btns.add_child(why_btn)
	insp_btns.add_child(_make_btn("Go to", "Centre the camera on this person now", func():
		if selected_id >= 0:
			var p := _agent_pos(selected_id)
			if p.x >= 0:
				cam.position = p
				last_input_time = Time.get_ticks_msec() / 1000.0
			_fx("ui_click")
	))
	var insp_spacer := Control.new()
	insp_spacer.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	insp_btns.add_child(insp_spacer)
	insp_btns.add_child(_make_btn("Close [Esc]", "Close the inspect panel [Esc]", func():
		selected_id = -1
		follow_id = -1
		panels.close(&"inspect")
		_fx("ui_click")
	))
	portrait = preload("res://scripts/portrait.gd").new()
	portrait.assets = assets
	insp_vbox.add_child(portrait)
	inspect_box = RichTextLabel.new()
	inspect_box.bbcode_enabled = true
	_style_text(inspect_box, 13)
	inspect_box.size_flags_vertical = Control.SIZE_EXPAND_FILL
	inspect_box.scroll_active = true
	insp_vbox.add_child(inspect_box)
	layer.add_child(inspect_panel)

	# Block inspection panel (#413): ground/rock & material, water kind & depth, minerals & metals, etc.
	block_panel = PanelContainer.new()
	block_panel.set_anchors_preset(Control.PRESET_TOP_RIGHT)
	block_panel.anchor_left = 1.0
	block_panel.anchor_right = 1.0
	block_panel.offset_left = -350
	block_panel.offset_top = 36
	block_panel.offset_right = -10
	block_panel.offset_bottom = 520
	block_panel.visible = false
	block_panel.add_theme_stylebox_override("panel", _panel_bg())
	var blk_vbox := VBoxContainer.new()
	blk_vbox.add_theme_constant_override("separation", 6)
	block_panel.add_child(blk_vbox)
	var blk_hdr := HBoxContainer.new()
	blk_vbox.add_child(blk_hdr)
	var blk_title := Label.new()
	blk_title.text = "Block inspection"
	_style_text(blk_title, 14)
	blk_title.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	blk_hdr.add_child(blk_title)
	blk_hdr.add_child(_make_btn("Go to", "Centre the camera on this block", func():
		if selected_block.x >= 0:
			follow_id = -1
			cam.position = Vector2(selected_block.x, selected_block.y) * TILE + Vector2(16, 16)
			if selected_block.z != view.view_z:
				view.view_z = clampi(selected_block.z, view.zmin, view.zmax)
				view.refresh()
			_fx("ui_click")
	))
	blk_hdr.add_child(_make_btn("Close [Esc]", "Close block inspection [Esc]", func():
		selected_block = Vector3i(-1, -1, -1)
		panels.close(&"block")
		_fx("ui_click")
	))
	block_box = RichTextLabel.new()
	block_box.bbcode_enabled = true
	_style_text(block_box, 13)
	block_box.size_flags_vertical = Control.SIZE_EXPAND_FILL
	block_box.scroll_active = true
	blk_vbox.add_child(block_box)
	layer.add_child(block_panel)

	# Favourites panel (#413): lists everyone favourited with live status, location, Go to & Follow.
	fav_panel = PanelContainer.new()
	fav_panel.set_anchors_preset(Control.PRESET_CENTER)
	fav_panel.offset_left = -360; fav_panel.offset_right = 360
	fav_panel.offset_top = -220; fav_panel.offset_bottom = 220
	fav_panel.add_theme_stylebox_override("panel", _panel_bg())
	fav_panel.visible = false
	var fav_vbox := VBoxContainer.new()
	fav_vbox.add_theme_constant_override("separation", 8)
	fav_panel.add_child(fav_vbox)
	var fav_hdr := HBoxContainer.new()
	fav_vbox.add_child(fav_hdr)
	var fav_title := Label.new()
	fav_title.text = "★ Favourites   (click a person to inspect & star them)"
	_style_text(fav_title, 14)
	fav_title.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	fav_hdr.add_child(fav_title)
	fav_hdr.add_child(_make_btn("Close [Esc]", "Close favourites [J or Esc]", func():
		panels.close(&"favourites")
		_fx("ui_click")
	))
	var fav_scroll := ScrollContainer.new()
	fav_scroll.size_flags_vertical = Control.SIZE_EXPAND_FILL
	fav_vbox.add_child(fav_scroll)
	fav_list_box = VBoxContainer.new()
	fav_list_box.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	fav_list_box.add_theme_constant_override("separation", 6)
	fav_scroll.add_child(fav_list_box)
	fav_empty_label = Label.new()
	fav_empty_label.text = "No favourites yet. Click anyone in the valley and press '☆ Favourite' on their panel to keep track of them here."
	fav_empty_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	_style_text(fav_empty_label, 13)
	fav_list_box.add_child(fav_empty_label)
	layer.add_child(fav_panel)

	# People panel (#351): everyone of the valley's communities, what they are doing and why.
	people_panel = PanelContainer.new()
	people_panel.set_anchors_preset(Control.PRESET_CENTER)
	people_panel.offset_left = -420; people_panel.offset_right = 420
	people_panel.offset_top = -280; people_panel.offset_bottom = 280
	people_panel.add_theme_stylebox_override("panel", _panel_bg())
	people_panel.visible = false
	var ppl_vbox := VBoxContainer.new()
	ppl_vbox.add_theme_constant_override("separation", 8)
	people_panel.add_child(ppl_vbox)
	var ppl_hdr := HBoxContainer.new()
	ppl_vbox.add_child(ppl_hdr)
	people_title = Label.new()
	people_title.text = "People"
	_style_text(people_title, 14)
	people_title.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	ppl_hdr.add_child(people_title)
	ppl_hdr.add_child(_make_btn("Close [Esc]", "Close people [P or Esc]", func():
		panels.close(&"people")
		_fx("ui_click")
	))
	var ppl_scroll := ScrollContainer.new()
	ppl_scroll.size_flags_vertical = Control.SIZE_EXPAND_FILL
	ppl_vbox.add_child(ppl_scroll)
	people_list_box = VBoxContainer.new()
	people_list_box.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	people_list_box.add_theme_constant_override("separation", 6)
	ppl_scroll.add_child(people_list_box)
	people_empty_label = Label.new()
	people_empty_label.text = "Nobody lives in the valley."
	_style_text(people_empty_label, 13)
	people_list_box.add_child(people_empty_label)
	layer.add_child(people_panel)
	# Waiting work (#292, WP-08 8.6): every order nobody has taken for an hour or more, and why.
	waiting_panel = PanelContainer.new()
	waiting_panel.set_anchors_preset(Control.PRESET_CENTER)
	waiting_panel.offset_left = -400; waiting_panel.offset_right = 400
	waiting_panel.offset_top = -240; waiting_panel.offset_bottom = 240
	waiting_panel.add_theme_stylebox_override("panel", _panel_bg())
	waiting_panel.visible = false
	var wt_vbox := VBoxContainer.new()
	wt_vbox.add_theme_constant_override("separation", 8)
	waiting_panel.add_child(wt_vbox)
	var wt_hdr := HBoxContainer.new()
	wt_vbox.add_child(wt_hdr)
	var wt_title := Label.new()
	wt_title.text = "Waiting work: what nobody has taken, and why"
	_style_text(wt_title, 14)
	wt_title.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	wt_hdr.add_child(wt_title)
	wt_hdr.add_child(_make_btn("Close [Esc]", "Close waiting work [K or Esc]", func():
		panels.close(&"waiting")
		_fx("ui_click")
	))
	var wt_scroll := ScrollContainer.new()
	wt_scroll.size_flags_vertical = Control.SIZE_EXPAND_FILL
	wt_vbox.add_child(wt_scroll)
	waiting_list_box = VBoxContainer.new()
	waiting_list_box.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	waiting_list_box.add_theme_constant_override("separation", 6)
	wt_scroll.add_child(waiting_list_box)
	layer.add_child(waiting_panel)

	# At the left edge under the HUD, as tall as its text (_place_help): in the middle it covered
	# the founding spot on a first start, and its keys were cut off (FIXLOG 2026-11-13, #797).
	help_panel = PanelContainer.new()
	help_panel.set_anchors_preset(Control.PRESET_TOP_LEFT)
	help_panel.add_theme_stylebox_override("panel", _panel_bg())
	var help_vbox := VBoxContainer.new()
	help_panel.add_child(help_vbox)
	help_hdr = HBoxContainer.new()
	help_vbox.add_child(help_hdr)
	var help_sp := Control.new()
	help_sp.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	help_hdr.add_child(help_sp)
	help_hdr.add_child(_make_btn("Close [Esc]", "Close help [? or Esc]", func():
		panels.close(&"help")
		_fx("ui_click")
	))
	help_text = RichTextLabel.new()
	help_text.bbcode_enabled = true
	help_text.text = HELP_TEXT
	help_text.size_flags_vertical = Control.SIZE_EXPAND_FILL
	_style_text(help_text, 14)
	help_vbox.add_child(help_text)
	# Once, on a hands-on first launch; then on [?].
	help_panel.visible = not persist.settings["help_seen"] and shot_path == "" and footage_dir == "" and DisplayServer.get_name() != "headless"
	if help_panel.visible:
		persist.settings["help_seen"] = true
		persist.save_settings()
	layer.add_child(help_panel)
	_place_help()

	summary_panel = PanelContainer.new()
	summary_panel.set_anchors_preset(Control.PRESET_CENTER)
	summary_panel.offset_left = -420; summary_panel.offset_right = 420
	summary_panel.offset_top = -300; summary_panel.offset_bottom = 300
	summary_panel.add_theme_stylebox_override("panel", _panel_bg())
	summary_panel.visible = false
	var sum_vbox := VBoxContainer.new()
	summary_panel.add_child(sum_vbox)
	var sum_hdr := HBoxContainer.new()
	sum_vbox.add_child(sum_hdr)
	var sum_sp := Control.new()
	sum_sp.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	sum_hdr.add_child(sum_sp)
	sum_hdr.add_child(_make_btn("Close [Esc]", "Close history / summary [H or Esc]", func():
		panels.close(&"summary")
		_fx("ui_click")
	))
	summary_box = RichTextLabel.new()
	summary_box.bbcode_enabled = true
	summary_box.size_flags_vertical = Control.SIZE_EXPAND_FILL
	_style_text(summary_box, 14)
	sum_vbox.add_child(summary_box)
	layer.add_child(summary_panel)

	legends_panel = PanelContainer.new()
	legends_panel.set_anchors_preset(Control.PRESET_CENTER)
	legends_panel.offset_left = -600; legends_panel.offset_right = 600
	legends_panel.offset_top = -340; legends_panel.offset_bottom = 340
	legends_panel.add_theme_stylebox_override("panel", _panel_bg())
	legends_panel.visible = false
	var leg_outer := VBoxContainer.new()
	legends_panel.add_child(leg_outer)
	var leg_hdr := HBoxContainer.new()
	leg_hdr.add_theme_constant_override("separation", 8)
	leg_outer.add_child(leg_hdr)
	leg_hdr.add_child(_make_btn("← Back [Backspace]", "Return to the previous Legends page [Backspace]", func():
		if legends_back.size() > 0:
			var prev: String = legends_back.pop_back()
			legends_text.set_meta("page", prev)
			_show_legends(prev)
			_fx("ui_click")
	))
	leg_hdr.add_child(_make_btn("World overview", "Open the top-level world page", func():
		legends_back.append(legends_text.get_meta("page", "world:0"))
		legends_text.set_meta("page", "world:0")
		_show_legends("world:0")
		_fx("ui_click")
	))
	var leg_sp := Control.new()
	leg_sp.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	leg_hdr.add_child(leg_sp)
	leg_hdr.add_child(_make_btn("Close [Esc]", "Close Legends [L or Esc]", func():
		panels.close(&"legends")
		_fx("ui_click")
	))
	var row := HBoxContainer.new()
	row.add_theme_constant_override("separation", 14)
	row.size_flags_vertical = Control.SIZE_EXPAND_FILL
	leg_outer.add_child(row)
	var left := VBoxContainer.new()
	row.add_child(left)
	legends_map = TextureRect.new()
	legends_map.custom_minimum_size = Vector2(384, 384)
	legends_map.texture_filter = CanvasItem.TEXTURE_FILTER_NEAREST
	legends_map.stretch_mode = TextureRect.STRETCH_SCALE
	left.add_child(legends_map)
	legends_year = HSlider.new()
	legends_year.custom_minimum_size = Vector2(384, 20)
	legends_year.value_changed.connect(_on_legends_year)
	left.add_child(legends_year)
	legends_year_label = Label.new()
	_style_text(legends_year_label, 13)
	left.add_child(legends_year_label)
	legends_away = RichTextLabel.new()
	legends_away.bbcode_enabled = true
	legends_away.fit_content = true
	legends_away.custom_minimum_size = Vector2(384, 0)
	legends_away.size_flags_vertical = Control.SIZE_EXPAND_FILL
	_style_text(legends_away, 12)
	left.add_child(legends_away)
	legends_text = RichTextLabel.new()
	legends_text.bbcode_enabled = true
	legends_text.custom_minimum_size = Vector2(760, 590)
	legends_text.meta_clicked.connect(_on_legends_link)
	_style_text(legends_text, 14)
	row.add_child(legends_text)
	layer.add_child(legends_panel)

	timelapse_panel = TimelapsePanel.new()
	timelapse_panel.visible = false
	layer.add_child(timelapse_panel)
	timelapse_panel.recording_changed.connect(_set_timelapse_recording)
	timelapse_panel.use_current_requested.connect(_use_current_timelapse_view)
	timelapse_panel.valley_requested.connect(_use_valley_timelapse_view)
	timelapse_panel.close_requested.connect(func(): panels.close(&"timelapse"))

	_build_bottom_bar(layer)

	# Registered after construction, so each is taken as it stands: the help panel
	# may already be showing on a first launch (#351).
	panels.register(&"inspect", inspect_panel)
	panels.register(&"block", block_panel)
	# The centred reading panels, one at a time (D92): a second over the first showed its rows
	# through the backing. The inspect, block and help panels sit at the edges, beside them.
	panels.register(&"favourites", fav_panel, &"reading")
	panels.register(&"people", people_panel, &"reading")
	panels.register(&"waiting", waiting_panel, &"reading")
	panels.register(&"help", help_panel)
	panels.register(&"summary", summary_panel, &"reading")
	panels.register(&"legends", legends_panel, &"reading")
	panels.register(&"timelapse", timelapse_panel, &"reading")
	# Its own layer above the panels, so a toast is never hidden behind one.
	toaster = Toaster.new()
	add_child(toaster)
	toaster.looked.connect(_on_toast_looked)
	_load_announce()

## Fill and open the Waiting work panel from the bridge's stuck_orders (#292, WP-08 8.6).
func _show_waiting() -> void:
	for c in waiting_list_box.get_children():
		c.queue_free()
	var rows: Array = sim.stuck_orders()
	if rows.is_empty():
		var empty := Label.new()
		empty.text = "Nothing is waiting: every order has been taken, or was posted within the hour."
		empty.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		_style_text(empty, 13)
		waiting_list_box.add_child(empty)
	for d in rows:
		var row := HBoxContainer.new()
		var l := Label.new()
		l.text = Rules.format_stuck_row(d)
		l.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		l.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		_style_text(l, 13)
		row.add_child(l)
		var p := Vector3i(int(d["x"]), int(d["y"]), int(d["z"]))
		row.add_child(_make_btn("Go to", "Look where it waits", func(): _on_toast_looked(p)))
		waiting_list_box.add_child(row)
	panels.open(&"waiting")
	_fx("ui_click")

func _build_bottom_bar(layer: CanvasLayer) -> void:
	bar_panel = PanelContainer.new()
	bar_panel.set_anchors_preset(Control.PRESET_BOTTOM_WIDE)
	bar_panel.anchor_top = 1.0
	bar_panel.offset_left = 10
	bar_panel.offset_right = -90
	bar_panel.offset_top = -44
	bar_panel.offset_bottom = -6
	bar_panel.add_theme_stylebox_override("panel", _panel_bg())
	var row := HBoxContainer.new()
	row.add_theme_constant_override("separation", 4)
	bar_panel.add_child(row)
	var last_group := ""
	for item in Rules.bar_items():
		var grp: String = item["group"]
		if last_group != "" and grp != last_group:
			var sep := VSeparator.new()
			row.add_child(sep)
		last_group = grp
		var id: String = item["id"]
		var b := _make_btn(item["label"], item["tip"], func(): _on_bar_pressed(id))
		row.add_child(b)
		bar_buttons[id] = b
	layer.add_child(bar_panel)

	# Hide / show button for the bottom bar (#413 A1).
	bar_toggle_btn = _make_btn("▼ Bar", "Hide or show the bottom button bar [B]", func():
		_set_bar_visible(not bar_panel.visible)
		_fx("ui_click")
	)
	bar_toggle_btn.set_anchors_preset(Control.PRESET_BOTTOM_RIGHT)
	bar_toggle_btn.anchor_left = 1.0
	bar_toggle_btn.anchor_top = 1.0
	bar_toggle_btn.offset_left = -82
	bar_toggle_btn.offset_right = -10
	bar_toggle_btn.offset_top = -40
	bar_toggle_btn.offset_bottom = -8
	layer.add_child(bar_toggle_btn)

	# The bar's Load asks before the running valley is replaced (D94).
	load_popup = PanelContainer.new()
	load_popup.set_anchors_preset(Control.PRESET_CENTER)
	load_popup.offset_left = -260; load_popup.offset_right = 260
	load_popup.offset_top = -50; load_popup.offset_bottom = 50
	var lp_bg: StyleBox = _panel_bg().duplicate()
	(lp_bg as StyleBoxFlat).bg_color.a = 1.0
	load_popup.add_theme_stylebox_override("panel", lp_bg)
	load_popup.visible = false
	var lp_vbox := VBoxContainer.new()
	lp_vbox.add_theme_constant_override("separation", 10)
	load_popup.add_child(lp_vbox)
	load_question_label = Label.new()
	load_question_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	_style_text(load_question_label, 15)
	lp_vbox.add_child(load_question_label)
	var lp_btns := HBoxContainer.new()
	lp_btns.alignment = BoxContainer.ALIGNMENT_END
	lp_btns.add_theme_constant_override("separation", 10)
	lp_vbox.add_child(lp_btns)
	lp_btns.add_child(_make_btn("Keep playing [Esc]", "Keep the valley as it is", func():
		load_popup.visible = false
		_fx("ui_click")
	))
	lp_btns.add_child(_make_btn("Load", "Load the saved valley", _do_load_manual))
	layer.add_child(load_popup)

	# Bookmarks popup for F1–F8 jump & set with the mouse (#413 A1).
	bookmarks_popup = PanelContainer.new()
	bookmarks_popup.set_anchors_preset(Control.PRESET_CENTER_BOTTOM)
	bookmarks_popup.anchor_top = 1.0
	bookmarks_popup.offset_left = -210; bookmarks_popup.offset_right = 210
	bookmarks_popup.offset_top = -220; bookmarks_popup.offset_bottom = -50
	bookmarks_popup.add_theme_stylebox_override("panel", _panel_bg())
	bookmarks_popup.visible = false
	var bm_vbox := VBoxContainer.new()
	bm_vbox.add_theme_constant_override("separation", 4)
	bookmarks_popup.add_child(bm_vbox)
	var bm_hdr := HBoxContainer.new()
	bm_vbox.add_child(bm_hdr)
	var bm_title := Label.new()
	bm_title.text = "Camera bookmarks (F1–F8)"
	_style_text(bm_title, 13)
	bm_title.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	bm_hdr.add_child(bm_title)
	bm_hdr.add_child(_make_btn("Close", "Close bookmarks", func():
		bookmarks_popup.visible = false
		_fx("ui_click")
	))
	var bm_grid := GridContainer.new()
	bm_grid.columns = 4
	bm_grid.add_theme_constant_override("h_separation", 8)
	bm_grid.add_theme_constant_override("v_separation", 4)
	bm_vbox.add_child(bm_grid)
	for n in [1, 2, 3, 4, 6, 7, 8]:
		var cell := HBoxContainer.new()
		cell.add_theme_constant_override("separation", 2)
		bm_grid.add_child(cell)
		cell.add_child(_make_btn("Go F%d" % n, "Jump to bookmark F%d [F%d]" % [n, n], func():
			if persist.bookmarks.has(n):
				var b = persist.bookmarks[n]
				cam.position = b["pos"]; cam.zoom = Vector2(b["zoom"], b["zoom"])
				view.view_z = b["z"]; view.refresh()
				last_input_time = Time.get_ticks_msec() / 1000.0; follow_id = -1
				_fx("ui_click")
			else:
				_note("bookmark %d not set yet (click Set)" % n)
		))
		cell.add_child(_make_btn("Set", "Save current camera to bookmark F%d [Ctrl+F%d]" % [n, n], func():
			persist.bookmarks[n] = {"pos": cam.position, "z": view.view_z, "zoom": cam.zoom.x}
			persist.save_settings(); _note("bookmark %d set" % n)
			_fx("ui_click")
		))
	layer.add_child(bookmarks_popup)

func _set_bar_visible(v: bool) -> void:
	if bar_panel == null:
		return
	bar_panel.visible = v
	if not v and bookmarks_popup != null:
		bookmarks_popup.visible = false
	if bar_toggle_btn != null:
		bar_toggle_btn.text = "▼ Bar" if v else "▲ Bar"
	if feed_box != null:
		feed_box.offset_bottom = -54 if v else -10
		feed_box.offset_top = feed_box.offset_bottom - feed_box.get_content_height() - 16

func _record_hint_progress() -> void:
	if not hints_active:
		return
	if Rules.first_run_hint(hint_inspected_person, hint_inspected_block, hint_used_bar) == "":
		hints_active = false
		persist.settings["hints_seen"] = true
		persist.save_settings()

func _set_feed_rate(mode: int) -> void:
	feed_rate = clampi(mode, 0, Rules.FEED_RATE_NAMES.size() - 1)
	persist.settings["feed_rate"] = feed_rate
	persist.save_settings()
	if feed_rate_btn != null:
		feed_rate_btn.text = "Feed rate: " + Rules.feed_rate_name(feed_rate)
	_update_bar_buttons()

## The name-tag layer follows the setting and who is followed or inspected (#1428).
func _sync_name_tags() -> void:
	var t = view.name_tags
	# The hovered person is named beside them (#1429); the HUD line stays for #294's readers.
	var hov := hover_id if hover_id >= 0 and hover_id != selected_id and hover_cache_id == hover_id else -1
	var htxt := hover_tag_text if hov >= 0 else ""
	if t.mode != name_tag_mode or t.followed != follow_id or t.selected != selected_id \
			or t.hover_id != hov or t.hover_text != htxt:
		t.mode = name_tag_mode
		t.followed = follow_id
		t.selected = selected_id
		t.hover_id = hov
		t.hover_text = htxt
		t.queue_redraw()

func _cycle_name_tags() -> void:
	name_tag_mode = (name_tag_mode + 1) % Rules.NAME_TAG_NAMES.size()
	persist.settings["name_tags"] = name_tag_mode
	persist.save_settings()
	if name_tag_btn != null:
		name_tag_btn.text = "Name tags: " + Rules.name_tag_name(name_tag_mode)
	_note("name tags: " + Rules.name_tag_name(name_tag_mode))
	_fx("ui_click")

func _cycle_feed_rate() -> void:
	_set_feed_rate((feed_rate + 1) % Rules.FEED_RATE_NAMES.size())
	_note("feed rate: " + Rules.feed_rate_name(feed_rate))
	_fx("ui_click")

func _on_bar_pressed(id: String) -> void:
	var now := Time.get_ticks_msec() / 1000.0
	last_input_time = now
	hint_used_bar = true
	_record_hint_progress()
	match id:
		"pause":
			speed_idx = 1 if speed_idx == 0 else 0
			_fx("ui_click")
		"spd1":
			speed_idx = 1; _fx("ui_click")
		"spd2":
			speed_idx = 2; _fx("ui_click")
		"spd3":
			speed_idx = 3; _fx("ui_click")
		"z_down":
			follow_id = -1
			view.view_z = maxi(view.view_z - 1, view.zmin); view.refresh(); _fx("ui_click")
		"z_up":
			follow_id = -1
			view.view_z = mini(view.view_z + 1, view.zmax); view.refresh(); _fx("ui_click")
		"zoom_out":
			var old := cam.zoom.x
			var nz: float = Rules.next_zoom(old, -1)
			cam.zoom = Vector2(nz, nz); _fx("ui_click")
		"zoom_in":
			var old := cam.zoom.x
			var nz: float = Rules.next_zoom(old, 1)
			cam.zoom = Vector2(nz, nz); _fx("ui_click")
		"home":
			_go_home(); _fx("ui_click")
		"director":
			_on_director_toggled(not director_on); _fx("ui_click")
		"favs":
			if panels.is_open(&"favourites"): panels.close(&"favourites"); _fx("ui_click")
			else: _show_favourites()
		"people":
			if panels.is_open(&"people"): panels.close(&"people"); _fx("ui_click")
			else: _show_people()
		"waiting":
			if panels.is_open(&"waiting"): panels.close(&"waiting"); _fx("ui_click")
			else: _show_waiting()
		"history":
			if panels.is_open(&"summary"): panels.close(&"summary"); _fx("ui_click")
			else: _show_history()
		"timelapse":
			panels.toggle(&"timelapse")
			_sync_timelapse_panel()
			_fx("ui_click")
		"legends":
			if panels.is_open(&"legends"): panels.close(&"legends"); _fx("ui_click")
			else:
				legends_back.clear()
				legends_text.set_meta("page", "world:0")
				_show_legends("world:0")
		"known":
			view.known_only = not view.known_only
			view.refresh()
			_note("view: %s" % ("what the community has seen" if view.known_only else "everything"))
			_fx("ui_click")
		"feed_rate":
			_cycle_feed_rate()
		"announce":
			announce_mode = (announce_mode + 1) % Rules.ANNOUNCE_NAMES.size()
			_note("announce: " + Rules.ANNOUNCE_NAMES[announce_mode])
			_save_announce(); _fx("ui_click")
		"pause_major":
			pause_on_major = not pause_on_major
			_note("pause on major events: " + ("on" if pause_on_major else "off"))
			_save_announce(); _fx("ui_click")
		"bookmarks":
			bookmarks_popup.visible = not bookmarks_popup.visible
			_fx("ui_click")
		"save":
			if sim.save_to(persist.manual_path()): _note("saving…"); _fx("ui_chime")
		"load":
			_load_manual(true)
		"settings":
			settings_panel.visible = not settings_panel.visible
			_fx("ui_click")
		"help":
			panels.toggle(&"help")
			_fx("ui_click")
	_update_bar_buttons()

func _go_home() -> void:
	follow_id = -1
	last_input_time = Time.get_ticks_msec() / 1000.0
	var fs: Vector3i = sim.settlement_centre()
	cam.position = Vector2(fs.x, fs.y) * TILE + Vector2(16, 16)
	if fs.z != view.view_z:
		view.view_z = clampi(fs.z, view.zmin, view.zmax)
		view.refresh()

func _update_bar_buttons() -> void:
	if bar_buttons.is_empty():
		return
	bar_buttons["pause"].text = "▶ Play" if speed_idx == 0 else "⏸ Pause"
	bar_buttons["spd1"].text = "[1×]" if speed_idx == 1 else "1×"
	bar_buttons["spd2"].text = "[3×]" if speed_idx == 2 else "3×"
	bar_buttons["spd3"].text = "[10×]" if speed_idx == 3 else "10×"
	bar_buttons["director"].text = "Director: on" if director_on else "Director: off"
	bar_buttons["known"].text = "Known: on" if (view != null and view.known_only) else "Known: all"
	if bar_buttons.has("feed_rate"):
		bar_buttons["feed_rate"].text = "Feed: " + Rules.feed_rate_name(feed_rate)
	bar_buttons["announce"].text = "Alerts: " + Rules.ANNOUNCE_NAMES[announce_mode]
	bar_buttons["pause_major"].text = "Pause!: on" if pause_on_major else "Pause!: off"
	var fav_cnt := sim.favourites().size() if sim != null else 0
	bar_buttons["favs"].text = ("★ Favs (%d)" % fav_cnt) if fav_cnt > 0 else "★ Favs"

func _update_inspect_buttons() -> void:
	if inspect_follow_btn == null or inspect_fav_btn == null or selected_id < 0:
		return
	var following := follow_id == selected_id
	inspect_follow_btn.text = "Following [F]" if following else "Follow [F]"
	if why_btn != null:
		why_btn.text = "Why? (on)" if why_open else "Why?"
	var is_fav := sim.is_favourite(selected_id)
	inspect_fav_btn.text = "★ Favourited" if is_fav else "☆ Favourite"

func _inspect_block(p: Vector3i) -> void:
	selected_block = p
	hint_inspected_block = true
	_record_hint_progress()
	var d: Dictionary = sim.inspect_block(p.x, p.y, p.z)
	block_box.text = Rules.format_block(d)
	panels.open(&"block")

func _show_favourites() -> void:
	_refresh_favourites()
	panels.open(&"favourites")
	_fx("ui_page")

func _refresh_favourites() -> void:
	if fav_list_box == null:
		return
	_fill_people(fav_list_box, fav_empty_label, sim.favourites(), true)

## The People panel (#351): everyone of the valley's communities, here or away.
func _show_people() -> void:
	_refresh_people()
	panels.open(&"people")
	_fx("ui_page")

func _refresh_people() -> void:
	if people_list_box == null:
		return
	var list: Array = sim.people()
	people_title.text = Rules.people_title(list)
	_fill_people(people_list_box, people_empty_label, list, false)

## Rows of people (Favourites #413, People #351): Inspect, Follow and, for favourites, remove.
func _fill_people(box: VBoxContainer, empty_label: Label, list: Array, removable: bool) -> void:
	for c in box.get_children():
		if c != empty_label:
			c.queue_free()
	empty_label.visible = list.is_empty()
	for d in list:
		var fid: int = int(d["id"])
		var away: bool = bool(d.get("away", false))
		var row := HBoxContainer.new()
		row.add_theme_constant_override("separation", 8)
		box.add_child(row)
		var info := RichTextLabel.new()
		info.bbcode_enabled = true
		info.fit_content = true
		info.scroll_active = false
		info.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		_style_text(info, 13)
		info.text = Rules.format_favourite_row(d)
		row.add_child(info)
		row.add_child(_make_btn("Inspect", "Open %s's inspect panel" % d["name"], func():
			selected_id = fid
			_update_inspect()
			if not away:
				var pos := _agent_pos(fid)
				if pos.x >= 0:
					cam.position = pos
				else:
					cam.position = Vector2(int(d["x"]), int(d["y"])) * TILE + Vector2(16, 16)
					view.view_z = clampi(int(d["z"]), view.zmin, view.zmax); view.refresh()
			_fx("ui_click")
		))
		if away:
			continue
		var following := follow_id == fid
		row.add_child(_make_btn("Following" if following else "Follow", "Keep the camera on %s (dragging stops follow)" % d["name"], func():
			if follow_id == fid:
				follow_id = -1
			else:
				follow_id = fid
				selected_id = fid
				var pos := _agent_pos(fid)
				if pos.x >= 0:
					cam.position = pos
			_refresh_favourites()
			_refresh_people()
			_update_inspect_buttons()
			_fx("ui_click")
		))
		if removable:
			row.add_child(_make_btn("✕", "Remove %s from favourites" % d["name"], func():
				sim.toggle_favourite(fid)
				_refresh_favourites()
				_update_inspect_buttons()
				_update_bar_buttons()
				_fx("ui_click")
			))

func _update_hud() -> void:
	var spd: int = SPEEDS[speed_idx]
	var spd_txt := "paused" if spd == 0 else "%dx" % spd
	# director_reason is the bridge's words for what the camera looks at (#200).
	var dir_txt := "director: %s" % ("on, watching %s" % director_reason if director_on and director_reason != "" else ("on" if director_on else "off"))
	var note := save_note if Time.get_ticks_msec() / 1000.0 < save_note_until else ""
	# Wrapped only between fields (#218).
	hud.text = Rules.hud_line([sim.world_name(), sim.calendar(), "z %d" % view.view_z, "pop %d" % sim.population(),
		sim.communities_summary(), spd_txt, dir_txt] + Rules.hud_watch_fields(debug_hud, sim.deity_name(), sim.engine_strain(), step_ms_avg) + ["[?] keys", note])
	# A known band of strangers is an active problem: it stays on screen until they go.
	for l in view.band_lines:
		hud.text += "\n! " + l
	# Troubles piling up (arcs.rs): one line while they last.
	var arc: String = sim.arc_line()
	if arc != "":
		hud.text += "\n! " + arc
	# Tracing is never silent (#567): the HUD says whose decisions are being written down.
	if why_traced_id >= 0 and why_name != "":
		hud.text += "\n" + Rules.why_indicator(why_name)
	if hover_id >= 0 and hover_id != selected_id:
		# One crossing per survivor, not one per frame.
		if hover_cache_id != hover_id:
			hover_cache_id = hover_id
			hover_cache_text = ""
			hover_tag_text = ""
			var hd: Dictionary = sim.inspect(hover_id)
			if not hd.is_empty():
				hover_cache_text = Rules.format_hover(hd)
				hover_tag_text = Rules.hover_tag(hd)
		if hover_cache_text != "":
			hud.text += "\n" + hover_cache_text
	elif hover_id < 0 and hover_tile.x >= 0:
		var site: String = sim.site_note(hover_tile.x, hover_tile.y, view.view_z)
		if site != "":
			hud.text += "\n> " + site
	if hints_active:
		var htxt := Rules.first_run_hint(hint_inspected_person, hint_inspected_block, hint_used_bar)
		if htxt != "":
			hud.text += "\n" + htxt
	_update_bar_buttons()
	# As wide as its longest line, wrapping only past the cap.
	var cap: float = Rules.hud_width(get_viewport_rect().size.x, panels.is_open(&"inspect") or panels.is_open(&"block"))
	var font := hud.get_theme_font("font")
	var widest := 0.0
	for line in hud.text.split("\n"):
		widest = maxf(widest, font.get_string_size(line, HORIZONTAL_ALIGNMENT_LEFT, -1, 14).x)
	hud.size = Vector2(minf(widest + 40.0, cap), 0)

var last_minor_ms := 0
## When each minor line not shown was let pass (ms), for the feed's "+N quieter lines" (#182):
## counted over the same 45 seconds a shown line stays.
var minor_dropped: Array = []

func _push_feed(rows: Array) -> void:
	var now_ms := Time.get_ticks_msec()
	for r in rows:
		var sev: int = r[4]
		var fav: bool = bool(r[7]) if r.size() > 7 else false
		# Minor lines (meals, sleeps, foraging) are throttled so that at 3× and 10× the feed
		# stays readable; notable and major lines always show. The count of lines not shown
		# is kept for the HUD so nothing is silently hidden.
		if sev <= 1:
			if not Rules.feed_allows_minor(feed_rate, now_ms - last_minor_ms):
				if minor_dropped.size() < 5000:
					minor_dropped.append(now_ms)
				continue
			last_minor_ms = now_ms
		var col := "#cfcfcf" if sev <= 1 else ("#ffe9a8" if sev == 2 else "#ffb3a0")
		if fav and sev >= 2:
			col = "#ffd866" if sev == 2 else "#ffb380"
		var raw_txt: String = r[5]
		var because_txt: String = String(r[8]) if r.size() > 8 else ""
		var formatted_line := Rules.format_feed_line(raw_txt, because_txt, col, fav, sev)
		var toast_txt := ("★ " + raw_txt) if (fav and sev >= 2) else raw_txt
		# How long it stays depends on what it says and how fast the valley runs (#1485).
		# The same news again joins the line already showing, "(x14)" (#1484, the key from #1483).
		feed_lines = Rules.feed_fold(feed_lines, {"t": now_ms, "hold": Rules.feed_hold_ms(sev, SPEEDS[speed_idx]),
			"text": formatted_line, "pos": Vector3i(r[1], r[2], r[3]), "key": String(r[9]) if r.size() > 9 else "",
			"count": 1, "raw": raw_txt, "because": because_txt, "col": col, "fav": fav, "sev": sev})
		# What the viewer chose to be told is said where the eye is (#329).
		var kind: String = r[6] if r.size() > 6 else ""
		if toaster != null and (Rules.announce(announce_mode, sev, kind) or (fav and sev >= 2)):
			# News is what is new (#1487): the same news on later days toasts as "again", muted, and
			# from the fourth time in ten game-days only the feed keeps it.
			var fold_key: String = String(r[9]) if r.size() > 9 else ""
			var nth := Rules.toast_repeat(toast_seen, fold_key, int(r[0])) if fold_key != "" else 1
			if nth <= 3:
				var shown_txt := toast_txt
				var shown_col := col
				if nth > 1:
					shown_txt = "%s %s" % [toast_txt, Rules.toast_again(nth, toast_seen[fold_key][0], int(r[0]))]
					shown_col = Color(col).lerp(Color(0.6, 0.6, 0.6), 0.6).to_html(false)
				toaster.toast(shown_txt, shown_col, kind, Vector3i(r[1], r[2], r[3]))
				# #351, WP-08 8.5: the same told event marks where it happened.
				if r.size() > 3 and r[1] >= 0 and view != null and view.world_marks != null:
					view.world_marks.mark(Vector3i(r[1], r[2], r[3]), Color(col), toast_txt)
			# A repeat is not news: only the first time pauses the valley.
			if pause_on_major and sev >= 3 and speed_idx != 0 and nth == 1:
				speed_idx = 0
				_note("paused: " + toast_txt)
	var max_lines := Rules.feed_max_lines(feed_rate)
	while feed_lines.size() > max_lines:
		feed_lines.pop_front()
	var now := Time.get_ticks_msec()
	# Lines stay for game time by severity, not a fixed 45 real seconds (#1485, FIXLOG 2026-11-14);
	# the count of quieter lines not shown follows the notable hold.
	feed_lines = feed_lines.filter(func(l): return now - l["t"] < int(l.get("hold", 45000)))
	var quiet_hold := Rules.feed_hold_ms(2, SPEEDS[speed_idx])
	minor_dropped = minor_dropped.filter(func(t): return now - t < quiet_hold)
	var txt: String = Rules.feed_text(feed_lines, minor_dropped.size() if not feed_lines.is_empty() else 0)
	if txt != feed_box.text:
		feed_box.text = txt
	feed_box.visible = txt != ""
	# fit_content sets the height from the text; anchor the box's bottom where it was.
	feed_box.offset_top = feed_box.offset_bottom - feed_box.get_content_height() - 16

## A click on a feed line looks at where it happened, on its level (#182).
func _on_feed_input(e: InputEvent) -> void:
	if not (e is InputEventMouseButton and e.pressed and e.button_index == MOUSE_BUTTON_LEFT):
		return
	var line_h := float(feed_box.get_content_height()) / maxf(1.0, float(feed_box.get_line_count()))
	var k: int = Rules.feed_line_at(e.position.y - 8.0, line_h, feed_lines.size())
	if k < 0:
		return
	var p: Vector3i = feed_lines[k]["pos"]
	if p.x < 0:
		return
	follow_id = -1
	last_input_time = Time.get_ticks_msec() / 1000.0
	cam.position = Vector2(p.x, p.y) * TILE + Vector2(16, 16)
	if p.z != view.view_z:
		view.view_z = clampi(p.z, view.zmin, view.zmax)
		view.refresh()
	_fx("ui_click")

func _agent_row(id: int) -> Array:
	var A: PackedInt32Array = view.agents
	for i in range(0, A.size(), view.AGENT_STRIDE):
		if A[i] == id:
			var row := []
			for k in view.AGENT_STRIDE:
				row.append(A[i + k])
			return row
	return []


func _update_inspect() -> void:
	# Whoever is selected is traced for the Why? lens (#567); nobody selected, nobody traced.
	if selected_id != why_traced_id:
		sim.set_why_target(selected_id)
		why_traced_id = selected_id
		why_name = ""
	if selected_id < 0:
		panels.close(&"inspect")
		inspect_text = ""
		return
	var d: Dictionary = sim.inspect(selected_id)
	if d.is_empty():
		selected_id = -1
		panels.close(&"inspect")
		return
	hint_inspected_person = true
	_record_hint_progress()
	why_name = String(d["name"])
	var row := _agent_row(selected_id)
	if row.is_empty():
		portrait.g = {}
		portrait.queue_redraw()
	else:
		if not view.species_art.has(row[22]):
			view.species_art = sim.species_art()
		portrait.show_person(row, view.species_art.get(row[22], {}))
	var s := "[b]%s[/b]  %s, %d years\n%s\n" % [d["name"], d["species"], d["age"], d["activity"]]
	var top_because := Rules.inspect_because_line(d)
	if top_because != "":
		s += "[color=#d8c8a0]%s[/color]\n" % top_because
	if why_open:
		s += Rules.format_why(d) + "\n"
	if d.has("kind"):
		s += "[color=#b8b0a0]%s[/color]\n" % d["kind"]
	if d.has("community") and d["community"] != "":
		s += "[color=#b9d3ff]%s[/color]%s\n" % [d["community"], ("  (" + d["role"] + ")") if d.get("role", "") != "" else ""]
		var people: PackedStringArray = d.get("people", PackedStringArray())
		if people.size() > 0:
			s += "[color=#8fa6c8]a people who %s[/color]\n" % "; who ".join(people)
	s += "\n"
	var needs: Dictionary = d["needs"]
	for k in ["hunger", "rest", "social", "comfort", "warmth"]:
		s += "%-8s %s\n" % [k, _bar(needs[k])]
	# Stress (#1418): sent with every inspect and never drawn; a full bar is the breaking point.
	if d.has("stress"):
		var st := int(round(clampf(float(d["stress"]), 0.0, 1.0) * 10))
		s += "%-8s [color=#d98f6f]%s[/color][color=#444]%s[/color]\n" % ["stress", "█".repeat(st), "█".repeat(10 - st)]
	s += "\nsurface: %s\n" % ", ".join(d["traits"])
	if d.get("body", "") != "":
		s += "\n" + Rules.format_body_summary(str(d["body"]))
	var skills: Dictionary = d["skills"]
	var best := []
	for k in skills.keys():
		if skills[k] > 0.5:
			best.append("%s %.0f" % [k, skills[k]])
	s += "skills: %s\n" % ", ".join(best)
	if d.get("tool", "") != "": s += "carries [color=#e0c890]%s[/color]\n" % d["tool"]
	if d.get("arms", "") != "": s += "armed with [color=#e0b0a0]%s[/color]\n" % d["arms"]
	if d.has("equipment") and d["equipment"].size() > 0:
		s += "\nequipped / wearing:\n"
		for e in d["equipment"]:
			s += "  [color=#e0c890]%s[/color]\n" % e
	if d.has("inventory") and d["inventory"].size() > 0:
		s += "inventory:\n"
		for it in d["inventory"]:
			s += "  [color=#c8d8e8]%s[/color]\n" % it
	if d.has("techniques") and d["techniques"].size() > 0:
		s += "knows how:\n"
		for t in d["techniques"]:
			s += "  [color=#b8d0e0]%s[/color]\n" % t
	s += Rules.format_told(d)
	if d.has("tales") and d["tales"].size() > 0:
		s += "has heard:\n"
		for t in d["tales"]:
			s += "  [color=#d8c0a0]%s[/color]\n" % t
	if d.get("partner", "") != "": s += "partner: %s\n" % d["partner"]
	if d.get("parents", "") != "": s += "parents: %s\n" % d["parents"]
	if d.get("children", "") != "": s += "children: %s\n" % d["children"]
	if d.has("customs") and d["customs"].size() > 0:
		s += "customs:\n"
		for c in d["customs"]:
			s += "  [color=#d9c9a8]%s[/color]\n" % c
	var mood: float = d["mood"]
	s += "mood: %s" % ("cheerful" if mood > 0.3 else ("content" if mood > 0.0 else ("low" if mood > -0.3 else "miserable")))
	# Why (#292): the thoughts pushing the mood its way, and what they feel most.
	if d.get("why", "") != "": s += ", because: %s" % d["why"]
	s += "\n"
	if d.get("feeling", "") != "": s += "feels: %s\n" % d["feeling"]
	# On their mind (#1418): each thought with its sign, strongest first.
	s += Rules.format_moodlets(d.get("thoughts", []))
	if d.has("relationships") and d["relationships"].size() > 0:
		s += "people: %s\n" % ", ".join(d["relationships"])
	if d.has("beliefs") and d["beliefs"].size() > 0:
		s += "believes:\n"
		for b in d["beliefs"]:
			s += "  [color=#c8d8b0]%s[/color]\n" % b
	if d.has("past") and d["past"].size() > 0:
		s += "before the portal, in %s:\n" % d.get("home", "the old village")
		for p in d["past"]:
			s += "  [color=#d8c7a6]%s[/color]\n" % p
	if d.has("memories") and d["memories"].size() > 0:
		s += "remembers:\n"
		for m in d["memories"]:
			s += "  [color=#c9c9c9]%s[/color]\n" % m
	# E16: personal experience is shown as bounded evidence, not as a global explanation.
	s += Rules.format_experience(d)
	s += "knows %d things\n" % d["known"]
	for k in d["known_list"]:
		s += "  [color=#9a9a9a]%s[/color]\n" % k
	# Set only when it changed: setting the text re-parses it and sends the scroll back to the
	# top, which every frame made the lower half unreachable (#178, QA 2026-10-20, C63).
	if s != inspect_text:
		var bar := inspect_box.get_v_scroll_bar()
		var at := bar.value
		inspect_text = s
		inspect_box.text = s
		bar.value = at
	_update_inspect_buttons()
	panels.open(&"inspect")

func _bar(v: float) -> String:
	var n := int(round(clampf(v, 0.0, 1.0) * 10))
	return "[color=#8fd48f]" + "█".repeat(n) + "[/color][color=#444]" + "█".repeat(10 - n) + "[/color]"

func _show_summary(from_tick: int) -> void:
	var d: Dictionary = sim.summary_since(from_tick)
	if d.is_empty():
		return
	var ticks: int = int(d["to"]) - int(d["from"])
	var s := "[b]While you were away[/b]  (%.1f game-hours)\n\n" % (ticks / 1200.0)
	var counts: Dictionary = d["counts"]
	var parts := []
	for k in counts.keys():
		parts.append("%s %d" % [k, counts[k]])
	s += ", ".join(parts) + "\n\n"
	for l in d["lines"]:
		s += l + "\n"
	s += "\n[color=#9a9a9a]H or Esc to close[/color]"
	summary_box.text = s
	panels.open(&"summary")
	_fx("ui_page")

func _show_legends(target: String = "world:0") -> void:
	var page: String = sim.legends_page(target)
	if page == "":
		return
	legends_text.text = page + "\n\n[color=#9a9a9a]L or Esc to close   Backspace: back   the slider shows the world in any year[/color]"
	legends_text.scroll_to_line(0)
	if not panels.is_open(&"legends"):
		var last: int = sim.realm_year()
		legends_year.min_value = 0
		legends_year.max_value = max(last, 1)
		legends_year.step = 1
		legends_year.set_value_no_signal(last)
		_on_legends_year(last)
	# Legends is 1200 px wide and runs under the inspect and block panels at the right edge, which
	# then covered its text (D92): it closes them, as their own Close buttons do.
	for side in [&"inspect", &"block"]:
		if panels.is_open(side):
			_close_panel(side)
	panels.open(&"legends")
	_fx("ui_page")

func _on_legends_link(meta) -> void:
	legends_back.append(legends_text.get_meta("page", "world:0"))
	legends_text.set_meta("page", str(meta))
	_show_legends(str(meta))
	_fx("ui_click")

func _on_legends_year(v: float) -> void:
	var size: int = sim.legends_map_size()
	if size <= 0:
		legends_map.texture = null
		legends_year_label.text = "This world has no map beyond the valley."
		return
	var bytes: PackedByteArray = sim.legends_map(int(v))
	var img := Image.create_from_data(size, size, false, Image.FORMAT_RGB8, bytes)
	# The map of the present marks where those of the valley are now (#562).
	var away: Array = sim.away_list() if int(v) >= sim.realm_year() else []
	AwayMap.draw(img, away)
	legends_map.texture = ImageTexture.create_from_image(img)
	legends_away.text = AwayMap.lines(away)
	var back: int = sim.realm_year() - int(v) + 1
	var when := "the year before the portal" if back == 1 else "%d years before the portal" % back
	if back <= 1 and sim.realm_year() > 0:
		when = "now"
	legends_year_label.text = "The world %s (the valley in white, ruins dark%s)" % [when, ", ringed: where those away are" if away.size() > 0 else ""]

func _show_history() -> void:
	var d: Dictionary = sim.history(14)
	if d.is_empty():
		return
	# The colony's own years first, newest first, then the record and its customs, then the
	# past (#203: it opened on "501 years before the portal" and ~150 rows of the wider world).
	var s := "[b]History[/b]\n\n"
	for e in d["colony"]:
		s += "[b]%s[/b]  %s\n" % [e[0], e[1] if e[1] != "" else "quiet"]
	s += "\n[b]The record[/b]\n"
	for l in d["lines"]:
		s += l + "\n"
	if d["customs"].size() > 0:
		s += "\n[b]Why they do what they do[/b]\n"
		for c in d["customs"]:
			s += c + "\n"
	if d["valley"].size() > 0:
		s += "\n[b]The valley before the portal[/b]\n"
		for e in d["valley"]:
			s += "[b]%s[/b]  %s\n" % [e[0], e[1]]
	if d["world"].size() > 0:
		s += "\n[b]The wider world[/b]  [color=#9a9a9a][L] Legends has every year[/color]\n"
		for e in d["world"]:
			s += "[b]%s[/b]  %s\n" % [e[0], e[1]]
			for n in e[2]:
				s += "    %s\n" % n
	s += "\n[color=#9a9a9a]H or Esc to close   [V] toggles the view to what the community has seen[/color]"
	summary_box.text = s
	panels.open(&"summary")
	_fx("ui_page")

# ---------------------------------------------------------------- timelapse (#484)
func _sync_timelapse_panel() -> void:
	if timelapse_panel != null:
		timelapse_panel.set_watch_mode(str(timelapse_watch.get("mode", "view")))
		timelapse_panel.sync(timelapse.frames, timelapse_recording, timelapse.total_bytes)

## The series of the world now running (#484 finish): samples live under the world's identity,
## so a resumed or reloaded world goes on with its own series and never shows another world's.
## Opening it removes what the bounds no longer allow (Timelapse.MAX_SERIES, MAX_SERIES_BYTES).
func _open_timelapse_series() -> void:
	timelapse_identity = sim.world_identity()
	timelapse_epoch = sim.epoch()
	timelapse_series_dir = Timelapse.series_dir(timelapse_identity, timelapse_root)
	DirAccess.make_dir_recursive_absolute(ProjectSettings.globalize_path(timelapse_series_dir))
	for old in timelapse.load_series(timelapse_series_dir):
		Timelapse.remove_frame_files(old)
	Timelapse.prune_series(timelapse_series_dir, timelapse_root)
	timelapse_watch = {}
	if not timelapse.frames.is_empty():
		var last: Dictionary = timelapse.frames[-1].get("camera", {})
		if last.has("position") and last.has("z"):
			timelapse_watch = last.duplicate(true)

func _set_timelapse_recording(enabled: bool) -> void:
	if enabled and DisplayServer.get_name() == "headless":
		_note("timelapse needs a renderer; no snapshots were started")
		return
	timelapse_recording = enabled
	persist.settings["timelapse"] = enabled
	persist.save_settings()
	if enabled:
		if timelapse_identity != sim.world_identity():
			_open_timelapse_series()
		if timelapse_watch.is_empty():
			_use_current_timelapse_view()
		else:
			timelapse_next_tick = Timelapse.next_sample_tick(sim.tick(), timelapse_epoch)
	else:
		timelapse_next_tick = -1
	_sync_timelapse_panel()

func _use_current_timelapse_view() -> void:
	if view == null or cam == null:
		return
	timelapse_watch = {
		"mode": "view",
		"position": [cam.position.x, cam.position.y],
		"z": view.view_z,
		"zoom": cam.zoom.x,
		"known_only": view.known_only,
	}
	_restart_timelapse_watch()

## The whole valley at the settlement's level, fitted to the window at capture time.
func _use_valley_timelapse_view() -> void:
	if view == null or cam == null:
		return
	var fs: Vector3i = sim.settlement_centre()
	timelapse_watch = {
		"mode": "valley",
		"position": [view.w * TILE / 2.0, view.h * TILE / 2.0],
		"z": clampi(fs.z, view.zmin, view.zmax),
		"zoom": 0.0,
		"known_only": view.known_only,
	}
	_restart_timelapse_watch()

func _restart_timelapse_watch() -> void:
	if timelapse_recording and not timelapse_capture_active:
		timelapse_next_tick = sim.tick()
		_capture_timelapse()
	_sync_timelapse_panel()

func _capture_timelapse() -> void:
	if timelapse_capture_active or not timelapse_recording or view == null or cam == null:
		return
	if DisplayServer.get_name() == "headless":
		return
	timelapse_capture_active = true
	if timelapse_series_dir == "":
		_open_timelapse_series()
	var live_position := cam.position
	var live_zoom := cam.zoom
	var live_z: int = view.view_z
	var live_known: bool = view.known_only
	var live_director := director_on
	var live_follow := follow_id
	var live_anim: float = view.anim_t
	var live_flash: float = view.light.flash
	var hidden_layers: Array = []
	var watch: Dictionary = timelapse_watch.duplicate(true)
	if str(watch.get("mode", "view")) == "valley":
		watch["zoom"] = Timelapse.valley_zoom(get_viewport_rect().size, Vector2(view.w, view.h) * TILE)
	cam.position = Vector2(watch["position"][0], watch["position"][1])
	cam.zoom = Vector2.ONE * float(watch["zoom"])
	view.view_z = int(watch["z"])
	view.known_only = bool(watch["known_only"])
	view.anim_t = float(sim.tick() % 28800) / 20.0
	view.animation_frozen = true
	director_on = false
	follow_id = -1
	_camera(0.0)
	view.prepare_still()
	view.light.flash = 0.0
	view.light.update(0.0)
	view.weather.update()
	# FIXLOG 2026-11-02 #484 (finish): an emitter that had not yet run a frame with its current
	# emitting flag (a world just made or loaded, rain that began at this tick) restarted into
	# different particles than one that had, so the first same-tick sample differed from the
	# rest. One frame at the watch, stepping held, settles every emitter before the restart.
	await get_tree().process_frame
	view.weather.update()
	Timelapse.freeze_particles(view, sim.tick())
	var viewport_size := get_viewport_rect().size / cam.zoom
	var top_left := (cam.position - viewport_size / 2.0) / TILE
	var bottom_right := (cam.position + viewport_size / 2.0) / TILE
	var captured_tick: int = sim.tick()
	# Guard for FIXLOG 2026-10-31 #484 (c144 slice): identity is read from the sampled state;
	# _process withholds stepping only while the corresponding pixels are copied, and review
	# itself has no SimHost reference, so neither capture nor playback rewrites history.
	var captured_digest: String = sim.state_digest()
	# FIXLOG 2026-11-02 #484 (finish): a re-run of days already sampled (a reload of an older
	# save) is checked against the stored state digest. The same digest confirms the stored
	# sample; a different one means the stored sample and every later one describe a history
	# this world no longer has, so they are removed rather than shown as this world's past.
	var replay := timelapse.reconcile(captured_tick, captured_digest)
	var previous_png := ""
	for f in timelapse.frames:
		if int(f["tick"]) == captured_tick:
			previous_png = str(f.get("png_sha256", ""))
	if replay == "diverged":
		for old in timelapse.drop_from(captured_tick):
			Timelapse.remove_frame_files(old)
		_note("timelapse: this world went differently from the stored samples; samples from here on were removed")
	var since := captured_tick - Timelapse.SAMPLE_INTERVAL_TICKS
	for f in timelapse.frames:
		if int(f["tick"]) < captured_tick:
			since = int(f["tick"])
	var events: Dictionary = sim.summary_between(since, captured_tick, 8)
	DirAccess.make_dir_recursive_absolute(ProjectSettings.globalize_path(timelapse_series_dir))
	for child in get_children():
		if child is CanvasLayer and child.visible:
			hidden_layers.append(child)
			child.visible = false
	for i in range(3):
		await RenderingServer.frame_post_draw
	var image := get_viewport().get_texture().get_image()
	var base := "%s/%020d" % [timelapse_series_dir, captured_tick]
	var png_path := base + ".png"
	var json_path := base + ".json"
	var err := FAILED
	if image != null:
		image = Timelapse.fit_capture(image)
		err = image.save_png(png_path)
	if err == OK:
		var frame := {
			"format": 2,
			"tick": captured_tick,
			"seed": sim.world_seed(),
			"world_identity": timelapse_identity,
			"state_digest": captured_digest,
			"calendar": sim.calendar(),
			"known_only": bool(watch["known_only"]),
			"camera": watch,
			"region_tiles": [top_left.x, top_left.y, bottom_right.x - top_left.x, bottom_right.y - top_left.y],
			"viewport_pixels": [get_viewport_rect().size.x, get_viewport_rect().size.y],
			"image_pixels": [image.get_width(), image.get_height()],
			"sample_interval_ticks": Timelapse.SAMPLE_INTERVAL_TICKS,
			"scheduled": captured_tick == Timelapse.next_sample_tick(captured_tick, timelapse_epoch),
			"replay": replay,
			"events": {"from": since, "counts": events.get("counts", {}), "lines": events.get("lines", [])},
			"path": png_path,
			"bytes": FileAccess.get_file_as_bytes(png_path).size(),
			"png_sha256": FileAccess.get_sha256(png_path),
		}
		if replay == "confirmed":
			frame["replay_png_match"] = previous_png == str(frame["png_sha256"])
		var metadata := FileAccess.open(json_path, FileAccess.WRITE)
		if metadata != null:
			var stored := frame.duplicate(true)
			stored.erase("path")
			metadata.store_string(JSON.stringify(stored, "\t") + "\n")
			metadata.close()
			var evicted: Array = timelapse.add_frame(frame)
			for old in evicted:
				Timelapse.remove_frame_files(old)
			print("TIMELAPSE_FRAME tick %d seed %d bytes %d retained %d replay %s" % [captured_tick, sim.world_seed(), frame["bytes"], timelapse.frames.size(), replay])
		else:
			err = FileAccess.get_open_error()
	for layer in hidden_layers:
		layer.visible = true
	cam.position = live_position
	cam.zoom = live_zoom
	view.view_z = live_z
	view.known_only = live_known
	view.anim_t = live_anim
	view.animation_frozen = false
	view.light.flash = live_flash
	Timelapse.resume_particles(view)
	director_on = live_director
	follow_id = live_follow
	_camera(0.0)
	view.end_still()
	timelapse_next_tick = Timelapse.next_sample_tick(captured_tick + 1, timelapse_epoch)
	timelapse_capture_active = false
	if err != OK:
		_note("timelapse snapshot failed (error %d)" % err)
		printerr("TIMELAPSE_FAILED tick %d error %d" % [captured_tick, err])
	_sync_timelapse_panel()

# ---------------------------------------------------------------- loop
func _process(d: float) -> void:
	if help_panel != null and help_panel.visible:
		_place_help()
	if view:
		view.focus_id = follow_id if follow_id >= 0 else selected_id
		if view.name_tags != null:
			_sync_name_tags()
	# Nothing to run until a world exists: the arrival screen can be up, asking its questions.
	if view == null or cam == null:
		return
	# Announcements start under the HUD and wait while a reading panel is open (D91).
	if toaster != null:
		toaster.set_top(hud.position.y + hud.size.y + 8.0)
		var reading := (settings_panel != null and settings_panel.visible) or (files_page != null and files_page.visible)
		toaster.hold(Rules.toasts_held(panels.open_ids(), reading))
	var spd: int = SPEEDS[speed_idx]
	if spd > 0 and shot_path == "" and footage_dir == "" and not timelapse_capture_active:
		tick_accum += d * TICKS_PER_SEC * spd
		var n := int(tick_accum)
		if n > 0:
			tick_accum -= n
			# A recorded sample is taken at exactly its tick (#484 finish): stepping stops there.
			var allowed := mini(n, 400)
			if timelapse_recording:
				allowed = Timelapse.step_allowance(sim.tick(), timelapse_next_tick, allowed)
			if allowed > 0:
				sim.step(allowed)
				# A recording takes its sample at the exact tick, so it waits for the ticks
				# (#603: on the step thread they would otherwise still be running).
				if timelapse_recording:
					sim.sync()
			step_ms_avg = lerpf(step_ms_avg, sim.last_step_ms(), 0.1)
	if timelapse_recording and Timelapse.sample_due(sim.tick(), timelapse_next_tick):
		_capture_timelapse()
	_push_feed(sim.drain_feed())
	var st: String = sim.save_status()
	if st == "ok":
		_note("saved (%.0f ms capture)" % sim.last_save_prepare_ms())
	elif st.begins_with("save failed"):
		_note(st)
	var autosave_ticks := int(persist.settings["autosave_days"] * 28800)
	# A staged scene (--stage=demo and friends) is art review only (scenario.rs): never let it
	# write into the autosave ring, or the next plain launch loads the review world - mine shaft
	# and all - as though it were the game (#293).
	if autosave_ticks > 0 and sim.tick() - last_autosave_tick >= autosave_ticks and shot_path == "" and footage_dir == "" and stage == "":
		if sim.save_to(sim.next_ring_slot(persist.ring_base(), persist.RING)):
			last_autosave_tick = sim.tick()
	_audio(d)
	ending_accum += d
	# Not over the away-summary, which shares the panel (#210): the card waits its turn.
	if not ending_shown and ending_accum > 2.0 and not panels.is_open(&"summary"):
		ending_accum = 0.0
		var end: String = sim.ending()
		if end != "":
			ending_shown = true
			summary_box.text = "[b]The valley's story has ended.[/b]\n\n%s\n\nThe valley goes on without them. [Esc] to close." % end
			panels.open(&"summary")
	refresh_accum += d
	if refresh_accum > 0.05:
		refresh_accum = 0.0
		view.refresh()
	if not timelapse_capture_active:
		_camera(d)
	_update_hud()
	# A few times a second is enough for needs and activity, and spares an inspect call (a find
	# and a chronicle read) every frame (#178). A new selection shows at once.
	inspect_accum += d
	if inspect_accum >= 0.25 or (selected_id >= 0 and inspect_text == "") or (selected_id < 0 and panels.is_open(&"inspect")):
		inspect_accum = 0.0
		_update_inspect()
	fav_accum += d
	if fav_accum >= 0.5 and panels.is_open(&"favourites"):
		fav_accum = 0.0
		_refresh_favourites()

func _camera(d: float) -> void:
	var v := Vector2.ZERO
	if Input.is_key_pressed(KEY_A) or Input.is_key_pressed(KEY_LEFT): v.x -= 1
	if Input.is_key_pressed(KEY_D) or Input.is_key_pressed(KEY_RIGHT): v.x += 1
	if Input.is_key_pressed(KEY_W) or Input.is_key_pressed(KEY_UP): v.y -= 1
	if Input.is_key_pressed(KEY_S) or Input.is_key_pressed(KEY_DOWN): v.y += 1
	var now := Time.get_ticks_msec() / 1000.0
	if v != Vector2.ZERO:
		cam.position += v.normalized() * 600.0 * d / cam.zoom.x
		last_input_time = now
		follow_id = -1
	if follow_id >= 0:
		var p := _agent_pos(follow_id)
		if p.x >= 0:
			cam.position = cam.position.lerp(p, 1.0 - pow(0.001, d))
	elif director_on and now - last_input_time > 60.0:
		if now > director_next_check:
			director_next_check = now + 2.0
			var sug: Array = sim.director_suggest()
			# The bridge can answer a short array while the stepper is in flight with no
			# cached suggestion yet (#759): only a full [x, y, z, score, dwell, reason]
			# answer may drive the camera.
			if sug.size() >= 6:
				var tz: int = sug[2]
				if tz != view.view_z:
					view.view_z = tz
					view.refresh()
				director_target = Vector2(sug[0], sug[1]) * TILE + Vector2(16, 16)
				director_reason = sug[5]
		cam.position = cam.position.lerp(director_target, 1.0 - pow(0.02, d))
	var vp := get_viewport_rect().size / cam.zoom
	var half := vp / 2
	# Let the camera run a quarter-frame past the valley edge so a subject at the edge can sit
	# well inside the frame instead of on its border.
	cam.position.x = clampf(cam.position.x, half.x * 0.5, view.w * TILE - half.x * 0.5) if view.w * TILE > vp.x else view.w * TILE / 2.0
	cam.position.y = clampf(cam.position.y, half.y * 0.5, view.h * TILE - half.y * 0.5) if view.h * TILE > vp.y else view.h * TILE / 2.0
	var tl := (cam.position - vp / 2) / TILE
	var br := (cam.position + vp / 2) / TILE
	view.show_rect = Rect2i(int(floor(tl.x)) - 2, int(floor(tl.y)) - 3, int(ceil(br.x - tl.x)) + 4, int(ceil(br.y - tl.y)) + 6)

func _agent_pos(id: int) -> Vector2:
	var a: PackedInt32Array = view.agents
	for i in range(0, a.size(), view.AGENT_STRIDE):
		if a[i] == id:
			if a[i + 3] != view.view_z:
				view.view_z = a[i + 3]
				view.refresh()
			return Rules.drawn_at(a, i)
	return Vector2(-1, -1)

## Whoever is drawn under the pointer, within a few screen pixels at any zoom (#183, QA
## 2026-10-20, C63: the tile's centre and a fixed world radius missed walking people).
func _pick_agent(world_pos: Vector2) -> int:
	return Rules.pick(view.agents, view.AGENT_STRIDE, view.view_z, world_pos, cam.zoom.x)

## One wheel step toward the pointer, onto whole-pixel zoom levels (#183).
func _zoom_step(dir: int) -> void:
	var old := cam.zoom.x
	var nz: float = Rules.next_zoom(old, dir)
	if nz == old:
		return
	cam.position = Rules.zoom_about(cam.position, get_global_mouse_position(), old, nz)
	cam.zoom = Vector2(nz, nz)

## Close a panel as its own Close button does, with what closing it means: the inspect panel
## also lets go of the person (and stops following), the block panel of the tile.
## Load the manual save: F9 at once, the bar's Load after asking (D94). With nothing saved, say
## so in a player's words (FIXLOG 2026-11-14).
func _load_manual(ask_first: bool) -> void:
	var manifest: String = persist.manual_path().path_join("manifest.json")
	if not FileAccess.file_exists(manifest):
		_note(Rules.LOAD_NOTHING_SAVED)
		_fx("ui_click")
		return
	if ask_first:
		load_question_label.text = Rules.load_question(Rules.saved_tick(FileAccess.get_file_as_string(manifest)), sim.tick())
		load_popup.visible = true
		_fx("ui_click")
		return
	_do_load_manual()

func _do_load_manual() -> void:
	load_popup.visible = false
	var err: String = sim.load_from(persist.manual_path())
	if err == "": _world_replaced(); _note("loaded manual save"); _fx("ui_chime")
	else: _note("load failed: " + err)

func _close_panel(id: StringName) -> void:
	match id:
		&"inspect":
			selected_id = -1
			follow_id = -1
		&"block":
			selected_block = Vector3i(-1, -1, -1)
	panels.close(id)

func _unhandled_input(e: InputEvent) -> void:
	# Nothing to steer and nobody to hover until a world exists: the arrival screen may be up.
	if view == null or cam == null:
		return
	var now := Time.get_ticks_msec() / 1000.0
	if e is InputEventKey and e.pressed:
		match e.keycode:
			# Looking up or down a level stops following, as every other camera move does;
			# following would snap the view back to the followed person's level (issue 100).
			KEY_PERIOD, KEY_PAGEUP:
				follow_id = -1
				view.view_z = mini(view.view_z + 1, view.zmax); view.refresh(); last_input_time = now
			KEY_COMMA, KEY_PAGEDOWN:
				follow_id = -1
				view.view_z = maxi(view.view_z - 1, view.zmin); view.refresh(); last_input_time = now
			KEY_SPACE:
				speed_idx = 1 if speed_idx == 0 else 0
			KEY_1: speed_idx = 1
			KEY_2: speed_idx = 2
			KEY_3: speed_idx = 3
			# C, not D: D is held to pan right, so a press there toggled the director (issue 34).
			# The same path as the settings checkbox, so both show one state (issue 89).
			KEY_C: _on_director_toggled(not director_on); _fx("ui_click")
			KEY_G:
				_cycle_feed_rate()
			KEY_N:
				# #329: what to be told of; with Shift, pause on a major line.
				if e.shift_pressed:
					pause_on_major = not pause_on_major
					_note("pause on major events: " + ("on" if pause_on_major else "off"))
				else:
					announce_mode = (announce_mode + 1) % Rules.ANNOUNCE_NAMES.size()
					_note("announce: " + Rules.ANNOUNCE_NAMES[announce_mode])
				_save_announce()
				_fx("ui_click")
			KEY_F:
				follow_id = -1 if follow_id == selected_id else selected_id
				_update_inspect_buttons()
			KEY_R:
				# #192, WP-08 8.4: relationship lines — who is bound to whom, coloured by
				# the from-side's opinion band; the same zoom gate as the emotes.
				view.entities.relationships_visible = not view.entities.relationships_visible
				view.refresh()
				_note("relationship lines: " + ("on" if view.entities.relationships_visible else "off"))
				_fx("ui_click")
			KEY_K:
				if panels.is_open(&"waiting"): panels.close(&"waiting"); _fx("ui_click")
				else: _show_waiting()
			KEY_J:
				if panels.is_open(&"favourites"): panels.close(&"favourites"); _fx("ui_click")
				else: _show_favourites()
			KEY_P:
				if panels.is_open(&"people"): panels.close(&"people"); _fx("ui_click")
				else: _show_people()
			KEY_B:
				_set_bar_visible(not bar_panel.visible)
				_fx("ui_click")
			KEY_HOME:
				_go_home()
				_fx("ui_click")
			KEY_H:
				if panels.is_open(&"summary"): panels.close(&"summary"); _fx("ui_click")
				else: _show_history()
			KEY_T:
				panels.toggle(&"timelapse")
				_sync_timelapse_panel()
				_fx("ui_click")
			KEY_L:
				if panels.is_open(&"legends"): panels.close(&"legends"); _fx("ui_click")
				else:
					legends_back.clear()
					legends_text.set_meta("page", "world:0")
					_show_legends("world:0")
			KEY_BACKSPACE:
				if panels.is_open(&"legends") and legends_back.size() > 0:
					var prev: String = legends_back.pop_back()
					legends_text.set_meta("page", prev)
					_show_legends(prev)
			KEY_V:
				_fx("ui_click")
				view.known_only = not view.known_only
				view.refresh()
				_note("view: %s" % ("what the community has seen" if view.known_only else "everything"))
			KEY_F5:
				if sim.save_to(persist.manual_path()): _note("saving…"); _fx("ui_chime")
			KEY_F9:
				_load_manual(false)
			KEY_O:
				settings_panel.visible = not settings_panel.visible
				_fx("ui_click")
			KEY_QUESTION, KEY_SLASH:
				panels.toggle(&"help")
				_fx("ui_click")
			KEY_F1, KEY_F2, KEY_F3, KEY_F4, KEY_F6, KEY_F7, KEY_F8:
				var n: int = {KEY_F1: 1, KEY_F2: 2, KEY_F3: 3, KEY_F4: 4, KEY_F6: 6, KEY_F7: 7, KEY_F8: 8}[e.keycode]
				if e.ctrl_pressed:
					persist.bookmarks[n] = {"pos": cam.position, "z": view.view_z, "zoom": cam.zoom.x}
					persist.save_settings(); _note("bookmark %d set" % n)
				elif persist.bookmarks.has(n):
					var b = persist.bookmarks[n]
					cam.position = b["pos"]; cam.zoom = Vector2(b["zoom"], b["zoom"])
					view.view_z = b["z"]; view.refresh(); last_input_time = now; follow_id = -1
			KEY_ESCAPE:
				# Esc closes what is on top. It used to name six panels one by one and missed People,
				# Waiting work and the inspect panel, though each says "Close [Esc]" (FIXLOG 2026-11-14).
				if load_popup != null and load_popup.visible: load_popup.visible = false; _fx("ui_click")
				elif bookmarks_popup != null and bookmarks_popup.visible: bookmarks_popup.visible = false; _fx("ui_click")
				elif files_page != null and files_page.visible: files_page.close(); _fx("ui_click")
				elif panels.top() != &"": _close_panel(panels.top()); _fx("ui_click")
				elif settings_panel.visible: settings_panel.visible = false; _fx("ui_click")
				else:
					selected_id = -1; follow_id = -1
	if e is InputEventMouseButton:
		last_input_time = now
		if e.pressed:
			if e.button_index == MOUSE_BUTTON_WHEEL_UP: _zoom_step(1)
			elif e.button_index == MOUSE_BUTTON_WHEEL_DOWN: _zoom_step(-1)
			elif e.button_index == MOUSE_BUTTON_LEFT:
				lmb_down = true
				lmb_start = e.position
				lmb_dragging = false
		else:
			if e.button_index == MOUSE_BUTTON_LEFT and lmb_down:
				lmb_down = false
				if not lmb_dragging:
					var mp: Vector2 = get_canvas_transform().affine_inverse() * e.position
					var id := _pick_agent(mp)
					if id >= 0:
						if id != selected_id: _fx("ui_click")  # the inspect panel opens
						selected_id = id
						panels.close(&"block")
						selected_block = Vector3i(-1, -1, -1)
					else:
						selected_id = -1
						follow_id = -1
						var tx := floori(mp.x / 32.0)
						var ty := floori(mp.y / 32.0)
						if tx >= 0 and ty >= 0 and tx < view.w and ty < view.h:
							_inspect_block(Vector3i(tx, ty, view.view_z))
							_fx("ui_click")
						else:
							panels.close(&"block")
							selected_block = Vector3i(-1, -1, -1)
				lmb_dragging = false
	# Only a drag pans, so only a drag stops following; the pointer merely moving over the
	# valley used to drop follow as soon as it twitched (issue 35). Left-drag also pans once it
	# moves past Rules.DRAG_PX (#413).
	if e is InputEventMouseMotion:
		if lmb_down and (lmb_dragging or Rules.is_drag(lmb_start, e.position)):
			lmb_dragging = true
			cam.position -= e.relative / cam.zoom
			follow_id = -1
		elif e.button_mask & (MOUSE_BUTTON_MASK_MIDDLE | MOUSE_BUTTON_MASK_RIGHT):
			cam.position -= e.relative / cam.zoom
			follow_id = -1
		var mp: Vector2 = get_canvas_transform().affine_inverse() * e.position
		hover_id = _pick_agent(mp)
		hover_tile = Vector2i(floori(mp.x / 32.0), floori(mp.y / 32.0))
		last_input_time = now

# ---------------------------------------------------------------- headless
## Write a stable, opt-in result envelope for headless client automation (#294 / #378).
## The optional inspect payload is the same read model used by the visible panels; no new
## simulation state is invented here. A shot or footage run writes this immediately before it
## quits, while a metadata-only run writes after the world has been prepared.
## Returns false when the file could not be written, so the caller's exit code says so (a failed
## write once exited 0: the writer's quit(1) was overridden by the caller's quit()).
func _write_automation_json() -> bool:
	if automation_json_path == "":
		return true
	var payload := {
		"schema": 1,
		"world_seed": sim.world_seed(),
		"tick": sim.tick(),
		"world_name": sim.world_name(),
		"display": DisplayServer.get_name(),
		"inspect_id": inspect_arg,
		"inspect": _json_safe(sim.inspect(inspect_arg)) if inspect_arg >= 0 else {},
		"inspect_block": _json_safe(sim.inspect_block(
			inspect_block_arg.x,
			inspect_block_arg.y,
			int(view.view_z) if inspect_block_arg.z == 9999 else inspect_block_arg.z,
		)) if inspect_block_arg.x >= 0 else {},
		"shot": shot_path,
		"footage": footage_dir,
	}
	var f := FileAccess.open(automation_json_path, FileAccess.WRITE)
	if f == null:
		printerr("AUTOMATION_JSON_FAILED %s" % automation_json_path)
		return false
	f.store_string(JSON.stringify(payload))
	f.close()
	print("AUTOMATION_JSON %s" % automation_json_path)
	return true

func _json_safe(value: Variant) -> Variant:
	match typeof(value):
		TYPE_FLOAT:
			return null if is_nan(value) or is_inf(value) else value
		TYPE_DICTIONARY:
			var out := {}
			for key in value:
				out[key] = _json_safe(value[key])
			return out
		TYPE_ARRAY:
			var out: Array = []
			for item in value:
				out.append(_json_safe(item))
			return out
		_:
			return value

## --save-at-end: write the save once it is complete (the save runs off-thread).
func _save_at_end() -> void:
	var ok: bool = sim.save_to(save_at_end)
	var st: String = sim.save_status()
	while st == "saving":
		await get_tree().process_frame
		st = sim.save_status()
	print("SAVE %s -> %s (%s, capture %.1f ms)" % [save_at_end, ok, st, sim.last_save_prepare_ms()])

## --save-at-end without --shot or --footage (issue 21): run, save, quit.
func _do_save_and_quit() -> void:
	await _save_at_end()
	get_tree().quit()

func _do_shot() -> void:
	_push_feed(sim.drain_feed())
	_update_inspect()
	_update_hud()
	# The sound's event pass runs on a 0.1 s accumulator: once here, so a shot on the strike
	# tick shows the flash and logs the thunder (#221).
	if sound: sound.pump()
	if save_at_end != "":
		await _save_at_end()
	# Godot's --headless has no renderer: no frame is ever drawn, so waiting for one hung for
	# ever (issues 22, 23). Say so and finish instead of hanging.
	if DisplayServer.get_name() == "headless":
		print("SHOT_FAILED %s: --headless draws nothing, so there is no picture to save. Run without --headless (on Linux, under Xvfb as tools/shots/shot.sh does)." % shot_path)
		get_tree().quit(2)
		return
	for i in range(3):
		await RenderingServer.frame_post_draw
	var err := get_viewport().get_texture().get_image().save_png(shot_path)
	if sound: sound.save_log()
	# Only a written file is a shot: tools/shots/shot.sh trusts this line (#108).
	if err != OK:
		printerr("SHOT_FAILED %s (error %d)" % [shot_path, err])
		get_tree().quit(1)
		return
	print("SHOT_SAVED %s" % shot_path)
	get_tree().quit(0 if _write_automation_json() else 1)

func _do_footage() -> void:
	# N frames, `footage_every` ticks apart, with the director driving the camera.
	last_input_time = -10000.0
	for i in range(footage_frames):
		sim.step(footage_every)
		sim.sync()  # the step thread (#603): the world at that tick before going on
		_push_feed(sim.drain_feed())
		view.refresh()
		var sug: Array = sim.director_suggest()
		if sug[2] != view.view_z:
			view.view_z = sug[2]; view.refresh()
		cam.position = Vector2(sug[0], sug[1]) * TILE + Vector2(16, 16)
		# _process keeps easing the camera toward director_target between frames; without this
		# line every frame drifted back to the opening focus (found in the Phase 11 review).
		director_target = cam.position
		director_reason = sug[5]
		_camera(0.0)
		_update_hud()
		for k in range(3):
			await RenderingServer.frame_post_draw
		var ferr := get_viewport().get_texture().get_image().save_png("%s/f%02d.png" % [footage_dir, i])
		if ferr != OK:
			printerr("FOOTAGE_FAILED %s/f%02d.png (error %d)" % [footage_dir, i, ferr])
			get_tree().quit(1)
			return
		print("FOOTAGE_FRAME %d %s" % [i, director_reason])
	if sound: sound.save_log()
	print("FOOTAGE_DONE")
	get_tree().quit(0 if _write_automation_json() else 1)

func _note(t: String, secs: float = 4.0) -> void:
	save_note = t
	save_note_until = Time.get_ticks_msec() / 1000.0 + secs

func _slider(label: String, key: String, lo: float, hi: float) -> void:
	var row := HBoxContainer.new()
	var l := Label.new(); l.text = label; l.custom_minimum_size.x = 150
	var sl := HSlider.new(); sl.min_value = lo; sl.max_value = hi; sl.step = 0.05
	sl.value = persist.settings[key]; sl.custom_minimum_size.x = 220
	sl.size_flags_vertical = Control.SIZE_SHRINK_CENTER
	# The value in words beside it (Rules.setting_text, FIXLOG 2026-11-14).
	var shown := Label.new(); shown.custom_minimum_size.x = 120
	shown.text = Rules.setting_text(key, sl.value)
	sl.value_changed.connect(func(v):
		persist.settings[key] = v; persist.save_settings(); _apply_audio()
		shown.text = Rules.setting_text(key, v))
	row.add_child(l); row.add_child(sl); row.add_child(shown); settings_box.add_child(row)

func _on_director_toggled(on: bool) -> void:
	persist.settings["director"] = on
	director_on = on
	if director_cb != null:
		director_cb.set_pressed_no_signal(on)
	persist.save_settings()

func _build_settings() -> void:
	var layer := CanvasLayer.new(); layer.layer = 2; add_child(layer)
	settings_panel = PanelContainer.new()
	settings_panel.set_anchors_preset(Control.PRESET_CENTER)
	settings_panel.offset_left = -220; settings_panel.offset_right = 220
	settings_panel.offset_top = -160; settings_panel.offset_bottom = 160
	var bg := StyleBoxFlat.new(); bg.bg_color = Color(0.05, 0.05, 0.07, 0.9)
	bg.content_margin_left = 14; bg.content_margin_right = 14; bg.content_margin_top = 10; bg.content_margin_bottom = 10
	settings_panel.add_theme_stylebox_override("panel", bg)
	settings_panel.visible = false
	settings_box = VBoxContainer.new(); settings_panel.add_child(settings_box)
	var hdr := HBoxContainer.new(); settings_box.add_child(hdr)
	var title := Label.new(); title.text = "Settings"; title.size_flags_horizontal = Control.SIZE_EXPAND_FILL; hdr.add_child(title)
	hdr.add_child(_make_btn("Close [Esc]", "Close settings [O or Esc]", func():
		settings_panel.visible = false
		_fx("ui_click")
	))
	_slider("Master volume", "master", 0.0, 1.0)
	_slider("Ambient volume", "ambient", 0.0, 1.0)
	_slider("Effects volume", "fx", 0.0, 1.0)
	_slider("Music volume", "music", 0.0, 1.0)
	_slider("Autosave", "autosave_days", 0.0, 5.0)
	feed_rate_btn = _make_btn("Feed rate: " + Rules.feed_rate_name(feed_rate), "Cycle feed rate: calm (notable only), normal, or full [G]", func():
		_cycle_feed_rate()
	)
	settings_box.add_child(feed_rate_btn)
	name_tag_btn = _make_btn("Name tags: " + Rules.name_tag_name(name_tag_mode), "Cycle the names over people: off, favourites and the followed person, or everyone when zoomed in", func():
		_cycle_name_tags()
	)
	settings_box.add_child(name_tag_btn)
	# The live state, not the saved preference: --no-director or a shot can start it off.
	director_cb = CheckBox.new(); director_cb.text = "Camera director"; director_cb.button_pressed = director_on
	director_cb.toggled.connect(_on_director_toggled)
	settings_box.add_child(director_cb)
	# The live state: the flag or env var can have it on though the saved preference is off.
	var log_cb := CheckBox.new(); log_cb.text = "Debug decision log (writes every decision to a file)"; log_cb.button_pressed = sim.debug_log_path() != ""
	log_cb.toggled.connect(_on_debug_log_toggled)
	settings_box.add_child(log_cb)
	debug_log_hint = Label.new(); debug_log_hint.add_theme_font_size_override("font_size", 11)
	debug_log_hint.text = "Decision log: " + (sim.debug_log_path() if sim.debug_log_path() != "" else "off")
	settings_box.add_child(debug_log_hint)
	var hint := Label.new(); hint.text = "Saves: " + persist.save_dir(); hint.add_theme_font_size_override("font_size", 11)
	settings_box.add_child(hint)
	# WP-09 9.12: every file Ostrog makes, from the files register (files_page.gd).
	settings_box.add_child(_make_btn("Your files and data", "Every file Ostrog keeps on this computer, its size, and whether it is ever sent", func():
		files_page.debug_log_on = sim.debug_log_path() != ""
		settings_panel.visible = false
		files_page.open()
		_fx("ui_click")
	))
	layer.add_child(settings_panel)
	files_page = preload("res://scripts/files_page.gd").new()
	files_page.setup(_make_btn, _panel_bg())
	files_page.closed.connect(func(): settings_panel.visible = true)
	layer.add_child(files_page)

func _build_audio() -> void:
	sound = preload("res://scripts/audio/sound.gd").new()
	view.add_child(sound)
	var quiet := shot_path != "" or footage_dir != ""
	sound.init(view, persist.settings, quiet and audio_log == "")
	sound.log_path = audio_log
	music = preload("res://scripts/audio/music_director.gd").new()
	view.add_child(music)
	music.init(view, sim, quiet and audio_log == "", sound)

func _fx(name: String) -> void:
	if sound: sound.ui(name)

func _apply_audio() -> void:
	if sound: sound.apply_volumes()

func _audio(_d: float) -> void:
	pass

## #329: the announcement choice, kept between runs.
func _load_announce() -> void:
	var c := ConfigFile.new()
	if c.load(ANNOUNCE_CFG) == OK:
		announce_mode = clampi(int(c.get_value("announce", "mode", 0)), 0, Rules.ANNOUNCE_NAMES.size() - 1)
		pause_on_major = bool(c.get_value("announce", "pause_on_major", false))

func _save_announce() -> void:
	var c := ConfigFile.new()
	c.set_value("announce", "mode", announce_mode)
	c.set_value("announce", "pause_on_major", pause_on_major)
	c.save(ANNOUNCE_CFG)

## A click on a toast looks where it happened (#329), as a click on a feed line does.
func _on_toast_looked(p: Vector3i) -> void:
	follow_id = -1
	last_input_time = Time.get_ticks_msec() / 1000.0
	cam.position = Vector2(p.x, p.y) * TILE + Vector2(16, 16)
	if p.z != view.view_z:
		view.view_z = clampi(p.z, view.zmin, view.zmax)
		view.refresh()
	_fx("ui_click")

