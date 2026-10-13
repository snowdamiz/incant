@group(0) @binding(1) var current:texture_2d<f32>;
@group(0) @binding(2) var motion:texture_2d<f32>;
@group(0) @binding(3) var current_depth:texture_depth_2d;
@group(0) @binding(4) var previous_color:texture_2d<f32>;
@group(0) @binding(5) var previous_depth:texture_2d<f32>;
struct Resolved { @location(0) color:vec4f, @location(1) depth:f32 }
// Interior current estimate: Catmull-Rom over the 4x4 taps around q when every tap
// with a nonzero weight is admitted, opaque and on the anchor surface, clamped to
// the central 2x2 (bilinear) range. Otherwise the existing bilinear estimate.
fn sharp_current(q:vec2f,nearest:f32,bilinear:vec4f)->vec4f {
    let base=vec2i(floor(q)); let f=fract(q);
    var sum=vec3f(0.0); var low=vec3f(65504.0); var high=vec3f(0.0); var central=false;
    for(var y=-1;y<=2;y++) {for(var x=-1;x<=2;x++) {
        let w=catmull_rom(f32(x)-f.x)*catmull_rom(f32(y)-f.y);
        if w==0.0 {continue;}
        let tap=clamp(base+vec2i(x,y),lo(frame.source),hi(frame.source));
        let c=textureLoad(current,tap,0);
        if !admitted(c) || c.a<1.0 || !compatible(linear_depth(textureLoad(current_depth,tap,0)),nearest) {
            return bilinear;
        }
        let v=clean(c).rgb;
        sum+=v*w;
        if x>=0 && x<=1 && y>=0 && y<=1 {low=min(low,v);high=max(high,v);central=true;}
    }}
    if !central {return bilinear;}
    return clean(vec4f(clamp(sum,low,high),1.0));
}
// Reprojected history: Catmull-Rom over the 4x4 previous taps around old when every
// tap with a nonzero weight is inside and admitted at the expected depth, clamped
// to the central 2x2 range. Otherwise the bilinear history with per-tap rejection.
fn sharp_history(old:vec2f,expected:f32,bilinear:vec4f)->vec4f {
    let oldq=old-vec2f(0.5); let base=vec2i(floor(oldq)); let f=fract(oldq);
    var sum=vec3f(0.0); var low=vec3f(65504.0); var high=vec3f(0.0); var central=false;
    for(var y=-1;y<=2;y++) {for(var x=-1;x<=2;x++) {
        let w=catmull_rom(f32(x)-f.x)*catmull_rom(f32(y)-f.y);
        if w==0.0 {continue;}
        let tap=base+vec2i(x,y);
        if any(tap<lo(frame.output)) || any(tap>hi(frame.output)) {return bilinear;}
        let c=textureLoad(previous_color,tap,0);let z=textureLoad(previous_depth,tap,0).x;
        if !admitted(c) || c.a<1.0 || !compatible(z,expected) {return bilinear;}
        let v=clean(c).rgb;
        sum+=v*w;
        if x>=0 && x<=1 && y>=0 && y<=1 {low=min(low,v);high=max(high,v);central=true;}
    }}
    if !central {return bilinear;}
    return clean(vec4f(clamp(sum,low,high),1.0));
}
@fragment fn resolve(@builtin(position) position:vec4f)->Resolved {
    var result=Resolved(vec4f(0.0),0.0);
    let p=position.xy;
    if !inside(p,frame.output) {return result;}
    let q=source_position(p)-vec2f(0.5);
    let base=vec2i(floor(q)); let f=fract(q);
    var anchor=clamp(base,lo(frame.source),hi(frame.source));
    var nearest=frame.depth.y+1.0;
    var same_surface=true;
    // Spatial current reconstruction preserves premultiplied coverage. At mixed
    // geometry/clear edges it is current-only rather than inventing solid coverage.
    for(var y=0;y<2;y++) {for(var x=0;x<2;x++) {
        let weight=tap_weight(f,x,y); if weight<=0.0 {continue;}
        let tap=clamp(base+vec2i(x,y),lo(frame.source),hi(frame.source));
        let c=textureLoad(current,tap,0);
        let z=linear_depth(textureLoad(current_depth,tap,0));
        result.color+=clean(c)*weight;
        if !admitted(c) || c.a<1.0 || z<=0.0 {same_surface=false;}
        if z>0.0 && z<nearest {nearest=z;anchor=tap;}
    }}
    if nearest>frame.depth.y {return result;}
    for(var y=0;y<2;y++) {for(var x=0;x<2;x++) {
        if tap_weight(f,x,y)<=0.0 {continue;}
        let tap=clamp(base+vec2i(x,y),lo(frame.source),hi(frame.source));
        same_surface=same_surface && compatible(linear_depth(textureLoad(current_depth,tap,0)),nearest);
    }}
    if !same_surface {return result;}
    result.color=sharp_current(q,nearest,clean(vec4f(result.color.rgb,1.0)));
    result.depth=nearest;
    if frame.depth.w<0.5 {return result;}
    let m=textureLoad(motion,anchor,0);
    if !finite4(m) || m.w<0.5 {return result;}
    let expected=linear_depth(m.z);
    // Motion includes both jittered projections. Output history is unjittered.
    let u=(p-frame.output.xy)/frame.output.zw;
    let old=frame.output.xy+(u+frame.jitter.xy+m.xy-frame.jitter.zw)*frame.output.zw;
    if !inside(old,frame.output) {return result;}
    let oldq=old-vec2f(0.5);let oldbase=vec2i(floor(oldq));let oldf=fract(oldq);
    var history=vec4f(0.0); var mass=0.0;
    for(var y=0;y<2;y++) {for(var x=0;x<2;x++) {
        let weight=tap_weight(oldf,x,y);if !(weight>0.0) {continue;}
        let tap=oldbase+vec2i(x,y);
        // Do not clamp an outside previous tap into plausible edge history.
        if any(tap<lo(frame.output)) || any(tap>hi(frame.output)) {continue;}
        let c=textureLoad(previous_color,tap,0);let z=textureLoad(previous_depth,tap,0).x;
        if !admitted(c) || c.a<1.0 || !compatible(z,expected) {continue;}
        history+=clean(c)*weight; mass+=weight;
    }}
    if !(mass>0.0) {return result;}
    history=sharp_history(old,expected,clean(history/mass));
    var minimum=result.color;var maximum=result.color;
    for(var y=-1;y<=1;y++) {for(var x=-1;x<=1;x++) {
        let tap=clamp(anchor+vec2i(x,y),lo(frame.source),hi(frame.source));
        let c=textureLoad(current,tap,0);
        if admitted(c) && c.a>=1.0 && compatible(linear_depth(textureLoad(current_depth,tap,0)),nearest) {
            minimum=min(minimum,clean(c));maximum=max(maximum,clean(c));
        }
    }}
    let change=abs(history.rgb-result.color.rgb);
    let reaction=clamp(max(change.x,max(change.y,change.z))/frame.options.w,0.0,1.0);
    let weight=history_fraction(frame.options.x*(1.0-frame.reactive.x*reaction),reaction);
    result.color=clean(mix(result.color,clamp(history,minimum,maximum),weight));
    return result;
}
