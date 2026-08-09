struct Uniforms {
    u_ratio: f32, // 0.0 = presque mort, 1.0 = plein
};

@group(2) @binding(0)
var<uniform> uniforms: Uniforms;

struct VertexOutput {
    @location(0) frag_tex_coord: vec2<f32>,
    @location(1) frag_color: vec4<f32>,
};

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // vert quand ratio=1.0, rouge quand ratio=0.0
    let green = vec3<f32>(0.0, 1.0, 0.0);
    let red   = vec3<f32>(1.0, 0.0, 0.0);
    let color = mix(red, green, uniforms.u_ratio);

    return vec4<f32>(color, 1.0) * in.frag_color;
}