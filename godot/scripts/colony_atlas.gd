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

# Only the two facts about land recorded by the survey are charted: biome and settlement.
const TERRAIN_COLOUR := {
	"ocean": "#44708c", "lake": "#6396ac", "ice": "#c0d4dc", "tundra": "#a49d89",
	"taiga": "#557b70", "temperate forest": "#698553", "grassland": "#a7ad74",
	"steppe": "#ab9a6a", "desert": "#d4b984", "savanna": "#b9a661",
	"rainforest": "#477562", "swamp": "#848967", "mountains": "#96969c"
}
const PARTY_INK := Color("#f4c879")
const MAP_FIELD := Color("#162027")
var parties := {}
var away_members := {}
var party_hits := {}

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
	map.resized.connect(ink.queue_redraw)
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
		parties.clear()
		away_members.clear()
		for member in rows.values():
			if not member.away:
				continue
			var region := _region(member)
			if region < 0 or region >= side * side:
				continue
			if not member.has("caravan"):
				away_members[member.id] = region
				continue
			if not parties.has(member.caravan):
				parties[member.caravan] = {"region": region, "member": member.id, "count": 0}
			var party: Dictionary = parties[member.caravan]
			party.count += 1
			party.member = mini(party.member, member.id)
		ink.queue_redraw()
		detail.text = region_note if region_note != "" else "Select a caravan to watch it, or a surveyed hex to inspect it. Dim hexes show the last observation."
		if not row.is_empty():
			detail.text = "%s · %s" % [row.name, row.get("line","At home")]
			action.text = "Watch caravan" if row.away else "Follow in %s" % main.sim.world_name()
		elif roster.item_count == 0 and chart.has(region_filter):
			detail.text += " · No known members here."

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
	ink.draw_rect(Rect2(Vector2.ZERO,map.size), MAP_FIELD)
	if chart.is_empty():
		return
	var bounds := Rect2(_hex_xy(chart.keys()[0]),Vector2.ZERO)
	for region in chart:
		bounds = bounds.expand(_hex_xy(region))
	for party in parties.values():
		bounds = bounds.expand(_hex_xy(party.region))
	for region in away_members.values():
		bounds = bounds.expand(_hex_xy(region))
	var selected_member: Dictionary = rows.get(selected, {})
	if selected_member.has("caravan") and selected_member.has("region"):
		bounds = bounds.expand(_hex_xy(selected_member.region))
	view_centre = bounds.get_center()
	hex_size = minf(42,minf((map.size.x-48)/(bounds.size.x+3),(map.size.y-100)/(bounds.size.y+3)))
	var biomes := {}
	var has_memory := false
	for region in chart:
		var cell: Dictionary = chart[region]
		var terrain: String = cell.terrain
		biomes[terrain] = true
		var tint := Color(str(TERRAIN_COLOUR.get(terrain, "#959394")))
		var centre := _map_point(region)
		var corners := _hex_corners(centre, hex_size + 0.1)
		if not cell.live:
			has_memory = true
			tint = tint.lerp(MAP_FIELD, 0.62)
		ink.draw_colored_polygon(corners, tint)
		if hex_size >= 9:
			corners.append(corners[0])
			ink.draw_polyline(corners, tint.darkened(0.12), 1.0, true)
			_draw_terrain_marks(terrain, region, centre, tint)
		if cell.settlement != "":
			var size := clampf(hex_size * 0.24, 3, 8)
			var beacon := Color("#f1e0be") if cell.live else Color("#89918c")
			var town := centre + Vector2(-0.30, 0.23) * hex_size
			ink.draw_rect(Rect2(town + Vector2(-0.62,-0.14) * size, Vector2(1.24,0.88) * size), beacon)
			ink.draw_colored_polygon(PackedVector2Array([town + Vector2(-size, 0), town + Vector2(0, -size), town + Vector2(size, 0)]), beacon)
			ink.draw_rect(Rect2(town + Vector2(-0.18,0.12) * size, Vector2(0.36,0.62) * size), MAP_FIELD)
	if chart.has(region_filter):
		var outline := _hex_corners(_map_point(region_filter), hex_size - 1)
		outline.append(outline[0])
		ink.draw_polyline(outline, Color("#f0e4c5"), 2.0, true)
	_draw_terrain_key(biomes, has_memory)

