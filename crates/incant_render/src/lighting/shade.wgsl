struct Light { position_range:vec4f, direction_outer:vec4f, radiance_inner:vec4f, kind:vec4u }
struct LightGrid { view:mat4x4f, viewport:vec4f, dimensions:vec4u, depth:vec4f, lights:vec4u }
@group(3) @binding(0) var<uniform> light_grid:LightGrid;
@group(3) @binding(1) var<storage,read> punctual_lights:array<Light>;
@group(3) @binding(2) var<storage,read> cluster_counts:array<u32>;
@group(3) @binding(3) var<storage,read> cluster_masks:array<u32>;
fn shade_light(light:Light,world:vec3f,n:vec3f,v:vec3f,base:vec3f,metallic:f32,roughness:f32)->vec3f {
    var l=-light.direction_outer.xyz;var attenuation=1.0;
    if light.kind.x!=0u {
        let delta=light.position_range.xyz-world;let distance2=dot(delta,delta);
        let range2=light.position_range.w*light.position_range.w;
        if distance2>=range2 {return vec3f(0.0);}
        l=unit(delta);
        // C1 range window: squaring (1-(d/r)^4) gives zero value and zero slope at
        // d=r, so the cutoff has no visible rim. 1 cm (1e-4 m^2) floor keeps
        // coincident sources finite. Filament "Physically based rendering in
        // Filament", punctual light attenuation, equation 65.
        let ratio2=distance2/range2;let window=max(1.0-ratio2*ratio2,0.0);
        attenuation=window*window/max(distance2,1e-4);
        if light.kind.x==2u {
            let cosine=dot(-l,light.direction_outer.xyz);
            let cone=clamp((cosine-light.direction_outer.w)/(light.radiance_inner.w-light.direction_outer.w),0.0,1.0);
            attenuation*=cone*cone;
        }
    }
    let h=unit(v+l);let nv=max(dot(n,v),0.0);let nl=max(dot(n,l),0.0);
    let nh=max(dot(n,h),0.0);let vh=max(dot(v,h),0.0);
    let a2=roughness*roughness*roughness*roughness;
    let denominator=nh*nh*(a2-1.0)+1.0;
    let distribution=a2/(3.14159265359*denominator*denominator);
    let visibility=0.5/max(nl*sqrt(nv*nv*(1.0-a2)+a2)+nv*sqrt(nl*nl*(1.0-a2)+a2),1e-7);
    let f0=mix(vec3f(0.04),base,metallic);let fresnel=f0+(1.0-f0)*pow(1.0-vh,5.0);
    let diffuse=(1.0-fresnel)*(1.0-metallic)*base/3.14159265359;
    return (diffuse+distribution*visibility*fresnel)*light.radiance_inner.xyz*(nl*attenuation);
}
fn direct_lighting(pixel:vec2f,world:vec3f,n:vec3f,v:vec3f,base:vec3f,metallic:f32,roughness:f32)->vec3f {
    var radiance=vec3f(0.0);
    for(var i=0u;i<light_grid.lights.x;i++) {
        radiance+=shade_light(punctual_lights[i],world,n,v,base,metallic,roughness);
    }
    if light_grid.lights.y==0u {return radiance;}
    // Explicit reference mode bypasses all culling and does not read grid buffers.
    if light_grid.lights.z!=0u {
        for(var i=light_grid.lights.x;i<light_grid.lights.x+light_grid.lights.y;i++) {
            radiance+=shade_light(punctual_lights[i],world,n,v,base,metallic,roughness);
        }
        return radiance;
    }
    let xy=min(vec2u(max(pixel-light_grid.viewport.xy,vec2f(0.0))/64.0),light_grid.dimensions.xy-vec2u(1u));
    let depth=max(-(light_grid.view*vec4f(world,1.0)).z,light_grid.depth.x);
    let z=min(u32(max(log(depth/light_grid.depth.x)*light_grid.depth.z,0.0)),light_grid.dimensions.z-1u);
    let cluster=(z*light_grid.dimensions.y+xy.y)*light_grid.dimensions.x+xy.x;
    if cluster_counts[cluster]==0u {return radiance;}
    // Ascending words and least-set bits give a stable light summation order,
    // including clusters containing all 4096 supported lights. No list overflow.
    for(var word=0u;word<light_grid.dimensions.w;word++) {
        var mask=cluster_masks[cluster*light_grid.dimensions.w+word];
        while mask!=0u {
            let index=light_grid.lights.x+word*32u+firstTrailingBit(mask);
            radiance+=shade_light(punctual_lights[index],world,n,v,base,metallic,roughness);
            mask&=mask-1u;
        }
    }
    return radiance;
}
