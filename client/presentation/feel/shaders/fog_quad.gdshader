shader_type canvas_item;

uniform vec4 fog_colour : source_color = vec4(0.33, 0.39, 0.52, 0.16);

float hash(vec2 value) {
	return fract(sin(dot(value, vec2(127.1, 311.7))) * 43758.5453);
}

void fragment() {
	vec2 drift = vec2(TIME * 0.006, TIME * -0.003);
	float coarse = hash(floor((UV + drift) * vec2(28.0, 18.0)));
	float veil = mix(0.58, 1.0, coarse);
	COLOR = vec4(fog_colour.rgb, fog_colour.a * veil);
}
