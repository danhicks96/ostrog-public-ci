extends Control

## The new-game wizard: who watches, the valley and how deep its past runs, the three of ten
## who came through together, and what they hold in common. Then Begin.
##
## Every choice is made before the world exists, because each shapes what is simulated: the
## deity watches the valley's past as it watches the living valley, the depth sets how many
## years of it run, the character steers generation. Nothing here invents history. The cards
## are generated from the seed and this session's reroll counters, so "roll until you like
## them" is honest: a different draw, not a lie about who they are.

signal confirmed(setup: Dictionary)

const COUNT := 10
const STARTING := 3
const PAGES := ["Who watches", "Landing site", "The peoples", "Who came through", "What they hold", "Begin"]
const RARITIES := ["common", "uncommon", "rare", "legendary"]
const ValleyPreview := preload("res://scripts/valley_preview.gd")
const Rules := preload("res://scripts/rules.gd")
## The seeds a New game rolls from: what the valley spin box can show.
const SEED_MAX := 999999

var sim = null
var seed_v: int = 1
var rerolls: Array = []
var chosen: Array = []
var deity := "steady"
var history_years := 150
## The region of the world map the valley is (-1: the one the seed would choose), and the
## world's age in years before the portal (M3).
var region_v := -1
var realm_years := 500
var world_rect: TextureRect
var world_note: Label
var world_rgb := PackedByteArray()
var world_size := 0
var world_seed := -1
const WORLD_SCALE := 1.5
var use_character := false
var character := PackedFloat32Array([0.0, 0.0, 0.0])
## topic key -> -1, 0 or 1
var held := {}
## The world's peoples (P5): species keys left out, rarity overrides (key -> rarity), and how
## many peoples the world draws.
var excluded := {}
var rarities := {}
var peoples_count := [4, 7]
var generated := "some"
var peoples_list: VBoxContainer
var page := 0

var title_l: Label
var steps_l: Label
var body: VBoxContainer
var status: Label
var back_button: Button
var next_button: Button
var cards_root: GridContainer
var card_scroll: ScrollContainer
var preview_rect: TextureRect
var preview_lines: Label
## The valley pictures made this New game (#560), by seed, region and character, so going back
## to the valley page or back to a place shows its picture at once: key -> {texture, lines}.
var preview_cache := {}
## The picture the page is waiting for ("" when none): it is made on a thread of its own and
## collected in _process.
var preview_waiting := ""
var preview_art
## The three character sliders on the valley page, so a new seed can move their thumbs.
var sliders: Array = []
var slider_dragging := false

func _ready() -> void:
	if body != null:
		return
	# The parent is not a Control, so anchors have nothing to fill: follow the window instead.
	size = get_viewport_rect().size
	get_viewport().size_changed.connect(func(): size = get_viewport_rect().size)
	# A chosen toggle (the depth of the past, a view held in common) must read as chosen.
	var t := Theme.new()
	var on := StyleBoxFlat.new()
	on.bg_color = Color(0.17, 0.25, 0.36)
	on.border_color = Color(0.55, 0.72, 0.95)
	on.set_border_width_all(1)
	on.content_margin_left = 8; on.content_margin_right = 8
	on.content_margin_top = 4; on.content_margin_bottom = 4
	t.set_stylebox("pressed", "Button", on)
	t.set_stylebox("hover_pressed", "Button", on)
	theme = t
	var bg := ColorRect.new()
	bg.set_anchors_preset(Control.PRESET_FULL_RECT)
	bg.color = Color(0.04, 0.05, 0.06, 0.97)
	add_child(bg)

	var margin := MarginContainer.new()
	margin.set_anchors_preset(Control.PRESET_FULL_RECT)
	for side in ["left", "right"]:
		margin.add_theme_constant_override("margin_" + side, 28)
	for side in ["top", "bottom"]:
		margin.add_theme_constant_override("margin_" + side, 22)
	add_child(margin)
	var column := VBoxContainer.new()
	column.add_theme_constant_override("separation", 10)
	margin.add_child(column)

	title_l = Label.new()
	title_l.add_theme_font_size_override("font_size", 26)
	column.add_child(title_l)
	steps_l = Label.new()
	steps_l.add_theme_color_override("font_color", Color(0.55, 0.6, 0.68))
	column.add_child(steps_l)

	var scroll := ScrollContainer.new()
	scroll.size_flags_vertical = Control.SIZE_EXPAND_FILL
	scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	column.add_child(scroll)
	body = VBoxContainer.new()
	body.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	body.add_theme_constant_override("separation", 10)
	scroll.add_child(body)

	var actions := HBoxContainer.new()
	column.add_child(actions)
	back_button = Button.new()
	back_button.text = "Back"
	back_button.pressed.connect(func(): goto(page - 1))
	actions.add_child(back_button)
	next_button = Button.new()
	next_button.pressed.connect(_next)
	actions.add_child(next_button)
	status = Label.new()
	status.add_theme_color_override("font_color", Color(0.75, 0.79, 0.85))
	actions.add_child(status)

