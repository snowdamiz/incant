// Resolve publishes color on the unjittered pixel grid, including registered
// current-only values on history refusal. Excluded surfaces then draw once.
struct Frame { rect:vec4f, depth:vec4f, options:vec4f, reactive:vec4f }
@group(0) @binding(0) var color:texture_2d<f32>;
@group(0) @binding(1) var<uniform> frame:Frame;
@vertex fn vertex(@builtin(vertex_index) i:u32)->@builtin(position) vec4f {
    let uv=vec2f(f32((i<<1u)&2u),f32(i&2u));
    return vec4f(uv*2.0-1.0,0.0,1.0);
}
@fragment fn copy(@builtin(position) p:vec4f)->@location(0) vec4f {
    let end=frame.rect.xy+frame.rect.zw;
    if any(p.xy<frame.rect.xy) || any(p.xy>=end) {return textureLoad(color,vec2i(p.xy),0);}
    let c=textureLoad(color,vec2i(p.xy),0);
    if all(c==c) && all(abs(c)<=vec4f(65504.0)) && c.a>0.0 {
        return vec4f(max(c.rgb,vec3f(0.0)),clamp(c.a,0.0,1.0));
    }
    return vec4f(0.0);
}
