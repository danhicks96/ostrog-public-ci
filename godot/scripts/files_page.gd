extends PanelContainer
## "Your files and data" (WP-09 slice 9.12): every file and folder Ostrog makes, in plain
## words, with where it is, its size on disk now, whether it is ever sent and whether it is safe
## to delete. The rows come from res://data/files_register.json, which
## tools/privacy/files_guide.py generates from docs/privacy/FILES-ON-YOUR-COMPUTER.md (CI fails
## when they differ), so this page, READ ME FIRST.txt and the register never drift apart.
## It reads the disk only when it opens; it never sends anything.

const REGISTER := "res://data/files_register.json"

## The page was closed (Close or Esc): main.gd shows Settings again, where it was opened from.
signal closed

## Where each place of the register is on this computer: "" when it cannot be known (the
## install folder in a run from the editor, where the executable is Godot itself).
var roots := {}
var _list: VBoxContainer
var _status: Label
var _make_btn: Callable
## Whether the debug log is being written now: main.gd sets it before open(), and "Delete logs"
## then leaves that file alone.
var debug_log_on := false

## `make_btn` is main.gd's button maker, so the page's buttons look like every other panel's.
func setup(make_btn: Callable, panel_bg: StyleBox) -> void:
	_make_btn = make_btn
	# Opaque: it opens over Settings and the valley, and long text over either is hard to read.
	var bg: StyleBox = panel_bg.duplicate()
	if bg is StyleBoxFlat:
		(bg as StyleBoxFlat).bg_color.a = 1.0
	add_theme_stylebox_override("panel", bg)
	set_anchors_preset(Control.PRESET_CENTER)
	offset_left = -430; offset_right = 430
	offset_top = -330; offset_bottom = 330
	visible = false
	var box := VBoxContainer.new()
	box.add_theme_constant_override("separation", 6)
	add_child(box)
	var hdr := HBoxContainer.new()
	box.add_child(hdr)
	var title := Label.new()
	title.text = "Your files and data"
	title.add_theme_font_size_override("font_size", 18)
	title.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	hdr.add_child(title)
	hdr.add_child(make_btn.call("Delete logs", "Delete the finished logs (never the one in use or your saves)", _delete_logs))
	hdr.add_child(make_btn.call("Close [Esc]", "Close this page [Esc]", close))
	var intro := Label.new()
	intro.text = "Everything Ostrog keeps on this computer, what it is for, and whether any of it is ever sent."
	intro.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	intro.add_theme_font_size_override("font_size", 12)
	box.add_child(intro)
	_status = Label.new()
	_status.add_theme_font_size_override("font_size", 12)
	_status.add_theme_color_override("font_color", Color(0.75, 0.9, 0.75))
	box.add_child(_status)
	var scroll := ScrollContainer.new()
	scroll.size_flags_vertical = Control.SIZE_EXPAND_FILL
	scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	box.add_child(scroll)
	_list = VBoxContainer.new()
	_list.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	_list.add_theme_constant_override("separation", 8)
	scroll.add_child(_list)

## The places on this computer: the game data folder, the temporary folder, and the install
## folder when this is a packaged game (ostrog.exe in the install's app/ folder).
static func find_roots() -> Dictionary:
	var install := ""
	var exe := OS.get_executable_path().get_base_dir()
	if OS.has_feature("template") and exe.get_file() == "app":
		install = exe.get_base_dir()
	return {
		"install": install,
		"userdata": ProjectSettings.globalize_path("user://"),
		"temp": OS.get_temp_dir(),
	}

static func load_sections() -> Array:
	var parsed = JSON.parse_string(FileAccess.get_file_as_string(REGISTER))
	if typeof(parsed) != TYPE_DICTIONARY:
		return []
	return parsed.get("sections", [])

## Opens the page, reading the sizes on disk now.
func open() -> void:
	if roots.is_empty():
		roots = find_roots()
	_status.text = ""
	_fill()
	visible = true

func close() -> void:
	if visible:
		visible = false
		closed.emit()

