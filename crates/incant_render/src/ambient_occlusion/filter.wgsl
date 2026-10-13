// Separable, plane-aware five-tap filter. Both intermediate writes use R8Unorm.
// Retain the authored shading normal and the original fractional viewport.
struct Parameters {
    projection:mat4x4f, inverse:mat4x4f, view:mat4x4f,
    viewport:vec4f, values:vec4f, directions:array<vec4f,16>, rotations:array<vec4f,16>
}
@group(0) @binding(0) var<uniform> ao:Parameters;
@group(0) @binding(1) var depths:texture_depth_2d;
@group(0) @binding(2) var normals:texture_2d<f32>;
@group(0) @binding(3) var accessibility:texture_2d<f32>;
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
fn filtered(pixel:vec2f,axis:vec2i)->f32 {
    if !inside(pixel) {return 1.;}
    let xy=vec2i(pixel);
    let depth=textureLoad(depths,xy,0);
    let stored=textureLoad(normals,xy,0).xyz;
    if depth>=1. || dot(stored,stored)<0.5 {return 1.;}
    let p=position(pixel,depth);
    let n=normalize((ao.view*vec4f(stored,0)).xyz);
    let tau=max(ao.values.y,0.01*(-p.z));
    // Explicit positive center term, without a duplicate/clamped edge sample.
    var total=textureLoad(accessibility,xy,0).r;
    var mass=1.;
    for(var offset=-2;offset<=2;offset++) {
        if offset==0 {continue;}
        let at=xy+axis*offset;
        let center=vec2f(at)+vec2f(0.5);
        if !inside(center) {continue;}
        let d=textureLoad(depths,at,0);
        if d>=1. {continue;}
        let q=position(center,d);
        let coefficient=select(1.,0.5,abs(offset)==2);
        let weight=coefficient*clamp(1.-abs(dot(q-p,n))/tau,0.,1.);
        total+=weight*textureLoad(accessibility,at,0).r;
        mass+=weight;
    }
    return clamp(total/mass,0.,1.);
}
@fragment fn horizontal(@builtin(position) pixel:vec4f)->@location(0) f32 {
    return filtered(pixel.xy,vec2i(1,0));
}
@fragment fn vertical(@builtin(position) pixel:vec4f)->@location(0) f32 {
    return filtered(pixel.xy,vec2i(0,1));
}
