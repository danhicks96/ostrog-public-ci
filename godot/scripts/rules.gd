extends RefCounted
## The client's small decisions as pure functions, so tools/gd_test.gd can check each one
## headless (C63: the QA round of 2026-10-20 found client faults no test could have caught).
## Scripts use it as `const Rules := preload("res://scripts/rules.gd")`.

const TILE := 32
## Zoom levels the wheel steps through: whole numbers of screen pixels per art pixel above 1x,
## so nearest-filtered art never shimmers as the camera pans (#183).
const ZOOMS := [0.5, 1.0, 2.0, 3.0, 4.0]
## How near the pointer must be to a person to pick them, in screen pixels at any zoom (#183).
const PICK_PX := 22.0
## The HUD's width: never past the inspect panel's left edge (#181).
const HUD_MAX := 900.0
const INSPECT_W := 340.0
## The panels read in the middle of the screen: announcements wait while one is open (D91,
## FIXLOG 2026-11-14). The inspect, block and help panels sit at the edges, clear of them.
const TOAST_HOLDING_PANELS := [&"summary", &"legends", &"people", &"waiting", &"favourites", &"timelapse"]

static func toasts_held(open_ids: Array, settings_open: bool) -> bool:
	if settings_open:
		return true
	for id in open_ids:
		if TOAST_HOLDING_PANELS.has(id):
			return true
	return false

## The wizard's words for the valley a seed would make, from the preview's lines (the same lines
## `ostrog describe` prints, kept as they are for the CLI and its tests). Shown raw they read
## as a debug print: "character: wetness +0.14 warmth +1.00 stoniness -0.36 (a mild valley), set
## by parameter", "flora not drawn: ..." (FIXLOG 2026-11-14). `label` is the preview's own
## ("a mild valley"); a kind of ground under a tenth of the valley is left out.
static func valley_note(lines: Array, label: String) -> String:
	var out: PackedStringArray = []
	if label != "":
		# The label carries its own article ("a mild valley").
		var said := label if label.begins_with("a ") or label.begins_with("an ") else "a " + label
		out.append(said.substr(0, 1).to_upper() + said.substr(1) + ".")
	for raw in lines:
		var l := str(raw)
		if l.begins_with("biome mix: "):
			var main_kinds: PackedStringArray = []
			for part in l.trim_prefix("biome mix: ").split(", "):
				var at := part.rfind(" ")
				if at < 0:
					continue
				var pc := part.substr(at + 1)
				if pc.ends_with("%") and not pc.begins_with("<") and int(pc.trim_suffix("%")) >= 10:
					main_kinds.append(part.substr(0, at))
			if not main_kinds.is_empty():
				out.append("Mostly %s." % _and_list(main_kinds))
		elif l.begins_with("flora draw: ") and l.count(": ") >= 2:
			var kinds := l.substr(l.find(": ", 12) + 2)
			if kinds != "":
				out.append("Wild plants here: %s." % kinds)
	return "\n".join(out)

static func _and_list(items: PackedStringArray) -> String:
	if items.size() <= 1:
		return "".join(items)
	return ", ".join(items.slice(0, items.size() - 1)) + " and " + items[items.size() - 1]

## A Settings slider's value in words, shown beside it (FIXLOG 2026-11-14): with no number a player
## could not tell autosave every day from every five, nor see that the left end turns it off.
static func setting_text(key: String, v: float) -> String:
	if key == "autosave_days":
		if v <= 0.0:
			return "off"
		if v < 1.0:
			var h := maxi(1, int(round(v * 24.0)))
			return "every %d hour%s" % [h, "" if h == 1 else "s"]
		if is_equal_approx(v, 1.0):
			return "every day"
		var d := snappedf(v, 0.1)
		return ("every %d days" % int(d)) if is_equal_approx(d, round(d)) else ("every %.1f days" % d)
	return "%d%%" % int(round(clampf(v, 0.0, 1.0) * 100.0))

## Loading the manual save (D94, FIXLOG 2026-11-14). With nothing saved the note says so in a
## player's words ("load failed: manifest missing" before); the bar's Load asks first, saying what
## would be lost. F9 stays an instant quick-load.
const LOAD_NOTHING_SAVED := "nothing saved yet: Save (F5) keeps the valley to come back to"

## The game tick a save's manifest.json records, or -1.
static func saved_tick(manifest_text: String) -> int:
	var m = JSON.parse_string(manifest_text)
	if typeof(m) != TYPE_DICTIONARY or not m.has("tick"):
		return -1
	return int(m["tick"])

## The bar's Load question, with how much play would be lost (a game day is 28 800 ticks).
static func load_question(saved: int, now: int) -> String:
	var q := "Load the valley as you saved it?"
	if saved < 0 or now <= saved:
		return q
	var days := int(float(now - saved) / 28800.0)
	if days < 1:
		return q + " What happened since (less than a day) is lost."
	if days == 1:
		return q + " The day since then is lost."
	return q + " The %d days since then are lost." % days

## How long a feed line stays, in real ms (#1485, FIXLOG 2026-11-14): a span of game time by
## severity (minor 1 game-hour, notable 2, major 12; 1 200 ticks a game-hour, 20 ticks a real
## second at 1x, docs/TIME.md) at the current speed, clamped to 30-180 real seconds. Paused counts
## as 1x, so a line neither vanishes at once nor stays for ever. It was a fixed 45 s: a death went
## as fast as a talk, and at 10x a line was gone after 7.5 game-hours.
static func feed_hold_ms(sev: int, speed: int) -> int:
	var game_hours := 12.0 if sev >= 3 else (2.0 if sev == 2 else 1.0)
	var ticks := game_hours * 1200.0
	var real_s := ticks / (20.0 * float(maxi(speed, 1)))
	return int(round(clampf(real_s, 30.0, 180.0) * 1000.0))

