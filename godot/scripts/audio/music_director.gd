extends Node
## Music director: folk by day, ambient by night (tools/audio/music.py writes the cues).
##
## Music comes in phrases: 2 to 4 minutes of a cue, then 1 to 3 minutes with ambience only.
## The cue follows what the view shows: the season, day or night (with hysteresis on the sun so
## dusk does not flicker), and "deep" whenever the view is below the ground. A cue is four
## equal-length looping stems played together by one AudioStreamSynchronized (drone, pad,
## melody, tension); the tension stem (frame drum and a second melody) rises with the
## community's strain from the bridge (SimHost.tension()) and is silent when nobody is under
## strain. A change of cue waits for the end of the current 4-bar phrase, then crossfades into
## the new cue's first bar, so a switch lands after a cadence. Bus: Music (made by sound.gd).
##
## Besides strain it answers what the valley comes to know (#290, WP-08 8.8; the moments come
## from hf_sim::view::music through sound.gd): a death among the people silences the melody
## stem for MOURN seconds, drone and pad carrying on; a birth, a band welcomed or a caravan
## home ends a rest within WELCOME_WAIT seconds, so the score comes back for it. It also hears
## what the people are doing (#290 item 3, Rules.music_activity): a fight lifts the tension
## layer to at least FIGHT_FLOOR (a fire in view to at least FIRE_FLOOR, the colony at work
## to at least WORK_FLOOR), three or more
## gathered talking or eating end a rest as a
## welcome does, and while every person sleeps no new phrase begins (the valley is quiet).
##
## `--music-fast` divides the phrase and rest lengths by ten, to see changes in a short run.

const Rules := preload("res://scripts/rules.gd")

const MANIFEST := "res://assets/audio/music.json"
const ROOT := "res://assets/audio/"
const FADE_IN := 6.0
const XFADE := 5.0
const FADE_OUT := 8.0
const FLOOR_DB := -60.0
const MOURN := 150.0
const WELCOME_WAIT := 6.0
const FIGHT_FLOOR := 0.6
## A fire burning in view lifts the tension layer to at least this (#290, WP-08 8.8).
const FIRE_FLOOR := 0.45
## The valley at work lifts the tension layer to at least this (#290, WP-08 8.8 work
## bustle): a busy day, gentle, below a fire and never lowering high strain.
const WORK_FLOOR := 0.2

var v        # world_view
var sim      # SimHost
var sound    # sound.gd, for its --audio-log lines
var silent := false
var manifest := {}
var stems: Array = []
var tension_i := 3
var melody_i := 2
var mourn_left := 0.0     # seconds the melody stays silent for a death (#290)
var melody := 1.0         # smoothed melody stem level, linear 0..1
var doing := {}           # Rules.music_activity of the last look (#290 item 3)
var gathered := false     # a gathering was on at the last look, so it welcomes once
var burning := false      # a fire was in view at the last look (#290)
var bustling := false     # three or more were at work at the last look (WP-08 8.8)
var players: Array = []   # two AudioStreamPlayers, alternated for crossfades
var cur := -1             # index into players of the sounding cue, -1 when none
var cue := ""
var pending := ""
var playing := false      # false = resting (ambience only)
var left := 0.0           # seconds until this phrase or rest is due to end
var phrase_len := 16.0
var clock := 0.0
var last_idx := 0
var phrases := 0         # phrase ends heard in the current cue (the log's proof it plays)
var is_day := true
var strain := 0.0
var level := 0.0          # smoothed tension layer level, linear 0..1
var band := -1
var fast := false
## `--verbose-audio` prints the director's lines to the console and godot.log.
var verbose := false
var rng := RandomNumberGenerator.new()
var acc := 0.0
var streams := {}

