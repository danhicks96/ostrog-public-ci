extends SceneTree
## The client's guard tests (C63): each checks one rule in res://scripts/rules.gd that a fault
## from a QA round broke, and fails without that fix. tools/ci.sh runs it from godot/ like
## gd_check.gd: tools/godot --path . --headless -s /abs/path/tools/gd_test.gd
## Prints GD_TEST <n> passed, <m> failed; exits 1 on any failure.

const FeedHarness := preload("gd_feed_harness.gd")
const Rules := preload("res://scripts/rules.gd")
const RoofLayer := preload("res://scripts/render/roof_layer.gd")
const Timelapse := preload("res://scripts/timelapse.gd")
const TimelapsePanel := preload("res://scripts/timelapse_panel.gd")
const Effects := preload("res://scripts/render/effects.gd")
const WorldView := preload("res://scripts/world_view.gd")
const LightLayer := preload("res://scripts/render/light_layer.gd")

class EffectsView:
	extends RefCounted
	const AGENT_STRIDE := 25
	var agents := PackedInt32Array()
	var thin := PackedInt32Array()
	var view_z := 0
	var anim_t := 0.0
	var w := 64
	var h := 64
	var levels := {}
	var texture: Texture2D

	func cull_rect() -> Rect2i:
		return Rect2i(0, 0, w, h)

	func tile_at(_z: int, _x: int, _y: int) -> int:
		return 1

	func _tex(_path: String) -> Texture2D:
		if texture == null:
			var image := Image.create(288, 48, false, Image.FORMAT_RGBA8)
			image.fill(Color.WHITE)
			texture = ImageTexture.create_from_image(image)
		return texture

class UndergroundView:
	extends RefCounted
	var w := 1
	var h := 1
	var view_z := -2
	var sky := 0
	var ground := PackedInt32Array([0])
	var anim_t := 0.0
	var animation_frozen := false
	var viewport: Viewport

	func tile_at(_z: int, _x: int, _y: int) -> int:
		return 0

	func shape_of(_tile: int) -> int:
		return 0

	func sun_level() -> float:
		return 1.0

	func get_viewport() -> Viewport:
		return viewport

var failed := 0
var passed := 0

func _init() -> void:
	for m in get_method_list():
		var n: String = m["name"]
		if n.begins_with("test_"):
			var before := failed
			call(n)
			if failed == before:
				passed += 1
			else:
				print("GD_TEST_FAIL %s" % n)
	# Layout tests need the GUI laid out, which takes frames: they run from _process.
	frames = 0

var frames := -1
var wizard: Node = null

func _process(_d: float) -> bool:
	if frames < 0:
		return false
	frames += 1
	if frames == 1:
		get_root().size = Vector2i(1280, 720)
		wizard = preload("res://scripts/setup.gd").new()
		get_root().add_child(wizard)
		wizard.start(SimHost.new(), 1)
		wizard.goto(2)
	elif frames == 6:
		var before := failed
		layout_peoples_page()
		if failed == before:
			passed += 1
		else:
			print("GD_TEST_FAIL layout_peoples_page")
		if wizard != null:
			wizard.free()
			wizard = null
		_scene_tests()
	return false

# Tests that need an active viewport run once the scene tree is ready.
func _scene_tests() -> void:
	for n in ["wizard_seed_is_random_per_new_game_and_kept_across_pages", "wizard_preview_follows_the_place", "batch_mode_draws_people_once_through_the_layered_renderer"]:
		var before := failed
		await call(n)
		if failed == before:
			passed += 1
		else:
			print("GD_TEST_FAIL %s" % n)
	print("GD_TEST %d passed, %d failed" % [passed, failed])
	quit(1 if failed > 0 else 0)

func _new_wizard(seed := -1) -> Node:
	var wz = preload("res://scripts/setup.gd").new()
	get_root().add_child(wz)
	wz.start(SimHost.new(), seed)
	return wz

## Habitable regions of the seed's world other than the one it would choose itself.
func _other_regions(wz: Node, n: int) -> Array:
	var wp: Dictionary = wz.sim.world_preview(wz.seed_v)
	var size: int = int(wp["size"])
	var out := []
	var r := 0
	while out.size() < n and r < size * size:
		if r != int(wp["region"]) and bool(wz.sim.region_info(wz.seed_v, r).get("habitable", false)):
			out.append(r)
		r += 97
	return out

func _wait_preview(wz: Node) -> bool:
	var waited := 0
	while wz.preview_pending() and waited < 1200:
		await process_frame
		waited += 1
	return not wz.preview_pending()

# #559: the valley page always opened on seed 7 (the launcher's --seed) or 1. Each New game
# rolls its own seed; going back a page and returning keeps the seed and the place chosen.
func wizard_seed_is_random_per_new_game_and_kept_across_pages() -> void:
	var seeds := {}
	for k in 4:
		var wz := _new_wizard()
		seeds[wz.seed_v] = true
		check(wz.seed_v >= 0 and wz.seed_v <= wz.SEED_MAX, "a rolled seed fits the spin box")
		wz.free()
	check(seeds.size() >= 3, "four New games rolled %d different seeds" % seeds.size())
	var fixed := _new_wizard(7)
	check(fixed.seed_v == 7, "an asked-for seed is kept (--fixed-seed, wizard shots, tests)")
	fixed.free()
	var wz := _new_wizard()
	wz.goto(1)
	await process_frame
	var seed: int = wz.seed_v
	var picks := _other_regions(wz, 1)
	check(picks.size() == 1, "the world has another habitable place")
	check(wz.choose_region(picks[0]), "the place is chosen")
	await _wait_preview(wz)
	var note: String = wz.world_note.text
	wz.goto(0)
	await process_frame
	wz.goto(1)
	await process_frame
	check(wz.seed_v == seed, "back to the valley page: the same seed")
	check(wz.region_v == picks[0], "back to the valley page: the same place")
	check(wz.world_note.text == note, "back to the valley page: the same place's description")
	var spin: SpinBox = null
	for c in wz.body.get_child(0).get_child(0).get_child(0).get_children():
		if c is SpinBox:
			spin = c
	check(spin != null and int(spin.value) == seed, "the spin box shows the kept seed")
	check(not wz.preview_pending(), "the kept place's picture shows at once")
	wz.free()

func _preview_image(wz: Node) -> PackedByteArray:
	var t: Texture2D = wz.preview_rect.texture
	return t.get_image().get_data() if t != null else PackedByteArray()

# #560: picking another place on the world map moved only the portal's mark on the same
# picture. The picture is the ground that place makes, and picking the first again restores it.
func wizard_preview_follows_the_place() -> void:
	var wz := _new_wizard(3)
	wz.goto(1)
	check(await _wait_preview(wz), "the first picture arrives")
	var picks := _other_regions(wz, 2)
	check(picks.size() == 2, "the world has two other habitable places")
	if picks.size() < 2:
		wz.free()
		return
	wz.choose_region(picks[0])
	check(await _wait_preview(wz), "the first place's picture arrives")
	var a := _preview_image(wz)
	wz.choose_region(picks[1])
	check(await _wait_preview(wz), "the second place's picture arrives")
	var b := _preview_image(wz)
	check(a.size() > 0 and a.size() == b.size(), "both pictures are drawn")
	var differ := 0
	for i in range(0, mini(a.size(), b.size()), 3):
		if a[i] != b[i] or a[i + 1] != b[i + 1] or a[i + 2] != b[i + 2]:
			differ += 1
	check(differ * 4 > a.size() / 3, "two places, two pictures: %d of %d pixels differ" % [differ, a.size() / 3])
	# Not only their colours: the ground itself is the place's own.
	var ha: PackedInt32Array = wz.sim.valley_preview(3, PackedFloat32Array(), picks[0])["heights"]
	var hb: PackedInt32Array = wz.sim.valley_preview(3, PackedFloat32Array(), picks[1])["heights"]
	var ground_differs := 0
	for i in ha.size():
		if ha[i] != hb[i]:
			ground_differs += 1
	check(ground_differs * 4 > ha.size(), "two places, two grounds: %d of %d columns" % [ground_differs, ha.size()])
	var t0 := Time.get_ticks_usec()
	wz.choose_region(picks[0])
	var click_ms := (Time.get_ticks_usec() - t0) / 1000.0
	check(not wz.preview_pending(), "back to the first place: its picture at once, from the cache")
	check(_preview_image(wz) == a, "back to the first place: the same picture")
	print("GD_TEST_INFO wizard click back to a cached place took %.1f ms" % click_ms)
	# A click on a new place returns before its valley is made (made off the main thread).
	var picks3 := _other_regions(wz, 3)
	t0 = Time.get_ticks_usec()
	wz.choose_region(picks3[2])
	click_ms = (Time.get_ticks_usec() - t0) / 1000.0
	print("GD_TEST_INFO wizard click on a new place took %.1f ms" % click_ms)
	check(click_ms < 100.0, "a click on a new place does not wait for its valley (%.1f ms)" % click_ms)
	await _wait_preview(wz)
	wz.free()

# #199: at 1280x720 the peoples page shows what the seed draws without scrolling, and no kind's
# description runs into the next kind's row.
func layout_peoples_page() -> void:
	var list: Control = wizard.peoples_list
	check(list != null and list.get_child_count() > 0, "the preview has lines")
	var vis: Rect2 = wizard.get_viewport().get_visible_rect()
	var back: Control = wizard.back_button
	check(list.get_global_rect().end.y <= back.get_global_rect().position.y, "the preview ends above Back and Next")
	check(list.get_global_rect().end.y <= vis.end.y, "the preview is on screen")
	var rows: Array = []
	for c in wizard.body.get_children():
		if c is VBoxContainer and c != list:
			rows = c.get_children()
	check(rows.size() >= 6, "a row per kind, saw %d" % rows.size())
	for k in range(rows.size() - 1):
		var a: Control = rows[k]
		var desc: Control = a.get_child(a.get_child_count() - 1)
		check(desc.get_global_rect().end.y <= rows[k + 1].get_global_rect().position.y + 0.5, "row %d's description ends before row %d" % [k, k + 1])

func _scenario_tile(shape: int, material: int, flags: int = 0) -> int:
	return material | (shape << 16) | (flags << 24)

func _underground_scenario():
	var view = preload("res://scripts/world_view.gd").new()
	view.w = 3
	view.h = 3
	view.zmin = -1
	view.zmax = 6
	view.view_z = 6
	view.ground = PackedInt32Array()
	view.ground.resize(9)
	view.ground.fill(6)
	for z in range(0, 7):
		var tiles := PackedInt32Array()
		tiles.resize(9)
		tiles.fill(0)
		view.levels[z] = tiles
	return view

# #519 review: a six-level peel must look for seepage beside the floor it actually reveals.
func test_underground_deep_peel_seepage_uses_visible_floor_level() -> void:
	var view = _underground_scenario()
	view.levels[0][4] = _scenario_tile(1, 8) # natural rock floor, six levels below view_z
	var ground: Array = view._ground_family(1, 1)
	check(ground[0] == "cave_floor" and ground[3] == 1, "the deep peel reports its visible floor level")
	check(not view._cave_has_adjacent_water(1, 1, ground[3]), "a dry deep floor has no seepage")
	view.levels[6][5] = _scenario_tile(0, 13, 1)
	check(not view._cave_has_adjacent_water(1, 1, ground[3]), "water only at view_z does not wet a deep floor")
	view.levels[6][5] = 0
	view.levels[1][5] = _scenario_tile(0, 13, 1)
	check(view._cave_has_adjacent_water(1, 1, ground[3]), "water at the visible level seeps into its neighbour")
	view.levels[1][5] = 0
	view.levels[0][5] = _scenario_tile(0, 13, 1)
	check(view._cave_has_adjacent_water(1, 1, ground[3]), "the corresponding lower water level also counts")
	view.free()

# #519 review: buried rock has no face, but an exposed underground natural face reaches strata.
func test_underground_strata_draws_only_on_exposed_underground_faces() -> void:
	var view = _underground_scenario()
	view.view_z = 2
	view.ground.fill(5)
	view.levels[2][4] = _scenario_tile(2, 8)
	check(view._natural_face_overlay(1, 1) == "strata", "an exposed underground natural face selects strata")
	view.levels[2][7] = _scenario_tile(2, 8)
	check(view._natural_face_overlay(1, 1) == "", "a natural face buried behind southern rock selects nothing")
	view.view_z = 5
	view.levels[5][4] = _scenario_tile(2, 8)
	check(view._natural_face_overlay(1, 1) == "cliff", "a surface cliff keeps its existing face without underground strata")
	view.free()

# The existing art-only stage always includes real gallery water for integrated overlay shots.
func test_underground_stage_contains_gallery_water() -> void:
	var sim := SimHost.new()
	sim.stage_demo(1)
	var fs: Vector3i = sim.founding_site()
	var width: int = sim.size().x
	var tiles: PackedInt32Array = sim.level_tiles(fs.z - 2)
	var water := tiles[(fs.y - 1) * width + fs.x + 25]
	check(((water >> 24) & 3) != 0, "the demo gallery contains its deterministic review water")
	sim.free()

# #459 review guard: person inspection exposes the authoritative held and carried item layers.
func test_person_inspection_exposes_equipment_and_inventory() -> void:
	var sim := SimHost.new()
	sim.stage_demo(1)
	var agents: PackedInt32Array = sim.agents()
	var saw_person := false
	var saw_equipment := false
	for i in range(0, agents.size(), 24):
		if agents[i + 4] != 0: # Species::Person is the first enum value.
			continue
		var d: Dictionary = sim.inspect(agents[i])
		if d.is_empty():
			continue
		saw_person = true
		check(d.has("equipment") and d["equipment"] is Array, "person inspect has an equipment array")
		check(d.has("inventory") and d["inventory"] is Array, "person inspect has an inventory array")
		if d.has("equipment") and d["equipment"] is Array and d["equipment"].size() > 0:
			saw_equipment = true
			check(String(d["equipment"][0]).contains(":"), "equipment inspection includes an item line")
			break
	check(saw_person, "staged scene exposes a person to inspect")
	check(saw_equipment, "person inspection includes held equipment")

# E16 inspection guard: the selected person's experience remains four bounded, personal sections.
func test_person_inspection_exposes_experience_sections() -> void:
	var sim := SimHost.new()
	sim.stage_demo(1)
	var agents: PackedInt32Array = sim.agents()
	for i in range(0, agents.size(), 24):
		if agents[i + 4] != 0:
			continue
		var d: Dictionary = sim.inspect(agents[i])
		check(d.has("observations") and d["observations"] is Array, "person inspect has observations")
		check(d.has("lessons") and d["lessons"] is Array, "person inspect has lessons")
		check(d.has("problems") and d["problems"] is Array, "person inspect has problems")
		check(d.has("reaction_trace") and d["reaction_trace"] is Array, "person inspect has reaction trace")
		check(d["observations"].size() <= 12, "observations stay bounded")
		check(d["lessons"].size() <= 8, "lessons stay bounded")
		check(d["problems"].size() <= 8, "problems stay bounded")
		check(d["reaction_trace"].size() <= 12, "reaction trace stays bounded")
		for section in [["observations", "tick"], ["lessons", "last_tested"], ["problems", "last_seen"], ["reaction_trace", "tick"]]:
			var rows: Array = d[section[0]]
			for row in range(1, rows.size()):
				check(int(rows[row - 1][section[1]]) >= int(rows[row][section[1]]), "%s are newest first" % section[0])
		return
	check(false, "experience inspection found a person")

# E16 review guard: exercise the same populated formatter used by the inspect panel.
func test_person_inspection_formats_populated_experience() -> void:
	var d := {
		"reaction_trace": [
			{"problem": "newer danger", "x": 3, "y": 4, "z": 1, "reaction": "move to safety", "outcome": "pending", "expected": "safety"},
			{"problem": "older danger", "x": 1, "y": 2, "z": 1, "reaction": "investigate", "outcome": "failed", "expected": ""},
		],
		"observations": [
			{"kind": "newer observation", "evidence": "told by Agent#7", "strength": 0.4, "full": false, "x": 3, "y": 4, "z": 1},
			{"kind": "older observation", "evidence": "directly observed", "strength": 0.9, "full": true, "x": 1, "y": 2, "z": 1},
		],
		"lessons": [
			{"cue": "newer cue", "outcome": "safety", "confidence": 0.47, "confirmations": 2, "contradictions": 0},
			{"cue": "older cue", "outcome": "harm", "confidence": 0.2, "confirmations": 0, "contradictions": 1},
		],
		"problems": [
			{"kind": "newer problem", "status": "attempted", "evidence": "inferred", "reaction": "move to safety", "outcome": "pending"},
			{"kind": "older problem", "status": "resolved", "evidence": "directly observed", "reaction": "", "outcome": "safety"},
		],
		"global_explanation": "omniscient text must not appear",
	}
	var text := Rules.format_experience(d)
	check(text.contains("newer danger at (3,4,1), chose move to safety; outcome: pending; expected safety"), "reaction formatting includes the personal attempt and expectation")
	check(text.contains("delta: newer observation, told by Agent#7 (40%) at (3,4,1)"), "observation formatting includes personal evidence and strength")
	check(text.contains("newer cue -> safety (confidence 47%; 2 confirmations, 0 contradictions)"), "lesson formatting includes the learned association")
	check(text.contains("newer problem (attempted; inferred); chose move to safety; outcome: pending"), "problem formatting includes status, evidence and reaction")
	check(not text.contains("omniscient text"), "experience formatting does not invent global explanations")
	for pair in [["newer danger", "older danger"], ["newer observation", "older observation"], ["newer cue", "older cue"], ["newer problem", "older problem"]]:
		check(text.find(pair[0]) >= 0 and text.find(pair[0]) < text.find(pair[1]), "%s stays before %s" % pair)
	var section_order := ["why they reacted:", "recent observations:", "lived lessons:", "personal problems:"]
	for i in range(1, section_order.size()):
		check(text.find(section_order[i - 1]) >= 0 and text.find(section_order[i - 1]) < text.find(section_order[i]), "%s precedes %s" % [section_order[i - 1], section_order[i]])

