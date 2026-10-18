// Correctness prototype. Current output4x geometry owns coverage. History never
// supplies visibility; it supplies only radiance for an identified current surface.
@group(0) @binding(6) var low_owner:texture_2d<u32>;
@group(0) @binding(7) var visible_xy:texture_multisampled_2d<u32>;
@group(0) @binding(8) var visible_zw:texture_multisampled_2d<u32>;
@group(0) @binding(9) var visible_owner:texture_multisampled_2d<u32>;
@group(0) @binding(10) var visible_position:texture_multisampled_2d<u32>;
@group(0) @binding(11) var visible_depth:texture_depth_multisampled_2d;
@group(0) @binding(12) var previous_owner:texture_2d<u32>;
fn unpack(v:vec4u)->vec2f { return bitcast<vec2f>(vec2u(v.x|(v.y<<16u),v.z|(v.w<<16u))); }
struct Witness { rgb:vec3f, mass:f32, minimum:vec3f, age:f32, maximum:vec3f }
fn current_witness(position:vec2f,id:u32,depth:f32)->Witness {
    let q=position/frame.output.xy*frame.source.zw+frame.source.xy+frame.jitter.xy;
    let center=vec2i(floor(q)); let lo=vec2i(frame.source.xy); let hi=lo+vec2i(frame.source.zw);
    var w=Witness(vec3f(0),0.0,vec3f(65504.0),8.0,vec3f(0));
    var nearest=vec3f(0);var distance=1e30;var count=0u;
    let tolerance=max(0.001,0.01*view_depth(depth));
    for(var y=-1;y<=1;y++) { for(var x=-1;x<=1;x++) {
        let tap=center+vec2i(x,y);
        if any(tap<lo) || any(tap>=hi) { continue; }
        if (textureLoad(low_owner,tap,0).x&2147483647u)!=id { continue; }
        let d=textureLoad(current_depth,tap,0);
        let c=textureLoad(current,tap,0);
        if !finite4(c) || !(c.a>0.0) || !(d>=0.0 && d<1.0)
            || abs(view_depth(d)-view_depth(depth))>tolerance { continue; }
        let color=clean(c).rgb/c.a;
        let delta=vec2f(tap)+vec2f(0.5)-q;
        let weight=lanczos(delta.x)*lanczos(delta.y);
        w.rgb+=color*weight;w.mass+=weight;
        w.minimum=min(w.minimum,color);w.maximum=max(w.maximum,color);count++;
        if dot(delta,delta)<distance { distance=dot(delta,delta);nearest=color; }
    }}
    if count==0u { w.mass=0.0;return w; }
    if abs(w.mass)>1e-5 { w.rgb=clamp(w.rgb/w.mass,w.minimum,w.maximum); }
    else { w.rgb=nearest; }
    w.mass=1.0;return w;
}
fn history_witness(position:vec2f,id:u32,m:vec4f)->Witness {
    var h=Witness(vec3f(0),0.0,vec3f(0),8.0,vec3f(0));
    if frame.depth.w<0.5 || !valid_motion(m) { return h; }
    // Visibility motion already uses UNJITTERED output coordinates. No second
    // jitter correction, and m.z remains expected PREVIOUS depth.
    let old=position+m.xy*frame.output.xy-vec2f(0.5);
    let base=vec2i(floor(old));let f=fract(old);
    let expected=view_depth(m.z);let tolerance=max(0.001,0.01*expected);
    for(var y=0;y<2;y++) { for(var x=0;x<2;x++) {
        let tap=base+vec2i(x,y);
        let weight=select(1.0-f.x,f.x,x==1)*select(1.0-f.y,f.y,y==1);
        if !(weight>0.0) || any(tap<vec2i(0)) || any(tap>=vec2i(frame.output.xy)) { continue; }
        if textureLoad(previous_owner,tap,0).x!=id { continue; }
        let s=textureLoad(previous_support,tap,0).xy;
        let c=textureLoad(previous_color,tap,0);
        if !finite4(vec4f(s,0,0)) || !(s.x>=0.0 && s.x<1.0)
            || !(s.y>1.0 && s.y<=8.0 && floor(s.y)==s.y)
            || abs(view_depth(s.x)-expected)>tolerance || !finite4(c) || !(c.a>0.0) { continue; }
        h.rgb+=clean(c).rgb/c.a*weight;h.mass+=weight;h.age=min(h.age,s.y-1.0);
    }}
    if h.mass>0.0 { h.rgb/=h.mass; }
    return h;
}
struct Resolved { @location(0) color:vec4f, @location(1) support:vec2f, @location(2) owner:u32 }
@fragment fn resolve(@builtin(position) p:vec4f)->Resolved {
    let pixel=vec2i(p.xy);var result=Resolved(vec4f(0),vec2f(1,0),0u);
    var common=0u;var mixed=false;var complete=true;var observed=0u;var age=8.0;
    for(var lane=0;lane<4;lane++) {
        let packed=textureLoad(visible_owner,pixel,lane);
        let id=(packed.x|(packed.y<<16u))&2147483647u;
        if id==0u { continue; }
        observed++;
        if common==0u { common=id; } else { mixed=mixed || common!=id; }
        let position=unpack(textureLoad(visible_position,pixel,lane));
        let depth=textureLoad(visible_depth,pixel,lane);
        // Preserve/check the passing companion representation, including its
        // exact bits. A malformed witness cannot seed reusable color history.
        if bitcast<u32>(position.x)!=(packed.z|(packed.w<<16u))
            || !finite4(vec4f(position,depth,0)) || !(depth>=0.0 && depth<1.0)
            || any(position<vec2f(pixel)) || any(position>=vec2f(pixel)+vec2f(1)) {
            complete=false;continue;
        }
        let m=vec4f(unpack(textureLoad(visible_xy,pixel,lane)),unpack(textureLoad(visible_zw,pixel,lane)));
        let c=current_witness(position,id,depth);let h=history_witness(position,id,m);
        var color=c.rgb;var remaining=8.0;
        if c.mass>0.0 {
            if h.mass>0.0 {
                let clipped=clamp(h.rgb,c.minimum,c.maximum);
                color=mix(color,clipped,history_weight(vec4f(color,1),vec4f(h.rgb,1),h.mass));
            }
        } else if h.mass>0.0 { color=h.rgb;remaining=h.age; }
        else { complete=false;continue; } // Never-observed radiance stays missing.
        result.color+=vec4f(clamp(color,vec3f(0),vec3f(65504)),1.0)*0.25;
        result.support.x=min(result.support.x,depth);age=min(age,remaining);
    }
    // A mixed-owner composite cannot later masquerade as one surface's color.
    if observed>0u && complete && !mixed { result.owner=common;result.support.y=age; }
    return result;
}