## A New game. Without a seed (p_seed < 0) the valley is a fresh random one each time (#559);
## going back and forth through the pages keeps it, and the place chosen on the world map.
func start(p_sim, p_seed: int = -1) -> void:
	sim = p_sim
	# FIXLOG 2026-11-02 (#559): a New game rolls its own valley unless one was asked for.
	seed_v = p_seed if p_seed >= 0 else roll_seed()
	region_v = -1
	world_seed = -1
	preview_cache = {}
	preview_waiting = ""
	if preview_art == null:
		preview_art = ValleyPreview.new()
	rerolls.resize(COUNT)
	rerolls.fill(0)
	chosen = [0, 1, 2]
	for t in sim.ideology_topics():
		held[String(t["key"])] = 0
	_ready()
	goto(0)

## A fresh random seed for a New game (#559).
static func roll_seed() -> int:
	var rng := RandomNumberGenerator.new()
	rng.randomize()
	return rng.randi_range(0, SEED_MAX)

func goto(p: int) -> void:
	page = clampi(p, 0, PAGES.size() - 1)
	for c in body.get_children():
		c.queue_free()
	cards_root = null
	preview_rect = null
	sliders = []
	title_l.text = "Ostrog: %s" % PAGES[page]
	var marks := []
	for k in PAGES.size():
		marks.append(("[%s]" if k == page else "%s") % PAGES[k])
	steps_l.text = "  ·  ".join(marks)
	back_button.disabled = page == 0
	next_button.text = "Begin" if page == PAGES.size() - 1 else "Next"
	status.text = ""
	match page:
		0: _deity_page()
		1: _valley_page()
		2: _peoples_page()
		3: _party_page()
		4: _held_page()
		5: _summary_page()
	_validate()

func _next() -> void:
	if page == PAGES.size() - 1:
		_begin()
	else:
		goto(page + 1)

func _validate() -> void:
	var ok := true
	status.remove_theme_color_override("font_color")
	status.add_theme_color_override("font_color", Color(0.75, 0.79, 0.85))
	if page == 3:
		ok = chosen.size() == STARTING
		status.text = "%d chosen; three came through here." % chosen.size()
	# A world nobody could live in cannot begin (#147, set AF step 3): say so before Begin.
	if page == PAGES.size() - 1 and sim.peoples_preview(seed_v, _peoples_setup()).is_empty():
		ok = false
		status.text = "Nobody could live in that world: go back to the peoples and leave in a kind that lives above ground."
		status.add_theme_color_override("font_color", Color(0.95, 0.6, 0.5))
	next_button.disabled = not ok

func _para(text: String, colour := Color(0.82, 0.85, 0.9), size := 14) -> Label:
	var l := Label.new()
	l.text = text
	l.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	l.custom_minimum_size = Vector2(560, 0)
	l.add_theme_font_size_override("font_size", size)
	l.add_theme_color_override("font_color", colour)
	return l

