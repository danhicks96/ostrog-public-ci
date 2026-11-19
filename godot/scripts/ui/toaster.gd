## Transient messages that can be noticed without watching the feed.
##
## The feed already ranks every line it receives - `_push_feed` gets a severity per row,
## where 1 and below is a minor line, 2 is notable and 3 is major - and it throttles the
## minor ones so the feed stays readable at speed. This shows the top rank only, as a
## short-lived line that fades, so something can be noticed without reading at all.
##
## Which lines deserve a toast is the viewer's choice (#329, Rules.announce, the N key). A
## toast of the same kind as one still showing joins it with a count, so a storm of one thing
## does not push out the one that matters; a click on a toast looks at where it happened.
extends CanvasLayer
class_name Toaster

## How long a toast stays at full strength.
const LIFE := 6.0
## How long it takes to fade out after that.
const FADE := 1.2
## How many may be on screen at once. Older ones go when this is exceeded.
const MAX_ON_SCREEN := 4
const WIDTH := 380.0

## How long a toast of one kind gathers others of its kind (#329).
const BATCH := 8.0

signal looked(pos: Vector3i)

var _box: VBoxContainer
var _items: Array = []
## While a reading panel is open the toasts wait (D91, FIXLOG 2026-11-14): hidden, not ageing,
## still gathering by kind, and shown when it closes. Over History or People they covered the
## text being read; behind a panel they would be lost.
var held := false


func _ready() -> void:
	# Above the panel layer, which is 2.
	layer = 3
	_box = VBoxContainer.new()
	_box.set_anchors_preset(Control.PRESET_CENTER_TOP); _box.offset_left = -WIDTH * 0.5; _box.offset_right = WIDTH * 0.5
	_box.add_theme_constant_override("separation", 6)
	# Toasts are an announcement, not a control: clicks must fall through to the world.
	_box.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_box.offset_top = 34
	add_child(_box)


## Where the column starts: main.gd keeps it just under the HUD's real bottom, which grows a line
## for strangers and troubles; a fixed 34 px put toasts over its hint line (FIXLOG 2026-11-14).
func set_top(y: float) -> void:
	if _box != null:
		_box.offset_top = y


func hold(on: bool) -> void:
	held = on
	if _box != null:
		_box.visible = not on


## Show `text` for a while. `colour` is a hex string, matching the feed's own palette.
func toast(text: String, colour: String = "#ffe9a8", kind: String = "", pos := Vector3i(-1, -1, 0)) -> void:
	if text == "":
		return
	# The same kind again while one is fresh: count it there (#329).
	if kind != "":
		for it in _items:
			if it.get("kind", "") == kind and it["age"] < BATCH and is_instance_valid(it["node"]):
				it["count"] += 1
				it["pos"] = pos
				var lb: Label = it["node"]
				lb.text = "%s  (×%d)" % [text, it["count"]]
				it["age"] = 0.0
				return
	var l := Label.new()
	l.text = text
	l.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	l.custom_minimum_size = Vector2(WIDTH, 0)
	# A toast with a map position takes a click (to look where it happened); non-positional lines pass clicks through.
	l.mouse_filter = Control.MOUSE_FILTER_STOP if pos.x >= 0 else Control.MOUSE_FILTER_IGNORE
	l.modulate = Color(1, 1, 1, 0)
	l.add_theme_color_override("font_color", Color(colour))
	l.add_theme_color_override("font_shadow_color", Color(0, 0, 0, 0.9))
	l.add_theme_constant_override("shadow_offset_x", 1)
	l.add_theme_constant_override("shadow_offset_y", 1)
	var bg := StyleBoxFlat.new()
	bg.bg_color = Color(0.05, 0.05, 0.07, 0.82)
	bg.content_margin_left = 10
	bg.content_margin_right = 10
	bg.content_margin_top = 6
	bg.content_margin_bottom = 6
	l.add_theme_stylebox_override("normal", bg)
	_box.add_child(l)
	var item := {"node": l, "age": 0.0, "kind": kind, "count": 1, "pos": pos}
	_items.append(item)
	l.gui_input.connect(func(e: InputEvent) -> void:
		if e is InputEventMouseButton and e.pressed and e.button_index == MOUSE_BUTTON_LEFT:
			var p: Vector3i = item["pos"]
			if p.x >= 0:
				looked.emit(p))
	while _items.size() > MAX_ON_SCREEN:
		var old: Dictionary = _items.pop_front()
		var n: Control = old["node"]
		if is_instance_valid(n):
			n.queue_free()


func _process(d: float) -> void:
	if _items.is_empty() or held:
		return
	var alive: Array = []
	for it in _items:
		var n: Control = it["node"]
		if not is_instance_valid(n):
			continue
		var age: float = it["age"] + d
		if age < LIFE:
			# Fade in over the first fraction of a second, then hold.
			n.modulate.a = minf(age / 0.15, 1.0)
			it["age"] = age
			alive.append(it)
		elif age < LIFE + FADE:
			n.modulate.a = 1.0 - (age - LIFE) / FADE
			it["age"] = age
			alive.append(it)
		else:
			n.queue_free()
	_items = alive
