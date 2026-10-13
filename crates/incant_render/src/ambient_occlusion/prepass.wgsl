// Appended to the unchanged material declarations+vertex entry. Retained animated
// vertex buffers, world and inverse-transpose normal matrices are identical.
@fragment fn fragment(in:VertexOut,@builtin(front_facing) front:bool)->@location(0) vec4f {
    let base=material.base*textureSample(base_map,base_sampler,in.uv);
    if material.flags.x==1u && base.a<material.factors.w {discard;}
    var n=unit(in.normal);
    if material.flags.z!=0u && !front {n=-n;}
    return vec4f(n,1);
}
