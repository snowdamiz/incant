// Core metallic-roughness equations; preview lighting is separate frame data.
struct Frame { view_projection: mat4x4f, eye:vec4f, environment:vec4f }
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
@group(2) @binding(0) var specular_environment:texture_cube<f32>;
@group(2) @binding(1) var diffuse_environment:texture_cube<f32>;
@group(2) @binding(2) var brdf_lut:texture_2d<f32>;
@group(2) @binding(3) var environment_sampler:sampler;
struct VertexOut { @builtin(position) position:vec4f, @location(0) world:vec3f, @location(1) normal:vec3f, @location(2) tangent:vec4f, @location(3) uv:vec2f }
fn unit(v:vec3f) -> vec3f { return v*inverseSqrt(max(dot(v,v),1e-16)); }
// Rotate world directions into the source image's coordinates (inverse +Y yaw).
fn environment_direction(d:vec3f) -> vec3f {
    let c=frame.environment.y;let s=frame.environment.z;
    return vec3f(c*d.x-s*d.z,d.y,s*d.x+c*d.z);
}
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
    var surface_normal=unit(in.normal);
    var n=surface_normal;
    if material.flags.y!=0u {
        let t=unit(in.tangent.xyz-n*dot(n,in.tangent.xyz));let b=cross(n,t)*in.tangent.w;
        let mapped=unit(normal_sample*vec3f(material.factors.y,material.factors.y,1.0));
        n=unit(mat3x3f(t,b,n)*mapped);
    }
    if material.flags.z!=0u && !front {n=-n;surface_normal=-surface_normal;}
    let v=unit(frame.eye.xyz-in.world);let nv=max(dot(n,v),0.0);
    let metallic=clamp(material.factors.x*mr.b,0.0,1.0);let roughness=clamp(material.emissive_roughness.w*mr.g,0.045,1.0);
    let f0=mix(vec3f(0.04),base.rgb,metallic);
    let direct=direct_lighting(in.position.xy,in.world,n,surface_normal,v,base.rgb,metallic,roughness);
    let dfg=textureSampleLevel(brdf_lut,environment_sampler,vec2f(nv,roughness),0.0).rg;
    let reflectance=f0*dfg.x+dfg.y;
    let irradiance=textureSampleLevel(diffuse_environment,environment_sampler,environment_direction(n),0.0).rgb;
    let reflected=textureSampleLevel(specular_environment,environment_sampler,
        environment_direction(reflect(-v,n)),roughness*frame.environment.w).rgb;
    // Single scattering loses energy at high roughness. Keep diffuse within the
    // remaining energy; multiscattering compensation is a separate future model.
    let indirect=((1.0-reflectance)*(1.0-metallic)*base.rgb*irradiance+reflectance*reflected)
        *frame.environment.x*occlusion;
    // Bound finite radiance before half-float storage; values above one survive.
    let color=clamp(direct+indirect+emissive,vec3f(0),vec3f(65504));
    return vec4f(color,select(1.0,base.a,material.flags.x==2u));
}
