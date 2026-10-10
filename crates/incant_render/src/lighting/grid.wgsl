// Position-only view-space clusters: 64-pixel tiles and 24 logarithmic Z slices.
// Bounds conservatively retain every sphere intersecting the cluster frustum.
struct Light { position_range:vec4f, direction_outer:vec4f, radiance_inner:vec4f, kind:vec4u }
struct Grid { view:mat4x4f, viewport:vec4f, dimensions:vec4u, depth:vec4f, lights:vec4u }
@group(0) @binding(0) var<uniform> grid:Grid;
@group(0) @binding(1) var<storage,read> lights:array<Light>;
@group(0) @binding(2) var<storage,read_write> counts:array<u32>;
@group(0) @binding(3) var<storage,read_write> masks:array<u32>;
var<workgroup> hits:atomic<u32>;
// 4096 local lights / 32 bits. Each set bit addresses one local light.
var<workgroup> membership:array<atomic<u32>,128>;
fn intersects(position:vec3f,radius:f32,id:vec3u)->bool {
    // Conservative tolerance covers view-transform/plane arithmetic at far Z.
    let r=radius+1e-5*max(1.0,max(max(abs(position.x),abs(position.y)),abs(position.z)));
    let z0=grid.depth.x*exp(f32(id.z)/grid.depth.z);
    let z1=grid.depth.x*exp(f32(id.z+1u)/grid.depth.z);
    let p=vec3f(position.xy,-position.z);
    if p.z+r<z0 || p.z-r>z1 {return false;}
    let lo=vec2f(id.xy)*64.0/grid.viewport.zw;
    let hi=min(vec2f(id.xy+vec2u(1u))*64.0/grid.viewport.zw,vec2f(1.0));
    let tan_y=grid.depth.w;let tan_x=tan_y*grid.viewport.z/grid.viewport.w;
    let left=(lo.x*2.0-1.0)*tan_x;let right=(hi.x*2.0-1.0)*tan_x;
    let top=(1.0-lo.y*2.0)*tan_y;let bottom=(1.0-hi.y*2.0)*tan_y;
    if grid.lights.w!=0u {
        // Parallel rays form a box in each Z slice, with depth-independent XY.
        return p.x+r>=left && p.x-r<=right && p.y+r>=bottom && p.y-r<=top;
    }
    // Non-unit plane normals: scale sphere radius by their lengths.
    return p.x-left*p.z>=-r*sqrt(1.0+left*left)
        && right*p.z-p.x>=-r*sqrt(1.0+right*right)
        && p.y-bottom*p.z>=-r*sqrt(1.0+bottom*bottom)
        && top*p.z-p.y>=-r*sqrt(1.0+top*top);
}
@compute @workgroup_size(64) fn build(@builtin(workgroup_id) id:vec3u,@builtin(local_invocation_index) lane:u32) {
    if lane==0u {atomicStore(&hits,0u);}
    for(var word=lane;word<grid.dimensions.w;word+=64u) {atomicStore(&membership[word],0u);}
    workgroupBarrier();
    let cluster=(id.z*grid.dimensions.y+id.y)*grid.dimensions.x+id.x;
    for(var local=lane;local<grid.lights.y;local+=64u) {
        let light=lights[grid.lights.x+local];
        let p=(grid.view*vec4f(light.position_range.xyz,1.0)).xyz;
        if intersects(p,light.position_range.w,id) {
            atomicAdd(&hits,1u);
            atomicOr(&membership[local/32u],1u<<(local%32u));
        }
    }
    workgroupBarrier();
    for(var word=lane;word<grid.dimensions.w;word+=64u) {
        masks[cluster*grid.dimensions.w+word]=atomicLoad(&membership[word]);
    }
    if lane==0u {counts[cluster]=atomicLoad(&hits);}
}