## What is on a person's mind for the inspect panel (#1418, FIXLOG 2026-11-14): the bridge's
## thoughts ("lost someone -0.60", strongest first), one per line, green when it lifts the mood and
## red when it weighs on it. The panel received them and never drew them.
static func format_moodlets(thoughts: Array) -> String:
	if thoughts.is_empty():
		return ""
	var out := "on their mind:\n"
	for t in thoughts:
		var line := str(t)
		var w := float(line.get_slice(" ", line.get_slice_count(" ") - 1))
		var col := "#8fd48f" if w > 0.0 else ("#e08a7a" if w < 0.0 else "#c9c9c9")
		out += "  [color=%s]%s[/color]\n" % [col, line]
	return out

## The HUD's watcher fields (#1740, FIXLOG 2026-11-14): the deity by name ("Aun watches"); the
## engine's strain and the step time only with --debug-hud. They made the always-on line read as a
## tool, and the strain is the storyteller's hidden reading (#1560).
static func hud_watch_fields(debug_hud: bool, deity: String, strain: float, step_ms: float) -> Array:
	var watches := ("%s watches" % deity) if deity != "" else ""
	if not debug_hud:
		return [watches]
	return ["%s  strain %d%%" % [watches, int(round(strain * 100.0))], "step %.2f ms" % step_ms]

## The feed's width (#182).
const FEED_W := 510.0
## The Mouse & Keys panel's width, and the room it leaves for the bottom bar (#797 finding 1).
const HELP_W := 560.0
const HELP_BAR_ROOM := 60.0

## Where Mouse & Keys goes: at the left edge just under the HUD, as tall as its text, never over
## the bottom bar. In the middle of the view it covered the founding spot on a first start, the
## one moment a player has not yet learned to close it (FIXLOG 2026-11-13, #797 finding 1).
static func help_rect(view: Vector2, hud_bottom: float, content_h: float) -> Rect2:
	var top := hud_bottom + 8.0
	var room := maxf(160.0, view.y - HELP_BAR_ROOM - top)
	return Rect2(10.0, top, minf(HELP_W, view.x - 20.0), minf(content_h, room))

# People sheet columns for the dead (entity_layer.gd P_FALL, P_LYING).
const P_FALL := 32
const P_LYING := 33
# #485 everyday cycles appended after eat (entity_layer.gd, tools/art/people.py FRAMES).
const P_SLEEP := 26
const P_SLEEP2 := 40   # breathing in
const P_SLEEP3 := 41   # stirring
const P_TEND := 42     # tend1-3: kneel, dress, check (a Tend order, work code 8)
const P_DRINK := 45    # drink1-3: scoop, lift, drink (Activity::Drinking, pose 12)
const WORK_TEND := 8
# Animal sheet columns (tools/art/animals.py FRAMES): idle, idle2, walk1-6, sleep.
const A_WALK := 2
const A_SLEEP := 8

## The next zoom level from `z` one wheel step in `dir` (+1 in, -1 out), snapped to ZOOMS.
## Whether closing the window should save the valley first (FIXLOG 2026-11-13, the DoD row 1 run:
## the first autosave comes a game day in, about 24 real minutes at 1x, and a friend who closed the
## window sooner lost the valley). Only a begun game that has moved since its last autosave, never
## a screenshot, footage, stage or wizard-shot run, never headless without --ring, and not when
## autosave is switched off.
static func save_on_close(world_running: bool, review_run: bool, headless: bool, ring_arg: bool,
		autosave_days: float, tick: int, last_autosave_tick: int) -> bool:
	if not world_running or review_run or autosave_days <= 0.0:
		return false
	if headless and not ring_arg:
		return false
	return tick != last_autosave_tick

static func next_zoom(z: float, dir: int) -> float:
	var i := 0
	for k in ZOOMS.size():
		if absf(ZOOMS[k] - z) < absf(ZOOMS[i] - z):
			i = k
	# Off the ladder (an old bookmark, --zoom=1.25): the first step lands on the nearest level
	# in the direction asked.
	if dir > 0 and ZOOMS[i] > z + 0.001 or dir < 0 and ZOOMS[i] < z - 0.001:
		return ZOOMS[i]
	return ZOOMS[clampi(i + dir, 0, ZOOMS.size() - 1)]

## The camera centre after zooming from `old` to `new` with the point `at` (world pixels)
## under the pointer, so that point stays under it (#183: zoom was about the screen centre).
static func zoom_about(cam: Vector2, at: Vector2, old: float, new: float) -> Vector2:
	return at + (cam - at) * (old / new)

## Where an agent row is drawn (world pixels, tile centre): between its last tile and its
## next by its move progress, as entity_layer.gd draws it.
static func drawn_at(a: PackedInt32Array, i: int) -> Vector2:
	var prog := a[i + 14] / 1000.0
	return Vector2(lerpf(a[i + 12], a[i + 1], prog), lerpf(a[i + 13], a[i + 2], prog)) * TILE + Vector2(16, 16)

## The id of whoever is drawn nearest `world_pos` on level `z`, within PICK_PX screen pixels
## at camera zoom `zoom`, else -1 (#183: it compared the tile, not the drawn position, with
## a fixed world radius). The point picked is the body's middle, half a tile above the feet.
static func pick(a: PackedInt32Array, stride: int, z: int, world_pos: Vector2, zoom: float) -> int:
	var best := -1
	var bd := PICK_PX / maxf(zoom, 0.01)
	for i in range(0, a.size(), stride):
		if a[i + 3] != z:
			continue
		var dd := (drawn_at(a, i) - Vector2(0, 8)).distance_to(world_pos)
		if dd < bd:
			bd = dd
			best = a[i]
	return best

