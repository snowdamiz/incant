// Test-only forward gather of unchanged, single-owner partial coverage. This is
// deliberately bounded to nearby old texels, not a general reprojection solver.
// Matrices are UNJITTERED; old depth belongs to the old resolved pixel center.
const CAMERA_RADIUS:i32=3;
struct CameraPair { previous_inverse:mat4x4f, current_matrix:mat4x4f }
@group(0) @binding(10) var<uniform> cameras:CameraPair;
fn camera_project(p:vec2f,z:f32)->vec4f {
    if !(z>=frame.depth.x && z<frame.depth.y) {return vec4f(0.0);}
    let u=(p-frame.output.xy)/frame.output.zw;
    let device_z=select((1.0/frame.depth.x-1.0/z)/(1.0/frame.depth.x-1.0/frame.depth.y),
        (z-frame.depth.x)/(frame.depth.y-frame.depth.x),frame.depth.z>0.5);
    let world=cameras.previous_inverse*vec4f(2.0*u.x-1.0,1.0-2.0*u.y,device_z,1.0);
    if !finite4(world) || abs(world.w)<1e-8 {return vec4f(0.0);}
    let clip=cameras.current_matrix*(world/world.w);
    if !finite4(clip) || clip.w<=0.0 {return vec4f(0.0);}
    let ndc=clip.xyz/clip.w;
    let depth=linear_depth(ndc.z);
    let pixel=frame.output.xy+vec2f(0.5*ndc.x+0.5,0.5-0.5*ndc.y)*frame.output.zw;
    if !(depth>0.0) || !inside(pixel,frame.output) {return vec4f(0.0);}
    return vec4f(pixel,depth,1.0);
}
struct CameraTap { color:vec4f, position:vec4f, mass:f32, age:f32, owner:u32 }
fn camera_tap(t:vec2i,p:vec2f,e:Evidence)->CameraTap {
    var v=CameraTap(vec4f(0.0),vec4f(0.0),0.0,0.0,0u);
    if any(t<lo(frame.output)) || any(t>hi(frame.output)) {return v;}
    let r=stored(textureLoad(previous_record,t,0).xy);
    // A mixed record has no single depth that can carry all of its colour.
    if !r.ok || r.over || r.b!=0u || changed_id(r.a) {return v;}
    let c=textureLoad(previous_color,t,0);
    let evidence=stored_evidence(textureLoad(previous_evidence,t,0));
    if !admitted(c) || !(c.a>0.0) || c.a>=1.0 || !(evidence.x>0.0) {return v;}
    let age=select(evidence.y+1.0,0.0,e.surface && sees(e.owners,r.a));
    if age>=MAX_AGE {return v;}
    let projected=camera_project(vec2f(t)+vec2f(0.5),textureLoad(previous_depth,t,0).x);
    if projected.w==0.0 {return v;}
    let distance=abs(projected.xy-p);
    if any(distance>=vec2f(1.0)) {return v;}
    // Current foreground cannot be replaced by a deeper old layer. The existing
    // depth tolerance remains unchanged; no colour clamp can validate ownership.
    if e.surface && e.nearest<projected.z && !compatible(e.nearest,projected.z) {return v;}
    let weight=(1.0-distance.x)*(1.0-distance.y);
    return CameraTap(clean(c),projected,evidence.x*weight,age,r.a);
}
@fragment fn resolve(@builtin(position) position:vec4f)->OwnerResolved {
    let p=position.xy;
    let ordinary=original_resolve(position);
    if !inside(p,frame.output) {return ordinary;}
    let e=current_evidence(p);
    // Static and opaque paths remain the exact existing owner resolver. This
    // experiment only asks whether moving fractional coverage can be recovered.
    if e.single || (changes.header.y&1u)!=0u || !e.valid || !e.owners.ok {return ordinary;}
    var result=ordinary;
    if e.surface {
        result.record=encode(e.owners); result.depth=e.nearest;
        result.evidence=vec2f(min(e.mass,evidence_cap()),0.0);
    }
    if frame.depth.w<0.5 || (changes.header.y&2u)!=0u || e.changed {return result;}
    if e.owners.over || e.owners.b!=0u {return result;}
    // At most (2R+1)^2 old centers. Full tent support for pure translation
    // requires each output-pixel displacement magnitude <= R. Beyond that the
    // bounded gather can retain only a partial partition or no history.
    // This is a small-motion diagnostic, not a claim of arbitrary camera support.
    let center=vec2i(p);
    var nearest=frame.depth.y+1.0; var selected=0u;
    for(var y=-CAMERA_RADIUS;y<=CAMERA_RADIUS;y++) {for(var x=-CAMERA_RADIUS;x<=CAMERA_RADIUS;x++) {
        let v=camera_tap(center+vec2i(x,y),p,e);
        if v.mass>0.0 && v.position.z<nearest {nearest=v.position.z;selected=v.owner;}
    }}
    if selected==0u || (e.surface && e.owners.a!=selected) {return result;}
    var numerator=vec4f(0.0); var mass=0.0; var age=0.0;
    for(var y=-CAMERA_RADIUS;y<=CAMERA_RADIUS;y++) {for(var x=-CAMERA_RADIUS;x<=CAMERA_RADIUS;x++) {
        let v=camera_tap(center+vec2i(x,y),p,e);
        if v.mass>0.0 && v.owner==selected && compatible(v.position.z,nearest) {
            numerator+=v.color*v.mass;mass+=v.mass;age=max(age,v.age);
        }
    }}
    if !(mass>0.0) {return result;}
    let weight=min(mass,evidence_cap()); let total=weight+e.mass;
    result.color=clean((numerator/mass*weight+e.sum)/total);
    result.depth=select(nearest,e.nearest,e.surface);
    result.record=encode(owner_set(selected));
    result.evidence=vec2f(min(total,evidence_cap()),age);
    return result;
}
