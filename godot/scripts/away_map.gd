extends RefCounted
## Those of the valley who are in another place (#562), on the Legends world map of the present
## and in the lines under it. Rows come from the bridge's away_list (hf_sim view::away): name,
## place, region (-1 when the valley does not know where), reaction, line.

const Setup := preload("res://scripts/setup.gd")

## The centre of a place's mark by how it goes for them there.
const REACTION_COLOURS := {
	"welcomed": Color(0.45, 0.85, 0.45),
	"threatened": Color(0.95, 0.3, 0.25),
	"working": Color(0.95, 0.8, 0.35),
	"grieving": Color(0.6, 0.6, 0.95),
	"travelling": Color(0.85, 0.85, 0.85),
	"among strangers": Color(1, 1, 1),
}

static func colour_of(reaction: String) -> Color:
	return REACTION_COLOURS.get(reaction, Color(1, 1, 1))

## Mark every place someone is at whose region the valley knows; with several at one place the
## mark takes the first one's colour, threatened before the rest.
static func draw(img: Image, rows: Array) -> void:
	var n := img.get_width()
	var at := {}
	for r in rows:
		var region := int(r.get("region", -1))
		if region < 0 or region >= n * n:
			continue
		var reaction := String(r.get("reaction", ""))
		if not at.has(region) or reaction == "threatened":
			at[region] = reaction
	for region in at:
		Setup.mark_place(img, int(region) % n, int(region) / n, colour_of(String(at[region])))

## The lines under the map, as BBCode: a line per person, coloured by how it goes for them.
static func lines(rows: Array) -> String:
	if rows.is_empty():
		return ""
	var s := "[b]Away from the valley[/b]\n"
	for r in rows:
		var c: Color = colour_of(String(r.get("reaction", "")))
		s += "[color=#%s]%s[/color] (%s): %s\n" % [c.to_html(false), r.get("name", "?"), r.get("reaction", ""), r.get("line", "")]
	return s
