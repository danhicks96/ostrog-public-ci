extends SceneTree
## Loads every GDScript under res://scripts so a parse or compile error fails CI (tools/ci.sh).
## ci.sh runs it from godot/ with this file's absolute path: tools/godot --path . --headless -s
## /abs/path/tools/gd_check.gd

func _init() -> void:
	var bad := 0
	var files: Array[String] = []
	_collect("res://scripts", files)
	for f in files:
		var s = load(f)
		if s == null or not (s is GDScript) or not s.can_instantiate():
			print("GD_CHECK_FAIL %s" % f)
			bad += 1
	print("GD_CHECK %d scripts, %d failed" % [files.size(), bad])
	quit(1 if bad > 0 else 0)

func _collect(dir: String, out: Array[String]) -> void:
	var d := DirAccess.open(dir)
	if d == null:
		return
	for f in d.get_files():
		if f.ends_with(".gd"):
			out.append(dir.path_join(f))
	for sub in d.get_directories():
		_collect(dir.path_join(sub), out)