# #567: the Why? lens renders the simulation's decision cycle, its score terms, rejections and what
# was put off, newest first, and says plainly when nobody is traced or nothing was decided yet.
func test_why_lens_renders_the_traced_cycle() -> void:
	var cycle := {
		"tick": 900, "when": "Spring day 2 09:00", "mode": "interrupted", "interrupted": "starving: stopped talking",
		"chosen": "gather for the store at (26,22,2)",
		"because": PackedStringArray(["the store holds 0.0 days of food, under the two that secure tomorrow"]),
		"put_off": PackedStringArray(["talk with Ash: put off until food and shelter are secure"]),
		"conditions": [{"name": "hunger", "value": 0.1, "ok": true}],
		"options": [
			{"cycle": "stock_the_store", "what": "gather for the store at (26,22,2)", "base": 0.8, "emotion": 1.0, "gate": 1.0, "score": 0.8, "rejected": "", "chosen": true},
			{"cycle": "talk", "what": "talk with Ash", "base": 1.2, "emotion": 1.0, "gate": 0.05, "score": 0.06, "rejected": "", "chosen": false},
			{"cycle": "tend", "what": "tend Rowan", "base": 0.5, "emotion": 1.0, "gate": 1.0, "learned": 1.3, "score": 0.65, "rejected": "", "chosen": false},
			{"cycle": "go_for_meals", "what": "eat the meal at (9,9,2)", "base": 2.0, "emotion": 1.0, "gate": 1.0, "score": 2.0, "rejected": "no way there they know of", "chosen": false},
		],
	}
	var older := {"tick": 700, "when": "Spring day 2 08:50", "mode": "deliberate", "chosen": "rest a moment"}
	var percepts := [
		{"domain": "the ground", "what": "poor ground: crops would grow at 65% of the pace", "salience": 0.62, "noticed": true, "habituated": false},
		{"domain": "animal sign", "what": "a deer passed near", "salience": 0.21, "noticed": false, "habituated": false},
	]
	var d := {"why_traced": true, "why_cycles": [cycle, older], "why_percepts": percepts}
	var text := Rules.format_why(d)
	check(text.contains("interrupted (starving: stopped talking)"), "the lens says what cut in")
	check(text.contains("chose [color=#e8d8a0]gather for the store"), "the lens names the choice")
	check(text.contains("because the store holds 0.0 days"), "the lens gives the reasons")
	check(text.contains("talk:[/color] talk with Ash  0.06 = 1.20 x 1.00 x 0.05 x 1.00"), "an option shows its cycle and score terms")
	check(text.contains("tend Rowan  0.65 = 0.50 x 1.00 x 1.00 x 1.30"), "a learned term from a judged quarrel is shown (#568)")
	check(text.contains("x eat the meal at (9,9,2): no way there they know of"), "a rejected option says why")
	check(text.contains("put off:") and text.contains("talk with Ash: put off"), "what was put off is listed")
	check(text.find("Spring day 2 09:00, interrupted") >= 0 and text.find("Spring day 2 09:00") < text.find("Spring day 2 08:50: rest a moment"), "newest cycle first, earlier ones after")
	check(Rules.format_why({"why_traced": false}).contains("not tracing"), "an untraced person says so")
	check(Rules.format_why({"why_traced": true, "why_cycles": []}).contains("no decision since"), "an empty trace says so")
	check(text.contains("the ground: poor ground: crops would grow at 65% of the pace, 0.62, noticed"), "a noticed percept is shown (#569)")
	check(text.contains("animal sign: a deer passed near, 0.21, there, not noticed"), "a stimulus present but not noticed is shown (#569)")
	check(Rules.why_indicator("Ash") == "(o) tracing why: Ash", "the HUD shows who is traced")
	check(Rules.why_indicator("") == "", "no indicator when nobody is traced")
	var main_source := FileAccess.get_file_as_string("res://scripts/main.gd")
	check(main_source.contains("sim.set_why_target(selected_id)"), "selecting a person traces them")
	check(main_source.contains("Rules.why_indicator"), "the HUD shows the tracing indicator")

# #487: what a person was told is shown with its teller and what they made of it.
func test_told_is_shown_with_its_teller() -> void:
	var d := {"told": PackedStringArray(["foraging far afield, from Ash: would not take it (trust 0.34)"])}
	var text := Rules.format_told(d)
	check(text.begins_with("was told:"), "a told section")
	check(text.contains("from Ash: would not take it"), "the teller and the refusal are shown")
	check(Rules.format_told({}) == "", "nothing told, no section")
	var main_source := FileAccess.get_file_as_string("res://scripts/main.gd")
	check(main_source.contains("Rules.format_told(d)"), "the inspect panel shows what was told")

# #488: public person views expose bounded surface expression, never the dimensional core.
func test_surface_traits_are_bounded_and_core_is_not_exposed() -> void:
	var sim := SimHost.new()
	var rerolls := PackedInt64Array()
	for card in sim.survivor_cards(488, rerolls):
		check(String(card["traits"]).split(", ", false).size() <= 5, "setup shows at most five surface traits")
		check(not card.has("dispositions") and not card.has("core"), "setup does not expose the core pool")
	sim.stage_demo(488)
	var agents: PackedInt32Array = sim.agents()
	for i in range(0, agents.size(), 24):
		if agents[i + 4] != 0:
			continue
		var d: Dictionary = sim.inspect(agents[i])
		if d.is_empty():
			continue
		check(d["traits"].size() <= 5, "inspection shows at most five surface traits")
		check(not d.has("dispositions") and not d.has("core"), "inspection does not expose the core pool")
		return
	check(false, "a staged person exposes surface traits")

# #128/C74 M5 guard: the inspector must render the simulation's wound layer and treatment text.
func test_person_inspection_renders_body_layers_and_treatment() -> void:
	var body := Rules.format_body_summary("cut chest (to the muscle, 45%, dressed)")
	check(body.contains("Body & wounds:"), "body inspection has a wound section")
	check(body.contains("to the muscle") and body.contains("dressed"), "body inspection keeps layer and treatment")
	check(Rules.format_body_summary("") == "", "empty body summary adds no empty section")
	var main_source := FileAccess.get_file_as_string("res://scripts/main.gd")
	check(main_source.contains("Rules.format_body_summary"), "person inspector uses the body formatter")

# #561: the history reel plays the realm's years in order within a bounded time, hurries to its
# end on the first press and comes into the world on the next; it shows the recorded lines.
func test_history_reel_plays_in_order_and_skips() -> void:
	const HistoryReel := preload("res://scripts/history_reel.gd")
	check(HistoryReel.length_for(50) == HistoryReel.MIN_SECONDS, "a short history still plays a while")
	check(HistoryReel.length_for(1000) == HistoryReel.MAX_SECONDS, "a long one is bounded")
	var prev := -1
	for k in 101:
		var y := HistoryReel.year_at(k * 0.1, 10.0, 0, 500)
		check(y >= prev, "years never go back (%d after %d)" % [y, prev])
		prev = y
	check(HistoryReel.year_at(0.0, 10.0, 0, 500) == 0, "it starts at the first year")
	check(HistoryReel.year_at(10.0, 10.0, 0, 500) == 500, "it ends at the last")
	check(HistoryReel.when(500, 500) == "the year before the portal", "the last year reads as the year before the portal")
	var sim := SimHost.new()
	check(sim.opening(5, [0, 1, 2], PackedInt64Array([]), "steady") == "", "a new world")
	var reel = HistoryReel.new()
	get_root().add_child(reel)
	check(reel.start(sim), "the world has a history to show")
	var years: Array = sim.history_reel()
	check(years.size() > 0, "the world's past has lines")
	reel._process(reel.length / 2.0)
	check(reel.shown_year > 0 and reel.shown_year < reel.last, "halfway it shows a middle year, saw %d" % reel.shown_year)
	check(reel.feed.size() > 0, "and the lines of the years so far")
	var entered := [false]
	reel.finished.connect(func(): entered[0] = true)
	reel._hurry_or_enter()
	check(reel.done and reel.shown_year == reel.last, "the first press hurries to the end")
	check(not entered[0], "and does not come in yet")
	check(reel.living > 0, "the world's last year has living settlements, saw %d" % reel.living)
	var tex: Texture2D = reel.map_rect.texture
	check(tex.get_width() == int(sim.legends_map_size()) * HistoryReel.SCALE, "the map is drawn large")
	reel._hurry_or_enter()
	check(entered[0], "the next press comes into the world")
	reel.free()
	sim.free()
# #564: the chosen place on the wizard's world map stands out on ice, open water and grass. At
# each such place of a real world map the mark has pixels far from the ground's brightness (the
# old 3x3 white square had none on ice) and is drawn across at least 9 regions.
func test_world_map_mark_shows_on_every_ground() -> void:
	const Setup := preload("res://scripts/setup.gd")
	var host = SimHost.new()
	var grounds := {"ice": Color8(233, 227, 214), "ocean": Color8(34, 75, 112), "grassland": Color8(126, 169, 79)}
	for name in grounds:
		var want: Color = grounds[name]
		var found := false
		for world_seed in range(1, 9):
			var wp: Dictionary = host.world_preview(world_seed)
			var n := int(wp["size"])
			var img := Image.create_from_data(n, n, false, Image.FORMAT_RGB8, wp["rgb"])
			var at := Vector2i(-1, -1)
			for y in range(8, n - 8):
				for x in range(8, n - 8):
					if img.get_pixel(x, y).is_equal_approx(want) and img.get_pixel(x + 6, y).is_equal_approx(want) and img.get_pixel(x - 6, y).is_equal_approx(want):
						at = Vector2i(x, y)
						break
				if at.x >= 0:
					break
			if at.x < 0:
				continue
			found = true
			var before := img.duplicate()
			Setup.mark_place(img, at.x, at.y)
			var standout := 0
			var left := at.x
			var right := at.x
			for dy in range(-8, 9):
				for dx in range(-8, 9):
					var c := img.get_pixel(at.x + dx, at.y + dy)
					if c != before.get_pixel(at.x + dx, at.y + dy) and absf(c.get_luminance() - want.get_luminance()) >= 0.4:
						standout += 1
					if dy == 0 and c != before.get_pixel(at.x + dx, at.y):
						left = mini(left, at.x + dx)
						right = maxi(right, at.x + dx)
			check(standout >= 9, "%s (seed %d): %d pixels of the mark stand out from the ground" % [name, world_seed, standout])
			check(right - left + 1 >= 9, "%s: the mark is %d regions across" % [name, right - left + 1])
			break
		check(found, "some world of seeds 1-8 has %s to test on" % name)
	host.free()
	# At the map's edge it is clipped, not an error.
	var edge := Image.create(192, 192, false, Image.FORMAT_RGB8)
	Setup.mark_place(edge, 0, 191)
	check(edge.get_pixel(0, 191) == Color(1, 1, 1), "the mark's centre at the corner")

# #562: those away are ringed on the Legends map of the present at the places the valley knows,
# coloured by how it goes for them (threatened wins a shared place), and listed under it; one
# whose place the valley does not know is listed but not drawn.
func test_away_marks_and_lines_on_the_world_map() -> void:
	const AwayMap := preload("res://scripts/away_map.gd")
	var img := Image.create(64, 64, false, Image.FORMAT_RGB8)
	img.fill(Color8(126, 169, 79))
	var before := img.duplicate()
	var rows := [
		{"name": "Ada", "place": "Ivouth", "region": 20 * 64 + 10, "reaction": "welcomed", "line": "at Ivouth to see it for themselves"},
		{"name": "Bram", "place": "Ivouth", "region": 20 * 64 + 10, "reaction": "threatened", "line": "at Ivouth; they hold a grudge against the valley"},
		{"name": "Cel", "place": "", "region": -1, "reaction": "threatened", "line": "held captive at a place the valley does not know"},
	]
	AwayMap.draw(img, rows)
	var got := img.get_pixel(10, 20)
	var want := AwayMap.colour_of("threatened")
	check(absf(got.r - want.r) < 0.01 and absf(got.g - want.g) < 0.01 and absf(got.b - want.b) < 0.01, "the shared place is marked threatened, saw %s" % got)
	var changed := 0
	for y in 64:
		for x in 64:
			if img.get_pixel(x, y) != before.get_pixel(x, y):
				changed += 1
				check(absi(x - 10) <= 4 and absi(y - 20) <= 4, "only the known place is marked, not (%d,%d)" % [x, y])
	check(changed > 0, "something was marked")
	var text: String = AwayMap.lines(rows)
	for n in ["Ada", "Bram", "Cel", "grudge", "does not know"]:
		check(text.contains(n), "the lines name %s" % n)
	check(AwayMap.lines([]) == "", "nobody away: no lines")

# #603: the sim steps on its own thread. Stepped there in uneven frames, a world is the same
# tick for tick as one stepped in place; a step returns without waiting and the view reads the
# snapshot meanwhile; and a save made through the threaded host, loaded and run on, gives the
# world the continuous run gives (the #558 resume guard, through the host).
func test_sim_thread_steps_the_same_world_and_saves_exactly() -> void:
	var a := SimHost.new()
	var b := SimHost.new()
	check(a.is_threaded(), "the sim steps on its own thread by default")
	b.set_threaded(false)
	check(not b.is_threaded(), "and in place when asked")
	check(a.opening(5, PackedInt64Array([0, 1, 2]), PackedInt64Array([]), "steady") == "", "a world")
	check(b.opening(5, PackedInt64Array([0, 1, 2]), PackedInt64Array([]), "steady") == "", "the same world")
	var frames := [1, 3, 0, 10, 1, 1, 7, 20, 2, 40, 1, 13]
	var total := 0
	for k in frames:
		a.step(k)
		b.step(k)
		total += k
		# Never waiting: the view's reads answer at once from the snapshot or the live world.
		check(a.agents().size() > 0, "the view reads people while ticks run")
	a.sync()
	check(a.tick() == b.tick(), "every tick asked for ran: %d and %d" % [a.tick(), b.tick()])
	check(a.state_digest() == b.state_digest(), "the threaded world is the in-place world at tick %d" % a.tick())
	var path := ProjectSettings.globalize_path("user://gd_test_603.hfsave")
	check(a.save_to(path), "a save through the threaded host")
	var st := ""
	for i in 3000:
		st = a.save_status()
		if st != "" and st != "saving":
			break
		OS.delay_msec(10)
	check(st == "ok", "the save is written, saw %s" % st)
	var c := SimHost.new()
	check(c.load_from(path) == "", "the save loads")
	check(c.state_digest() == a.state_digest(), "loaded as saved")
	for i in 30:
		a.step(9)
		c.step(9)
		b.step(9)
	a.sync()
	c.sync()
	check(c.tick() == b.tick() and c.state_digest() == b.state_digest(), "the resumed world runs on as the continuous one")
	check(a.state_digest() == b.state_digest(), "and so does the one that saved")
	a.free()
	b.free()
	c.free()

func check(ok: bool, what: String) -> void:
	if not ok:
		failed += 1
		print("  failed: %s" % what)

## One agent row of the bridge's layout (lib.rs agents(), AGENT_STRIDE 23).
func _row(id: int, x: int, y: int, z: int, fx: int, fy: int, prog: int, pose := 1) -> PackedInt32Array:
	var r := PackedInt32Array()
	r.resize(23)
	r[0] = id; r[1] = x; r[2] = y; r[3] = z; r[12] = fx; r[13] = fy; r[14] = prog; r[15] = pose
	return r

# #183: a walking person is picked where they are drawn, not on the tile they are entering.
func test_pick_uses_the_drawn_position() -> void:
	# Walking from (10,10) to (11,10), a quarter of the way: drawn at x = 10.25 tiles.
	var a := _row(7, 11, 10, 0, 10, 10, 250)
	var drawn := Vector2(10.25 * 32 + 16, 10 * 32 + 16 - 8)
	check(Rules.pick(a, 23, 0, drawn, 1.0) == 7, "a click on the drawn body picks them")
	var tile := Vector2(11 * 32 + 16, 10 * 32 + 16)
	check(Rules.pick(a, 23, 0, tile, 1.0) == -1, "a click on the tile they are only entering does not")

# #183: the pick radius is in screen pixels, so it is not tiny at 0.5x nor huge at 4x.
func test_pick_radius_follows_the_zoom() -> void:
	var a := _row(3, 5, 5, 0, 5, 5, 1000)
	var c := Vector2(5 * 32 + 16, 5 * 32 + 8)
	check(Rules.pick(a, 23, 0, c + Vector2(30, 0), 0.5) == 3, "30 world px is 15 screen px at 0.5x")
	check(Rules.pick(a, 23, 0, c + Vector2(30, 0), 2.0) == -1, "30 world px is 60 screen px at 2x")
	check(Rules.pick(a, 23, 1, c, 1.0) == -1, "never someone on another level")

# #183: wheel zoom lands on whole-pixel levels and keeps the point under the pointer.
func test_zoom_steps_are_whole_pixels() -> void:
	var z := 1.0
	var seen := []
	for k in 6:
		z = Rules.next_zoom(z, 1)
		seen.append(z)
	check(seen == [2.0, 3.0, 4.0, 4.0, 4.0, 4.0], "in from 1x: %s" % [seen])
	check(Rules.next_zoom(1.0, -1) == 0.5 and Rules.next_zoom(0.5, -1) == 0.5, "out to 0.5x and no further")
	check(Rules.next_zoom(1.25, 1) == 2.0 and Rules.next_zoom(1.25, -1) == 1.0, "off the ladder, onto it")
	var cam := Vector2(1000, 1000)
	var at := Vector2(1200, 900)
	var after := Rules.zoom_about(cam, at, 1.0, 2.0)
	# The pointer's screen offset from the centre, (at - cam) * zoom, is the same after.
	check(((at - cam) * 1.0).is_equal_approx((at - after) * 2.0), "the point under the pointer stays")

