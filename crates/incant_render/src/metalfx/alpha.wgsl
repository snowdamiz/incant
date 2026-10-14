// Final alpha RNE represented exactly in f32; native half casts may flush.
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
