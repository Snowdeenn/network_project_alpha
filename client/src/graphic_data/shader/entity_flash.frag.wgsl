@group(1) @binding(0) var sprite: texture_2d<f32>;
@group(1) @binding(1) var sprite_sampler: sampler;

struct FragmentInput {
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
};

@fragment
fn fs_main(input: FragmentInput) -> @location(0) vec4<f32> {
    let alpha = textureSample(sprite, sprite_sampler, input.uv).a;
    return vec4<f32>(input.color.rgb, alpha * input.color.a);
}
