extends PanelContainer
## Image-only timelapse review. This panel has no SimHost and cannot alter the source world.

signal recording_changed(enabled: bool)
signal use_current_requested
signal valley_requested
signal close_requested

var frames: Array = []
var selected := -1
var reference := 0
var selected_tick := -1
var reference_tick := -1
var recording := false
var playing := false
var follow_latest := true
var playback_speed := 1.0
var playback_accum := 0.0
var watch_mode := "view"
const Timelapse := preload("res://scripts/timelapse.gd")

var record_button: Button
var watch_button: Button
var valley_button: Button
var markers: Control
var events_label: RichTextLabel
var play_button: Button
var speed_button: Button
var previous_button: Button
var next_button: Button
var pin_button: Button
var slider: HSlider
var status_label: Label
var reference_label: Label
var selected_label: Label
var reference_image: TextureRect
var selected_image: TextureRect

func _ready() -> void:
	set_anchors_preset(Control.PRESET_CENTER)
	offset_left = -570
	offset_right = 570
	offset_top = -330
	offset_bottom = 330
	var bg := StyleBoxFlat.new()
	bg.bg_color = Color(0.05, 0.05, 0.07, 0.94)
	bg.content_margin_left = 12
	bg.content_margin_right = 12
	bg.content_margin_top = 10
	bg.content_margin_bottom = 10
	add_theme_stylebox_override("panel", bg)
	var outer := VBoxContainer.new()
	outer.add_theme_constant_override("separation", 7)
	add_child(outer)
	var header := HBoxContainer.new()
	outer.add_child(header)
	var title := Label.new()
	title.text = "Timelapse — a sample each noon"
	title.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	header.add_child(title)
	record_button = _button("Start recording", func():
		recording_changed.emit(not recording)
	)
	header.add_child(record_button)
	watch_button = _button("Use current view", func(): use_current_requested.emit())
	watch_button.tooltip_text = "Move the watched region to the current camera, level, zoom and knowledge filter"
	header.add_child(watch_button)
	valley_button = _button("Whole local map", func(): valley_requested.emit())
	valley_button.tooltip_text = "Watch the whole local map at the settlement's level instead of one view"
	header.add_child(valley_button)
	header.add_child(_button("Close [T]", func(): close_requested.emit()))
	status_label = Label.new()
	status_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	outer.add_child(status_label)
	var images := HBoxContainer.new()
	images.size_flags_vertical = Control.SIZE_EXPAND_FILL
	images.add_theme_constant_override("separation", 10)
	outer.add_child(images)
	var left := _image_column("Reference (earliest unless pinned)", images)
	reference_label = left[0]
	reference_image = left[1]
	var right := _image_column("Selected sample", images)
	selected_label = right[0]
	selected_image = right[1]
	events_label = RichTextLabel.new()
	events_label.bbcode_enabled = true
	events_label.fit_content = false
	events_label.custom_minimum_size = Vector2(0, 84)
	events_label.scroll_active = true
	events_label.add_theme_font_size_override("normal_font_size", 12)
	outer.add_child(events_label)
	# One mark per retained sample above the scrubber: taller where more happened in the record
	# the viewer was told of since the sample before it; the selected and pinned samples are lit.
	markers = Control.new()
	markers.custom_minimum_size = Vector2(0, 16)
	markers.mouse_filter = Control.MOUSE_FILTER_STOP
	markers.draw.connect(_draw_markers)
	markers.gui_input.connect(_markers_input)
	outer.add_child(markers)
	var controls := HBoxContainer.new()
	controls.add_theme_constant_override("separation", 5)
	outer.add_child(controls)
	previous_button = _button("←", func(): select_frame(selected - 1))
	controls.add_child(previous_button)
	play_button = _button("Play review", func():
		if frames.size() < 2:
			return
		if selected == frames.size() - 1:
			select_frame(0)
		playing = not playing
		follow_latest = false
		_sync_controls()
	)
	controls.add_child(play_button)
	next_button = _button("→", func(): select_frame(selected + 1))
	controls.add_child(next_button)
	speed_button = _button("1×", func():
		var speeds := [0.5, 1.0, 2.0, 4.0]
		playback_speed = speeds[(speeds.find(playback_speed) + 1) % speeds.size()]
		_sync_controls()
	)
	speed_button.tooltip_text = "Review playback speed in sampled frames per second; the source simulation keeps its own speed"
	controls.add_child(speed_button)
	slider = HSlider.new()
	slider.min_value = 0
	slider.step = 1
	slider.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	slider.value_changed.connect(func(v: float): select_frame(int(v)))
	controls.add_child(slider)
	pin_button = _button("Pin selected", func():
		if selected >= 0:
			reference_tick = int(frames[selected]["tick"])
			reference = selected
			_show_images()
	)
	pin_button.tooltip_text = "Compare the selected sample against this reference"
	controls.add_child(pin_button)
	_sync_controls()

