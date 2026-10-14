    // Canonical empty color + clear depth + cleared motion proves background.
    // Unsupported/invalid current records continue through the original refusal.
    if clear_sample(c,z,m) {
        out.color=vec4f(frame.reactive.rgb,1.0);
        // Constant infinity has no unjittered motion, including camera rotation.
        out.reactive=select(1.0,0.0,frame.reactive.w>0.5 && frame.depth.w>0.5);
        return out;
    }
    if (bitcast<u32>(z)&0x7f800000u)==0x7f800000u {return out;}