func _panel(selected: bool) -> PanelContainer:
	var p := PanelContainer.new()
	var bg := StyleBoxFlat.new()
	bg.bg_color = Color(0.10, 0.12, 0.15, 0.95) if selected else Color(0.06, 0.07, 0.09, 0.9)
	bg.border_color = Color(0.55, 0.72, 0.95, 0.9) if selected else Color(0.2, 0.22, 0.26, 0.8)
	bg.set_border_width_all(2 if selected else 1)
	bg.content_margin_left = 12; bg.content_margin_right = 12
	bg.content_margin_top = 10; bg.content_margin_bottom = 10
	p.add_theme_stylebox_override("panel", bg)
	return p

# --- Who watches ------------------------------------------------------------------------------

func _deity_page() -> void:
	body.add_child(_para("Ten came through the portal onto land that was lived on long before " \
		+ "them. Something watches it, and has watched it all along. It sends nothing; it leans " \
		+ "on what the world already does. Choose who."))
	var row := HFlowContainer.new()
	row.add_theme_constant_override("h_separation", 12)
	row.add_theme_constant_override("v_separation", 12)
	body.add_child(row)
	for d in sim.deities():
		var key := String(d["key"])
		var card := _panel(key == deity)
		card.custom_minimum_size = Vector2(300, 170)
		var col := VBoxContainer.new()
		card.add_child(col)
		var n := Label.new()
		n.text = "%s, %s" % [d["name"], d["title"]]
		n.add_theme_font_size_override("font_size", 18)
		col.add_child(n)
		var a := _para(String(d["about"]), Color(0.78, 0.8, 0.84), 13)
		a.custom_minimum_size = Vector2(276, 0)
		col.add_child(a)
		var pick := Button.new()
		pick.text = "Watching" if key == deity else "Choose"
		pick.disabled = key == deity
		pick.pressed.connect(func(): deity = key; goto(0))
		col.add_child(pick)
		row.add_child(card)
	body.add_child(_para("The one you choose also watched the history of this place, so the people who " \
		+ "live beyond it may already have noticed what it does.", Color(0.6, 0.65, 0.72), 12))

# --- The valley -------------------------------------------------------------------------------

