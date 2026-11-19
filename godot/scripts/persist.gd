extends RefCounted
## Save ring, autosave schedule, bookmarks and settings persistence for the client.

const RING := 3
const SETTINGS_PATH := "user://settings.cfg"

var settings := {
	"master": 0.8, "ambient": 0.7, "fx": 0.8, "music": 0.6,
	"director": true, "debug_log": false, "autosave_days": 1.0, "ui_scale": 1.0, "timelapse": false,
	# The key list has been shown once (#181): later launches show it on [?] only.
	"help_seen": false,
	# Readability (#413 A5): feed rate (0 calm, 1 normal, 2 full) and first-run hints.
	"feed_rate": 1,
	# Name tags over people (#1428): 0 off, 1 favourites and followed, 2 everyone up close.
	"name_tags": 1,
	"hints_seen": false,
}
var bookmarks := {}  # int -> {"pos": Vector2, "z": int, "zoom": float}

func save_dir() -> String:
	var d := ProjectSettings.globalize_path("user://saves")
	DirAccess.make_dir_recursive_absolute(d)
	return d

func ring_base() -> String:
	return save_dir().path_join("auto")

func manual_path() -> String:
	return save_dir().path_join("manual.hfsave")

func load_settings() -> void:
	var cf := ConfigFile.new()
	if cf.load(SETTINGS_PATH) != OK:
		return
	for k in settings.keys():
		settings[k] = cf.get_value("settings", k, settings[k])
	for i in range(1, 9):
		if cf.has_section_key("bookmarks", str(i)):
			bookmarks[i] = cf.get_value("bookmarks", str(i))

func save_settings() -> void:
	var cf := ConfigFile.new()
	for k in settings.keys():
		cf.set_value("settings", k, settings[k])
	for i in bookmarks.keys():
		cf.set_value("bookmarks", str(i), bookmarks[i])
	cf.save(SETTINGS_PATH)
