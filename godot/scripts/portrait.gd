extends Control
## A procedural portrait for the inspect panel (#192, WP-08 8.4 third increment):
## the person's head and shoulders drawn from the same gene rows as the world
## sprites (agent_row fields 6-11, 16-18, 22), so the panel face is the same
## person the watcher sees on the ground. No new art: shapes only, in the
## palette ramps the sheets already swap (assets.palette + swaps.json groups).
##
## show_person(row, art) sets the genes and redraws; variant_key(row) is the
## deterministic summary the gd_test guard asserts on.

const W := 88
const H := 96

var g := {}
var assets   # the shared assets.gd instance (palette + swaps), set by main.gd


static func variant_key(row: Array) -> String:
	## The drawn identity of a person, in one line: same key, same portrait.
	return "%s,%s,%s,%s,%s,%s,%s" % [
		row[6], row[7], row[8], row[9], row[10], row[11], row[16],
	]


func _ready() -> void:
	custom_minimum_size = Vector2(W, H)


func show_person(row: Array, art: Dictionary, arm_marks: int = 0) -> void:
	g = {
		"body": row[6],
		"skin": row[7],
		"hair_style": row[8],
		"hair": row[9],
		"cloth": row[10],
		"cloth_colour": row[11],
		"child": row[16] == 1,
		"injured": row[17] == 1 and (arm_marks == 0 or (arm_marks & 4) != 0),
		"elder": row[18] == 1,
		"plan": art.get("plan", "biped"),
		"has_hair": art.get("hair", true),
		"body_set": art.get("body", "human"),
	}
	queue_redraw()


func _ramp(group: String, idx: int, step: int) -> Color:
	## The colour the sheets would use for this gene: swaps.json's target ramp
	## entry at a mid step, looked up in the palette by name.
	if assets == null or assets.swaps.is_empty() or not assets.swaps.has(group):
		return Color(0.6, 0.55, 0.5)
	var targets: Array = assets.swaps[group].get("targets", [])
	if targets.is_empty():
		return Color(0.6, 0.55, 0.5)
	var entry: Array = targets[absi(idx) % targets.size()]
	var name: String = entry[clampi(step, 0, entry.size() - 1)]
	if name.begins_with("#"):
		return Color(name)
	for p in assets.palette:
		if p.get("name", "") == name:
			return Color(p["hex"])
	return Color(0.6, 0.55, 0.5)


func _draw() -> void:
	if g.is_empty():
		return
	var cx := size.x / 2.0
	var scale := 0.78 if g.child else 1.0
	var skin := _ramp("skin", g.skin, 3)
	var hair_c := _ramp("hair", g.hair, 2)
	if g.elder:
		hair_c = hair_c.lerp(Color(0.75, 0.75, 0.72), 0.6)
	var cloth := _ramp("cloth", g.cloth_colour, 3)
	# Shoulders: a rounded slab of the clothing colour, sitting on the frame's floor.
	var sh := Rect2(cx - 30 * scale, size.y - 34 * scale, 60 * scale, 34 * scale)
	draw_rect(sh, cloth)
	draw_rect(Rect2(sh.position, Vector2(sh.size.x, 3)), cloth.darkened(0.18))
	# Head.
	var head_r := 17.0 * scale
	var head_c := Vector2(cx, size.y - 40 * scale - head_r)
	draw_circle(head_c, head_r, skin)
	# Hair: a cap over the crown; five shapes cycle the ten styles. A kind without
	# hair (#485) shows its head marking instead: one short stripe.
	if g.has_hair:
		var style: int = absi(g.hair_style) % 5
		var cap := Rect2(head_c.x - head_r, head_c.y - head_r, head_r * 2, head_r * 1.15)
		match style:
			0:  # short
				draw_rect(Rect2(cap.position, Vector2(cap.size.x, cap.size.y * 0.55)), hair_c)
			1:  # long: cap plus a fall behind the shoulders
				draw_rect(Rect2(cap.position, Vector2(cap.size.x, cap.size.y * 0.55)), hair_c)
				draw_rect(Rect2(head_c.x - head_r, head_c.y, 5, head_r + 12 * scale), hair_c)
				draw_rect(Rect2(head_c.x + head_r - 5, head_c.y, 5, head_r + 12 * scale), hair_c)
			2:  # topknot
				draw_rect(Rect2(cap.position, Vector2(cap.size.x, cap.size.y * 0.5)), hair_c)
				draw_circle(Vector2(head_c.x, head_c.y - head_r - 3 * scale), 5 * scale, hair_c)
			3:  # bald pate: hair at the sides only
				draw_rect(Rect2(cap.position.x, cap.position.y + 4, 5, cap.size.y * 0.5), hair_c)
				draw_rect(Rect2(cap.position.x + cap.size.x - 5, cap.position.y + 4, 5, cap.size.y * 0.5), hair_c)
			_:  # swept
				draw_rect(Rect2(cap.position, Vector2(cap.size.x, cap.size.y * 0.6)), hair_c)
				draw_circle(Vector2(head_c.x + head_r * 0.5, head_c.y - head_r * 0.5), 6 * scale, hair_c)
	else:
		var mark := _ramp("skin", g.skin, 5).darkened(0.35)
		draw_rect(Rect2(head_c.x - 6 * scale, head_c.y - head_r * 0.4, 12 * scale, 4 * scale), mark)
	# Face: two quiet eyes; no mouth (the sheets' faces are implied at this size).
	var eye := skin.darkened(0.55)
	draw_circle(head_c + Vector2(-6, 1) * scale, 1.8 * scale, eye)
	draw_circle(head_c + Vector2(6, 1) * scale, 1.8 * scale, eye)
	# A bandage over the crown when injured, as the world sprite wears.
	if g.injured:
		draw_rect(Rect2(head_c.x - head_r, head_c.y - 4 * scale, head_r * 2, 5 * scale),
			Color(0.85, 0.8, 0.7))
