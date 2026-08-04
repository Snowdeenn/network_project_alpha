struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;

    // Génère un triangle géant qui couvre tout l'écran [-1, 3]x[-1, 3]
    let x = f32(i32(vertex_index & 1u) * 4 - 1);
    let y = f32(i32(vertex_index & 2u) * 2 - 1);

    out.clip_position = vec4<f32>(x, y, 0.0, 1.0);
    // Convertit les coordonnées NDC [-1, 1] en UV [0, 1]
    out.uv = vec2<f32>((x + 1.0) * 0.5, (1.0 - y) * 0.5);

    return out;
}