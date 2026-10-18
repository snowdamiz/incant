//! Numerical test output only. This module never participates in frame admission.
use crate::{Renderer, RetainedScene};
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub(super) struct Sample {
    pub color: [f32; 4],
    pub depth: f32,
    pub mask: u32,
    pub visible_depth: f32,
    pub visible_owner: u32,
}
const COPY: &str = r#"
@group(0) @binding(0) var color:texture_multisampled_2d<f32>;
@group(0) @binding(1) var depth:texture_depth_multisampled_2d;
@group(0) @binding(2) var mask:texture_2d<u32>;
@group(0) @binding(3) var visible_owner:texture_multisampled_2d<u32>;
@group(0) @binding(4) var visible_depth:texture_depth_multisampled_2d;
struct Sample {color:vec4f,depth:f32,mask:u32,visible_depth:f32,visible_owner:u32}
@group(0) @binding(5) var<storage,read_write> result:array<Sample>;
@compute @workgroup_size(64) fn copy(@builtin(global_invocation_id) id:vec3u) {
    let size=textureDimensions(color);let i=id.x;if i>=size.x*size.y*4u {return;}
    let pixel=i/4u;let p=vec2i(vec2u(pixel%size.x,pixel/size.x));let lane=i32(i%4u);
    let owner=textureLoad(visible_owner,p,lane);
    result[i]=Sample(textureLoad(color,p,lane),textureLoad(depth,p,lane),textureLoad(mask,p,0).x,textureLoad(visible_depth,p,lane),owner.x|(owner.y<<16u));
}
"#;
pub(super) struct Reader(wgpu::ComputePipeline);
impl Reader {
    pub fn new(r: &Renderer) -> Self {
        let shader = r.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Reconstruction numerical readback"),
            source: wgpu::ShaderSource::Wgsl(COPY.into()),
        });
        Self(
            r.device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: None,
                    layout: None,
                    module: &shader,
                    entry_point: Some("copy"),
                    compilation_options: Default::default(),
                    cache: None,
                }),
        )
    }
    pub fn read(&self, r: &Renderer, scene: &RetainedScene) -> Vec<Sample> {
        let views = scene.visibility_current_views();
        let count = views[0].texture().width() * views[0].texture().height() * 4;
        assert!(count <= 128 * 128 * 4);
        let bytes = u64::from(count) * size_of::<Sample>() as u64;
        let storage = r.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: bytes,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let readback = r.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: bytes,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut entries: Vec<_> = views
            .iter()
            .enumerate()
            .map(|(i, v)| wgpu::BindGroupEntry {
                binding: i as u32,
                resource: wgpu::BindingResource::TextureView(v),
            })
            .collect();
        entries.push(wgpu::BindGroupEntry {
            binding: 5,
            resource: storage.as_entire_binding(),
        });
        let group = r.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &self.0.get_bind_group_layout(0),
            entries: &entries,
        });
        let mut e = r.device.create_command_encoder(&Default::default());
        {
            let mut p = e.begin_compute_pass(&Default::default());
            p.set_pipeline(&self.0);
            p.set_bind_group(0, &group, &[]);
            p.dispatch_workgroups(count.div_ceil(64), 1, 1);
        }
        e.copy_buffer_to_buffer(&storage, 0, &readback, 0, bytes);
        r.queue.submit([e.finish()]);
        let (tx, rx) = std::sync::mpsc::channel();
        readback.slice(..).map_async(wgpu::MapMode::Read, move |v| {
            let _ = tx.send(v);
        });
        r.device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(std::time::Duration::from_secs(10)),
            })
            .unwrap();
        rx.recv_timeout(std::time::Duration::from_secs(10))
            .unwrap()
            .unwrap();
        let map = readback.slice(..).get_mapped_range();
        let pixels = map
            .as_chunks::<32>()
            .0
            .iter()
            .map(|bytes| bytemuck::pod_read_unaligned(bytes))
            .collect();
        drop(map);
        readback.unmap();
        pixels
    }
}
#[test]
fn numerical_current_radiance_copy_validates() {
    let module = naga::front::wgsl::parse_str(COPY).unwrap();
    naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    )
    .validate(&module)
    .unwrap();
    assert_eq!(size_of::<Sample>(), 32);
}