## The ladder has its own sheet; stairs up, down and both theirs (#184).
static func stair_sheet(shape: int) -> String:
	match shape:
		5: return "stairs_up"
		6: return "stairs_down"
		8: return "ladder"
	return "stairs_both"

## The frame of an animated sheet `width` pixels wide at time `t` (frames a second already
## applied): every frame the sheet has, not the first two (#184: fire had 4, drew 2).
static func fire_frame(t: float, phase: int, width: int, tile: int) -> int:
	return (int(t) + phase) % maxi(1, int(width / tile))

## An animal's frame for a bridge pose (lib.rs agents(): 0 idle, 1 walk, 2 work or graze,
## 3 carry, 4 sleep, 5 sit or eat, 6 fighting, 7 struck). The sheets have idle, walk and
## sleep only, so a fight is a quick lunge on the walk cycle, a blow a start between the idles,
## eating the slow idle bob (#187: all of these were frame 0, a fighting wolf stood still).
static func animal_frame(pose: int, t: float, id: int) -> int:
	match pose:
		1, 3: return A_WALK + int(t * 10.0 + (id % 6)) % 6
		2, 5: return int(t * 0.8 + id) % 2
		4: return A_SLEEP
		6: return A_WALK + [1, 4][int(t * 8.0 + id) % 2]
		7: return 1
	return int(t * 1.5 + id) % 2

## Consume the emitted agent row exactly as the animal renderer does (#485 shape regression).
static func animal_frame_for_row(a: PackedInt32Array, i: int, t: float) -> int:
	return animal_frame(a[i + 15], t, a[i])

## The people sheet column for the dead (lib.rs poses 8 falling, 9 lying, view::bodies), or
## -1 for any other pose (#185).
static func dead_frame(pose: int) -> int:
	match pose:
		8: return P_FALL
		9: return P_LYING
	return -1

## Frame within the three-cell talking/eating cycles appended for #485. Other poses must not
## borrow these gestures: the bridge emits 10 only for Talking and 11 only for Eating.
static func everyday_frame(pose: int, t: float) -> int:
	match pose:
		10: return int(t * 3.4) % 3
		11: return int(t * 4.0) % 3
	return -1

## The sleeping person's column at time t (#485): at rest, a breath in every few seconds, and
## now and then a stir. `id` staggers sleepers so a room does not breathe in step.
static func sleep_frame(t: float, id: int) -> int:
	var u := t + float(id % 11) * 0.53
	var breath := fmod(u, 3.2)
	if int(u / 3.2) % 9 == 4 and breath < 1.0:
		return P_SLEEP3
	return P_SLEEP2 if breath >= 2.0 else P_SLEEP

## The column of the drinking or tending cycle at time t: each frame held for its own time.
static func held_frame(base: int, t: float, times: Array) -> int:
	var total := 0.0
	for d in times:
		total += d
	var u := fmod(t, total)
	for k in times.size():
		u -= times[k]
		if u < 0.0:
			return base + k
	return base + times.size() - 1

## The head-marking sheet a hairless kind wears in the hair slot (#485, tools/art/people.py
## MARKED): one of three patterns by the hair-style gene, drawn per build and age.
static func marking_sheet(body_set: String, hair_gene: int, build_age: String) -> String:
	return "people/marks_%s_%d_%s.png" % [body_set, posmod(hair_gene, 3) + 1, build_age]

const TEND_TIMES := [0.6, 0.5, 0.7]
const DRINK_TIMES := [0.7, 0.35, 0.9]

## Contact shadows a built wall casts on the open ground beside it (#505), with the light
## from the upper left as everywhere in the art: a soft two-step shadow on the east neighbour,
## and one under the wall's southern face where it meets the floor. Rects are relative to the
## wall tile's top-left corner; each entry is [Rect2, alpha]. Nothing is cast onto another wall.
static func wall_contact_shadows(open_east: bool, open_south: bool, face_h: int) -> Array:
	var out: Array = []
	if open_east:
		out.append([Rect2(TILE, 0, 5, TILE), 0.34])
		out.append([Rect2(TILE + 5, 0, 6, TILE), 0.15])
	if open_south:
		var y := TILE + maxi(0, face_h)
		out.append([Rect2(0, y, TILE, 4), 0.34])
		out.append([Rect2(0, y + 4, TILE, 5), 0.15])
		if open_east:
			out.append([Rect2(TILE, y, 8, 6), 0.15])
	return out

## The HUD's width on a canvas `canvas_w` wide: room left for the inspect panel when it is up.
## The HUD's first line: fields joined by three spaces, the words inside a field held together
## with non-breaking spaces, so a wrap falls only between fields (#218: "step" and "0.00 ms",
## "[?]" and "keys" landed on different lines at 720p).
static func hud_line(fields: Array) -> String:
	var out: Array = []
	for f in fields:
		var s := str(f)
		if s == "":
			continue
		out.append(s.replace(" ", "\u00A0"))
	return "   ".join(out)

## Where the inspect panel's bottom edge sits: just above the canvas's bottom margin (#221: it
## stopped at 520 canvas px, 54% of the screen, while its text needed scrolling).
static func inspect_bottom(canvas_h: float) -> float:
	return -10.0 if canvas_h > 0.0 else -10.0

static func hud_width(canvas_w: float, inspect_open: bool) -> float:
	var room := canvas_w - 20.0 - (INSPECT_W + 10.0 if inspect_open else 0.0)
	return clampf(room, 240.0, HUD_MAX)

## The feed's text: its lines, and how many quieter lines were let pass, so nothing is hidden
## without a word (#182).
static func feed_text(lines: Array, dropped: int) -> String:
	var txt := ""
	for l in lines:
		txt += l["text"] + "\n"
	if dropped > 0:
		txt += "[color=#8a8a8a]+%d quieter line%s[/color]\n" % [dropped, "" if dropped == 1 else "s"]
	return txt.strip_edges(false, true)

