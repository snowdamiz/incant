// History stays paired with jittered motion/depth. Only presentation removes
// current jitter; excluded surfaces subsequently use their unjittered depth.
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
    let lo=vec2i(ceil(frame.rect.xy-vec2f(0.5)));
    let hi=vec2i(ceil(end-vec2f(0.5)))-vec2i(1);
    if any(hi<lo) {return vec4f(0.0);}
    let q=p.xy+frame.reactive.yz-vec2f(0.5);
    let base=vec2i(floor(q)); let f=fract(q);
    var result=vec4f(0.0);
    for(var y=0;y<2;y++) {for(var x=0;x<2;x++) {
        let w=select(1.0-f.x,f.x,x==1)*select(1.0-f.y,f.y,y==1);
        if !(w>0.0) {continue;}
        let c=textureLoad(color,clamp(base+vec2i(x,y),lo,hi),0);
        if all(c==c) && all(abs(c)<=vec4f(65504.0)) && c.a>0.0 {
            result+=vec4f(max(c.rgb,vec3f(0.0)),clamp(c.a,0.0,1.0))*w;
        }
    }}
    return result;
}