func _valley_page() -> void:
	var top := HBoxContainer.new()
	top.add_theme_constant_override("separation", 18)
	body.add_child(top)
	var left := VBoxContainer.new()
	left.custom_minimum_size = Vector2(300, 0)
	top.add_child(left)

	var seed_row := HBoxContainer.new()
	left.add_child(seed_row)
	var sl := Label.new()
	sl.text = "World seed"
	seed_row.add_child(sl)
	var spin := SpinBox.new()
	spin.min_value = 0
	spin.max_value = SEED_MAX
	spin.value = seed_v
	spin.value_changed.connect(func(v): _set_seed(int(v)))
	seed_row.add_child(spin)
	var another := Button.new()
	another.text = "Generate another world"
	another.pressed.connect(func(): _set_seed((seed_v * 7919 + 104729) % 1000000); goto(1))
	seed_row.add_child(another)

	var own := CheckBox.new()
	own.text = "Shape the landing site myself"
	own.button_pressed = use_character
	own.toggled.connect(func(on): _shape_myself(on))
	left.add_child(own)
	var axes := [["Dry", "Wet"], ["Cold", "Mild"], ["Soft", "Stony"]]
	var seed_char := seed_character()
	for k in 3:
		var r := HBoxContainer.new()
		left.add_child(r)
		var lo := Label.new()
		lo.text = axes[k][0]
		lo.custom_minimum_size = Vector2(44, 0)
		r.add_child(lo)
		var s := HSlider.new()
		s.min_value = -1.0
		s.max_value = 1.0
		s.step = 0.1
		s.value = character[k] if use_character else seed_char[k]
		s.custom_minimum_size = Vector2(170, 0)
		s.editable = use_character
		# value_changed covers keys, wheel and clicks as well as drags; while the mouse is
		# dragging, the preview waits for the release instead of generating a world per pixel.
		s.value_changed.connect(func(v):
			character[k] = v
			if not slider_dragging:
				_refresh_preview())
		s.drag_started.connect(func(): slider_dragging = true)
		s.drag_ended.connect(func(_changed): slider_dragging = false; _refresh_preview())
		sliders.append(s)
		r.add_child(s)
		var hi := Label.new()
		hi.text = axes[k][1]
		r.add_child(hi)

	left.add_child(_para("Years of local history", Color(0.9, 0.9, 0.92), 14))
	var depth_row := HBoxContainer.new()
	left.add_child(depth_row)
	var group := ButtonGroup.new()
	for y in sim.history_choices():
		var b := Button.new()
		b.text = "none" if y == 0 else "%d years" % y
		b.toggle_mode = true
		b.button_group = group
		b.button_pressed = int(y) == history_years
		b.pressed.connect(func(): history_years = int(y); goto(1))
		depth_row.add_child(b)
	var depth_note := {
		0: "No one lived here before. No ruins, no graves, no tales.",
		50: "A generation or two of hamlets; their ruins are still standing.",
		150: "Several lifetimes: hamlets founded, burned, emptied and settled again.",
		300: "Centuries: old foundations under newer ones, stone ruins, long-dead names.",
	}
	left.add_child(_para(String(depth_note.get(history_years, "")), Color(0.72, 0.76, 0.82), 12))

	left.add_child(_para("How long the world has stood", Color(0.9, 0.9, 0.92), 14))
	var age_row := HBoxContainer.new()
	left.add_child(age_row)
	var age_group := ButtonGroup.new()
	for y in sim.realm_year_choices():
		var b := Button.new()
		b.text = "%d years" % y
		b.toggle_mode = true
		b.button_group = age_group
		b.button_pressed = int(y) == realm_years
		b.pressed.connect(func(): realm_years = int(y); goto(1))
		age_row.add_child(b)
	left.add_child(_para("Choose a landing site on the world map. The ring marks your selection.", Color(0.9, 0.9, 0.92), 14))
	world_rect = TextureRect.new()
	world_rect.texture_filter = CanvasItem.TEXTURE_FILTER_NEAREST
	world_rect.stretch_mode = TextureRect.STRETCH_SCALE
	world_rect.mouse_filter = Control.MOUSE_FILTER_STOP
	# Its own size, not the column's: a click maps to a region by the fixed scale.
	world_rect.size_flags_horizontal = Control.SIZE_SHRINK_BEGIN
	world_rect.gui_input.connect(_on_world_click)
	left.add_child(world_rect)
	world_note = _para("", Color(0.72, 0.76, 0.82), 12)
	world_note.custom_minimum_size = Vector2(288, 0)
	left.add_child(world_note)
	_draw_world()

	var right := VBoxContainer.new()
	top.add_child(right)
	preview_rect = TextureRect.new()
	preview_rect.custom_minimum_size = Vector2(384, 384)
	preview_rect.texture_filter = CanvasItem.TEXTURE_FILTER_NEAREST
	preview_rect.stretch_mode = TextureRect.STRETCH_KEEP_ASPECT
	preview_rect.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
	right.add_child(preview_rect)
	preview_lines = _para("", Color(0.75, 0.79, 0.85), 12)
	preview_lines.custom_minimum_size = Vector2(384, 0)
	right.add_child(preview_lines)
	_refresh_preview()

## The world map with the valley's region marked.
func _draw_world() -> void:
	if world_rect == null:
		return
	if world_seed != seed_v:
		var wp: Dictionary = sim.world_preview(seed_v)
		world_rgb = wp["rgb"]
		world_size = int(wp["size"])
		world_seed = seed_v
		if region_v < 0:
			region_v = int(wp["region"])
	if world_size <= 0:
		return
	var img := Image.create_from_data(world_size, world_size, false, Image.FORMAT_RGB8, world_rgb)
	mark_place(img, region_v % world_size, region_v / world_size)
	world_rect.texture = ImageTexture.create_from_image(img)
	world_rect.custom_minimum_size = Vector2(world_size, world_size) * WORLD_SCALE
	var info: Dictionary = sim.region_info(seed_v, region_v)
	if info.has("lines"):
		world_note.text = "\n".join(info["lines"])