func _hex_corners(centre: Vector2, radius: float) -> PackedVector2Array:
	var corners := PackedVector2Array()
	for i in 6:
		corners.append(centre + Vector2.from_angle((i + 0.5) * TAU / 6.0) * radius)
	return corners

# Biome symbols break up a broad patch of a single surveyed biome. Spacing is cartographic;
# the report contains neither individual trees nor their density.
func _draw_terrain_marks(terrain: String, region: int, centre: Vector2, tint: Color) -> void:
	var pattern := posmod(hash(Vector2i(region % side, region / side)), 137)
	if pattern % 5 == 0:
		return
	var shade := tint.lerp(MAP_FIELD, 0.52)
	var count := 1 if hex_size < 18 else 1 + pattern % 3
	for i in count:
		var angle := float((pattern * 83 + i * 137) % 360) * PI / 180.0
		var pos := centre + Vector2.from_angle(angle) * hex_size * (0.15 if count == 1 else 0.43)
		var scale := hex_size * (0.19 + float((pattern + i) % 4) * 0.023)
		_biome_symbol(terrain, pos, scale, shade)

func _biome_symbol(biome: String, at: Vector2, s: float, tint: Color) -> void:
	var width := clampf(s * 0.18, 1.0, 2.0)
	match biome:
		"taiga":
			_pine(at, s, tint)
		"temperate forest", "rainforest", "savanna":
			ink.draw_line(at, at + Vector2.DOWN * s, tint, width)
			var canopy := PackedVector2Array()
			for i in 10:
				var radius := Vector2(1.0, 0.42) if biome == "savanna" else Vector2(0.79, 0.68)
				canopy.append(at + (Vector2.from_angle(i * TAU / 10) * radius + Vector2(0,-0.14)) * s)
			ink.draw_colored_polygon(canopy, tint)
		"swamp", "grassland", "steppe", "tundra":
			var base := at + Vector2.DOWN * s * 0.5
			for blade in [-1, 0, 1]:
				var reach := Vector2(blade * 0.73, -1.26 if blade == 0 else -0.78)
				var stem: Vector2 = base + Vector2.RIGHT * blade * s * 0.17
				ink.draw_polyline(PackedVector2Array([stem, stem + Vector2(reach.x * 0.38, reach.y * 0.72) * s, stem + reach * s]), tint, width, true)
			if biome == "swamp":
				ink.draw_line(at + Vector2(-1.08,0.69) * s, at + Vector2(-0.22,0.69) * s, tint, width, true)
				ink.draw_line(at + Vector2(0.1,0.91) * s, at + Vector2(1.0,0.91) * s, tint, width, true)
		"ocean", "lake", "desert":
			for row in (1 if biome == "desert" else 2):
				var line := PackedVector2Array()
				for i in 9:
					line.append(at + Vector2((i - 4) * 0.26, row * 0.58 - cos(i * PI / 4) * 0.23) * s)
				ink.draw_polyline(line, tint, width, true)
		"mountains", "ice":
			ink.draw_polyline(PackedVector2Array([at + Vector2(-1, 0.5) * s, at + Vector2(-0.3, -0.7) * s, at + Vector2(0.25, 0.05) * s, at + Vector2(0.55,-0.4) * s, at + Vector2(1,0.5) * s]), tint, width, true)

func _pine(at: Vector2, scale: float, tint: Color) -> void:
	var tree := PackedVector2Array()
	for vertex in [
		Vector2(0,-1.2), Vector2(0.55,-0.32), Vector2(0.28,-0.35), Vector2(0.86,0.53),
		Vector2(0.12,0.47), Vector2(0.12,0.88), Vector2(-0.12,0.88), Vector2(-0.12,0.47),
		Vector2(-0.86,0.53), Vector2(-0.28,-0.35), Vector2(-0.55,-0.32)
	]:
		tree.append(at + vertex * scale)
	ink.draw_colored_polygon(tree, tint)
	ink.draw_colored_polygon(PackedVector2Array([
		at + Vector2(0,-1.1) * scale, at + Vector2(-0.78,0.49) * scale, at + Vector2(0,0.19) * scale
	]), tint.lightened(0.12))

