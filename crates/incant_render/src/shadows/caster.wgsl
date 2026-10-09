struct Material { base:vec4f, emissive_roughness:vec4f, factors:vec4f, flags:vec4u }
@group(0) @binding(0) var<uniform> view_projection:mat4x4f;
@group(1) @binding(0) var<uniform> material:Material;
@group(1) @binding(1) var base_map:texture_2d<f32>;
@group(1) @binding(2) var base_sampler:sampler;
struct VertexOut { @builtin(position) position:vec4f, @location(0) uv:vec2f }
@vertex fn vertex(@location(0) position:vec3f,
    @location(2) w0:vec4f,@location(3) w1:vec4f,@location(4) w2:vec4f,@location(5) w3:vec4f,
    @location(9) uv:vec2f) -> VertexOut {
    var out:VertexOut;
    out.position=view_projection*mat4x4f(w0,w1,w2,w3)*vec4f(position,1.0);
    out.uv=uv;return out;
}
@fragment fn fragment(in:VertexOut) {
    let alpha=material.base.a*textureSample(base_map,base_sampler,in.uv).a;
    if alpha<material.factors.w { discard; }
}
