// All coordinates are pixel centers in top-left rectangles; jitter is UV in
// the corresponding current/previous INTERNAL viewport, never attachment size.
struct Frame { source:vec4f, output:vec4f, depth:vec4f, options:vec4f, reactive:vec4f, jitter:vec4f }
@group(0) @binding(0) var<uniform> frame:Frame;
@vertex fn vertex(@builtin(vertex_index) i:u32)->@builtin(position) vec4f {
    let uv=vec2f(f32((i<<1u)&2u),f32(i&2u));
    return vec4f(uv*2.0-1.0,0.0,1.0);
}
// Classify bits before arithmetic: WGSL finite-math assumptions make v==v
// or abs(v)<=MAX unsuitable as a NaN/Inf boundary proof.
fn finite4(v:vec4f)->bool {return all((bitcast<vec4u>(v)&vec4u(0x7f800000u))!=vec4u(0x7f800000u));}
fn admitted(v:vec4f)->bool {return finite4(v) && all(v>=vec4f(0.0)) && v.a<=1.0;}
fn clean(v:vec4f)->vec4f {
    if !finite4(v) || v.a<=0.0 {return vec4f(0.0);}
    return vec4f(clamp(v.rgb,vec3f(0.0),vec3f(65504.0)),clamp(v.a,0.0,1.0));
}
fn inside(p:vec2f,r:vec4f)->bool {return all(p>=r.xy) && all(p<r.xy+r.zw);}
fn lo(r:vec4f)->vec2i {return vec2i(ceil(r.xy-vec2f(0.5)));}
fn hi(r:vec4f)->vec2i {return vec2i(ceil(r.xy+r.zw-vec2f(0.5)))-vec2i(1);}
fn source_position(p:vec2f)->vec2f {
    let u=(p-frame.output.xy)/frame.output.zw;
    return frame.source.xy+(u+frame.jitter.xy)*frame.source.zw;
}
fn linear_depth(d:f32)->f32 {
    if !(d>=0.0 && d<1.0) {return 0.0;}
    if frame.depth.z>0.5 {return (1.0-d)*frame.depth.x+d*frame.depth.y;}
    return 1.0/((1.0-d)/frame.depth.x+d/frame.depth.y);
}
fn compatible(a:f32,b:f32)->bool {
    return a>0.0 && b>0.0 && a<=frame.depth.y && b<=frame.depth.y
        && abs(a-b)<=max(frame.options.z,frame.options.y*b);
}
fn tap_weight(f:vec2f,x:i32,y:i32)->f32 {
    return select(1.0-f.x,f.x,x==1)*select(1.0-f.y,f.y,y==1);
}
