shader_type spatial;
render_mode cull_disabled, diffuse_lambert, specular_disabled;

uniform sampler2D albedo_texture : source_color, filter_nearest;
uniform float wind_strength : hint_range(0.0, 1.0) = 0.0;
uniform float time_offset = 0.0;

void vertex() {
	float crown = smoothstep(0.18, 1.0, 1.0 - UV.y);
	VERTEX.x += sin(TIME * 1.15 + time_offset + VERTEX.y * 1.7) * wind_strength * 0.055 * crown;
}

void fragment() {
	vec4 texel = texture(albedo_texture, UV);
	ALBEDO = texel.rgb;
	ALPHA = texel.a;
	ALPHA_SCISSOR_THRESHOLD = 0.12;
	ROUGHNESS = 0.88;
}