# #184: a ladder draws as a ladder; fire uses every frame of its sheet.
func test_ladders_and_fire_frames() -> void:
	check(Rules.stair_sheet(8) == "ladder", "shape 8 is a ladder")
	check(Rules.stair_sheet(5) == "stairs_up" and Rules.stair_sheet(6) == "stairs_down" and Rules.stair_sheet(7) == "stairs_both", "stairs")
	var frames := {}
	for t in 8:
		frames[Rules.fire_frame(float(t), 0, 128, 32)] = true
	check(frames.size() == 4, "a 4-frame fire sheet shows 4 frames, saw %d" % frames.size())

# #423: every kind of animal the sim sends has its sheet, the three new bodies at the end in
# the sim's order (agents.rs Species, appended), so none is drawn as the deer fallback.
func test_every_animal_kind_has_a_sheet() -> void:
	var names: Array = WorldView.ANIMAL_NAME
	check(names.slice(14) == ["slime", "rats", "centipede", "blind_cave_bear", "cave_croc"], "the #423 beasts and #336's cave kinds follow the burrower")
	for k in range(1, names.size()):
		var n: String = names[k]
		var path := "res://assets/animals/%s.png" % n
		check(ResourceLoader.exists(path) or FileAccess.file_exists(path), "a sheet for %s" % n)

## #192 (WP-08 8.4): the activity emote is the work's own tool icon; every kind the bridge can
## send (agents row field 24, an item kind + 1) must have art in the small or the full set.
func test_activity_emote_art_exists() -> void:
	for slug in ["tool", "sledge_hammer", "hammer", "saw", "vessel", "bucket", "poultice",
			"weapon", "record"]:
		check(WorldView.ITEM_NAME.has(slug), "%s is in ITEM_NAME" % slug)
		var small := "res://assets/items32/%s.png" % slug
		var full := "res://assets/items/%s.png" % slug
		check(ResourceLoader.exists(small) or FileAccess.file_exists(small)
			or ResourceLoader.exists(full) or FileAccess.file_exists(full),
			"an icon for the %s emote" % slug)

# #187: an animal fighting moves; struck and eating are not the plain idle.
func test_animal_poses_are_not_all_frame_zero() -> void:
	var fight := {}
	for k in 8:
		fight[Rules.animal_frame(6, k * 0.07, 3)] = true
	check(fight.size() >= 2, "a fighting animal changes frame")
	for f in fight:
		check(f >= Rules.A_WALK and f < Rules.A_WALK + 6, "fight frames are on the walk cycle")
	check(Rules.animal_frame(7, 0.0, 0) == 1, "struck: a start")
	check(Rules.animal_frame(4, 0.0, 0) == Rules.A_SLEEP, "asleep")

# #485 review: a wolf-shaped person's emitted pose 5 follows the animal eating renderer.
func test_beast_shape_row_selects_animal_eating_frames() -> void:
	var row := _row(3, 0, 0, 0, 0, 0, 1000, 5)
	row[4] = 3 # Species::Wolf; the Rust row guard covers the shape/pose pairing.
	var frames := {}
	for k in 8:
		var t := k * 0.25
		var frame: int = Rules.animal_frame_for_row(row, 0, t)
		check(frame == Rules.animal_frame(5, t, 3), "row and animal pose select the same frame")
		frames[frame] = true
	check(frames.size() == 2, "the emitted animal eating pose slow-bobs both idle frames")
	check(Rules.everyday_frame(row[15], 1.0) == -1, "the wolf row never enters the people eat cycle")

# #185: the dead are drawn falling, then lying.
func test_the_dead_have_frames() -> void:
	check(Rules.dead_frame(8) == Rules.P_FALL, "pose 8 falls")
	check(Rules.dead_frame(9) == Rules.P_LYING, "pose 9 lies")
	check(Rules.dead_frame(1) == -1, "the living are not")

# #485: talking and eating use their own complete cycles; idle and sitting cannot borrow them.
func test_everyday_poses_use_three_distinct_frames() -> void:
	for pose in [10, 11]:
		var frames := {}
		for k in 12:
			frames[Rules.everyday_frame(pose, k * 0.1)] = true
		check(frames.size() == 3, "pose %d shows all three action frames" % pose)
	for pose in [0, 5, 8, 9]:
		check(Rules.everyday_frame(pose, 1.0) == -1, "pose %d has no social/eating gesture" % pose)

# #485: sleep breathes and now and then stirs, staggered per sleeper; tending and drinking run
# their own three held frames; the sheets carry the columns the client asks for.
func test_sleep_tend_and_drink_cycles() -> void:
	var seen := {}
	for k in 400:
		seen[Rules.sleep_frame(k * 0.1, 3)] = true
	check(seen.size() == 3 and seen.has(Rules.P_SLEEP) and seen.has(Rules.P_SLEEP2) and seen.has(Rules.P_SLEEP3), "sleep shows rest, breath and stir: %s" % [seen.keys()])
	var rest := 0
	for k in 400:
		if Rules.sleep_frame(k * 0.1, 3) == Rules.P_SLEEP:
			rest += 1
	check(rest > 200, "a sleeper is mostly at rest (%d of 400)" % rest)
	var differ := false
	for k in 40:
		if Rules.sleep_frame(k * 0.1, 1) != Rules.sleep_frame(k * 0.1, 2):
			differ = true
	check(differ, "two sleepers do not breathe in step")
	for base in [Rules.P_TEND, Rules.P_DRINK]:
		var cols := {}
		for k in 60:
			cols[Rules.held_frame(base, k * 0.05, Rules.TEND_TIMES if base == Rules.P_TEND else Rules.DRINK_TIMES)] = true
		check(cols.keys().size() == 3 and cols.has(base) and cols.has(base + 2), "cycle at %d shows its three frames" % base)
	for kind in ["murrow", "skarri", "screeling", "amphibious", "below"]:
		for gene in [0, 1, 2, 9]:
			var sheet := Rules.marking_sheet(kind, gene, "slim_child")
			check(ResourceLoader.exists("res://assets/" + sheet), "%s marking sheet exists for gene %d" % [kind, gene])
	check(Rules.marking_sheet("skarri", 4, "average_adult") == "people/marks_skarri_2_average_adult.png", "the gene picks the pattern")
	var layout = JSON.parse_string(FileAccess.get_file_as_string("res://assets/people/swaps.json"))["layout"]["columns"]
	check(layout[Rules.P_SLEEP] == "sleep" and layout[Rules.P_SLEEP2] == "sleep2" and layout[Rules.P_SLEEP3] == "sleep3", "sleep columns match the sheets")
	check(layout[Rules.P_TEND] == "tend1" and layout[Rules.P_DRINK] == "drink1" and layout.size() == Rules.P_DRINK + 3, "tend and drink columns match the sheets")

# #505: a built wall casts contact shadows only onto open ground east and south of it, below
# its face, and never onto a neighbouring wall.
func test_wall_contact_shadows_fall_only_on_open_ground() -> void:
	check(Rules.wall_contact_shadows(false, false, 10).is_empty(), "a wall boxed in by walls casts nothing")
	var east := Rules.wall_contact_shadows(true, false, 10)
	check(east.size() == 2, "an open east side gets a two-step shadow")
	for e in east:
		var r: Rect2 = e[0]
		check(r.position.x >= Rules.TILE and r.position.y >= 0 and r.end.y <= Rules.TILE, "the east shadow lies on the east tile only")
	var south := Rules.wall_contact_shadows(false, true, 12)
	for e in south:
		var r: Rect2 = e[0]
		check(r.position.y >= Rules.TILE + 12, "the south shadow starts below the wall's face")
		check(float(e[1]) > 0.0 and float(e[1]) < 0.5, "shadows darken without blacking out the ground")
	check(Rules.wall_contact_shadows(true, true, 12).size() == 5, "an open corner gets the corner fill")

func _effect_agent(id: int, species: int, pose: int, work := 0) -> PackedInt32Array:
	var row := PackedInt32Array()
	row.resize(EffectsView.AGENT_STRIDE)
	row[0] = id
	row[1] = 4
	row[2] = 4
	row[4] = species
	row[15] = pose
	row[20] = work
	return row

func _effects_fixture() -> Array:
	var view := EffectsView.new()
	var fx := Effects.new()
	get_root().add_child(fx)
	fx.init(view)
	return [fx, view]

# #414 effects slice: the client decorates only bridge-backed event names and keeps the
# production effect pool bounded, with material-aware variants selected from visible tiles.
func test_effects_are_bounded_and_fact_backed() -> void:
	check(Effects.MAX_ACTIVE == 48, "effect pool is bounded")
	check(Effects.effect_for_sound("flare") == "fire_flare", "fire start has a visual accent")
	check(Effects.effect_for_sound("douse") == "environmental_hit", "douse has environmental feedback")
	check(Effects.effect_for_sound("projectile") == "weapon_impact", "projectile hook maps to impact")
	check(Effects.effect_for_sound("psychic") == "resonance", "psychic hook maps to resonance")
	check(Effects.effect_for_sound("made_up_event") == "", "unsupported facts do not create effects")
	check(Effects.material_family(13) == "water", "water feedback keeps its material family")
	check(Effects.material_family(5) == "stone", "rock feedback keeps its material family")
	check(Effects.material_family(14) == "wood", "wood feedback keeps its material family")
	check(Effects.material_family(15) == "stone", "stone-floor feedback is stone")
	check(Effects.material_family(23) == "earth", "packed-earth feedback is not wood")
	check(Effects.material_family(26) == "stone", "scree feedback is broken stone")
	check(Effects.material_family(62) == "earth", "bone-moss feedback keeps its ground family")

func test_effect_poses_never_invent_weapon_wounds() -> void:
	for spec in [[0, 6], [0, 7], [1, 6], [1, 7]]:
		var fixture := _effects_fixture()
		var fx = fixture[0]
		var view = fixture[1]
		view.agents = _effect_agent(42, spec[0], spec[1])
		fx.update(0.0)
		check(fx.active_count() == 1, "species %d pose %d emits one accent" % spec)
		if fx.active_count() == 1:
			check(fx._active[0].get("kind", "") == "environmental_hit", "species %d pose %d uses a generic contact/wound accent, not a weapon event" % spec)
		fx.free()

func test_effect_pool_expires_and_never_exceeds_its_capacity() -> void:
	var fixture := _effects_fixture()
	var fx = fixture[0]
	for k in range(Effects.MAX_ACTIVE + 12):
		fx._spawn("fire_flare", Vector2(k, k), Color.WHITE)
	check(fx.active_count() == Effects.MAX_ACTIVE, "spawning past capacity retains exactly the pool cap")
	var visible := 0
	for sprite in fx._pool:
		if sprite.visible:
			visible += 1
	check(visible == Effects.MAX_ACTIVE and fx._pool.size() == Effects.MAX_ACTIVE, "active effects and allocated sprites stay within the fixed pool")
	fx._advance(1.0)
	check(fx.active_count() == 0, "all short-lived effects expire")
	for sprite in fx._pool:
		check(not sprite.visible, "expiry returns its sprite to the pool")
	fx.free()

func test_effect_history_prunes_departed_and_excess_ids() -> void:
	var fixture := _effects_fixture()
	var fx = fixture[0]
	var view = fixture[1]
	view.agents = PackedInt32Array()
	for id in range(Effects.MAX_HISTORY + 12):
		view.agents.append_array(_effect_agent(id, 0, 2, 1))
	fx._scan_agents()
	check(fx._last_pose.size() <= Effects.MAX_HISTORY, "pose history has a hard bound")
	check(fx._last_work_stamp.size() <= Effects.MAX_HISTORY, "work history has a hard bound")
	fx._advance(1.0)
	fx._scan_agents()
	check(fx.active_count() == 0, "agents beyond the history cap do not respawn accents every frame")
	view.agents = PackedInt32Array()
	for id in range(8):
		view.agents.append_array(_effect_agent(id, 0, 1))
	fx._scan_agents()
	check(fx._last_pose.size() == 8, "departed agent ids are pruned from pose history")
	check(fx._last_work_stamp.size() == 8 and not fx._last_work_stamp.has(8), "departed agent ids are pruned from work history")
	fx._advance(1.0)
	view.agents = _effect_agent(0, 0, 7)
	fx.update(0.0)
	check(fx.active_count() == 1, "a returning id is not suppressed by stale pose history")
	fx.free()

func test_world_load_resets_effect_pool_and_history() -> void:
	var fixture := _effects_fixture()
	var fx = fixture[0]
	var view = fixture[1]
	view.agents = _effect_agent(42, 0, 7)
	fx.update(0.0)
	fx._last_work_stamp[42] = 9
	fx._last_resonance_stamp = 12
	var sim := SimHost.new()
	check(sim.opening(1, [0, 1, 2], PackedInt64Array([]), "steady") == "", "world-reset fixture opens")
	var world := WorldView.new()
	world.sim = sim
	world.effects = fx
	world.reset_world()
	check(fx.active_count() == 0, "world load clears active effects")
	check(fx._last_pose.is_empty() and fx._last_work_stamp.is_empty(), "world load clears agent history")
	check(fx._last_resonance_stamp == -1, "world load clears resonance history")
	for sprite in fx._pool:
		check(not sprite.visible, "world load hides every pooled sprite")
	world.free()
	fx.free()
	sim.free()

# Double-density art shares one foot anchor with ordinary clothing, injuries and equipment.
func test_expressive_biped_layers_follow_the_same_cell_and_reuse() -> void:
	var world := WorldView.new()
	world.assets = preload("res://scripts/assets.gd").new()
	world.species_art[0] = {"body": "human", "plan": "biped"}
	var entities := preload("res://scripts/render/entity_layer.gd").new()
	entities.v = world
	var person := entities._new_person(entities)
	var appearance := PackedInt32Array()
	appearance.resize(world.AGENT_STRIDE)
	appearance[6] = 1 # average adult
	appearance[8] = 1 # bowl cut
	appearance[10] = 7 # vest
	appearance[17] = 1 # existing bandage on the ordinary resolution sheet
	appearance[19] = -1
	var plan := entities.dress_person(person, appearance, 0, 7)
	for facing in 4:
		entities.set_person_frame(person, plan, entities.P_CARRY, facing)
		for layer_name in ["body", "hair", "cloth", "band"]:
			var layer: Sprite2D = person.get_node(layer_name)
			var cell := 48 if layer_name == "band" else 96
			check(layer.visible and layer.region_rect == Rect2(entities.P_CARRY * cell, facing * cell, cell, cell), "correct source cell: " + layer_name)
			check(layer.offset * layer.scale == Vector2(-8, -48), "same foot anchor: " + layer_name)
	appearance[8] = 0 # same pooled node, next person is a child with cropped hair
	appearance[16] = 1
	entities.dress_person(person, appearance, 0, -1)
	entities.set_person_frame(person, plan, entities.P_IDLE, 0)
	var body: Sprite2D = person.get_node("body")
	check(body.scale == Vector2.ONE and body.region_rect.size == Vector2(48,48), "ordinary appearance restored on reuse")
	entities.free()
	world.free()

func test_long_hair_on_the_broad_rig_keeps_the_load_at_the_hands() -> void:
	var world := WorldView.new()
	world.assets = preload("res://scripts/assets.gd").new()
	world.species_art = {0: {"body": "human", "plan": "biped"}, 1: {"body": "hollin", "plan": "biped"}}
	var entities := preload("res://scripts/render/entity_layer.gd").new()
	entities.v = world
	var person := entities._new_person(entities)
	var appearance := PackedInt32Array()
	appearance.resize(world.AGENT_STRIDE)
	appearance[6] = 2 # broad build
	appearance[19] = -1
	var ordinary_hands: Image
	for variant in ["short hair", "long hair", "child", "hollin"]:
		appearance[8] = 0 if variant == "short hair" else 2
		appearance[16] = 1 if variant == "child" else 0
		appearance[22] = 1 if variant == "hollin" else 0
		var plan := entities.dress_person(person, appearance, 0, 7)
		entities.set_person_frame(person, plan, entities.P_CARRY, 0)
		var expected := 48 if variant in ["child", "hollin"] else 96
		for layer_name in ["body", "hair"]:
			var layer: Sprite2D = person.get_node(layer_name)
			check(layer.visible and layer.region_rect.size == Vector2(expected, expected),
					variant + " selects the fitted " + layer_name + " cell")
			check(layer.region_rect.size * layer.scale == Vector2(48, 48),
					variant + " keeps the logical rig size")
		if variant in ["short hair", "long hair"]:
			var body: Sprite2D = person.get_node("body")
			var skin := body.texture.get_image().get_region(Rect2i(body.region_rect))
			skin.resize(48, 48, Image.INTERPOLATE_NEAREST)
			# Compare the band of skin alongside the held load in logical coordinates.
			var anchor := Vector2i(entities._carry_at(0, plan) - body.offset * body.scale)
			var hands := skin.get_region(Rect2i(anchor + Vector2i(-6, 2), Vector2i(26, 8)))
			if variant == "short hair":
				var original := world._swapped("people/body_broad_adult.png", "skin", appearance[7]).get_image()
				var cell := original.get_region(Rect2i(entities.P_CARRY * 48, 0, 48, 48))
				ordinary_hands = cell.get_region(Rect2i(anchor + Vector2i(-6, 2), Vector2i(26, 8)))
				check(ordinary_hands.get_used_rect().has_area() and hands.get_data() == ordinary_hands.get_data(),
						"the short-haired broad hands meet the original load")
			else:
				check(hands.get_data() == ordinary_hands.get_data(), "the broad hands still meet the load")
	entities.free()
	world.free()