## The feed line under a click `y` pixels below the feed's top, given each line's height,
## or -1.
static func feed_line_at(y: float, line_h: float, count: int) -> int:
	if y < 0.0 or line_h <= 0.0:
		return -1
	var k := int(y / line_h)
	return k if k < count else -1

## Does a tile stop light (#180: hearth light leaked through walls onto the field): a wall,
## rock or soil face (shape 2), or a door (flag DOOR, 1 << 3, which blocks sight in the sim).
static func blocks_light(t: int) -> bool:
	return ((t >> 16) & 0xff) == 2 or ((t >> 24) & 0x08) != 0

## Samples a light's ray takes per tile; grade.gdshader marches at the same step, so this and
## the shader agree on what is lit.
const LIGHT_STEP := 0.25

## Whether a light at `light` (world pixels) reaches `px`: march the line between them in
## LIGHT_STEP tiles and stop on any tile that blocks light, the light's own tile and the lit
## pixel's tile aside (a wall is lit on the face toward the light). `opaque` is one byte a
## tile, row-major, `w` tiles wide, non-zero where blocks_light. grade.gdshader does the same.
static func lit(opaque: PackedByteArray, w: int, h: int, light: Vector2, px: Vector2) -> bool:
	var a := light / TILE
	var b := px / TILE
	var la := Vector2i(a.floor())
	var lb := Vector2i(b.floor())
	var n := int(ceil(a.distance_to(b) / LIGHT_STEP))
	for k in range(1, n):
		var p := Vector2i(a.lerp(b, float(k) / n).floor())
		if p == la or p == lb or p.x < 0 or p.y < 0 or p.x >= w or p.y >= h:
			continue
		if opaque[p.y * w + p.x] != 0:
			return false
	return true

## A wall from the valley's past (sim.ruins) is drawn darker than a wall built today, the
## ruin overlay over it (#202).
const RUIN_TINT := Color(0.66, 0.65, 0.6)

## Which of the four ruin and rubble variants a tile takes: fixed by place, so a ruin keeps
## its look, and neighbours differ.
static func ruin_variant(x: int, y: int) -> int:
	return (x * 7 + y * 13) % 4


# ---------------------------------------------------------------- announcements (#329)
## What the viewer is told without reading the feed: 0 majors only (the default), 1 life and
## death and the valley's great turns whatever their rank, 2 every notable line, 3 nothing.
const ANNOUNCE_NAMES := ["major events", "life and death", "everything notable", "nothing"]
const ANNOUNCE_LIFE := ["born", "died", "laid_to_rest", "arrived", "joined", "fire_started",
	"besieged", "siege_lifted", "host_sated", "captured", "returned", "exiled", "rose", "shifted", "breached",
	"walled", "moved_camp", "left_the_valley", "craft_lost"]

static func announce(mode: int, sev: int, kind: String) -> bool:
	match mode:
		0: return sev >= 3
		1: return sev >= 3 or ANNOUNCE_LIFE.has(kind)
		2: return sev >= 2
		_: return false


# ---------------------------------------------------------------- mouse UI (#413)
## How far a left-button press must move before it becomes a camera drag rather than a click.
const DRAG_PX := 6.0

static func is_drag(start: Vector2, cur: Vector2) -> bool:
	return start.distance_to(cur) >= DRAG_PX

## Every button on the bottom button bar (#413), grouped, with its id, label, key name, and
## tooltip naming the key and what the button does.
const BAR_ITEMS := [
	{"id": "pause", "group": "speed", "label": "Pause", "key": "Space", "tip": "Pause or resume the simulation [Space]"},
	{"id": "spd1", "group": "speed", "label": "1×", "key": "1", "tip": "Run at normal watching speed (20 ticks/s) [1]"},
	{"id": "spd2", "group": "speed", "label": "3×", "key": "2", "tip": "Run at 3× speed [2]"},
	{"id": "spd3", "group": "speed", "label": "10×", "key": "3", "tip": "Run at 10× speed [3]"},
	{"id": "z_down", "group": "camera", "label": "Z−", "key": ", / PgDn", "tip": "Look one level deeper [, / PgDn]"},
	{"id": "z_up", "group": "camera", "label": "Z+", "key": ". / PgUp", "tip": "Look one level higher [. / PgUp]"},
	{"id": "zoom_out", "group": "camera", "label": "−", "key": "Wheel", "tip": "Zoom out [Wheel down]"},
	{"id": "zoom_in", "group": "camera", "label": "+", "key": "Wheel", "tip": "Zoom in [Wheel up]"},
	{"id": "home", "group": "camera", "label": "Home", "key": "Home", "tip": "Centre camera on the settlement [Home]"},
	{"id": "director", "group": "camera", "label": "Director", "key": "C", "tip": "Camera director: follows notable events automatically [C]"},
	{"id": "favs", "group": "views", "label": "★ Favs", "key": "J", "tip": "Favourites: people you have marked to watch and follow [J]"},
	{"id": "people", "group": "views", "label": "People", "key": "P", "tip": "People: everyone of the valley, here or away, what they are doing and why [P]"},
	{"id": "history", "group": "views", "label": "History", "key": "H", "tip": "Open the valley and colony history [H]"},
	{"id": "timelapse", "group": "views", "label": "Timelapse", "key": "T", "tip": "Record and review bounded daily snapshots of a watched view [T]"},
	{"id": "legends", "group": "views", "label": "Legends", "key": "L", "tip": "Open the Legends browser and world map [L]"},
	{"id": "known", "group": "views", "label": "Known", "key": "V", "tip": "Toggle between what the community has seen and everything [V]"},
	{"id": "feed_rate", "group": "views", "label": "Feed", "key": "G", "tip": "Cycle feed rate: calm (notable only), normal, or full [G]"},
	{"id": "announce", "group": "views", "label": "Alerts", "key": "N", "tip": "Cycle which events appear as top-screen announcements [N]"},
	{"id": "pause_major", "group": "views", "label": "Pause!", "key": "Shift+N", "tip": "Toggle pausing automatically when a major event happens [Shift+N]"},
	{"id": "waiting", "group": "views", "label": "Waiting", "key": "K", "tip": "Work nobody has taken, and why [K]"},
	{"id": "bookmarks", "group": "views", "label": "Marks", "key": "F1–F8", "tip": "Camera bookmarks: jump to F1–F8 or save with Ctrl+F1–F8 [F1–F8]"},
	{"id": "save", "group": "system", "label": "Save", "key": "F5", "tip": "Save the valley to the manual slot [F5]"},
	{"id": "load", "group": "system", "label": "Load", "key": "F9", "tip": "Load the manual save slot [F9]"},
	{"id": "settings", "group": "system", "label": "Settings", "key": "O", "tip": "Open audio, autosave, and camera settings [O]"},
	{"id": "help", "group": "system", "label": "?", "key": "?", "tip": "Show mouse and keyboard controls [? or /]"},
]

