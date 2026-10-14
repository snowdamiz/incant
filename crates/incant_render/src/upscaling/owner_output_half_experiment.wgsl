// Test-only RGBA16Float output boundary shared by Tent and Cubic. Quantize the
// final alpha explicitly to binary16 round-to-nearest, ties-to-even, expressed
// as an exact f32. Native pack/unpack half casts can choose different rounding
// or flush subnormals; they did not preserve the actual tiny-alpha control.
fn output_alpha(value:f32)->f32 {
    if !(value>0.0) {return 0.0;}
    if value>=1.0 {return 1.0;}
    let bits=bitcast<u32>(value);
    // Below the midpoint between zero and the smallest positive binary16.
    if bits<0x33000000u {return 0.0;}
    if bits<0x38800000u {
        let shift=126u-(bits>>23u);
        let significand=(bits&0x7fffffu)|0x800000u;
        let whole=significand>>shift;
        let remainder=significand&((1u<<shift)-1u);
        let midpoint=1u<<(shift-1u);
        let increment=remainder>midpoint || (remainder==midpoint && (whole&1u)!=0u);
        let rounded=whole+select(0u,1u,increment);
        // Integer conversion and this power-of-two product are exact in f32.
        return f32(rounded)*0.000000059604644775390625;
    }
    let rounded=(bits+0xfffu+((bits>>13u)&1u))&0xffffe000u;
    return bitcast<f32>(rounded);
}
// Only the final attachment value is changed; no intermediate color, evidence
// mass or provenance is quantized here. Zero alpha requires exact zero RGB.
@fragment fn resolve(@builtin(position) position:vec4f)->OwnerResolved {
    var result=experiment_resolve(position);
    let stored_alpha=output_alpha(result.color.a);
    if stored_alpha==0.0 {result.color=vec4f(0.0);}
    else {result.color.a=stored_alpha;}
    return result;
}
