struct VertexOutput {
    [[builtin(position)]]
    position: vec4<f32>;
    [[location(0)]]
    uv: vec2<f32>;
};

[[stage(vertex)]]
fn vs_main([[builtin(vertex_index)]] vertex_index: u32) -> VertexOutput {
    // let x = f32(vertex_index) - 1.0;
    // let y = f32((vertex_index & 1u) * 2u - 1u);

    var out: VertexOutput;

    let x = f32(u32(vertex_index + 2u) / 3u % 2u);
    let y = f32(u32(vertex_index + 1u) / 3u % 2u);
    out.position = vec4<f32>(-1.0 + x * 2.0, -1.0 + y * 2.0, 0.0, 1.0);
    out.uv = vec2<f32>(x, 1.0 - y);

    return out;
}

[[group(0), binding(0)]]
var view_texture: texture_2d<f32>;
[[group(0), binding(1)]]
var view_sampler: sampler;

[[stage(fragment)]]
fn fs_main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    let color = textureSample(view_texture, view_sampler, in.uv);
    return color;
}
