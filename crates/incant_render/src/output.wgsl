// Transfer/composition math. tone_map is supplied by Claude's look-dev snippet.
@group(0) @binding(0) var scene: texture_2d<f32>;
struct Output { backdrop: vec4f, rect: vec4f, radii: vec4f, canvas: vec4f }
@group(0) @binding(1) var<uniform> display: Output;
@vertex fn vertex(@builtin(vertex_index) index: u32) -> @builtin(position) vec4f {
    let positions = array<vec2f, 3>(vec2f(-1,-1),vec2f(3,-1),vec2f(-1,3));
    return vec4f(positions[index],0,1);
}
fn srgb_encode(value: vec3f) -> vec3f {
    return select(1.055*pow(value,vec3f(1.0/2.4))-0.055,
        12.92*value, value<=vec3f(0.0031308));
}
@fragment fn fragment(@builtin(position) position: vec4f) -> @location(0) vec4f {
    let hdr = textureLoad(scene,vec2i(position.xy),0);
    let coverage = clamp(hdr.a,0.0,1.0);
    // The HDR pass stores the scene over transparent black. Unpremultiply only
    // after all scene layers blend; keep display-referred editor colors outside
    // the scene transform. Opaque scene pixels have coverage=1.
    let radiance = clamp(hdr.rgb/max(coverage,1e-8),vec3f(0),vec3f(65504));
    var color = mix(display.backdrop.rgb,tone_map(radiance),coverage);
    if display.canvas.a != 0.0 {
        color=mix(color,display.canvas.rgb,chrome_coverage(position.xy));
    }
    if display.backdrop.a != 0.0 { color=srgb_encode(color); }
    return vec4f(color,1);
}

// Same native rounded-rectangle mask, composed in linear display space.
fn chrome_coverage(position: vec2f) -> f32 {
    let half_size = display.rect.zw * 0.5;
    let p = position - display.rect.xy - half_size;
    let top = select(display.radii.x, display.radii.y, p.x > 0);
    let bottom = select(display.radii.w, display.radii.z, p.x > 0);
    let radius = min(select(top, bottom, p.y > 0), min(half_size.x, half_size.y));
    let q = abs(p) - half_size + vec2f(radius);
    let distance = length(max(q, vec2f(0))) + min(max(q.x, q.y), 0) - radius;
    let coverage = smoothstep(-0.5, 0.5, distance);
    return coverage;
}
