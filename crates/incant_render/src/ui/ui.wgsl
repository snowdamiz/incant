// Screen-space game UI over the display output; contract in ui/mod.rs.
struct Frame {
    // Physical [x, y, width, height] of the logical viewport inside the target.
    region: vec4f,
    // Physical pixels per logical pixel, then the physical target (surface) size.
    scale: vec2f,
    surface: vec2f,
    // x = 1 for UNORM targets: blend the encoded sRGB values directly.
    encode: vec4f,
}
@group(0) @binding(0) var<uniform> frame: Frame;
@group(1) @binding(0) var atlas: texture_2d<f32>;
@group(1) @binding(1) var bilinear: sampler;
struct Quad {
    @location(0) rect: vec4f,
    @location(1) clip: vec4f,
    @location(2) extra: vec4f,
    @location(3) pen: vec2f,
    @location(4) color: vec4f,
    @location(5) kind: u32,
}
struct Varyings {
    @builtin(position) position: vec4f,
    @location(0) texel: vec2f,
    @location(1) @interpolate(flat) texels: vec4f,
    @location(2) @interpolate(flat) rect: vec4f,
    @location(3) @interpolate(flat) shape: vec2f,
    @location(4) @interpolate(flat) color: vec4f,
    @location(5) @interpolate(flat) kind: u32,
}
fn snap(v: vec2f) -> vec2f {
    return floor(v + 0.5);
}
fn physical(logical: vec2f) -> vec2f {
    return frame.region.xy + logical * frame.scale;
}
@vertex fn vertex(@builtin(vertex_index) index: u32, quad: Quad) -> Varyings {
    let corners = array<vec2f, 6>(vec2f(0, 0), vec2f(1, 0), vec2f(0, 1), vec2f(0, 1), vec2f(1, 0), vec2f(1, 1));
    var p0: vec2f;
    var p1: vec2f;
    if quad.kind == 2u {
        // The pen snaps to a physical pixel; texels follow at `scale` pixels each.
        let pen = snap(physical(quad.pen));
        p0 = pen + quad.rect.xy * frame.scale;
        p1 = pen + quad.rect.zw * frame.scale;
    } else {
        p0 = snap(physical(quad.rect.xy));
        p1 = snap(physical(quad.rect.zw));
    }
    // Intersect with the snapped clip and the viewport region; empty stays empty.
    let c0 = max(snap(physical(quad.clip.xy)), snap(frame.region.xy));
    let c1 = min(snap(physical(quad.clip.zw)), snap(frame.region.xy + frame.region.zw));
    let low = max(p0, c0);
    let high = max(min(p1, c1), low);
    let q = mix(low, high, corners[index]);
    var out: Varyings;
    out.position = vec4f(q.x / frame.surface.x * 2.0 - 1.0, 1.0 - q.y / frame.surface.y * 2.0, 0.0, 1.0);
    out.texel = mix(quad.extra.xy, quad.extra.zw, (q - p0) / max(p1 - p0, vec2f(1e-6)));
    out.texels = quad.extra;
    out.rect = vec4f(p0, p1);
    let unit = min(frame.scale.x, frame.scale.y);
    out.shape = vec2f(quad.extra.x * unit, max(1.0, floor(quad.extra.y * unit + 0.5)));
    out.color = quad.color;
    out.kind = quad.kind;
    return out;
}
fn rounded(p: vec2f, half: vec2f, radius: f32) -> f32 {
    let q = abs(p) - half + vec2f(radius);
    return length(max(q, vec2f(0))) + min(max(q.x, q.y), 0.0) - radius;
}
fn decode(c: vec3f) -> vec3f {
    return select(pow((c + 0.055) / 1.055, vec3f(2.4)), c / 12.92, c <= vec3f(0.04045));
}
@fragment fn fragment(input: Varyings) -> @location(0) vec4f {
    // Derivatives in uniform control flow; rectangles carry a constant texel.
    let dx = dpdx(input.texel);
    let dy = dpdy(input.texel);
    var coverage = 1.0;
    if input.kind == 2u {
        // One bilinear tap up to a 1.25-texel footprint, then an NxN box
        // (N <= 4) clamped to this glyph's padded texels, as for world text.
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
        coverage = sum / f32(taps * taps);
    } else {
        // Rounded-rectangle area coverage at the pixel centre; snapped straight
        // edges are exact, corners are antialiased over one physical pixel.
        let p = input.position.xy;
        let center = (input.rect.xy + input.rect.zw) * 0.5;
        let half = max((input.rect.zw - input.rect.xy) * 0.5, vec2f(0));
        let radius = min(input.shape.x, min(half.x, half.y));
        coverage = clamp(0.5 - rounded(p - center, half, radius), 0.0, 1.0);
        if input.kind == 1u {
            let inner = half - vec2f(input.shape.y);
            if all(inner > vec2f(0)) {
                let hole = clamp(0.5 - rounded(p - center, inner, max(radius - input.shape.y, 0.0)), 0.0, 1.0);
                coverage = max(coverage - hole, 0.0);
            }
        }
    }
    var color = input.color.rgb;
    if frame.encode.x == 0.0 {
        color = decode(color);
    }
    let alpha = input.color.a * coverage;
    return vec4f(color * alpha, alpha);
}
