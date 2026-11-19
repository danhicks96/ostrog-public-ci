extends RefCounted
## Bounded metadata for the viewer's sampled images. No simulation reference lives here.

const SAMPLE_INTERVAL_TICKS := 28800
## Samples fall at noon of each calendar day (docs/TIME.md: 1,200 ticks an hour, the day
## starting at the world's epoch), so every sample of a series is lit the same way and a
## re-run of the same days lands on the same ticks.
const SAMPLE_OFFSET_TICKS := 14400
const MAX_FRAMES := 120
## Hard bounds on disk (#484 finish). A sample is at most CAPTURE_MAX pixels of RGB, so one PNG
## is at most about 2.8 MB; a series keeps at most MAX_FRAMES samples and MAX_SERIES_BYTES of
## PNG; at most MAX_SERIES worlds keep a series. The worst case on disk is therefore
## MAX_SERIES x MAX_SERIES_BYTES plus the small JSON records (docs/TIMELAPSE.md).
const CAPTURE_MAX := Vector2i(1280, 720)
const MAX_SERIES_BYTES := 160 * 1048576
const MAX_SERIES := 6
const ROOT := "user://timelapse"

var frames: Array = []
var total_bytes := 0

static func sample_due(tick: int, next_tick: int) -> bool:
	return next_tick >= 0 and tick >= next_tick

## The first sampling tick at or after `tick`: noon of a calendar day counted from `epoch`.
static func next_sample_tick(tick: int, epoch: int) -> int:
	var phase := posmod(tick - epoch - SAMPLE_OFFSET_TICKS, SAMPLE_INTERVAL_TICKS)
	return tick if phase == 0 else tick + SAMPLE_INTERVAL_TICKS - phase

## How many ticks the client may step now without passing the next sample, so the sample is
## taken at exactly its tick rather than at whatever tick the frame's step ended on.
static func step_allowance(tick: int, next_tick: int, wanted: int) -> int:
	if next_tick < 0 or next_tick < tick:
		return wanted
	return mini(wanted, next_tick - tick)

static func series_dir(identity: String, root := ROOT) -> String:
	return "%s/world-%s" % [root, identity.substr(0, 16)]

## Where a replayed tick stands against what the series already holds: "new" (no sample at
## that tick), "confirmed" (a sample with the same state digest: the re-run reached the same
## world, so the stored image still describes it) or "diverged" (a different digest: the stored
## sample and every later one describe a history this world no longer has).
func reconcile(tick: int, digest: String) -> String:
	for f in frames:
		if int(f["tick"]) == tick:
			return "confirmed" if str(f.get("state_digest", "")) == digest else "diverged"
	return "new"

## Removes and returns every sample at or after `tick` (the stale future after a divergence).
func drop_from(tick: int) -> Array:
	var kept: Array = []
	var dropped: Array = []
	for f in frames:
		if int(f["tick"]) >= tick:
			dropped.append(f)
			total_bytes -= int(f.get("bytes", 0))
		else:
			kept.append(f)
	frames = kept
	return dropped

## Adds a sample in tick order (a same-tick sample replaces the old one) and returns the samples
## evicted to keep within MAX_FRAMES and MAX_SERIES_BYTES, oldest first. The newest sample is
## never evicted.
func add_frame(frame: Dictionary) -> Array:
	var tick := int(frame["tick"])
	for i in frames.size():
		if int(frames[i]["tick"]) == tick:
			total_bytes -= int(frames[i].get("bytes", 0))
			frames[i] = frame.duplicate(true)
			total_bytes += int(frame.get("bytes", 0))
			return _enforce_bounds()
	var at := frames.size()
	while at > 0 and int(frames[at - 1]["tick"]) > tick:
		at -= 1
	frames.insert(at, frame.duplicate(true))
	total_bytes += int(frame.get("bytes", 0))
	return _enforce_bounds()

func _enforce_bounds() -> Array:
	var evicted: Array = []
	while frames.size() > MAX_FRAMES or (total_bytes > MAX_SERIES_BYTES and frames.size() > 1):
		var old: Dictionary = frames.pop_front()
		total_bytes -= int(old.get("bytes", 0))
		evicted.append(old)
	return evicted

func clear() -> void:
	frames.clear()
	total_bytes = 0

