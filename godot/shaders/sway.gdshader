shader_type canvas_item;
// Wind on canopies and stems: the top of the sprite leans by whole pixels, the base stays put.
// Sheets are single-row, so UV.y runs 0 at the top to 1 at the base. Phase comes from the
// sprite's world position, so neighbouring trees do not move in lockstep.
uniform float wind = 1.0;
uniform float amp = 1.5;
uniform float presentation_time = 0.0;

void vertex() {
	vec2 wpos = (MODEL_MATRIX * vec4(0.0, 0.0, 0.0, 1.0)).xy;
	float phase = wpos.x * 0.071 + wpos.y * 0.033;
	float s = sin(presentation_time * (1.0 + 0.35 * wind) + phase) + 0.35 * sin(presentation_time * 2.3 + phase * 1.7);
	VERTEX.x += round(s * amp * wind * (1.0 - UV.y));
}

void fragment() {
	COLOR = texture(TEXTURE, UV) * COLOR;
}
