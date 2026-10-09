// The existing Phase 0 diagnostic appearance, with indexed model geometry and
// GPU world/normal transforms. Material appearance remains a separate handoff.
struct Camera { view_projection: mat4x4f }
@group(0) @binding(0) var<uniform> camera: Camera;
struct VertexOut { @builtin(position) position: vec4f, @location(0) intensity: f32 }
@vertex fn vertex(
    @location(0) position: vec3f, @location(1) normal: vec3f,
    @location(2) world0: vec4f, @location(3) world1: vec4f,
    @location(4) world2: vec4f, @location(5) world3: vec4f,
    @location(6) normal0: vec4f, @location(7) normal1: vec4f,
    @location(8) normal2: vec4f,
) -> VertexOut {
    var out: VertexOut;
    let world = mat4x4f(world0, world1, world2, world3);
    let transform_normal = mat3x3f(normal0.xyz, normal1.xyz, normal2.xyz);
    let n = transform_normal * normal;
    let unit = n * inverseSqrt(max(dot(n, n), 1e-16));
    out.position = camera.view_projection * world * vec4f(position, 1.0);
    out.intensity = 0.25 + 0.65 * abs(dot(unit, normalize(vec3f(1, 2, 3))));
    return out;
}
@fragment fn fragment(in: VertexOut) -> @location(0) vec4f {
    return vec4f(vec3f(in.intensity), 1.0);
}
