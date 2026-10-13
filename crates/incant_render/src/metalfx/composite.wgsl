@group(0) @binding(1) var current:texture_2d<f32>;
@group(0) @binding(2) var native_color:texture_2d<f32>;
@group(0) @binding(3) var depth:texture_depth_2d;
struct Resolved { @location(0) color:vec4f, @location(1) depth:f32 }
@fragment fn normalize(@builtin(position) p:vec4f)->Resolved {
    var out:Resolved;out.color=vec4f(0.0);out.depth=0.0;
    if !inside(p.xy,frame.output) {return out;}
    let q=source_position(p.xy)-vec2f(0.5);let base=vec2i(floor(q));let f=fract(q);
    var color=vec4f(0.0);var opaque=true;var z=frame.depth.y+1.0;
    for(var y=0;y<2;y++) {for(var x=0;x<2;x++) {
        let w=tap_weight(f,x,y);if w<=0.0 {continue;}
        let tap=clamp(base+vec2i(x,y),lo(frame.source),hi(frame.source));
        let c=textureLoad(current,tap,0);let d=linear_depth(textureLoad(depth,tap,0));
        color+=clean(c)*w;
        opaque=opaque && admitted(c) && c.a==1.0 && d>0.0;
        if d>0.0 {z=min(z,d);}
    }}
    out.color=clean(color);
    for(var y=0;y<2;y++) {for(var x=0;x<2;x++) {
        if tap_weight(f,x,y)<=0.0 {continue;}
        let tap=clamp(base+vec2i(x,y),lo(frame.source),hi(frame.source));
        opaque=opaque && compatible(linear_depth(textureLoad(depth,tap,0)),z);
    }}
    if opaque {out.depth=z;}
    let n=textureLoad(native_color,vec2i(p.xy-frame.output.xy),0);
    // MetalFX alpha is deliberately ignored. Coverage remains the current
    // engine's premultiplied coverage, so clear/fractional edges cannot inherit
    // history opacity or residual RGB. Current overlays are composited later.
    if opaque && finite4(n) && all(n.rgb>=vec3f(0.0)) {
        out.color=vec4f(min(n.rgb,vec3f(65504.0)),1.0);
    }
    return out;
}
