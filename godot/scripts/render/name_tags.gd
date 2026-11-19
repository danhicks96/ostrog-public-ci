extends Node2D
## Names over people in the world (#1428, part of #1423): a viewer can keep track of a story
## without clicking. Which people get a tag is Rules.name_tag_ids: by default favourites and the
## followed or selected person; with the setting at "everyone", every person once the camera is
## close enough. Drawn above the entity layer, one small label over each tagged head, the way
## world_marks.gd labels its rings.
##
## The tags follow the entity layer's person sprites (their head anchor, as the relationship
## lines use), so they sit where the person is drawn and only move when the sprites move.

const Rules := preload("res://scripts/rules.gd")
const TILE := 32
const FONT_SIZE := 12
## The followed or selected person's tag is warmer, so the one being watched stands out.
const WATCHED := Color(1.0, 0.86, 0.5)
const PLAIN := Color(0.93, 0.93, 0.9)
## The hovered person's tag (#1429): the answer where the eye already is.
const HOVER := Color(0.75, 0.92, 1.0)
## A label's height with its backdrop, and a pixel apart: how far a crowded label moves up.
const ROW := 15.0
## Zoomed out, labels grow in the world by up to this much, so on screen they stay readable.
const MAX_GROW := 2.5

var v   # world_view
var sim
## 0 off, 1 favourites and followed, 2 everyone up close (Rules.NAME_TAG_NAMES).
var mode := 1
var followed := -1
var selected := -1
## Who the mouse is over and the words for them (#1429): drawn beside them whatever the mode.
var hover_id := -1
var hover_text := ""
var _names := {}   # id -> name, asked of the bridge once per person
var _zoom := 0.0
## The scale of this frame's labels in the world (1 at 1x zoom and closer).
var _k := 1.0


func init(view, s) -> void:
	v = view
	sim = s


## A loaded world has other people under the same ids.
func reset() -> void:
	_names.clear()
	queue_redraw()


## The camera's zoom (1 when not yet in the tree, as in a guard test).
func _zoom_now() -> float:
	if not is_inside_tree():
		return 1.0
	return get_viewport().get_canvas_transform().get_scale().x


func _process(_d: float) -> void:
	var z := _zoom_now()
	if z != _zoom:
		_zoom = z
		queue_redraw()


func _name(id: int) -> String:
	if not _names.has(id):
		_names[id] = str(sim.agent_name(id))
	return _names[id]


## Each person sprite in view: [id, head anchor].
func people() -> Array:
	var out := []
	if v == null or v.entities == null:
		return out
	var A: PackedInt32Array = v.agents
	for e in v.entities.animated:
		if e[0] != "person":
			continue
		var i: int = e[2]
		if i + v.AGENT_STRIDE > A.size():
			continue
		out.append([A[i], e[1].position + Vector2(TILE / 2.0, v.entities._person_offset(e[3]).y - 4)])
	return out


## The tags drawn now: [id, name, where the label's baseline starts, colour].
func tags() -> Array:
	var out := []
	var hovering := hover_id >= 0 and hover_text != ""
	if sim == null or (mode <= 0 and not hovering):
		return out
	var seen := people()
	var ids := []
	var favs := {}
	var zoom := _zoom_now()
	# Favourites matter unless everyone is tagged anyway ("everyone" further out tags as 1).
	var ask_favs := not (mode >= 2 and zoom >= Rules.NAME_TAG_ALL_ZOOM)
	for p in seen:
		ids.append(p[0])
		if ask_favs and sim.is_favourite(p[0]):
			favs[p[0]] = true
	var keep := Rules.name_tag_ids(ids, mode, zoom, favs, followed, selected)
	if hovering and not keep.has(hover_id):
		keep.append(hover_id)
	var font := ThemeDB.fallback_font
	_k = clampf(1.0 / maxf(zoom, 0.01), 1.0, MAX_GROW)
	var size := int(round(FONT_SIZE * _k))
	var want := []
	for p in seen:
		if not keep.has(p[0]):
			continue
		var is_hover: bool = hovering and p[0] == hover_id
		var text := hover_text if is_hover else _name(p[0])
		if text == "":
			continue
		var w := font.get_string_size(text, HORIZONTAL_ALIGNMENT_LEFT, -1, size).x
		var watched: bool = p[0] == followed or p[0] == selected
		var rank := 0 if watched else (1 if favs.has(p[0]) else 2)
		var col := WATCHED if watched else PLAIN
		if is_hover:
			rank = -1
			col = HOVER
		want.append([rank, p[0], text, p[1] + Vector2(-w / 2.0, 0), w, col])
	# In a crowd, labels must not print over each other: the hovered person first (#1429), then
	# the watched person, then favourites, then top to bottom; a label that would overlap one already placed goes up a
	# row or two, and is left out if it still overlaps.
	want.sort_custom(func(x, y): return x[0] < y[0] or (x[0] == y[0] and x[3].y < y[3].y))
	var placed: Array[Rect2] = []
	for t in want:
		for lift in [0.0, -ROW * _k, -2.0 * ROW * _k]:
			var at: Vector2 = t[3] + Vector2(0, lift)
			var r := _box(at, t[4])
			var clear := true
			for q in placed:
				if q.intersects(r):
					clear = false
					break
			if clear:
				placed.append(r)
				out.append([t[1], t[2], at, t[5]])
				break
	return out


## The backdrop of a label whose baseline starts at `at` and is `w` wide, at this frame's scale.
func _box(at: Vector2, w: float) -> Rect2:
	return Rect2(at + Vector2(-3, -11) * _k, Vector2(w + 6 * _k, 14 * _k))


func _draw() -> void:
	var font := ThemeDB.fallback_font
	for t in tags():
		var size := int(round(FONT_SIZE * _k))
		var w := font.get_string_size(t[1], HORIZONTAL_ALIGNMENT_LEFT, -1, size).x
		draw_rect(_box(t[2], w), Color(0.05, 0.05, 0.07, 0.7))
		draw_string(font, t[2], t[1], HORIZONTAL_ALIGNMENT_LEFT, -1, size, t[3])
