extends Node2D
## In-world toasters (#351, WP-08 8.5): local events the valley knows, drawn where they
## happened — a pulsing ring with the toast's line as its label, complementing the screen
## toasters #329 already gives major events. Only events the sim has told the viewer reach
## here (the feed is the told stream), so the layer is inherently "what the valley knows".
##
## main.gd calls mark() where it toasts; a marker stands for Rules.MARKER_LIFE seconds, a
## retold tile refreshes its marker instead of stacking another ring, and at most MAX stand.
## Off-level marks draw dimmer: the event is elsewhere in the column, not here. (#738 merged
## the first marks layer; the label, long hold, dedup and cap are the #735 design, ported
## here after the duplication was triaged — see the PR.)

const Rules := preload("res://scripts/rules.gd")
const TILE := 32
const MAX := 12
## Off-level marks dim to this share of their alpha (#738).
const OFF_LEVEL_DIM := 0.35

var v   # world_view
var marks: Array = []   # [{tile: Vector3i, text: String, color: Color, until: float}]
var _t := 0.0


func init(view) -> void:
	v = view


func mark(pos: Vector3i, color: Color, text := "") -> void:
	if pos.x < 0:
		return
	# The same place told again while its marker stands: refresh it, never stack rings.
	for m in marks:
		if m.tile == pos:
			m.text = Rules.marker_label(text)
			m.color = color
			m.until = _t + Rules.MARKER_LIFE
			queue_redraw()
			return
	marks.append({
		"tile": pos,
		"text": Rules.marker_label(text),
		"color": color,
		"until": _t + Rules.MARKER_LIFE,
	})
	while marks.size() > MAX:
		marks.pop_front()
	queue_redraw()


func clear() -> void:
	marks.clear()
	queue_redraw()


func _process(delta: float) -> void:
	_t += delta
	var n := marks.size()
	marks = marks.filter(func(m): return _t < m.until)
	if marks.size() != n:
		queue_redraw()


func _draw() -> void:
	if v == null:
		return
	var on_z: int = v.view_z
	var font := ThemeDB.fallback_font
	for m in marks:
		var age: float = Rules.MARKER_LIFE - (m.until - _t)
		var a: float = Rules.marker_alpha(age)
		if a <= 0.0:
			continue
		if m.tile.z != on_z:
			a *= OFF_LEVEL_DIM
		var c: Color = m.color
		c.a = a
		var centre := Vector2(m.tile.x, m.tile.y) * TILE + Vector2(TILE / 2.0, TILE / 2.0)
		var r := Rules.marker_radius(age)
		draw_arc(centre, r + 1.5, 0, TAU, 32, Color(0, 0, 0, 0.6 * a), 4.0)
		draw_arc(centre, r, 0, TAU, 32, c, 2.5)
		if m.text != "":
			var w := font.get_string_size(m.text, HORIZONTAL_ALIGNMENT_LEFT, -1, 13).x
			var at := centre + Vector2(-w / 2.0, -r - 8)
			draw_rect(Rect2(at + Vector2(-4, -13), Vector2(w + 8, 17)), Color(0.05, 0.05, 0.07, 0.75 * a))
			draw_string(font, at, m.text, HORIZONTAL_ALIGNMENT_LEFT, -1, 13, c)
