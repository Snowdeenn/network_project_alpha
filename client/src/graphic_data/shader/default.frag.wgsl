// @group(1) @binding(0) var t_diffuse: texture_2d<f32>;
// @group(1) @binding(1) var s_diffuse: sampler;

struct FragmentInput {
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
};

@fragment
fn fs_main(in: FragmentInput) -> @location(0) vec4<f32> {
    // let tex_color = textureSample(t_diffuse, s_diffuse, in.uv);
    // return tex_color * in.color;
    return in.color;
}