func init(view, host, quiet: bool, snd = null) -> void:
	v = view
	sim = host
	sound = snd
	silent = quiet
	rng.seed = 7
	fast = "--music-fast" in OS.get_cmdline_user_args()
	verbose = "--verbose-audio" in OS.get_cmdline_user_args()
	var f := FileAccess.open(MANIFEST, FileAccess.READ)
	if f:
		var m = JSON.parse_string(f.get_as_text())
		if m is Dictionary:
			manifest = m
	stems = manifest.get("stems", [])
	tension_i = maxi(0, stems.find("tension"))
	melody_i = stems.find("melody")
	if snd:
		snd.music = self
	for k in 2:
		var p := AudioStreamPlayer.new()
		p.bus = "Music"
		p.volume_db = -80.0
		add_child(p)
		players.append(p)
	is_day = v.sun_level() >= 0.35
	left = _span(8.0, 20.0)  # a short quiet start, then the first phrase
	_log("music: ready, %d cues, first phrase in %.0f s%s" % [(manifest.get("cues", {}) as Dictionary).size(), left, " (fast)" if fast else ""])

func _span(a: float, b: float) -> float:
	var s := rng.randf_range(a, b)
	return s / 10.0 if fast else s

func _log(line: String) -> void:
	# Quiet unless asked: these lines were most of godot.log in a normal run (#565).
	if verbose:
		print(line)
	if sound and sound.log_path != "":
		sound.log_lines.append("%.2f %s" % [sound.t, line])

## The cue the view asks for now.
func desired() -> String:
	if not v.under_sky_at_centre():
		return "deep"
	var sun: float = v.sun_level()
	if is_day and sun < 0.2:
		is_day = false
	elif not is_day and sun > 0.5:
		is_day = true
	var names := ["spring", "summer", "autumn", "winter"]
	var want := "%s_%s" % [names[clampi(int(v.season), 0, 3)], "day" if is_day else "night"]
	return want if manifest.get("cues", {}).has(want) else ""

func _ogg(path: String) -> AudioStream:
	if streams.has(path):
		return streams[path]
	var st: AudioStream = null
	var full := ROOT + path
	if ResourceLoader.exists(full):
		st = load(full)
	elif FileAccess.file_exists(full):
		st = AudioStreamOggVorbis.load_from_file(full)
	if st is AudioStreamOggVorbis:
		st.loop = true
	streams[path] = st
	return st

func _build(cn: String) -> AudioStreamSynchronized:
	var c: Dictionary = manifest["cues"][cn]
	var files: Dictionary = c["files"]
	var s := AudioStreamSynchronized.new()
	s.stream_count = stems.size()
	for i in stems.size():
		s.set_sync_stream(i, _ogg(str(files[stems[i]])))
		s.set_sync_stream_volume(i, 0.0)
	s.set_sync_stream_volume(tension_i, _tension_db())
	if melody_i >= 0:
		s.set_sync_stream_volume(melody_i, _db(melody))
	return s

func _db(lin: float) -> float:
	return FLOOR_DB if lin < 0.001 else maxf(FLOOR_DB, linear_to_db(lin))

## A music moment from the sounds rows (#290): "mourn" or "welcome".
func moment(kind: String) -> void:
	if kind == "mourn":
		mourn_left = MOURN / 10.0 if fast else MOURN
		_log("music: mourning, melody silent for %.0f s" % mourn_left)
	elif kind == "welcome":
		var was := left
		left = Rules.welcome_rest(left, WELCOME_WAIT / 10.0 if fast else WELCOME_WAIT, playing)
		if left < was:
			_log("music: a welcome, the rest ends in %.0f s" % left)

func _tension_db() -> float:
	return FLOOR_DB if level < 0.001 else maxf(FLOOR_DB, linear_to_db(level))

func _fade(p: AudioStreamPlayer, to_db: float, secs: float, stop_after := false) -> void:
	var tw := create_tween()
	tw.tween_property(p, "volume_db", to_db, secs)
	if stop_after:
		tw.tween_callback(p.stop)

