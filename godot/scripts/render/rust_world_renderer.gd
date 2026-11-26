extends Node2D

## WP-06.2/6.3 experimental bridge. Rust builds agent, static-thing and ground rows on rayon
## workers; this node only uploads compact lists to MultiMeshInstance2D batches. It is deliberately
## opt-in until the whole view has the same art coverage and frame evidence as the layered renderer.

const TILE := 32.0
const STRIDE := 8
const GROUND_STRIDE := 5
var view
var animals: MultiMeshInstance2D
var ground: MultiMeshInstance2D
var plants: MultiMeshInstance2D
var structures: MultiMeshInstance2D
var items: MultiMeshInstance2D

func setup(v) -> void:
	view = v
	animals = _batch(v._tex("animals/deer.png"), 2)
	ground = _batch(v._tex("terrain/grass.png"), -3)
	plants = _batch(v._tex("plants/shrub.png"), 2)
	structures = _batch(v._tex("structures/crate.png"), 3)
	items = _batch(v._tex("items/log.png"), 4)
	animals.z_index = 2
	ground.z_index = -3
	plants.z_index = 2
	structures.z_index = 3
	items.z_index = 4
	add_child(animals)
	add_child(ground)
	add_child(plants)
	add_child(structures)
	add_child(items)
	rebuild()

func _batch(tex: Texture2D, z: int) -> MultiMeshInstance2D:
	var n := MultiMeshInstance2D.new()
	n.texture = tex
	n.z_index = z
	var mm := MultiMesh.new()
	mm.transform_format = MultiMesh.TRANSFORM_2D
	mm.use_colors = true
	mm.instance_count = 0
	n.multimesh = mm
	return n

## Static things are rebuilt at most this often, as the layered path does (world_view `_static_age`).
const STATIC_EVERY := 0.5

# What each batch was last built from, so a call that would rebuild the same thing does nothing
# (WP-06 6.4, FIXLOG 2026-11-13): rebuild() runs every frame from world_view._process and again in
# refresh(), and each full rebuild cost about 15 ms on the main thread at 500 people, 9 of them
# for the whole level's ground (16 384 tiles) that only changes when the ground does.
var _animal_key := ""
var _static_z := -999999
var _static_age := 999.0
var _ground_key := 0
## How many times each batch has been built (read by tools/gd_test.gd).
var builds := {"animals": 0, "static": 0, "ground": 0}

## Rebuild what changed. `delta` is the frame time when called from _process (0 from refresh), for
## the static batches' half-second rule. Animals: when the tick or the level changes
## (their rows are the sim's state, the same until the next tick). Static things: when the level
## changes, else at most every STATIC_EVERY seconds. Ground: when the level or any chunk's version
## changes (`chunk_versions` sums every chunk's version, so any changed tile on any level shows).
func rebuild(delta := 0.0, force := false) -> void:
	if view == null or view.sim == null:
		return
	var z: int = view.view_z
	var animal_key := "%d|%d" % [z, view.sim.tick()]
	if force or animal_key != _animal_key:
		_animal_key = animal_key
		builds["animals"] += 1
		var rows: PackedFloat32Array = view.sim.render_batches(z)
		_upload_animals(rows)
	_static_age += delta
	if force or z != _static_z or _static_age >= STATIC_EVERY:
		_static_z = z
		_static_age = 0.0
		builds["static"] += 1
		var static_rows: PackedFloat32Array = view.sim.render_static_batches(z)
		_upload_static(plants, static_rows, 0)
		_upload_static(structures, static_rows, 1)
		_upload_static(items, static_rows, 2)
	var zr: Vector2i = view.sim.z_range()
	var ground_key := hash([z, view.sim.chunk_versions(zr.x, zr.y)])
	if force or ground_key != _ground_key:
		_ground_key = ground_key
		builds["ground"] += 1
		var ground_rows: PackedInt32Array = view.sim.render_ground_batches(z)
		_upload_ground(ground, ground_rows)

func _upload_animals(rows: PackedFloat32Array) -> void:
	var count := 0
	for i in range(0, rows.size(), STRIDE):
		if int(rows[i + 4]) != 0:
			count += 1
	var mm: MultiMesh = animals.multimesh
	mm.instance_count = count
	var k := 0
	for i in range(0, rows.size(), STRIDE):
		if int(rows[i + 4]) == 0:
			continue
		var at := Vector2(rows[i + 1] * TILE, rows[i + 2] * TILE)
		mm.set_instance_transform_2d(k, Transform2D(0.0, at))
		mm.set_instance_color(k, Color(1, 1, 1, 1))
		k += 1

func _upload_static(n: MultiMeshInstance2D, rows: PackedFloat32Array, category: int) -> void:
	var mm: MultiMesh = n.multimesh
	var count := 0
	for i in range(0, rows.size(), STRIDE):
		if int(rows[i]) == category:
			count += 1
	mm.instance_count = count
	var k := 0
	for i in range(0, rows.size(), STRIDE):
		if int(rows[i]) != category:
			continue
		var at := Vector2(rows[i + 2] * TILE, rows[i + 3] * TILE)
		mm.set_instance_transform_2d(k, Transform2D(0.0, at))
		mm.set_instance_color(k, Color(1, 1, 1, 1))
		k += 1

func _upload_ground(n: MultiMeshInstance2D, rows: PackedInt32Array) -> void:
	var mm: MultiMesh = n.multimesh
	mm.instance_count = rows.size() / GROUND_STRIDE
	for i in range(0, rows.size(), GROUND_STRIDE):
		var at := Vector2(rows[i] * TILE, rows[i + 1] * TILE)
		var k := i / GROUND_STRIDE
		mm.set_instance_transform_2d(k, Transform2D(0.0, at))
		mm.set_instance_color(k, Color(1, 1, 1, 1))