func test_inherited_adult_hair_covers_every_build_and_direction() -> void:
	var world := WorldView.new()
	world.assets = preload("res://scripts/assets.gd").new()
	world.species_art[0] = {"body": "human", "plan": "biped"}
	var entities := preload("res://scripts/render/entity_layer.gd").new()
	entities.v = world
	var person := entities._new_person(entities)
	var appearance := PackedInt32Array()
	appearance.resize(world.AGENT_STRIDE)
	appearance[19] = -1
	var hair: Sprite2D = person.get_node("hair")
	var body: Sprite2D = person.get_node("body")
	for build in 3:
		appearance[6] = build
		var silhouettes := {}
		for style in 10:
			appearance[8] = style
			var plan := entities.dress_person(person, appearance, 0, 7)
			var sizes_ok := true
			var facing_shapes := []
			for facing in 4:
				entities.set_person_frame(person, plan, entities.P_CARRY, facing)
				for layer: Sprite2D in [hair, body]:
					sizes_ok = sizes_ok and layer.visible and layer.region_rect.size == Vector2(96, 96)
					sizes_ok = sizes_ok and layer.scale * layer.offset == Vector2(-8, -48)
				if hair.texture != null:
					var image := hair.texture.get_image().get_region(Rect2i(hair.region_rect))
					facing_shapes.append(hash(image.get_data()))
			check(sizes_ok, "adult body and hair fit all four carry poses: %d/%d" % [build, style])
			silhouettes[hash(facing_shapes)] = true
		check(silhouettes.size() == 10, "each inherited hairstyle draws differently on build %d" % build)
	entities.free()
	world.free()

func test_arm_bruise_follows_side_and_clears_when_pooled() -> void:
	var world := WorldView.new()
	world.assets = preload("res://scripts/assets.gd").new()
	world.species_art[0] = {"body": "human", "plan": "biped"}
	var entities := preload("res://scripts/render/entity_layer.gd").new()
	entities.v = world
	var person := entities._new_person(entities)
	var appearance := PackedInt32Array()
	appearance.resize(world.AGENT_STRIDE)
	appearance[6] = 2 # Rosun wears a broad adult robe.
	appearance[10] = 6
	appearance[17] = 1
	appearance[19] = -1
	for flag in [2, 1]:
		world.arm_bruises[0] = flag
		var plan := entities.dress_person(person, appearance, 0, -1)
		var near_side := 2 if flag == 2 else 1
		var bruise: Sprite2D = person.get_node("bruise_right" if flag == 2 else "bruise_left")
		check(bruise.visible and not person.get_node("band").visible, "untreated arm bruise wears no head dressing")
		var image := bruise.texture.get_image()
		for facing in 4:
			entities.set_person_frame(person, plan, 4, facing)
			var mark := image.get_region(Rect2i(bruise.region_rect)).get_used_rect()
			if facing in [1, 2]:
				check(mark.has_area() == (facing == near_side), "the far wrist is occluded in profile")
			else:
				check(mark.has_area() and (mark.get_center().x < 24) == ((facing == 0) == (flag == 2)), "front and back show the anatomical side")
	world.arm_bruises.clear()
	appearance[17] = 0
	entities.dress_person(person, appearance, 0, -1)
	for layer in ["bruise_left", "bruise_right", "band"]:
		check(not person.get_node(layer).visible, "cleared wound is absent after sprite reuse")
	entities.free()
	world.free()

func test_building_stack_tracks_real_count_and_resets_when_pooled() -> void:
	var entities := preload("res://scripts/render/entity_layer.gd").new()
	var carry := Sprite2D.new()
	carry.texture = ImageTexture.create_from_image(Image.create_empty(32, 32, false, Image.FORMAT_RGBA8))
	carry.scale = Vector2(0.5, 0.5)
	for count in [1, 3, 2, 20, 1, 0]:
		entities._set_carry_stack(carry, "log", count, 0.5)
		var extras := 0
		for piece in carry.get_children():
			if piece.visible:
				extras += 1
				check(piece.texture == carry.texture and piece.position != Vector2.ZERO, "same material, distinct silhouette")
		check(extras == clampi(count - 1, 0, 2), "carried stack follows current inventory on reuse")
	entities._set_carry_stack(carry, "plank", 2, 0.5)
	var second: Sprite2D = carry.get_child(0)
	check(carry.region_enabled and carry.region_rect == Rect2(0, 6, 32, 10), "one board face per held plank")
	check(second.visible and second.region_rect == carry.region_rect and second.self_modulate == carry.self_modulate, "both boards keep the same wood")
	check(second.position * carry.scale == Vector2(4, -4), "second board exposes its end")
	entities._set_carry_stack(carry, "log", 2, 0.5)
	check(not carry.region_enabled and not second.region_enabled and carry.offset == Vector2.ZERO and carry.scale == Vector2(0.5, 0.5), "next load uses its whole icon")
	check(carry.self_modulate == Color.WHITE and second.self_modulate == Color.WHITE, "next load keeps its usual colours")
	entities._set_carry_stack(carry, "root", 3, 0.5)
	for piece in carry.get_children():
		check(not piece.visible, "a subsequent root load uses its own single icon")
	carry.free()
	entities.free()

# #181: the HUD keeps clear of the inspect panel.
func test_hud_keeps_clear_of_the_inspector() -> void:
	for w in [1280.0, 1600.0, 1920.0]:
		var hw := Rules.hud_width(w, true)
		check(10.0 + hw <= w - Rules.INSPECT_W - 10.0 + 0.01, "at %d wide the HUD ends before the panel" % w)
	check(Rules.hud_width(1600.0, false) == Rules.HUD_MAX, "capped when the panel is closed")

# #218: the HUD wraps only between fields.
func test_hud_wraps_between_fields_only() -> void:
	var line := Rules.hud_line(["Ashcross", "step 0.00 ms", "[?] keys", ""])
	check(not line.contains("step 0.00"), "no plain space inside a field: %s" % line)
	check(line.contains("step\u00A00.00\u00A0ms"), "the words of a field are held together")
	check(line.contains("   [?]\u00A0keys"), "fields keep their three-space gap")
	check(not line.ends_with("   "), "an empty field adds nothing")

# #221: the inspect panel runs to the bottom of the canvas.
func test_inspect_panel_reaches_the_bottom() -> void:
	check(Rules.inspect_bottom(720.0) == -10.0 and Rules.inspect_bottom(900.0) == -10.0, "anchored to the bottom, a margin above it")

# #182: the feed says how many quieter lines it let pass, and a click finds its line.
func test_feed_counts_what_it_drops() -> void:
	var t := Rules.feed_text([{"text": "a"}, {"text": "b"}], 3)
	check(t.contains("+3 quieter lines"), "dropped lines are counted: %s" % t)
	check(not Rules.feed_text([{"text": "a"}], 0).contains("quieter"), "no note when none were dropped")
	check(not Rules.feed_text([{"text": "a"}], 0).ends_with("\n"), "no empty last line to pad the box")
	check(Rules.feed_line_at(25.0, 20.0, 3) == 1, "second line")
	check(Rules.feed_line_at(70.0, 20.0, 3) == -1, "below the last line")

# #179: a roof sorts on its south edge: under it whoever stands on its rows, in front of it
# whoever stands south of the house.
func test_roofs_sort_with_the_people() -> void:
	var south_row := 12
	var roof_y: float = RoofLayer.sort_y(south_row)
	# A person's foot on tile row y is drawn at (y + 1) * TILE - 1 (entity_layer.gd _agent).
	var inside := (south_row + 1) * 32 - 1
	var outside := (south_row + 2) * 32 - 1
	check(inside < roof_y, "someone on the roof's south row is under it")
	check(outside > roof_y, "someone a row south of the house is in front of it")

# #180: a hearth inside a stone house lights the house and its walls, not the field outside.
func test_walls_stop_light() -> void:
	var w := 12
	var h := 12
	var opaque := PackedByteArray()
	opaque.resize(w * h)
	# A house of walls from (2,2) to (8,8), a door on the south wall at (5,8).
	for k in range(2, 9):
		for p in [Vector2i(k, 2), Vector2i(k, 8), Vector2i(2, k), Vector2i(8, k)]:
			opaque[p.y * w + p.x] = 1
	var at := func(x: float, y: float) -> Vector2: return Vector2(x * 32 + 16, y * 32 + 16)
	var hearth: Vector2 = at.call(5, 5)
	check(Rules.lit(opaque, w, h, hearth, at.call(4, 6)), "the room is lit")
	check(Rules.lit(opaque, w, h, hearth, at.call(8, 5)), "the wall's inner face is lit")
	check(not Rules.lit(opaque, w, h, hearth, at.call(10, 5)), "the field beyond the east wall is dark")
	check(not Rules.lit(opaque, w, h, hearth, at.call(5, 10)), "a shut door stops it too")
	check(not Rules.lit(opaque, w, h, hearth, at.call(9, 9)), "nor round the corner")
	# What stops light: walls, rock and soil faces (shape 2) and doors; not floors, ramps, stairs.
	check(Rules.blocks_light(2 << 16), "a wall")
	check(Rules.blocks_light((1 << 16) | (0x08 << 24)), "a door")
	check(not Rules.blocks_light(1 << 16) and not Rules.blocks_light(3 << 16) and not Rules.blocks_light(0), "floor, ramp, open")


# FIXLOG 2026-10-30 fix-506-underground-context (#506): exercise the real covered-tile path,
# including both sides of the shader contract rather than a standalone constant range.
func test_underground_ambient_keeps_caves_readable() -> void:
	var view := UndergroundView.new()
	view.viewport = get_root()
	var layer := LightLayer.new()
	layer.init(view)
	layer.mat.set_shader_parameter("under_level", -1.0)
	layer.mat.set_shader_parameter("light_count", 0)
	layer.update(0.0)

	var sky_pixel := layer.sky_tex.get_image().get_pixel(0, 0)
	check(sky_pixel.r == 0.0, "the z=-2 scenario is covered, with no sky or point light")
	var runtime_level = layer.mat.get_shader_parameter("under_level")
	check(runtime_level is float or runtime_level is int, "LightLayer.update assigns under_level")
	if runtime_level is float or runtime_level is int:
		check(is_equal_approx(float(runtime_level), 0.18), "the live unlit-cavern grade is 0.18")

	var declaration := RegEx.new()
	declaration.compile("uniform\\s+float\\s+under_level\\s*=\\s*([0-9.]+)\\s*;")
	var found := declaration.search(layer.mat.shader.code)
	check(found != null, "the active grade shader declares an under_level default")
	if found != null:
		check(is_equal_approx(float(found.get_string(1)), 0.18), "the shader fallback is also 0.18")
	layer.free()


# #329: the announcement filter's four modes.
func test_announcements_follow_the_chosen_mode() -> void:
	check(Rules.announce(0, 3, "fought"), "majors: a major line is told")
	check(not Rules.announce(0, 2, "born"), "majors: a notable birth is not")
	check(Rules.announce(1, 2, "born"), "life and death: a birth is told")
	check(not Rules.announce(1, 2, "taught"), "life and death: a lesson is not")
	check(Rules.announce(2, 2, "taught"), "notable: every notable line")
	check(not Rules.announce(3, 3, "died"), "nothing: not even a death")

# #413 A1 & A4: the bottom bar covers every keyboard command with a tooltip naming its key,
# and a pointer move under DRAG_PX stays a click while a larger move becomes a camera drag.
func test_button_bar_and_drag_threshold() -> void:
	var items := Rules.bar_items()
	check(items.size() >= 19, "bottom bar has buttons for all commands, saw %d" % items.size())
	var ids := {}
	for it in items:
		ids[it["id"]] = true
		check(str(it["tip"]).contains(str(it["key"])), "button %s tooltip names its key %s" % [it["id"], it["key"]])
	for req in ["pause", "spd1", "spd2", "spd3", "z_down", "z_up", "zoom_out", "zoom_in", "home", "director", "favs", "history", "timelapse", "legends", "known", "announce", "pause_major", "bookmarks", "save", "load", "settings", "help"]:
		check(ids.has(req), "bar has %s" % req)
	check(not Rules.is_drag(Vector2(100, 100), Vector2(103, 102)), "small twitch is still a click")
	check(Rules.is_drag(Vector2(100, 100), Vector2(108, 100)), "6+ px move is a camera drag")

# #413 A3 & A6: block inspection reports ground, water kind & depth, and minerals/strata;
# favourites toggle, report live status, and survive save and load.
func test_favourites_persistence_and_block_inspect() -> void:
	var sim := SimHost.new()
	var err := sim.opening(1, [0, 1, 2], PackedInt64Array([]), "steady")
	check(err == "", "opening seed 1 succeeds")
	var fs := sim.settlement_centre()
	var blk: Dictionary = sim.inspect_block(fs.x, fs.y, fs.z)
	check(not blk.is_empty(), "inspect_block returns a dictionary")
	check(str(blk.get("ground", "")) != "", "block inspection reports ground material")
	check(str(blk.get("water", "")) != "", "block inspection reports water status")
	var water_kind := str(blk.get("water_kind", "none"))
	check(water_kind in ["none", "fresh", "swamp"], "block inspection uses only simulated water kinds")
	check(blk.get("minerals", PackedStringArray()).size() > 0, "block inspection reports strata/minerals")
	var formatted := Rules.format_block(blk)
	check(formatted.contains("Ground & rock:") and formatted.contains("Water:") and formatted.contains("Minerals & strata:"), "format_block renders sections")
	check(not formatted.contains("salt water") and not formatted.contains("foul water"), "block formatter does not invent water quality")

	# Toggle favourite on inhabitant 0, verify in inspect(0) and favourites(), save and reload.
	check(not sim.is_favourite(0), "inhabitant 0 starts unfavourited")
	check(sim.toggle_favourite(0), "toggle_favourite(0) turns it on")
	check(sim.is_favourite(0), "is_favourite(0) is true")
	var insp0: Dictionary = sim.inspect(0)
	check(bool(insp0.get("favourite", false)), "inspect(0) includes favourite=true")
	var favs: Array = sim.favourites()
	check(favs.size() == 1 and int(favs[0]["id"]) == 0, "favourites() lists inhabitant 0")
	check(str(favs[0]["name"]) != "" and str(favs[0]["activity"]) != "" and str(favs[0]["community"]) != "", "favourite entry has name, activity, community")

	var save_dir := OS.get_user_data_dir() + "/test_413_fav.hfsave"
	check(sim.save_to(save_dir), "save_to starts")
	var st := sim.save_status()
	while st == "saving":
		OS.delay_msec(10)
		st = sim.save_status()
	check(st == "ok", "save_to finishes ok: %s" % st)

	var sim2 := SimHost.new()
	var lerr := sim2.load_from(save_dir)
	check(lerr == "", "load_from succeeds: %s" % lerr)
	check(sim2.is_favourite(0), "favourite 0 survives save and load")
	check(sim2.favourites().size() == 1, "loaded favourites() has 1 entry")
	sim.free()
	sim2.free()

# #484: a sampled review stays bounded and carries the facts needed to reproduce its view.
func test_timelapse_ring_is_bounded_and_preserves_metadata() -> void:
	var ring := Timelapse.new()
	var expected_bytes := 0
	for tick in range(Timelapse.MAX_FRAMES + 1):
		var bytes := 1000 + tick
		expected_bytes += bytes
		if tick >= Timelapse.MAX_FRAMES:
			expected_bytes -= 1000 + tick - Timelapse.MAX_FRAMES
		var evicted: Array = ring.add_frame({
			"tick": tick, "seed": 7, "state_digest": "digest-%d" % tick,
			"path": "user://timelapse/%d.png" % tick, "bytes": bytes,
			"known_only": true, "camera": {"z": 3, "zoom": 1.0},
		})
		if tick < Timelapse.MAX_FRAMES:
			check(evicted.is_empty(), "appends before the bound evict nothing (tick %d)" % tick)
		else:
			check(evicted.size() == 1 and int(evicted[0]["tick"]) == tick - Timelapse.MAX_FRAMES, "only append 121 evicts the oldest sample")
	check(ring.frames.size() == Timelapse.MAX_FRAMES, "the ring holds exactly its frame bound")
	check(int(ring.frames[0]["tick"]) == 1 and int(ring.frames[-1]["tick"]) == Timelapse.MAX_FRAMES, "retention preserves chronological order")
	check(ring.total_bytes == expected_bytes, "reported storage tracks retained PNGs only: %d vs %d" % [ring.total_bytes, expected_bytes])
	check(int(ring.frames[0]["seed"]) == 7 and ring.frames[0]["state_digest"] == "digest-1", "seed and sampled-state identity survive retention")
	check(bool(ring.frames[0]["known_only"]) and int(ring.frames[0]["camera"]["z"]) == 3, "viewer filter and watched level survive retention")
	check(Timelapse.sample_due(28800, 28800) and not Timelapse.sample_due(28799, 28800), "daily sampling uses simulation ticks")
	check(Timelapse.sample_due(57601, 28800), "a fast step cannot skip a scheduled sample")
	var replacement: Dictionary = ring.frames[0].duplicate(true)
	replacement["bytes"] = 2001
	var replaced: Array = ring.add_frame(replacement)
	check(replaced.is_empty() and ring.frames.size() == Timelapse.MAX_FRAMES, "same-tick recapture replaces metadata instead of duplicating a sample")
	check(ring.total_bytes == expected_bytes + 1000, "same-tick recapture replaces its old byte cost")

# #484: the bridge exposes read-only seed and sampled-state identity for snapshot metadata.
func test_timelapse_state_identity_is_deterministic() -> void:
	var a := SimHost.new()
	var b := SimHost.new()
	a.stage_demo(7)
	b.stage_demo(7)
	check(a.world_seed() == 7 and b.world_seed() == 7, "snapshot metadata can name the actual world seed")
	check(a.state_digest() == b.state_digest(), "identical seed and tick produce the same sampled-state digest")
	var before: String = a.state_digest()
	var tick_before: int = a.tick()
	var digest_b: String = b.state_digest()
	check(a.tick() == tick_before and a.state_digest() == before and b.state_digest() == digest_b, "reading snapshot identity neither advances nor changes either world")
	a.free()
	b.free()

