extends Control
## The world's history before the portal, shown at the start of a new game (#561), as Dwarf
## Fortress shows its world being made: the world map year by year, settlements founded,
## ruined and taken, with the year's notable events beside it, then a click to come in. It
## replays what the simulation already worked out when the world was made (the Legends map of
## each year and the chronicle's rows before the arrival); it computes nothing and changes
## nothing. Any key or click hurries it to the end; at the end a click comes into the world.

signal finished

## The reel's length: about a fortieth of a second a year, never under 6 or over 20 seconds.
const SECONDS_PER_YEAR := 0.025
const MIN_SECONDS := 6.0
const MAX_SECONDS := 20.0
## How many of the latest lines the feed keeps.
const FEED := 14
## The map is drawn this many pixels a region, and each settlement as a block a little wider
## than its region, so one settlement can be seen at all.
const SCALE := 3
const DOT := 5
## How the Legends map draws a ruin (view::legends map_rgb).
const RUIN := Color8(20, 18, 24)

var sim = null
var first := 0
var last := 0
var size_v := 0
## realm year -> PackedStringArray of that year's lines.
var lines_by_year := {}
var elapsed := 0.0
var length := MIN_SECONDS
var shown_year := -1
var done := false

var map_rect: TextureRect
var year_l: Label
var feed_l: RichTextLabel
var hint_l: Label
var feed: Array = []
## The country with nobody in it (the map before the first year), as bytes and drawn large.
var base := PackedByteArray()
var base_big: Image
## Settlements living and in ruins in the year shown.
var living := 0
var ruins := 0

## How long a history of `years` years plays.
static func length_for(years: int) -> float:
	return clampf(years * SECONDS_PER_YEAR, MIN_SECONDS, MAX_SECONDS)

## The realm year shown `t` seconds into a reel of `length` seconds over first..last.
static func year_at(t: float, length: float, first_year: int, last_year: int) -> int:
	if length <= 0.0 or t >= length:
		return last_year
	return first_year + int(floor((last_year - first_year) * clampf(t / length, 0.0, 1.0)))

## How a realm year reads, counted back from the portal (`last` is the year before it).
static func when(year: int, last_year: int) -> String:
	var back := last_year - year + 1
	return "the year before the portal" if back == 1 else "%d years before the portal" % back

## Start on the world the sim holds. False when it has no world beyond the valley, and then
## there is nothing to show.
func start(p_sim) -> bool:
	sim = p_sim
	size_v = int(sim.legends_map_size())
	last = int(sim.realm_year())
	if size_v <= 0 or last <= 0:
		return false
	first = 0
	for row in sim.history_reel():
		lines_by_year[int(row[0])] = row[1]
		# The valley's own past can run deeper than the world's (300 years of it in a world of
		# 200): start where the record starts.
		first = mini(first, int(row[0]))
	base = sim.legends_map(first - 1)
	base_big = Image.create_from_data(size_v, size_v, false, Image.FORMAT_RGB8, base)
	base_big.resize(size_v * SCALE, size_v * SCALE, Image.INTERPOLATE_NEAREST)
	length = length_for(last - first)
	_build()
	_show(first)
	return true