static func bar_items() -> Array:
	return BAR_ITEMS

## Format the selected person's bounded experience from `SimHost.inspect(id)` (#480).
static func format_experience(d: Dictionary) -> String:
	var s := ""
	if d.has("reaction_trace") and d["reaction_trace"].size() > 0:
		s += "why they reacted:\n"
		for r in d["reaction_trace"]:
			var expected: String = String(r.get("expected", ""))
			var expectation := ("; expected " + expected) if expected != "" else ""
			s += "  noticed %s at (%d,%d,%d), chose %s; outcome: %s%s\n" % [r["problem"], r["x"], r["y"], r["z"], r["reaction"], r["outcome"], expectation]
	if d.has("observations") and d["observations"].size() > 0:
		s += "recent observations:\n"
		for o in d["observations"]:
			var shape := "full" if bool(o.get("full", false)) else "delta"
			s += "  %s: %s, %s (%.0f%%) at (%d,%d,%d)\n" % [shape, o["kind"], o["evidence"], float(o["strength"]) * 100.0, o["x"], o["y"], o["z"]]
			var reading: String = String(o.get("reading", ""))
			if reading != "":
				s += "    read as: %s\n" % reading
	if d.has("lessons") and d["lessons"].size() > 0:
		s += "lived lessons:\n"
		for l in d["lessons"]:
			s += "  %s -> %s (confidence %.0f%%; %d confirmations, %d contradictions)\n" % [l["cue"], l["outcome"], float(l["confidence"]) * 100.0, l["confirmations"], l["contradictions"]]
	if d.has("problems") and d["problems"].size() > 0:
		s += "personal problems:\n"
		for p in d["problems"]:
			var reaction: String = String(p.get("reaction", ""))
			var action := ("; chose " + reaction) if reaction != "" else ""
			s += "  %s (%s; %s)%s; outcome: %s\n" % [p["kind"], p["status"], p["evidence"], action, p["outcome"]]
	return s

## Format the Why? lens from `SimHost.inspect(id)` (#567): the traced person's latest decision
## cycle in full (how they came to decide, the reasons, every option with its score terms, what
## was rejected and why, what was put off), then the earlier cycles one line each. Only what the
## simulation's read model recorded; nothing is inferred here.
static func format_why(d: Dictionary) -> String:
	var s := "[b]Why?[/b]"
	if not bool(d.get("why_traced", false)):
		return s + " [color=#9a9a9a]not tracing this person[/color]\n"
	s += " [color=#9a9a9a](tracing; newest first)[/color]\n"
	var cycles: Array = d.get("why_cycles", [])
	if cycles.is_empty():
		return s + "  [color=#9a9a9a]no decision since they were picked; they decide when what they are doing ends[/color]\n"
	var c: Dictionary = cycles[0]
	var how := "deliberate"
	if String(c.get("mode", "")) == "interrupted":
		how = "interrupted (%s)" % c.get("interrupted", "")
	s += "  %s, %s\n" % [c.get("when", "tick %d" % int(c.get("tick", 0))), how]
	s += "  chose [color=#e8d8a0]%s[/color]\n" % c.get("chosen", "")
	for b in c.get("because", PackedStringArray()):
		s += "    because %s\n" % b
	var options: Array = c.get("options", [])
	if options.size() > 0:
		s += "  weighed (score = base x feelings x secured gate x learned):\n"
		for o in options:
			var rejected: String = String(o.get("rejected", ""))
			if rejected != "":
				s += "    [color=#9a8a8a]x %s: %s[/color]\n" % [o["what"], rejected]
				continue
			var mark := "> " if bool(o.get("chosen", false)) else "  "
			s += "    %s[color=#8a9aa8]%s:[/color] %s  %.2f = %.2f x %.2f x %.2f x %.2f\n" % [mark, o["cycle"], o["what"], float(o["score"]), float(o["base"]), float(o["emotion"]), float(o["gate"]), float(o.get("learned", 1.0))]
	var put_off: PackedStringArray = c.get("put_off", PackedStringArray())
	if put_off.size() > 0:
		s += "  put off:\n"
		for p in put_off:
			s += "    [color=#c8b8e0]%s[/color]\n" % p
	var percepts: Array = d.get("why_percepts", [])
	if percepts.size() > 0:
		s += "  what their eye caught (%.2f to notice):\n" % 0.5
		for p in percepts:
			var seen := "noticed"
			if not bool(p.get("noticed", false)):
				seen = "habituated, not noticed" if bool(p.get("habituated", false)) else "there, not noticed"
			s += "    %s: %s, %.2f, %s\n" % [p["domain"], p["what"], float(p["salience"]), seen]
	if cycles.size() > 1:
		s += "  earlier:\n"
		for k in range(1, cycles.size()):
			var e: Dictionary = cycles[k]
			var tag := " (interrupted)" if String(e.get("mode", "")) == "interrupted" else ""
			s += "    %s: %s%s\n" % [e.get("when", "tick %d" % int(e.get("tick", 0))), e.get("chosen", ""), tag]
	return s