## The chosen place on the world map: a white centre in a dark ring with a thin white edge, so
## one part of it stands out on every ground, from ice to ocean. Sized to the map (a 192-region
## map gets a mark 11 regions across, about 16 pixels at WORLD_SCALE). The centre may be
## coloured (the Legends map's marks for those away, #562).
## FIXLOG 2026-11-02 (#564): it was a 3x3 white square, lost on snow and ice.
static func mark_place(img: Image, x: int, y: int, centre := Color(1, 1, 1)) -> void:
	var n := img.get_width()
	var inner := maxi(1, n / 96)
	var ring := inner + maxi(2, n / 96)
	var edge := ring + 1
	for dy in range(-edge, edge + 1):
		for dx in range(-edge, edge + 1):
			var px := x + dx
			var py := y + dy
			if px < 0 or py < 0 or px >= n or py >= img.get_height():
				continue
			var d := sqrt(float(dx * dx + dy * dy))
			if d <= inner + 0.5:
				img.set_pixel(px, py, centre)
			elif d <= ring + 0.5:
				img.set_pixel(px, py, Color(0.05, 0.05, 0.07))
			elif d <= edge + 0.5:
				img.set_pixel(px, py, Color(1, 1, 1))

func _on_world_click(e: InputEvent) -> void:
	if not (e is InputEventMouseButton and e.pressed and e.button_index == MOUSE_BUTTON_LEFT):
		return
	if world_size <= 0:
		return
	var x := clampi(int(e.position.x / WORLD_SCALE), 0, world_size - 1)
	var y := clampi(int(e.position.y / WORLD_SCALE), 0, world_size - 1)
	choose_region(y * world_size + x)

## Put the valley at region r of the world map, if people could live there; else say why and
## keep the place chosen before. True when it moved.
func choose_region(r: int) -> bool:
	var info: Dictionary = sim.region_info(seed_v, r)
	if not bool(info.get("habitable", false)):
		if world_note != null:
			world_note.text = "\n".join(info.get("lines", PackedStringArray())) + "\nChoose somewhere people could live."
		return false
	region_v = r
	_draw_world()
	_refresh_preview()
	return true

func _shape_myself(on: bool) -> void:
	use_character = on
	if on:
		# Start from the valley the seed made, so the sliders move it rather than replace it.
		character = seed_character()
	goto(1)

## The character the seed would give, for the sliders when the player has not shaped it: the
## chosen region's climate, which needs no valley generated.
func seed_character() -> PackedFloat32Array:
	return sim.valley_character(seed_v, PackedFloat32Array(), region_v)["character"]

func _chosen_character() -> PackedFloat32Array:
	return character if use_character else PackedFloat32Array()

func _set_seed(s: int) -> void:
	if s == seed_v:
		return
	seed_v = s
	# A new valley means new people came through it, in a new world.
	rerolls.fill(0)
	region_v = -1
	_draw_world()
	_refresh_preview()

func _preview_key() -> String:
	return "%d|%d|%s" % [seed_v, region_v, str(_chosen_character()) if use_character else "-"]

## FIXLOG 2026-11-02 (#560): the picture is the generated valley, made off the main thread.
## Show the picture of the valley as now chosen: at once if it was made earlier in this New
## game, else ask for it and show it when _process collects it.
func _refresh_preview() -> void:
	if preview_rect == null:
		return
	if not use_character:
		# The seed's own character: keep the (locked) thumbs on it when the seed changes.
		var seed_char := seed_character()
		for k in sliders.size():
			sliders[k].set_value_no_signal(seed_char[k])
	var key := _preview_key()
	if preview_cache.has(key):
		preview_waiting = ""
		_show_preview(preview_cache[key])
		return
	preview_waiting = key
	if sim.valley_preview_begin(seed_v, _chosen_character(), region_v):
		_collect_preview()
		return
	preview_lines.text = "Drawing the landing site..."
	set_process(true)

func _process(_d: float) -> void:
	if preview_waiting == "":
		set_process(false)
		return
	_collect_preview()

