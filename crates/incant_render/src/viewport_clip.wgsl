// Mathematical composition mask. Colors and radii are supplied by the Claude-owned UI.
struct Clip {
    rect: vec4f,
    radii: vec4f,
    canvas: vec4f,
}
@group(0) @binding(0) var<uniform> clip: Clip;

@vertex fn vertex(@builtin(vertex_index) index: u32) -> @builtin(position) vec4f {
    let positions = array<vec2f, 3>(vec2f(-1, -1), vec2f(3, -1), vec2f(-1, 3));
    return vec4f(positions[index], 0, 1);
}

@fragment fn fragment(@builtin(position) position: vec4f) -> @location(0) vec4f {
    let half_size = clip.rect.zw * 0.5;
    let p = position.xy - clip.rect.xy - half_size;
    let top = select(clip.radii.x, clip.radii.y, p.x > 0);
    let bottom = select(clip.radii.w, clip.radii.z, p.x > 0);
    let radius = min(select(top, bottom, p.y > 0), min(half_size.x, half_size.y));
    let q = abs(p) - half_size + vec2f(radius);
    let distance = length(max(q, vec2f(0))) + min(max(q.x, q.y), 0) - radius;
    let coverage = smoothstep(-0.5, 0.5, distance);
    if coverage == 0 { discard; }
    return vec4f(clip.canvas.rgb, coverage);
}