# #484 review: following and pinning use sample identity across append and eviction, not indices.
func test_timelapse_panel_follows_and_pins_ticks_across_eviction() -> void:
	var dir := "user://test_484_panel"
	DirAccess.make_dir_recursive_absolute(ProjectSettings.globalize_path(dir))
	var ring := Timelapse.new()
	for tick in range(Timelapse.MAX_FRAMES):
		var path := "%s/%d.png" % [dir, tick]
		var image := Image.create(2, 2, false, Image.FORMAT_RGBA8)
		image.fill(Color(float(tick % 255) / 255.0, 0.2, 0.4, 1.0))
		check(image.save_png(path) == OK, "panel fixture PNG %d writes" % tick)
		ring.add_frame({"tick": tick, "seed": 7, "state_digest": "d", "path": path, "bytes": 1, "camera": {}})
	var sim := SimHost.new()
	sim.stage_demo(7)
	var source_tick: int = sim.tick()
	var source_digest: String = sim.state_digest()
	var panel := TimelapsePanel.new()
	panel._ready()
	panel.sync(ring.frames, true, ring.total_bytes)
	panel.select_frame(0)
	panel.pin_button.pressed.emit()
	panel.select_frame(Timelapse.MAX_FRAMES - 1)
	var last_path := "%s/%d.png" % [dir, Timelapse.MAX_FRAMES]
	var last_image := Image.create(2, 2, false, Image.FORMAT_RGBA8)
	last_image.fill(Color.WHITE)
	last_image.save_png(last_path)
	ring.add_frame({"tick": Timelapse.MAX_FRAMES, "seed": 7, "state_digest": "d", "path": last_path, "bytes": 1, "camera": {}})
	panel.sync(ring.frames, true, ring.total_bytes)
	check(panel.selected_tick == Timelapse.MAX_FRAMES, "follow-latest selects the newly appended tick even though the ring Array was mutated in place")
	check(panel.reference_tick == 1, "an evicted pinned tick falls back to the new oldest rather than the old array index")
	panel.select_frame(9)
	panel.pin_button.pressed.emit()
	panel.play_button.pressed.emit()
	panel._process(1.1)
	check(panel.selected_tick == 11, "review playback advances only the selected sample")
	panel.select_frame(19)
	check(not panel.playing and panel.selected_tick == 20 and panel.reference_tick == 10, "scrub pauses review and preserves the pinned tick")
	check(sim.tick() == source_tick and sim.state_digest() == source_digest, "playback, scrub and pin leave source state and time untouched")
	panel.free()
	sim.free()
	for tick in range(Timelapse.MAX_FRAMES + 1):
		DirAccess.remove_absolute(ProjectSettings.globalize_path("%s/%d.png" % [dir, tick]))
	DirAccess.remove_absolute(ProjectSettings.globalize_path(dir))

# #484 review: shaders and particles use the captured tick, never wall-clock presentation time.
func test_timelapse_presentation_clock_can_be_frozen() -> void:
	for shader in ["res://shaders/sway.gdshader", "res://shaders/grade.gdshader"]:
		var source := FileAccess.get_file_as_string(shader)
		check(not source.contains("TIME"), "%s has no wall-clock TIME dependency" % shader)
		check(source.contains("presentation_time"), "%s exposes the renderer presentation clock" % shader)
	var root := Node.new()
	get_root().add_child(root)
	var weather := CPUParticles2D.new()
	var embers := CPUParticles2D.new()
	root.add_child(weather)
	root.add_child(embers)
	Timelapse.freeze_particles(root, 484)
	for p in [weather, embers]:
		check(p.use_fixed_seed and p.seed == 484, "particle seed comes from the captured tick")
		check(p.fixed_fps == 30 and p.speed_scale == 0.0, "particle presentation is fixed and frozen for capture")
	Timelapse.resume_particles(root)
	check(weather.speed_scale == 1.0 and embers.speed_scale == 1.0, "live particles resume after capture")
	root.free()


# #565: the music director no longer prints to godot.log unless --verbose-audio is given, and
# the sim host exposes the debug log (off by default, so its path is empty).
func test_music_is_quiet_and_debug_log_is_off_by_default() -> void:
	var m = preload("res://scripts/audio/music_director.gd").new()
	check(not m.verbose, "music director prints by default")
	m.free()
	var host := SimHost.new()
	check(host.debug_log_path() == "", "debug log on without a flag")
	var pr = preload("res://scripts/persist.gd").new()
	check(pr.settings.has("debug_log") and pr.settings["debug_log"] == false, "settings toggle exists and defaults off")
	# The settings toggle's path: on names the file, off clears it.
	var lp := OS.get_user_data_dir().path_join("gd-test-debug.log")
	host.debug_log_start(lp, "command=gd_test", "gd_test")
	check(host.debug_log_path() == lp, "debug log start reports its path")
	host.debug_log_stop()
	check(host.debug_log_path() == "", "debug log stop clears the path")
	host.free()

# #484 finish: samples fall at noon of each calendar day and stepping stops on them, so a sample
# is taken at exactly its tick and a re-run of the same days lands on the same ticks.
func test_timelapse_samples_fall_on_exact_noon_ticks() -> void:
	check(Timelapse.next_sample_tick(0, 0) == 14400, "the first sample of a world at its epoch is that day's noon")
	check(Timelapse.next_sample_tick(14400, 0) == 14400 and Timelapse.next_sample_tick(14401, 0) == 43200, "a noon tick is its own sample; the tick after waits for the next noon")
	check(Timelapse.next_sample_tick(5000, 4000) == 18400, "noon is counted from the world's epoch")
	check(Timelapse.next_sample_tick(100, 30000) == 100 + posmod(30000 + 14400 - 100, 28800), "ticks before the epoch still find a noon of the calendar")
	check(Timelapse.step_allowance(14000, 14400, 400) == 400 and Timelapse.step_allowance(14200, 14400, 400) == 200, "stepping is cut short at the sample tick")
	check(Timelapse.step_allowance(14400, 14400, 400) == 0, "no step past a sample that is due")
	check(Timelapse.step_allowance(500, -1, 400) == 400, "without a schedule stepping is unchanged")

# #484 finish: the series is bounded in bytes as well as samples, keeps tick order for samples
# that arrive out of order, and a re-run is confirmed or its stale future removed by digest.
func test_timelapse_byte_bound_order_and_replay() -> void:
	var ring := Timelapse.new()
	var big := Timelapse.MAX_SERIES_BYTES / 4
	for tick in [100, 300, 200]:
		ring.add_frame({"tick": tick, "state_digest": "d%d" % tick, "path": "user://x/%d.png" % tick, "bytes": 1})
	check(ring.frames.map(func(f): return int(f["tick"])) == [100, 200, 300], "samples are kept in tick order whatever order they arrive in")
	ring.clear()
	var evicted: Array = []
	for i in 6:
		evicted.append_array(ring.add_frame({"tick": i, "state_digest": "d", "path": "user://x/%d.png" % i, "bytes": big}))
	check(ring.total_bytes <= Timelapse.MAX_SERIES_BYTES, "retained bytes never exceed the series bound: %d" % ring.total_bytes)
	check(evicted.size() == 2 and int(evicted[0]["tick"]) == 0 and int(evicted[1]["tick"]) == 1, "the byte bound evicts the oldest samples first")
	check(int(ring.frames[-1]["tick"]) == 5, "the newest sample is kept")
	var lone := Timelapse.new()
	lone.add_frame({"tick": 9, "path": "user://x/9.png", "bytes": Timelapse.MAX_SERIES_BYTES * 2})
	check(lone.frames.size() == 1, "a single oversized sample is kept rather than leaving the series empty")
	check(ring.reconcile(3, "d") == "confirmed" and ring.reconcile(3, "other") == "diverged" and ring.reconcile(99, "d") == "new", "a re-run tick is confirmed, diverged or new by its state digest")
	var dropped: Array = ring.drop_from(4)
	check(dropped.size() == 2 and ring.frames.size() == 2 and int(ring.frames[-1]["tick"]) == 3, "a divergence drops that sample and every later one")
	check(ring.total_bytes == 2 * big, "dropping keeps the byte count exact")
	var shot := Image.create(2560, 1440, false, Image.FORMAT_RGBA8)
	shot.fill(Color(0.2, 0.4, 0.6, 1.0))
	var fitted := Timelapse.fit_capture(shot)
	check(fitted.get_width() == Timelapse.CAPTURE_MAX.x and fitted.get_height() == Timelapse.CAPTURE_MAX.y and fitted.get_format() == Image.FORMAT_RGB8, "a capture is stored at most CAPTURE_MAX pixels of RGB")
	var small := Image.create(640, 360, false, Image.FORMAT_RGBA8)
	check(Timelapse.fit_capture(small).get_width() == 640, "a small capture is never scaled up")
	check(Timelapse.fit_capture(shot).get_data() == fitted.get_data(), "the same viewport pixels always store the same image")
	check(is_equal_approx(Timelapse.valley_zoom(Vector2(1280, 720), Vector2(8192, 8192)), 720.0 / 8192.0), "the valley zoom fits the whole map into the window")

# #484 finish: a series is read back from its own files (a resumed world goes on with it), and
# the number of worlds keeping a series is bounded.
func test_timelapse_series_reloads_and_prunes() -> void:
	var root := "user://test_484_series"
	var series := root + "/world-a"
	DirAccess.make_dir_recursive_absolute(ProjectSettings.globalize_path(series))
	for tick in [14400, 43200]:
		var image := Image.create(4, 4, false, Image.FORMAT_RGB8)
		image.save_png("%s/%020d.png" % [series, tick])
		var f := FileAccess.open("%s/%020d.json" % [series, tick], FileAccess.WRITE)
		f.store_string(JSON.stringify({"tick": tick, "state_digest": "s%d" % tick, "events": {"lines": ["Day 1  a house was built"]}}))
		f.close()
	var orphan := FileAccess.open("%s/%020d.json" % [series, 72000], FileAccess.WRITE)
	orphan.store_string(JSON.stringify({"tick": 72000}))
	orphan.close()
	var ring := Timelapse.new()
	ring.load_series(series)
	check(ring.frames.size() == 2 and int(ring.frames[0]["tick"]) == 14400 and int(ring.frames[1]["tick"]) == 43200, "the stored series reloads in tick order")
	check(str(ring.frames[1]["path"]).ends_with("%020d.png" % 43200) and ring.total_bytes > 0, "reloaded samples point at their images and count their bytes")
	check(ring.reconcile(43200, "s43200") == "confirmed", "a reloaded sample can confirm a re-run")
	DirAccess.remove_absolute(ProjectSettings.globalize_path("%s/%020d.json" % [series, 72000]))
	for i in 4:
		var other := "%s/world-o%d" % [root, i]
		DirAccess.make_dir_recursive_absolute(ProjectSettings.globalize_path(other))
		var image := Image.create(2, 2, false, Image.FORMAT_RGB8)
		image.save_png(other + "/a.png")
	var removed: Array = Timelapse.prune_series(series, root, 3)
	check(removed.size() == 2, "only the current series and the two most recent others are kept: %s" % [removed])
	var left := DirAccess.open(ProjectSettings.globalize_path(root)).get_directories()
	check(left.size() == 3 and left.has("world-a"), "the current series is never pruned")
	for name in left:
		var d := ProjectSettings.globalize_path(root).path_join(name)
		for f in DirAccess.open(d).get_files():
			DirAccess.remove_absolute(d.path_join(f))
		DirAccess.remove_absolute(d)
	DirAccess.remove_absolute(ProjectSettings.globalize_path(root))

# #484 finish: the review shows the record between the compared samples from what each sample
# stored, and the bridge gives a world identity that a running world keeps.
func test_timelapse_event_markers_and_world_identity() -> void:
	var panel := TimelapsePanel.new()
	panel._ready()
	var frames := [
		{"tick": 1, "seed": 7, "path": "user://none/1.png", "events": {"lines": []}},
		{"tick": 2, "seed": 7, "path": "user://none/2.png", "events": {"lines": ["Day 2  Ada was born"]}},
		{"tick": 3, "seed": 7, "path": "user://none/3.png", "events": {"lines": ["Day 3  a house was built", "Day 3  Bo died"]}},
	]
	panel.sync(frames, false, 0)
	check(panel.events_text().contains("3 noted") and panel.events_text().contains("Ada was born") and panel.events_text().contains("Bo died"), "the review lists every stored event between reference and selected")
	panel.select_frame(1)
	check(panel.events_text().contains("Ada was born") and not panel.events_text().contains("Bo died"), "a nearer selection lists only what came before it")
	check(TimelapsePanel.marker_weight(frames[2]) == 2 and TimelapsePanel.marker_weight(frames[0]) == 0, "markers weigh what the record holds")
	panel.free()
	var sim := SimHost.new()
	sim.stage_demo(7)
	var identity: String = sim.world_identity()
	var digest: String = sim.state_digest()
	check(identity.length() == 64 and sim.epoch() >= 0, "the bridge names the world and its epoch")
	var ev: Dictionary = sim.summary_between(0, sim.tick(), 4)
	check(ev.has("lines") and ev.has("counts") and (ev["lines"] as Array).size() <= 4, "event markers come from the record, bounded")
	check(sim.state_digest() == digest, "reading identity and markers changes nothing")
	sim.step(30)
	sim.sync()
	check(sim.world_identity() == identity, "a running world keeps its identity")
	var other := SimHost.new()
	other.stage_demo(8)
	check(other.world_identity() != identity, "another seed is another world")
	sim.free()
	other.free()

# #483 viewer review: stepping to an upper storey dims lower ground more strongly than ordinary
# terrain relief, and person inspection reports relative level against surface_z.
func test_upper_storey_dimming_and_person_level_readout() -> void:
	var view = _underground_scenario()
	view.ground.fill(1) # surface ground_z = 1 (walking level z = 2)
	view.levels[1][4] = _scenario_tile(1, 3) # grass at z = 1 (visible floor at z = 2)
	view.view_z = 3 # viewing upper storey (view_z > ground + 1)
	var upper_view_ground: Array = view._ground_family(1, 1)
	check(is_equal_approx(float(upper_view_ground[1]), 0.25), "lower ground dims to 0.25 when viewing an upper storey")
	view.ground.fill(2) # hill ground_z = 2 so view_z = 3 is normal surface walking level
	var surface_view_ground: Array = view._ground_family(1, 1)
	check(is_equal_approx(float(surface_view_ground[1]), 0.10), "normal terrain relief keeps gentle 0.10 dimming at peel 2")
	view.free()
	var sim := SimHost.new()
	sim.stage_demo(1)
	var agents: PackedInt32Array = sim.agents()
	var checked_person := false
	for i in range(0, agents.size(), 24):
		if agents[i + 4] != 0:
			continue
		var d: Dictionary = sim.inspect(agents[i])
		if d.is_empty():
			continue
		check(d.has("surface_z"), "person inspect includes surface_z")
		check(String(d.get("where", "")).contains(" — "), "person inspect where includes relative level")
		check(String(d.get("activity", "")).contains(" — at ("), "person inspect activity includes location and level")
		checked_person = true
		break
	check(checked_person, "inspected a person for level readout")
	sim.free()

# #413 slice 4.3: Toaster centers horizontally and passes clicks through non-positional toasts
# so modal panel buttons (Favourites, History) are never occluded during a click-only session.
func test_toaster_centering_and_mouse_passthrough() -> void:
	var t := Toaster.new()
	t._ready()
	check(is_equal_approx(t._box.offset_left, -Toaster.WIDTH * 0.5) and is_equal_approx(t._box.offset_right, Toaster.WIDTH * 0.5), "Toaster centers its banner box horizontally")
	t.toast("Ashcross was founded (seed 1).", "#ffb3a0", "founded", Vector3i(-1, -1, 0))
	t.toast("Halowick arrived.", "#ffe9a8", "arrived", Vector3i(83, 65, 3))
	check(t._items.size() == 2, "Toaster queued both toasts")
	var n0: Control = t._items[0]["node"]
	var n1: Control = t._items[1]["node"]
	check(n0.mouse_filter == Control.MOUSE_FILTER_IGNORE, "non-positional toast ignores mouse clicks so panels underneath remain clickable")
	check(n1.mouse_filter == Control.MOUSE_FILTER_STOP, "positional toast accepts mouse click to look at where it happened")
	t.free()

# D91, FIXLOG 2026-11-14: announcements start under the HUD and wait while a reading panel is
# open, hidden and not ageing, still gathering by kind; they show when it closes.
func test_toasts_wait_while_a_reading_panel_is_open() -> void:
	check(not Rules.toasts_held([], false), "nothing open: toasts show")
	check(not Rules.toasts_held([&"inspect", &"block", &"help"], false), "edge panels do not hold them")
	for id in [&"summary", &"legends", &"people", &"waiting", &"favourites", &"timelapse"]:
		check(Rules.toasts_held([&"inspect", id], false), "%s holds them" % id)
	check(Rules.toasts_held([], true), "Settings (or the files page) holds them")
	var t := Toaster.new()
	t._ready()
	t.set_top(96.0)
	check(is_equal_approx(t._box.offset_top, 96.0), "the column starts where it is told (under the HUD)")
	t.toast("Moresin and Aelwen became partners.", "#ffe9a8", "paired", Vector3i(10, 10, 3))
	t.hold(true)
	check(not t._box.visible, "held: not on screen")
	t._process(Toaster.LIFE + Toaster.FADE + 1.0)
	check(t._items.size() == 1 and is_equal_approx(t._items[0]["age"], 0.0), "held: it does not age away unseen")
	t.toast("Rosthi and Kella became partners.", "#ffe9a8", "paired", Vector3i(11, 10, 3))
	check(t._items.size() == 1 and t._items[0]["count"] == 2, "held: the same kind still gathers with a count")
	t.hold(false)
	check(t._box.visible, "released: back on screen")
	t._process(1.0)
	check(t._items.size() == 1 and t._items[0]["age"] > 0.9, "released: it ages again from where it was")
	t.free()