func _button(text: String, callback: Callable) -> Button:
	var b := Button.new()
	b.text = text
	b.focus_mode = Control.FOCUS_NONE
	b.add_theme_font_size_override("font_size", 12)
	b.pressed.connect(callback)
	return b

func _image_column(title: String, parent: HBoxContainer) -> Array:
	var column := VBoxContainer.new()
	column.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	parent.add_child(column)
	var label := Label.new()
	label.text = title
	label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	column.add_child(label)
	var image := TextureRect.new()
	image.custom_minimum_size = Vector2(500, 330)
	image.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	image.size_flags_vertical = Control.SIZE_EXPAND_FILL
	image.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
	image.stretch_mode = TextureRect.STRETCH_KEEP_ASPECT_CENTERED
	image.texture_filter = CanvasItem.TEXTURE_FILTER_NEAREST
	column.add_child(image)
	return [label, image]

func sync(new_frames: Array, is_recording: bool, total_bytes: int) -> void:
	# FIXLOG 2026-10-31 #484 (c145 review): copying guards against the recorder's in-place
	# append changing old_count; tick identity keeps follow/pin stable across eviction.
	frames = new_frames.duplicate()
	recording = is_recording
	if frames.is_empty():
		selected = -1
		reference = 0
		selected_tick = -1
		reference_tick = -1
	else:
		if selected_tick < 0 or follow_latest:
			selected_tick = int(frames[-1]["tick"])
		selected = _index_of_tick(selected_tick)
		if selected < 0:
			selected = frames.size() - 1
			selected_tick = int(frames[selected]["tick"])
		if reference_tick < 0:
			reference_tick = int(frames[0]["tick"])
		reference = _index_of_tick(reference_tick)
		if reference < 0:
			reference = 0
			reference_tick = int(frames[0]["tick"])
	status_label.text = "%d / %d samples · %.1f of %d MiB kept · recording %s · watching %s\nReview is image-only: pause, scrub and compare without changing or pausing the source world." % [
		frames.size(), Timelapse.MAX_FRAMES, total_bytes / 1048576.0, Timelapse.MAX_SERIES_BYTES / 1048576,
		"on" if recording else "off", "the whole local map" if watch_mode == "valley" else "one view",
	]
	_sync_controls()
	_show_images()

func _index_of_tick(tick: int) -> int:
	for i in frames.size():
		if int(frames[i]["tick"]) == tick:
			return i
	return -1

func select_frame(index: int) -> void:
	if frames.is_empty():
		return
	selected = clampi(index, 0, frames.size() - 1)
	selected_tick = int(frames[selected]["tick"])
	playing = false
	follow_latest = selected == frames.size() - 1
	_sync_controls()
	_show_images()

func _sync_controls() -> void:
	if record_button == null:
		return
	record_button.text = "Stop recording" if recording else "Start recording"
	play_button.text = "Pause review" if playing else "Play review"
	play_button.disabled = frames.size() < 2
	# FIXLOG 2026-10-31 #484 (c146 verification): Godot formatting rejects %g.
	speed_button.text = "%s×" % playback_speed
	previous_button.disabled = selected <= 0
	next_button.disabled = selected < 0 or selected >= frames.size() - 1
	pin_button.disabled = selected < 0
	watch_button.disabled = not recording
	valley_button.disabled = not recording
	slider.max_value = maxi(0, frames.size() - 1)
	slider.set_value_no_signal(maxi(0, selected))

func set_watch_mode(mode: String) -> void:
	watch_mode = mode

