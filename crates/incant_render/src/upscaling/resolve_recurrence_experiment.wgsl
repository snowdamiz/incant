// TEST-ONLY derivative of resolve.wgsl at 72e5d626. No production pipeline references this.
@group(0) @binding(1) var current:texture_2d<f32>;
@group(0) @binding(2) var motion:texture_2d<f32>;
@group(0) @binding(3) var current_depth:texture_depth_2d;
@group(0) @binding(4) var previous_color:texture_2d<f32>;
@group(0) @binding(5) var previous_depth:texture_2d<f32>;
@group(0) @binding(6) var previous_evidence:texture_2d<f32>;
@group(0) @binding(7) var previous_recurrence:texture_2d<f32>;
// Output history: premultiplied colour, linear depth and RG16Float evidence
// (x = effective sample mass, not a calibrated confidence; y = integer age).
// frame.reactive.z rescales stored mass by the source-rect area ratio
// (old area / new area); frame.reactive.w is 1 only when previous_evidence was
// written by this resolve for the accepted slot of this output allocation.
struct Resolved { @location(0) color:vec4f, @location(1) depth:f32, @location(2) evidence:vec2f, @location(3) recurrence:vec4f }
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
// origin, nonfinite or out-of-wire-range mass, non-integer or retired age.
fn stored_evidence(v:vec4f)->vec2f {
    if frame.reactive.w<0.5 || !(v.x>=0.0 && v.x<=65504.0) || !(v.y>=0.0 && v.y<MAX_AGE) || v.y!=floor(v.y) {
        return vec2f(-1.0);
    }
    // Validated only against the RG16F wire range. The current cap must not clip a
    // tap before accumulation: density, cap and reaction act once on the
    // associated mass C = sum(b*W), so H = N/C keeps every tap's true weight.
    return v.xy;
}
// Current evidence over the declared support: the 3x3 internal texels around the
// one containing the jittered source position. Every in-rect tap with positive
// weight contributes; clear is zero colour at full weight. A malformed tap gains
// no weight and refuses history. `single` holds only when every contributor is an
// opaque surface compatible with the nearest one (the anchor): only then may one
// depth and one motion describe the stored history. Ambiguous support keeps its
// current colour and coverage but writes no reusable history.
struct Evidence { sum:vec4f, mass:f32, valid:bool, single:bool, nearest:f32, anchor:vec2i }
fn current_evidence(p:vec2f)->Evidence {
    var e=Evidence(vec4f(0.0),0.0,true,true,frame.depth.y+1.0,vec2i(0));
    let c=vec2i(floor(source_position(p)));
    for(var y=-1;y<=1;y++) {for(var x=-1;x<=1;x++) {
        let t=c+vec2i(x,y);
        if any(t<lo(frame.source)) || any(t>hi(frame.source)) {continue;}
        let w=kernel_weight(p,t);
        if !(w>0.0) {continue;}
        let v=textureLoad(current,t,0);
        if !admitted(v) {e.valid=false;continue;}
        e.sum+=clean(v)*w; e.mass+=w;
        let z=linear_depth(textureLoad(current_depth,t,0));
        if v.a<1.0 || z<=0.0 {e.single=false;}
        // Only a covered sample (a > 0) can be a surface; an alpha-0 claim is clear.
        if v.a>0.0 && z>0.0 && z<e.nearest {e.nearest=z;e.anchor=t;}
    }}
    if !(e.nearest<=frame.depth.y) {e.single=false;}
    if e.single {
        for(var y=-1;y<=1;y++) {for(var x=-1;x<=1;x++) {
            let t=c+vec2i(x,y);
            if any(t<lo(frame.source)) || any(t>hi(frame.source)) || !(kernel_weight(p,t)>0.0) {continue;}
            if !admitted(textureLoad(current,t,0)) {continue;}
            e.single=e.single && compatible(linear_depth(textureLoad(current_depth,t,0)),e.nearest);
        }}
    }
    return e;
}
// Single-motion proof. One depth does not prove one motion: coplanar samples may
// move independently. Every contributing current sample must carry valid motion
// whose reprojection agrees with the anchor's within motion_slack() output pixels
// and whose previous depth is compatible with the anchor's (the existing depth
// tolerance, not widened). Otherwise the pixel keeps its current evidence but
// reuses no history.
// The slack, 1/16 internal pixel in output pixels, is an empirical visual
// candidate: 1/256 output pixel left nearly all perspective motion current-only in
// the R4 images. It is not a positional-quantization bound (jitter positions do not
// quantize reconstructed history) and not a same-motion proof: distinct same-depth
// surfaces moving by less than the slack can share one history. Pixels pending.
fn motion_slack()->f32 {return frame.output.z/frame.source.z/16.0;}
fn motion_support_coherent(p:vec2f,a:vec4f)->bool {
    let c=vec2i(floor(source_position(p)));
    let za=linear_depth(a.z);
    for(var y=-1;y<=1;y++) {for(var x=-1;x<=1;x++) {
        let t=c+vec2i(x,y);
        if any(t<lo(frame.source)) || any(t>hi(frame.source)) || !(kernel_weight(p,t)>0.0) {continue;}
        let m=textureLoad(motion,t,0);
        if !finite4(m) || m.w<0.5 || !compatible(linear_depth(m.z),za) {return false;}
        if any(abs(m.xy-a.xy)*frame.output.zw>=vec2f(motion_slack())) {return false;}
    }}
    return true;
}
@fragment fn resolve(@builtin(position) position:vec4f)->Resolved {
    var result=Resolved(vec4f(0.0),0.0,vec2f(0.0),vec4f(-1.0));
    let p=position.xy;
    if !inside(p,frame.output) {return result;}
    let e=current_evidence(p);
    // Fresh reconstruction; with no admitted evidence the canonical result is clear.
    var fresh=vec4f(0.0);
    if e.mass>0.0 {fresh=clean(e.sum/e.mass);}
    result.color=fresh;
    // Clear, mixed-layer or malformed support: current-only, no reusable history.
    if !e.single || !e.valid {return result;}
    // Every admitted contributor is exactly opaque. Preserve that invariant
    // explicitly: weighted division can round just below one before RGBA16F
    // storage, which would make the next frame reject this valid opaque history.
    result.color.a=1.0;
    result.depth=e.nearest;
    result.recurrence=vec4f(recurrence_luma(fresh.rgb),-1.0,-1.0,-1.0);
    result.evidence=vec2f(min(e.mass,evidence_cap()),0.0);
    if frame.depth.w<0.5 {return result;}
    let m=textureLoad(motion,e.anchor,0);
    if !finite4(m) || m.w<0.5 || !motion_support_coherent(p,m) {return result;}
    let expected=linear_depth(m.z);
    // Motion includes both jittered projections. Output history is unjittered.
    let u=(p-frame.output.xy)/frame.output.zw;
    let old=frame.output.xy+(u+frame.jitter.xy+m.xy-frame.jitter.zw)*frame.output.zw;
    if !inside(old,frame.output) {return result;}
    // Colour, depth and evidence of each tap are admitted together; the history
    // numerator and its mass come from the same nonnegative bilinear support.
    let oldq=old-vec2f(0.5);let oldbase=vec2i(floor(oldq));let oldf=fract(oldq);
    var numerator=vec4f(0.0); var mass=0.0;
    var recurrence_sum=vec4f(0.0);var recurrence_ok=vec4<bool>(true);
    for(var y=0;y<2;y++) {for(var x=0;x<2;x++) {
        let b=tap_weight(oldf,x,y);if !(b>0.0) {continue;}
        let tap=oldbase+vec2i(x,y);
        if any(tap<lo(frame.output)) || any(tap>hi(frame.output)) {continue;}
        let c=textureLoad(previous_color,tap,0);let z=textureLoad(previous_depth,tap,0).x;
        let s=stored_evidence(textureLoad(previous_evidence,tap,0));
        if !admitted(c) || c.a<1.0 || !compatible(z,expected) || s.x<0.0 {continue;}
        numerator+=clean(c)*(b*s.x); mass+=b*s.x;
        // These are the exact joint colour/mass taps, never a separately
        // renormalized subset. Zero-mass taps cannot contribute an observation.
        if b*s.x>0.0 {
            let observation=textureLoad(previous_recurrence,tap,0);
            let valid=recurrence_valid(observation);
            recurrence_ok=recurrence_ok & valid;
            recurrence_sum+=select(vec4f(0.0),observation,valid)*(b*s.x);
        }
    }}
    if !(mass>0.0) {return result;}
    var history=numerator/mass;
    let recurrence=recurrence_quantize(select(vec4f(-1.0),recurrence_sum/mass,recurrence_ok));
    result.recurrence=vec4f(recurrence_luma(fresh.rgb),recurrence.xyz);
    // The clamp box is exactly the contributing (single-surface) support.
    var minimum=fresh;var maximum=fresh;
    let c=vec2i(floor(source_position(p)));
    for(var y=-1;y<=1;y++) {for(var x=-1;x<=1;x++) {
        let t=c+vec2i(x,y);
        if any(t<lo(frame.source)) || any(t>hi(frame.source)) || !(kernel_weight(p,t)>0.0) {continue;}
        let v=textureLoad(current,t,0);
        if admitted(v) {minimum=min(minimum,clean(v));maximum=max(maximum,clean(v));}
    }}
    // Reaction measures how far the clamp must move the history, not its distance
    // from this frame's single (aliased) estimate: history inside the contributing
    // support's range is consistent evidence and keeps its mass, so high-contrast
    // texture keeps accumulating; history outside it loses mass as before.
    var clamped=clamp(history,minimum,maximum);
    if recurrence_retains(history.rgb,fresh.rgb,minimum.rgb,maximum.rgb,recurrence) {clamped=history;}
    let change=abs(history.rgb-clamped.rgb);
    let reaction=clamp(max(change.x,max(change.y,change.z))/frame.options.w,0.0,1.0);
    // Density and reaction scale the associated history mass; the clamp is a
    // biased correction and adds no sample weight.
    let w=min(mass*max(frame.reactive.z,0.0),evidence_cap())*(1.0-frame.reactive.x*reaction);
    history=clamped;
    let total=w+e.mass;
    if !(total>0.0) {return result;}
    result.color=clean((history*w+e.sum)/total);
    result.color.a=1.0;
    result.evidence=vec2f(min(total,evidence_cap()),0.0);
    return result;
}
