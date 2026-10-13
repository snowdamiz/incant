// Opaque premultiplied scene-linear HDR only. Alpha is one for opaque/MASK and
// zero for clear. History also retains fractional coverage; excluded surfaces
// are drawn after history publication and never enter this neighborhood.
struct Frame { rect:vec4f, depth:vec4f, options:vec4f, reactive:vec4f }
@group(0) @binding(0) var<uniform> frame:Frame;
@group(0) @binding(1) var current:texture_2d<f32>;
@group(0) @binding(2) var motion:texture_2d<f32>;
@group(0) @binding(3) var previous_depth:texture_depth_2d;
@group(0) @binding(4) var previous_color:texture_2d<f32>;
@vertex fn vertex(@builtin(vertex_index) i:u32)->@builtin(position) vec4f {
    let uv=vec2f(f32((i<<1u)&2u),f32(i&2u));
    return vec4f(uv*2.0-1.0,0.0,1.0);
}
fn finite4(v:vec4f)->bool { return all(v==v) && all(abs(v)<=vec4f(3.402823e38)); }
fn clean(v:vec4f)->vec4f {
    if !finite4(v) || v.a<=0.0 { return vec4f(0.0); }
    return vec4f(clamp(v.rgb,vec3f(0.0),vec3f(65504.0)),clamp(v.a,0.0,1.0));
}
fn view_depth(d:f32)->f32 {
    if frame.depth.z>0.5 { return (1.0-d)*frame.depth.x+d*frame.depth.y; }
    return 1.0/((1.0-d)/frame.depth.x+d/frame.depth.y);
}
fn valid_motion(m:vec4f)->bool {
    return finite4(m) && m.w>=0.5 && m.z>=0.0 && m.z<=1.0;
}
@fragment fn resolve(@builtin(position) p:vec4f)->@location(0) vec4f {
    let pixel=vec2i(p.xy);
    let raw=textureLoad(current,pixel,0); let c=clean(raw);
    let end=frame.rect.xy+frame.rect.zw;
    if frame.depth.w<0.5 || !finite4(raw)
        || any(p.xy<frame.rect.xy) || any(p.xy>=end) { return c; }
    let center=textureLoad(motion,pixel,0);
    // New/invalid opaque owners must not acquire another owner's history. Clear
    // has no motion record and is instead the far-plane coverage surface.
    if c.a>0.0 && !valid_motion(center) { return c; }
    // Only pixel centers in this viewport are eligible, including offset/odd rects.
    let lo=vec2i(ceil(frame.rect.xy-vec2f(0.5)));
    let hi=vec2i(ceil(end-vec2f(0.5)))-vec2i(1);
    if any(hi<lo) { return c; }
    var minimum=c; var maximum=c;
    var near=frame.depth.y; var far=frame.depth.x;
    var m=vec4f(0.0,0.0,1.0,1.0); var closest=2.0;
    if c.a>0.0 { m=center; closest=center.z; }
    // Exactly nine current samples: range admission keeps both sides of a
    // silhouette. The nearest valid expected-depth motion dilates foreground
    // reprojection by at most one pixel; pure background disocclusion rejects it.
    for(var y=-1;y<=1;y++) { for(var x=-1;x<=1;x++) {
        let tap=clamp(pixel+vec2i(x,y),lo,hi);
        let value=textureLoad(current,tap,0);
        if !finite4(value) { continue; }
        let n=clean(value);
        minimum=min(minimum,n); maximum=max(maximum,n);
        if n.a==0.0 { far=frame.depth.y; continue; }
        let candidate=textureLoad(motion,tap,0);
        if !valid_motion(candidate) { continue; }
        let z=view_depth(candidate.z);
        near=min(near,z); far=max(far,z);
        // Strict comparison keeps center motion on equal-depth interiors.
        if candidate.z<closest { m=candidate; closest=candidate.z; }
    }}
    if far<near { return c; }
    near-=max(frame.options.z,frame.options.y*near);
    far+=max(frame.options.z,frame.options.y*far);
    let old=p.xy+m.xy*frame.rect.zw;
    if any(old<frame.rect.xy) || any(old>=end) { return c; }
    let q=old-vec2f(0.5); let base=vec2i(floor(q)); let f=fract(q);
    var h=vec4f(0.0); var mass=0.0;
    for(var y=0;y<2;y++) { for(var x=0;x<2;x++) {
        let wx=select(1.0-f.x,f.x,x==1); let wy=select(1.0-f.y,f.y,y==1);
        let weight=wx*wy; if !(weight>0.0) {continue;}
        let tap=base+vec2i(x,y);
        if any(tap<lo) || any(tap>hi) {continue;}
        let d=textureLoad(previous_depth,tap,0);
        let color=textureLoad(previous_color,tap,0);
        if !(d>=0.0 && d<=1.0) || !finite4(color)
            || any(color<vec4f(0.0)) || color.a>1.0 {continue;}
        let z=view_depth(d);
        if z<near || z>far {continue;}
        h+=clean(color)*weight; mass+=weight;
    }}
    // Reject disocclusion only when no finite, depth-compatible coverage support
    // survives. Never borrow clamped outside history or divide by zero.
    if !(mass>0.0) {return c;}
    h=clean(h/mass);
    // Measure reaction before clipping so an abrupt radiance change cannot hide
    // inside the clipped value. All arithmetic is bounded by finite half range.
    let change=abs(h.rgb-c.rgb);
    let reaction=clamp(max(change.x,max(change.y,change.z))/frame.options.w,0.0,1.0);
    let weight=frame.options.x*(1.0-frame.reactive.x*reaction);
    return clean(mix(c,clamp(h,minimum,maximum),weight));
}