## Whether the page still waits for its picture (the wizard shot waits on it).
func preview_pending() -> bool:
	return preview_waiting != ""

func _collect_preview() -> void:
	var p: Dictionary = sim.valley_preview_take(seed_v, _chosen_character(), region_v)
	if p.is_empty():
		return
	var entry := {
		"texture": ImageTexture.create_from_image(preview_art.image(p)),
		# In a player's words, not the describe command's (Rules.valley_note, FIXLOG 2026-11-14).
		"lines": Rules.valley_note(Array(p["lines"]), str(p.get("label", "")), p.get("surface_plants", PackedStringArray())) + "\nThe red marker shows the egress point where the founders emerge.",
	}
	preview_cache[preview_waiting] = entry
	preview_waiting = ""
	set_process(false)
	_show_preview(entry)

func _show_preview(entry: Dictionary) -> void:
	if preview_rect == null:
		return
	preview_rect.texture = entry["texture"]
	preview_lines.text = entry["lines"]

# --- Who came through -------------------------------------------------------------------------

func _party_page() -> void:
	body.add_child(_para("Ten came through. Only three of you are here together; the rest were " \
		+ "thrown elsewhere, alive or dead, nobody here knows which. Choose the three who begin."))
	var reroll_all := Button.new()
	reroll_all.text = "Reroll all ten"
	reroll_all.size_flags_horizontal = Control.SIZE_SHRINK_BEGIN
	reroll_all.pressed.connect(func():
		for i in COUNT:
			rerolls[i] = int(rerolls[i]) + 1
		goto(3))
	body.add_child(reroll_all)
	cards_root = GridContainer.new()
	var view_size := get_viewport_rect().size
	cards_root.columns = clampi(int(maxf(view_size.x - 56.0, 320.0) / 220.0), 2, 5)
	cards_root.add_theme_constant_override("h_separation", 10)
	cards_root.add_theme_constant_override("v_separation", 10)
	body.add_child(cards_root)
	for card in sim.survivor_cards(seed_v, PackedInt64Array(rerolls)):
		cards_root.add_child(_card(card))

func _toggle(i: int) -> void:
	if chosen.has(i):
		chosen.erase(i)
	elif chosen.size() < STARTING:
		chosen.append(i)
	else:
		goto(3)
		status.text = "Only three came through here. Let one of them go first."
		return
	goto(3)

func _card(card: Dictionary) -> Control:
	var i: int = int(card["index"])
	var root := _panel(chosen.has(i))
	root.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	root.custom_minimum_size = Vector2(160, 130)
	var col := VBoxContainer.new()
	root.add_child(col)
	var name_l := Label.new()
	name_l.text = "%s" % card["name"]
	name_l.add_theme_font_size_override("font_size", 14)
	col.add_child(name_l)
	var age_l := Label.new()
	age_l.text = "%d years · %s" % [int(card["age"]), card["traits"]]
	age_l.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	age_l.add_theme_font_size_override("font_size", 12)
	age_l.custom_minimum_size = Vector2(140, 26)
	age_l.add_theme_color_override("font_color", Color(0.72, 0.76, 0.82))
	col.add_child(age_l)
	for s in card["skills"]:
		var sk := Label.new()
		sk.text = "  %s" % s
		sk.add_theme_font_size_override("font_size", 12)
		sk.add_theme_color_override("font_color", Color(0.66, 0.7, 0.76))
		col.add_child(sk)
	# Who they were: a few lines of the life they lived before the portal.
	for line in card.get("past", []):
		var pl := Label.new()
		pl.text = line
		pl.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		pl.custom_minimum_size = Vector2(140, 0)
		pl.add_theme_font_size_override("font_size", 11)
		pl.add_theme_color_override("font_color", Color(0.85, 0.78, 0.65))
		col.add_child(pl)
	var row := HBoxContainer.new()
	col.add_child(row)
	var pick := Button.new()
	pick.text = "Here" if chosen.has(i) else "Choose"
	pick.pressed.connect(func(): _toggle(i))
	row.add_child(pick)
	var roll := Button.new()
	roll.text = "Reroll"
	roll.pressed.connect(func(): rerolls[i] = int(rerolls[i]) + 1; goto(3))
	row.add_child(roll)
	return root

