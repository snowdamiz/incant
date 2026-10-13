// Right-handed view space, forward -Z; WebGPU depth [0,1], top-left pixels.
// Fixed 16 cosine-weighted directions x 4 distance steps. This is a bounded
// first-layer screen-space visibility approximation, not hidden/offscreen geometry.
struct Parameters {
    projection:mat4x4f, inverse:mat4x4f, view:mat4x4f,
    viewport:vec4f, values:vec4f, directions:array<vec4f,16>
}
@group(0) @binding(0) var<uniform> ao:Parameters;
@group(0) @binding(1) var depths:texture_depth_2d;
@group(0) @binding(2) var normals:texture_2d<f32>;
@vertex fn vertex(@builtin(vertex_index) index:u32)->@builtin(position) vec4f {
    let p=array<vec2f,3>(vec2f(-1,-1),vec2f(3,-1),vec2f(-1,3));
    return vec4f(p[index],0,1);
}
fn inside(pixel:vec2f)->bool {
    return all(pixel>=ao.viewport.xy) && all(pixel<ao.viewport.xy+ao.viewport.zw);
}
fn position(pixel:vec2f,depth:f32)->vec3f {
    let uv=(pixel-ao.viewport.xy)/ao.viewport.zw;
    let p=ao.inverse*vec4f(uv*vec2f(2,-2)+vec2f(-1,1),depth,1);
    return p.xyz/p.w;
}
@fragment fn fragment(@builtin(position) pixel:vec4f)->@location(0) f32 {
    if !inside(pixel.xy) {return 1.;}
    let xy=vec2i(pixel.xy);let depth=textureLoad(depths,xy,0);
    let world_normal=textureLoad(normals,xy,0).xyz;
    if depth>=1. || dot(world_normal,world_normal)<0.5 {return 1.;}
    let p=position(pixel.xy,depth);
    let n=normalize((ao.view*vec4f(world_normal,0)).xyz);
    // Stable orthogonal frame even for normals parallel to either reference axis.
    let axis=select(vec3f(0,0,1),vec3f(0,1,0),abs(n.z)>0.9);
    let tangent=normalize(cross(axis,n));let bitangent=cross(n,tangent);
    var blocked=0.;
    for(var ray=0u;ray<16u;ray++) {
        let k=ao.directions[ray].xyz;
        let direction=tangent*k.x+bitangent*k.y+n*k.z;
        for(var step=1u;step<=4u;step++) {
            let q=p+direction*(ao.values.x*f32(step)/4.);
            let clip=ao.projection*vec4f(q,1);
            // Perspective points behind the eye / either clip plane do not occlude.
            if clip.w<=0. || clip.z<0. || clip.z>=clip.w {continue;}
            let uv=clip.xy/clip.w*vec2f(0.5,-0.5)+vec2f(0.5);
            let probe=ao.viewport.xy+uv*ao.viewport.zw;
            if !inside(probe) {continue;}
            let at=vec2i(probe);
            let center=vec2f(at)+vec2f(0.5);
            if !inside(center) {continue;}
            let d=textureLoad(depths,at,0);
            if d>=1. {continue;}
            let surface=position(center,d);
            let delta=surface-p;
            // Visible surface lies in front of the ray step and within the local
            // hemisphere/radius. The normal-plane bias excludes flat self-shadow.
            if surface.z>=q.z+ao.values.y && dot(delta,n)>ao.values.y
                && dot(delta,delta)<=ao.values.x*ao.values.x {
                blocked+=1.;break;
            }
        }
    }
    return clamp(1.-ao.values.z*blocked/16.,0.,1.);
}
