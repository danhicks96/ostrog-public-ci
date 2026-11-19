extends Node2D
## The ground that moves: water frames, fire on tiles, doors opening as people pass. Drawn
## from the lists the cached chunks collect, at 8 Hz (the fastest frame rate any of it uses).

var v
var acc := 0.0

func _process(d: float) -> void:
	acc += d
	# FIXLOG 2026-11-02 #484 (finish): while a timelapse sample is taken the layer redraws every
	# frame, so the image shows the sample's frozen water and fire frames, not the last 8 Hz draw.
	if acc >= 0.125 or v.animation_frozen:
		acc = 0.0
		queue_redraw()

func _draw() -> void:
	v._draw_dynamic(self)