# --- What they hold in common -----------------------------------------------------------------

func _held_page() -> void:
	body.add_child(_para("Each of them arrives with beliefs their own life taught them. If the " \
		+ "three hold something in common, it joins what each one already believes; it does not " \
		+ "replace it, and hard experience can still turn them. Leave a row at \"each their own\" " \
		+ "to let their lives decide."))
	var grid := GridContainer.new()
	grid.columns = 4
	grid.add_theme_constant_override("h_separation", 8)
	grid.add_theme_constant_override("v_separation", 6)
	body.add_child(grid)
	for t in sim.ideology_topics():
		var key := String(t["key"])
		var name_l := Label.new()
		name_l.text = key.capitalize()
		name_l.custom_minimum_size = Vector2(90, 0)
		grid.add_child(name_l)
		var group := ButtonGroup.new()
		for opt in [[-1, String(t["against"])], [0, "each their own"], [1, String(t["for"])]]:
			var b := Button.new()
			b.text = opt[1]
			b.toggle_mode = true
			b.button_group = group
			b.button_pressed = int(held[key]) == int(opt[0])
			var v: int = int(opt[0])
			b.pressed.connect(func(): held[key] = v)
			grid.add_child(b)

# --- The peoples ------------------------------------------------------------------------------

func _peoples_page() -> void:
	body.add_child(_para("The world beyond the landing site is lived in by peoples of several kinds. " \
		+ "Choose which kinds there can be and how rare each is. Rarity sets how a kind starts, not " \
		+ "how it ends: a rare people has no land of its own and lives as households among others; " \
		+ "a legendary one's land fails it over the centuries. The world's history decides the rest."))
	var row := HBoxContainer.new()
	row.add_child(_para("Peoples in the world:", Color(0.82, 0.85, 0.9), 14))
	for k in 2:
		var sb := SpinBox.new()
		sb.min_value = 1
		sb.max_value = 16
		sb.value = peoples_count[k]
		sb.value_changed.connect(func(v): peoples_count[k] = int(v); _peoples_changed())
		row.add_child(sb)
		if k == 0:
			var to := Label.new()
			to.text = "to"
			row.add_child(to)
	var gen := OptionButton.new()
	for g in ["off", "some", "many"]:
		gen.add_item("kinds of its own: %s" % g)
	gen.selected = ["off", "some", "many"].find(generated)
	gen.item_selected.connect(func(i): generated = ["off", "some", "many"][i]; _peoples_changed())
	row.add_child(gen)
	body.add_child(row)
	# What the seed would draw comes first, where it is seen without scrolling past twelve
	# kinds; at 720p it was cut off below the list (#199, QA 2026-10-20, C63).
	body.add_child(_para("What this seed would draw:", Color(0.9, 0.9, 0.92), 15))
	peoples_list = VBoxContainer.new()
	body.add_child(peoples_list)
	body.add_child(_para("The kinds there can be:", Color(0.9, 0.9, 0.92), 15))
	# One row per kind, each as tall as its description wraps to: the description takes the
	# row's remaining width and wraps in it, so it cannot run into the next kind's row (#199).
	var grid := VBoxContainer.new()
	grid.add_theme_constant_override("separation", 6)
	body.add_child(grid)
	for sp in sim.species_list():
		var line := HBoxContainer.new()
		line.add_theme_constant_override("separation", 10)
		grid.add_child(line)
		var key := String(sp["key"])
		var cb := CheckBox.new()
		cb.text = String(sp["plural"])
		cb.button_pressed = not excluded.get(key, false)
		cb.toggled.connect(func(on): excluded[key] = not on; _peoples_changed())
		cb.custom_minimum_size = Vector2(150, 0)
		cb.size_flags_vertical = Control.SIZE_SHRINK_BEGIN
		line.add_child(cb)
		var ob := OptionButton.new()
		for r in RARITIES:
			ob.add_item(r)
		ob.selected = RARITIES.find(String(rarities.get(key, sp["rarity"])))
		ob.item_selected.connect(func(i): rarities[key] = RARITIES[i]; _peoples_changed())
		ob.size_flags_vertical = Control.SIZE_SHRINK_BEGIN
		line.add_child(ob)
		var d := _para(String(sp["describe"]), Color(0.70, 0.74, 0.80), 13)
		d.custom_minimum_size = Vector2(360, 0)
		d.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		line.add_child(d)
	_peoples_changed()

