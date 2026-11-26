extends Node
## Present-day colony roster and read-only expedition camera.

const Road := preload("res://scripts/caravan_road.gd")
@onready var main = get_parent()
var bridge := ColonyAtlas.new()
var panel := PanelContainer.new()
var colony_name := Label.new()
var roster := ItemList.new()
var map := TextureRect.new()
var ink := Control.new()
var callouts := {}
var road := Road.new()
var detail := Label.new()
var action := Button.new()
var overview := Button.new()
var zoom_controls := HBoxContainer.new()
var community := -1
var selected := -1
var watching := false
var region_filter := -1
var rows := {}
var signature := []
var valley_follow := -1
var elapsed := 0.0
var side := 0
var chart := {}
var survey_tick := -1
var region_note := ""
var view_centre := Vector2.ZERO
var hex_size := 14.0
var home := -1

var markers_shown := {}

func _install() -> void:
	for source in [main.people_panel, main.legends_panel]:
		var launch := Button.new()
		launch.text = "Colony map"
		source.get_child(0).get_child(0).add_child(launch)
		launch.pressed.connect(open_map)
	var overlay := CanvasLayer.new()
	overlay.layer = 4
	add_child(overlay)
	overlay.add_child(panel)
	panel.hide()
	panel.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	panel.offset_left = 16
	panel.offset_right = -16
	panel.offset_top = 16
	panel.offset_bottom = -16
	var background: StyleBoxFlat = main._panel_bg()
	background.bg_color = Color(0.08,0.09,0.09)
	panel.add_theme_stylebox_override("panel", background)
	main.panels.register(&"colony_map", panel, &"reading")
	var outer := VBoxContainer.new()
	panel.add_child(outer)
	var header := HBoxContainer.new()
	outer.add_child(header)
	header.add_child(colony_name)
	header.add_child(zoom_controls)
	for step in [-1, 1]:
		var zoom_button := Button.new()
		zoom_button.text = "Zoom out" if step < 0 else "Zoom in"
		zoom_button.pressed.connect(road.change_zoom.bind(step))
		zoom_controls.add_child(zoom_button)
	header.add_child(overview)
	overview.text = "All members"
	overview.pressed.connect(_reset)
	var close := Button.new()
	close.text = "Close map"
	header.add_child(close)
	close.pressed.connect(func(): main.panels.close(&"colony_map"))
	var body := HBoxContainer.new()
	body.size_flags_vertical = Control.SIZE_EXPAND_FILL
	outer.add_child(body)
	body.add_child(roster)
	roster.custom_minimum_size.x = 230
	roster.item_selected.connect(_select)
	roster.item_activated.connect(func(i): _select(i); _focus())
	var scene := VBoxContainer.new()
	scene.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	body.add_child(scene)
	scene.add_child(map)
	map.size_flags_vertical = Control.SIZE_EXPAND_FILL
	map.custom_minimum_size.y = 320
	map.stretch_mode = TextureRect.STRETCH_KEEP_ASPECT_CENTERED
	map.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
	map.texture_filter = CanvasItem.TEXTURE_FILTER_NEAREST
	map.gui_input.connect(_map_click)
	map.add_child(ink)
	ink.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	map.clip_contents = true
	ink.mouse_filter = Control.MOUSE_FILTER_IGNORE
	ink.draw.connect(_draw_route)
	scene.add_child(road)
	road.main = main
	road.size_flags_vertical = Control.SIZE_EXPAND_FILL
	road.custom_minimum_size.y = 320
	road.clip_contents = true
	road.hide()
	scene.add_child(detail)
	detail.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	scene.add_child(action)
	action.pressed.connect(_focus)

func _unhandled_key_input(event: InputEvent) -> void:
	if event is InputEventKey and event.pressed and not event.echo and event.keycode == KEY_M and panel.is_inside_tree():
		get_viewport().set_input_as_handled()
		if panel.visible and not watching:
			main.panels.close(&"colony_map")
		else:
			open_map()

func open_map() -> void:
	valley_follow = -1
	main.panels.open(&"colony_map")
	_reset()

func _reset() -> void:
	watching = false
	selected = -1
	region_filter = -1
	region_note = ""
	signature.clear()
	_refresh()

func _process(delta: float) -> void:
	if not panel.is_inside_tree():
		if is_instance_valid(main.legends_panel):
			_install()
		return
	elapsed += delta
	if panel.is_visible_in_tree():
		if elapsed >= 0.2:
			elapsed = 0
			_refresh()
	elif valley_follow >= 0 and elapsed >= 1.0:
		elapsed = 0
		if main.follow_id != valley_follow:
			valley_follow = -1
			return
		var snapshot: Dictionary = bridge.snapshot(main.sim, community)
		for row in snapshot.get("members", []):
			if row.id == valley_follow:
				if row.has("caravan"):
					open_map()
					selected = row.id
					watching = true
					main.follow_id = -1
					valley_follow = -1
					_refresh()
				return
		valley_follow = -1

