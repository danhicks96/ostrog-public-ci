extends Control
## Render the observed route cells. The simulation owns the resources and their depletion.
var main
var people := {}
var flora := {}
var occupants := Node2D.new()
var road := 0
var tiles := PackedInt32Array()
var zoom := 1
var textures := {}
var last_party := []
var last_focus := {}

func _ready() -> void:
	occupants.y_sort_enabled = true
	add_child(occupants)
	resized.connect(_reframe)

func _reframe() -> void:
	if visible and not last_focus.is_empty():
		observe(last_party, last_focus)

func change_zoom(step: int) -> void:
	zoom = clampi(zoom + step, 1, 3)
	if not last_focus.is_empty():
		observe(last_party, last_focus)

const GROUND := ["grass","packed_earth","mossy","scree","water","mud","wildflower"]
const FLORA := ["","birch_2","pine_2","oak_2","fern_2","berry_bush_bare","root_crops_1"]

func _texture(path: String) -> Texture2D:
	if not textures.has(path):
		textures[path] = main.view._tex(path)
	return textures[path]


func observe(party: Array, focus: Dictionary) -> void:
	last_party = party
	last_focus = focus
	road = int(focus.get("road",0))
	var travelling: bool = focus.stage in ["Outbound","Returning"]

	var keep := {}
	var labels: Array[Rect2] = []
	party.sort_custom(func(a,b): return a.id < b.id)
	for i in party.size():
		var row: Dictionary = party[i]
		keep[row.id] = true
		if not people.has(row.id):
			var node: Node2D = main.view.entities._new_person(occupants)
			var tag := Label.new()
			tag.name = "TravelName"
			tag.add_theme_font_size_override("font_size", 14)
			tag.add_theme_constant_override("outline_size", 3)
			tag.position = Vector2(0, 2)
			node.add_child(tag)
			people[row.id] = node
		var p: Node2D = people[row.id]
		main.view.entities.dress_person(p,row.appearance,0,-1)
		var tag: Label = p.get_node("TravelName")
		tag.text = row.name
		tag.scale = Vector2.ONE / zoom
		tag.position = Vector2(16 - tag.get_minimum_size().x / (2 * zoom), 3)
		p.scale = Vector2.ONE * zoom
		var offset_x := (i - (party.size()-1) / 2.0) * 44
		var terrain_x := floori((road+offset_x)/32.0)
		var floor_y: int = focus.get("trail_y",0)
		if tiles.size() >= 4 and terrain_x >= tiles[0] and terrain_x < tiles[0]+tiles[2]:
			for iy in tiles[3]:
				var code := tiles[4+iy*tiles[2]+terrain_x-tiles[0]]
				if code >= 0 and (code & 15) == 1:
					floor_y = iy+tiles[1]
					break
		p.position = size/2 + Vector2(offset_x-16,8+32*(floor_y-int(focus.get("trail_y",0)))) * zoom
		# Labels stay screen-sized at all three zoom settings. Move a crowded label
		# below earlier labels while keeping it centred under its own traveller.
		var label_box := Rect2(p.position + tag.position * zoom, tag.get_minimum_size()).grow(2)
		for _attempt in labels.size():
			var bottom := label_box.position.y
			for placed in labels:
				if label_box.intersects(placed):
					bottom = maxf(bottom, placed.end.y)
			if bottom == label_box.position.y:
				break
			tag.position.y += (bottom - label_box.position.y) / zoom
			label_box.position.y = bottom
		labels.append(label_box)
		var frame: int = main.view.entities.P_WALK + (main.sim.tick() / 3 + i) % 6 if travelling else 0
		var facing := 1 if focus.stage == "Returning" else 2
		for key in ["body","cloth","hair","band","equip"]:
			var layer: Sprite2D = p.get_node(key)
			if layer.texture:
				var cell := layer.texture.get_height() / 4
				layer.region_rect = Rect2(frame*cell,facing*cell,cell,cell)
		p.modulate = Color.WHITE if row.id == focus.id else Color(0.88,0.88,0.88)
	for id in people.keys():
		if not keep.has(id):
			people[id].queue_free()
			people.erase(id)
	_place_flora()
	queue_redraw()

