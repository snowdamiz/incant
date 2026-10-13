// Fixed area reductions followed by normalized multiscale reconstruction.
@group(0) @binding(0) var source: texture_2d<f32>;
@group(0) @binding(1) var coarse: texture_2d<f32>;
struct Settings { rect: vec4f, sizes: vec4u, params: vec4f }
@group(0) @binding(2) var<uniform> settings: Settings;

@vertex fn vertex(@builtin(vertex_index) index: u32) -> @builtin(position) vec4f {
    let positions = array<vec2f, 3>(vec2f(-1,-1),vec2f(3,-1),vec2f(-1,3));
    return vec4f(positions[index],0,1);
}

fn extract(p: vec2i) -> vec3f {
    let center = vec2f(p) + vec2f(0.5);
    if any(center < settings.rect.xy) || any(center >= settings.rect.xy + settings.rect.zw) {
        return vec3f(0);
    }
    let hdr = textureLoad(source,p,0);
    let alpha = clamp(hdr.a,0.0,1.0);
    // Transparent residual RGB is never a light source, even for malformed
    // premultiplied content. Valid input otherwise has H = alpha * R.
    if alpha == 0.0 { return vec3f(0); }
    let radiance = clamp(hdr.rgb / max(alpha,1e-8),vec3f(0),vec3f(65504));
    let luminance = dot(radiance,vec3f(0.2126,0.7152,0.0722));
    let gain = max(luminance-settings.params.x,0.0)/max(luminance,1e-8);
    return clamp(hdr.rgb,vec3f(0),vec3f(65504)) * gain;
}

fn reduce(position: vec2f, extraction: bool) -> vec4f {
    let pixel = floor(position);
    // First extraction uses the unrounded scene span of the physical output.
    // Later passes use their complete, center-aligned pyramid extents.
    let ratio = settings.params.zw / vec2f(settings.sizes.zw);
    let lo = pixel * ratio;
    let hi = (pixel + vec2f(1)) * ratio;
    let first = vec2i(floor(lo));
    var total = vec3f(0);
    // Output-anchored ceil-half footprints span <=2 source texels at any
    // admitted DRS scale (<=1), hence at most three intersections per axis.
    // This includes fractional final source texels and all later odd levels.
    let area = (hi.x-lo.x)*(hi.y-lo.y);
    for (var y=0; y<3; y++) {
        for (var x=0; x<3; x++) {
            let p = first + vec2i(x,y);
            if all(p >= vec2i(0)) && all(p < vec2i(settings.sizes.xy)) {
                let overlap = max(min(hi,vec2f(p)+vec2f(1))-max(lo,vec2f(p)),vec2f(0));
                let weight = overlap.x*overlap.y/area;
                if weight > 0.0 {
                    var value = textureLoad(source,p,0).rgb;
                    if extraction { value = extract(p); }
                    total += clamp(value,vec3f(0),vec3f(65504)) * weight;
                }
            }
        }
    }
    return vec4f(clamp(total,vec3f(0),vec3f(65504)),1);
}
@fragment fn first(@builtin(position) p: vec4f) -> @location(0) vec4f { return reduce(p.xy,true); }
@fragment fn down(@builtin(position) p: vec4f) -> @location(0) vec4f { return reduce(p.xy,false); }

fn sample_coarse(uv: vec2f) -> vec3f {
    let size = vec2i(textureDimensions(coarse));
    let p = uv * vec2f(size) - vec2f(0.5);
    let base = vec2i(floor(p));
    let f = fract(p);
    let a = textureLoad(coarse,clamp(base,vec2i(0),size-vec2i(1)),0).rgb;
    let b = textureLoad(coarse,clamp(base+vec2i(1,0),vec2i(0),size-vec2i(1)),0).rgb;
    let c = textureLoad(coarse,clamp(base+vec2i(0,1),vec2i(0),size-vec2i(1)),0).rgb;
    let d = textureLoad(coarse,clamp(base+vec2i(1,1),vec2i(0),size-vec2i(1)),0).rgb;
    return mix(mix(a,b,f.x),mix(c,d,f.x),f.y);
}
@fragment fn up(@builtin(position) p: vec4f) -> @location(0) vec4f {
    let fine = textureLoad(source,vec2i(p.xy),0).rgb;
    let low = sample_coarse(p.xy / vec2f(settings.sizes.zw));
    // Equal weight for every level; the already-normalized coarse chain contains
    // n-1 levels. This convex combination preserves a constant field for any n.
    let result = mix(fine,low,settings.params.y);
    return vec4f(clamp(result,vec3f(0),vec3f(65504)),1);
}
