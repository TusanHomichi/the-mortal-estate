shader_type spatial;
render_mode diffuse_lambert, specular_schlick_ggx;

uniform sampler2D swatch : source_color, repeat_enable, filter_nearest;
uniform vec2 cell_origin = vec2(0.0);
uniform float swatch_period = 3.0;
uniform float joint_width = 0.028;
uniform float wetness : hint_range(0.0, 1.0) = 0.0;
uniform vec3 time_tint : source_color = vec3(1.0);

void fragment() {
	vec2 world_uv = (cell_origin + UV - vec2(0.5)) / swatch_period;
	vec3 base = texture(swatch, world_uv).rgb * time_tint;
	float edge_distance = min(min(UV.x, 1.0 - UV.x), min(UV.y, 1.0 - UV.y));
	float joint = smoothstep(0.0, joint_width, edge_distance);
	base *= mix(0.72, 1.0, joint);
	ALBEDO = mix(base, base * vec3(0.66, 0.74, 0.82), wetness * 0.32);
	ROUGHNESS = mix(0.92, 0.62, wetness);
	SPECULAR = mix(0.18, 0.275, wetness);
}