func _draw() -> void:
	draw_rect(Rect2(Vector2.ZERO,size), Color("#131b1f"))
	if tiles.size() < 4:
		return
	var tile := 32 * zoom
	var centre_y: int = int(last_focus.get("trail_y",0)) * 32 + 16
	var origin := size / 2 - Vector2(road,centre_y) * zoom
	for iy in tiles[3]:
		for ix in tiles[2]:
			var code := tiles[4+iy*tiles[2]+ix]
			if code < 0:
				continue
			var x := tiles[0]+ix
			var y := tiles[1]+iy
			var pos := origin + Vector2(x,y) * tile
			if not Rect2(-tile*2,-tile*2,size.x+tile*4,size.y+tile*4).has_point(pos):
				continue
			var dim := Color("#6c7175") if code & 65536 else Color.WHITE
			var season := (code>>12)&3
			var family: String = GROUND[code & 15]
			if family == "grass":
				if season == 2: family = "grass_autumn"
				if season == 3: family = "snow"
			var tex := _texture("terrain/%s.png" % family)
			if tex != null:
				var column := posmod(hash(Vector2i(x,y)),maxi(1,tex.get_width()/32))
				draw_texture_rect_region(tex,Rect2(pos,Vector2.ONE*tile),Rect2(32*column,0,32,32),dim)
			var neighbours := [Vector2i.UP,Vector2i.RIGHT,Vector2i.DOWN,Vector2i.LEFT]
			for edge in 4:
				var next := Vector2i(ix,iy) + Vector2i(neighbours[edge])
				if next.x < 0 or next.x >= tiles[2] or next.y < 0 or next.y >= tiles[3]:
					continue
				var other := tiles[4+next.y*tiles[2]+next.x]
				if other < 0 or (other&15) == (code&15):
					continue
				var other_family: String = GROUND[other&15]
				if main.view.BLEND_ORDER.find(other_family) <= main.view.BLEND_ORDER.find(GROUND[code&15]):
					continue
				if other_family == "grass" and season == 2: other_family = "grass_autumn"
				var mask := _texture("terrain/edges/%s.png" % other_family)
				if mask:
					var v := posmod(hash(Vector2i(x,y)),4)
					draw_texture_rect_region(mask,Rect2(pos,Vector2.ONE*tile),Rect2(edge*32,v*32,32,32),dim)
	var light: float = main.view.entities.sun
	var lean := clampf((float(main.view.entities.hour)-13.0)/6.0,-1,1)
	for sprite: Sprite2D in flora.values():
		var tex := sprite.texture
		var foot := sprite.position
		var reach := 0.26+0.3*absf(lean)
		# Project the canopy silhouette from its trunk onto the ground.
		var transform := Transform2D(Vector2(zoom,0),Vector2(lean*zoom*0.6,-reach*zoom),foot)
		draw_set_transform_matrix(transform)
		draw_texture(tex,Vector2(-tex.get_width()*0.5,-tex.get_height()),Color(0.02,0.03,0.04,0.27*light))
		draw_set_transform_matrix(Transform2D.IDENTITY)
	for person: Node2D in people.values():
		var foot := person.position+Vector2(16,-2)*zoom
		draw_texture_rect(main.view.entities.blob,Rect2(foot-Vector2(10,3)*zoom,Vector2(20,7)*zoom),false,Color(0,0,0,0.45))

func _place_flora() -> void:
	var keep := {}
	if tiles.size() >= 4:
		var tile := 32*zoom
		var origin := size/2-Vector2(road,int(last_focus.get("trail_y",0))*32+16)*zoom
		for i in range(4,tiles.size()):
			var code := tiles[i]
			var kind := (code>>4)&15
			if code < 0 or kind == 0:
				continue
			var cell := Vector2i(tiles[0]+(i-4)%tiles[2],tiles[1]+(i-4)/tiles[2])
			var foot := origin+(Vector2(cell)+Vector2(0.5,1))*tile
			if not Rect2(-tile*2,-tile*2,size.x+tile*4,size.y+tile*4).has_point(foot):
				continue
			var season := (code>>12)&3
			var name: String = FLORA[kind]
			var has_stock := ((code>>8)&15)>0
			if kind == 5 and season == 1 and has_stock: name = "berry_bush_fruit"
			if kind == 6 and season == 2 and has_stock: name = "root_crops_2"
			if kind == 6 and not has_stock: name = "root_crops_0"
			if season == 3 and name != "pine_2": name += "_winter"
			var tex := _texture("plants/%s.png" % name)
			if not tex:
				continue
			keep[cell] = true
			if not flora.has(cell):
				var plant := Sprite2D.new()
				plant.centered = false
				occupants.add_child(plant)
				flora[cell] = plant
			var plant: Sprite2D = flora[cell]
			plant.texture = tex
			plant.offset = Vector2(-tex.get_width()*0.5,-tex.get_height())
			plant.position = foot
			plant.scale = Vector2.ONE*zoom
			plant.modulate = Color("#6c7175") if code&65536 else Color.WHITE
	for cell in flora.keys():
		if not keep.has(cell):
			flora[cell].queue_free()
			flora.erase(cell)
