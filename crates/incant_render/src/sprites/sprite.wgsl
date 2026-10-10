@group(0) @binding(0) var<uniform> camera: mat4x4f;
@group(1) @binding(0) var image: texture_2d<f32>;
@group(1) @binding(1) var filtering: sampler;
struct Input { @location(0) position:vec3f, @location(1) uv:vec2f, @location(2) tint:vec4f, @location(3) uv_bounds:vec4f }
struct Output { @builtin(position) position:vec4f, @location(0) uv:vec2f, @location(1) tint:vec4f, @location(2) @interpolate(flat) uv_bounds:vec4f }
@vertex fn vertex(input:Input)->Output {
    var out:Output;
    out.position=camera*vec4f(input.position,1);out.uv=input.uv;out.tint=input.tint;out.uv_bounds=input.uv_bounds;
    return out;
}
@fragment fn fragment(input:Output)->@location(0) vec4f {
    // Premultiplication happened before texture filtering. LOD zero and region
    // clamping prevent adjacent source-sheet frames from bleeding into this one.
    let pixel=textureSampleLevel(image,filtering,clamp(input.uv,input.uv_bounds.xy,input.uv_bounds.zw),0);
    return vec4f(pixel.rgb*input.tint.rgb*input.tint.a,pixel.a*input.tint.a);
}
