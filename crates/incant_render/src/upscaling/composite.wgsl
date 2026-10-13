@group(0) @binding(1) var resolved:texture_2d<f32>;
@group(0) @binding(2) var overlay:texture_2d<f32>;
@fragment fn composite(@builtin(position) position:vec4f)->@location(0) vec4f {
    let p=position.xy;
    if !inside(p,frame.output) {return vec4f(0.0);}
    // Excluded overlays are rasterized on the unjittered internal grid.
    let source=frame.source.xy+(p-frame.output.xy)/frame.output.zw*frame.source.zw;
    let q=source-vec2f(0.5);let base=vec2i(floor(q));let f=fract(q);
    var over=vec4f(0.0);
    for(var y=0;y<2;y++) {for(var x=0;x<2;x++) {
        let tap=clamp(base+vec2i(x,y),lo(frame.source),hi(frame.source));
        over+=clean(textureLoad(overlay,tap,0))*tap_weight(f,x,y);
    }}
    let under=clean(textureLoad(resolved,vec2i(p),0));
    // Both attachments store premultiplied scene-linear color over transparent
    // black. Preserve depth/order in the earlier shared-depth overlay passes.
    return clean(over+under*(1.0-over.a));
}