## What the selected person was told and what they made of it (#487), from `inspect(id)`.
static func format_told(d: Dictionary) -> String:
	var told: PackedStringArray = d.get("told", PackedStringArray())
	if told.size() == 0:
		return ""
	var s := "was told:\n"
	for t in told:
		s += "  [color=#c8c0e0]%s[/color]\n" % t
	return s

## The HUD's tracing indicator (#567): always shown while the Why? lens traces someone, so it
## is plain when tracing is on.
static func why_indicator(name: String) -> String:
	if name == "":
		return ""
	return "(o) tracing why: %s" % name

## Format the authoritative body summary sent by the simulation for person inspection.
## The summary includes wound part, tissue layer, severity and treatment (#128, C74).
static func format_body_summary(body: String) -> String:
	var text := body.strip_edges()
	if text == "":
		return ""
	return "[b]Body & wounds:[/b] [color=#f0b0a0]%s[/color]\n" % text

## Format the block inspection dictionary from `SimHost.inspect_block(x, y, z)` as BBCode (#413).
static func format_block(d: Dictionary) -> String:
	if d.is_empty():
		return "[b]Outside the valley[/b]"
	var s := "[b]%s[/b]\n" % d.get("title", "Block")
	var biome: String = d.get("biome", "")
	var surf: int = int(d.get("surface_z", 0))
	if biome != "":
		s += "[color=#a8c4a0]%s[/color]  (surface z%d)\n" % [biome, surf]
	s += "\n"
	s += "[b]Ground & rock:[/b] %s\n" % d.get("ground", "")
	var wkind: String = d.get("water_kind", "none")
	var wqual: String = d.get("water_quality", "none")
	var wcol := "#9ad0f5"
	if wkind == "swamp": wcol = "#a3b878"
	elif wkind == "foul": wcol = "#c8a070"
	elif wkind == "none": wcol = "#9a9a9a"
	var wtxt: String = d.get("water", "Dry")
	if wqual in ["salt", "brackish"] and not wtxt.to_lower().contains(wqual):
		wtxt += " (%s)" % wqual
	s += "[b]Water:[/b] [color=%s]%s[/color]\n" % [wcol, wtxt]
	s += "[b]Temperature & sky:[/b] %s\n" % d.get("temperature", "")
	var mins: PackedStringArray = d.get("minerals", PackedStringArray())
	if mins.size() > 0:
		s += "\n[b]Minerals & strata:[/b]\n"
		for m in mins:
			s += "  [color=#e0c890]%s[/color]\n" % m
	var structs: PackedStringArray = d.get("structures", PackedStringArray())
	if structs.size() > 0:
		s += "\n[b]Built here:[/b]\n"
		for st in structs:
			s += "  [color=#d8c8b0]%s[/color]\n" % st
	var plants: PackedStringArray = d.get("plants", PackedStringArray())
	if plants.size() > 0:
		s += "\n[b]Growing here:[/b]\n"
		for pl in plants:
			s += "  [color=#9fd090]%s[/color]\n" % pl
	var items: PackedStringArray = d.get("items", PackedStringArray())
	if items.size() > 0:
		s += "\n[b]Lying here:[/b]\n"
		for it in items:
			s += "  [color=#e5d5b5]%s[/color]\n" % it
	var notes: PackedStringArray = d.get("notes", PackedStringArray())
	if notes.size() > 0:
		s += "\n[b]History & traces:[/b]\n"
		for n in notes:
			s += "  [color=#c8b8d8]%s[/color]\n" % n
	return s.strip_edges(false, true)

# ---------------------------------------------------------------- readability (#413 A5, #378)
## Feed rate modes (#413 A5, #291): 0 calm (notable and major events only; all minor lines
## counted in +N quieter lines), 1 normal (minor lines throttled to one every 700 ms, up to 9
## lines), 2 full (every minor line shown with no throttle, up to 14 lines).
const FEED_RATE_NAMES := ["calm", "normal", "full"]
const FEED_MINOR_GAP_MS := [999999999, 700, 0]
const FEED_MAX_LINES := [7, 9, 14]

static func feed_rate_name(mode: int) -> String:
	return FEED_RATE_NAMES[clampi(mode, 0, FEED_RATE_NAMES.size() - 1)]

static func feed_allows_minor(mode: int, elapsed_ms: int) -> bool:
	var m := clampi(mode, 0, FEED_RATE_NAMES.size() - 1)
	if m == 0:
		return false
	return elapsed_ms >= FEED_MINOR_GAP_MS[m]

static func feed_max_lines(mode: int) -> int:
	return FEED_MAX_LINES[clampi(mode, 0, FEED_MAX_LINES.size() - 1)]

## Name tags over people in the world (#1428): 0 off, 1 favourites and the followed or
## selected person (the default), 2 everyone, but only with the camera at NAME_TAG_ALL_ZOOM or
## closer; further out, "everyone" tags as 1 does, so a wide view never fills with names.
const NAME_TAG_NAMES := ["off", "favourites and followed", "everyone up close"]
const NAME_TAG_ALL_ZOOM := 1.25

