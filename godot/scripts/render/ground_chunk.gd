extends Node2D
## One cached 32×32-tile canvas of ground (or of wall and cliff faces). world_view.gd redraws
## it only when the simulation's chunk version, the season, the sky or the level changes.

var v
var cx := 0
var cy := 0
var faces := false
var dyn := []   # tiles that animate or change every refresh: [x, y, kind, fam, dim]

func _draw() -> void:
	v._draw_chunk(self, cx, cy, faces)
