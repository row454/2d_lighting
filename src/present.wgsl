struct Viewport {
    viewport: mat4x4<f32>,
};
@group(1) @binding(0) // 1.
var<uniform> viewport: Viewport;

struct VertexInput {
    @location(0) position: vec3f,
    @location(1) tex_coords: vec2f,
}
struct VertexOutput {
    @builtin(position) clip_position: vec4f,
    @location(0) tex_coords: vec2f,
}

@vertex
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.tex_coords = model.tex_coords;
    out.clip_position = viewport.viewport * vec4f(model.position[0], model.position[1], 0.0, 1.0);
    return out;
}

@group(0) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(0) @binding(1)
var s_diffuse: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4f {
    return textureSample(t_diffuse, s_diffuse, in.tex_coords);
}