static func name_tag_name(mode: int) -> String:
	return NAME_TAG_NAMES[clampi(mode, 0, NAME_TAG_NAMES.size() - 1)]

## The ids of `ids` (people in view) that get a tag, in their order. `favs` holds favourite ids
## as keys. The followed and selected person are tagged whenever tags are on at all.
static func name_tag_ids(ids: Array, mode: int, zoom: float, favs: Dictionary, followed: int, selected: int) -> Array:
	var out := []
	if mode <= 0:
		return out
	var everyone := mode >= 2 and zoom >= NAME_TAG_ALL_ZOOM
	for id in ids:
		if everyone or favs.has(id) or id == followed or id == selected:
			out.append(id)
	return out

## Format a feed item with its simulation-backed because-line when present (#413 A5).
static func format_feed_line(text: String, because: String, col: String, fav: bool, sev: int) -> String:
	var head := ("★ " + text) if (fav and sev >= 2) else text
	var b := because.strip_edges()
	if b != "" and not head.contains(b) and not head.contains("because"):
		return "[color=%s]%s[/color] [color=#b8b09a]— because: %s[/color]" % [col, head, b]
	return "[color=%s]%s[/color]" % [col, head]

## Fold a feed line into the lines showing (#1484, FIXLOG 2026-11-14): a line whose fold key (feed
## row field 9, `view::fold::key`) matches one still showing raises that line's count, takes the
## new line's time, hold and place and reads "... (x14)", and moves to the newest place; a line with
## no key, or a key not showing, is appended. Seed 5 put 43 dream and suspicion lines on one night.
static func feed_fold(lines: Array, line: Dictionary) -> Array:
	var key: String = line.get("key", "")
	if key != "":
		for i in lines.size():
			var old: Dictionary = lines[i]
			if old.get("key", "") == key:
				var n: int = int(old.get("count", 1)) + 1
				var merged := line.duplicate()
				merged["count"] = n
				merged["raw"] = "%s (x%d)" % [line.get("raw", ""), n]
				merged["text"] = format_feed_line(merged["raw"], line.get("because", ""), line.get("col", "#cfcfcf"), bool(line.get("fav", false)), int(line.get("sev", 2)))
				var out := lines.duplicate()
				out.remove_at(i)
				out.append(merged)
				return out
	var out2 := lines.duplicate()
	out2.append(line)
	return out2

## Which time this is that `key` is toasted within ten game-days (288 000 ticks): records `tick` in
## `key_counts` (key -> ticks, oldest first) and returns 1 for news, 2 and 3 for "again", 4 and on
## for not toasted at all (#1487, FIXLOG 2026-11-14). Seed 6 toasted a custom formed and lapsed
## every morning for nine days as fresh news.
static func toast_repeat(key_counts: Dictionary, key: String, tick: int) -> int:
	var ticks: Array = key_counts.get(key, [])
	ticks = ticks.filter(func(t): return tick - int(t) < 288000)
	ticks.append(tick)
	key_counts[key] = ticks
	return ticks.size()

## "(again, the 2nd time in 2 days)": `first` is the first of them in the window.
static func toast_again(nth: int, first: int, tick: int) -> String:
	var days := int(float(tick - first) / 28800.0) + 1
	var ord := "%d%s" % [nth, "nd" if nth == 2 else ("rd" if nth == 3 else "th")]
	return "(again, the %s time in %d day%s)" % [ord, days, "" if days == 1 else "s"]

## The simulation-backed because-line for an inspected person (#413 A5): latest traced decision
## reason, or mood thoughts (`why`), or strongest feeling (`feeling`), or recent reaction trace.
static func inspect_because_line(d: Dictionary) -> String:
	var cycles: Array = d.get("why_cycles", [])
	if cycles.size() > 0:
		var c: Dictionary = cycles[0]
		var reasons: PackedStringArray = c.get("because", PackedStringArray())
		if reasons.size() > 0:
			return "because: " + ", ".join(reasons)
	var why_txt := String(d.get("why", "")).strip_edges()
	if why_txt != "":
		return "because: " + why_txt
	var feel_txt := String(d.get("feeling", "")).strip_edges()
	if feel_txt != "":
		return "because: feels " + feel_txt
	var trace: Array = d.get("reaction_trace", [])
	if trace.size() > 0:
		var r: Dictionary = trace[0]
		return "because: noticed %s (%s)" % [r.get("problem", ""), r.get("reaction", "")]
	return ""

## Format a favourite inhabitant's row for the Favourites panel, including their because-line (#413 A5).
static func format_favourite_row(d: Dictionary) -> String:
	var alive := bool(d.get("alive", true))
	var state_col := "#a8e0a0" if alive else "#e09090"
	var state_word := "alive" if alive else "dead"
	var line := "[b]%s[/b] ([color=#b9d3ff]%s[/color], [color=%s]%s[/color]) — %s\n[color=#cfcfcf]%s[/color]" % [
		d.get("name", ""), d.get("community", ""), state_col, state_word, d.get("where", ""), d.get("activity", "")
	]
	var why_txt := String(d.get("why", "")).strip_edges()
	var feel_txt := String(d.get("feeling", "")).strip_edges()
	if why_txt != "":
		line += " [color=#d8c8a0]— because: %s[/color]" % why_txt
	elif feel_txt != "":
		line += " [color=#d8c8a0]— because: feels %s[/color]" % feel_txt
	return line

## Format the HUD hover preview for a person, including their because-line when present (#413 A5).
## The words beside a hovered person in the world (#1429): their name and what they are doing.
static func hover_tag(d: Dictionary) -> String:
	var name := String(d.get("name", "")).strip_edges()
	# The inspect activity ends with where ("talking — at (80, 53, z2 — surface)"): the tag is
	# drawn at the person, so the place is left off.
	var act := String(d.get("activity", "")).get_slice(" — at ", 0).strip_edges()
	if name == "":
		return ""
	return name if act == "" else "%s, %s" % [name, act]