## The record between the reference and the selected sample, from each sample's stored event
## markers (what the viewer had been told of; nothing is looked up or made up here).
func events_text() -> String:
	if selected < 0 or frames.is_empty():
		return ""
	var lo := mini(reference, selected)
	var hi := maxi(reference, selected)
	var lines: Array = []
	var counted := 0
	for i in range(lo + 1 if lo != hi else hi, hi + 1):
		var ev: Dictionary = frames[i].get("events", {})
		for line in ev.get("lines", []):
			counted += 1
			if lines.size() < 12:
				lines.append(str(line))
	var head := "Between the two samples" if lo != hi else "Since the sample before"
	if counted == 0:
		return "[color=#9a9a9a]%s: nothing of note in the record you were told of.[/color]" % head
	var more := "" if counted <= lines.size() else "\n[color=#9a9a9a]and %d more in the history view [H][/color]" % (counted - lines.size())
	return "[b]%s[/b] (%d noted):\n%s%s" % [head, counted, "\n".join(lines), more]

static func marker_weight(frame: Dictionary) -> int:
	return (frame.get("events", {}) as Dictionary).get("lines", []).size()

func _draw_markers() -> void:
	var n := frames.size()
	var area := markers.size
	markers.draw_rect(Rect2(Vector2.ZERO, area), Color(1, 1, 1, 0.05))
	if n == 0:
		return
	var step := area.x / float(n)
	for i in n:
		var wgt := marker_weight(frames[i])
		var x := (i + 0.5) * step
		var hgt := 3.0 if wgt == 0 else minf(area.y, 5.0 + 2.0 * wgt)
		var col := Color(0.55, 0.55, 0.6, 0.6) if wgt == 0 else Color(0.95, 0.75, 0.35, 0.9)
		if i == selected:
			col = Color(1, 1, 1, 1)
		elif i == reference:
			col = Color(0.5, 0.8, 1.0, 1.0)
		markers.draw_line(Vector2(x, area.y), Vector2(x, area.y - hgt), col, maxf(1.0, minf(3.0, step - 1.0)))

func _markers_input(e: InputEvent) -> void:
	if e is InputEventMouseButton and e.pressed and e.button_index == MOUSE_BUTTON_LEFT and not frames.is_empty():
		select_frame(int(e.position.x / maxf(1.0, markers.size.x) * frames.size()))

func _show_images() -> void:
	if markers != null:
		markers.queue_redraw()
	if events_label != null:
		events_label.text = events_text()
	if selected < 0 or frames.is_empty():
		reference_image.texture = null
		selected_image.texture = null
		reference_label.text = "No reference yet"
		selected_label.text = "Start recording to sample this view now and once each game day."
		return
	_show_frame(reference_image, reference_label, reference, "Reference")
	_show_frame(selected_image, selected_label, selected, "Selected")

func _show_frame(target: TextureRect, label: Label, index: int, prefix: String) -> void:
	var frame: Dictionary = frames[index]
	var image := Image.load_from_file(ProjectSettings.globalize_path(str(frame["path"])))
	if image == null or image.is_empty():
		target.texture = null
		label.text = "%s: image unavailable at tick %d" % [prefix, int(frame["tick"])]
		return
	target.texture = ImageTexture.create_from_image(image)
	var filter := "community-known" if bool(frame.get("known_only", false)) else "all visible"
	var camera: Dictionary = frame.get("camera", {})
	var where := "whole local map" if str(camera.get("mode", "view")) == "valley" else "view"
	var replay := ""
	match str(frame.get("replay", "")):
		"confirmed":
			replay = " · re-run matched" if bool(frame.get("replay_png_match", false)) else " · re-run same state"
	label.text = "%s · %s · tick %d · seed %d\n%s · z %d · %.2f× · %s · state %s…%s" % [
		prefix, frame.get("calendar", ""), int(frame["tick"]), int(frame["seed"]),
		where, int(camera.get("z", 0)), float(camera.get("zoom", 1.0)), filter,
		str(frame.get("state_digest", "")).substr(0, 12), replay,
	]

func _process(delta: float) -> void:
	if not visible or not playing or frames.size() < 2:
		return
	playback_accum += delta
	if playback_accum < 1.0 / playback_speed:
		return
	playback_accum = 0.0
	selected = mini(selected + 1, frames.size() - 1)
	selected_tick = int(frames[selected]["tick"])
	if selected == frames.size() - 1:
		playing = false
		follow_latest = true
	_sync_controls()
	_show_images()