func _region(row: Dictionary) -> int:
	if not row.away:
		return home
	var target := int(row.get("region", -1))
	if side <= 0 or home < 0 or target < 0 or not row.has("fraction"):
		return target
	var start := Vector2(home % side, home / side)
	var end := Vector2(target % side, target / side)
	var cell := Vector2i(start.lerp(end, float(row.fraction)).round())
	return cell.y * side + cell.x

func _refresh() -> void:
	var data: Dictionary = bridge.snapshot(main.sim, community)
	side = data.get("side",0)
	chart = data.get("chart",{})
	survey_tick = data.get("survey_tick",-1)
	community = data.get("community",-1)
	home = data.get("home",-1)
	colony_name.text = data.get("colony_name","")
	rows.clear()
	var list := []
	for row in data.get("members",[]):
		rows[row.id] = row
		if region_filter == -1 or _region(row) == region_filter:
			list.append([str(row.name), int(row.id), bool(row.away)])
	list.sort_custom(func(a,b): return a[0] < b[0] or (a[0] == b[0] and a[1] < b[1]))
	if list != signature or roster.item_count != list.size():
		signature = list
		roster.clear()
		for entry in list:
			var i := roster.add_item("%s  ·  %s" % [entry[0], "Away" if entry[2] else "Home"])
			roster.set_item_metadata(i,entry[1])
			if entry[1] == selected:
				roster.select(i)
	if not rows.has(selected):
		selected = -1
		watching = false
	if watching and not rows[selected].away:
		_focus()
		return
	if watching and not rows[selected].has("caravan"):
		watching = false
	_render()

func _select(i: int) -> void:
	region_note = ""
	roster.select(i)
	selected = roster.get_item_metadata(i)
	watching = watching and rows[selected].has("caravan")
	_render()

func _render() -> void:
	var row: Dictionary = rows.get(selected,{})
	map.visible = not watching
	road.visible = watching
	zoom_controls.visible = watching
	overview.disabled = not watching and region_filter < 0 and selected < 0
	action.visible = not watching and (row.has("tile") or row.has("caravan"))
	if watching:
		var party := []
		for member in rows.values():
			if member.get("caravan",-1) == row.caravan:
				party.append(member)
		road.tiles = bridge.level_tiles(main.sim, community, selected)
		road.observe(party, row)
		detail.text = "%s  ·  %s, day %d  ·  %d rations\n%s" % [row.name,row.stage,row.day,row.rations,row.line]
		detail.text += "   ·   Surface   ·   Cargo: " + str(row.get("cargo","") if row.get("cargo","") != "" else "empty")
	else:
		var markers := {}
		for member in rows.values():
			var r := _region(member)
			if r >= 0 and r < side * side:
				markers[r] = Color.GOLD if r != home else Color.WHITE
		markers_shown = markers
		ink.queue_redraw()
		detail.text = region_note if region_note != "" else "Select a hex. Gold marks members away from home. Dim terrain shows the last observation."
		if not row.is_empty():
			detail.text = "%s · %s" % [row.name, row.get("line","At home")]
			action.text = "Watch caravan" if row.away else "Follow in %s" % main.sim.world_name()
		elif roster.item_count == 0 and region_note == "":
			detail.text = "No known members here."

func _focus() -> void:
	var row: Dictionary = rows.get(selected,{})
	if row.has("tile"):
		var tile: Vector3i = row.tile
		main.view.view_z = tile.z
		main.view.refresh()
		main.cam.position = Vector2(tile.x * 32 + 16, tile.y * 32 + 16)
		main.selected_id = selected
		main.follow_id = selected
		valley_follow = selected
		main.last_input_time = Time.get_ticks_msec() / 1000.0
		main.panels.close(&"colony_map")
	elif row.has("caravan"):
		watching = true
		_render()

func _hex_xy(region: int) -> Vector2:
	var y := region / side
	return Vector2((region % side + (y % 2) * 0.5) * sqrt(3.0),y * 1.5)

func _map_point(region: int) -> Vector2:
	return (map.size - Vector2(0,50)) * 0.5 + (_hex_xy(region) - view_centre) * hex_size

func _draw_chart() -> void:
	ink.draw_rect(Rect2(Vector2.ZERO,map.size),Color("#11171c"))
	if chart.is_empty():
		return
	var bounds := Rect2(_hex_xy(chart.keys()[0]),Vector2.ZERO)
	for region in chart:
		bounds = bounds.expand(_hex_xy(region))
	for region in markers_shown:
		bounds = bounds.expand(_hex_xy(region))
	var selected_member: Dictionary = rows.get(selected, {})
	if selected_member.has("caravan") and selected_member.has("region"):
		bounds = bounds.expand(_hex_xy(selected_member.region))
	view_centre = bounds.get_center()
	hex_size = minf(32,minf((map.size.x-48)/(bounds.size.x+3),(map.size.y-100)/(bounds.size.y+3)))
	var palette := {"ocean":"345577","lake":"5489a4","ice":"c4d2d3","tundra":"999a88","taiga":"527161","temperate forest":"54794e","grassland":"9baa6a","steppe":"af9a64","desert":"d1b77f","savanna":"b3a765","rainforest":"36785e","swamp":"60735b","mountains":"8b8990"}
	for region in chart:
		var cell: Dictionary = chart[region]
		var color := Color(str(palette.get(cell.terrain,"8b8990")))
		var centre := _map_point(region)
		var corners := PackedVector2Array()
		for k in 6:
			corners.append(centre + Vector2.from_angle((k+0.5)*TAU/6.0) * (hex_size - 0.5))
		ink.draw_colored_polygon(corners,color if cell.live else color.darkened(0.56))
		if cell.settlement != "":
			ink.draw_rect(Rect2(centre-Vector2.ONE*2,Vector2.ONE*4),Color.BISQUE if cell.live else Color.DIM_GRAY)