func _build() -> void:
	# The parent is not a Control, so anchors have nothing to fill: follow the window instead.
	if is_inside_tree():
		size = get_viewport_rect().size
		get_viewport().size_changed.connect(func(): size = get_viewport_rect().size)
	mouse_filter = Control.MOUSE_FILTER_STOP
	var bg := ColorRect.new()
	bg.set_anchors_preset(Control.PRESET_FULL_RECT)
	bg.color = Color(0.04, 0.05, 0.06, 1.0)
	add_child(bg)
	var margin := MarginContainer.new()
	margin.set_anchors_preset(Control.PRESET_FULL_RECT)
	for side in ["left", "right", "top", "bottom"]:
		margin.add_theme_constant_override("margin_" + side, 28)
	add_child(margin)
	var column := VBoxContainer.new()
	column.add_theme_constant_override("separation", 10)
	margin.add_child(column)
	var title := Label.new()
	title.text = "The world before the portal"
	title.add_theme_font_size_override("font_size", 26)
	column.add_child(title)
	year_l = Label.new()
	year_l.add_theme_font_size_override("font_size", 18)
	year_l.add_theme_color_override("font_color", Color(0.85, 0.82, 0.7))
	column.add_child(year_l)
	var row := HBoxContainer.new()
	row.add_theme_constant_override("separation", 20)
	row.size_flags_vertical = Control.SIZE_EXPAND_FILL
	column.add_child(row)
	map_rect = TextureRect.new()
	map_rect.custom_minimum_size = Vector2(500, 500)
	map_rect.texture_filter = CanvasItem.TEXTURE_FILTER_NEAREST
	map_rect.stretch_mode = TextureRect.STRETCH_KEEP_ASPECT
	map_rect.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
	row.add_child(map_rect)
	feed_l = RichTextLabel.new()
	feed_l.bbcode_enabled = true
	feed_l.scroll_active = false
	feed_l.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	feed_l.add_theme_font_size_override("normal_font_size", 14)
	feed_l.add_theme_color_override("default_color", Color(0.78, 0.81, 0.86))
	row.add_child(feed_l)
	hint_l = Label.new()
	hint_l.add_theme_color_override("font_color", Color(0.6, 0.64, 0.7))
	column.add_child(hint_l)
	hint_l.text = "Settlements in their peoples' colours, ruins dark, the landing site in white. Click or press a key to hurry."

func _process(d: float) -> void:
	if done or sim == null:
		return
	elapsed += d
	var y := year_at(elapsed, length, first, last)
	if y != shown_year:
		_show(y)
	if y >= last:
		_end()

## Show the map at `year`, with every year's lines from the last shown up to it.
func _show(year: int) -> void:
	map_rect.texture = ImageTexture.create_from_image(picture(sim.legends_map(year)))
	for y in range(shown_year + 1, year + 1):
		for l in lines_by_year.get(y, PackedStringArray()):
			feed.append("[color=#8c8c8c]%d[/color]  %s" % [last - y + 1, l])
	while feed.size() > FEED:
		feed.pop_front()
	feed_l.text = "\n".join(feed)
	shown_year = year
	year_l.text = "The world %s: %d settlements, %d in ruins" % [when(year, last), living, ruins]

## The map of a year drawn large: the country, and every settlement of that year (what differs
## from the empty country) as a block in its colour; counts the living and the ruined.
func picture(bytes: PackedByteArray) -> Image:
	var img: Image = base_big.duplicate()
	living = 0
	ruins = 0
	var half := DOT / 2
	for i in size_v * size_v:
		var k := i * 3
		if bytes[k] == base[k] and bytes[k + 1] == base[k + 1] and bytes[k + 2] == base[k + 2]:
			continue
		var c := Color8(bytes[k], bytes[k + 1], bytes[k + 2])
		if c == RUIN:
			ruins += 1
		else:
			living += 1
		var x := (i % size_v) * SCALE + SCALE / 2 - half
		var y := (i / size_v) * SCALE + SCALE / 2 - half
		img.fill_rect(Rect2i(x, y, DOT, DOT).intersection(Rect2i(0, 0, size_v * SCALE, size_v * SCALE)), c)
	return img

func _end() -> void:
	if done:
		return
	_show(last)
	done = true
	year_l.text = "The world as it stands in the year before the portal: %d settlements, %d in ruins" % [living, ruins]
	hint_l.text = "Click to come into the world."

func _gui_input(e: InputEvent) -> void:
	if e is InputEventMouseButton and e.pressed:
		accept_event()
		_hurry_or_enter()

func _unhandled_input(e: InputEvent) -> void:
	if e is InputEventKey and e.pressed and not e.echo:
		get_viewport().set_input_as_handled()
		_hurry_or_enter()

## The first press hurries the reel to its end; a press at the end comes into the world.
func _hurry_or_enter() -> void:
	if not done:
		_end()
	else:
		finished.emit()
