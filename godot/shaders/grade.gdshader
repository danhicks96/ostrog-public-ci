shader_type canvas_item;
// The light and colour grade over the whole world view (art bible: the grade may leave the
// palette, sprites may not). Ambient light comes from the sun outdoors and from nothing
// underground; point lights (hearths, lamps, fires) add warm light with a smooth falloff in
// world space, and the glowing fungus of the caverns a cool one (negative intensity). Night keeps values and pulls colour toward blue; dusk and dawn warm.
uniform sampler2D screen_tex : hint_screen_texture, filter_nearest;
uniform sampler2D sky_mask : filter_linear, repeat_disable;  // r: 1 where the sky is overhead; g: 1 where a tile stops light
uniform vec2 world_tl;    // world px at the top-left of the screen
uniform vec2 world_br;    // world px at the bottom-right
uniform vec2 map_px;      // map size in world px
uniform float sun = 1.0;          // 0 night .. 1 full day
uniform float overcast = 0.0;     // 0 clear .. 1 storm
uniform float night_level = 0.28; // art bible: night floor 72 % dark
// FIXLOG 2026-10-30 fix-506-underground-context (#506): keep unlit rock readable, not daylight.
uniform float under_level = 0.18;
uniform float flash = 0.0;        // lightning
uniform float presentation_time = 0.0;
uniform vec4 lights[64];          // xy world px, z radius px, w intensity (negative: cool glow)
uniform int light_count = 0;

float hash(vec2 p) { return fract(sin(dot(p, vec2(12.9898, 78.233))) * 43758.5453); }

// Whether light from `from` reaches `to` (world px): march the line a quarter tile at a time
// and stop on a tile that blocks light, the light's tile and the lit tile aside, so a wall is
// lit on its face toward the light (#180: hearth light leaked through walls). rules.gd lit()
// is the same march, and tools/gd_test.gd checks it.
bool lit(vec2 from, vec2 to) {
	vec2 a = from / 32.0;
	vec2 b = to / 32.0;
	ivec2 la = ivec2(floor(a));
	ivec2 lb = ivec2(floor(b));
	ivec2 size = textureSize(sky_mask, 0);
	int n = int(ceil(distance(a, b) / 0.25));
	for (int k = 1; k < n; k++) {
		ivec2 p = ivec2(floor(mix(a, b, float(k) / float(n))));
		if (p == la || p == lb || p.x < 0 || p.y < 0 || p.x >= size.x || p.y >= size.y) {
			continue;
		}
		if (texelFetch(sky_mask, p, 0).g > 0.5) {
			return false;
		}
	}
	return true;
}

void fragment() {
	vec3 base = texture(screen_tex, SCREEN_UV).rgb;
	vec2 wp = mix(world_tl, world_br, SCREEN_UV);
	vec2 muv = wp / map_px;
	float outdoor = 1.0;
	if (muv.x >= 0.0 && muv.y >= 0.0 && muv.x <= 1.0 && muv.y <= 1.0) {
		outdoor = texture(sky_mask, muv).r;
	}
	float day = sun * (1.0 - 0.35 * overcast);
	float amb_out = mix(night_level, 1.0, day);
	float amb = mix(under_level, amb_out, outdoor);

	float art = 0.0;
	float glow = 0.0;
	for (int i = 0; i < light_count; i++) {
		vec4 L = lights[i];
		float d = distance(wp, L.xy) / L.z;
		if (d < 1.0 && lit(L.xy, wp)) {
			float f = pow(1.0 - d, 1.6) * abs(L.w);
			if (L.w < 0.0) {
				// Fungus light does not flicker; it breathes, slowly.
				glow = max(glow, f * (1.0 + 0.08 * sin(presentation_time * 0.9 + hash(L.xy) * 6.0)));
			} else {
				float flick = 1.0 + 0.07 * sin(presentation_time * 9.0 + hash(L.xy) * 40.0) + 0.04 * sin(presentation_time * 23.0 + L.x);
				art = max(art, f * flick);
			}
		}
	}

	float luma = dot(base, vec3(0.299, 0.587, 0.114));
	// Outdoor grade by the sun: warm low sun, neutral noon, blue desaturated night.
	vec3 warm = mix(vec3(1.06, 0.80, 0.60), vec3(1.0, 0.99, 0.96), smoothstep(0.25, 0.85, sun));
	vec3 day_col = base * warm;
	day_col = mix(day_col, vec3(luma) * vec3(0.92, 0.95, 1.0), 0.35 * overcast);
// Keep the night grade blue, but retain enough source hue for grass, water, stone and clothing
// to remain identifiable at the game's 72% night floor. Darkness should create atmosphere, not
// turn the whole valley into one near-black silhouette.
vec3 night_col = mix(base, vec3(luma) * vec3(0.58, 0.70, 1.0), 0.50);
	vec3 out_col = mix(night_col, day_col, smoothstep(0.0, 0.6, day));
	// Underground: earthy and dim.
// Underground stays earthy and dim, while preserving material hue for cave floors, fungi and
// miners' clothing. Point lights still provide the strong warm pools around lamps and hearths.
vec3 under_col = mix(base, vec3(luma) * vec3(0.85, 0.82, 0.78), 0.28);
	vec3 amb_col = mix(under_col, out_col, outdoor) * amb;
	// Firelight restores the picture warm where it reaches, more so where it is dark.
	vec3 fire = (base * vec3(1.12, 0.84, 0.58) + vec3(0.09, 0.04, 0.0)) * art * (1.0 - amb * 0.93);
	vec3 cool = (base * vec3(0.66, 0.88, 1.16) + vec3(0.0, 0.02, 0.06)) * glow * (1.0 - amb * 0.93);
	vec3 col = amb_col + max(fire, cool) + vec3(0.85, 0.9, 1.0) * flash * outdoor * 0.6;
	COLOR = vec4(col, 1.0);
}
