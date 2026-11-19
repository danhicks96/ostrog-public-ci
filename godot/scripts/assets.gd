extends RefCounted

## Loads generated sheets by name; returns null when a sheet is missing so the renderer can
## fall back to flat colours (development only; release acceptance requires every sheet).

var cache := {}
var missing := {}
var palette = []
var swaps := {}

func tex(path: String) -> Texture2D:
	if cache.has(path):
		return cache[path]
	var full := "res://assets/" + path
	var t: Texture2D = null
	if ResourceLoader.exists(full):
		t = load(full)
	# Not imported, or the import record is there but its texture is not (an import pass that
	# died part way, issue 42): read the PNG itself rather than draw a flat fallback colour.
	if t == null and FileAccess.file_exists(full):
		var img := Image.load_from_file(full)
		if img:
			t = ImageTexture.create_from_image(img)
	if t == null:
		missing[path] = true
	cache[path] = t
	return t

## A sheet that may legitimately not exist (a season variant): null without counting it as
## missing.
func optional(path: String) -> Texture2D:
	if cache.has(path):
		return cache[path]
	var full := "res://assets/" + path
	if ResourceLoader.exists(full) or FileAccess.file_exists(full):
		return tex(path)
	cache[path] = null
	return null

func load_json(path: String) -> Variant:
	var full := "res://assets/" + path
	if not FileAccess.file_exists(full):
		return null
	var f := FileAccess.open(full, FileAccess.READ)
	var v = JSON.parse_string(f.get_as_text())
	return v

func _init() -> void:
	var p = load_json("palette/ostrog.json")
	if p != null:
		palette = p
	var s = load_json("people/swaps.json")
	if s != null:
		swaps = s

func report_missing() -> Array:
	return missing.keys()
