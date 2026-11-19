extends Node2D
## Roofs (building.rs), drawn over the people inside.
##
## Each connected roof is a piece in the entity layer's y-sort, its sort point on the roof's
## south edge: whoever stands inside, under it, is drawn beneath; whoever stands south of the
## house, and a canopy south of it, in front. This node only places the pieces. On a layer of
## its own above the entities every roof covered someone standing at its south wall (#179, QA
## 2026-10-20, C63).
##
## Each roof tile picks its piece from roof_thatch.png or roof_shingle.png by which of its four
## neighbours are roofed (the walls' 4-bit mask), so eaves close round the edge. Each connected
## roof is shaded as two slopes: the north half in the light, the south half a little darker.
## Snow lying on a roof fades in by how much lies there. A roof still going up shows its
## rafters once the timber is at the site.
##
## Cut-away: from 1.75x zoom roofs are drawn faint so the rooms show, and the roof over whoever
## is being followed or inspected is always faint.

const TILE := 32
const THATCH := 16
const STRIDE := 5
const CUT_ZOOM := 1.75
const FAINT := 0.22

var v  # world_view.gd
var pieces: Array = []

## One roof: its draw calls, made relative to its sort point.
class Piece extends Node2D:
	var calls: Array = []  # [texture, rect, source rect or null, colour]
	func _draw() -> void:
		for c in calls:
			if c[2] == null:
				draw_texture_rect(c[0], c[1], false, c[3])
			else:
				draw_texture_rect_region(c[0], c[1], c[2], c[3])

## The roof's sort point: the bottom edge of its southmost row, just below the feet of anyone
## standing on that row (their foot is a pixel above it).
static func sort_y(south_row: int) -> float:
	return (south_row + 1) * TILE

func _piece(k: int) -> Piece:
	while pieces.size() <= k:
		var p := Piece.new()
		v.entities.add_child(p)
		pieces.append(p)
	var p: Piece = pieces[k]
	p.visible = true
	return p

func _draw() -> void:
	for p in pieces:
		p.visible = false
		p.calls.clear()
	if v == null:
		return
	var R: PackedInt32Array = v.roofs
	var D: PackedInt32Array = v.designations
	var zoom: float = get_viewport().get_canvas_transform().get_scale().x
	var base_alpha := FAINT if zoom >= CUT_ZOOM else 1.0
	# Index the roofs and find each connected roof (so one house is one roof).
	var at := {}
	for i in range(0, R.size(), STRIDE):
		at[Vector2i(R[i], R[i + 1])] = i
	var comp := {}
	var spans := []  # per component: [min_y, max_y]
	for k in at.keys():
		if comp.has(k):
			continue
		var id := spans.size()
		var lo: int = k.y
		var hi: int = k.y
		var stack := [k]
		comp[k] = id
		while not stack.is_empty():
			var p: Vector2i = stack.pop_back()
			lo = mini(lo, p.y)
			hi = maxi(hi, p.y)
			for d in [Vector2i(1, 0), Vector2i(-1, 0), Vector2i(0, 1), Vector2i(0, -1)]:
				var q: Vector2i = p + d
				if at.has(q) and not comp.has(q):
					comp[q] = id
					stack.append(q)
		spans.append([lo, hi])
	# The roof over the followed or inspected person is cut away.
	var faint_comp := -1
	if v.focus_id >= 0:
		var A: PackedInt32Array = v.agents
		for i in range(0, A.size(), v.AGENT_STRIDE):
			if A[i] == v.focus_id:
				var fp := Vector2i(A[i + 1], A[i + 2])
				if comp.has(fp):
					faint_comp = comp[fp]
				break
	var thatch: Texture2D = v._tex("structures/roof_thatch.png")
	var shingle: Texture2D = v._tex("structures/roof_shingle.png")
	var snow: Texture2D = v._tex("structures/roof_snow.png")
	for k in at.keys():
		var i: int = at[k]
		var tex := thatch if R[i + 2] == THATCH else shingle
		if tex == null:
			continue
		var bits := 0
		if at.has(k + Vector2i(0, -1)): bits |= 1
		if at.has(k + Vector2i(1, 0)): bits |= 2
		if at.has(k + Vector2i(0, 1)): bits |= 4
		if at.has(k + Vector2i(-1, 0)): bits |= 8
		var c: int = comp[k]
		var alpha := FAINT if c == faint_comp else base_alpha
		var span: Array = spans[c]
		var south: bool = span[1] > span[0] and float(k.y - span[0]) > float(span[1] - span[0]) / 2.0
		var shade := Color(0.86, 0.84, 0.86, alpha) if south else Color(1, 1, 1, alpha)
		var piece: Piece = _piece(c)
		piece.position = Vector2(0, sort_y(span[1]))
		var r := Rect2(k.x * TILE, k.y * TILE - piece.position.y, TILE, TILE)
		piece.calls.append([tex, r, Rect2(bits * TILE, 0, TILE, TILE), shade])
		var lying: float = clampf(R[i + 4] / 24.0, 0.0, 0.9)
		if snow and lying > 0.02:
			piece.calls.append([snow, Rect2(r.position, snow.get_size()), null, Color(1, 1, 1, lying * alpha)])
	for k in spans.size():
		if k < pieces.size():
			pieces[k].queue_redraw()
	# Roofs going up.
	var frame: Texture2D = v._tex("structures/roof_frame.png")
	var n_frames := 0
	if frame:
		for j in range(0, D.size(), v.DESIG_STRIDE):
			if D[j + 4] == 16 and D[j + 3] == v.view_z and D[j + 6] == 1:
				# Rafters stand where the roof will: sorted on their own row.
				var piece: Piece = _piece(spans.size() + n_frames)
				n_frames += 1
				piece.position = Vector2(0, sort_y(D[j + 2]))
				piece.calls.append([frame, Rect2(Vector2(D[j + 1] * TILE, D[j + 2] * TILE - piece.position.y), frame.get_size()), null, Color(1, 1, 1, base_alpha)])
				piece.queue_redraw()