func _fill() -> void:
	for c in _list.get_children():
		c.queue_free()
	for s in load_sections():
		var root: String = roots.get(s["place"], "")
		var head := HBoxContainer.new()
		_list.add_child(head)
		var t := Label.new()
		t.text = s["title"]
		t.add_theme_font_size_override("font_size", 16)
		t.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		head.add_child(t)
		if root != "":
			head.add_child(_make_btn.call("Open folder", "Open %s in the file browser" % root, func(): OS.shell_open(root)))
		var where := Label.new()
		where.text = "Where: " + (root if root != "" else s["where"])
		where.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		where.add_theme_font_size_override("font_size", 11)
		_list.add_child(where)
		for r in s["rows"]:
			_list.add_child(_row(r, root))
		_list.add_child(HSeparator.new())

func _row(r: Dictionary, root: String) -> Control:
	var l := RichTextLabel.new()
	l.bbcode_enabled = true
	l.fit_content = true
	l.scroll_active = false
	l.add_theme_font_size_override("normal_font_size", 13)
	l.add_theme_font_size_override("bold_font_size", 13)
	var size_txt := ""
	if root != "":
		var n := size_of(root, r["names"])
		size_txt = "   [color=#9fb8d0]on disk now: %s[/color]" % (human_size(n) if n >= 0 else "not there")
	l.text = "[b]%s[/b]%s\n%s\n[color=#c8c8a0]Sent anywhere?[/color] %s   [color=#c8c8a0]Safe to delete?[/color] %s" % [
		_esc(", ".join(r["names"])), size_txt, _esc(r["what"]), _esc(r["sent"]), _esc(r["delete"])]
	return l

static func _esc(s: String) -> String:
	return s.replace("[", "[lb]")

## Bytes on disk for a row's names under `root` (a folder counts everything inside it); -1 when
## none of them is there. A name with a wildcard ("godot<date>...", "*.zip.part") is skipped.
static func size_of(root: String, names: Array) -> int:
	var total := -1
	for n in names:
		var name: String = n
		if name.contains("*") or name.contains("<"):
			continue
		var p := root.path_join(name.trim_suffix("/"))
		var b := _bytes(p)
		if b >= 0:
			total = maxi(total, 0) + b
	return total

static func _bytes(path: String) -> int:
	if FileAccess.file_exists(path):
		var f := FileAccess.open(path, FileAccess.READ)
		return f.get_length() if f else 0
	if not DirAccess.dir_exists_absolute(path):
		return -1
	var total := 0
	var d := DirAccess.open(path)
	if d == null:
		return 0
	d.include_hidden = true
	for f in d.get_files():
		total += maxi(_bytes(path.path_join(f)), 0)
	for sub in d.get_directories():
		total += maxi(_bytes(path.path_join(sub)), 0)
	return total

static func human_size(b: int) -> String:
	if b < 1024:
		return "%d bytes" % b
	if b < 1024 * 1024:
		return "%.0f KB" % (b / 1024.0)
	if b < 1024 * 1024 * 1024:
		return "%.1f MB" % (b / 1048576.0)
	return "%.2f GB" % (b / 1073741824.0)

## Which logs "Delete logs" removes, from the names in the game data folder and its logs/
## folder: the engine's older logs and the debug log's older half. Never the engine log being
## written (logs/godot.log) and never the debug log while debug mode writes it.
static func deletable_logs(log_files: Array, data_files: Array, debug_on: bool) -> Array:
	var out := []
	for f in log_files:
		var name: String = f
		if name.begins_with("godot") and name.ends_with(".log") and name != "godot.log":
			out.append("logs/" + name)
	for f in data_files:
		var name: String = f
		if name == "ostrog-debug.log.1" or (name == "ostrog-debug.log" and not debug_on):
			out.append(name)
	return out

func _delete_logs() -> void:
	var root: String = roots.get("userdata", "")
	if root == "":
		return
	var logs_dir := DirAccess.open(root.path_join("logs"))
	var data_dir := DirAccess.open(root)
	var gone := deletable_logs(
		Array(logs_dir.get_files()) if logs_dir else [],
		Array(data_dir.get_files()) if data_dir else [],
		debug_log_on)
	var freed := 0
	var removed := 0
	for rel in gone:
		var p: String = root.path_join(rel)
		var b := _bytes(p)
		if DirAccess.remove_absolute(p) == OK:
			removed += 1
			freed += maxi(b, 0)
	_status.text = "Deleted %d old log%s (%s)." % [removed, "" if removed == 1 else "s", human_size(freed)] if removed > 0 else "No old logs to delete."
	_fill()
