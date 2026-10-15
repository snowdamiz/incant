// Finite restrained base color while the matching engine material PSO is pending.
// Same centroid UVs, texture/sampler, alpha multiplication and MASK threshold as
// the real material; pipeline state preserves culling, depth, MSAA and blending.
@fragment fn fragment(in:VertexOut,@builtin(front_facing) front:bool)->@location(0) vec4f {
    let base=material.base*textureSample(base_map,base_sampler,in.uv);
    if material.flags.x==1u && base.a<material.factors.w {discard;}
    let color=clamp(base.rgb,vec3f(0),vec3f(1))*0.65;
    return vec4f(color,select(1.0,base.a,material.flags.x==2u));
}