## Start cue `cn` on the free player, fading in; the old one (if any) fades out.
func _start(cn: String, fade: float) -> void:
	var nxt := 0 if cur != 0 else 1
	var p: AudioStreamPlayer = players[nxt]
	p.stop()
	p.stream = _build(cn)
	p.volume_db = -40.0
	p.play()
	_fade(p, 0.0, fade)
	if cur >= 0:
		_fade(players[cur], -80.0, fade, true)
	var from := cue
	var phrases_was := phrases
	cur = nxt
	cue = cn
	pending = ""
	var c: Dictionary = manifest["cues"][cn]
	phrase_len = float(c["seconds"]) * float(manifest.get("phrase_bars", 4)) / float(c["bars"])
	clock = 0.0
	last_idx = 0
	phrases = 0
	if from == "":
		_log("music: start %s (%s, %d bpm), phrase for %.0f s, tension %.2f" % [cn, c["key"], c["bpm"], left, level])
	else:
		_log("music: crossfade %s -> %s at a phrase end (%d phrases)" % [from, cn, phrases_was])

func _stop() -> void:
	if cur >= 0:
		_fade(players[cur], -80.0, FADE_OUT, true)
	_log("music: rest after %s (%d phrases), ambience only for %.0f s" % [cue, phrases, left])
	cur = -1
	cue = ""
	pending = ""

func _process(d: float) -> void:
	if silent or manifest.is_empty():
		return
	left -= d
	mourn_left = maxf(0.0, mourn_left - d)
	if playing:
		clock += d
		var pos := clock
		var p: AudioStreamPlayer = players[cur]
		if p.playing and p.get_playback_position() > 0.0:
			pos = p.get_playback_position()
		var idx := int(pos / phrase_len)
		if idx != last_idx:
			last_idx = idx
			_phrase_end()
	acc += d
	if acc < 0.5:
		return
	acc = 0.0
	var want := desired()
	_activity()
	if not playing:
		if left <= 0.0 and want != "" and not doing.get("asleep", false):
			playing = true
			left = _span(120.0, 240.0)
			_start(want, FADE_IN)
	elif want != "" and want != cue and want != pending:
		pending = want
		_log("music: %s wanted, waiting for the phrase end" % want)
	_tension()

## A 4-bar phrase just ended: switch cue if the view changed, or rest if this phrase is over.
func _phrase_end() -> void:
	if not playing:
		return
	phrases += 1
	if left <= 0.0:
		playing = false
		left = _span(60.0, 180.0)
		_stop()
	elif pending != "":
		_start(pending, XFADE)

## What the people are doing now (#290 item 3); a gathering that begins is welcomed once.
func _activity() -> void:
	if v == null or not ("agents" in v):
		return
	var was_fight: bool = doing.get("fight", false)
	doing = Rules.music_activity(v.agents, v.AGENT_STRIDE)
	if doing["fight"] and not was_fight:
		_log("music: a fight, tension at least %.2f" % FIGHT_FLOOR)
	if doing["gathering"] and not gathered:
		moment("welcome")
	gathered = doing["gathering"]
	var fire: bool = ("fires" in v) and not v.fires.is_empty()
	if fire and not burning:
		_log("music: a fire in view, tension at least %.2f" % FIRE_FLOOR)
	burning = fire
	var bustle: bool = doing.get("bustle", false)
	if bustle and not bustling:
		_log("music: the colony at work, tension at least %.2f" % WORK_FLOOR)
	bustling = bustle

func _tension() -> void:
	if sim == null or not sim.has_method("tension"):
		return
	strain = sim.tension()
	var target := Rules.tension_target(strain, doing.get("fight", false), FIGHT_FLOOR, burning, FIRE_FLOOR, bustling, WORK_FLOOR)
	level = lerpf(level, target, 0.08)
	# The melody falls away over a few seconds for a death and comes back as slowly (#290).
	melody = lerpf(melody, Rules.melody_target(mourn_left), 0.15)
	if cur >= 0:
		var s: AudioStreamSynchronized = players[cur].stream
		if s:
			s.set_sync_stream_volume(tension_i, _tension_db())
			if melody_i >= 0:
				s.set_sync_stream_volume(melody_i, _db(melody))
	var b := int(level * 3.0 + 0.5)
	if b != band:
		band = b
		_log("music: tension layer %.2f (strain %.2f)" % [level, strain])
