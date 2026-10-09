struct ShadowCascade { matrix:mat4x4f, parameters:vec4f }
struct ShadowData { cascades:array<ShadowCascade,16> }
@group(3) @binding(4) var shadow_maps:texture_depth_2d_array;
@group(3) @binding(5) var shadow_sampler:sampler_comparison;
@group(3) @binding(6) var<uniform> shadow_data:ShadowData;
fn sample_cascade(index:u32,world:vec3f,n:vec3f)->f32 {
    let cascade=shadow_data.cascades[index];
    let p=cascade.matrix*vec4f(world+n*cascade.parameters.z*0.25,1.0);
    let uv=vec2f(p.x,-p.y)*0.5+0.5;
    if any(uv<vec2f(0.0)) || any(uv>vec2f(1.0)) || p.z<0.0 || p.z>1.0 {return 1.0;}
    let texel=1.0/vec2f(textureDimensions(shadow_maps));
    var sum=0.0;
    for(var y=-1;y<=1;y++) {
        for(var x=-1;x<=1;x++) {
            sum+=textureSampleCompareLevel(shadow_maps,shadow_sampler,uv+vec2f(f32(x),f32(y))*texel,i32(index),p.z);
        }
    }
    return sum/9.0;
}
fn directional_shadow(slot:u32,world:vec3f,n:vec3f)->f32 {
    if slot==0u {return 1.0;}
    let base=(slot-1u)*4u;
    let depth=-(light_grid.view*vec4f(world,1.0)).z;
    for(var i=0u;i<4u;i++) {
        let params=shadow_data.cascades[base+i].parameters;
        if depth<=params.x && params.x>0.0 {
            let shadow=sample_cascade(base+i,world,n);
            if depth<=params.y {return shadow;}
            var next=1.0;
            if i<3u {next=sample_cascade(base+i+1u,world,n);}
            return mix(shadow,next,clamp((depth-params.y)/(params.x-params.y),0.0,1.0));
        }
    }
    return 1.0;
}
