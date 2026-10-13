// All coordinates are pixel centers in top-left rectangles; jitter is UV in
// the corresponding current/previous INTERNAL viewport, never attachment size.
struct Frame { source:vec4f, output:vec4f, depth:vec4f, options:vec4f, reactive:vec4f, jitter:vec4f }
@group(0) @binding(0) var<uniform> frame:Frame;
@vertex fn vertex(@builtin(vertex_index) i:u32)->@builtin(position) vec4f {
    let uv=vec2f(f32((i<<1u)&2u),f32(i&2u));
    return vec4f(uv*2.0-1.0,0.0,1.0);
}
fn finite4(v:vec4f)->bool {return all(v==v) && all(abs(v)<=vec4f(3.402823e38));}
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
// Catmull-Rom kernel at texel distance d. The four taps at offsets -1..2 from
// floor(q) sum to one and interpolate; callers clamp the result to the central
// 2x2 (bilinear) range, so it adds no new extrema.
fn catmull_rom(d:f32)->f32 {
    let x=abs(d);
    if x<1.0 {return (1.5*x-2.5)*x*x+1.0;}
    if x<2.0 {return ((-0.5*x+2.5)*x-4.0)*x+2.0;}
    return 0.0;
}
// History fraction. It never exceeds `limit`, the existing reactive-reduced
// history weight. reactive.y is the host's expected admitted history evidence in
// frames of unit evidence; only below h/(1-h), after a cut or a source-scale
// increase, does it lower the fraction to mass/(mass+1).
fn history_fraction(limit:f32,reaction:f32)->f32 {
    let w=clamp(select(0.0,frame.reactive.y,frame.reactive.y>0.0),0.0,1048576.0);
    let mass=w*(1.0-frame.reactive.x*reaction);
    return min(limit,mass/(mass+1.0));
}
