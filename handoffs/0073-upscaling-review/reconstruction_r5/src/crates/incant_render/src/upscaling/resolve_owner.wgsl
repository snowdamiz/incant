// Owner-provenance output history (Kind::UpscaleOwner, after common.wgsl). The
// evidence estimator matches resolve.wgsl; every positive-weight contributor to
// stored colour (current samples, history taps, clamp support) enters the record:
// x is bit31 present plus the bits 0-16 primary ID; y is a second ID or 0, or
// bit31 plus a conservative class mask (bit ID%31) that always holds the primary
// class. (0,0) is no history. IDs are 1..highest. RG16Float evidence stores the
// effective sample mass (not a calibrated confidence) and an exact integer age of
// consecutive frames without current primary-owner surface evidence.
@group(0) @binding(1) var current:texture_2d<f32>;
@group(0) @binding(2) var motion:texture_2d<f32>;
@group(0) @binding(3) var current_depth:texture_depth_2d;
@group(0) @binding(4) var previous_color:texture_2d<f32>;
@group(0) @binding(5) var previous_depth:texture_2d<f32>;
@group(0) @binding(6) var previous_evidence:texture_2d<f32>;
@group(0) @binding(7) var owner:texture_2d<u32>;
@group(0) @binding(8) var previous_record:texture_2d<u32>;
// header: changed-class mask, flags (bit0 camera static, bit1 light or caster
// change), highest owner ID, 0. Owner n changed: words[n>>7][(n>>5)&3] bit n&31.
struct OwnerChanges { header:vec4u, words:array<vec4u,782> }
@group(0) @binding(9) var<uniform> changes:OwnerChanges;
struct OwnerResolved { @location(0) color:vec4f, @location(1) depth:f32, @location(2) record:vec2u, @location(3) evidence:vec2f }
const HIGH_BIT:u32=0x80000000u;
const ID_BITS:u32=0x1ffffu;
// MAX_INSTANCES; also keeps every changed-table read in bounds (100000>>7 = 781).
const MAX_OWNER:u32=100000u;
// One complete eight-phase jitter cycle bounds consecutive absence.
const MAX_AGE:f32=8.0;
fn valid_id(n:u32)->bool {return n>=1u && n<=min(changes.header.z,MAX_OWNER);}
fn changed_id(n:u32)->bool {return ((changes.words[n>>7u][(n>>5u)&3u]>>(n&31u))&1u)!=0u;}
fn class_of(n:u32)->u32 {return 1u<<(n%31u);}
// Owners a colour depends on: a primary, one more exact ID, or overflow; m keeps
// every class seen. One invalid ID makes the whole set unprovable. An empty set
// (a = 0) takes its first valid ID as primary.
struct Owners { a:u32, b:u32, m:u32, over:bool, ok:bool }
fn owner_set(a:u32)->Owners {return Owners(a,0u,class_of(a),false,valid_id(a));}
fn with_id(s:Owners,n:u32)->Owners {
    var o=s;
    if !valid_id(n) {o.ok=false;return o;}
    o.m|=class_of(n);
    if o.a==0u {o.a=n;} else if n!=o.a && n!=o.b {if o.b==0u {o.b=n;} else {o.over=true;}}
    return o;
}
fn encode(s:Owners)->vec2u {
    if !s.ok || s.a==0u {return vec2u(0u);}
    return vec2u(HIGH_BIT|s.a,select(s.b,HIGH_BIT|s.m,s.over));
}
// Canonical previous record; anything else is no history.
struct Stored { a:u32, b:u32, m:u32, over:bool, ok:bool }
fn stored(v:vec2u)->Stored {
    var s=Stored(v.x&ID_BITS,0u,0u,false,false);
    if (v.x&HIGH_BIT)==0u || (v.x&~(HIGH_BIT|ID_BITS))!=0u || !valid_id(s.a) {return s;}
    if (v.y&HIGH_BIT)!=0u {
        s.m=v.y&~HIGH_BIT;s.over=true;s.ok=(s.m&class_of(s.a))!=0u;
    } else {
        s.b=v.y;s.ok=v.y==0u || (v.y!=s.a && valid_id(v.y));
    }
    return s;
}
fn with_record(s:Owners,r:Stored)->Owners {
    var o=with_id(s,r.a);
    if r.over {o.m|=r.m;o.over=true;} else if r.b!=0u {o=with_id(o,r.b);}
    return o;
}
// The carried record keeps its primary (the absence clock's identity); every
// fresh contributor joins after it, overflowing conservatively.
fn carried(r:Stored,s:Owners)->Owners {
    var o=owner_set(r.a);
    if r.over {o.m|=r.m;o.over=true;} else if r.b!=0u {o=with_id(o,r.b);}
    if s.a!=0u {o=with_id(o,s.a);}
    if s.over {o.m|=s.m;o.over=true;} else if s.b!=0u {o=with_id(o,s.b);}
    o.ok=o.ok && s.ok;
    return o;
}
// Exact recorded IDs by the changed table; overflow records by class.
fn record_unchanged(r:Stored,support_classes:u32,changed_classes:u32)->bool {
    if r.over {return ((r.m|support_classes)&changed_classes)==0u;}
    return !changed_id(r.a) && (r.b==0u || !changed_id(r.b));
}
// A reprojected tap follows the motion of the current anchor, which the caller
// requires to be r.a, so only that owner may have changed. Any other changed exact
// owner rejects the tap at every positive weight; overflow needs disjoint classes.
fn record_admissible(r:Stored,support_classes:u32,changed_classes:u32)->bool {
    if r.over {return ((r.m|support_classes)&changed_classes)==0u;}
    return r.b==0u || !changed_id(r.b);
}
fn evidence_cap()->f32 {
    let h=clamp(frame.options.x,0.0,0.99);
    let r=frame.output.z/frame.source.z;
    return h/(1.0-h)*min(r*r,4.0);
}
fn kernel_weight(p:vec2f,t:vec2i)->f32 {
    let r=frame.output.z/frame.source.z;
    let rho=0.35+0.325*max(r-2.0,0.0);
    let at=frame.output.xy+((vec2f(t)+vec2f(0.5)-frame.source.xy)/frame.source.zw-frame.jitter.xy)*frame.output.zw;
    let d=p-at;
    return r*r/(6.283185307*rho*rho)*exp(-dot(d,d)/(2.0*rho*rho));
}
// Single-motion proof. One depth does not prove one motion: coplanar samples may
// move independently. Every contributing current sample must carry valid motion
// whose reprojection agrees with the anchor's within motion_slack() output pixels
// and whose previous depth is compatible with the anchor's (the existing depth
// tolerance, not widened). Otherwise the pixel keeps its current evidence but
// reuses no history.
// The slack is one step of the upscale jitter lattice (1/16 internal pixel) in
// output pixels: accumulated positions are resolved only to that step, so smaller
// disagreement is below the history's positional resolution. (1/256 output pixel
// rejected nearly all perspective motion and left moving content current-only.)
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
// Stored mass and integer age of one previous texel, or (-1,-1) when unusable.
fn stored_evidence(v:vec4f)->vec2f {
    if frame.reactive.w<0.5 || !(v.x>=0.0 && v.x<=65504.0) || !(v.y>=0.0 && v.y<MAX_AGE) || v.y!=floor(v.y) {
        return vec2f(-1.0);
    }
    // Validated only against the RG16F wire range. The current cap must not clip a
    // tap before accumulation: density, cap and reaction act once on the
    // associated mass C = sum(b*W), so H = N/C keeps every tap's true weight.
    return v.xy;
}
// The owner word of a surface tap: its ID and whether it changed this frame.
fn owner_of(t:vec2i)->vec2u {
    let word=textureLoad(owner,t,0).x; let n=word&~HIGH_BIT;
    return vec2u(n,select(0u,1u,(word&HIGH_BIT)!=0u || (valid_id(n) && changed_id(n))));
}
// Current evidence with provenance over the same declared 3x3 support as
// resolve.wgsl. Every positive in-rect contributor counts (clear: zero colour,
// full weight); every contributing surface owner joins `owners`, the nearest
// surface's owner first. Changed owners stay valid current evidence; they only
// forbid unclipped persistence. An invalid owner makes the record unprovable.
struct Evidence {
    sum:vec4f, mass:f32, valid:bool, single:bool, surface:bool, changed:bool,
    nearest:f32, anchor:vec2i, owners:Owners, classes:u32, changed_secondary:bool,
}
fn current_evidence(p:vec2f)->Evidence {
    var e=Evidence(vec4f(0.0),0.0,true,true,false,false,frame.depth.y+1.0,vec2i(0),Owners(0u,0u,0u,false,true),0u,false);
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
        // Only a covered sample (a > 0) is surface evidence; an alpha-0 claim with a
        // positive (malformed) depth is clear and never current primary evidence.
        if v.a>0.0 && z>0.0 {e.surface=true; if z<e.nearest {e.nearest=z;e.anchor=t;}}
    }}
    if !e.surface {e.single=false; return e;}
    e.owners=owner_set(owner_of(e.anchor).x);
    for(var y=-1;y<=1;y++) {for(var x=-1;x<=1;x++) {
        let t=c+vec2i(x,y);
        if any(t<lo(frame.source)) || any(t>hi(frame.source)) || !(kernel_weight(p,t)>0.0) {continue;}
        let v=textureLoad(current,t,0);
        if !admitted(v) {continue;}
        let z=linear_depth(textureLoad(current_depth,t,0));
        if !(v.a>0.0) || z<=0.0 {continue;}
        e.single=e.single && compatible(z,e.nearest);
        let o=owner_of(t);
        e.owners=with_id(e.owners,o.x);
        if valid_id(o.x) {
            e.classes|=class_of(o.x);
            if o.y!=0u {e.changed=true; if o.x!=e.owners.a {e.changed_secondary=true;}}
        }
    }}
    return e;
}
// Whether the current exact support includes primary owner n as a surface sample.
fn sees(s:Owners,n:u32)->bool {return n!=0u && (s.a==n || s.b==n);}
@fragment fn resolve(@builtin(position) position:vec4f)->OwnerResolved {
    var result=OwnerResolved(vec4f(0.0),0.0,vec2u(0u),vec2f(0.0));
    let p=position.xy;
    if !inside(p,frame.output) {return result;}
    let e=current_evidence(p);
    var fresh=vec4f(0.0);
    if e.mass>0.0 {fresh=clean(e.sum/e.mass);}
    result.color=fresh;
    if !e.valid || !e.owners.ok {return result;}
    var changed_classes=changes.header.x;
    if e.changed {changed_classes|=e.classes;}
    let flags=changes.header.y;
    // Static proof: camera static, no light/caster change, every contributing
    // owner valid and unchanged, and a motionless anchor when a surface exists.
    var still=frame.depth.w>=0.5 && (flags&1u)!=0u && (flags&2u)==0u && !e.changed;
    if still && e.surface {
        let m=textureLoad(motion,e.anchor,0);
        let u=(p-frame.output.xy)/frame.output.zw;
        let old=frame.output.xy+(u+frame.jitter.xy+m.xy-frame.jitter.zw)*frame.output.zw;
        still=finite4(m) && m.w>=0.5 && all(abs(old-p)<vec2f(1.0/256.0));
    }
    if still {
        // Unclipped static persistence of the own texel: colour, depth, record and
        // evidence admitted together. Partial coverage (thin geometry, edges) may
        // persist over clear support; an opaque history never does. Age resets only
        // on current primary-owner surface evidence and retires at MAX_AGE.
        let own=vec2i(p);
        let c=textureLoad(previous_color,own,0); let z=textureLoad(previous_depth,own,0).x;
        let r=stored(textureLoad(previous_record,own,0).xy);
        let s=stored_evidence(textureLoad(previous_evidence,own,0));
        // The candidate age is decided BEFORE blending: history whose primary would
        // reach MAX_AGE is refused, so no old colour survives the eighth absent frame.
        let age=select(s.y+1.0,0.0,e.surface && sees(e.owners,r.a));
        let persist=admitted(c) && c.a>0.0 && r.ok && s.x>=0.0 && age<MAX_AGE
            && record_unchanged(r,e.classes,changed_classes) && (e.surface || c.a<1.0)
            && z==z && z>=0.0 && z<=frame.depth.y;
        if persist {
            // One own texel (b = 1), same extent (density 1): cap once, no reaction.
            let w=min(s.x,evidence_cap());
            let total=w+e.mass;
            if total>0.0 {
                result.color=clean((clean(c)*w+e.sum)/total);
                // The carried primary stays primary while its colour survives, so a
                // fresh owner cannot restart the old primary's absence clock.
                result.record=encode(carried(r,e.owners));
                result.depth=select(z,e.nearest,e.surface && e.owners.a==r.a);
                result.evidence=vec2f(min(total,evidence_cap()),age);
                if result.record.x==0u {result.depth=0.0;result.evidence=vec2f(0.0);}
                return result;
            }
        }
        // Seed: only fresh surface evidence starts a genuinely fresh record (age 0);
        // clear alone never seeds.
        if e.surface {
            result.record=encode(e.owners);
            result.depth=e.nearest;
            result.evidence=vec2f(min(e.mass,evidence_cap()),0.0);
            if result.record.x==0u {result.depth=0.0;result.evidence=vec2f(0.0);}
        }
        return result;
    }
    // Ordinary reprojection: only a single opaque surface may reuse history; mixed
    // layers and clear coverage stay current-only (stated limitation).
    if !e.single {return result;}
    result.depth=e.nearest;
    let anchor_id=e.owners.a;
    var joined=e.owners;
    result.record=encode(joined);
    result.evidence=vec2f(min(e.mass,evidence_cap()),0.0);
    // A changed non-anchor current contributor does not follow the anchor's motion
    // proof: current-only.
    if frame.depth.w<0.5 || e.changed_secondary {return result;}
    let m=textureLoad(motion,e.anchor,0);
    if !finite4(m) || m.w<0.5 || !motion_support_coherent(p,m) {return result;}
    let expected=linear_depth(m.z);
    let u=(p-frame.output.xy)/frame.output.zw;
    let old=frame.output.xy+(u+frame.jitter.xy+m.xy-frame.jitter.zw)*frame.output.zw;
    if !inside(old,frame.output) {return result;}
    let oldq=old-vec2f(0.5);let oldbase=vec2i(floor(oldq));let oldf=fract(oldq);
    var numerator=vec4f(0.0); var mass=0.0;
    for(var y=0;y<2;y++) {for(var x=0;x<2;x++) {
        let b=tap_weight(oldf,x,y);if !(b>0.0) {continue;}
        let tap=oldbase+vec2i(x,y);
        if any(tap<lo(frame.output)) || any(tap>hi(frame.output)) {continue;}
        let c=textureLoad(previous_color,tap,0);let z=textureLoad(previous_depth,tap,0).x;
        let s=stored_evidence(textureLoad(previous_evidence,tap,0));
        let r=stored(textureLoad(previous_record,tap,0).xy);
        if !admitted(c) || c.a<1.0 || !compatible(z,expected) || s.x<0.0 || !r.ok || r.a!=anchor_id
            || !record_admissible(r,e.classes,changed_classes) {continue;}
        numerator+=clean(c)*(b*s.x); mass+=b*s.x; joined=with_record(joined,r);
    }}
    if !(mass>0.0) {return result;}
    var history=numerator/mass;
    // Clamp support is exactly the contributing current support; its owners are
    // already in `joined`.
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
    let clamped=clamp(history,minimum,maximum);
    let change=abs(history.rgb-clamped.rgb);
    let reaction=clamp(max(change.x,max(change.y,change.z))/frame.options.w,0.0,1.0);
    let w=min(mass*max(frame.reactive.z,0.0),evidence_cap())*(1.0-frame.reactive.x*reaction);
    history=clamped;
    let total=w+e.mass;
    if !(total>0.0) {return result;}
    result.color=clean((history*w+e.sum)/total);
    result.record=encode(joined);
    result.evidence=vec2f(min(total,evidence_cap()),0.0);
    return result;
}
