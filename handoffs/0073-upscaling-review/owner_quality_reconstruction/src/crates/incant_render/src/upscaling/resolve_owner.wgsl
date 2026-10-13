// Owner-provenance output history (Kind::UpscaleOwner, after common.wgsl). Color
// and depth follow resolve.wgsl. Each output pixel also records every owner whose
// color reached its history since the last drop: x is bit31 present plus the
// bits 0-16 anchor ID; y is a second ID or 0, or bit31 plus a class mask (bit ID%31)
// that always holds the anchor class. (0,0) is no history. IDs are 1..highest.
// Catmull-Rom current and history taps join that set exactly like bilinear taps.
@group(0) @binding(1) var current:texture_2d<f32>;
@group(0) @binding(2) var motion:texture_2d<f32>;
@group(0) @binding(3) var current_depth:texture_depth_2d;
@group(0) @binding(4) var previous_color:texture_2d<f32>;
@group(0) @binding(5) var previous_depth:texture_2d<f32>;
@group(0) @binding(6) var owner:texture_2d<u32>;
@group(0) @binding(7) var previous_record:texture_2d<u32>;
// header: changed-class mask, flags (bit0 camera static, bit1 light or caster
// change), highest owner ID, 0. Owner n changed: words[n>>7][(n>>5)&3] bit n&31.
struct OwnerChanges { header:vec4u, words:array<vec4u,782> }
@group(0) @binding(8) var<uniform> changes:OwnerChanges;
struct OwnerResolved { @location(0) color:vec4f, @location(1) depth:f32, @location(2) record:vec2u }
const HIGH_BIT:u32=0x80000000u;
const ID_BITS:u32=0x1ffffu;
// MAX_INSTANCES; also keeps every changed-table read in bounds (100000>>7 = 781).
const MAX_OWNER:u32=100000u;
fn valid_id(n:u32)->bool {return n>=1u && n<=min(changes.header.z,MAX_OWNER);}
fn changed_id(n:u32)->bool {return ((changes.words[n>>7u][(n>>5u)&3u]>>(n&31u))&1u)!=0u;}
fn class_of(n:u32)->u32 {return 1u<<(n%31u);}
// Owners a color depends on: the anchor, one more exact ID, or overflow; m keeps
// every class seen. One invalid ID makes the whole set unprovable.
struct Owners { a:u32, b:u32, m:u32, over:bool, ok:bool }
fn owner_set(a:u32)->Owners {return Owners(a,0u,class_of(a),false,valid_id(a));}
fn with_id(s:Owners,n:u32)->Owners {
    var o=s;
    if !valid_id(n) {o.ok=false;return o;}
    o.m|=class_of(n);
    if n!=o.a && n!=o.b {if o.b==0u {o.b=n;} else {o.over=true;}}
    return o;
}
fn encode(s:Owners)->vec2u {
    if !s.ok {return vec2u(0u);}
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
// Exact recorded IDs by the changed table; overflow records by class.
fn record_unchanged(r:Stored,support_classes:u32,changed_classes:u32)->bool {
    if r.over {return ((r.m|support_classes)&changed_classes)==0u;}
    return !changed_id(r.a) && (r.b==0u || !changed_id(r.b));
}
// A reprojected tap follows the motion of the current anchor, which the caller
// requires to be r.a, so only that owner may have changed. Any other changed exact
// owner rejects the tap at every current support weight: arbitrarily small support
// does not validate stale history. An overflow record needs a changed set disjoint
// from its classes.
fn record_admissible(r:Stored,support_classes:u32,changed_classes:u32)->bool {
    if r.over {return ((r.m|support_classes)&changed_classes)==0u;}
    return r.b==0u || !changed_id(r.b);
}
// Interior current estimate (resolve.wgsl sharp_current) with provenance. A tap
// outside the central 2x2, whose owners are already support, must also carry a
// valid unchanged owner, which then joins the support set. Any failure keeps the
// existing bilinear estimate and its 2x2 support.
struct Sharp { color:vec4f, owners:Owners, classes:u32 }
fn sharp_current(q:vec2f,nearest:f32,bilinear:vec4f,start:Owners)->Sharp {
    let fallback=Sharp(bilinear,start,0u);
    let base=vec2i(floor(q)); let f=fract(q);
    var sum=vec3f(0.0); var low=vec3f(65504.0); var high=vec3f(0.0); var central=false;
    var joined=start; var classes=0u;
    for(var y=-1;y<=2;y++) {for(var x=-1;x<=2;x++) {
        let w=catmull_rom(f32(x)-f.x)*catmull_rom(f32(y)-f.y);
        if w==0.0 {continue;}
        let tap=clamp(base+vec2i(x,y),lo(frame.source),hi(frame.source));
        let c=textureLoad(current,tap,0);
        if !admitted(c) || c.a<1.0 || !compatible(linear_depth(textureLoad(current_depth,tap,0)),nearest) {
            return fallback;
        }
        let v=clean(c).rgb;
        sum+=v*w;
        if x>=0 && x<=1 && y>=0 && y<=1 {
            low=min(low,v);high=max(high,v);central=true;
        } else {
            let word=textureLoad(owner,tap,0).x; let n=word&~HIGH_BIT;
            if !valid_id(n) || (word&HIGH_BIT)!=0u || changed_id(n) {return fallback;}
            joined=with_id(joined,n); classes|=class_of(n);
        }
    }}
    if !central {return fallback;}
    return Sharp(clean(vec4f(clamp(sum,low,high),1.0)),joined,classes);
}
// Reprojected history (resolve.wgsl sharp_history) with provenance. Every tap with
// a nonzero weight needs the bilinear tap admission, including its own canonical
// record with the anchor owner; all their records join the set. Any failure keeps
// the bilinear history and its set.
struct SharpHistory { color:vec4f, owners:Owners }
fn sharp_history(old:vec2f,expected:f32,anchor_id:u32,support_classes:u32,changed_classes:u32,
                 fresh:Owners,bilinear:SharpHistory)->SharpHistory {
    let oldq=old-vec2f(0.5); let base=vec2i(floor(oldq)); let f=fract(oldq);
    var sum=vec3f(0.0); var low=vec3f(65504.0); var high=vec3f(0.0); var central=false;
    var joined=fresh;
    for(var y=-1;y<=2;y++) {for(var x=-1;x<=2;x++) {
        let w=catmull_rom(f32(x)-f.x)*catmull_rom(f32(y)-f.y);
        if w==0.0 {continue;}
        let tap=base+vec2i(x,y);
        if any(tap<lo(frame.output)) || any(tap>hi(frame.output)) {return bilinear;}
        let c=textureLoad(previous_color,tap,0);let z=textureLoad(previous_depth,tap,0).x;
        if !admitted(c) || c.a<1.0 || !compatible(z,expected) {return bilinear;}
        let r=stored(textureLoad(previous_record,tap,0).xy);
        if !r.ok || r.a!=anchor_id || !record_admissible(r,support_classes,changed_classes) {return bilinear;}
        let v=clean(c).rgb;
        sum+=v*w; joined=with_record(joined,r);
        if x>=0 && x<=1 && y>=0 && y<=1 {low=min(low,v);high=max(high,v);central=true;}
    }}
    if !central {return bilinear;}
    return SharpHistory(clean(vec4f(clamp(sum,low,high),1.0)),joined);
}
@fragment fn resolve(@builtin(position) position:vec4f)->OwnerResolved {
    var result=OwnerResolved(vec4f(0.0),0.0,vec2u(0u));
    let p=position.xy;
    if !inside(p,frame.output) {return result;}
    let q=source_position(p)-vec2f(0.5);
    let base=vec2i(floor(q)); let f=fract(q);
    var anchor=clamp(base,lo(frame.source),hi(frame.source));
    var nearest=frame.depth.y+1.0;
    var same_surface=true;
    // Support: IDs (low 31 owner bits) of positive-weight taps with depth, 0 for
    // other taps; their classes; all valid; either changed signal (texture or table).
    var support=vec4u(0u); var support_classes=0u; var support_ok=true; var support_changed=false;
    var changed_classes=changes.header.x;
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
        if z>0.0 {
            let word=textureLoad(owner,tap,0).x; let n=word&~HIGH_BIT;
            support[u32(y*2+x)]=n;
            if !valid_id(n) {support_ok=false;} else {
                support_classes|=class_of(n);
                if (word&HIGH_BIT)!=0u || changed_id(n) {support_changed=true;changed_classes|=class_of(n);}
            }
        }
    }}
    // Clear and mixed exits keep depth 0, which history never admits: no record.
    if nearest>frame.depth.y {return result;}
    for(var y=0;y<2;y++) {for(var x=0;x<2;x++) {
        if tap_weight(f,x,y)<=0.0 {continue;}
        let tap=clamp(base+vec2i(x,y),lo(frame.source),hi(frame.source));
        same_surface=same_surface && compatible(linear_depth(textureLoad(current_depth,tap,0)),nearest);
    }}
    if !same_surface {return result;}
    result.color=clean(vec4f(result.color.rgb,1.0));
    result.depth=nearest;
    // Every later exit keeps this fresh current record, or (0,0) if unprovable.
    let anchor_id=textureLoad(owner,anchor,0).x&~HIGH_BIT;
    var fresh=owner_set(anchor_id);
    for(var i=0;i<4;i++) {if support[u32(i)]!=0u {fresh=with_id(fresh,support[u32(i)]);}}
    fresh.ok=fresh.ok && support_ok;
    // Catmull-Rom current (sharp_current) joins its outer owners to the support.
    let sharp=sharp_current(q,nearest,result.color,fresh);
    result.color=sharp.color; fresh=sharp.owners; support_classes|=sharp.classes;
    let provable=fresh.ok;
    result.record=encode(fresh);
    if frame.depth.w<0.5 {return result;}
    let m=textureLoad(motion,anchor,0);
    if !finite4(m) || m.w<0.5 {return result;}
    let expected=linear_depth(m.z);
    // Motion includes both jittered projections. Output history is unjittered.
    let u=(p-frame.output.xy)/frame.output.zw;
    let old=frame.output.xy+(u+frame.jitter.xy+m.xy-frame.jitter.zw)*frame.output.zw;
    if !inside(old,frame.output) {return result;}
    // Unclipped persistence needs no anchor match, only the complete proof: static
    // camera, motion under 1/256 px, no global change, provable unchanged current
    // support, and own-texel admitted history whose every recorded owner is
    // unchanged. Only this branch reads texel P directly; any failure falls through
    // to ordinary reprojection at the actual old position below.
    let flags=changes.header.y;
    let own=vec2i(p);
    if (flags&1u)!=0u && (flags&2u)==0u && all(abs(old-p)<vec2f(1.0/256.0)) && provable
        && !support_changed && all(own>=lo(frame.output)) && all(own<=hi(frame.output)) {
        let c=textureLoad(previous_color,own,0);let z=textureLoad(previous_depth,own,0).x;
        let r=stored(textureLoad(previous_record,own,0).xy);
        if admitted(c) && c.a>=1.0 && compatible(z,expected) && r.ok
            && record_unchanged(r,support_classes,changed_classes) {
            result.color=clean(mix(result.color,clean(c),history_fraction(frame.options.x,0.0)));
            result.record=encode(with_record(fresh,r));
            return result;
        }
    }
    let oldq=old-vec2f(0.5);let oldbase=vec2i(floor(oldq));let oldf=fract(oldq);
    var history=vec4f(0.0); var mass=0.0; var joined=fresh;
    for(var y=0;y<2;y++) {for(var x=0;x<2;x++) {
        let weight=tap_weight(oldf,x,y);if !(weight>0.0) {continue;}
        let tap=oldbase+vec2i(x,y);
        // Do not clamp an outside previous tap into plausible edge history.
        if any(tap<lo(frame.output)) || any(tap>hi(frame.output)) {continue;}
        let c=textureLoad(previous_color,tap,0);let z=textureLoad(previous_depth,tap,0).x;
        if !admitted(c) || c.a<1.0 || !compatible(z,expected) {continue;}
        // Each tap needs its own canonical record with the current anchor owner and
        // no other changed owner. Rejected taps leave the mass.
        let r=stored(textureLoad(previous_record,tap,0).xy);
        if !provable || !r.ok || r.a!=anchor_id
            || !record_admissible(r,support_classes,changed_classes) {continue;}
        history+=clean(c)*weight; mass+=weight; joined=with_record(joined,r);
    }}
    if !(mass>0.0) {return result;}
    let sharp_old=sharp_history(old,expected,anchor_id,support_classes,changed_classes,fresh,
        SharpHistory(clean(history/mass),joined));
    history=sharp_old.color; joined=sharp_old.owners;
    var minimum=result.color;var maximum=result.color;
    for(var y=-1;y<=1;y++) {for(var x=-1;x<=1;x++) {
        let tap=clamp(anchor+vec2i(x,y),lo(frame.source),hi(frame.source));
        let c=textureLoad(current,tap,0);
        if admitted(c) && c.a>=1.0 && compatible(linear_depth(textureLoad(current_depth,tap,0)),nearest) {
            minimum=min(minimum,clean(c));maximum=max(maximum,clean(c));
            // The clamp bounds carry this owner's color into history.
            joined=with_id(joined,textureLoad(owner,tap,0).x&~HIGH_BIT);
        }
    }}
    let change=abs(history.rgb-result.color.rgb);
    let reaction=clamp(max(change.x,max(change.y,change.z))/frame.options.w,0.0,1.0);
    let weight=history_fraction(frame.options.x*(1.0-frame.reactive.x*reaction),reaction);
    result.color=clean(mix(result.color,clamp(history,minimum,maximum),weight));
    result.record=encode(joined);
    return result;
}
