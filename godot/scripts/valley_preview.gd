extends RefCounted
## The wizard's picture of a valley (#560): the valley the world will be made on, seen from
## above at one pixel a tile, in the colours the main view draws it with. Each ground, water
## and tree colour is the average of the sheet world_view.gd and entity_layer.gd draw (the
## flat colour world_view.gd falls back on where a sheet is missing), and each tile is lit
## from the north-west by the lie of the land. The red mark is where the portal opens.

const WorldView := preload("res://scripts/world_view.gd")
const EntityLayer := preload("res://scripts/render/entity_layer.gd")
const PORTAL := Color(0.95, 0.35, 0.25)

var assets
## Sheet path -> its average colour, worked out once.
var _colours := {}

func _init(p_assets = null) -> void:
	assets = p_assets if p_assets != null else preload("res://scripts/assets.gd").new()

## The average colour of a sheet's drawn pixels, or `fallback` when there is no sheet.
func colour_of(path: String, fallback: Color) -> Color:
	if _colours.has(path):
		return _colours[path]
	var c := fallback
	var t: Texture2D = assets.optional(path)
	if t != null:
		var img := t.get_image()
		if img != null:
			if img.is_compressed():
				img.decompress()
			c = _average(img, fallback)
	_colours[path] = c
	return c

static func _average(img: Image, fallback: Color) -> Color:
	var sum := Vector3.ZERO
	var n := 0.0
	# Every other pixel each way is plenty for an average, and keeps a wide sheet quick.
	for y in range(0, img.get_height(), 2):
		for x in range(0, img.get_width(), 2):
			var p := img.get_pixel(x, y)
			if p.a < 0.5:
				continue
			sum += Vector3(p.r, p.g, p.b)
			n += 1.0
	if n == 0.0:
		return fallback
	sum /= n
	return Color(sum.x, sum.y, sum.z)

func _ground_colour(tile: int) -> Color:
	var flags := (tile >> 24) & 0xff
	if flags & 2:
		return colour_of("terrain/deep_water.png", WorldView.FALLBACK["water"].darkened(0.25))
	if flags & 1:
		return colour_of("terrain/water.png", WorldView.FALLBACK["water"])
	var fam: String = WorldView.FAMILY.get(tile & 0xffff, "soil")
	return colour_of("terrain/%s.png" % fam, WorldView.FALLBACK.get(fam, WorldView.FALLBACK["soil"]))

## The crown seen from above where a grown tree stands, else null.
func _canopy(kind: int, x: int, y: int):
	if kind <= 0 or kind > WorldView.PLANT_KIND.size():
		return null
	var name: String = WorldView.PLANT_KIND[kind - 1]
	if name == "tree":
		# The generic tree is drawn as one of three species by its own visual seed, which the
		# preview does not carry: any of the three by place.
		name = EntityLayer.TREE_SPECIES[(x * 7 + y * 13) % EntityLayer.TREE_SPECIES.size()]
	elif not name in EntityLayer.TREE_KINDS:
		return null
	return colour_of("plants/%s_2.png" % name, Color(0.16, 0.30, 0.14))

## The picture of a preview from SimHost.valley_preview / valley_preview_take.
func image(p: Dictionary) -> Image:
	var w: int = int(p["width"])
	var h: int = int(p["height"])
	var tiles: PackedInt32Array = p["tiles"]
	var heights: PackedInt32Array = p["heights"]
	var plants: PackedByteArray = p["plants"]
	var data := PackedByteArray()
	data.resize(w * h * 3)
	for y in h:
		for x in w:
			var i := y * w + x
			var c := _ground_colour(tiles[i])
			var crown = _canopy(plants[i], x, y)
			if crown != null:
				c = crown
			# Lit from the north-west: a step up from that neighbour catches the light, a step
			# down is in its shadow.
			var dz := heights[i] - heights[maxi(y - 1, 0) * w + maxi(x - 1, 0)]
			if dz > 0:
				c = c.lightened(minf(0.08 * dz, 0.24))
			elif dz < 0:
				c = c.darkened(minf(0.12 * -dz, 0.36))
			data[i * 3] = c.r8
			data[i * 3 + 1] = c.g8
			data[i * 3 + 2] = c.b8
	var img := Image.create_from_data(w, h, false, Image.FORMAT_RGB8, data)
	var f: Vector3i = p["founding"]
	for dy in range(-2, 3):
		for dx in range(-2, 3):
			if absi(dx) + absi(dy) <= 2:
				img.set_pixel(clampi(f.x + dx, 0, w - 1), clampi(f.y + dy, 0, h - 1), PORTAL)
	return img
