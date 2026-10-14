@group(0) @binding(1) var current:texture_2d<f32>;
@group(0) @binding(2) var motion:texture_2d<f32>;
@group(0) @binding(3) var depth:texture_depth_2d;
struct Packed { @location(0) color:vec4f, @location(1) depth:f32, @location(2) motion:vec2f, @location(3) reactive:f32 }
@fragment fn pack(@builtin(position) p:vec4f)->Packed {
    var out:Packed; out.color=vec4f(0.0);out.depth=1.0;out.motion=vec2f(0.0);out.reactive=1.0;
    if any(p.xy>=frame.source.zw) {return out;}
    let at=vec2i(frame.source.xy+p.xy);
    let c=textureLoad(current,at,0);let z=textureLoad(depth,at,0);let m=textureLoad(motion,at,0);
    if !admitted(c) || c.a!=1.0 || !(z>=0.0 && z<1.0) {return out;}
    out.color=clean(c);out.depth=z;
    // Current->previous UV includes raster jitter. Remove both exact accepted
    // jitter offsets before converting to current-content pixel displacement.
    // The engine producer supplies two viewport UV positions in [0,1].
    // Check that bounded domain before subtraction/multiplication.
    if !finite4(m) || m.w<=0.5 || any(abs(m.xy)>vec2f(1.0)) {return out;}
    let delta=(m.xy+frame.jitter.xy-frame.jitter.zw)*frame.source.zw;
    if all(abs(delta)<=vec2f(65504.0)) {
        out.motion=delta; out.reactive=select(1.0,0.0,frame.depth.w>0.5);
    }
    return out;
}