# #413 slice 4.2: block inspection on water and underground tiles reports only simulation-backed facts.
func test_block_inspect_water_and_underground_facts() -> void:
	var sim := SimHost.new()
	check(sim.opening(1, [0, 1, 2], PackedInt64Array([]), "steady") == "", "opening seed 1 succeeds")
	var fs := sim.settlement_centre()
	var water_blk := {}
	for dy in range(-35, 35):
		for dx in range(-35, 35):
			for dz in [0, -1, -2]:
				var d: Dictionary = sim.inspect_block(fs.x + dx, fs.y + dy, fs.z + dz)
				if int(d.get("water_depth", 0)) >= 1 and str(d.get("title", "")).begins_with("Water at"):
					water_blk = d
					break
			if not water_blk.is_empty():
				break
		if not water_blk.is_empty():
			break
	check(not water_blk.is_empty(), "found a water block near the settlement on seed 1")
	if not water_blk.is_empty():
		print("GD_TEST_INFO water_block=(%d,%d,%d) water=%s kind=%s quality=%s" % [water_blk["x"], water_blk["y"], water_blk["z"], water_blk["water"], water_blk["water_kind"], water_blk["water_quality"]])
		check(str(water_blk.get("water_kind", "")) in ["fresh", "swamp", "foul"], "water_kind is simulation-backed")
		check(str(water_blk.get("water_quality", "")) in ["fresh", "salt", "brackish"], "water_quality is simulation-backed (#453)")
		check(str(water_blk.get("ground", "")).contains("bed"), "water block reports its bed material")
		var wfmt := Rules.format_block(water_blk)
		check(wfmt.contains(str(water_blk["water"])), "format_block renders the simulation's water description verbatim")

	var ug_z := fs.z - 4
	var ug_blk: Dictionary = sim.inspect_block(fs.x, fs.y, ug_z)
	print("GD_TEST_INFO underground_block=(%d,%d,%d) ground=%s temp=%s" % [ug_blk["x"], ug_blk["y"], ug_blk["z"], ug_blk["ground"], ug_blk["temperature"]])
	check(int(ug_blk.get("z", 0)) == ug_z and ug_z < int(ug_blk.get("surface_z", 0)), "underground block is below surface_z")
	check(str(ug_blk.get("ground", "")).contains("below ground"), "underground block reports depth below ground")
	check(str(ug_blk.get("temperature", "")).contains("underground"), "underground block reports underground shelter and lighting")
	check(ug_blk.get("minerals", PackedStringArray()).size() > 0, "underground block reports column minerals and strata")
	var ufmt := Rules.format_block(ug_blk)
	check(ufmt.contains("underground") and ufmt.contains("Minerals & strata:"), "format_block renders underground details and strata")

# #413 A5 (#378): because-lines on feed items and panels, feed-rate control, and first-run hints.
func test_readability_feed_rate_because_lines_and_first_run_hints() -> void:
	# Feed-rate control modes: 0 calm, 1 normal, 2 full.
	check(Rules.feed_rate_name(0) == "calm" and Rules.feed_rate_name(1) == "normal" and Rules.feed_rate_name(2) == "full", "feed rate names")
	check(not Rules.feed_allows_minor(0, 10000), "calm mode drops all minor feed lines into +N quieter lines")
	check(not Rules.feed_allows_minor(1, 300) and Rules.feed_allows_minor(1, 700), "normal mode throttles minor lines to 700 ms")
	check(Rules.feed_allows_minor(2, 0), "full mode shows every minor line without throttling")
	check(Rules.feed_max_lines(0) < Rules.feed_max_lines(1) and Rules.feed_max_lines(1) < Rules.feed_max_lines(2), "feed max lines scales with rate mode")

	var ids := {}
	for it in Rules.bar_items():
		ids[it["id"]] = it
	check(ids.has("feed_rate") and str(ids["feed_rate"]["key"]) == "G", "bottom bar exposes the feed-rate button [G]")

	# Feed line formatting with simulation-backed because-lines.
	var with_cause := Rules.format_feed_line("Urist was drenched", "Rain began to fall", "#cfcfcf", false, 1)
	check(with_cause.contains("— because: Rain began to fall"), "feed line appends its simulation-backed because-line: %s" % with_cause)
	var already := Rules.format_feed_line("Urist refused the call — because they had barely slept", "slept on the ground", "#ffe9a8", true, 2)
	check(already.contains("★ ") and already.count("because") == 1, "feed line never duplicates an inline because-clause: %s" % already)

	# Panel & hover because-lines from simulation read models.
	check(Rules.inspect_because_line({"why_cycles": [{"because": PackedStringArray(["hunger was pressing", "stew was near"])}], "why": "hungry"}) == "because: hunger was pressing, stew was near", "inspect_because_line prefers traced decision reasons")
	check(Rules.inspect_because_line({"why": "slept on the ground, a quarrel"}) == "because: slept on the ground, a quarrel", "inspect_because_line uses mood why")
	check(Rules.inspect_because_line({"why": "", "feeling": "afraid of Greyjaw"}) == "because: feels afraid of Greyjaw", "inspect_because_line falls back to feeling")
	var fav_row := Rules.format_favourite_row({"name": "Ada", "community": "Ashcross", "alive": true, "where": "at (10, 10, z1)", "activity": "resting", "why": "slept on the ground", "feeling": ""})
	check(fav_row.contains("— because: slept on the ground"), "favourite row includes because-line: %s" % fav_row)
	var hov := Rules.format_hover({"name": "Ada", "activity": "working", "why": "warm meal", "feeling": ""})
	check(hov.contains("— because: warm meal") and hov.contains("(click to inspect)"), "hover line includes because-line: %s" % hov)

	# First-run hints progress through person inspect -> block inspect -> bottom bar -> done.
	var h1 := Rules.first_run_hint(false, false, false)
	var h2 := Rules.first_run_hint(true, false, false)
	var h3 := Rules.first_run_hint(true, true, false)
	var h4 := Rules.first_run_hint(true, true, true)
	check(h1.contains("click any person") and h2.contains("ground, water, or rock") and h3.contains("bottom bar") and h4 == "", "first-run hints step through all three actions and then clear")

	var pr = preload("res://scripts/persist.gd").new()
	check(int(pr.settings.get("feed_rate", -1)) == 1 and pr.settings.has("hints_seen") and pr.settings["hints_seen"] == false, "persist.gd stores feed_rate and hints_seen defaults")

	# Bridge integration: drain_feed rows carry 9 elements (including because) and favourites() carries why & feeling.
	var sim := SimHost.new()
	check(sim.opening(1, [0, 1, 2], PackedInt64Array([]), "steady") == "", "opening seed 1 for readability check")
	sim.toggle_favourite(0)
	var favs: Array = sim.favourites()
	check(favs.size() == 1 and favs[0].has("why") and favs[0].has("feeling"), "favourites() exposes why and feeling keys")
	var feed: Array = sim.drain_feed()
	# 10 since #1483: the fold key follows because (a string, "" for a row that never folds).
	check(feed.size() > 0 and (feed[0] as Array).size() == 10 and (feed[0] as Array)[9] is String, "drain_feed rows carry 10 elements: because, then the fold key")
	sim.free()

## #290 (WP-08 8.8): the music answers a death and a welcome. The moments are music, not
## effects; a welcome ends a rest early but never stretches one or cuts a phrase; the melody
## stem is silent only while a death is mourned.
func test_music_moments_mourn_and_welcome() -> void:
	check(Rules.is_music_moment("mourn") and Rules.is_music_moment("welcome"), "mourn and welcome are music moments")
	check(not Rules.is_music_moment("flare") and not Rules.is_music_moment("douse") and not Rules.is_music_moment("lightning"), "effect sounds are not music moments")
	check(is_equal_approx(Rules.welcome_rest(90.0, 6.0, false), 6.0), "a welcome ends a long rest within the wait")
	check(is_equal_approx(Rules.welcome_rest(3.0, 6.0, false), 3.0), "a welcome never lengthens a rest about to end")
	check(is_equal_approx(Rules.welcome_rest(90.0, 6.0, true), 90.0), "a welcome leaves a sounding phrase alone")
	check(Rules.melody_target(10.0) == 0.0 and Rules.melody_target(0.0) == 1.0, "the melody is silent only while mourning")

## #290 item 3 (WP-08 8.8): the music hears what the people are doing, from the agents rows.
func test_music_hears_activity() -> void:
	var st := 25
	var rows := func(specs: Array) -> PackedInt32Array:
		var a := PackedInt32Array()
		a.resize(specs.size() * st)
		for k in specs.size():
			a[k * st + 4] = specs[k][0]
			a[k * st + 15] = specs[k][1]
		return a
	var none: Dictionary = Rules.music_activity(PackedInt32Array(), st)
	check(not none["fight"] and not none["gathering"] and not none["asleep"], "no people: nothing")
	var night: Dictionary = Rules.music_activity(rows.call([[0, 4], [0, 4], [3, 0], [0, 9]]), st)
	check(night["asleep"], "every person asleep is quiet, whatever the wolves do, a body lying too: %s" % night)
	var one_up: Dictionary = Rules.music_activity(rows.call([[0, 4], [0, 2]]), st)
	check(not one_up["asleep"], "one person awake is not a sleeping valley")
	var talk: Dictionary = Rules.music_activity(rows.call([[0, 10], [0, 10], [0, 11], [1, 10]]), st)
	check(talk["gathering"], "three people talking or eating is a gathering: %s" % talk)
	var two: Dictionary = Rules.music_activity(rows.call([[0, 10], [0, 10], [1, 11]]), st)
	check(not two["gathering"], "two people and a deer are not a gathering")
	var fight: Dictionary = Rules.music_activity(rows.call([[0, 6], [0, 2]]), st)
	check(fight["fight"], "a person who fought just now is a fight")
	check(is_equal_approx(Rules.tension_target(0.1, true, 0.6), 0.6), "a fight lifts a calm valley's tension to the floor")
	check(is_equal_approx(Rules.tension_target(0.9, true, 0.6), 1.0), "a fight never lowers high strain")
	check(is_equal_approx(Rules.tension_target(0.1, false, 0.6), 0.0), "without a fight, strain alone")

func test_portrait_identity_key_and_real_ramp_colour() -> void:
	## #192, WP-08 8.4 third increment: the inspect-panel portrait is the same person as the
	## world sprite — the identity key is deterministic and every gene matters, and the ramp
	## lookup returns the sheet colours (swaps.json targets are raw hex).
	var Portrait := preload("res://scripts/portrait.gd")
	var a := []
	var b := []
	for k in 25:
		a.append(0)
		b.append(0)
	a[7] = 3; a[8] = 4; a[9] = 2; a[11] = 5
	b[7] = 3; b[8] = 4; b[9] = 2; b[11] = 5
	check(Portrait.variant_key(a) == Portrait.variant_key(a.duplicate()), "same genes, same key")
	b[16] = 1  # the child bit
	check(Portrait.variant_key(a) != Portrait.variant_key(b), "the child bit changes the key")
	var p = Portrait.new()
	p.assets = preload("res://scripts/assets.gd").new()
	check(p._ramp("skin", 0, 3) == Color("#a2694a"),
		"skin gene 0 at the portrait's step is the sheet's own hex")
	check(p._ramp("cloth", 12345, 3) == p._ramp("cloth", 12345, 3),
		"out-of-range genes wrap deterministically")
	p.show_person(a, {"plan": "biped", "hair": true, "body": "human"})
	check(p.g.get("skin", -1) == 3, "show_person stores the genes")
	check(p.g.get("child", true) == false, "show_person keeps the child bit off for an adult")
	# A Control never added to the tree: free it, or the run ends with an orphan (#724 review).
	p.free()


## #290 remainder (WP-08 8.8): a fire in view lifts the tension layer, less than a fight does.
func test_a_fire_in_view_lifts_the_music() -> void:
	check(is_equal_approx(Rules.tension_target(0.1, false, 0.6, true, 0.45), 0.45), "a calm valley with a fire in view: the fire floor")
	check(is_equal_approx(Rules.tension_target(0.1, true, 0.6, true, 0.45), 0.6), "a fight outranks a fire")
	check(is_equal_approx(Rules.tension_target(0.9, false, 0.6, true, 0.45), 1.0), "a fire never lowers high strain")
	check(is_equal_approx(Rules.tension_target(0.1, false, 0.6), 0.0), "no fire, no fight: strain alone (old callers unchanged)")

## #292 (WP-08 8.6): a row of the Waiting work panel says what, why and how long.
func test_waiting_work_row() -> void:
	var row := Rules.format_stuck_row({"label": "felling", "reason": "nobody can: 2 will not fell a living tree", "hours": 5})
	check(row == "Felling: nobody can: 2 will not fell a living tree (for 5 h)", "a row: %s" % row)
	check(Rules.format_stuck_row({"label": "wall", "reason": "x", "hours": 72}).ends_with("(for 3 days)"), "long waits read in days")
	var ids := {}
	for it in Rules.bar_items():
		ids[it["id"]] = it
	check(ids.has("waiting") and str(ids["waiting"]["key"]) == "K", "the bar opens Waiting work [K]")

## #290 (WP-08 8.8 work bustle): the valley at work lifts the tension layer, gently.
func test_music_hears_work_bustle() -> void:
	var st := 25
	var rows := func(specs: Array) -> PackedInt32Array:
		var a := PackedInt32Array()
		a.resize(specs.size() * st)
		for k in specs.size():
			a[k * st + 4] = specs[k][0]
			a[k * st + 15] = specs[k][1]
		return a
	var two: Dictionary = Rules.music_activity(rows.call([[0, 2], [0, 2], [0, 0]]), st)
	check(not two["bustle"], "two at the work is not a bustle: %s" % two)
	var beast: Dictionary = Rules.music_activity(rows.call([[1, 2], [0, 2], [0, 2], [0, 2]]), st)
	check(beast["bustle"], "three people at the work is a bustle, whatever the deer is doing: %s" % beast)
	check(is_equal_approx(Rules.tension_target(0.1, false, 0.6, false, 0.45, true, 0.2), 0.2), "a calm working valley: the work floor")
	check(is_equal_approx(Rules.tension_target(0.1, false, 0.6, true, 0.45, true, 0.2), 0.45), "a fire outranks work")
	check(is_equal_approx(Rules.tension_target(0.9, false, 0.6, false, 0.45, true, 0.2), 1.0), "work never lowers high strain")
	check(is_equal_approx(Rules.tension_target(0.1, false, 0.6), 0.0), "no work, no fire, no fight: strain alone (old callers unchanged)")

func test_world_marks_anchor_label_dedup_and_cap() -> void:
	## #351, WP-08 8.5: an in-world toaster sits at the event's tile with the toast's line as
	## its label; a retold tile refreshes its marker instead of stacking rings; at most twelve
	## stand; a marker past its life is gone. (#735's design ported onto #738's marks layer.)
	var Marks := preload("res://scripts/render/world_marks.gd")
	var m = Marks.new()
	m.init(null)
	m.mark(Vector3i(10, 20, 1), Color("#ffe9a8"), "A fire")
	assert(m.marks.size() == 1, "one mark")
	assert(m.marks[0].tile == Vector3i(10, 20, 1), "the tile is kept for the layer and the dimming")
	assert(m.marks[0].text == "A fire", "the line rides along as the label")
	m.mark(Vector3i(10, 20, 1), Color.RED, "The fire spreads")
	assert(m.marks.size() == 1, "the same tile refreshes, never stacks")
	assert(m.marks[0].text == "The fire spreads", "the label refreshes with the retelling")
	for i in 20:
		m.mark(Vector3i(i, 0, 0), Color.WHITE, "e")
	assert(m.marks.size() == 12, "at most twelve stand")
	assert(m.marks[-1].tile == Vector3i(19, 0, 0), "the newest stands")
	assert(m.marks[0].tile == Vector3i(8, 0, 0), "the oldest was evicted")
	m._process(Rules.MARKER_LIFE + 0.1)
	assert(m.marks.is_empty(), "a mark past its life is gone")
	m.mark(Vector3i(0, 0, 0), Color.WHITE, "back")
	m.mark(Vector3i(1, 1, 0), Color.WHITE, "again")
	m.clear()
	assert(m.marks.is_empty(), "clear empties the layer")
	m.free()

func test_marker_rules() -> void:
	## #351, WP-08 8.5: a marker holds, then fades within its life; its ring pulses within
	## bounds; its label is cut at a word. (#735's design, ported.)
	var life: float = Rules.MARKER_LIFE
	check(Rules.marker_alpha(0.0) == 1.0 and Rules.marker_alpha(life * 0.5) == 1.0, "a fresh marker is full")
	var late := Rules.marker_alpha(life * 0.9)
	check(late > 0.0 and late < 1.0, "a marker near its end is fading: %s" % late)
	check(Rules.marker_alpha(life) == 0.0 and Rules.marker_alpha(-1.0) == 0.0, "gone at its life, nothing before birth")
	check(Rules.marker_alpha(life * 0.8) > Rules.marker_alpha(life * 0.95), "it fades, not flickers")
	for t in [0.0, 0.3, 1.1, 7.7, 42.0]:
		var r := Rules.marker_radius(t)
		check(r >= 14.0 and r <= 22.0, "the ring stays between 14 and 22 px: %s" % r)
	check(Rules.marker_label("A fire") == "A fire", "a short line is kept whole")
	var long := Rules.marker_label("Halowick drowned crossing the river below Ashcross in the flood")
	check(long.ends_with("…") and long.length() <= 33 and not long.contains("  "), "a long line is cut at a word: %s" % long)

func test_people_panel_title_counts_here_and_away() -> void:
	check(Rules.people_title([]).begins_with("People: 0 in the valley"), "empty valley")
	var t := Rules.people_title([{"away": false}, {"away": false}, {"away": true}])
	check(t.begins_with("People: 2 in the valley, 1 away"), "two here, one away: %s" % t)
	check(not Rules.people_title([{"away": false}]).contains("away"), "nobody away: no away count")
	var with_p := 0
	for b in Rules.BAR_ITEMS:
		if str(b.get("key", "")) == "P":
			with_p += 1
	check(with_p == 1, "the People button, and only it, has P: %d" % with_p)

