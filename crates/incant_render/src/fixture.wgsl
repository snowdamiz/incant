// Diagnostic geometry for Phase 0 surface validation. Look-dev belongs to Claude.
struct VertexOut { @builtin(position) position: vec4f, @location(0) intensity: f32 }
@vertex fn vertex(@location(0) position: vec4f, @location(1) normal: vec3f) -> VertexOut {
    var out: VertexOut;
    out.position = position;
    out.intensity = 0.25 + 0.65 * abs(dot(normalize(normal), normalize(vec3f(1, 2, 3))));
    return out;
}
@fragment fn fragment(in: VertexOut) -> @location(0) vec4f {
    return vec4f(vec3f(in.intensity), 1.0);
}
