// Placement and coverage for the initial text route; contract in text/mod.rs.
struct Frame { matrix: mat4x4f, color: vec4f }
@group(0) @binding(0) var<uniform> frame: Frame;
// Entity world x layout-to-local: 100 logical px per local unit, layout Y down.
@group(1) @binding(0) var<uniform> node: mat4x4f;
@group(2) @binding(0) var atlas: texture_2d<f32>;
@group(2) @binding(1) var bilinear: sampler;
// rect: padded layout rectangle (px, Y down); texels: the same rectangle in the atlas.
struct Glyph { @location(0) rect: vec4f, @location(1) texels: vec4f }
struct Varyings {
    @builtin(position) position: vec4f,
    @location(0) @interpolate(perspective, centroid) texel: vec2f,
    @location(1) @interpolate(flat) texels: vec4f,
}
@vertex fn vertex(@builtin(vertex_index) index: u32, glyph: Glyph) -> Varyings {
    // Two triangles over the padded rectangle; layout and texel corners agree.
    let corners = array<vec2f, 6>(vec2f(0, 0), vec2f(1, 0), vec2f(0, 1), vec2f(0, 1), vec2f(1, 0), vec2f(1, 1));
    let corner = corners[index];
    var out: Varyings;
    out.position = frame.matrix * node * vec4f(mix(glyph.rect.xy, glyph.rect.zw, corner), 0, 1);
    out.texel = mix(glyph.texels.xy, glyph.texels.zw, corner);
    out.texels = glyph.texels;
    return out;
}
@fragment fn fragment(input: Varyings) -> @location(0) vec4f {
    // Pixel footprint in atlas texels, evaluated in uniform control flow.
    let dx = dpdx(input.texel);
    let dy = dpdy(input.texel);
    // One bilinear tap up to a 1.25-texel footprint, then an NxN box filter
    // (N <= 4) across it. Taps stay within this glyph's padded texels, so atlas
    // neighbours never bleed in; stronger minification still aliases.
    let taps = u32(clamp(ceil(max(length(dx), length(dy)) - 0.25), 1.0, 4.0));
    let size = vec2f(textureDimensions(atlas));
    let low = input.texels.xy + 0.5;
    let high = input.texels.zw - 0.5;
    var sum = 0.0;
    for (var j = 0u; j < taps; j++) {
        for (var i = 0u; i < taps; i++) {
            let offset = (vec2f(f32(i), f32(j)) + 0.5) / f32(taps) - 0.5;
            let at = clamp(input.texel + dx * offset.x + dy * offset.y, low, high);
            sum += textureSampleLevel(atlas, bilinear, at / size, 0.0).r;
        }
    }
    // Linear area coverage over unlit linear radiance, premultiplied.
    let coverage = sum / f32(taps * taps);
    return vec4f(frame.color.rgb * coverage, coverage);
}