# #797 finding 2 (FIXLOG 2026-11-14): the camera director never moved in a running game. The
# client steps every frame, so the session is on the step thread nearly whenever the director
# asks, and the bridge answered only a suggestion made at home: an empty array, every time.
# Stepping as the client does, with the director asking, a full answer must come back.
func test_director_answers_while_the_game_runs() -> void:
	var sim := SimHost.new()
	sim.stage_demo(1)
	sim.set_threaded(true)
	check(sim.is_threaded(), "the step thread is on")
	# The game is running when the director takes the camera, so its first ask, like every
	# later one, comes while the ticks are on the step thread.
	sim.step(1)
	sim.director_suggest()
	# The first asks come before any batch made for an asking director is home; from the first
	# full answer on, every ask must have one (timing-free: how soon depends on the machine).
	var first_full := -1
	var empty_after := 0
	var last: Array = []
	for k in 80:
		sim.step(1)  # as main.gd _process: hand the next ticks over at once
		OS.delay_msec(5)
		var sug: Array = sim.director_suggest()
		if sug.size() >= 6:
			if first_full < 0:
				first_full = k
			last = sug
		elif first_full >= 0:
			empty_after += 1
	sim.sync()
	print("GD_TEST_INFO director: first full answer at ask %d of 80, %d empty after it" % [first_full, empty_after])
	check(first_full >= 0, "a full suggestion while the game runs (master's bridge: never)")
	check(empty_after == 0, "once answered, never empty again: %d empty" % empty_after)
	if last.size() >= 6:
		check(int(last[0]) >= 0 and int(last[1]) >= 0 and str(last[5]) != "", "a place and a reason: %s" % [last])

# #797 finding 1 (FIXLOG 2026-11-13): Mouse & Keys opened by itself in the middle of the view on a
# first start, over the founding spot, and its key list was cut off. It now sits at the left edge
# under the HUD, as tall as its text, and every line fits its width.
func test_mouse_and_keys_leaves_the_middle_of_the_view_clear() -> void:
	for view in [Vector2(1600, 900), Vector2(1280, 720)]:
		var r: Rect2 = Rules.help_rect(view, 90.0, 470.0)
		check(r.end.x < view.x / 2.0 - 40.0, "%s: clear of the middle (right edge %.0f)" % [view, r.end.x])
		check(r.position.y >= 90.0, "%s: under the HUD" % view)
		check(r.size.y == 470.0, "%s: as tall as its text, no scrolling" % view)
		check(r.end.y <= view.y - Rules.HELP_BAR_ROOM, "%s: clear of the bottom bar" % view)
	var tall: Rect2 = Rules.help_rect(Vector2(1280, 720), 140.0, 2000.0)
	check(tall.end.y <= 720.0 - Rules.HELP_BAR_ROOM, "a short window: capped above the bar (%.0f)" % tall.end.y)
	var narrow: Rect2 = Rules.help_rect(Vector2(400, 720), 90.0, 470.0)
	check(narrow.end.x <= 390.0, "a narrow window: inside it")
	# Every line fits the panel's width at its 14 px text, so none wraps (10 px margins each side).
	var help: String = preload("res://scripts/main.gd").HELP_TEXT
	var font := ThemeDB.fallback_font
	var widest := 0.0
	for line in help.replace("[b]", "").replace("[/b]", "").split("\n"):
		widest = maxf(widest, font.get_string_size(line, HORIZONTAL_ALIGNMENT_LEFT, -1, 14).x)
	check(widest <= Rules.HELP_W - 24.0, "widest help line %.0f px fits %.0f" % [widest, Rules.HELP_W - 24.0])

# WP-09 9.12: "Your files and data" lists the files register's rows (generated into
# res://data/files_register.json), sizes what is on disk now, and "Delete logs" removes only
# finished logs: never the engine log being written, the debug log while it is on, or a save.
func test_files_page_lists_the_register_and_deletes_only_finished_logs() -> void:
	var FilesPage = load("res://scripts/files_page.gd")
	var sections: Array = FilesPage.load_sections()
	var places := []
	for sec in sections:
		places.append(sec["place"])
		check(not sec["rows"].is_empty(), "%s has rows" % sec["place"])
		for r in sec["rows"]:
			check(not r["names"].is_empty() and r["what"] != "" and r["sent"] != "" and r["delete"] != "", "a full row: %s" % [r["names"]])
	check(places == ["install", "userdata", "temp"], "the three places players have today: %s" % [places])
	var names := []
	for sec in sections:
		for r in sec["rows"]:
			names.append_array(r["names"])
	check(names.has("saves/") and names.has("settings.cfg") and names.has("READ ME FIRST.txt"), "the rows name the real files")
	# Sizes: a file, a folder counted through, a missing name, a wildcard skipped.
	var root := OS.get_temp_dir().path_join("hf-files-page-test-%d" % OS.get_process_id())
	DirAccess.make_dir_recursive_absolute(root.path_join("saves/auto.0.hfsave"))
	var f := FileAccess.open(root.path_join("settings.cfg"), FileAccess.WRITE); f.store_buffer(PackedByteArray([1, 2, 3])); f.close()
	f = FileAccess.open(root.path_join("saves/auto.0.hfsave/world.bin"), FileAccess.WRITE); f.store_buffer(PackedByteArray(range(256))); f.close()
	check(FilesPage.size_of(root, ["settings.cfg"]) == 3, "a file's size")
	check(FilesPage.size_of(root, ["saves/"]) == 256, "a folder counts what is inside it: %d" % FilesPage.size_of(root, ["saves/"]))
	check(FilesPage.size_of(root, ["settings.cfg", "saves/"]) == 259, "two names add up")
	check(FilesPage.size_of(root, ["timelapse/"]) == -1, "a missing folder reads as not there")
	check(FilesPage.size_of(root, ["godot<date>T<time>.log", "*.zip.part"]) == -1, "wildcard names are not sized")
	DirAccess.remove_absolute(root.path_join("saves/auto.0.hfsave/world.bin"))
	DirAccess.remove_absolute(root.path_join("saves/auto.0.hfsave"))
	DirAccess.remove_absolute(root.path_join("saves"))
	DirAccess.remove_absolute(root.path_join("settings.cfg"))
	DirAccess.remove_absolute(root)
	check(FilesPage.human_size(512) == "512 bytes" and FilesPage.human_size(3 * 1048576) == "3.0 MB", "sizes in plain units")
	# Delete logs: older engine logs and the debug log's older half; never what is in use.
	var logs := ["godot.log", "godot2026-11-13T20.10.01.log", "godot2026-11-12T08.00.00.log", "notes.txt"]
	var data := ["settings.cfg", "ostrog-debug.log", "ostrog-debug.log.1", "announce.cfg"]
	var on: Array = FilesPage.deletable_logs(logs, data, true)
	check(on == ["logs/godot2026-11-13T20.10.01.log", "logs/godot2026-11-12T08.00.00.log", "ostrog-debug.log.1"], "debug on: %s" % [on])
	var off: Array = FilesPage.deletable_logs(logs, data, false)
	check(off.has("ostrog-debug.log") and not off.has("logs/godot.log"), "debug off: its log may go, the engine's own never: %s" % [off])
	for keep in ["settings.cfg", "announce.cfg", "logs/notes.txt"]:
		check(not off.has(keep), "never %s" % keep)

# FIXLOG 2026-11-14: Esc closes the panel on top. The manager's top() is the panel opened
# last that is still open, so People opened under History is reached by a second Esc.
# D92, FIXLOG 2026-11-14: the centred reading panels are one group, read one at a time; opening
# one closes the other, and a panel at the edge (inspect) stays open beside it.
func test_reading_panels_open_one_at_a_time() -> void:
	var pm = load("res://scripts/ui/panel_manager.gd").new()
	var people := Control.new(); people.visible = false
	var legends := Control.new(); legends.visible = false
	var inspect := Control.new(); inspect.visible = false
	pm.register(&"people", people, &"reading")
	pm.register(&"legends", legends, &"reading")
	pm.register(&"inspect", inspect)
	pm.open(&"people")
	pm.open(&"inspect")
	check(pm.is_open(&"people") and pm.is_open(&"inspect"), "an edge panel opens beside a reading panel")
	pm.open(&"legends")
	check(not pm.is_open(&"people") and not people.visible, "opening Legends closes People")
	check(pm.is_open(&"inspect") and pm.top() == &"legends", "the edge panel stays; Legends is on top")
	pm.toggle(&"people")
	check(pm.is_open(&"people") and not pm.is_open(&"legends"), "and the other way round")
	for n in [people, legends, inspect]:
		n.free()

func test_panel_manager_top_is_the_last_opened_still_open() -> void:
	var pm = load("res://scripts/ui/panel_manager.gd").new()
	var a := Control.new(); a.visible = false
	var b := Control.new(); b.visible = false
	var c := Control.new(); c.visible = true
	check(pm.top() == &"", "nothing registered: no top")
	pm.register(&"a", a); pm.register(&"b", b); pm.register(&"c", c)
	check(pm.top() == &"c", "a panel visible when registered counts as open: %s" % pm.top())
	pm.open(&"a"); pm.open(&"b")
	check(pm.top() == &"b", "the last opened is on top: %s" % pm.top())
	pm.close(pm.top())
	check(pm.top() == &"a" and not b.visible, "closing the top shows the one under it: %s" % pm.top())
	pm.open(&"c")
	check(pm.top() == &"c", "opening again raises it: %s" % pm.top())
	pm.close_all()
	check(pm.top() == &"" and not a.visible and not c.visible, "all closed: no top")
	for n in [a, b, c]:
		n.free()

# FIXLOG 2026-11-14: the wizard's valley page spoke the describe command's lines ("character:
# wetness +0.14 ...", "flora not drawn: ..."). It now says the valley in a player's words, from a
# real preview, so a change to those lines fails here rather than reaching the page.
func test_the_wizard_says_the_valley_in_a_players_words() -> void:
	var sim := SimHost.new()
	var p: Dictionary = sim.valley_preview(1, PackedFloat32Array([0.14, 1.0, -0.36]), -1)
	check(not p.is_empty() and p.has("lines") and p.has("label"), "a preview with lines and a label")
	var note: String = Rules.valley_note(Array(p["lines"]), str(p["label"]))
	for dev in ["character:", "wetness", "set by parameter", "biome mix", "flora", "not drawn", " of 22", "%"]:
		check(not note.contains(dev), "no '%s' on the page: %s" % [dev, note])
	check(note.begins_with("A ") and not note.begins_with("A a ") and not note.begins_with("A an "), "one article: %s" % note)
	check(note.to_lower().begins_with(str(p["label"]).to_lower()) or note.to_lower().begins_with("a " + str(p["label"]).to_lower()), "it opens with the valley's kind: %s" % note)
	check(note.contains("Mostly ") and note.contains("Wild plants here: "), "the ground and the plants: %s" % note)
	var made := Rules.valley_note(["biome mix: grassland 31%, fen 23%, heath 4%, fell <1%", "flora draw: 2 of 22 wild kinds: yews, hazels", "flora not drawn: alders"], "a mild valley")
	check(made == "A mild valley.\nMostly grassland and fen.\nWild plants here: yews, hazels.", "the wording: %s" % made)

# FIXLOG 2026-11-14: each Settings slider shows its value; autosave at the left end says "off".
func test_settings_sliders_say_their_value() -> void:
	check(Rules.setting_text("autosave_days", 0.0) == "off", "autosave off at the left end")
	check(Rules.setting_text("autosave_days", 0.25) == "every 6 hours", "a quarter day")
	check(Rules.setting_text("autosave_days", 0.05) == "every 1 hour", "the smallest step")
	check(Rules.setting_text("autosave_days", 1.0) == "every day", "one day: %s" % Rules.setting_text("autosave_days", 1.0))
	check(Rules.setting_text("autosave_days", 1.5) == "every 1.5 days", "a day and a half")
	check(Rules.setting_text("autosave_days", 5.0) == "every 5 days", "five days")
	check(Rules.setting_text("master", 0.8) == "80%" and Rules.setting_text("music", 0.0) == "0%", "volumes as a percentage")

# D94, FIXLOG 2026-11-14: Load says plainly when nothing is saved, and the bar's Load asks first,
# saying how much play would be lost.
func test_load_says_what_is_lost_and_when_nothing_is_saved() -> void:
	check(not Rules.LOAD_NOTHING_SAVED.contains("manifest") and Rules.LOAD_NOTHING_SAVED.contains("Save (F5)"), "nothing saved: in a player's words")
	check(Rules.saved_tick('{"tick": 289161537, "format": 3}') == 289161537, "the save's tick from its manifest")
	check(Rules.saved_tick("") == -1 and Rules.saved_tick("[]") == -1, "no manifest: no tick")
	var base := 289000000
	check(Rules.load_question(base, base + 100).ends_with("(less than a day) is lost."), "less than a day")
	check(Rules.load_question(base, base + 28800 + 5).ends_with("The day since then is lost."), "one day")
	check(Rules.load_question(base, base + 3 * 28800).ends_with("The 3 days since then are lost."), "three days")
	check(Rules.load_question(-1, base) == "Load the valley as you saved it?", "an unknown save time: just the question")

# #1485, FIXLOG 2026-11-14: a feed line stays for game time by severity, clamped to 30-180 real
# seconds, instead of a fixed 45 s.
func test_feed_keeps_majors_longer() -> void:
	check(Rules.feed_hold_ms(3, 1) == 180000 and Rules.feed_hold_ms(2, 1) == 120000, "1x: major 180 s, notable 120 s")
	check(Rules.feed_hold_ms(3, 10) == 72000 and Rules.feed_hold_ms(2, 10) == 30000, "10x: major 72 s, notable 30 s")
	check(Rules.feed_hold_ms(3, 3) == 180000 and Rules.feed_hold_ms(2, 3) == 40000, "3x: major clamped to 180 s, notable 40 s")
	check(Rules.feed_hold_ms(1, 1) == 60000 and Rules.feed_hold_ms(0, 10) == 30000, "minor: one game-hour, at least 30 s")
	check(Rules.feed_hold_ms(3, 0) == Rules.feed_hold_ms(3, 1), "paused counts as 1x")
	check(Rules.feed_hold_ms(3, 10) > Rules.feed_hold_ms(2, 10), "a major line outlives a notable one")

# FIXLOG 2026-11-14: in a running game the feed stayed empty (and event sounds silent): the feed
# went to the step thread with each batch, and step() hands the next batch over the moment one comes
# home. Stepped and drained every frame as main.gd does, lines must arrive while it runs, and the
# same lines as stepping in place, none lost or doubled.
func test_feed_arrives_while_the_game_runs() -> void:
	# FIXLOG 2026-11-19: a poll cap compared different durations on slower workers.
	var runs: Array[Dictionary] = []
	for threaded in [true, false]:
		var sim := SimHost.new()
		sim.set_threaded(false)
		var opening_error := sim.opening(5, [0, 1, 2], PackedInt64Array([]), "steady")
		check(opening_error == "", "opening seed 5")
		if opening_error != "":
			return
		sim.set_threaded(threaded)
		var run := FeedHarness.collect(sim, threaded, Time.get_ticks_msec,
			func(): OS.delay_msec(1))
		runs.append(run)
		if run.error != "":
			check(false, "feed run completion failed: %s; asked %d, completed %d" % [run.error, run.asked, run.ticks - run.start])
			return
	print("GD_TEST_INFO feed rows: threaded %d (%d while running), in place %d; ticks %s" % [runs[0].rows, runs[0].while_running, runs[1].rows, [runs[0].ticks, runs[1].ticks]])
	var errors := FeedHarness.parity_errors(runs[0], runs[1])
	check(errors.is_empty(), "6000-tick feed completion, live delivery and parity: %s" % [errors])

# FIXLOG 2026-11-14: a tile's "Built by ..." note comes from the chronicle, which travels with the
# session on the step thread; inspected while a batch ran, the note was left off.
func test_block_inspection_names_the_builder_while_the_game_runs() -> void:
	var sim := SimHost.new()
	sim.set_threaded(false)
	check(sim.opening(5, [0, 1, 2], PackedInt64Array([]), "steady") == "", "opening seed 5")
	sim.step(12000)
	var c: Vector3i = sim.settlement_centre()
	var found := Vector3i(-1, -1, -1)
	for dy in range(-12, 13):
		for dx in range(-12, 13):
			for dz in [0, 1, -1]:
				if found.x >= 0:
					break
				var d: Dictionary = sim.inspect_block(c.x + dx, c.y + dy, c.z + dz)
				for n in d.get("notes", []):
					if str(n).begins_with("Built by "):
						found = Vector3i(c.x + dx, c.y + dy, c.z + dz)
	check(found.x >= 0, "something near the settlement was built by someone")
	if found.x < 0:
		return
	sim.set_threaded(true)
	sim.step(20)  # a batch on the step thread, as in a running game
	var notes: Array = sim.inspect_block(found.x, found.y, found.z).get("notes", [])
	var named := false
	for n in notes:
		named = named or str(n).begins_with("Built by ")
	check(named, "inspected while the game runs, it still says who built it: %s" % [notes])
	sim.sync()

# #1418, FIXLOG 2026-11-14: the inspect panel shows what is on a person's mind, coloured by sign.
func test_inspect_moodlets_colour_by_sign() -> void:
	var m := Rules.format_moodlets(["lost someone -0.60", "a good talk +0.15"])
	var lines := m.strip_edges().split("\n")
	check(lines.size() == 3 and lines[0] == "on their mind:", "a heading and two lines: %s" % m)
	check(lines[1].contains("#e08a7a") and lines[1].contains("lost someone -0.60"), "the loss in red, first: %s" % lines[1])
	check(lines[2].contains("#8fd48f") and lines[2].contains("a good talk +0.15"), "the talk in green, second: %s" % lines[2])
	check(Rules.format_moodlets([]) == "", "nothing on their mind: no block")
	var sim := SimHost.new()
	sim.stage_demo(1)
	var d: Dictionary = sim.inspect(0)
	check(d.has("stress") and d.has("thoughts"), "the bridge sends stress and thoughts")