## Reads a series back from its JSON records (a resumed or reloaded world). Records whose PNG
## is missing are skipped; the result is in tick order and within bounds, and the evicted
## records are returned so the caller can remove their files.
func load_series(dir: String) -> Array:
	clear()
	var absolute := ProjectSettings.globalize_path(dir)
	var d := DirAccess.open(absolute)
	if d == null:
		return []
	var names: Array = []
	for name in d.get_files():
		if name.ends_with(".json"):
			names.append(name)
	names.sort()
	var loaded: Array = []
	for name in names:
		var parsed = JSON.parse_string(FileAccess.get_file_as_string(absolute.path_join(name)))
		if not (parsed is Dictionary) or not parsed.has("tick"):
			continue
		var png := dir.path_join(name.get_basename() + ".png")
		if not FileAccess.file_exists(png):
			continue
		parsed["tick"] = int(parsed["tick"])
		parsed["path"] = png
		parsed["bytes"] = FileAccess.get_file_as_bytes(png).size()
		loaded.append(parsed)
	var evicted: Array = []
	for f in loaded:
		evicted.append_array(add_frame(f))
	return evicted

static func remove_frame_files(frame: Dictionary) -> void:
	var png := ProjectSettings.globalize_path(str(frame["path"]))
	DirAccess.remove_absolute(png)
	DirAccess.remove_absolute(png.get_basename() + ".json")

## Keeps at most MAX_SERIES series under ROOT: the current one and the most recently written
## others. Returns the directories removed. Only files this recorder writes (PNG and JSON) are
## removed, and a directory only once it is empty.
static func prune_series(keep: String, root := ROOT, max_series := MAX_SERIES) -> Array:
	var absolute := ProjectSettings.globalize_path(root)
	var d := DirAccess.open(absolute)
	if d == null:
		return []
	var keep_abs := ProjectSettings.globalize_path(keep)
	var others: Array = []
	for name in d.get_directories():
		var path := absolute.path_join(name)
		if path == keep_abs:
			continue
		var newest := 0
		var sub := DirAccess.open(path)
		if sub != null:
			for f in sub.get_files():
				newest = maxi(newest, FileAccess.get_modified_time(path.path_join(f)))
		others.append([newest, path])
	others.sort_custom(func(a, b): return a[0] > b[0] or (a[0] == b[0] and a[1] < b[1]))
	var removed: Array = []
	for i in range(max_series - 1, others.size()):
		var path: String = others[i][1]
		var sub := DirAccess.open(path)
		if sub == null:
			continue
		for f in sub.get_files():
			if f.ends_with(".png") or f.ends_with(".json"):
				DirAccess.remove_absolute(path.path_join(f))
		if DirAccess.open(path).get_files().is_empty() and DirAccess.open(path).get_directories().is_empty():
			DirAccess.remove_absolute(path)
		removed.append(path)
	return removed

## The stored form of a viewport capture: scaled down to fit CAPTURE_MAX (never up) with a
## fixed filter, and without alpha, so a sample's size has a hard upper bound and the same
## viewport pixels always give the same file.
static func fit_capture(image: Image) -> Image:
	var out := image.duplicate() as Image
	if out.is_compressed():
		out.decompress()
	out.convert(Image.FORMAT_RGB8)
	var s := minf(1.0, minf(float(CAPTURE_MAX.x) / out.get_width(), float(CAPTURE_MAX.y) / out.get_height()))
	if s < 1.0:
		out.resize(maxi(1, int(round(out.get_width() * s))), maxi(1, int(round(out.get_height() * s))), Image.INTERPOLATE_BILINEAR)
	return out

## The camera zoom that fits a whole map of `map_pixels` into `viewport` (the valley view).
static func valley_zoom(viewport: Vector2, map_pixels: Vector2) -> float:
	if map_pixels.x <= 0.0 or map_pixels.y <= 0.0:
		return 1.0
	return minf(viewport.x / map_pixels.x, viewport.y / map_pixels.y)

static func freeze_particles(root: Node, tick: int) -> void:
	# FIXLOG 2026-10-31 #484 (c145 review): wall-clock particle advancement would make
	# otherwise identical sampled views produce different pixels.
	if root is CPUParticles2D:
		root.use_fixed_seed = true
		root.seed = tick % 2147483647
		root.fixed_fps = 30
		root.restart()
		root.speed_scale = 0.0
	for child in root.get_children():
		freeze_particles(child, tick)

static func resume_particles(root: Node) -> void:
	if root is CPUParticles2D:
		root.speed_scale = 1.0
	for child in root.get_children():
		resume_particles(child)
