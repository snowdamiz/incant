// Top-left normalized opaque-surface motion; temporal projections include jitter.
struct Frame { current:mat4x4f, previous:mat4x4f, rect:vec4f }
struct Material { base:vec4f, emissive_roughness:vec4f, factors:vec4f, flags:vec4u }
@group(0) @binding(0) var<uniform> frame:Frame;
@group(1) @binding(0) var<uniform> material:Material;
@group(1) @binding(1) var base_map:texture_2d<f32>;
@group(1) @binding(2) var base_sampler:sampler;
struct Output {
    @builtin(position) position:vec4f,
    @location(0) previous:vec4f,
    @location(1) uv:vec2f,
    @location(2) @interpolate(flat) valid:f32,
}
@vertex fn vertex(@location(0) position:vec3f,@location(1) uv:vec2f,
    @location(2) old_position:vec3f,
    @location(3) w0:vec4f,@location(4) w1:vec4f,@location(5) w2:vec4f,@location(6) w3:vec4f,
    @location(7) p0:vec4f,@location(8) p1:vec4f,@location(9) p2:vec4f,@location(10) p3:vec4f,
    @location(11) valid:vec4f) -> Output {
    var out:Output;
    out.position=frame.current*mat4x4f(w0,w1,w2,w3)*vec4f(position,1);
    out.previous=frame.previous*mat4x4f(p0,p1,p2,p3)*vec4f(old_position,1);
    out.uv=uv;out.valid=valid.x;return out;
}
fn mask(in:Output) {
    // Match canonical MASK UV sampling, multiplier, cutoff and derivative order.
    let base=material.base*textureSample(base_map,base_sampler,in.uv);
    if material.flags.x==1u && base.a<material.factors.w { discard; }
}
@fragment fn depth_only(in:Output) {mask(in);}
@fragment fn fragment(in:Output)->@location(0) vec4f {
    mask(in);
    // Comparisons also reject NaN; w and all homogeneous coordinates are bounded.
    if in.valid==0 || !(in.previous.w>0) || !all(abs(in.previous)<=vec4f(3.402823e38)) { return vec4f(0); }
    let old=in.previous.xyz/in.previous.w;
    if !all(abs(old)<=vec3f(3.402823e38)) || old.z<0 || old.z>1 || any(abs(old.xy)>vec2f(1)) { return vec4f(0); }
    let previous_uv=vec2f(old.x*0.5+0.5,0.5-old.y*0.5);
    if any(previous_uv<vec2f(0)) || any(previous_uv>=vec2f(1)) { return vec4f(0); }
    let current_uv=(in.position.xy-frame.rect.xy)/frame.rect.zw;
    return vec4f(previous_uv-current_uv,old.z,1);
}
