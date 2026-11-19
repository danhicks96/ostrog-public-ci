extends Node2D
## Drawn above the creatures and below the grade: stack counts on items, and the fog over
## ground the community has not seen (discovery-respecting view).

const TILE := 32
var v

func init(view) -> void:
	v = view

func _draw() -> void:
	var cull: Rect2i = v.cull_rect()
	var x0 := cull.position.x
	var y0 := cull.position.y
	var x1 := cull.position.x + cull.size.x
	var y1 := cull.position.y + cull.size.y
	var I: PackedInt32Array = v.items
	for i in range(0, I.size(), v.ITEM_STRIDE):
		if I[i + 3] != v.view_z:
			continue
		var x: int = I[i + 1]
		var y: int = I[i + 2]
		if x < x0 or x >= x1 or y < y0 or y >= y1:
			continue
		# Quality marks (C35): one gold pip for fine work, a star for a masterwork, top right.
		var tier: int = I[i + 7]
		if tier >= 4:
			_mark(Vector2(x * TILE + TILE - 6, y * TILE + 5), tier >= 5)
		if I[i + 5] <= 1:
			continue
		var at := Vector2(x * TILE + 2, y * TILE + TILE - 3)
		draw_string(ThemeDB.fallback_font, at + Vector2(1, 1), str(I[i + 5]), HORIZONTAL_ALIGNMENT_LEFT, -1, 9, Color(0, 0, 0, 0.8))
		draw_string(ThemeDB.fallback_font, at, str(I[i + 5]), HORIZONTAL_ALIGNMENT_LEFT, -1, 9, Color(1, 1, 1, 0.92))
	# Carved walls (C35): grooves cut into the wall's face, more of them for finer work.
	var E: PackedInt32Array = v.engravings
	for i in range(0, E.size(), 4):
		if E[i + 2] != v.view_z:
			continue
		var ex: int = E[i]
		var ey: int = E[i + 1]
		if ex < x0 or ex >= x1 or ey < y0 or ey >= y1:
			continue
		var rows: int = 2 + mini(E[i + 3], 4) / 2
		for r in rows:
			var yy: float = ey * TILE + 9 + r * 5
			var x_a: float = ex * TILE + 7 + (r % 2) * 3
			var x_b: float = ex * TILE + TILE - 7 - ((r + 1) % 2) * 4
			draw_line(Vector2(x_a, yy), Vector2(x_b, yy), Color(0.08, 0.06, 0.08, 0.75), 1.0)
			draw_line(Vector2(x_a, yy + 1), Vector2(x_b, yy + 1), Color(0.95, 0.9, 0.78, 0.45), 1.0)
		draw_circle(Vector2(ex * TILE + TILE / 2.0, ey * TILE + 6), 1.5, Color(0.08, 0.06, 0.08, 0.7))
	if v.known_only:
		for cy in range(y0 >> 3, (y1 >> 3) + 1):
			for cx in range(x0 >> 3, (x1 >> 3) + 1):
				if not v.explored.has(Vector2i(cx, cy)):
					draw_rect(Rect2(cx * 8 * TILE, cy * 8 * TILE, 8 * TILE, 8 * TILE), Color(0.02, 0.02, 0.04, 0.86))

## A fine piece's pip, or a masterwork's four-pointed star, with a dark outline so it reads on
## any ground.
func _mark(c: Vector2, star: bool) -> void:
	var gold := Color(0.94, 0.85, 0.58)
	var ink := Color(0.05, 0.04, 0.07, 0.9)
	if star:
		var pts := PackedVector2Array([c + Vector2(0, -4), c + Vector2(1, -1), c + Vector2(4, 0),
			c + Vector2(1, 1), c + Vector2(0, 4), c + Vector2(-1, 1), c + Vector2(-4, 0), c + Vector2(-1, -1)])
		draw_colored_polygon(pts, ink)
		var inner := PackedVector2Array()
		for p in pts:
			inner.append(c + (p - c) * 0.7)
		draw_colored_polygon(inner, gold)
	else:
		draw_rect(Rect2(c - Vector2(2, 2), Vector2(4, 4)), ink)
		draw_rect(Rect2(c - Vector2(1, 1), Vector2(2, 2)), gold)

