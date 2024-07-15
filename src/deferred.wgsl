struct CameraUniform {
    view_proj: mat4x4<f32>,
    view: mat4x4<f32>,
    dimensions: vec2f,
};
@group(1) @binding(0) // 1.
var<uniform> camera: CameraUniform;

struct DeferredVertexInput {
    @location(0) position: vec3f,
    @location(1) albedo_coords: vec2f,
    @location(2) normal_coords: vec2f,
}
struct DeferredVertexOutput {
    @builtin(position) clip_position: vec4f,
    @location(0) albedo_coords: vec2f,
    @location(1) normal_coords: vec2f,
}

@vertex
fn vs_main(
    model: DeferredVertexInput,
) -> DeferredVertexOutput {
    var out: DeferredVertexOutput;
    out.albedo_coords = model.albedo_coords;
    out.normal_coords = model.normal_coords;
    out.clip_position = camera.view_proj * vec4f(model.position[0], model.position[1], 0.0, 1.0);
    return out;
}
struct DeferredFragmentOutput {
    @location(0) albedo_color: vec4f,
    @location(1) normal_color: vec4f,
}

@group(0) @binding(0)
var t_deferred: texture_2d<f32>;
@group(0) @binding(1)
var s_pair: sampler;

@fragment
fn fs_main(in: DeferredVertexOutput) -> DeferredFragmentOutput {
    var out: DeferredFragmentOutput;
    out.albedo_color = textureSample(t_deferred, s_pair, in.albedo_coords);
    out.normal_color = textureSample(t_deferred, s_pair, in.normal_coords);
    return out;
}