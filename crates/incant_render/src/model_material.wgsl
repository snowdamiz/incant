// Core metallic-roughness equations; preview lighting is separate frame data.
struct Frame { view_projection: mat4x4f, eye:vec4f, light:vec4f, radiance:vec4f, environment:vec4f }
struct Material { base:vec4f, emissive_roughness:vec4f, factors:vec4f, flags:vec4u }
@group(0) @binding(0) var<uniform> frame:Frame;
@group(1) @binding(0) var<uniform> material:Material;
@group(1) @binding(1) var base_map:texture_2d<f32>;
@group(1) @binding(2) var base_sampler:sampler;
@group(1) @binding(3) var mr_map:texture_2d<f32>;
@group(1) @binding(4) var mr_sampler:sampler;
@group(1) @binding(5) var normal_map:texture_2d<f32>;
@group(1) @binding(6) var normal_sampler:sampler;
@group(1) @binding(7) var occlusion_map:texture_2d<f32>;
@group(1) @binding(8) var occlusion_sampler:sampler;
@group(1) @binding(9) var emissive_map:texture_2d<f32>;
@group(1) @binding(10) var emissive_sampler:sampler;
struct VertexOut { @builtin(position) position:vec4f, @location(0) world:vec3f, @location(1) normal:vec3f, @location(2) tangent:vec4f, @location(3) uv:vec2f }
fn unit(v:vec3f) -> vec3f { return v*inverseSqrt(max(dot(v,v),1e-16)); }
@vertex fn vertex(@location(0) position:vec3f,@location(1) normal:vec3f,
    @location(2) w0:vec4f,@location(3) w1:vec4f,@location(4) w2:vec4f,@location(5) w3:vec4f,
    @location(6) n0:vec4f,@location(7) n1:vec4f,@location(8) n2:vec4f,@location(9) uv:vec2f,@location(10) tangent:vec4f) -> VertexOut {
    let world=mat4x4f(w0,w1,w2,w3); let linear=mat3x3f(w0.xyz,w1.xyz,w2.xyz);
    var out:VertexOut; let p=world*vec4f(position,1.0);
    out.position=frame.view_projection*p;out.world=p.xyz;
    out.normal=mat3x3f(n0.xyz,n1.xyz,n2.xyz)*normal;
    out.tangent=vec4f(linear*tangent.xyz,tangent.w*n0.w);
    out.uv=uv;return out;
}
@fragment fn fragment(in:VertexOut,@builtin(front_facing) front:bool) -> @location(0) vec4f {
    // Derivative-based samples are evaluated uniformly before alpha discard.
    let base=material.base*textureSample(base_map,base_sampler,in.uv);
    let mr=textureSample(mr_map,mr_sampler,in.uv);
    let normal_sample=textureSample(normal_map,normal_sampler,in.uv).xyz*2.0-1.0;
    let occlusion=1.0+material.factors.z*(textureSample(occlusion_map,occlusion_sampler,in.uv).r-1.0);
    let emissive=material.emissive_roughness.xyz*textureSample(emissive_map,emissive_sampler,in.uv).xyz;
    if material.flags.x==1u && base.a<material.factors.w { discard; }
    var n=unit(in.normal);
    if material.flags.y!=0u {
        let t=unit(in.tangent.xyz-n*dot(n,in.tangent.xyz));let b=cross(n,t)*in.tangent.w;
        let mapped=unit(normal_sample*vec3f(material.factors.y,material.factors.y,1.0));
        n=unit(mat3x3f(t,b,n)*mapped);
    }
    if material.flags.z!=0u && !front {n=-n;}
    let v=unit(frame.eye.xyz-in.world);let l=unit(frame.light.xyz);let h=unit(v+l);
    let nv=max(dot(n,v),0.0);let nl=max(dot(n,l),0.0);let nh=max(dot(n,h),0.0);let vh=max(dot(v,h),0.0);
    let metallic=clamp(material.factors.x*mr.b,0.0,1.0);let roughness=clamp(material.emissive_roughness.w*mr.g,0.045,1.0);
    let alpha=roughness*roughness;let a2=alpha*alpha;
    let denominator=nh*nh*(a2-1.0)+1.0;let distribution=a2/(3.14159265359*denominator*denominator);
    let visibility=0.5/max(nl*sqrt(nv*nv*(1.0-a2)+a2)+nv*sqrt(nl*nl*(1.0-a2)+a2),1e-7);
    let f0=mix(vec3f(0.04),base.rgb,metallic);let fresnel=f0+(1.0-f0)*pow(1.0-vh,5.0);
    let diffuse=(1.0-fresnel)*(1.0-metallic)*base.rgb/3.14159265359;
    let direct=(diffuse+distribution*visibility*fresnel)*frame.radiance.xyz*nl;
    // Diffuse preview environment only: no specular IBL or shadow claim.
    let indirect=frame.environment.xyz*base.rgb*(1.0-metallic)*occlusion;
    let color=direct+indirect+emissive;
    return vec4f(color,select(1.0,base.a,material.flags.x==2u));
}
