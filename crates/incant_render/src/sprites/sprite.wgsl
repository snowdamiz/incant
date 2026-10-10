@group(0) @binding(0) var<uniform> camera: mat4x4f;
@group(1) @binding(0) var image: texture_2d<f32>;
@group(1) @binding(1) var filtering: sampler;
struct Input { @location(0) position:vec3f, @location(1) uv:vec2f, @location(2) tint:vec4f, @location(3) uv_bounds:vec4f, @location(4) texel:vec2f, @location(5) sampling:u32 }
struct Output { @builtin(position) position:vec4f, @location(0) uv:vec2f, @location(1) tint:vec4f, @location(2) @interpolate(flat) uv_bounds:vec4f, @location(3) texel:vec2f, @location(4) @interpolate(flat) sampling:u32 }
@vertex fn vertex(input:Input)->Output {
    var out:Output;
    out.position=camera*vec4f(input.position,1);out.uv=input.uv;out.tint=input.tint;out.uv_bounds=input.uv_bounds;
    out.texel=input.texel;out.sampling=input.sampling;
    return out;
}
@fragment fn fragment(input:Output)->@location(0) vec4f {
    // Premultiplication happened before texture filtering. LOD zero and region
    // clamping prevent adjacent source-sheet frames from bleeding into this one.
    // Derivatives are evaluated in uniform control flow. At a nearest texel
    // boundary, select the side toward increasing screen coordinate on the
    // dominant axis (X wins equal slopes). A 1/1024-texel band defines
    // the tie tolerance for the supported 8192-texel region. It is unrelated
    // to an adapter's raster subpixel precision. Local texels avoid sheet-size
    // dependent rounding; textureLoad avoids hardware nearest tie choices.
    let dx=dpdx(input.texel);
    let dy=dpdy(input.texel);
    let direction=select(dy,dx,abs(dx)>=abs(dy));
    var pixel:vec4f;
    if input.sampling==0u {
        let nearest=vec2i(clamp(floor(input.texel+sign(direction)*(1.0/1024.0)),vec2f(0),input.uv_bounds.zw-vec2f(1)));
        pixel=textureLoad(image,vec2i(input.uv_bounds.xy)+nearest,0);
    } else {
        pixel=textureSampleLevel(image,filtering,clamp(input.uv,input.uv_bounds.xy,input.uv_bounds.zw),0);
    }
    return vec4f(pixel.rgb*input.tint.rgb*input.tint.a,pixel.a*input.tint.a);
}
