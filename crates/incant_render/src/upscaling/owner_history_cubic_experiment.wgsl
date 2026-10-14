// Test-only history-color reconstruction. The baseline positive tent still
// supplies the effective evidence mass. This is deliberately a different
// estimator, not a signed mass numerator divided by a positive denominator.
struct CubicHistory { color:vec4f, age:f32, ok:bool }
fn cubic_history(p:vec2f,selected_old:vec2i,selected_projection:vec2f,
    selected:u32,nearest:f32,e:Evidence)->CubicHistory {
    let refused=CubicHistory(vec4f(0.0),0.0,false);
    // One regular old-grid query from the selected displacement. Every
    // nonzero contributor must independently prove that same local translation.
    // A depth tolerance alone does not establish a common sampling grid.
    let query=vec2f(selected_old)+vec2f(0.5)+p-selected_projection;
    if !inside(query,frame.output) {return refused;}
    let base=vec2i(floor(query-vec2f(0.5)));
    let fraction=fract(query-vec2f(0.5));
    // Unlike the positive support search, cubic support extends an extra old
    // grid cell. No texture clamping or partial-kernel renormalization.
    if any(base-vec2i(1)<lo(frame.output)) || any(base+vec2i(2)>hi(frame.output)) {return refused;}
    var sum=vec4f(0.0);var normalization=0.0;var age=0.0;
    var minimum=vec4f(65504.0);var maximum=vec4f(0.0);
    for(var y=-1;y<=2;y++) {for(var x=-1;x<=2;x++) {
        let weight=catmull_rom(f32(x)-fraction.x)*catmull_rom(f32(y)-fraction.y);
        // The central range is part of the operation and must have the same
        // provenance checks even when its interpolation coefficient is zero.
        let central=x>=0 && x<=1 && y>=0 && y<=1;
        if weight==0.0 && !central {continue;}
        let t=base+vec2i(x,y);
        let color=textureLoad(previous_color,t,0);
        let raw_record=textureLoad(previous_record,t,0).xy;
        let depth=textureLoad(previous_depth,t,0).x;
        let raw_evidence=textureLoad(previous_evidence,t,0).xy;
        if !admitted(color) {return refused;}
        let empty=all(color==vec4f(0.0)) && all(raw_record==vec2u(0u))
            && depth==0.0 && all(raw_evidence==vec2f(0.0));
        if !empty {
            let record=stored(raw_record);
            let evidence=stored_evidence(vec4f(raw_evidence,0.0,0.0));
            if !record.ok || record.over || record.b!=0u || record.a!=selected
                || changed_id(record.a) || !(color.a>0.0 && color.a<1.0)
                || !(evidence.x>0.0) {return refused;}
            let next_age=select(evidence.y+1.0,0.0,e.surface && sees(e.owners,record.a));
            if next_age>=MAX_AGE {return refused;}
            let projected=camera_project(vec2f(t)+vec2f(0.5),depth);
            if projected.w==0.0 || !compatible(projected.z,nearest) {return refused;}
            let expected=p+vec2f(t)+vec2f(0.5)-query;
            if any(abs(projected.xy-expected)>=vec2f(1.0/256.0)) {return refused;}
            if e.surface && e.nearest<projected.z && !compatible(e.nearest,projected.z) {return refused;}
            age=max(age,next_age);
        }
        if central {minimum=min(minimum,color);maximum=max(maximum,color);}
        sum+=color*weight;normalization+=weight;
    }}
    if !finite4(sum) || !(normalization>0.0) {return refused;}
    // Clamp signed interpolation to the admitted central range. The PMA cleanup
    // retains canonical zero-alpha output and never introduces negative RGB.
    return CubicHistory(clean(clamp(sum/normalization,minimum,maximum)),age,true);
}
