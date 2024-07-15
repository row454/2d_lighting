struct CameraUniform {
    view_proj: mat4x4<f32>,
    view: mat4x4<f32>,
    dimensions: vec2f,
};
@group(1) @binding(0) // 1.
var<uniform> camera: CameraUniform;

struct VertexInput {
    @location(0) position: vec3f,
    @location(1) color: vec3f,
    @location(2) center: vec3f,
    @location(3) radius: f32,
}
struct VertexOutput {
    @builtin(position) clip_position: vec4f,
    @location(0) center: vec3f,
    @location(1) tex_coords: vec2f,
    @location(2) color: vec4f,
    @location(3) radius: f32,
}

@vertex
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;

    out.clip_position = camera.view_proj * vec4f(model.position, 1.0);
    out.center = ((camera.view * vec4f(model.center, 1.0)).xyz * vec3f(1.0, -1.0, 1.0)) + vec3f(camera.dimensions.x/2.0, camera.dimensions.y/2.0, 0.0);
    out.tex_coords = out.clip_position.xy * vec2f(0.5, -0.5) + vec2f(0.5, 0.5);
    out.radius = model.radius;
    out.color = vec4f(model.color, 1.0);
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
    if distance(in.clip_position.xy, in.center.xy) > in.radius {
        discard;
    }
    let albedo_color = textureSample(albedo, g_buffer_sampler, in.tex_coords);
    let normal_color = textureSample(normal, g_buffer_sampler, in.tex_coords);

    let normal = vec3f(normal_color.xyz * 2.0 - vec3f(1.0, 1.0, 1.0));
    
    var dir_to_light = normalize(in.center.xyz - vec3f(in.clip_position.xy, 0.0));
    dir_to_light.y = -dir_to_light.y;

    let normal_multiplier = saturate(dot(normal, dir_to_light));

    return vec4f(albedo_color.xyz * in.color.xyz * normal_multiplier, 1.0);
}