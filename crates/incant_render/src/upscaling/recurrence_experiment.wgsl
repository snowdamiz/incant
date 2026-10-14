// TEST-ONLY candidate. Four reprojected raw lumas are a recurrence heuristic,
// not a surface/radiance identity proof. In particular, genuinely oscillating
// lighting can look exactly like undersampling. No shipping pipeline uses this.
// Binary16 neighbours account for storage conversion, NOT all GPU arithmetic.
fn recurrence_luma(rgb:vec3f)->f32 {return dot(rgb,vec3f(0.2126,0.7152,0.0722));}
fn recurrence_valid(v:vec4f)->vec4<bool> {
    return (v>=vec4f(0.0)) & (v<=vec4f(65504.0));
}
fn recurrence_quantize(v:vec4f)->vec4f {
    return vec4f(unpack2x16float(pack2x16float(v.xy)),unpack2x16float(pack2x16float(v.zw)));
}
fn recurrence_bounds(v:f32)->vec2f {
    // Clear the sign bit explicitly: clamp is allowed to preserve negative zero.
    let bits=pack2x16float(vec2f(clamp(v,0.0,65504.0),0.0))&32767u;
    // Avoid unsigned underflow at zero. Both endpoints are finite half values.
    let lower=bits-min(bits,1u);
    let upper=min(bits+1u,31743u);
    return vec2f(unpack2x16float(lower).x,unpack2x16float(upper).x);
}
struct RecurrenceCheck { consistent:bool, recurring:bool, turn:bool }
fn recurrence_check(history:vec3f,fresh:vec3f,minimum:vec3f,maximum:vec3f,ring:vec4f)->RecurrenceCheck {
    if !all(recurrence_valid(ring)) {return RecurrenceCheck(false,false,false);}
    let fy=recurrence_luma(fresh);let hy=recurrence_luma(history);
    if !(fy>0.0 && fy<=65504.0 && hy>0.0 && hy<=65504.0) {return RecurrenceCheck(false,false,false);}
    let ordered=array<f32,5>(ring.w,ring.z,ring.y,ring.x,fy);
    var lower=65504.0;var upper=0.0;var rising=false;var falling=false;
    var previous=recurrence_bounds(ordered[0]);
    for(var i=0u;i<5u;i++) {
        let bounds=recurrence_bounds(ordered[i]);
        lower=min(lower,bounds.x);upper=max(upper,bounds.y);
        if i>0u {
            rising=rising || bounds.x>previous.y;
            falling=falling || bounds.y<previous.x;
        }
        previous=bounds;
    }
    let hb=recurrence_bounds(hy);
    if hb.x<lower || hb.y>upper {return RecurrenceCheck(false,false,false);}
    let chroma=history*(fy/hy);
    if !all(chroma>=vec3f(0.0)) || !all(chroma<=vec3f(65504.0)) {return RecurrenceCheck(false,false,false);}
    for(var i=0u;i<3u;i++) {
        if chroma[i]<recurrence_bounds(minimum[i]).x || chroma[i]>recurrence_bounds(maximum[i]).y {return RecurrenceCheck(false,false,false);}
    }
    let older=recurrence_bounds(ring.y);let last=recurrence_bounds(ring.x);
    let now=recurrence_bounds(fy);
    let turn=(last.x>older.y && now.y<last.x) || (last.y<older.x && now.x>last.y);
    return RecurrenceCheck(true,rising && falling,turn);
}
fn recurrence_retains(history:vec3f,fresh:vec3f,minimum:vec3f,maximum:vec3f,ring:vec4f)->bool {
    let check=recurrence_check(history,fresh,minimum,maximum,ring);
    return check.consistent && check.recurring;
}
// Diagnostic only: one eight-phase jitter cycle including the recognition frame.
// Only a NEW observed reversal renews the remaining seven frames. An old reversal
// remaining in the ring cannot renew the lease. Geometry/missing lag/color refusal
// discards it immediately. An actual oscillating light remains a counterexample.
fn recurrence_lock(history:vec3f,fresh:vec3f,minimum:vec3f,maximum:vec3f,ring:vec4f,remaining:f32)->vec2f {
    let check=recurrence_check(history,fresh,minimum,maximum,ring);
    if !check.consistent {return vec2f(0.0);}
    if check.turn && check.recurring {return vec2f(1.0,7.0);}
    if remaining>0.0 {return vec2f(1.0,max(remaining-1.0,0.0));}
    return vec2f(select(0.0,1.0,check.recurring),0.0);
}