# #1740, FIXLOG 2026-11-14: the HUD shows the watching deity by name; the step time and the
# engine's strain only with --debug-hud.
func test_hud_hides_debug_fields_by_default() -> void:
	var off: Array = Rules.hud_watch_fields(false, "Aun", 0.34, 0.73)
	var joined_off := " ".join(off)
	check(off == ["Aun watches"], "off: the deity by name alone: %s" % [off])
	check(not joined_off.contains("ms") and not joined_off.contains("%"), "off: no ms, no %%")
	var on: Array = Rules.hud_watch_fields(true, "Aun", 0.34, 0.73)
	var joined_on := " ".join(on)
	check(joined_on.contains("strain 34%") and joined_on.contains("step 0.73 ms"), "on: both: %s" % [on])
	var sim := SimHost.new()
	sim.stage_demo(1)
	check(str(sim.deity_name()) != "" and not str(sim.deity_name()).contains("%"), "the bridge gives the deity by name: %s" % sim.deity_name())
	check(sim.engine_strain() >= 0.0 and sim.engine_strain() <= 1.0, "and the strain apart, 0 to 1")

# #1429, FIXLOG 2026-11-15: the hovered person is named beside them in the world, whatever the
# name-tag setting, and before any other tag.
func test_hover_tag_draws_beside_the_person() -> void:
	var v := _TagView.new()
	var rows := PackedInt32Array()
	rows.resize(3 * v.AGENT_STRIDE)
	var nodes := []
	for k in 3:
		rows[k * v.AGENT_STRIDE] = 11 + k
		var n := Node2D.new()
		n.position = Vector2(100 + 200 * k, 300)
		nodes.append(n)
		v.entities.animated.append(["person", n, k * v.AGENT_STRIDE, ""])
	v.agents = rows
	var tags = load("res://scripts/render/name_tags.gd").new()
	root.add_child(tags)
	tags.init(v, _TagSim.new())
	tags.mode = 0
	tags.hover_id = 12
	tags.hover_text = Rules.hover_tag({"name": "Kella", "activity": "foraging"})
	var t: Array = tags.tags()
	check(t.size() == 1 and t[0][0] == 12 and t[0][1] == "Kella, foraging", "names off, a hover still tags that one person: %s" % [t])
	var w := ThemeDB.fallback_font.get_string_size("Kella, foraging", HORIZONTAL_ALIGNMENT_LEFT, -1, 12).x
	var head: Vector2 = nodes[1].position + Vector2(16, -44)
	if t.size() == 1:
		check(absf(t[0][2].x + w / 2.0 - head.x) < 0.5 and absf(t[0][2].y - head.y) < 0.5, "centred over row 1's person: %s vs %s" % [t[0][2], head])
	tags.mode = 1
	tags.followed = 13
	t = tags.tags()
	check(t.size() == 2 and t[0][0] == 12 and t[1][0] == 13, "the hover comes before the followed person: %s" % [t])
	tags.hover_id = -1
	tags.mode = 0
	check(tags.tags().is_empty(), "no hover, names off: nothing")
	check(Rules.hover_tag({"name": "Kella", "activity": ""}) == "Kella" and Rules.hover_tag({}) == "", "a name alone; nothing without one")
	check(Rules.hover_tag({"name": "Galela", "activity": "talking — at (80, 53, z2 — surface)"}) == "Galela, talking", "the place is left off: the tag is at the person")
	tags.queue_free()
	for n in nodes:
		n.free()

# #1428, FIXLOG 2026-11-15: names over people follow the setting and the camera's zoom.
func test_name_tags_follow_setting_and_zoom() -> void:
	var ids := [11, 12, 13]
	var favs := {12: true}
	var followed := 13
	check(Rules.name_tag_ids(ids, 0, 1.5, favs, followed, -1).is_empty(), "off: no tags")
	check(Rules.name_tag_ids(ids, 1, 1.5, favs, followed, -1) == [12, 13], "favourites and followed: the favourite and the followed (%s)" % [Rules.name_tag_ids(ids, 1, 1.5, favs, followed, -1)])
	check(Rules.name_tag_ids(ids, 2, 1.5, favs, followed, -1) == [11, 12, 13], "everyone, close up: all three")
	check(Rules.name_tag_ids(ids, 2, 1.0, favs, followed, -1) == [12, 13], "everyone, zoomed out: as favourites and followed")
	check(Rules.name_tag_ids(ids, 1, 1.0, {}, -1, 11) == [11], "the selected person is tagged too")
	check(Rules.name_tag_name(1) == "favourites and followed" and Rules.name_tag_name(9) == "everyone up close", "names clamp")
	var p = load("res://scripts/persist.gd").new()
	check(int(p.settings.get("name_tags", -1)) == 1, "the default is favourites and followed")

# #1484, FIXLOG 2026-11-14: the same news again joins the line already showing with a count.
func test_feed_folds_repeats_with_count() -> void:
	var lines := []
	for k in 14:
		lines = Rules.feed_fold(lines, {"t": k, "hold": 120000, "key": "suspected:7", "count": 1,
			"raw": "Irlin suspected Galn of the fever.", "because": "", "col": "#ffe9a8", "fav": false, "sev": 2,
			"text": "", "pos": Vector3i(1, 2, 3)})
	lines = Rules.feed_fold(lines, {"t": 20, "hold": 180000, "key": "", "count": 1,
		"raw": "Raiders came over the ridge.", "because": "", "col": "#ffb3a0", "fav": false, "sev": 3,
		"text": "raid", "pos": Vector3i(4, 5, 6)})
	check(lines.size() == 2, "fourteen of one kind and a raid: two lines (%d)" % lines.size())
	check(str(lines[0]["raw"]).ends_with("(x14)") and int(lines[0]["count"]) == 14, "the first reads (x14): %s" % lines[0]["raw"])
	check(str(lines[0]["text"]).contains("(x14)") and int(lines[0]["t"]) == 13, "its text and time are the newest one's")
	check(str(lines[1]["text"]) == "raid", "the raid is its own line")
	var after := Rules.feed_fold(lines, {"t": 30, "hold": 1, "key": "suspected:7", "count": 1, "raw": "Irlin suspected Galn of the fever.",
		"because": "", "col": "#ffe9a8", "fav": false, "sev": 2, "text": "", "pos": Vector3i(1, 2, 3)})
	check(after.size() == 2 and str(after[1]["raw"]).ends_with("(x15)"), "a repeat after the raid moves to the newest place")
	var plain := Rules.feed_fold([{"key": "", "text": "a"}], {"key": "", "text": "b"})
	check(plain.size() == 2, "lines without a key never fold")

# #1487, FIXLOG 2026-11-14: the same news on later mornings toasts "again", then not at all.
func test_toast_repeat_across_days_says_again() -> void:
	var seen := {}
	var morning := 289000000
	var nth := []
	for d in 4:
		nth.append(Rules.toast_repeat(seen, "custom_formed:3:take in no strangers", morning + d * 28800))
	check(nth == [1, 2, 3, 4], "first, again, again, then not toasted: %s" % [nth])
	check(Rules.toast_again(2, morning, morning + 28800) == "(again, the 2nd time in 2 days)", Rules.toast_again(2, morning, morning + 28800))
	check(Rules.toast_again(3, morning, morning + 2 * 28800) == "(again, the 3rd time in 3 days)", "third")
	check(Rules.toast_repeat(seen, "custom_formed:3:take in no strangers", morning + 15 * 28800) == 1, "after ten quiet game-days it is news again")
	check(Rules.toast_repeat(seen, "suspected:7", morning) == 1, "another key is its own news")

## Stand-ins for the name-tag layer's view, entity layer and bridge (#1429).
class _TagEntities extends RefCounted:
	var animated: Array = []
	func _person_offset(_plan) -> Vector2:
		return Vector2(0, -40)

class _TagView extends RefCounted:
	var AGENT_STRIDE := 25
	var agents := PackedInt32Array()
	var entities := _TagEntities.new()

class _TagSim extends RefCounted:
	func agent_name(id: int) -> String:
		return "P%d" % id
	func is_favourite(_id: int) -> bool:
		return false

## A stand-in for world_view: the Rust renderer reads only sim, view_z and _tex().
class _RendererView extends Node:
	var sim
	var view_z := 0
	func _tex(_p: String) -> Texture2D:
		return PlaceholderTexture2D.new()

func batch_mode_draws_people_once_through_the_layered_renderer() -> void:
	var sim := SimHost.new()
	sim.stage_demo(1)
	var key := "ostrog/render/world_renderer"
	var previous = ProjectSettings.get_setting(key, false)
	ProjectSettings.set_setting(key, true)
	var world := WorldView.new()
	root.add_child(world)
	world.setup(sim, preload("res://scripts/assets.gd").new())
	ProjectSettings.set_setting(key, previous)
	world.view_z = sim.founding_site().z
	world.refresh()
	var people := {}
	for entry in world.entities.animated:
		check(entry[0] == "person", "the animal batch still owns animals")
		var id: int = world.agents[entry[2]]
		check(not people.has(id), "one layered sprite per person")
		people[id] = true
	for i in range(0, world.agents.size(), world.AGENT_STRIDE):
		if world.agents[i + 3] == world.view_z and world.agents[i + 4] == 0:
			check(people.has(world.agents[i]), "batch mode draws each person on this level")
	check(not people.is_empty(), "fixture contains people in view")
	for batch in world.rust_renderer.get_children():
		check(not batch.texture.resource_path.begins_with("res://assets/people/"), "no duplicate body batch")
	world.free()
	sim.free()

# WP-06 6.4 (FIXLOG 2026-11-13): the Rust renderer rebuilds only what changed. It runs every frame
# from world_view._process and again in refresh(), and rebuilding everything each time (the whole
# level's ground included) cost about 15 ms of every frame on the main thread at 500 people.
func test_rust_renderer_rebuilds_only_what_changed() -> void:
	var sim := SimHost.new()
	sim.stage_demo(1)
	var v := _RendererView.new()
	v.sim = sim
	v.view_z = sim.founding_site().z
	var rr = preload("res://scripts/render/rust_world_renderer.gd").new()
	rr.setup(v)
	check(rr.builds["animals"] == 1 and rr.builds["static"] == 1 and rr.builds["ground"] == 1, "setup builds each batch once")
	rr.rebuild(1.0 / 60.0)
	check(rr.builds["animals"] == 1 and rr.builds["static"] == 1 and rr.builds["ground"] == 1, "a frame with nothing changed rebuilds nothing")
	var t0: int = sim.tick()
	sim.step(1)
	sim.sync()
	check(sim.tick() > t0, "fixture: the tick advanced")
	rr.rebuild(1.0 / 60.0)
	check(rr.builds["animals"] == 2, "a new tick rebuilds animals")
	check(rr.builds["static"] == 1, "static things wait for their half second")
	rr.rebuild(0.6)
	check(rr.builds["static"] == 2, "after half a second static things rebuild")
	var ground_before: int = rr.builds["ground"]
	v.view_z += 1
	rr.rebuild(0.0)
	check(rr.builds["animals"] == 3 and rr.builds["static"] == 3 and rr.builds["ground"] == ground_before + 1, "a new level rebuilds all three")
	rr.free()
	v.free()
	sim.free()

# FIXLOG 2026-11-13 (DoD row 1 run): closing the window saves the valley first, but only a begun
# game that moved since its last autosave, never a review run, never headless without --ring, and
# not when autosave is off.
func test_closing_the_window_saves_only_a_running_game_that_moved() -> void:
	var R := preload("res://scripts/rules.gd")
	check(R.save_on_close(true, false, false, false, 1.0, 5000, 0), "a running game that moved is saved on close")
	check(not R.save_on_close(false, false, false, false, 1.0, 5000, 0), "the wizard, before Begin, is not saved")
	check(not R.save_on_close(true, true, false, false, 1.0, 5000, 0), "a screenshot, footage, stage or wizard-shot run is not saved")
	check(not R.save_on_close(true, false, false, false, 0.0, 5000, 0), "autosave switched off: nothing is saved on close")
	check(not R.save_on_close(true, false, false, false, 1.0, 5000, 5000), "nothing moved since the last autosave: no extra save")
	check(not R.save_on_close(true, false, true, false, 1.0, 5000, 0), "headless without --ring is not saved")
	check(R.save_on_close(true, false, true, true, 1.0, 5000, 0), "headless with --ring is saved, as the autosave would be")

# OST-011: Catch Up has its own panel at the left edge under the HUD, as tall as its text, clear of
# the middle where the reading panels (History among them) open, and clear of the bottom bar.
func test_catch_up_panel_sits_at_the_edge_clear_of_the_reading_panels() -> void:
	for view in [Vector2(1600, 900), Vector2(1280, 720)]:
		var r: Rect2 = Rules.catch_up_rect(view, 90.0, 400.0)
		check(r.end.x < view.x / 2.0 - 40.0, "%s: clear of the middle (right edge %.0f)" % [view, r.end.x])
		check(r.position.y >= 90.0, "%s: under the HUD" % view)
		check(r.size.y == 400.0, "%s: as tall as its text" % view)
		check(r.end.y <= view.y - Rules.HELP_BAR_ROOM, "%s: clear of the bottom bar" % view)
	var tall: Rect2 = Rules.catch_up_rect(Vector2(1280, 720), 140.0, 2000.0)
	check(tall.end.y <= 720.0 - Rules.HELP_BAR_ROOM, "a short window: capped above the bar")
	var narrow: Rect2 = Rules.catch_up_rect(Vector2(400, 720), 90.0, 400.0)
	check(narrow.end.x <= 390.0, "a narrow window: inside it")

# OST-011: opening a reading panel closes the others of its group; Catch Up is in none, so History
# opening does not close or replace it.
func test_opening_history_does_not_close_catch_up() -> void:
	var pm := PanelManager.new()
	var catchup := PanelContainer.new()
	var summary := PanelContainer.new()
	var legends := PanelContainer.new()
	pm.register(&"catchup", catchup)
	pm.register(&"summary", summary, &"reading")
	pm.register(&"legends", legends, &"reading")
	pm.open(&"catchup")
	pm.open(&"summary")
	check(pm.is_open(&"catchup") and pm.is_open(&"summary"), "History opens beside Catch Up")
	pm.open(&"legends")
	check(pm.is_open(&"catchup") and not pm.is_open(&"summary"), "another reading panel replaces History, not Catch Up")
	pm.close(&"catchup")
	check(not pm.is_open(&"catchup") and pm.is_open(&"legends"), "closing Catch Up leaves the reading panel")
	for n in [catchup, summary, legends]:
		n.free()

# OST-011: the bridge hands the client a Catch Up card (BBCode text) beside the unchanged
# `summary_since` counts the timelapse still reads.
func test_catch_up_card_comes_through_the_bridge_and_summary_since_is_unchanged() -> void:
	var sim := SimHost.new()
	sim.stage_demo(1)
	var from: int = sim.tick()
	sim.step(24000)
	var card: Dictionary = sim.catch_up_since(from)
	check(card.has("text") and card["text"] is String, "the card has its text")
	check(int(card["to"]) > int(card["from"]), "it spans the time that passed")
	var raw: Dictionary = sim.summary_since(from)
	check(raw.has("counts") and raw.has("lines") and raw.has("counts_words"), "summary_since keeps its raw counts and lines")
	sim.free()

# OST-012: coming back opens Catch Up only after MORE than 300 s away and only with news (a named
# change or notable line); 299 and 300 s do not, 301 s does, and no news never does.
func test_catch_up_opens_on_return_after_idle_with_news() -> void:
	check(Rules.CATCH_UP_IDLE_S == 300.0, "the default is 300 s")
	check(not Rules.catch_up_due(299.0, true), "299 s with news: not due")
	check(not Rules.catch_up_due(300.0, true), "300 s with news: not due (more than 300)")
	check(Rules.catch_up_due(301.0, true), "301 s with news: due")
	check(not Rules.catch_up_due(400.0, false), "400 s with no news: not due")
	check(not Rules.catch_up_due(30.0, true), "30 s with news: not due")

# OST-012: the manual control: one key and one button, and a span for each name the bridge knows.
func test_catch_up_has_one_control_and_the_spans_the_bridge_knows() -> void:
	var with_key := 0
	for b in Rules.BAR_ITEMS:
		if str(b.get("key", "")) == "U":
			with_key += 1
			check(str(b["id"]) == "catchup", "the Catch Up button owns U")
	check(with_key == 1, "one button has U: %d" % with_key)
	# The key must not collide with anything main.gd already binds (A was the pan-left key).
	var src := FileAccess.get_file_as_string("res://scripts/main.gd")
	check(src.count("KEY_U:") == 1, "KEY_U is bound once in main.gd: %d" % src.count("KEY_U:"))
	check(src.count("is_key_pressed(KEY_U)") == 0, "U is not a held movement key")
	check(src.count("\t\t\tKEY_A:") == 0, "A is left to the pan keys")
	var letters := {}
	for b in Rules.BAR_ITEMS:
		var k := str(b.get("key", ""))
		if k.length() == 1:
			check(not letters.has(k), "bar key %s used twice" % k)
			letters[k] = true
	var ids := []
	for sp in Rules.CATCH_UP_SPANS:
		ids.append(sp["id"])
	check(ids == ["since_look", "current_day", "previous_day", "current_season", "previous_season"], "the spans: %s" % [ids])
	var sim := SimHost.new()
	sim.stage_demo(1)
	sim.step(24000)
	for sid in ["current_day", "current_season"]:
		var d: Dictionary = sim.catch_up_span(sid)
		check(d.has("text") and d.has("has_news") and int(d["to"]) >= int(d["from"]), "%s answers with a card" % sid)
	check(sim.catch_up_span("last_week").is_empty(), "an unknown span answers nothing")
	var since: Dictionary = sim.catch_up_since(0)
	check(since.has("has_news"), "since-look says whether there is news")
	sim.free()
