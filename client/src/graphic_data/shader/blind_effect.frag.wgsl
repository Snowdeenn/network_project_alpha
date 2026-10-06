@group(0) @binding(0) var scene_texture: texture_2d<f32>;
@group(0) @binding(1) var scene_sampler: sampler;

struct BlindUniform {
		blind_amount: f32,
		aspect_ratio: f32,
}

@group(1) @binding(0) var<uniform> blind: BlindUniform;

struct FragementInput {
		@location(0) uv: vec2<f32>
}

@fragment
fn fs_main(in: FragementInput) -> @location(0) vec4<f32> {
	let scene = textureSample(scene_texture, scene_sampler, in.uv);
	var position = in.uv - vec2<f32>(0.5, 0.5);

	position.x *= blind.aspect_ratio;
	let distance_from_center = length(position);

	let full_radius = length(vec2<f32>(0.5 * blind.aspect_ratio, 0.5));
	let radius = mix(full_radius, 0.1, blind.blind_amount);
	const softness = 0.08;

	let vignette = smoothstep(radius, radius + softness, distance_from_center);
	let darkness = vignette * blind.blind_amount;
	let blind_color = vec4<f32>(0.0, 0.0, 0.0, 1.0);

	return vec4<f32>(scene.rgb * (1.0 - darkness), scene.a);
}
