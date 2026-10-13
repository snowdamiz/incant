// Added only to the opt-in output variant. Original tone/chrome/transfer stay intact.
@group(0) @binding(2) var bloom_texture: texture_2d<f32>;
@group(0) @binding(3) var<uniform> bloom_gain: vec4f;
fn bloom_light(position: vec2f) -> vec3f {
    let size = vec2i(textureDimensions(bloom_texture));
    let p = position / bloom_gain.yz * vec2f(size) - vec2f(0.5);
    let base = vec2i(floor(p));
    let f = fract(p);
    let a = textureLoad(bloom_texture,clamp(base,vec2i(0),size-vec2i(1)),0).rgb;
    let b = textureLoad(bloom_texture,clamp(base+vec2i(1,0),vec2i(0),size-vec2i(1)),0).rgb;
    let c = textureLoad(bloom_texture,clamp(base+vec2i(0,1),vec2i(0),size-vec2i(1)),0).rgb;
    let d = textureLoad(bloom_texture,clamp(base+vec2i(1,1),vec2i(0),size-vec2i(1)),0).rgb;
    return clamp(mix(mix(a,b,f.x),mix(c,d,f.x),f.y)*bloom_gain.x,vec3f(0),vec3f(65504));
}
fn bloom_color(radiance: vec3f, coverage: f32, position: vec2f) -> vec3f {
    let light = bloom_light(position);
    // Scene-linear addition on covered pixels; additive display halo over the
    // backdrop elsewhere. This coverage policy never invents material alpha.
    return clamp(mix(display.backdrop.rgb+tone_map(light),
        tone_map(clamp(radiance+light,vec3f(0),vec3f(65504))),coverage),vec3f(0),vec3f(1));
}
