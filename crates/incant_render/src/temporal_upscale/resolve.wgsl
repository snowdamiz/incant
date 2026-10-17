// GPU-only color reconstruction. Mathematical basis/attribution: gpu-color-taau.md.
// All UVs are top-left. Input motion.xy = previous jittered UV - current jittered
// UV; motion.z is EXPECTED PREVIOUS depth, never current depth for dilation.
// Color/support history is output-sized and UNJITTERED. Jitter is right/down
// internal pixels. Unjitter velocity exactly once before output reprojection.
struct Frame { source:vec4f, output:vec4f, depth:vec4f, jitter:vec4f }
@group(0) @binding(0) var<uniform> frame:Frame;
@group(0) @binding(1) var current:texture_2d<f32>;
@group(0) @binding(2) var motion:texture_2d<f32>;
@group(0) @binding(3) var current_depth:texture_depth_2d;
@group(0) @binding(4) var previous_color:texture_2d<f32>;
@group(0) @binding(5) var previous_support:texture_2d<f32>;
@vertex fn vertex(@builtin(vertex_index) i:u32)->@builtin(position) vec4f {
    let uv=vec2f(f32((i<<1u)&2u),f32(i&2u)); return vec4f(uv*2.0-1.0,0.0,1.0);
}
fn finite4(v:vec4f)->bool { return all(v==v) && all(abs(v)<=vec4f(3.402823e38)); }
fn clean(v:vec4f)->vec4f {
    if !finite4(v) || v.a<=0.0 { return vec4f(0.0); }
    let a=clamp(v.a,0.0,1.0);
    return vec4f(clamp(v.rgb,vec3f(0.0),vec3f(65504.0*a)),a);
}
fn ycocg(c:vec3f)->vec3f { return vec3f(dot(c,vec3f(0.25,0.5,0.25)),(c.r-c.b)*0.5,(-c.r+2.0*c.g-c.b)*0.25); }
fn rgb(c:vec3f)->vec3f { return vec3f(c.x+c.y-c.z,c.x+c.z,c.x-c.y-c.z); }
fn view_depth(d:f32)->f32 {
    if frame.depth.z>0.5 { return mix(frame.depth.x,frame.depth.y,d); }
    return 1.0/((1.0-d)/frame.depth.x+d/frame.depth.y);
}
fn valid_motion(m:vec4f)->bool { return finite4(m) && m.w>=0.5 && m.z>=0.0 && m.z<=1.0; }
fn lanczos(x:f32)->f32 {
    if abs(x)<1e-5 { return 1.0; }
    if abs(x)>=2.0 { return 0.0; }
    let p=3.14159265359*x; return sin(p)*sin(p*0.5)/(p*p*0.5);
}
fn cubic(x:f32)->f32 {
    let a=abs(x);
    if a<1.0 { return 1.5*a*a*a-2.5*a*a+1.0; }
    if a<2.0 { return -0.5*a*a*a+2.5*a*a-4.0*a+2.0; }
    return 0.0;
}
struct Resolved { @location(0) color:vec4f, @location(1) support:vec2f }
@fragment fn resolve(@builtin(position) p:vec4f)->Resolved {
    let uv=p.xy/frame.output.xy;
    // Geometry is shifted by positive jitter in input texels. Sampling this
    // shifted location reconstructs the unjittered output location.
    let q=uv*frame.source.zw+frame.source.xy+frame.jitter.xy;
    let center=vec2i(floor(q));
    let lo=vec2i(frame.source.xy); let hi=lo+vec2i(frame.source.zw)-vec2i(1);
    let center_tap=clamp(center,lo,hi);
    let center_color=clean(textureLoad(current,center_tap,0));
    let center_motion=textureLoad(motion,center_tap,0);
    var sum=vec4f(0.0); var mass=0.0;
    var mean=vec3f(0.0); var moment=vec3f(0.0);
    var minimum=vec4f(65504.0); var maximum=vec4f(0.0);
    var nearest=1.0; var chosen=vec4f(0.0); var found=false;
    var malformed=false;
    for(var y=-1;y<=1;y++) { for(var x=-1;x<=1;x++) {
        let raw_tap=center+vec2i(x,y); let tap=clamp(raw_tap,lo,hi);
        let raw=textureLoad(current,tap,0); let c=clean(raw);
        malformed=malformed || !finite4(raw);
        let delta=vec2f(raw_tap)+vec2f(0.5)-q;
        let w=lanczos(delta.x)*lanczos(delta.y);
        sum+=c*w;mass+=w;
        minimum=min(minimum,c);maximum=max(maximum,c);
        let v=ycocg(c.rgb); mean+=v;moment+=v*v;
        let d=textureLoad(current_depth,tap,0);
        let m=textureLoad(motion,tap,0);
        // Depth is actual CURRENT depth from the matching single-sample owner.
        if c.a>0.0 && d>=0.0 && d<1.0 && (!found || d<nearest) {
            nearest=d;chosen=m;found=true;
        }
    }}
    var c=center_color;
    if abs(mass)>1e-5 { c=clean(clamp(sum/mass,minimum,maximum)); }
    var result=Resolved(c,vec2f(nearest,select(0.0,1.0,found)));
    // Never let a neighboring valid owner supply history to an invalid/new
    // center owner. Clear pixels can reconstruct fractional edge coverage.
    if frame.depth.w<0.5 || malformed || !found || !valid_motion(chosen)
        || (center_color.a>0.0 && !valid_motion(center_motion)) { return result; }
    let velocity=chosen.xy+(frame.jitter.xy-frame.jitter.zw)/frame.source.zw;
    let old=(uv+velocity)*frame.output.xy-vec2f(0.5);
    let base=vec2i(floor(old));
    let expected=view_depth(chosen.z);
    let tolerance=max(0.001,0.01*expected);
    // Geometric confidence has positive bilinear support, independent of the
    // signed color reconstruction footprint. Missing support retains its weight
    // as lost confidence; do not renormalize the compatible subset.
    let f=fract(old); var confidence=0.0;
    for(var y=0;y<2;y++) { for(var x=0;x<2;x++) {
        let w=select(1.0-f.x,f.x,x==1)*select(1.0-f.y,f.y,y==1);
        if !(w>0.0) { continue; }
        let tap=base+vec2i(x,y);
        if any(tap<vec2i(0)) || any(tap>=vec2i(frame.output.xy)) { continue; }
        let s=textureLoad(previous_support,tap,0).xy;
        if finite4(vec4f(s,0.0,0.0)) && s.x>=0.0 && s.x<=1.0 && s.y>0.5
            && abs(view_depth(s.x)-expected)<=tolerance { confidence+=w; }
    }}
    if !(confidence>0.0) { return result; }
    var history=vec4f(0.0);var history_mass=0.0;
    // Edge colors span surfaces. Keep the complete finite signed color kernel;
    // a different-depth outer color tap must not veto central geometric support.
    for(var y=-1;y<=2;y++) { for(var x=-1;x<=2;x++) {
        let tap=base+vec2i(x,y);
        let delta=old-vec2f(tap); let w=cubic(delta.x)*cubic(delta.y);
        if abs(w)<1e-7 { continue; }
        if any(tap<vec2i(0)) || any(tap>=vec2i(frame.output.xy)) { return result; }
        let h=textureLoad(previous_color,tap,0);
        if !finite4(h) { return result; }
        history+=h*w;history_mass+=w;
    }}
    if abs(history_mass)<1e-5 { return result; }
    let h=clean(history/history_mass);
    mean/=9.0;
    let sigma=sqrt(max(moment/9.0-mean*mean,vec3f(0.0)));
    let clipped=clean(vec4f(rgb(clamp(ycocg(h.rgb),mean-sigma,mean+sigma)),clamp(h.a,minimum.a,maximum.a)));
    // Existing fixed temporal reaction policy, fixed candidate values. Radiance
    // change is measured before clipping; no confidence stored in coverage alpha.
    let change=abs(h.rgb-c.rgb);
    let reaction=clamp(max(change.x,max(change.y,change.z))/0.25,0.0,1.0);
    result.color=clean(mix(c,clipped,clamp(confidence,0.0,1.0)*0.9*(1.0-0.5*reaction)));
    return result;
}
