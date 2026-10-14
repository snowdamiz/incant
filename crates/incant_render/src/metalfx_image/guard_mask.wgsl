// Diagnostic only: never supplies history, normalized color, or displayed output.
// Bits: coverage1, positive-depth2, compatible-depth4, native-finite8,
// native-nonnegative16, actual native-RGB selection32. Reserved bits stay zero.
@fragment fn guard_mask(@builtin(position) p:vec4f)->@location(0) u32 {
    if !inside(p.xy,frame.output) { return 0u; }
    let q=source_position(p.xy)-vec2f(0.5);let base=vec2i(floor(q));let f=fract(q);
    var coverage=true;var positive=true;var z=frame.depth.y+1.0;
    for(var y=0;y<2;y++) {for(var x=0;x<2;x++) {
        if tap_weight(f,x,y)<=0.0 {continue;}
        let tap=clamp(base+vec2i(x,y),lo(frame.source),hi(frame.source));
        let c=textureLoad(current,tap,0);let d=linear_depth(textureLoad(depth,tap,0));
        coverage=coverage && admitted(c) && c.a==1.0;
        positive=positive && d>0.0;
        if d>0.0 {z=min(z,d);}
    }}
    var depth_agrees=true;
    for(var y=0;y<2;y++) {for(var x=0;x<2;x++) {
        if tap_weight(f,x,y)<=0.0 {continue;}
        let tap=clamp(base+vec2i(x,y),lo(frame.source),hi(frame.source));
        depth_agrees=depth_agrees && compatible(linear_depth(textureLoad(depth,tap,0)),z);
    }}
    let n=textureLoad(native_color,vec2i(p.xy-frame.output.xy),0);
    let finite=finite4(n);
    // No arithmetic/comparison on nonfinite vendor data before bit classification.
    var nonnegative=false;if finite {nonnegative=all(n.rgb>=vec3f(0.0));}
    return select(0u,1u,coverage)|select(0u,2u,positive)|select(0u,4u,depth_agrees)
        |select(0u,8u,finite)|select(0u,16u,nonnegative)
        |select(0u,32u,coverage && positive && depth_agrees && finite && nonnegative);
}