func _draw_terrain_key(biomes: Dictionary, has_memory: bool) -> void:
	var font := ink.get_theme_default_font()
	var pos := Vector2(12, 12)
	var names := biomes.keys()
	names.sort()
	for name in names:
		var caption := str(name).capitalize()
		var width := font.get_string_size(caption, HORIZONTAL_ALIGNMENT_LEFT, -1, 14).x + 43
		if pos.x + width > map.size.x - 10 and pos.x > 12:
			pos = Vector2(12, pos.y + 27)
		ink.draw_rect(Rect2(pos, Vector2(width - 5, 25)), Color("#23313a"))
		var tint := Color(str(TERRAIN_COLOUR.get(name, "#959394")))
		_biome_symbol(name, pos + Vector2(13, 12), 7, tint.lightened(0.34))
		ink.draw_string(font, pos + Vector2(26, 17), caption, HORIZONTAL_ALIGNMENT_LEFT, -1, 14, Color("#dce3de"))
		pos.x += width
	if has_memory:
		var note := "Dim · last seen"
		var width := font.get_string_size(note, HORIZONTAL_ALIGNMENT_LEFT, -1, 14).x + 30
		if pos.x + width > map.size.x - 10:
			pos = Vector2(12, pos.y + 27)
		ink.draw_rect(Rect2(pos, Vector2(width, 25)), Color("#23313a"))
		ink.draw_circle(pos + Vector2(12, 12), 5, Color("#526164"))
		ink.draw_string(font, pos + Vector2(23,17), note, HORIZONTAL_ALIGNMENT_LEFT, -1, 14, Color("#c1ccca"))

func _route_label(caption: String, color: Color, at: Vector2, member: int) -> void:
	var font := ink.get_theme_default_font()
	var width := font.get_string_size(caption,HORIZONTAL_ALIGNMENT_LEFT,-1,14).x
	at.x = clampf(at.x, 3, maxf(3, map.size.x - width - 3))
	callouts[Rect2(at,Vector2(width,20))] = member
	ink.draw_string(font,at + Vector2(0,15),caption,HORIZONTAL_ALIGNMENT_LEFT,-1,14,color)

func _draw_route() -> void:
	callouts.clear()
	party_hits.clear()
	_draw_chart()
	if chart.is_empty() or home < 0 or home >= side * side:
		return
	var start := _map_point(home)
	var crew: Dictionary = rows.get(selected,{})
	if crew.has("caravan") and crew.has("region"):
		_draw_selected_route(crew, start)
	if chart.has(home):
		var home_live: bool = chart[home].live
		var home_ink := Color("#e8eee8") if home_live else Color("#9baba9")
		ink.draw_circle(start, 9, MAP_FIELD)
		ink.draw_arc(start, 7, 0, TAU, 24, home_ink, 2.0, true)
		_map_badge("Home" if home_live else "Home · last seen", start, home_ink, -1)
	for member_id in away_members:
		var point := _map_point(away_members[member_id])
		var accent := Color("#babde4")
		ink.draw_circle(point, 6, accent)
		callouts[Rect2(point - Vector2.ONE * 8, Vector2.ONE * 16)] = member_id
		_map_badge(rows[member_id].name, point, accent, member_id)
	var ids := parties.keys()
	ids.sort()
	for id in ids:
		var party: Dictionary = parties[id]
		var point := _map_point(party.region)
		ink.draw_circle(point, 12, MAP_FIELD)
		ink.draw_arc(point, 10, 0, TAU, 28, PARTY_INK, 2, true)
		ink.draw_line(point + Vector2(-2, 5), point + Vector2(-2, -5), PARTY_INK, 2, true)
		ink.draw_colored_polygon(PackedVector2Array([point + Vector2(-2,-6), point + Vector2(6,-3), point + Vector2(-2,0)]), PARTY_INK)
		var hit := Rect2(point - Vector2.ONE * 12, Vector2.ONE * 24)
		if not party_hits.has(hit):
			party_hits[hit] = party.member
		var member: Dictionary = rows[party.member]
		var title := "Caravan"
		if member.get("stage", "") == "Outbound" and member.get("place", "") != "":
			title = "Caravan to %s" % member.place
		elif member.get("stage", "") == "Returning":
			title = "Returning caravan"
		var label := "%s · %d %s" % [title, party.count, "traveller" if party.count == 1 else "travellers"]
		_map_badge(label, point, PARTY_INK, party.member)