func _peoples_setup() -> Dictionary:
	var ex := PackedStringArray()
	for k in excluded:
		if excluded[k]:
			ex.append(k)
	var lo := mini(int(peoples_count[0]), int(peoples_count[1]))
	return {"exclude": ex, "count": PackedInt64Array([lo, maxi(lo, int(peoples_count[1]))]), "rarity": rarities.duplicate(), "generated": generated}

func _peoples_changed() -> void:
	if peoples_list == null:
		return
	for c in peoples_list.get_children():
		c.queue_free()
	var drawn: Array = sim.peoples_preview(seed_v, _peoples_setup())
	if drawn.is_empty():
		peoples_list.add_child(_para("Nobody could live in that world: leave in at least one kind that lives above ground.", Color(0.95, 0.6, 0.5)))
		return
	for p in drawn:
		var where := " (no land of its own)" if String(p["rarity"]) == "rare" else (" (its land failing)" if String(p["rarity"]) == "legendary" else "")
		peoples_list.add_child(_para("The %s, %s %s%s" % [p["name"], p["species"], p["living"], where], Color(0.82, 0.85, 0.9), 13))

# --- Begin ------------------------------------------------------------------------------------

func _summary_page() -> void:
	var d_name := deity
	for d in sim.deities():
		if String(d["key"]) == deity:
			d_name = "%s, %s" % [d["name"], d["title"]]
	var names := []
	var cards: Array = sim.survivor_cards(seed_v, PackedInt64Array(rerolls))
	for card in cards:
		if chosen.has(int(card["index"])):
			names.append(String(card["name"]))
	var common := []
	for t in sim.ideology_topics():
		var v: int = int(held[String(t["key"])])
		if v != 0:
			common.append(String(t["for"] if v > 0 else t["against"]))
	var p: Dictionary = sim.valley_character(seed_v, _chosen_character(), region_v)
	body.add_child(_para("Watched by %s." % d_name, Color(0.9, 0.9, 0.92), 16))
	body.add_child(_para("World seed %d. The founders arrive at %s." % [seed_v, p["label"]]))
	body.add_child(_para("%s of local history, in a world that has stood %d years." % ["No years" if history_years == 0 else "%d years" % history_years, realm_years]))
	body.add_child(_para("%s come through together." % ", ".join(names)))
	body.add_child(_para("They hold in common: %s." % ("nothing beyond what each has lived" if common.is_empty() else "; ".join(common))))
	body.add_child(_para("Generating local history takes a moment.", Color(0.6, 0.65, 0.72), 12))

func setup() -> Dictionary:
	var ordered := chosen.duplicate()
	ordered.sort()
	var ideology := {}
	for k in held:
		if int(held[k]) != 0:
			ideology[k] = float(held[k]) * 0.8
	return {
		"seed": seed_v,
		"deity": deity,
		"history_years": history_years,
		"region": region_v,
		"realm_years": realm_years,
		"character": _chosen_character(),
		"chosen": PackedInt64Array(ordered),
		"rerolls": PackedInt64Array(rerolls),
		"ideology": ideology,
		"peoples": _peoples_setup(),
	}

func _begin() -> void:
	confirmed.emit(setup())

## The world could not be made as chosen (#147): show why, on the page it concerns, and stay.
func refused(err: String) -> void:
	if err.begins_with("peoples"):
		goto(2)
	elif err.begins_with("no deity"):
		goto(0)
	status.text = "Could not begin: %s." % err.trim_suffix(".")
	status.add_theme_color_override("font_color", Color(0.95, 0.6, 0.5))
