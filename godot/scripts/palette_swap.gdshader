shader_type canvas_item;
// Replace up to 16 source colours with target colours (exact match within a small tolerance).
uniform int count = 0;
uniform vec4 src[16];
uniform vec4 dst[16];

void fragment() {
	vec4 c = texture(TEXTURE, UV);
	for (int i = 0; i < count; i++) {
		if (distance(c.rgb, src[i].rgb) < 0.02) {
			c.rgb = dst[i].rgb;
			break;
		}
	}
	COLOR = c * COLOR;
}
