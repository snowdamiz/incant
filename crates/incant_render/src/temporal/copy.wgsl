@group(0) @binding(0) var color:texture_2d<f32>;
@vertex fn vertex(@builtin(vertex_index) i:u32)->@builtin(position) vec4f {
    let uv=vec2f(f32((i<<1u)&2u),f32(i&2u));
    return vec4f(uv*2.0-1.0,0.0,1.0);
}
@fragment fn copy(@builtin(position) p:vec4f)->@location(0) vec4f {
    return textureLoad(color,vec2i(p.xy),0);
}
