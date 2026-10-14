// An actual constant infinite background, not inferred vendor alpha/coverage.
fn clear_sample(c:vec4f,z:f32,m:vec4f)->bool {
    return all(bitcast<vec4u>(c)==vec4u(0u)) && bitcast<u32>(z)==0x3f800000u
        && all(bitcast<vec4u>(m)==vec4u(0u));
}
fn domain_sample(c:vec4f,z:f32,m:vec4f)->bool {
    return clear_sample(c,z,m) || (admitted(c) && c.a==1.0
        && (bitcast<u32>(z)&0x7f800000u)!=0x7f800000u && z>=0.0 && z<1.0);
}