# A pin and its caption are both targets. Put coincident parties on separate lines.
func _map_badge(caption: String, point: Vector2, colour: Color, member: int) -> void:
	var font := ink.get_theme_default_font()
	var width := font.get_string_size(caption, HORIZONTAL_ALIGNMENT_LEFT, -1, 16).x + 20
	var x := point.x + 16 if point.x + 16 + width <= map.size.x - 4 else point.x - width - 16
	var box := Rect2(Vector2(clampf(x, 4, maxf(4, map.size.x - width - 4)), clampf(point.y - 14, 4, maxf(4, map.size.y - 32))), Vector2(width, 28))
	for attempt in callouts.size() + party_hits.size() + 1:
		var overlaps := false
		for used in callouts.keys() + party_hits.keys():
			if box.intersects(used):
				overlaps = true
				break
		if not overlaps:
			break
		box.position.y += 30
		if box.end.y > map.size.y - 4:
			box.position.y = 4
	var edge := Vector2(clampf(point.x, box.position.x, box.end.x), clampf(point.y, box.position.y, box.end.y))
	ink.draw_line(point + point.direction_to(edge) * 10, edge, colour, 1.0, true)
	ink.draw_rect(box, Color("#1d282d"))
	ink.draw_rect(box, colour.darkened(0.30), false, 1.0)
	ink.draw_string(font, box.position + Vector2(10, 20), caption, HORIZONTAL_ALIGNMENT_LEFT, -1, 16, colour)
	if member >= 0 and rows[member].has("caravan"):
		party_hits[box] = member
	else:
		callouts[box] = member

func _draw_selected_route(crew: Dictionary, start: Vector2) -> void:
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
	for box in party_hits:
		if box.has_point(event.position):
			selected = party_hits[box]
			region_filter = -1
			signature.clear()
			watching = true
			region_note = ""
			_refresh()
			return
	for box in callouts:
		if box.has_point(event.position):
			selected = callouts[box]
			region_note = ""
			region_filter = home if selected < 0 else -1
			signature.clear()
			_refresh()
			return
	var hit := _region_at(event.position)
	selected = -1
	roster.deselect_all()
	region_filter = hit if hit >= 0 else -2
	if hit < 0:
		region_note = "Unobserved territory."
	else:
		var cell: Dictionary = chart[hit]
		var age: int = maxi(0,main.sim.tick()-int(cell.seen))
		var observation := "In sight"
		if not cell.live:
			var hours := age / 1200
			observation = "Last seen less than an hour ago" if hours == 0 else "Last seen %d %s ago" % [hours, "hour" if hours == 1 else "hours"]
		region_note = "%s · %s" % [cell.terrain.capitalize(), observation]
		if cell.settlement != "":
			region_note += " · " + str(cell.settlement)
	signature.clear()
	_refresh()

## The corners outside a pointy hex belong to the neighbouring cell, which may be unknown.
func _region_at(point: Vector2) -> int:
	var hit := -1
	var nearest := hex_size * hex_size
	for region in chart:
		var delta := _map_point(region) - point
		if not inside_hex(delta, hex_size):
			continue
		var distance := delta.length_squared()
		if distance <= nearest:
			nearest = distance
			hit = region
	return hit

static func inside_hex(offset: Vector2, radius: float) -> bool:
	var d := offset.abs()
	return d.x <= sqrt(3.0) * radius / 2 and d.x + sqrt(3.0) * d.y <= sqrt(3.0) * radius
