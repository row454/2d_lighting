struct VertexInput {
    @location(0) position: vec3f,
    @location(1) tex_coords: vec2f,
    @location(2) color: vec3f,
}
struct VertexOutput {
    @builtin(position) clip_position: vec4f,
    @location(0) tex_coords: vec2f,
    @location(3) color: vec3f,
}

@vertex
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.tex_coords = model.tex_coords;
    out.clip_position = vec4f(model.position[0], model.position[1], 0.0, 1.0);
    out.color = model.color;
    return out;
}

@group(0) @binding(0)
var albedo: texture_2d<f32>;
@group(0) @binding(1)
var normal: texture_2d<f32>;
@group(0) @binding(2)
var g_buffer_sampler: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4f {
    return textureSample(albedo, g_buffer_sampler, in.tex_coords) * vec4f(in.color, 0);
}