func _route_label(caption: String, color: Color, at: Vector2, member: int) -> void:
	var font := ink.get_theme_default_font()
	var width := font.get_string_size(caption,HORIZONTAL_ALIGNMENT_LEFT,-1,14).x
	at.x = clampf(at.x, 3, maxf(3, map.size.x - width - 3))
	callouts[Rect2(at,Vector2(width,20))] = member
	ink.draw_string(font,at + Vector2(0,15),caption,HORIZONTAL_ALIGNMENT_LEFT,-1,14,color)

func _draw_route() -> void:
	callouts.clear()
	_draw_chart()
	if chart.is_empty() or home < 0 or home >= side * side:
		return
	var start := _map_point(home)
	var crew: Dictionary = rows.get(selected,{})
	for region in markers_shown:
		if region != home:
			ink.draw_circle(_map_point(region), 5, markers_shown[region])
	ink.draw_arc(start,6,0,TAU,24,Color.WHITE,2,true)
	_route_label("Home",Color.WHITE,start+Vector2(8,3),-1)
	if not crew.has("caravan") or not crew.has("region"):
		return
	var destination := int(crew.region)
	if destination < 0 or destination >= side * side:
		return
	var end := _map_point(destination)
	var blue := Color.SKY_BLUE
	if start.distance_squared_to(end) > 1:
		ink.draw_line(start,end,Color.BLACK,4,true)
		ink.draw_dashed_line(start,end,blue,2,5,true,true)
	ink.draw_circle(end,4,blue)
	_route_label(crew.place,blue,end+Vector2(7,-21),crew.id)
	# Neighbouring settlements can occupy adjacent world pixels. The route strip
	# beneath the geography remains readable even when both pins share a pixel.
	var width := minf(map.size.x,500.0)
	var left := (map.size.x - width) / 2
	var y := map.size.y - 22
	var box := Rect2(left,y-25,width,47)
	ink.draw_rect(box,Color(0.05,0.07,0.09,0.96))
	var x0 := left + 12
	var x1 := left + width - 12
	var font := ink.get_theme_default_font()
	var target_name := str(crew.place)
	var target_width := font.get_string_size(target_name,HORIZONTAL_ALIGNMENT_LEFT,-1,14).x
	_route_label("Home",Color.WHITE,Vector2(x0,y-23),-1)
	_route_label(target_name,blue,Vector2(x1-target_width,y-23),crew.id)
	_route_label("Estimated progress %d%%" % roundi(100 * crew.fraction),Color.GOLD,Vector2(x0+60,y-23),crew.id)
	var progress := Vector2(lerpf(x0,x1,crew.fraction),y+9)
	ink.draw_line(Vector2(x0,y+9),Vector2(x1,y+9),blue,2,true)
	ink.draw_circle(Vector2(x1,y+9),4,blue)
	ink.draw_arc(Vector2(x0,y+9),6,0,TAU,24,Color.WHITE,2,true)
	ink.draw_circle(progress,4,Color.GOLD)
	callouts[Rect2(progress-Vector2.ONE*7,Vector2.ONE*14)] = crew.id

func _map_click(event: InputEvent) -> void:
	if not (event is InputEventMouseButton and event.pressed and event.button_index == MOUSE_BUTTON_LEFT) or chart.is_empty():
		return
	for box in callouts:
		if box.has_point(event.position):
			selected = callouts[box]
			region_filter = home if selected < 0 else -1
			signature.clear()
			_refresh()
			return
	var hit := -1
	var nearest := hex_size * hex_size
	for region in chart:
		var distance := _map_point(region).distance_squared_to(event.position)
		if distance <= nearest:
			nearest = distance
			hit = region
	selected = -1
	roster.deselect_all()
	region_filter = hit if hit >= 0 else -2
	if hit < 0:
		region_note = "Unobserved territory."
	else:
		var cell: Dictionary = chart[hit]
		var age: int = maxi(0,main.sim.tick()-int(cell.seen))
		region_note = "%s · %s" % [cell.terrain.capitalize(),"In sight" if cell.live else "Last seen %d hours ago" % (age / 1200)]
		if cell.settlement != "":
			region_note += " · " + str(cell.settlement)
		for member in rows.values():
			if member.has("caravan") and _region(member) == hit:
				selected = member.id
				watching = true
				break
	signature.clear()
	_refresh()
