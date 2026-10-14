@group(0) @binding(1) var current:texture_2d<f32>;
@group(0) @binding(2) var motion:texture_2d<f32>;
@group(0) @binding(3) var current_depth:texture_depth_2d;
@group(0) @binding(4) var previous_color:texture_2d<f32>;
@group(0) @binding(5) var previous_depth:texture_2d<f32>;
@group(0) @binding(6) var previous_evidence:texture_2d<f32>;
// Output history: premultiplied colour, linear depth and RG16Float evidence
// (x = effective sample mass, not a calibrated confidence; y = integer age).
// frame.reactive.z rescales stored mass by the source-rect area ratio
// (old area / new area); frame.reactive.w is 1 only when previous_evidence was
// written by this resolve for the accepted slot of this output allocation.
struct Resolved { @location(0) color:vec4f, @location(1) depth:f32, @location(2) evidence:vec2f }
// One complete eight-phase jitter cycle bounds consecutive absence.
const MAX_AGE:f32=8.0;
fn evidence_cap()->f32 {
    let h=clamp(frame.options.x,0.0,0.99);
    let r=frame.output.z/frame.source.z;
    return h/(1.0-h)*min(r*r,4.0);
}
// Gaussian at the true jittered output position of internal texel t (the in-tree
// kernel oracle: rho output pixels, unit expected mass per frame).
fn kernel_weight(p:vec2f,t:vec2i)->f32 {
    let r=frame.output.z/frame.source.z;
    let rho=0.35+0.325*max(r-2.0,0.0);
    let at=frame.output.xy+((vec2f(t)+vec2f(0.5)-frame.source.xy)/frame.source.zw-frame.jitter.xy)*frame.output.zw;
    let d=p-at;
    return r*r/(6.283185307*rho*rho)*exp(-dot(d,d)/(2.0*rho*rho));
}
// Stored mass and age of one previous tap, or (-1,-1) when unusable: invalid
// origin, nonfinite or out-of-range mass, non-integer or retired age.
fn stored_evidence(v:vec4f)->vec2f {
    if frame.reactive.w<0.5 || !(v.x>=0.0 && v.x<=65504.0) || !(v.y>=0.0 && v.y<MAX_AGE) || v.y!=floor(v.y) {
        return vec2f(-1.0);
    }
    return vec2f(min(v.x,evidence_cap()),v.y);
}
struct Evidence { sum:vec4f, mass:f32, valid:bool }
// Clear-inclusive current evidence over the declared support: the 3x3 internal
// texels around the one containing the jittered source position. Every in-rect
// tap with positive weight contributes; clear is zero colour at full weight. A
// malformed tap gains no weight and refuses history for this pixel.
fn current_evidence(p:vec2f)->Evidence {
    var e=Evidence(vec4f(0.0),0.0,true);
    let c=vec2i(floor(source_position(p)));
    for(var y=-1;y<=1;y++) {for(var x=-1;x<=1;x++) {
        let t=c+vec2i(x,y);
        if any(t<lo(frame.source)) || any(t>hi(frame.source)) {continue;}
        let w=kernel_weight(p,t);
        if !(w>0.0) {continue;}
        let v=textureLoad(current,t,0);
        if !admitted(v) {e.valid=false;continue;}
        e.sum+=clean(v)*w; e.mass+=w;
    }}
    return e;
}
@fragment fn resolve(@builtin(position) position:vec4f)->Resolved {
    var result=Resolved(vec4f(0.0),0.0,vec2f(0.0));
    let p=position.xy;
    if !inside(p,frame.output) {return result;}
    let q=source_position(p)-vec2f(0.5);
    let base=vec2i(floor(q)); let f=fract(q);
    var anchor=clamp(base,lo(frame.source),hi(frame.source));
    var nearest=frame.depth.y+1.0;
    var same_surface=true;
    var bilinear=vec4f(0.0);
    // The 2x2 surface test decides only whether one depth/motion describes the
    // pixel's history; it never removes current samples.
    for(var y=0;y<2;y++) {for(var x=0;x<2;x++) {
        let weight=tap_weight(f,x,y); if weight<=0.0 {continue;}
        let tap=clamp(base+vec2i(x,y),lo(frame.source),hi(frame.source));
        let c=textureLoad(current,tap,0);
        let z=linear_depth(textureLoad(current_depth,tap,0));
        bilinear+=clean(c)*weight;
        if !admitted(c) || c.a<1.0 || z<=0.0 {same_surface=false;}
        if z>0.0 && z<nearest {nearest=z;anchor=tap;}
    }}
    if nearest<=frame.depth.y {
        for(var y=0;y<2;y++) {for(var x=0;x<2;x++) {
            if tap_weight(f,x,y)<=0.0 {continue;}
            let tap=clamp(base+vec2i(x,y),lo(frame.source),hi(frame.source));
            same_surface=same_surface && compatible(linear_depth(textureLoad(current_depth,tap,0)),nearest);
        }}
    } else {same_surface=false;}
    let e=current_evidence(p);
    var fresh=clean(bilinear);
    if e.mass>0.0 {fresh=clean(e.sum/e.mass);}
    result.color=fresh;
    // Clear or mixed layers: current-only coverage, no depth, no stored evidence.
    if !same_surface {return result;}
    result.depth=nearest;
    result.evidence=vec2f(min(e.mass,evidence_cap()),0.0);
    if !e.valid || !(e.mass>0.0) || frame.depth.w<0.5 {return result;}
    let m=textureLoad(motion,anchor,0);
    if !finite4(m) || m.w<0.5 {return result;}
    let expected=linear_depth(m.z);
    // Motion includes both jittered projections. Output history is unjittered.
    let u=(p-frame.output.xy)/frame.output.zw;
    let old=frame.output.xy+(u+frame.jitter.xy+m.xy-frame.jitter.zw)*frame.output.zw;
    if !inside(old,frame.output) {return result;}
    // Colour, depth and evidence of each tap are admitted together; the history
    // numerator and its mass come from the same nonnegative bilinear support.
    let oldq=old-vec2f(0.5);let oldbase=vec2i(floor(oldq));let oldf=fract(oldq);
    var numerator=vec4f(0.0); var mass=0.0;
    for(var y=0;y<2;y++) {for(var x=0;x<2;x++) {
        let b=tap_weight(oldf,x,y);if !(b>0.0) {continue;}
        let tap=oldbase+vec2i(x,y);
        if any(tap<lo(frame.output)) || any(tap>hi(frame.output)) {continue;}
        let c=textureLoad(previous_color,tap,0);let z=textureLoad(previous_depth,tap,0).x;
        let s=stored_evidence(textureLoad(previous_evidence,tap,0));
        if !admitted(c) || !(c.a>0.0) || !compatible(z,expected) || s.x<0.0 {continue;}
        numerator+=clean(c)*(b*s.x); mass+=b*s.x;
    }}
    if !(mass>0.0) {return result;}
    var history=numerator/mass;
    var minimum=fresh;var maximum=fresh;
    for(var y=-1;y<=1;y++) {for(var x=-1;x<=1;x++) {
        let tap=clamp(anchor+vec2i(x,y),lo(frame.source),hi(frame.source));
        let c=textureLoad(current,tap,0);
        if admitted(c) && c.a>=1.0 && compatible(linear_depth(textureLoad(current_depth,tap,0)),nearest) {
            minimum=min(minimum,clean(c));maximum=max(maximum,clean(c));
        }
    }}
    let change=abs(history.rgb-fresh.rgb);
    let reaction=clamp(max(change.x,max(change.y,change.z))/frame.options.w,0.0,1.0);
    // Density and reaction scale the associated history mass; the clamp is a
    // biased correction and adds no sample weight.
    let w=min(mass*max(frame.reactive.z,0.0),evidence_cap())*(1.0-frame.reactive.x*reaction);
    history=clamp(history,minimum,maximum);
    let total=w+e.mass;
    if !(total>0.0) {return result;}
    result.color=clean((history*w+e.sum)/total);
    result.evidence=vec2f(min(total,evidence_cap()),0.0);
    return result;
}
