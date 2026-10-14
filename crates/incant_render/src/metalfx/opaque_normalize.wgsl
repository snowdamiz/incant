@group(0) @binding(1) var current:texture_2d<f32>;
@group(0) @binding(2) var native_color:texture_2d<f32>;
@group(0) @binding(3) var depth:texture_depth_2d;
@group(0) @binding(4) var source_motion:texture_2d<f32>;
struct Resolved { @location(0) color:vec4f }
@fragment fn normalize(@builtin(position) p:vec4f)->Resolved {
    var out:Resolved;out.color=vec4f(0.0);
    if !inside(p.xy,frame.output) {return out;}
    let q=source_position(p.xy)-vec2f(0.5);let base=vec2i(floor(q));let f=fract(q);
    var fallback=vec4f(0.0);var domain=true;
    for(var y=0;y<2;y++) {for(var x=0;x<2;x++) {
        let w=tap_weight(f,x,y);if w<=0.0 {continue;}
        let tap=clamp(base+vec2i(x,y),lo(frame.source),hi(frame.source));
        let c=textureLoad(current,tap,0);let z=textureLoad(depth,tap,0);let m=textureLoad(source_motion,tap,0);
        domain=domain && domain_sample(c,z,m);
        if clear_sample(c,z,m) {fallback+=vec4f(frame.reactive.rgb,1.0)*w;}
        else {fallback+=clean(c)*w;}
    }}
    out.color=clean(fallback);
    // This domain is genuinely opaque. Neither vendor alpha nor current low-res
    // geometry visibility/depth coherence is used as reconstructed coverage.
    let n=textureLoad(native_color,vec2i(p.xy-frame.output.xy),0);
    if domain {
        out.color.a=1.0;
        if finite4(n) && all(n.rgb>=vec3f(0.0)) {
            out.color=vec4f(min(n.rgb,vec3f(65504.0)),1.0);
        }
    }
    out.color.a=output_alpha(out.color.a);
    if out.color.a==0.0 {out.color=vec4f(0.0);}
    return out;
}