static func format_hover(d: Dictionary) -> String:
	if d.is_empty():
		return ""
	var why_txt := String(d.get("why", "")).strip_edges()
	if why_txt == "" and String(d.get("feeling", "")).strip_edges() != "":
		why_txt = "feels " + String(d["feeling"]).strip_edges()
	var tail := (" — because: " + why_txt) if why_txt != "" else ""
	return "> %s  %s%s   (click to inspect)" % [d.get("name", ""), d.get("activity", ""), tail]

## An in-world marker (#351, WP-08 8.5; the #735 design, ported onto #738's marks layer)
## stands this long; it holds full for most of its life, then fades out.
const MARKER_LIFE := 60.0

static func marker_alpha(age: float, life: float = MARKER_LIFE) -> float:
	if age < 0.0 or age >= life or life <= 0.0:
		return 0.0
	var hold := life * 0.7
	return 1.0 if age <= hold else clampf(1.0 - (age - hold) / (life - hold), 0.0, 1.0)

## The ring's radius in world pixels: a slow pulse between 14 and 22.
static func marker_radius(age: float) -> float:
	return 18.0 + 4.0 * sin(age * 3.0)

## The marker's label: the announcement cut to `n` characters at a word, with an ellipsis.
static func marker_label(text: String, n: int = 32) -> String:
	if text.length() <= n:
		return text
	var cut := text.substr(0, n)
	var sp := cut.rfind(" ")
	return (cut.substr(0, sp) if sp > n / 2 else cut).strip_edges() + "…"

## The sounds rows that are music moments, not effects (#290, WP-08 8.8): hf_sim::view::music.
static func is_music_moment(name: String) -> bool:
	return name == "mourn" or name == "welcome"

## How long a resting score waits after a welcome: the rest ends within `wait` seconds, never
## later than it would have; a sounding phrase is left alone (#290).
static func welcome_rest(left: float, wait: float, playing: bool) -> float:
	return left if playing else minf(left, wait)

## The melody stem's target level, linear: silent while a death is mourned, full otherwise (#290).
static func melody_target(mourn_left: float) -> float:
	return 0.0 if mourn_left > 0.0 else 1.0

## What the people of the valley are doing, for the music (#290 item 3): from the agents rows
## (species at +4, 0 a person; pose at +15: 6 fought just now, 4 asleep, 10 talking, 11 eating;
## 8 and 9 are the dead lying where they fell, view::bodies, and are not counted),
## {"fight": someone fought just now, "gathering": three or more talking or eating,
## "asleep": every person is asleep}. No people, nothing.
static func music_activity(a: PackedInt32Array, stride: int) -> Dictionary:
	var people := 0
	var asleep := 0
	var together := 0
	var working := 0
	var fight := false
	for i in range(0, a.size(), stride):
		var pose: int = a[i + 15]
		if a[i + 4] != 0 or pose == 8 or pose == 9:
			continue
		people += 1
		if pose == 6:
			fight = true
		elif pose == 4:
			asleep += 1
		elif pose == 10 or pose == 11:
			together += 1
		elif pose == 2:
			# At the work, not walking to it: the bridge gates pose 2 on the working
			# pose (#290, WP-08 8.8 work bustle).
			working += 1
	return {"fight": fight, "gathering": together >= 3, "asleep": people > 0 and asleep == people, "bustle": working >= 3}

## The tension layer's target from strain, raised to at least `fight_floor` while a fight is on
## (#290 item 3).
## A fire in view lifts it too, to at least `fire_floor` (#290 remainder, WP-08 8.8).
## A working valley lifts it gently to at least `work_floor` (WP-08 8.8 work bustle): a busy
## day, below a fire and never lowering high strain.
static func tension_target(strain: float, fight: bool, fight_floor: float, fire := false, fire_floor := 0.45, work := false, work_floor := 0.2) -> float:
	var t := smoothstep(0.25, 0.7, strain)
	if fight:
		t = maxf(t, fight_floor)
	if fire:
		t = maxf(t, fire_floor)
	if work:
		t = maxf(t, work_floor)
	return t

## A row of the "Waiting work" panel (#292, WP-08 8.6): what, why nobody has taken it, and how
## long it has waited ("for 3 h", "for 2 days").
static func format_stuck_row(d: Dictionary) -> String:
	var h := int(d.get("hours", 0))
	var waited := ("for %d h" % h) if h < 48 else ("for %d days" % (h / 24))
	return "%s: %s (%s)" % [String(d.get("label", "?")).capitalize(), d.get("reason", ""), waited]

## Step-by-step first-run hint (#413 A5): guides the player through inspecting a person,
## inspecting a block, and using the bottom bar/feed controls, then disappears.
static func first_run_hint(inspected_person: bool, inspected_block: bool, used_bar: bool) -> String:
	if not inspected_person:
		return "Hint: click any person to inspect who they are, why they feel as they do, and ★ Favourite or Follow them."
	if not inspected_block:
		return "Hint: click any ground, water, or rock tile to inspect its material, water depth, minerals, and history."
	if not used_bar:
		return "Hint: use the bottom bar (or Space / 1–3 / G / H / L) to pace the valley, tune the feed rate [G], or browse History [H]."
	return ""

## The People panel's title (#351): how many are here and how many away.
static func people_title(list: Array) -> String:
	var here := 0
	var away := 0
	for d in list:
		if bool(d.get("away", false)):
			away += 1
		else:
			here += 1
	var t := "People: %d in the valley" % here
	if away > 0:
		t += ", %d away" % away
	return t + "   (Inspect to look, Follow to keep them in view)"
